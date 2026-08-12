use parking_lot::Mutex;
use std::collections::HashMap;
use std::fmt;
use std::process::Command;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

/// Process info stored in cache
#[derive(Clone, Debug)]
struct ProcessInfo {
    command: String,
    parent_pid: Option<u32>,
}

/// Cached process tree for efficient child process lookup
struct ProcessTreeCache {
    /// Map of PID -> ProcessInfo
    processes: HashMap<u32, ProcessInfo>,
    /// When the cache was last updated
    last_update: Instant,
}

impl ProcessTreeCache {
    fn new() -> Self {
        Self {
            processes: HashMap::new(),
            last_update: Instant::now() - Duration::from_secs(100), // Force initial refresh
        }
    }

    fn needs_refresh(&self) -> bool {
        self.last_update.elapsed() > Duration::from_millis(500)
    }

    fn refresh(&mut self) {
        // Get all processes in one call: PID, PPID, COMMAND
        let output = Command::new("ps")
            .args(["-A", "-o", "pid=,ppid=,command="])
            .output();

        let output = match output {
            Ok(o) if o.status.success() => o,
            _ => return,
        };

        self.processes.clear();
        let stdout = String::from_utf8_lossy(&output.stdout);

        for line in stdout.lines() {
            let parts: Vec<&str> = line.trim().splitn(3, char::is_whitespace).collect();
            if parts.len() >= 3 {
                if let (Ok(pid), Ok(ppid)) = (parts[0].parse::<u32>(), parts[1].parse::<u32>()) {
                    let cmd = parts[2].trim().to_string();
                    let parent = if ppid == 0 { None } else { Some(ppid) };
                    self.processes.insert(
                        pid,
                        ProcessInfo {
                            command: cmd,
                            parent_pid: parent,
                        },
                    );
                }
            }
        }

        self.last_update = Instant::now();
    }

    fn get_child_commands(&self, pid: u32, max_depth: u32) -> Vec<String> {
        let mut commands = Vec::new();
        self.collect_children(pid, &mut commands, 0, max_depth);
        commands
    }

    fn collect_children(&self, pid: u32, commands: &mut Vec<String>, depth: u32, max_depth: u32) {
        if depth >= max_depth {
            return;
        }

        // Find all processes with this pid as parent
        for (&child_pid, info) in &self.processes {
            if info.parent_pid == Some(pid) {
                // Add full command
                commands.push(info.command.clone());
                // Add base name
                if let Some(first) = info.command.split_whitespace().next() {
                    if let Some(base) = first.rsplit('/').next() {
                        if base != info.command {
                            commands.push(base.to_string());
                        }
                    }
                }
                // Recurse
                self.collect_children(child_pid, commands, depth + 1, max_depth);
            }
        }
    }

    fn get_cmdline(&self, pid: u32) -> Option<String> {
        self.processes.get(&pid).map(|info| info.command.clone())
    }
}

static PROCESS_CACHE: OnceLock<Mutex<ProcessTreeCache>> = OnceLock::new();

fn get_process_cache() -> &'static Mutex<ProcessTreeCache> {
    PROCESS_CACHE.get_or_init(|| Mutex::new(ProcessTreeCache::new()))
}

/// Refresh the process cache if needed (call once per poll cycle)
pub fn refresh_process_cache() {
    let mut cache = get_process_cache().lock();
    if cache.needs_refresh() {
        cache.refresh();
    }
}

/// Represents a tmux pane with its identifying information
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaneInfo {
    /// Session name
    pub session: String,
    /// Window index
    pub window: u32,
    /// Window name
    pub window_name: String,
    /// Pane index
    pub pane: u32,
    /// Current command running in the pane
    pub command: String,
    /// Pane title (often contains useful info like "Claude Code")
    pub title: String,
    /// Current working directory of the pane
    pub path: String,
    /// Process ID of the pane
    pub pid: u32,
    /// Full command line of the process
    pub cmdline: String,
    /// Child process commands (for detecting running agents)
    pub child_commands: Vec<String>,
}

impl PaneInfo {
    /// Returns the tmux target string (e.g., "session:0.1")
    pub fn target(&self) -> String {
        format!("{}:{}.{}", self.session, self.window, self.pane)
    }

    /// Parses a pane info from tmux list-panes output
    /// Expected format: "session:window.pane\twindow_name\tcommand\tpid\ttitle\tpath"
    pub fn parse(line: &str) -> Option<Self> {
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() < 6 {
            return None;
        }

        let target = parts[0];
        let window_name = parts[1].to_string();
        let command = parts[2].to_string();
        let pid: u32 = parts[3].parse().ok()?;
        let title = parts[4].to_string();
        let path = parts[5].to_string();

        // Parse target "session:window.pane"
        let (session, rest) = target.split_once(':')?;
        let (window_str, pane_str) = rest.split_once('.')?;

        let window: u32 = window_str.parse().ok()?;
        let pane: u32 = pane_str.parse().ok()?;

        // Use cached process tree for fast lookups
        let cache = get_process_cache().lock();
        let cmdline = cache.get_cmdline(pid).unwrap_or_default();
        // Depth 6: agents can sit several levels below the pane shell
        // (e.g. fish -> shell-integration wrapper -> fish -> claude)
        let child_commands = cache.get_child_commands(pid, 6);

        Some(Self {
            session: session.to_string(),
            window,
            window_name,
            pane,
            command,
            title,
            path,
            pid,
            cmdline,
            child_commands,
        })
    }

    /// Returns all detection strings (command, title, cmdline, child commands) for agent matching
    pub fn detection_strings(&self) -> Vec<&str> {
        let mut strings = vec![
            self.command.as_str(),
            self.title.as_str(),
            self.cmdline.as_str(),
        ];

        // Add child command strings
        for cmd in &self.child_commands {
            strings.push(cmd.as_str());
        }

        strings
    }
}

impl fmt::Display for PaneInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.target())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_target() {
        let pane = PaneInfo {
            session: "dev".to_string(),
            window: 2,
            window_name: "editor".to_string(),
            pane: 3,
            command: "bash".to_string(),
            title: "".to_string(),
            path: "/home/user".to_string(),
            pid: 99999,
            cmdline: "".to_string(),
            child_commands: Vec::new(),
        };
        assert_eq!(pane.target(), "dev:2.3");
    }

    #[test]
    fn test_parse_invalid() {
        assert!(PaneInfo::parse("invalid").is_none());
        assert!(PaneInfo::parse("").is_none());
    }

    #[test]
    fn test_detection_strings() {
        let pane = PaneInfo {
            session: "main".to_string(),
            window: 0,
            window_name: "code".to_string(),
            pane: 0,
            command: "zsh".to_string(),
            title: "~".to_string(),
            path: "/home/user".to_string(),
            pid: 1234,
            cmdline: "-zsh".to_string(),
            child_commands: vec!["claude -c".to_string(), "claude".to_string()],
        };
        let strings = pane.detection_strings();
        assert!(strings.contains(&"zsh"));
        assert!(strings.contains(&"claude -c"));
        assert!(strings.contains(&"claude"));
    }
}

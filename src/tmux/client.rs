use anyhow::{Context, Result};
use std::process::Command;

use super::pane::PaneInfo;

/// Client for interacting with tmux
pub struct TmuxClient {
    /// Number of lines to capture from pane
    capture_lines: u32,
}

impl TmuxClient {
    /// Creates a new TmuxClient with default settings
    pub fn new() -> Self {
        Self { capture_lines: 100 }
    }

    /// Creates a new TmuxClient with custom capture lines
    pub fn with_capture_lines(capture_lines: u32) -> Self {
        Self { capture_lines }
    }

    /// Check if tmux is available and running
    pub fn is_available(&self) -> bool {
        Command::new("tmux")
            .arg("list-sessions")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    /// Lists all panes across all attached sessions
    pub fn list_panes(&self) -> Result<Vec<PaneInfo>> {
        // Use tab separator to handle spaces in titles/paths
        // Include session_attached to filter out detached sessions
        let output = Command::new("tmux")
            .args([
                "list-panes",
                "-a",
                "-F",
                "#{session_attached}\t#{session_name}:#{window_index}.#{pane_index}\t#{window_name}\t#{pane_current_command}\t#{pane_pid}\t#{pane_title}\t#{pane_current_path}",
            ])
            .output()
            .context("Failed to execute tmux list-panes")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("tmux list-panes failed: {}", stderr);
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let panes: Vec<PaneInfo> = stdout
            .lines()
            .filter_map(|line| {
                // First field is session_attached (0 or 1)
                let (attached, rest) = line.split_once('\t')?;
                
                

                // Only include panes from attached sessions
                if attached == "1" {
                    PaneInfo::parse(rest)
                } else {
                    None
                }
            })
            .collect();

        Ok(panes)
    }

    /// Captures the content of a specific pane
    pub fn capture_pane(&self, target: &str) -> Result<String> {
        let start_line = format!("-{}", self.capture_lines);

        let output = Command::new("tmux")
            .args(["capture-pane", "-p", "-t", target, "-S", &start_line])
            .output()
            .context("Failed to execute tmux capture-pane")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("tmux capture-pane failed for {}: {}", target, stderr);
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    /// Sends keys to a specific pane
    pub fn send_keys(&self, target: &str, keys: &str) -> Result<()> {
        let output = Command::new("tmux")
            .args(["send-keys", "-t", target, keys])
            .output()
            .context("Failed to execute tmux send-keys")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("tmux send-keys failed for {}: {}", target, stderr);
        }

        Ok(())
    }

    /// Selects (focuses) a specific pane
    pub fn select_pane(&self, target: &str) -> Result<()> {
        let output = Command::new("tmux")
            .args(["select-pane", "-t", target])
            .output()
            .context("Failed to execute tmux select-pane")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("tmux select-pane failed for {}: {}", target, stderr);
        }

        Ok(())
    }

    /// Selects a specific window
    pub fn select_window(&self, target: &str) -> Result<()> {
        // Extract session:window from full target
        let window_target = if let Some(pos) = target.rfind('.') {
            &target[..pos]
        } else {
            target
        };

        let output = Command::new("tmux")
            .args(["select-window", "-t", window_target])
            .output()
            .context("Failed to execute tmux select-window")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("tmux select-window failed for {}: {}", window_target, stderr);
        }

        Ok(())
    }

    /// Focuses on a pane by selecting its window and pane
    pub fn focus_pane(&self, target: &str) -> Result<()> {
        self.select_window(target)?;
        self.select_pane(target)?;
        Ok(())
    }
}

impl Default for TmuxClient {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = TmuxClient::new();
        assert_eq!(client.capture_lines, 100);

        let custom_client = TmuxClient::with_capture_lines(200);
        assert_eq!(custom_client.capture_lines, 200);
    }
}

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::mpsc;
use tracing::{debug, error, warn};

use crate::agents::{AgentStatus, MonitoredAgent};
use crate::app::{AgentTree, NonAgentPane};
use crate::parsers::ParserRegistry;
use crate::tmux::{refresh_process_cache, TmuxClient};

/// Hysteresis duration - keep "Processing" status for this long after last active detection
const STATUS_HYSTERESIS_MS: u64 = 2000;

/// Update message sent from monitor to UI
#[derive(Debug, Clone)]
pub struct MonitorUpdate {
    pub agents: AgentTree,
    pub all_sessions: Vec<String>,
}

/// Background task that monitors tmux panes for AI agents
pub struct MonitorTask {
    tmux_client: Arc<TmuxClient>,
    parser_registry: Arc<ParserRegistry>,
    tx: mpsc::Sender<MonitorUpdate>,
    poll_interval: Duration,
    /// Track when each agent was last seen as "active" (Processing/AwaitingApproval)
    /// Key: agent target string
    last_active: HashMap<String, Instant>,
}

impl MonitorTask {
    pub fn new(
        tmux_client: Arc<TmuxClient>,
        parser_registry: Arc<ParserRegistry>,
        tx: mpsc::Sender<MonitorUpdate>,
        poll_interval: Duration,
    ) -> Self {
        Self {
            tmux_client,
            parser_registry,
            tx,
            poll_interval,
            last_active: HashMap::new(),
        }
    }

    /// Runs the monitoring loop
    pub async fn run(mut self) {
        loop {
            match self.poll_agents().await {
                Ok((tree, all_sessions)) => {
                    let update = MonitorUpdate {
                        agents: tree,
                        all_sessions,
                    };
                    if self.tx.send(update).await.is_err() {
                        debug!("Monitor channel closed, stopping");
                        break;
                    }
                }
                Err(e) => {
                    warn!("Monitor poll error: {}", e);
                }
            }

            tokio::time::sleep(self.poll_interval).await;
        }
    }

    async fn poll_agents(&mut self) -> anyhow::Result<(AgentTree, Vec<String>)> {
        // Refresh process cache once per poll cycle (much faster than per-pane)
        refresh_process_cache();

        let all_sessions = self.tmux_client.list_sessions().unwrap_or_default();
        let panes = self.tmux_client.list_panes()?;
        let mut tree = AgentTree::new();

        for pane in panes {
            // Try to find a matching parser for the pane (checks command, title, cmdline)
            if let Some(parser) = self.parser_registry.find_parser_for_pane(&pane) {
                let target = pane.target();

                // Capture pane content
                let content = match self.tmux_client.capture_pane(&target) {
                    Ok(c) => c,
                    Err(e) => {
                        error!("Failed to capture pane {}: {}", target, e);
                        continue;
                    }
                };

                // Parse status from content
                let mut status = parser.parse_status(&content);

                // Check pane title for spinner (Claude Code specific)
                // Spinners like ⠐⠇⠋⠙⠸ in title indicate processing
                let title_has_spinner = pane.title.chars().any(|c| {
                    matches!(
                        c,
                        '⠿' | '⠇'
                            | '⠋'
                            | '⠙'
                            | '⠸'
                            | '⠴'
                            | '⠦'
                            | '⠧'
                            | '⠖'
                            | '⠏'
                            | '⠹'
                            | '⠼'
                            | '⠷'
                            | '⠾'
                            | '⠽'
                            | '⠻'
                            | '⠐'
                            | '⠑'
                            | '⠒'
                            | '⠓'
                    )
                });

                // If title has spinner, override to Processing
                if title_has_spinner && matches!(status, AgentStatus::Idle | AgentStatus::Unknown) {
                    status = AgentStatus::Processing {
                        activity: "Working...".to_string(),
                    };
                }

                // Apply hysteresis: if status is now Idle but was recently active, keep as Processing
                let now = Instant::now();
                let is_active = matches!(
                    status,
                    AgentStatus::Processing { .. } | AgentStatus::AwaitingApproval { .. }
                );

                if is_active {
                    // Update last active time
                    self.last_active.insert(target.clone(), now);
                } else if matches!(status, AgentStatus::Idle) {
                    // Check if we were recently active
                    if let Some(last) = self.last_active.get(&target) {
                        if now.duration_since(*last) < Duration::from_millis(STATUS_HYSTERESIS_MS) {
                            // Keep as Processing to avoid flicker
                            status = AgentStatus::Processing {
                                activity: "Working...".to_string(),
                            };
                        }
                    }
                }

                // Parse subagents
                let subagents = parser.parse_subagents(&content);

                // Parse context remaining
                let context_remaining = parser.parse_context_remaining(&content);

                // Create monitored agent
                let mut agent = MonitoredAgent::new(
                    format!("{}-{}", target, pane.pid),
                    target,
                    pane.session.clone(),
                    pane.window,
                    pane.window_name.clone(),
                    pane.pane,
                    pane.path.clone(),
                    parser.agent_type(),
                    pane.pid,
                );
                agent.status = status;
                agent.subagents = subagents;
                agent.last_content = content;
                agent.context_remaining = context_remaining;
                agent.touch(); // Update last_updated

                tree.root_agents.push(agent);
            } else {
                // Non-agent pane
                tree.non_agent_panes.push(NonAgentPane {
                    target: pane.target(),
                    session: pane.session.clone(),
                    window: pane.window,
                    window_name: pane.window_name.clone(),
                    pane: pane.pane,
                    command: pane.command.clone(),
                    path: pane.path.clone(),
                });
            }
        }

        // Sort agents: first by session priority (activity), then by target within each session
        sort_agents_by_activity(&mut tree.root_agents);

        // Sort non-agent panes by session then target
        tree.non_agent_panes
            .sort_by(|a, b| a.session.cmp(&b.session).then(a.target.cmp(&b.target)));

        Ok((tree, all_sessions))
    }
}

/// Compute session priority: lower number = higher priority (sorted first)
fn session_priority(agents: &[&MonitoredAgent]) -> u8 {
    let mut has_awaiting = false;
    let mut has_processing = false;
    let mut has_error = false;

    for agent in agents {
        match &agent.status {
            AgentStatus::AwaitingApproval { .. } => has_awaiting = true,
            AgentStatus::Processing { .. } => has_processing = true,
            AgentStatus::Error { .. } => has_error = true,
            _ => {}
        }
    }

    if has_awaiting {
        0
    } else if has_processing {
        1
    } else if has_error {
        2
    } else {
        3
    }
}

/// Sort agents by session activity priority, then by target within each session
fn sort_agents_by_activity(agents: &mut Vec<MonitoredAgent>) {
    // Group agents by session
    let mut session_agents: BTreeMap<String, Vec<MonitoredAgent>> = BTreeMap::new();
    for agent in agents.drain(..) {
        session_agents
            .entry(agent.session.clone())
            .or_default()
            .push(agent);
    }

    // Sort agents within each session by target (window/pane order)
    for group in session_agents.values_mut() {
        group.sort_by(|a, b| a.target.cmp(&b.target));
    }

    // Compute session priority and sort sessions
    let mut session_order: Vec<(String, u8, Instant)> = session_agents
        .iter()
        .map(|(session, group)| {
            let refs: Vec<&MonitoredAgent> = group.iter().collect();
            let priority = session_priority(&refs);
            let most_recent = group
                .iter()
                .map(|a| a.last_updated)
                .max()
                .unwrap_or_else(Instant::now);
            (session.clone(), priority, most_recent)
        })
        .collect();

    // Sort by priority, then by most recent activity (most recent first for same priority)
    session_order.sort_by(|a, b| a.1.cmp(&b.1).then_with(|| b.2.cmp(&a.2)));

    // Rebuild agents in sorted order
    for (session, _, _) in session_order {
        if let Some(group) = session_agents.remove(&session) {
            agents.extend(group);
        }
    }
}

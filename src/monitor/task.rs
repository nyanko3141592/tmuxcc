use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::mpsc;
use tracing::{debug, error, warn};

use crate::agents::{AgentStatus, MonitoredAgent};
use crate::app::AgentTree;
use crate::git::get_git_branch;
use crate::parsers::ParserRegistry;
use crate::tmux::{refresh_process_cache, TmuxClient};

/// Hysteresis duration - keep "Processing" status for this long after last active detection
const STATUS_HYSTERESIS_MS: u64 = 2000;

/// Update message sent from monitor to UI
#[derive(Debug, Clone)]
pub struct MonitorUpdate {
    pub agents: AgentTree,
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
                Ok(tree) => {
                    let update = MonitorUpdate { agents: tree };
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

    async fn poll_agents(&mut self) -> anyhow::Result<AgentTree> {
        // Refresh process cache once per poll cycle (much faster than per-pane)
        refresh_process_cache();

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

                // Get git branch info
                let git_branch = get_git_branch(&pane.path);

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
                agent.git_branch = git_branch;
                agent.touch(); // Update last_updated

                tree.root_agents.push(agent);
            }
        }

        // Sort agents by target for consistent ordering
        tree.root_agents.sort_by(|a, b| a.target.cmp(&b.target));

        Ok(tree)
    }
}

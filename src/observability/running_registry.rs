use chrono::{Datelike, Local};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunningAgentRecord {
    pub agent_id: String,
    pub started_ts: String,
    pub last_heartbeat_ts: String,
    pub kind: String,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub pid: Option<u32>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct RunningState {
    pub agents: BTreeMap<String, RunningAgentRecord>,
}

/// Maintains an always-updating RUNNING.toml for cross-agent coordination.
///
/// Path:
///   <workspace>/.housaky/logs/days/<ddMMMyyyy>/RUNNING.toml
pub struct RunningRegistry {
    workspace_dir: PathBuf,
    state: Arc<Mutex<RunningState>>,
}

impl RunningRegistry {
    pub fn new(workspace_dir: impl AsRef<Path>) -> Self {
        Self {
            workspace_dir: workspace_dir.as_ref().to_path_buf(),
            state: Arc::new(Mutex::new(RunningState::default())),
        }
    }

    fn day_folder_name() -> String {
        let now = Local::now();
        // Must match FlightJournal day folder.
        format!(
            "{:02}{}{}",
            now.day(),
            match now.month() {
                1 => "Jan",
                2 => "Feb",
                3 => "Mar",
                4 => "Apr",
                5 => "May",
                6 => "Jun",
                7 => "Jul",
                8 => "Aug",
                9 => "Sep",
                10 => "Oct",
                11 => "Nov",
                12 => "Dec",
                _ => "???",
            },
            now.year()
        )
    }

    fn now_ts() -> String {
        Local::now().to_rfc3339()
    }

    fn running_path(&self) -> anyhow::Result<PathBuf> {
        let day = Self::day_folder_name();
        let dir = self
            .workspace_dir
            .join(".housaky")
            .join("logs")
            .join("days")
            .join(day);
        fs::create_dir_all(&dir)?;
        Ok(dir.join("RUNNING.toml"))
    }

    async fn flush_locked(&self, state: &RunningState) -> anyhow::Result<()> {
        let path = self.running_path()?;
        let tmp_path = path.with_extension("toml.tmp");
        let toml = toml::to_string_pretty(state)?;
        fs::write(&tmp_path, toml)?;
        fs::rename(&tmp_path, &path)?;
        Ok(())
    }

    pub async fn register_start(
        &self,
        agent_id: &str,
        kind: &str,
        provider: Option<&str>,
        model: Option<&str>,
    ) -> anyhow::Result<()> {
        let mut state = self.state.lock().await;
        let ts = Self::now_ts();
        state.agents.insert(
            agent_id.to_string(),
            RunningAgentRecord {
                agent_id: agent_id.to_string(),
                started_ts: ts.clone(),
                last_heartbeat_ts: ts,
                kind: kind.to_string(),
                provider: provider.map(|s| s.to_string()),
                model: model.map(|s| s.to_string()),
                pid: Some(std::process::id()),
            },
        );
        self.flush_locked(&state).await
    }

    pub async fn heartbeat(&self, agent_id: &str) -> anyhow::Result<()> {
        let mut state = self.state.lock().await;
        if let Some(rec) = state.agents.get_mut(agent_id) {
            rec.last_heartbeat_ts = Self::now_ts();
            self.flush_locked(&state).await?;
        }
        Ok(())
    }

    pub async fn register_stop(&self, agent_id: &str) -> anyhow::Result<()> {
        let mut state = self.state.lock().await;
        state.agents.remove(agent_id);
        self.flush_locked(&state).await
    }

    /// Read the current RUNNING.toml from disk (best-effort).
    pub fn read_current(&self) -> anyhow::Result<RunningState> {
        let path = self.running_path()?;
        if !path.exists() {
            return Ok(RunningState::default());
        }
        let content = fs::read_to_string(path)?;
        Ok(toml::from_str(&content).unwrap_or_default())
    }
}

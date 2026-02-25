use chrono::{Datelike, Local};
use serde::Serialize;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

/// Day-partitioned, append-only TOML journal for cross-agent review/grep.
///
/// Directory layout:
///   <workspace>/.housaky/logs/days/<ddMMMyyyy>/<agent_id>/events.toml
///
/// Example:
///   .housaky/logs/days/24Feb2026/agent/170...-pid1234/events.toml
///
/// Notes:
/// - This is intentionally line-oriented and append-only.
/// - Each event is written as a single TOML table, preceded by `[[event]]`.
/// - For grep, common fields are stable: `ts`, `agent_id`, `kind`.
#[derive(Debug)]
pub struct FlightJournal {
    workspace_dir: PathBuf,
    agent_id: String,
    seq: AtomicU64,
    // Serialize writes to keep the file append safe inside a single process.
    write_lock: Mutex<()>,
}

#[derive(Debug, Serialize)]
struct JournalEvent<'a> {
    ts: String,
    day: String,
    agent_id: &'a str,
    seq: u64,
    kind: &'a str,

    // generic payload
    provider: Option<&'a str>,
    model: Option<&'a str>,
    route: Option<&'a str>,
    resolved_model: Option<&'a str>,
    tool: Option<&'a str>,
    success: Option<bool>,
    duration_ms: Option<u64>,
    error: Option<&'a str>,
    message: Option<&'a str>,
}

impl FlightJournal {
    pub fn new(workspace_dir: impl AsRef<Path>, agent_id: impl Into<String>) -> Self {
        Self {
            workspace_dir: workspace_dir.as_ref().to_path_buf(),
            agent_id: agent_id.into(),
            seq: AtomicU64::new(0),
            write_lock: Mutex::new(()),
        }
    }

    pub fn agent_id(&self) -> &str {
        &self.agent_id
    }

    fn day_folder_name() -> String {
        // Requested format example: 24feb2026 / 25feb2026.
        // We'll keep it readable but stable: 24Feb2026.
        let now = Local::now();
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

    fn events_path(&self) -> anyhow::Result<PathBuf> {
        let day = Self::day_folder_name();
        let dir = self
            .workspace_dir
            .join(".housaky")
            .join("logs")
            .join("days")
            .join(&day)
            .join(&self.agent_id);
        fs::create_dir_all(&dir)?;
        Ok(dir.join("events.toml"))
    }

    pub fn append_event(
        &self,
        kind: &str,
        provider: Option<&str>,
        model: Option<&str>,
        route: Option<&str>,
        resolved_model: Option<&str>,
        tool: Option<&str>,
        success: Option<bool>,
        duration_ms: Option<u64>,
        error: Option<&str>,
        message: Option<&str>,
    ) -> anyhow::Result<()> {
        let _guard = self.write_lock.lock().expect("flight journal lock");
        let path = self.events_path()?;

        let seq = self.seq.fetch_add(1, Ordering::Relaxed) + 1;
        let day = Self::day_folder_name();
        let ts = Self::now_ts();

        let evt = JournalEvent {
            ts,
            day,
            agent_id: &self.agent_id,
            seq,
            kind,
            provider,
            model,
            route,
            resolved_model,
            tool,
            success,
            duration_ms,
            error,
            message,
        };

        let toml = toml::to_string(&evt)?;

        let mut f = OpenOptions::new().create(true).append(true).open(&path)?;
        // record separator for TOML arrays-of-tables
        writeln!(f, "[[event]]")?;
        // toml::to_string already ends with \n
        f.write_all(toml.as_bytes())?;
        writeln!(f)?;
        Ok(())
    }
}

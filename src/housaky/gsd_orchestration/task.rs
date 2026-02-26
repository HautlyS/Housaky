use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum GSDTaskStatus {
    Pending,
    Ready,
    InProgress,
    Completed,
    Failed,
    Verified,
    Skipped,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskVerification {
    pub command: String,
    pub expected_result: String,
    pub actual_result: Option<String>,
    pub passed: Option<bool>,
    pub verified_at: Option<DateTime<Utc>>,
    pub notes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GSDTask {
    pub id: String,
    pub name: String,
    pub description: String,
    pub phase_id: String,
    pub wave: u32,
    pub files: Vec<String>,
    pub action: String,
    pub verify: String,
    pub done_criteria: String,
    pub status: GSDTaskStatus,
    pub priority: TaskPriority,
    pub dependencies: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub verification: Option<TaskVerification>,
    pub context: HashMap<String, String>,
    pub artifacts: Vec<Artifact>,
    pub commit_sha: Option<String>,
    pub error_log: Option<String>,
    pub attempts: u32,
    pub max_attempts: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TaskPriority {
    Critical,
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Artifact {
    pub name: String,
    pub artifact_type: ArtifactType,
    pub path: String,
    pub content: Option<String>,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ArtifactType {
    File,
    Directory,
    Config,
    Documentation,
    Test,
    Binary,
}

impl GSDTask {
    pub fn new(name: String, phase_id: String) -> Self {
        Self {
            id: format!("task_{}", uuid::Uuid::new_v4()),
            name,
            description: String::new(),
            phase_id,
            wave: 1,
            files: Vec::new(),
            action: String::new(),
            verify: String::new(),
            done_criteria: String::new(),
            status: GSDTaskStatus::Pending,
            priority: TaskPriority::Medium,
            dependencies: Vec::new(),
            created_at: Utc::now(),
            started_at: None,
            completed_at: None,
            verification: None,
            context: HashMap::new(),
            artifacts: Vec::new(),
            commit_sha: None,
            error_log: None,
            attempts: 0,
            max_attempts: 3,
        }
    }

    pub fn with_action(mut self, action: String) -> Self {
        self.action = action;
        self
    }

    pub fn with_files(mut self, files: Vec<String>) -> Self {
        self.files = files;
        self
    }

    pub fn with_verification(mut self, verify: String) -> Self {
        self.verify = verify;
        self
    }

    pub fn with_done_criteria(mut self, criteria: String) -> Self {
        self.done_criteria = criteria;
        self
    }

    pub fn with_dependencies(mut self, deps: Vec<String>) -> Self {
        self.dependencies = deps;
        self
    }

    pub fn with_wave(mut self, wave: u32) -> Self {
        self.wave = wave;
        self
    }

    pub fn with_priority(mut self, priority: TaskPriority) -> Self {
        self.priority = priority;
        self
    }

    pub fn start(&mut self) {
        self.status = GSDTaskStatus::InProgress;
        self.started_at = Some(Utc::now());
        self.attempts += 1;
    }

    pub fn complete(&mut self) {
        self.status = GSDTaskStatus::Completed;
        self.completed_at = Some(Utc::now());
    }

    pub fn fail(&mut self, error: String) {
        self.status = GSDTaskStatus::Failed;
        self.error_log = Some(error);

        if self.attempts >= self.max_attempts {
            self.status = GSDTaskStatus::Failed;
        } else {
            self.status = GSDTaskStatus::Ready;
        }
    }

    pub fn verify(&mut self, command: String, expected: String, actual: String) -> bool {
        let passed = actual.contains(&expected) || expected.is_empty();

        self.verification = Some(TaskVerification {
            command,
            expected_result: expected,
            actual_result: Some(actual),
            passed: Some(passed),
            verified_at: Some(Utc::now()),
            notes: String::new(),
        });

        if passed {
            self.status = GSDTaskStatus::Verified;
        }

        passed
    }

    pub fn add_artifact(&mut self, artifact: Artifact) {
        self.artifacts.push(artifact);
    }

    pub fn to_xml(&self) -> String {
        let files_str = if self.files.is_empty() {
            String::new()
        } else {
            format!(
                "\n  <files>{}</files>",
                self.files
                    .iter()
                    .map(|f| format!("\n    <file>{}</file>", f))
                    .collect::<Vec<_>>()
                    .join("")
            )
        };

        let deps_str = if self.dependencies.is_empty() {
            String::new()
        } else {
            format!(
                "\n  <dependencies>{}</dependencies>",
                self.dependencies
                    .iter()
                    .map(|d| format!("\n    <dep>{}</dep>", d))
                    .collect::<Vec<_>>()
                    .join("")
            )
        };

        format!(
            r#"<task type="auto" id="{}">
  <name>{}</name>{}
  <action>
    {}
  </action>
  <verify>{}</verify>
  <done>{}</done>{}
</task>"#,
            self.id, self.name, files_str, self.action, self.verify, self.done_criteria, deps_str
        )
    }

    pub fn is_ready(&self, completed_tasks: &[String]) -> bool {
        if self.status != GSDTaskStatus::Pending && self.status != GSDTaskStatus::Ready {
            return false;
        }

        self.dependencies
            .iter()
            .all(|dep| completed_tasks.contains(dep))
    }

    pub fn duration_ms(&self) -> Option<i64> {
        match (self.started_at, self.completed_at) {
            (Some(start), Some(end)) => Some((end - start).num_milliseconds()),
            _ => None,
        }
    }
}

impl Default for GSDTask {
    fn default() -> Self {
        Self::new(String::new(), String::new())
    }
}

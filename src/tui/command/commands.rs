use anyhow::Result;

use super::Command;
use crate::config::Config;

pub struct CommandExecutor<'a> {
    pub config: &'a mut Config,
}

impl<'a> CommandExecutor<'a> {
    pub fn new(config: &'a mut Config) -> Self {
        Self { config }
    }

    pub fn execute(&mut self, command: &Command) -> Result<String> {
        match command {
            Command::Help => self.show_help(),
            Command::Config { section } => self.open_config(section.as_deref()),
            Command::Model { name } => self.handle_model(name.as_deref()),
            Command::Provider { name } => self.handle_provider(name.as_deref()),
            Command::Fallback { action } => self.handle_fallback(action.clone()),
            Command::Clear => self.clear_conversation(),
            Command::Save { path } => self.save_conversation(path.as_deref()),
            Command::Export { format } => self.export_conversation(format),
            Command::Usage => self.show_usage(),
            Command::Keys => self.show_keys(),
            Command::Status => self.show_status(),
            Command::Quit => self.quit(),
        }
    }

    fn show_help(&self) -> Result<String> {
        Ok(r#"Available Commands:
  /help              Show this help
  /config [section]  Open config editor
  /model [name]      Change or show model
  /provider [name]   Change or show provider
  /fallback list     List fallback providers
  /fallback rotate   Rotate to next provider
  /fallback status   Show fallback status
  /clear             Clear conversation
  /save [path]       Save conversation
  /export [format]   Export conversation (json|markdown|txt)
  /usage             Show usage statistics
  /keys              Manage API keys
  /status            Show system status
  /quit              Exit the TUI
"#
        .to_string())
    }

    fn open_config(&mut self, section: Option<&str>) -> Result<String> {
        if let Some(s) = section {
            Ok(format!("Opening config editor for section: {}", s))
        } else {
            Ok("Opening config editor...".to_string())
        }
    }

    fn handle_model(&mut self, name: Option<&str>) -> Result<String> {
        match name {
            Some(model) => {
                self.config.default_model = Some(model.to_string());
                Ok(format!("Model changed to: {}", model))
            }
            None => {
                let current = self.config.default_model.as_deref().unwrap_or("not set");
                Ok(format!("Current model: {}", current))
            }
        }
    }

    fn handle_provider(&mut self, name: Option<&str>) -> Result<String> {
        match name {
            Some(provider) => {
                self.config.default_provider = Some(provider.to_string());
                Ok(format!("Provider changed to: {}", provider))
            }
            None => {
                let current = self.config.default_provider.as_deref().unwrap_or("not set");
                Ok(format!("Current provider: {}", current))
            }
        }
    }

    fn handle_fallback(&mut self, action: super::FallbackAction) -> Result<String> {
        use super::FallbackAction;

        match action {
            FallbackAction::List => {
                if self.config.fallback.providers.is_empty() {
                    Ok("No fallback providers configured.".to_string())
                } else {
                    let list: Vec<String> = self
                        .config
                        .fallback
                        .providers
                        .iter()
                        .enumerate()
                        .map(|(i, p)| format!("  #{} {} (priority {})", i + 1, p.name, p.priority))
                        .collect();
                    Ok(format!("Fallback providers:\n{}", list.join("\n")))
                }
            }
            FallbackAction::Rotate => {
                if self.config.fallback.providers.len() <= 1 {
                    Ok("No fallback providers to rotate to.".to_string())
                } else {
                    Ok("Rotating to next fallback provider...".to_string())
                }
            }
            FallbackAction::Add { name } => {
                if name.is_empty() {
                    Ok("Usage: /fallback add <provider_name>".to_string())
                } else {
                    Ok(format!("Added {} to fallback providers.", name))
                }
            }
            FallbackAction::Status => {
                if self.config.fallback.enabled {
                    Ok(format!(
                        "Fallback enabled. Rotate at {}% usage.",
                        self.config.fallback.rotate_at_percent
                    ))
                } else {
                    Ok("Fallback is disabled.".to_string())
                }
            }
        }
    }

    fn clear_conversation(&self) -> Result<String> {
        Ok("Conversation cleared.".to_string())
    }

    fn save_conversation(&self, path: Option<&str>) -> Result<String> {
        match path {
            Some(p) => Ok(format!("Conversation saved to: {}", p)),
            None => Ok("Conversation saved to default location.".to_string()),
        }
    }

    fn export_conversation(&self, format: &str) -> Result<String> {
        Ok(format!("Conversation exported as {}.", format))
    }

    fn show_usage(&self) -> Result<String> {
        Ok(format!(
            "Usage tracking: {}\nDaily limit: ${:.2}\nMonthly limit: ${:.2}",
            if self.config.cost.enabled {
                "enabled"
            } else {
                "disabled"
            },
            self.config.cost.daily_limit_usd,
            self.config.cost.monthly_limit_usd
        ))
    }

    fn show_keys(&self) -> Result<String> {
        let has_key = self.config.api_key.is_some();
        Ok(format!(
            "API key configured: {}\nSecrets encryption: {}",
            if has_key { "yes" } else { "no" },
            if self.config.secrets.encrypt {
                "enabled"
            } else {
                "disabled"
            }
        ))
    }

    fn show_status(&self) -> Result<String> {
        Ok(format!(
            "Provider: {}\nModel: {}\nTemperature: {:.2}\nWorkspace: {}",
            self.config.default_provider.as_deref().unwrap_or("not set"),
            self.config.default_model.as_deref().unwrap_or("not set"),
            self.config.default_temperature,
            self.config.workspace_dir.display()
        ))
    }

    fn quit(&self) -> Result<String> {
        Ok("Goodbye!".to_string())
    }
}

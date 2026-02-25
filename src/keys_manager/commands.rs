use crate::config::Config;
use crate::keys_manager::manager::{KeysManager, ProviderPriority};
use anyhow::Result;
use clap::{Args, Subcommand};
use std::collections::HashMap;

#[derive(Debug, Subcommand, Clone)]
pub enum KeysManagerCommands {
    /// List providers/keys in keys.json store.
    List,

    /// Launch interactive TUI.
    Tui,

    /// Add a provider based on a built-in template.
    AddProvider(KeysAddProviderArgs),

    /// Add a custom provider.
    AddCustomProvider(KeysAddCustomProviderArgs),

    /// Add a key to an existing provider.
    AddKey(KeysAddKeyArgs),

    /// Remove a provider.
    RemoveProvider(KeysRemoveProviderArgs),

    /// Set provider priority (primary/secondary/tertiary/quaternary/disabled).
    SetPriority(KeysSetPriorityArgs),

    /// Set provider default model.
    SetDefaultModel(KeysSetDefaultModelArgs),

    /// Sync the legacy config.toml api_key/default_provider/default_model into keys.json
    /// without overwriting an existing api_key in config.toml.
    SyncFromConfig,

    /// Print a config.toml snippet for task routing (per-subject routes).
    PrintRoutesExample,
}

#[derive(Debug, Args, Clone)]
pub struct KeysAddProviderArgs {
    pub name: String,
    #[arg(long, default_value = "openrouter")]
    pub template: String,
    #[arg(long = "key", short = 'k')]
    pub keys: Vec<String>,
    #[arg(long, default_value = "primary")]
    pub priority: String,
}

#[derive(Debug, Args, Clone)]
pub struct KeysAddCustomProviderArgs {
    pub name: String,
    #[arg(long)]
    pub base_url: String,
    #[arg(long, default_value = "bearer")]
    pub auth_method: String,
    #[arg(long = "key", short = 'k')]
    pub keys: Vec<String>,
    #[arg(long = "model", short = 'm')]
    pub models: Vec<String>,
}

#[derive(Debug, Args, Clone)]
pub struct KeysAddKeyArgs {
    pub provider: String,
    pub key: String,
}

#[derive(Debug, Args, Clone)]
pub struct KeysRemoveProviderArgs {
    pub name: String,
}

#[derive(Debug, Args, Clone)]
pub struct KeysSetPriorityArgs {
    pub provider: String,
    pub priority: String,
}

#[derive(Debug, Args, Clone)]
pub struct KeysSetDefaultModelArgs {
    pub provider: String,
    pub model: String,
}

pub async fn handle_keys_manager_command(
    config: &mut Config,
    manager: &KeysManager,
    cmd: KeysManagerCommands,
) -> Result<()> {
    match cmd {
        KeysManagerCommands::List => {
            let _ = manager.load().await;
            let store = manager.store.read().await;
            if store.providers.is_empty() {
                println!("No providers configured in keys.json.");
                return Ok(());
            }
            for (name, p) in &store.providers {
                let enabled = p.keys.iter().filter(|k| k.enabled).count();
                println!(
                    "[{name}] keys={} enabled={} priority={} default_model={}",
                    p.keys.len(),
                    enabled,
                    p.priority,
                    p.default_model.as_deref().unwrap_or("<none>"),
                );
            }
            Ok(())
        }
        KeysManagerCommands::Tui => crate::keys_manager::tui::run_keys_tui(manager).await,
        KeysManagerCommands::AddProvider(args) => {
            let priority = ProviderPriority::from_str(&args.priority)
                .ok_or_else(|| anyhow::anyhow!("Invalid priority: {}", args.priority))?;
            manager
                .add_provider_with_template(&args.name, &args.template, args.keys, priority)
                .await?;
            println!("Added provider {}", args.name);
            Ok(())
        }
        KeysManagerCommands::AddCustomProvider(args) => {
            manager
                .add_custom_provider(
                    &args.name,
                    &args.base_url,
                    &args.auth_method,
                    args.keys,
                    args.models,
                    ProviderPriority::Primary,
                    HashMap::new(),
                )
                .await?;
            println!("Added custom provider {}", args.name);
            Ok(())
        }
        KeysManagerCommands::AddKey(args) => {
            manager.add_key(&args.provider, args.key, None).await?;
            println!("Added key to {}", args.provider);
            Ok(())
        }
        KeysManagerCommands::RemoveProvider(args) => {
            manager.remove_provider(&args.name).await?;
            println!("Removed provider {}", args.name);
            Ok(())
        }
        KeysManagerCommands::SetPriority(args) => {
            let priority = ProviderPriority::from_str(&args.priority)
                .ok_or_else(|| anyhow::anyhow!("Invalid priority: {}", args.priority))?;
            manager.set_provider_priority(&args.provider, priority).await?;
            println!("Updated priority");
            Ok(())
        }
        KeysManagerCommands::SetDefaultModel(args) => {
            manager.set_default_model(&args.provider, &args.model).await?;
            println!("Updated default model");
            Ok(())
        }
        KeysManagerCommands::SyncFromConfig => {
            let _ = manager.load().await;

            // Preserve existing config.api_key by NOT overwriting it.
            // If config.api_key exists and keys store has no providers yet, create one.
            let api_key = config.api_key.clone().unwrap_or_default();
            let provider = config
                .default_provider
                .clone()
                .unwrap_or_else(|| "openrouter".to_string());

            if !api_key.trim().is_empty() {
                // Only inject if provider does not exist or has no keys.
                let store = manager.store.write().await;
                let needs_key = store
                    .providers
                    .get(&provider)
                    .map(|p| p.keys.is_empty())
                    .unwrap_or(true);
                drop(store);

                if needs_key {
                    // Use template if known.
                    let template = if manager.provider_templates.contains_key(provider.as_str()) {
                        provider.as_str()
                    } else {
                        "custom"
                    };
                    let _ = manager
                        .add_provider_with_template(
                            &provider,
                            template,
                            vec![api_key.clone()],
                            ProviderPriority::Primary,
                        )
                        .await;
                }
            }

            // Default model, if present.
            if let Some(model) = config.default_model.clone() {
                let _ = manager.set_default_model(&provider, &model).await;
            }

            manager.save().await?;
            println!("Synced from config.toml into keys.json (did not modify config.toml api_key)." );
            Ok(())
        }
        KeysManagerCommands::PrintRoutesExample => {
            println!(
                "# Example per-subject routing (use with RouterProvider: hint:<subject>)\n\
[[model_routes]]\nhint = \"code\"\nprovider = \"openrouter\"\nmodel = \"anthropic/claude-sonnet-4-20250514\"\n\n\
[[model_routes]]\nhint = \"review\"\nprovider = \"anthropic\"\nmodel = \"claude-opus-4-20250514\"\n\n\
[[model_routes]]\nhint = \"debug\"\nprovider = \"openai\"\nmodel = \"gpt-4o\"\n"
            );
            Ok(())
        }
    }
}

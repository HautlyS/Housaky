use crate::keys_manager::manager::{KeysManager, ProviderPriority};
use anyhow::Result;
use std::collections::HashMap;

/// Minimal interactive TUI for managing providers/keys/models.
///
/// NOTE: This is intentionally lightweight (stdin prompts) to avoid coupling to the
/// main chat TUI (ratatui). It can be wired into a richer ratatui screen later.
///
/// Actions supported:
/// - list providers
/// - add provider from template
/// - add custom provider
/// - add key
/// - set default model
/// - set provider priority
///
/// The TUI reads/writes the KeysManager store (keys.json) and does not touch
/// ~/.housaky/config.toml api_key unless explicitly requested via commands.
pub async fn run_keys_tui(manager: &KeysManager) -> Result<()> {
    let _ = manager.load().await;

    loop {
        println!("\n== Housaky Keys Manager ==");
        println!("1) List providers");
        println!("2) Add provider (template)" );
        println!("3) Add provider (custom)" );
        println!("4) Add key to provider" );
        println!("5) Set provider priority" );
        println!("6) Set provider default model" );
        println!("7) Save" );
        println!("0) Exit" );
        print!("> ");
        std::io::Write::flush(&mut std::io::stdout())?;

        let mut choice = String::new();
        std::io::stdin().read_line(&mut choice)?;
        match choice.trim() {
            "1" => list_providers(manager).await?,
            "2" => add_provider_template(manager).await?,
            "3" => add_provider_custom(manager).await?,
            "4" => add_key(manager).await?,
            "5" => set_priority(manager).await?,
            "6" => set_default_model(manager).await?,
            "7" => {
                manager.save().await?;
                println!("Saved.");
            }
            "0" => break,
            _ => println!("Unknown option"),
        }
    }

    Ok(())
}

async fn list_providers(manager: &KeysManager) -> Result<()> {
    let store = manager.store.read().await;
    if store.providers.is_empty() {
        println!("No providers configured.");
        return Ok(());
    }

    println!("Providers:");
    for (name, p) in &store.providers {
        let enabled = p.keys.iter().filter(|k| k.enabled).count();
        println!(
            "- {name}: {} keys ({} enabled) priority={} default_model={}",
            p.keys.len(),
            enabled,
            p.priority,
            p.default_model.as_deref().unwrap_or("<none>"),
        );
        if !p.models.is_empty() {
            println!("    models: {}", p.models.join(", "));
        }
    }
    Ok(())
}

async fn add_provider_template(manager: &KeysManager) -> Result<()> {
    println!("Template names: {}", manager.get_template_names().join(", "));

    let template = prompt("Template")?;
    let name = prompt("Provider name")?;
    let key = prompt("API key (leave empty to skip)")?;
    let keys = if key.trim().is_empty() { vec![] } else { vec![key] };

    // Default to Primary if new provider.
    manager
        .add_provider_with_template(&name, &template, keys, ProviderPriority::Primary)
        .await?;

    println!("Added provider '{name}' from template '{template}'.");
    Ok(())
}

async fn add_provider_custom(manager: &KeysManager) -> Result<()> {
    let name = prompt("Provider name")?;
    let base_url = prompt("Base URL")?;
    let auth_method = prompt("Auth method (bearer/x-api-key/query/custom)")?;
    let key = prompt("API key (leave empty to skip)")?;

    let models_raw = prompt("Models (comma-separated)")?;
    let models: Vec<String> = models_raw
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect();

    let keys = if key.trim().is_empty() { vec![] } else { vec![key] };

    manager
        .add_custom_provider(
            &name,
            &base_url,
            &auth_method,
            keys,
            models,
            ProviderPriority::Primary,
            HashMap::new(),
        )
        .await?;

    println!("Added custom provider '{name}'.");
    Ok(())
}

async fn add_key(manager: &KeysManager) -> Result<()> {
    let provider = prompt("Provider")?;
    let key = prompt("API key")?;
    manager.add_key(&provider, key, None).await?;
    println!("Key added.");
    Ok(())
}

async fn set_priority(manager: &KeysManager) -> Result<()> {
    let provider = prompt("Provider")?;
    let priority = prompt("Priority (primary/secondary/tertiary/quaternary/disabled)")?;
    let Some(p) = ProviderPriority::from_str(&priority) else {
        anyhow::bail!("Invalid priority");
    };

    manager.set_provider_priority(&provider, p).await?;
    println!("Updated.");
    Ok(())
}

async fn set_default_model(manager: &KeysManager) -> Result<()> {
    let provider = prompt("Provider")?;
    let model = prompt("Default model")?;
    manager.set_default_model(&provider, &model).await?;
    println!("Updated.");
    Ok(())
}

fn prompt(label: &str) -> Result<String> {
    print!("{label}: ");
    std::io::Write::flush(&mut std::io::stdout())?;
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    Ok(input.trim().to_string())
}

use super::{CommandSuggestion, COMMAND_HELP};

pub fn get_suggestions(input: &str) -> Vec<CommandSuggestion> {
    let trimmed = input.trim();
    if !trimmed.starts_with('/') {
        return Vec::new();
    }

    let query = &trimmed[1..].to_lowercase();
    let parts: Vec<&str> = query.split_whitespace().collect();

    if parts.is_empty() {
        return get_all_commands();
    }

    if parts.len() == 1 {
        filter_commands(parts[0])
    } else {
        get_arg_suggestions(parts[0], &parts[1..])
    }
}

fn get_all_commands() -> Vec<CommandSuggestion> {
    COMMAND_HELP
        .iter()
        .map(|(cmd, desc, hint)| CommandSuggestion {
            command: cmd.to_string(),
            description: desc.to_string(),
            args_hint: hint.to_string(),
        })
        .collect()
}

fn filter_commands(prefix: &str) -> Vec<CommandSuggestion> {
    COMMAND_HELP
        .iter()
        .filter(|(cmd, _, _)| {
            let cmd_name = cmd.trim_start_matches('/');
            cmd_name.starts_with(prefix) || cmd_name.contains(prefix)
        })
        .map(|(cmd, desc, hint)| CommandSuggestion {
            command: cmd.to_string(),
            description: desc.to_string(),
            args_hint: hint.to_string(),
        })
        .collect()
}

fn get_arg_suggestions(command: &str, _args: &[&str]) -> Vec<CommandSuggestion> {
    match command {
        "config" | "cfg" => get_config_sections(),
        "model" | "m" => get_models(),
        "provider" | "p" => get_providers(),
        "fallback" | "fb" => get_fallback_actions(),
        "export" | "e" => get_export_formats(),
        _ => Vec::new(),
    }
}

fn get_config_sections() -> Vec<CommandSuggestion> {
    vec![
        CommandSuggestion {
            command: "/config agent".to_string(),
            description: "Agent settings".to_string(),
            args_hint: String::new(),
        },
        CommandSuggestion {
            command: "/config tools".to_string(),
            description: "Tool settings".to_string(),
            args_hint: String::new(),
        },
        CommandSuggestion {
            command: "/config channels".to_string(),
            description: "Channel settings".to_string(),
            args_hint: String::new(),
        },
        CommandSuggestion {
            command: "/config gateway".to_string(),
            description: "Gateway settings".to_string(),
            args_hint: String::new(),
        },
        CommandSuggestion {
            command: "/config memory".to_string(),
            description: "Memory settings".to_string(),
            args_hint: String::new(),
        },
        CommandSuggestion {
            command: "/config providers".to_string(),
            description: "Provider settings".to_string(),
            args_hint: String::new(),
        },
        CommandSuggestion {
            command: "/config fallback".to_string(),
            description: "Fallback settings".to_string(),
            args_hint: String::new(),
        },
        CommandSuggestion {
            command: "/config security".to_string(),
            description: "Security settings".to_string(),
            args_hint: String::new(),
        },
        CommandSuggestion {
            command: "/config cost".to_string(),
            description: "Cost settings".to_string(),
            args_hint: String::new(),
        },
    ]
}

fn get_models() -> Vec<CommandSuggestion> {
    vec![
        CommandSuggestion {
            command: "/model anthropic/claude-sonnet-4".to_string(),
            description: "Claude Sonnet 4 (recommended)".to_string(),
            args_hint: String::new(),
        },
        CommandSuggestion {
            command: "/model anthropic/claude-opus-4".to_string(),
            description: "Claude Opus 4 (most capable)".to_string(),
            args_hint: String::new(),
        },
        CommandSuggestion {
            command: "/model openai/gpt-4o".to_string(),
            description: "GPT-4o".to_string(),
            args_hint: String::new(),
        },
        CommandSuggestion {
            command: "/model openai/gpt-4o-mini".to_string(),
            description: "GPT-4o Mini (fast, cheap)".to_string(),
            args_hint: String::new(),
        },
        CommandSuggestion {
            command: "/model google/gemini-2.0-flash".to_string(),
            description: "Gemini 2.0 Flash".to_string(),
            args_hint: String::new(),
        },
        CommandSuggestion {
            command: "/model google/gemini-1.5-pro".to_string(),
            description: "Gemini 1.5 Pro".to_string(),
            args_hint: String::new(),
        },
    ]
}

fn get_providers() -> Vec<CommandSuggestion> {
    vec![
        CommandSuggestion {
            command: "/provider openrouter".to_string(),
            description: "OpenRouter (multi-model)".to_string(),
            args_hint: String::new(),
        },
        CommandSuggestion {
            command: "/provider anthropic".to_string(),
            description: "Anthropic (Claude)".to_string(),
            args_hint: String::new(),
        },
        CommandSuggestion {
            command: "/provider openai".to_string(),
            description: "OpenAI (GPT)".to_string(),
            args_hint: String::new(),
        },
        CommandSuggestion {
            command: "/provider gemini".to_string(),
            description: "Google (Gemini)".to_string(),
            args_hint: String::new(),
        },
        CommandSuggestion {
            command: "/provider ollama".to_string(),
            description: "Ollama (local)".to_string(),
            args_hint: String::new(),
        },
    ]
}

fn get_fallback_actions() -> Vec<CommandSuggestion> {
    vec![
        CommandSuggestion {
            command: "/fallback list".to_string(),
            description: "List fallback providers".to_string(),
            args_hint: String::new(),
        },
        CommandSuggestion {
            command: "/fallback rotate".to_string(),
            description: "Rotate to next provider".to_string(),
            args_hint: String::new(),
        },
        CommandSuggestion {
            command: "/fallback status".to_string(),
            description: "Show fallback status".to_string(),
            args_hint: String::new(),
        },
        CommandSuggestion {
            command: "/fallback add <name>".to_string(),
            description: "Add a fallback provider".to_string(),
            args_hint: String::new(),
        },
    ]
}

fn get_export_formats() -> Vec<CommandSuggestion> {
    vec![
        CommandSuggestion {
            command: "/export markdown".to_string(),
            description: "Markdown format".to_string(),
            args_hint: String::new(),
        },
        CommandSuggestion {
            command: "/export json".to_string(),
            description: "JSON format".to_string(),
            args_hint: String::new(),
        },
        CommandSuggestion {
            command: "/export txt".to_string(),
            description: "Plain text".to_string(),
            args_hint: String::new(),
        },
    ]
}

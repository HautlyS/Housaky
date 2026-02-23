use super::{Command, FallbackAction};

pub fn parse(input: &str) -> Option<Command> {
    let trimmed = input.trim();
    if !trimmed.starts_with('/') {
        return None;
    }

    let parts: Vec<&str> = trimmed[1..].split_whitespace().collect();
    if parts.is_empty() {
        return None;
    }

    let command = parts[0].to_lowercase();
    let args = &parts[1..];

    match command.as_str() {
        "help" | "h" | "?" => Some(Command::Help),
        "config" | "cfg" => Some(Command::Config {
            section: args.first().map(|s| s.to_string()),
        }),
        "model" | "m" => Some(Command::Model {
            name: args.first().map(|s| s.to_string()),
        }),
        "provider" | "p" => Some(Command::Provider {
            name: args.first().map(|s| s.to_string()),
        }),
        "fallback" | "fb" => parse_fallback(args),
        "clear" | "c" => Some(Command::Clear),
        "save" | "s" => Some(Command::Save {
            path: args.first().map(|s| s.to_string()),
        }),
        "export" | "e" => Some(Command::Export {
            format: args.first().unwrap_or(&"markdown").to_string(),
        }),
        "usage" | "u" => Some(Command::Usage),
        "keys" | "k" => Some(Command::Keys),
        "status" => Some(Command::Status),
        "quit" | "q" | "exit" => Some(Command::Quit),
        _ => None,
    }
}

fn parse_fallback(args: &[&str]) -> Option<Command> {
    let action = args.first()?;

    match *action {
        "list" | "ls" => Some(Command::Fallback {
            action: FallbackAction::List,
        }),
        "rotate" | "next" => Some(Command::Fallback {
            action: FallbackAction::Rotate,
        }),
        "add" => Some(Command::Fallback {
            action: FallbackAction::Add {
                name: args.get(1).unwrap_or(&"").to_string(),
            },
        }),
        "status" | "st" => Some(Command::Fallback {
            action: FallbackAction::Status,
        }),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_help() {
        assert_eq!(parse("/help"), Some(Command::Help));
        assert_eq!(parse("/h"), Some(Command::Help));
        assert_eq!(parse("/?"), Some(Command::Help));
    }

    #[test]
    fn parse_model() {
        assert_eq!(
            parse("/model claude-3-opus"),
            Some(Command::Model {
                name: Some("claude-3-opus".to_string())
            })
        );
        assert_eq!(parse("/m"), Some(Command::Model { name: None }));
    }

    #[test]
    fn parse_config() {
        assert_eq!(
            parse("/config agent"),
            Some(Command::Config {
                section: Some("agent".to_string())
            })
        );
    }

    #[test]
    fn parse_fallback() {
        assert_eq!(
            parse("/fallback list"),
            Some(Command::Fallback {
                action: FallbackAction::List
            })
        );
        assert_eq!(
            parse("/fb rotate"),
            Some(Command::Fallback {
                action: FallbackAction::Rotate
            })
        );
    }

    #[test]
    fn parse_invalid() {
        assert_eq!(parse("not a command"), None);
        assert_eq!(parse("/unknowncommand"), None);
    }
}

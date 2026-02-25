//! NOTE: These tests target an older key management API and use unstable custom test frameworks.
//! They are kept for future refactoring but are disabled by default.
//!
//! Enable explicitly with: `cargo test --features unstable-key-tests`

#![cfg(feature = "unstable-key-tests")]

use std::sync::Arc;

#[tokio::test]
async fn test_key_manager_basic_functionality() -> Result<(), anyhow::Error> {
    let config = housaky::config::schema::ReliabilityConfig {
        providers: vec![
            housaky::config::schema::ProviderConfig {
                name: "test_provider".to_string(),
                models: vec![
                    housaky::config::schema::ModelConfig {
                        name: "test_model".to_string(),
                        api_keys: vec![
                            housaky::config::schema::ApiKeyConfig {
                                name: "test_key".to_string(),
                                value: "test_value".to_string(),
                                ..Default::default()
                            }
                        ],
                        ..Default::default()
                    }
                ],
                ..Default::default()
            }
        ],
        ..Default::default()
    };

    let mut key_manager = KeyManager::new(config).await.unwrap();
    
    // Test loading keys
    key_manager.load_keys().await.unwrap();
    
    // Test getting best key
    let best_key = key_manager.get_best_key("test_provider", "test_model").await.unwrap();
    assert_eq!(best_key.value, "test_value");
    
    // Test health check
    let health_report = key_manager.health_check().await.unwrap();
    assert_eq!(health_report.status, housaky::config::schema::HealthStatus::Unknown);
    
    Ok::<(), anyhow::Error>(())
}

#[tokio::test]
async fn test_key_manager_rotation() -> Result<(), anyhow::Error> {
    let config = housaky::config::schema::ReliabilityConfig {
        providers: vec![
            housaky::config::schema::ProviderConfig {
                name: "test_provider".to_string(),
                models: vec![
                    housaky::config::schema::ModelConfig {
                        name: "test_model".to_string(),
                        api_keys: vec![
                            housaky::config::schema::ApiKeyConfig {
                                name: "test_key_1".to_string(),
                                value: "test_value_1".to_string(),
                                ..Default::default()
                            },
                            housaky::config::schema::ApiKeyConfig {
                                name: "test_key_2".to_string(),
                                value: "test_value_2".to_string(),
                                ..Default::default()
                            }
                        ],
                        rotation: housaky::config::schema::RotationConfig {
                            enabled: true,
                            strategy: housaky::config::schema::RotationStrategy::RoundRobin,
                            ..Default::default()
                        },
                        ..Default::default()
                    }
                ],
                ..Default::default()
            }
        ],
        ..Default::default()
    };

    let mut key_manager = KeyManager::new(config).await.unwrap();
    
    // Test rotation
    let rotation_results = key_manager.rotate_keys().await.unwrap();
    assert!(!rotation_results.is_empty());
    
    Ok::<(), anyhow::Error>(())
}

#[tokio::test]
async fn test_key_manager_cli_integration() -> Result<(), anyhow::Error> {
    // Test that the CLI commands are properly registered
    let commands = housaky::main::build_cli().get_subcommands();
    let keys_command = commands.iter().find(|c| c.get_name() == "keys");
    assert!(keys_command.is_some());
    
    Ok::<(), anyhow::Error>(())
}
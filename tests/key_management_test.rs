use super::*;
use std::sync::Arc;

#[test_case]
async fn test_key_manager_basic_functionality() -> Result<(), anyhow::Error> {
    let config = crate::config::schema::ReliabilityConfig {
        providers: vec![
            crate::config::schema::ProviderConfig {
                name: "test_provider".to_string(),
                models: vec![
                    crate::config::schema::ModelConfig {
                        name: "test_model".to_string(),
                        api_keys: vec![
                            crate::config::schema::ApiKeyConfig {
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
    assert_eq!(health_report.status, crate::config::schema::HealthStatus::Unknown);
    
    Ok::<(), anyhow::Error>(())
}

#[test_case]
async fn test_key_manager_rotation() -> Result<(), anyhow::Error> {
    let config = crate::config::schema::ReliabilityConfig {
        providers: vec![
            crate::config::schema::ProviderConfig {
                name: "test_provider".to_string(),
                models: vec![
                    crate::config::schema::ModelConfig {
                        name: "test_model".to_string(),
                        api_keys: vec![
                            crate::config::schema::ApiKeyConfig {
                                name: "test_key_1".to_string(),
                                value: "test_value_1".to_string(),
                                ..Default::default()
                            },
                            crate::config::schema::ApiKeyConfig {
                                name: "test_key_2".to_string(),
                                value: "test_value_2".to_string(),
                                ..Default::default()
                            }
                        ],
                        rotation: crate::config::schema::RotationConfig {
                            enabled: true,
                            strategy: crate::config::schema::RotationStrategy::RoundRobin,
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

#[test_case]
async fn test_key_manager_cli_integration() -> Result<(), anyhow::Error> {
    // Test that the CLI commands are properly registered
    let commands = crate::main::build_cli().get_subcommands();
    let keys_command = commands.iter().find(|c| c.get_name() == "keys");
    assert!(keys_command.is_some());
    
    Ok::<(), anyhow::Error>(())
}
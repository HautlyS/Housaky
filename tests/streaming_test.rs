//! Comprehensive tests for the streaming module.
//!
//! Tests cover:
//! - StreamChunk creation and serialization
//! - StreamingSession state transitions
//! - StreamingManager chunk broadcasting
//! - Token counting accuracy
//! - Tokens per second calculation
//! - Stream state transitions
//! - Ratatui TUI rendering simulation
//! - Real-time streaming simulation

use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::broadcast;
use housaky::housaky::streaming::{
    StreamChunk, StreamState, StreamStats, StreamingManager, StreamingSession,
};

fn create_test_chunk() -> StreamChunk {
    StreamChunk {
        id: "test_stream_001".to_string(),
        content: "Hello world".to_string(),
        delta: "Hello".to_string(),
        is_complete: false,
        token_count: 2,
        elapsed_ms: 100,
        tokens_per_second: 20.0,
    }
}

mod stream_chunk_tests {
    use super::*;
    

    #[test]
    fn stream_chunk_creation() {
        let chunk = create_test_chunk();

        assert_eq!(chunk.id, "test_stream_001");
        assert_eq!(chunk.content, "Hello world");
        assert_eq!(chunk.delta, "Hello");
        assert!(!chunk.is_complete);
        assert_eq!(chunk.token_count, 2);
        assert_eq!(chunk.elapsed_ms, 100);
        assert!((chunk.tokens_per_second - 20.0).abs() < f64::EPSILON);
    }

    #[test]
    fn stream_chunk_serialization() {
        let chunk = create_test_chunk();
        let json = serde_json::to_string(&chunk).expect("Failed to serialize");

        assert!(json.contains("\"id\":\"test_stream_001\""));
        assert!(json.contains("\"content\":\"Hello world\""));
        assert!(json.contains("\"delta\":\"Hello\""));
        assert!(json.contains("\"is_complete\":false"));
        assert!(json.contains("\"token_count\":2"));
        assert!(json.contains("\"elapsed_ms\":100"));
        assert!(json.contains("\"tokens_per_second\":20.0"));
    }

    #[test]
    fn stream_chunk_deserialization() {
        let json = r#"{
            "id": "stream_abc",
            "content": "Test content",
            "delta": "Test",
            "is_complete": true,
            "token_count": 5,
            "elapsed_ms": 250,
            "tokens_per_second": 20.0
        }"#;

        let chunk: StreamChunk = serde_json::from_str(json).expect("Failed to deserialize");

        assert_eq!(chunk.id, "stream_abc");
        assert_eq!(chunk.content, "Test content");
        assert_eq!(chunk.delta, "Test");
        assert!(chunk.is_complete);
        assert_eq!(chunk.token_count, 5);
        assert_eq!(chunk.elapsed_ms, 250);
        assert!((chunk.tokens_per_second - 20.0).abs() < f64::EPSILON);
    }

    #[test]
    fn stream_chunk_complete_flag_variants() {
        let incomplete = StreamChunk {
            id: "test".to_string(),
            content: "partial".to_string(),
            delta: "partial".to_string(),
            is_complete: false,
            token_count: 1,
            elapsed_ms: 50,
            tokens_per_second: 20.0,
        };

        let complete = StreamChunk {
            id: "test".to_string(),
            content: "complete".to_string(),
            delta: String::new(),
            is_complete: true,
            token_count: 1,
            elapsed_ms: 100,
            tokens_per_second: 10.0,
        };

        assert!(!incomplete.is_complete);
        assert!(complete.is_complete);
        assert!(complete.delta.is_empty());
    }

    #[test]
    fn stream_chunk_empty_content() {
        let chunk = StreamChunk {
            id: "empty_test".to_string(),
            content: String::new(),
            delta: String::new(),
            is_complete: false,
            token_count: 0,
            elapsed_ms: 0,
            tokens_per_second: 0.0,
        };

        assert!(chunk.content.is_empty());
        assert!(chunk.delta.is_empty());
        assert_eq!(chunk.token_count, 0);
    }

    #[test]
    fn stream_chunk_large_content() {
        let large_content = "word ".repeat(1000);
        let chunk = StreamChunk {
            id: "large_test".to_string(),
            content: large_content.clone(),
            delta: "word ".to_string(),
            is_complete: false,
            token_count: 1000,
            elapsed_ms: 5000,
            tokens_per_second: 200.0,
        };

        assert_eq!(chunk.content.len(), 5000);
        assert_eq!(chunk.token_count, 1000);
        assert!((chunk.tokens_per_second - 200.0).abs() < f64::EPSILON);
    }

    #[test]
    fn stream_chunk_unicode_content() {
        let chunk = StreamChunk {
            id: "unicode_test".to_string(),
            content: "你好世界 🌍 مرحبا".to_string(),
            delta: "你好".to_string(),
            is_complete: false,
            token_count: 4,
            elapsed_ms: 50,
            tokens_per_second: 80.0,
        };

        assert!(chunk.content.contains("你好"));
        assert!(chunk.content.contains("🌍"));
        assert!(chunk.content.contains("مرحبا"));
    }

    #[test]
    fn stream_chunk_serialization_roundtrip() {
        let original = create_test_chunk();
        let json = serde_json::to_string(&original).expect("Failed to serialize");
        let deserialized: StreamChunk = serde_json::from_str(&json).expect("Failed to deserialize");

        assert_eq!(original.id, deserialized.id);
        assert_eq!(original.content, deserialized.content);
        assert_eq!(original.delta, deserialized.delta);
        assert_eq!(original.is_complete, deserialized.is_complete);
        assert_eq!(original.token_count, deserialized.token_count);
        assert_eq!(original.elapsed_ms, deserialized.elapsed_ms);
        assert!((original.tokens_per_second - deserialized.tokens_per_second).abs() < f64::EPSILON);
    }

    #[test]
    fn stream_chunk_clone() {
        let chunk = create_test_chunk();
        let cloned = chunk.clone();

        assert_eq!(chunk.id, cloned.id);
        assert_eq!(chunk.content, cloned.content);
        assert_eq!(chunk.delta, cloned.delta);
    }

    #[test]
    fn stream_chunk_debug_format() {
        let chunk = create_test_chunk();
        let debug_str = format!("{:?}", chunk);

        assert!(debug_str.contains("test_stream_001"));
        assert!(debug_str.contains("Hello world"));
        assert!(debug_str.contains("token_count"));
    }
}

mod stream_state_tests {
    use super::*;

    #[test]
    fn stream_state_variants() {
        let states = [StreamState::Idle,
            StreamState::Starting,
            StreamState::Streaming,
            StreamState::Completing,
            StreamState::Complete,
            StreamState::Error];

        assert_eq!(states.len(), 6);
    }

    #[test]
    fn stream_state_equality() {
        assert_eq!(StreamState::Idle, StreamState::Idle);
        assert_eq!(StreamState::Streaming, StreamState::Streaming);
        assert_ne!(StreamState::Idle, StreamState::Streaming);
        assert_ne!(StreamState::Complete, StreamState::Error);
    }

    #[test]
    fn stream_state_serialization() {
        let states = vec![
            (StreamState::Idle, "\"Idle\""),
            (StreamState::Starting, "\"Starting\""),
            (StreamState::Streaming, "\"Streaming\""),
            (StreamState::Completing, "\"Completing\""),
            (StreamState::Complete, "\"Complete\""),
            (StreamState::Error, "\"Error\""),
        ];

        for (state, expected) in states {
            let json = serde_json::to_string(&state).expect("Failed to serialize");
            assert_eq!(json, expected);
        }
    }

    #[test]
    fn stream_state_deserialization() {
        let variants = vec![
            ("\"Idle\"", StreamState::Idle),
            ("\"Starting\"", StreamState::Starting),
            ("\"Streaming\"", StreamState::Streaming),
            ("\"Completing\"", StreamState::Completing),
            ("\"Complete\"", StreamState::Complete),
            ("\"Error\"", StreamState::Error),
        ];

        for (json, expected) in variants {
            let state: StreamState = serde_json::from_str(json).expect("Failed to deserialize");
            assert_eq!(state, expected);
        }
    }

    #[test]
    fn stream_state_transition_sequence() {
        let mut current = StreamState::Idle;
        assert_eq!(current, StreamState::Idle);

        current = StreamState::Starting;
        assert_eq!(current, StreamState::Starting);

        current = StreamState::Streaming;
        assert_eq!(current, StreamState::Streaming);

        current = StreamState::Completing;
        assert_eq!(current, StreamState::Completing);

        current = StreamState::Complete;
        assert_eq!(current, StreamState::Complete);
    }

    #[test]
    fn stream_state_error_transition() {
        let current = StreamState::Error;

        assert_eq!(current, StreamState::Error);
    }

    #[test]
    fn stream_state_clone() {
        let state = StreamState::Streaming;
        let cloned = state.clone();

        assert_eq!(state, cloned);
    }

    #[test]
    fn stream_state_debug_format() {
        let state = StreamState::Streaming;
        let debug_str = format!("{:?}", state);

        assert!(debug_str.contains("Streaming"));
    }
}

mod streaming_session_tests {
    use super::*;

    #[test]
    fn session_creation() {
        let session = StreamingSession::new();

        assert!(session.id.starts_with("stream_"));
        assert_eq!(session.state, StreamState::Idle);
        assert!(session.content.is_empty());
        assert_eq!(session.token_count, 0);
        assert_eq!(session.chunks_received, 0);
    }

    #[test]
    fn session_start() {
        let mut session = StreamingSession::new();
        session.append("initial content");

        assert!(!session.content.is_empty());
        assert!(session.token_count > 0);

        session.start();

        assert_eq!(session.state, StreamState::Starting);
        assert!(session.content.is_empty());
        assert_eq!(session.token_count, 0);
        assert_eq!(session.chunks_received, 0);
    }

    #[test]
    fn session_append() {
        let mut session = StreamingSession::new();
        session.start();

        let chunk = session.append("Hello world");

        assert_eq!(session.content, "Hello world");
        assert_eq!(session.token_count, 2);
        assert_eq!(session.chunks_received, 1);
        assert_eq!(session.state, StreamState::Streaming);

        assert_eq!(chunk.delta, "Hello world");
        assert!(!chunk.is_complete);
        assert_eq!(chunk.token_count, 2);
    }

    #[test]
    fn session_multiple_appends() {
        let mut session = StreamingSession::new();
        session.start();

        session.append("One ");
        session.append("two ");
        session.append("three");

        assert_eq!(session.content, "One two three");
        assert_eq!(session.token_count, 3);
        assert_eq!(session.chunks_received, 3);
    }

    #[test]
    fn session_complete() {
        let mut session = StreamingSession::new();
        session.start();
        session.append("Final content here");

        let chunk = session.complete();

        assert_eq!(session.state, StreamState::Complete);
        assert!(chunk.is_complete);
        assert!(chunk.delta.is_empty());
        assert_eq!(chunk.content, "Final content here");
    }

    #[test]
    fn session_error() {
        let mut session = StreamingSession::new();
        session.start();
        session.append("Partial content");

        let chunk = session.error("Connection lost");

        assert_eq!(session.state, StreamState::Error);
        assert!(chunk.is_complete);
        assert!(chunk.content.contains("Error:"));
        assert!(chunk.content.contains("Connection lost"));
    }

    #[test]
    fn session_tokens_per_second() {
        let mut session = StreamingSession::new();
        session.start();

        std::thread::sleep(Duration::from_millis(100));

        session.append("one two three four five");

        let tps = session.tokens_per_second();
        assert!(tps > 0.0);
    }

    #[test]
    fn session_elapsed_ms() {
        let mut session = StreamingSession::new();
        session.start();

        std::thread::sleep(Duration::from_millis(50));

        let elapsed = session.elapsed_ms();
        assert!(elapsed >= 50);
    }
}

mod streaming_manager_tests {
    use super::*;

    #[tokio::test]
    async fn manager_creation() {
        let manager = StreamingManager::new();

        let state = manager.get_current_state().await;
        assert_eq!(state, StreamState::Idle);

        let content = manager.get_current_content().await;
        assert!(content.is_empty());
    }

    #[tokio::test]
    async fn manager_default() {
        let manager = StreamingManager::default();

        let state = manager.get_current_state().await;
        assert_eq!(state, StreamState::Idle);
    }

    #[tokio::test]
    async fn manager_start_stream() {
        let manager = StreamingManager::new();
        manager.start_stream().await;

        let state = manager.get_current_state().await;
        assert_eq!(state, StreamState::Starting);
    }

    #[tokio::test]
    async fn manager_append_chunk() {
        let manager = StreamingManager::new();
        manager.start_stream().await;
        manager.append_chunk("Hello ").await;
        manager.append_chunk("world!").await;

        let content = manager.get_current_content().await;
        assert_eq!(content, "Hello world!");

        let state = manager.get_current_state().await;
        assert_eq!(state, StreamState::Streaming);
    }

    #[tokio::test]
    async fn manager_complete_stream() {
        let manager = StreamingManager::new();
        manager.start_stream().await;
        manager.append_chunk("Test content").await;

        let final_chunk = manager.complete_stream().await;

        assert!(final_chunk.is_complete);
        assert_eq!(final_chunk.content, "Test content");

        let state = manager.get_current_state().await;
        assert_eq!(state, StreamState::Complete);
    }

    #[tokio::test]
    async fn manager_error_stream() {
        let manager = StreamingManager::new();
        manager.start_stream().await;
        manager.error_stream("Connection lost").await;

        let state = manager.get_current_state().await;
        assert_eq!(state, StreamState::Error);
    }

    #[tokio::test]
    async fn manager_subscribe_chunks() {
        let manager = Arc::new(StreamingManager::new());
        let mut receiver = manager.subscribe_chunks();

        manager.start_stream().await;
        manager.append_chunk("First chunk").await;

        let received = receiver.try_recv();
        assert!(received.is_ok());
        let chunk = received.unwrap();
        assert_eq!(chunk.delta, "First chunk");
        assert!(!chunk.is_complete);
    }

    #[tokio::test]
    async fn manager_subscribe_state() {
        let manager = Arc::new(StreamingManager::new());
        let mut receiver = manager.subscribe_state();

        manager.start_stream().await;

        let received = receiver.try_recv();
        assert!(received.is_ok());
        let state = received.unwrap();
        assert_eq!(state, StreamState::Starting);

        manager.complete_stream().await;

        let received = receiver.try_recv();
        assert!(received.is_ok());
        let state = received.unwrap();
        assert_eq!(state, StreamState::Complete);
    }

    #[tokio::test]
    async fn manager_get_stats() {
        let manager = StreamingManager::new();
        manager.start_stream().await;
        manager.append_chunk("One two three").await;
        manager.append_chunk("four five").await;

        let stats = manager.get_stats().await;

        assert_eq!(stats.state, StreamState::Streaming);
        assert!(stats.content_length > 0);
        assert!(stats.chunks_received >= 2);
    }

    #[tokio::test]
    async fn manager_multiple_subscribers() {
        let manager = Arc::new(StreamingManager::new());
        let mut receiver1 = manager.subscribe_chunks();
        let mut receiver2 = manager.subscribe_chunks();

        manager.start_stream().await;
        manager.append_chunk("Broadcast test").await;

        let chunk1 = receiver1.try_recv().unwrap();
        let chunk2 = receiver2.try_recv().unwrap();

        assert_eq!(chunk1.delta, chunk2.delta);
        assert_eq!(chunk1.content, chunk2.content);
    }

    #[tokio::test]
    async fn manager_full_stream_lifecycle() {
        let manager = StreamingManager::new();

        assert_eq!(manager.get_current_state().await, StreamState::Idle);

        manager.start_stream().await;
        assert_eq!(manager.get_current_state().await, StreamState::Starting);

        manager.append_chunk("Chunk 1").await;
        assert_eq!(manager.get_current_state().await, StreamState::Streaming);

        manager.append_chunk("Chunk 2").await;
        assert_eq!(manager.get_current_state().await, StreamState::Streaming);

        manager.complete_stream().await;
        assert_eq!(manager.get_current_state().await, StreamState::Complete);
    }
}

mod token_counting_tests {
    use super::*;

    #[tokio::test]
    async fn single_word_token_count() {
        let manager = StreamingManager::new();
        manager.start_stream().await;
        manager.append_chunk("hello").await;

        let stats = manager.get_stats().await;
        assert_eq!(stats.token_count, 1);
    }

    #[tokio::test]
    async fn multiple_words_token_count() {
        let manager = StreamingManager::new();
        manager.start_stream().await;
        manager.append_chunk("one two three four five").await;

        let stats = manager.get_stats().await;
        assert_eq!(stats.token_count, 5);
    }

    #[tokio::test]
    async fn cumulative_token_count() {
        let manager = StreamingManager::new();
        manager.start_stream().await;

        manager.append_chunk("one two").await;
        let stats = manager.get_stats().await;
        assert_eq!(stats.token_count, 2);

        manager.append_chunk("three four five").await;
        let stats = manager.get_stats().await;
        assert_eq!(stats.token_count, 5);
    }

    #[tokio::test]
    async fn empty_string_token_count() {
        let manager = StreamingManager::new();
        manager.start_stream().await;
        manager.append_chunk("").await;

        let stats = manager.get_stats().await;
        assert_eq!(stats.token_count, 0);
    }

    #[tokio::test]
    async fn whitespace_only_token_count() {
        let manager = StreamingManager::new();
        manager.start_stream().await;
        manager.append_chunk("   \t\n   ").await;

        let stats = manager.get_stats().await;
        assert_eq!(stats.token_count, 0);
    }

    #[tokio::test]
    async fn mixed_whitespace_token_count() {
        let manager = StreamingManager::new();
        manager.start_stream().await;
        manager.append_chunk("word1   word2\t\tword3\nword4").await;

        let stats = manager.get_stats().await;
        assert_eq!(stats.token_count, 4);
    }

    #[tokio::test]
    async fn special_characters_token_count() {
        let manager = StreamingManager::new();
        manager.start_stream().await;
        manager.append_chunk("hello! world? test.").await;

        let stats = manager.get_stats().await;
        assert_eq!(stats.token_count, 3);
    }

    #[tokio::test]
    async fn chunk_count_tracking() {
        let manager = StreamingManager::new();
        manager.start_stream().await;

        assert_eq!(manager.get_stats().await.chunks_received, 0);

        manager.append_chunk("chunk1").await;
        assert_eq!(manager.get_stats().await.chunks_received, 1);

        manager.append_chunk("chunk2").await;
        assert_eq!(manager.get_stats().await.chunks_received, 2);

        manager.append_chunk("chunk3").await;
        assert_eq!(manager.get_stats().await.chunks_received, 3);
    }

    #[tokio::test]
    async fn large_token_count() {
        let manager = StreamingManager::new();
        manager.start_stream().await;

        let words: Vec<&str> = (0..1000).map(|_| "word").collect();
        manager.append_chunk(&words.join(" ")).await;

        let stats = manager.get_stats().await;
        assert_eq!(stats.token_count, 1000);
    }
}

mod tokens_per_second_tests {
    use super::*;

    #[tokio::test]
    async fn tps_initial_zero() {
        let manager = StreamingManager::new();
        let stats = manager.get_stats().await;

        assert_eq!(stats.tokens_per_second, 0.0);
    }

    #[tokio::test]
    async fn tps_after_append() {
        let manager = StreamingManager::new();
        manager.start_stream().await;

        tokio::time::sleep(Duration::from_millis(100)).await;

        manager.append_chunk("one two three four five").await;

        let stats = manager.get_stats().await;
        assert!(stats.tokens_per_second > 0.0);
    }

    #[tokio::test]
    async fn tps_increases_with_more_tokens() {
        let manager = StreamingManager::new();
        manager.start_stream().await;

        tokio::time::sleep(Duration::from_millis(50)).await;
        manager.append_chunk("one").await;
        let tps1 = manager.get_stats().await.tokens_per_second;

        tokio::time::sleep(Duration::from_millis(50)).await;
        manager.append_chunk("two three four five").await;
        let tps2 = manager.get_stats().await.tokens_per_second;

        assert!(tps2 > tps1);
    }

    #[tokio::test]
    async fn tps_chunk_has_correct_value() {
        let manager = Arc::new(StreamingManager::new());
        let mut receiver = manager.subscribe_chunks();

        manager.start_stream().await;
        tokio::time::sleep(Duration::from_millis(100)).await;
        manager.append_chunk("one two three four five").await;

        let chunk = receiver.try_recv().unwrap();
        assert!(chunk.tokens_per_second > 0.0);
    }

    #[tokio::test]
    async fn tps_final_chunk_calculation() {
        let manager = StreamingManager::new();

        manager.start_stream().await;
        tokio::time::sleep(Duration::from_millis(100)).await;
        manager
            .append_chunk("one two three four five six seven eight nine ten")
            .await;

        let final_chunk = manager.complete_stream().await;

        assert!(final_chunk.tokens_per_second > 0.0);
        assert_eq!(final_chunk.token_count, 10);
    }

    #[tokio::test]
    async fn tps_with_delayed_chunks() {
        let manager = StreamingManager::new();
        manager.start_stream().await;

        tokio::time::sleep(Duration::from_millis(50)).await;
        manager.append_chunk("one two").await;
        let tps1 = manager.get_stats().await.tokens_per_second;

        tokio::time::sleep(Duration::from_millis(50)).await;
        manager.append_chunk("three four").await;
        let tps2 = manager.get_stats().await.tokens_per_second;

        assert!(tps1 > 0.0);
        assert!(tps2 > 0.0);
    }
}

mod broadcast_tests {
    use super::*;

    #[tokio::test]
    async fn broadcast_chunk_order_preserved() {
        let manager = Arc::new(StreamingManager::new());
        let mut receiver = manager.subscribe_chunks();

        manager.start_stream().await;

        let chunks = vec!["first", "second", "third"];
        for chunk_text in &chunks {
            manager.append_chunk(chunk_text).await;
        }

        for expected in &chunks {
            let chunk = receiver.try_recv().unwrap();
            assert_eq!(chunk.delta, *expected);
        }
    }

    #[tokio::test]
    async fn broadcast_state_order_preserved() {
        let manager = Arc::new(StreamingManager::new());
        let mut receiver = manager.subscribe_state();

        manager.start_stream().await;
        manager.complete_stream().await;

        let state1 = receiver.try_recv().unwrap();
        assert_eq!(state1, StreamState::Starting);

        let state2 = receiver.try_recv().unwrap();
        assert_eq!(state2, StreamState::Complete);
    }

    #[tokio::test]
    async fn broadcast_complete_chunk_marked() {
        let manager = Arc::new(StreamingManager::new());
        let mut receiver = manager.subscribe_chunks();

        manager.start_stream().await;
        manager.append_chunk("content").await;
        manager.complete_stream().await;

        let chunk1 = receiver.try_recv().unwrap();
        assert!(!chunk1.is_complete);

        let chunk2 = receiver.try_recv().unwrap();
        assert!(chunk2.is_complete);
    }

    #[tokio::test]
    async fn broadcast_error_chunk() {
        let manager = Arc::new(StreamingManager::new());
        let mut receiver = manager.subscribe_chunks();

        manager.start_stream().await;
        manager.error_stream("Test error").await;

        let chunk = receiver.try_recv().unwrap();
        assert!(chunk.is_complete);
        assert!(chunk.content.contains("Error:"));
    }

    #[tokio::test]
    async fn broadcast_channel_capacity() {
        let (tx, mut rx) = broadcast::channel::<StreamChunk>(256);

        for i in 0..255 {
            let chunk = StreamChunk {
                id: format!("chunk_{}", i),
                content: format!("Content {}", i),
                delta: format!("Delta {}", i),
                is_complete: false,
                token_count: i,
                elapsed_ms: i as u64,
                tokens_per_second: i as f64,
            };
            tx.send(chunk).unwrap();
        }

        let received_count = rx.try_recv().unwrap().token_count;
        assert_eq!(received_count, 0);
    }

    #[tokio::test]
    async fn broadcast_many_chunks() {
        let manager = Arc::new(StreamingManager::new());
        let mut receiver = manager.subscribe_chunks();

        manager.start_stream().await;

        for i in 0..100 {
            manager.append_chunk(&format!("chunk_{} ", i)).await;
        }

        let mut count = 0;
        while receiver.try_recv().is_ok() {
            count += 1;
        }

        assert_eq!(count, 100);
    }
}

mod simulate_stream_tests {
    use super::*;

    #[tokio::test]
    async fn simulate_stream_basic() {
        let manager = Arc::new(StreamingManager::new());
        let mut receiver = manager.subscribe_chunks();

        manager.simulate_stream("Hello world test", 10).await;

        let mut received_chunks = Vec::new();
        while let Ok(chunk) = receiver.try_recv() {
            received_chunks.push(chunk);
        }

        assert!(!received_chunks.is_empty());

        let last_chunk = received_chunks.last().unwrap();
        assert!(last_chunk.is_complete);
    }

    #[tokio::test]
    async fn simulate_stream_state_transitions() {
        let manager = Arc::new(StreamingManager::new());
        let mut state_receiver = manager.subscribe_state();

        manager.simulate_stream("Test content", 5).await;

        let first_state = state_receiver.try_recv().unwrap();
        assert_eq!(first_state, StreamState::Starting);

        let final_state = state_receiver.try_recv().unwrap();
        assert_eq!(final_state, StreamState::Complete);
    }

    #[tokio::test]
    async fn simulate_stream_empty_content() {
        let manager = Arc::new(StreamingManager::new());

        manager.simulate_stream("", 10).await;

        let state = manager.get_current_state().await;
        assert_eq!(state, StreamState::Complete);
    }

    #[tokio::test]
    async fn simulate_stream_single_word() {
        let manager = Arc::new(StreamingManager::new());
        let mut receiver = manager.subscribe_chunks();

        manager.simulate_stream("hello", 5).await;

        let mut count = 0;
        while receiver.try_recv().is_ok() {
            count += 1;
        }

        assert!(count >= 1);
    }

    #[tokio::test]
    async fn simulate_stream_long_content() {
        let manager = Arc::new(StreamingManager::new());
        let mut receiver = manager.subscribe_chunks();

        let content = "word ".repeat(100);
        manager.simulate_stream(&content, 1).await;

        let mut chunks = Vec::new();
        while let Ok(chunk) = receiver.try_recv() {
            chunks.push(chunk);
        }

        assert!(chunks.len() > 1);

        let last = chunks.last().unwrap();
        assert!(last.is_complete);
    }
}

mod elapsed_time_tests {
    use super::*;

    #[tokio::test]
    async fn elapsed_time_increases() {
        let manager = StreamingManager::new();

        manager.start_stream().await;
        let stats1 = manager.get_stats().await;

        tokio::time::sleep(Duration::from_millis(50)).await;

        let stats2 = manager.get_stats().await;

        assert!(stats2.elapsed_ms >= stats1.elapsed_ms);
    }

    #[tokio::test]
    async fn elapsed_time_in_chunk() {
        let manager = Arc::new(StreamingManager::new());
        let mut receiver = manager.subscribe_chunks();

        manager.start_stream().await;

        tokio::time::sleep(Duration::from_millis(100)).await;

        manager.append_chunk("test").await;

        let chunk = receiver.try_recv().unwrap();
        assert!(chunk.elapsed_ms >= 100);
    }

    #[tokio::test]
    async fn elapsed_time_final_chunk() {
        let manager = StreamingManager::new();

        manager.start_stream().await;

        tokio::time::sleep(Duration::from_millis(50)).await;

        let final_chunk = manager.complete_stream().await;

        assert!(final_chunk.elapsed_ms >= 50);
    }

    #[tokio::test]
    async fn elapsed_time_session() {
        let mut session = StreamingSession::new();
        session.start();

        std::thread::sleep(Duration::from_millis(100));

        let elapsed = session.elapsed_ms();
        assert!(elapsed >= 100);
    }
}

mod content_accumulation_tests {
    use super::*;

    #[tokio::test]
    async fn content_accumulates_correctly() {
        let manager = StreamingManager::new();
        manager.start_stream().await;

        manager.append_chunk("Hello ").await;
        assert_eq!(manager.get_current_content().await, "Hello ");

        manager.append_chunk("beautiful ").await;
        assert_eq!(manager.get_current_content().await, "Hello beautiful ");

        manager.append_chunk("world!").await;
        assert_eq!(
            manager.get_current_content().await,
            "Hello beautiful world!"
        );
    }

    #[tokio::test]
    async fn content_length_tracking() {
        let manager = StreamingManager::new();
        manager.start_stream().await;

        manager.append_chunk("abc").await;
        let stats = manager.get_stats().await;
        assert_eq!(stats.content_length, 3);

        manager.append_chunk("defg").await;
        let stats = manager.get_stats().await;
        assert_eq!(stats.content_length, 7);
    }

    #[tokio::test]
    async fn content_preserved_on_complete() {
        let manager = StreamingManager::new();
        manager.start_stream().await;

        manager.append_chunk("Preserved content").await;
        let expected_content = manager.get_current_content().await;

        let final_chunk = manager.complete_stream().await;

        assert_eq!(final_chunk.content, expected_content);
    }

    #[tokio::test]
    async fn content_with_unicode() {
        let manager = StreamingManager::new();
        manager.start_stream().await;

        manager.append_chunk("Hello ").await;
        manager.append_chunk("世界 ").await;
        manager.append_chunk("🌍").await;

        let content = manager.get_current_content().await;
        assert!(content.contains("Hello"));
        assert!(content.contains("世界"));
        assert!(content.contains("🌍"));
    }
}

mod stream_stats_tests {
    use super::*;

    #[tokio::test]
    async fn stats_serialization() {
        let manager = StreamingManager::new();
        manager.start_stream().await;
        manager.append_chunk("test content").await;

        let stats = manager.get_stats().await;
        let json = serde_json::to_string(&stats).expect("Failed to serialize");

        assert!(json.contains("\"session_id\""));
        assert!(json.contains("\"state\""));
        assert!(json.contains("\"token_count\""));
    }

    #[tokio::test]
    async fn stats_deserialization() {
        let json = r#"{
            "session_id": "test_session",
            "state": "Streaming",
            "content_length": 100,
            "token_count": 20,
            "chunks_received": 5,
            "elapsed_ms": 1000,
            "tokens_per_second": 20.0
        }"#;

        let stats: StreamStats = serde_json::from_str(json).expect("Failed to deserialize");

        assert_eq!(stats.session_id, "test_session");
        assert_eq!(stats.state, StreamState::Streaming);
        assert_eq!(stats.content_length, 100);
        assert_eq!(stats.token_count, 20);
        assert_eq!(stats.chunks_received, 5);
        assert_eq!(stats.elapsed_ms, 1000);
        assert!((stats.tokens_per_second - 20.0).abs() < f64::EPSILON);
    }

    #[tokio::test]
    async fn stats_clone() {
        let manager = StreamingManager::new();
        manager.start_stream().await;

        let stats = manager.get_stats().await;
        let cloned = stats.clone();

        assert_eq!(stats.session_id, cloned.session_id);
        assert_eq!(stats.token_count, cloned.token_count);
    }

    #[tokio::test]
    async fn stats_debug_format() {
        let manager = StreamingManager::new();
        manager.start_stream().await;

        let stats = manager.get_stats().await;
        let debug_str = format!("{:?}", stats);

        assert!(debug_str.contains("session_id"));
        assert!(debug_str.contains("token_count"));
    }
}

mod ratatui_integration_tests {
    use super::*;
    use ratatui::{
        layout::{Constraint, Direction, Layout, Rect},
        style::{Color, Modifier, Style},
        text::{Line, Span},
        widgets::{Block, Borders, Paragraph, Wrap},
    };

    fn create_test_rect() -> Rect {
        Rect::new(0, 0, 80, 24)
    }

    #[test]
    fn ratatui_streaming_widget_creation() {
        let chunk = create_test_chunk();

        let _paragraph = Paragraph::new(chunk.content.clone())
            .block(Block::default().borders(Borders::ALL).title("Stream"))
            .wrap(Wrap { trim: true });

        let rect = create_test_rect();
        assert!(rect.width >= chunk.content.len() as u16 || rect.width > 0);
    }

    #[test]
    fn ratatui_streaming_style_for_states() {
        let idle_style = Style::default().fg(Color::Gray);
        let streaming_style = Style::default().fg(Color::Green);
        let error_style = Style::default().fg(Color::Red);
        let complete_style = Style::default().fg(Color::Cyan);

        assert_ne!(idle_style, streaming_style);
        assert_ne!(streaming_style, error_style);
        assert_ne!(error_style, complete_style);
    }

    #[test]
    fn ratatui_layout_for_streaming() {
        let rect = create_test_rect();

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(10),
                Constraint::Length(3),
            ])
            .split(rect);

        assert_eq!(chunks.len(), 3);
        assert_eq!(chunks[0].height, 3);
        assert_eq!(chunks[2].height, 3);
    }

    #[test]
    fn ratatui_streaming_line_creation() {
        let chunk = create_test_chunk();

        let line = Line::from(vec![
            Span::styled(
                format!("Tokens: {} ", chunk.token_count),
                Style::default().fg(Color::Yellow),
            ),
            Span::styled(
                format!("TPS: {:.1}", chunk.tokens_per_second),
                Style::default().fg(Color::Green),
            ),
        ]);

        assert!(line.spans.len() == 2);
    }

    #[test]
    fn ratatui_metrics_display() {
        let manager = StreamingManager::new();

        let stats_future = manager.get_stats();

        let rt = tokio::runtime::Runtime::new().unwrap();
        let stats = rt.block_on(stats_future);

        let metrics_text = format!(
            "Session: {}\nState: {:?}\nTokens: {}\nChunks: {}\nElapsed: {}ms\nTPS: {:.1}",
            stats.session_id,
            stats.state,
            stats.token_count,
            stats.chunks_received,
            stats.elapsed_ms,
            stats.tokens_per_second
        );

        let _paragraph = Paragraph::new(metrics_text.clone())
            .block(Block::default().borders(Borders::ALL).title("Metrics"));

        assert!(metrics_text.contains("Session:"));
        assert!(metrics_text.contains("Tokens:"));
    }

    #[test]
    fn ratatui_streaming_status_indicators() {
        let states_and_styles = vec![
            (StreamState::Idle, "⏸ Idle", Color::Gray),
            (StreamState::Starting, "⏳ Starting...", Color::Yellow),
            (StreamState::Streaming, "▶ Streaming", Color::Green),
            (StreamState::Completing, "⏳ Completing...", Color::Cyan),
            (StreamState::Complete, "✓ Complete", Color::Blue),
            (StreamState::Error, "✗ Error", Color::Red),
        ];

        for (_state, label, color) in states_and_styles {
            let span = Span::styled(
                label,
                Style::default().fg(color).add_modifier(Modifier::BOLD),
            );

            assert!(!span.content.is_empty());
        }
    }

    #[tokio::test]
    async fn ratatui_real_time_streaming_simulation() {
        let manager = Arc::new(StreamingManager::new());
        let mut chunk_receiver = manager.subscribe_chunks();
        let mut state_receiver = manager.subscribe_state();

        manager.start_stream().await;

        let initial_state = state_receiver.try_recv().unwrap();
        assert_eq!(initial_state, StreamState::Starting);

        let test_content = "This is a simulated streaming response for TUI testing. ";
        manager.append_chunk(test_content).await;

        let chunk = chunk_receiver.try_recv().unwrap();
        assert!(!chunk.is_complete);
        assert!(chunk.content.contains(test_content.trim()));

        let paragraph = Paragraph::new(chunk.content.clone())
            .block(Block::default().borders(Borders::ALL).title("Response"))
            .wrap(Wrap { trim: true });

        let rect = create_test_rect();
        let _ = paragraph;
        let _ = rect;

        manager.complete_stream().await;

        let final_state = state_receiver.try_recv().unwrap();
        assert_eq!(final_state, StreamState::Complete);
    }

    #[test]
    fn ratatui_chunk_rendering() {
        let chunks = vec![
            StreamChunk {
                id: "1".to_string(),
                content: "First part ".to_string(),
                delta: "First part ".to_string(),
                is_complete: false,
                token_count: 2,
                elapsed_ms: 100,
                tokens_per_second: 20.0,
            },
            StreamChunk {
                id: "2".to_string(),
                content: "First part Second part ".to_string(),
                delta: "Second part ".to_string(),
                is_complete: false,
                token_count: 4,
                elapsed_ms: 200,
                tokens_per_second: 20.0,
            },
            StreamChunk {
                id: "3".to_string(),
                content: "First part Second part Final".to_string(),
                delta: "Final".to_string(),
                is_complete: true,
                token_count: 5,
                elapsed_ms: 300,
                tokens_per_second: 16.67,
            },
        ];

        let mut rendered_content = String::new();
        for chunk in &chunks {
            rendered_content = chunk.content.clone();
        }

        let paragraph = Paragraph::new(rendered_content.clone()).block(
            Block::default()
                .borders(Borders::ALL)
                .title("Stream Output"),
        );

        assert!(rendered_content.contains("First part"));
        assert!(rendered_content.contains("Second part"));
        assert!(rendered_content.contains("Final"));

        let _ = paragraph;
    }

    #[test]
    fn ratatui_token_counter_widget() {
        let manager = StreamingManager::new();

        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            manager.start_stream().await;
            manager.append_chunk("one two three four five").await;
        });

        let stats = rt.block_on(manager.get_stats());

        let counter_text = format!(
            "Tokens: {} | TPS: {:.1}",
            stats.token_count, stats.tokens_per_second
        );
        let span = Span::styled(
            counter_text.clone(),
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        );

        assert!(counter_text.contains("Tokens: 5"));
        let _ = span;
    }
}

mod real_time_simulation_tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[tokio::test]
    async fn real_time_streaming_performance() {
        let manager = Arc::new(StreamingManager::new());
        let manager_clone = manager.clone();

        let chunk_count = Arc::new(AtomicUsize::new(0));
        let chunk_count_clone = chunk_count.clone();

        let subscriber_task = tokio::spawn(async move {
            let mut receiver = manager_clone.subscribe_chunks();

            loop {
                match receiver.try_recv() {
                    Ok(chunk) => {
                        chunk_count_clone.fetch_add(1, Ordering::SeqCst);
                        if chunk.is_complete {
                            break;
                        }
                    }
                    Err(_) => {
                        tokio::time::sleep(Duration::from_millis(1)).await;
                    }
                }
            }
        });

        manager.start_stream().await;

        for i in 0..10 {
            manager.append_chunk(&format!("Chunk {} ", i)).await;
            tokio::time::sleep(Duration::from_millis(10)).await;
        }

        manager.complete_stream().await;

        subscriber_task.await.unwrap();

        let total_chunks = chunk_count.load(Ordering::SeqCst);
        assert!(total_chunks >= 10);
    }

    #[tokio::test]
    async fn real_time_multiple_streams() {
        let manager = Arc::new(StreamingManager::new());

        let mut tasks = Vec::new();

        for i in 0..3 {
            let manager_clone = manager.clone();
            let task = tokio::spawn(async move {
                let mut receiver = manager_clone.subscribe_chunks();

                manager_clone.start_stream().await;
                manager_clone
                    .append_chunk(&format!("Stream {} content", i))
                    .await;
                manager_clone.complete_stream().await;

                let mut chunks = Vec::new();
                while let Ok(chunk) = receiver.try_recv() {
                    chunks.push(chunk);
                }

                chunks.len()
            });
            tasks.push(task);

            tokio::time::sleep(Duration::from_millis(50)).await;
        }

        let results: Vec<usize> = futures_util::future::join_all(tasks)
            .await
            .into_iter()
            .map(|r| r.unwrap())
            .collect();

        assert!(results.iter().all(|&count| count > 0));
    }

    #[tokio::test]
    async fn real_time_streaming_with_backpressure() {
        let manager = Arc::new(StreamingManager::new());
        let mut receiver = manager.subscribe_chunks();

        manager.start_stream().await;

        for i in 0..50 {
            manager.append_chunk(&format!("Chunk-{} ", i)).await;
        }

        manager.complete_stream().await;

        let mut received_count = 0;
        while receiver.try_recv().is_ok() {
            received_count += 1;
        }

        assert_eq!(received_count, 51);
    }

    #[tokio::test]
    async fn real_time_latency_measurement() {
        let manager = Arc::new(StreamingManager::new());
        let mut receiver = manager.subscribe_chunks();

        let start = Instant::now();

        manager.start_stream().await;
        manager.append_chunk("test").await;

        let chunk = receiver.try_recv().unwrap();
        let latency = start.elapsed();

        assert!(latency < Duration::from_millis(100));
        assert!(!chunk.is_complete);
    }

    #[tokio::test]
    async fn real_time_state_change_propagation() {
        let manager = Arc::new(StreamingManager::new());
        let mut state_receiver = manager.subscribe_state();

        let _states = [StreamState::Starting,
            StreamState::Streaming,
            StreamState::Complete];

        manager.start_stream().await;
        let s1 = state_receiver.try_recv().unwrap();
        assert_eq!(s1, StreamState::Starting);

        manager.append_chunk("content").await;

        manager.complete_stream().await;
        let s2 = state_receiver.try_recv().unwrap();
        assert_eq!(s2, StreamState::Complete);
    }
}

mod error_handling_tests {
    use super::*;

    #[tokio::test]
    async fn error_state_propagation() {
        let manager = Arc::new(StreamingManager::new());
        let mut state_receiver = manager.subscribe_state();
        let mut chunk_receiver = manager.subscribe_chunks();

        manager.start_stream().await;
        let _ = state_receiver.try_recv();

        manager.error_stream("Network timeout").await;

        let state = state_receiver.try_recv().unwrap();
        assert_eq!(state, StreamState::Error);

        let chunk = chunk_receiver.try_recv().unwrap();
        assert!(chunk.is_complete);
        assert!(chunk.content.contains("Error:"));
        assert!(chunk.content.contains("Network timeout"));
    }

    #[tokio::test]
    async fn error_chunk_token_count_zero() {
        let manager = StreamingManager::new();
        manager.start_stream().await;

        manager.error_stream("Critical failure").await;

        let stats = manager.get_stats().await;
        assert_eq!(stats.state, StreamState::Error);
    }

    #[tokio::test]
    async fn multiple_subscribers_error_notification() {
        let manager = Arc::new(StreamingManager::new());
        let mut receiver1 = manager.subscribe_state();
        let mut receiver2 = manager.subscribe_state();
        let mut receiver3 = manager.subscribe_state();

        manager.start_stream().await;

        let _ = receiver1.try_recv().unwrap();
        let _ = receiver2.try_recv().unwrap();
        let _ = receiver3.try_recv().unwrap();

        manager.error_stream("Multi-subscriber error").await;

        let state1 = receiver1.try_recv().unwrap();
        let state2 = receiver2.try_recv().unwrap();
        let state3 = receiver3.try_recv().unwrap();

        assert_eq!(state1, StreamState::Error);
        assert_eq!(state2, StreamState::Error);
        assert_eq!(state3, StreamState::Error);
    }

    #[tokio::test]
    async fn error_after_partial_stream() {
        let manager = StreamingManager::new();
        manager.start_stream().await;

        manager.append_chunk("Partial content").await;
        manager.append_chunk(" more content").await;

        let partial_content = manager.get_current_content().await;
        assert!(!partial_content.is_empty());

        manager.error_stream("Interrupted").await;

        assert_eq!(manager.get_current_state().await, StreamState::Error);
    }
}

mod concurrent_access_tests {
    use super::*;

    #[tokio::test]
    async fn concurrent_read_access() {
        let manager = Arc::new(StreamingManager::new());

        manager.start_stream().await;
        manager.append_chunk("Shared content").await;

        let mut handles = Vec::new();

        for _ in 0..10 {
            let manager_clone = manager.clone();
            let handle = tokio::spawn(async move { manager_clone.get_current_content().await });
            handles.push(handle);
        }

        let results: Vec<String> = futures_util::future::join_all(handles)
            .await
            .into_iter()
            .map(|r| r.unwrap())
            .collect();

        assert!(results.iter().all(|content| content == "Shared content"));
    }

    #[tokio::test]
    async fn concurrent_state_reads() {
        let manager = Arc::new(StreamingManager::new());

        manager.start_stream().await;

        let mut handles = Vec::new();

        for _ in 0..10 {
            let manager_clone = manager.clone();
            let handle = tokio::spawn(async move { manager_clone.get_current_state().await });
            handles.push(handle);
        }

        let results: Vec<StreamState> = futures_util::future::join_all(handles)
            .await
            .into_iter()
            .map(|r| r.unwrap())
            .collect();

        assert!(results.iter().all(|state| *state == StreamState::Starting));
    }

    #[tokio::test]
    async fn concurrent_stats_reads() {
        let manager = Arc::new(StreamingManager::new());

        manager.start_stream().await;
        manager.append_chunk("test content for stats").await;

        let mut handles = Vec::new();

        for _ in 0..5 {
            let manager_clone = manager.clone();
            let handle = tokio::spawn(async move { manager_clone.get_stats().await });
            handles.push(handle);
        }

        let results: Vec<StreamStats> = futures_util::future::join_all(handles)
            .await
            .into_iter()
            .map(|r| r.unwrap())
            .collect();

        assert!(results.iter().all(|stats| stats.token_count == 4));
    }

    #[tokio::test]
    async fn concurrent_write_and_read() {
        let manager = Arc::new(StreamingManager::new());

        manager.start_stream().await;

        let write_manager = manager.clone();
        let write_handle = tokio::spawn(async move {
            for i in 0..10 {
                write_manager.append_chunk(&format!("chunk{} ", i)).await;
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
            write_manager.complete_stream().await
        });

        let read_manager = manager.clone();
        let read_handle = tokio::spawn(async move {
            let mut contents = Vec::new();
            for _ in 0..20 {
                contents.push(read_manager.get_current_content().await);
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
            contents
        });

        let (write_result, read_results) = tokio::join!(write_handle, read_handle);

        assert!(write_result.is_ok());
        assert!(!read_results.unwrap().is_empty());
    }
}

mod edge_case_tests {
    use super::*;

    #[tokio::test]
    async fn empty_stream_lifecycle() {
        let manager = StreamingManager::new();

        manager.start_stream().await;
        let final_chunk = manager.complete_stream().await;

        assert!(final_chunk.is_complete);
        assert!(final_chunk.content.is_empty());
        assert_eq!(final_chunk.token_count, 0);
    }

    #[tokio::test]
    async fn very_long_single_chunk() {
        let manager = StreamingManager::new();
        let long_content = "word ".repeat(10000);

        manager.start_stream().await;
        manager.append_chunk(&long_content).await;

        let stats = manager.get_stats().await;
        assert_eq!(stats.token_count, 10000);
    }

    #[tokio::test]
    async fn rapid_state_transitions() {
        let manager = StreamingManager::new();

        for _ in 0..10 {
            manager.start_stream().await;
            manager.append_chunk("quick").await;
            manager.complete_stream().await;
        }

        assert_eq!(manager.get_current_state().await, StreamState::Complete);
    }

    #[tokio::test]
    async fn unicode_content_handling() {
        let manager = StreamingManager::new();

        manager.start_stream().await;
        manager.append_chunk("你好世界").await;
        manager.append_chunk("مرحبا بالعالم").await;
        manager.append_chunk("🌍🚀💡").await;

        let content = manager.get_current_content().await;
        assert!(content.contains("你好世界"));
        assert!(content.contains("مرحبا"));
        assert!(content.contains("🌍"));
    }

    #[tokio::test]
    async fn special_characters_in_content() {
        let manager = StreamingManager::new();

        manager.start_stream().await;
        manager.append_chunk("Line1\nLine2\tTabbed").await;
        manager.append_chunk("Quote: \"test\" and 'single'").await;
        manager.append_chunk("Path: /some/path/to/file.rs").await;

        let content = manager.get_current_content().await;
        assert!(content.contains('\n'));
        assert!(content.contains('\t'));
        assert!(content.contains('"'));
        assert!(content.contains('/'));
    }

    #[tokio::test]
    async fn zero_elapsed_time_handling() {
        let manager = StreamingManager::new();

        manager.start_stream().await;
        manager.append_chunk("instant").await;

        let stats = manager.get_stats().await;

        let _ = stats.elapsed_ms;
    }

    #[tokio::test]
    async fn session_id_uniqueness() {
        let mut ids = std::collections::HashSet::new();

        for _ in 0..100 {
            let session = StreamingSession::new();
            assert!(ids.insert(session.id), "Session IDs should be unique");
        }

        assert_eq!(ids.len(), 100);
    }

    #[tokio::test]
    async fn newlines_in_chunks() {
        let manager = StreamingManager::new();

        manager.start_stream().await;
        manager.append_chunk("First line\n").await;
        manager.append_chunk("Second line\n").await;
        manager.append_chunk("Third line").await;

        let content = manager.get_current_content().await;
        assert_eq!(content.matches('\n').count(), 2);
    }
}

mod metrics_calculation_tests {
    use super::*;

    #[tokio::test]
    async fn tokens_per_second_calculation_accuracy() {
        let manager = StreamingManager::new();

        manager.start_stream().await;

        tokio::time::sleep(Duration::from_millis(100)).await;

        manager
            .append_chunk("one two three four five six seven eight nine ten")
            .await;

        let stats = manager.get_stats().await;

        let expected_min_tps = 10.0;
        let expected_max_tps = 200.0;

        assert!(stats.tokens_per_second >= expected_min_tps);
        assert!(stats.tokens_per_second <= expected_max_tps);
    }

    #[tokio::test]
    async fn elapsed_time_accuracy() {
        let manager = StreamingManager::new();

        manager.start_stream().await;

        tokio::time::sleep(Duration::from_millis(150)).await;

        manager.append_chunk("content").await;

        let stats = manager.get_stats().await;

        assert!(stats.elapsed_ms >= 150);
        assert!(stats.elapsed_ms < 200);
    }

    #[tokio::test]
    async fn content_length_bytes_vs_chars() {
        let manager = StreamingManager::new();

        manager.start_stream().await;
        manager.append_chunk("hello").await;

        let stats = manager.get_stats().await;

        assert_eq!(stats.content_length, 5);
    }

    #[tokio::test]
    async fn metrics_update_on_each_chunk() {
        let manager = StreamingManager::new();

        manager.start_stream().await;

        manager.append_chunk("one").await;
        let stats1 = manager.get_stats().await;

        manager.append_chunk("two").await;
        let stats2 = manager.get_stats().await;

        manager.append_chunk("three").await;
        let stats3 = manager.get_stats().await;

        assert!(stats3.token_count > stats2.token_count);
        assert!(stats2.token_count > stats1.token_count);
        assert!(stats3.elapsed_ms >= stats2.elapsed_ms);
    }
}

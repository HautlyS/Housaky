//! Multimodal AI Module - Advanced cross-modal fusion
pub mod fusion;
pub mod transformer;
pub mod clip;
pub mod temporal;

pub use fusion::{MultimodalFusion, ModalityEmbedding, Modality, CrossModalRetrieval};
pub use transformer::CrossAttentionTransformer;
pub use clip::CLIPAlignment;
pub use temporal::TemporalFusion;

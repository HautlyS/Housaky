//! Content chunking for large files
use housaky_core::crypto::hash;

/// Chunk a file into pieces
pub fn chunk_data(data: &[u8], chunk_size: usize) -> Vec<(Vec<u8>, [u8; 32])> {
    data.chunks(chunk_size)
        .map(|chunk| {
            let hash = hash(chunk);
            (chunk.to_vec(), hash)
        })
        .collect()
}

/// Reassemble chunks
pub fn reassemble_chunks(chunks: &[Vec<u8>]) -> Vec<u8> {
    chunks.iter().flatten().copied().collect()
}

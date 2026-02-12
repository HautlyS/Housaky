//! Quantum-Inspired Photon State Database
//!
//! This module implements a quantum-inspired database that uses light-based
//! data representations and quantum computing concepts for efficient storage
//! and querying of photon state measurements.

use anyhow::Result;
use housaky_core::quantum::{QuantumCircuit, QuantumGate, QuantumState};
use serde::{Deserialize, Serialize};
use sled::Db;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

/// Photon state record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhotonRecord {
    /// Unique identifier
    pub id: String,
    /// Spatial coordinates (x, y, z)
    pub position: (f64, f64, f64),
    /// Temporal coordinate (timestamp)
    pub timestamp: u64,
    /// Polarization angle (0-180 degrees)
    pub polarization: f64,
    /// Intensity (0.0 - 1.0)
    pub intensity: f64,
    /// Wavelength in nanometers
    pub wavelength_nm: f64,
    /// Phase angle (0-360 degrees)
    pub phase: f64,
    /// Quantum superposition state (if measured)
    pub quantum_state: Option<Vec<f64>>,
    /// Confidence/probability of measurement
    pub confidence: f64,
    /// Raw sensor data
    pub raw_data: Vec<u8>,
}

/// Quantum superposition storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuperpositionStorage {
    /// Multiple hypotheses stored simultaneously
    pub hypotheses: Vec<Hypothesis>,
    /// Entangled record IDs
    pub entangled_with: Vec<String>,
    /// Measurement probability distribution
    pub probabilities: Vec<f64>,
}

/// Hypothesis for superposition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hypothesis {
    /// Hypothesis ID
    pub id: String,
    /// Description
    pub description: String,
    /// Confidence weight
    pub weight: f64,
    /// Associated data
    pub data: Vec<u8>,
}

/// Spatial index for efficient spatial queries
#[derive(Debug, Clone)]
pub struct SpatialIndex {
    /// Grid-based spatial partitioning
    grid: HashMap<(i64, i64, i64), Vec<String>>,
    /// Grid cell size
    cell_size: f64,
}

impl SpatialIndex {
    /// Create new spatial index
    pub fn new(cell_size: f64) -> Self {
        Self {
            grid: HashMap::new(),
            cell_size,
        }
    }

    /// Insert record into spatial index
    pub fn insert(&mut self, record_id: String, position: (f64, f64, f64)) {
        let cell = self.position_to_cell(position);
        self.grid
            .entry(cell)
            .or_insert_with(Vec::new)
            .push(record_id);
    }

    /// Query records within spatial range
    pub fn query_range(&self, center: (f64, f64, f64), radius: f64) -> Vec<String> {
        let mut results = Vec::new();
        let center_cell = self.position_to_cell(center);
        let cell_radius = (radius / self.cell_size).ceil() as i64;

        for dx in -cell_radius..=cell_radius {
            for dy in -cell_radius..=cell_radius {
                for dz in -cell_radius..=cell_radius {
                    let cell = (center_cell.0 + dx, center_cell.1 + dy, center_cell.2 + dz);
                    if let Some(records) = self.grid.get(&cell) {
                        results.extend(records.clone());
                    }
                }
            }
        }

        results
    }

    /// Convert position to grid cell
    fn position_to_cell(&self, position: (f64, f64, f64)) -> (i64, i64, i64) {
        (
            (position.0 / self.cell_size).floor() as i64,
            (position.1 / self.cell_size).floor() as i64,
            (position.2 / self.cell_size).floor() as i64,
        )
    }
}

/// Temporal index for time-series queries
#[derive(Debug, Clone)]
pub struct TemporalIndex {
    /// Time-based B-tree index
    time_tree: HashMap<u64, Vec<String>>,
    /// Time bucket size in seconds
    bucket_size: u64,
}

impl TemporalIndex {
    /// Create new temporal index
    pub fn new(bucket_size: u64) -> Self {
        Self {
            time_tree: HashMap::new(),
            bucket_size,
        }
    }

    /// Insert record into temporal index
    pub fn insert(&mut self, record_id: String, timestamp: u64) {
        let bucket = timestamp / self.bucket_size;
        self.time_tree
            .entry(bucket)
            .or_insert_with(Vec::new)
            .push(record_id);
    }

    /// Query records within time range
    pub fn query_range(&self, start: u64, end: u64) -> Vec<String> {
        let mut results = Vec::new();
        let start_bucket = start / self.bucket_size;
        let end_bucket = end / self.bucket_size;

        for bucket in start_bucket..=end_bucket {
            if let Some(records) = self.time_tree.get(&bucket) {
                results.extend(records.clone());
            }
        }

        results
    }
}

/// Quantum-inspired database query
#[derive(Debug, Clone)]
pub struct QuantumQuery {
    /// Spatial constraints (position, radius)
    pub spatial_constraint: Option<((f64, f64, f64), f64)>,
    /// Temporal constraints (start, end)
    pub temporal_constraint: Option<(u64, u64)>,
    /// Polarization filter
    pub polarization_filter: Option<(f64, f64)>, // (min, max) angle
    /// Wavelength filter
    pub wavelength_filter: Option<(f64, f64)>, // (min, max) nm
    /// Intensity threshold
    pub intensity_threshold: Option<f64>,
    /// Quantum superposition query
    pub superposition_query: Option<SuperpositionQuery>,
}

/// Superposition query parameters
#[derive(Debug, Clone)]
pub struct SuperpositionQuery {
    /// Measurement basis
    pub basis: Vec<f64>,
    /// Collapse to most probable state
    pub collapse: bool,
    /// Number of states to return
    pub top_k: usize,
}

/// Photon database with quantum-inspired features
pub struct PhotonDatabase {
    /// Underlying key-value store
    db: Db,
    /// Spatial index
    spatial_index: SpatialIndex,
    /// Temporal index
    temporal_index: TemporalIndex,
    /// Superposition storage
    superposition_store: HashMap<String, SuperpositionStorage>,
    /// Quantum circuit for query processing
    query_circuit: QuantumCircuit,
}

impl PhotonDatabase {
    /// Create new photon database
    pub fn new(path: &str) -> Result<Self> {
        let db = sled::open(path)?;
        let spatial_index = SpatialIndex::new(1.0); // 1 meter cells
        let temporal_index = TemporalIndex::new(60); // 1 minute buckets
        let query_circuit = QuantumCircuit::new(8); // 8-qubit circuit

        Ok(Self {
            db,
            spatial_index,
            temporal_index,
            superposition_store: HashMap::new(),
            query_circuit,
        })
    }

    /// Insert photon record
    pub fn insert(&mut self, record: PhotonRecord) -> Result<()> {
        let id = record.id.clone();

        // Update indexes
        self.spatial_index.insert(id.clone(), record.position);
        self.temporal_index.insert(id.clone(), record.timestamp);

        // Serialize and store
        let value = bincode::serialize(&record)?;
        self.db.insert(format!("record:{}", id), value)?;

        Ok(())
    }

    /// Insert record with superposition state
    pub fn insert_superposition(
        &mut self,
        record_id: String,
        superposition: SuperpositionStorage,
    ) -> Result<()> {
        // Store superposition
        self.superposition_store
            .insert(record_id.clone(), superposition.clone());

        // Serialize and store
        let value = bincode::serialize(&superposition)?;
        self.db
            .insert(format!("superposition:{}", record_id), value)?;

        Ok(())
    }

    /// Get record by ID
    pub fn get(&self, id: &str) -> Result<Option<PhotonRecord>> {
        let key = format!("record:{}", id);
        match self.db.get(&key)? {
            Some(value) => {
                let record: PhotonRecord = bincode::deserialize(&value)?;
                Ok(Some(record))
            }
            None => Ok(None),
        }
    }

    /// Get superposition state
    pub fn get_superposition(&self, id: &str) -> Result<Option<SuperpositionStorage>> {
        // Check in-memory cache first
        if let Some(sup) = self.superposition_store.get(id) {
            return Ok(Some(sup.clone()));
        }

        // Check database
        let key = format!("superposition:{}", id);
        match self.db.get(&key)? {
            Some(value) => {
                let sup: SuperpositionStorage = bincode::deserialize(&value)?;
                Ok(Some(sup))
            }
            None => Ok(None),
        }
    }

    /// Perform quantum-inspired query
    pub fn query(&self, query: QuantumQuery) -> Result<Vec<PhotonRecord>> {
        // Start with spatial/temporal filtering
        let mut candidate_ids: Vec<String> =
            if let Some(((x, y, z), radius)) = query.spatial_constraint {
                self.spatial_index.query_range((x, y, z), radius)
            } else if let Some((start, end)) = query.temporal_constraint {
                self.temporal_index.query_range(start, end)
            } else {
                // No index constraints - scan all (inefficient)
                self.scan_all_ids()?
            };

        // Apply quantum superposition filtering if requested
        if let Some(sup_query) = query.superposition_query {
            candidate_ids = self.apply_superposition_filter(candidate_ids, &sup_query)?;
        }

        // Fetch and filter records
        let mut results = Vec::new();
        for id in candidate_ids {
            if let Some(record) = self.get(&id)? {
                // Apply attribute filters
                if self.matches_filters(&record, &query) {
                    results.push(record);
                }
            }
        }

        Ok(results)
    }

    /// Apply superposition filter using quantum measurement
    fn apply_superposition_filter(
        &self,
        ids: Vec<String>,
        query: &SuperpositionQuery,
    ) -> Result<Vec<String>> {
        let mut results = Vec::new();

        for id in ids {
            if let Some(sup) = self.get_superposition(&id)? {
                // Calculate measurement probabilities
                let mut state_probs: Vec<(String, f64)> = sup
                    .hypotheses
                    .iter()
                    .zip(sup.probabilities.iter())
                    .map(|(h, p)| (h.id.clone(), *p))
                    .collect();

                // Sort by probability
                state_probs.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

                if query.collapse {
                    // Return only most probable state
                    if let Some((hyp_id, prob)) = state_probs.first() {
                        if *prob > 0.5 {
                            results.push(hyp_id.clone());
                        }
                    }
                } else {
                    // Return top-k states
                    for (hyp_id, _) in state_probs.iter().take(query.top_k) {
                        results.push(hyp_id.clone());
                    }
                }
            }
        }

        Ok(results)
    }

    /// Check if record matches query filters
    fn matches_filters(&self, record: &PhotonRecord, query: &QuantumQuery) -> bool {
        // Polarization filter
        if let Some((min, max)) = query.polarization_filter {
            if record.polarization < min || record.polarization > max {
                return false;
            }
        }

        // Wavelength filter
        if let Some((min, max)) = query.wavelength_filter {
            if record.wavelength_nm < min || record.wavelength_nm > max {
                return false;
            }
        }

        // Intensity threshold
        if let Some(threshold) = query.intensity_threshold {
            if record.intensity < threshold {
                return false;
            }
        }

        true
    }

    /// Scan all record IDs (inefficient - for small databases only)
    fn scan_all_ids(&self) -> Result<Vec<String>> {
        let mut ids = Vec::new();
        for item in self.db.iter() {
            let (key, _) = item?;
            let key_str = String::from_utf8_lossy(&key);
            if key_str.starts_with("record:") {
                let id = key_str[7..].to_string();
                ids.push(id);
            }
        }
        Ok(ids)
    }

    /// Delete record
    pub fn delete(&mut self, id: &str) -> Result<()> {
        // Remove from database
        self.db.remove(format!("record:{}", id))?;
        self.db.remove(format!("superposition:{}", id))?;

        // Remove from superposition store
        self.superposition_store.remove(id);

        // Note: Removing from spatial/temporal indexes is complex
        // In production, use tombstone markers or periodic rebuilds

        Ok(())
    }

    /// Create entanglement between records
    pub fn entangle(&mut self, record_id1: &str, record_id2: &str) -> Result<()> {
        // Update superposition storage
        if let Some(mut sup1) = self.superposition_store.get(record_id1).cloned() {
            if !sup1.entangled_with.contains(&record_id2.to_string()) {
                sup1.entangled_with.push(record_id2.to_string());
                self.superposition_store
                    .insert(record_id1.to_string(), sup1);
            }
        }

        if let Some(mut sup2) = self.superposition_store.get(record_id2).cloned() {
            if !sup2.entangled_with.contains(&record_id1.to_string()) {
                sup2.entangled_with.push(record_id1.to_string());
                self.superposition_store
                    .insert(record_id2.to_string(), sup2);
            }
        }

        Ok(())
    }

    /// Measure record (collapse superposition)
    pub fn measure(&self, id: &str) -> Result<Option<(Hypothesis, f64)>> {
        if let Some(sup) = self.get_superposition(id)? {
            // Simulate measurement by sampling from probability distribution
            let mut rng = rand::thread_rng();
            let rand_val: f64 = rand::Rng::gen(&mut rng);

            let mut cumulative = 0.0;
            for (hypothesis, prob) in sup.hypotheses.iter().zip(sup.probabilities.iter()) {
                cumulative += prob;
                if rand_val <= cumulative {
                    return Ok(Some((hypothesis.clone(), *prob)));
                }
            }

            // Return most probable if none selected
            if let Some((hyp, prob)) = sup
                .hypotheses
                .into_iter()
                .zip(sup.probabilities.into_iter())
                .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
            {
                return Ok(Some((hyp, prob)));
            }
        }

        Ok(None)
    }

    /// Get database statistics
    pub fn stats(&self) -> Result<DatabaseStats> {
        let total_records = self.scan_all_ids()?.len();
        let total_superpositions = self.superposition_store.len();

        Ok(DatabaseStats {
            total_records,
            total_superpositions,
            spatial_cells: self.spatial_index.grid.len(),
            temporal_buckets: self.temporal_index.time_tree.len(),
        })
    }

    /// Flush all changes to disk
    pub fn flush(&self) -> Result<()> {
        self.db.flush()?;
        Ok(())
    }
}

/// Database statistics
#[derive(Debug, Clone)]
pub struct DatabaseStats {
    pub total_records: usize,
    pub total_superpositions: usize,
    pub spatial_cells: usize,
    pub temporal_buckets: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_spatial_index() {
        let mut index = SpatialIndex::new(1.0);

        index.insert("rec1".to_string(), (0.5, 0.5, 0.5));
        index.insert("rec2".to_string(), (2.0, 2.0, 2.0));
        index.insert("rec3".to_string(), (0.1, 0.1, 0.1));

        let results = index.query_range((0.0, 0.0, 0.0), 1.0);
        assert!(results.contains(&"rec1".to_string()));
        assert!(results.contains(&"rec3".to_string()));
        assert!(!results.contains(&"rec2".to_string()));
    }

    #[test]
    fn test_temporal_index() {
        let mut index = TemporalIndex::new(60);

        index.insert("rec1".to_string(), 30);
        index.insert("rec2".to_string(), 90);
        index.insert("rec3".to_string(), 150);

        let results = index.query_range(0, 120);
        assert!(results.contains(&"rec1".to_string()));
        assert!(results.contains(&"rec2".to_string()));
        assert!(!results.contains(&"rec3".to_string()));
    }

    #[test]
    fn test_photon_record() {
        let record = PhotonRecord {
            id: "test-1".to_string(),
            position: (1.0, 2.0, 3.0),
            timestamp: 1234567890,
            polarization: 45.0,
            intensity: 0.8,
            wavelength_nm: 650.0,
            phase: 90.0,
            quantum_state: None,
            confidence: 0.95,
            raw_data: vec![1, 2, 3],
        };

        assert_eq!(record.id, "test-1");
        assert_eq!(record.position, (1.0, 2.0, 3.0));
    }

    #[test]
    fn test_superposition_storage() {
        let sup = SuperpositionStorage {
            hypotheses: vec![
                Hypothesis {
                    id: "h1".to_string(),
                    description: "Hypothesis 1".to_string(),
                    weight: 0.6,
                    data: vec![1, 2, 3],
                },
                Hypothesis {
                    id: "h2".to_string(),
                    description: "Hypothesis 2".to_string(),
                    weight: 0.4,
                    data: vec![4, 5, 6],
                },
            ],
            entangled_with: vec![],
            probabilities: vec![0.6, 0.4],
        };

        assert_eq!(sup.hypotheses.len(), 2);
        assert_eq!(sup.probabilities.len(), 2);
    }

    #[test]
    fn test_photon_database() {
        let temp_dir = TempDir::new().unwrap();
        let mut db = PhotonDatabase::new(temp_dir.path().to_str().unwrap()).unwrap();

        let record = PhotonRecord {
            id: "test-1".to_string(),
            position: (1.0, 2.0, 3.0),
            timestamp: 1234567890,
            polarization: 45.0,
            intensity: 0.8,
            wavelength_nm: 650.0,
            phase: 90.0,
            quantum_state: None,
            confidence: 0.95,
            raw_data: vec![1, 2, 3],
        };

        db.insert(record.clone()).unwrap();

        let retrieved = db.get("test-1").unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id, "test-1");

        let stats = db.stats().unwrap();
        assert_eq!(stats.total_records, 1);
    }

    #[test]
    fn test_quantum_query() {
        let temp_dir = TempDir::new().unwrap();
        let mut db = PhotonDatabase::new(temp_dir.path().to_str().unwrap()).unwrap();

        // Insert test records
        for i in 0..10 {
            let record = PhotonRecord {
                id: format!("rec-{}", i),
                position: (i as f64, i as f64, i as f64),
                timestamp: i as u64 * 60,
                polarization: (i * 10) as f64,
                intensity: 0.5 + (i as f64 * 0.05),
                wavelength_nm: 600.0 + (i as f64 * 10.0),
                phase: 0.0,
                quantum_state: None,
                confidence: 0.9,
                raw_data: vec![],
            };
            db.insert(record).unwrap();
        }

        // Query by spatial range
        let query = QuantumQuery {
            spatial_constraint: Some(((2.0, 2.0, 2.0), 2.0)),
            temporal_constraint: None,
            polarization_filter: None,
            wavelength_filter: None,
            intensity_threshold: None,
            superposition_query: None,
        };

        let results = db.query(query).unwrap();
        assert!(!results.is_empty());

        // Query by polarization
        let query = QuantumQuery {
            spatial_constraint: None,
            temporal_constraint: None,
            polarization_filter: Some((20.0, 50.0)),
            wavelength_filter: None,
            intensity_threshold: None,
            superposition_query: None,
        };

        let results = db.query(query).unwrap();
        assert!(!results.is_empty());
    }
}

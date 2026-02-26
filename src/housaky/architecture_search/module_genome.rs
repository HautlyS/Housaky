//! Module Genome — Represent the entire system architecture as a searchable genome.
//!
//! Each `ArchitectureGenome` encodes a full configuration of cognitive modules,
//! their connections, and parameters. The genome can be mutated, crossed-over,
//! and evaluated for fitness — enabling neural-architecture-search (NAS) inspired
//! self-redesign.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

// ── Resource Budget ───────────────────────────────────────────────────────────

/// Computational budget allocated to a single module.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceBudget {
    pub max_memory_mb: u64,
    pub max_cpu_ms_per_call: u64,
    pub max_concurrent_tasks: usize,
    pub priority: u8,
}

impl Default for ResourceBudget {
    fn default() -> Self {
        Self {
            max_memory_mb: 256,
            max_cpu_ms_per_call: 5000,
            max_concurrent_tasks: 4,
            priority: 5,
        }
    }
}

// ── Module Type ───────────────────────────────────────────────────────────────

/// Broad functional category of a cognitive module.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ModuleType {
    Reasoning,
    Memory,
    Perception,
    Action,
    Meta,
    Planning,
    Learning,
    Communication,
    Alignment,
    Custom(String),
}

impl ModuleType {
    pub fn as_str(&self) -> &str {
        match self {
            ModuleType::Reasoning => "reasoning",
            ModuleType::Memory => "memory",
            ModuleType::Perception => "perception",
            ModuleType::Action => "action",
            ModuleType::Meta => "meta",
            ModuleType::Planning => "planning",
            ModuleType::Learning => "learning",
            ModuleType::Communication => "communication",
            ModuleType::Alignment => "alignment",
            ModuleType::Custom(s) => s.as_str(),
        }
    }
}

// ── Module Spec ───────────────────────────────────────────────────────────────

/// Specification for a single cognitive module in the architecture.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleSpec {
    pub id: String,
    pub name: String,
    pub module_type: ModuleType,
    /// Generated Rust source code for this module.
    pub rust_source: String,
    /// Which traits this module implements (e.g. `CognitiveModule`, `MemoryStore`).
    pub trait_implementations: Vec<String>,
    pub resource_budget: ResourceBudget,
    pub enabled: bool,
    pub version: u32,
    pub description: String,
    pub dependencies: Vec<String>, // module IDs this depends on
}

impl ModuleSpec {
    pub fn new(name: &str, module_type: ModuleType) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.to_string(),
            module_type,
            rust_source: String::new(),
            trait_implementations: Vec::new(),
            resource_budget: ResourceBudget::default(),
            enabled: true,
            version: 1,
            description: String::new(),
            dependencies: Vec::new(),
        }
    }
}

// ── Module Connection ─────────────────────────────────────────────────────────

/// A typed, weighted edge between two modules in the data-flow graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleConnection {
    pub id: String,
    pub from: String,  // module ID
    pub to: String,    // module ID
    pub data_type: String,
    pub bandwidth: f64,       // messages/second (soft limit)
    pub latency_budget_ms: u64,
    pub bidirectional: bool,
    pub weight: f64,          // how important this connection is (0.0–1.0)
}

impl ModuleConnection {
    pub fn new(from: &str, to: &str, data_type: &str) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            from: from.to_string(),
            to: to.to_string(),
            data_type: data_type.to_string(),
            bandwidth: 100.0,
            latency_budget_ms: 100,
            bidirectional: false,
            weight: 1.0,
        }
    }
}

// ── Architecture Fitness ──────────────────────────────────────────────────────

/// Multi-dimensional fitness score for an architecture.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ArchitectureFitness {
    pub overall: f64,
    pub reasoning_score: f64,
    pub memory_efficiency: f64,
    pub task_completion_rate: f64,
    pub latency_score: f64,
    pub alignment_score: f64,
    pub modularity_score: f64,
    pub resource_efficiency: f64,
    pub evaluated_at: Option<DateTime<Utc>>,
    pub benchmark_details: HashMap<String, f64>,
}

impl ArchitectureFitness {
    pub fn compute_overall(&mut self) {
        self.overall = self.reasoning_score * 0.25
            + self.memory_efficiency * 0.15
            + self.task_completion_rate * 0.25
            + self.latency_score * 0.10
            + self.alignment_score * 0.15
            + self.modularity_score * 0.05
            + self.resource_efficiency * 0.05;
        self.evaluated_at = Some(Utc::now());
    }

    pub fn is_better_than(&self, other: &ArchitectureFitness) -> bool {
        self.overall > other.overall
    }
}

// ── Architecture Genome ───────────────────────────────────────────────────────

/// The full genome encoding a candidate cognitive architecture.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchitectureGenome {
    pub id: String,
    pub modules: Vec<ModuleSpec>,
    pub connections: Vec<ModuleConnection>,
    pub parameters: HashMap<String, f64>,
    pub fitness: Option<ArchitectureFitness>,
    pub generation: u64,
    pub parent: Option<String>,  // parent genome ID
    pub lineage: Vec<String>,    // ancestry chain
    pub created_at: DateTime<Utc>,
    pub description: String,
    pub tags: Vec<String>,
}

impl ArchitectureGenome {
    pub fn new(generation: u64) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            modules: Vec::new(),
            connections: Vec::new(),
            parameters: HashMap::new(),
            fitness: None,
            generation,
            parent: None,
            lineage: Vec::new(),
            created_at: Utc::now(),
            description: String::new(),
            tags: Vec::new(),
        }
    }

    /// Add a module to this genome.
    pub fn add_module(&mut self, module: ModuleSpec) {
        self.modules.push(module);
    }

    /// Add a connection between modules.
    pub fn add_connection(&mut self, connection: ModuleConnection) {
        self.connections.push(connection);
    }

    /// Get a module by name.
    pub fn get_module(&self, name: &str) -> Option<&ModuleSpec> {
        self.modules.iter().find(|m| m.name == name)
    }

    /// Get a module by ID.
    pub fn get_module_by_id(&self, id: &str) -> Option<&ModuleSpec> {
        self.modules.iter().find(|m| m.id == id)
    }

    /// Count enabled modules.
    pub fn enabled_module_count(&self) -> usize {
        self.modules.iter().filter(|m| m.enabled).count()
    }

    /// Total resource budget across all enabled modules.
    pub fn total_memory_budget_mb(&self) -> u64 {
        self.modules
            .iter()
            .filter(|m| m.enabled)
            .map(|m| m.resource_budget.max_memory_mb)
            .sum()
    }

    /// Produce a child genome inheriting this genome's lineage.
    pub fn spawn_child(&self, generation: u64) -> ArchitectureGenome {
        let mut child = ArchitectureGenome::new(generation);
        child.parent = Some(self.id.clone());
        child.lineage = {
            let mut l = self.lineage.clone();
            l.push(self.id.clone());
            l
        };
        child.modules = self.modules.clone();
        child.connections = self.connections.clone();
        child.parameters = self.parameters.clone();
        child
    }

    /// Compute a simple structural hash for deduplication.
    pub fn structural_signature(&self) -> String {
        let mut names: Vec<String> = self.modules.iter().map(|m| m.name.clone()).collect();
        names.sort();
        names.join(",")
    }
}

// ── Genome Population ─────────────────────────────────────────────────────────

/// A population of architecture genomes for evolutionary search.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenomePopulation {
    pub genomes: Vec<ArchitectureGenome>,
    pub generation: u64,
    pub best_fitness: f64,
    pub best_genome_id: Option<String>,
    pub history: Vec<GenerationStats>,
}

/// Per-generation statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationStats {
    pub generation: u64,
    pub best_fitness: f64,
    pub mean_fitness: f64,
    pub population_size: usize,
    pub timestamp: DateTime<Utc>,
}

impl GenomePopulation {
    pub fn new() -> Self {
        Self {
            genomes: Vec::new(),
            generation: 0,
            best_fitness: 0.0,
            best_genome_id: None,
            history: Vec::new(),
        }
    }

    pub fn add(&mut self, genome: ArchitectureGenome) {
        self.genomes.push(genome);
    }

    pub fn best(&self) -> Option<&ArchitectureGenome> {
        self.genomes.iter().max_by(|a, b| {
            let fa = a.fitness.as_ref().map(|f| f.overall).unwrap_or(0.0);
            let fb = b.fitness.as_ref().map(|f| f.overall).unwrap_or(0.0);
            fa.partial_cmp(&fb).unwrap_or(std::cmp::Ordering::Equal)
        })
    }

    /// Update best-genome tracking after evaluation.
    pub fn refresh_best(&mut self) {
        let best_info = self.best().map(|b| {
            let fit = b.fitness.as_ref().map(|f| f.overall).unwrap_or(0.0);
            (fit, b.id.clone())
        });
        if let Some((fit, id)) = best_info {
            self.best_fitness = fit;
            self.best_genome_id = Some(id);
        }
    }

    /// Record a generation snapshot.
    pub fn record_generation(&mut self) {
        let fitnesses: Vec<f64> = self
            .genomes
            .iter()
            .filter_map(|g| g.fitness.as_ref().map(|f| f.overall))
            .collect();
        let mean = if fitnesses.is_empty() {
            0.0
        } else {
            fitnesses.iter().sum::<f64>() / fitnesses.len() as f64
        };
        self.history.push(GenerationStats {
            generation: self.generation,
            best_fitness: self.best_fitness,
            mean_fitness: mean,
            population_size: self.genomes.len(),
            timestamp: Utc::now(),
        });
        self.generation += 1;
    }
}

impl Default for GenomePopulation {
    fn default() -> Self {
        Self::new()
    }
}

//! Topology Search — NAS-inspired search over cognitive module architectures.
//!
//! Adapts Neural Architecture Search (NAS) techniques to cognitive module
//! topologies. Uses evolutionary search with crossover, mutation, and selection
//! to discover novel high-fitness architectures.

use crate::housaky::architecture_search::module_genome::{
    ArchitectureGenome, GenomePopulation, ModuleConnection, ModuleSpec,
    ModuleType, ResourceBudget,
};
use crate::housaky::architecture_search::data_flow_graph::DataFlowGraph;
use rand::Rng;
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

// ── Search Config ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopologySearchConfig {
    pub population_size: usize,
    pub max_generations: usize,
    pub mutation_rate: f64,
    pub crossover_rate: f64,
    pub elite_fraction: f64,
    pub min_modules: usize,
    pub max_modules: usize,
    pub max_connections_per_module: usize,
    pub allow_new_module_types: bool,
    pub forbidden_module_names: Vec<String>,
}

impl Default for TopologySearchConfig {
    fn default() -> Self {
        Self {
            population_size: 20,
            max_generations: 50,
            mutation_rate: 0.15,
            crossover_rate: 0.70,
            elite_fraction: 0.10,
            min_modules: 3,
            max_modules: 20,
            max_connections_per_module: 5,
            allow_new_module_types: true,
            forbidden_module_names: vec![
                "security".to_string(),
                "alignment".to_string(),
                "alignment_proof".to_string(),
            ],
        }
    }
}

// ── Mutation Operator ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TopologyMutation {
    AddModule { spec: ModuleSpec },
    RemoveModule { module_id: String },
    AddConnection { connection: ModuleConnection },
    RemoveConnection { connection_id: String },
    ModifyConnectionWeight { connection_id: String, new_weight: f64 },
    ModifyResourceBudget { module_id: String, budget: ResourceBudget },
    EnableModule { module_id: String },
    DisableModule { module_id: String },
    SwapModuleType { module_id: String, new_type: ModuleType },
}

// ── Crossover Result ──────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct CrossoverResult {
    pub child1: ArchitectureGenome,
    pub child2: ArchitectureGenome,
}

// ── Topology Searcher ─────────────────────────────────────────────────────────

pub struct TopologySearcher {
    pub config: TopologySearchConfig,
}

impl TopologySearcher {
    pub fn new(config: TopologySearchConfig) -> Self {
        Self { config }
    }

    pub fn with_defaults() -> Self {
        Self::new(TopologySearchConfig::default())
    }

    /// Seed the initial population from the current (baseline) architecture.
    pub fn seed_population(
        &self,
        baseline: &ArchitectureGenome,
    ) -> GenomePopulation {
        let mut pop = GenomePopulation::new();
        pop.add(baseline.clone());

        let mut rng = rand::thread_rng();
        for _ in 1..self.config.population_size {
            let mut candidate = baseline.spawn_child(0);
            let n_mutations = rng.gen_range(1..=3);
            for mutation in self.random_mutations(&candidate, n_mutations, &mut rng) {
                self.apply_mutation(&mut candidate, mutation);
            }
            pop.add(candidate);
        }
        pop
    }

    /// Apply a single topology mutation to a genome.
    pub fn apply_mutation(&self, genome: &mut ArchitectureGenome, mutation: TopologyMutation) {
        match mutation {
            TopologyMutation::AddModule { spec } => {
                if !self
                    .config
                    .forbidden_module_names
                    .iter()
                    .any(|f| spec.name.contains(f.as_str()))
                {
                    genome.add_module(spec);
                }
            }
            TopologyMutation::RemoveModule { module_id } => {
                genome.modules.retain(|m| m.id != module_id);
                genome
                    .connections
                    .retain(|c| c.from != module_id && c.to != module_id);
            }
            TopologyMutation::AddConnection { connection } => {
                let has_from = genome.modules.iter().any(|m| m.id == connection.from);
                let has_to = genome.modules.iter().any(|m| m.id == connection.to);
                if has_from && has_to {
                    genome.add_connection(connection);
                }
            }
            TopologyMutation::RemoveConnection { connection_id } => {
                genome.connections.retain(|c| c.id != connection_id);
            }
            TopologyMutation::ModifyConnectionWeight {
                connection_id,
                new_weight,
            } => {
                if let Some(c) = genome.connections.iter_mut().find(|c| c.id == connection_id) {
                    c.weight = new_weight.clamp(0.0, 1.0);
                }
            }
            TopologyMutation::ModifyResourceBudget { module_id, budget } => {
                if let Some(m) = genome.modules.iter_mut().find(|m| m.id == module_id) {
                    m.resource_budget = budget;
                }
            }
            TopologyMutation::EnableModule { module_id } => {
                if let Some(m) = genome.modules.iter_mut().find(|m| m.id == module_id) {
                    m.enabled = true;
                }
            }
            TopologyMutation::DisableModule { module_id } => {
                let is_core = genome
                    .modules
                    .iter()
                    .find(|m| m.id == module_id)
                    .map(|m| {
                        matches!(m.module_type, ModuleType::Alignment)
                            || self
                                .config
                                .forbidden_module_names
                                .iter()
                                .any(|f| m.name.contains(f.as_str()))
                    })
                    .unwrap_or(false);
                if !is_core {
                    if let Some(m) = genome.modules.iter_mut().find(|m| m.id == module_id) {
                        m.enabled = false;
                    }
                }
            }
            TopologyMutation::SwapModuleType { module_id, new_type } => {
                if !matches!(new_type, ModuleType::Alignment) {
                    if let Some(m) = genome.modules.iter_mut().find(|m| m.id == module_id) {
                        m.module_type = new_type;
                    }
                }
            }
        }
    }

    /// Uniform crossover: randomly select modules from each parent.
    pub fn crossover(
        &self,
        parent1: &ArchitectureGenome,
        parent2: &ArchitectureGenome,
        generation: u64,
    ) -> CrossoverResult {
        let mut rng = rand::thread_rng();

        let mut child1 = parent1.spawn_child(generation);
        let mut child2 = parent2.spawn_child(generation);

        // Swap modules at crossover points
        let len = parent1.modules.len().min(parent2.modules.len());
        for i in 0..len {
            if rng.gen_bool(0.5) {
                if i < child1.modules.len() && i < parent2.modules.len() {
                    child1.modules[i] = parent2.modules[i].clone();
                }
                if i < child2.modules.len() && i < parent1.modules.len() {
                    child2.modules[i] = parent1.modules[i].clone();
                }
            }
        }

        // Inherit parameter maps from both parents
        for (k, v) in &parent2.parameters {
            if rng.gen_bool(0.5) {
                child1.parameters.insert(k.clone(), *v);
            }
        }
        for (k, v) in &parent1.parameters {
            if rng.gen_bool(0.5) {
                child2.parameters.insert(k.clone(), *v);
            }
        }

        CrossoverResult { child1, child2 }
    }

    /// Validate that a genome is structurally sound (no cycles, enough modules).
    pub fn validate(&self, genome: &ArchitectureGenome) -> ValidationResult {
        let graph = DataFlowGraph::build(&genome.modules, &genome.connections);
        let summary = graph.summary();

        let mut issues = Vec::new();
        let enabled = genome.enabled_module_count();

        if enabled < self.config.min_modules {
            issues.push(format!(
                "Too few enabled modules: {} < {}",
                enabled, self.config.min_modules
            ));
        }
        if enabled > self.config.max_modules {
            issues.push(format!(
                "Too many enabled modules: {} > {}",
                enabled, self.config.max_modules
            ));
        }
        if summary.has_cycles {
            issues.push("Architecture contains cycles in the data flow graph".to_string());
        }
        if summary.isolated_modules > enabled / 2 {
            issues.push(format!(
                "Too many isolated modules: {}",
                summary.isolated_modules
            ));
        }

        ValidationResult {
            valid: issues.is_empty(),
            issues,
            graph_summary: summary,
        }
    }

    /// Generate N random mutations for a genome.
    fn random_mutations(
        &self,
        genome: &ArchitectureGenome,
        n: usize,
        rng: &mut impl Rng,
    ) -> Vec<TopologyMutation> {
        let mut mutations = Vec::new();
        for _ in 0..n {
            let op = rng.gen_range(0..5_u8);
            match op {
                0 => {
                    // Add a new generic module
                    let name = format!("gen_module_{}", uuid::Uuid::new_v4().to_string()[..8].to_string());
                    let types = [
                        ModuleType::Reasoning,
                        ModuleType::Memory,
                        ModuleType::Learning,
                        ModuleType::Meta,
                    ];
                    let mt = types[rng.gen_range(0..types.len())].clone();
                    mutations.push(TopologyMutation::AddModule {
                        spec: ModuleSpec::new(&name, mt),
                    });
                }
                1 => {
                    if !genome.modules.is_empty() {
                        let idx = rng.gen_range(0..genome.modules.len());
                        mutations.push(TopologyMutation::RemoveModule {
                            module_id: genome.modules[idx].id.clone(),
                        });
                    }
                }
                2 => {
                    if genome.modules.len() >= 2 {
                        let a = rng.gen_range(0..genome.modules.len());
                        let mut b = rng.gen_range(0..genome.modules.len());
                        while b == a {
                            b = rng.gen_range(0..genome.modules.len());
                        }
                        mutations.push(TopologyMutation::AddConnection {
                            connection: ModuleConnection::new(
                                &genome.modules[a].id,
                                &genome.modules[b].id,
                                "CognitiveContent",
                            ),
                        });
                    }
                }
                3 => {
                    if !genome.connections.is_empty() {
                        let idx = rng.gen_range(0..genome.connections.len());
                        mutations.push(TopologyMutation::RemoveConnection {
                            connection_id: genome.connections[idx].id.clone(),
                        });
                    }
                }
                _ => {
                    if !genome.connections.is_empty() {
                        let idx = rng.gen_range(0..genome.connections.len());
                        let new_w: f64 = rng.gen_range(0.1..1.0);
                        mutations.push(TopologyMutation::ModifyConnectionWeight {
                            connection_id: genome.connections[idx].id.clone(),
                            new_weight: new_w,
                        });
                    }
                }
            }
        }
        mutations
    }

    /// Run one generation of evolutionary search (selection + crossover + mutation).
    /// Returns the new population. Fitness must already be set on the input population.
    pub fn evolve(&self, population: &GenomePopulation) -> GenomePopulation {
        let mut rng = rand::thread_rng();
        let mut scored: Vec<(f64, &ArchitectureGenome)> = population
            .genomes
            .iter()
            .map(|g| (g.fitness.as_ref().map(|f| f.overall).unwrap_or(0.0), g))
            .collect();
        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        let elite_n = ((self.config.elite_fraction * scored.len() as f64).ceil() as usize).max(1);
        let mut next_gen = GenomePopulation::new();
        next_gen.generation = population.generation + 1;

        // Elitism: carry forward best genomes unchanged
        for (_, g) in scored.iter().take(elite_n) {
            let mut elite = (*g).clone();
            elite.generation = next_gen.generation;
            next_gen.add(elite);
        }

        // Fill remainder via crossover + mutation
        let total = self.config.population_size;
        let parents: Vec<&ArchitectureGenome> = scored.iter().map(|(_, g)| *g).collect();

        while next_gen.genomes.len() < total {
            let p1_idx = rng.gen_range(0..parents.len().min(10));
            let mut p2_idx = rng.gen_range(0..parents.len().min(10));
            while p2_idx == p1_idx && parents.len() > 1 {
                p2_idx = rng.gen_range(0..parents.len().min(10));
            }

            let mut child = if rng.gen_bool(self.config.crossover_rate) {
                let res = self.crossover(parents[p1_idx], parents[p2_idx], next_gen.generation);
                res.child1
            } else {
                parents[p1_idx].spawn_child(next_gen.generation)
            };

            if rng.gen_bool(self.config.mutation_rate) {
                let n_mut = rng.gen_range(1..=3);
                for m in self.random_mutations(&child.clone(), n_mut, &mut rng) {
                    self.apply_mutation(&mut child, m);
                }
            }

            let validation = self.validate(&child);
            if validation.valid {
                next_gen.add(child);
            } else {
                debug!("Rejected invalid genome: {:?}", validation.issues);
            }
        }

        info!(
            "Evolved generation {} → {} genomes",
            next_gen.generation,
            next_gen.genomes.len()
        );
        next_gen
    }
}

// ── Validation Result ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub valid: bool,
    pub issues: Vec<String>,
    pub graph_summary: crate::housaky::architecture_search::data_flow_graph::DataFlowSummary,
}

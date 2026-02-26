use super::annealer::{AnnealingConfig, AnnealingResult, IsingModel, QuantumAnnealer};
use super::backend::{AmazonBraketBackend, QuantumBackend, QuantumConfig, SimulatorBackend};
use super::grover::{GroverConfig, GroverResult, GroverSearch, SearchProblem};
use super::optimizer::{OptimizationProblem, OptimizationResult, QAOAConfig, QAOAOptimizer};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SolverStrategy {
    FullClassical,
    FullQuantum,
    HybridQAOA,
    HybridAnnealing,
    HybridGrover,
    Adaptive,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HybridProblem {
    pub problem_id: String,
    pub problem_type: HybridProblemType,
    pub size: usize,
    pub classical_part: Option<ClassicalSubproblem>,
    pub quantum_part: Option<QuantumSubproblem>,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum HybridProblemType {
    GoalScheduling,
    KnowledgeGraphInference,
    ReasoningBranchSelection,
    ParameterOptimization,
    ToolSelection,
    GeneralOptimization,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassicalSubproblem {
    pub description: String,
    pub precomputed_values: HashMap<String, f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuantumSubproblem {
    pub description: String,
    pub variables: Vec<String>,
    pub objective: HashMap<String, f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HybridSolution {
    Optimization(OptimizationResult),
    Annealing(AnnealingResult),
    Search(GroverResult),
    Classical(ClassicalSolution),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassicalSolution {
    pub solution: Vec<bool>,
    pub value: f64,
    pub labels: HashMap<String, bool>,
    pub runtime_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HybridResult {
    pub problem_id: String,
    pub strategy_used: SolverStrategy,
    pub solution: HybridSolution,
    pub quantum_fraction: f64,
    pub classical_fraction: f64,
    pub total_runtime_ms: u64,
    pub quantum_advantage_achieved: bool,
}

pub struct HybridSolver {
    pub backend: Arc<dyn QuantumBackend>,
    pub threshold: usize,
    pub qaoa_config: QAOAConfig,
    pub annealing_config: AnnealingConfig,
    pub grover_config: GroverConfig,
}

impl HybridSolver {
    pub fn new(backend: Arc<dyn QuantumBackend>, threshold: usize) -> Self {
        Self {
            backend,
            threshold,
            qaoa_config: QAOAConfig::default(),
            annealing_config: AnnealingConfig::default(),
            grover_config: GroverConfig::default(),
        }
    }

    /// Construct a `HybridSolver` from a `QuantumConfig`.
    ///
    /// - `backend = "braket"` → uses `AmazonBraketBackend` (requires `braket_s3_bucket`)
    /// - anything else → uses local `SimulatorBackend`
    pub async fn from_config(cfg: &QuantumConfig) -> Result<Self> {
        let backend: Arc<dyn QuantumBackend> = if cfg.backend == "braket" {
            Arc::new(AmazonBraketBackend::from_config(cfg).await?)
        } else {
            Arc::new(SimulatorBackend::new(cfg.max_qubits, cfg.shots))
        };
        Ok(Self::new(backend, cfg.hybrid_threshold))
    }

    pub async fn solve(&self, problem: &HybridProblem) -> Result<HybridResult> {
        let start = std::time::Instant::now();
        let strategy = self.select_strategy(problem);

        let (solution, quantum_fraction) = match &strategy {
            SolverStrategy::HybridQAOA => {
                let result = self.solve_with_qaoa(problem).await?;
                (HybridSolution::Optimization(result), 0.7)
            }
            SolverStrategy::HybridAnnealing => {
                let result = self.solve_with_annealing(problem).await?;
                (HybridSolution::Annealing(result), 0.8)
            }
            SolverStrategy::HybridGrover => {
                let result = self.solve_with_grover(problem).await?;
                (HybridSolution::Search(result), 0.6)
            }
            SolverStrategy::FullClassical | SolverStrategy::Adaptive => {
                let result = self.solve_classical(problem);
                (HybridSolution::Classical(result), 0.0)
            }
            SolverStrategy::FullQuantum => {
                let result = self.solve_with_qaoa(problem).await?;
                (HybridSolution::Optimization(result), 1.0)
            }
        };

        let quantum_advantage = self.assess_quantum_advantage(&solution, problem.size);

        Ok(HybridResult {
            problem_id: problem.problem_id.clone(),
            strategy_used: strategy,
            solution,
            quantum_fraction,
            classical_fraction: 1.0 - quantum_fraction,
            total_runtime_ms: start.elapsed().as_millis() as u64,
            quantum_advantage_achieved: quantum_advantage,
        })
    }

    fn select_strategy(&self, problem: &HybridProblem) -> SolverStrategy {
        let max_q = self.backend.max_qubits();

        match problem.problem_type {
            HybridProblemType::KnowledgeGraphInference => {
                if problem.size <= max_q {
                    SolverStrategy::HybridAnnealing
                } else {
                    SolverStrategy::FullClassical
                }
            }
            HybridProblemType::ReasoningBranchSelection => {
                if problem.size <= max_q {
                    SolverStrategy::HybridGrover
                } else {
                    SolverStrategy::FullClassical
                }
            }
            HybridProblemType::GoalScheduling | HybridProblemType::ParameterOptimization => {
                if problem.size > self.threshold && problem.size <= max_q {
                    SolverStrategy::HybridQAOA
                } else if problem.size <= max_q {
                    SolverStrategy::HybridQAOA
                } else {
                    SolverStrategy::FullClassical
                }
            }
            HybridProblemType::ToolSelection => {
                if problem.size <= max_q {
                    SolverStrategy::HybridGrover
                } else {
                    SolverStrategy::FullClassical
                }
            }
            HybridProblemType::GeneralOptimization => {
                if problem.size <= max_q {
                    SolverStrategy::HybridQAOA
                } else {
                    SolverStrategy::FullClassical
                }
            }
        }
    }

    async fn solve_with_qaoa(&self, problem: &HybridProblem) -> Result<OptimizationResult> {
        let opt_problem = self.to_optimization_problem(problem);
        let optimizer = QAOAOptimizer::new(self.backend.clone(), self.qaoa_config.clone());
        optimizer.solve(&opt_problem).await
    }

    async fn solve_with_annealing(&self, problem: &HybridProblem) -> Result<AnnealingResult> {
        let mut model = IsingModel::new(problem.size);
        if let Some(qpart) = &problem.quantum_part {
            for (i, var) in qpart.variables.iter().enumerate() {
                let coeff = qpart.objective.get(var).copied().unwrap_or(0.0);
                model.add_linear(i, -coeff);
            }
            let n = qpart.variables.len();
            for i in 0..n.saturating_sub(1) {
                model.add_quadratic(i, i + 1, -0.1);
            }
        }
        let annealer = QuantumAnnealer::new(self.annealing_config.clone());
        annealer.anneal(&model).await
    }

    async fn solve_with_grover(&self, problem: &HybridProblem) -> Result<GroverResult> {
        let searcher = GroverSearch::new(self.backend.clone(), self.grover_config.clone());
        let items: Vec<String> = if let Some(qpart) = &problem.quantum_part {
            qpart.variables.clone()
        } else {
            (0..problem.size).map(|i| format!("item_{}", i)).collect()
        };
        let targets: Vec<String> = if let Some(qpart) = &problem.quantum_part {
            qpart
                .objective
                .iter()
                .filter(|(_, &v)| v > 0.5)
                .filter_map(|(k, _)| items.iter().find(|i| *i == k).cloned())
                .collect()
        } else {
            vec![items[0].clone()]
        };
        let search_problem = SearchProblem {
            search_space_size: items.len(),
            oracle_description: problem.problem_id.clone(),
            target_items: targets,
            items,
        };
        searcher.search(&search_problem).await
    }

    fn solve_classical(&self, problem: &HybridProblem) -> ClassicalSolution {
        let start = std::time::Instant::now();
        let vars: Vec<String> = if let Some(qpart) = &problem.quantum_part {
            qpart.variables.clone()
        } else {
            (0..problem.size).map(|i| format!("x{}", i)).collect()
        };
        let objective: HashMap<String, f64> = if let Some(qpart) = &problem.quantum_part {
            qpart.objective.clone()
        } else {
            HashMap::new()
        };

        let solution: Vec<bool> = vars
            .iter()
            .map(|v| objective.get(v).copied().unwrap_or(0.0) > 0.0)
            .collect();

        let value: f64 = solution
            .iter()
            .zip(vars.iter())
            .filter(|(&b, _)| b)
            .map(|(_, v)| objective.get(v).copied().unwrap_or(0.0))
            .sum();

        let labels: HashMap<String, bool> =
            vars.iter().cloned().zip(solution.iter().cloned()).collect();

        ClassicalSolution {
            solution,
            value,
            labels,
            runtime_ms: start.elapsed().as_millis() as u64,
        }
    }

    fn to_optimization_problem(&self, problem: &HybridProblem) -> OptimizationProblem {
        let (variables, objective) = if let Some(qpart) = &problem.quantum_part {
            (qpart.variables.clone(), qpart.objective.clone())
        } else {
            let vars: Vec<String> = (0..problem.size).map(|i| format!("x{}", i)).collect();
            let obj: HashMap<String, f64> = vars.iter().map(|v| (v.clone(), 1.0)).collect();
            (vars, obj)
        };

        OptimizationProblem {
            problem_type: super::optimizer::ProblemType::GoalScheduling,
            variables,
            objective,
            constraints: vec![],
            metadata: problem.metadata.clone(),
        }
    }

    fn assess_quantum_advantage(&self, solution: &HybridSolution, size: usize) -> bool {
        match solution {
            HybridSolution::Optimization(r) => r.quantum_advantage_estimate > 2.0,
            HybridSolution::Search(r) => r.theoretical_speedup > 1.5 && size > 16,
            HybridSolution::Annealing(_) => size > 10,
            HybridSolution::Classical(_) => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::quantum::backend::SimulatorBackend;

    #[tokio::test]
    async fn test_hybrid_solver_goal_scheduling() {
        let backend = Arc::new(SimulatorBackend::new(8, 512));
        let solver = HybridSolver::new(backend, 4);

        let mut objective = HashMap::new();
        objective.insert("goal_a".to_string(), 1.0);
        objective.insert("goal_b".to_string(), 2.0);
        objective.insert("goal_c".to_string(), 0.5);

        let problem = HybridProblem {
            problem_id: "test-scheduling".to_string(),
            problem_type: HybridProblemType::GoalScheduling,
            size: 3,
            classical_part: None,
            quantum_part: Some(QuantumSubproblem {
                description: "schedule 3 goals".to_string(),
                variables: vec!["goal_a".to_string(), "goal_b".to_string(), "goal_c".to_string()],
                objective,
            }),
            metadata: HashMap::new(),
        };

        let result = solver.solve(&problem).await.unwrap();
        assert_eq!(result.problem_id, "test-scheduling");
        assert!(result.total_runtime_ms < 10_000);
    }

    #[tokio::test]
    async fn test_hybrid_classical_fallback() {
        let backend = Arc::new(SimulatorBackend::new(4, 256));
        let solver = HybridSolver::new(backend, 100);

        let problem = HybridProblem {
            problem_id: "classical-test".to_string(),
            problem_type: HybridProblemType::GoalScheduling,
            size: 200,
            classical_part: None,
            quantum_part: None,
            metadata: HashMap::new(),
        };

        let result = solver.solve(&problem).await.unwrap();
        assert!(matches!(result.solution, HybridSolution::Classical(_)));
        assert!((result.classical_fraction - 1.0).abs() < 1e-9);
    }
}

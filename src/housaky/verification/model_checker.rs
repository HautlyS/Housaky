//! Model Checker — Bounded model checking for state machines.
//!
//! Provides bounded model checking (BMC) for cognitive module state machines.
//! Checks reachability, safety properties, and liveness properties within a
//! bounded number of steps. Designed to integrate with kani/prusti in future.

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use tracing::info;
use uuid::Uuid;

// ── State Machine ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct StateId(pub String);

impl StateId {
    pub fn new(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl std::fmt::Display for StateId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct State {
    pub id: StateId,
    pub label: String,
    pub is_initial: bool,
    pub is_accepting: bool,
    pub is_error: bool,
    pub invariants: Vec<String>, // invariant IDs that must hold in this state
    pub variables: HashMap<String, StateValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StateValue {
    Bool(bool),
    Int(i64),
    Float(f64),
    Text(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transition {
    pub id: String,
    pub from: StateId,
    pub to: StateId,
    pub label: String,
    pub guard: Option<Guard>,
    pub action: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Guard {
    pub expression: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateMachine {
    pub id: String,
    pub name: String,
    pub module_name: String,
    pub states: HashMap<StateId, State>,
    pub transitions: Vec<Transition>,
    pub initial_state: StateId,
}

impl StateMachine {
    pub fn new(name: &str, module_name: &str) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.to_string(),
            module_name: module_name.to_string(),
            states: HashMap::new(),
            transitions: Vec::new(),
            initial_state: StateId::new("init"),
        }
    }

    pub fn add_state(&mut self, state: State) {
        self.states.insert(state.id.clone(), state);
    }

    pub fn add_transition(&mut self, transition: Transition) {
        self.transitions.push(transition);
    }

    pub fn successors(&self, state_id: &StateId) -> Vec<&StateId> {
        self.transitions
            .iter()
            .filter(|t| &t.from == state_id)
            .map(|t| &t.to)
            .collect()
    }

    pub fn initial_state(&self) -> Option<&State> {
        self.states.get(&self.initial_state)
    }
}

// ── Model Checking Properties ─────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ModelProperty {
    /// Safety: a bad state is never reached.
    Safety { bad_state: StateId },
    /// Reachability: a target state is reachable from the initial state.
    Reachability { target_state: StateId },
    /// Deadlock freedom: no state with no outgoing transitions (except accepting states).
    DeadlockFreedom,
    /// Invariant: a predicate holds in all reachable states.
    Invariant { predicate: String },
    /// Liveness (bounded): a target state is reached within `max_steps`.
    BoundedLiveness { target_state: StateId, max_steps: usize },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckProperty {
    pub id: String,
    pub name: String,
    pub property: ModelProperty,
    pub critical: bool,
}

impl CheckProperty {
    pub fn safety(name: &str, bad_state: StateId) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.to_string(),
            property: ModelProperty::Safety { bad_state },
            critical: true,
        }
    }

    pub fn reachability(name: &str, target: StateId) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.to_string(),
            property: ModelProperty::Reachability { target_state: target },
            critical: false,
        }
    }
}

// ── Check Result ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Verdict {
    Holds,
    Violated { counterexample: Vec<StateId> },
    Unknown { reason: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropertyCheckOutcome {
    pub property_id: String,
    pub property_name: String,
    pub verdict: Verdict,
    pub states_explored: usize,
    pub check_duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelCheckReport {
    pub machine_id: String,
    pub machine_name: String,
    pub outcomes: Vec<PropertyCheckOutcome>,
    pub all_critical_hold: bool,
    pub states_explored: usize,
    pub checked_at: DateTime<Utc>,
    pub bound: usize,
}

// ── Model Checker ─────────────────────────────────────────────────────────────

pub struct ModelChecker {
    pub bound: usize,
}

impl ModelChecker {
    pub fn new(bound: usize) -> Self {
        Self { bound }
    }

    /// Check all properties against the given state machine within `self.bound` steps.
    pub fn check(
        &self,
        machine: &StateMachine,
        properties: &[CheckProperty],
    ) -> Result<ModelCheckReport> {
        let mut outcomes = Vec::new();
        let mut total_explored = 0usize;

        for prop in properties {
            let start = std::time::Instant::now();
            let (verdict, explored) = self.check_property(machine, &prop.property);
            total_explored = total_explored.max(explored);
            let elapsed = start.elapsed().as_millis() as u64;

            info!(
                property = %prop.name,
                verdict = ?verdict,
                states = %explored,
                "Model check result"
            );

            outcomes.push(PropertyCheckOutcome {
                property_id: prop.id.clone(),
                property_name: prop.name.clone(),
                verdict,
                states_explored: explored,
                check_duration_ms: elapsed,
            });
        }

        let all_critical = outcomes.iter().all(|o| {
            let prop = properties.iter().find(|p| p.id == o.property_id);
            match prop {
                Some(p) if p.critical => matches!(o.verdict, Verdict::Holds),
                _ => true,
            }
        });

        Ok(ModelCheckReport {
            machine_id: machine.id.clone(),
            machine_name: machine.name.clone(),
            outcomes,
            all_critical_hold: all_critical,
            states_explored: total_explored,
            checked_at: Utc::now(),
            bound: self.bound,
        })
    }

    fn check_property(
        &self,
        machine: &StateMachine,
        property: &ModelProperty,
    ) -> (Verdict, usize) {
        match property {
            ModelProperty::Safety { bad_state } => {
                self.bfs_safety(machine, bad_state)
            }
            ModelProperty::Reachability { target_state } => {
                self.bfs_reachability(machine, target_state)
            }
            ModelProperty::DeadlockFreedom => {
                self.check_deadlock_freedom(machine)
            }
            ModelProperty::BoundedLiveness { target_state, max_steps } => {
                self.bfs_bounded_liveness(machine, target_state, *max_steps)
            }
            ModelProperty::Invariant { predicate } => {
                // Invariant predicates are syntactic; flag as unknown until an
                // external solver (kani/prusti) is integrated.
                (
                    Verdict::Unknown {
                        reason: format!(
                            "Invariant '{}' requires external solver integration",
                            predicate
                        ),
                    },
                    0,
                )
            }
        }
    }

    /// BFS safety check: return Violated if bad_state is reachable, else Holds.
    fn bfs_safety(&self, machine: &StateMachine, bad_state: &StateId) -> (Verdict, usize) {
        let mut visited: HashSet<StateId> = HashSet::new();
        let mut queue: VecDeque<(StateId, Vec<StateId>)> = VecDeque::new();
        let mut steps = 0;

        queue.push_back((machine.initial_state.clone(), vec![machine.initial_state.clone()]));

        while let Some((current, path)) = queue.pop_front() {
            if visited.contains(&current) {
                continue;
            }
            visited.insert(current.clone());
            steps += 1;

            if &current == bad_state {
                return (Verdict::Violated { counterexample: path }, steps);
            }

            if steps >= self.bound {
                break;
            }

            for next in machine.successors(&current) {
                if !visited.contains(next) {
                    let mut new_path = path.clone();
                    new_path.push(next.clone());
                    queue.push_back((next.clone(), new_path));
                }
            }
        }

        (Verdict::Holds, steps)
    }

    /// BFS reachability: return Holds if target is reachable within bound.
    fn bfs_reachability(
        &self,
        machine: &StateMachine,
        target: &StateId,
    ) -> (Verdict, usize) {
        let mut visited: HashSet<StateId> = HashSet::new();
        let mut queue: VecDeque<StateId> = VecDeque::new();
        let mut steps = 0;

        queue.push_back(machine.initial_state.clone());

        while let Some(current) = queue.pop_front() {
            if visited.contains(&current) {
                continue;
            }
            visited.insert(current.clone());
            steps += 1;

            if &current == target {
                return (Verdict::Holds, steps);
            }

            if steps >= self.bound {
                break;
            }

            for next in machine.successors(&current) {
                if !visited.contains(next) {
                    queue.push_back(next.clone());
                }
            }
        }

        (
            Verdict::Violated {
                counterexample: vec![machine.initial_state.clone()],
            },
            steps,
        )
    }

    /// Check that no non-accepting state has zero outgoing transitions.
    fn check_deadlock_freedom(&self, machine: &StateMachine) -> (Verdict, usize) {
        let mut visited: HashSet<StateId> = HashSet::new();
        let mut queue: VecDeque<StateId> = VecDeque::new();
        queue.push_back(machine.initial_state.clone());
        let mut steps = 0;

        while let Some(current) = queue.pop_front() {
            if visited.contains(&current) {
                continue;
            }
            visited.insert(current.clone());
            steps += 1;

            let state = machine.states.get(&current);
            let is_accepting = state.map(|s| s.is_accepting).unwrap_or(false);
            let successors = machine.successors(&current);

            if successors.is_empty() && !is_accepting {
                return (
                    Verdict::Violated {
                        counterexample: vec![current],
                    },
                    steps,
                );
            }

            for next in successors {
                if !visited.contains(next) {
                    queue.push_back(next.clone());
                }
            }
        }

        (Verdict::Holds, steps)
    }

    fn bfs_bounded_liveness(
        &self,
        machine: &StateMachine,
        target: &StateId,
        max_steps: usize,
    ) -> (Verdict, usize) {
        let bound = max_steps.min(self.bound);
        let mut visited: HashSet<StateId> = HashSet::new();
        let mut queue: VecDeque<(StateId, usize)> = VecDeque::new();
        queue.push_back((machine.initial_state.clone(), 0));
        let mut total = 0;

        while let Some((current, depth)) = queue.pop_front() {
            if visited.contains(&current) || depth > bound {
                continue;
            }
            visited.insert(current.clone());
            total += 1;

            if &current == target {
                return (Verdict::Holds, total);
            }

            for next in machine.successors(&current) {
                if !visited.contains(next) {
                    queue.push_back((next.clone(), depth + 1));
                }
            }
        }

        (
            Verdict::Violated {
                counterexample: vec![machine.initial_state.clone()],
            },
            total,
        )
    }
}

impl Default for ModelChecker {
    fn default() -> Self {
        Self::new(10_000)
    }
}

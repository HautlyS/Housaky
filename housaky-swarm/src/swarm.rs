//! Swarm Intelligence - Distributed Multi-Agent System
//! Based on 2025-2026 research on collective intelligence

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::mpsc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agent {
    pub id: String,
    pub position: [f64; 3],
    pub velocity: [f64; 3],
    pub fitness: f64,
    pub personal_best: [f64; 3],
    pub personal_best_fitness: f64,
    pub agent_type: AgentType,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AgentType {
    Explorer,   // High exploration, low exploitation
    Exploiter,  // Low exploration, high exploitation
    Balanced,   // Balanced exploration/exploitation
    Specialist, // Domain-specific agent
}

#[derive(Debug, Clone)]
pub struct SwarmMessage {
    pub from: String,
    pub to: Option<String>, // None = broadcast
    pub content: MessageContent,
}

#[derive(Debug, Clone)]
pub enum MessageContent {
    PositionUpdate([f64; 3], f64), // position, fitness
    BestFound([f64; 3], f64),
    Query(String),
    Response(String),
    Consensus(Vec<u8>),
}

pub struct SwarmIntelligence {
    agents: HashMap<String, Agent>,
    global_best: [f64; 3],
    global_best_fitness: f64,
    message_queue: Vec<SwarmMessage>,
    inertia: f64,
    cognitive_weight: f64,
    social_weight: f64,
    max_agents: usize,
}

impl SwarmIntelligence {
    pub fn new(num_agents: usize, dimensions: usize) -> Self {
        assert_eq!(dimensions, 3, "Currently only 3D space supported");
        
        let mut agents = HashMap::new();
        let mut global_best = [0.0; 3];
        let mut global_best_fitness = f64::NEG_INFINITY;

        for i in 0..num_agents {
            let agent_type = match i % 4 {
                0 => AgentType::Explorer,
                1 => AgentType::Exploiter,
                2 => AgentType::Balanced,
                _ => AgentType::Specialist,
            };

            let position = [
                rand::random::<f64>() * 100.0 - 50.0,
                rand::random::<f64>() * 100.0 - 50.0,
                rand::random::<f64>() * 100.0 - 50.0,
            ];

            let agent = Agent {
                id: format!("agent_{}", i),
                position,
                velocity: [0.0; 3],
                fitness: f64::NEG_INFINITY,
                personal_best: position,
                personal_best_fitness: f64::NEG_INFINITY,
                agent_type,
            };

            agents.insert(agent.id.clone(), agent);
        }

        Self {
            agents,
            global_best,
            global_best_fitness,
            message_queue: Vec::new(),
            inertia: 0.7,
            cognitive_weight: 1.5,
            social_weight: 1.5,
            max_agents: num_agents * 2,
        }
    }

    /// Update swarm with fitness function
    pub fn step<F>(&mut self, fitness_fn: F)
    where
        F: Fn(&[f64; 3]) -> f64 + Sync,
    {
        // Evaluate fitness for all agents
        for agent in self.agents.values_mut() {
            agent.fitness = fitness_fn(&agent.position);

            // Update personal best
            if agent.fitness > agent.personal_best_fitness {
                agent.personal_best = agent.position;
                agent.personal_best_fitness = agent.fitness;
            }

            // Update global best
            if agent.fitness > self.global_best_fitness {
                self.global_best = agent.position;
                self.global_best_fitness = agent.fitness;

                // Broadcast best found
                self.message_queue.push(SwarmMessage {
                    from: agent.id.clone(),
                    to: None,
                    content: MessageContent::BestFound(self.global_best, self.global_best_fitness),
                });
            }
        }

        // Update velocities and positions
        let agent_ids: Vec<_> = self.agents.keys().cloned().collect();
        for id in agent_ids {
            self.update_agent_velocity(&id);
            self.update_agent_position(&id);
        }

        // Process messages
        self.process_messages();
    }

    fn update_agent_velocity(&mut self, agent_id: &str) {
        if let Some(agent) = self.agents.get_mut(agent_id) {
            let exploration_factor = match agent.agent_type {
                AgentType::Explorer => 1.5,
                AgentType::Exploiter => 0.5,
                AgentType::Balanced => 1.0,
                AgentType::Specialist => 0.8,
            };

            for i in 0..3 {
                let r1: f64 = rand::random();
                let r2: f64 = rand::random();

                let cognitive = self.cognitive_weight * r1 * (agent.personal_best[i] - agent.position[i]);
                let social = self.social_weight * r2 * (self.global_best[i] - agent.position[i]);

                agent.velocity[i] = self.inertia * agent.velocity[i] 
                    + exploration_factor * (cognitive + social);

                // Velocity clamping
                agent.velocity[i] = agent.velocity[i].clamp(-10.0, 10.0);
            }
        }
    }

    fn update_agent_position(&mut self, agent_id: &str) {
        if let Some(agent) = self.agents.get_mut(agent_id) {
            for i in 0..3 {
                agent.position[i] += agent.velocity[i];
                // Boundary handling
                agent.position[i] = agent.position[i].clamp(-100.0, 100.0);
            }

            // Broadcast position update
            self.message_queue.push(SwarmMessage {
                from: agent.id.clone(),
                to: None,
                content: MessageContent::PositionUpdate(agent.position, agent.fitness),
            });
        }
    }

    fn process_messages(&mut self) {
        // Simple message processing - can be extended
        self.message_queue.clear();
    }

    /// Add new agent to swarm (self-replication)
    pub fn spawn_agent(&mut self, parent_id: &str) -> Option<String> {
        if self.agents.len() >= self.max_agents {
            return None;
        }

        if let Some(parent) = self.agents.get(parent_id) {
            let new_id = format!("agent_{}_{}", parent_id, rand::random::<u32>());
            
            // Spawn near parent with slight variation
            let position = [
                parent.position[0] + (rand::random::<f64>() - 0.5) * 10.0,
                parent.position[1] + (rand::random::<f64>() - 0.5) * 10.0,
                parent.position[2] + (rand::random::<f64>() - 0.5) * 10.0,
            ];

            let new_agent = Agent {
                id: new_id.clone(),
                position,
                velocity: [0.0; 3],
                fitness: f64::NEG_INFINITY,
                personal_best: position,
                personal_best_fitness: f64::NEG_INFINITY,
                agent_type: parent.agent_type.clone(),
            };

            self.agents.insert(new_id.clone(), new_agent);
            Some(new_id)
        } else {
            None
        }
    }

    /// Get swarm statistics
    pub fn stats(&self) -> SwarmStats {
        let fitnesses: Vec<f64> = self.agents.values().map(|a| a.fitness).collect();
        let avg_fitness = fitnesses.iter().sum::<f64>() / fitnesses.len() as f64;
        
        let diversity = self.calculate_diversity();

        SwarmStats {
            num_agents: self.agents.len(),
            global_best_fitness: self.global_best_fitness,
            avg_fitness,
            diversity,
        }
    }

    fn calculate_diversity(&self) -> f64 {
        if self.agents.len() < 2 {
            return 0.0;
        }

        let positions: Vec<_> = self.agents.values().map(|a| a.position).collect();
        let mut total_distance = 0.0;
        let mut count = 0;

        for i in 0..positions.len() {
            for j in i + 1..positions.len() {
                let dist = euclidean_distance(&positions[i], &positions[j]);
                total_distance += dist;
                count += 1;
            }
        }

        total_distance / count as f64
    }

    pub fn global_best(&self) -> ([f64; 3], f64) {
        (self.global_best, self.global_best_fitness)
    }

    pub fn agents(&self) -> &HashMap<String, Agent> {
        &self.agents
    }
}

#[derive(Debug, Clone)]
pub struct SwarmStats {
    pub num_agents: usize,
    pub global_best_fitness: f64,
    pub avg_fitness: f64,
    pub diversity: f64,
}

fn euclidean_distance(a: &[f64; 3], b: &[f64; 3]) -> f64 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_swarm_creation() {
        let swarm = SwarmIntelligence::new(10, 3);
        assert_eq!(swarm.agents().len(), 10);
    }

    #[test]
    fn test_swarm_optimization() {
        let mut swarm = SwarmIntelligence::new(20, 3);
        
        // Sphere function (minimum at origin)
        let fitness_fn = |pos: &[f64; 3]| {
            -(pos[0].powi(2) + pos[1].powi(2) + pos[2].powi(2))
        };

        for _ in 0..50 {
            swarm.step(fitness_fn);
        }

        let (best_pos, best_fitness) = swarm.global_best();
        assert!(best_fitness > -100.0); // Should converge towards 0
    }

    #[test]
    fn test_agent_spawning() {
        let mut swarm = SwarmIntelligence::new(5, 3);
        let first_agent_id = swarm.agents().keys().next().unwrap().clone();
        
        let new_id = swarm.spawn_agent(&first_agent_id);
        assert!(new_id.is_some());
        assert_eq!(swarm.agents().len(), 6);
    }
}

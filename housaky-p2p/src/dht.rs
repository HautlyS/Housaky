//! Kademlia DHT - Distributed Hash Table

use std::collections::HashMap;

pub struct KademliaDHT {
    routing_table: HashMap<String, Vec<String>>,
    k: usize,
    node_id: String,
}

impl KademliaDHT {
    pub fn new(node_id: String, k: usize) -> Self {
        Self {
            routing_table: HashMap::new(),
            k,
            node_id,
        }
    }

    pub fn find_node(&self, id: &str) -> Option<Vec<String>> {
        self.routing_table.get(id).cloned()
    }

    pub fn store(&mut self, key: String, value: Vec<String>) {
        self.routing_table.insert(key, value);
    }

    pub fn find_closest_nodes(&self, target: &str, count: usize) -> Vec<String> {
        let mut nodes: Vec<(String, usize)> = self.routing_table
            .keys()
            .map(|k| (k.clone(), self.xor_distance(target, k)))
            .collect();
        
        nodes.sort_by_key(|(_, dist)| *dist);
        nodes.truncate(count);
        nodes.into_iter().map(|(k, _)| k).collect()
    }

    fn xor_distance(&self, a: &str, b: &str) -> usize {
        a.bytes().zip(b.bytes()).map(|(x, y)| (x ^ y).count_ones() as usize).sum()
    }

    pub fn add_node(&mut self, node_id: String, peers: Vec<String>) {
        self.routing_table.insert(node_id, peers);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dht_creation() {
        let dht = KademliaDHT::new("node1".to_string(), 20);
        assert_eq!(dht.k, 20);
    }

    #[test]
    fn test_store_and_find() {
        let mut dht = KademliaDHT::new("node1".to_string(), 20);
        dht.store("key1".to_string(), vec!["value1".to_string()]);
        
        let result = dht.find_node("key1");
        assert!(result.is_some());
    }
}

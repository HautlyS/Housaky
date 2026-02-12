//! Simple blockchain for token economy

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    pub index: u64,
    pub timestamp: u64,
    pub transactions: Vec<Transaction>,
    pub previous_hash: String,
    pub hash: String,
    pub nonce: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub from: String,
    pub to: String,
    pub amount: f64,
    pub tx_type: TransactionType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransactionType {
    Compute,
    Storage,
    Bandwidth,
    Reward,
}

impl Block {
    pub fn new(index: u64, transactions: Vec<Transaction>, previous_hash: String) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let mut block = Self {
            index,
            timestamp,
            transactions,
            previous_hash,
            hash: String::new(),
            nonce: 0,
        };

        block.hash = block.calculate_hash();
        block
    }

    pub fn calculate_hash(&self) -> String {
        let data = format!(
            "{}{}{}{}{}",
            self.index,
            self.timestamp,
            bincode::serialize(&self.transactions).unwrap_or_default().len(),
            self.previous_hash,
            self.nonce
        );

        let mut hasher = Sha256::new();
        hasher.update(data.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    pub fn mine(&mut self, difficulty: usize) {
        let target = "0".repeat(difficulty);
        while !self.hash.starts_with(&target) {
            self.nonce += 1;
            self.hash = self.calculate_hash();
        }
    }
}

pub struct Blockchain {
    pub chain: Vec<Block>,
    pub pending_transactions: Vec<Transaction>,
    pub difficulty: usize,
    pub mining_reward: f64,
}

impl Blockchain {
    pub fn new() -> Self {
        let mut chain = Self {
            chain: Vec::new(),
            pending_transactions: Vec::new(),
            difficulty: 2,
            mining_reward: 10.0,
        };
        chain.create_genesis_block();
        chain
    }

    fn create_genesis_block(&mut self) {
        let genesis = Block::new(0, vec![], "0".to_string());
        self.chain.push(genesis);
    }

    pub fn add_transaction(&mut self, transaction: Transaction) {
        self.pending_transactions.push(transaction);
    }

    pub fn mine_pending_transactions(&mut self, miner_address: &str) {
        let mut block = Block::new(
            self.chain.len() as u64,
            self.pending_transactions.clone(),
            self.get_latest_block().hash.clone(),
        );

        block.mine(self.difficulty);
        self.chain.push(block);

        // Reward miner
        self.pending_transactions = vec![Transaction {
            from: "system".to_string(),
            to: miner_address.to_string(),
            amount: self.mining_reward,
            tx_type: TransactionType::Reward,
        }];
    }

    pub fn get_latest_block(&self) -> &Block {
        self.chain.last().unwrap()
    }

    pub fn get_balance(&self, address: &str) -> f64 {
        let mut balance = 0.0;

        for block in &self.chain {
            for tx in &block.transactions {
                if tx.from == address {
                    balance -= tx.amount;
                }
                if tx.to == address {
                    balance += tx.amount;
                }
            }
        }

        balance
    }

    pub fn is_valid(&self) -> bool {
        for i in 1..self.chain.len() {
            let current = &self.chain[i];
            let previous = &self.chain[i - 1];

            if current.hash != current.calculate_hash() {
                return false;
            }

            if current.previous_hash != previous.hash {
                return false;
            }
        }
        true
    }
}

impl Default for Blockchain {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blockchain() {
        let mut chain = Blockchain::new();
        
        chain.add_transaction(Transaction {
            from: "alice".to_string(),
            to: "bob".to_string(),
            amount: 50.0,
            tx_type: TransactionType::Compute,
        });

        chain.mine_pending_transactions("miner1");
        
        assert!(chain.is_valid());
        assert_eq!(chain.chain.len(), 2);
    }

    #[test]
    fn test_balance() {
        let mut chain = Blockchain::new();
        
        chain.add_transaction(Transaction {
            from: "system".to_string(),
            to: "alice".to_string(),
            amount: 100.0,
            tx_type: TransactionType::Reward,
        });

        chain.mine_pending_transactions("miner1");
        
        let balance = chain.get_balance("alice");
        assert_eq!(balance, 100.0);
    }
}

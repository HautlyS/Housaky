//! Token system
use anyhow::Result;
use housaky_core::crypto::{hash, Identity};
use serde::{Deserialize, Serialize};

/// Token amount
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TokenAmount(pub u64);

impl TokenAmount {
    pub fn new(amount: u64) -> Self {
        Self(amount)
    }

    pub fn add(&self, other: TokenAmount) -> Self {
        Self(self.0 + other.0)
    }

    pub fn sub(&self, other: TokenAmount) -> Option<Self> {
        self.0.checked_sub(other.0).map(Self)
    }
}

/// Transaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub from: Identity,
    pub to: Identity,
    pub amount: TokenAmount,
    pub timestamp: u64,
    pub nonce: u64,
}

impl Transaction {
    pub fn new(from: Identity, to: Identity, amount: TokenAmount) -> Self {
        Self {
            from,
            to,
            amount,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            nonce: rand::random(),
        }
    }

    pub fn hash(&self) -> [u8; 32] {
        let data = serde_json::to_vec(self).unwrap();
        hash(&data)
    }
}

/// Account balance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    pub identity: Identity,
    pub balance: TokenAmount,
}

/// Token ledger
pub struct Ledger {
    accounts: std::collections::HashMap<Identity, Account>,
    transactions: Vec<Transaction>,
}

impl Ledger {
    pub fn new() -> Self {
        Self {
            accounts: std::collections::HashMap::new(),
            transactions: Vec::new(),
        }
    }

    pub fn create_account(&mut self, identity: Identity, initial_balance: TokenAmount) {
        self.accounts.insert(
            identity.clone(),
            Account {
                identity,
                balance: initial_balance,
            },
        );
    }

    pub fn transfer(&mut self, from: &Identity, to: &Identity, amount: TokenAmount) -> Result<()> {
        let from_account = self
            .accounts
            .get_mut(from)
            .ok_or_else(|| anyhow::anyhow!("Source account not found"))?;

        let new_balance = from_account
            .balance
            .sub(amount)
            .ok_or_else(|| anyhow::anyhow!("Insufficient balance"))?;

        from_account.balance = new_balance;

        let to_account = self
            .accounts
            .get_mut(to)
            .ok_or_else(|| anyhow::anyhow!("Destination account not found"))?;

        to_account.balance = to_account.balance.add(amount);

        let tx = Transaction::new(from.clone(), to.clone(), amount);
        self.transactions.push(tx);

        Ok(())
    }

    pub fn balance(&self, identity: &Identity) -> Option<TokenAmount> {
        self.accounts.get(identity).map(|a| a.balance)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_amount() {
        let a = TokenAmount::new(100);
        let b = TokenAmount::new(50);

        assert_eq!(a.add(b), TokenAmount::new(150));
        assert_eq!(a.sub(b), Some(TokenAmount::new(50)));
    }

    #[test]
    fn test_ledger() {
        let mut ledger = Ledger::new();
        let id1 = Identity::from_public_key([1u8; 32]);
        let id2 = Identity::from_public_key([2u8; 32]);

        ledger.create_account(id1.clone(), TokenAmount::new(1000));
        ledger.create_account(id2.clone(), TokenAmount::new(0));

        ledger.transfer(&id1, &id2, TokenAmount::new(100)).unwrap();

        assert_eq!(ledger.balance(&id1), Some(TokenAmount::new(900)));
        assert_eq!(ledger.balance(&id2), Some(TokenAmount::new(100)));
    }
}

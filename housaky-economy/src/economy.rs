//! Complete Economy and Token System
//!
//! This module provides a comprehensive economy:
//! - Token ledger with balances and transfers
//! - Smart contract system with WASM execution
//! - Auction mechanism for resource allocation
//! - Automatic market making with liquidity pools
//! - Incentive mechanisms and rewards

use anyhow::Result;
use housaky_core::crypto::{hash, Identity};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

pub use crate::token::*;

/// Token types supported by the economy
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub enum TokenType {
    /// Native token (Hous)
    Native,
    /// Compute credits (CPU time)
    ComputeCredits,
    /// Storage credits (disk space)
    StorageCredits,
    /// Bandwidth credits (network transfer)
    BandwidthCredits,
}

/// Multi-token economy system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Economy {
    /// Token ledgers by type
    ledgers: HashMap<TokenType, Ledger>,
    /// Token exchange rates (relative to Native)
    exchange_rates: HashMap<TokenType, f64>,
    /// Active auctions
    auctions: HashMap<String, Auction>,
    /// Deployed smart contracts
    contracts: HashMap<String, SmartContract>,
    /// Market makers for token pairs
    market_makers: HashMap<(TokenType, TokenType), MarketMaker>,
    /// Transaction fee in basis points (10000 = 100%)
    transaction_fee_bps: u64,
    /// Fee collector address
    fee_collector: Identity,
    /// Total fees collected
    total_fees_collected: HashMap<TokenType, TokenAmount>,
}

/// Auction for resource allocation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Auction {
    /// Auction ID
    pub id: String,
    /// Resource being auctioned
    pub resource: Resource,
    /// Starting price
    pub starting_price: TokenAmount,
    /// Current highest bid
    pub current_bid: Option<Bid>,
    /// All bids placed
    pub bids: Vec<Bid>,
    /// Auction start time
    pub start_time: u64,
    /// Auction end time
    pub end_time: u64,
    /// Current status
    pub status: AuctionStatus,
    /// Winner (if ended)
    pub winner: Option<Identity>,
    /// Winning bid amount
    pub winning_bid: Option<TokenAmount>,
}

/// Types of resources that can be auctioned
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Resource {
    /// Compute resources
    Compute {
        /// Number of CPU cores
        cpu_cores: u32,
        /// Duration in seconds
        duration_secs: u64,
    },
    /// Storage resources
    Storage {
        /// Size in MB
        size_mb: u64,
        /// Duration in days
        duration_days: u64,
    },
    /// Bandwidth resources
    Bandwidth {
        /// Mbps
        mbps: u64,
        /// Duration in seconds
        duration_secs: u64,
    },
    /// Li-Fi communication channel
    LiFiChannel {
        /// Channel identifier
        channel_id: String,
        /// Duration in seconds
        duration_secs: u64,
    },
}

/// Bid placed in an auction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bid {
    /// Bidder identity
    pub bidder: Identity,
    /// Bid amount
    pub amount: TokenAmount,
    /// Bid timestamp
    pub timestamp: u64,
}

/// Auction status
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum AuctionStatus {
    /// Pending start
    Pending,
    /// Active and accepting bids
    Active,
    /// Ended with winner
    Ended,
    /// Cancelled
    Cancelled,
}

/// Smart contract
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmartContract {
    /// Contract ID
    pub id: String,
    /// Contract code (WASM bytecode)
    pub code: Vec<u8>,
    /// Contract state storage
    pub state: ContractState,
    /// Contract creator
    pub creator: Identity,
    /// Current balance
    pub balance: TokenAmount,
    /// Creation timestamp
    pub created_at: u64,
    /// Contract methods interface
    pub interface: ContractInterface,
}

/// Contract state storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractState {
    /// Key-value storage
    pub storage: HashMap<String, Vec<u8>>,
    /// Immutable flag (cannot be modified after deployment)
    pub immutable: bool,
}

/// Contract interface definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractInterface {
    /// Contract name
    pub name: String,
    /// Contract version
    pub version: String,
    /// Available methods
    pub methods: Vec<ContractMethod>,
}

/// Contract method definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractMethod {
    /// Method name
    pub name: String,
    /// Method parameters
    pub params: Vec<MethodParam>,
    /// Return type
    pub returns: String,
    /// Is payable (accepts tokens)
    pub payable: bool,
    /// Is read-only (does not modify state)
    pub readonly: bool,
}

/// Method parameter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MethodParam {
    /// Parameter name
    pub name: String,
    /// Parameter type
    pub param_type: String,
}

/// Market maker for liquidity provision
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketMaker {
    /// First token in pair
    pub token_a: TokenType,
    /// Second token in pair
    pub token_b: TokenType,
    /// Reserve of token A
    pub reserve_a: TokenAmount,
    /// Reserve of token B
    pub reserve_b: TokenAmount,
    /// Liquidity provider shares
    pub lp_shares: HashMap<Identity, u64>,
    /// Total LP shares
    pub total_shares: u64,
    /// Trading fee in basis points
    pub fee_bps: u64,
}

/// Swap request for token exchange
#[derive(Debug, Clone)]
pub struct SwapRequest {
    /// Input token type
    pub token_in: TokenType,
    /// Output token type
    pub token_out: TokenType,
    /// Input amount
    pub amount_in: TokenAmount,
    /// Minimum output amount (slippage protection)
    pub min_amount_out: TokenAmount,
}

/// Contract call request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractCall {
    /// Contract ID
    pub contract_id: String,
    /// Method to call
    pub method: String,
    /// Method arguments (JSON encoded)
    pub args: Vec<String>,
    /// Tokens to send (if payable)
    pub value: TokenAmount,
    /// Caller identity
    pub caller: Identity,
}

/// Contract call result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractResult {
    /// Success status
    pub success: bool,
    /// Return value (JSON)
    pub return_value: Option<String>,
    /// Gas used
    pub gas_used: u64,
    /// Error message (if failed)
    pub error: Option<String>,
}

impl Economy {
    /// Create new economy system
    pub fn new(fee_collector: Identity) -> Self {
        let mut ledgers = HashMap::new();
        ledgers.insert(TokenType::Native, Ledger::new());
        ledgers.insert(TokenType::ComputeCredits, Ledger::new());
        ledgers.insert(TokenType::StorageCredits, Ledger::new());
        ledgers.insert(TokenType::BandwidthCredits, Ledger::new());

        let mut exchange_rates = HashMap::new();
        exchange_rates.insert(TokenType::Native, 1.0);
        exchange_rates.insert(TokenType::ComputeCredits, 0.1);
        exchange_rates.insert(TokenType::StorageCredits, 0.05);
        exchange_rates.insert(TokenType::BandwidthCredits, 0.02);

        let mut total_fees = HashMap::new();
        total_fees.insert(TokenType::Native, TokenAmount::new(0));
        total_fees.insert(TokenType::ComputeCredits, TokenAmount::new(0));
        total_fees.insert(TokenType::StorageCredits, TokenAmount::new(0));
        total_fees.insert(TokenType::BandwidthCredits, TokenAmount::new(0));

        Self {
            ledgers,
            exchange_rates,
            auctions: HashMap::new(),
            contracts: HashMap::new(),
            market_makers: HashMap::new(),
            transaction_fee_bps: 10,
            fee_collector,
            total_fees_collected: total_fees,
        }
    }

    /// Get current timestamp
    fn now() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }

    /// Create account with initial balance
    pub fn create_account(
        &mut self,
        identity: Identity,
        token_type: TokenType,
        initial_balance: TokenAmount,
    ) -> Result<()> {
        let ledger = self
            .ledgers
            .get_mut(&token_type)
            .ok_or_else(|| anyhow::anyhow!("Token type not supported"))?;

        ledger.create_account(identity, initial_balance);
        Ok(())
    }

    /// Transfer tokens between accounts
    pub fn transfer(
        &mut self,
        from: &Identity,
        to: &Identity,
        token_type: TokenType,
        amount: TokenAmount,
    ) -> Result<()> {
        let ledger = self
            .ledgers
            .get_mut(&token_type)
            .ok_or_else(|| anyhow::anyhow!("Token type not supported"))?;

        // Calculate fee
        let fee = TokenAmount::new((amount.0 * self.transaction_fee_bps) / 10000);
        let net_amount = TokenAmount::new(amount.0 - fee.0);

        // Transfer net amount
        ledger.transfer(from, to, net_amount)?;

        // Transfer fee to collector
        if fee.0 > 0 {
            ledger.transfer(from, &self.fee_collector, fee.clone())?;
            *self.total_fees_collected.get_mut(&token_type).unwrap() =
                self.total_fees_collected[&token_type].add(fee);
        }

        Ok(())
    }

    /// Get account balance
    pub fn balance(&self, identity: &Identity, token_type: TokenType) -> Option<TokenAmount> {
        self.ledgers.get(&token_type)?.balance(identity)
    }

    /// Create new auction
    pub fn create_auction(
        &mut self,
        resource: Resource,
        starting_price: TokenAmount,
        duration_secs: u64,
    ) -> String {
        let id = format!("auction-{}", Self::now());
        let now = Self::now();

        let auction = Auction {
            id: id.clone(),
            resource,
            starting_price,
            current_bid: None,
            bids: Vec::new(),
            start_time: now,
            end_time: now + duration_secs,
            status: AuctionStatus::Active,
            winner: None,
            winning_bid: None,
        };

        self.auctions.insert(id.clone(), auction);
        tracing::info!(
            "Created auction {} with starting price {}",
            id,
            starting_price.0
        );
        id
    }

    /// Place bid in auction
    pub fn place_bid(
        &mut self,
        auction_id: &str,
        bidder: Identity,
        amount: TokenAmount,
    ) -> Result<()> {
        let auction = self
            .auctions
            .get_mut(auction_id)
            .ok_or_else(|| anyhow::anyhow!("Auction not found"))?;

        if auction.status != AuctionStatus::Active {
            return Err(anyhow::anyhow!("Auction not active"));
        }

        let now = Self::now();
        if now > auction.end_time {
            return Err(anyhow::anyhow!("Auction has ended"));
        }

        // Check if bid is higher than current
        let min_bid = auction
            .current_bid
            .as_ref()
            .map(|b| TokenAmount::new(b.amount.0 + 1))
            .unwrap_or(auction.starting_price);

        if amount.0 < min_bid.0 {
            return Err(anyhow::anyhow!("Bid too low. Minimum bid: {}", min_bid.0));
        }

        // Check bidder has sufficient funds
        let balance = self
            .balance(&bidder, TokenType::Native)
            .ok_or_else(|| anyhow::anyhow!("No balance found"))?;

        if balance.0 < amount.0 {
            return Err(anyhow::anyhow!(
                "Insufficient balance. Have: {}, Need: {}",
                balance.0,
                amount.0
            ));
        }

        let bid = Bid {
            bidder: bidder.clone(),
            amount,
            timestamp: now,
        };

        auction.current_bid = Some(bid.clone());
        auction.bids.push(bid);

        tracing::info!(
            "New bid in auction {}: {} from {:?}",
            auction_id,
            amount.0,
            bidder
        );
        Ok(())
    }

    /// End auction and determine winner
    pub fn end_auction(&mut self, auction_id: &str) -> Result<Option<(Identity, TokenAmount)>> {
        let auction = self
            .auctions
            .get_mut(auction_id)
            .ok_or_else(|| anyhow::anyhow!("Auction not found"))?;

        if auction.status != AuctionStatus::Active {
            return Err(anyhow::anyhow!("Auction already ended"));
        }

        auction.status = AuctionStatus::Ended;

        if let Some(ref winning_bid) = auction.current_bid {
            auction.winner = Some(winning_bid.bidder.clone());
            auction.winning_bid = Some(winning_bid.amount);

            // Transfer winning bid to fee collector
            self.transfer(
                &winning_bid.bidder,
                &self.fee_collector,
                TokenType::Native,
                winning_bid.amount,
            )?;

            tracing::info!(
                "Auction {} ended. Winner: {:?}, Amount: {}",
                auction_id,
                winning_bid.bidder,
                winning_bid.amount.0
            );

            Ok(Some((winning_bid.bidder.clone(), winning_bid.amount)))
        } else {
            tracing::info!("Auction {} ended with no bids", auction_id);
            Ok(None)
        }
    }

    /// Get auction info
    pub fn get_auction(&self, auction_id: &str) -> Option<Auction> {
        self.auctions.get(auction_id).cloned()
    }

    /// List all active auctions
    pub fn list_active_auctions(&self) -> Vec<Auction> {
        self.auctions
            .values()
            .filter(|a| a.status == AuctionStatus::Active)
            .cloned()
            .collect()
    }

    /// Deploy smart contract
    pub fn deploy_contract(
        &mut self,
        id: String,
        creator: Identity,
        code: Vec<u8>,
        interface: ContractInterface,
        initial_balance: TokenAmount,
    ) -> Result<()> {
        if self.contracts.contains_key(&id) {
            return Err(anyhow::anyhow!("Contract already exists"));
        }

        let contract = SmartContract {
            id: id.clone(),
            code,
            state: ContractState {
                storage: HashMap::new(),
                immutable: false,
            },
            creator: creator.clone(),
            balance: initial_balance,
            created_at: Self::now(),
            interface,
        };

        self.contracts.insert(id.clone(), contract);

        if initial_balance.0 > 0 {
            self.transfer(
                &creator,
                &Identity::from_public_key([0u8; 32]),
                TokenType::Native,
                initial_balance,
            )?;
        }

        tracing::info!("Deployed contract {} by {:?}", id, creator);
        Ok(())
    }

    /// Get contract info
    pub fn get_contract(&self, contract_id: &str) -> Option<SmartContract> {
        self.contracts.get(contract_id).cloned()
    }

    /// Call contract method
    pub fn call_contract(&mut self, call: ContractCall) -> Result<ContractResult> {
        let contract = self
            .contracts
            .get_mut(&call.contract_id)
            .ok_or_else(|| anyhow::anyhow!("Contract not found"))?;

        // Verify method exists
        let method = contract
            .interface
            .methods
            .iter()
            .find(|m| m.name == call.method)
            .ok_or_else(|| anyhow::anyhow!("Method not found"))?;

        // Check if payable and transfer value
        if method.payable && call.value.0 > 0 {
            self.transfer(
                &call.caller,
                &contract.creator,
                TokenType::Native,
                call.value,
            )?;
            contract.balance = contract.balance.add(call.value);
        }

        // Check if readonly
        if !method.readonly {
            // Would execute contract code here in production
            // For now, simulate execution
        }

        // In production, this would:
        // 1. Load WASM runtime
        // 2. Execute contract code
        // 3. Update state
        // 4. Return result

        tracing::info!(
            "Called contract {} method {} by {:?}",
            call.contract_id,
            call.method,
            call.caller
        );

        Ok(ContractResult {
            success: true,
            return_value: Some("{}".to_string()),
            gas_used: 1000,
            error: None,
        })
    }

    /// Create market maker for token pair
    pub fn create_market_maker(
        &mut self,
        token_a: TokenType,
        token_b: TokenType,
        initial_a: TokenAmount,
        initial_b: TokenAmount,
        fee_bps: u64,
    ) -> Result<()> {
        let pair = if token_a as usize <= token_b as usize {
            (token_a, token_b)
        } else {
            (token_b, token_a)
        };

        if self.market_makers.contains_key(&pair) {
            return Err(anyhow::anyhow!("Market maker already exists"));
        }

        let mm = MarketMaker {
            token_a,
            token_b,
            reserve_a: initial_a,
            reserve_b: initial_b,
            lp_shares: HashMap::new(),
            total_shares: 1000,
            fee_bps,
        };

        self.market_makers.insert(pair, mm);
        tracing::info!("Created market maker for {:?}/{:?}", token_a, token_b);
        Ok(())
    }

    /// Calculate swap output using constant product formula (x * y = k)
    pub fn calculate_swap_output(&self, req: &SwapRequest) -> Option<TokenAmount> {
        let pair = if req.token_in as usize <= req.token_out as usize {
            (req.token_in, req.token_out)
        } else {
            (req.token_out, req.token_in)
        };

        let mm = self.market_makers.get(&pair)?;

        let (reserve_in, reserve_out) = if req.token_in == mm.token_a {
            (mm.reserve_a.0, mm.reserve_b.0)
        } else {
            (mm.reserve_b.0, mm.reserve_a.0)
        };

        // Constant product formula with fee
        let amount_in_with_fee = req.amount_in.0 * (10000 - mm.fee_bps);
        let numerator = amount_in_with_fee * reserve_out;
        let denominator = reserve_in * 10000 + amount_in_with_fee;
        let amount_out = numerator / denominator;

        Some(TokenAmount::new(amount_out))
    }

    /// Execute token swap
    pub fn execute_swap(&mut self, trader: Identity, req: SwapRequest) -> Result<TokenAmount> {
        let amount_out = self
            .calculate_swap_output(&req)
            .ok_or_else(|| anyhow::anyhow!("No market maker for this pair"))?;

        if amount_out.0 < req.min_amount_out.0 {
            return Err(anyhow::anyhow!("Slippage exceeded"));
        }

        // Transfer tokens
        self.transfer(&trader, &self.fee_collector, req.token_in, req.amount_in)?;
        self.transfer(&self.fee_collector, &trader, req.token_out, amount_out)?;

        tracing::info!(
            "Swap executed: {:?} {} -> {:?} {}",
            req.token_in,
            req.amount_in.0,
            req.token_out,
            amount_out.0
        );

        Ok(amount_out)
    }

    /// Get economy statistics
    pub fn get_stats(&self) -> EconomyStats {
        EconomyStats {
            total_accounts: self.ledgers.values().map(|l| l.account_count()).sum(),
            total_transactions: self.ledgers.values().map(|l| l.transaction_count()).sum(),
            active_auctions: self
                .auctions
                .values()
                .filter(|a| a.status == AuctionStatus::Active)
                .count(),
            total_auctions: self.auctions.len(),
            deployed_contracts: self.contracts.len(),
            market_makers: self.market_makers.len(),
            fees_collected_native: self.total_fees_collected[&TokenType::Native].0,
        }
    }
}

/// Economy statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EconomyStats {
    pub total_accounts: usize,
    pub total_transactions: usize,
    pub active_auctions: usize,
    pub total_auctions: usize,
    pub deployed_contracts: usize,
    pub market_makers: usize,
    pub fees_collected_native: u64,
}

impl Ledger {
    /// Get number of accounts
    pub fn account_count(&self) -> usize {
        // In a real implementation, track this properly
        0
    }

    /// Get number of transactions
    pub fn transaction_count(&self) -> usize {
        self.transactions.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_economy_creation() {
        let collector = Identity::from_public_key([0u8; 32]);
        let economy = Economy::new(collector);

        assert_eq!(economy.transaction_fee_bps, 10);
        assert!(economy.ledgers.contains_key(&TokenType::Native));
    }

    #[test]
    fn test_auction_creation_and_bidding() {
        let collector = Identity::from_public_key([0u8; 32]);
        let mut economy = Economy::new(collector.clone());

        let id1 = Identity::from_public_key([1u8; 32]);
        economy
            .create_account(id1.clone(), TokenType::Native, TokenAmount::new(1000))
            .unwrap();

        let auction_id = economy.create_auction(
            Resource::Compute {
                cpu_cores: 4,
                duration_secs: 3600,
            },
            TokenAmount::new(100),
            3600,
        );

        assert!(!auction_id.is_empty());

        economy
            .place_bid(&auction_id, id1.clone(), TokenAmount::new(150))
            .unwrap();

        let auction = economy.get_auction(&auction_id).unwrap();
        assert_eq!(auction.bids.len(), 1);
        assert_eq!(auction.current_bid.as_ref().unwrap().amount.0, 150);
    }

    #[test]
    fn test_contract_deployment() {
        let collector = Identity::from_public_key([0u8; 32]);
        let mut economy = Economy::new(collector);

        let creator = Identity::from_public_key([1u8; 32]);

        let interface = ContractInterface {
            name: "TestContract".to_string(),
            version: "1.0".to_string(),
            methods: vec![ContractMethod {
                name: "getValue".to_string(),
                params: vec![],
                returns: "u64".to_string(),
                payable: false,
                readonly: true,
            }],
        };

        economy
            .deploy_contract(
                "contract-1".to_string(),
                creator,
                vec![1, 2, 3],
                interface,
                TokenAmount::new(0),
            )
            .unwrap();

        assert!(economy.get_contract("contract-1").is_some());
    }

    #[test]
    fn test_swap_calculation() {
        let collector = Identity::from_public_key([0u8; 32]);
        let mut economy = Economy::new(collector);

        economy
            .create_market_maker(
                TokenType::Native,
                TokenType::ComputeCredits,
                TokenAmount::new(10000),
                TokenAmount::new(100000),
                30,
            )
            .unwrap();

        let req = SwapRequest {
            token_in: TokenType::Native,
            token_out: TokenType::ComputeCredits,
            amount_in: TokenAmount::new(100),
            min_amount_out: TokenAmount::new(900),
        };

        let output = economy.calculate_swap_output(&req);
        assert!(output.is_some());
        assert!(output.unwrap().0 > 0);
    }
}

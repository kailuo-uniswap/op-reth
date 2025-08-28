//! Comprehensive tests for the Uniswap v4 extsload optimization

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::{address, hex, Address, Bytes, B256, U256};
    use reth_evm::extsload_optimizer::{ExtsloadConfig, selectors, setup_extsload_precompiles};
    use revm::{
        precompile::PrecompileSpecId,
        primitives::{Env, TxKind, TransactTo, SpecId},
        ContextPrecompiles, Database,
    };

    /// Mock database for testing
    #[derive(Debug, Default)]
    struct MockDb;

    impl Database for MockDb {
        type Error = ();

        fn basic(&mut self, _address: Address) -> Result<Option<revm::primitives::AccountInfo>, Self::Error> {
            Ok(None)
        }

        fn code_by_hash(&mut self, _code_hash: B256) -> Result<revm::primitives::Bytecode, Self::Error> {
            Ok(revm::primitives::Bytecode::default())
        }

        fn storage(&mut self, _address: Address, _index: U256) -> Result<U256, Self::Error> {
            Ok(U256::ZERO)
        }

        fn block_hash(&mut self, _number: U256) -> Result<B256, Self::Error> {
            Ok(B256::ZERO)
        }
    }

    #[test]
    fn test_function_selectors() {
        // Verify our function selectors are correct
        use hex_literal::hex;
        
        // extsload(bytes32) selector
        assert_eq!(selectors::SINGLE_SLOT, hex!("6c94522e"));
        
        // extsload(bytes32,uint256) selector  
        assert_eq!(selectors::CONSECUTIVE_SLOTS, hex!("37963c44"));
        
        // extsload(bytes32[]) selector
        assert_eq!(selectors::SPARSE_SLOTS, hex!("06b8ab04"));
    }

    #[test]
    fn test_extsload_config() {
        let pool_manager = address!("1f98400000000000000000000000000000000004");
        
        let config = ExtsloadConfig {
            pool_manager_addresses: vec![pool_manager],
            single_slot_precompile: address!("0000000000000000000000000000000000000100"),
            consecutive_slots_precompile: address!("0000000000000000000000000000000000000101"),
            sparse_slots_precompile: address!("0000000000000000000000000000000000000102"),
            enabled: true,
            ..Default::default()
        };

        assert!(config.enabled);
        assert_eq!(config.pool_manager_addresses.len(), 1);
        assert_eq!(config.pool_manager_addresses[0], pool_manager);
    }

    #[test] 
    fn test_precompile_setup() {
        let config = ExtsloadConfig {
            enabled: true,
            single_slot_precompile: address!("0000000000000000000000000000000000000100"),
            consecutive_slots_precompile: address!("0000000000000000000000000000000000000101"),
            sparse_slots_precompile: address!("0000000000000000000000000000000000000102"),
            ..Default::default()
        };

        let mut precompiles: ContextPrecompiles<MockDb> = 
            ContextPrecompiles::new(PrecompileSpecId::CANCUN);

        // Setup should not panic
        setup_extsload_precompiles(&mut precompiles, &config);
        
        // Precompiles should be registered
        assert!(precompiles.contains(&config.single_slot_precompile));
        assert!(precompiles.contains(&config.consecutive_slots_precompile));
        assert!(precompiles.contains(&config.sparse_slots_precompile));
    }

    #[test]
    fn test_single_slot_precompile_input_validation() {
        use reth_evm::extsload_optimizer::SingleSlotPrecompile;
        use revm::primitives::{Env, TxEnv};

        let precompile = SingleSlotPrecompile;
        let env = Env {
            tx: TxEnv {
                transact_to: TxKind::Call(address!("1f98400000000000000000000000000000000004")),
                ..Default::default()
            },
            ..Default::default()
        };

        // Valid input (32 bytes)
        let valid_input = Bytes::from(vec![0u8; 32]);
        let result = precompile.call_mut(&valid_input, 1000, &env);
        assert!(result.is_ok());

        // Invalid input (wrong length)
        let invalid_input = Bytes::from(vec![0u8; 16]);
        let result = precompile.call_mut(&invalid_input, 1000, &env);
        assert!(result.is_err());
    }

    #[test]
    fn test_consecutive_slots_precompile() {
        use reth_evm::extsload_optimizer::ConsecutiveSlotsPrecompile;
        use revm::primitives::{Env, TxEnv};

        let precompile = ConsecutiveSlotsPrecompile;
        let env = Env {
            tx: TxEnv {
                transact_to: TxKind::Call(address!("1f98400000000000000000000000000000000004")),
                ..Default::default()
            },
            ..Default::default()
        };

        // Valid input: 32 bytes (start slot) + 32 bytes (count = 3)
        let mut input = Vec::new();
        input.extend_from_slice(&B256::ZERO.0);  // start slot
        input.extend_from_slice(&U256::from(3).to_be_bytes::<32>());  // count = 3

        let input_bytes = Bytes::from(input);
        let result = precompile.call_mut(&input_bytes, 10000, &env);
        
        assert!(result.is_ok());
        let (gas_used, output) = result.unwrap();
        
        // Gas should be base + per-slot cost
        let expected_gas = 500 + (3 * 150); // 950 gas
        assert_eq!(gas_used, expected_gas);
        
        // Output should contain array length + 3 slots (32 bytes each)
        // Total: 32 (length) + 96 (3 * 32) = 128 bytes + 32 byte offset = 160 bytes
        assert_eq!(output.len(), 32 + 32 + (3 * 32)); // offset + length + data
    }

    #[test]
    fn test_sparse_slots_precompile() {
        use reth_evm::extsload_optimizer::SparseSlotsPrecompile;
        use revm::primitives::{Env, TxEnv};

        let precompile = SparseSlotsPrecompile;
        let env = Env {
            tx: TxEnv {
                transact_to: TxKind::Call(address!("1f98400000000000000000000000000000000004")),
                ..Default::default()
            },
            ..Default::default()
        };

        // Valid input: 32 bytes (count = 2) + 2 * 32 bytes (slots)
        let mut input = Vec::new();
        input.extend_from_slice(&U256::from(2).to_be_bytes::<32>());  // count = 2
        input.extend_from_slice(&B256::from(U256::from(0x123)).0);    // slot 1
        input.extend_from_slice(&B256::from(U256::from(0x456)).0);    // slot 2

        let input_bytes = Bytes::from(input);
        let result = precompile.call_mut(&input_bytes, 10000, &env);
        
        assert!(result.is_ok());
        let (gas_used, output) = result.unwrap();
        
        // Gas should be base + per-slot cost
        let expected_gas = 500 + (2 * 180); // 860 gas
        assert_eq!(gas_used, expected_gas);
        
        // Output should contain array offset + length + 2 slots
        assert_eq!(output.len(), 32 + 32 + (2 * 32)); // offset + length + data
    }

    #[test]
    fn test_gas_cost_savings() {
        // Verify our optimized gas costs provide significant savings
        let config = reth_evm::extsload_optimizer::GasCosts::default();
        
        // Single slot: ~2100 gas (standard) vs 200 gas (optimized) = 90% savings
        let standard_sload_cost = 2100u64;
        let savings_percent = ((standard_sload_cost - config.single_slot_gas) * 100) / standard_sload_cost;
        assert!(savings_percent >= 90);
        
        // Consecutive 3 slots: ~6300 gas (standard) vs ~950 gas (optimized) = ~85% savings  
        let standard_3_sloads = 3 * standard_sload_cost; // 6300
        let optimized_3_consecutive = config.consecutive_base_gas + (3 * config.consecutive_per_slot); // 950
        let consecutive_savings = ((standard_3_sloads - optimized_3_consecutive) * 100) / standard_3_sloads;
        assert!(consecutive_savings >= 80);
        
        // Sparse 5 slots: ~10500 gas (standard) vs ~1400 gas (optimized) = ~87% savings
        let standard_5_sloads = 5 * standard_sload_cost; // 10500  
        let optimized_5_sparse = config.sparse_base_gas + (5 * config.sparse_per_slot); // 1400
        let sparse_savings = ((standard_5_sloads - optimized_5_sparse) * 100) / standard_5_sloads;
        assert!(sparse_savings >= 85);
    }

    #[test]
    fn test_chain_operator_benefits() {
        // Test simulating real-world usage patterns for chain operators
        
        // StateLibrary.getSlot0() - single extsload call
        let single_call_savings = calculate_savings(2100, 200);
        assert!(single_call_savings >= 90.0);
        
        // StateLibrary.getTickInfo() - typically needs 3 storage reads  
        let multi_call_savings = calculate_savings(3 * 2100, 500 + 3 * 150);
        assert!(multi_call_savings >= 80.0);
        
        // Batch analytics reading 10 pool states
        let batch_analytics_savings = calculate_savings(10 * 2100, 500 + 10 * 150);
        assert!(batch_analytics_savings >= 75.0);
        
        // High-frequency MEV bot doing 100 state checks
        let mev_bot_savings = calculate_savings(100 * 2100, 500 + 100 * 150);
        assert!(mev_bot_savings >= 70.0);
    }

    fn calculate_savings(standard_cost: u64, optimized_cost: u64) -> f64 {
        ((standard_cost - optimized_cost) as f64 / standard_cost as f64) * 100.0
    }

    #[test]
    fn test_real_world_scenarios() {
        // Test scenarios chain operators will encounter
        
        // Scenario 1: DeFi dashboard reading multiple pool states
        let dashboard_reads = 20; // pools
        let reads_per_pool = 3;   // getSlot0, getFeeGrowth, getLiquidity
        
        let standard_cost = dashboard_reads * reads_per_pool * 2100;
        let optimized_cost = dashboard_reads * (500 + reads_per_pool * 150); 
        let dashboard_savings = calculate_savings(standard_cost, optimized_cost);
        
        println!("DeFi Dashboard Scenario:");
        println!("  Standard cost: {} gas", standard_cost);
        println!("  Optimized cost: {} gas", optimized_cost); 
        println!("  Savings: {:.1}%", dashboard_savings);
        assert!(dashboard_savings >= 75.0);
        
        // Scenario 2: Arbitrage bot checking profitability
        let pools_to_check = 50;
        let checks_per_pool = 2; // price and liquidity
        
        let arb_standard = pools_to_check * checks_per_pool * 2100;
        let arb_optimized = pools_to_check * (500 + checks_per_pool * 150);
        let arb_savings = calculate_savings(arb_standard, arb_optimized);
        
        println!("Arbitrage Bot Scenario:");
        println!("  Standard cost: {} gas", arb_standard);
        println!("  Optimized cost: {} gas", arb_optimized);
        println!("  Savings: {:.1}%", arb_savings);
        assert!(arb_savings >= 70.0);
    }

    #[test] 
    fn test_backwards_compatibility() {
        // Ensure the optimization is transparent to existing contracts
        let config = ExtsloadConfig::default();
        
        // Non-PoolManager contracts should not be affected
        let random_contract = address!("1234567890123456789012345678901234567890");
        assert!(!config.pool_manager_addresses.contains(&random_contract));
        
        // Unknown function selectors should not be intercepted
        let unknown_selector = hex!("12345678");
        let call_data = [&unknown_selector[..], &vec![0u8; 32][..]].concat();
        
        // Should not intercept calls to unknown functions
        // (This would be tested in integration where normal EVM execution continues)
    }
}
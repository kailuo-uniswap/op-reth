//! Configuration loader for Uniswap v4 extsload optimizations

use alloy_primitives::Address;
use reth_evm::extsload_optimizer::{ExtsloadConfig, GasCosts};
use serde::Deserialize;
use std::{fs, path::Path};

/// TOML configuration structure
#[derive(Debug, Deserialize)]
pub struct TomlConfig {
    pub extsload_optimization: ExtsloadOptimization,
    pub precompile_addresses: PrecompileAddresses,
    pub gas_costs: Option<TomlGasCosts>,
    pub monitoring: Option<Monitoring>,
    pub expected_savings: Option<ExpectedSavings>,
}

#[derive(Debug, Deserialize)]
pub struct ExtsloadOptimization {
    pub enabled: bool,
    pub pool_manager_addresses: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct PrecompileAddresses {
    pub single_slot: String,
    pub consecutive_slots: String,
    pub sparse_slots: String,
}

#[derive(Debug, Deserialize)]
pub struct TomlGasCosts {
    pub single_slot_gas: Option<u64>,
    pub consecutive_base_gas: Option<u64>,
    pub consecutive_per_slot: Option<u64>,
    pub sparse_base_gas: Option<u64>,
    pub sparse_per_slot: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct Monitoring {
    pub collect_metrics: bool,
    pub log_intercepted_calls: bool,
}

#[derive(Debug, Deserialize)]
pub struct ExpectedSavings {
    pub single_extsload: String,
    pub batch_extsload: String,
    #[serde(rename = "stateLibrary_calls")]
    pub state_library_calls: String,
}

impl TomlConfig {
    /// Load configuration from a TOML file
    pub fn from_file<P: AsRef<Path>>(path: P) -> eyre::Result<Self> {
        let content = fs::read_to_string(path)?;
        let config: TomlConfig = toml::from_str(&content)?;
        Ok(config)
    }

    /// Convert to ExtsloadConfig for use in op-reth
    pub fn to_extsload_config(self) -> eyre::Result<ExtsloadConfig> {
        let pool_manager_addresses: Result<Vec<Address>, _> = self
            .extsload_optimization
            .pool_manager_addresses
            .iter()
            .map(|s| s.parse())
            .collect();

        let gas_costs = if let Some(toml_costs) = self.gas_costs {
            GasCosts {
                single_slot_gas: toml_costs.single_slot_gas.unwrap_or(200),
                consecutive_base_gas: toml_costs.consecutive_base_gas.unwrap_or(500),
                consecutive_per_slot: toml_costs.consecutive_per_slot.unwrap_or(150),
                sparse_base_gas: toml_costs.sparse_base_gas.unwrap_or(500),
                sparse_per_slot: toml_costs.sparse_per_slot.unwrap_or(180),
            }
        } else {
            GasCosts::default()
        };

        Ok(ExtsloadConfig {
            enabled: self.extsload_optimization.enabled,
            pool_manager_addresses: pool_manager_addresses?,
            single_slot_precompile: self.precompile_addresses.single_slot.parse()?,
            consecutive_slots_precompile: self.precompile_addresses.consecutive_slots.parse()?,
            sparse_slots_precompile: self.precompile_addresses.sparse_slots.parse()?,
            gas_costs,
        })
    }

    /// Print configuration summary
    pub fn print_summary(&self) {
        println!("🔧 Extsload Optimization Configuration:");
        println!("   Enabled: {}", self.extsload_optimization.enabled);
        println!("   PoolManager addresses:");
        for addr in &self.extsload_optimization.pool_manager_addresses {
            println!("     - {}", addr);
        }
        println!("   Precompile addresses:");
        println!("     - Single slot: {}", self.precompile_addresses.single_slot);
        println!("     - Consecutive: {}", self.precompile_addresses.consecutive_slots);
        println!("     - Sparse: {}", self.precompile_addresses.sparse_slots);

        if let Some(costs) = &self.gas_costs {
            println!("   Gas costs:");
            if let Some(single) = costs.single_slot_gas {
                println!("     - Single slot: {} gas", single);
            }
            if let Some(base) = costs.consecutive_base_gas {
                println!("     - Consecutive base: {} gas", base);
            }
        }

        if let Some(monitoring) = &self.monitoring {
            println!("   Monitoring:");
            println!("     - Collect metrics: {}", monitoring.collect_metrics);
            println!("     - Log calls: {}", monitoring.log_intercepted_calls);
        }

        if let Some(savings) = &self.expected_savings {
            println!("   Expected savings:");
            println!("     - Single extsload: {}", savings.single_extsload);
            println!("     - Batch extsload: {}", savings.batch_extsload);
            println!("     - StateLibrary calls: {}", savings.state_library_calls);
        }
    }
}

/// Creates a default configuration file
pub fn create_default_config_file<P: AsRef<Path>>(path: P) -> eyre::Result<()> {
    let default_config = r#"# Uniswap v4 Extsload Optimization Configuration
# Copy this file and modify for your chain deployment

[extsload_optimization]
# Enable/disable the optimization
enabled = true

# PoolManager contract addresses to optimize
# Add the addresses of PoolManager contracts deployed on your chain
pool_manager_addresses = [
    "0x1f98400000000000000000000000000000000004",  # Uniswap v4 PoolManager
    # Add additional addresses as needed
]

[precompile_addresses]
# Precompile addresses for optimized functions
# These should be available addresses in your precompile range
single_slot = "0x0000000000000000000000000000000000000100"
consecutive_slots = "0x0000000000000000000000000000000000000101" 
sparse_slots = "0x0000000000000000000000000000000000000102"

[gas_costs]
# Optimized gas costs (much lower than standard SLOAD costs)
single_slot_gas = 200         # vs ~2100 for standard SLOAD
consecutive_base_gas = 500    # base cost for batch operations
consecutive_per_slot = 150    # per additional slot
sparse_base_gas = 500         # base cost for sparse reads
sparse_per_slot = 180         # per slot in sparse read

[monitoring]
# Optional: Enable metrics collection for optimization effectiveness
collect_metrics = true
log_intercepted_calls = false  # Set to true for debugging

# Expected performance improvements
[expected_savings]
single_extsload = "90%"           # 2100 → 200 gas
batch_extsload = "87%"            # 6300 → 800 gas  
stateLibrary_calls = "60-80%"     # Overall StateLibrary improvement
"#;

    fs::write(path, default_config)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_config_loading() {
        let config_content = r#"
[extsload_optimization]
enabled = true
pool_manager_addresses = ["0x1f98400000000000000000000000000000000004"]

[precompile_addresses]
single_slot = "0x0000000000000000000000000000000000000100"
consecutive_slots = "0x0000000000000000000000000000000000000101"
sparse_slots = "0x0000000000000000000000000000000000000102"
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "{}", config_content).unwrap();
        
        let config = TomlConfig::from_file(temp_file.path()).unwrap();
        assert!(config.extsload_optimization.enabled);
        assert_eq!(config.extsload_optimization.pool_manager_addresses.len(), 1);
    }

    #[test]
    fn test_config_conversion() {
        let toml_config = TomlConfig {
            extsload_optimization: ExtsloadOptimization {
                enabled: true,
                pool_manager_addresses: vec!["0x1f98400000000000000000000000000000000004".to_string()],
            },
            precompile_addresses: PrecompileAddresses {
                single_slot: "0x0000000000000000000000000000000000000100".to_string(),
                consecutive_slots: "0x0000000000000000000000000000000000000101".to_string(),
                sparse_slots: "0x0000000000000000000000000000000000000102".to_string(),
            },
            gas_costs: None,
            monitoring: None,
            expected_savings: None,
        };

        let extsload_config = toml_config.to_extsload_config().unwrap();
        assert!(extsload_config.enabled);
        assert_eq!(extsload_config.pool_manager_addresses.len(), 1);
        assert_eq!(extsload_config.gas_costs.single_slot_gas, 200);
    }
}
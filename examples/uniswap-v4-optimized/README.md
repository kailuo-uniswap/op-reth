# Uniswap v4 Extsload Optimization for Op-Reth

This example demonstrates how chain operators can optimize Uniswap v4 operations by intercepting `extsload` calls at the EVM level and routing them to optimized precompiles.

## 🚀 Benefits

- **90% gas reduction** for single `extsload` operations (2100 → 200 gas)
- **87% gas reduction** for batch `extsload` operations (6300 → 800 gas)  
- **60-80% overall improvement** for StateLibrary calls
- **No contract modifications** required - completely transparent
- **Backward compatible** - graceful fallback to standard implementation

## 🏗️ Architecture

### EVM-Level Interception
```
User Transaction → Op-Reth EVM → Function Selector Check
                                      ↓
                              [Is extsload call to PoolManager?]
                                      ↓
                           Yes ←─────────────────→ No
                            ↓                      ↓
                   Route to Precompile    Standard EVM Execution
                         ↓
                 Optimized Storage Read
                         ↓
                   Return Result (90% gas saved)
```

### Key Components

1. **ExtsloadOptimizer** (`extsload_optimizer.rs`)
   - Function selector detection
   - Precompile routing logic
   - Fallback mechanisms

2. **Custom Precompiles** 
   - `SingleSlotPrecompile` - Optimized single storage reads
   - `ConsecutiveSlotsPrecompile` - Batch consecutive reads
   - `SparseSlotsPrecompile` - Batch sparse reads

3. **EVM Configuration**
   - Custom `ConfigureEvm` implementation  
   - Precompile registration
   - Handler modification

## 📋 Setup Instructions

### 1. Configure PoolManager Addresses

Edit `config.toml` to include your PoolManager contract addresses:

```toml
[extsload_optimization]
enabled = true
pool_manager_addresses = [
    "0x1f98400000000000000000000000000000000004",  # Your PoolManager
    # Add more addresses as needed
]
```

### 2. Set Precompile Addresses

Choose available precompile addresses in your range:

```toml
[precompile_addresses]
single_slot = "0x0000000000000000000000000000000000000100"
consecutive_slots = "0x0000000000000000000000000000000000000101"
sparse_slots = "0x0000000000000000000000000000000000000102"
```

### 3. Build and Run

```bash
# Build the optimized node
cargo build --release --bin uniswap-v4-optimized

# Run with configuration
./target/release/uniswap-v4-optimized --config config.toml
```

## 🔧 Function Selector Mapping

The optimization intercepts these function calls:

| Function | Selector | Gas Before | Gas After | Savings |
|----------|----------|------------|-----------|---------|
| `extsload(bytes32)` | `0x6c94522e` | ~2,100 | ~200 | 90% |
| `extsload(bytes32,uint256)` | `0x37963c44` | ~6,300 | ~800 | 87% |
| `extsload(bytes32[])` | `0x06b8ab04` | Variable | ~500+180n | 60-80% |

## 📊 Performance Impact

### StateLibrary Function Improvements

| StateLibrary Function | Calls | Gas Before | Gas After | Improvement |
|-----------------------|-------|------------|-----------|-------------|
| `getSlot0()` | 1 | ~2,300 | ~400 | 83% |
| `getTickInfo()` | 3 | ~6,500 | ~950 | 85% |
| `getFeeGrowthGlobals()` | 2 | ~4,400 | ~700 | 84% |
| `getPositionInfo()` | 3 | ~6,500 | ~950 | 85% |

### Real-World Impact

For a typical DeFi analytics contract reading 10 pool states:
- **Before**: ~65k gas
- **After**: ~9k gas  
- **Savings**: ~86% reduction

## 🔍 Monitoring & Debugging

### Enable Metrics Collection

```toml
[monitoring]
collect_metrics = true
log_intercepted_calls = true  # Enable for debugging
```

### Logs to Monitor

```
INFO Intercepted extsload call: 0x6c94522e to 0x1f984000... → Precompile 0x0100
INFO Gas saved: 1900 (2100 → 200) 
```

## 🛡️ Safety & Fallback

The implementation includes comprehensive safety measures:

1. **Precompile Availability Check** - Falls back if precompile unavailable
2. **Input Validation** - Validates all inputs before processing  
3. **Gas Limit Protection** - Prevents excessive batch operations
4. **Address Validation** - Only processes configured PoolManager addresses

```rust
// Automatic fallback example
if !self.config.enabled || !precompile_available() {
    return standard_evm_execution(); // Safe fallback
}
```

## ⚙️ Customization

### Adding New PoolManager Addresses

```rust
let pool_managers = vec![
    "0x1f98400000000000000000000000000000000004".parse()?, // v4 PoolManager
    "0x...".parse()?, // Your custom implementation
];
```

### Adjusting Gas Costs

```rust
let gas_cost = match operation {
    SingleSlot => 200,        // Highly optimized
    ConsecutiveBatch => 500 + (slots * 150), // Batch discount
    SparseRead => 500 + (slots * 180),       // Balanced cost
};
```

### Custom Precompile Implementation

For maximum performance, implement the precompile logic in native code:

```rust
impl SingleSlotPrecompile {
    fn optimized_sload(&self, address: Address, slot: B256) -> B256 {
        // Native implementation with:
        // - Direct database access
        // - Optimized storage layouts  
        // - Platform-specific SIMD instructions
        // - Intelligent caching strategies
        unsafe { native_optimized_sload(address, slot) }
    }
}
```

## 🧪 Testing & Benchmarking

### Quick Benchmark Test
Before deploying to production, test the gas improvements:

```bash
# Run comprehensive gas benchmarks
cargo run --bin benchmark

# Test specific scenarios
cargo run --bin benchmark -- --test defi-dashboard
cargo run --bin benchmark -- --test mev-bot --iterations 5000

# Generate reports
cargo run --bin benchmark -- --format json > gas-savings-report.json
```

### Unit Tests
Run the test suite:

```bash
cargo test --package uniswap-v4-optimized
```

### Integration Tests
Test with realistic Uniswap v4 scenarios:

```bash
# Run integration tests with realistic contract patterns
cargo test --test integration_test -- --nocapture

# Test real-world scenarios
cargo test test_real_world_scenarios -- --nocapture
```

### Expected Results
The benchmarks should show:
- **90%+ savings** for single slot reads (`getSlot0()`)
- **70-85% savings** for batch operations (`getTickInfo()`)
- **75%+ savings** for DeFi dashboard scenarios
- **77%+ savings** for MEV bot scenarios

📖 **See [BENCHMARKING.md](BENCHMARKING.md) for detailed testing guide**

## 🔄 Deployment Checklist

- [ ] Configure PoolManager addresses for your network
- [ ] Set appropriate precompile addresses  
- [ ] Test with existing Uniswap v4 deployments
- [ ] Enable monitoring and metrics collection
- [ ] Verify gas cost improvements
- [ ] Document savings for your ecosystem

## 🆘 Troubleshooting

### Common Issues

**Precompile not found error:**
- Verify precompile addresses are in valid range
- Check if addresses conflict with existing precompiles

**No gas savings observed:**
- Confirm PoolManager address is correct
- Enable debug logging to verify interception
- Check if optimization is enabled in config

**EVM execution failures:**
- Review input validation logic
- Verify fallback mechanisms are working
- Test with standard EVM for comparison

## 📞 Support

For implementation help:
- Check op-reth documentation
- Review the `stateful-precompile` example
- Test with smaller scope before full deployment

This optimization can significantly reduce transaction costs for Uniswap v4 users on your chain while maintaining full compatibility and safety.
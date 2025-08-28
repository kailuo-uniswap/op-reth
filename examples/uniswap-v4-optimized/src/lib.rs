//! Uniswap v4 Extsload Optimization for Op-Reth
//!
//! This library provides EVM-level optimizations for Uniswap v4 extsload operations,
//! delivering significant gas savings without requiring contract modifications.

pub mod standalone_benchmark;

pub use standalone_benchmark::*;
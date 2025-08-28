//! Standalone benchmark tool for testing extsload optimizations
//!
//! This tool demonstrates the gas improvements from EVM-level extsload optimizations

use clap::{Arg, Command};
use eyre::Result;
use serde::Serialize;
use std::time::Instant;
use uniswap_v4_optimized::standalone_benchmark::{StandaloneBenchmark, print_benchmark_summary, BenchmarkResult};

#[derive(Debug, Clone, Serialize)]
struct JsonResults {
    benchmark_run: BenchmarkRun,
    results: Vec<BenchmarkResult>,
    summary: BenchmarkSummary,
}

#[derive(Debug, Clone, Serialize)]
struct BenchmarkRun {
    timestamp: String,
    iterations_per_test: usize,
    total_duration_ms: u128,
}

#[derive(Debug, Clone, Serialize)]
struct BenchmarkSummary {
    total_tests: usize,
    total_standard_gas: u64,
    total_optimized_gas: u64,
    total_savings_gas: u64,
    overall_savings_percent: f64,
    best_performing_test: String,
    best_savings_percent: f64,
}

fn main() -> Result<()> {
    let matches = Command::new("Uniswap v4 Extsload Benchmark")
        .version("1.0.0")
        .about("Benchmark tool for measuring extsload optimization gas improvements")
        .arg(
            Arg::new("iterations")
                .long("iterations")
                .short('i')
                .value_name("N")
                .help("Number of iterations per test")
                .default_value("1000"),
        )
        .arg(
            Arg::new("test")
                .long("test")
                .short('t')
                .value_name("TEST_NAME")
                .help("Run specific test only")
                .value_parser([
                    "single-slot",
                    "consecutive-3",
                    "consecutive-5", 
                    "consecutive-10",
                    "sparse-3",
                    "sparse-5",
                    "statelib-getslot0",
                    "statelib-gettickinfo",
                    "defi-dashboard",
                    "mev-bot",
                ]),
        )
        .arg(
            Arg::new("format")
                .long("format")
                .short('f')
                .value_name("FORMAT")
                .help("Output format")
                .value_parser(["human", "json", "csv"])
                .default_value("human"),
        )
        .arg(
            Arg::new("quiet")
                .long("quiet")
                .short('q')
                .help("Suppress individual test output, show only summary")
                .action(clap::ArgAction::SetTrue),
        )
        .get_matches();

    let iterations: usize = matches.get_one::<String>("iterations").unwrap().parse()?;
    let specific_test = matches.get_one::<String>("test");
    let format = matches.get_one::<String>("format").unwrap();
    let quiet = matches.get_flag("quiet");

    if !quiet {
        print_banner();
        print_config_info(iterations);
    }

    let start_time = Instant::now();
    let benchmark = StandaloneBenchmark::new(iterations);

    // Run benchmarks
    let results = if let Some(test_name) = specific_test {
        run_specific_test(&benchmark, test_name, quiet)
    } else {
        run_all_tests(&benchmark, quiet)
    };

    let duration = start_time.elapsed();

    // Output results
    match format.as_str() {
        "json" => output_json(results, iterations, duration)?,
        "csv" => output_csv(results)?,
        _ => {
            if !quiet {
                print_benchmark_summary(&results);
            }
            output_human_summary(&results, duration);
        }
    }

    Ok(())
}

fn print_banner() {
    println!("Extsload Optimization Benchmark");
}

fn print_config_info(iterations: usize) {
    println!("Iterations: {}", iterations);
}

fn run_all_tests(benchmark: &StandaloneBenchmark, quiet: bool) -> Vec<BenchmarkResult> {
    benchmark.run_all_benchmarks()
}

fn run_specific_test(benchmark: &StandaloneBenchmark, test_name: &str, quiet: bool) -> Vec<BenchmarkResult> {

    vec![match test_name {
        "single-slot" => benchmark.benchmark_single_slot(),
        "consecutive-3" => benchmark.benchmark_consecutive_slots(3),
        "consecutive-5" => benchmark.benchmark_consecutive_slots(5),
        "consecutive-10" => benchmark.benchmark_consecutive_slots(10),
        "sparse-3" => benchmark.benchmark_sparse_slots(3),
        "sparse-5" => benchmark.benchmark_sparse_slots(5),
        "statelib-getslot0" => benchmark.benchmark_statelib_getslot0(),
        "statelib-gettickinfo" => benchmark.benchmark_statelib_gettickinfo(),
        "defi-dashboard" => benchmark.benchmark_defi_dashboard(),
        "mev-bot" => benchmark.benchmark_mev_bot_scenario(),
        _ => {
            eprintln!("❌ Unknown test: {}", test_name);
            std::process::exit(1);
        }
    }]
}

fn output_human_summary(results: &[BenchmarkResult], duration: std::time::Duration) {
    let total_standard: u64 = results.iter().map(|r| r.standard_gas).sum();
    let total_optimized: u64 = results.iter().map(|r| r.optimized_gas).sum();
    let total_savings = total_standard.saturating_sub(total_optimized);
    let overall_percent = if total_standard > 0 {
        (total_savings as f64 / total_standard as f64) * 100.0
    } else {
        0.0
    };

    println!("Standard Gas:  {} gas", total_standard);
    println!("Optimized Gas: {} gas", total_optimized);
    println!("Gas Savings:   {} gas ({:.1}%)", total_savings, overall_percent);
}

fn output_json(
    results: Vec<BenchmarkResult>, 
    iterations: usize,
    duration: std::time::Duration,
) -> Result<()> {
    let total_standard: u64 = results.iter().map(|r| r.standard_gas).sum();
    let total_optimized: u64 = results.iter().map(|r| r.optimized_gas).sum();
    let total_savings = total_standard.saturating_sub(total_optimized);
    let overall_percent = if total_standard > 0 {
        (total_savings as f64 / total_standard as f64) * 100.0
    } else {
        0.0
    };

    let best_test = results
        .iter()
        .max_by(|a, b| a.savings_percent.partial_cmp(&b.savings_percent).unwrap())
        .map(|r| (r.test_name.clone(), r.savings_percent))
        .unwrap_or(("None".to_string(), 0.0));

    let json_results = JsonResults {
        benchmark_run: BenchmarkRun {
            timestamp: chrono::Utc::now().to_rfc3339(),
            iterations_per_test: iterations,
            total_duration_ms: duration.as_millis(),
        },
        summary: BenchmarkSummary {
            total_tests: results.len(),
            total_standard_gas: total_standard,
            total_optimized_gas: total_optimized,
            total_savings_gas: total_savings,
            overall_savings_percent: overall_percent,
            best_performing_test: best_test.0,
            best_savings_percent: best_test.1,
        },
        results,
    };

    println!("{}", serde_json::to_string_pretty(&json_results)?);
    Ok(())
}

fn output_csv(results: Vec<BenchmarkResult>) -> Result<()> {
    println!("test_name,standard_gas,optimized_gas,gas_savings,savings_percent,sload_count,iterations");
    
    for result in results {
        println!(
            "{},{},{},{},{:.2},{},{}",
            result.test_name,
            result.standard_gas,
            result.optimized_gas,
            result.gas_savings,
            result.savings_percent,
            result.sload_count,
            result.iterations
        );
    }
    
    Ok(())
}
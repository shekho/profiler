//! A basic Rust-based profiler that listens to perf_events and tracepoints.
//!
//! This profiler uses Microsoft's LinuxTracepoints-Rust crates for tracepoint handling
//! and the perf-event crate for live perf event monitoring.

mod perf;
mod tracepoint;

use anyhow::Result;
use clap::{Parser, Subcommand};

/// A basic Rust-based profiler for perf_events and tracepoints
#[derive(Parser)]
#[command(name = "profiler")]
#[command(about = "A Rust-based profiler using perf_events and tracepoints", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Profile using hardware performance counters
    Perf {
        /// Duration in seconds to collect samples
        #[arg(short, long, default_value = "5")]
        duration: u64,

        /// Target PID to profile (0 for current process)
        #[arg(short, long, default_value = "0")]
        pid: i32,
    },

    /// Read and decode a perf.data file containing tracepoint events
    Tracepoint {
        /// Path to the perf.data file
        #[arg(short, long)]
        file: String,
    },

    /// Show available hardware events
    ListEvents,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Perf { duration, pid } => {
            perf::run_perf_profiler(duration, pid)?;
        }
        Commands::Tracepoint { file } => {
            tracepoint::read_tracepoint_file(&file)?;
        }
        Commands::ListEvents => {
            perf::list_available_events();
        }
    }

    Ok(())
}

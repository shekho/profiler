//! Perf event profiling module.
//!
//! This module provides functionality to collect hardware performance counter data
//! using the Linux perf_event subsystem.

use anyhow::{Context, Result};
use perf_event::events::Hardware;
use perf_event::{Builder, Group};
use std::thread;
use std::time::Duration;

/// Available hardware performance events that can be monitored.
pub struct PerfEvent {
    pub name: &'static str,
    pub description: &'static str,
}

/// List of commonly available hardware performance events.
pub const HARDWARE_EVENTS: &[PerfEvent] = &[
    PerfEvent {
        name: "cpu-cycles",
        description: "Total CPU cycles",
    },
    PerfEvent {
        name: "instructions",
        description: "Retired instructions",
    },
    PerfEvent {
        name: "cache-references",
        description: "Cache references",
    },
    PerfEvent {
        name: "cache-misses",
        description: "Cache misses",
    },
    PerfEvent {
        name: "branch-instructions",
        description: "Branch instructions",
    },
    PerfEvent {
        name: "branch-misses",
        description: "Branch mispredictions",
    },
];

/// Print a list of available hardware events.
pub fn list_available_events() {
    println!("Available hardware performance events:");
    println!("{:-<50}", "");
    for event in HARDWARE_EVENTS {
        println!("  {:<25} - {}", event.name, event.description);
    }
    println!();
    println!("Note: Availability depends on your CPU and kernel configuration.");
    println!("Some events may require root privileges or specific perf_event_paranoid settings.");
}

/// Results from a perf profiling session.
#[derive(Debug)]
pub struct ProfilingResult {
    pub cpu_cycles: u64,
    pub instructions: u64,
    pub cache_references: u64,
    pub cache_misses: u64,
    pub duration_secs: u64,
}

impl ProfilingResult {
    /// Calculate instructions per cycle (IPC).
    pub fn ipc(&self) -> f64 {
        if self.cpu_cycles == 0 {
            0.0
        } else {
            self.instructions as f64 / self.cpu_cycles as f64
        }
    }

    /// Calculate cache miss rate.
    pub fn cache_miss_rate(&self) -> f64 {
        if self.cache_references == 0 {
            0.0
        } else {
            self.cache_misses as f64 / self.cache_references as f64 * 100.0
        }
    }

    /// Calculate CPU cycles per second.
    pub fn cycles_per_second(&self) -> f64 {
        if self.duration_secs == 0 {
            0.0
        } else {
            self.cpu_cycles as f64 / self.duration_secs as f64
        }
    }
}

/// Run the perf profiler for a specified duration.
///
/// # Arguments
///
/// * `duration_secs` - Duration in seconds to collect performance data
/// * `pid` - Target process ID (0 for current process)
///
/// # Returns
///
/// Returns a `ProfilingResult` containing the collected performance counters.
pub fn run_perf_profiler(duration_secs: u64, pid: i32) -> Result<ProfilingResult> {
    println!("Starting perf profiler...");
    println!("Duration: {} seconds", duration_secs);
    if pid == 0 {
        println!("Target: Current process");
    } else {
        println!("Target: PID {}", pid);
    }
    println!();

    // Create a group to collect multiple counters atomically
    let mut group = Group::new().context("Failed to create perf event group")?;

    // Set up hardware counters
    let cycles = Builder::new()
        .group(&mut group)
        .kind(Hardware::CPU_CYCLES)
        .build()
        .context("Failed to create CPU cycles counter")?;

    let instructions = Builder::new()
        .group(&mut group)
        .kind(Hardware::INSTRUCTIONS)
        .build()
        .context("Failed to create instructions counter")?;

    let cache_refs = Builder::new()
        .group(&mut group)
        .kind(Hardware::CACHE_REFERENCES)
        .build()
        .context("Failed to create cache references counter")?;

    let cache_misses = Builder::new()
        .group(&mut group)
        .kind(Hardware::CACHE_MISSES)
        .build()
        .context("Failed to create cache misses counter")?;

    // Enable counters and collect data
    println!("Collecting performance data...");
    group.enable().context("Failed to enable perf counters")?;

    // Sleep for the specified duration while counters are active
    thread::sleep(Duration::from_secs(duration_secs));

    group.disable().context("Failed to disable perf counters")?;

    // Read the counter values
    let counts = group.read().context("Failed to read perf counters")?;

    let result = ProfilingResult {
        cpu_cycles: counts[&cycles],
        instructions: counts[&instructions],
        cache_references: counts[&cache_refs],
        cache_misses: counts[&cache_misses],
        duration_secs,
    };

    // Print results
    println!();
    println!("Profiling Results:");
    println!("{:=<50}", "");
    println!("  CPU Cycles:        {:>15}", result.cpu_cycles);
    println!("  Instructions:      {:>15}", result.instructions);
    println!("  Cache References:  {:>15}", result.cache_references);
    println!("  Cache Misses:      {:>15}", result.cache_misses);
    println!("{:-<50}", "");
    println!("  IPC:               {:>15.3}", result.ipc());
    println!("  Cache Miss Rate:   {:>14.2}%", result.cache_miss_rate());
    println!("{:=<50}", "");

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_profiling_result_ipc() {
        let result = ProfilingResult {
            cpu_cycles: 1000,
            instructions: 500,
            cache_references: 100,
            cache_misses: 10,
            duration_secs: 1,
        };
        assert!((result.ipc() - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn test_profiling_result_ipc_zero_cycles() {
        let result = ProfilingResult {
            cpu_cycles: 0,
            instructions: 500,
            cache_references: 100,
            cache_misses: 10,
            duration_secs: 1,
        };
        assert!((result.ipc() - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_profiling_result_cache_miss_rate() {
        let result = ProfilingResult {
            cpu_cycles: 1000,
            instructions: 500,
            cache_references: 100,
            cache_misses: 10,
            duration_secs: 1,
        };
        assert!((result.cache_miss_rate() - 10.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_profiling_result_cache_miss_rate_zero_refs() {
        let result = ProfilingResult {
            cpu_cycles: 1000,
            instructions: 500,
            cache_references: 0,
            cache_misses: 10,
            duration_secs: 1,
        };
        assert!((result.cache_miss_rate() - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_profiling_result_cycles_per_second() {
        let result = ProfilingResult {
            cpu_cycles: 1000,
            instructions: 500,
            cache_references: 100,
            cache_misses: 10,
            duration_secs: 2,
        };
        assert!((result.cycles_per_second() - 500.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_profiling_result_cycles_per_second_zero_duration() {
        let result = ProfilingResult {
            cpu_cycles: 1000,
            instructions: 500,
            cache_references: 100,
            cache_misses: 10,
            duration_secs: 0,
        };
        assert!((result.cycles_per_second() - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_list_available_events_runs() {
        // Just verify it doesn't panic
        list_available_events();
    }
}

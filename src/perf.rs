//! Perf event profiling module.
//!
//! This module provides functionality to collect hardware performance counter data
//! using the Linux perf_event subsystem, as well as CPU profiling with callchain/stacktrace
//! support using microsoft/one-collect.

use anyhow::{Context, Result};
use one_collect::perf_event::{RingBufBuilder, RingBufOptions, RingBufSessionBuilder};
use perf_event::events::Hardware;
use perf_event::{Builder, Group};
use std::cell::Cell;
use std::rc::Rc;
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
/// * `_pid` - Target process ID (currently unused, always profiles current process)
///
/// # Returns
///
/// Returns a `ProfilingResult` containing the collected performance counters.
///
/// # Note
///
/// Currently only profiles the current process. PID targeting is not yet implemented.
pub fn run_perf_profiler(duration_secs: u64, _pid: i32) -> Result<ProfilingResult> {
    println!("Starting perf profiler...");
    println!("Duration: {} seconds", duration_secs);
    println!("Target: Current process (PID targeting not yet implemented)");
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

/// Results from a CPU profiling session with callchain/stacktrace data.
#[derive(Debug, Default)]
pub struct CallchainProfilingResult {
    /// Total number of samples collected
    pub sample_count: u64,
    /// Duration of the profiling session in seconds
    pub duration_secs: u64,
    /// Sampling frequency used (Hz)
    pub sampling_frequency: u64,
}

/// Run CPU profiler with callchain/stacktrace collection using microsoft/one-collect.
///
/// This function collects CPU profiling samples with full callchain (stack trace) data
/// using the perf_event subsystem via the one_collect crate.
///
/// # Arguments
///
/// * `duration_secs` - Duration in seconds to collect profiling data
/// * `pid` - Target process ID (-1 for all processes, 0 for current process)
/// * `sampling_frequency` - Sampling frequency in Hz (e.g., 99 for 99 samples/second)
///
/// # Returns
///
/// Returns a `CallchainProfilingResult` containing the profiling statistics.
///
/// # Example
///
/// ```no_run
/// use profiler::perf::run_callchain_profiler;
///
/// // Profile for 5 seconds at 99 Hz
/// let result = run_callchain_profiler(5, 0, 99).unwrap();
/// println!("Collected {} samples", result.sample_count);
/// ```
pub fn run_callchain_profiler(
    duration_secs: u64,
    pid: i32,
    sampling_frequency: u64,
) -> Result<CallchainProfilingResult> {
    println!("Starting callchain profiler with one_collect...");
    println!("Duration: {} seconds", duration_secs);
    println!("Sampling frequency: {} Hz", sampling_frequency);
    println!("Target PID: {}", if pid == -1 { "all".to_string() } else if pid == 0 { "current".to_string() } else { pid.to_string() });
    println!();

    // Create a profiling builder with callchain support
    let profiling_builder = RingBufBuilder::for_profiling(sampling_frequency)
        .with_callchain_data()
        .with_ip();

    // Build the session
    let mut session_builder = RingBufSessionBuilder::new()
        .with_page_count(64) // 64 pages for ring buffer
        .with_profiling_events(profiling_builder);

    // Add target PID if specified (not -1 for all)
    if pid >= 0 {
        session_builder = session_builder.with_target_pid(pid);
    }

    let mut session = session_builder
        .build()
        .context("Failed to build perf session")?;

    // Set up sample counter using Rc<Cell> for interior mutability in callback
    let sample_count = Rc::new(Cell::new(0u64));
    let sample_count_clone = sample_count.clone();

    // Add callback to the CPU profile event to count samples
    session.cpu_profile_event().add_callback(move |_event_data| {
        sample_count_clone.set(sample_count_clone.get() + 1);
        Ok(())
    });

    // Enable the session and collect data
    println!("Collecting callchain profiling data...");
    session.enable().context("Failed to enable perf session")?;

    // Parse events for the specified duration
    let duration = Duration::from_secs(duration_secs);
    session
        .parse_for_duration(duration)
        .context("Failed to parse perf events")?;

    session.disable().context("Failed to disable perf session")?;

    let result = CallchainProfilingResult {
        sample_count: sample_count.get(),
        duration_secs,
        sampling_frequency,
    };

    // Print results
    println!();
    println!("Callchain Profiling Results:");
    println!("{:=<50}", "");
    println!("  Samples Collected: {:>15}", result.sample_count);
    println!("  Duration:          {:>12} s", result.duration_secs);
    println!("  Sampling Freq:     {:>12} Hz", result.sampling_frequency);
    println!(
        "  Effective Rate:    {:>12.1} samples/s",
        result.sample_count as f64 / result.duration_secs as f64
    );
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

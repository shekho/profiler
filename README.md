# Profiler

A basic Rust-based profiler that listens to perf_events and tracepoints using Microsoft's LinuxTracepoints-Rust crates.

## Features

- **Hardware Performance Counters**: Collect CPU cycles, instructions, cache references/misses, and branch statistics using Linux perf_events
- **Tracepoint Decoding**: Read and decode perf.data files containing tracepoint events
- **EventHeader Support**: Full support for Microsoft EventHeader-encoded tracepoints

## Installation

```bash
cargo build --release
```

## Usage

### List Available Events

Show the hardware performance events that can be monitored:

```bash
./target/release/profiler list-events
```

### Profile Using Hardware Counters

Collect hardware performance data for a specified duration:

```bash
# Profile for 5 seconds (default)
./target/release/profiler perf

# Profile for 10 seconds
./target/release/profiler perf --duration 10

# Profile a specific process (not yet implemented, profiles current process)
./target/release/profiler perf --pid 1234
```

**Note**: Requires appropriate permissions. You may need to adjust `/proc/sys/kernel/perf_event_paranoid`:

```bash
# Allow unprivileged users to collect performance data (temporary)
sudo sysctl kernel.perf_event_paranoid=-1
```

### Read Tracepoint Data

Decode a perf.data file containing tracepoint events:

```bash
./target/release/profiler tracepoint --file perf.data
```

## Dependencies

This profiler uses the following key crates:

- **[perf-event](https://crates.io/crates/perf-event)**: Rust interface to Linux performance monitoring
- **[tracepoint_perf](https://crates.io/crates/tracepoint_perf)**: Microsoft's Rust API for reading perf.data files
- **[tracepoint_decode](https://crates.io/crates/tracepoint_decode)**: Microsoft's Rust API for decoding tracepoints

## Example Output

```
$ ./target/release/profiler perf --duration 1

Starting perf profiler...
Duration: 1 seconds
Target: Current process

Collecting performance data...

Profiling Results:
==================================================
  CPU Cycles:              1234567890
  Instructions:             987654321
  Cache References:           1234567
  Cache Misses:                 12345
--------------------------------------------------
  IPC:                          0.800
  Cache Miss Rate:              1.00%
==================================================
```

## License

MIT
//! Tracepoint handling module.
//!
//! This module provides functionality to read and decode perf.data files
//! containing tracepoint events using Microsoft's LinuxTracepoints-Rust crates.

use anyhow::{Context, Result};
use std::path::Path;
use tracepoint_decode::{self as td, PerfEventHeaderType};
use tracepoint_perf::{PerfDataFileEventOrder, PerfDataFileReader, PerfHeaderIndex};

/// Statistics about the tracepoint data file.
#[derive(Debug, Default)]
pub struct TracepointStats {
    pub total_events: u64,
    pub sample_events: u64,
    pub non_sample_events: u64,
}

/// Read and decode a perf.data file containing tracepoint events.
///
/// # Arguments
///
/// * `file_path` - Path to the perf.data file
///
/// # Returns
///
/// Returns statistics about the events found in the file.
pub fn read_tracepoint_file(file_path: &str) -> Result<TracepointStats> {
    let path = Path::new(file_path);
    if !path.exists() {
        anyhow::bail!("File not found: {}", file_path);
    }

    println!("Reading tracepoint data from: {}", file_path);
    println!();

    // Create the reader
    let mut reader = PerfDataFileReader::new();

    // Open the file with time-ordered events
    reader
        .open_file(file_path, PerfDataFileEventOrder::Time)
        .context("Failed to open perf.data file")?;

    let mut stats = TracepointStats::default();

    // Print header information
    println!("File Information:");
    println!("{:-<50}", "");

    let hostname = reader.header_string(PerfHeaderIndex::Hostname);
    if !hostname.is_empty() {
        println!("  Hostname: {}", String::from_utf8_lossy(hostname));
    }

    let os_release = reader.header_string(PerfHeaderIndex::OSRelease);
    if !os_release.is_empty() {
        println!("  OS Release: {}", String::from_utf8_lossy(os_release));
    }

    let arch = reader.header_string(PerfHeaderIndex::Arch);
    if !arch.is_empty() {
        println!("  Architecture: {}", String::from_utf8_lossy(arch));
    }
    println!();

    // Print event descriptors
    println!("Event Descriptors:");
    println!("{:-<50}", "");
    for desc in reader.event_desc_list() {
        println!("  Event: {}", desc.name());
        for id in desc.ids() {
            println!("    ID: {}", id);
        }
    }
    println!();

    // Create an enumerator context for EventHeader decoding
    let mut enumerator_ctx = td::EventHeaderEnumeratorContext::new();

    // Read and process events
    println!("Processing events...");
    println!("{:-<50}", "");

    let mut sample_count = 0;

    loop {
        match reader.move_next_event() {
            Err(e) => {
                anyhow::bail!("Error reading event: {}", e);
            }
            Ok(false) => break, // EOF
            Ok(true) => {}      // Got an event
        }

        let event = reader.current_event();
        stats.total_events += 1;

        if event.header.ty != PerfEventHeaderType::Sample {
            // Non-sample event
            stats.non_sample_events += 1;

            // Only print first few non-sample events
            if stats.non_sample_events <= 3 {
                println!("  Non-sample event: {}", event.header.ty);
                println!("    Size: {} bytes", event.header.size);
            }
        } else {
            // Sample event (tracepoint)
            stats.sample_events += 1;
            sample_count += 1;

            // Get event info
            let sample_event_info = match reader.get_sample_event_info(&event) {
                Ok(info) => info,
                Err(e) => {
                    if sample_count <= 5 {
                        println!(
                            "  Sample event #{} - error getting info: {}",
                            sample_count, e
                        );
                    }
                    continue;
                }
            };

            // Print first few sample events
            if sample_count <= 5 {
                println!(
                    "  Sample event #{}: {}",
                    sample_count,
                    sample_event_info.name()
                );

                // Try to decode using EventHeader
                if let Ok(mut enumerator) = enumerator_ctx.enumerate(&sample_event_info) {
                    let eh_event_info = enumerator.event_info();
                    println!(
                        "    EventHeader info: {}",
                        eh_event_info.json_meta_display(Some(&sample_event_info))
                    );

                    // Move past initial state and print first few fields
                    enumerator.move_next();
                    let mut field_count = 0;
                    while enumerator.state() >= td::EventHeaderEnumeratorState::BeforeFirstItem
                        && field_count < 3
                    {
                        let item_info = enumerator.item_info();
                        println!("    Field: {}", item_info.name_and_tag_display());
                        if !enumerator.move_next_sibling() {
                            break;
                        }
                        field_count += 1;
                    }
                } else if let Some(event_format) = sample_event_info.format() {
                    // Decode using TraceFS format
                    let skip_fields = event_format.common_field_count();
                    for (field_count, field_format) in
                        event_format.fields().iter().skip(skip_fields).enumerate()
                    {
                        if field_count >= 3 {
                            break;
                        }
                        let field_value = field_format.get_field_value(&sample_event_info);
                        println!("    {}: {}", field_format.name(), field_value.display());
                    }
                }
            }
        }
    }

    // Print summary
    println!();
    println!("Event Summary:");
    println!("{:=<50}", "");
    println!("  Total Events:      {:>10}", stats.total_events);
    println!("  Sample Events:     {:>10}", stats.sample_events);
    println!("  Non-Sample Events: {:>10}", stats.non_sample_events);
    println!("{:=<50}", "");

    Ok(stats)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tracepoint_stats_default() {
        let stats = TracepointStats::default();
        assert_eq!(stats.total_events, 0);
        assert_eq!(stats.sample_events, 0);
        assert_eq!(stats.non_sample_events, 0);
    }

    #[test]
    fn test_read_nonexistent_file() {
        let result = read_tracepoint_file("/nonexistent/file.data");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("File not found"));
    }
}

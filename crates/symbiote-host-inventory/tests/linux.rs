use symbiote_domain::Timestamp;
use symbiote_host_inventory::{Fact, linux::*};

#[test]
fn memory_observation_preserves_pressure_and_kernel_units() {
    let memory =
        parse_meminfo("MemTotal: 1024 kB\nMemFree: 1000 kB\nMemAvailable: 0 kB\n").unwrap();
    assert_eq!(memory.total_bytes, 1_048_576);
    assert_eq!(memory.available_bytes, 0);
    assert_eq!(
        parse_meminfo("MemTotal: 1 kB\nMemAvailable: 1 kB")
            .unwrap()
            .available_bytes,
        1024
    );
}

#[test]
fn memory_missing_duplicate_malformed_overflow_or_inconsistent_fails() {
    for (input, reason) in [
        ("MemTotal: 1 kB", ProbeError::MissingField),
        ("MemTotal: 1 kB\nMemTotal: 1 kB", ProbeError::DuplicateField),
        ("MemTotal: 1 MB\nMemAvailable: 0 kB", ProbeError::Malformed),
        ("MemTotal: +1 kB\nMemAvailable: 0 kB", ProbeError::Malformed),
        (
            "MemTotal: 1 kB extra\nMemAvailable: 0 kB",
            ProbeError::Malformed,
        ),
        (
            "MemTotal: 18446744073709551615 kB\nMemAvailable: 0 kB",
            ProbeError::Overflow,
        ),
        (
            "MemTotal: 18446744073709551616 kB\nMemAvailable: 0 kB",
            ProbeError::Overflow,
        ),
        (
            "MemTotal: 0 kB\nMemAvailable: 0 kB",
            ProbeError::Inconsistent,
        ),
        (
            "MemTotal: 1 kB\nMemAvailable: 2 kB",
            ProbeError::Inconsistent,
        ),
        (
            "MemTotal: 1 kB\nMemAvailable: 0 kB\nMemAvailable: 0 kB",
            ProbeError::DuplicateField,
        ),
    ] {
        assert_eq!(parse_meminfo(input), Err(reason));
    }
    assert_eq!(
        parse_meminfo(&" ".repeat(MAX_MEMINFO_BYTES + 1)),
        Err(ProbeError::TooLarge)
    );
}

#[test]
fn logical_cpu_counts_online_rows_without_assuming_contiguous_ids_or_utilization() {
    assert_eq!(
        parse_cpu_stat("cpu 1 2 3 4\ncpu0 1 2 3 4\ncpu8 1 2 3 4 0 0 0 0 0 0\nintr 100\n"),
        Ok(2)
    );
}

#[test]
fn cpu_duplicate_missing_malformed_and_overflow_fail_closed() {
    for (input, reason) in [
        ("cpu 1 2 3 4", ProbeError::MissingField),
        ("cpu0 1 2 3 4", ProbeError::MissingField),
        ("cpu 1 2 3 4\ncpu 1 2 3 4", ProbeError::DuplicateField),
        (
            "cpu 1 2 3 4\ncpu0 1 2 3 4\ncpu00 1 2 3 4",
            ProbeError::DuplicateField,
        ),
        ("cpu 1 2 3 4\ncpu0 1 2 3", ProbeError::Malformed),
        ("cpu 1 2 3 4\ncpu0 1 2 3 -1", ProbeError::Malformed),
        ("cpu 1 2 3 4\ncpu4294967296 1 2 3 4", ProbeError::Overflow),
        (
            "cpu 1 2 3 4\ncpu0 1 2 3 18446744073709551616",
            ProbeError::Overflow,
        ),
    ] {
        assert_eq!(parse_cpu_stat(input), Err(reason));
    }
    assert_eq!(
        parse_cpu_stat(&" ".repeat(MAX_STAT_BYTES + 1)),
        Err(ProbeError::TooLarge)
    );
}

#[test]
fn passive_sample_has_timestamp_and_never_claims_effective_capacity() {
    let sample = probe(Timestamp(123));
    assert_eq!(sample.observed_at, Timestamp(123));
    assert_eq!(sample.os, Fact::Known(std::env::consts::OS.into()));
    assert_eq!(
        sample.architecture,
        Fact::Known(std::env::consts::ARCH.into())
    );
    assert_eq!(sample.resources.effective_cpu_millicores, Fact::Unknown);
    assert_eq!(sample.resources.effective_memory_limit_bytes, Fact::Unknown);
    assert_eq!(
        sample.resources.effective_memory_available_bytes,
        Fact::Unknown
    );
    for failure in &sample.failures {
        match failure.resource {
            ProbeResource::PhysicalMemory => {
                assert_eq!(sample.resources.physical_memory_total_bytes, Fact::Unknown);
                assert_eq!(
                    sample.resources.physical_memory_available_bytes,
                    Fact::Unknown
                );
            }
            ProbeResource::LogicalCpuCount => {
                assert_eq!(sample.resources.logical_cpu_count, Fact::Unknown)
            }
        }
    }
    let serialized = serde_json::to_string(&sample).unwrap();
    assert!(!serialized.contains("/proc"));
    assert!(!serialized.contains("/home"));
}

use std::collections::BTreeSet;
use symbiote_domain::{CommandId, HostId, Timestamp};
use symbiote_host_inventory::{Fact, HostPulse, linux::*};

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
fn cgroup_path_memory_and_cpu_are_parsed_or_reasoned() {
    assert_eq!(
        parse_cgroup_path("5:cpu:/legacy\n0::/user.slice/app.service\n"),
        Ok("/user.slice/app.service".into())
    );
    assert_eq!(
        parse_cgroup_path("5:cpu:/legacy\n"),
        Err(ProbeError::UnsupportedHierarchy)
    );
    assert_eq!(
        parse_cgroup_path("0::/a\n0::/b\n"),
        Err(ProbeError::DuplicateField)
    );
    for malformed in ["0::relative", "0::/../../etc", "0::/a\u{7}"] {
        assert_eq!(parse_cgroup_path(malformed), Err(ProbeError::Malformed));
    }
    assert_eq!(parse_memory_max("max\n"), Ok(None));
    assert_eq!(parse_memory_max("1073741824\n"), Ok(Some(1_073_741_824)));
    assert_eq!(parse_memory_max("1G"), Err(ProbeError::Malformed));
    assert_eq!(parse_memory_current("500\n"), Ok(500));
    assert_eq!(parse_memory_current("5 0"), Err(ProbeError::Malformed));
    assert_eq!(parse_cpu_max("max 100000\n"), Ok(None));
    assert_eq!(parse_cpu_max("150000 100000\n"), Ok(Some(1_500)));
    assert_eq!(parse_cpu_max("1000 100000\n"), Ok(Some(10)));
    assert_eq!(parse_cpu_max("1000 30000"), Ok(Some(33)));
    assert_eq!(parse_cpu_max("1000\n"), Err(ProbeError::Malformed));
    assert_eq!(parse_cpu_max("1000 0\n"), Err(ProbeError::Inconsistent));
    assert_eq!(
        parse_cpu_max("18446744073709551615 1\n"),
        Err(ProbeError::Overflow)
    );
}

#[test]
fn effective_capacity_is_observed_and_never_invented() {
    // A finite ceiling bounds availability by the headroom under it, and never
    // by more than the machine reports available.
    assert_eq!(
        derive_memory(Ok(Some(1_000)), Ok(400), &Fact::Known(5_000)),
        (Fact::Known(1_000), Fact::Known(600), None)
    );
    assert_eq!(
        derive_memory(Ok(Some(1_000)), Ok(400), &Fact::Known(500)),
        (Fact::Known(1_000), Fact::Known(500), None)
    );
    assert_eq!(
        derive_memory(Ok(Some(1_000)), Ok(2_000), &Fact::Known(5_000)),
        (Fact::Known(1_000), Fact::Known(0), None)
    );
    // No enforced ceiling: the machine's own availability is the bound, and no
    // ceiling is claimed from it.
    assert_eq!(
        derive_memory(Ok(None), Ok(400), &Fact::Known(5_000)),
        (Fact::Unknown, Fact::Known(5_000), None)
    );
    assert_eq!(
        derive_memory(Ok(None), Err(ProbeError::Unreadable), &Fact::Known(5_000)),
        (Fact::Unknown, Fact::Known(5_000), None)
    );
    // An unreadable ceiling is an absence with its own static reason, never a
    // guess from the machine's totals.
    assert_eq!(
        derive_memory(Err(ProbeError::Unreadable), Ok(400), &Fact::Known(5_000)),
        (Fact::Unknown, Fact::Unknown, Some(ProbeError::Unreadable))
    );
    // A ceiling whose charge could not be read keeps the observed ceiling and
    // reports availability unknown.
    assert_eq!(
        derive_memory(
            Ok(Some(1_000)),
            Err(ProbeError::Malformed),
            &Fact::Known(5_000)
        ),
        (
            Fact::Known(1_000),
            Fact::Unknown,
            Some(ProbeError::Malformed)
        )
    );
}

#[test]
fn effective_cpu_is_the_processs_own_bound_and_never_the_machines_total() {
    let four = || BTreeSet::from([0, 1, 2, 3]);
    // Without a quota the bound is the CPUs this process may run on.
    assert_eq!(
        derive_cpu(Ok(None), Ok(four()), Ok(four())),
        (Fact::Known(4_000), None)
    );
    // A quota below the allowed CPUs binds; one above them does not.
    assert_eq!(
        derive_cpu(Ok(Some(1_500)), Ok(four()), Ok(four())),
        (Fact::Known(1_500), None)
    );
    assert_eq!(
        derive_cpu(Ok(Some(64_000)), Ok(four()), Ok(four())),
        (Fact::Known(4_000), None)
    );
    // Only the CPUs that are both allowed and online count, in either
    // direction: the machine's total is never substituted for the bound.
    assert_eq!(
        derive_cpu(Ok(None), Ok(four()), Ok(BTreeSet::from([2, 3]))),
        (Fact::Known(2_000), None)
    );
    assert_eq!(
        derive_cpu(Ok(None), Ok(BTreeSet::from([0, 1])), Ok(four())),
        (Fact::Known(2_000), None)
    );
    // Allowed to run on no online CPU is a fact about capacity, not a failure.
    assert_eq!(
        derive_cpu(
            Ok(None),
            Ok(BTreeSet::from([0, 1])),
            Ok(BTreeSet::from([2, 3]))
        ),
        (Fact::Known(0), None)
    );
    // A quota that cannot be read may still bind, and a mask that cannot be
    // read may be narrower than the machine: each leaves the fact unknown with
    // its own static reason.
    assert_eq!(
        derive_cpu(
            Err(ProbeError::UnsupportedHierarchy),
            Ok(four()),
            Ok(four())
        ),
        (Fact::Unknown, Some(ProbeError::UnsupportedHierarchy))
    );
    assert_eq!(
        derive_cpu(Ok(None), Err(ProbeError::Unreadable), Ok(four())),
        (Fact::Unknown, Some(ProbeError::Unreadable))
    );
    assert_eq!(
        derive_cpu(Ok(None), Ok(four()), Err(ProbeError::MissingField)),
        (Fact::Unknown, Some(ProbeError::MissingField))
    );
}

#[test]
fn cpu_masks_and_online_rows_are_parsed_or_reasoned() {
    assert_eq!(
        parse_cpu_list("0-3,8,10-11"),
        Ok(BTreeSet::from([0, 1, 2, 3, 8, 10, 11]))
    );
    assert_eq!(parse_cpu_list("5"), Ok(BTreeSet::from([5])));
    for malformed in ["", "-1", "3-1", "0-", "a", "1 2"] {
        assert_eq!(parse_cpu_list(malformed), Err(ProbeError::Malformed));
    }
    // A list that names a CPU twice — repeated or overlapping, and therefore
    // not ascending — is refused as a duplicate rather than as syntax.
    for duplicated in ["0,0", "2-3,1-2", "0-1,1-2"] {
        assert_eq!(parse_cpu_list(duplicated), Err(ProbeError::DuplicateField));
    }
    assert_eq!(parse_cpu_list("4294967296"), Err(ProbeError::Overflow));
    assert_eq!(
        parse_cpu_list(&format!("0-{MAX_OBSERVED_CPUS}")),
        Err(ProbeError::TooLarge)
    );
    assert_eq!(
        parse_cpus_allowed("Name:\tsymbioted\nCpus_allowed_list:\t0-2\nThreads:\t3\n"),
        Ok(BTreeSet::from([0, 1, 2]))
    );
    assert_eq!(
        parse_cpus_allowed("Name:\tsymbioted\n"),
        Err(ProbeError::MissingField)
    );
    assert_eq!(
        parse_cpus_allowed("Cpus_allowed_list:\t0\nCpus_allowed_list:\t1\n"),
        Err(ProbeError::DuplicateField)
    );
    assert_eq!(
        parse_online_cpus("cpu 1 2 3 4\ncpu0 1 2 3 4\ncpu8 1 2 3 4 0 0 0 0 0 0\n"),
        Ok(BTreeSet::from([0, 8]))
    );
}

#[test]
fn passive_sample_never_claims_capacity_it_did_not_observe() {
    let sample = probe(Timestamp(123));
    assert_eq!(sample.observed_at, Timestamp(123));
    assert_eq!(sample.os, Fact::Known(std::env::consts::OS.into()));
    assert_eq!(
        sample.architecture,
        Fact::Known(std::env::consts::ARCH.into())
    );
    // Whatever hierarchy this host runs, the observation the probe produces is
    // one the pulse contract accepts: every recorded failure explains an absent
    // fact, and EffectiveMemory has the one shape its contract allows — the
    // ceiling may be observed while only the charge against it was unreadable.
    let pulse = HostPulse::from_observation(
        HostId::new("host-probe").unwrap(),
        CommandId::new("probe").unwrap(),
        sample.clone(),
    )
    .unwrap();
    assert_eq!(pulse.probe_failures, sample.failures);
    let failed = |resource: ProbeResource| {
        sample
            .failures
            .iter()
            .any(|failure| failure.resource == resource)
    };
    if failed(ProbeResource::EffectiveMemory) {
        assert_eq!(
            sample.resources.effective_memory_available_bytes,
            Fact::Unknown
        );
    }
    if failed(ProbeResource::EffectiveCpu) {
        assert_eq!(sample.resources.effective_cpu_millicores, Fact::Unknown);
    }
    // Nothing about the machine is echoed into the observation.
    let serialized = serde_json::to_string(&sample).unwrap();
    assert!(!serialized.contains("/proc"));
    assert!(!serialized.contains("/sys/fs/cgroup"));
    assert!(!serialized.contains("/home"));
}

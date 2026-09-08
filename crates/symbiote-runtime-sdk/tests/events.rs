use symbiote_domain::{CommandId, DispatchId, HostId, RequestId, RuntimeKind, SessionId};
use symbiote_runtime_sdk::events::*;

fn binding() -> SessionBinding {
    SessionBinding {
        session_id: SessionId::new("session").unwrap(),
        dispatch_id: DispatchId::new("dispatch").unwrap(),
        host_id: HostId::new("host").unwrap(),
        runtime: RuntimeKind::NativeSymbiote,
    }
}
fn text(value: &str) -> EventText {
    EventText::new(value).unwrap()
}
fn tool() -> RequestId {
    RequestId::new("tool-1").unwrap()
}
fn event(sequence: u64, payload: RuntimeEventKind) -> RuntimeEvent {
    RuntimeEvent::new(
        CommandId::new(format!("event-{sequence}")).unwrap(),
        sequence,
        binding(),
        payload,
    )
    .unwrap()
}
fn tracker() -> SessionTracker {
    SessionTracker::new(binding(), TrackerLimits::default()).unwrap()
}
fn proposed() -> RuntimeEventKind {
    RuntimeEventKind::ToolProposed {
        tool_call_id: tool(),
        name: text("read_file"),
        arguments: text("bounded redacted arguments"),
    }
}

#[test]
fn tool_lifecycle_report_and_exit_never_create_canonical_completion() {
    let mut session = tracker();
    session.apply(event(1, RuntimeEventKind::Ready {})).unwrap();
    session.apply(event(2, proposed())).unwrap();
    session
        .apply(event(
            3,
            RuntimeEventKind::ToolStarted {
                tool_call_id: tool(),
            },
        ))
        .unwrap();
    session
        .apply(event(
            4,
            RuntimeEventKind::ToolCompleted {
                tool_call_id: tool(),
                output: text("observed output"),
            },
        ))
        .unwrap();
    let report = session
        .apply(event(
            5,
            RuntimeEventKind::CompletionRequested {
                report: text("worker says done"),
            },
        ))
        .unwrap();
    assert_eq!(report.state, SessionState::Running);
    let exit = session
        .apply(event(6, RuntimeEventKind::Exit { code: Some(0) }))
        .unwrap();
    assert_eq!(exit.state, SessionState::Exited);
    assert_eq!(session.tracked_tool_count(), 1);
    assert_eq!(
        session.apply(event(
            7,
            RuntimeEventKind::CompletionRequested {
                report: text("done again")
            }
        )),
        Err(EventError::InvalidTransition)
    );
    let value = serde_json::to_value(session).unwrap();
    assert_eq!(value["state"], "exited");
    assert!(!value.as_object().unwrap().contains_key("task"));
}

#[test]
fn missing_out_of_order_and_changed_events_cannot_advance() {
    let mut session = tracker();
    let ready = event(1, RuntimeEventKind::Ready {});
    let original = session.apply(ready.clone()).unwrap();
    let duplicate = session.apply(ready).unwrap();
    assert_eq!(duplicate.sequence, original.sequence);
    assert_eq!(duplicate.state, original.state);
    assert!(duplicate.replayed);
    assert_eq!(
        session.apply(event(
            1,
            RuntimeEventKind::Message {
                text: text("changed")
            }
        )),
        Err(EventError::IdempotencyConflict)
    );
    assert_eq!(
        session.apply(event(3, RuntimeEventKind::Message { text: text("gap") })),
        Err(EventError::SequenceGap {
            expected: 2,
            received: 3
        })
    );
    let same_sequence = RuntimeEvent::new(
        CommandId::new("different-id").unwrap(),
        1,
        binding(),
        RuntimeEventKind::Ready {},
    )
    .unwrap();
    assert_eq!(
        session.apply(same_sequence),
        Err(EventError::SequenceConflict)
    );
    assert_eq!(session.next_sequence(), 2);
    assert_eq!(session.retained_event_count(), 1);
}

#[test]
fn immutable_binding_rejects_cross_session_dispatch_host_and_runtime() {
    let mut variants = vec![];
    let mut other = binding();
    other.session_id = SessionId::new("other").unwrap();
    variants.push(other);
    let mut other = binding();
    other.dispatch_id = DispatchId::new("other").unwrap();
    variants.push(other);
    let mut other = binding();
    other.host_id = HostId::new("other").unwrap();
    variants.push(other);
    let mut other = binding();
    other.runtime = RuntimeKind::ExternalHarness;
    variants.push(other);
    let mut session = tracker();
    for other in variants {
        let foreign = RuntimeEvent::new(
            CommandId::new("foreign").unwrap(),
            1,
            other,
            RuntimeEventKind::Ready {},
        )
        .unwrap();
        assert_eq!(session.apply(foreign), Err(EventError::BindingMismatch));
    }
    assert_eq!(session.next_sequence(), 1);
}

#[test]
fn cancellation_rejects_late_tool_effects_and_never_reports_clean_success() {
    let mut session = tracker();
    session.apply(event(1, RuntimeEventKind::Ready {})).unwrap();
    session.apply(event(2, proposed())).unwrap();
    session
        .apply(event(
            3,
            RuntimeEventKind::ToolStarted {
                tool_call_id: tool(),
            },
        ))
        .unwrap();
    session
        .apply(event(
            4,
            RuntimeEventKind::CancellationRequested {
                reason: text("client cancelled"),
            },
        ))
        .unwrap();
    assert_eq!(
        session.apply(event(
            5,
            RuntimeEventKind::ToolCompleted {
                tool_call_id: tool(),
                output: text("late write")
            }
        )),
        Err(EventError::ResidualEffectUnknown)
    );
    assert_eq!(session.next_sequence(), 5);
    assert_eq!(session.residual_effects(), ResidualEffects::Unknown);
    // The rejected effect is not silently skipped: the authoritative producer
    // must reconcile its stream; this fixture explicitly supplies the same next slot.
    session
        .apply(event(5, RuntimeEventKind::CancelAcknowledged {}))
        .unwrap();
    assert_eq!(session.state(), SessionState::Cancelled);
    assert_eq!(
        session.apply(event(
            6,
            RuntimeEventKind::Observation {
                resource: ObservationKind::FilesystemMutation,
                detail: text("late file event")
            }
        )),
        Err(EventError::ResidualEffectUnknown)
    );
    session
        .apply(event(6, RuntimeEventKind::Exit { code: Some(0) }))
        .unwrap();
    assert_eq!(session.state(), SessionState::Cancelled);
    assert_eq!(session.residual_effects(), ResidualEffects::Unknown);
}

#[test]
fn usage_unknown_is_distinct_from_zero_and_hidden_children_stay_visible() {
    let unknown = UsageMeasurement::Unknown {
        reason: UnknownUsageReason::NotReported,
    };
    let measured = UsageMeasurement::Measured { value: 0 };
    assert_ne!(
        serde_json::to_value(&unknown).unwrap(),
        serde_json::to_value(&measured).unwrap()
    );
    let payload = RuntimeEventKind::Usage {
        usage: RuntimeUsage {
            input_tokens: measured,
            output_tokens: unknown.clone(),
            cached_input_tokens: unknown.clone(),
            billed_micro_units: unknown,
            coverage: UsageCoverage::RootOnly,
        },
    };
    let envelope = event(1, payload);
    assert_eq!(
        serde_json::from_value::<RuntimeEvent>(serde_json::to_value(&envelope).unwrap()).unwrap(),
        envelope
    );
    let mut missing = serde_json::to_value(&envelope).unwrap();
    missing["payload"]["usage"]
        .as_object_mut()
        .unwrap()
        .remove("output_tokens");
    assert!(serde_json::from_value::<RuntimeEvent>(missing).is_err());
}

#[test]
fn disconnect_resume_preserves_readiness_and_cancellation_intent() {
    let mut session = tracker();
    session
        .apply(event(
            1,
            RuntimeEventKind::Disconnected {
                reason: text("handshake transport lost"),
            },
        ))
        .unwrap();
    session
        .apply(event(2, RuntimeEventKind::Reconnected {}))
        .unwrap();
    assert_eq!(session.state(), SessionState::AwaitingReady);
    assert_eq!(
        session.apply(event(3, proposed())),
        Err(EventError::InvalidTransition)
    );
    session.apply(event(3, RuntimeEventKind::Ready {})).unwrap();
    session
        .apply(event(
            4,
            RuntimeEventKind::Disconnected {
                reason: text("transport lost"),
            },
        ))
        .unwrap();
    session
        .apply(event(
            5,
            RuntimeEventKind::CancellationRequested {
                reason: text("cancel while disconnected"),
            },
        ))
        .unwrap();
    session
        .apply(event(6, RuntimeEventKind::Reconnected {}))
        .unwrap();
    assert_eq!(session.state(), SessionState::Cancelling);
    assert_eq!(
        session.apply(event(7, proposed())),
        Err(EventError::ResidualEffectUnknown)
    );
    assert_eq!(session.residual_effects(), ResidualEffects::Unknown);
}

#[test]
fn tool_failure_and_unknown_or_reused_tool_ids_are_deterministic() {
    let mut session = tracker();
    session.apply(event(1, RuntimeEventKind::Ready {})).unwrap();
    assert_eq!(
        session.apply(event(
            2,
            RuntimeEventKind::ToolStarted {
                tool_call_id: tool()
            }
        )),
        Err(EventError::UnknownTool)
    );
    session.apply(event(2, proposed())).unwrap();
    assert_eq!(
        session.apply(event(
            3,
            RuntimeEventKind::ToolCompleted {
                tool_call_id: tool(),
                output: text("unstarted")
            }
        )),
        Err(EventError::ToolTransitionRejected)
    );
    session
        .apply(event(
            3,
            RuntimeEventKind::ToolStarted {
                tool_call_id: tool(),
            },
        ))
        .unwrap();
    session
        .apply(event(
            4,
            RuntimeEventKind::ToolFailed {
                tool_call_id: tool(),
                error: text("bounded error"),
            },
        ))
        .unwrap();
    assert_eq!(
        session.apply(event(5, proposed())),
        Err(EventError::ToolAlreadyExists)
    );
    session
        .apply(event(
            5,
            RuntimeEventKind::Crashed {
                reason: text("runtime died"),
            },
        ))
        .unwrap();
    assert_eq!(session.state(), SessionState::Crashed);
    assert_eq!(session.residual_effects(), ResidualEffects::Unknown);
}

#[test]
fn replay_window_eviction_cannot_reuse_old_ids_and_total_memory_is_bounded() {
    let mut session = SessionTracker::new(
        binding(),
        TrackerLimits {
            replay_capacity: 2,
            max_tool_calls: 1,
            max_events: 4,
        },
    )
    .unwrap();
    let ready = event(1, RuntimeEventKind::Ready {});
    session.apply(ready.clone()).unwrap();
    session
        .apply(event(
            2,
            RuntimeEventKind::Message {
                text: text("second"),
            },
        ))
        .unwrap();
    session
        .apply(event(
            3,
            RuntimeEventKind::Message {
                text: text("third"),
            },
        ))
        .unwrap();
    assert_eq!(session.retained_event_count(), 2);
    assert_eq!(session.replay_floor(), 2);
    assert_eq!(
        session.apply(ready),
        Err(EventError::ReplayUnavailable { floor: 2 })
    );
    let reused = RuntimeEvent::new(
        CommandId::new("event-1").unwrap(),
        4,
        binding(),
        RuntimeEventKind::Message {
            text: text("reuse"),
        },
    )
    .unwrap();
    assert_eq!(session.apply(reused), Err(EventError::IdempotencyConflict));
    session
        .apply(event(
            4,
            RuntimeEventKind::Message {
                text: text("fourth"),
            },
        ))
        .unwrap();
    assert_eq!(
        session.apply(event(
            5,
            RuntimeEventKind::Message {
                text: text("fifth")
            }
        )),
        Err(EventError::CapacityExceeded)
    );
    assert_eq!(session.next_sequence(), 5);
    assert_eq!(session.retained_event_count(), 2);
}

#[test]
fn tool_capacity_and_limits_fail_closed() {
    assert!(
        SessionTracker::new(
            binding(),
            TrackerLimits {
                replay_capacity: 0,
                ..TrackerLimits::default()
            }
        )
        .is_err()
    );
    let mut session = SessionTracker::new(
        binding(),
        TrackerLimits {
            max_tool_calls: 1,
            ..TrackerLimits::default()
        },
    )
    .unwrap();
    session.apply(event(1, RuntimeEventKind::Ready {})).unwrap();
    session.apply(event(2, proposed())).unwrap();
    assert_eq!(
        session.apply(event(
            3,
            RuntimeEventKind::ToolProposed {
                tool_call_id: RequestId::new("other-tool").unwrap(),
                name: text("read"),
                arguments: text("")
            }
        )),
        Err(EventError::CapacityExceeded)
    );
    assert_eq!(session.tracked_tool_count(), 1);
    assert_eq!(session.next_sequence(), 3);
}

#[test]
fn wire_validation_rejects_zero_sequence_oversized_utf8_and_unknown_fields() {
    assert!(EventText::new("x".repeat(MAX_EVENT_TEXT_BYTES)).is_ok());
    assert_eq!(
        EventText::new("x".repeat(MAX_EVENT_TEXT_BYTES + 1)),
        Err(EventError::TextTooLarge)
    );
    assert_eq!(
        EventText::new("🦀".repeat(MAX_EVENT_TEXT_BYTES / 4 + 1)),
        Err(EventError::TextTooLarge)
    );
    let original = event(1, RuntimeEventKind::Ready {});
    let mut zero = serde_json::to_value(&original).unwrap();
    zero["sequence"] = serde_json::json!(0);
    assert!(serde_json::from_value::<RuntimeEvent>(zero).is_err());
    let mut unknown = serde_json::to_value(&original).unwrap();
    unknown["payload"]["task_completed"] = serde_json::json!(true);
    assert!(serde_json::from_value::<RuntimeEvent>(unknown).is_err());
    let mut oversized = serde_json::to_value(event(
        1,
        RuntimeEventKind::Message {
            text: text("small"),
        },
    ))
    .unwrap();
    oversized["payload"]["text"] = serde_json::json!("x".repeat(MAX_EVENT_TEXT_BYTES + 1));
    assert!(serde_json::from_value::<RuntimeEvent>(oversized).is_err());
}

#[test]
fn event_schema_keeps_namespaced_evidence_separate_and_bounded() {
    let original = event(1, RuntimeEventKind::Ready {})
        .with_vendor_reference(VendorEventReference {
            namespace: text("vendor.runtime"),
            id: RequestId::new("raw-reference").unwrap(),
        })
        .unwrap();
    assert_eq!(
        serde_json::from_str::<RuntimeEvent>(&serde_json::to_string(&original).unwrap()).unwrap(),
        original
    );
    let schema = serde_json::to_value(schemars::schema_for!(RuntimeEvent)).unwrap();
    assert_eq!(
        schema["$defs"]["EventText"]["maxLength"],
        MAX_EVENT_TEXT_BYTES
    );
    assert!(original.vendor_reference().is_some());
}

#[test]
fn invalid_known_usage_subsets_and_totals_reject_construction_and_wire_without_advancing() {
    let unknown = UsageMeasurement::Unknown {
        reason: UnknownUsageReason::NotReported,
    };
    let measured = |value| UsageMeasurement::Measured { value };
    let valid = RuntimeUsage {
        input_tokens: measured(1),
        output_tokens: measured(2),
        cached_input_tokens: measured(1),
        billed_micro_units: unknown.clone(),
        coverage: UsageCoverage::RootOnly,
    };
    let mut session = tracker();
    session.apply(event(1, RuntimeEventKind::Ready {})).unwrap();
    for invalid in [
        RuntimeUsage {
            cached_input_tokens: measured(100),
            ..valid.clone()
        },
        RuntimeUsage {
            input_tokens: measured(u64::MAX),
            output_tokens: measured(1),
            ..valid.clone()
        },
    ] {
        assert_eq!(
            RuntimeEvent::new(
                CommandId::new("invalid-usage").unwrap(),
                2,
                binding(),
                RuntimeEventKind::Usage {
                    usage: invalid.clone()
                }
            ),
            Err(EventError::InvalidUsage)
        );
        assert!(
            serde_json::from_value::<RuntimeUsage>(serde_json::to_value(&invalid).unwrap())
                .is_err()
        );
        let mut wire = serde_json::to_value(event(
            2,
            RuntimeEventKind::Usage {
                usage: valid.clone(),
            },
        ))
        .unwrap();
        wire["payload"]["usage"] = serde_json::to_value(invalid).unwrap();
        assert!(serde_json::from_value::<RuntimeEvent>(wire).is_err());
        assert_eq!(session.next_sequence(), 2);
    }
    let partial = RuntimeUsage {
        input_tokens: unknown,
        cached_input_tokens: measured(100),
        ..valid
    };
    session
        .apply(event(2, RuntimeEventKind::Usage { usage: partial }))
        .unwrap();
    assert_eq!(
        session.next_sequence(),
        3,
        "unknown input is not zero and cannot disprove a cached subset"
    );
}

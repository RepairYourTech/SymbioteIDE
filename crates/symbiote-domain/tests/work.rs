use std::collections::BTreeSet;
use symbiote_domain::*;
fn user() -> UserId {
    UserId::new("user").unwrap()
}
fn host() -> HostId {
    HostId::new("host").unwrap()
}
fn spec(id: WorkId) -> WorkSpec {
    WorkSpec {
        utterance: matches!(id, WorkId::Request(_)).then_some(RequestUtterance::RequestedWork),
        objective_class: matches!(id, WorkId::Objective(_)).then_some(ObjectiveClass::Outcome),
        id,
        project_id: ProjectId::new("project").unwrap(),
        role_id: RoleId::new("lead").unwrap(),
        title: "Deliver feature".into(),
        description: "Bounded canonical work".into(),
        parent: None,
        dependencies: BTreeSet::new(),
        requirements: vec![],
        constraints: vec![],
        risks: vec![],
        acceptance: vec!["Demonstration is verified".into()],
        priority: 2,
        budget: None,
        external_references: vec![],
    }
}
fn objective() -> WorkItem {
    WorkItem::new(
        spec(WorkId::Objective(ObjectiveId::new("objective").unwrap())),
        user(),
        Timestamp(1),
    )
    .unwrap()
}
fn command(item: &WorkItem, action: WorkAction) -> WorkCommand {
    WorkCommand {
        id: CommandId::new(format!("command-{}", item.revision().0 + 1)).unwrap(),
        expected_revision: item.revision(),
        actor: Actor::User(user()),
        at: Timestamp(item.revision().0 + 2),
        action,
    }
}
fn apply(item: &mut WorkItem, action: WorkAction) {
    item.apply(command(item, action)).unwrap();
}
fn evidence(item: &WorkItem) -> WorkCompletionEvidence {
    WorkCompletionEvidence {
        work: item.spec().reference(),
        revision: item.revision(),
        verified_by: host(),
        verified_at: Timestamp(20),
        acceptance: vec![AcceptanceEvidence {
            criterion: 0,
            id: EvidenceId::new("acceptance").unwrap(),
            artifact_id: ArtifactId::new("acceptance-artifact").unwrap(),
            outcome: VerificationOutcome::Passed,
        }],
        gates: Gate::required()
            .into_iter()
            .enumerate()
            .map(|(n, gate)| WorkGateEvidence {
                gate,
                id: EvidenceId::new(format!("gate-{n}")).unwrap(),
                run_id: VerificationRunId::new(format!("run-{n}")).unwrap(),
                artifact_id: ArtifactId::new(format!("artifact-{n}")).unwrap(),
                outcome: VerificationOutcome::Passed,
                reviewer: Some(RoleId::new("independent").unwrap()),
            })
            .collect(),
    }
}
fn completion(item: &WorkItem, evidence: WorkCompletionEvidence) -> WorkCommand {
    WorkCommand {
        id: CommandId::new("complete").unwrap(),
        expected_revision: item.revision(),
        actor: Actor::Host(host()),
        at: Timestamp(20),
        action: WorkAction::Complete {
            evidence: Box::new(evidence),
        },
    }
}
fn pending_completion() -> WorkItem {
    let mut item = objective();
    for action in [
        WorkAction::RequestApproval {},
        WorkAction::Approve {},
        WorkAction::Start {},
        WorkAction::RequestCompletion {},
    ] {
        apply(&mut item, action);
    }
    item
}

#[test]
fn exact_gate_and_acceptance_evidence_completes_only_through_host() {
    let mut item = pending_completion();
    let complete = completion(&item, evidence(&item));
    let mut forged = complete.clone();
    forged.actor = Actor::User(user());
    assert_eq!(item.apply(forged), Err(WorkError::PermissionDenied));
    let receipt = item.apply(complete.clone()).unwrap();
    assert_eq!(item.state(), WorkState::Completed);
    assert_eq!(
        item.apply(complete).unwrap(),
        WorkReceipt {
            revision: receipt.revision,
            replayed: true
        }
    );
    let encoded = serde_json::to_string(&item).unwrap();
    assert_eq!(WorkItem::parse(&encoded).unwrap(), item);
    let mut reopen = command(
        &item,
        WorkAction::Reopen {
            reason: "Acceptance changed".into(),
        },
    );
    reopen.at = Timestamp(21);
    item.apply(reopen).unwrap();
    assert_eq!(item.state(), WorkState::Draft);
}

#[test]
fn incomplete_stale_failed_and_self_review_evidence_never_advances() {
    let item = pending_completion();
    let good = evidence(&item);
    let mut variants = Vec::new();
    let mut v = good.clone();
    v.gates.pop();
    variants.push(v);
    let mut v = good.clone();
    v.revision = Revision(0);
    variants.push(v);
    let mut v = good.clone();
    v.acceptance.clear();
    variants.push(v);
    let mut v = good.clone();
    v.acceptance[0].outcome = VerificationOutcome::Failed;
    variants.push(v);
    let mut v = good.clone();
    v.gates
        .iter_mut()
        .find(|g| g.gate == Gate::IndependentReview)
        .unwrap()
        .reviewer = Some(item.role_id().clone());
    variants.push(v);
    let mut v = good.clone();
    v.work.project_id = ProjectId::new("other").unwrap();
    variants.push(v);
    let mut v = good.clone();
    v.verified_by = HostId::new("other").unwrap();
    variants.push(v);
    let mut v = good.clone();
    v.verified_at = Timestamp(1);
    variants.push(v);
    for variant in variants {
        let mut candidate = item.clone();
        assert_eq!(
            candidate.apply(completion(&item, variant)),
            Err(WorkError::InvalidEvidence)
        );
        assert_eq!(candidate, item);
    }
}

#[test]
fn revisions_workers_cancellation_and_approval_staleness_are_explicit() {
    let mut item = objective();
    let mut worker = command(&item, WorkAction::RequestApproval {});
    worker.actor = Actor::Worker(DispatchId::new("worker").unwrap());
    assert_eq!(item.apply(worker), Err(WorkError::PermissionDenied));
    assert_eq!(
        item.apply(command(&item, WorkAction::Start {})),
        Err(WorkError::IllegalTransition)
    );
    apply(&mut item, WorkAction::RequestApproval {});
    apply(&mut item, WorkAction::Approve {});
    let mut revised = item.spec().clone();
    revised.description = "New scope".into();
    apply(
        &mut item,
        WorkAction::Revise {
            spec: Box::new(revised),
        },
    );
    assert_eq!(item.state(), WorkState::Draft);
    assert_eq!(
        item.apply(command(&item, WorkAction::Start {})),
        Err(WorkError::IllegalTransition)
    );
    let cancel = command(
        &item,
        WorkAction::Cancel {
            reason: "Stopped".into(),
        },
    );
    item.apply(cancel.clone()).unwrap();
    let mut conflict = cancel.clone();
    conflict.at = Timestamp(100);
    assert_eq!(item.apply(conflict), Err(WorkError::IdempotencyConflict));
    assert!(item.apply(cancel).unwrap().replayed);
    apply(
        &mut item,
        WorkAction::Reopen {
            reason: "Try again".into(),
        },
    );
    let mut stale = command(&item, WorkAction::RequestApproval {});
    stale.expected_revision = Revision(0);
    assert_eq!(item.apply(stale), Err(WorkError::RevisionConflict));
}

#[test]
fn forged_snapshots_and_duplicate_history_are_rejected_by_replay() {
    let mut item = objective();
    apply(&mut item, WorkAction::RequestApproval {});
    let value = serde_json::to_value(&item).unwrap();
    for field in ["state", "revision", "spec"] {
        let mut forged = value.clone();
        match field {
            "state" => forged[field] = serde_json::json!("completed"),
            "revision" => forged[field] = serde_json::json!(100),
            _ => forged[field]["title"] = serde_json::json!("forged"),
        };
        assert!(serde_json::from_value::<WorkItem>(forged).is_err());
    }
    let mut forged = value.clone();
    forged["history"]
        .as_array_mut()
        .unwrap()
        .push(value["history"][0].clone());
    assert!(serde_json::from_value::<WorkItem>(forged).is_err());
    assert_eq!(serde_json::from_value::<WorkItem>(value).unwrap(), item);
}

#[test]
fn work_action_and_embedded_history_reject_unknown_authority_fields() {
    for kind in ["request_approval", "approve", "start", "request_completion"] {
        let valid = serde_json::json!({"kind":kind,"data":{}});
        assert!(serde_json::from_value::<WorkAction>(valid.clone()).is_ok());
        let mut extra = valid.clone();
        extra["actor"] = serde_json::json!("host");
        assert!(serde_json::from_value::<WorkAction>(extra).is_err());
        let mut extra = valid;
        extra["data"]["actor"] = serde_json::json!("host");
        assert!(serde_json::from_value::<WorkAction>(extra).is_err());
    }
    let mut item = objective();
    apply(&mut item, WorkAction::RequestApproval {});
    let mut encoded = serde_json::to_value(item).unwrap();
    encoded["history"][0]["action"]["data"]["actor"] = serde_json::json!("host");
    assert!(serde_json::from_value::<WorkItem>(encoded).is_err());
}

#[test]
fn graphs_reject_cycles_missing_refs_wrong_parent_kinds_and_wrong_projects() {
    let request = WorkItem::new(
        spec(WorkId::Request(RequestId::new("same").unwrap())),
        user(),
        Timestamp(1),
    )
    .unwrap();
    let mut cap = spec(WorkId::Capability(CapabilityId::new("same").unwrap()));
    cap.parent = Some(request.spec().reference());
    let capability = WorkItem::new(cap, user(), Timestamp(1)).unwrap();
    assert_ne!(request.id().key(), capability.id().key());
    assert!(validate_work_graph(&[request.clone(), capability.clone()]).is_ok());
    assert_eq!(
        validate_work_graph(std::slice::from_ref(&capability)),
        Err(WorkError::MissingReference)
    );
    let mut cyclic = request.spec().clone();
    cyclic.dependencies.insert(capability.spec().reference());
    let cyclic = WorkItem::new(cyclic, user(), Timestamp(1)).unwrap();
    assert_eq!(
        validate_work_graph(&[cyclic, capability.clone()]),
        Err(WorkError::Cycle)
    );
    let mut wrong = capability.spec().clone();
    wrong.parent.as_mut().unwrap().project_id = ProjectId::new("other").unwrap();
    let wrong = WorkItem::new(wrong, user(), Timestamp(1)).unwrap();
    assert_eq!(
        validate_work_graph(&[request.clone(), wrong]),
        Err(WorkError::MissingReference)
    );
    let mut milestone = spec(WorkId::Milestone(MilestoneId::new("milestone").unwrap()));
    milestone.parent = Some(request.spec().reference());
    let milestone = WorkItem::new(milestone, user(), Timestamp(1)).unwrap();
    assert_eq!(
        validate_work_graph(&[request, milestone]),
        Err(WorkError::ParentKindMismatch)
    );
    let mut foreign = capability.spec().clone();
    foreign.project_id = ProjectId::new("other").unwrap();
    let foreign = WorkItem::new(foreign, user(), Timestamp(1)).unwrap();
    // Exact cross-Project references are structurally supported, not implicit permission grants.
    let request = WorkItem::new(
        spec(WorkId::Request(RequestId::new("same").unwrap())),
        user(),
        Timestamp(1),
    )
    .unwrap();
    assert!(validate_work_graph(&[request, foreign]).is_ok());
}

#[test]
fn task_origins_only_capabilities_or_maintenance_operational_objectives() {
    let mut item = objective();
    let origin = TaskOrigin::Objective(item.spec().reference());
    assert_eq!(
        origin.validate_target(&item),
        Err(WorkError::InvalidTaskOrigin)
    );
    for class in [ObjectiveClass::Maintenance, ObjectiveClass::Operational] {
        let mut revised = item.spec().clone();
        revised.objective_class = Some(class);
        apply(
            &mut item,
            WorkAction::Revise {
                spec: Box::new(revised),
            },
        );
        assert!(origin.validate_target(&item).is_ok());
        assert_eq!(item.state(), WorkState::Draft);
    }
    assert_eq!(
        TaskOrigin::Capability(item.spec().reference()).validate_target(&item),
        Err(WorkError::InvalidTaskOrigin)
    );
}

#[test]
fn historical_references_remain_visible_and_identity_cannot_change() {
    let mut item = objective();
    let reference = WorkRef {
        project_id: ProjectId::new("foreign").unwrap(),
        id: WorkId::Objective(ObjectiveId::new("dependency").unwrap()),
    };
    let mut revised = item.spec().clone();
    revised.dependencies.insert(reference.clone());
    apply(
        &mut item,
        WorkAction::Revise {
            spec: Box::new(revised),
        },
    );
    let mut revised = item.spec().clone();
    revised.dependencies.clear();
    apply(
        &mut item,
        WorkAction::Revise {
            spec: Box::new(revised),
        },
    );
    assert!(!item.spec().references().contains(&reference));
    assert!(item.references().contains(&reference));
    let mut revised = item.spec().clone();
    revised.role_id = RoleId::new("other").unwrap();
    assert_eq!(
        item.apply(command(
            &item,
            WorkAction::Revise {
                spec: Box::new(revised)
            }
        )),
        Err(WorkError::IdentityChanged)
    );
}

#[test]
fn history_and_serialized_capacity_fail_without_partial_mutation() {
    let mut item = objective();
    let first = command(
        &item,
        WorkAction::Clarify {
            question: "Explain".into(),
        },
    );
    item.apply(first.clone()).unwrap();
    for _ in 1..128 {
        apply(
            &mut item,
            WorkAction::Clarify {
                question: "Explain".into(),
            },
        );
    }
    assert_eq!(
        item.apply(command(&item, WorkAction::RequestApproval {})),
        Err(WorkError::ResourceLimit)
    );
    assert_eq!(item.apply(first).unwrap().revision, Revision(1));
    let mut item = objective();
    let mut large = item.spec().clone();
    large.description = "x".repeat(16_384);
    large.requirements = vec!["x".repeat(2048); 4];
    for n in 0..128 {
        let before = item.clone();
        let mut revised = large.clone();
        revised.title = format!("Revision {n}");
        let result = item.apply(command(
            &item,
            WorkAction::Revise {
                spec: Box::new(revised),
            },
        ));
        if result == Err(WorkError::ResourceLimit) {
            assert_eq!(item, before);
            break;
        }
        result.unwrap();
    }
    let encoded = serde_json::to_string(&item).unwrap();
    assert!(encoded.len() <= 262_144);
    assert_eq!(WorkItem::parse(&encoded).unwrap(), item);
    let mut too_large = large;
    too_large.requirements = vec!["x".repeat(2048); 32];
    assert_eq!(too_large.validate(), Err(WorkError::ResourceLimit));
}

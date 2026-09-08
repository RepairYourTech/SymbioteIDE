use symbiote_projection::toml::*;
fn change(
    path: &[&str],
    before: Option<PrimitiveValue>,
    desired: Option<PrimitiveValue>,
) -> ManagedChange {
    ManagedChange {
        path: path.iter().map(|s| (*s).into()).collect(),
        before,
        desired,
        ownership: ManagedOwnership::SymbioteManaged,
    }
}
fn string(v: &str) -> Option<PrimitiveValue> {
    Some(PrimitiveValue::String(v.into()))
}

#[test]
fn preserves_unknown_content_and_comments_during_managed_replacement() {
    let base = "# user preface\nunknown = 'unchanged' # private\n[agent]\n# model choice\nmodel = 'old' # keep this\n";
    let current = base.replace("'unchanged'", "'human edit'");
    let result = plan(
        base,
        &current,
        &[change(&["agent", "model"], string("old"), string("new"))],
    )
    .unwrap();
    assert!(
        result
            .candidate
            .contains("unknown = 'human edit' # private")
    );
    assert!(result.candidate.contains("# user preface"));
    assert!(result.candidate.contains("# model choice"));
    assert!(result.candidate.contains("# keep this"));
    assert_eq!(result.diffs.len(), 1);
    assert_eq!(result.diffs[0].before, string("old"));
    assert_eq!(result.diffs[0].after, string("new"));
    let second = plan(
        base,
        &result.candidate,
        &[change(&["agent", "model"], string("old"), string("new"))],
    )
    .unwrap();
    assert_eq!(second.candidate, result.candidate);
    assert!(second.diffs.is_empty());
}

#[test]
fn insert_delete_nested_primitives_and_deterministic_order() {
    let base = "[existing]\nremove = 1\nkeep = true\n";
    let mut changes = vec![
        change(
            &["new", "enabled"],
            None,
            Some(PrimitiveValue::Boolean(true)),
        ),
        change(
            &["existing", "remove"],
            Some(PrimitiveValue::Integer(1)),
            None,
        ),
    ];
    let result = plan(base, base, &changes).unwrap();
    assert!(!result.candidate.contains("remove"));
    assert!(result.candidate.contains("keep = true"));
    assert!(result.candidate.contains("enabled = true"));
    changes.reverse();
    assert_eq!(result, plan(base, base, &changes).unwrap());
    assert_eq!(result.diffs.len(), 2);
}

#[test]
fn refuses_concurrent_managed_edits_and_false_baselines() {
    assert_eq!(
        plan(
            "key='base'",
            "key='human'",
            &[change(&["key"], string("base"), string("desired"))]
        ),
        Err(ProjectionError::ConcurrentEdit)
    );
    assert_eq!(
        plan(
            "key='base'",
            "key='base'",
            &[change(&["key"], string("false"), string("desired"))]
        ),
        Err(ProjectionError::BaselineMismatch)
    );
    assert_eq!(
        plan(
            "",
            "key='human'",
            &[change(&["key"], None, string("desired"))]
        ),
        Err(ProjectionError::ConcurrentEdit)
    );
}

#[test]
fn refuses_unsupported_tables_arrays_and_nonprimitive_values() {
    for input in [
        "key=[1,2]",
        "[key]\nchild=1",
        "key={child=1}",
        "key=1.5",
        "key=2020-01-01",
    ] {
        assert_eq!(
            plan(input, input, &[change(&["key"], None, string("desired"))]),
            Err(ProjectionError::UnsupportedValue)
        );
    }
    assert_eq!(
        plan(
            "key=[]",
            "key=[]",
            &[change(&["key", "child"], None, string("desired"))]
        ),
        Err(ProjectionError::UnsupportedValue)
    );
}

#[test]
fn refuses_duplicates_malformed_overlap_unmanaged_and_comments_deletion() {
    for input in ["key=1\nkey=2", "key=["] {
        assert_eq!(plan(input, input, &[]), Err(ProjectionError::MalformedToml));
    }
    let mut edit = change(&["key"], None, string("new"));
    edit.ownership = ManagedOwnership::Unmanaged;
    assert_eq!(plan("", "", &[edit]), Err(ProjectionError::UnmanagedKey));
    assert_eq!(
        plan(
            "",
            "",
            &[
                change(&["a"], None, string("new")),
                change(&["a", "b"], None, string("new"))
            ]
        ),
        Err(ProjectionError::OverlappingPaths)
    );
    for input in ["key=1 # keep", "# keep\nkey=1"] {
        assert_eq!(
            plan(
                input,
                input,
                &[change(&["key"], Some(PrimitiveValue::Integer(1)), None)]
            ),
            Err(ProjectionError::UnsupportedCommentedDeletion)
        );
    }
}

#[test]
fn errors_and_debug_never_contain_values_or_paths() {
    let edit = change(&["sensitive-key"], None, string("private-token"));
    let result = plan("", "", std::slice::from_ref(&edit)).unwrap();
    for debug in [
        format!("{edit:?}"),
        format!("{result:?}"),
        format!("{:?}", result.diffs),
        format!("{:?}", edit.desired),
    ] {
        assert!(!debug.contains("private-token"));
        assert!(!debug.contains("sensitive-key"));
    }
    assert_eq!(
        plan(&" ".repeat(1_048_577), "", &[]),
        Err(ProjectionError::LimitExceeded)
    );
}

#[test]
fn aggregate_values_and_escaped_rendering_are_bounded() {
    let changes: Vec<_> = (0..17)
        .map(|i| change(&[&format!("key{i}")], None, string(&"x".repeat(65_536))))
        .collect();
    assert_eq!(plan("", "", &changes), Err(ProjectionError::LimitExceeded));
    // Each NUL expands to a six-byte TOML escape, but raw input remains under the budget.
    assert_eq!(
        plan(
            "",
            "",
            &[change(&["key"], None, string(&"\0".repeat(200_000)))]
        ),
        Err(ProjectionError::LimitExceeded)
    );
}

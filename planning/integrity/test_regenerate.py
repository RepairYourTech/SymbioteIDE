"""Run: python3 planning/integrity/test_regenerate.py.

Each test below is one of #470's acceptance criteria for regeneration safety,
exercised through the rules rather than through a transport: the fake runner
records every call, so "no mutation was attempted" is an assertion rather than a
claim.
"""

import unittest

from regenerate import AmbiguousCreate, apply, body_hash, diff_of, key_of, plan, verify


def task_body(key, dependencies="- None", notes="original", revision="approved-v1"):
    return (
        f"<!-- symbiote-plan-key: {key} -->\n"
        f"<!-- symbiote-plan-revision: {revision} -->\n"
        f"**Parent epic:** #2 · **Wave:** 0\n"
        f"## Dependencies\n{dependencies}\n"
        f"## Notes\n{notes}\n"
    )


def issue(number, key, body, state="open"):
    return dict(
        number=number,
        title=key,
        state=state,
        state_reason=None,
        updated_at="2026-09-07T12:00:00Z",
        labels=[{"name": "planning:canonical"}],
        body=body,
    )


def marked(key, body, revision="approved-v1"):
    return (
        f"<!-- symbiote-plan-key: {key} -->\n"
        f"<!-- symbiote-plan-revision: {revision} -->\n" + body
    )


def reference_body(key, owner, history="This earlier/duplicate roadmap entry is retained for provenance.\n"):
    """A reference entry: its own key, and the canonical issue it resolves to."""
    return (
        f"<!-- symbiote-reference-key: {key} -->\n"
        f"<!-- symbiote-plan-revision: approved-v1 -->\n"
        f"Canonical owner: #{owner}\n"
        f"<details>\n<summary>history</summary>\n{history}"
        f"<!-- symbiote-plan-key: {key} -->\n</details>\n"
    )


def master():
    return issue(1, "ROADMAP", marked("ROADMAP", "## Canonical epics\n- #2\n"))


def epic(children="- [ ] #3\n- [ ] #4\n"):
    return issue(2, "E00", marked("E00", f"## Child issues\n{children}"))


def snapshot(children="- [ ] #3\n- [ ] #4\n", **bodies):
    base = [
        master(),
        epic(children),
        issue(3, "A01", task_body("A01")),
        issue(4, "A02", task_body("A02", "- #3")),
    ]
    by_number = {raw["number"]: raw for raw in base}
    for number, body in bodies.items():
        by_number[int(number)]["body"] = body
    return [by_number[n] for n in sorted(by_number)]


def amendment(key, body, base_body, kind="amendment", **extra):
    entry = {
        "key": key,
        "body": body,
        "base_revision": "approved-v1",
        "base_body_sha256": body_hash(base_body),
        "kind": kind,
    }
    entry.update(extra)
    return entry


class RecordingRunner:
    """A transport that remembers every call and can lose a create's response."""

    def __init__(self, snapshot_, ambiguous=0, lands_on_ambiguity=True):
        self.issues = {raw["number"]: dict(raw) for raw in snapshot_}
        self.calls = []
        self.ambiguous = ambiguous
        self.lands_on_ambiguity = lands_on_ambiguity
        self.next_number = max(self.issues, default=0) + 1
        self.read_back = None
        self.tamper_after_update = None

    def mutations(self):
        return [call for call in self.calls if call[0] in ("create", "update")]

    def read(self, number):
        self.calls.append(("read", number))
        if self.read_back is not None and number in self.read_back:
            return dict(self.issues[number], body=self.read_back[number])
        return dict(self.issues[number])

    def find(self, key):
        self.calls.append(("find", key))
        for number, raw in self.issues.items():
            if key_of(raw["body"]) == key:
                return number
        return None

    def _land(self, title, body):
        number = self.next_number
        self.next_number += 1
        self.issues[number] = dict(
            number=number, title=title, state="open", state_reason=None, updated_at="", labels=[], body=body
        )
        return number

    def create(self, title, body):
        self.calls.append(("create", title))
        if self.ambiguous:
            self.ambiguous -= 1
            if self.lands_on_ambiguity:
                self._land(title, body)
            raise AmbiguousCreate("the response was lost")
        return self._land(title, body)

    def update(self, number, body):
        self.calls.append(("update", number))
        self.issues[number]["body"] = body
        if self.tamper_after_update is not None:
            self.read_back = {number: self.tamper_after_update}
        return dict(self.issues[number])

    def snapshot(self):
        return [self.issues[n] for n in sorted(self.issues)]


class RegenerationSafety(unittest.TestCase):
    def setUp(self):
        self.base = snapshot()
        self.live = snapshot()

    def one(self, key, body, base_body, **extra):
        return plan(self.base, self.live, [amendment(key, body, base_body, **extra)])

    # 1. Duplicate, dangling and cyclic fixtures fail before any remote mutation.
    def test_a_plan_cannot_break_the_graph_before_any_mutation(self):
        cases = {
            "duplicate key": (task_body("A01"), "duplicate active key"),
            "dangling dependency": (task_body("A02", "- #999"), "dangling dependency"),
            "cycle": (task_body("A01", "- #4"), "dependency cycle"),
        }
        for label, (body, expected) in cases.items():
            with self.subTest(label):
                if label == "duplicate key":
                    planned = self.one("A02", body, task_body("A02", "- #3"))
                elif label == "dangling dependency":
                    planned = self.one("A02", body, task_body("A02", "- #3"))
                else:
                    planned = self.one("A01", body, task_body("A01"))
                self.assertEqual(planned["mutations"], [])
                self.assertTrue(planned["refusals"])
                detail = " ".join(entry.get("detail", "") for entry in planned["refusals"])
                self.assertIn(expected, detail)
                runner = RecordingRunner(self.live)
                with self.assertRaisesRegex(Exception, "carries refusals"):
                    apply(planned, runner)
                self.assertEqual(runner.mutations(), [], "a refused plan must attempt no mutation")

    def test_a_valid_plan_validates_the_intended_registry(self):
        planned = self.one("A02", task_body("A02", "- None"), task_body("A02", "- #3"))
        self.assertEqual(planned["refusals"], [])
        self.assertTrue(planned["intended_checked"])
        self.assertEqual([m["action"] for m in planned["mutations"]], ["update"])

    # 2. A stale bootstrap refuses to overwrite a newer approved body.
    def test_a_stale_payload_refuses_to_overwrite_newer_authority(self):
        self.live = snapshot(**{"4": task_body("A02", "- #3", revision="approved-v2")})
        planned = self.one("A02", task_body("A02", "- #3", notes="stale import"), task_body("A02", "- #3"))
        self.assertEqual(planned["mutations"], [])
        refusal = planned["refusals"][0]
        self.assertEqual(refusal["reason"], "stale revision refuses to overwrite newer authority")
        self.assertEqual(refusal["amendment_revision"], "approved-v1")
        self.assertEqual(refusal["live_revision"], "approved-v2")
        runner = RecordingRunner(self.live)
        with self.assertRaisesRegex(Exception, "carries refusals"):
            apply(planned, runner)
        self.assertEqual(runner.mutations(), [])
        self.assertEqual(self.live[3]["body"], task_body("A02", "- #3", revision="approved-v2"))

    def test_an_amendment_without_a_base_revision_is_refused(self):
        planned = plan(self.base, self.live, [{"key": "A02", "body": task_body("A02")}])
        self.assertEqual(planned["mutations"], [])
        self.assertIn("declares no base revision", planned["refusals"][0]["reason"])

    # 3. Concurrent human edits abort or merge visibly, never last-write-wins.
    def test_a_concurrent_human_edit_merges_when_disjoint(self):
        self.live = snapshot(**{"4": task_body("A02", "- #3", notes="original\nhuman note")})
        planned = self.one("A02", task_body("A02", "- #3", notes="regenerated"), task_body("A02", "- #3"))
        self.assertEqual(planned["refusals"], [])
        mutation = planned["mutations"][0]
        self.assertEqual(mutation["merge"], "merged")
        self.assertIn("human note", mutation["body"])
        self.assertIn("regenerated", mutation["body"])
        runner = RecordingRunner(self.live)
        report = apply(planned, runner)
        self.assertEqual(report["applied"][0]["number"], 4)
        self.assertIn("human note", runner.issues[4]["body"])
        self.assertEqual(verify(planned, self.live, runner.snapshot()), [])

    def test_a_concurrent_human_edit_conflicts_and_aborts(self):
        self.live = snapshot(**{"4": task_body("A02", "- #3", notes="human edit")})
        planned = self.one("A02", task_body("A02", "- #3", notes="regenerated"), task_body("A02", "- #3"))
        self.assertEqual(planned["mutations"], [])
        refusal = planned["refusals"][0]
        self.assertEqual(refusal["reason"], "concurrent human edit conflicts; abort rather than last-write-wins")
        self.assertEqual(refusal["conflicts"], [{"ours": ["regenerated"], "theirs": ["human edit"]}])
        runner = RecordingRunner(self.live)
        with self.assertRaisesRegex(Exception, "carries refusals"):
            apply(planned, runner)
        self.assertEqual(runner.mutations(), [])
        self.assertIn("human edit", runner.issues[4]["body"])

    def test_a_bootstrap_payload_may_not_drop_a_live_section(self):
        body = task_body("A02", "- #3") + "## Accepted amendment\nnewer wording\n"
        self.live = snapshot(**{"4": body})
        planned = self.one("A02", task_body("A02", "- #3"), body, kind="bootstrap")
        self.assertEqual(planned["mutations"], [])
        self.assertIn("Accepted amendment", planned["refusals"][0]["lost_sections"])

    # 3 (continued). An ambiguous create is reconciled, never duplicated.
    def create_plan(self, children="- [ ] #3\n- [ ] #4\n- [ ] #5\n"):
        """A create arrives with its epic's membership, as the graph requires."""
        epic_body = self.base[1]["body"]
        return plan(
            self.base,
            self.live,
            [
                amendment("E00", marked("E00", f"## Child issues\n{children}"), epic_body),
                amendment("A03", task_body("A03"), ""),
            ],
        )

    def test_a_create_is_validated_in_the_graph_before_any_mutation(self):
        planned = self.create_plan(children="- [ ] #3\n- [ ] #4\n")
        self.assertEqual(planned["mutations"], [])
        self.assertIn("epic membership differs", planned["refusals"][0]["detail"])
        runner = RecordingRunner(self.live)
        with self.assertRaisesRegex(Exception, "carries refusals"):
            apply(planned, runner)
        self.assertEqual(runner.mutations(), [])

    def test_an_ambiguous_create_reconciles_instead_of_duplicating(self):
        planned = self.create_plan()
        actions = [mutation["action"] for mutation in planned["mutations"]]
        self.assertEqual(actions, ["update", "create"])
        runner = RecordingRunner(self.live, ambiguous=1)
        report = apply(planned, runner)
        creates = [call for call in runner.calls if call[0] == "create"]
        self.assertEqual(creates, [("create", "A03")], "the response was lost, so it is not retried blind")
        carrying = [raw for raw in runner.issues.values() if key_of(raw["body"]) == "A03"]
        self.assertEqual(len(carrying), 1)
        self.assertEqual(carrying[0]["number"], report["applied"][1]["number"])
        self.assertEqual(verify(planned, self.live, runner.snapshot()), [])

    def test_a_lost_response_that_landed_nothing_creates_once(self):
        """The create's answer was lost and no issue landed: the key is read back, then created once."""
        planned = self.create_plan()
        runner = RecordingRunner(self.live, ambiguous=1, lands_on_ambiguity=False)
        report = apply(planned, runner)
        self.assertEqual(
            [call for call in runner.calls if call[0] == "create"],
            [("create", "A03"), ("create", "A03")],
            "the ambiguous attempt is reconciled before it is retried",
        )
        carrying = [raw for raw in runner.issues.values() if key_of(raw["body"]) == "A03"]
        self.assertEqual(len(carrying), 1, "no duplicate is left behind")
        self.assertEqual(carrying[0]["number"], report["applied"][1]["number"])
        self.assertEqual(verify(planned, self.live, runner.snapshot()), [])

    def test_a_second_ambiguity_fails_visibly(self):
        planned = self.create_plan()
        runner = RecordingRunner(self.live, ambiguous=2, lands_on_ambiguity=False)
        with self.assertRaisesRegex(Exception, "ambiguous create"):
            apply(planned, runner)
        self.assertEqual(
            [call for call in runner.calls if call[0] == "create"],
            [("create", "A03"), ("create", "A03")],
        )
        self.assertEqual([raw for raw in runner.issues.values() if key_of(raw["body"]) == "A03"], [])

    def test_an_existing_key_is_never_created_again(self):
        """A key a human made first is planned as an update, so no create is ever attempted."""
        self.live = snapshot() + [issue(5, "A03", task_body("A03"))]
        planned = plan(
            self.base,
            self.live,
            [
                amendment(
                    "E00",
                    marked("E00", "## Child issues\n- [ ] #3\n- [ ] #4\n- [ ] #5\n"),
                    self.base[1]["body"],
                ),
                amendment("A03", task_body("A03", notes="regenerated"), task_body("A03")),
            ],
        )
        self.assertEqual(planned["refusals"], [])
        self.assertEqual([mutation["action"] for mutation in planned["mutations"]], ["update", "update"])
        runner = RecordingRunner(self.live)
        report = apply(planned, runner)
        self.assertEqual([call for call in runner.calls if call[0] == "create"], [])
        self.assertEqual(report["applied"][1]["number"], 5)
        self.assertEqual(
            len([raw for raw in runner.issues.values() if key_of(raw["body"]) == "A03"]),
            1,
            "the key is carried by exactly one issue",
        )

    # 4. A reference-only entry keeps its history and resolves to canonical work.
    def test_a_reference_only_entry_is_never_regenerated(self):
        body = reference_body("A03", owner=3)
        alias = issue(5, "A03", body)
        alias["labels"] = [{"name": "planning:reference"}]
        self.live = snapshot() + [alias]
        # The amendment arrives with the epic listing the number a create would
        # land on, so the intended graph is valid and nothing but this rule stands
        # between the plan and a second A03.
        planned = plan(
            self.base,
            self.live,
            [
                amendment(
                    "E00",
                    marked("E00", "## Child issues\n- [ ] #3\n- [ ] #4\n- [ ] #6\n"),
                    self.base[1]["body"],
                ),
                amendment("A03", task_body("A03"), body),
            ],
        )
        self.assertEqual(planned["mutations"], [], "a reference entry is not an amendment target")
        refusal = planned["refusals"][0]
        self.assertIn("reference-only entries are never regenerated", refusal["reason"])
        self.assertEqual((refusal["number"], refusal["canonical_issue"]), (5, 3))
        runner = RecordingRunner(self.live)
        with self.assertRaisesRegex(Exception, "carries refusals"):
            apply(planned, runner)
        self.assertEqual(runner.mutations(), [])
        self.assertEqual(runner.issues[5]["body"], body, "the retained history is untouched")

    def test_an_amendment_reaches_canonical_work_the_reference_points_at(self):
        """The same edit addressed to the canonical issue is planned, not refused."""
        body = reference_body("A03", owner=3)
        alias = issue(5, "A03", body)
        alias["labels"] = [{"name": "planning:reference"}]
        self.live = snapshot() + [alias]
        planned = self.one("A01", task_body("A01", notes="regenerated"), task_body("A01"))
        self.assertEqual(planned["refusals"], [])
        self.assertEqual([mutation["number"] for mutation in planned["mutations"]], [3])

    # 6. Read-back proves the exact bodies and leaves unrelated state intact.
    def test_apply_refuses_when_a_target_moved_since_planning(self):
        planned = self.one("A02", task_body("A02", "- #3", notes="regenerated"), task_body("A02", "- #3"))
        runner = RecordingRunner(self.live)
        runner.issues[4]["body"] = task_body("A02", "- #3", notes="moved under us")
        with self.assertRaisesRegex(Exception, "moved since the plan was made"):
            apply(planned, runner)
        self.assertEqual(runner.mutations(), [], "no mutation was attempted")
        self.assertIn("moved under us", runner.issues[4]["body"])

    def test_a_read_back_that_does_not_match_is_a_failure(self):
        planned = self.one("A02", task_body("A02", "- #3", notes="regenerated"), task_body("A02", "- #3"))
        runner = RecordingRunner(self.live)
        runner.tamper_after_update = task_body("A02", "- #3", notes="something else landed")
        with self.assertRaisesRegex(Exception, "read-back of #4 does not match"):
            apply(planned, runner)

    def test_verify_reports_an_unplanned_modification(self):
        planned = self.one("A02", task_body("A02", "- #3", notes="regenerated"), task_body("A02", "- #3"))
        runner = RecordingRunner(self.live)
        apply(planned, runner)
        runner.issues[3]["body"] = task_body("A01", notes="edited without a plan")
        problems = verify(planned, self.live, runner.snapshot())
        self.assertEqual(problems, ["#3 was modified without being planned"])

    def test_verify_reports_a_state_change_and_a_disappearance(self):
        planned = self.one("A02", task_body("A02", "- #3", notes="regenerated"), task_body("A02", "- #3"))
        runner = RecordingRunner(self.live)
        apply(planned, runner)
        runner.issues[3]["state"] = "closed"
        del runner.issues[2]
        problems = verify(planned, self.live, runner.snapshot())
        self.assertEqual(problems, ["#2 disappeared from the inventory", "#3 changed state without being planned"])

    def test_the_dry_run_diff_shows_the_live_body_becoming_the_planned_one(self):
        live_body = task_body("A02", "- #3")
        planned = self.one("A02", task_body("A02", "- #3", notes="regenerated"), live_body)
        mutation = planned["mutations"][0]
        text = diff_of(mutation["key"], live_body, mutation["body"])
        self.assertIn("--- A02 (live)", text)
        self.assertIn("+++ A02 (planned)", text)
        self.assertIn("-original", text)
        self.assertIn("+regenerated", text)

    def test_a_dry_run_mutates_nothing(self):
        planned = self.one("A02", task_body("A02", "- #3", notes="regenerated"), task_body("A02", "- #3"))
        runner = RecordingRunner(self.live)
        report = apply(planned, runner, dry_run=True)
        self.assertTrue(report["dry_run"])
        self.assertEqual(runner.calls, [], "a dry run reads and writes nothing")
        self.assertEqual(runner.issues[4]["body"], task_body("A02", "- #3"))


if __name__ == "__main__":
    unittest.main()

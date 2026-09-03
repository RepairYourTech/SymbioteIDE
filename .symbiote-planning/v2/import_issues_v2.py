#!/usr/bin/env python3
"""
Idempotently import the Symbiote master program, epic trackers, and child issues.

Safety properties:
- Never deletes, closes, or reopens an issue.
- Never touches an issue unless its body contains an exact symbiote-plan-key marker.
- Preserves unrelated existing labels while adding required plan labels.
- Creates the master and epics first, then children, then resolves all cross-references.
- Can be run repeatedly after interruption.
"""
from __future__ import annotations
import argparse
import json
import re
import shutil
import subprocess
import sys
import time
from pathlib import Path
from typing import Any

MARKER_RE = re.compile(r"<!--\s*symbiote-plan-key:\s*([A-Z0-9-]+)\s*-->")
DEFAULT_BUNDLE = Path(__file__).with_name("issue-bundle-v2.json.gz.b64")

class ImportErrorWithContext(RuntimeError):
    pass

def load_bundle(path: Path) -> dict[str, Any]:
    if path.name.endswith(".json.gz.b64"):
        import base64
        import gzip
        raw = base64.b64decode(path.read_bytes())
        return json.loads(gzip.decompress(raw).decode("utf-8"))
    data = json.loads(path.read_text(encoding="utf-8"))
    issue_files = data.pop("issue_files", None)
    if issue_files is not None:
        issues: list[dict[str, Any]] = []
        for relative in issue_files:
            payload = json.loads((path.parent / relative).read_text(encoding="utf-8"))
            if not isinstance(payload, list):
                raise ImportErrorWithContext(f"Issue part {relative} must contain a JSON array.")
            issues.extend(payload)
        data["issues"] = issues
    return data

def run_gh(args: list[str], *, input_json: dict[str, Any] | None = None) -> Any:
    command = ["gh", *args]
    proc = subprocess.run(command, input=json.dumps(input_json) if input_json is not None else None, text=True, capture_output=True)
    if proc.returncode != 0:
        raise ImportErrorWithContext(f"Command failed ({proc.returncode}): {' '.join(command)}\nSTDOUT:\n{proc.stdout}\nSTDERR:\n{proc.stderr}")
    text = proc.stdout.strip()
    if not text:
        return None
    try:
        return json.loads(text)
    except json.JSONDecodeError:
        return text

def require_gh(repo: str) -> None:
    if shutil.which("gh") is None:
        raise ImportErrorWithContext("GitHub CLI (`gh`) is not installed or not on PATH.")
    run_gh(["auth", "status"])
    run_gh(["repo", "view", repo, "--json", "nameWithOwner"])

def paged_issues(repo: str) -> list[dict[str, Any]]:
    result = run_gh(["api", "--paginate", "--slurp", f"repos/{repo}/issues?state=all&per_page=100"])
    pages = result if isinstance(result, list) else []
    issues: list[dict[str, Any]] = []
    for page in pages:
        if isinstance(page, list):
            issues.extend(page)
    return [x for x in issues if "pull_request" not in x]

def plan_key(issue: dict[str, Any]) -> str | None:
    match = MARKER_RE.search(issue.get("body") or "")
    return match.group(1) if match else None

def ensure_labels(repo: str, labels: list[dict[str, str]], apply: bool) -> None:
    for label in labels:
        print(f"LABEL  {label['name']}")
        if apply:
            run_gh(["label", "create", label["name"], "--repo", repo, "--color", label["color"], "--description", label.get("description", ""), "--force"])

def merge_labels(existing: dict[str, Any] | None, required: list[str]) -> list[str]:
    current = []
    if existing:
        current = [x["name"] for x in existing.get("labels", []) if isinstance(x, dict) and "name" in x]
    return sorted(set(current) | set(required))

def create_issue(repo: str, item: dict[str, Any]) -> dict[str, Any]:
    payload = {"title": item["title"], "body": item["body"], "labels": item.get("labels", [])}
    return run_gh(["api", "-X", "POST", f"repos/{repo}/issues", "--input", "-"], input_json=payload)

def update_issue(repo: str, number: int, item: dict[str, Any], existing: dict[str, Any]) -> dict[str, Any]:
    payload = {"title": item["title"], "body": item["body"], "labels": merge_labels(existing, item.get("labels", []))}
    return run_gh(["api", "-X", "PATCH", f"repos/{repo}/issues/{number}", "--input", "-"], input_json=payload)

def resolve_body(body: str, key_to_number: dict[str, int]) -> str:
    output: list[str] = []
    keys = sorted(key_to_number, key=len, reverse=True)
    for line in body.splitlines():
        if line.startswith("<!-- symbiote-plan-key:") or line.startswith("**Plan key:**"):
            output.append(line)
            continue
        changed = line
        for key in keys:
            number = key_to_number[key]
            changed = changed.replace(f"`{key}`", f"#{number}")
            changed = changed.replace(f"**Parent epic:** {key}", f"**Parent epic:** #{number}")
        output.append(changed)
    return "\n".join(output).rstrip() + "\n"

def flatten(bundle: dict[str, Any]) -> list[dict[str, Any]]:
    return [bundle["program"], *bundle["epics"], *bundle["issues"]]

def validate_bundle(bundle: dict[str, Any]) -> None:
    items = flatten(bundle)
    keys = [x["key"] for x in items]
    if len(keys) != len(set(keys)):
        raise ImportErrorWithContext("Bundle contains duplicate plan keys.")
    child_keys = {x["key"] for x in bundle["issues"]}
    epic_keys = {x["key"] for x in bundle["epics"]}
    for item in bundle["issues"]:
        if item["epic"] not in epic_keys:
            raise ImportErrorWithContext(f"{item['key']} references unknown epic {item['epic']}.")
        unknown = [x for x in item.get("depends", []) if x not in child_keys]
        if unknown:
            raise ImportErrorWithContext(f"{item['key']} has unknown dependencies: {unknown}")
    for item in items:
        body = item.get("body", "")
        marker = MARKER_RE.search(body)
        if not marker or marker.group(1) != item["key"]:
            raise ImportErrorWithContext(f"{item['key']} is missing its exact body marker.")
        if len(body) > 65536:
            raise ImportErrorWithContext(f"{item['key']} body exceeds GitHub's issue-body limit.")

def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--bundle", type=Path, default=DEFAULT_BUNDLE)
    parser.add_argument("--repo", default=None)
    mode = parser.add_mutually_exclusive_group()
    mode.add_argument("--apply", action="store_true")
    mode.add_argument("--dry-run", action="store_true")
    parser.add_argument("--no-labels", action="store_true")
    parser.add_argument("--map-out", type=Path, default=Path("symbiote-created-issues.json"))
    parser.add_argument("--sleep", type=float, default=0.15)
    args = parser.parse_args()

    apply = bool(args.apply)
    bundle = load_bundle(args.bundle)
    validate_bundle(bundle)
    repo = args.repo or bundle["default_repository"]
    print(f"Bundle revision: {bundle['revision']}")
    print(f"Repository:      {repo}")
    print(f"Mode:            {'APPLY' if apply else 'DRY RUN'}")
    print(f"Objects:         1 program + {len(bundle['epics'])} epics + {len(bundle['issues'])} children")
    if apply:
        require_gh(repo)
    if not args.no_labels:
        ensure_labels(repo, bundle.get("labels", []), apply)

    existing_by_key: dict[str, dict[str, Any]] = {}
    if apply:
        for issue in paged_issues(repo):
            key = plan_key(issue)
            if key:
                if key in existing_by_key:
                    raise ImportErrorWithContext(f"Duplicate existing issues contain plan marker {key}: #{existing_by_key[key]['number']} and #{issue['number']}")
                existing_by_key[key] = issue

    key_to_number: dict[str, int] = {}
    all_items = flatten(bundle)
    for item in all_items:
        key = item["key"]
        existing = existing_by_key.get(key)
        if existing:
            key_to_number[key] = int(existing["number"])
            print(f"FOUND  {key:10s} #{existing['number']}  {item['title']}")
            continue
        print(f"CREATE {key:10s}             {item['title']}")
        if apply:
            created = create_issue(repo, item)
            key_to_number[key] = int(created["number"])
            existing_by_key[key] = created
            time.sleep(args.sleep)

    if not apply:
        print("\nDry run complete. No GitHub data was changed.")
        return 0

    for original in all_items:
        key = original["key"]
        number = key_to_number[key]
        resolved = dict(original)
        resolved["body"] = resolve_body(original["body"], key_to_number)
        existing = existing_by_key[key]
        print(f"SYNC   {key:10s} #{number:<5d} {original['title']}")
        updated = update_issue(repo, number, resolved, existing)
        existing_by_key[key] = updated
        time.sleep(args.sleep)

    result = {"repository": repo, "revision": bundle["revision"], "issues": {key: {"number": number, "url": existing_by_key[key].get("html_url"), "title": existing_by_key[key].get("title"), "state": existing_by_key[key].get("state")} for key, number in sorted(key_to_number.items())}}
    args.map_out.write_text(json.dumps(result, indent=2), encoding="utf-8")
    print(f"\nImported and synchronized {len(key_to_number)} plan-owned issues.")
    return 0

if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except ImportErrorWithContext as exc:
        print(f"\nERROR: {exc}", file=sys.stderr)
        raise SystemExit(2)

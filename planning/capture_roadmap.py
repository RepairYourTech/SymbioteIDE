"""Capture issue evidence without invoking historical importers (see #470)."""
import argparse
import json
import subprocess
from pathlib import Path


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    pages = json.loads(subprocess.check_output([
        "gh", "api", "--paginate", "--slurp",
        "repos/RepairYourTech/SymbioteIDE/issues?state=all&per_page=100",
    ], text=True))
    fields = ("number", "title", "body", "labels", "state", "state_reason",
              "updated_at", "html_url", "assignees", "milestone")
    rows = [{key: item.get(key) for key in fields}
            for page in pages for item in page if "pull_request" not in item]
    rows.sort(key=lambda item: item["number"])
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(rows, indent=2) + "\n", encoding="utf-8")
    print(f"Captured {len(rows)} issues to {args.output}; no remote writes")


if __name__ == "__main__":
    main()

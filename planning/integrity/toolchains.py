"""What this tree's manifests and workflows state about the Rust they run.

One reader, because two rules compare these facts and a fact with two readers drifts:
`test_toolchain_floors.py` holds the floor a crate declares against the toolchains the job that
builds it runs, and `test_handoff.py` holds the minimum the handoff names against what this
workspace declares and the legs its job proves it on. Both call what is here — measured when
each read the workflow for itself, one reformat of the workspace job's matrix red one rule and
left the other green.

Each fact is read out of the form it is written in, once: a manifest as the TOML it is, a
workflow as text (the standard library holds no YAML parser), with `entries` the one reader of
what a key states and `under` the one rule for what a path is. What cannot be resolved — a
manifest absent or malformed, a collection that never closes, a value only the runner knows —
is refused by name rather than read as nothing or as whatever happens to follow it.

What nothing here decides: whether a crate compiles on the floor it declares (a leg is a promise
to run, and only CI answers it), and which job a rule should be asking about.
"""
from __future__ import annotations

import pathlib
import re
import tomllib

ROOT = pathlib.Path(__file__).resolve().parents[2]
WORKFLOWS = ROOT / ".github/workflows"
JOB = re.compile(r"^  ([A-Za-z0-9_-]+):\s*$")
KEY = re.compile(r"^(\s*)(?:-\s+)?([A-Za-z0-9_.\-]+):(.*)$")
ITEM = re.compile(r"^(\s*)-\s*(.*?)\s*$")
FLAG = re.compile(r"--manifest-path[= ](\S+)")
GATED = re.compile(r"matrix\.toolchain == '([^']+)'")
FLOW = re.compile(r"^(\s*(?:-\s+)?(?:[A-Za-z0-9_.\-]+:\s*)?)\{(.*)$")
WORKSPACE = "${{ github.workspace }}"
ENTRY = tuple[list[str], int, int]  # what an entry states, and the lines it spans


def jobs(workflow: str) -> dict[str, str]:
    """Each job of a workflow, with the text of its own block.

    A job key is indented two spaces under `jobs:` and everything the job holds is indented
    further, so a job's block ends where a line starts at column zero.
    """
    found: dict[str, str] = {}
    name: str | None = None
    inside = False
    for line in workflow.splitlines():
        if line.startswith("jobs:"):
            inside = True
            continue
        if not inside:
            continue
        if line and not line.startswith(" "):
            break
        key = JOB.fullmatch(line)
        if key:
            name = key.group(1)
            found[name] = ""
        elif name is not None:
            found[name] += line + "\n"
    return found


def flow_items(collection: str) -> list[str]:
    """The top-level items a flow collection holds, without its punctuation.

    Commas inside a nested collection, and inside quotes, separate nothing here: a matrix
    entry is one item however many brackets it carries.
    """
    items, depth, quote, current = [], 0, "", ""
    for character in collection.strip()[1:-1]:
        if quote:
            quote = "" if character == quote else quote
        elif character in "\"'":
            quote = character
        elif character in "[{":
            depth += 1
        elif character in "]}":
            depth -= 1
        elif character == "," and not depth:
            items.append(current)
            current = ""
            continue
        current += character
    items.append(current)
    return [item.strip() for item in items if item.strip()]


def expanded(block: str, where: str) -> str:
    """The block with every flow mapping on one line put into the block form it means.

    YAML lets a mapping be written inline (`matrix: {toolchain: [a, b]}`, `- {run: …}`), so it
    is unfolded before anything reads the block, and a key written either way is found where
    the job wrote it.
    """
    out: list[str] = []
    for line in block.splitlines():
        opened = FLOW.match(line)
        if not opened:
            out.append(line)
            continue
        prefix, body = opened.group(1), opened.group(2)
        if not body.rstrip().endswith("}"):
            raise AssertionError(f"{where} writes {line.strip()}, a mapping this reader does "
                                 f"not read across lines")
        items = flow_items("{" + body)
        if prefix.rstrip().endswith(":"):
            out.append(prefix.rstrip())
        else:
            out.append((prefix + items[0]).rstrip())  # a step: its first key stays on the dash
            items = items[1:]
        out.extend(" " * (len(prefix) - len(prefix.lstrip()) + 2) + item for item in items)
    return "\n".join(out)


def entries(block: str, name: str, where: str) -> list[ENTRY]:
    """Every entry a block states under `name`: what it says, and the lines it spans.

    The forms YAML allows: the text after the key, a flow collection on one line or across
    several, a block sequence whose items are at its own indentation, or a block scalar.
    """
    lines = expanded(block, where).splitlines()
    found: list[ENTRY] = []
    index = 0
    while index < len(lines):
        key = KEY.match(lines[index])
        if not key or key.group(2) != name:
            index += 1
            continue
        indent, tail = len(key.group(1)), key.group(3).split("#")[0].strip()
        first, index = index, index + 1
        if tail.startswith("["):
            while not tail.endswith("]"):
                if index >= len(lines):
                    raise AssertionError(f"{where} states {tail}, which never closes")
                tail += " " + lines[index].split("#")[0].strip()
                index += 1
            found.append(([item.strip().strip("\"'") for item in flow_items(tail)],
                          first, index - 1))
        elif tail[:1] in ("|", ">"):
            body: list[str] = []
            while index < len(lines) and (not lines[index].strip() or len(lines[index])
                                          - len(lines[index].lstrip()) > indent):
                body.extend(lines[index].split())
                index += 1
            found.append((body, first, index - 1))
        elif tail:
            found.append(([tail.strip("\"'")], first, first))
        else:
            while index < len(lines):
                if not lines[index].strip() or lines[index].lstrip().startswith("#"):
                    index += 1  # a comment between the items states no item and ends none
                    continue
                item = ITEM.match(lines[index])
                if not item or len(item.group(1)) < indent:
                    break
                end = index + 1
                while end < len(lines) and (not lines[end].strip() or len(lines[end])
                                            - len(lines[end].lstrip()) > len(item.group(1))):
                    end += 1
                stated = (item.group(2) or "").split("#")[0].strip().strip("\"'")
                found.append(([stated] if stated else [], index, end - 1))
                index = end
    return found


def parts(block: str, where: str) -> tuple[list[str], str]:
    """A job's steps, and the job's own text outside them.

    An item whose keys are indented under a bare dash is a step like any other, so what is
    left of the block is the job's own text — where its default `working-directory` sits,
    before or after the steps, in whichever order the job writes its keys.
    """
    lines = expanded(block, where).splitlines(keepends=True)
    spans = entries(block, "steps", where)
    inside = {line for _, first, last in spans for line in range(first, last + 1)}
    return (["".join(lines[first:last + 1]) for _, first, last in spans],
            "".join(line for index, line in enumerate(lines) if index not in inside))


def under(base: pathlib.Path, value: str, where: str, what: str) -> pathlib.Path:
    """The path a workflow states: against the directory it runs in, or this repository.

    The workspace template is this checkout, and a value carrying a template or a shell
    variable names something only the runner knows — refused rather than read as a path this
    tree would then be checked against.
    """
    if value.startswith(WORKSPACE):
        return ROOT / value[len(WORKSPACE):].strip("/")
    if "$" in value:
        raise AssertionError(f"{where} names {what} {value}, which this file cannot resolve")
    return base / value


def directory(text: str, where: str) -> pathlib.Path | None:
    """The `working-directory` a block states, resolved; None when it states none."""
    stated = entries(text, "working-directory", where)
    return under(ROOT, stated[0][0][0], where, "a directory") if stated and stated[0][0] else None


def manifests(workflow: str, job: str, block: str) -> list[pathlib.Path]:
    """The manifests a job's own text names, each against the directory its step runs in.

    A job names its crate in every cargo step it runs; one crate is one pair.
    """
    where = f"{workflow}: the job {job}"
    steps, outside = parts(block, where)
    default = directory(outside, where)
    found = [under(directory(step, f"{where}, step {number}") or default or ROOT,
                   named.strip("\"'"), f"{where}, step {number}", "its crate")
             for number, step in enumerate(steps, 1) for named in FLAG.findall(step)]
    return list(dict.fromkeys(found))


def pairs() -> list[tuple[str, str, str, pathlib.Path]]:
    """(workflow, job, the job's own block, the manifest it names) for every job."""
    return [(path.name, job, block, manifest)
            for path in sorted(WORKFLOWS.glob("*.yml"))
            for job, block in jobs(path.read_text()).items()
            for manifest in manifests(path.name, job, block)]


def legs(block: str, where: str = "a job") -> list[str]:
    """Every toolchain a job's own block states, as the union of the ways it states them.

    A matrix states its legs as a flow or block sequence and a step may install one toolchain
    beside that; a job that does both runs what both name. A `${{ … }}` template names nothing
    comparable and states no leg.
    """
    stated = [leg for values, _, _ in entries(block, "toolchain", where) for leg in values]
    return list(dict.fromkeys(leg.strip("\"'") for leg in stated if leg and "${{" not in leg))


def gated(block: str) -> list[str]:
    """The legs a job's own steps are gated on (`if: matrix.toolchain == 'stable'`)."""
    return list(dict.fromkeys(GATED.findall(block)))


def parsed(manifest: pathlib.Path) -> dict:
    """A manifest's TOML, naming an absent or malformed one rather than raising for it."""
    try:
        text = manifest.read_text()
    except OSError as error:
        raise AssertionError(f"{manifest} is not a manifest this tree holds: {error}") from error
    try:
        return tomllib.loads(text)
    except tomllib.TOMLDecodeError as error:
        raise AssertionError(f"{manifest} is not TOML: {error}") from error


def declared(manifest: pathlib.Path, table: str) -> str | None:
    """The `rust-version` a named table of a manifest states, or None."""
    stated: object = parsed(manifest)
    for name in (*table.split("."), "rust-version"):
        stated = stated.get(name) if isinstance(stated, dict) else None
    return stated if isinstance(stated, str) else None


def workspace_of(manifest: pathlib.Path) -> pathlib.Path | None:
    """The manifest that is this crate's workspace, as cargo resolves it.

    A crate may name the root itself (`[package] workspace = ".."`), and cargo honours that
    over the directory it sits in, so the named root is read first; failing that, the nearest
    manifest above it that declares a workspace.
    """
    package = parsed(manifest).get("package")
    named = package.get("workspace") if isinstance(package, dict) else None
    if isinstance(named, str):
        root = (manifest.parent / named).resolve() / "Cargo.toml"
        if not root.is_file():
            raise AssertionError(f"{manifest} names its workspace {named}, "
                                 f"which holds no Cargo.toml")
        return root
    for directory in manifest.parents:
        candidate = directory / "Cargo.toml"
        if candidate.is_file() and "workspace" in parsed(candidate):
            return candidate
    return None


def floor(manifest: pathlib.Path) -> str | None:
    """The floor a manifest declares, or the one its own workspace declares.

    Only a string under `[package]` is the crate's own floor — that workspace root's own
    `[package]` floor is not the member's. A crate inheriting one (`rust-version.workspace =
    true`) takes its workspace's `[workspace.package]` floor, and a crate inheriting one its
    workspace does not state is one cargo refuses to build, so there is nothing to hold.
    """
    package = parsed(manifest).get("package")
    own = package.get("rust-version") if isinstance(package, dict) else None
    if isinstance(own, str):
        return own
    if not (isinstance(own, dict) and own.get("workspace") is True):
        return None
    workspace = workspace_of(manifest)
    return declared(workspace, "workspace.package") if workspace else None


def version(stated: str) -> str:
    """A floor as a toolchain leg is spelled: `1.85` and `1.85.0` are one version."""
    return stated if stated.count(".") == 2 else f"{stated}.0"

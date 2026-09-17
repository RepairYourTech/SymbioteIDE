"""What this tree's manifests and workflows state about the Rust they run.

One reader, because two rules compare these facts and a fact with two readers is a fact
that drifts: `test_toolchain_floors.py` holds the floor a crate declares against the
toolchains the job that builds it runs, and `test_handoff.py` holds the minimum
`docs/engineering-handoff.md` names against what this workspace declares and the legs the job
that runs the workspace proves it on. Both call what is here, because measured when each read
the workflow for itself, rewriting the workspace job's matrix as a block sequence — the same
two legs — red the handoff's rule and left this one green.

Each fact is read out of the form it is written in, once.

* A manifest is TOML, so it is parsed as TOML: a table header may carry a comment or inner
  whitespace, a floor may be quoted or spaced either way, and `rust-version.workspace = true`
  is the dotted key it is. A crate's workspace is the root it names itself, else the nearest
  one above it — the root cargo would inherit a floor from — and a manifest that is absent, is
  not TOML, or names a workspace holding no manifest, is refused by name.
* A workflow is read as text, because the standard library holds no YAML parser. `entries` is
  the one reader of what a key states: the text after the key, a flow collection on one line
  or across several, a flow mapping (unfolded into the block form it means, on a key or on a
  step), a block sequence, or a block scalar. A job's toolchains are the union of every
  `toolchain:` it states, so its matrix and its own step's install are read the same way and
  the order of the lines decides nothing; `${{ … }}` names nothing comparable. A collection
  that never closes, and a mapping written across lines, are refused by name rather than read
  as whatever happens to follow them.
* A path a workflow states is resolved by `under` against the directory its step runs in, or
  against this repository when the value is the workspace template — that is what GitHub
  fills it with, and so is every path under it. A value only the runner knows (a matrix
  entry, a shell variable) names nothing this file can resolve and is refused: a job stating
  its crate through a template does not name a file here, and a path invented for it would be
  checked against this tree as if the run had named it.

What nothing here decides: whether a crate compiles on the floor it declares (a leg is a
promise to run, and only CI answers it), and which job a rule should be asking about.
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
MAPPING = re.compile(r"^(\s*(?:-\s+)?[A-Za-z0-9_.\-]+:\s*)\{(.*)$")
STEP_MAPPING = re.compile(r"^(\s*-)\s*\{(.*)$")
WORKSPACE = "${{ github.workspace }}"
ENTRY = tuple[list[str], int, int]  # what an entry states, and the lines it spans


def jobs(workflow: str) -> dict[str, str]:
    """Each job of a workflow, with the text of its own block.

    A job key is indented two spaces under `jobs:` and everything a job holds is
    indented further, so the block ends where a line starts at column zero.
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
    """The block with every flow mapping written on one line put into block form.

    YAML lets a mapping be written inline (`matrix: {toolchain: [a, b]}`, `- {run: …}` under
    `steps:`) and every key in it states what the block form states, so it is unfolded
    before anything is read: one reader then answers both spellings, and a key written
    either way is found where the job wrote it.
    """
    out: list[str] = []
    for line in block.splitlines():
        keyed = MAPPING.match(line)
        step = None if keyed else STEP_MAPPING.match(line)
        opened = keyed or step
        if not opened:
            out.append(line)
            continue
        if not opened.group(2).rstrip().endswith("}"):
            raise AssertionError(f"{where} writes {line.strip()}, a mapping this reader does "
                                 f"not read across lines")
        items = flow_items("{" + opened.group(2))
        if not all(":" in item for item in items):
            out.append(line)  # a brace expression, not a mapping of keys
            continue
        if step:
            out.append(f"{opened.group(1)} {items[0]}".rstrip())
            items = items[1:]
        else:
            out.append(opened.group(1).rstrip())
        indent = len(opened.group(1)) - len(opened.group(1).lstrip()) + 2
        out.extend(" " * indent + item for item in items)
    return "\n".join(out)


def entries(block: str, name: str, where: str) -> list[ENTRY]:
    """Every entry a block states under `name`: what it says, and the lines it spans.

    A key states its value in one of the forms YAML allows, and all of them are read here,
    once: the text after the key, a flow collection — on one line or across several — a
    block sequence, whose entries are the items at its own indentation, or a block scalar.
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
            found.append(([item.strip().strip("\"'") for item in flow_items(tail)], first, index - 1))
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
                stated = (item.group(2) or "").split("#")[0].strip().strip("\"'{},")
                found.append(([stated] if stated else [], index, end - 1))
                index = end
    return found


def parts(block: str, where: str) -> tuple[list[str], str]:
    """A job's steps, and the job's own text outside them.

    A job's steps are the entries of its `steps:` sequence — an item whose keys are indented
    under a bare dash is a step like any other — so what is left is the job's own text, where
    its default `working-directory` sits, before or after the steps, in whichever order the
    job writes them.
    """
    lines = expanded(block, where).splitlines(keepends=True)
    spans = entries(block, "steps", where)
    inside = {line for _, first, last in spans for line in range(first, last + 1)}
    return (["".join(lines[first:last + 1]) for _, first, last in spans],
            "".join(line for index, line in enumerate(lines) if index not in inside))


def under(base: pathlib.Path, value: str, where: str, what: str) -> pathlib.Path:
    """The path a workflow states: against the base it runs in, or this repository.

    The workspace template is this checkout, so it and every path under it resolve here; a
    value carrying a template or a shell variable names something only the runner knows, and
    is refused by name rather than read as a path this tree would then be checked against.
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
                   named.strip("\"'").rstrip(",}]"), f"{where}, step {number}", "its crate")
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

    A matrix states its legs as a flow sequence — on one line or across several, inline or in
    block form — or as a block sequence, and a step may install one toolchain beside that; a
    job that does both runs what both name. A block scalar is read as the value under its
    key, and a `${{ … }}` template names nothing comparable and states no leg.
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
    """The `rust-version` a named table of a manifest states, or None.

    Only `[package]` states a crate's own floor and only `[workspace.package]` states the
    one its members inherit, so the table asked for decides what the answer means.
    """
    stated: object = parsed(manifest)
    for name in (*table.split("."), "rust-version"):
        stated = stated.get(name) if isinstance(stated, dict) else None
    return stated if isinstance(stated, str) else None


def workspace_of(manifest: pathlib.Path) -> pathlib.Path | None:
    """The manifest that is this crate's workspace, as cargo resolves it.

    A crate may name the root itself (`[package] workspace = ".."`), and cargo honours that
    over the directory the crate sits in, so the named root is read first — a crate pinning a
    root above a nested `[workspace]` inherits from the root it names. Failing that, the
    nearest manifest above it that declares a workspace.
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

    Only a string under `[package]` is the crate's own floor; a crate that inherits one
    (`rust-version.workspace = true`) takes its workspace's `[workspace.package]` floor, and
    a crate inheriting one its workspace does not state is one cargo refuses to build — there
    is nothing here to hold it against. The workspace root's own `[package]` floor is not the
    member's.
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

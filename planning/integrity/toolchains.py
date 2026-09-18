"""What this tree's manifests and workflows state about the Rust they run.

One reader, because two rules compare these facts and a fact with two readers drifts:
`test_toolchain_floors.py` holds the floor a crate declares against the toolchains the job that
builds it runs, and `test_handoff.py` holds the minimum the handoff names against what this
workspace declares and the legs its job proves it on. Both call what is here — measured: when
each read the workflow for itself, one reformat of the workspace job's matrix red one rule and
left the other green.

A workflow is read as text (no YAML parser in the standard library) and a manifest as the TOML
it is; nothing unresolvable is read as nothing — an absent or malformed manifest, a collection
that never closes, and a value only the runner knows are each refused by name.
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
FLOW = re.compile(r"^(\s*(?:-\s+)?(?:[A-Za-z0-9_.\-]+:\s*)?)\{(.*)$")
WORKSPACE = "${{ github.workspace }}"
ENTRY = tuple[list[str], int, int]  # what an entry states, and the lines it spans


def jobs(workflow: str) -> dict[str, str]:
    """Each job of a workflow, with the text of its own block: a job key is two spaces under
    `jobs:`, and its block ends where a line starts at column zero.
    """
    lines = workflow.splitlines()
    start = next((index for index, line in enumerate(lines) if line.startswith("jobs:")), None)
    if start is None:
        return {}
    body = [line for line in lines[start + 1:] if not line.startswith("jobs:")]
    stop = next((n for n, text in enumerate(body) if text and not text.startswith(" ")),
                len(body))
    keys = [index for index, line in enumerate(body) if index < stop and JOB.fullmatch(line)]
    return {JOB.fullmatch(body[key]).group(1): "".join(line + "\n" for line in body[key + 1:end])
            for key, end in zip(keys, keys[1:] + [stop])}


def flow_items(collection: str) -> list[str]:
    """The top-level items a flow collection holds, without its punctuation: a comma inside a
    nested collection, or inside quotes, separates nothing.
    """
    out, depth, quote, current = [], 0, "", ""
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
            out.append(current)
            current = ""
            continue
        current += character
    return [item.strip() for item in (*out, current) if item.strip()]


def unfolded(block: str, where: str) -> str:
    """The block with every flow mapping written on one line put into the block form it means.

    A mapping written inline (`matrix: {toolchain: [a, b]}`, `- {run: …}`) is unfolded before
    anything reads the lines, so a key written either way is found where the job wrote it.
    """
    def splice(match: re.Match[str]) -> str:
        prefix, body = match.group(1), match.group(2)
        if not body.rstrip().endswith("}"):
            raise AssertionError(f"{where} writes {match.group(0).strip()}, a mapping this "
                                 f"reader does not read across lines")
        items = flow_items("{" + body)
        head, rest = prefix.rstrip(), items
        if not head.endswith(":"):  # a step: its first key stays on the dash
            head, rest = (prefix + items[0]).rstrip(), items[1:]
        indent = " " * (len(prefix) - len(prefix.lstrip()) + 2)
        return head + "".join("\n" + indent + item for item in rest)
    return "\n".join(FLOW.sub(splice, line) for line in block.splitlines())


def entries(block: str, name: str, where: str) -> list[ENTRY]:
    """Every entry a block states under `name`: what it says, and the lines it spans.

    One pass reads the forms a value is written in: the text after the key, a flow collection on
    one line or across several, a block scalar, and a block sequence, whose items are one entry
    each — so removing the step lines leaves the job's own text. A value written over several
    lines is one entry with them: a plain scalar runs onto every line deeper than its key, which
    is what a parser folds into one value, and a shell continuation (`\\`) also carries the
    lines it runs onto where the next line is not deeper. The marker is not part of what it
    says, and a line stating a key of its own, or one stating no word (blank, or a comment), is
    where it ends.
    """
    lines = unfolded(block, where).splitlines()
    found: list[ENTRY] = []
    index = 0
    while index < len(lines):
        key = KEY.match(lines[index])
        if not key or key.group(2) != name:
            index += 1
            continue
        first, indent = index, key.start(2)  # the key's own column, not the dash's
        tail, index = key.group(3).split("#")[0].strip(), index + 1
        if tail.startswith("["):
            while not tail.endswith("]"):
                if index == len(lines):
                    raise AssertionError(f"{where} states {tail}, which never closes")
                tail += " " + lines[index].split("#")[0].strip()
                index += 1
            found.append(([item.strip("\"'") for item in flow_items(tail)], first, index - 1))
        elif tail[:1] in ("|", ">"):
            words: list[str] = []
            while index < len(lines) and (not lines[index].strip() or len(lines[index])
                                          - len(lines[index].lstrip()) > indent):
                words, index = words + lines[index].split(), index + 1
            found.append((words, first, index - 1))
        elif tail:
            said = [tail]  # a plain scalar runs onto the lines deeper than it, and a marker
            while index < len(lines) and not KEY.match(lines[index]):
                marker = said[-1].rstrip().endswith("\\")
                if not (marker or len(lines[index]) - len(lines[index].lstrip()) > indent):
                    break  # neither a continuation nor a line the scalar runs onto
                if marker:
                    # the marker, and the space before it, are not part of what the entry says
                    said[-1] = said[-1].rstrip()[:-1].rstrip()
                stated = lines[index].split("#")[0].strip()
                if not stated:
                    break  # a blank or commented line states no word, and ends the entry
                said.append(stated)
                index += 1
            found.append(([word.strip("\"'") for word in said], first, index - 1))
        else:
            while index < len(lines):  # a block sequence: one entry per item
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
                stated = item.group(2).split("#")[0].strip().strip("\"'")
                found.append(([stated] if stated else [], index, end - 1))
                index = end
    return found


def under(base: pathlib.Path, value: str, where: str, what: str) -> pathlib.Path:
    """The path a workflow states: against the directory it runs in, or this repository.

    `${{ github.workspace }}` is this checkout; any other template, or a shell variable, names
    something only the runner knows — refused by name rather than read as a path here.
    """
    if value.startswith(WORKSPACE):
        return ROOT / value[len(WORKSPACE):].strip("/")
    if "$" in value:
        raise AssertionError(f"{where} names {what} {value}, which this file cannot resolve")
    return base / value


def directory(text: str, where: str) -> pathlib.Path | None:
    """The `working-directory` a block states, resolved; None when it states none.

    Every word the entry states, folded the way a plain scalar folds its line breaks, rather
    than its first word alone: a directory written over more than one line is one directory,
    and reading the first word took a continued value with its marker still in it.
    """
    stated = entries(text, "working-directory", where)
    return (under(ROOT, " ".join(stated[0][0]), where, "a directory")
            if stated and stated[0][0] else None)


def manifests(workflow: str, job: str, block: str) -> list[pathlib.Path]:
    """The manifests a job's own text names, each against the directory its step runs in.

    A job names its crate in every cargo step it runs; a step's own `working-directory` decides
    for that step, and the job's default — written before or after the steps, in whichever order
    the job writes its keys — for the rest.
    """
    where = f"{workflow}: the job {job}"
    lines = unfolded(block, where).splitlines(keepends=True)
    spans = entries(block, "steps", where)
    inside = {line for _, first, last in spans for line in range(first, last + 1)}
    default = directory("".join(line for index, line in enumerate(lines)
                                if index not in inside), where)
    steps = ["".join(lines[first:last + 1]) for _, first, last in spans]
    found = [under(directory(step, f"{where}, step {number}") or default or ROOT,
                   named.strip("\"'"), f"{where}, step {number}", "its crate")
             for number, step in enumerate(steps, 1)
             for named in re.findall(r"--manifest-path[= ](\S+)", step)]
    return list(dict.fromkeys(found))


def pairs() -> list[tuple[str, str, str, pathlib.Path]]:
    """(workflow, job, the job's own block, the manifest it names) for every job."""
    return [(path.name, job, block, manifest) for path in sorted(WORKFLOWS.glob("*.yml"))
            for job, block in jobs(path.read_text()).items()
            for manifest in manifests(path.name, job, block)]


def legs(block: str, where: str = "a job") -> list[str]:
    """Every toolchain a job's own block states, as the union of the ways it states them.

    A matrix states its legs as a flow or block sequence, a step may install one toolchain
    beside that, and a `${{ … }}` template states no leg.
    """
    stated = [leg for values, _, _ in entries(block, "toolchain", where) for leg in values]
    return list(dict.fromkeys(leg.strip("\"'") for leg in stated if leg and "${{" not in leg))


def gated(block: str) -> list[str]:
    """The legs a job's own steps are gated on (`if: matrix.toolchain == 'stable'`)."""
    return list(dict.fromkeys(re.findall(r"matrix\.toolchain == '([^']+)'", block)))


def workspace_jobs(workflow: str) -> list[str]:
    """The blocks of the jobs whose steps run the workspace's own tests.

    Which job proves the workspace is a fact of the workflow rather than of a name — measured, one
    job of `rust-contracts.yml` runs `cargo test --workspace`, and renaming it changes nothing about
    what CI proves. A name would also make the answer depend on the order the jobs are written in,
    and reading only the first would ignore a second job that proves the workspace just as much.
    A step's own words are joined before they are read, so a command written inline and the same
    command under `run: |` are the same command.
    """
    return [block for block in jobs(workflow).values()
            if any("cargo test" in stated and "--workspace" in stated
                   for values, _, _ in entries(block, "run", "a job")
                   for stated in [" ".join(values)])]


def parsed(manifest: pathlib.Path) -> dict:
    """A manifest's TOML, naming an absent or malformed one rather than raising for it."""
    try:
        return tomllib.loads(manifest.read_text())
    except (OSError, tomllib.TOMLDecodeError) as error:
        what = ("not TOML" if isinstance(error, tomllib.TOMLDecodeError)
                else "not a manifest this tree holds")
        raise AssertionError(f"{manifest} is {what}: {error}") from error


def declared(manifest: pathlib.Path, table: str) -> str | None:
    """The `rust-version` a named table of a manifest states, or None."""
    stated: object = parsed(manifest)
    for name in (*table.split("."), "rust-version"):
        stated = stated.get(name) if isinstance(stated, dict) else None
    return stated if isinstance(stated, str) else None


def workspace_of(manifest: pathlib.Path) -> pathlib.Path | None:
    """The manifest that is this crate's workspace, as cargo resolves it.

    A crate may name the root itself (`[package] workspace = ".."`), which cargo honours over
    the directory it sits in; failing that, the nearest manifest above it declaring a workspace.
    """
    package = parsed(manifest).get("package")
    named = package.get("workspace") if isinstance(package, dict) else None
    if isinstance(named, str):
        root = (manifest.parent / named).resolve() / "Cargo.toml"
        if not root.is_file():
            raise AssertionError(f"{manifest} names its workspace {named}, "
                                 f"which holds no Cargo.toml")
        return root
    return next((parent / "Cargo.toml" for parent in manifest.parents
                 if (parent / "Cargo.toml").is_file()
                 and "workspace" in parsed(parent / "Cargo.toml")), None)


def floor(manifest: pathlib.Path) -> str | None:
    """The floor a manifest declares, or the one its own workspace declares.

    Only a string under `[package]` is the crate's own floor — a workspace root's own
    `[package]` floor is not its member's. A crate inheriting one (`rust-version.workspace =
    true`) takes its workspace's `[workspace.package]` floor, and one whose workspace states
    none is a crate cargo refuses to build, so nothing has to hold it.
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

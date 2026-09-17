"""What this tree's manifests and workflows state about the Rust they run.

One reader, because two rules compare these facts and a fact with two readers is a fact
that drifts: `test_toolchain_floors.py` holds the floor a crate declares against the
toolchains the job that builds it runs, and `test_handoff.py` holds the minimum
`docs/engineering-handoff.md` names against what this workspace declares and the legs the
job that runs the workspace proves it on. Both call what is here, because measured when each
read the workflow for itself, rewriting the workspace job's matrix as a block sequence — the
same two legs — red the handoff's rule and left this one green.

A manifest is TOML, so it is parsed as TOML rather than matched line by line: `[package]`
may carry a comment or inner whitespace, a floor may be quoted either way or spaced either
way, and `rust-version.workspace = true` is a dotted key rather than a spelling to hunt
for. Three guesses at that text went away with the parser, and a manifest that is not TOML
is named rather than left as a decode error.

A workflow is read as text, because the standard library holds no YAML parser: a job's
toolchains are the union of what its own block states — a matrix's legs as a flow sequence
on one line or across several, or as a block sequence at any indentation, plus the one
toolchain a step installs (`with: toolchain: stable`) — so which of those lines comes first
decides nothing, and a `${{ … }}` template names nothing comparable. Reading one form as nothing reports a job as running
none, which is how a job with a commented or a same-indent matrix was read before this.
A manifest is resolved the way CI runs the command that names it: against the
`working-directory` of its own step, else the job's own default, else the repository root —
not against the first such line in the block, which is what let one step's directory decide
every other step, and not against the root when the line was a template. The workspace
template is read as that root — and so is every path under it — because that is what GitHub
resolves it to; any other template names a directory this file cannot see and is refused by
name. A crate's workspace is the root it names itself, else the nearest one above it, which
is the root cargo would inherit a floor from. A floor and a leg are compared as versions
rather than as strings, so that normalisation (`1.85` and `1.85.0` are one version) has one
home here too.

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
FLAG = re.compile(r"--manifest-path[= ](\S+)")
TOOLCHAIN = re.compile(r"^(\s*)toolchain:(.*)$")
# a sequence item, whose content may sit on the next line instead of after the dash
ITEM = re.compile(r"^(\s*)-(?:\s+(\S.*?))?\s*$")
GATED = re.compile(r"matrix\.toolchain == '([^']+)'")
# a step's first key sits after its `- `, a job's own default does not
DIRECTORY = re.compile(r"^(\s*)(?:-\s+)?working-directory:(.*)$", re.M)
STEPS = re.compile(r"^(\s*)steps:\s*$")
WORKSPACE = "${{ github.workspace }}"


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


def parts(block: str) -> tuple[list[str], str]:
    """A job's steps, and the job's own text outside them; block order decides nothing.

    A job's steps are the items of the shallowest `-` sequence under its `steps:` key — an
    item whose keys are indented under a bare dash is a step like any other — so what is
    left is the job's own text, where its default `working-directory` sits, before or after
    the steps, in whichever order the job writes them.
    """
    lines = block.splitlines(keepends=True)
    head = next((index for index, line in enumerate(lines) if STEPS.match(line)), None)
    if head is None:
        return [], block
    items = [(len(found.group(1)), index) for index, line in enumerate(lines[head + 1:], head + 1)
             if (found := ITEM.match(line))]
    if not items:
        return [], block
    indent = min(i for i, _ in items)
    spans = []
    for start in (index for i, index in items if i == indent):
        end = start + 1
        while end < len(lines) and (not lines[end].strip()
                                    or len(lines[end]) - len(lines[end].lstrip()) > indent):
            end += 1
        spans.append((start, end))
    return (["".join(lines[start:end]) for start, end in spans],
            "".join(line for index, line in enumerate(lines)
                    if not any(start <= index < end for start, end in spans)))


def directory(text: str, where: str) -> pathlib.Path | None:
    """The `working-directory` a block states, resolved; None when it states none.

    The workspace template is the repository root — that is what GitHub resolves it to —
    and so is every path under it; any other template names a directory this file cannot
    see, and is refused rather than read as one.
    """
    stated = DIRECTORY.search(text)
    if not stated:
        return None
    value = stated.group(2).split("#")[0].strip().strip("\"'")
    if value.startswith(WORKSPACE):
        return ROOT / value[len(WORKSPACE):].strip("/")
    if "${{" in value:
        raise AssertionError(f"{where} runs in {value}, which this tree cannot resolve")
    return ROOT / value


def manifests(workflow: str, job: str, block: str) -> list[pathlib.Path]:
    """The manifests a job's own text names, each against the directory its step runs in.

    A job names its crate in every cargo step it runs; one crate is one pair.
    """
    steps, outside = parts(block)
    default = directory(outside, f"{workflow}: the job {job}")
    found = [(directory(step, f"{workflow}: a step of the job {job}") or default or ROOT)
             / named.strip("\"'")
             for step in steps for named in FLAG.findall(step)]
    return list(dict.fromkeys(found))


def pairs() -> list[tuple[str, str, str, pathlib.Path]]:
    """(workflow, job, the job's own block, the manifest it names) for every job."""
    return [(path.name, job, block, manifest)
            for path in sorted(WORKFLOWS.glob("*.yml"))
            for job, block in jobs(path.read_text()).items()
            for manifest in manifests(path.name, job, block)]


def legs(block: str) -> list[str]:
    """Every toolchain a job's own block states, as the union of the ways it states them.

    A matrix states its legs as a flow sequence — on one line or across several — or as a
    block sequence, and a step may install one toolchain beside that; a job that does both
    runs what both name. A block scalar is read as the value under its key, and a `${{ … }}`
    template names nothing comparable and states no leg.
    """
    lines = block.splitlines()
    stated: list[str] = []
    for index, line in enumerate(lines):
        key = TOOLCHAIN.match(line)
        if not key:
            continue
        indent, tail = len(key.group(1)), key.group(2).split("#")[0].strip()
        if tail.startswith("["):
            while "]" not in tail and index + 1 < len(lines):
                index += 1
                tail += " " + lines[index].split("#")[0].strip()
            stated.extend(leg.strip().strip("\"'") for leg in tail.strip("[]").split(",")
                          if leg.strip())
        elif tail[:1] in ("|", ">"):
            stated.append(next((under.strip() for under in lines[index + 1:] if under.strip()), ""))
        elif tail and "${{" not in tail:  # a template names no leg
            stated.append(tail.strip("\"'"))
        else:
            for following in lines[index + 1:]:
                item = ITEM.match(following)
                if not item or len(item.group(1)) < indent:
                    break
                stated.append((item.group(2) or "").split("#")[0].strip().strip("\"'"))
    return list(dict.fromkeys(leg for leg in stated if leg))


def gated(block: str) -> list[str]:
    """The legs a job's own steps are gated on (`if: matrix.toolchain == 'stable'`)."""
    return list(dict.fromkeys(GATED.findall(block)))


def parsed(manifest: pathlib.Path) -> dict:
    """A manifest's TOML, naming a malformed one rather than raising a decode error."""
    try:
        return tomllib.loads(manifest.read_text())
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


def inherits(manifest: pathlib.Path) -> bool:
    """Whether a manifest takes its floor from its workspace (`rust-version.workspace`)."""
    stated = parsed(manifest).get("package")
    stated = stated.get("rust-version") if isinstance(stated, dict) else None
    return isinstance(stated, dict) and stated.get("workspace") is True


def workspace_of(manifest: pathlib.Path) -> pathlib.Path | None:
    """The manifest that is this crate's workspace, as cargo resolves it.

    A crate may name the root itself (`[package] workspace = ".."`), and cargo honours
    that over the directory the crate sits in, so the named root is read first — a crate
    pinning a root above a nested `[workspace]` inherits from the root it names. Failing
    that, the nearest manifest above it that declares a workspace.
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

    A crate inheriting a floor that its workspace's `[workspace.package]` does not state
    is one cargo refuses to build, so there is nothing here to hold it against — and the
    workspace root's own `[package]` floor is not the member's.
    """
    own = declared(manifest, "package")
    if own:
        return own
    if not inherits(manifest):
        return None
    workspace = workspace_of(manifest)
    return declared(workspace, "workspace.package") if workspace else None


def version(declared: str) -> str:
    """A floor as a toolchain leg is spelled: `1.85` and `1.85.0` are one version."""
    return declared if declared.count(".") == 2 else f"{declared}.0"

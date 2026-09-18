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
JOBS = re.compile(r"""^(?:"jobs"|'jobs'|jobs):[ \t]*(.*)$""")
JOB = re.compile(r"""^  (?:"([^"]*)"|'([^']*)'|([A-Za-z0-9_.\-]+)):(.*)$""")
KEY = re.compile(r"^(\s*)(?:-\s+)?([A-Za-z0-9_.\-]+):(.*)$")
ITEM = re.compile(r"^(\s*)-\s*(.*?)\s*$")
FLOW = re.compile(r"^(\s*(?:-\s+)?(?:[A-Za-z0-9_.\-]+:\s*)?)\{(.*)$")
SEQUENCE = re.compile(r"^(\s*(?:-\s+)?(?:[A-Za-z0-9_.\-]+:\s*)?)\[(.*)\]\s*$")
ANCHOR = re.compile(r"^(\s*)(?:-\s+)?(?:[A-Za-z0-9_.\-]+):[ \t]*&([A-Za-z0-9_-]+)[ \t]*(.*)$")
ALIAS = re.compile(r"^(\s*)((?:-\s+)?(?:[A-Za-z0-9_.\-]+:[ \t]*)?)\*([A-Za-z0-9_-]+)[ \t]*$")
MERGE = re.compile(r"^(\s*)((?:-\s+)?)<<:[ \t]*(.*?)[ \t]*$")
NAMED = re.compile(r"\*([A-Za-z0-9_-]+)")
WORKSPACE = "${{ github.workspace }}"
COMMAND = "run"  # the one entry a shell runs: a marker and a blank line are a shell's there alone
ENTRY = tuple[list[str], int, int, bool]  # what an entry states, the lines it spans, and whether
# it is a collection's items rather than one value


def job_name(line: str) -> tuple[str, str] | None:
    """The job a line names, two spaces under `jobs:`, with the text it states on that line.

    A key written the way YAML writes one — bare or quoted, with a trailing comment or an anchor —
    and an inline block (`build: {runs-on: …}`), which is that job's own mapping, so its text is
    the mapping rather than the lines under its key. An alias states no block of its own: the job
    it repeats is the one the anchor is written on, read where that is written.
    """
    found = JOB.fullmatch(line)
    if not found:
        return None
    name = next((name for name in found.groups()[:3] if name is not None), None)
    tail = found.group(4).strip()
    if tail.startswith(("#", "&", "*")):
        return name, ""
    return (name, tail) if not tail or tail.startswith("{") else None


def anchored_keys(text: str) -> list[tuple[str, str]]:
    """A mapping's keys, each with the text it states, read from the anchor's own lines at column 0."""
    if text.lstrip().startswith("{"):
        return [(item.partition(":")[0].strip().strip("\"'"), item) for item in flow_items(text)]
    lines, found, index = text.splitlines(), [], 0
    while index < len(lines):
        match = KEY.match(lines[index])
        if not match:
            index += 1
            continue
        end = index + 1
        while end < len(lines) and (not lines[end].strip()
                                    or len(lines[end]) - len(lines[end].lstrip()) > 0):
            end += 1
        found.append((match.group(2), "\n".join(lines[index:end])))
        index = end
    return found


def written_out(line: str, anchors: dict[str, str], where: str) -> list[str] | None:
    """One alias line as the anchored text it names, or None when the line is no alias.

    An alias names a mapping or a sequence by a name an anchor stated before it, as a parser reads
    it; a name with no anchor before it — a forward reference, another document's anchor, a typo —
    is refused by name, since what it names is written where this reader cannot see it.
    """
    found = ALIAS.match(line)
    if not found:
        return None
    indent, prefix, name = found.groups()
    if name not in anchors:
        raise AssertionError(f"{where} writes {line.strip()}, and no anchor named {name} is stated "
                             f"before it: an alias this reader cannot resolve where it is written")
    lines = anchors[name].splitlines()
    head = f"{indent}{prefix.rstrip()}".rstrip()
    if len(lines) == 1 and not lines[0].lstrip().startswith("-"):
        return [f"{head} {lines[0]}".rstrip()]
    if prefix.strip().endswith(":"):
        return [head] + [indent + "  " + text if text.strip() else text for text in lines]
    return [f"{head} {lines[0]}".rstrip()] + [indent + "  " + text if text.strip() else text
                                               for text in lines[1:]]


def merged_in(lines: list[str], index: int, anchors: dict[str, str], taken: dict[int, set[str]],
              where: str) -> list[str]:
    """A merge key as the keys it states: the anchored mapping's, minus the ones the mapping states
    itself and the ones an earlier merge in it already stated — a mapping's own keys win, and the
    first merge wins over the next, which is what both parsers read.
    """
    found = MERGE.match(lines[index])
    dash = bool(found.group(2))
    depth = len(found.group(1)) + len(found.group(2))
    names = NAMED.findall(found.group(3))
    if not names:
        raise AssertionError(f"{where} writes {lines[index].strip()}, a merge this reader does not "
                             f"read: it states no anchor to merge")
    start = index
    while start > 0 and (not lines[start - 1].strip()
                         or len(lines[start - 1]) - len(lines[start - 1].lstrip()) >= depth):
        start -= 1
    end = index + 1
    while end < len(lines) and (not lines[end].strip()
                                or len(lines[end]) - len(lines[end].lstrip()) >= depth):
        end += 1
    own = {KEY.match(text).group(2) for text in lines[start:end]
           if text.startswith(" " * depth) and not text.startswith(" " * (depth + 1))
           and KEY.match(text)}
    added = taken.setdefault(start, set())
    out: list[str] = []
    for name in names:
        if name not in anchors:
            raise AssertionError(f"{where} merges {name}, and no anchor named {name} is stated "
                                 f"before it: a merge this reader cannot resolve")
        for key, body in anchored_keys(anchors[name]):
            if key in own or key in added:
                continue
            added.add(key)
            body_lines = [text for text in body.splitlines() if text.strip()]
            if not body_lines:
                continue
            if dash and not out:  # the item's first key stays on the dash
                out.append(" " * depth + "- " + body_lines[0])
                out.extend(" " * (depth + 2) + text for text in body_lines[1:])
                continue
            lead = " " * (depth + 2) if dash else " " * depth
            out.extend(lead + text for text in body_lines)
    return out


def resolved(workflow: str, where: str) -> str:
    """The document with its anchors, aliases and merge keys written out.

    A job or a step stated as an alias is a job or a step, and a merge states the keys it merges:
    measured, a job whose block is an alias read as a job with no block at all, and a job that
    inherited its proving step through `<<:` was not counted as proving the workspace. An anchor's
    own value stays where it is written, and an alias is written out only where an anchor before it
    states the name — a forward reference, another document's anchor or a typo is refused by name,
    as both parsers refuse it. A stream of more than one document is refused too.
    """
    stated = workflow.splitlines()
    for number, line in enumerate(stated):
        if line.startswith("---") and any(text.strip() and not text.lstrip().startswith(("#", "---"))
                                           for text in stated[:number]):
            raise AssertionError(f"{where} writes more than one document, and the rule reads one")
    text = workflow
    for _ in range(6):  # an alias can name a mapping that itself states a merge
        anchors: dict[str, str] = {}
        taken: dict[int, set[str]] = {}
        lines, out, index, changed = text.splitlines(), [], 0, False
        while index < len(lines):
            line = lines[index]
            found = ANCHOR.match(line)
            if found:
                changed = True
                name = found.group(2)
                rest, depth = found.group(3).strip(), len(found.group(1))
                block: list[str] = []
                if rest:  # the anchor's own value is on its line
                    anchors[name], index = rest, index + 1
                else:  # the anchor's own value is the block under its key
                    index += 1
                    while index < len(lines) and (not lines[index].strip() or len(lines[index])
                                                  - len(lines[index].lstrip()) > depth):
                        block.append(lines[index])
                        index += 1
                    while block and not block[-1].strip():
                        block.pop()
                    stripped = min((len(text) - len(text.lstrip()) for text in block
                                    if text.strip()), default=0)
                    anchors[name] = "\n".join(text[stripped:] if text.strip() else ""
                                              for text in block)
                out.append(line.replace(f"&{name}", "", 1).rstrip())
                out.extend(block)  # an anchor's own value stays where it is written
                continue
            written = written_out(line, anchors, where)
            if written is not None:
                changed = True
                out.extend(written)
            elif MERGE.match(line):
                changed = True
                out.extend(merged_in(lines, index, anchors, taken, where))
            else:
                out.append(line)
            index += 1
        once = "\n".join(out)
        if not changed or once == text:
            return text
        text = once
    raise AssertionError(f"{where} nests anchors deeper than this reader follows")


def jobs(workflow: str) -> dict[str, str]:
    """Each job of a workflow, with the text of its own block: a job key is two spaces under
    `jobs:`, and its block ends where a line starts at column zero.

    A key this does not read is a job no rule checks, silently: measured against PyYAML, a
    quoted key, a dotted key, a key with a trailing comment and a key carrying an anchor were each
    read as no job at all — as were a `jobs:` mapping written on the key's own line
    (`jobs: {build: …}`) and a job whose mapping is written on its key's line. The whole mapping on
    one line is read here, and the jobs in it are named by their own keys.
    """
    lines = resolved(workflow, "a workflow").splitlines()
    start = next((index for index, line in enumerate(lines) if JOBS.match(line)), None)
    if start is None:
        return {}
    head = JOBS.match(lines[start]).group(1).strip()
    if head and not head.startswith("{"):
        head = head.split("#")[0].strip()  # a comment after the key states no job
    if head:
        if not head.endswith("}"):
            raise AssertionError(f"a workflow writes jobs: {head}, a mapping this reader does not "
                                 f"read across lines")
        return {name.strip().strip("\"'") or name: stated.strip()
                for name, _, stated in (item.partition(":") for item in flow_items(head))}
    body = [line for line in lines[start + 1:] if not JOBS.match(line)]
    stop = next((n for n, text in enumerate(body) if text and not text.startswith(" ")),
                len(body))
    found = []
    for index, line in enumerate(body):
        if index >= stop:
            break
        named = job_name(line)
        if named is not None:
            found.append((index, *named))
    return {name: inline or "".join(line + "\n" for line in body[key + 1:end])
            for (key, name, inline), (end, _, _) in zip(found, found[1:] + [(stop, "", "")])}


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
    """The block with every flow mapping, and every list of mappings, put into the block form it
    means: a mapping written inline (`matrix: {toolchain: [a, b]}`, `- {run: …}`) and a list of
    mappings (`steps: [{run: …}]`), so a key written either way is found where the job wrote it. A
    list of values is a value, and stays one — `toolchain: [a, b]` is the two legs it states.
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

    def splice_sequence(match: re.Match[str]) -> str:
        prefix, body = match.group(1), match.group(2)
        items = flow_items("[" + body + "]")
        if not items or not all(item.startswith("{") for item in items):
            return match.group(0)  # a list of values is a value, not a list of mappings
        indent = " " * (len(prefix) - len(prefix.lstrip()))
        return prefix.rstrip() + "".join(
            f"\n{indent}- {keys[0]}" + "".join(f"\n{indent}  {key}" for key in keys[1:])
            for keys in (flow_items(item) for item in items))

    for _ in range(3):  # a splice writes the line the other one unfolds
        once = "\n".join(SEQUENCE.sub(splice_sequence, FLOW.sub(splice, line))
                         for line in block.splitlines())
        if once == block:
            break
        block = once
    return block


def entries(block: str, name: str, where: str) -> list[ENTRY]:
    """Every entry a block states under `name`: what it says, and the lines it spans.

    One pass reads the forms a value is written in: the text after the key, a flow collection on
    one line or across several, a block scalar, and a block sequence, whose items are one entry
    each — so removing the step lines leaves the job's own text, and each entry says which it is:
    a collection's items, or one value.

    `run` is the one entry a shell runs, and it alone is read the way a shell reads it: a
    trailing `\\` is that shell's continuation, and a blank line ends the command. Every other
    value is read the way a parser states it: the `\\` is the literal character YAML says it is,
    a continuation line must be deeper than its key, and a blank line is a paragraph break the
    value goes on past. A line stating a key of its own, or a comment line, ends any entry.
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
            found.append(([item.strip("\"'") for item in flow_items(tail)], first, index - 1,
                          True))
        elif tail[:1] in ("|", ">"):
            words: list[str] = []
            while index < len(lines) and (not lines[index].strip() or len(lines[index])
                                          - len(lines[index].lstrip()) > indent):
                words, index = words + lines[index].split(), index + 1
            found.append((words, first, index - 1, False))
        elif tail:
            said = [tail]  # a plain scalar runs onto every line deeper than its key
            while index < len(lines) and not KEY.match(lines[index]):
                if not lines[index].strip():
                    if name == COMMAND:
                        break  # a blank line ends the command a shell would run
                    index += 1
                    continue  # a blank line inside a value is the paragraph a parser keeps
                marker = name == COMMAND and said[-1].rstrip().endswith("\\")
                if not (marker or len(lines[index]) - len(lines[index].lstrip()) > indent):
                    break  # neither a shell continuation nor a line the scalar runs onto
                if marker:
                    # the marker, and the space before it, are not part of what a shell runs
                    said[-1] = said[-1].rstrip()[:-1].rstrip()
                stated = lines[index].split("#")[0].strip()
                if not stated:
                    break  # a comment line states no word, and ends the entry
                said.append(stated)
                index += 1
            found.append(([word.strip("\"'") for word in said], first, index - 1, False))
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
                found.append(([stated] if stated else [], index, end - 1, True))
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
    inside = {line for _, first, last, _ in spans for line in range(first, last + 1)}
    default = directory("".join(line for index, line in enumerate(lines)
                                if index not in inside), where)
    steps = ["".join(lines[first:last + 1]) for _, first, last, _ in spans]
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
    beside that, and a `${{ … }}` template states no leg. An entry that is one value is one leg:
    a toolchain is not a command, so nothing here is a shell's to join, and a marker inside it
    is the literal character both parsers read.
    """
    stated = [leg for values, _, _, items in entries(block, "toolchain", where)
              for leg in (values if items else [" ".join(values)])]
    return list(dict.fromkeys(leg.strip("\"'") for leg in stated if leg and "${{" not in leg))


def gated(block: str, where: str = "a job") -> list[str]:
    """The legs a job's own steps are gated on (`if: matrix.toolchain == 'stable'`).

    Read through the one reader rather than off the block's own text: a condition written over
    two lines is the one condition a parser reads, and reading the text alone saw the
    single-line spelling and missed the folded one — silently weakening this rule's own
    assertion, since a gate it does not see is a leg it never checks.
    """
    lines = unfolded(block, where).splitlines()
    spans = [(first, last) for _, first, last, _ in entries(block, "if", where)]
    return list(dict.fromkeys(
        leg for first, last in spans
        for leg in re.findall(r"matrix\.toolchain == '([^']+)'",
                              " ".join(" ".join(lines[index].split())
                                       for index in range(first, last + 1)))))


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
                   for values, _, _, _ in entries(block, "run", "a job")
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

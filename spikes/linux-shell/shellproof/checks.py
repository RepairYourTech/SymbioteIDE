"""What a result may not claim, what the path would never read, and what a value could be.

Every refusal this driver makes before it publishes lives here: the attestation an
obligation needs from the fixture's own log, the revision a run records having to be
the tree it ran in, the platform a result records having to be one the session it
starts can be, the measurements a contract predeclares being observed or named with
their reason, and the options, fingerprints and terms an invocation can name without
anything reading or bearing them. Each answers with a named refusal rather than a
traceback, and each is asked before anything in the tree is written.
"""
from shellproof.contract import platform_key
from shellproof.records import OPERATING_SYSTEMS, sha256_of
from shellproof.session import SESSION_DISPLAY

# The contract obligations this fixture's own log can attest, the marker the
# fixture emits for each, and how many of them one run has to show. An
# attestation is read from the app's log and never from the command line, so an
# obligation whose marker is missing stays unattested: four concurrent agent
# streams are absent below because this fixture starts four *synthetic* streams,
# not agent streams, and no marker of its own says otherwise.
ATTESTED_BY = (
    ('#38: three active terminal tabs with bounded scrollback in real PTYs',
     'PROOF_PTY_START', 3),
    ("#38: a live integrated Preview of the run's own output",
     'PROOF_READY preview_origin=', 1),
    ('#38: Preview origins, localhost included, that inherit neither workbench nor Host authority '
     'and reach the app only through validated bridge operations',
     'PROOF_PREVIEW_REPORT', 2),
)


def attested(log_text):
    """The contract obligations this log attests, and nothing it does not show."""
    return [obligation for obligation, marker, least in ATTESTED_BY
            if log_text.count(marker) >= least]


def uncommitted(text):
    """Tracked files that differ from HEAD, from ``git status --porcelain``.

    Untracked files are the run's own new evidence and are left alone; a modified
    tracked file means the revision a run would record is not the tree it ran, and
    the run refuses to publish rather than record a revision it cannot stand on.
    """
    return [line for line in text.splitlines() if line[:2] not in ('??', '')]


def revision_problems(commit, head, status):
    """Why a run may not record this revision, if it may not.

    Nothing in the tree can re-derive a build's revision from a build log, so the
    revision a run records has to be the tree it ran in — clean, at HEAD — and the
    recipe it cites has to have logged the same revision. Anything else is a claim
    about a tree nobody can find.
    """
    problems = []
    changed = uncommitted(status)
    if changed:
        problems.append('the tree differs from HEAD in:\n    ' + '\n    '.join(changed))
    if commit and head and commit != head:
        problems.append(f'the run would record revision {commit} and this tree is {head}')
    return problems


def platform_problems(platform, session, system):
    """Why a result may not record this platform, if it may not.

    The platform a result records is a claim about a machine, and until this rule the
    only thing that bore it was the invocation: a kwin Wayland session on this Linux
    host could publish a run recording ``macOS 14 arm64``, and the untested list the
    ledger reads was derived from that word. What the driver can hold the claim to is
    the session it starts — the display server that session provides, and the
    operating system this process runs on — so a platform naming another display
    server, or another operating system, is refused rather than recorded.

    Which of the platforms a session could be is still the operator's judgement, and
    the record says so: a label naming neither of those facts cannot be contradicted
    from here, and ``session_record`` carries both beside it so a reader compares them
    with what the run's own machine showed rather than trusting the label.
    """
    problems = []
    provided = SESSION_DISPLAY[session]
    for display in SESSION_DISPLAY.values():
        if platform_key(display) in platform_key(platform) and display != provided:
            problems.append(f'{platform!r} names {display}, and this run starts a {session} session '
                            f'that provides {provided}: a result would record a platform the '
                            'session it started cannot be')
    for reported, name in OPERATING_SYSTEMS.items():
        if platform_key(reported) in platform_key(platform) and name != system:
            problems.append(f'{platform!r} names {name}, and this session runs on {system}: a run '
                            'measured here cannot be one of those')
    return problems


def unconsumed_options(args, declared):
    """Options the path this invocation selected never reads, each named.

    Three defects in this driver were one class: an invocation that exited successfully
    while silently dropping what it asked for — evidence copied before a subscript
    crashed, a dossier published and then refused by the ledger, and an X11 run that
    satisfied `--results` and wrote no dossier at all. One the selected path never reads
    is refused here by name, before the git gate, before the artifacts directory exists
    and before any session starts, rather than being accepted and dropped.

    Where an option is read is not decided here. Each option declares its own row of the
    option→path matrix beside itself in the parser — the paths that read it, how an
    invocation is seen to have named it, what it needs to be borne, and the sentence to
    refuse it with — and this reads those declarations, so an option added to the command
    line is refused by its own declaration or not at all, and the label `--help` prints
    for it is drawn from the same word.

    It is asked of what the invocation named, before `named_defaults` fills the terms it
    left out: an option nobody passed is not an option that was dropped. `--contracts`,
    which names the document itself, keeps its default, and the terms it names are judged
    rather than checked because a default cannot be told from an option an invocation
    passed.
    """
    def named(entry):
        value = getattr(args, entry['dest'])
        return value is not None if entry['named'] == 'set' else bool(value)

    rows = {entry['option']: entry for entry in declared}
    problems = []
    for entry in declared:
        if not named(entry):
            continue
        option = entry['option']
        if entry['needs'] and not named(rows[entry['needs']]):
            problems.append(f"{option} {entry['refusal']}")
        reads = entry['reads']
        if reads == 'none':
            problems.append(f"{option} {entry['refusal']}")
        elif reads == 'result' and not args.results:
            problems.append(f'{option} says what a result records, and this invocation asks for '
                            'no result: add --results or drop it')
        elif reads == 'wayland' and args.session != 'wayland':
            problems.append(f'{option} belongs to the Wayland entry point: the X11 entry point '
                            'prints one measured lifetime and records no build, revision or '
                            'contract, so nothing here would read it')
        elif reads == 'record' and not (args.results or args.build_log):
            problems.append(f'{option} says which revision a run records, and this invocation '
                            'records neither a result nor a build log: add --results or --build-log')
    return problems


def fingerprint_problems(fingerprint, document):
    """Why the value an invocation supplies cannot be the contract's fingerprint.

    The ledger's fingerprint is the crate's ``fingerprint(contract)``: the SHA-256 of
    the contract's own canonical JSON, so a result is tied to the thresholds it was
    measured against and adding another contract to the document does not move it.
    This driver does not derive that value — deriving it would be a second
    implementation of the crate's convention — so what it can honestly decide is that
    the value it was handed is a fingerprint at all, and that it is not the SHA-256 of
    the document file: a quantity of the same length that an operator reaching for
    "the contract's hash" lands on, and that would record a result the ledger reports
    as measured against a contract that is not the one committed now.

    Which contract a fingerprint belongs to is not decidable here, because nothing in
    this driver knows any contract's fingerprint; that is the ledger's own identity
    check, which re-hashes the contract the run named and refuses a result measured
    under another one.
    """
    problems = []
    if not isinstance(fingerprint, str) or len(fingerprint) != 64 \
            or any(character not in '0123456789abcdef' for character in fingerprint):
        problems.append(f'{fingerprint!r} is not a fingerprint: the ledger holds the crate\'s '
                        'lowercase-hex SHA-256 of the contract\'s own canonical JSON')
    if document.exists() and sha256_of(document) == fingerprint:
        problems.append(f'{fingerprint} is the SHA-256 of {document}, the document file, and not '
                        'the fingerprint of the contract inside it')
    return problems


def untested_platforms(contract, runs):
    """The contract's platforms this run set holds no run for.

    Read from the runs the dossier actually carries, never from the platform this
    invocation was pointed at: a platform with recorded runs is measured, and one
    another invocation already recorded is not declared untested by this one.
    """
    measured = {platform_key(run['platform']) for run in runs}
    return [platform for platform in contract.applicable_platforms
            if platform_key(platform) not in measured]


def unknown_of(args):
    """The measurements this instrument cannot see, and why, in declared order."""
    unknown = []
    for item in args.unobservable:
        name, _, reason = item.partition('=')
        unknown.append((name.strip(), reason.strip() or 'no reason recorded'))
    return unknown


def result_problems(args, contract, runs):
    """Everything that would make this result claim more than the run shows.

    Checked before anything is published: a platform the contract does not apply
    to, a stop condition it does not declare, a measurement it predeclares that is
    neither observed nor named unknown, an unknown or an observation the contract
    never declared, and an obligation answered here with no marker in the run's
    own log.
    """
    problems = []
    if not contract.applies_to(args.platform):
        problems.append(f'{args.platform!r} is not a platform this contract applies to: '
                        + ', '.join(contract.applicable_platforms))
    if not contract.declares(args.stop_condition):
        problems.append(f'{args.stop_condition!r} is not a stop condition this contract declares')
    predeclared = list(contract.measurement_names)
    observed = {figure['measurement'] for row in runs for figure in row['figures']}
    unknown = dict(unknown_of(args))
    for name in sorted(set(unknown) - set(predeclared)):
        problems.append(f'{name} is named unobservable and the contract does not predeclare it')
    for name in sorted(observed - set(predeclared)):
        problems.append(f'{name} is observed and the contract does not predeclare it')
    for name in predeclared:
        if name not in observed and name not in unknown:
            problems.append(f'{name} is predeclared and neither observed nor named unobservable, '
                            'so this result would leave it silent')
    for row in runs:
        if not row['exercised']:
            problems.append('a run attests no obligation: its log carries no marker this fixture '
                            'emits for one')
    return problems

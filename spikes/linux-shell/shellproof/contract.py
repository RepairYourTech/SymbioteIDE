"""The frame a run stands in, and the terms it answers.

``ROOT`` is this driver's own directory and ``REPO`` is the tree it publishes into.
``CONTRACTS_PATH`` is where the committed contract document lives: the one contract
term this driver states rather than asks for, because the crate names the same path in
Rust (``spike::CONTRACTS_PATH``) and no committed data maps a contract id to the
document that holds it. Every other term is read from the document itself.

``Contract`` is the driver's only view of that document's data — the platforms a
contract applies to, the stop conditions it declares and the measurements it
predeclares — so the driver cannot answer with a copy of its own that drifts from the
document the crate, which reads the same file, refuses by. A contract declares a
ceiling beside each measurement and this module carries none of them: the driver
records what it observed, and the crate is what judges an observation against its
maximum.
"""
from dataclasses import dataclass
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
REPO = ROOT.parents[1]
CONTRACTS_PATH = 'docs/architecture/spike-contracts.json'


def read_document(path, what):
    """A JSON document this run depends on, or a refusal naming what is missing.

    The contract document and the dossier a run merges into are both read here, so a
    file that is absent, unreadable or not JSON is refused by name rather than
    raised as a traceback from whichever line happened to touch it first: a run that
    cannot establish the terms it answers has nothing to record, and saying which
    document is short is the whole of what a reader can do with it.
    """
    try:
        return json.loads(Path(path).read_text())
    except FileNotFoundError:
        raise SystemExit(f'{what} {path} does not exist')
    except (OSError, ValueError) as error:
        raise SystemExit(f'{what} {path} cannot be read as JSON: {error}')


def platform_key(value):
    """One platform, as a name is compared: trimmed and case-insensitive.

    The contract, the artifact and the ledger all name platforms as strings and the
    ledger compares them this way, so a run that spelled its own platform
    differently from the contract would otherwise be counted as a second platform.
    """
    return value.strip().lower()


@dataclass(frozen=True)
class Contract:
    """The terms a run is judged against, read from the document that holds them.

    The platforms a contract applies to, the stop conditions it declares and the
    measurements it predeclares are the document's data, and this is the driver's
    only view of them: every judgement below asks this object, so the driver cannot
    answer with a copy of its own that drifts from the document the crate — which
    reads the same file — refuses by. A contract declares a ceiling beside each
    measurement and the driver carries none of them: it records what it observed,
    and the crate is what judges an observation against its maximum.
    """

    id: str
    applicable_platforms: tuple[str, ...]
    stop_conditions: tuple[str, ...]
    # The predeclared names, in the order the document declares them, because that
    # order is the order a refusal about a silent measurement is reported in.
    measurement_names: tuple[str, ...]

    def applies_to(self, platform):
        """Whether this contract is about a platform, spelled as the document spells it."""
        return platform in self.applicable_platforms

    def declares(self, stop_condition):
        """Whether this contract declares a stop condition."""
        return stop_condition in self.stop_conditions

    @classmethod
    def read(cls, path, identity=None):
        """The contract a run answers, from the document that holds it.

        ``identity`` names the contract; with none, the document's own first one is
        meant, which is what an invocation that names no contract is measured
        against. A document that cannot be read, that names no contract at all, or
        whose contract does not carry the terms a run is judged by is refused by
        name — as every other failure in this driver is — because a term read out of
        whatever happens to be present would judge a run by something the document
        never declared.
        """
        path = Path(path)
        document = read_document(path, 'the contract document')
        entries = document.get('contracts') or []
        if not entries:
            raise SystemExit(f'{path} names no contract, so a run has no terms to answer')
        for entry in entries:
            if identity is not None and entry.get('id') != identity:
                continue
            absent = [what for term, what in (('applicable_platforms', 'the platforms it applies to'),
                                              ('stop_conditions', 'the stop conditions it declares'),
                                              ('measurements', 'the measurements it predeclares'))
                      if not entry.get(term)]
            if absent or not all(item.get('name') for item in entry.get('measurements') or []):
                raise SystemExit(f'the contract {entry.get("id")!r} in {path} does not carry '
                                 + ' or '.join(absent or ['the names of the measurements it '
                                                          'predeclares']))
            return cls(entry['id'], tuple(entry['applicable_platforms']),
                       tuple(entry['stop_conditions']),
                       tuple(measurement['name'] for measurement in entry['measurements']))
        raise SystemExit(f'no committed contract named {identity!r} in {path}')

    @classmethod
    def named_by_the_document(cls, path=CONTRACTS_PATH):
        """The contract an invocation that names none gets: the document's own.

        The command line still overrides both this and the platform it supplies, and
        either is refused below if the contract does not declare it, so a default
        cannot put a term into a result that the document never held.
        """
        return cls.read(REPO / path)

    def default_platform(self):
        """The platform a run takes when none is named: the first one it applies to."""
        return self.applicable_platforms[0]

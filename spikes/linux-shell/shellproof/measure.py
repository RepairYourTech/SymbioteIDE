"""The sequence both entry points share: run it, sample it, clean up after it, read its log.

One place, so a change to how a run is observed or cleaned up lands once, and the two
entry points differ only in what they pass in and what they publish.
"""
from shellproof.observation import cleanup, observe_app, processes, survivors_of


def measure(session, artifacts, binary, log_name, seconds, cancel_after, trace):
    """One app lifetime: run it, sample it, clean up after it, read its own log.

    The sequence both entry points share, in one place, so a change to how a run is
    observed or cleaned up lands once. ``survivors_before_cleanup`` is taken before
    this function's own cleanup, because that is the figure the X11 record
    publishes; ``cancellation`` is the cleanup record itself, and ``text`` is the
    log the attestation is read from.
    """
    log_path = artifacts / log_name
    code, elapsed, samples, seen, marks = observe_app(session.env, log_path, binary, seconds,
                                                     cancel_after, trace)
    survivors = survivors_of(seen, processes())
    cleaned = cleanup(seen, [])
    return {'log': log_path, 'log_name': log_name, 'code': code, 'elapsed': elapsed,
            'samples': samples, 'marks': marks, 'survivors_before_cleanup': survivors,
            'cancellation': cleaned, 'text': log_path.read_text()}

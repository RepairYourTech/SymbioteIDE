"""The shell proof driver's concerns, one per module.

The driver is a package so that each concern it carries can be read and changed
alone. ``run-proof.py`` beside it is the executable entry point, where the gate and
the two entry points live, and it re-exports every name the fixture's own suite reads.
This file holds no code of its own.

* ``contract`` — the terms a run is judged against, read from the committed document;
* ``session`` — where a run happens, and the one thing that closes it;
* ``observation`` — the host and the process tree as they ran;
* ``measure`` — the sequence both entry points share;
* ``records`` — what a run records about itself, and the shape a session record declares;
* ``checks`` — what a result may not claim, what the path would never read, and what a
  supplied fingerprint can honestly be held to;
* ``publication`` — how a run's entries and the evidence they cite reach the tree.
"""

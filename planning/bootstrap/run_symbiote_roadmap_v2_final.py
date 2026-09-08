#!/usr/bin/env python3
"""Final validated runner for the Symbiote v2 roadmap importer."""
raise SystemExit("RETIRED_STALE_IMPORTER: historical payload predates accepted amendments; see #470. No remote writes permitted.")
from pathlib import Path

source_path = Path(__file__).with_name("import_symbiote_roadmap_v2.py")
source = source_path.read_text(encoding="utf-8")
replacements = {
    'add("B05", "Implement Project intelligence lifecycle and resource policies", "E01", 2,':
        'add("B05", "Implement Project intelligence lifecycle and resource policies", "E01", 3,',
    'add("B06", "Implement shared, isolated, and external SurrealDB Project profiles", "E01", 2,':
        'add("B06", "Implement shared, isolated, and external SurrealDB Project profiles", "E01", 3,',
    '], deps=["B04", "G02", "N08"])':
        '], deps=["B04", "N08"])',
    'add("B07", "Implement Project import, restore, archive, clone, and terminology preferences", "E01", 3,':
        'add("B07", "Implement Project import, restore, archive, clone, and terminology preferences", "E01", 4,',
    '], deps=["B04", "N10", "H03"])':
        '], deps=["B04", "H03"])',
    '], deps=["G01", "G02", "N11"])':
        '], deps=["G01", "G02"])',
    '], deps=["G02", "G04", "H09"])':
        '], deps=["G02", "G04"])',
    '], deps=["F01", "D08", "I06"])':
        '], deps=["F01", "D08", "I01"])',
    '], deps=["D05", "D07", "C02", "M02", "I01"])':
        '], deps=["D05", "D07", "C02", "M02"])',
    '], deps=["G01", "F06", "M04"])':
        '], deps=["G01", "F03", "M04"])',
    '], deps=["J08", "F06"])':
        '], deps=["J08", "F03"])',
    '], deps=["K03", "Q09", "Q10", "Q11"])':
        '], deps=["K03"])',
    '], deps=["K03", "Q08"])':
        '], deps=["K03", "Q01"])',
    '], deps=["J14", "K06", "D07"])':
        '], deps=["J03", "J08", "K06", "D07"])',
    '], deps=["M02", "I11", "H03"])':
        '], deps=["M02", "H03"])',
    '], deps=["M06", "K10", "D03"])':
        '], deps=["M06", "D03"])',
    'add("M10", "Implement worktree/branch/PR cleanup, salvage, and interrupted-operation recovery", "E11", 5,':
        'add("M10", "Implement worktree/branch/PR cleanup, salvage, and interrupted-operation recovery", "E11", 6,',
    '], deps=["Q01", "N09", "C01"])':
        '], deps=["Q01", "C01"])',
    '], deps=["N05", "N06", "N07", "N08", "D04"])':
        '], deps=["N05", "N06", "N07", "N08", "D04", "Q02"])',
    '], deps=["H06", "F07", "Q01"])':
        '], deps=["H06", "F07", "Q01", "K06"])',
    'add("P07", "Implement mobile artifact, diff, PR, CI, review, and merge workflows", "E13", 5,':
        'add("P07", "Implement mobile artifact, diff, PR, CI, review, and merge workflows", "E13", 6,',
    '], deps=["D01", "C03", "N01", "H06"])':
        '], deps=["D01", "C03", "H06"])',
    '], deps=["I02", "G14"])':
        '], deps=["I02", "G13", "G05", "G07"])',
    '], deps=["G06", "G07", "G08", "G09", "G10", "G14"])':
        '], deps=["G06", "G07", "G08", "G09", "G14"])',
    '], deps=["J08", "G11", "I07"])':
        '], deps=["J08", "G11"])',
    '], deps=["L02", "D10", "F07", "I12", "K08"])':
        '], deps=["L02", "D10", "F07", "K08"])',
}
for old, new in replacements.items():
    count = source.count(old)
    if count != 1:
        raise SystemExit(f"Expected exactly one replacement match, found {count}: {old}")
    source = source.replace(old, new)

exec(compile(source, str(source_path), "exec"), {
    "__name__": "__main__",
    "__file__": str(source_path),
})

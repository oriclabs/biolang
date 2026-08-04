#!/usr/bin/env python3
"""Run the one-liner correctness cases across BioLang, Python and R.

The task-per-file layout in the directory above is right for things that need
real data and twenty lines of setup. It is the wrong shape for "does mean()
agree with statistics.mean()" — three files per builtin would mean six hundred
files to cover the two hundred or so builtins that have equivalents at all.

So the cases live in cases.tsv, one row per behaviour, and this generates a
single script per language. Each case is wrapped so one failure reports itself
without taking the rest of the batch down.

Usage:
    python run_oneliners.py [--bl PATH] [--python PATH] [--rscript PATH]
                            [--json OUT] [--verbose]
"""
from __future__ import annotations

import argparse
import json
import os
import shutil
import subprocess
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
CASES = os.path.join(HERE, "cases.tsv")

PY_PREAMBLE = """import json, math, statistics
from Bio.Seq import Seq
from Bio.SeqUtils import gc_fraction
"""

R_PREAMBLE = """suppressPackageStartupMessages(library(Biostrings))
suppressPackageStartupMessages(library(jsonlite))
"""


def load_cases(path: str) -> list[dict]:
    rows: list[dict] = []
    with open(path, encoding="utf-8") as handle:
        header = None
        for line in handle:
            line = line.rstrip("\n")
            if not line.strip() or line.lstrip().startswith("#"):
                continue
            parts = line.split("\t")
            if header is None:
                header = parts
                continue
            row = dict(zip(header, parts))
            if row.get("name") and row.get("biolang"):
                rows.append(row)
    return rows


def gen_biolang(cases: list[dict]) -> str:
    out = []
    for c in cases:
        # try/catch per case: an unknown builtin or a type error reports itself
        # as one failing row instead of aborting the script.
        # json_stringify pretty-prints, so a list or record spans several lines.
        # Each value is delimited rather than put on one line with its name.
        out.append("try {")
        out.append(f'  println("<<{c["name"]}")')
        out.append(f"  println(json_stringify({c['biolang']}))")
        out.append('  println(">>")')
        out.append("} catch e {")
        out.append(f'  println("<<{c["name"]}")')
        out.append('  println("\\"__ERROR__\\"")')
        out.append('  println(">>")')
        out.append("}")
    return "\n".join(out) + "\n"


def gen_python(cases: list[dict]) -> str:
    out = [PY_PREAMBLE]
    for c in cases:
        out.append("try:")
        out.append(f"    print('<<' + {json.dumps(c['name'])}); print(json.dumps({c['python']})); print('>>')")
        out.append("except Exception:")
        out.append(f"    print('<<' + {json.dumps(c['name'])}); print('\"__ERROR__\"'); print('>>')")
    return "\n".join(out) + "\n"


def gen_r(cases: list[dict]) -> str:
    out = [R_PREAMBLE]
    for c in cases:
        expr = (c.get("r") or "").strip()
        if not expr:
            continue
        # auto_unbox keeps scalars as scalars rather than one-element arrays,
        # which is how the other two languages emit them.
        out.append("tryCatch({")
        out.append(f'  cat("<<{c["name"]}\\n"); cat(toJSON({expr}, auto_unbox = TRUE, digits = 12)); cat("\\n>>\\n")')
        out.append("}, error = function(e) {")
        out.append(f'  cat("<<{c["name"]}\\n\\"__ERROR__\\"\\n>>\\n")')
        out.append("})")
    return "\n".join(out) + "\n"


def run(cmd: list[str], cwd: str) -> dict[str, object]:
    proc = subprocess.run(cmd, capture_output=True, text=True, cwd=cwd)
    values: dict[str, object] = {}
    current: str | None = None
    buf: list[str] = []
    for line in proc.stdout.splitlines():
        stripped = line.strip()
        if stripped.startswith("<<"):
            current, buf = stripped[2:].strip(), []
            continue
        if stripped == ">>" and current is not None:
            raw = "\n".join(buf).strip()
            try:
                values[current] = json.loads(raw)
            except json.JSONDecodeError:
                values[current] = f"__UNPARSEABLE__ {raw[:40]}"
            current, buf = None, []
            continue
        if current is not None:
            buf.append(line)
    return {"values": values, "stderr": proc.stderr[-400:], "code": proc.returncode}


def equal(a, b, tol: float = 1e-9) -> bool:
    if isinstance(a, bool) or isinstance(b, bool):
        return a == b
    if isinstance(a, (int, float)) and isinstance(b, (int, float)):
        return abs(float(a) - float(b)) <= tol
    if isinstance(a, list) and isinstance(b, list):
        return len(a) == len(b) and all(equal(x, y, tol) for x, y in zip(a, b))
    return a == b


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--bl", default=os.environ.get("BIOLANG_CLI", "bl"))
    ap.add_argument("--python", default=sys.executable)
    ap.add_argument("--rscript", default="Rscript")
    ap.add_argument("--json", dest="json_out")
    ap.add_argument("--md", dest="md_out")
    ap.add_argument("--verbose", action="store_true")
    args = ap.parse_args()

    cases = load_cases(CASES)
    has_r = shutil.which(args.rscript) is not None

    tmp = tempfile.mkdtemp(prefix="bl-oneliners-")
    bl_file = os.path.join(tmp, "cases.bl")
    py_file = os.path.join(tmp, "cases.py")
    r_file = os.path.join(tmp, "cases.R")
    with open(bl_file, "w", encoding="utf-8", newline="\n") as h:
        h.write(gen_biolang(cases))
    with open(py_file, "w", encoding="utf-8", newline="\n") as h:
        h.write(gen_python(cases))
    with open(r_file, "w", encoding="utf-8", newline="\n") as h:
        h.write(gen_r(cases))

    bl = run([args.bl, "run", bl_file], tmp)
    py = run([args.python, py_file], tmp)
    r = run([args.rscript, r_file], tmp) if has_r else {"values": {}, "stderr": "", "code": 0}

    print(f"=== One-liner correctness ({len(cases)} cases) ===\n")
    results = []
    counts = {"py_pass": 0, "py_fail": 0, "r_pass": 0, "r_fail": 0, "r_skip": 0}

    for c in cases:
        name = c["name"]
        b = bl["values"].get(name, "__MISSING__")
        p = py["values"].get(name, "__MISSING__")
        # A case marked expect=differ documents a known convention difference.
        # It is not a failure — but it *is* a failure if it starts agreeing,
        # because then the note is stale and should be removed.
        expect_differ = (c.get("expect") or "").strip() == "differ"
        agrees = equal(b, p) and b != "__ERROR__"
        if expect_differ:
            verdict_py = "DIFFER" if not agrees else "FAIL"
        else:
            verdict_py = "PASS" if agrees else "FAIL"
        counts["py_pass" if verdict_py in ("PASS", "DIFFER") else "py_fail"] += 1

        rv = (c.get("r") or "").strip()
        if not rv or not has_r:
            verdict_r = "SKIP"
            counts["r_skip"] += 1
        else:
            rval = r["values"].get(name, "__MISSING__")
            r_agrees = equal(b, rval) and b != "__ERROR__"
            if expect_differ:
                verdict_r = "DIFFER" if not r_agrees else "FAIL"
            else:
                verdict_r = "PASS" if r_agrees else "FAIL"
            counts["r_pass" if verdict_r in ("PASS", "DIFFER") else "r_fail"] += 1

        results.append({"name": name, "category": c.get("category", ""),
                        "biolang": b, "python": p,
                        "r": r["values"].get(name) if rv and has_r else None,
                        "vs_python": verdict_py, "vs_r": verdict_r})

        ok = verdict_py in ("PASS", "DIFFER") and verdict_r in ("PASS", "SKIP", "DIFFER")
        flag = "" if ok else "  <-"
        if args.verbose or flag:
            print(f"  {name:30} py:{verdict_py:4} r:{verdict_r:4}{flag}")
            if flag:
                print(f"      bl={b!r}")
                print(f"      py={p!r}")
                if rv and has_r:
                    print(f"      r ={r['values'].get(name)!r}")

    print()
    print(f"  vs Python: {counts['py_pass']} passed, {counts['py_fail']} failed")
    if has_r:
        print(f"  vs R:      {counts['r_pass']} passed, {counts['r_fail']} failed, {counts['r_skip']} skipped")
    else:
        print("  vs R:      Rscript not found, skipped")

    if args.json_out:
        with open(args.json_out, "w", encoding="utf-8") as h:
            json.dump({"cases": len(cases), "counts": counts, "results": results}, h, indent=2)

    if args.md_out:
        # A verdict alone is not evidence. Show what each language returned so a
        # reader can check the comparison rather than take the word PASS for it.
        def fmt(v):
            if v is None:
                return "-"
            t = json.dumps(v)
            return "`" + (t if len(t) <= 60 else t[:60] + "...") + "`"
        lines = ["# One-liner correctness", "",
                 f"{len(cases)} cases. Values are compared as JSON: floats to 1e-9, "
                 "integers and strings exactly.", "",
                 "`DIFFER` marks a known convention difference, recorded on purpose; "
                 "such a case fails if it ever starts agreeing.", "",
                 "| Case | Category | BioLang | Python | R | vs Py | vs R |",
                 "|---|---|---|---|---|---|---|"]
        for r in results:
            lines.append(f"| {r['name']} | {r['category']} | {fmt(r['biolang'])} | "
                         f"{fmt(r['python'])} | {fmt(r['r'])} | {r['vs_python']} | {r['vs_r']} |")
        lines += ["", f"vs Python: {counts['py_pass']} passed, {counts['py_fail']} failed",
                  "", f"vs R: {counts['r_pass']} passed, {counts['r_fail']} failed, "
                  f"{counts['r_skip']} skipped", ""]
        with open(args.md_out, "w", encoding="utf-8", newline=chr(10)) as h:
            h.write(chr(10).join(lines))

    if bl["code"] != 0 and not bl["values"]:
        print(f"\nBioLang produced nothing. stderr:\n{bl['stderr']}")
    return 1 if counts["py_fail"] or counts["r_fail"] else 0


if __name__ == "__main__":
    sys.exit(main())

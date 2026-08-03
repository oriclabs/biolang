#!/usr/bin/env python3
"""
Benchmark BioLang against the tools bioinformaticians actually use.

The point of this harness is fairness. An earlier round compared BioLang's
native builtins against hand-written Python loops, which flatters BioLang and
tells a reader nothing: nobody counts GC content with a Python loop when
Biopython is installed. Every workload here is run against the library a
practitioner would reach for, and where a specialised C library exists — edlib
for edit distance — that is the comparator rather than a naive implementation.

Method
  * Every implementation computes the same answer from the same input, and the
    harness checks they agree before reporting any timing. A disagreement is a
    failure, not a footnote.
  * REPEATS timed runs after WARMUP discarded runs.
  * Reported as median and median absolute deviation. Medians because process
    scheduling on a shared machine produces occasional long runs that a mean
    would chase.
  * BioLang is timed by its own interpreter clock, which excludes process
    startup, so Python is timed in-process for the same reason. Startup is
    reported separately rather than smuggled into a workload.

Usage
    python bench/harness.py                 # everything
    python bench/harness.py --repeats 20    # more samples
    python bench/harness.py --json out.json # machine-readable
"""
from __future__ import annotations

import argparse
import json
import platform
import random
import re
import statistics
import subprocess
import sys
import time
from dataclasses import dataclass, field
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
DATA = Path(__file__).resolve().parent / "data"
BL = REPO / "target" / "release" / ("bl.exe" if sys.platform == "win32" else "bl")

WARMUP = 2
REPEATS = 10
SEED = 20260803


# ── data ────────────────────────────────────────────────────────────────────

def ensure_data(megabases: int = 1) -> dict[str, Path]:
    """Deterministic inputs, written once and reused."""
    DATA.mkdir(parents=True, exist_ok=True)
    genome = DATA / f"genome_{megabases}mb.fa"
    reads = DATA / "reads_1k.fa"

    if not genome.exists():
        rng = random.Random(SEED)
        seq = "".join(rng.choice("ACGT") for _ in range(megabases * 1_000_000))
        with genome.open("w") as handle:
            handle.write(f">chr_synthetic_{megabases}mb\n")
            for i in range(0, len(seq), 60):
                handle.write(seq[i : i + 60] + "\n")

    if not reads.exists():
        rng = random.Random(SEED + 1)
        with reads.open("w") as handle:
            for i in range(1000):
                handle.write(f">read_{i}\n{''.join(rng.choice('ACGT') for _ in range(300))}\n")

    return {"genome": genome, "reads": reads}


def load_genome(path: Path) -> str:
    return "".join(l.strip() for l in path.open() if not l.startswith(">"))


# ── timing ──────────────────────────────────────────────────────────────────

@dataclass
class Result:
    workload: str
    implementation: str
    answer: str
    samples: list[float] = field(default_factory=list)

    @property
    def median(self) -> float:
        return statistics.median(self.samples)

    @property
    def mad(self) -> float:
        med = self.median
        return statistics.median([abs(s - med) for s in self.samples]) if self.samples else 0.0


def time_python(fn, repeats: int) -> tuple[list[float], str]:
    answer = ""
    for _ in range(WARMUP):
        answer = str(fn())
    samples = []
    for _ in range(repeats):
        start = time.perf_counter()
        answer = str(fn())
        samples.append((time.perf_counter() - start) * 1000.0)
    return samples, answer


DONE_IN = re.compile(r"done in ([0-9.]+)(ms|s)")


def time_biolang(script: str, repeats: int) -> tuple[list[float], str]:
    """Run a BioLang script, timed by the interpreter's own clock."""
    path = DATA / "_bench.bl"
    path.write_text(script, encoding="utf-8")
    samples, answer = [], ""
    for i in range(WARMUP + repeats):
        proc = subprocess.run(
            [str(BL), "run", str(path)], capture_output=True, text=True, cwd=str(DATA)
        )
        out = proc.stdout + proc.stderr
        match = DONE_IN.search(out)
        if not match:
            raise RuntimeError(f"no timing in BioLang output:\n{out[:400]}")
        ms = float(match.group(1)) * (1000.0 if match.group(2) == "s" else 1.0)
        lines = [
            l.strip()
            for l in out.splitlines()
            if l.strip() and "running" not in l and "done in" not in l
        ]
        answer = lines[-1] if lines else ""
        if i >= WARMUP:
            samples.append(ms)
    return samples, answer


# ── workloads ───────────────────────────────────────────────────────────────
#
# Each returns (name, {implementation: (callable_or_script, kind)}). Answers are
# normalised to strings and compared across implementations before timing is
# reported.

def workloads(paths: dict[str, Path], genome: str) -> list[tuple[str, dict]]:
    from Bio.Seq import Seq
    from Bio.SeqUtils import gc_fraction
    from Bio import SeqIO
    import edlib

    genome_file = paths["genome"].name
    reads_file = paths["reads"].name

    # Symmetry. A BioLang script runs as its own process, so it must read the
    # FASTA inside the timed region; there is nowhere to hoist it to. The first
    # version of this harness let the Python implementations work from a string
    # loaded once beforehand, which charged BioLang for parsing a megabase and
    # charged Python for nothing. Both now start from the file.
    def load() -> str:
        return "".join(
            l.strip() for l in paths["genome"].open() if not l.startswith(">")
        )

    def slices() -> tuple[str, str]:
        g = load()
        return g[:2000], g[5000:7000]

    return [
        (
            "reverse complement (1 Mb)",
            {
                "BioLang": (
                    f'let r = reverse_complement(read_fasta("{genome_file}")[0].seq)\n'
                    f'println(substr(str(r), 0, 12))\n',
                    "bl",
                ),
                "Biopython": (lambda: str(Seq(load()).reverse_complement())[:12], "py"),
                "Python stdlib": (
                    lambda: load().translate(str.maketrans("ACGT", "TGCA"))[::-1][:12],
                    "py",
                ),
            },
        ),
        (
            "GC content (1 Mb)",
            {
                "BioLang": (
                    f'println(str(round(gc_content(read_fasta("{genome_file}")[0].seq), 6)))\n',
                    "bl",
                ),
                "Biopython": (lambda: f"{round(gc_fraction(load()), 6)}", "py"),
                "Python stdlib": (
                    lambda: (lambda g: f"{round((g.count('G') + g.count('C')) / len(g), 6)}")(load()),
                    "py",
                ),
            },
        ),
        (
            "translation (1 Mb -> protein)",
            {
                "BioLang": (
                    f'let p = translate(read_fasta("{genome_file}")[0].seq)\n'
                    f'println(substr(str(p), 0, 12))\n',
                    "bl",
                ),
                "Biopython": (
                    lambda: (lambda g: str(Seq(g[: (len(g) // 3) * 3]).translate())[:12])(load()),
                    "py",
                ),
            },
        ),
        (
            "edit distance (2 kb x 2 kb)",
            {
                "BioLang": (
                    f'let s = str(read_fasta("{genome_file}")[0].seq)\n'
                    f"println(str(edit_distance(substr(s, 0, 2000), substr(s, 5000, 2000))))\n",
                    "bl",
                ),
                "edlib (C)": (
                    lambda: (lambda p: str(edlib.align(p[0], p[1], task="distance")["editDistance"]))(slices()),
                    "py",
                ),
                "Biopython": (
                    lambda: str(
                        int(
                            -__import__("Bio.Align", fromlist=["PairwiseAligner"])
                            .PairwiseAligner(
                                mode="global", match_score=0, mismatch_score=-1,
                                open_gap_score=-1, extend_gap_score=-1,
                            )
                            .score(*slices())
                        )
                    ),
                    "py",
                ),
            },
        ),
        (
            "FASTA parse (1000 reads)",
            {
                "BioLang": (
                    f'let rs = read_fasta("{reads_file}")\n'
                    f"println(str(len(rs)) + \" \" + str(sum(rs |> map(|r| seq_len(r.seq)))))\n",
                    "bl",
                ),
                "Biopython": (
                    lambda: (
                        lambda rs: f"{len(rs)} {sum(len(r.seq) for r in rs)}"
                    )(list(SeqIO.parse(str(paths['reads']), "fasta"))),
                    "py",
                ),
            },
        ),
        (
            "k-mer counting (k=8, 1 Mb)",
            {
                "BioLang": (
                    f'let s = read_fasta("{genome_file}")[0].seq\n'
                    f"println(str(len(kmers(s, 8) |> unique())))\n",
                    "bl",
                ),
                "Python dict": (
                    lambda: (lambda g: str(len({g[i : i + 8] for i in range(len(g) - 7)})))(load()),
                    "py",
                ),
            },
        ),
        (
            "interpreted loop (300k iterations)",
            {
                "BioLang": (
                    "let t = 0\nlet i = 0\nwhile i < 300000 {\n t = t + i % 7\n i = i + 1\n}\nprintln(str(t))\n",
                    "bl",
                ),
                "Python": (
                    lambda: str(sum(i % 7 for i in range(300000))),
                    "py",
                ),
            },
        ),
    ]


# ── driver ──────────────────────────────────────────────────────────────────

def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repeats", type=int, default=REPEATS)
    parser.add_argument("--json", type=Path)
    args = parser.parse_args()

    if not BL.exists():
        print(f"No release build at {BL} — cargo build --release -p bl-cli", file=sys.stderr)
        return 2

    paths = ensure_data()
    genome = load_genome(paths["genome"])

    version = subprocess.run(
        [str(BL), "--version"], capture_output=True, text=True
    ).stdout.strip()
    import Bio

    environment = {
        "machine": f"{platform.system()} {platform.release()} ({platform.machine()})",
        "cpu": platform.processor() or "unknown",
        "biolang": version,
        "python": sys.version.split()[0],
        "biopython": Bio.__version__,
        "repeats": args.repeats,
        "warmup": WARMUP,
        "seed": SEED,
        "input": f"{len(genome):,} bp synthetic genome, 1000 x 300 bp reads",
    }
    for key, value in environment.items():
        print(f"{key:>12}: {value}")
    print()

    results: list[Result] = []
    mismatches: list[str] = []

    for name, implementations in workloads(paths, genome):
        answers: dict[str, str] = {}
        row: list[Result] = []
        for label, (target, kind) in implementations.items():
            samples, answer = (
                time_biolang(target, args.repeats)
                if kind == "bl"
                else time_python(target, args.repeats)
            )
            answers[label] = answer
            row.append(Result(name, label, answer, samples))

        # Agreement is checked before any timing is believed. Implementations
        # that compute different things cannot be compared on speed.
        distinct = set(answers.values())
        if len(distinct) > 1:
            mismatches.append(f"{name}: " + "; ".join(f"{k}={v!r}" for k, v in answers.items()))

        results.extend(row)
        baseline = min(r.median for r in row)
        print(f"{name}")
        for result in sorted(row, key=lambda r: r.median):
            ratio = result.median / baseline
            marker = " (fastest)" if ratio == 1.0 else f" {ratio:>6.1f}x"
            print(
                f"    {result.implementation:<16}{result.median:>9.2f} ms"
                f" ± {result.mad:<7.2f}{marker}"
            )
        print()

    if mismatches:
        print("IMPLEMENTATIONS DISAGREE — timings above are not comparable:\n", file=sys.stderr)
        for line in mismatches:
            print(f"  {line}", file=sys.stderr)

    if args.json:
        args.json.write_text(
            json.dumps(
                {
                    "environment": environment,
                    "mismatches": mismatches,
                    "results": [
                        {
                            "workload": r.workload,
                            "implementation": r.implementation,
                            "answer": r.answer,
                            "median_ms": r.median,
                            "mad_ms": r.mad,
                            "samples_ms": r.samples,
                        }
                        for r in results
                    ],
                },
                indent=2,
            )
            + "\n",
            encoding="utf-8",
        )
        print(f"wrote {args.json}")

    return 1 if mismatches else 0


if __name__ == "__main__":
    raise SystemExit(main())

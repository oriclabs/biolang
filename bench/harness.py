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
import shutil
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

    # The HMM observation string, written once. Both implementations read it
    # inside their timed region, which is the only symmetric arrangement: the
    # first version of this workload had BioLang build the string with an
    # interpreted map-and-join while Python got it precomputed at import. That
    # charged BioLang 91 ms of string building and reported it as decode time,
    # turning a 4x gap into a 19x one. It is the same mistake this harness was
    # written to avoid, made again.
    observations = DATA / "hmm_observed.txt"
    if not observations.exists():
        observations.write_text("".join("xyz"[i % 3] for i in range(100_000)), encoding="utf-8")

    return {"genome": genome, "reads": reads, "observations": observations}


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


RSCRIPT = shutil.which("Rscript")


def time_r(script: str, repeats: int) -> tuple[list[float], str]:
    """Run an R snippet, timed inside the process by Sys.time().

    Timed the same way as the others: the measurement excludes interpreter
    startup and library loading, because charging R for `library(Biostrings)`
    would repeat the mistake an earlier round of this harness made with
    `import Bio` — it measured the import and called it a parse.
    """
    if RSCRIPT is None:
        raise RuntimeError("Rscript not on PATH")
    path = DATA / "_bench.R"
    # The script must print its answer last and nothing else.
    path.write_text(script, encoding="utf-8")
    samples, answer = [], ""
    for i in range(WARMUP + repeats):
        proc = subprocess.run(
            [RSCRIPT, "--vanilla", str(path)],
            capture_output=True,
            text=True,
            cwd=str(DATA),
        )
        if proc.returncode != 0:
            raise RuntimeError("R failed: " + proc.stderr[:400])
        lines = [l.strip() for l in proc.stdout.splitlines() if l.strip()]
        # Last two lines: elapsed milliseconds, then the answer.
        ms = float(lines[-2])
        answer = lines[-1]
        if i >= WARMUP:
            samples.append(ms)
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


# ── comparators for the post-1.0.0 workloads ────────────────────────────────
#
# Each is written the way a practitioner would write it, not the way that makes
# BioLang look good. Where a C-backed library exists it is the comparator; the
# pure-Python version is kept alongside so a reader can see what the library is
# worth, not so BioLang can be compared against the slower of the two.

HMM_STATES = 2
HMM_TRANSITION = [[0.641, 0.359], [0.729, 0.271]]
HMM_EMISSION = [[0.117, 0.691, 0.192], [0.097, 0.42, 0.483]]
def _hmm_observed() -> list[int]:
    """Read the observations, as BioLang does, inside the timed region."""
    text = (DATA / "hmm_observed.txt").read_text(encoding="utf-8")
    index = {"x": 0, "y": 1, "z": 2}
    return [index[c] for c in text]


def viterbi_python() -> str:
    """Viterbi in log space, which is what anyone would write without a library."""
    import math

    observed = _hmm_observed()

    def ln(x: float) -> float:
        return math.log(x) if x > 0 else float("-inf")

    n = HMM_STATES
    score = [ln(1.0 / n) + ln(HMM_EMISSION[s][observed[0]]) for s in range(n)]
    back = [[0] * n for _ in range(len(observed))]
    for t in range(1, len(observed)):
        nxt = [float("-inf")] * n
        for s in range(n):
            best, best_from = float("-inf"), 0
            for p in range(n):
                cand = score[p] + ln(HMM_TRANSITION[p][s])
                if cand > best:
                    best, best_from = cand, p
            back[t][s] = best_from
            nxt[s] = best + ln(HMM_EMISSION[s][observed[t]])
        score = nxt
    end = max(range(n), key=lambda s: score[s])
    path = [0] * len(observed)
    path[-1] = end
    for t in range(len(observed) - 1, 0, -1):
        path[t - 1] = back[t][path[t]]
    return "".join("AB"[s] for s in path[:12])


def viterbi_hmmlearn() -> str:
    """The library a practitioner reaches for. NumPy underneath."""
    import numpy as np
    from hmmlearn import hmm

    observed = _hmm_observed()

    model = hmm.CategoricalHMM(n_components=HMM_STATES, init_params="")
    model.startprob_ = np.array([0.5, 0.5])
    model.transmat_ = np.array(HMM_TRANSITION)
    model.emissionprob_ = np.array(HMM_EMISSION)
    observed = np.array(observed).reshape(-1, 1)
    _, path = model.decode(observed, algorithm="viterbi")
    return "".join("AB"[s] for s in path[:12])


def suffix_array_python() -> str:
    """Prefix doubling, the same algorithm BioLang's builtin uses."""
    text = _bench_genome[:200_000]
    n = len(text)
    sa = list(range(n))
    rank = [ord(c) for c in text]
    span = 1
    while span < n:
        key = lambda i: (rank[i], rank[i + span] if i + span < n else -1)
        sa.sort(key=key)
        nxt = [0] * n
        for pos in range(1, n):
            nxt[sa[pos]] = nxt[sa[pos - 1]] + (key(sa[pos - 1]) != key(sa[pos]))
        rank = nxt
        if rank[sa[-1]] == n - 1:
            break
        span *= 2
    return ",".join(str(v) for v in sa[:4])


def suffix_array_c() -> str:
    """pydivsufsort: SA-IS in C, which is the right comparator."""
    from pydivsufsort import divsufsort

    sa = divsufsort(_bench_genome[:200_000].encode())
    return ",".join(str(int(v)) for v in sa[:4])


REVERSAL_PAIRS = [
    ([1, 2, 3, 4, 5, 6, 7, 8, 9, 10], [3, 1, 5, 2, 7, 4, 9, 6, 10, 8]),
    ([3, 10, 8, 2, 5, 4, 7, 1, 6, 9], [5, 2, 3, 1, 7, 4, 10, 8, 6, 9]),
    ([8, 6, 7, 9, 4, 1, 3, 10, 2, 5], [8, 2, 7, 6, 9, 1, 5, 3, 10, 4]),
]


def reversal_distance_python() -> str:
    """Bidirectional BFS, the same approach the builtin takes.

    Included because there is no standard Python library for reversal distance
    — which is itself the finding. The comparison is BioLang against what a
    practitioner would have to write themselves.
    """
    from collections import deque

    def solve(source: list[int], target: list[int]) -> int:
        rank = {v: i for i, v in enumerate(target)}
        start = tuple(rank[v] for v in source)
        goal = tuple(range(len(source)))
        if start == goal:
            return 0
        moves = [
            (i, j) for i in range(len(goal)) for j in range(i + 1, len(goal))
        ]
        seen_a, seen_b = {start: 0}, {goal: 0}
        edge_a, edge_b = deque([start]), deque([goal])
        while True:
            if len(edge_a) <= len(edge_b):
                edge, seen, other = edge_a, seen_a, seen_b
            else:
                edge, seen, other = edge_b, seen_b, seen_a
            for _ in range(len(edge)):
                cur = edge.popleft()
                for i, j in moves:
                    nxt = cur[:i] + cur[i : j + 1][::-1] + cur[j + 1 :]
                    if nxt in seen:
                        continue
                    seen[nxt] = seen[cur] + 1
                    if nxt in other:
                        return seen[nxt] + other[nxt]
                    edge.append(nxt)

    return " ".join(str(solve(a, b)) for a, b in REVERSAL_PAIRS)


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
        # ── workloads exercising code added after v1.0.0 ────────────────────
        #
        # The seven above were chosen when BioLang's only advantage was that a
        # handful of sequence operations are Rust builtins. They show the
        # pattern honestly: BioLang wins where a builtin does the work and
        # loses three to five times over wherever the work happens in
        # interpreted BioLang. These four cover the Rust modules added since,
        # where the comparison is against a library rather than against the
        # interpreter.
        (
            "Viterbi decode (2-state HMM, 100k symbols)",
            {
                "BioLang": (
                    'let model = {\n'
                    '    states: ["A", "B"],\n'
                    '    symbols: ["x", "y", "z"],\n'
                    '    transition: { A: { A: 0.641, B: 0.359 }, B: { A: 0.729, B: 0.271 } },\n'
                    '    emission: { A: { x: 0.117, y: 0.691, z: 0.192 },\n'
                    '                B: { x: 0.097, y: 0.42,  z: 0.483 } },\n'
                    '}\n'
                    'let observed = read_text("hmm_observed.txt")\n'
                    'let path = viterbi(observed, model) |> join("")\n'
                    'println(substr(path, 0, 12))\n',
                    "bl",
                ),
                "hmmlearn (NumPy)": (viterbi_hmmlearn, "py"),
                "pure Python": (viterbi_python, "py"),
            },
        ),
        (
            "suffix array (200 kb)",
            {
                "BioLang": (
                    f'let g = substr(str(read_fasta("{genome_file}")[0].seq), 0, 200000)\n'
                    'let sa = suffix_array(g)\n'
                    'println(join(map(slice(sa, 0, 4), |v| str(v)), ","))\n',
                    "bl",
                ),
                "pydivsufsort (C)": (suffix_array_c, "py"),
                "pure Python": (suffix_array_python, "py"),
            },
        ),
        (
            "reversal distance (length-10 permutations)",
            {
                "BioLang": (
                    'let pairs = [\n'
                    '    { a: [1,2,3,4,5,6,7,8,9,10], b: [3,1,5,2,7,4,9,6,10,8] },\n'
                    '    { a: [3,10,8,2,5,4,7,1,6,9], b: [5,2,3,1,7,4,10,8,6,9] },\n'
                    '    { a: [8,6,7,9,4,1,3,10,2,5], b: [8,2,7,6,9,1,5,3,10,4] },\n'
                    ']\n'
                    'println(join(map(pairs, |p| str(reversal_distance(p.a, p.b))), " "))\n',
                    "bl",
                ),
                "pure Python BFS": (reversal_distance_python, "py"),
            },
        ),
        (
            "GC content (1 Mb) — with R",
            {
                "BioLang": (
                    f'println(str(round(gc_content(read_fasta("{genome_file}")[0].seq), 6)))\n',
                    "bl",
                ),
                "Biopython": (lambda: f"{round(gc_fraction(load()), 6)}", "py"),
                "R Biostrings": (
                    'suppressPackageStartupMessages(library(Biostrings))\n'
                    f's <- readDNAStringSet("{genome_file}")[[1]]\n'
                    't0 <- Sys.time()\n'
                    'f <- letterFrequency(s, c("G","C"), as.prob=TRUE)\n'
                    'ans <- format(round(sum(f), 6), nsmall=6)\n'
                    't1 <- Sys.time()\n'
                    'cat(as.numeric(difftime(t1, t0, units="secs")) * 1000, "\\n")\n'
                    'cat(ans, "\\n")\n',
                    "r",
                ),
            },
        ),
        (
            "reverse complement (1 Mb) — with R",
            {
                "BioLang": (
                    f'let r = reverse_complement(read_fasta("{genome_file}")[0].seq)\n'
                    f'println(substr(str(r), 0, 12))\n',
                    "bl",
                ),
                "Biopython": (lambda: str(Seq(load()).reverse_complement())[:12], "py"),
                "R Biostrings": (
                    'suppressPackageStartupMessages(library(Biostrings))\n'
                    f's <- readDNAStringSet("{genome_file}")[[1]]\n'
                    't0 <- Sys.time()\n'
                    'ans <- substr(as.character(reverseComplement(s)), 1, 12)\n'
                    't1 <- Sys.time()\n'
                    'cat(as.numeric(difftime(t1, t0, units="secs")) * 1000, "\\n")\n'
                    'cat(ans, "\\n")\n',
                    "r",
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
    globals()["_bench_genome"] = genome

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
            if kind == "bl":
                samples, answer = time_biolang(target, args.repeats)
            elif kind == "r":
                if RSCRIPT is None:
                    continue   # R absent: report the rest rather than nothing
                samples, answer = time_r(target, args.repeats)
            else:
                samples, answer = time_python(target, args.repeats)
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

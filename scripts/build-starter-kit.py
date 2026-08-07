#!/usr/bin/env python3
"""Build the single-cell starter kit that the book offers for download.

The kit exists because there is no package registry: `bl install` takes a local
path or a git URL, and a git URL clones a whole repository into the package
slot, which cannot work for a package living in a subdirectory. So a reader
arriving from the website has no way to get `singlecell` without cloning the
repo. The kit ships the package instead of pointing at it.

It also ships the dataset already generated, so the reader does not need Python
to run a BioLang tutorial.

This is a build step rather than a committed binary. The zip contains a copy of
the package and of the book's example scripts, and a copy that is committed once
and never rebuilt is a copy that goes stale the first time either changes -
which is the failure this repository keeps hitting.

Usage:
    python scripts/build-starter-kit.py
"""

import os
import shutil
import subprocess
import sys
import tempfile
import zipfile

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
BOOK = os.path.join(ROOT, "books", "single-cell-rna-seq", "src")
PACKAGE = os.path.join(ROOT, "packages", "singlecell")
GENERATOR = os.path.join(PACKAGE, "examples", "make_demo_10x.py")
OUT = os.path.join(BOOK, "downloads", "singlecell-starter.zip")

KIT = "singlecell-starter"


def main():
    for required in (PACKAGE, GENERATOR, os.path.join(BOOK, "downloads")):
        if not os.path.exists(required):
            sys.exit("missing: %s" % required)

    with tempfile.TemporaryDirectory() as tmp:
        stage = os.path.join(tmp, KIT)
        os.makedirs(stage)

        # The package itself, minus build noise.
        shutil.copytree(
            PACKAGE,
            os.path.join(stage, "singlecell"),
            ignore=shutil.ignore_patterns("__pycache__", "*.pyc", ".git"),
        )

        # The dataset, generated here so the reader needs no Python. The
        # generator is seeded, so this is reproducible.
        subprocess.run(
            [sys.executable, GENERATOR, "--output", os.path.join(stage, "nsclc_like")],
            check=True,
            stdout=subprocess.DEVNULL,
        )
        shutil.copy2(GENERATOR, stage)

        # The book's own scripts and notebook.
        downloads = os.path.join(BOOK, "downloads")
        for name in sorted(os.listdir(downloads)):
            if name.endswith((".bl", ".bln")):
                shutil.copy2(os.path.join(downloads, name), stage)

        readme = os.path.join(downloads, "starter-kit-README.md")
        if os.path.exists(readme):
            shutil.copy2(readme, os.path.join(stage, "README.md"))

        if os.path.exists(OUT):
            os.remove(OUT)

        # Forward slashes explicitly. PowerShell's Compress-Archive writes
        # backslash separators, producing an archive that every Linux and macOS
        # reader gets a warning from and cannot extract cleanly.
        count = 0
        with zipfile.ZipFile(OUT, "w", zipfile.ZIP_DEFLATED, compresslevel=9) as z:
            for folder, dirs, files in os.walk(stage):
                dirs[:] = [d for d in dirs if d != "__pycache__"]
                for name in sorted(files):
                    full = os.path.join(folder, name)
                    rel = os.path.relpath(full, tmp).replace(os.sep, "/")
                    z.write(full, rel)
                    count += 1

    with zipfile.ZipFile(OUT) as z:
        bad = [n for n in z.namelist() if "\\" in n]
        if bad:
            sys.exit("archive has backslash paths: %s" % bad[:3])

    print("singlecell-starter.zip: %d files, %d KB" % (count, os.path.getsize(OUT) // 1024))


if __name__ == "__main__":
    main()

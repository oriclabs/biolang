#!/usr/bin/env python3
"""Fetch the two 10x matrices this book runs on.

The data is the PBMC control/interferon-stimulated experiment the Harvard Chan
Bioinformatics Core uses in its scRNA-seq course, hosted by them on Dropbox.

The awkward part, stated up front: **the archive is 3.2 GB and the two
directories we need total about 90 MB.** Zip stores its index at the end of the
file, so there is no way to pull two members out of a remote archive without
fetching the whole thing. HBC's own download.sh has the same problem. This
script downloads once, extracts what the book uses, and deletes the archive, so
the 3.2 GB is transient rather than resident.

The rest of the archive is R objects (a 2.2 GB seurat_integrated.RData.bz2 among
them) that BioLang cannot read and this book does not need.

Usage:
    python get-data.py            # download, extract, clean up
    python get-data.py --keep     # leave the archive in place
"""

import argparse
import os
import shutil
import sys
import urllib.request
import zipfile

URL = (
    "https://www.dropbox.com/scl/fi/uoro3sbex3tj1e6m61z16/"
    "single_cell_rnaseq.zip?rlkey=cfay7tqm3ta5qlh2gph7h2wko&dl=1"
)
ARCHIVE = "single_cell_rnaseq.zip"

# (member prefix inside the archive, directory we extract it to)
WANTED = [
    ("single_cell_rnaseq/data/ctrl_raw_feature_bc_matrix/", "ctrl_raw"),
    ("single_cell_rnaseq/data/stim_raw_feature_bc_matrix/", "stim_raw"),
]
EXPECTED = {"barcodes.tsv.gz", "features.tsv.gz", "matrix.mtx.gz"}


def report(done, total):
    if total <= 0:
        sys.stdout.write("\r  %d MB" % (done // (1 << 20)))
    else:
        sys.stdout.write(
            "\r  %d / %d MB (%d%%)"
            % (done // (1 << 20), total // (1 << 20), 100 * done // total)
        )
    sys.stdout.flush()


def download():
    if os.path.exists(ARCHIVE):
        print("Archive already here, skipping the download.")
        return
    print("Downloading 3.2 GB. This is the slow part; it happens once.")
    request = urllib.request.Request(URL, headers={"User-Agent": "biolang-book"})
    with urllib.request.urlopen(request) as response:
        total = int(response.headers.get("Content-Length", 0))
        done = 0
        with open(ARCHIVE + ".part", "wb") as out:
            while True:
                chunk = response.read(1 << 20)
                if not chunk:
                    break
                out.write(chunk)
                done += len(chunk)
                report(done, total)
    print()
    os.replace(ARCHIVE + ".part", ARCHIVE)


def extract():
    with zipfile.ZipFile(ARCHIVE) as archive:
        names = archive.namelist()
        for prefix, target in WANTED:
            members = [
                n
                for n in names
                if n.startswith(prefix)
                and not n.endswith("/")
                and "__MACOSX" not in n
            ]
            if not members:
                sys.exit("archive has no %s — has the source layout changed?" % prefix)
            os.makedirs(target, exist_ok=True)
            for member in members:
                leaf = os.path.basename(member)
                with archive.open(member) as src, open(
                    os.path.join(target, leaf), "wb"
                ) as dst:
                    shutil.copyfileobj(src, dst)
            got = set(os.listdir(target))
            missing = EXPECTED - got
            if missing:
                sys.exit("%s is incomplete, missing %s" % (target, sorted(missing)))
            size = sum(
                os.path.getsize(os.path.join(target, f)) for f in os.listdir(target)
            )
            print("  %-10s %s  (%d MB)" % (target, sorted(got), size // (1 << 20)))


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--keep", action="store_true", help="do not delete the archive afterwards"
    )
    args = parser.parse_args()

    download()
    print("Extracting the two matrix directories:")
    extract()
    if not args.keep:
        os.remove(ARCHIVE)
        print("Removed the archive; the extracted matrices are ~90 MB.")
    print("\nReady. Run the book's scripts from this directory.")


if __name__ == "__main__":
    main()

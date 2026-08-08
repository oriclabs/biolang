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

The download resumes. If it dies partway — and over 3.2 GB it may — run the
script again and it continues from the partial file rather than starting over.

Usage:
    python get-data.py            # download, extract, clean up
    python get-data.py --keep     # leave the archive in place
"""

import argparse
import os
import shutil
import sys
import urllib.error
import urllib.request
import zipfile

URL = (
    "https://www.dropbox.com/scl/fi/uoro3sbex3tj1e6m61z16/"
    "single_cell_rnaseq.zip?rlkey=cfay7tqm3ta5qlh2gph7h2wko&dl=1"
)
ARCHIVE = "single_cell_rnaseq.zip"
MAX_ATTEMPTS = 6

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


def fetch(have):
    """One attempt, resuming from byte `have`. Returns bytes on disk afterwards.

    A 3.2 GB transfer over a connection that drops is the normal case, not the
    unlucky one — this download died at 687 MB with a connection reset while the
    book was being written. So the partial file is kept and continued with a
    Range request rather than restarted. If the server ignores the Range (no 206)
    we start over, because appending a full body to a partial file would produce
    a corrupt archive that only fails much later, at unzip time.
    """
    headers = {"User-Agent": "biolang-book"}
    if have:
        headers["Range"] = "bytes=%d-" % have
    request = urllib.request.Request(URL, headers=headers)
    with urllib.request.urlopen(request) as response:
        resumed = response.status == 206
        if have and not resumed:
            print("\n  server ignored the resume request; starting over")
            have = 0
        total = int(response.headers.get("Content-Length", 0)) + have
        mode = "ab" if have else "wb"
        done = have
        with open(ARCHIVE + ".part", mode) as out:
            while True:
                chunk = response.read(1 << 20)
                if not chunk:
                    break
                out.write(chunk)
                done += len(chunk)
                report(done, total)
    return done, total


def download():
    if os.path.exists(ARCHIVE):
        print("Archive already here, skipping the download.")
        return
    part = ARCHIVE + ".part"
    have = os.path.getsize(part) if os.path.exists(part) else 0
    if have:
        print("Resuming from %d MB." % (have // (1 << 20)))
    else:
        print("Downloading 3.2 GB. This is the slow part; it happens once.")

    for attempt in range(MAX_ATTEMPTS):
        try:
            done, total = fetch(have)
            if total and done < total:
                raise OSError("connection closed at %d of %d bytes" % (done, total))
            break
        except (OSError, urllib.error.URLError) as exc:
            have = os.path.getsize(part) if os.path.exists(part) else 0
            if attempt == MAX_ATTEMPTS - 1:
                sys.exit(
                    "\nDownload failed after %d attempts: %s\n"
                    "The %d MB already fetched is kept in %s — run this again to "
                    "resume." % (MAX_ATTEMPTS, exc, have // (1 << 20), part)
                )
            print("\n  %s — retrying from %d MB" % (exc, have // (1 << 20)))
    print()
    os.replace(part, ARCHIVE)


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

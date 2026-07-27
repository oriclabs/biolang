"""Compact blosc conformance vectors, packed into one file + JSON manifest.

Ground truth comes from numcodecs — the same encoder scanpy/anndata use — so
these vectors pin our decoder to the real format rather than to our reading of
the spec.

Covers the full codec x shuffle x typesize grid, and uses an explicit small
`blocksize` to force multi-block layouts (and thus out-of-order block offset
tables) without needing megabyte-sized buffers.

  vectors.bin   — all compressed and raw payloads concatenated
  vectors.json  — manifest: offsets/lengths into vectors.bin + parameters

Regenerate (writes ./packed/, then copy both files up one level):

  docker run --rm -v "$PWD":/out -w /out python:3.11-slim \
    sh -c "pip -q install numcodecs numpy && python generate_vectors.py"

The store.zarr / store_X.f32 fixtures used by the end-to-end test come from
the sibling script in the session scratchpad; store.zarr is written by
zarr with default settings (blosc + lz4 + byte shuffle) and store_X.f32 is
its X matrix dumped as little-endian float32.
"""
import json
import os

import numpy as np
from numcodecs import Blosc

OUT = "packed"
os.makedirs(OUT, exist_ok=True)

DTYPES = [("|u1", 1), ("<i2", 2), ("<f4", 4), ("<f8", 8)]
CNAMES = ["blosclz", "lz4", "lz4hc", "zlib", "zstd"]
SHUFFLES = [("noshuffle", Blosc.NOSHUFFLE), ("shuffle", Blosc.SHUFFLE),
            ("bitshuffle", Blosc.BITSHUFFLE)]

# (nelem, blocksize) — blocksize 0 means "let blosc choose".
# The small explicit blocksizes force multi-block buffers cheaply, and sizes
# that divide neither evenly by blocksize nor by typesize exercise the
# leftover-block and shuffle-tail paths.
SHAPES = [(1, 0), (7, 0), (100, 0), (200, 0), (333, 0),
          (200, 64), (200, 128), (150, 32), (333, 96)]

blob = bytearray()
cases = []


def put(b: bytes):
    off = len(blob)
    blob.extend(b)
    return off, len(b)


idx = 0
for dt, tsize in DTYPES:
    for cname in CNAMES:
        for sh_name, sh in SHUFFLES:
            for nelem, bsz in SHAPES:
                rng = np.random.default_rng(9000 + idx)
                if tsize == 1:
                    arr = rng.integers(0, 256, size=nelem, dtype=np.uint8)
                elif tsize == 2:
                    arr = rng.integers(-3000, 3000, size=nelem).astype("<i2")
                elif tsize == 4:
                    arr = np.where(rng.random(nelem) < 0.5, 1.5,
                                   rng.random(nelem)).astype("<f4")
                else:
                    arr = np.where(rng.random(nelem) < 0.5, 0.0,
                                   rng.random(nelem)).astype("<f8")

                kw = dict(cname=cname, clevel=5, shuffle=sh)
                if bsz:
                    kw["blocksize"] = bsz
                try:
                    comp = bytes(Blosc(**kw).encode(arr))
                except Exception as e:
                    print("skip", cname, sh_name, e)
                    continue

                raw = arr.tobytes()
                c_off, c_len = put(comp)
                r_off, r_len = put(raw)
                nblocks = -1
                import struct
                nb, blocksize, _cb = struct.unpack("<III", comp[4:16])
                nblocks = (nb + blocksize - 1) // blocksize if blocksize else 0
                cases.append({
                    "name": f"{cname}_{sh_name}_ts{tsize}_n{nelem}_bs{bsz}",
                    "cname": cname, "shuffle": sh_name, "typesize": tsize,
                    "nelem": nelem, "req_blocksize": bsz,
                    "hdr_flags": comp[2], "hdr_typesize": comp[3],
                    "blocksize": blocksize, "nblocks": nblocks,
                    "memcpyed": bool(comp[2] & 0x02),
                    "c_off": c_off, "c_len": c_len,
                    "r_off": r_off, "r_len": r_len,
                })
                idx += 1

with open(os.path.join(OUT, "vectors.bin"), "wb") as f:
    f.write(blob)
with open(os.path.join(OUT, "vectors.json"), "w") as f:
    json.dump(cases, f, indent=0)

multi = sum(1 for c in cases if c["nblocks"] > 1)
comp_n = sum(1 for c in cases if not c["memcpyed"])
print(f"{len(cases)} cases, {len(blob)/1024:.0f} KiB packed")
print(f"  genuinely compressed: {comp_n}   multi-block: {multi}")

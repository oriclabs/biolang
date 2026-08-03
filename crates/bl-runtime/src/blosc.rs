//! Pure-Rust decompressor for the Blosc1 container format.
//!
//! Blosc is the default compressor for Zarr v2 stores, and therefore for
//! AnnData `.zarr` written by scanpy/anndata with default settings. Without it
//! [`crate::anndata_zarr`] can only read stores that were deliberately re-saved
//! with a plain gzip codec.
//!
//! Layout of a compressed buffer:
//!
//! ```text
//!   16-byte header
//!     0      version
//!     1      versionlz
//!     2      flags      bit0 shuffle, bit1 memcpyed, bit2 bitshuffle,
//!                       bits5-7 inner codec id
//!     3      typesize
//!     4..8   nbytes     uncompressed size
//!     8..12  blocksize
//!     12..16 cbytes     total compressed size
//!   nblocks x i32 LE    offset of each block's payload
//!   ... per-block payloads ...
//! ```
//!
//! Two behaviours here are not obvious from the format description and were
//! established empirically against 540 conformance vectors produced by
//! numcodecs (see `tests/blosc_conformance.rs`):
//!
//! 1. **Block offsets are not sorted.** Blosc compresses blocks in parallel and
//!    writes them in thread-completion order, so a block's payload extent
//!    cannot be derived from the next entry in the offset table.
//! 2. **Bitshuffle is skipped when a block's element count is not a multiple
//!    of 8.** Such a block is stored unshuffled even though the header still
//!    advertises the bitshuffle flag.

use std::io::Read;

use bl_core::error::{BioLangError, ErrorKind, Result};

const FLAG_SHUFFLE: u8 = 0x01;
const FLAG_MEMCPYED: u8 = 0x02;
const FLAG_BITSHUFFLE: u8 = 0x04;

const CODEC_BLOSCLZ: u8 = 0;
const CODEC_LZ4: u8 = 1;
const CODEC_SNAPPY: u8 = 2;
const CODEC_ZLIB: u8 = 3;
const CODEC_ZSTD: u8 = 4;

/// Minimum sub-buffer size for a block to be split into per-byte streams.
/// Matches c-blosc's `MIN_BUFFERSIZE`.
const MIN_SPLIT_BUFFERSIZE: usize = 128;

pub const HEADER_LEN: usize = 16;

fn err(msg: impl Into<String>) -> BioLangError {
    BioLangError::runtime(ErrorKind::IOError, format!("blosc: {}", msg.into()), None)
}

/// True if `src` looks like a Blosc1 buffer (used to auto-detect chunks whose
/// `.zarray` metadata is missing or lies about the compressor).
pub fn is_blosc(src: &[u8]) -> bool {
    if src.len() < HEADER_LEN {
        return false;
    }
    let version = src[0];
    let codec = (src[2] >> 5) & 0x07;
    (1..=2).contains(&version) && codec <= CODEC_ZSTD
}

fn u32_at(src: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([src[at], src[at + 1], src[at + 2], src[at + 3]])
}

fn i32_at(src: &[u8], at: usize) -> i32 {
    i32::from_le_bytes([src[at], src[at + 1], src[at + 2], src[at + 3]])
}

/// Whether blosc splits a block into `typesize` independently-compressed
/// streams. Derived from the conformance vectors: zstd never splits; every
/// other codec splits once each stream would be at least `MIN_SPLIT_BUFFERSIZE`
/// bytes.
fn splits(codec: u8, typesize: usize, blocksize: usize) -> bool {
    codec != CODEC_ZSTD && typesize > 1 && blocksize / typesize >= MIN_SPLIT_BUFFERSIZE
}

/// Decompress a complete Blosc1 buffer.
pub fn decompress(src: &[u8]) -> Result<Vec<u8>> {
    if src.len() < HEADER_LEN {
        return Err(err(format!("buffer too short ({} bytes)", src.len())));
    }
    let flags = src[2];
    let typesize = src[3] as usize;
    let nbytes = u32_at(src, 4) as usize;
    let blocksize = u32_at(src, 8) as usize;
    let cbytes = u32_at(src, 12) as usize;

    if cbytes > src.len() {
        return Err(err(format!(
            "header claims {cbytes} compressed bytes but only {} are present",
            src.len()
        )));
    }
    if nbytes == 0 {
        return Ok(Vec::new());
    }

    // A memcpyed buffer stores the payload verbatim, with no shuffle applied,
    // regardless of what the shuffle flags say.
    if flags & FLAG_MEMCPYED != 0 {
        let start = HEADER_LEN;
        let end = start
            .checked_add(nbytes)
            .filter(|e| *e <= src.len())
            .ok_or_else(|| err("memcpyed payload runs past end of buffer"))?;
        return Ok(src[start..end].to_vec());
    }

    if blocksize == 0 {
        return Err(err("blocksize is zero"));
    }
    let codec = (flags >> 5) & 0x07;
    let nblocks = nbytes.div_ceil(blocksize);
    let table_end = HEADER_LEN + 4 * nblocks;
    if table_end > src.len() {
        return Err(err("block offset table runs past end of buffer"));
    }

    let offsets: Vec<usize> = (0..nblocks)
        .map(|i| {
            let o = i32_at(src, HEADER_LEN + 4 * i);
            if o < 0 || (o as usize) >= src.len() {
                Err(err(format!("block {i} has out-of-range offset {o}")))
            } else {
                Ok(o as usize)
            }
        })
        .collect::<Result<_>>()?;

    // Offsets are in thread-completion order, so a block's payload ends at the
    // next-larger offset anywhere in the table (or at `cbytes` for the last).
    let mut sorted = offsets.clone();
    sorted.sort_unstable();

    let mut out = vec![0u8; nbytes];
    let mut block = Vec::new();

    for (i, &start) in offsets.iter().enumerate() {
        let bsize = blocksize.min(nbytes - i * blocksize);
        let end = sorted
            .iter()
            .copied()
            .find(|&o| o > start)
            .unwrap_or(cbytes)
            .min(src.len());
        if end < start {
            return Err(err(format!("block {i} has inverted extent")));
        }

        block.clear();
        decode_block(
            &src[start..end],
            codec,
            typesize,
            blocksize,
            bsize,
            &mut block,
        )?;
        if block.len() != bsize {
            return Err(err(format!(
                "block {i} decoded to {} bytes, expected {bsize}",
                block.len()
            )));
        }

        let dst = i * blocksize;
        unshuffle_into(flags, typesize, &block, &mut out[dst..dst + bsize]);
    }

    Ok(out)
}

/// Decode one block's payload (still shuffled) into `out`.
///
/// The number of streams is predicted by [`splits`], but the prediction is
/// verified against the payload extent and retried with the alternative before
/// failing — the split rule is only established for the codecs and typesizes
/// covered by the conformance vectors, and a mispredicted split would otherwise
/// silently corrupt data.
fn decode_block(
    payload: &[u8],
    codec: u8,
    typesize: usize,
    blocksize: usize,
    bsize: usize,
    out: &mut Vec<u8>,
) -> Result<()> {
    let predicted = if splits(codec, typesize, blocksize) {
        typesize
    } else {
        1
    };
    let alternative = if predicted == 1 { typesize } else { 1 };

    for nstreams in [predicted, alternative] {
        if nstreams == 0 || bsize % nstreams != 0 {
            continue;
        }
        out.clear();
        if decode_streams(payload, codec, nstreams, bsize, out).is_ok() && out.len() == bsize {
            return Ok(());
        }
        if nstreams == alternative {
            break;
        }
    }
    Err(err(format!(
        "could not decode block ({} payload bytes, bsize {bsize}, typesize {typesize})",
        payload.len()
    )))
}

fn decode_streams(
    payload: &[u8],
    codec: u8,
    nstreams: usize,
    bsize: usize,
    out: &mut Vec<u8>,
) -> Result<()> {
    let neblock = bsize / nstreams;
    let mut p = 0usize;
    for _ in 0..nstreams {
        if p + 4 > payload.len() {
            return Err(err("truncated stream length"));
        }
        let clen = i32_at(payload, p);
        p += 4;
        if clen < 0 {
            return Err(err("negative stream length"));
        }
        let clen = clen as usize;
        if p + clen > payload.len() {
            return Err(err("stream runs past block payload"));
        }
        let chunk = &payload[p..p + clen];
        p += clen;

        if clen == neblock {
            // incompressible stream, stored verbatim
            out.extend_from_slice(chunk);
        } else {
            let before = out.len();
            decode_codec(codec, chunk, neblock, out)?;
            if out.len() - before != neblock {
                return Err(err("stream decoded to unexpected length"));
            }
        }
    }
    Ok(())
}

fn decode_codec(codec: u8, src: &[u8], expect: usize, out: &mut Vec<u8>) -> Result<()> {
    match codec {
        CODEC_BLOSCLZ => {
            let mut buf = vec![0u8; expect];
            let n = blosclz_decompress(src, &mut buf)?;
            if n != expect {
                return Err(err(format!(
                    "blosclz produced {n} bytes, expected {expect}"
                )));
            }
            out.extend_from_slice(&buf);
            Ok(())
        }
        CODEC_LZ4 => {
            let mut buf = vec![0u8; expect];
            let n = lz4_flex::block::decompress_into(src, &mut buf)
                .map_err(|e| err(format!("lz4: {e}")))?;
            if n != expect {
                return Err(err(format!("lz4 produced {n} bytes, expected {expect}")));
            }
            out.extend_from_slice(&buf);
            Ok(())
        }
        CODEC_ZLIB => {
            let mut d = flate2::read::ZlibDecoder::new(src);
            d.read_to_end(out).map_err(|e| err(format!("zlib: {e}")))?;
            Ok(())
        }
        CODEC_ZSTD => {
            let mut d =
                ruzstd::StreamingDecoder::new(src).map_err(|e| err(format!("zstd: {e}")))?;
            d.read_to_end(out).map_err(|e| err(format!("zstd: {e}")))?;
            Ok(())
        }
        CODEC_SNAPPY => Err(err(
            "snappy-compressed blosc chunks are not supported; re-save the store \
             with cname='lz4', 'zstd', 'zlib' or 'blosclz'",
        )),
        other => Err(err(format!("unknown inner codec id {other}"))),
    }
}

// ── shuffle filters ──────────────────────────────────────────────────────────

fn unshuffle_into(flags: u8, typesize: usize, block: &[u8], dst: &mut [u8]) {
    let bsize = block.len();
    if typesize <= 1 || bsize < typesize {
        dst.copy_from_slice(block);
        return;
    }
    if flags & FLAG_BITSHUFFLE != 0 {
        unbitshuffle_into(typesize, block, dst);
    } else if flags & FLAG_SHUFFLE != 0 {
        unshuffle_bytes_into(typesize, block, dst);
    } else {
        dst.copy_from_slice(block);
    }
}

/// Undo blosc's byte shuffle: the shuffled block holds every element's byte 0,
/// then every element's byte 1, and so on, with any trailing bytes that do not
/// fill a whole element left verbatim.
fn unshuffle_bytes_into(typesize: usize, src: &[u8], dst: &mut [u8]) {
    let bsize = src.len();
    let nelem = bsize / typesize;
    for j in 0..typesize {
        let plane = j * nelem;
        for i in 0..nelem {
            dst[i * typesize + j] = src[plane + i];
        }
    }
    let tail = nelem * typesize;
    if tail < bsize {
        dst[tail..].copy_from_slice(&src[tail..]);
    }
}

/// Undo blosc's bit shuffle, a transpose of the (nelem x typesize*8) bit matrix
/// with LSB-first bit order within each byte.
///
/// Blosc only bitshuffles when the block holds a whole multiple of 8 elements;
/// otherwise the block is stored unshuffled despite the header flag.
fn unbitshuffle_into(typesize: usize, src: &[u8], dst: &mut [u8]) {
    let bsize = src.len();
    let nelem = bsize / typesize;
    if nelem == 0 || nelem % 8 != 0 || nelem * typesize != bsize {
        dst.copy_from_slice(src);
        return;
    }
    let nbits = typesize * 8;
    dst.fill(0);
    // shuffled bit index (bit, elem) -> natural bit index (elem, bit)
    for bit in 0..nbits {
        let row = bit * nelem;
        for e in 0..nelem {
            let s = row + e;
            if src[s >> 3] >> (s & 7) & 1 != 0 {
                let d = e * nbits + bit;
                dst[d >> 3] |= 1 << (d & 7);
            }
        }
    }
}

// ── blosclz ──────────────────────────────────────────────────────────────────

/// Decompress a blosclz stream (a FastLZ level-1 derivative) into `dst`.
/// Returns the number of bytes written.
fn blosclz_decompress(src: &[u8], dst: &mut [u8]) -> Result<usize> {
    let mut ip = 0usize;
    let mut op = 0usize;
    if src.is_empty() {
        return Ok(0);
    }
    let mut ctrl = (src[ip] & 31) as usize;
    ip += 1;

    loop {
        if ctrl >= 32 {
            // back-reference
            let mut len = (ctrl >> 5) - 1;
            let mut ofs = (ctrl & 31) << 8;
            if len == 6 {
                loop {
                    if ip >= src.len() {
                        return Err(err("blosclz: truncated length"));
                    }
                    let code = src[ip];
                    ip += 1;
                    len += code as usize;
                    if code != 255 {
                        break;
                    }
                }
            }
            if ip >= src.len() {
                return Err(err("blosclz: truncated offset"));
            }
            let code = src[ip];
            ip += 1;
            ofs += code as usize;
            if code == 255 && (ctrl & 31) == 31 {
                if ip + 1 >= src.len() {
                    return Err(err("blosclz: truncated long offset"));
                }
                ofs = ((src[ip] as usize) << 8) + src[ip + 1] as usize;
                ip += 2;
                ofs += 8191;
            }
            len += 3;
            if ofs + 1 > op {
                return Err(err("blosclz: back-reference before start of output"));
            }
            let mut r = op - ofs - 1;
            if op + len > dst.len() {
                return Err(err("blosclz: output overflow"));
            }
            // byte-by-byte: references may overlap the region being written
            for _ in 0..len {
                dst[op] = dst[r];
                op += 1;
                r += 1;
            }
        } else {
            // literal run
            let n = ctrl + 1;
            if ip + n > src.len() || op + n > dst.len() {
                return Err(err("blosclz: literal run overflow"));
            }
            dst[op..op + n].copy_from_slice(&src[ip..ip + n]);
            ip += n;
            op += n;
        }

        if ip >= src.len() {
            break;
        }
        ctrl = src[ip] as usize;
        ip += 1;
    }

    Ok(op)
}

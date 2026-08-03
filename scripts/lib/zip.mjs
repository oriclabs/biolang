/**
 * Minimal ZIP writer.
 *
 * Node ships `zlib.deflateRawSync` and `zlib.crc32`, which are exactly the two
 * primitives a ZIP needs, so a real deflate-compressed archive costs a page of
 * code and no dependency. Written here rather than pulled in because the pack
 * artifacts are the only thing in this repository that needs one.
 *
 * Deliberately supports only what pack downloads require: no directories, no
 * ZIP64, no encryption. Everything is stored with a fixed timestamp so a
 * rebuild of unchanged sources produces an identical archive — the docs
 * pipeline is checked for staleness by comparing generated output, and an
 * archive that changed every run would defeat that.
 */

import { crc32, deflateRawSync } from "node:zlib";

const SIGNATURE_LOCAL = 0x04034b50;
const SIGNATURE_CENTRAL = 0x02014b50;
const SIGNATURE_END = 0x06054b50;
// MS-DOS time/date for 1980-01-01 00:00, the earliest the format can express.
const DOS_TIME = 0;
const DOS_DATE = 33;

/**
 * Build a ZIP from `[name, contents]` entries.
 *
 * @param {Array<[string, string | Buffer]>} entries
 * @returns {Buffer}
 */
export function createZip(entries) {
  const locals = [];
  const centrals = [];
  let offset = 0;

  for (const [name, contents] of entries) {
    const nameBytes = Buffer.from(name, "utf8");
    const data = Buffer.isBuffer(contents) ? contents : Buffer.from(contents, "utf8");
    const compressed = deflateRawSync(data);
    // Deflate can be larger than the input for tiny or random files; store the
    // raw bytes when that happens rather than paying to make the file bigger.
    const useDeflate = compressed.length < data.length;
    const payload = useDeflate ? compressed : data;
    const method = useDeflate ? 8 : 0;
    const checksum = crc32(data);

    const local = Buffer.alloc(30);
    local.writeUInt32LE(SIGNATURE_LOCAL, 0);
    local.writeUInt16LE(20, 4); // version needed
    local.writeUInt16LE(0, 6); // flags
    local.writeUInt16LE(method, 8);
    local.writeUInt16LE(DOS_TIME, 10);
    local.writeUInt16LE(DOS_DATE, 12);
    local.writeUInt32LE(checksum, 14);
    local.writeUInt32LE(payload.length, 18);
    local.writeUInt32LE(data.length, 22);
    local.writeUInt16LE(nameBytes.length, 26);
    local.writeUInt16LE(0, 28); // extra field length
    locals.push(local, nameBytes, payload);

    const central = Buffer.alloc(46);
    central.writeUInt32LE(SIGNATURE_CENTRAL, 0);
    central.writeUInt16LE(20, 4); // version made by
    central.writeUInt16LE(20, 6); // version needed
    central.writeUInt16LE(0, 8); // flags
    central.writeUInt16LE(method, 10);
    central.writeUInt16LE(DOS_TIME, 12);
    central.writeUInt16LE(DOS_DATE, 14);
    central.writeUInt32LE(checksum, 16);
    central.writeUInt32LE(payload.length, 20);
    central.writeUInt32LE(data.length, 24);
    central.writeUInt16LE(nameBytes.length, 28);
    central.writeUInt16LE(0, 30); // extra
    central.writeUInt16LE(0, 32); // comment
    central.writeUInt16LE(0, 34); // disk number
    central.writeUInt16LE(0, 36); // internal attributes
    central.writeUInt32LE(0, 38); // external attributes
    central.writeUInt32LE(offset, 42);
    centrals.push(central, nameBytes);

    offset += local.length + nameBytes.length + payload.length;
  }

  const centralBuffer = Buffer.concat(centrals);
  const end = Buffer.alloc(22);
  end.writeUInt32LE(SIGNATURE_END, 0);
  end.writeUInt16LE(0, 4); // disk number
  end.writeUInt16LE(0, 6); // disk with central directory
  end.writeUInt16LE(entries.length, 8);
  end.writeUInt16LE(entries.length, 10);
  end.writeUInt32LE(centralBuffer.length, 12);
  end.writeUInt32LE(offset, 16);
  end.writeUInt16LE(0, 20); // comment length

  return Buffer.concat([...locals, centralBuffer, end]);
}

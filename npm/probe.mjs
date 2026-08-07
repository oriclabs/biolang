globalThis.window = globalThis;
globalThis.__blFetch = { sync: () => "ERROR:no" };
const w = await import("./pkg-node/bl_wasm.js");
w.init();
const b = JSON.parse(w.list_builtins()).map(x => x.name);
console.log("total builtins in wasm:", b.length);
for (const p of ["ensembl", "ncbi", "uniprot", "kegg", "pdb"])
  console.log("  " + p.padEnd(9), b.filter(n => n.startsWith(p)).join(", ") || "(none)");

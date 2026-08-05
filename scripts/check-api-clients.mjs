#!/usr/bin/env node
/**
 * Contract test for every API client, against the real services.
 *
 * Background: bl-apis has 69 unit tests and all of them feed hand-written JSON
 * to a serde roundtrip. That shape of test cannot fail when an upstream API
 * changes, which is the only way these clients actually break. Two examples
 * found within a week of each other:
 *
 *   - BioContainers moved to Quay; the mocked tests stayed green while every
 *     real call returned nothing.
 *   - The Galaxy ToolShed started answering `?q=` with a search envelope
 *     instead of a bare array, and named the owner `repo_owner_username`.
 *     `galaxy_search()` failed outright with "invalid type: map, expected a
 *     sequence" and no test noticed.
 *
 * So each case here calls a real endpoint and asserts on fields a caller would
 * actually read. The assertions live in BioLang so they exercise the same path
 * a user's script takes, builtin included, not just the Rust client.
 *
 * A remote service being down is not a failure of this repository, so outages
 * and missing credentials are reported as SKIP and do not affect the exit code.
 * Only a response that parses into the wrong shape — or fails to parse — fails
 * the run.
 *
 * Usage:
 *   node scripts/check-api-clients.mjs                 # all clients
 *   node scripts/check-api-clients.mjs --only galaxy   # substring filter
 *   node scripts/check-api-clients.mjs --timeout 90000
 *   node scripts/check-api-clients.mjs --json          # machine-readable
 */

import { execFile } from "node:child_process";
import { mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { promisify } from "node:util";

const execFileAsync = promisify(execFile);
const repositoryRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const exe = process.platform === "win32" ? "bl.exe" : "bl";
const cli =
  process.env.BIOLANG_CLI ?? path.join(repositoryRoot, "target", "release", exe);

const argv = process.argv.slice(2);
const flag = (name, fallback) => {
  const i = argv.indexOf(name);
  return i >= 0 ? argv[i + 1] : fallback;
};
const timeout = Number(flag("--timeout", 60_000));
const only = flag("--only", null);
const asJson = argv.includes("--json");

/**
 * One case per client. `code` must assert on fields a caller reads — a call
 * that only checks "no error" would still pass if every field came back empty,
 * which is exactly how the BioContainers breakage hid.
 */
const CASES = [
  {
    client: "ncbi",
    code: `
      let ids = ncbi_search("gene", "TP53")
      assert len(ids) > 0, "ncbi_search returned no ids"
    `,
  },
  {
    client: "ncbi_datasets",
    code: `
      let genes = datasets_gene("TP53")
      assert len(genes) > 0, "datasets_gene returned nothing"
      let g = genes |> first()
      assert g.symbol == "TP53", "expected symbol TP53, got " + g.symbol
      assert g.gene_id != nil, "gene_id missing"
    `,
  },
  {
    client: "ensembl",
    code: `
      let gene = ensembl_symbol("human", "TP53")
      assert gene.id == "ENSG00000141510", "unexpected gene id: " + gene.id
      assert gene.chromosome == "17", "unexpected chromosome: " + gene.chromosome
      # A gene ID is rejected by /sequence/id for cds and protein, so callers
      # need the canonical transcript to be populated.
      assert starts_with(gene.canonical_transcript, "ENST"), "canonical_transcript missing: " + gene.canonical_transcript
    `,
  },
  {
    client: "uniprot",
    code: `
      let entry = uniprot_entry("P04637")
      assert entry.accession == "P04637", "unexpected accession: " + entry.accession
      assert len(entry.organism) > 0, "organism empty"
      assert len(entry.name) > 0, "name empty"
    `,
  },
  {
    client: "pdb",
    code: `
      let e = pdb_entry("1TUP")
      assert len(e.title) > 0, "title empty"
      assert len(e.method) > 0, "method empty"
    `,
  },
  {
    client: "kegg",
    code: `
      let entry = kegg_get("hsa:7157")
      assert contains(entry, "TP53"), "kegg entry does not mention TP53"
    `,
  },
  {
    client: "go",
    code: `
      let term = go_term("GO:0006915")
      assert term.id == "GO:0006915", "unexpected id: " + term.id
      assert len(term.definition) > 0, "definition empty"
    `,
  },
  {
    client: "reactome",
    code: `
      let hits = reactome_search("apoptosis")
      assert len(hits) > 0, "no reactome hits"
      let first = hits |> first()
      assert starts_with(first.id, "R-"), "unexpected pathway id: " + first.id
      # The search endpoint marks up matched terms; a caller printing 'name'
      # should not get a <span> in it.
      assert not contains(first.name, "<"), "name contains markup: " + first.name
    `,
  },
  {
    client: "string_db",
    code: `
      let edges = string_network(["TP53", "MDM2"], 9606)
      assert len(edges) > 0, "no interactions returned"
      let e = edges |> first()
      assert e.score > 0.0, "score not populated"
      assert len(e.protein_a) > 0, "protein_a empty"
    `,
  },
  {
    client: "ucsc",
    code: `
      let genomes = ucsc_genomes()
      assert len(genomes) > 0, "no genomes returned"
      assert len((genomes |> first()).name) > 0, "genome name empty"
    `,
  },
  {
    client: "nfcore",
    code: `
      let pipelines = nfcore_list()
      assert len(pipelines) > 0, "no pipelines returned"
      let p = pipelines |> first()
      assert len(p.name) > 0, "pipeline name empty"
      assert len(p.description) > 0, "pipeline description empty"
    `,
  },
  {
    client: "biocontainers",
    code: `
      let hits = biocontainers_search("bwa")
      assert len(hits) > 0, "no biocontainers hits"
      let h = hits |> first()
      assert len(h.name) > 0, "name empty"
      assert h.organization == "biocontainers", "unexpected org: " + h.organization
    `,
  },
  {
    client: "galaxy",
    code: `
      # Regression: the search endpoint answers with a {hits: [{repository}]}
      # envelope, not a bare array, and names the owner repo_owner_username.
      let hits = galaxy_search("bwa")
      assert len(hits) > 0, "no toolshed hits"
      let h = hits |> first()
      assert len(h.name) > 0, "name empty"
      assert len(h.owner) > 0, "owner empty - check the repo_owner_username alias"
      assert h.downloads > 0, "downloads not populated"
    `,
  },
  {
    client: "clinvar",
    code: `
      let variants = clinvar_gene("BRCA1")
      assert len(variants) > 0, "no clinvar variants"
      assert len((variants |> first()).variation_name) > 0, "variation_name empty"
    `,
  },
  {
    client: "geo",
    code: `
      let hits = geo_search("cancer")
      assert len(hits) > 0, "no geo hits"
      assert len((hits |> first()).title) > 0, "title empty"
    `,
  },
  {
    client: "cbioportal",
    code: `
      let studies = cbio_studies()
      assert len(studies) > 0, "no studies returned"
      let s = studies |> first()
      assert len(s.study_id) > 0, "study_id empty"
      assert s.n_samples > 0, "n_samples not populated"
    `,
  },
  {
    client: "opentargets",
    code: `
      let target = ot_target("ENSG00000141510")
      assert target.approved_symbol == "TP53", "unexpected symbol: " + target.approved_symbol
      assert len(target.biotype) > 0, "biotype empty"
    `,
  },
  {
    client: "pubmed",
    code: `
      let result = pubmed_search("CRISPR", 2)
      assert result.count > 0, "count not populated"
      assert len(result.ids) == 2, "expected 2 ids, got " + len(result.ids)
    `,
  },
  {
    client: "gtex",
    code: `
      let tissues = gtex_tissues()
      assert tissues != nil, "gtex_tissues() returned nil"
      assert len(tissues) > 0, "no tissues returned"
    `,
  },
  {
    client: "gnomad",
    code: `
      let gene = gnomad_gene("BRCA1")
      assert gene != nil, "gnomad_gene returned nil"
    `,
  },
  {
    client: "biomart",
    code: `
      let rows = biomart_query("hsapiens_gene_ensembl",
                               ["ensembl_gene_id", "external_gene_name"],
                               ["chromosome_name", "17"])
      assert len(rows) > 0, "biomart returned no rows"
    `,
  },
  {
    client: "cosmic",
    // Needs COSMIC_API_KEY; reported as SKIP when absent.
    code: `
      let gene = cosmic_gene("TP53")
      assert gene != nil, "cosmic_gene returned nil"
    `,
  },
];

/** An outage or a missing key is not this repository being wrong. */
function classify(output) {
  const text = output.toLowerCase();
  if (/api_key|api key|auth error|unauthorized|forbidden|401|403/.test(text)) {
    return "skip-credentials";
  }
  if (
    /network error|timed out|timeout|connection|dns|temporarily unavailable|http 5\d\d|502|503|504|econn|enotfound/.test(
      text,
    )
  ) {
    return "skip-network";
  }
  return "fail";
}

function firstMeaningfulLine(output) {
  const lines = output
    .split(/\r?\n/)
    .map((l) => l.replace(/\[[0-9;]*m/g, "").trim())
    .filter((l) => l && !/^[▶✓]/.test(l) && !/running /i.test(l));
  return lines[0] ?? "(no output)";
}

async function runCase(directory, testCase) {
  const file = path.join(directory, `${testCase.client}.bl`);
  await writeFile(file, testCase.code.trim() + "\n", "utf8");
  try {
    await execFileAsync(cli, ["run", file], { timeout, maxBuffer: 64 * 1024 * 1024 });
    return { client: testCase.client, status: "ok", detail: "" };
  } catch (error) {
    const output = `${error.stdout ?? ""}\n${error.stderr ?? ""}`.trim();
    const status = error.killed ? "skip-network" : classify(output);
    return {
      client: testCase.client,
      status,
      detail: error.killed ? `timed out after ${timeout}ms` : firstMeaningfulLine(output),
    };
  }
}

const selected = only
  ? CASES.filter((c) => c.client.includes(only))
  : CASES;

if (selected.length === 0) {
  console.error(`No client matches "${only}".`);
  process.exit(2);
}

const directory = await mkdtemp(path.join(tmpdir(), "bl-api-"));
let results;
try {
  // Sequential on purpose: several of these hit the same host, and hammering a
  // public API in parallel is how a contract test turns into rate-limit noise.
  results = [];
  for (const testCase of selected) {
    results.push(await runCase(directory, testCase));
  }
} finally {
  await rm(directory, { recursive: true, force: true });
}

const failed = results.filter((r) => r.status === "fail");
const skipped = results.filter((r) => r.status.startsWith("skip"));
const passed = results.filter((r) => r.status === "ok");

if (asJson) {
  console.log(JSON.stringify({ passed: passed.length, failed, skipped }, null, 2));
} else {
  for (const r of results) {
    const label =
      r.status === "ok" ? "ok  " : r.status === "fail" ? "FAIL" : "skip";
    const suffix = r.detail ? `  ${r.detail}` : "";
    const reason =
      r.status === "skip-credentials"
        ? " (credentials)"
        : r.status === "skip-network"
          ? " (network)"
          : "";
    console.log(`  ${label}  ${r.client}${reason}${suffix.slice(0, 160)}`);
  }
  console.log(
    `\n${passed.length} of ${selected.length} API clients returned the expected shape ` +
      `(${failed.length} wrong shape, ${skipped.length} skipped)`,
  );
}

process.exit(failed.length > 0 ? 1 : 0);

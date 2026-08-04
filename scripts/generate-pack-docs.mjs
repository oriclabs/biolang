#!/usr/bin/env node
/**
 * Generate a docs page per example pack from its manifest.
 *
 * The hand-written `website/docs/examples/rosalind-armory.html` carried its own
 * copy of every solution — a second variant of code that already exists in the
 * repository, and the one a reader sees is not the one `bl test` asserts. This
 * renders the page from `pack.toml` and the real sources, so the page cannot
 * show something the test suite does not check.
 *
 * The coverage table is generated too, including the `blocked_on` reasons, so
 * "partial" is visible to a reader rather than buried in a manifest.
 *
 * Usage: node scripts/generate-pack-docs.mjs [--out website/docs/examples]
 */

import { mkdir, readFile, writeFile } from "node:fs/promises";
import path from "node:path";
import { listPackIds, packCounts, readPack, repositoryRoot } from "./lib/pack-manifest.mjs";
import { createZip } from "./lib/zip.mjs";

const outIndex = process.argv.indexOf("--out");
const outputRoot = path.resolve(
  repositoryRoot,
  outIndex >= 0 && process.argv[outIndex + 1]
    ? process.argv[outIndex + 1]
    : path.join("website", "docs", "examples"),
);

const escape = (text) =>
  String(text ?? "")
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;");

const STATUS_STYLE = {
  solved: "bg-emerald-500/10 text-emerald-300 border-emerald-500/30",
  partial: "bg-amber-500/10 text-amber-300 border-amber-500/30",
  blocked: "bg-rose-500/10 text-rose-300 border-rose-500/30",
};

function badge(status) {
  const style = STATUS_STYLE[status] ?? STATUS_STYLE.blocked;
  return `<span class="inline-block px-2 py-0.5 text-xs rounded border ${style}">${escape(status)}</span>`;
}

/**
 * Which sub-page a problem belongs on.
 *
 * The first entry of the manifest's `topics` list. Grouping needs no new
 * metadata as a result, and the groups come out at a workable size: seven per
 * pack today, around fifteen problems each once the Stronghold is complete.
 */
function groupOf(problem) {
  return (problem.topics ?? ["other"])[0];
}

function groupsOf(manifest) {
  const groups = new Map();
  for (const problem of manifest.problem) {
    const key = groupOf(problem);
    groups.set(key, [...(groups.get(key) ?? []), problem]);
  }
  return groups;
}

const groupTitle = (group) =>
  group.replaceAll("-", " ").replace(/^./, (c) => c.toUpperCase());

/**
 * Builtins the browser build ships, or null when the catalog is not present.
 *
 * `website/wasm/builtins.json` is generated from the compiled module, so it is
 * the same source `tests/run_pack_wasm.mjs` checks against. When it is absent —
 * a checkout that has not built WASM — the column is dropped rather than
 * guessed at.
 */
async function browserBuiltins() {
  const catalog = path.join(repositoryRoot, "website", "wasm", "builtins.json");
  const raw = await readFile(catalog, "utf8").catch(() => "");
  if (!raw) return null;
  return new Set(JSON.parse(raw).builtins ?? []);
}

const CALL_KEYWORDS = new Set([
  "if", "for", "while", "fn", "let", "return", "else", "then", "and", "or",
  "not", "try", "catch", "in", "assert", "import", "true", "false",
  "dna", "rna", "protein",
]);

/** Builtins a source calls that the browser build does not have. */
/**
 * Blank out comments and string contents in one pass, tracking which of the two
 * we are inside.
 *
 * Two passes — comments first, then strings — breaks on a string containing a
 * hash: a + "#" + b becomes a + ", leaving an unbalanced quote that makes the
 * string stripper mis-pair every quote after it and expose later string
 * contents as code. Must match blStripNonCode in website/js/playground.js.
 */
function stripNonCode(code) {
  const NEWLINE = 10;
  const QUOTE = 34;
  const HASH = 35;
  const BACKSLASH = 92;
  let out = "";
  let inString = false;
  for (let i = 0; i < code.length; i += 1) {
    const ch = code.charCodeAt(i);
    if (inString) {
      if (ch === BACKSLASH) { out += "  "; i += 1; continue; }
      if (ch === QUOTE) { inString = false; out += code[i]; continue; }
      out += ch === NEWLINE ? code[i] : " ";
      continue;
    }
    if (ch === QUOTE) { inString = true; out += code[i]; continue; }
    if (ch === HASH) {
      while (i < code.length && code.charCodeAt(i) !== NEWLINE) i += 1;
      out += code[i] === undefined ? "" : code[i];
      continue;
    }
    out += code[i];
  }
  return out;
}

function missingInBrowser(source, available) {
  if (!available) return [];
  const stripped = stripNonCode(source);
  // Both ways of naming a function: `fn name(...)` and a lambda bound with
  // `let name = |...|`. Missing the second form reported every such helper as a
  // builtin the browser lacks, which marked working examples "CLI only".
  const declared = new Set([
    ...[...stripped.matchAll(/\bfn\s+([A-Za-z_]\w*)/g)].map((m) => m[1]),
    ...[...stripped.matchAll(/\blet\s+([A-Za-z_]\w*)\s*=\s*\|/g)].map((m) => m[1]),
  ]);
  const called = new Set(
    [...stripped.matchAll(/\b([a-z_][a-z0-9_]*)\s*\(/g)].map((m) => m[1]),
  );
  return [...called].filter(
    (name) => !CALL_KEYWORDS.has(name) && !declared.has(name) && !available.has(name),
  );
}

/**
 * Where a problem can run, shown beside its status on the section page.
 *
 * The coverage table carries this too, but a reader arriving at a problem
 * directly — from a deep link, or by scrolling — never sees the table, and
 * "Download .bl" alone reads as though the CLI is the only option.
 */
function runsBadge(problem, source, available) {
  if (!available) return "";
  const missing = missingInBrowser(source, available);
  if (missing.length > 0) {
    return `<span class="inline-block px-2 py-0.5 text-xs rounded border bg-amber-500/10 text-amber-300 border-amber-500/30" title="Needs ${escape(missing.join(", "))}, which the browser build does not have">CLI only</span>`;
  }
  const note = problem.network ? "runs in your browser, but calls a remote service" : "runs in your browser and from the CLI";
  const label = problem.network ? "browser + CLI · online" : "browser + CLI";
  return `<span class="inline-block px-2 py-0.5 text-xs rounded border bg-sky-500/10 text-sky-300 border-sky-500/30" title="${escape(note)}">${label}</span>`;
}

function coverageTable(manifest, packId, sources, available) {
  const rows = manifest.problem
    .map((problem) => {
      const checked = problem.asserted
        ? "verified by <code>bl test</code>"
        : problem.network
          ? "needs NCBI"
          : "not asserted";
      const href = `${packId}/${groupOf(problem)}.html#${problem.id.toLowerCase()}`;

      let runsIn = "";
      if (available) {
        const missing = missingInBrowser(sources.get(problem.id) ?? "", available);
        if (missing.length > 0) {
          runsIn = `<span class="text-amber-300/80">CLI only</span> <span class="text-slate-600">— needs ${escape(missing.join(", "))}</span>`;
        } else if (problem.network) {
          runsIn = 'browser + CLI <span class="text-slate-600">— online</span>';
        } else {
          runsIn = "browser + CLI";
        }
        runsIn = `\n            <td class="py-2 pr-4 text-slate-500">${runsIn}</td>`;
      }

      // The whole row is a target, not just the id: a bare table gives no hint
      // that anything is clickable. `data-row-href` is picked up by main.js for
      // a plain left click; the anchor remains the accessible route, so
      // keyboard focus and open-in-new-tab are unaffected.
      return `          <tr data-row-href="${escape(href)}" class="group border-t border-slate-800 cursor-pointer hover:bg-slate-900/60 transition-colors">
            <td class="py-2 pr-4"><a href="${escape(href)}" class="text-violet-400 group-hover:text-violet-300">${escape(problem.id)}</a></td>
            <td class="py-2 pr-4 text-slate-400">${escape(problem.title)}</td>
            <td class="py-2 pr-4">${badge(problem.status)}</td>
            <td class="py-2 pr-4 text-slate-500">${escape(groupTitle(groupOf(problem)))}</td>${runsIn}
            <td class="py-2 text-slate-500">${checked}</td>
          </tr>`;
    })
    .join("\n");

  return `        <table class="w-full text-sm my-6 not-prose">
          <thead><tr class="text-left text-slate-500">
            <th class="pb-2 pr-4 font-medium">Problem</th>
            <th class="pb-2 pr-4 font-medium">Title</th>
            <th class="pb-2 pr-4 font-medium">Status</th>
            <th class="pb-2 pr-4 font-medium">Section</th>${
              available ? '\n            <th class="pb-2 pr-4 font-medium">Runs in</th>' : ""
            }
            <th class="pb-2 font-medium">Checked</th>
          </tr></thead>
          <tbody>
${rows}
          </tbody>
        </table>`;
}

/**
 * The page as a BioLang notebook.
 *
 * A `.bln` is YAML frontmatter, markdown, and ```biolang blocks — the same
 * shape as the page itself — so the whole page converts rather than being
 * summarised. `bl notebook` runs it, and `--export`/`--to-ipynb` turn it into
 * HTML, PDF or a Jupyter file from there.
 */
async function renderNotebook(pack, sources) {
  const { manifest, packId } = pack;
  const parts = [
    "---",
    `title: ${manifest.pack.name}`,
    // The pack version, not today's date: this file is committed and CI fails
    // if regenerating it produces a diff, so nothing here may change with the
    // clock.
    `version: ${manifest.pack.version}`,
    `abstract: ${manifest.pack.description}`,
    "---",
    "",
    `# ${manifest.pack.name}`,
    "",
    `${manifest.pack.description} Generated from \`packs/${packId}/pack.toml\`.`,
    "",
    `Run the whole notebook with \`bl notebook ${packId}.bln\`.`,
    "",
  ];

  for (const problem of manifest.problem) {
    parts.push(`## ${problem.id} — ${problem.title}`, "");
    parts.push(`[Problem statement](${problem.url})`, "");
    if (problem.blocked_on) parts.push(`**Partial:** ${problem.blocked_on}.`, "");
    parts.push("```biolang");
    // Network problems would abort a `bl notebook` run offline, so they are
    // present but not executed. @skip is the format's own directive for this.
    if (problem.network) {
      parts.push("# @skip  (needs a network connection — remove this line to run it)");
    }
    parts.push(sources.get(problem.id).trim(), "```", "");
  }

  return `${parts.join("\n")}\n`;
}

async function problemSection(pack, problem, available) {
  const source = await readFile(path.join(pack.directory, problem.file), "utf8");
  const anchor = problem.id.toLowerCase();
  // Opened in a new tab: following it in place throws away the page the reader
  // was studying, and the workbench is a place to try the code rather than a
  // destination to navigate to.
  const workbench = `/workbench/?pack=${encodeURIComponent(pack.packId)}&problem=${encodeURIComponent(problem.id)}`;
  // Per-problem download, offered only because each Rosalind problem is a whole
  // program. Blocks on the cumulative tutorial pages depend on the blocks above
  // them, so a single-block download there would not run.
  //
  // Sections are rendered onto the section pages, which already live inside
  // `<packId>/` next to the .bl files, so this is a bare filename.
  const download = `${problem.id.toLowerCase()}.bl`;

  const notes = [];
  if (problem.blocked_on) {
    notes.push(
      `<p class="text-sm text-amber-300/80 mb-4"><strong>Partial:</strong> ${escape(problem.blocked_on)}.</p>`,
    );
  }
  if (problem.network) {
    notes.push(
      `<p class="text-sm text-slate-500 mb-4">Calls NCBI, so it needs a network connection and its answer can change over time.</p>`,
    );
  }
  if (problem.note) {
    notes.push(`<p class="text-sm text-slate-500 mb-4">${escape(problem.note)}</p>`);
  }

  // data-standalone: every problem is self-contained, so the playground must not
  // replay the preceding ones before running this block.
  return `        <section id="${escape(anchor)}" class="mt-12">
          <h2 class="text-2xl font-bold text-white mb-2">${escape(problem.id)} &mdash; ${escape(problem.title)}</h2>
          <p class="mb-3 flex items-center gap-3 text-sm">
            ${badge(problem.status)}${runsBadge(problem, source, available)}
            <a href="${escape(problem.url)}" target="_blank" rel="noopener" class="text-violet-400 hover:text-violet-300">Problem statement</a>
            <a href="${escape(workbench)}" target="_blank" rel="noopener" class="text-violet-400 hover:text-violet-300">Open in the workbench</a>
            <a href="${escape(download)}" download class="text-violet-400 hover:text-violet-300">Download .bl</a>
          </p>
${notes.join("\n")}
          <pre data-standalone><code class="language-biolang">${escape(source.trim())}</code></pre>
        </section>`;
}

/**
 * One sub-page per section.
 *
 * The single page was heading for roughly 260 KiB and 107 syntax-highlighted
 * blocks, each with a Run button attached at load. Splitting keeps any one page
 * to a readable size and gives every problem a stable deep link.
 */
function renderGroupPage(pack, group, problems, sections, options = {}) {
  const { packId, manifest } = pack;
  const heading = options.heading ?? groupTitle(group);
  const title = `${manifest.pack.name} — ${heading}`;
  const intro = options.intro
    ?? `${problems.length} problem${problems.length === 1 ? "" : "s"} from`;

  return `<!DOCTYPE html>
<html lang="en" class="dark">
<head>
  <meta charset="utf-8">
  <script>if(localStorage.getItem("theme")==="light")document.documentElement.classList.remove("dark")</script>
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>${escape(title)} — BioLang Examples</title>
  <meta name="description" content="${escape(heading)} problems from ${escape(manifest.pack.name)}.">
  <link rel="icon" href="../../../assets/favicon.svg">
  <link rel="stylesheet" href="../../../assets/styles.css">
  <link id="hljs-theme" rel="stylesheet" href="https://cdnjs.cloudflare.com/ajax/libs/highlight.js/11.9.0/styles/github-dark.min.css">
  <script src="https://cdnjs.cloudflare.com/ajax/libs/highlight.js/11.9.0/highlight.min.js"></script>
  <script src="../../../js/biolang-highlight.js"></script>
</head>
<body class="bg-slate-950 text-slate-300">
  <!-- Generated by scripts/generate-pack-docs.mjs from packs/${escape(packId)}/pack.toml. Do not edit by hand. -->
  <div data-component="header" data-base-path="../../.."></div>
  <div class="flex max-w-[90rem] mx-auto">
    <div data-component="nav" data-base-path="../../.." data-active="examples/${escape(packId)}"></div>
    <main class="flex-1 min-w-0 px-6 lg:px-10 py-10">
      <nav class="mb-6 text-sm text-slate-500">
        <a href="../../../index.html" class="hover:text-violet-400">Home</a><span class="mx-2">/</span>
        <a href="../index.html" class="hover:text-violet-400">Examples</a><span class="mx-2">/</span>
        <a href="../${escape(packId)}.html" class="hover:text-violet-400">${escape(manifest.pack.name)}</a><span class="mx-2">/</span>
        <span class="text-slate-300">${escape(heading)}</span>
      </nav>
      <article class="prose prose-invert max-w-none">
        <h1 class="text-4xl font-bold text-white mb-2">${escape(heading)}</h1>
        <p class="text-lg text-slate-400 mb-6">${escape(intro)}
        <a href="../${escape(packId)}.html" class="text-violet-400 hover:text-violet-300">${escape(manifest.pack.name)}</a>.
        Press <strong>Run</strong> on any block to execute it in your browser.${
          options.heavy
            ? " Every problem is on this page, so it is large and takes a moment to settle — the sections are lighter."
            : ""
        }</p>

        <!-- In-page jump list. The index already deep-links each problem, but a
             section can hold fifteen of them, so it needs its own navigation. -->
        <nav aria-label="Problems on this page" class="flex flex-wrap gap-2 mb-10 not-prose">
${problems
  .map(
    (problem) =>
      `          <a href="#${escape(problem.id.toLowerCase())}" class="px-2.5 py-1 text-xs rounded border border-slate-800 bg-slate-900/50 text-slate-300 hover:border-violet-500/50 hover:text-violet-300 transition-colors" title="${escape(problem.title)}">${escape(problem.id)}</a>`,
  )
  .join("\n")}
        </nav>

${sections.join("\n\n")}
      </article>
      <nav class="mt-12 flex justify-between border-t border-slate-800 pt-6">
        <a href="../${escape(packId)}.html" class="text-violet-400 hover:text-violet-300">&larr; ${escape(manifest.pack.name)}</a>
      </nav>
    </main>
  </div>
  <div data-component="footer" data-base-path="../../.."></div>
  <script src="../../../js/main.js"></script>
  <script src="../../../js/copy-code.js"></script>
  <script src="../../../js/playground.js"></script>
</body>
</html>
`;
}

async function renderPack(packId, available) {
  const pack = await readPack(packId);
  const { manifest } = pack;
  const counts = packCounts(manifest);
  const sources = new Map();
  const sectionFor = new Map();
  for (const problem of manifest.problem) {
    sources.set(problem.id, await readFile(path.join(pack.directory, problem.file), "utf8"));
    sectionFor.set(problem.id, await problemSection(pack, problem, available));
  }

  const groups = groupsOf(manifest);
  const groupPages = new Map();
  for (const [group, problems] of groups) {
    groupPages.set(
      group,
      renderGroupPage(pack, group, problems, problems.map((p) => sectionFor.get(p.id))),
    );
  }

  // Everything on one page, for reading straight through, Ctrl+F across the
  // whole pack, or printing. Kept as an explicit choice rather than the default
  // because it carries every code block at once.
  groupPages.set(
    "all",
    renderGroupPage(
      pack,
      "all",
      manifest.problem,
      manifest.problem.map((p) => sectionFor.get(p.id)),
      {
        heading: "Every problem",
        intro: `All ${manifest.problem.length} problems from`,
        heavy: true,
      },
    ),
  );

  const menu = [...groups]
    .map(([group, problems]) => {
      const solved = problems.filter((p) => p.status === "solved").length;
      return `          <a href="${escape(packId)}/${escape(group)}.html" class="block p-4 rounded-xl border border-slate-800 bg-slate-900/50 hover:border-violet-500/50 transition-colors">
            <h3 class="text-lg font-semibold text-slate-100 mb-1">${escape(groupTitle(group))}</h3>
            <p class="text-sm text-slate-400">${problems.length} problem${problems.length === 1 ? "" : "s"} &mdash; ${solved} solved</p>
            <p class="text-xs text-slate-500 mt-2">${escape(problems.map((p) => p.id).join(", "))}</p>
          </a>`;
    })
    .join("\n");

  const title = manifest.pack.name;
  const html = `<!DOCTYPE html>
<html lang="en" class="dark">
<head>
  <meta charset="utf-8">
  <script>if(localStorage.getItem("theme")==="light")document.documentElement.classList.remove("dark")</script>
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>${escape(title)} — BioLang Examples</title>
  <meta name="description" content="${escape(manifest.pack.description)}">
  <link rel="icon" href="../../assets/favicon.svg">
  <link rel="stylesheet" href="../../assets/styles.css">
  <link id="hljs-theme" rel="stylesheet" href="https://cdnjs.cloudflare.com/ajax/libs/highlight.js/11.9.0/styles/github-dark.min.css">
  <script src="https://cdnjs.cloudflare.com/ajax/libs/highlight.js/11.9.0/highlight.min.js"></script>
  <script src="../../js/biolang-highlight.js"></script>
</head>
<body class="bg-slate-950 text-slate-300">
  <!-- Generated by scripts/generate-pack-docs.mjs from packs/${escape(packId)}/pack.toml. Do not edit by hand. -->
  <div data-component="header" data-base-path="../.."></div>
  <div class="flex max-w-[90rem] mx-auto">
    <div data-component="nav" data-base-path="../.." data-active="examples/${escape(packId)}"></div>
    <main class="flex-1 min-w-0 px-6 lg:px-10 py-10">
      <nav class="mb-6 text-sm text-slate-500">
        <a href="../../index.html" class="hover:text-violet-400">Home</a><span class="mx-2">/</span>
        <a href="index.html" class="hover:text-violet-400">Examples</a><span class="mx-2">/</span>
        <span class="text-slate-300">${escape(title)}</span>
      </nav>
      <article class="prose prose-invert max-w-none">
        <h1 class="text-4xl font-bold text-white mb-2">${escape(title)}</h1>
        <p class="text-lg text-slate-400 mb-4">${escape(manifest.pack.description)}
        <a href="${escape(manifest.pack.list_url ?? "https://rosalind.info")}" target="_blank" rel="noopener" class="text-violet-400 hover:text-violet-300">See the problem list</a>.</p>

        <p class="text-sm text-slate-500 mb-8">${manifest.problem.length} problems &mdash;
        ${counts.solved} solved, ${counts.partial} partial.
        ${counts.asserted} carry assertions that run on every commit;
        ${counts.network} need a network connection and are checked separately.
        Press <strong>Run</strong> on any block to execute it in your browser, or
        <a href="/workbench/?pack=${escape(packId)}" target="_blank" rel="noopener" class="text-violet-400 hover:text-violet-300">open the whole pack in the workbench</a>.</p>

        <p class="text-sm text-slate-500 mb-8">Take it with you:
        <a href="${escape(packId)}.bln" download class="text-violet-400 hover:text-violet-300">download this page as a notebook</a>
        (<code>.bln</code> — run it with <code>bl notebook</code>, or export it to HTML, PDF or Jupyter),
        <a href="${escape(packId)}.zip" download class="text-violet-400 hover:text-violet-300">every problem as a zip</a>,
        or grab a single problem with the <strong>Download .bl</strong> link in its section.</p>

        <h2 class="text-2xl font-bold text-white mt-10 mb-2">Sections</h2>
        <p class="text-sm text-slate-500 mb-4">Or read
        <a href="${escape(packId)}/all.html" class="text-violet-400 hover:text-violet-300">every problem on a single page</a>
        &mdash; heavier to load, but one place to scroll, search and print.</p>
        <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4 not-prose">
${menu}
        </div>

        <h2 class="text-2xl font-bold text-white mt-12 mb-4">Every problem</h2>
${coverageTable(manifest, packId, sources, available)}
      </article>
      <nav class="mt-12 flex justify-between border-t border-slate-800 pt-6">
        <a href="index.html" class="text-violet-400 hover:text-violet-300">&larr; All examples</a>
      </nav>
    </main>
  </div>
  <div data-component="footer" data-base-path="../.."></div>
  <script src="../../js/main.js"></script>
  <script src="../../js/copy-code.js"></script>
  <script src="../../js/playground.js"></script>
</body>
</html>
`;

  return { html, groupPages, notebook: await renderNotebook(pack, sources), manifest, sources };
}

await mkdir(outputRoot, { recursive: true });
const packIds = await listPackIds();
const available = await browserBuiltins();
for (const packId of packIds) {
  const { html, groupPages, notebook, manifest, sources } = await renderPack(packId, available);

  const target = path.join(outputRoot, `${packId}.html`);
  await writeFile(target, html);

  // One page per section, beside the .bl downloads.
  const sectionDir = path.join(outputRoot, packId);
  await mkdir(sectionDir, { recursive: true });
  for (const [group, page] of groupPages) {
    await writeFile(path.join(sectionDir, `${group}.html`), page);
  }

  // The page as a runnable notebook.
  await writeFile(path.join(outputRoot, `${packId}.bln`), notebook);

  // One .bl per problem. Offered here and not on the tutorial pages because a
  // Rosalind problem is a whole program, while a tutorial block is not.
  const sourceDir = path.join(outputRoot, packId);
  await mkdir(sourceDir, { recursive: true });
  for (const problem of manifest.problem) {
    await writeFile(
      path.join(sourceDir, `${problem.id.toLowerCase()}.bl`),
      sources.get(problem.id),
    );
  }

  // Every problem in one archive, for people who want the set rather than a
  // file at a time. The JSON bundle already carries the same sources, but that
  // is built for the playground and the CLI to parse — a zip is what a person
  // double-clicks.
  const archive = createZip([
    [`${packId}/README.md`, notebook],
    ...manifest.problem.map((problem) => [
      `${packId}/${problem.id.toLowerCase()}.bl`,
      sources.get(problem.id),
    ]),
  ]);
  await writeFile(path.join(outputRoot, `${packId}.zip`), archive);

  // Two hand-written places link to a pack page: the examples gallery and the
  // shared sidebar. Both were missed for the Stronghold — generating the page
  // is not enough if nothing points at it, and checking only one of the two
  // still left it missing from the sidebar.
  const linkSites = [
    { file: path.join(outputRoot, "index.html"), what: "a card in the examples gallery" },
    {
      file: path.join(repositoryRoot, "website", "components", "nav.html"),
      what: "an entry in the sidebar nav",
    },
  ];
  for (const { file, what } of linkSites) {
    const contents = await readFile(file, "utf8").catch(() => "");
    if (contents && !contents.includes(`${packId}.html`)) {
      console.error(
        `\n${packId}: generated, but ${path.relative(repositoryRoot, file).replaceAll("\\", "/")} ` +
          `has no link to it — add ${what}, or the pack is unreachable.`,
      );
      process.exitCode = 1;
    }
  }

  console.log(
    `${packId} -> ${path.relative(repositoryRoot, target).replaceAll("\\", "/")} ` +
      `(${(Buffer.byteLength(html) / 1024).toFixed(1)} KiB index, ${groupPages.size} sections, ` +
      `${(Buffer.byteLength(notebook) / 1024).toFixed(1)} KiB bln, ` +
      `${manifest.problem.length} .bl)`,
  );
}

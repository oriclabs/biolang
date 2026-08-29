#!/usr/bin/env node
// Measure the plot test suite rather than the plot code.
//
// A test that cannot fail is worse than no test: it reports safety it does not
// provide. The discrete palette was wrong at every group count except two and
// shipped under a green suite, because the test guarding it asserted only that
// the SVG contained the first colour - true of the correct palette and the
// broken table alike.
//
// So this applies one realistic regression at a time and records whether any
// test notices. A mutation that survives names a behaviour nothing is holding
// in place. Run it after changing anything in the plotting layer:
//
//   node scripts/mutation-audit.mjs            # every mutation
//   node scripts/mutation-audit.mjs palette    # only matching names
//
// Exit status is non-zero when a mutation survives.

import { spawnSync } from "node:child_process";
import { readFileSync, writeFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
// plot.rs is a directory module, so each mutation names the submodule that
// owns the behaviour it breaks. A mutation whose target has moved is
// reported as unusable rather than silently passing.
const CANVAS = "crates/bl-runtime/src/plot/canvas.rs";
const THEME = "crates/bl-runtime/src/plot/theme.rs";
const HISTOGRAM = "crates/bl-runtime/src/plot/histogram.rs";
const STATS = "crates/bl-runtime/src/stats_explore.rs";

// Each entry is a behaviour worth holding in place, expressed as the smallest
// edit that breaks it. `find` must appear in the file; every occurrence is
// replaced, so a mutation that matches twice breaks the behaviour twice.
const MUTATIONS = [
  {
    name: "hue_pal step",
    file: THEME,
    find: "let hue = (15.0 + 360.0 * index as f64 / count as f64).to_radians();",
    replace: "let hue = (15.0 + 180.0 * index as f64).to_radians();",
  },
  {
    name: "histogram bar fill",
    file: HISTOGRAM,
    find: 'let bar_fill = if ggplot_like { "#595959" } else { PALETTE[0] };',
    replace: "let bar_fill = PALETTE[0];",
  },
  {
    name: "bin_rule default",
    file: HISTOGRAM,
    find: '.unwrap_or("ggplot")',
    replace: '.unwrap_or("span")',
  },
  {
    name: "theme_grey panel",
    file: THEME,
    find: 'panel_colour: "#ebebeb",',
    replace: 'panel_colour: "#ffffff",',
  },
  {
    name: "legacy marker opacity",
    file: CANVAS,
    find: "self.add_circle_with_opacity(cx, cy, r, fill, 0.7);",
    replace: "self.add_circle_with_opacity(cx, cy, r, fill, 1.0);",
  },
  {
    name: "text metrics",
    file: CANVAS,
    find: "u32::from(ADVANCE_PER_MILLE[(code - 32) as usize])",
    replace: "540",
  },
  { name: "ribbon grey60", file: STATS, find: '"#999999"', replace: '"#f8766d"' },
  { name: "scale expansion", file: STATS, find: "span * 0.05", replace: "span * 0.08" },
  { name: "marker radius", file: STATS, find: "2.6,", replace: "3.4," },
  { name: "fit line width", file: STATS, find: "2.85,", replace: "1.5," },
  {
    name: "boxplot geometry",
    file: STATS,
    find: '("#333333", "#ffffff", 1.42, 2.85)',
    replace: '("#1e3a8a", "#bfdbfe", 1.5, 2.0)',
  },
  {
    name: "draw order",
    file: STATS,
    find: "points.sort_unstable_by_key(|(row, _, _, _)| *row);",
    replace: "points.sort_unstable_by_key(|(_, _, _, group)| *group);",
  },
  {
    name: "per-group fit range",
    file: STATS,
    find:
      "let minimum = group.xs.iter().copied().min_by(f64::total_cmp).unwrap();\n" +
      "        let maximum = group.xs.iter().copied().max_by(f64::total_cmp).unwrap();",
    replace: "let minimum = 0.0_f64;\n        let maximum = 100.0_f64;",
  },
  {
    name: "facet shared bins",
    file: STATS,
    find: "shared_edges.clone()",
    replace:
      "crate::plot::histogram_ggplot_edges(&rows.iter().map(|(x, _)| *x).collect::<Vec<_>>(), bins)",
  },
];

const TEST_ARGS = [
  "test", "-p", "bl-runtime",
  "--lib",
  "--test", "plot_tests",
  "--test", "ggplot_conformance",
  "--test", "stats_exploration_tests",
  "--test", "png_export_tests",
];

function runTests() {
  const result = spawnSync("cargo", TEST_ARGS, {
    cwd: root,
    encoding: "utf8",
    maxBuffer: 64 * 1024 * 1024,
  });
  const output = `${result.stdout ?? ""}${result.stderr ?? ""}`;
  if (output.includes("error[E") || output.includes("error: could not compile")) {
    return { built: false, failing: [] };
  }
  const failing = new Set();
  for (const line of output.split(/\r?\n/)) {
    const trimmed = line.trim();
    if (trimmed.startsWith("test ") && trimmed.endsWith("FAILED")) {
      failing.add(trimmed.slice(5).split(" ...")[0]);
    }
  }
  return { built: true, failing: [...failing] };
}

const filter = process.argv[2]?.toLowerCase();
const selected = filter
  ? MUTATIONS.filter((mutation) => mutation.name.toLowerCase().includes(filter))
  : MUTATIONS;
if (selected.length === 0) {
  console.error(`No mutation matches ${JSON.stringify(filter)}.`);
  process.exit(2);
}

const originals = new Map();
for (const { file } of selected) {
  if (!originals.has(file)) {
    originals.set(file, readFileSync(path.join(root, file), "utf8"));
  }
}

const restore = () => {
  for (const [file, text] of originals) {
    writeFileSync(path.join(root, file), text);
  }
};
process.on("SIGINT", () => {
  restore();
  process.exit(130);
});

const survived = [];
const unusable = [];
let caught = 0;

try {
  for (const mutation of selected) {
    const absolute = path.join(root, mutation.file);
    const original = originals.get(mutation.file);
    if (!original.includes(mutation.find)) {
      unusable.push(mutation.name);
      console.log(`  ${mutation.name.padEnd(22)} SKIP      the code it targets has moved`);
      continue;
    }
    writeFileSync(absolute, original.split(mutation.find).join(mutation.replace));
    const { built, failing } = runTests();
    writeFileSync(absolute, original);

    if (!built) {
      unusable.push(mutation.name);
      console.log(`  ${mutation.name.padEnd(22)} NO-BUILD  the mutation does not compile`);
    } else if (failing.length > 0) {
      caught += 1;
      console.log(`  ${mutation.name.padEnd(22)} CAUGHT    ${failing.slice(0, 2).join(", ")}`);
    } else {
      survived.push(mutation.name);
      console.log(`  ${mutation.name.padEnd(22)} SURVIVED  no test failed`);
    }
  }
} finally {
  restore();
}

console.log(
  `\ncaught: ${caught}   survived: ${survived.length}   unusable: ${unusable.length}`,
);

if (unusable.length > 0) {
  console.log(
    "\nThese mutations proved nothing - a mutation that will not compile, or whose\n" +
      "target has moved, measures no test. Rewrite them against the current code:",
  );
  for (const name of unusable) console.log(`  - ${name}`);
}

if (survived.length > 0) {
  console.log("\nThese regressions ship silently. Each needs a test that fails on it:");
  for (const name of survived) console.log(`  - ${name}`);
  process.exit(1);
}

console.log("\nEvery mutation was caught.");

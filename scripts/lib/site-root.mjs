/**
 * Where the published site lives.
 *
 * The website moved to its own repository, so the pages these generators write
 * are no longer inside this checkout. A sibling clone is the ordinary layout;
 * BIOLANG_SITE_ROOT overrides it for anything else.
 *
 * `siteRoot` is for reading, where a missing clone means "skip this", and
 * `requireSiteRoot` is for writing, where it means the output would land
 * somewhere nothing publishes from -- which is exactly what happened when
 * these paths still pointed at a `website/` directory inside this repository.
 */

import fs from "node:fs";
import path from "node:path";

export function siteRoot(repositoryRoot) {
  const configured = process.env.BIOLANG_SITE_ROOT;
  return configured
    ? path.resolve(configured)
    : path.resolve(repositoryRoot, "..", "biolang-website");
}

export function requireSiteRoot(repositoryRoot) {
  const root = siteRoot(repositoryRoot);
  if (!fs.existsSync(root)) {
    throw new Error(
      `no website checkout at ${root}. Clone the biolang-website repository `
        + "beside this one, or set BIOLANG_SITE_ROOT to where it lives.",
    );
  }
  return root;
}

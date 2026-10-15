// Assemble latest.json (manifeste de mise à jour Tauri) pour la release GitHub.
//   node scripts/make-latest-json.mjs <dossier-contenant-installeur-et-.sig> <sortie.json>
// Notes = section "## <version>" de CHANGELOG.md.
import { readFileSync, writeFileSync, existsSync } from "node:fs";
import { join } from "node:path";

const { version } = JSON.parse(readFileSync("package.json", "utf8"));
const [dir, out] = process.argv.slice(2);
const exe = `PureRGB_${version}_x64-setup.exe`;
const sigPath = join(dir, `${exe}.sig`);
if (!existsSync(sigPath)) throw new Error(`Signature introuvable : ${sigPath}`);

const log = existsSync("CHANGELOG.md") ? readFileSync("CHANGELOG.md", "utf8") : "";
const section = log.split(/^## /m).find((s) => s.trimStart().startsWith(version));
const notes = section ? section.split(/\r?\n/).slice(1).join("\n").trim() : `PureRGB ${version}`;

writeFileSync(
  out,
  JSON.stringify(
    {
      version,
      notes,
      pub_date: new Date().toISOString(),
      platforms: {
        "windows-x86_64": {
          signature: readFileSync(sigPath, "utf8").trim(),
          url: `https://github.com/Heiphaistos/PureRGB/releases/download/v${version}/${exe}`,
        },
      },
    },
    null,
    2,
  ),
);
console.log(`écrit ${out}`);

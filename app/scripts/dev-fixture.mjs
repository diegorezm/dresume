// Snapshots the example workspace into src/lib/dev/fixture.json so the UI can
// run in a plain browser (`pnpm dev`) with mocked Tauri IPC. Needs a built CLI.
import { execFileSync } from "node:child_process";
import { mkdtempSync, readFileSync, writeFileSync, existsSync, readdirSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";

const root = resolve(import.meta.dirname, "../..");
const exe = join(root, "target/debug", process.platform === "win32" ? "dresume.exe" : "dresume");
const ws = join(root, "example");
const cli = (...args) => JSON.parse(execFileSync(exe, ["-w", ws, "--json", ...args], { encoding: "utf8" }));
const text = (...args) => execFileSync(exe, ["-w", ws, ...args], { encoding: "utf8" });

const { templates } = cli("list");
const resumes = templates.flatMap((t) => t.resumes.map((id) => ({ id, template: t.id, path: `resumes/${id}.yaml` })));
const out = mkdtempSync(join(tmpdir(), "dresume-fixture-"));

const fixture = {
  // Neutral name and path: these screens end up in the README.
  summary: { name: "My resumes", root: "~/Documents/resumes", templates, resumes },
  profile: readFileSync(join(ws, "profile.yaml"), "utf8"),
  resumes: {}, schemas: {}, svg: {}, diagnostics: {}, templateFiles: {},
};
for (const t of templates) {
  fixture.schemas[t.id] = JSON.parse(text("schema", t.id));
  const dir = join(ws, "templates", t.id);
  fixture.templateFiles[t.id] = Object.fromEntries(
    readdirSync(dir).filter((f) => /\.(typ|yaml|json)$/.test(f)).map((f) => [f, readFileSync(join(dir, f), "utf8")]),
  );
}
for (const r of resumes) {
  fixture.resumes[r.id] = JSON.parse(text("show", r.id));
  const res = cli("build", r.id, "--format", "svg", "-o", out).results[0];
  fixture.diagnostics[r.id] = res.diagnostics;
  fixture.svg[r.id] = [];
  for (let p = 1; existsSync(join(out, `${r.id}-${p}.svg`)); p++) {
    fixture.svg[r.id].push(readFileSync(join(out, `${r.id}-${p}.svg`), "utf8"));
  }
}
writeFileSync(join(import.meta.dirname, "../src/lib/dev/fixture.json"), JSON.stringify(fixture));
console.log(`fixture: ${resumes.length} resumes, ${templates.length} templates`);

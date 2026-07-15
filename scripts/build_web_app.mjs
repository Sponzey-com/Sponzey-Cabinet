import { join } from "node:path";
import { fileURLToPath } from "node:url";

let build;

try {
  ({ build } = await import("esbuild"));
} catch {
  console.error("Web app build requires npm dependencies. Run `npm install` once, then retry.");
  process.exit(1);
}

const root = join(fileURLToPath(new URL(".", import.meta.url)), "..");

await build({
  entryPoints: [join(root, "apps/web/public/app.js")],
  outfile: join(root, "apps/web/public/app.bundle.js"),
  bundle: true,
  format: "iife",
  target: "es2022",
  logLevel: "silent",
});

import { join } from "node:path";
import { fileURLToPath } from "node:url";

import { buildDesktopAssets } from "./desktop_asset_builder.mjs";

const root = join(fileURLToPath(new URL(".", import.meta.url)), "..");
const result = await buildDesktopAssets(root);
console.log(`desktop_assets_built=${result.distDir}`);

// Copies the wasm-pack bundle into the Vite public dir when present, so the
// dev server and the production build can load real Rust-backed math.
// Always succeeds: when no bundle exists the app runs on the JS fallback.
import { existsSync, mkdirSync, cpSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = join(dirname(fileURLToPath(import.meta.url)), '..', '..');
const pkgDir = join(root, 'crates', 'wasm_bridge', 'pkg');
const glue = join(pkgDir, 'wasm_bridge.js');
const dest = join(root, 'frontend', 'public', 'pkg');

if (existsSync(glue)) {
  mkdirSync(dest, { recursive: true });
  cpSync(pkgDir, dest, { recursive: true });
  console.log('[sync-pkg] Rust WASM bundle linked into frontend/public/pkg');
} else {
  console.log('[sync-pkg] no wasm-pack bundle; frontend will use the JS fallback');
}
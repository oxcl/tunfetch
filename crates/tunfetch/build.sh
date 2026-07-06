#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PKG_DIR="$SCRIPT_DIR/pkg"

echo "Building tunfetch with wasm-pack..."
wasm-pack build --target bundler "$SCRIPT_DIR"


# in order for the js bindings to work with cloudflare workers they have to be patched
# https://developers.cloudflare.com/workers/languages/rust/#:~:text=If%20you%20are%20using,to/mylib.js%22%3B
echo "Patching tunfetch.js for Cloudflare Workers..."
cat > "$PKG_DIR/tunfetch.js" << 'PATCH'
/* @ts-self-types="./tunfetch.d.ts" */
import * as imports from "./tunfetch_wasm_bg.js";
import wkmod from "./tunfetch_wasm_bg.wasm";

const instance = new WebAssembly.Instance(wkmod, {
  "./tunfetch_wasm_bg.js": imports,
});
imports.__wbg_set_wasm(instance.exports);
instance.exports.__wbindgen_start();

export * from "./tunfetch_wasm_bg.js";
PATCH

echo "Build complete: $PKG_DIR"

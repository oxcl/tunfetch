import { cloudflareTest } from "@cloudflare/vitest-pool-workers";
import { defineConfig } from "vitest/config";
import path from "path";

const root = __dirname;

export default defineConfig({
  root,
  plugins: [
    cloudflareTest({
      wrangler: { configPath: path.join(root, "./wrangler.jsonc") },
      miniflare: {
        // Allow workerd to connect to localhost (for httpbin container)
        outgoing: {
          allow: ["localhost", "127.0.0.1"],
        },
      },
    }),
  ],
  test: {
    include: ["./**/*.test.ts"],
    globalSetup: [path.join(root, "./global-setup.ts")],
  },
});

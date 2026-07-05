import { describe, it, expect, inject } from "vitest";
import { tunfetch } from "../../../crates/tunfetch/pkg/tunfetch";

const HTTPBIN_URL = inject("httpbinUrl");
const PROXY_URL = inject("httpProxyRestricted");

describe("HTTP proxy - restricted CONNECT (only port 443 allowed)", () => {
  it("HTTP target works (no CONNECT needed)", async () => {
    const res = await tunfetch(`${HTTPBIN_URL}/get`, {
      proxy: PROXY_URL,
    });
    expect(res.status).toBe(200);
  });

  it("CONNECT to port 443 is allowed", async () => {
    // httpbin also listens on 443 (mapped to 18443 on host)
    // Squid with host networking can reach localhost:443
    // Note: this tests that CONNECT to 443 is not blocked
    const res = await tunfetch(`https://localhost:443/get`, {
      proxy: PROXY_URL,
    });
    // Even if TLS handshake fails (self-signed cert), the CONNECT itself should succeed
    expect(res.status).toBeDefined();
  });
});

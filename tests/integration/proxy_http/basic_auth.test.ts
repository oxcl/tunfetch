import { describe, it, expect, inject } from "vitest";
import { tunfetch } from "../../../crates/tunfetch/pkg/tunfetch";

const HTTPBIN_URL = inject("httpbinUrl");
const PROXY_URL = inject("httpProxyBasicAuth"); // http://localhost:3129
const PROXY_PORT = new URL(PROXY_URL).port;

describe("HTTP proxy - basic auth", () => {
  it("succeeds with valid preemptive credentials", async () => {
    const res = await tunfetch(`${HTTPBIN_URL}/get`, {
      proxy: `http://testuser:testpass@localhost:${PROXY_PORT}`,
    });
    expect(res.status).toBe(200);
  });

  it("returns 407 with no credentials", async () => {
    const res = await tunfetch(`${HTTPBIN_URL}/get`, {
      proxy: PROXY_URL,
    });
    expect(res.status).toBe(407);
  });

  it("returns 407 with invalid credentials", async () => {
    const res = await tunfetch(`${HTTPBIN_URL}/get`, {
      proxy: `http://wrong:creds@localhost:${PROXY_PORT}`,
    });
    expect(res.status).toBe(407);
  });

  it("succeeds after retry with correct credentials", async () => {
    // First request without auth → 407
    const first = await tunfetch(`${HTTPBIN_URL}/get`, {
      proxy: PROXY_URL,
    });
    expect(first.status).toBe(407);

    // Second request with auth → 200
    const second = await tunfetch(`${HTTPBIN_URL}/get`, {
      proxy: `http://testuser:testpass@localhost:${PROXY_PORT}`,
    });
    expect(second.status).toBe(200);
  });
});

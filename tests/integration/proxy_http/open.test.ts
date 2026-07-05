import { describe, it, expect, inject } from "vitest";
import { tunfetch } from "../../../crates/tunfetch/pkg/tunfetch";

const HTTPBIN_URL = inject("httpbinUrl");
const PROXY_URL = inject("httpProxyOpen");

describe("HTTP proxy - open (no auth)", () => {
  it("GET through proxy", async () => {
    const res = await tunfetch(`${HTTPBIN_URL}/get`, {
      proxy: PROXY_URL,
    });
    expect(res.status).toBe(200);
    const data = await res.json();
    expect(data.url).toContain("/get");
  });

  it("POST with body through proxy", async () => {
    const res = await tunfetch(`${HTTPBIN_URL}/post`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ key: "value" }),
      proxy: PROXY_URL,
    });
    expect(res.status).toBe(200);
    const data = await res.json();
    expect(data.json).toEqual({ key: "value" });
  });

  it("response status is preserved through proxy", async () => {
    const res = await tunfetch(`${HTTPBIN_URL}/status/201`, {
      proxy: PROXY_URL,
    });
    expect(res.status).toBe(201);
  });

  it("response body is intact through proxy", async () => {
    const res = await tunfetch(`${HTTPBIN_URL}/json`, {
      proxy: PROXY_URL,
    });
    expect(res.status).toBe(200);
    const data = await res.json();
    expect(data.slideshow).toBeDefined();
  });

  it("response headers are preserved through proxy", async () => {
    const res = await tunfetch(`${HTTPBIN_URL}/headers`, {
      proxy: PROXY_URL,
      headers: { "X-Test": "proxy-test" },
    });
    expect(res.status).toBe(200);
    const data = await res.json();
    expect(data.headers["X-Test"]).toBe("proxy-test");
  });
});

import { describe, it, expect } from "vitest";
import { tunfetch } from "../../../crates/tunfetch/pkg/tunfetch";

// httpbin is started on this port by global-setup.ts
const HTTPBIN_URL = "http://localhost:18080";

// ---------------------------------------------------------------------------
// 1. tunfetch & basic contract
// ---------------------------------------------------------------------------
describe("tunfetch exports", () => {
  it("exports tunfetch as a function", () => {
    expect(typeof tunfetch).toBe("function");
  });

  it("tunfetch returns a Promise", () => {
    const result = tunfetch(`${HTTPBIN_URL}/get`);
    expect(result).toBeInstanceOf(Promise);
    // Clean up – we don't care about the result here
    result.catch(() => {});
  });
});

// ---------------------------------------------------------------------------
// 2. HTTPBIN sanity check (proves the container is alive)
// ---------------------------------------------------------------------------
describe("httpbin sanity", () => {
  it("tunfetch can reach httpbin /get", async () => {
    const res = await tunfetch(`${HTTPBIN_URL}/get`);
    expect(res.status).toBe(200);
    const data = await res.json();
    expect(data).toBeDefined();
  });
});

// ---------------------------------------------------------------------------
// 3. tunfetch – basic GET
// ---------------------------------------------------------------------------
describe("tunfetch GET", () => {
  it("performs a basic GET and returns a Response-like object", async () => {
    const res = await tunfetch(`${HTTPBIN_URL}/get`);
    // The final API should return a Response (or Response-like object).
    // For now the impl returns a string, so this test captures the
    // desired end-state. It will fail until the impl is updated.
    expect(res).toBeDefined();
    expect(res.status).toBe(200);
  });

  it("response body contains the url field from httpbin", async () => {
    const res = await tunfetch(`${HTTPBIN_URL}/get`);
    const data = await res.json();
    expect(data.url).toBe(`${HTTPBIN_URL}/get`);
  });
});

// ---------------------------------------------------------------------------
// 4. tunfetch – error handling
// ---------------------------------------------------------------------------
describe("tunfetch error handling", () => {
  it("rejects on invalid URL", async () => {
    await expect(tunfetch("not-a-valid-url")).rejects.toThrow();
  });

  it("rejects when signal is aborted", async () => {
    const controller = new AbortController();
    setTimeout(() => controller.abort(), 1);
    await expect(
      tunfetch(`${HTTPBIN_URL}/delay/5`, { signal: controller.signal })
    ).rejects.toThrow();
  });
});

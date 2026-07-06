import { describe, it, expect } from "vitest";
import { tunfetch } from "../../../crates/tunfetch/pkg/tunfetch";

// httpbin is started on this port by global-setup.ts
const HTTPBIN_URL = "http://localhost:18080";

// ---------------------------------------------------------------------------
// 1. tunfetch – POST with body
// ---------------------------------------------------------------------------
describe("tunfetch POST", () => {
  it("sends a POST request with a JSON body", async () => {
    const payload = { message: "hello from tunfetch" };
    const res = await tunfetch(`${HTTPBIN_URL}/post`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(payload),
    });
    expect(res.status).toBe(200);

    const data = await res.json();
    // httpbin /post echoes the parsed JSON body back
    expect(data.data).toEqual(JSON.stringify(payload));
  });

  it("sends a POST with a plain text body", async () => {
    const res = await tunfetch(`${HTTPBIN_URL}/post`, {
      method: "POST",
      headers: { "Content-Type": "text/plain" },
      body: "raw text payload",
    });
    expect(res.status).toBe(200);

    const data = await res.json();
    expect(data.data).toBe("raw text payload");
  });
});

// ---------------------------------------------------------------------------
// 2. tunfetch – query parameters
// ---------------------------------------------------------------------------
describe("tunfetch query parameters", () => {
  it("sends query parameters via URL", async () => {
    const res = await tunfetch(`${HTTPBIN_URL}/get?foo=bar&baz=qux`);
    expect(res.status).toBe(200);

    const data = await res.json();
    expect(data.args).toEqual({ foo: "bar", baz: "qux" });
  });
});

// ---------------------------------------------------------------------------
// 3. tunfetch – response body parsing
// ---------------------------------------------------------------------------
describe("tunfetch response body", () => {
  it("can read response as text", async () => {
    const res = await tunfetch(`${HTTPBIN_URL}/ip`);
    expect(res.status).toBe(200);

    const text = await res.text();
    expect(typeof text).toBe("string");
    // httpbin /ip returns JSON with an "origin" field
    const parsed = JSON.parse(text);
    expect(parsed).toHaveProperty("origin");
  });

  it("can read response as JSON", async () => {
    const res = await tunfetch(`${HTTPBIN_URL}/ip`);
    const data = await res.json();
    expect(data).toHaveProperty("origin");
    expect(typeof data.origin).toBe("string");
  });
});

// ---------------------------------------------------------------------------
// 4. tunfetch – response formats
// ---------------------------------------------------------------------------
describe("tunfetch response formats", () => {
  it("returns HTML from /html", async () => {
    const res = await tunfetch(`${HTTPBIN_URL}/html`);
    expect(res.status).toBe(200);
    const text = await res.text();
    expect(text).toContain("<!DOCTYPE html>");
  });

  it("returns XML from /xml", async () => {
    const res = await tunfetch(`${HTTPBIN_URL}/xml`);
    expect(res.status).toBe(200);
    const text = await res.text();
    expect(text).toContain("<?xml");
  });

  it("returns JSON from /json", async () => {
    const res = await tunfetch(`${HTTPBIN_URL}/json`);
    expect(res.status).toBe(200);
    const data = await res.json();
    expect(data).toBeDefined();
    // /json returns a sample JSON object with "slideshow" key
    expect(data.slideshow).toBeDefined();
  });

  it("returns robots.txt from /robots.txt", async () => {
    const res = await tunfetch(`${HTTPBIN_URL}/robots.txt`);
    expect(res.status).toBe(200);
    const text = await res.text();
    expect(text).toContain("User-agent");
  });

  it("returns denial message from /deny", async () => {
    const res = await tunfetch(`${HTTPBIN_URL}/deny`);
    expect(res.status).toBe(200);
    const text = await res.text();
    expect(text).toContain("YOU SHOULDN'T BE HERE");
  });

  it("returns UTF-8 content from /encoding/utf8", async () => {
    const res = await tunfetch(`${HTTPBIN_URL}/encoding/utf8`);
    expect(res.status).toBe(200);
    const text = await res.text();
    // Should contain multi-byte UTF-8 characters
    expect(text.length).toBeGreaterThan(0);
  });
});

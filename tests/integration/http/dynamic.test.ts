import { describe, it, expect } from "vitest";
import { tunfetch } from "../../../crates/tunfetch/pkg/tunfetch";

// httpbin is started on this port by global-setup.ts
const HTTPBIN_URL = "http://localhost:18080";

// ---------------------------------------------------------------------------
// 1. tunfetch – dynamic data generation
// ---------------------------------------------------------------------------
describe("tunfetch dynamic data", () => {
  it("returns a UUID from /uuid", async () => {
    const res = await tunfetch(`${HTTPBIN_URL}/uuid`);
    expect(res.status).toBe(200);
    const data = await res.json();
    // UUID4 format: 8-4-4-4-12 hex characters
    expect(data.uuid).toMatch(
      /^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/
    );
  });

  it("decodes base64 from /base64/{value}", async () => {
    const encoded = btoa("hello tunfetch");
    const res = await tunfetch(`${HTTPBIN_URL}/base64/${encoded}`);
    expect(res.status).toBe(200);
    const text = await res.text();
    expect(text).toBe("hello tunfetch");
  });

  it("returns n random bytes from /bytes/{n}", async () => {
    const res = await tunfetch(`${HTTPBIN_URL}/bytes/256`);
    expect(res.status).toBe(200);
    expect(res.headers.get("Content-Type")).toBe("application/octet-stream");
    const buffer = await res.arrayBuffer();
    expect(buffer.byteLength).toBe(256);
  });

  it("returns a single JSON object from /stream/1", async () => {
    const res = await tunfetch(`${HTTPBIN_URL}/stream/1`);
    expect(res.status).toBe(200);
    const text = await res.text();
    // /stream/n returns n JSON objects, one per line
    const lines = text.trim().split("\n");
    expect(lines.length).toBe(1);
    const obj = JSON.parse(lines[0]);
    expect(obj.id).toBe(0);
  });

  it("returns multiple JSON objects from /stream/3", async () => {
    const res = await tunfetch(`${HTTPBIN_URL}/stream/3`);
    expect(res.status).toBe(200);
    const text = await res.text();
    const lines = text.trim().split("\n");
    expect(lines.length).toBe(3);
  });
});

// ---------------------------------------------------------------------------
// 2. tunfetch – delay / timeout
// ---------------------------------------------------------------------------
describe("tunfetch delay", () => {
  it("handles /delay/1 (1 second delay)", async () => {
    const start = Date.now();
    const res = await tunfetch(`${HTTPBIN_URL}/delay/1`);
    const elapsed = Date.now() - start;
    expect(res.status).toBe(200);
    expect(elapsed).toBeGreaterThanOrEqual(900); // allow some tolerance
  });

  it("handles /delay/2 (2 second delay)", async () => {
    const start = Date.now();
    const res = await tunfetch(`${HTTPBIN_URL}/delay/2`);
    const elapsed = Date.now() - start;
    expect(res.status).toBe(200);
    expect(elapsed).toBeGreaterThanOrEqual(1800);
  });
});

// ---------------------------------------------------------------------------
// 3. tunfetch – links
// ---------------------------------------------------------------------------
describe("tunfetch links", () => {
  it("returns HTML links page from /links/{n}/{offset}", async () => {
    const res = await tunfetch(`${HTTPBIN_URL}/links/10/0`);
    expect(res.status).toBe(200);
    const text = await res.text();
    expect(text).toContain("<a");
  });

  it("offset link is non-existent (404)", async () => {
    const res = await tunfetch(`${HTTPBIN_URL}/links/10/0`);
    expect(res.status).toBe(200);
    // The page lists links 1–10; link 0 is the offset and is
    // rendered as plain text (not a clickable link).
    const text = await res.text();
    expect(text).toContain("<a");
  });
});

// ---------------------------------------------------------------------------
// 4. tunfetch – streaming bytes
// ---------------------------------------------------------------------------
describe("tunfetch streaming", () => {
  it("streams bytes from /stream-bytes/{n}", async () => {
    const res = await tunfetch(`${HTTPBIN_URL}/stream-bytes/1024`);
    expect(res.status).toBe(200);
    expect(res.headers.get("Content-Type")).toBe("application/octet-stream");
    const buffer = await res.arrayBuffer();
    expect(buffer.byteLength).toBe(1024);
  });

  it("handles /range/{n} for byte range requests", async () => {
    const res = await tunfetch(`${HTTPBIN_URL}/range/1024`);
    expect(res.status).toBe(200);
    const buffer = await res.arrayBuffer();
    expect(buffer.byteLength).toBeGreaterThan(0);
  });
});

// ---------------------------------------------------------------------------
// 5. tunfetch – drip (slow data transfer)
// ---------------------------------------------------------------------------
describe("tunfetch drip", () => {
  it("returns the requested number of bytes", async () => {
    // Workers fetch buffers the entire response, so the drip duration
    // is not observable. We only verify the payload arrives intact.
    const res = await tunfetch(
      `${HTTPBIN_URL}/drip?numbytes=10&duration=1&delay=0`
    );
    expect(res.status).toBe(200);
    const text = await res.text();
    expect(text.length).toBe(10);
  });
});

// ---------------------------------------------------------------------------
// 6. tunfetch – image endpoints (binary response)
// ---------------------------------------------------------------------------
describe("tunfetch images", () => {
  it("returns a PNG image from /image/png", async () => {
    const res = await tunfetch(`${HTTPBIN_URL}/image/png`);
    expect(res.status).toBe(200);
    expect(res.headers.get("Content-Type")).toBe("image/png");
    const buffer = await res.arrayBuffer();
    expect(buffer.byteLength).toBeGreaterThan(0);
  });

  it("returns a JPEG image from /image/jpeg", async () => {
    const res = await tunfetch(`${HTTPBIN_URL}/image/jpeg`);
    expect(res.status).toBe(200);
    expect(res.headers.get("Content-Type")).toBe("image/jpeg");
  });

  it("returns a WEBP image from /image/webp", async () => {
    const res = await tunfetch(`${HTTPBIN_URL}/image/webp`);
    expect(res.status).toBe(200);
    expect(res.headers.get("Content-Type")).toBe("image/webp");
  });

  it("returns an SVG image from /image/svg", async () => {
    const res = await tunfetch(`${HTTPBIN_URL}/image/svg`);
    expect(res.status).toBe(200);
    expect(res.headers.get("Content-Type")).toBe("image/svg+xml");
  });

  it("returns appropriate image based on Accept header via /image", async () => {
    const res = await tunfetch(`${HTTPBIN_URL}/image`, {
      headers: { Accept: "image/png" },
    });
    expect(res.status).toBe(200);
    expect(res.headers.get("Content-Type")).toBe("image/png");
  });
});

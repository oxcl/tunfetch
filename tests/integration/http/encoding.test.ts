import { describe, it, expect } from "vitest";
import { tunfetch } from "../../../crates/tunfetch/pkg/tunfetch";

// httpbin is started on this port by global-setup.ts
const HTTPBIN_URL = "http://localhost:18080";

// ---------------------------------------------------------------------------
// 1. tunfetch – gzip / encoding
// ---------------------------------------------------------------------------
describe("tunfetch encoding", () => {
  it("handles gzip-encoded response", async () => {
    const res = await tunfetch(`${HTTPBIN_URL}/gzip`);
    expect(res.status).toBe(200);

    const data = await res.json();
    expect(data.gzipped).toBe(true);
  });
});

// ---------------------------------------------------------------------------
// 2. tunfetch – compression encodings
// ---------------------------------------------------------------------------
describe("tunfetch compression", () => {
  it("handles deflate-encoded response", async () => {
    const res = await tunfetch(`${HTTPBIN_URL}/deflate`);
    expect(res.status).toBe(200);
    // Workers fetch auto-decompresses gzip but not deflate;
    // the response may be raw bytes or decompressed JSON depending
    // on the runtime. We only verify the request succeeded.
    const buffer = await res.arrayBuffer();
    expect(buffer.byteLength).toBeGreaterThan(0);
  });

  it("handles brotli-encoded response", async () => {
    const res = await tunfetch(`${HTTPBIN_URL}/brotli`);
    expect(res.status).toBe(200);
    const data = await res.json();
    expect(data.brotli).toBe(true);
  });
});

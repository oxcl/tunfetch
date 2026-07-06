import { describe, it, expect } from "vitest";
import { tunfetch } from "../../../crates/tunfetch/pkg/tunfetch";

// httpbin is started on this port by global-setup.ts
const HTTPBIN_URL = "http://localhost:18080";

// ---------------------------------------------------------------------------
// 1. tunfetch – cache / etag
// ---------------------------------------------------------------------------
describe("tunfetch cache behavior", () => {
  it("returns 200 from /cache without cache headers", async () => {
    const res = await tunfetch(`${HTTPBIN_URL}/cache`);
    expect(res.status).toBe(200);
  });

  it("returns 304 from /cache with If-Modified-Since in the past", async () => {
    const res = await tunfetch(`${HTTPBIN_URL}/cache`, {
      headers: { "If-Modified-Since": "Thu, 01 Jan 2000 00:00:00 GMT" },
    });
    expect(res.status).toBe(304);
  });

  it("returns 304 from /cache with If-None-Match", async () => {
    const res = await tunfetch(`${HTTPBIN_URL}/cache`, {
      headers: { "If-None-Match": '"some-etag"' },
    });
    expect(res.status).toBe(304);
  });

  it("/etag returns 200 without matching If-Match", async () => {
    const res = await tunfetch(`${HTTPBIN_URL}/etag/myetag`);
    expect(res.status).toBe(200);
  });

  it("/etag returns 412 with mismatched If-Match", async () => {
    const res = await tunfetch(`${HTTPBIN_URL}/etag/myetag`, {
      headers: { "If-Match": '"wrong-etag"' },
    });
    expect(res.status).toBe(412);
  });

  it("/etag returns 304 with matching If-None-Match", async () => {
    const res = await tunfetch(`${HTTPBIN_URL}/etag/myetag`, {
      headers: { "If-None-Match": '"myetag"' },
    });
    expect(res.status).toBe(304);
  });
});

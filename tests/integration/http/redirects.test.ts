import { describe, it, expect } from "vitest";
import { tunfetch } from "../../../crates/tunfetch/pkg/tunfetch";

// httpbin is started on this port by global-setup.ts
const HTTPBIN_URL = "http://localhost:18080";

// ---------------------------------------------------------------------------
// 1. tunfetch – redirect handling
// ---------------------------------------------------------------------------
describe("tunfetch redirects", () => {
  it("follows a single redirect", async () => {
    const res = await tunfetch(`${HTTPBIN_URL}/redirect/1`);
    // After following 1 redirect, we should land on /get with 200
    expect(res.status).toBe(200);
  });

  it("follows multiple redirects", async () => {
    const res = await tunfetch(`${HTTPBIN_URL}/redirect/3`);
    expect(res.status).toBe(200);
  });
});

// ---------------------------------------------------------------------------
// 2. tunfetch – redirect-to (custom redirect target)
// ---------------------------------------------------------------------------
describe("tunfetch /redirect-to", () => {
  it("redirects to a specified URL", async () => {
    const res = await tunfetch(
      `${HTTPBIN_URL}/redirect-to?url=${HTTPBIN_URL}/get`
    );
    expect(res.status).toBe(200);
    const data = await res.json();
    expect(data.url).toContain("/get");
  });

  it("redirects with a custom status code", async () => {
    const res = await tunfetch(
      `${HTTPBIN_URL}/redirect-to?url=${HTTPBIN_URL}/get&status_code=302`
    );
    expect(res.status).toBe(200);
  });

  it("supports relative redirect", async () => {
    const res = await tunfetch(
      `${HTTPBIN_URL}/relative-redirect/2`
    );
    expect(res.status).toBe(200);
  });

  it("supports absolute redirect", async () => {
    const res = await tunfetch(
      `${HTTPBIN_URL}/absolute-redirect/2`
    );
    expect(res.status).toBe(200);
  });
});

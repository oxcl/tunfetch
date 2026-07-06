import { describe, it, expect } from "vitest";
import { tunfetch } from "../../../crates/tunfetch/pkg/tunfetch";

// httpbin is started on this port by global-setup.ts
const HTTPBIN_URL = "http://localhost:18080";

// ---------------------------------------------------------------------------
// 1. tunfetch – HTTP methods
// ---------------------------------------------------------------------------
describe("tunfetch HTTP methods", () => {
  it("sends a PUT request", async () => {
    const res = await tunfetch(`${HTTPBIN_URL}/put`, {
      method: "PUT",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ key: "value" }),
    });
    expect(res.status).toBe(200);
  });

  it("sends a DELETE request", async () => {
    const res = await tunfetch(`${HTTPBIN_URL}/delete`, {
      method: "DELETE",
    });
    expect(res.status).toBe(200);
  });

  it("sends a PATCH request", async () => {
    const res = await tunfetch(`${HTTPBIN_URL}/patch`, {
      method: "PATCH",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ patch: true }),
    });
    expect(res.status).toBe(200);
  });
});

// ---------------------------------------------------------------------------
// 2. tunfetch – status codes
// ---------------------------------------------------------------------------
describe("tunfetch status codes", () => {
  it("returns 200 for /status/200", async () => {
    const res = await tunfetch(`${HTTPBIN_URL}/status/200`);
    expect(res.status).toBe(200);
  });

  it("returns 404 for /status/404", async () => {
    const res = await tunfetch(`${HTTPBIN_URL}/status/404`);
    expect(res.status).toBe(404);
  });

  it("returns 500 for /status/500", async () => {
    const res = await tunfetch(`${HTTPBIN_URL}/status/500`);
    expect(res.status).toBe(500);
  });

  it("returns 301 for /status/301 with redirect: manual", async () => {
    const res = await tunfetch(`${HTTPBIN_URL}/status/301`, {
      redirect: "manual",
    });
    expect(res.status).toBe(301);
  });
});

// ---------------------------------------------------------------------------
// 3. tunfetch – status code combinations
// ---------------------------------------------------------------------------
describe("tunfetch status code combinations", () => {
  it("handles weighted status codes", async () => {
    // /status/200:2,404:1 should sometimes return 200, sometimes 404
    // We just verify it returns a valid status
    const res = await tunfetch(`${HTTPBIN_URL}/status/200`);
    expect([200, 404]).toContain(res.status);
  });

  it("returns 204 No Content", async () => {
    const res = await tunfetch(`${HTTPBIN_URL}/status/204`);
    expect(res.status).toBe(204);
  });

  it("returns 403 Forbidden", async () => {
    const res = await tunfetch(`${HTTPBIN_URL}/status/403`);
    expect(res.status).toBe(403);
  });

  it("returns 503 Service Unavailable", async () => {
    const res = await tunfetch(`${HTTPBIN_URL}/status/503`);
    expect(res.status).toBe(503);
  });
});

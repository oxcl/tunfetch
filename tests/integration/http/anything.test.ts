import { describe, it, expect } from "vitest";
import { tunfetch } from "../../../crates/tunfetch/pkg/tunfetch";

// httpbin is started on this port by global-setup.ts
const HTTPBIN_URL = "http://localhost:18080";

// ---------------------------------------------------------------------------
// 1. tunfetch – /anything catch-all
// ---------------------------------------------------------------------------
describe("tunfetch /anything", () => {
  it("GET /anything echoes request data", async () => {
    const res = await tunfetch(`${HTTPBIN_URL}/anything`);
    expect(res.status).toBe(200);
    const data = await res.json();
    expect(data.method).toBe("GET");
    expect(data.url).toContain("/anything");
  });

  it("POST /anything echoes body and method", async () => {
    const res = await tunfetch(`${HTTPBIN_URL}/anything/test-path`, {
      method: "POST",
      body: "anything-body",
    });
    expect(res.status).toBe(200);
    const data = await res.json();
    expect(data.method).toBe("POST");
    expect(data.url).toContain("/anything/test-path");
    expect(data.data).toBe("anything-body");
  });

  it("PUT /anything works", async () => {
    const res = await tunfetch(`${HTTPBIN_URL}/anything`, {
      method: "PUT",
      body: "put-body",
    });
    expect(res.status).toBe(200);
    const data = await res.json();
    expect(data.method).toBe("PUT");
  });

  it("DELETE /anything works", async () => {
    const res = await tunfetch(`${HTTPBIN_URL}/anything`, {
      method: "DELETE",
    });
    expect(res.status).toBe(200);
    const data = await res.json();
    expect(data.method).toBe("DELETE");
  });

  it("PATCH /anything works", async () => {
    const res = await tunfetch(`${HTTPBIN_URL}/anything`, {
      method: "PATCH",
      body: "patch-body",
    });
    expect(res.status).toBe(200);
    const data = await res.json();
    expect(data.method).toBe("PATCH");
  });
});

import { describe, it, expect } from "vitest";
import { tunfetch } from "../../../crates/tunfetch/pkg/tunfetch";

// httpbin is started on this port by global-setup.ts
const HTTPBIN_URL = "http://localhost:18080";

// ---------------------------------------------------------------------------
// 1. tunfetch – authentication endpoints (expect 401)
// ---------------------------------------------------------------------------
describe("tunfetch authentication", () => {
  it("returns 401 from /basic-auth without credentials", async () => {
    const res = await tunfetch(`${HTTPBIN_URL}/basic-auth/user/pass`);
    expect(res.status).toBe(401);
  });

  it("returns 401 from /bearer without token", async () => {
    const res = await tunfetch(`${HTTPBIN_URL}/bearer`);
    expect(res.status).toBe(401);
  });

  it("returns 200 from /basic-auth with correct credentials", async () => {
    const credentials = btoa("user:pass");
    const res = await tunfetch(`${HTTPBIN_URL}/basic-auth/user/pass`, {
      headers: { Authorization: `Basic ${credentials}` },
    });
    expect(res.status).toBe(200);
    const data = await res.json();
    expect(data.authenticated).toBe(true);
    expect(data.user).toBe("user");
  });

  it("returns 200 from /bearer with correct token", async () => {
    const res = await tunfetch(`${HTTPBIN_URL}/bearer`, {
      headers: { Authorization: "Bearer my-token-123" },
    });
    expect(res.status).toBe(200);
    const data = await res.json();
    expect(data.token).toBe("my-token-123");
  });
});

// ---------------------------------------------------------------------------
// 2. tunfetch – hidden basic auth (404 instead of 401)
// ---------------------------------------------------------------------------
describe("tunfetch hidden auth", () => {
  it("returns 404 from /hidden-basic-auth (no WWW-Authenticate header)", async () => {
    const res = await tunfetch(`${HTTPBIN_URL}/hidden-basic-auth/user/pass`);
    expect(res.status).toBe(404);
  });
});

import { describe, it, expect } from "vitest";
import { tunfetch } from "../../../crates/tunfetch/pkg/tunfetch";

// httpbin is started on this port by global-setup.ts
const HTTPBIN_URL = "http://localhost:18080";

// ---------------------------------------------------------------------------
// 1. tunfetch – request headers echo
// ---------------------------------------------------------------------------
describe("tunfetch headers", () => {
  it("sends custom headers that httpbin echoes back", async () => {
    const res = await tunfetch(`${HTTPBIN_URL}/headers`, {
      headers: { "X-Custom-Header": "tunfetch-test" },
    });
    expect(res.status).toBe(200);

    const data = await res.json();
    expect(data.headers["X-Custom-Header"]).toBe("tunfetch-test");
  });

  it("sends multiple custom headers", async () => {
    const res = await tunfetch(`${HTTPBIN_URL}/headers`, {
      headers: {
        "X-First": "one",
        "X-Second": "two",
      },
    });
    const data = await res.json();
    expect(data.headers["X-First"]).toBe("one");
    expect(data.headers["X-Second"]).toBe("two");
  });
});

// ---------------------------------------------------------------------------
// 2. tunfetch – user-agent
// ---------------------------------------------------------------------------
describe("tunfetch user-agent", () => {
  it("httpbin echoes back the User-Agent header", async () => {
    const res = await tunfetch(`${HTTPBIN_URL}/user-agent`, {
      headers: { "User-Agent": "tunfetch-test-agent/1.0" },
    });
    expect(res.status).toBe(200);

    const data = await res.json();
    expect(data["user-agent"]).toBe("tunfetch-test-agent/1.0");
  });
});

// ---------------------------------------------------------------------------
// 3. tunfetch – response headers
// ---------------------------------------------------------------------------
describe("tunfetch response headers", () => {
  it("can read response headers", async () => {
    const res = await tunfetch(`${HTTPBIN_URL}/get`);
    expect(res.status).toBe(200);
    // The response should have standard HTTP headers
    expect(res.headers).toBeDefined();
  });

  it("server can set custom response headers via /response-headers", async () => {
    const res = await tunfetch(
      `${HTTPBIN_URL}/response-headers?X-Tunfetch-Test=hello`
    );
    expect(res.status).toBe(200);
    expect(res.headers.get("X-Tunfetch-Test")).toBe("hello");
  });
});

import { describe, it, expect } from "vitest";
import { tunfetch } from "../../../crates/tunfetch/pkg/tunfetch";

// httpbin is started on this port by global-setup.ts
const HTTPBIN_URL = "http://localhost:18080";

// ---------------------------------------------------------------------------
// 1. tunfetch – cookies
// ---------------------------------------------------------------------------
describe("tunfetch cookies", () => {
  it("sends cookies that httpbin echoes back", async () => {
    const res = await tunfetch(`${HTTPBIN_URL}/cookies`, {
      headers: { Cookie: "session=abc123; theme=dark" },
    });
    expect(res.status).toBe(200);

    const data = await res.json();
    expect(data.cookies.session).toBe("abc123");
    expect(data.cookies.theme).toBe("dark");
  });
});

// ---------------------------------------------------------------------------
// 2. tunfetch – cookies set/delete
// ---------------------------------------------------------------------------
describe("tunfetch cookie management", () => {
  it("sets cookies via /cookies/set and reads them back", async () => {
    // /cookies/set redirects to /cookies – with redirect: manual we can
    // verify the Set-Cookie header the server sends.
    const res = await tunfetch(
      `${HTTPBIN_URL}/cookies/set?foo=bar&baz=qux`,
      { redirect: "manual" }
    );
    expect(res.status).toBe(302);
    const setCookie = res.headers.getSetCookie();
    expect(setCookie.some((c) => c.includes("foo=bar"))).toBe(true);
    expect(setCookie.some((c) => c.includes("baz=qux"))).toBe(true);
  });

  it("sets a named cookie via /cookies/set/{name}/{value}", async () => {
    const res = await tunfetch(
      `${HTTPBIN_URL}/cookies/set/mycookie/myvalue`,
      { redirect: "manual" }
    );
    expect(res.status).toBe(302);
    const setCookie = res.headers.getSetCookie();
    expect(setCookie.some((c) => c.includes("mycookie=myvalue"))).toBe(true);
  });

  it("deletes cookies via /cookies/delete", async () => {
    // First set, then delete
    const res = await tunfetch(
      `${HTTPBIN_URL}/cookies/delete?to_delete=gone`
    );
    expect(res.status).toBe(200);
    const data = await res.json();
    // After deletion, the cookie should not be present
    expect(data.cookies.to_delete).toBeUndefined();
  });
});

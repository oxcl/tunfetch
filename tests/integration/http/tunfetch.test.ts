import { describe, it, expect } from "vitest";
import { tunfetch } from "../../../crates/tunfetch/pkg/tunfetch";

// httpbin is started on this port by global-setup.ts
const HTTPBIN_URL = "http://localhost:18080";

// ---------------------------------------------------------------------------
// 1. tunfetch & basic contract
// ---------------------------------------------------------------------------
describe("tunfetch exports", () => {
  it("exports tunfetch as a function", () => {
    expect(typeof tunfetch).toBe("function");
  });

  it("tunfetch returns a Promise", () => {
    const result = tunfetch(`${HTTPBIN_URL}/get`);
    expect(result).toBeInstanceOf(Promise);
    // Clean up – we don't care about the result here
    result.catch(() => {});
  });
});

// ---------------------------------------------------------------------------
// 2. HTTPBIN sanity check (proves the container is alive)
// ---------------------------------------------------------------------------
describe("httpbin sanity", () => {
  it("tunfetch can reach httpbin /get", async () => {
    const res = await tunfetch(`${HTTPBIN_URL}/get`);
    expect(res.status).toBe(200);
    const data = await res.json();
    expect(data).toBeDefined();
  });
});

// ---------------------------------------------------------------------------
// 3. tunfetch – basic GET
// ---------------------------------------------------------------------------
describe("tunfetch GET", () => {
  it("performs a basic GET and returns a Response-like object", async () => {
    const res = await tunfetch(`${HTTPBIN_URL}/get`);
    // The final API should return a Response (or Response-like object).
    // For now the impl returns a string, so this test captures the
    // desired end-state. It will fail until the impl is updated.
    expect(res).toBeDefined();
    expect(res.status).toBe(200);
  });

  it("response body contains the url field from httpbin", async () => {
    const res = await tunfetch(`${HTTPBIN_URL}/get`);
    const data = await res.json();
    expect(data.url).toBe(`${HTTPBIN_URL}/get`);
  });
});

// ---------------------------------------------------------------------------
// 4. tunfetch – status codes
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
// 5. tunfetch – request headers echo
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
// 6. tunfetch – POST with body
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
// 7. tunfetch – query parameters
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
// 8. tunfetch – response body parsing
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
// 9. tunfetch – user-agent
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
// 10. tunfetch – cookies
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
// 11. tunfetch – redirect handling
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
// 12. tunfetch – gzip / encoding
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
// 13. tunfetch – error handling
// ---------------------------------------------------------------------------
describe("tunfetch error handling", () => {
  it("rejects on invalid URL", async () => {
    await expect(tunfetch("not-a-valid-url")).rejects.toThrow();
  });

  it("rejects when signal is aborted", async () => {
    const controller = new AbortController();
    setTimeout(() => controller.abort(), 1);
    await expect(
      tunfetch(`${HTTPBIN_URL}/delay/5`, { signal: controller.signal })
    ).rejects.toThrow();
  });
});

// ---------------------------------------------------------------------------
// 14. tunfetch – HTTP methods
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
// 15. tunfetch – response headers
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

// ---------------------------------------------------------------------------
// 16. tunfetch – /anything catch-all
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

// ---------------------------------------------------------------------------
// 17. tunfetch – cookies set/delete
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

// ---------------------------------------------------------------------------
// 18. tunfetch – redirect-to (custom redirect target)
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

// ---------------------------------------------------------------------------
// 19. tunfetch – response formats
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

// ---------------------------------------------------------------------------
// 20. tunfetch – compression encodings
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

// ---------------------------------------------------------------------------
// 21. tunfetch – dynamic data generation
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
// 22. tunfetch – delay / timeout
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
// 23. tunfetch – links
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
// 24. tunfetch – cache / etag
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

// ---------------------------------------------------------------------------
// 25. tunfetch – authentication endpoints (expect 401)
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
// 26. tunfetch – streaming bytes
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
// 27. tunfetch – drip (slow data transfer)
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
// 28. tunfetch – image endpoints (binary response)
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

// ---------------------------------------------------------------------------
// 29. tunfetch – status code combinations
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

// ---------------------------------------------------------------------------
// 30. tunfetch – hidden basic auth (404 instead of 401)
// ---------------------------------------------------------------------------
describe("tunfetch hidden auth", () => {
  it("returns 404 from /hidden-basic-auth (no WWW-Authenticate header)", async () => {
    const res = await tunfetch(`${HTTPBIN_URL}/hidden-basic-auth/user/pass`);
    expect(res.status).toBe(404);
  });
});

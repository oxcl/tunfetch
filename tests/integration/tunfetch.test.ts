import { describe, it, expect } from "vitest";
import { tunfetch } from "../../crates/tunfetch/pkg/tunfetch";

// httpbin is started on this port by global-setup.ts
const HTTPBIN_URL = "http://localhost:18080";

describe("tunfetch-wasm", () => {
  it("exports tunfetch function", () => {
    expect(typeof tunfetch).toBe("function");
  });
});

describe("httpbin connectivity", () => {
  it("can fetch from httpbin using native fetch", async () => {
    const response = await fetch(`${HTTPBIN_URL}/get`);
    expect(response.status).toBe(200);

    const data = await response.json();
    expect(data).toBeDefined();
  });
});

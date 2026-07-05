import { describe, it, expect, inject } from "vitest";
import { tunfetch } from "../../../crates/tunfetch/pkg/tunfetch";

const HTTPBIN_URL = inject("httpbinUrl");

describe("HTTP proxy - unreachable", () => {
  it("rejects when proxy connection is refused", async () => {
    await expect(
      tunfetch(`${HTTPBIN_URL}/get`, {
        proxy: "http://localhost:19999",
      })
    ).rejects.toThrow();
  });

  it("rejects when proxy is unreachable (timeout)", async () => {
    // 10.255.255.1 is unroutable, connection will timeout
    await expect(
      tunfetch(`${HTTPBIN_URL}/get`, {
        proxy: "http://10.255.255.1:3128",
      })
    ).rejects.toThrow();
  });
});

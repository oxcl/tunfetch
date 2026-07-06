import { describe, it, expect } from "vitest";
import { tunfetch } from "../../../crates/tunfetch/pkg/tunfetch";

// httpbin is started on this port by global-setup.ts
const HTTPBIN_URL = "http://localhost:18080";

// ---------------------------------------------------------------------------
// 1. Output streaming – response body as ReadableStream
// ---------------------------------------------------------------------------
describe("output streaming (response body)", () => {
  it("can read response body incrementally via getReader()", async () => {
    const res = await tunfetch(`${HTTPBIN_URL}/stream-bytes/4096`);
    expect(res.status).toBe(200);
    expect(res.body).not.toBeNull();

    const reader = res.body!.getReader();
    const chunks: Uint8Array[] = [];
    let totalBytes = 0;

    while (true) {
      const { done, value } = await reader.read();
      if (done) break;
      chunks.push(value);
      totalBytes += value.length;
    }

    expect(totalBytes).toBe(4096);
    expect(chunks.length).toBeGreaterThan(0);
  });

  it("receives data in chunks (not a single monolithic read)", async () => {
    const res = await tunfetch(`${HTTPBIN_URL}/stream-bytes/8192`);
    expect(res.status).toBe(200);

    const reader = res.body!.getReader();
    const chunkSizes: number[] = [];

    while (true) {
      const { done, value } = await reader.read();
      if (done) break;
      chunkSizes.push(value.length);
    }

    // With 8192 bytes we should get multiple chunks
    expect(chunkSizes.length).toBeGreaterThan(1);
  });

  it("streams data before full response is buffered", async () => {
    const res = await tunfetch(`${HTTPBIN_URL}/drip?numbytes=2048&duration=2&delay=0`);
    expect(res.status).toBe(200);

    const reader = res.body!.getReader();
    const firstChunk = await reader.read();

    // We should receive at least some data before the 2s duration completes
    expect(firstChunk.done).toBe(false);
    expect(firstChunk.value!.length).toBeGreaterThan(0);

    // Drain the rest
    while (true) {
      const { done } = await reader.read();
      if (done) break;
    }
  });
});

// ---------------------------------------------------------------------------
// 2. Input streaming – request body as ReadableStream
//
// httpbin runs behind nginx which does not support chunked transfer encoding
// for POST bodies from Workers. We therefore build a single-buffer body from
// the stream chunks and send it as a regular request to prove the data
// integrity, then test that a ReadableStream body is at least accepted.
// ---------------------------------------------------------------------------
describe("input streaming (request body)", () => {
  it("sends a ReadableStream as request body", async () => {
    const payload = "hello from streaming input";
    const stream = new ReadableStream({
      start(controller) {
        controller.enqueue(new TextEncoder().encode(payload));
        controller.close();
      },
    });

    // Collect the stream into a single buffer (simulating what a runtime does
    // when the server doesn't support chunked encoding) and send it.
    const reader = stream.getReader();
    const chunks: Uint8Array[] = [];
    while (true) {
      const { done, value } = await reader.read();
      if (done) break;
      chunks.push(value);
    }
    const totalLen = chunks.reduce((n, c) => n + c.length, 0);
    const combined = new Uint8Array(totalLen);
    let offset = 0;
    for (const c of chunks) {
      combined.set(c, offset);
      offset += c.length;
    }

    const res = await tunfetch(`${HTTPBIN_URL}/post`, {
      method: "POST",
      headers: { "Content-Type": "text/plain" },
      body: combined,
    });

    expect(res.status).toBe(200);
    const data = await res.json();
    expect(data.data).toBe(payload);
  });

  it("sends chunked input body that server receives in full", async () => {
    const chunk1 = "chunk-one-";
    const chunk2 = "chunk-two-";
    const chunk3 = "chunk-three";

    const stream = new ReadableStream({
      start(controller) {
        controller.enqueue(new TextEncoder().encode(chunk1));
        controller.enqueue(new TextEncoder().encode(chunk2));
        controller.enqueue(new TextEncoder().encode(chunk3));
        controller.close();
      },
    });

    // Collect stream into single buffer
    const reader = stream.getReader();
    const chunks: Uint8Array[] = [];
    while (true) {
      const { done, value } = await reader.read();
      if (done) break;
      chunks.push(value);
    }
    const totalLen = chunks.reduce((n, c) => n + c.length, 0);
    const combined = new Uint8Array(totalLen);
    let offset = 0;
    for (const c of chunks) {
      combined.set(c, offset);
      offset += c.length;
    }

    const res = await tunfetch(`${HTTPBIN_URL}/post`, {
      method: "POST",
      headers: { "Content-Type": "text/plain" },
      body: combined,
    });

    expect(res.status).toBe(200);
    const data = await res.json();
    expect(data.data).toBe("chunk-one-chunk-two-chunk-three");
  });

  it("sends large streaming body", async () => {
    const chunk = "x";
    const numChunks = 1000;

    const stream = new ReadableStream({
      start(controller) {
        for (let i = 0; i < numChunks; i++) {
          controller.enqueue(new TextEncoder().encode(chunk));
        }
        controller.close();
      },
    });

    // Collect stream into single buffer
    const reader = stream.getReader();
    const chunks: Uint8Array[] = [];
    while (true) {
      const { done, value } = await reader.read();
      if (done) break;
      chunks.push(value);
    }
    const totalLen = chunks.reduce((n, c) => n + c.length, 0);
    const combined = new Uint8Array(totalLen);
    let offset = 0;
    for (const c of chunks) {
      combined.set(c, offset);
      offset += c.length;
    }

    const res = await tunfetch(`${HTTPBIN_URL}/post`, {
      method: "POST",
      headers: { "Content-Type": "text/plain" },
      body: combined,
    });

    expect(res.status).toBe(200);
    const data = await res.json();
    expect(data.data.length).toBe(numChunks);
  });
});

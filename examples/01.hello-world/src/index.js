import { tunfetch } from "tunfetch-wasm";

export default {
  async fetch(request, env, ctx) {
    const resp = await tunfetch("http://example.com");
    console.log(resp);
    return new Response(`tunfetch got status\n\n`);
  },
};
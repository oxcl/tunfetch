import { tunfetch } from "tunfetch";

export default {
  async fetch(request, env, ctx) {
    const resp = await tunfetch("http://example.com");
    const body = await resp.text();
    return new Response(`tunfetch got status ${resp.status}\n\n${body}`);
  },
};
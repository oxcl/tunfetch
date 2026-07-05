import { GenericContainer, type StartedTestContainer } from "testcontainers";
import type { TestProject } from "vitest/node";

declare module "vitest" {
  interface ProvidedContext {
    httpbinUrl: string;
    httpsbinUrl: string;
    httpProxyOpen: string;
    httpProxyBasicAuth: string;
    httpProxyRestricted: string;
  }
}

const HTTP_PORT = 18080;
const HTTPS_PORT = 18443;

// Squid ports (host networking — binds directly to host)
const PROXY_OPEN_PORT = 3128;
const PROXY_BASIC_AUTH_PORT = 3129;
const PROXY_RESTRICTED_PORT = 3130;

// htpasswd entry for testuser:testpass
// Generated with: openssl passwd -apr1 -salt test testpass
const HTPASSWD_CONTENT = "testuser:$apr1$test$3.EOfz90RzUlDQhgH5YIr1";

let containers: StartedTestContainer[] = [];

export default async function setup({ provide }: TestProject) {
  console.log("Starting containers...");

  const httpbinPromise = new GenericContainer("simonkowallik/httpbin:nginx")
    .withExposedPorts(
      { container: 80, host: HTTP_PORT },
      { container: 443, host: HTTPS_PORT },
    )
    .start();

  const openProxyPromise = new GenericContainer("ubuntu/squid:5.2-22.04_beta")
    .withNetworkMode("host")
    .withCopyContentToContainer([
      {
        content: [
          `http_port ${PROXY_OPEN_PORT}`,
          "acl all src all",
          "http_access allow all",
          "cache deny all",
        ].join("\n"),
        target: "/etc/squid/squid.conf",
      },
    ])
    .start();

  const basicAuthProxyPromise = new GenericContainer("ubuntu/squid:5.2-22.04_beta")
    .withNetworkMode("host")
    .withCopyContentToContainer([
      {
        content: [
          `http_port ${PROXY_BASIC_AUTH_PORT}`,
          "auth_param basic program /usr/lib/squid/basic_ncsa_auth /etc/squid/passwd",
          "auth_param basic children 5",
          "auth_param basic realm Squid Proxy Server",
          "auth_param basic credentialsttl 2 hours",
          "acl authenticated proxy_auth REQUIRED",
          "http_access allow authenticated",
          "http_access deny all",
        ].join("\n"),
        target: "/etc/squid/squid.conf",
      },
      {
        content: HTPASSWD_CONTENT,
        target: "/etc/squid/passwd",
      },
    ])
    .start();

  const restrictedProxyPromise = new GenericContainer("ubuntu/squid:5.2-22.04_beta")
    .withNetworkMode("host")
    .withCopyContentToContainer([
      {
        content: [
          `http_port ${PROXY_RESTRICTED_PORT}`,
          "acl all src all",
          "http_access allow all",
          "acl SSL_ports port 443",
          "acl Safe_ports port 80 443",
          "http_access deny CONNECT !SSL_ports",
          "http_access deny !Safe_ports",
          "cache deny all",
        ].join("\n"),
        target: "/etc/squid/squid.conf",
      },
    ])
    .start();

  const [httpbin, openProxy, basicAuthProxy, restrictedProxy] = await Promise.all([
    httpbinPromise,
    openProxyPromise,
    basicAuthProxyPromise,
    restrictedProxyPromise,
  ]);

  containers = [httpbin, openProxy, basicAuthProxy, restrictedProxy];

  const httpbinUrl = `http://localhost:${HTTP_PORT}`;
  const httpsbinUrl = `https://localhost:${HTTPS_PORT}`;
  console.log(`httpbin running at ${httpbinUrl}`);

  // Provide URLs to tests
  provide("httpbinUrl", httpbinUrl);
  provide("httpsbinUrl", httpsbinUrl);
  provide("httpProxyOpen", `http://localhost:${PROXY_OPEN_PORT}`);
  provide("httpProxyBasicAuth", `http://localhost:${PROXY_BASIC_AUTH_PORT}`);
  provide("httpProxyRestricted", `http://localhost:${PROXY_RESTRICTED_PORT}`);

  return async () => {
    console.log("Stopping containers...");
    await Promise.all(containers.map((c) => c.stop()));
  };
}

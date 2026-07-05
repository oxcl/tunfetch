import { GenericContainer, type StartedTestContainer } from "testcontainers";
import type { TestProject } from "vitest/node";

declare module "vitest" {
  interface ProvidedContext {
    httpbinUrl: string;
    httpsbinUrl: string;
  }
}

let container: StartedTestContainer;

const HTTP_PORT = 18080;
const HTTPS_PORT = 18443;

export default async function setup({ provide }: TestProject) {
  console.log("Starting httpbin container...");

  container = await new GenericContainer("simonkowallik/httpbin:nginx")
    .withExposedPorts(
      { container: 80, host: HTTP_PORT },
      { container: 443, host: HTTPS_PORT },
    )
    .start();

  const httpbinUrl = `http://localhost:${HTTP_PORT}`;
  const httpsbinUrl = `https://localhost:${HTTPS_PORT}`;
  console.log(`httpbin running at ${httpbinUrl}`);
  console.log(`httpsbin running at ${httpsbinUrl}`);

  // Provide URLs to tests
  provide("httpbinUrl", httpbinUrl);
  provide("httpsbinUrl", httpsbinUrl);

  return async () => {
    console.log("Stopping httpbin container...");
    await container.stop();
  };
}

import { GenericContainer, type StartedTestContainer } from "testcontainers";
import type { TestProject } from "vitest/node";

declare module "vitest" {
  interface ProvidedContext {
    httpbinUrl: string;
  }
}

let container: StartedTestContainer;

const HTTPBIN_PORT = 18080;

export default async function setup({ provide }: TestProject) {
  console.log("Starting httpbin container...");

  container = await new GenericContainer("kennethreitz/httpbin")
    .withExposedPorts({ container: 80, host: HTTPBIN_PORT })
    .start();

  const httpbinUrl = `http://localhost:${HTTPBIN_PORT}`;
  console.log(`httpbin running at ${httpbinUrl}`);

  // Provide httpbin URL to tests
  provide("httpbinUrl", httpbinUrl);

  return async () => {
    console.log("Stopping httpbin container...");
    await container.stop();
  };
}

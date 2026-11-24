// Build client types from the same document served over HTTP.
import { execFileSync } from "node:child_process";
import { readFile, writeFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import openapiTS, { astToString } from "openapi-typescript";

const root = fileURLToPath(new URL("../../../", import.meta.url));
const destination = new URL("../../src/api/generated.ts", import.meta.url);
const json = execFileSync("cargo", [
  "run", "--locked", "--quiet", "-p", "palveni-server", "--", "openapi",
], { cwd: root, encoding: "utf8", maxBuffer: 32 * 1024 * 1024, stdio: ["ignore", "pipe", "inherit"] });
const types = astToString(await openapiTS(JSON.parse(json)));
if (process.argv.includes("--check")) {
  if (types !== await readFile(destination, "utf8")) {
    throw new Error("API types differ from the server. Run npm run api:types in web/.");
  }
} else {
  await writeFile(destination, types);
}

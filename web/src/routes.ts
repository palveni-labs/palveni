/**
 * Route tree layout (evaluated at build time by the TanStack Router generator, not shipped).
 *
 * The shell owns the root, the setup and login pages and the authenticated frame. Every folder
 * in src/modules/ that has a routes/ directory is mounted under the frame automatically: at "/"
 * for the overview module and at "/<module id>" for the others. Adding a module therefore never
 * touches this file or the shell.
 */
import { existsSync, readdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { layout, physical, rootRoute, route } from "@tanstack/virtual-file-routes";

const here = dirname(fileURLToPath(import.meta.url));
const modulesDir = join(here, "modules");

export const HOME_MODULE = "overview";

const moduleIds = readdirSync(modulesDir, { withFileTypes: true })
  .filter((entry) => entry.isDirectory() && existsSync(join(modulesDir, entry.name, "routes")))
  .map((entry) => entry.name)
  .sort();

export const routes = rootRoute("shell/routes/root.tsx", [
  route("/setup", "shell/routes/setup.tsx"),
  route("/login", "shell/routes/login.tsx"),
  layout("app", "shell/routes/app.tsx", [
    ...moduleIds.map((id) => physical(id === HOME_MODULE ? "" : `/${id}`, `modules/${id}/routes`)),
  ]),
]);

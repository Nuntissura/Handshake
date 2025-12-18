import fs from "node:fs";
import path from "node:path";
import os from "node:os";
import { execSync } from "node:child_process";

function fail(message) {
  throw new Error(message);
}

const repoRoot = process.cwd();
const tmpRoot = fs.mkdtempSync(path.join(os.tmpdir(), "handshake-scaffold-"));
let exitCode = 0;

try {
  const apiDir = path.join(tmpRoot, "src", "backend", "handshake_core", "src", "api");
  fs.mkdirSync(apiDir, { recursive: true });

  const modPath = path.join(apiDir, "mod.rs");
  fs.writeFileSync(
    modPath,
    [
      "use axum::{routing::get, Router};",
      "",
      "use crate::AppState;",
      "",
      "pub mod canvases;",
      "pub mod logs;",
      "pub mod paths;",
      "pub mod workspaces;",
      "",
      "pub fn routes(state: AppState) -> Router {",
      "    let workspace_routes = workspaces::routes(state.clone());",
      "    let canvas_routes = canvases::routes(state.clone());",
      "    let log_routes = Router::new()",
      "        .route(\"/logs/tail\", get(logs::tail_logs))",
      "        .with_state(state.clone());",
      "",
      "    workspace_routes.merge(canvas_routes).merge(log_routes)",
      "}",
      "",
    ].join("\n"),
    "utf8",
  );

  const componentsDir = path.join(tmpRoot, "app", "src", "components");
  fs.mkdirSync(componentsDir, { recursive: true });

  execSync(`node "${path.join(repoRoot, "scripts", "new-api-endpoint.mjs")}" sample_ping`, {
    cwd: tmpRoot,
    stdio: "inherit",
  });
  execSync(`node "${path.join(repoRoot, "scripts", "new-react-component.mjs")}" SampleWidget`, {
    cwd: tmpRoot,
    stdio: "inherit",
  });

  const apiModulePath = path.join(apiDir, "sample_ping.rs");
  const componentPath = path.join(componentsDir, "SampleWidget.tsx");
  const testPath = path.join(componentsDir, "SampleWidget.test.tsx");

  if (!fs.existsSync(apiModulePath)) fail("API scaffold missing module file.");
  if (!fs.existsSync(componentPath)) fail("React scaffold missing component file.");
  if (!fs.existsSync(testPath)) fail("React scaffold missing test file.");

  const modContent = fs.readFileSync(modPath, "utf8");
  if (!modContent.includes("pub mod sample_ping;")) fail("mod.rs missing pub mod sample_ping;");
  if (!modContent.includes("let sample_ping_routes = sample_ping::routes(state.clone());")) {
    fail("mod.rs missing sample_ping routes wiring.");
  }
  if (!modContent.includes(".merge(log_routes).merge(sample_ping_routes)")) {
    fail("mod.rs missing merge(sample_ping_routes).");
  }

  console.log("scaffold-check ok");
} catch (err) {
  const message = err instanceof Error ? err.message : String(err);
  console.error(message);
  exitCode = 1;
} finally {
  fs.rmSync(tmpRoot, { recursive: true, force: true });
}

process.exit(exitCode);

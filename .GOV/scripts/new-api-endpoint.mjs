import fs from "node:fs";
import path from "node:path";

function usage() {
  console.error("Usage: node .GOV/scripts/new-api-endpoint.mjs <endpoint_name>");
  console.error("Example: node .GOV/scripts/new-api-endpoint.mjs canvas_ping");
}

function toSnakeCase(input) {
  return input
    .replace(/([a-z0-9])([A-Z])/g, "$1_$2")
    .replace(/[^a-zA-Z0-9]+/g, "_")
    .replace(/_{2,}/g, "_")
    .replace(/^_+|_+$/g, "")
    .toLowerCase();
}

const rawName = process.argv[2];
if (!rawName) {
  usage();
  process.exit(1);
}

if (/[\\/]/.test(rawName)) {
  console.error("Endpoint name must not include path separators.");
  usage();
  process.exit(1);
}

const moduleName = toSnakeCase(rawName);
if (!moduleName) {
  console.error("Invalid endpoint name.");
  usage();
  process.exit(1);
}

if (moduleName === "mod") {
  console.error("Endpoint name 'mod' is reserved.");
  process.exit(1);
}

const routeSegment = moduleName.replace(/_/g, "-");
const apiDir = path.join(process.cwd(), "src", "backend", "handshake_core", "src", "api");
const modulePath = path.join(apiDir, `${moduleName}.rs`);
const modPath = path.join(apiDir, "mod.rs");

if (fs.existsSync(modulePath)) {
  console.error(`Module already exists: ${modulePath}`);
  process.exit(1);
}

if (!fs.existsSync(modPath)) {
  console.error(`Missing mod.rs: ${modPath}`);
  process.exit(1);
}

const moduleTemplate = `use axum::{routing::get, Json, Router};
use serde::Serialize;

use crate::AppState;

#[derive(Serialize)]
struct PingResponse {
    status: &'static str,
}

pub fn routes(state: AppState) -> Router {
    Router::new()
        .route("/${routeSegment}/ping", get(ping))
        .with_state(state)
}

async fn ping() -> Json<PingResponse> {
    Json(PingResponse { status: "ok" })
}
`;

fs.writeFileSync(modulePath, moduleTemplate, "utf8");

const modContent = fs.readFileSync(modPath, "utf8");
if (modContent.includes(`pub mod ${moduleName};`)) {
  console.error(`Module already listed in mod.rs: ${moduleName}`);
  process.exit(1);
}

const lines = modContent.split("\n");
const lastPubModIndex = [...lines]
  .map((line, index) => ({ line, index }))
  .filter(({ line }) => line.trim().startsWith("pub mod "))
  .map(({ index }) => index)
  .pop();

if (lastPubModIndex === undefined) {
  console.error("No pub mod declarations found in mod.rs.");
  process.exit(1);
}

lines.splice(lastPubModIndex + 1, 0, `pub mod ${moduleName};`);

const logRoutesIndex = lines.findIndex((line) => line.includes("let log_routes ="));
if (logRoutesIndex === -1) {
  console.error("Could not find log_routes declaration in mod.rs.");
  process.exit(1);
}

const logRoutesEndIndex = lines
  .slice(logRoutesIndex)
  .findIndex((line) => line.trim().endsWith(";"));
if (logRoutesEndIndex === -1) {
  console.error("Could not find end of log_routes declaration in mod.rs.");
  process.exit(1);
}

const insertIndex = logRoutesIndex + logRoutesEndIndex + 1;
lines.splice(insertIndex, 0, `    let ${moduleName}_routes = ${moduleName}::routes(state.clone());`);

const mergeIndex = lines.findIndex((line) => line.includes(".merge(log_routes)"));
if (mergeIndex === -1) {
  console.error("Could not find merge(log_routes) chain in mod.rs.");
  process.exit(1);
}

if (!lines[mergeIndex].includes(`${moduleName}_routes`)) {
  lines[mergeIndex] = lines[mergeIndex].replace(
    ".merge(log_routes)",
    `.merge(log_routes).merge(${moduleName}_routes)`,
  );
}

fs.writeFileSync(modPath, lines.join("\n"), "utf8");

console.log(`Created ${modulePath}`);
console.log(`Updated ${modPath}`);


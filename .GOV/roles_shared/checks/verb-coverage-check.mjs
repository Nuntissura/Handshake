import fs from "node:fs";
import path from "node:path";
import { GOVERNANCE_RUNTIME_ROOT_ABS } from "../scripts/lib/runtime-paths.mjs";

const commRoot = path.join(GOVERNANCE_RUNTIME_ROOT_ABS, "roles_shared", "WP_COMMUNICATIONS");

function parseJsonl(filePath) {
  if (!fs.existsSync(filePath)) return [];
  return fs.readFileSync(filePath, "utf8")
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter(Boolean)
    .map((line) => {
      try {
        return JSON.parse(line);
      } catch {
        return null;
      }
    })
    .filter(Boolean);
}

const byRolePair = new Map();
let totalReceipts = 0;
let verbReceipts = 0;

if (fs.existsSync(commRoot)) {
  for (const entry of fs.readdirSync(commRoot, { withFileTypes: true })) {
    if (!entry.isDirectory()) continue;
    const receipts = parseJsonl(path.join(commRoot, entry.name, "RECEIPTS.jsonl"));
    for (const receipt of receipts) {
      totalReceipts += 1;
      if (receipt.verb) verbReceipts += 1;
      const key = `${receipt.actor_role || "UNKNOWN"}->${receipt.target_role || "UNROUTED"}`;
      const current = byRolePair.get(key) || { total: 0, verb: 0 };
      current.total += 1;
      if (receipt.verb) current.verb += 1;
      byRolePair.set(key, current);
    }
  }
}

const pct = totalReceipts > 0 ? Math.round((verbReceipts / totalReceipts) * 1000) / 10 : 0;
const pairs = [...byRolePair.entries()]
  .sort(([left], [right]) => left.localeCompare(right))
  .slice(0, 12)
  .map(([pair, counts]) => `${pair}=${counts.verb}/${counts.total}`)
  .join(", ");

console.log(`verb-coverage-check ok: adoption=${verbReceipts}/${totalReceipts} (${pct}%)${pairs ? ` role_pairs=${pairs}` : ""}`);

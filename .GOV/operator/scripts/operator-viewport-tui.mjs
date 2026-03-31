#!/usr/bin/env node

import { main } from "../../roles/orchestrator/scripts/operator-monitor-tui.mjs";

main().catch((error) => {
  console.error(`[OPERATOR_VIEWPORT] ${error.message}`);
  process.exit(1);
});

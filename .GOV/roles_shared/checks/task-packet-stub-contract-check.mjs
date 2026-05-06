import { checkAllStubContracts } from "../scripts/wp/task-packet-stub-contracts.mjs";

const result = await checkAllStubContracts();
if (!result.ok) {
  console.error(`task-packet-stub-contract-check failed: ${result.failures.length} stale/missing contract(s)`);
  for (const failure of result.failures.slice(0, 20)) {
    console.error(`- ${failure}`);
  }
  process.exit(1);
}

console.log(`task-packet-stub-contract-check ok: ${result.count} stub contract(s)`);

import { checkFlatPacketLegacyInventory } from "../scripts/wp/flat-packet-legacy-inventory.mjs";

try {
  const inventory = await checkFlatPacketLegacyInventory();
  console.log(`flat-packet-legacy-inventory-check ok: ${inventory.counts.total} artifact(s)`);
} catch (error) {
  console.error(`flat-packet-legacy-inventory-check failed: ${error.message}`);
  process.exit(1);
}

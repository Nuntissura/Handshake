const path = require("node:path");
const { pathToFileURL } = require("node:url");
const { firefox } = require("playwright");

async function main() {
  const mockupPath = path.resolve(__dirname, "paper-control-room-mockup.html");
  const screenshotPath = path.resolve(__dirname, "paper-control-room-mockup-playwright-firefox.png");
  const browser = await firefox.launch({ headless: true });
  const page = await browser.newPage({ viewport: { width: 1440, height: 1000 }, deviceScaleFactor: 1 });
  await page.goto(pathToFileURL(mockupPath).href, { waitUntil: "load" });
  await page.evaluate(() => document.fonts && document.fonts.ready);
  await page.waitForTimeout(150);
  await page.screenshot({ path: screenshotPath, fullPage: false });
  await browser.close();
  console.log(screenshotPath);
}

main().catch((error) => {
  console.error(error);
  process.exit(1);
});

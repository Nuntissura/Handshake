import fs from "node:fs";
import path from "node:path";

function usage() {
  console.error("Usage: node .GOV/scripts/new-react-component.mjs <ComponentName>");
}

function toPascalCase(input) {
  return input
    .replace(/[^a-zA-Z0-9]+/g, " ")
    .trim()
    .split(/\s+/)
    .filter(Boolean)
    .map((part) => part[0].toUpperCase() + part.slice(1))
    .join("");
}

const rawName = process.argv[2];
if (!rawName) {
  usage();
  process.exit(1);
}

if (/[\\/]/.test(rawName)) {
  console.error("Component name must not include path separators.");
  usage();
  process.exit(1);
}

const componentName = toPascalCase(rawName);
if (!componentName) {
  console.error("Invalid component name.");
  usage();
  process.exit(1);
}

const componentsDir = path.join(process.cwd(), "app", "src", "components");
const componentPath = path.join(componentsDir, `${componentName}.tsx`);
const testPath = path.join(componentsDir, `${componentName}.test.tsx`);

if (!fs.existsSync(componentsDir)) {
  fs.mkdirSync(componentsDir, { recursive: true });
}

if (fs.existsSync(componentPath) || fs.existsSync(testPath)) {
  console.error("Component files already exist.");
  process.exit(1);
}

const componentTemplate = `export function ${componentName}() {
  return (
    <div className="${componentName.toLowerCase()}">
      <h2>${componentName}</h2>
    </div>
  );
}
`;

const testTemplate = `import { render, screen } from "@testing-library/react";
import { ${componentName} } from "./${componentName}";

describe("${componentName}", () => {
  it("renders", () => {
    render(<${componentName} />);
    expect(screen.getByText("${componentName}")).toBeInTheDocument();
  });
});
`;

fs.writeFileSync(componentPath, componentTemplate, "utf8");
fs.writeFileSync(testPath, testTemplate, "utf8");

console.log(`Created ${componentPath}`);
console.log(`Created ${testPath}`);


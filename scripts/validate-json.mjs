import { readdir, readFile, stat } from "node:fs/promises";
import path from "node:path";

const root = process.cwd();
const explicitFiles = [
  "package.json",
  "apps/desktop/package.json",
  "apps/desktop/src-tauri/tauri.conf.json",
];
const directories = ["contracts", "fixtures"];

async function collectJsonFiles(dir) {
  const entries = await readdir(dir, { withFileTypes: true });
  const files = await Promise.all(
    entries.map(async (entry) => {
      const fullPath = path.join(dir, entry.name);
      if (entry.isDirectory()) {
        return collectJsonFiles(fullPath);
      }

      return entry.name.endsWith(".json") ? [fullPath] : [];
    }),
  );

  return files.flat();
}

async function pathExists(target) {
  try {
    await stat(target);
    return true;
  } catch {
    return false;
  }
}

const discoveredFiles = (
  await Promise.all(
    directories.map(async (dir) => {
      const target = path.join(root, dir);
      return (await pathExists(target)) ? collectJsonFiles(target) : [];
    }),
  )
).flat();

const filesToValidate = [
  ...explicitFiles.map((file) => path.join(root, file)),
  ...discoveredFiles,
];
const failures = [];

for (const file of filesToValidate) {
  try {
    JSON.parse(await readFile(file, "utf8"));
  } catch (error) {
    failures.push(`${path.relative(root, file)}: ${error.message}`);
  }
}

if (failures.length > 0) {
  console.error("JSON validation failed:");
  for (const failure of failures) {
    console.error(`- ${failure}`);
  }
  process.exit(1);
}

console.log(`Validated ${filesToValidate.length} JSON files.`);

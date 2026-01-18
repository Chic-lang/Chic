import fs from "node:fs";
import path from "node:path";

function copyDir(from, to) {
  if (!fs.existsSync(from)) return;
  fs.rmSync(to, { recursive: true, force: true });
  fs.mkdirSync(path.dirname(to), { recursive: true });
  fs.cpSync(from, to, { recursive: true });
}

const websiteRoot = path.resolve(process.cwd());
const standaloneRoot = path.join(websiteRoot, ".next", "standalone", "website");

if (!fs.existsSync(standaloneRoot)) {
  process.exit(0);
}

copyDir(path.join(websiteRoot, ".next", "static"), path.join(standaloneRoot, ".next", "static"));
copyDir(path.join(websiteRoot, "public"), path.join(standaloneRoot, "public"));


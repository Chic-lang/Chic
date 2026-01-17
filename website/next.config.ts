import path from "node:path";
import type { NextConfig } from "next";

const nextConfig: NextConfig = {
  output: "standalone",
  outputFileTracingRoot: path.join(__dirname, ".."),
  outputFileTracingIncludes: {
    "/*": ["../docs/**", "../SPEC.md", "../README.md", "./content/**"]
  }
};

export default nextConfig;

import { defineConfig, globalIgnores } from "eslint/config";
import nextVitals from "eslint-config-next/core-web-vitals";
import nextTs from "eslint-config-next/typescript";

const eslintConfig = defineConfig([
  ...nextVitals,
  ...nextTs,
  // Override default ignores of eslint-config-next.
  globalIgnores([
    // Default ignores of eslint-config-next:
    ".next/**",
    "out/**",
    "build/**",
    "next-env.d.ts",
    // Generated and vendored, not authored here. `mkdocs build` writes its
    // theme bundle into public/mkdocs/ -- minified JS plus the lunr search
    // workers -- which accounted for 863 of the 866 problems this config
    // reported, none of them actionable and none of them even tracked in git.
    // Linting build output taught us nothing and hid the three real findings.
    "public/**",
  ]),
]);

export default eslintConfig;

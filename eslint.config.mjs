import { defineConfig, globalIgnores } from "eslint/config";
import nextVitals from "eslint-config-next/core-web-vitals";
import nextTs from "eslint-config-next/typescript";

const eslintConfig = defineConfig([
  ...nextVitals,
  ...nextTs,
  {
    rules: {
      "no-restricted-imports": [
        "error",
        { name: "next/link", message: 'Use "@/components/link": it turns prefetch off by default.' },
      ],
    },
  },
  { files: ["src/components/link.tsx"], rules: { "no-restricted-imports": "off" } },
  // Override default ignores of eslint-config-next.
  globalIgnores([
    // Default ignores of eslint-config-next:
    ".next/**",
    "out/**",
    "build/**",
    "next-env.d.ts",
    // Build artifacts:
    ".open-next/**",
    ".wrangler/**",
    "dist/**",
    "electron/**",
    // Archived copy of the Google Apps Script this app replaces.
    "docs/**",
  ]),
]);

export default eslintConfig;

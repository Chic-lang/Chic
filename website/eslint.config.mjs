import nextCoreWebVitals from "eslint-config-next/core-web-vitals";

const config = [
  {
    ignores: ["coverage/**", "playwright-report/**", "test-results/**", ".next/**"]
  },
  ...nextCoreWebVitals,
  {
    rules: {
      "jsx-a11y/no-autofocus": "error"
    }
  }
];

export default config;

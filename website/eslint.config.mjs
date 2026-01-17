import nextCoreWebVitals from "eslint-config-next/core-web-vitals";

const config = [
  ...nextCoreWebVitals,
  {
    rules: {
      "jsx-a11y/no-autofocus": "error"
    }
  }
];

export default config;

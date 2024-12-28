import xwbx from "@xwbx/eslint-config";

export default xwbx({
  unocss: true,
  ignores: ["app/src/common/schema.d.ts", "docs/sql"],
  rules: {
    "antfu/top-level-function": "error",
  },
  typescript: true,
});

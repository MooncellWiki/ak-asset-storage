import xwbx from "@xwbx/eslint-config";

export default xwbx({
  unocss: true,
  ignores: ["app/common/schema.d.ts", "docs/sql", ".sqlx"],
  rules: {
    "antfu/top-level-function": "error",
  },
  typescript: true,
});

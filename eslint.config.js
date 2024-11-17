import xwbx from "@xwbx/eslint-config";

export default xwbx({
  unocss: true,
  ignores: ["packages/ui/src/common/schema.d.ts", "docs/sql"],
  rules: {
    "antfu/top-level-function": "error",
  },
});

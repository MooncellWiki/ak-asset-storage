import xwbx from "@xwbx/eslint-config";

export default xwbx({
  unocss: true,
  ignores: ["packages/ui/src/common/schema.d.ts"],
  rules: {
    "antfu/top-level-function": "error",
  },
});

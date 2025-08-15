export default {
  "*": "eslint --fix",
  "*.rs": () => [
    "cargo fmt --all -- --check",
    "cargo clippy --all-features -- -D warnings",
  ],
};

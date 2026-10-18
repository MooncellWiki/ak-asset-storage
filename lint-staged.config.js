export default {
  "*": "eslint --fix",
  "*.rs": () => [
    "cargo fmt --all -- --check",
    "cargo clippy --locked --all-features -- -D warnings",
  ],
};

# Summary
- Fix CI Clippy failures when warnings are treated as errors (`-D warnings`).

# Verification (local)
- [x] `cargo fmt --check`
- [x] `cargo clippy --locked --all-targets --all-features -- -D warnings`
- [x] `cargo test --locked --all-targets --all-features`

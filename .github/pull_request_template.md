# Summary
- Fix Clippy gate failures when CI treats warnings as errors (`-D warnings`).

# Verification (local)
- [x] `cargo fmt --check`
- [x] `cargo clippy --locked --all-targets --all-features -- -D warnings`
- [x] `cargo test --locked --all-targets --all-features`

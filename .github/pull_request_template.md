# Summary
- Fix Clippy gate failures when CI treats warnings as errors (`-D warnings`).

# What Changed
- Mechanical clippy cleanups across services/APIs and small targeted `#[allow]` where signature refactors would be risky.

# Verification (local)
- [x] `cargo fmt --check`
- [x] `cargo clippy --locked --all-targets --all-features -- -D warnings`
- [x] `cargo test --locked --all-targets --all-features`

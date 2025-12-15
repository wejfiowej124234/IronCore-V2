# Summary
- Fix Clippy gate failures when CI treats warnings as errors (`-D warnings`).

# Verification
- [x] `cargo fmt --check`
- [x] `cargo clippy --locked --all-targets --all-features -- -D warnings`
- [x] `cargo test --locked --all-targets --all-features`

# Notes
- Includes mechanical clippy cleanups and targeted `#[allow]` on stable public APIs where refactoring signatures would be risky.

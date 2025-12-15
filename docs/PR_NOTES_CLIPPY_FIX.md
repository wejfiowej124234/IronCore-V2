This branch fixes CI-equivalent Clippy gates when warnings are treated as errors (`-D warnings`).

Local verification:
- cargo fmt --check
- cargo clippy --locked --all-targets --all-features -- -D warnings
- cargo test --locked --all-targets --all-features

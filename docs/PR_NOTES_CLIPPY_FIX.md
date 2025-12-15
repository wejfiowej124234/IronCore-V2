This PR fixes CI-equivalent Clippy gates when warnings are treated as errors (`-D warnings`).

Local verification:
- cargo fmt --check
- cargo clippy --locked --all-targets --all-features -- -D warnings
- cargo test --locked --all-targets --all-features

Note:
- Safe, mechanical lint cleanups + minimal targeted `#[allow]` where signature refactors would be risky.
- This branch can be squashed on merge.

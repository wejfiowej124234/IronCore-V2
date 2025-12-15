#!/bin/bash
# 临时修复脚本 - 禁用多链API以快速验证主要功能

echo "暂时注释掉多链API以便后续完善..."

cd "$(dirname "$0")/../.."

# 注释掉 api/mod.rs 中的 multi_chain_api 集成
sed -i 's/^pub mod multi_chain_api;/\/\/ pub mod multi_chain_api;/' src/api/mod.rs
sed -i 's/\.merge(multi_chain_api::create_multi_chain_routes(state.clone()))/\/\/ .merge(multi_chain_api::create_multi_chain_routes(state.clone()))/' src/api/mod.rs

# 注释掉 lib.rs 中的 domain 模块
sed -i 's/^pub mod domain;/\/\/ pub mod domain;/' src/lib.rs

echo "✅ 已临时禁用多链API模块"
echo "现在可以编译主要后端功能了"
echo ""
echo "运行: cargo build"

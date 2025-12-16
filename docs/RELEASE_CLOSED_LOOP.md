# Release Closed-Loop Verification (推荐)

目标：把“发布没问题”变成可重复、可审计、可自动化的闭环。

这套闭环强调两点：
1) **门禁严格**：`cargo clippy ... -- -D warnings`、tests、release build、security audit 都必须通过。
2) **验证可脚本化**：不依赖 Actions 日志下载/网页操作，通过 GitHub API 验证结果。

---

## 1) 本地预检（强烈推荐）

在仓库根目录执行：

```bash
cargo fmt --all -- --check
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked --all-targets --all-features
```

说明：CI 同样使用 `--locked` + `-D warnings`，本地全绿意味着 CI 大概率一次过。

---

## 2) 推送/合并策略（严格门禁固化的核心）

推荐的严格策略（main）：
- **只能通过 PR 合并**（禁止直接 push）
- **Required checks 全部必须 success**：
  - `Gates (ubuntu-latest)`
  - `Gates (windows-latest)`
  - `Security Audit`
  - `Clippy (annotated)`
- **Enforce admins**（管理员也不能 bypass）

> 注意：分支保护/规则集属于 GitHub 设置层面，无法仅靠 SSH git 命令完成；
> 需要 `gh` 登录或 GitHub API Token 执行配置。

---

## 3) CI 结果验证（闭环验证，不依赖日志）

仓库提供脚本：`scripts/ci_verify.py`

### 验证 main 是否全绿

```bash
python scripts/ci_verify.py --owner wejfiowej124234 --repo IronCore-V2 --branch main
```

### 提升 API 额度（可选）

如果你在 CI 轮询时遇到 rate limit，可设置 token：

```bash
set GITHUB_TOKEN=YOUR_TOKEN
python scripts/ci_verify.py --owner wejfiowej124234 --repo IronCore-V2 --branch main
```

脚本会等待 run 完成，并强制校验 required jobs 是否全部 `success`，否则返回非 0。

---

## 4) Tag 发行闭环（推荐）

CI 已配置为对 `v*` tag 也执行同样门禁。

示例：

```bash
git tag -a v1.0.0 -m "release v1.0.0"
git push origin v1.0.0
python scripts/ci_verify.py --owner wejfiowej124234 --repo IronCore-V2 --branch main
```

> 如果你希望脚本验证某个 tag 对应的 run，可用 `--run-id`（从 API 或 Actions 页面获得）。

---

## 5) 结论标准（什么时候可以说“发布没问题”）

满足以下全部条件：
- 本地预检（fmt/clippy/tests）通过
- `main` 分支 CI 最新 run `success`（且 required jobs 全绿）
- （如发 tag）该 tag 触发的 CI 也 `success`
- 分支保护已固化：main 禁止直接 push，必须 PR + required checks

达到上述标准，就是真正意义上的“发布闭环完成”。

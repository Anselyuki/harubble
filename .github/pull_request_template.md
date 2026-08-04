## 变更摘要

-

## 测试计划

- [ ] `bun run check`
- [ ] `bun run test`
- [ ] `cargo test --workspace`
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`

> `bun run check` 已包含前端格式、ESLint、TypeScript、Svelte、构建、rustfmt 与 `cargo check --workspace`；测试和严格 Clippy 由 CI 的 `test` job 单独执行。

## 发布信息

- [ ] 合并到 `main` 后发布新版本
- `Release-As:` 为可选项。示例：`v1.2.3`、`v1.2`、`v2`、`v1.3-beta`
- 勾选发布但留空 `Release-As:` 时，PR 合并到 `main` 后会自动升级到下一个次版本号。
- 不勾选且不填写 `Release-As:` 时，本次合并不会触发 release。

Release-As:

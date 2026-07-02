# Release 流程

本文档说明当前仓库的 CI 与发布行为，对应的 workflow 为：

- `.github/workflows/ci.yml`
- `.github/workflows/distribute.yml`

## 流程总览

1. 提交 Pull Request 到 `main` 或 `develop` 时，只会触发 CI。
2. CI 只负责检查与测试，不会创建 GitHub Release，也不会上传发布产物。
3. Pull Request 合并到 `main` 后，会触发发布 workflow。
4. 发布 workflow 只会在合并 PR 存在明确发布意图时继续执行。
5. 如果 PR 明确要求发布但没有指定版本号，则基于最新正式版自动提升一个小版本号。
6. 发布 workflow 会构建 macOS、Windows 与 Linux 产物，并上传到对应的 GitHub Release。

## PR 阶段

触发条件：

- `pull_request` 到 `main`
- `pull_request` 到 `develop`

当前 CI 执行内容：

- `bun run format:check`
- `bun run lint:eslint`
- `bun run check:types`
- `bun run check:svelte`
- `bun run check:build`
- `cargo fmt --all --check`
- `cargo check --workspace`
- `cargo test --workspace`

PR 阶段不会执行以下行为：

- 不创建 tag
- 不创建 GitHub Release
- 不上传安装包或可执行文件

## 发布阶段

触发条件：

- 代码被合并并进入 `main`

发布 workflow 只在 `main` 的 push 上运行，并会先检查该提交是否关联到一个已合并的 Pull Request。
如果当前提交不是通过合并 PR 进入 `main`，则 workflow 会直接跳过发布。

## 什么情况下会触发 Release

只有满足以下任一条件，合并到 `main` 后才会继续发布：

- PR 描述中勾选了“合并到 `main` 后发布新版本”
- PR 标题或描述中填写了 `Release-As:`

如果这两个条件都不满足，则即使 PR 被合并到 `main`，也不会创建 release。

这意味着：

- 纯文档更新默认不会发版
- 普通维护 PR 默认不会发版
- 只有明确表达“这次要发版”的 PR 才会进入发布流程

## 版本号规则

### 显式标记版本

可以在合并前的 PR 标题或 PR 描述中写入版本标记。当前发布 workflow 会解析：

- `Release-As: v1.2.3`

支持缺省写法，会自动补全为三段版本号：

- `v1.1` -> `v1.1.0`
- `v2` -> `v2.0.0`

如果包含预发布后缀，也会保留：

- `Release-As: v1.3-beta` -> `v1.3.0-beta`

### 勾选发布但不指定版本

如果 PR 勾选了“合并到 `main` 后发布新版本”，但没有填写 `Release-As:`：

1. workflow 会读取最新的正式版 release tag。
2. 自动提升一个小版本号。
3. patch 会重置为 `0`。

示例：

- 最新正式版为 `v0.1.1`，则下一个默认版本为 `v0.2.0`
- 最新正式版为 `v1.4.3`，则下一个默认版本为 `v1.5.0`

如果仓库里还没有任何正式版 release，则默认从 `v0.1.0` 开始。

### 不勾选且不填写版本

如果 PR：

- 没有勾选“合并到 `main` 后发布新版本”
- 也没有填写 `Release-As:`

则本次合并不会触发 release。

## 发布时会同步修改的版本位置

发布 workflow 在构建前会把解析出的版本号同步到以下文件：

- `package.json`
- `src-tauri/tauri.conf.json`
- `src-tauri/Cargo.toml`

这些修改仅作用于 workflow 运行时的构建上下文，用来保证生成产物的版本一致。

## 发布产物

当前发布产物如下：

- `harubble_<version>_macos_intel.dmg`
- `harubble_<version>_macos_apple_silicon.dmg`
- `harubble_<version>_windows_x64_portable_webview2.exe`
- `harubble_<version>_linux_x64.AppImage`

说明：

- `macos_intel` 对应 Intel Mac
- `macos_apple_silicon` 对应 Apple Silicon Mac
- macOS 当前发布的是未签名、未公证的 DMG；workflow 会在 DMG 内加入 `README-macOS.txt` 和 Applications 快捷方式，但不会执行 codesign 或 notarization
- Windows 当前发布的是依赖系统 `WebView2` 运行时的精简便携 `.exe`，不是 NSIS 安装包
- Linux 当前发布的是 AppImage 格式；构建环境为 Ubuntu 22.04，要求 glibc 2.35+，但发布包尚未经过完整测试，用户反馈应优先收敛到 Issues

## 推荐用法

### 普通合并

如果这次 PR 只是普通功能、修复、文档或维护更新：

1. 正常提 PR 到 `main` 或 `develop`
2. 等待 CI 通过
3. 合并 PR
4. 不触发 release

### 自动升版本发布

如果这次合并需要发版，但不想手动指定版本号：

1. 在 PR 模板里勾选 `合并到 `main` 后发布新版本`
2. 不填写 `Release-As:`
3. 等待 CI 通过
4. 合并 PR
5. 发布 workflow 会自动按最新正式版升一个小版本号并发布

### 指定版本发布

如果这次合并需要明确版本号：

1. 在 PR 标题或描述中填写 `Release-As: vX.Y.Z`
2. 可选地勾选发布开关，但即使不勾选，只要填写了 `Release-As:` 也会触发发布
3. 等待 CI 通过
4. 合并 PR
5. 发布 workflow 会使用你指定的版本号进行发布

## 注意事项

- 不要在 PR 阶段期待生成 Release；PR 只做检查和测试。
- `Release-As:` 建议保留原样书写，避免与 workflow 解析规则不一致。
- 如果指定的版本号已经存在同名 GitHub Release，发布会继续上传当前矩阵产物，并使用 `gh release upload --clobber` 覆盖同名 asset。
- 如果未来要恢复 Windows 安装包发布策略，需要同时更新 workflow 和 README 中对 Windows 产物的说明。

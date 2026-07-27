# 文档目录

## 本地开发

### 环境要求

- Rust
- Bun 1.3+（唯一 JS 包管理器）

### 常用命令

```bash
# 安装依赖与启动开发
bun install
bun run tauri:dev
```

```bash
# 检查与测试
bun run format:check
bun run lint
bun run check
cargo test --workspace
```

```bash
# 构建
bun run build
bun run tauri:build
```

### 提交前检查

仓库使用 `.pre-commit-config.yaml` 统一执行行尾检查、Prettier 和 `cargo fmt`。首次开发时安装 hook：

```bash
pip install pre-commit
pre-commit install
```

需要手动检查全部文件时运行：

```bash
pre-commit run --all-files
```

如果 hook 自动修改了文件，检查 diff、重新暂存后再提交。

### 生成 Rust 文档（rustdoc）

项目中的 Rust API 文档统一通过 `cargo doc` 生成，产物默认输出到 `target/doc/`。

```bash
# 生成核心库文档
cargo doc -p harubble_core --no-deps

# 生成桌面应用库文档（包含 private items）
cargo doc -p harubble --lib --no-deps --document-private-items

# 生成桌面应用二进制入口文档（包含 private items）
cargo doc -p harubble --bin harubble --no-deps --document-private-items
```

- `--no-deps` 只生成当前工作区包的文档，避免展开依赖库。
- `--document-private-items` 适合本地排查模块职责与内部状态。
- 生成后打开 `target/doc/index.html` 查看文档首页。

---

## 文档索引

### reference/ — 技术参考

#### [frontend-guide.md](./reference/frontend-guide.md)

前端架构、组件约定、域边界、运行时架构、UI 系统（设计 token、字体方案、动效规则）、Visual Contract（family × depth 视觉族 · §9）、`theme_packages_v1` 灰度状态与运维清单（§9.4）、Phase 4 CSS 覆盖层触发条件（§9.5）、国际化、交互模式与内容规范。

#### [resource-update.md](./reference/resource-update.md)

标签注册表（Tag Registry）的更新机制与数据结构说明。

#### [internationalization.md](./reference/internationalization.md)

国际化架构决策、支持语言、品牌标识规范与翻译层技术选型。

#### [playback-state-machine.md](./reference/playback-state-machine.md)

播放控制面、音频数据面、输出面与前端同步面的状态机设计。包含状态/事件/守卫条件、音频安全不变量、错误恢复策略和播放链路验证清单。

#### [playback-command-scheduling.md](./reference/playback-command-scheduling.md)

播放资源隔离与 command 调度当前实现。包含 command domain/priority 划分、PlaybackActor、资源域、PlaybackLoadGate、降级策略、外部音乐播放器架构参考和验证指标。

### process/ — 项目规定

#### [release-process.md](./process/release-process.md)

CI/CD 流程、发布触发条件、版本号策略与产物构建。

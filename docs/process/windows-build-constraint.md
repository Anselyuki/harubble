# Windows 构建约束：控制台子系统

> 约束 Windows release 构建不得弹出黑色终端窗口，并指导 Agent 在修改入口点或构建配置时遵守本约束。

## 1. 问题描述

**现象**：Windows release 版本启动后，除了 GUI 窗口外还会出现一个黑色终端窗口。若用户关闭该终端，整个应用进程同步退出。

**根因**：Rust 在 Windows 上默认以**控制台子系统**（console subsystem）链接二进制产物。控制台子系统会为 stdin/stdout/stderr 创建一个终端窗口。Tauri GUI 应用应使用 **Windows 子系统**（windows subsystem）。

**影响范围**：

- `src-tauri/src/main.rs`：二进制入口点
- `src-tauri/Cargo.toml`：`[[bin]]` 声明
- 未来任何新增的 `[[bin]]` 入口

## 2. 修复方式

在 `src-tauri/src/main.rs` 文件**最顶部**（rustdoc 注释之前）添加：

```rust
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
```

**`cfg_attr` 条件编译的意图**：

| 构建模式                 | 子系统  | 行为                                                      |
| ------------------------ | ------- | --------------------------------------------------------- |
| Debug（`tauri:dev` 等）  | Console | 保留终端，便于查看 `println!` / `eprintln!` / `dbg!` 输出 |
| Release（`tauri:build`） | Windows | 无终端窗口，纯 GUI 体验                                   |

## 3. Agent 约束

### 3.1 不得移除该属性

在未充分理解后果的情况下，不得从 `main.rs` 顶部删除 `#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]`。

如果确有需要改为始终使用 console 子系统（例如排查 release 构建的启动问题），应在修复后立即恢复。

### 3.2 新增二进制入口点必须同步设置

如果在 `src-tauri/Cargo.toml` 中新增 `[[bin]]` 入口，该入口的 `.rs` 文件也必须添加相同的 `#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]` 属性，否则该二进制产物会在 release 中弹出终端窗口。

### 3.3 入口文件重命名或重构时保持属性跟随

如果对 `main.rs` 进行重命名、拆分或重构（例如将 `fn main()` 抽取到另一个文件），确保该属性始终位于实际二进制入口文件的最顶部。入口文件 = 包含 `fn main()` 且被 `Cargo.toml` 中 `[[bin]]` 的 `path` 指向的文件。

### 3.4 release 构建中避免裸 println! / eprintln!

在 `windows_subsystem = "windows"` 的 release 构建中，`println!` / `eprintln!` 的输出没有可见终端接收（除非用户通过命令行重定向）。release 路径中的日志输出应统一走项目的日志系统：

```rust
state.record_log(
    LogPayload::new(LogLevel::Warn, "module", "event_key", "message")
        .details(error.to_string()),
);
```

目前 `main.rs` 的 `flush_logs_on_exit` 函数中存在一处 `eprintln!`（flush logs 失败时的 fallback），由 `RunEvent::ExitRequested` 与 `RunEvent::Exit` 两个分支共同调用；属于终端不可见时无法观测的极端兜底，暂时保留但不作为推荐模式。

### 3.5 构建版本对齐约束

`@tauri-apps/api`（npm）与 `tauri`（Rust crate）的 **minor 版本必须对齐**，否则 `tauri build` 会在版本检查阶段直接拒绝构建。

典型错误信息：

```
Error Found version mismatched Tauri packages.
tauri (v2.11.1) : @tauri-apps/api (v2.10.1)
```

修复方式：将 npm 侧的 `@tauri-apps/api` 升级到对应的 minor 版本：

```bash
bun install @tauri-apps/api@^2.11.0
```

**Agent 约束**：当 `tauri build` 失败并报出版本不匹配错误时，应优先尝试升级 npm 包到对应 minor 的最新补丁版本，而不是降级 Rust 侧的 tauri crate。

## 4. 验证方法

### 4.1 确认属性存在

```bash
head -1 src-tauri/src/main.rs
```

应输出：

```
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
```

### 4.2 确认 release 二进制不含控制台子系统

构建完成后，检查二进制 PE 头中的子系统字段：

```powershell
# PowerShell: 检查子系统是否为 WINDOWS (值=2)，而非 CONSOLE (值=3)
dumpbin /headers target\release\harubble.exe | Select-String "subsystem"
```

预期输出应包含 `Windows GUI`，而非 `Windows Console`。

### 4.3 实际运行验证

构建 release 后直接双击 `target\release\harubble.exe`，确认不会弹出黑色终端窗口。关闭 GUI 窗口后确认进程正常退出（任务管理器中无残留进程）。

## 5. 相关文件

| 文件                    | 角色                                                      |
| ----------------------- | --------------------------------------------------------- |
| `src-tauri/src/main.rs` | 二进制入口，承载 `windows_subsystem` 属性                 |
| `src-tauri/Cargo.toml`  | `[[bin]]` 声明，决定哪些 `.rs` 文件需要该属性             |
| `package.json`          | `@tauri-apps/api` 版本声明，需与 Rust 侧 `tauri` 版本对齐 |
| `src-tauri/Cargo.lock`  | `tauri` crate 实际解析版本，版本对齐的事实来源            |

## 6. 变更历史

| 日期       | 变更                                                 | 原因                                        |
| ---------- | ---------------------------------------------------- | ------------------------------------------- |
| 2026-05-11 | 新增本文档与 `main.rs` 中的 `windows_subsystem` 属性 | 修复 Windows release 构建弹出黑色终端的问题 |

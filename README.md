# Harubble

<div>
  <img alt="platform" src="https://img.shields.io/badge/platform-macOS%20%7C%20Windows%20%7C%20Linux-4c8bf5">
  <img alt="license" src="https://img.shields.io/github/license/Anselyuki/harubble">
  <img alt="stars" src="https://img.shields.io/github/stars/Anselyuki/harubble?style=social">
</div>

Harubble 是面向 [塞壬唱片](https://monster-siren.hypergryph.com/) 的桌面音乐播放器与下载器。

## 特性

- **专辑目录 · 全库检索 · 桌面播放**：完整的塞壬唱片曲库浏览、下载与本地库管理。
- **Windows Mini Player**：Windows 上提供独立置顶小窗口，支持快捷控制并实时同步主窗口主题。
- **主题包系统**：内置五套 Ark UI inspired 原创主题包（Industrial Cyan / Field
  Signal / Astral Archive / Co-op Pop / Studio Lime），也可导入 `.json` 覆盖配色 /
  动效 / 圆角 / 密度 / 阴影 / 模糊 / 视觉族 / 字体栈，以及命名空间隔离的自定义 CSS
  变量（含明暗变体）。设置页 → 主题包库支持本地文件或 URL 安装（自动做 SSRF 校验与
  CSS sanitize）；任意 CSS stylesheet、ZIP 和包内 assets 暂不支持。
- **国际化**：内建简中 / 英文 UI 语言切换。

> 名字来自《明日方舟》角色 **遥**（Haruka）和她漂浮在空中的透明泡泡（Bubble）。Haru 是 Haruka 的简写。
>
> 她胆小、容易害怕，也常常会哭。可就算哭得一塌糊涂，她还是会站出来做那件勇敢的事。她随身带着泡泡水，源石技艺只有借由浮泡才能发挥到最好；那些漂在空中的泡泡，就是她给大家撑起的庇护。
>
> 「横竖都是死，我和你们拼了！」

## 下载与安装

从 [GitHub Releases](https://github.com/Anselyuki/harubble/releases) 下载对应平台的发布文件。应用启动时会联网获取专辑目录；封面、歌词和音频在浏览或播放时按需获取。

### macOS

1. 根据设备芯片下载 `harubble_<version>_macos_intel.dmg` 或 `harubble_<version>_macos_apple_silicon.dmg`。
2. 打开 DMG，先阅读其中的双语 `README-macOS.txt`，再把 `Harubble.app` 拖到 `Applications` 文件夹。
3. 按说明执行一次：

   ```bash
   xattr -dr com.apple.quarantine /Applications/Harubble.app
   ```

4. 从 `Applications` 启动 Harubble。

> 当前 macOS 发布包没有做代码签名，也没有走 Apple notarization 流程，所以系统可能在首次打开时拦截它。Apple 官方对面向分发的 macOS 软件要求使用 Developer ID 签名，并通过 notarization；
>
> 相关说明见 [Notarizing macOS software before distribution](https://developer.apple.com/documentation/security/notarizing-macos-software-before-distribution) 和 [Distributing your app for beta testing and releases](https://developer.apple.com/documentation/xcode/distributing-your-app-for-beta-testing-and-releases)。
>
> 这条 `xattr` 命令只是移除下载隔离标记，不会给应用签名，只应对来自可信来源的软件执行。

### Windows

1. 下载 `harubble_<version>_windows_x64_portable_webview2.exe`。
2. 双击 `.exe` 直接运行。当前 Windows 发布版是便携程序，不提供安装型打包。
3. 程序依赖系统里的 Microsoft Edge WebView2 Runtime；如果无法启动，先安装或更新 WebView2 Runtime。

### Linux

1. 下载 `harubble_<version>_linux_x64.AppImage`。
2. 给文件添加执行权限：

   ```bash
   chmod +x harubble_<version>_linux_x64.AppImage
   ```

3. 直接运行 AppImage。当前构建基于 Ubuntu 22.04，要求 glibc 2.35 或更高版本。

> 当前 Linux 发布包还没有经过完整测试，使用中如果发现问题，欢迎到 [Issues](https://github.com/Anselyuki/harubble/issues) 提交反馈，也欢迎直接留下 bug report。

## 文档

[docs/README.md](./docs/README.md) 包含本地开发指南和文档索引。

## 说明

- Harubble 使用塞壬唱片的公开接口与公开资源；如果上游接口或资源地址变化，应用可能需要同步更新。
- 本项目是桌面端体验整合与学习项目，与塞壬唱片或鹰角网络没有官方关系。
- 使用中遇到问题，或有改进建议，可以提交 [Issue](https://github.com/Anselyuki/harubble/issues) 或 Pull Request。
- 也欢迎到 [Discussions](https://github.com/Anselyuki/harubble/discussions) 交流想法、提问或分享反馈。

## 许可证

本项目基于 [MIT](./LICENSE) 许可证开源。

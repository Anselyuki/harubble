Harubble macOS 安装说明
=========================

请先阅读本文件，再安装 Harubble。

安装步骤：

1. 将 Harubble.app 拖到 Applications 文件夹。
2. 打开 Terminal。
3. 运行以下命令：

    xattr -dr com.apple.quarantine /Applications/Harubble.app

4. 从 Applications 启动 Harubble。

当前 macOS 发布包没有做代码签名，也没有走 Apple notarization 流程，所以系统可能在首次打开时拦截它。Apple 官方对面向分发的 macOS 软件要求使用 Developer ID 签名，并通过 notarization。上面的 xattr 命令只是移除下载隔离标记，不会给应用签名；只对你信任来源的软件运行这条命令。


Harubble macOS Installation Guide
=================================

Please read this file first, then install Harubble.

Steps:

1. Drag Harubble.app to the Applications folder.
2. Open Terminal.
3. Run this command:

    xattr -dr com.apple.quarantine /Applications/Harubble.app

4. Launch Harubble from Applications.

The current macOS release is not code signed or notarized, so macOS may block it on first launch. Apple requires Developer ID signing and notarization for macOS software distributed outside the Mac App Store. The xattr command above only removes the download quarantine attribute; it does not sign the app. Only run it for software you downloaded from a source you trust.

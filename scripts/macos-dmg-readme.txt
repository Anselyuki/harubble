Harubble macOS 安装说明
=========================

请先阅读本文件，再安装 Harubble。

Harubble for macOS is not notarized yet. After copying Harubble.app to
Applications, macOS may report that the app is damaged or cannot be opened. To
run this unsigned open-source build, remove the download quarantine attribute
once:

    xattr -dr com.apple.quarantine /Applications/Harubble.app

安装步骤：

1. 将 Harubble.app 拖到 Applications 文件夹。
2. 打开 Terminal。
3. 运行上面的 xattr 命令。
4. 从 Applications 启动 Harubble。

只对你信任来源的软件运行这条命令。


Harubble macOS Installation Guide
=================================

Please read this file first, then install Harubble.

Harubble for macOS is not notarized yet. After copying Harubble.app to
Applications, macOS may report that the app is damaged or cannot be opened. To
run this unsigned open-source build, remove the download quarantine attribute
once:

    xattr -dr com.apple.quarantine /Applications/Harubble.app

Steps:

1. Drag Harubble.app to the Applications folder.
2. Open Terminal.
3. Run the xattr command above.
4. Launch Harubble from Applications.

Only run this command for software you downloaded from a source you trust.

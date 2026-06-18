Harubble for macOS is not notarized yet.

After copying Harubble.app to Applications, macOS may report that the app is
damaged or cannot be opened. To run this unsigned open-source build, remove the
download quarantine attribute once:

    xattr -dr com.apple.quarantine /Applications/Harubble.app

Steps:

1. Drag Harubble.app to the Applications folder.
2. Open Terminal.
3. Run the command above.
4. Launch Harubble from Applications.

Only run this command for software you downloaded from a source you trust.

## System notifications

notification-download-completed = Download completed
notification-album-completed = Album download completed ({ $count } songs)
notification-album-partial = Album download completed ({ $completed } succeeded, { $failed } failed)
notification-test-title = Test Notification
notification-test-body = Siren Music Downloader notification is working.
notification-selection-title = { $count } songs selected
notification-selection-title-cross-albums = { $count } songs selected · { $albumCount } albums

## Preferences validation

preferences-unsupported-format = Unsupported format: { $format }
preferences-unsupported-log-level = Unsupported log level: { $level }
preferences-output-dir-must-be-absolute = Output path must be an absolute directory path
preferences-output-dir-not-exists = Output path does not exist
preferences-output-dir-is-symlink = Output path cannot be a symlink
preferences-output-dir-not-directory = Output path is not a directory
preferences-export-path-must-be-absolute = Export path must be an absolute file path
preferences-export-path-is-symlink = Export path cannot be a symlink
preferences-export-path-is-directory = Export path must be a file path
preferences-import-path-must-be-absolute = Import path must be an absolute file path
preferences-import-file-not-exists = Import file does not exist
preferences-import-path-is-symlink = Import path cannot be a symlink
preferences-import-path-not-file = Import path must be a file

## Preferences persistence

preferences-dir-invalid = Preferences directory is invalid
preferences-dir-create-failed = Failed to create preferences directory
preferences-file-write-failed = Failed to write preferences file
preferences-export-dir-create-failed = Failed to create export directory
preferences-export-file-write-failed = Failed to write export file
preferences-import-file-read-failed = Failed to read import file
preferences-load-invalid = Preferences are invalid, reverted to defaults
preferences-load-corrupted = Preferences are corrupted, reverted to defaults
preferences-load-read-failed = Failed to read preferences, reverted to defaults

## Download session

download-session-read-failed = Failed to read download history, reverted to empty state
download-session-parse-failed = Download history is corrupted, reverted to empty state
download-session-schema-incompatible = Download history version is incompatible, reverted to empty state
download-session-dir-invalid = Download session directory is invalid
download-session-save-failed = Failed to save download history
download-session-interrupted-cancelled = Interrupted by app restart
download-session-interrupted-failed = Download interrupted by app restart

## Local inventory

inventory-output-dir-not-directory = outputDir is not a directory
inventory-read-dir-failed = Failed to read directory
inventory-enumerate-dir-failed = Failed to enumerate directory
inventory-read-metadata-failed = Failed to read file metadata
inventory-read-audio-failed = Failed to read audio file

## Search

search-query-empty = Search query cannot be empty
search-query-too-long = Search query cannot exceed 128 characters
search-index-build-failed = Failed to build search index

## Player

player-no-active-track = No active track
player-still-loading = Playback is still loading
player-no-next-track = No next track available
player-no-previous-track = No previous track available

## Desktop menu

desktop-menu-not-playing = Not Playing
desktop-menu-previous = Previous Track
desktop-menu-play = Play
desktop-menu-pause = Pause
desktop-menu-next = Next Track
desktop-menu-show = Show Harubble
desktop-menu-quit = Quit Harubble

## Application menu bar

appmenu-app = Harubble
appmenu-edit = Edit
appmenu-window = Window
appmenu-view = View
appmenu-playback = Playback

appmenu-app-preferences = Preferences…
appmenu-app-test-notification = Send Test Notification

appmenu-file = File
appmenu-file-new-collection = New Collection…
appmenu-file-import-collection = Import Collection…
appmenu-file-export-collection = Export Current Collection…
appmenu-file-import-tag-registry = Import Tag Library…
appmenu-file-export-tag-registry = Export Tag Library…
appmenu-file-import-preferences = Import Preferences…
appmenu-file-export-preferences = Export Preferences…
appmenu-file-clear-listening-history = Clear Listening History
appmenu-file-clear-download-history = Clear Download History

appmenu-view-home = Home
appmenu-view-search = Search
appmenu-view-overview = All Albums
appmenu-view-library = Library
appmenu-view-collection = Collections
appmenu-view-tag-editor = Tag Editor
appmenu-view-go-back = Back
appmenu-view-toggle-sidebar = Toggle Sidebar
appmenu-view-toggle-downloads = Downloads
appmenu-view-refresh = Refresh
appmenu-view-rescan-inventory = Rescan Local Library
appmenu-view-logs = View Logs…
appmenu-view-appearance = Appearance
appmenu-view-appearance-auto = Follow System
appmenu-view-appearance-light = Light
appmenu-view-appearance-dark = Dark

appmenu-playback-toggle = Play / Pause
appmenu-playback-next = Next Track
appmenu-playback-previous = Previous Track
appmenu-playback-seek-forward = Skip Forward 10s
appmenu-playback-seek-backward = Skip Backward 10s
appmenu-playback-volume-up = Volume Up
appmenu-playback-volume-down = Volume Down
appmenu-playback-toggle-mute = Toggle Mute
appmenu-playback-toggle-shuffle = Shuffle
appmenu-playback-repeat = Repeat
appmenu-playback-repeat-off = Repeat Off
appmenu-playback-repeat-all = Repeat All
appmenu-playback-repeat-one = Repeat One
appmenu-playback-toggle-lyrics = Show Lyrics
appmenu-playback-toggle-playlist = Show Playlist
appmenu-playback-toggle-fullscreen = Fullscreen Player

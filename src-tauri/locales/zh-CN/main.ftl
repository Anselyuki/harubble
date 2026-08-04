## 系统通知

notification-download-completed = 下载完成
notification-album-completed = 专辑下载完成（{ $count } 首歌曲）
notification-album-partial = 专辑下载完成（{ $completed } 首成功，{ $failed } 首失败）
notification-test-title = 测试通知
notification-test-body = 塞壬音乐下载器通知功能正常。
notification-selection-title = 已选 { $count } 首
notification-selection-title-cross-albums = 已选 { $count } 首 · { $albumCount } 张专辑

## 偏好校验

preferences-unsupported-format = 不支持的格式: { $format }
preferences-unsupported-log-level = 不支持的日志等级: { $level }
preferences-output-dir-must-be-absolute = 保存路径必须是绝对目录路径
preferences-output-dir-not-exists = 保存路径不存在
preferences-output-dir-is-symlink = 保存路径不能是符号链接
preferences-output-dir-not-directory = 保存路径不是目录
preferences-export-path-must-be-absolute = 导出路径必须是绝对文件路径
preferences-export-path-is-symlink = 导出路径不能是符号链接
preferences-export-path-is-directory = 导出路径必须是文件路径
preferences-import-path-must-be-absolute = 导入路径必须是绝对文件路径
preferences-import-file-not-exists = 导入文件不存在
preferences-import-path-is-symlink = 导入路径不能是符号链接
preferences-import-path-not-file = 导入路径必须是文件

## 偏好持久化

preferences-dir-invalid = 偏好目录无效
preferences-dir-create-failed = 创建偏好目录失败
preferences-file-write-failed = 写入偏好文件失败
preferences-export-dir-create-failed = 创建导出目录失败
preferences-export-file-write-failed = 写入导出文件失败
preferences-import-file-read-failed = 读取导入文件失败
preferences-load-invalid = 偏好配置无效，已回退到默认设置
preferences-load-corrupted = 偏好配置损坏，已回退到默认设置
preferences-load-read-failed = 读取偏好配置失败，已回退到默认设置

## 下载会话

download-session-read-failed = 下载历史读取失败，已回退为空状态
download-session-parse-failed = 下载历史已损坏，已回退为空状态
download-session-schema-incompatible = 下载历史版本不兼容，已回退为空状态
download-session-dir-invalid = 下载 session 目录无效
download-session-save-failed = 下载历史保存失败
download-session-interrupted-cancelled = 因应用重启而取消
download-session-interrupted-failed = 因应用重启而中断

## 本地库存

inventory-output-dir-not-directory = outputDir 不是目录
inventory-read-dir-failed = 读取目录失败
inventory-enumerate-dir-failed = 枚举目录失败
inventory-read-metadata-failed = 读取文件元信息失败
inventory-read-audio-failed = 读取音频文件失败

## 搜索

search-query-empty = 搜索关键词不能为空
search-query-too-long = 搜索关键词长度不能超过 128 个字符
search-index-build-failed = 搜索索引构建失败

## 播放器

player-no-active-track = 当前没有正在播放的曲目
player-still-loading = 播放正在加载中
player-no-next-track = 没有下一首可播放的曲目
player-no-previous-track = 没有上一首可播放的曲目

## 桌面菜单

desktop-menu-not-playing = 未在播放
desktop-menu-previous = 上一首
desktop-menu-play = 播放
desktop-menu-pause = 暂停
desktop-menu-next = 下一首
desktop-menu-show = 显示 Harubble
desktop-menu-quit = 退出 Harubble

## 应用菜单栏

appmenu-app = Harubble
appmenu-edit = 编辑
appmenu-window = 窗口
appmenu-view = 视图
appmenu-playback = 播放

appmenu-app-preferences = 偏好设置…
appmenu-app-test-notification = 发送测试通知

appmenu-file = 文件
appmenu-file-new-collection = 新建合集…
appmenu-file-import-collection = 导入合集…
appmenu-file-export-collection = 导出当前合集…
appmenu-file-import-tag-registry = 导入标签库…
appmenu-file-export-tag-registry = 导出标签库…
appmenu-file-import-preferences = 导入偏好…
appmenu-file-export-preferences = 导出偏好…
appmenu-file-clear-listening-history = 清空最近播放
appmenu-file-clear-download-history = 清空下载历史

appmenu-view-home = 首页
appmenu-view-search = 搜索
appmenu-view-overview = 全部专辑
appmenu-view-library = 曲库
appmenu-view-collection = 合集
appmenu-view-tag-editor = 标签编辑器
appmenu-view-go-back = 后退
appmenu-view-toggle-sidebar = 收起 / 展开侧栏
appmenu-view-toggle-downloads = 下载任务
appmenu-view-refresh = 刷新数据
appmenu-view-rescan-inventory = 重新扫描本地库存
appmenu-view-logs = 查看日志…
appmenu-view-appearance = 外观
appmenu-view-appearance-auto = 跟随系统
appmenu-view-appearance-light = 浅色
appmenu-view-appearance-dark = 深色

appmenu-playback-toggle = 播放 / 暂停
appmenu-playback-next = 下一曲
appmenu-playback-previous = 上一曲
appmenu-playback-seek-forward = 快进 10 秒
appmenu-playback-seek-backward = 快退 10 秒
appmenu-playback-volume-up = 提高音量
appmenu-playback-volume-down = 降低音量
appmenu-playback-toggle-mute = 静音
appmenu-playback-toggle-shuffle = 随机播放
appmenu-playback-repeat = 循环模式
appmenu-playback-repeat-off = 关闭循环
appmenu-playback-repeat-all = 列表循环
appmenu-playback-repeat-one = 单曲循环
appmenu-playback-toggle-lyrics = 显示歌词
appmenu-playback-toggle-playlist = 显示播放列表
appmenu-playback-toggle-fullscreen = 全屏播放器

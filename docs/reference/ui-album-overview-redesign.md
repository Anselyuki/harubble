# UI 改版方案：Album Overview 页与 Sidebar 精简

> 参考画布：Penpot「Harubble App Beta」帧（1280×800）  
> 状态：设计草案，待前端实现

---

## 一、背景与动机

### 当前问题

| 问题                   | 位置                       | 说明                                                                                      |
| ---------------------- | -------------------------- | ----------------------------------------------------------------------------------------- |
| 专辑列表与导航耦合     | `AlbumSidebarSection`      | 专辑列表挤占 sidebar 可用空间，与搜索栏、合集区域争夺纵向高度，随专辑数量增长体验劣化     |
| 首页信息密度低         | `HomeView`                 | 当前首页以横向滚动条（`HomeLatestAlbums`）+ 多个分组区块组成，没有一个"全量总览"入口      |
| 搜索结果停留在 sidebar | `AlbumSidebarSection`      | 搜索命中后结果挤压在 240px 宽的 sidebar 列中，阅读性差，无法展示封面图                    |
| Collections 语义模糊   | `CollectionSidebarSection` | 合集与专辑列表并列，不像其他音乐软件（Spotify / Apple Music）那样具有独立的"播放列表"语义 |

### 目标

1. **Sidebar 只承担导航职责**：Logo、Nav、Search Bar、Collections 四个区域，不再嵌入专辑滚动列表。
2. **Album Overview 成为主内容区默认页**：卡片式信息流展示全量专辑，点击「首页」导航或激活搜索栏均路由到此页。
3. **Collections 作为"播放列表"语义入口**：类比 Spotify 左侧 Library，用户合集与官方合集列在 sidebar 底部，点击直接进入合集详情。

---

## 二、Penpot Beta 画布对应关系

Penpot 中「Harubble App Beta」帧已呈现本方案的目标布局，以下是各区域的对应：

```
┌──────────────────────────────────────────────────────────────┐
│ Sidebar (240px)           │ Workspace (1040px)               │
│                           │                                   │
│  HARUKA                   │  [ 最近收听 ]                     │
│  BUBBLE                   │  ┌────┐ ┌────┐ ┌────┐ ┌────┐    │
│  ───────────────          │  │    │ │    │ │    │ │    │    │
│  ● Home  (active)         │  └────┘ └────┘ └────┘ └────┘    │
│  ○ Library                │                                   │
│  ○ Tags                   │  [ 全部专辑 ]                     │
│  [ Search library... ALL] │  ┌────┐ ┌────┐ ┌────┐ ┌────┐    │
│  ───────────────          │  │    │ │    │ │    │ │    │    │
│  Collections              │  └────┘ └────┘ └────┘ └────┘    │
│  ♪ Winter Nights          │  ┌────┐ ┌────┐ ┌────┐ ┌────┐    │
│  ♪ Study Vibes            │  │    │ │    │ │    │ │    │    │
│                           │  └────┘ └────┘ └────┘ └────┘    │
│                           │                                   │
├───────────────────────────┴───────────────────────────────────┤
│ Player Dock (80px) — 横贯全宽                                 │
└───────────────────────────────────────────────────────────────┘
```

---

## 三、Sidebar 精简

### 3.1 移除 `AlbumSidebarSection`

**涉及文件**

- `src/lib/components/app/AppSidebar.svelte` — 删除 `<AlbumSidebarSection>` 及其 `sidebar-library-region` 包裹层与相关 Props
- `src/App.svelte` — 移除向 `AppSidebar` 传递 `albums`、`selectedAlbumCid`、`searchLoading`、`searchResponse`、`onSelectSearchResult` 等专辑列表相关 props

> `reducedMotion`、`loadingAlbums`、`errorMsg` 这三个 prop 当前仅被转发给 `AlbumSidebarSection`，随 `AlbumSidebarSection` 一同从 `AppSidebar` 的 Props 接口中移除，不转移到其他组件。

**保留的 Props（Sidebar 仍需要）**

```typescript
// AppSidebar 精简后最小 Props
interface Props {
  isMacOS: boolean;
  currentView: AppView;
  searchQuery: string;
  searchScope: LibrarySearchScope;
  onNavigate: (view: AppView) => void;
  onSearchQueryChange: (query: string) => void;
  onSearchScopeChange: (scope: LibrarySearchScope) => void;

  collections: CollectionSummary[];
  selectedCollectionId: string | null;
  collectionsLoading: boolean;
  onCollectionSelect: (id: string) => void;
  onCollectionCreate: () => void;
  onCollectionImport: () => void;
}
```

### 3.2 Sidebar 布局结构（改版后）

```
aside.sidebar
  ├── .sidebar-drag-region       (macOS only)
  ├── BrandLogo
  ├── .sidebar-nav-region        ← SidebarNav（Nav + Search Bar）
  └── .sidebar-collections-region ← CollectionSidebarSection（撑满剩余高度）
```

`sidebar-library-region` 整体移除，`sidebar-collections-region` 从 `max-height: 35%` 改为 `flex: 1`，让合集列表充分利用纵向空间。

### 3.3 Search Bar 交互变更

搜索栏聚焦（`onfocus`）时改为切换到 `overview` 视图（见第四节），并将搜索结果渲染在主内容区而非 sidebar。

---

## 四、Album Overview 页

### 4.1 新增 AppView 状态

在 `shellStore` 中扩展 `AppView` 类型：

```typescript
// src/lib/features/shell/store.svelte.ts
export type AppView =
  | 'home'
  | 'overview'
  | 'library'
  | 'tagEditor'
  | 'collection';
//                            ^^^^^^^^^^^ 新增
```

> **注意**：`overview` 与原有 `library`（`AlbumWorkspace` 专辑详情页）相互独立。
> `overview` = 全量专辑总览 + 搜索结果展示；
> `library` = 进入某张专辑的曲目详情页（行为保持不变）。

### 4.2 导航触发条件

| 用户操作                      | 路由目标              | 说明                                                     |
| ----------------------------- | --------------------- | -------------------------------------------------------- |
| 点击 Sidebar「Library」       | `overview`            | Library 导航项绑定 AlbumOverview，作为专辑库主内容区入口 |
| 点击 Sidebar「Home」          | `home`                | Home 导航项保持原 HomeView，视图保留不变                 |
| 搜索栏聚焦 / 输入             | `overview`            | 聚焦后直接切换到 overview 页内展示搜索结果               |
| 点击专辑卡片                  | `library`（专辑详情） | 与现有 `handleSelectAlbum` 行为一致                      |
| 点击 Sidebar Collections 条目 | `collection`          | 与现有合集详情行为一致                                   |

### 4.3 AlbumOverview 组件设计

新建 `src/lib/components/app/AlbumOverview.svelte`：

```
AlbumOverview
├── 页头区（可选，非搜索状态时展示）
│   └── "最近收听" 横向滚动行 ← 复用 HomeRecentHistory 数据，精简展示
├── 主内容区（滚动容器）
│   ├── [搜索模式] 搜索结果列表（复用 AlbumSidebarSection 现有搜索结果渲染逻辑）
│   └── [浏览模式] 全量专辑卡片网格
│       ├── 区块标题："全部专辑 (N)"
│       └── .album-grid（CSS Grid，4列，响应式）
│           └── AlbumCard × N
└── 空状态（库为空时的引导）
```

**卡片布局**（对应 Penpot Beta 画布中 4 列网格）：

```css
.album-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(160px, 1fr));
  gap: 20px;
  padding: 0 24px 32px;
}
```

`AlbumCard` 在网格中以竖向布局展示（封面居上，文字居下），区别于 sidebar 中的横向紧凑布局。可通过给 `AlbumCard` 增加 `layout="grid"` prop 来切换内部样式，避免重写组件。

**卡片视觉风格参考**：对标 Apple Music 点击搜索后呈现的专辑卡片样式——封面图占据卡片上方主体区域（约 70-75%），圆角处理；封面图下方为专辑名（14px / 500）与艺人名（12px / 400），两者均左对齐，适当截断。整体无明显描边，悬停时轻微投影上浮。具体比例与间距在实现阶段根据视觉效果迭代确定。

### 4.4 搜索结果在 Overview 中的展示

搜索激活时，Overview 页替换网格内容为搜索结果卡片流：

- 结果分组：Album 命中 / Song 命中 用 section header 区分
- 每条结果展示封面缩略图（48×48）、标题、副标题（艺人/所属专辑）
- 搜索索引构建中的进度条保留在此区域（从 sidebar 迁过来）

---

## 五、Collections 作为「播放列表」

### 5.1 语义定位

| 对比       | Spotify                  | Apple Music         | Harubble（改版后）      |
| ---------- | ------------------------ | ------------------- | ----------------------- |
| 左侧固定列 | Your Library（歌单列表） | Library（分类浏览） | Collections（合集列表） |
| 点击条目   | 打开歌单详情             | 打开专辑/歌单       | 打开合集详情            |
| 支持操作   | 新建、排序、搜索         | 添加到库            | 新建、导入（现有）      |

### 5.2 Sidebar Collections 区域布局

```
.sidebar-collections-region
  ├── .section-header
  │   ├── "Collections"（标签）
  │   └── [+ 新建] [⬇ 导入]（操作按钮）
  └── .collection-list（flex: 1, overflow-y: auto）
      ├── [官方合集] 官方合集条目 × N（图标 + 名称 + 曲目数角标）
      └── [我的合集] 用户合集条目 × N（同上）
```

`CollectionSidebarSection` 现有实现基本满足需求，主要调整：

1. **去除 `max-height: 35%` 上限**：让合集列表在 sidebar 中充分展开
2. **增加曲目数/时长角标**：在条目右侧展示 `songCount`，信息密度对齐主流音乐 App
3. **播放按钮**：鼠标悬停时在条目右侧显示"播放合集"图标按钮（类 Spotify hover 交互）

### 5.3 合集详情页（Collection View）

`collection` AppView 对应的 `CollectionDetailPanel` 保持现有行为，不在本次改版范围内。

---

## 六、导航条（SidebarNav）调整

### 6.1 导航项变更

导航项顺序调整为 **Library → Home → Tags**（Library 上移至首位），各项路由目标如下：

| 导航项  | 当前目标    | 改版后目标                                  |
| ------- | ----------- | ------------------------------------------- |
| Library | `library`   | `overview`（绑定 AlbumOverview 专辑库总览） |
| Home    | `home`      | `home`（保持不变，HomeView 保留）           |
| Tags    | `tagEditor` | `tagEditor`（不变）                         |

### 6.2 Search Bar 聚焦行为

```typescript
// SidebarNav.svelte 现有逻辑
onfocus={() => {
  if (currentView !== 'library') onNavigate('library');
}}

// 改版后
onfocus={() => {
  if (currentView !== 'overview') onNavigate('overview');
}}
```

---

## 七、受影响的文件清单

| 文件                                                     | 变更类型      | 说明                                                                                      |
| -------------------------------------------------------- | ------------- | ----------------------------------------------------------------------------------------- |
| `src/lib/features/shell/store.svelte.ts`                 | **修改**      | `AppView` 增加 `'overview'`，增加 `navigateToOverview()`                                  |
| `src/lib/components/app/AppSidebar.svelte`               | **修改**      | 移除 `AlbumSidebarSection` 及相关 Props                                                   |
| `src/lib/components/app/SidebarNav.svelte`               | **修改**      | Library 与 Home 导航项位置互换；Library 目标改为 `overview`；搜索聚焦目标改为 `overview`  |
| `src/lib/components/app/CollectionSidebarSection.svelte` | **修改**      | 移除 `max-height` 限制，增加曲目数角标，增加 hover 播放按钮                               |
| `src/lib/components/app/AlbumOverview.svelte`            | **新增**      | 全量专辑卡片网格 + 搜索结果展示                                                           |
| `src/lib/components/AlbumCard.svelte`                    | **修改**      | 增加 `layout?: 'list' \| 'grid'` prop，grid 模式下封面居上、文字居下                      |
| `src/App.svelte`                                         | **修改**      | 增加 `overview` 视图的路由渲染；移除向 `AppSidebar` 传递的专辑列表 props                  |
| `src/lib/types.ts`                                       | 可能 **修改** | 如 `AlbumOverview` 需要新的数据结构；`AppView` 定义在 `store.svelte.ts`，此文件仅按需调整 |

**不在本次范围内（保持不变）**

- `AlbumSidebarSection.svelte`（保留文件，仅从 `AppSidebar` 中解除挂载）
- `HomeView.svelte` 及其子组件（`HomeLatestAlbums` 等）
- `CollectionDetailPanel.svelte`
- 所有 Rust 后端 command（数据已足够，不需要新增接口）

---

## 八、实现顺序建议

1. **shellStore 扩展** — 加 `overview` 视图类型，成本最低，无风险
2. **SidebarNav 调整** — Library 与 Home 导航项位置互换；Library 目标改为 `overview`；`onfocus` 目标改为 `overview`
3. **AlbumCard `layout` prop** — 为 grid 模式准备样式变体，不影响现有 list 行为
4. **AlbumOverview 组件** — 新建，复用现有 `albums` 数据流与搜索数据流
5. **App.svelte 路由接入** — 在 workspace 区域增加 `overview` 分支渲染，此步完成后可完整验证 Overview 功能
6. **AppSidebar Props 精简** — 移除 `AlbumSidebarSection` 挂载及相关 props，验证 sidebar 视觉完整性
7. **CollectionSidebarSection 增强** — 曲目数角标、hover 播放按钮（可独立迭代）

> **顺序约束**：步骤 6（AppSidebar 精简）必须在步骤 5（App.svelte 路由接入）完成后执行，避免专辑数据从 sidebar 移除后主内容区尚未就位导致应用出现空白状态。

---

## 九、设计 Token 参考

以下 Token 已在现有 `app.css` 定义，`AlbumOverview` 直接引用即可：

| 用途         | Token                                                   |
| ------------ | ------------------------------------------------------- |
| 网格背景     | `--surface-workspace`                                   |
| 卡片悬停     | `--hover-bg-elevated`                                   |
| 选中态底色   | `--surface-state`（`rgba(accent-rgb, 0.08)`）           |
| 区块标题字体 | `--font-display`，`font-size: 1rem`，`font-weight: 600` |
| 专辑名字体   | `14px / 500`，`var(--text-primary)`                     |
| 艺人字体     | `12px / 400`，`var(--text-secondary)`                   |
| 卡片圆角     | `12px`（与现有 `AlbumCard` 一致）                       |

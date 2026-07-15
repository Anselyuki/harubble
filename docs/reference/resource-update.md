# 资源更新说明

## 标签注册表（Tag Registry）

`data/tag_registry.json` 维护专辑和歌曲的元数据标签，用于分组浏览与搜索索引。

### 运行时更新

- **发布模式**：应用启动后从仓库 `main` 分支拉取注册表，并用 `updatedAt` 与本地缓存比较。新版本会先原子写入缓存，再替换内存数据。
- **开发模式**：启动时读取工作区的 `data/tag_registry.json`，便于直接验证本地修改。
- **搜索索引**：注册表变化少于 50 个专辑时优先增量刷新；无活跃索引、增量失败或变化规模较大时回退到全量重建。
- **失败处理**：网络、解析或远端 schema 校验失败时保留当前内存和缓存，不阻塞启动；本地缓存缺失、损坏或 schema 不兼容时以空注册表初始化，等待有效同步。

### 发布标签数据

1. 在 Tag Editor 中编辑并导出完整注册表，或直接修改 `data/tag_registry.json`。
2. 更新根级 `updatedAt` 为当前 ISO 8601 时间。
3. 运行 `bun run sort:tags` 统一专辑顺序。
4. 检查 diff，并运行 `cargo test --manifest-path src-tauri/Cargo.toml tag_registry`。
5. 合入 `main` 后，发布版会在后续启动时自动同步。

### 数据结构

| 字段              | 说明                                                                    |
| ----------------- | ----------------------------------------------------------------------- |
| `schemaVersion`   | Schema 版本号，当前为 2                                                 |
| `updatedAt`       | 注册表版本标识，用于判断远端内容是否变化                                |
| `tagDimensions`   | 标签维度定义（key + 多语种 label），可选 `scope: "song"` 限定为单曲维度 |
| `typeDefinitions` | 专辑类型枚举的多语种映射                                                |
| `albums`          | 专辑标签条目，每条以 `cid` 标识                                         |
| `songs`           | 单曲标签条目；读取时与所属专辑标签合并                                  |

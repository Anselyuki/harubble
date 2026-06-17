//! 上游接口访问与响应模型。
//!
//! 该模块定义专辑、单曲与专辑详情等上游响应结构，并提供带响应缓存的 API 客户端，
//! 供下载、播放预取、本地库存增强与前端命令层复用。

use crate::local_inventory::{AlbumDownloadBadge, TrackDownloadBadge};
use anyhow::Result;
use lru::LruCache;
use reqwest::Client;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::num::NonZeroUsize;
use std::sync::{Arc, Mutex, RwLock};
use std::time::Duration;

const DEFAULT_BASE_URL: &str = "https://monster-siren.hypergryph.com/api";
const DEFAULT_CACHE_CAPACITY: usize = 100;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(15);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(300);
const POOL_IDLE_TIMEOUT: Duration = Duration::from_secs(60);
const MAX_JSON_RESPONSE_SIZE: u64 = 10 * 1024 * 1024;
const MAX_AUDIO_RESPONSE_SIZE: u64 = 2 * 1024 * 1024 * 1024;

/// 专辑列表查询返回的基础条目。
///
/// 适用于专辑列表页、搜索结果中的专辑摘要展示，或在不需要完整曲目明细时
/// 进行轻量展示。
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Album {
    /// 专辑 CID。
    pub cid: String,
    /// 专辑名称。
    pub name: String,
    /// 专辑封面地址。
    #[serde(rename = "coverUrl")]
    pub cover_url: String,
    /// 专辑艺术家列表。
    #[serde(alias = "artistes")]
    pub artists: Vec<String>,
    /// 本地库存补充后的下载徽标信息。
    #[serde(default)]
    pub download: AlbumDownloadBadge,
    /// 自定义元数据 tag，由后端 enrichment 注入。
    #[serde(default)]
    pub tags: Vec<TagEntry>,
}

/// 单张专辑的完整详情快照。
///
/// 适用于专辑详情页、批量下载前的曲目枚举，或需要结合曲目列表推导本地库存
/// 状态的场景。
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct AlbumDetail {
    /// 专辑 CID。
    pub cid: String,
    /// 专辑名称。
    pub name: String,
    /// 专辑简介。
    pub intro: Option<String>,
    /// 专辑归属信息。
    pub belong: String,
    /// 标准封面地址。
    #[serde(rename = "coverUrl")]
    pub cover_url: String,
    /// 深色封面地址。
    #[serde(rename = "coverDeUrl")]
    pub cover_de_url: Option<String>,
    /// 专辑艺术家列表；上游缺失时可能为空。
    #[serde(alias = "artistes")]
    pub artists: Option<Vec<String>>,
    /// 本地库存补充后的专辑下载徽标。
    #[serde(default)]
    pub download: AlbumDownloadBadge,
    /// 自定义元数据 tag，由后端 enrichment 注入。
    #[serde(default)]
    pub tags: Vec<TagEntry>,
    /// 专辑内歌曲列表。
    pub songs: Vec<SongEntry>,
}

/// 专辑详情中的歌曲摘要条目。
///
/// 适用于在专辑上下文中展示曲目列表；当需要单曲下载地址、歌词或 MV 信息时，
/// 应进一步调用 [`ApiClient::get_song_detail`] 获取完整详情。
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct SongEntry {
    /// 歌曲 CID。
    pub cid: String,
    /// 歌曲名称。
    pub name: String,
    /// 艺术家列表。
    #[serde(alias = "artistes")]
    pub artists: Vec<String>,
    /// 本地库存补充后的歌曲下载徽标。
    #[serde(default)]
    pub download: TrackDownloadBadge,
    /// 自定义元数据 tag，由后端 enrichment 注入。
    #[serde(default)]
    pub tags: Vec<TagEntry>,
}

/// 单曲的完整详情快照。
///
/// 适用于播放、下载、歌词抓取或展示 MV 入口等需要访问单曲级资源地址的场景。
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct SongDetail {
    /// 歌曲 CID。
    pub cid: String,
    /// 歌曲名称。
    pub name: String,
    /// 所属专辑 CID。
    #[serde(rename = "albumCid")]
    pub album_cid: String,
    /// 原始音频下载地址。
    #[serde(rename = "sourceUrl")]
    pub source_url: String,
    /// 歌词地址；未提供时为空。
    #[serde(rename = "lyricUrl")]
    pub lyric_url: Option<String>,
    /// MV 地址；未提供时为空。
    #[serde(rename = "mvUrl")]
    pub mv_url: Option<String>,
    /// MV 封面地址；未提供时为空。
    #[serde(rename = "mvCoverUrl")]
    pub mv_cover_url: Option<String>,
    /// 艺术家列表。
    pub artists: Vec<String>,
    /// 本地库存补充后的歌曲下载徽标。
    #[serde(default)]
    pub download: TrackDownloadBadge,
    /// 自定义元数据 tag，由后端 enrichment 注入。
    #[serde(default)]
    pub tags: Vec<TagEntry>,
}

/// 已解析的单维度 tag 条目，用于前端展示和搜索。
///
/// `dimension` 为当前 locale 下的维度展示名，`values` 为同一维度下所有命中值的去重列表。
/// enrichment 阶段负责将 i18n 原始数据解析为单语言文本，前端无需处理 locale 逻辑。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TagEntry {
    /// 维度展示名（已本地化）。
    pub dimension: String,
    /// 当前维度下的 tag 值列表（已本地化、已去重）。
    pub values: Vec<String>,
    /// 每个 tag 值对应的可选颜色（hex 色值），与 `values` 索引一一对应。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub colors: Vec<Option<String>>,
}

#[derive(Debug, Deserialize)]
struct ApiResponse<T> {
    code: i32,
    #[allow(dead_code)]
    msg: String,
    data: T,
}

/// 用于访问上游公开接口的强类型客户端。
///
/// 该客户端统一封装了专辑/单曲查询、远端资源下载与响应缓存逻辑，适合作为
/// 上层播放器、下载服务或 Tauri command 的共享数据入口。
#[derive(Clone)]
pub struct ApiClient {
    client: Arc<RwLock<Client>>,
    base_url: String,
    response_cache: Arc<Mutex<LruCache<String, Vec<u8>>>>,
}

impl ApiClient {
    /// 使用默认基址与缓存容量创建 API 客户端。
    ///
    /// 适用于生产环境默认接入；返回值为可复用的客户端实例。若 HTTP 客户端构造
    /// 失败，会直接返回错误。
    pub fn new() -> Result<Self> {
        Self::new_with_config(DEFAULT_BASE_URL.to_string(), DEFAULT_CACHE_CAPACITY)
    }

    fn new_with_config(base_url: String, capacity: usize) -> Result<Self> {
        let client = Self::build_client()?;
        let capacity = NonZeroUsize::new(capacity).expect("cache capacity must be non-zero");
        Ok(Self {
            client: Arc::new(RwLock::new(client)),
            base_url: base_url.trim_end_matches('/').to_string(),
            response_cache: Arc::new(Mutex::new(LruCache::new(capacity))),
        })
    }

    fn build_client() -> Result<Client> {
        Ok(Client::builder()
            .user_agent("Mozilla/5.0 (compatible; harubble)")
            .connect_timeout(CONNECT_TIMEOUT)
            .timeout(REQUEST_TIMEOUT)
            .pool_idle_timeout(POOL_IDLE_TIMEOUT)
            .build()?)
    }

    fn http_client(&self) -> Client {
        self.client
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    /// 重建内部 HTTP 客户端以适应网络环境变化。
    ///
    /// 当系统代理设置变更或网络接口切换后，已有的 HTTP 客户端可能持有过期的连接池
    /// 与代理配置。调用此方法会丢弃旧客户端并以当前系统环境重新构建，使后续请求
    /// 能够正确路由。若重建失败则保留原有客户端不变。
    pub fn reset_http_client(&self) -> Result<()> {
        let new_client = Self::build_client()?;
        *self.client.write().unwrap_or_else(|e| e.into_inner()) = new_client;
        self.clear_response_cache();
        Ok(())
    }

    fn api_url(&self, path: &str) -> String {
        format!("{}/{}", self.base_url, path.trim_start_matches('/'))
    }

    fn cache_key(method: &str, resource: &str) -> String {
        format!("{:x}", md5::compute(format!("{method}:{resource}")))
    }

    fn read_cached_bytes(&self, cache_key: &str) -> Option<Vec<u8>> {
        self.response_cache
            .lock()
            .ok()
            .and_then(|mut cache| cache.get(cache_key).cloned())
    }

    fn write_cached_bytes(&self, cache_key: String, bytes: &[u8]) {
        if let Ok(mut cache) = self.response_cache.lock() {
            cache.put(cache_key, bytes.to_vec());
        }
    }

    async fn fetch_response_bytes(&self, url: &str, accept_json: bool) -> Result<Vec<u8>> {
        self.fetch_response_bytes_with_retry(url, accept_json, 1)
            .await
    }

    async fn fetch_response_bytes_with_retry(
        &self,
        url: &str,
        accept_json: bool,
        max_retries: usize,
    ) -> Result<Vec<u8>> {
        let mut last_error = None;

        for attempt in 0..=max_retries {
            let mut request = self.http_client().get(url);
            if accept_json {
                request = request.header("Accept", "application/json");
            }

            match request.send().await {
                Ok(response) => match response.error_for_status() {
                    Ok(response) => {
                        if let Some(len) = response.content_length() {
                            anyhow::ensure!(
                                len <= MAX_JSON_RESPONSE_SIZE,
                                "response too large: {len} bytes exceeds limit of {MAX_JSON_RESPONSE_SIZE}"
                            );
                        }
                        let bytes = response.bytes().await?;
                        anyhow::ensure!(
                            (bytes.len() as u64) <= MAX_JSON_RESPONSE_SIZE,
                            "response too large: {} bytes exceeds limit of {MAX_JSON_RESPONSE_SIZE}",
                            bytes.len()
                        );
                        return Ok(bytes.to_vec());
                    }
                    Err(e) => {
                        last_error = Some(e.into());
                    }
                },
                Err(e) => {
                    // 检测是否为网络连接错误
                    let is_connection_error = e.is_connect() || e.is_timeout();
                    last_error = Some(e.into());

                    // 如果是连接错误且还有重试机会，尝试重建客户端
                    if is_connection_error && attempt < max_retries {
                        let _ = self.reset_http_client();
                        // 短暂延迟，让系统有时间完成网络配置变更
                        tokio::time::sleep(Duration::from_millis(300)).await;
                        continue;
                    }
                }
            }

            // 非连接错误或已用尽重试次数，直接返回错误
            if let Some(err) = last_error {
                return Err(err);
            }
        }

        Err(last_error.unwrap_or_else(|| anyhow::anyhow!("Request failed after retries")))
    }

    async fn fetch_streamed_bytes(
        &self,
        url: &str,
        on_progress: impl FnMut(u64, Option<u64>),
    ) -> Result<Vec<u8>> {
        self.fetch_streamed_bytes_with_retry(url, on_progress, 1)
            .await
    }

    async fn fetch_streamed_bytes_with_retry(
        &self,
        url: &str,
        mut on_progress: impl FnMut(u64, Option<u64>),
        max_retries: usize,
    ) -> Result<Vec<u8>> {
        use futures::StreamExt;

        let mut last_error = None;

        for attempt in 0..=max_retries {
            match self.http_client().get(url).send().await {
                Ok(response) => match response.error_for_status() {
                    Ok(response) => {
                        let total = response.content_length();
                        if let Some(len) = total {
                            anyhow::ensure!(
                                len <= MAX_AUDIO_RESPONSE_SIZE,
                                "response too large: {len} bytes exceeds limit of {MAX_AUDIO_RESPONSE_SIZE}"
                            );
                        }
                        let mut stream = response.bytes_stream();
                        let mut bytes = Vec::new();
                        let mut downloaded = 0_u64;

                        while let Some(chunk) = stream.next().await {
                            let chunk = chunk?;
                            downloaded += chunk.len() as u64;
                            anyhow::ensure!(
                                downloaded <= MAX_AUDIO_RESPONSE_SIZE,
                                "download exceeded size limit of {MAX_AUDIO_RESPONSE_SIZE} bytes"
                            );
                            bytes.extend_from_slice(&chunk);
                            on_progress(downloaded, total);
                        }

                        return Ok(bytes);
                    }
                    Err(e) => {
                        last_error = Some(e.into());
                    }
                },
                Err(e) => {
                    let is_connection_error = e.is_connect() || e.is_timeout();
                    last_error = Some(e.into());

                    if is_connection_error && attempt < max_retries {
                        let _ = self.reset_http_client();
                        tokio::time::sleep(Duration::from_millis(300)).await;
                        continue;
                    }
                }
            }

            if let Some(err) = last_error {
                return Err(err);
            }
        }

        Err(last_error.unwrap_or_else(|| anyhow::anyhow!("Request failed after retries")))
    }

    fn decode_api_response<T: DeserializeOwned>(bytes: &[u8]) -> Result<T> {
        let response: ApiResponse<T> = serde_json::from_slice(bytes)?;
        anyhow::ensure!(response.code == 0, "API error code {}", response.code);
        Ok(response.data)
    }

    async fn get_cached_api_data<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        let cache_key = Self::cache_key("GET", path);
        if let Some(bytes) = self.read_cached_bytes(&cache_key) {
            return Self::decode_api_response(&bytes);
        }

        let bytes = self.fetch_response_bytes(&self.api_url(path), true).await?;
        let data = Self::decode_api_response(&bytes)?;
        self.write_cached_bytes(cache_key, &bytes);
        Ok(data)
    }

    /// 清空内部响应缓存。
    ///
    /// 适用于上游资源已更新、需要强制重新拉取，或测试场景下希望消除缓存影响时
    /// 调用。该操作只影响内存缓存，不会触发任何网络请求。
    pub fn clear_response_cache(&self) {
        if let Ok(mut cache) = self.response_cache.lock() {
            cache.clear();
        }
    }

    /// 获取专辑列表。
    ///
    /// 返回值为上游当前可见的专辑列表；当缓存命中时不会发起新的网络请求。
    pub async fn get_albums(&self) -> Result<Vec<Album>> {
        self.get_cached_api_data("albums").await
    }

    /// 根据专辑 CID 获取专辑详情。
    ///
    /// 入参 `album_cid` 为上游专辑唯一标识；返回值包含专辑元信息与曲目列表。
    /// 当缓存命中时会直接复用内存中的响应快照。
    pub async fn get_album_detail(&self, album_cid: &str) -> Result<AlbumDetail> {
        self.get_cached_api_data(&format!("album/{album_cid}/detail"))
            .await
    }

    /// 根据歌曲 CID 获取单曲详情。
    ///
    /// 入参 `cid` 为上游歌曲唯一标识；返回值包含下载地址、歌词地址与关联专辑信息。
    pub async fn get_song_detail(&self, cid: &str) -> Result<SongDetail> {
        self.get_cached_api_data(&format!("song/{cid}")).await
    }

    /// 以流式方式下载远端资源。
    ///
    /// 入参 `url` 为待下载资源地址；`on_chunk` 会在每个分块到达后收到当前分块、
    /// 累计已下载字节数与可选总大小。当回调返回 `false` 时会提前停止下载，并把当前
    /// 中断视为正常结束而不是错误。
    ///
    /// 适用于边下载边处理的场景，例如实时写盘、播放器边缓冲边解码，或需要自行控制
    /// 取消时机的流水线下载。
    ///
    /// 该方法在遇到连接错误时会自动重试一次，并在重试前重建 HTTP 客户端以适应可能的
    /// 网络环境变化（如代理开关、VPN 切换）。
    pub async fn download_stream(
        &self,
        url: &str,
        mut on_chunk: impl FnMut(&[u8], u64, Option<u64>) -> Result<bool>,
    ) -> Result<()> {
        use futures::StreamExt;

        crate::url_validator::validate_download_url(url)?;

        let mut last_error = None;
        let max_retries = 1;

        for attempt in 0..=max_retries {
            match self.http_client().get(url).send().await {
                Ok(resp) => match resp.error_for_status() {
                    Ok(resp) => {
                        let total = resp.content_length();
                        if let Some(len) = total {
                            anyhow::ensure!(
                                len <= MAX_AUDIO_RESPONSE_SIZE,
                                "response too large: {len} bytes exceeds limit of {MAX_AUDIO_RESPONSE_SIZE}"
                            );
                        }
                        let mut stream = resp.bytes_stream();
                        let mut downloaded = 0_u64;

                        while let Some(chunk) = stream.next().await {
                            let chunk = chunk?;
                            downloaded += chunk.len() as u64;
                            anyhow::ensure!(
                                downloaded <= MAX_AUDIO_RESPONSE_SIZE,
                                "download exceeded size limit of {MAX_AUDIO_RESPONSE_SIZE} bytes"
                            );
                            if !on_chunk(&chunk, downloaded, total)? {
                                break;
                            }
                        }

                        return Ok(());
                    }
                    Err(e) => {
                        last_error = Some(e.into());
                    }
                },
                Err(e) => {
                    let is_connection_error = e.is_connect() || e.is_timeout();
                    last_error = Some(e.into());

                    if is_connection_error && attempt < max_retries {
                        let _ = self.reset_http_client();
                        tokio::time::sleep(Duration::from_millis(300)).await;
                        continue;
                    }
                }
            }

            if let Some(err) = last_error {
                return Err(err);
            }
        }

        Err(last_error.unwrap_or_else(|| anyhow::anyhow!("Request failed after retries")))
    }

    /// 下载完整字节内容，并在可命中时复用内部缓存。
    ///
    /// 入参 `url` 为待下载资源地址；`on_progress` 会收到累计已下载字节数与可选总大小。
    /// 返回值为完整资源字节；若缓存命中，会立即以“已完成”进度回调一次。
    ///
    /// 适用于封面、歌词附件或整段音频等需要一次性拿到完整载荷的场景；对于超大文件
    /// 或希望边下载边消费的调用方，应改用 [`ApiClient::download_stream`]。
    pub async fn download_bytes(
        &self,
        url: &str,
        mut on_progress: impl FnMut(u64, Option<u64>),
    ) -> Result<Vec<u8>> {
        crate::url_validator::validate_download_url(url)?;

        let cache_key = Self::cache_key("GET", url);
        if let Some(bytes) = self.read_cached_bytes(&cache_key) {
            on_progress(bytes.len() as u64, Some(bytes.len() as u64));
            return Ok(bytes);
        }

        let bytes = self.fetch_streamed_bytes(url, on_progress).await?;
        self.write_cached_bytes(cache_key, &bytes);
        Ok(bytes)
    }

    /// 下载文本内容，并自动移除 UTF-8 BOM。
    ///
    /// 入参 `url` 为文本资源地址；返回值为按 UTF-8 宽松解码后的字符串内容。
    /// 该接口内部会先下载完整字节，再统一做 BOM 清理，因此适用于歌词等体量较小的
    /// 文本资源，不适合超大文本流式处理。
    pub async fn download_text(&self, url: &str) -> Result<String> {
        crate::url_validator::validate_download_url(url)?;

        let bytes = self.download_bytes(url, |_, _| {}).await?;
        Ok(String::from_utf8_lossy(&bytes)
            .trim_start_matches('\u{feff}')
            .to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::ApiClient;
    use httpmock::prelude::*;

    impl ApiClient {
        fn new_for_test(base_url: String, capacity: usize) -> anyhow::Result<Self> {
            Self::new_with_config(base_url, capacity)
        }
    }

    fn album_detail_body(cid: &str, name: &str) -> String {
        format!(
            r#"{{"code":0,"msg":"ok","data":{{"cid":"{cid}","name":"{name}","intro":null,"belong":"EP","coverUrl":"https://example.com/{cid}.jpg","coverDeUrl":null,"artistes":["Test Artist"],"songs":[]}}}}"#
        )
    }

    #[tokio::test]
    async fn returns_cached_album_detail_without_second_network_call() {
        let server = MockServer::start();
        let album_mock = server.mock(|when, then| {
            when.method(GET).path("/api/album/alpha/detail");
            then.status(200)
                .header("content-type", "application/json")
                .body(album_detail_body("alpha", "Alpha"));
        });

        let client =
            ApiClient::new_for_test(format!("{}/api", server.base_url()), 100).expect("client");

        let first = client.get_album_detail("alpha").await.expect("first call");
        let second = client.get_album_detail("alpha").await.expect("second call");

        assert_eq!(first.cid, "alpha");
        assert_eq!(second.name, "Alpha");
        album_mock.assert_hits(1);
    }

    #[tokio::test]
    async fn does_not_cache_failed_upstream_response() {
        let server = MockServer::start();
        let failure_mock = server.mock(|when, then| {
            when.method(GET).path("/api/album/beta/detail");
            then.status(500);
        });

        let client =
            ApiClient::new_for_test(format!("{}/api", server.base_url()), 100).expect("client");

        assert!(client.get_album_detail("beta").await.is_err());
        assert!(client.get_album_detail("beta").await.is_err());
        failure_mock.assert_hits(2);
    }

    #[tokio::test]
    async fn evicts_least_recently_used_response_when_capacity_is_exceeded() {
        let server = MockServer::start();
        let alpha_mock = server.mock(|when, then| {
            when.method(GET).path("/api/album/alpha/detail");
            then.status(200)
                .header("content-type", "application/json")
                .body(album_detail_body("alpha", "Alpha"));
        });
        let beta_mock = server.mock(|when, then| {
            when.method(GET).path("/api/album/beta/detail");
            then.status(200)
                .header("content-type", "application/json")
                .body(album_detail_body("beta", "Beta"));
        });

        let client =
            ApiClient::new_for_test(format!("{}/api", server.base_url()), 1).expect("client");

        client.get_album_detail("alpha").await.expect("alpha first");
        client.get_album_detail("beta").await.expect("beta first");
        client
            .get_album_detail("alpha")
            .await
            .expect("alpha second");

        alpha_mock.assert_hits(2);
        beta_mock.assert_hits(1);
    }

    #[tokio::test]
    #[ignore = "hits the live upstream API"]
    async fn downloads_real_lyrics_text() {
        let client = ApiClient::new().expect("client");
        let detail = client.get_song_detail("048760").await.expect("song detail");
        let lyric_url = detail.lyric_url.expect("lyric url");
        let lyric_text = client.download_text(&lyric_url).await.expect("lyric text");

        assert!(
            !lyric_text.trim().is_empty(),
            "lyric text should not be empty"
        );
        assert!(
            lyric_text.contains("[00:"),
            "lyric text should contain timestamped LRC lines"
        );
    }
}

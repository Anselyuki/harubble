//! 主题包 URL 下载器 + SSRF 防护（Phase 1 遗留 · B）。
//!
//! # 模块职责
//!
//! `download_theme_package` 从远程 URL 抓取主题包 JSON，全流程强制以下约束：
//!
//! 1. **协议白名单**：仅接受 `https://`；`http/ftp/file/data/gopher/...` 全拒
//! 2. **DNS 解析后逐 IP 校验**：拒绝 IANA 特殊地址段（loopback / private / link-local /
//!    CGNAT / multicast / broadcast / unspecified / documentation / reserved）
//! 3. **主机头单独校验**：URL 本身若直接使用 IP，也走同一套段禁名单
//! 4. **响应大小限制**：`Content-Length` header + 流式读取双重限制在
//!    `MAX_PACKAGE_JSON_BYTES`（512 KiB）
//! 5. **超时**：连接 5s + 总读取 15s
//! 6. **重定向禁用**：`redirect(Policy::none())`，防止 302 跳到私有段的 DNS rebinding 变种
//! 7. **Content-Type**：允许 `application/json` / `text/plain` / `application/octet-stream`；
//!    其他一律拒绝
//!
//! # 与主方案 §12 的对齐
//!
//! 主方案指出 URL 抓取必须"防 DNS rebinding、防重定向、防私有段"，本模块通过
//! 「先解析 IP → 校验段 → 用 IP 直连」的手法闭合 DNS rebinding 的 TOCTOU 窗口。

use crate::theme_packages::service::MAX_PACKAGE_JSON_BYTES;
use ipnet::{IpNet, Ipv4Net, Ipv6Net};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::sync::OnceLock;
use std::time::Duration;
use tokio::net::lookup_host;
use url::Url;

/// URL 下载连接建立超时。
const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
/// URL 下载总耗时上限（含 TLS + 首字节 + 完整读取）。
const TOTAL_TIMEOUT: Duration = Duration::from_secs(15);
/// Content-Type 白名单前缀（大小写不敏感，头部包含即可）。
const ALLOWED_CONTENT_TYPES: &[&str] =
    &["application/json", "text/plain", "application/octet-stream"];

/// IANA 特殊用途 IPv4 段禁名单（RFC 6890 / RFC 1918 / RFC 6598）。
///
/// 每个 CIDR 覆盖一类 SSRF 攻击面：
/// - `0.0.0.0/8`：unspecified / current network（RFC 6890）
/// - `10.0.0.0/8`：RFC 1918 私有段
/// - `100.64.0.0/10`：CGNAT（RFC 6598）
/// - `127.0.0.0/8`：loopback
/// - `169.254.0.0/16`：link-local（含云元数据 169.254.169.254）
/// - `172.16.0.0/12`：RFC 1918 私有段
/// - `192.0.0.0/24` / `192.0.2.0/24`：IETF protocol assignments / documentation
/// - `192.168.0.0/16`：RFC 1918 私有段
/// - `198.18.0.0/15`：benchmark（RFC 2544）
/// - `198.51.100.0/24` / `203.0.113.0/24`：documentation
/// - `224.0.0.0/4`：multicast
/// - `240.0.0.0/4`：reserved
/// - `255.255.255.255/32`：limited broadcast
fn blocked_ipv4_networks() -> &'static [Ipv4Net] {
    static NETS: OnceLock<Vec<Ipv4Net>> = OnceLock::new();
    NETS.get_or_init(|| {
        [
            "0.0.0.0/8",
            "10.0.0.0/8",
            "100.64.0.0/10",
            "127.0.0.0/8",
            "169.254.0.0/16",
            "172.16.0.0/12",
            "192.0.0.0/24",
            "192.0.2.0/24",
            "192.168.0.0/16",
            "198.18.0.0/15",
            "198.51.100.0/24",
            "203.0.113.0/24",
            "224.0.0.0/4",
            "240.0.0.0/4",
            "255.255.255.255/32",
        ]
        .iter()
        .filter_map(|s| s.parse().ok())
        .collect()
    })
}

/// IPv6 段禁名单：`::/128` / `::1/128` / `fc00::/7` / `fe80::/10` / `ff00::/8`。
fn blocked_ipv6_networks() -> &'static [Ipv6Net] {
    static NETS: OnceLock<Vec<Ipv6Net>> = OnceLock::new();
    NETS.get_or_init(|| {
        [
            "::/128",        // unspecified
            "::1/128",       // loopback
            "fc00::/7",      // unique local
            "fe80::/10",     // link-local
            "ff00::/8",      // multicast
            "2001:db8::/32", // documentation
        ]
        .iter()
        .filter_map(|s| s.parse().ok())
        .collect()
    })
}

/// 判断单个 IP 是否命中禁名单。
pub(crate) fn is_blocked_ip(addr: IpAddr) -> bool {
    match addr {
        IpAddr::V4(v4) => is_blocked_ipv4(v4),
        IpAddr::V6(v6) => is_blocked_ipv6(v6),
    }
}

fn is_blocked_ipv4(addr: Ipv4Addr) -> bool {
    let ip = IpNet::V4(Ipv4Net::from(addr));
    blocked_ipv4_networks()
        .iter()
        .any(|net| IpNet::V4(*net).contains(&ip))
}

fn is_blocked_ipv6(addr: Ipv6Addr) -> bool {
    // 兼容 IPv4-mapped IPv6：先转 v4 再校验
    if let Some(v4) = addr.to_ipv4() {
        if is_blocked_ipv4(v4) {
            return true;
        }
    }
    let ip = IpNet::V6(Ipv6Net::from(addr));
    blocked_ipv6_networks()
        .iter()
        .any(|net| IpNet::V6(*net).contains(&ip))
}

/// 校验 URL 协议 + 端口 + 主机名合法性（形态层）。
///
/// 语义层的 IP 段禁名单由 `resolve_and_verify_host` 在异步阶段处理。
pub(crate) fn validate_url_shape(raw: &str) -> Result<Url, String> {
    let url = Url::parse(raw).map_err(|e| format!("invalid url: {e}"))?;
    match url.scheme() {
        "https" => {}
        other => return Err(format!("only https scheme allowed, got '{other}'")),
    }
    if url.host_str().is_none() {
        return Err("url must have a host".to_string());
    }
    if url.username() != "" || url.password().is_some() {
        return Err("url must not contain credentials".to_string());
    }
    // 端口白名单：仅允许 443 / None（默认）；防止利用非常规端口越过防火墙
    match url.port() {
        None | Some(443) => Ok(url),
        Some(p) => Err(format!("only port 443 allowed, got {p}")),
    }
}

/// 解析 URL 主机对应的所有 IP 地址，校验都不在禁名单内，返回首个合法地址。
///
/// **DNS rebinding 防护**：即使解析出多个地址，只要其中任一命中禁名单就整体拒绝。
/// 返回的地址会通过 reqwest 的静态 resolver 绑定到本次 client；请求仍保留原始
/// hostname，以便 TLS SNI 和证书校验继续使用站点域名。
pub(crate) async fn resolve_and_verify_host(host: &str, port: u16) -> Result<IpAddr, String> {
    // 处理形如 `[::1]:443` 的 v6 字面量
    let host_port = format!("{host}:{port}");
    let addrs: Vec<_> = lookup_host(&host_port)
        .await
        .map_err(|e| format!("dns resolution failed for '{host}': {e}"))?
        .collect();
    if addrs.is_empty() {
        return Err(format!("no address records for '{host}'"));
    }
    for sa in &addrs {
        if is_blocked_ip(sa.ip()) {
            return Err(format!(
                "host '{host}' resolves to a blocked address: {}",
                sa.ip()
            ));
        }
    }
    Ok(addrs[0].ip())
}

/// 从 URL 下载主题包 JSON 字节。
///
/// 完整流程：形态校验 → DNS 解析 + IP 段校验 → 单请求下载 + 大小限制 + Content-Type 校验。
/// 返回原始字节（未 sanitize），交给 `install_from_bytes` 走后续 pipeline。
pub(crate) async fn download_theme_package(raw_url: &str) -> Result<Vec<u8>, String> {
    let url = validate_url_shape(raw_url)?;
    let host = url
        .host_str()
        .ok_or_else(|| "url missing host".to_string())?
        .to_string();
    let port = url.port_or_known_default().unwrap_or(443);
    // 先做 SSRF 校验
    let resolved_ip = resolve_and_verify_host(&host, port).await?;

    let client = reqwest::Client::builder()
        .connect_timeout(CONNECT_TIMEOUT)
        .timeout(TOTAL_TIMEOUT)
        .redirect(reqwest::redirect::Policy::none())
        .https_only(true)
        // Pin the already-validated DNS answer. This keeps the URL hostname
        // for Host/SNI while preventing reqwest from resolving it again after
        // the SSRF check (DNS rebinding TOCTOU).
        .resolve(&host, SocketAddr::new(resolved_ip, port))
        .user_agent(concat!("Harubble/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|e| format!("failed to build http client: {e}"))?;

    let mut response = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("failed to fetch theme package: {e}"))?;

    if !response.status().is_success() {
        return Err(format!(
            "theme package fetch failed with status: {}",
            response.status()
        ));
    }
    // Content-Length 优先拦截（省流）
    if let Some(len) = response.content_length() {
        if len > MAX_PACKAGE_JSON_BYTES as u64 {
            return Err(format!(
                "theme package too large: {len} > {MAX_PACKAGE_JSON_BYTES} bytes"
            ));
        }
    }
    // Content-Type 校验（缺失也拒绝）
    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_ascii_lowercase();
    let ct_ok = ALLOWED_CONTENT_TYPES
        .iter()
        .any(|allowed| content_type.starts_with(allowed));
    if !ct_ok {
        return Err(format!(
            "unsupported content-type: '{content_type}' (expected application/json)"
        ));
    }

    // Consume one chunk at a time. `Response::bytes()` buffers the complete
    // body before returning, so checking its length afterwards would still
    // allow an unbounded chunked response to exhaust memory.
    let mut body = Vec::with_capacity(
        response
            .content_length()
            .unwrap_or(0)
            .min(MAX_PACKAGE_JSON_BYTES as u64) as usize,
    );
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|e| format!("failed to read theme package body: {e}"))?
    {
        if body.len().saturating_add(chunk.len()) > MAX_PACKAGE_JSON_BYTES {
            return Err(format!(
                "theme package too large after read: > {MAX_PACKAGE_JSON_BYTES} bytes"
            ));
        }
        body.extend_from_slice(&chunk);
    }
    Ok(body)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ipv4_private_ranges_are_blocked() {
        // RFC 1918 三段
        assert!(is_blocked_ip(IpAddr::V4("10.0.0.1".parse().unwrap())));
        assert!(is_blocked_ip(IpAddr::V4("10.255.255.254".parse().unwrap())));
        assert!(is_blocked_ip(IpAddr::V4("172.16.0.1".parse().unwrap())));
        assert!(is_blocked_ip(IpAddr::V4("172.31.255.254".parse().unwrap())));
        assert!(is_blocked_ip(IpAddr::V4("192.168.0.1".parse().unwrap())));
        assert!(is_blocked_ip(IpAddr::V4(
            "192.168.255.254".parse().unwrap()
        )));
    }

    #[test]
    fn ipv4_loopback_and_linklocal_blocked() {
        assert!(is_blocked_ip(IpAddr::V4("127.0.0.1".parse().unwrap())));
        assert!(is_blocked_ip(IpAddr::V4(
            "169.254.169.254".parse().unwrap()
        )));
    }

    #[test]
    fn ipv4_cgnat_and_multicast_blocked() {
        assert!(is_blocked_ip(IpAddr::V4("100.64.0.1".parse().unwrap())));
        assert!(is_blocked_ip(IpAddr::V4(
            "100.127.255.254".parse().unwrap()
        )));
        assert!(is_blocked_ip(IpAddr::V4("224.0.0.1".parse().unwrap())));
        assert!(is_blocked_ip(IpAddr::V4(
            "239.255.255.255".parse().unwrap()
        )));
    }

    #[test]
    fn ipv4_public_addresses_allowed() {
        assert!(!is_blocked_ip(IpAddr::V4("1.1.1.1".parse().unwrap())));
        assert!(!is_blocked_ip(IpAddr::V4("8.8.8.8".parse().unwrap())));
        assert!(!is_blocked_ip(IpAddr::V4("140.82.114.3".parse().unwrap())));
    }

    #[test]
    fn ipv6_loopback_and_multicast_blocked() {
        assert!(is_blocked_ip(IpAddr::V6("::1".parse().unwrap())));
        assert!(is_blocked_ip(IpAddr::V6("::".parse().unwrap())));
        assert!(is_blocked_ip(IpAddr::V6("ff02::1".parse().unwrap())));
        assert!(is_blocked_ip(IpAddr::V6("fe80::1".parse().unwrap())));
        assert!(is_blocked_ip(IpAddr::V6("fd00::1".parse().unwrap())));
    }

    #[test]
    fn ipv6_public_addresses_allowed() {
        assert!(!is_blocked_ip(IpAddr::V6(
            "2606:4700:4700::1111".parse().unwrap()
        )));
        assert!(!is_blocked_ip(IpAddr::V6(
            "2001:4860:4860::8888".parse().unwrap()
        )));
    }

    #[test]
    fn ipv4_mapped_ipv6_delegates_to_ipv4_rules() {
        // IPv4-mapped IPv6 形如 ::ffff:127.0.0.1 应转到 ipv4 规则
        assert!(is_blocked_ip(IpAddr::V6(
            "::ffff:127.0.0.1".parse().unwrap()
        )));
        assert!(is_blocked_ip(IpAddr::V6(
            "::ffff:10.0.0.1".parse().unwrap()
        )));
    }

    #[test]
    fn validate_url_shape_accepts_https_default_port() {
        assert!(validate_url_shape("https://example.com/theme.json").is_ok());
        assert!(validate_url_shape("https://example.com:443/theme.json").is_ok());
    }

    #[test]
    fn validate_url_shape_rejects_non_https_schemes() {
        assert!(validate_url_shape("http://example.com/theme.json").is_err());
        assert!(validate_url_shape("ftp://example.com/theme.json").is_err());
        assert!(validate_url_shape("file:///etc/passwd").is_err());
        assert!(validate_url_shape("data:text/html,<script>").is_err());
        assert!(validate_url_shape("javascript:alert(1)").is_err());
    }

    #[test]
    fn validate_url_shape_rejects_non_standard_port() {
        assert!(validate_url_shape("https://example.com:8080/theme.json").is_err());
        assert!(validate_url_shape("https://example.com:22/theme.json").is_err());
    }

    #[test]
    fn validate_url_shape_rejects_credentials_in_url() {
        assert!(validate_url_shape("https://user:pass@example.com/theme.json").is_err());
        assert!(validate_url_shape("https://user@example.com/theme.json").is_err());
    }

    #[test]
    fn validate_url_shape_rejects_malformed_urls() {
        assert!(validate_url_shape("").is_err());
        assert!(validate_url_shape("not-a-url").is_err());
        assert!(validate_url_shape("https://").is_err());
    }
}

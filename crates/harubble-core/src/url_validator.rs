//! 下载 URL 域名白名单校验。
//!
//! 该模块提供对外部资源 URL 的安全校验，确保只有来自已知可信域名的 HTTPS 资源
//! 才能被下载，防止 SSRF 攻击。

use anyhow::{bail, Result};
use url::Url;

/// 精确匹配的可信域名。
const ALLOWED_HOSTS: &[&str] = &[
    "monster-siren.hypergryph.com",
    "monster-siren-cdn.hypergryph.com",
];

/// 后缀匹配的可信域名（覆盖其所有子域）。
const ALLOWED_SUFFIXES: &[&str] = &[".hycdn.cn", ".hypergryph.com"];

fn is_host_allowed(host: &str) -> bool {
    ALLOWED_HOSTS.contains(&host)
        || ALLOWED_SUFFIXES
            .iter()
            .any(|&suffix| host.ends_with(suffix))
}

/// 校验下载 URL 是否属于可信域名白名单。
///
/// 仅允许 `https` scheme 且域名在白名单或可信后缀内的 URL 通过校验。
/// 校验失败时返回描述性错误信息。
pub fn validate_download_url(url: &str) -> Result<()> {
    let parsed = Url::parse(url)?;

    if parsed.scheme() != "https" {
        bail!("only https URLs are allowed, got: {}", parsed.scheme());
    }

    let host = parsed
        .host_str()
        .ok_or_else(|| anyhow::anyhow!("URL has no host: {url}"))?;

    if !is_host_allowed(host) {
        bail!("host '{host}' is not in the allowed download domain list");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_valid_monster_siren_url() {
        let url = "https://monster-siren.hypergryph.com/api/album/123/detail";
        assert!(validate_download_url(url).is_ok());
    }

    #[test]
    fn accepts_valid_cdn_url() {
        let url = "https://web.hycdn.cn/audio/song.flac";
        assert!(validate_download_url(url).is_ok());
    }

    #[test]
    fn accepts_any_hycdn_subdomain() {
        for sub in ["res01", "res02", "static", "cdn-xx"] {
            let url = format!("https://{sub}.hycdn.cn/audio/song.flac");
            assert!(
                validate_download_url(&url).is_ok(),
                "should accept {sub}.hycdn.cn"
            );
        }
    }

    #[test]
    fn accepts_any_hypergryph_subdomain() {
        let url = "https://ak-cdn.hypergryph.com/assets/cover.png";
        assert!(validate_download_url(url).is_ok());
    }

    #[test]
    fn rejects_http_scheme() {
        let url = "http://monster-siren.hypergryph.com/api/album/123";
        assert!(validate_download_url(url).is_err());
    }

    #[test]
    fn rejects_unknown_host() {
        let url = "https://evil.example.com/malicious";
        assert!(validate_download_url(url).is_err());
    }

    #[test]
    fn rejects_suffix_trick() {
        let url = "https://evil-hycdn.cn/fake";
        assert!(validate_download_url(url).is_err());
    }

    #[test]
    fn rejects_file_scheme() {
        let url = "file:///etc/passwd";
        assert!(validate_download_url(url).is_err());
    }
}

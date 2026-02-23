use std::time::Duration;

use reqwest::{Client, NoProxy, Proxy};
use serde::de::DeserializeOwned;

use crate::config::AppConfig;
use crate::error::{AppError, Result};
use crate::utils::proxy::{build_proxy_url, DEFAULT_NO_PROXY_VALUE};

pub(crate) const USER_AGENT: &str = concat!(
    env!("CARGO_PKG_NAME"),
    "/",
    env!("CARGO_PKG_VERSION"),
    " (+https://github.com/AstrBotDevs/astrbot-launcher)"
);

pub(crate) fn build_http_client_with_proxy_fields(
    url: &str,
    port: &str,
    username: &str,
    password: &str,
) -> Result<Client> {
    let mut builder = Client::builder().timeout(Duration::from_secs(30));

    if let Some(proxy_url) = build_proxy_url(url, port, username, password)? {
        let proxy = Proxy::all(proxy_url)
            .map_err(|e| AppError::config(format!("代理地址无效: {}", e)))?
            .no_proxy(NoProxy::from_string(DEFAULT_NO_PROXY_VALUE));
        builder = builder.proxy(proxy);
    }

    builder
        .build()
        .map_err(|e| AppError::network(format!("创建网络客户端失败: {}", e)))
}

pub(crate) fn build_http_client(config: &AppConfig) -> Result<Client> {
    build_http_client_with_proxy_fields(
        &config.proxy_url,
        &config.proxy_port,
        &config.proxy_username,
        &config.proxy_password,
    )
}

pub(crate) fn build_get_request<'a>(client: &'a Client, url: &'a str) -> reqwest::RequestBuilder {
    client.get(url).header("User-Agent", USER_AGENT)
}

pub(crate) async fn send_get(
    client: &Client,
    url: &str,
    accept_json: bool,
) -> std::result::Result<reqwest::Response, reqwest::Error> {
    let request = if accept_json {
        build_get_request(client, url).header("Accept", "application/json")
    } else {
        build_get_request(client, url)
    };
    request.send().await
}

pub(crate) fn ensure_success_status(
    resp: &reqwest::Response,
    make_error: impl FnOnce(String) -> AppError,
) -> Result<()> {
    if resp.status().is_success() {
        Ok(())
    } else {
        Err(make_error(resp.status().to_string()))
    }
}

/// Fetch JSON from `url` and deserialize into `T`.
pub(crate) async fn fetch_json<T: DeserializeOwned>(client: &Client, url: &str) -> Result<T> {
    let resp = send_get(client, url, true)
        .await
        .map_err(|e| AppError::network(e.to_string()))?;
    ensure_success_status(&resp, AppError::network)?;

    resp.json::<T>()
        .await
        .map_err(|e| AppError::network(format!("Failed to parse response: {}", e)))
}

/// Check whether `url` is reachable (HTTP GET returns a success status).
pub(crate) async fn check_url(client: &Client, url: &str) -> Result<()> {
    let resp = send_get(client, url, false)
        .await
        .map_err(|e| AppError::network_with_url(url, e.to_string()))?;
    ensure_success_status(&resp, |detail| AppError::network_with_url(url, detail))?;

    Ok(())
}

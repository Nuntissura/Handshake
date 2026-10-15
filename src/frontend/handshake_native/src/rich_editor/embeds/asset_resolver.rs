//! Native authenticated asset transport and REST routes.

pub use handshake_asset_resolver::*;
use std::sync::Arc;
use std::time::Duration;

/// `GET /workspaces/{ws}/assets/{id}` — the asset metadata endpoint (verified backend pattern
/// `api/loom.rs get_asset_metadata`). Path components are NOT percent-encoded here because the
/// validated asset id is already constrained to `[A-Za-z0-9._-]` (no path/percent chars), and
/// the workspace id is a backend-owned opaque id; the React code encodes defensively, but the
/// native side has already fail-closed-rejected any id that would need encoding.
pub fn asset_metadata_url(base_url: &str, workspace_id: &str, asset_id: &str) -> String {
    format!("{base_url}/workspaces/{workspace_id}/assets/{asset_id}")
}

/// `GET /workspaces/{ws}/assets/{id}/content` — full-res content bytes.
pub fn asset_content_url(base_url: &str, workspace_id: &str, asset_id: &str) -> String {
    format!(
        "{}/content",
        asset_metadata_url(base_url, workspace_id, asset_id)
    )
}

/// `GET /workspaces/{ws}/assets/{id}/thumbnail` — thumbnail bytes.
pub fn asset_thumbnail_url(base_url: &str, workspace_id: &str, asset_id: &str) -> String {
    format!(
        "{}/thumbnail",
        asset_metadata_url(base_url, workspace_id, asset_id)
    )
}

/// Tier-aware content URL used by the current React parity surface.
pub fn asset_tier_url(
    base_url: &str,
    workspace_id: &str,
    asset_id: &str,
    tier: MediaTier,
) -> String {
    if tier == MediaTier::Full {
        asset_content_url(base_url, workspace_id, asset_id)
    } else {
        format!(
            "{}?tier={}",
            asset_content_url(base_url, workspace_id, asset_id),
            tier.as_str()
        )
    }
}

pub fn collection_url(base_url: &str, workspace_id: &str, collection_id: &str) -> String {
    format!("{base_url}/workspaces/{workspace_id}/loom/collections/{collection_id}")
}

/// The production [`AssetMetadataFetcher`]: a thin wrapper over a `reqwest::Client` that GETs
/// the verified backend metadata endpoint and maps the HTTP status to the typed
/// [`EmbedError`] vocabulary (mirrors the React `resolveEmbedAsset` status handling). It REUSES
/// the existing `handshake_native::backend_client` REST stack (reqwest 0.12, rustls) — NO new
/// HTTP crate is introduced (MT scope). Backend access is read-only GET; no backend code is
/// touched (consume-via-API-only).
#[derive(Clone)]
pub struct ReqwestAssetFetcher {
    client: Option<reqwest::Client>,
    client_init_error: Option<Arc<str>>,
    base_url: String,
    request_timeout: Duration,
    /// MT-154: `/workspaces/:ws/assets/*` and `/workspaces/:ws/loom/collections/*` run as the account
    /// record user. `None` fails every request with "Account login required" before any socket.
    authenticated_context: Option<Arc<crate::local_account::AuthenticatedContext>>,
}

/// Send one embed GET through the canonical authenticated transport, bounded by `request_timeout`.
/// A missing account is a typed [`EmbedError::Forbidden`] raised before any socket opens; a transport
/// timeout keeps its typed [`EmbedError::TimedOut`] mapping.
async fn send_authenticated(
    client: &reqwest::Client,
    account: Option<Arc<crate::local_account::AuthenticatedContext>>,
    url: &str,
    request_timeout: Duration,
    label: &str,
) -> Result<reqwest::Response, EmbedError> {
    let context = account.ok_or_else(|| {
        EmbedError::Forbidden(format!("{label} is not accessible: Account login required"))
    })?;
    crate::local_account::AuthenticatedRequest::new(
        client.clone(),
        Some(context),
        client.get(url).timeout(request_timeout),
    )
    .send()
    .await
    .map_err(|error| {
        if error.contains("kind=timeout") {
            EmbedError::TimedOut(format!("{label} exceeded its transport deadline: {error}"))
        } else {
            EmbedError::NetworkError(format!("{label} failed: {error}"))
        }
    })
}

impl ReqwestAssetFetcher {
    /// Build a fetcher against `base_url` (e.g. `backend_client::BACKEND_BASE_URL`).
    pub fn new(base_url: impl Into<String>) -> Self {
        Self::with_timeouts(
            base_url,
            EMBED_HTTP_CONNECT_TIMEOUT,
            EMBED_OPERATION_TIMEOUT,
        )
    }

    fn with_timeouts(
        base_url: impl Into<String>,
        connect_timeout: Duration,
        request_timeout: Duration,
    ) -> Self {
        let base_url = base_url.into();
        let client = reqwest::Client::builder()
            .connect_timeout(connect_timeout)
            .timeout(request_timeout)
            .build();
        match client {
            Ok(client) => Self {
                client: Some(client),
                client_init_error: None,
                base_url,
                request_timeout,
                authenticated_context: None,
            },
            Err(error) => Self {
                client: None,
                client_init_error: Some(Arc::from(error.to_string())),
                base_url,
                request_timeout,
                authenticated_context: None,
            },
        }
    }

    /// MT-154: bind the immutable account context every embed request carries.
    pub fn with_authenticated_context(
        mut self,
        context: Option<Arc<crate::local_account::AuthenticatedContext>>,
    ) -> Self {
        self.authenticated_context = context;
        self
    }

    /// The production fetcher against the hardcoded backend base URL.
    pub fn production() -> Self {
        Self::new(crate::backend_client::BACKEND_BASE_URL)
    }

    /// The REST base this fetcher resolves content/thumbnail URLs against (so the resolver and
    /// the fetcher agree on the base).
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    fn cloned_client(&self) -> Result<reqwest::Client, EmbedError> {
        self.client.clone().ok_or_else(|| {
            EmbedError::NetworkError(format!(
                "bounded embed HTTP client could not initialize: {}",
                self.client_init_error.as_deref().unwrap_or("unknown error")
            ))
        })
    }
}

fn map_reqwest_error(label: &str, error: reqwest::Error) -> EmbedError {
    if error.is_timeout() {
        EmbedError::TimedOut(format!("{label} exceeded its transport deadline: {error}"))
    } else {
        EmbedError::NetworkError(format!("{label} failed: {error}"))
    }
}

async fn read_bounded_response(
    mut response: reqwest::Response,
    max_bytes: usize,
    label: &str,
) -> Result<Vec<u8>, EmbedError> {
    if let Some(length) = response.content_length() {
        if length > max_bytes as u64 {
            return Err(EmbedError::ResourceLimit(format!(
                "{label} content-length {length} exceeds {max_bytes} bytes"
            )));
        }
    }
    let mut body = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|error| map_reqwest_error(&format!("{label} body read"), error))?
    {
        if body.len().saturating_add(chunk.len()) > max_bytes {
            return Err(EmbedError::ResourceLimit(format!(
                "{label} streamed body exceeds {max_bytes} bytes"
            )));
        }
        body.extend_from_slice(&chunk);
    }
    Ok(body)
}

fn map_response_status(status: reqwest::StatusCode, label: &str) -> Result<(), EmbedError> {
    match status.as_u16() {
        404 => Err(EmbedError::NotFound(format!("{label} not found"))),
        401 | 403 => Err(EmbedError::Forbidden(format!(
            "{label} is not accessible (HTTP {status})"
        ))),
        _ if !status.is_success() => Err(EmbedError::ServerError(format!(
            "{label} request returned HTTP {status}"
        ))),
        _ => Ok(()),
    }
}

fn tier_body_limit(tier: MediaTier, used_original_fallback: bool) -> usize {
    if used_original_fallback {
        MAX_FULL_IMAGE_BYTES
    } else {
        tier.max_body_bytes()
    }
}

impl AssetMetadataFetcher for ReqwestAssetFetcher {
    fn asset_urls(&self, base_url: &str, workspace_id: &str, asset_id: &str) -> ResolvedAssetUrls {
        ResolvedAssetUrls {
            content_url: asset_content_url(base_url, workspace_id, asset_id),
            thumbnail_url: asset_tier_url(base_url, workspace_id, asset_id, MediaTier::Thumbnail),
            preview_url: asset_tier_url(base_url, workspace_id, asset_id, MediaTier::Preview),
            poster_url: asset_tier_url(base_url, workspace_id, asset_id, MediaTier::Poster),
        }
    }


    fn fetch_metadata<'a>(
        &'a self,
        workspace_id: &'a str,
        asset_id: &'a str,
    ) -> MetadataFuture<'a> {
        let url = asset_metadata_url(&self.base_url, workspace_id, asset_id);
        let client = self.cloned_client();
        let account = self.authenticated_context.clone();
        let asset_id = asset_id.to_owned();
        let request_timeout = self.request_timeout;
        Box::pin(async move {
            run_with_deadline(
                request_timeout,
                format!("asset '{asset_id}' metadata request"),
                async move {
                    let client = client?;
                    let response = send_authenticated(
                        &client,
                        account,
                        &url,
                        request_timeout,
                        "asset metadata request",
                    )
                    .await?;
                    let status = response.status();
                    map_response_status(status, &format!("asset '{asset_id}' metadata"))?;
                    let body =
                        read_bounded_response(response, MAX_METADATA_BYTES, "asset metadata")
                            .await?;
                    serde_json::from_slice(&body).map_err(|error| {
                        EmbedError::ServerError(format!("asset metadata body is invalid: {error}"))
                    })
                },
            )
            .await
        })
    }

    fn fetch_content<'a>(&'a self, workspace_id: &'a str, asset_id: &'a str) -> ContentFuture<'a> {
        let url = asset_content_url(&self.base_url, workspace_id, asset_id);
        let client = self.cloned_client();
        let account = self.authenticated_context.clone();
        let asset_id = asset_id.to_owned();
        let request_timeout = self.request_timeout;
        Box::pin(async move {
            run_with_deadline(
                request_timeout,
                format!("asset '{asset_id}' content request"),
                async move {
                    let client = client?;
                    let response = send_authenticated(
                        &client,
                        account,
                        &url,
                        request_timeout,
                        "asset content request",
                    )
                    .await?;
                    let status = response.status();
                    map_response_status(status, &format!("asset content '{asset_id}'"))?;
                    read_bounded_response(response, MAX_FULL_IMAGE_BYTES, "asset content").await
                },
            )
            .await
        })
    }

    fn fetch_tier<'a>(
        &'a self,
        workspace_id: &'a str,
        asset_id: &'a str,
        kind: MediaEmbedKind,
        tier: MediaTier,
    ) -> ContentFuture<'a> {
        let url = asset_tier_url(&self.base_url, workspace_id, asset_id, tier);
        let original_url = asset_content_url(&self.base_url, workspace_id, asset_id);
        let client = self.cloned_client();
        let account = self.authenticated_context.clone();
        let asset_id = asset_id.to_owned();
        let request_timeout = self.request_timeout;
        Box::pin(async move {
            run_with_deadline(
                request_timeout,
                format!("{} tier request for asset '{asset_id}'", tier.as_str()),
                async move {
                    let client = client?;
                    let mut response = send_authenticated(
                        &client,
                        account.clone(),
                        &url,
                        request_timeout,
                        &format!("{} request", tier.as_str()),
                    )
                    .await?;

                    // React parity only falls a single-image thumbnail back to its original. Album and
                    // slideshow grids keep a missing thumbnail typed/visible, and a missing video poster
                    // must never trigger a full video-body download.
                    let used_original_fallback = kind == MediaEmbedKind::Images
                        && tier == MediaTier::Thumbnail
                        && response.status() == reqwest::StatusCode::NOT_FOUND;
                    if used_original_fallback {
                        response = send_authenticated(
                            &client,
                            account,
                            &original_url,
                            request_timeout,
                            &format!("{} fallback content request", tier.as_str()),
                        )
                        .await?;
                    }
                    map_response_status(
                        response.status(),
                        &format!("{} tier for asset '{asset_id}'", tier.as_str()),
                    )?;
                    read_bounded_response(
                        response,
                        tier_body_limit(tier, used_original_fallback),
                        tier.as_str(),
                    )
                    .await
                },
            )
            .await
        })
    }

    fn fetch_collection<'a>(
        &'a self,
        workspace_id: &'a str,
        collection_id: &'a str,
    ) -> CollectionFuture<'a> {
        let url = collection_url(&self.base_url, workspace_id, collection_id);
        let client = self.cloned_client();
        let account = self.authenticated_context.clone();
        let collection_id = collection_id.to_owned();
        let request_timeout = self.request_timeout;
        Box::pin(async move {
            run_with_deadline(
                request_timeout,
                format!("collection '{collection_id}' request"),
                async move {
                    let client = client?;
                    let response = send_authenticated(
                        &client,
                        account,
                        &url,
                        request_timeout,
                        "collection request",
                    )
                    .await?;
                    map_response_status(
                        response.status(),
                        &format!("collection '{collection_id}'"),
                    )?;
                    let body =
                        read_bounded_response(response, MAX_METADATA_BYTES, "collection").await?;
                    serde_json::from_slice(&body).map_err(|error| {
                        EmbedError::ServerError(format!("collection body is invalid: {error}"))
                    })
                },
            )
            .await
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;

    fn scripted_http_server(
        responses: Vec<(&'static str, u16, Vec<u8>)>,
    ) -> (String, std::thread::JoinHandle<Vec<String>>) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind scripted HTTP server");
        let address = listener.local_addr().expect("scripted server address");
        listener
            .set_nonblocking(true)
            .expect("bound scripted accept");
        let handle = std::thread::spawn(move || {
            let mut observed = Vec::new();
            for (expected_path, status, body) in responses {
                let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
                let (mut stream, _) = loop {
                    match listener.accept() {
                        Ok(connection) => break connection,
                        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                            assert!(
                                std::time::Instant::now() < deadline,
                                "timed out waiting for scripted request {expected_path}"
                            );
                            std::thread::sleep(std::time::Duration::from_millis(5));
                        }
                        Err(error) => panic!("accept scripted request failed: {error}"),
                    }
                };
                stream
                    .set_nonblocking(false)
                    .expect("scripted request stream must use bounded blocking reads");
                stream
                    .set_read_timeout(Some(std::time::Duration::from_secs(3)))
                    .expect("bound scripted request read");
                let mut request = [0_u8; 4096];
                let read = stream.read(&mut request).expect("read scripted request");
                let request = String::from_utf8_lossy(&request[..read]);
                let path = request
                    .lines()
                    .next()
                    .and_then(|line| line.split_whitespace().nth(1))
                    .expect("HTTP request path")
                    .to_owned();
                assert_eq!(path, expected_path);
                observed.push(path);
                let reason = match status {
                    200 => "OK",
                    404 => "Not Found",
                    500 => "Internal Server Error",
                    _ => "Test Status",
                };
                write!(
                    stream,
                    "HTTP/1.1 {status} {reason}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                )
                .expect("write scripted response headers");
                stream
                    .write_all(&body)
                    .expect("write scripted response body");
            }
            observed
        });
        (format!("http://{address}"), handle)
    }

    fn raw_http_server(
        response: Vec<u8>,
        delay_before_response: Duration,
    ) -> (String, std::thread::JoinHandle<String>) {
        raw_http_server_parts(Vec::new(), delay_before_response, response)
    }

    fn raw_http_server_parts(
        response_prefix: Vec<u8>,
        delay_before_suffix: Duration,
        response_suffix: Vec<u8>,
    ) -> (String, std::thread::JoinHandle<String>) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind raw HTTP server");
        let address = listener.local_addr().expect("raw server address");
        listener.set_nonblocking(true).expect("bound raw accept");
        let handle = std::thread::spawn(move || {
            let deadline = std::time::Instant::now() + Duration::from_secs(3);
            let (mut stream, _) = loop {
                match listener.accept() {
                    Ok(connection) => break connection,
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        assert!(
                            std::time::Instant::now() < deadline,
                            "timed out waiting for raw HTTP request"
                        );
                        std::thread::sleep(Duration::from_millis(5));
                    }
                    Err(error) => panic!("accept raw request failed: {error}"),
                }
            };
            stream
                .set_read_timeout(Some(Duration::from_secs(3)))
                .expect("bound raw request read");
            stream
                .set_write_timeout(Some(Duration::from_secs(3)))
                .expect("bound raw response write");
            let mut request = [0_u8; 4096];
            let read = stream.read(&mut request).expect("read raw request");
            let request = String::from_utf8_lossy(&request[..read]);
            let path = request
                .lines()
                .next()
                .and_then(|line| line.split_whitespace().nth(1))
                .expect("raw HTTP request path")
                .to_owned();
            // A timeout or an oversized Content-Length may make the client close before the
            // test server writes. That is the expected negative path, so BrokenPipe is not a
            // server-test failure.
            let _ = stream.write_all(&response_prefix);
            let _ = stream.flush();
            if !delay_before_suffix.is_zero() {
                std::thread::sleep(delay_before_suffix);
            }
            let _ = stream.write_all(&response_suffix);
            path
        });
        (format!("http://{address}"), handle)
    }

    fn metadata_body(workspace_id: &str, asset_id: &str, mime: &str) -> Vec<u8> {
        serde_json::to_vec(&EmbedAssetMetadata {
            asset_id: asset_id.to_owned(),
            workspace_id: workspace_id.to_owned(),
            kind: "image".to_owned(),
            mime: mime.to_owned(),
            original_filename: Some(format!("{asset_id}.png")),
            content_hash: "test-hash".to_owned(),
            size_bytes: 16,
            width: Some(1),
            height: Some(1),
        })
        .expect("serialize metadata fixture")
    }

    #[test]
    fn urls_match_backend_pattern() {
        let base = "http://127.0.0.1:37501";
        let urls = ReqwestAssetFetcher::new(base).asset_urls(base, "ws1", "a1");
        assert_eq!(urls.content_url, asset_content_url(base, "ws1", "a1"));
        assert_eq!(
            urls.thumbnail_url,
            asset_tier_url(base, "ws1", "a1", MediaTier::Thumbnail)
        );
        assert_eq!(
            urls.preview_url,
            asset_tier_url(base, "ws1", "a1", MediaTier::Preview)
        );
        assert_eq!(
            urls.poster_url,
            asset_tier_url(base, "ws1", "a1", MediaTier::Poster)
        );
        assert_eq!(
            asset_metadata_url(base, "ws1", "a1"),
            "http://127.0.0.1:37501/workspaces/ws1/assets/a1"
        );
        assert_eq!(
            asset_content_url(base, "ws1", "a1"),
            "http://127.0.0.1:37501/workspaces/ws1/assets/a1/content"
        );
        assert_eq!(
            asset_thumbnail_url(base, "ws1", "a1"),
            "http://127.0.0.1:37501/workspaces/ws1/assets/a1/thumbnail"
        );
    }

    #[tokio::test]
    async fn single_image_thumbnail_404_falls_back_to_bounded_original() {
        let tier_path = "/workspaces/ws/assets/img/content?tier=thumb";
        let original_path = "/workspaces/ws/assets/img/content";
        let fallback_len = MAX_THUMBNAIL_BYTES + 1;
        let (base_url, server) = scripted_http_server(vec![
            (tier_path, 404, Vec::new()),
            (original_path, 200, vec![0x5a; fallback_len]),
        ]);
        let fetcher = ReqwestAssetFetcher::new(base_url.clone()).with_authenticated_context(Some(
            crate::local_account::mock_account_context(&base_url),
        ));
        let bytes = fetcher
            .fetch_tier("ws", "img", MediaEmbedKind::Images, MediaTier::Thumbnail)
            .await
            .expect("single-image thumbnail fallback succeeds");
        assert_eq!(
            bytes.len(),
            fallback_len,
            "fallback accepts an original larger than the thumbnail limit but within the full-image limit"
        );
        assert!(bytes.iter().all(|byte| *byte == 0x5a));
        assert_eq!(
            server.join().expect("scripted server completed"),
            vec![tier_path.to_owned(), original_path.to_owned()]
        );
        assert_eq!(
            tier_body_limit(MediaTier::Thumbnail, true),
            MAX_FULL_IMAGE_BYTES,
            "a selected original fallback must use the full-image byte bound"
        );
        assert_eq!(
            tier_body_limit(MediaTier::Thumbnail, false),
            MAX_THUMBNAIL_BYTES,
            "a real thumbnail response keeps the tighter thumbnail bound"
        );
    }

    #[tokio::test]
    async fn video_poster_404_never_fetches_full_video_body() {
        let poster_path = "/workspaces/ws/assets/video/content?tier=poster";
        let (base_url, server) = scripted_http_server(vec![(poster_path, 404, Vec::new())]);
        let fetcher = ReqwestAssetFetcher::new(base_url.clone()).with_authenticated_context(Some(
            crate::local_account::mock_account_context(&base_url),
        ));
        let error = fetcher
            .fetch_tier("ws", "video", MediaEmbedKind::Video, MediaTier::Poster)
            .await
            .expect_err("missing poster remains a typed error");
        assert_eq!(error.kind_str(), "not_found");
        assert_eq!(
            server.join().expect("scripted server completed"),
            vec![poster_path.to_owned()],
            "no request may reach the canonical full video content route"
        );
    }

    #[tokio::test]
    async fn album_thumbnail_404_does_not_expand_into_original_body_fanout() {
        let thumb_path = "/workspaces/ws/assets/cell/content?tier=thumb";
        let (base_url, server) = scripted_http_server(vec![(thumb_path, 404, Vec::new())]);
        let fetcher = ReqwestAssetFetcher::new(base_url.clone()).with_authenticated_context(Some(
            crate::local_account::mock_account_context(&base_url),
        ));
        let error = fetcher
            .fetch_tier("ws", "cell", MediaEmbedKind::Album, MediaTier::Thumbnail)
            .await
            .expect_err("missing album thumbnail remains typed/visible");
        assert_eq!(error.kind_str(), "not_found");
        assert_eq!(
            server.join().expect("scripted server completed"),
            vec![thumb_path.to_owned()]
        );
    }

    #[tokio::test]
    async fn derived_tier_server_error_never_falls_back_to_original() {
        let thumb_path = "/workspaces/ws/assets/img/content?tier=thumb";
        let (base_url, server) = scripted_http_server(vec![(thumb_path, 500, Vec::new())]);
        let fetcher = ReqwestAssetFetcher::new(base_url.clone()).with_authenticated_context(Some(
            crate::local_account::mock_account_context(&base_url),
        ));
        let error = fetcher
            .fetch_tier("ws", "img", MediaEmbedKind::Images, MediaTier::Thumbnail)
            .await
            .expect_err("only a 404 may enter the single-image fallback path");
        assert_eq!(error.kind_str(), "server_error");
        assert_eq!(
            server.join().expect("scripted server completed"),
            vec![thumb_path.to_owned()]
        );
    }

    #[tokio::test]
    async fn malformed_and_missing_required_metadata_are_typed_server_errors() {
        let path = "/workspaces/ws/assets/img";
        for body in [
            br#"{"asset_id":"img","workspace_id":"ws","mime": }"#.to_vec(),
            br#"{"asset_id":"img","workspace_id":"ws"}"#.to_vec(),
        ] {
            let (base_url, server) = scripted_http_server(vec![(path, 200, body)]);
            let fetcher = ReqwestAssetFetcher::new(base_url.clone()).with_authenticated_context(
                Some(crate::local_account::mock_account_context(&base_url)),
            );
            let error = fetcher
                .fetch_metadata("ws", "img")
                .await
                .expect_err("invalid metadata must fail closed");
            assert_eq!(error.kind_str(), "server_error");
            assert_eq!(
                server.join().expect("scripted server completed"),
                vec![path.to_owned()]
            );
        }
    }

    #[tokio::test]
    async fn metadata_identity_workspace_and_mime_bindings_fail_closed() {
        let cases = [
            ("other", "ws", "image/png", "id"),
            ("img", "other-workspace", "image/png", "workspace"),
            ("img", "ws", "   ", "mime"),
        ];
        for (returned_id, returned_workspace, mime, expected_message) in cases {
            let path = "/workspaces/ws/assets/img";
            let body = metadata_body(returned_workspace, returned_id, mime);
            let (base_url, server) = scripted_http_server(vec![(path, 200, body)]);
            let fetcher = ReqwestAssetFetcher::new(base_url.clone()).with_authenticated_context(
                Some(crate::local_account::mock_account_context(&base_url)),
            );
            let error = resolve_one(MediaEmbedKind::Images, "ws", "img", &base_url, &fetcher)
                .await
                .expect_err("unbound metadata must fail closed");
            assert_eq!(error.kind_str(), "server_error");
            assert!(error.to_string().contains(expected_message));
            assert_eq!(
                server.join().expect("scripted server completed"),
                vec![path.to_owned()]
            );
        }
    }

    #[tokio::test]
    async fn missing_asset_metadata_is_typed_not_found() {
        let path = "/workspaces/ws/assets/missing";
        let (base_url, server) = scripted_http_server(vec![(path, 404, Vec::new())]);
        let fetcher = ReqwestAssetFetcher::new(base_url.clone()).with_authenticated_context(Some(
            crate::local_account::mock_account_context(&base_url),
        ));
        let error = resolve_one(MediaEmbedKind::Images, "ws", "missing", &base_url, &fetcher)
            .await
            .expect_err("missing asset must remain visible and typed");
        assert_eq!(error.kind_str(), "not_found");
        assert_eq!(
            server.join().expect("scripted server completed"),
            vec![path.to_owned()]
        );
    }

    #[tokio::test]
    async fn oversized_declared_content_length_is_rejected_before_body_read() {
        let declared = MAX_METADATA_BYTES + 1;
        let response =
            format!("HTTP/1.1 200 OK\r\nContent-Length: {declared}\r\nConnection: close\r\n\r\n")
                .into_bytes();
        let (base_url, server) = raw_http_server(response, Duration::ZERO);
        let fetcher = ReqwestAssetFetcher::new(base_url.clone()).with_authenticated_context(Some(
            crate::local_account::mock_account_context(&base_url),
        ));
        let error = fetcher
            .fetch_metadata("ws", "oversized")
            .await
            .expect_err("oversized Content-Length must fail before allocation/body read");
        assert_eq!(error.kind_str(), "resource_limit");
        assert!(error.to_string().contains("content-length"));
        assert_eq!(
            server.join().expect("raw server completed"),
            "/workspaces/ws/assets/oversized"
        );
    }

    #[tokio::test]
    async fn oversized_chunked_stream_is_rejected_while_reading() {
        let body = vec![b'x'; MAX_METADATA_BYTES + 1];
        let mut response = format!(
            "HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n{:X}\r\n",
            body.len()
        )
        .into_bytes();
        response.extend_from_slice(&body);
        response.extend_from_slice(b"\r\n0\r\n\r\n");
        let (base_url, server) = raw_http_server(response, Duration::ZERO);
        let fetcher = ReqwestAssetFetcher::new(base_url.clone()).with_authenticated_context(Some(
            crate::local_account::mock_account_context(&base_url),
        ));
        let error = fetcher
            .fetch_metadata("ws", "streamed")
            .await
            .expect_err("streamed body must be bounded even without Content-Length");
        assert_eq!(error.kind_str(), "resource_limit");
        assert!(error.to_string().contains("streamed body"));
        assert_eq!(
            server.join().expect("raw server completed"),
            "/workspaces/ws/assets/streamed"
        );
    }

    #[tokio::test]
    async fn reqwest_request_deadline_bounds_headers_and_body() {
        let response =
            b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{}".to_vec();
        let (base_url, server) = raw_http_server(response, Duration::from_millis(150));
        let fetcher = ReqwestAssetFetcher::with_timeouts(
            base_url.clone(),
            Duration::from_millis(25),
            Duration::from_millis(40),
        )
        .with_authenticated_context(Some(crate::local_account::mock_account_context(&base_url)));
        let started = std::time::Instant::now();
        let error = fetcher
            .fetch_metadata("ws", "slow")
            .await
            .expect_err("slow headers must hit the request deadline");
        assert_eq!(error.kind_str(), "timed_out");
        assert!(
            started.elapsed() < Duration::from_secs(1),
            "the short test deadline must not inherit the production 15-second timeout"
        );
        assert_eq!(
            server.join().expect("raw server completed"),
            "/workspaces/ws/assets/slow"
        );
    }

    #[tokio::test]
    async fn reqwest_request_deadline_bounds_a_stalled_streamed_body() {
        let headers = b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\n".to_vec();
        let (base_url, server) =
            raw_http_server_parts(headers, Duration::from_millis(150), b"{}".to_vec());
        let fetcher = ReqwestAssetFetcher::with_timeouts(
            base_url.clone(),
            Duration::from_millis(25),
            Duration::from_millis(40),
        )
        .with_authenticated_context(Some(crate::local_account::mock_account_context(&base_url)));
        let started = std::time::Instant::now();
        let error = fetcher
            .fetch_metadata("ws", "slow-body")
            .await
            .expect_err("a stalled response body must hit the request deadline");
        assert_eq!(error.kind_str(), "timed_out");
        assert!(started.elapsed() < Duration::from_secs(1));
        assert_eq!(
            server.join().expect("raw server completed"),
            "/workspaces/ws/assets/slow-body"
        );
    }

}

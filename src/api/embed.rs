use axum::{
    Router,
    handler::HandlerWithoutStateExt,
    http::{HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
};
use rust_embed::Embed;
use std::{
    io,
    path::{Path, PathBuf},
    pin::Pin,
    task::{Context, Poll},
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::io::{AsyncRead, AsyncSeek, ReadBuf};
use tower::ServiceBuilder;
use tower_http::{
    services::{
        ServeDir,
        fs::{Backend, File as FsFile, Metadata as FsMetadata},
    },
    set_header::SetResponseHeaderLayer,
};

const INDEX_HTML: &str = "index.html";
/// `build.assetsDir` in vite.config.ts: every file in it has a content hash
/// in its name.
const ASSETS_DIR: &str = "fe";
const IMMUTABLE: HeaderValue = HeaderValue::from_static("public, max-age=31536000, immutable");
/// index.html names the current release's chunks, so it is revalidated on
/// every load.
const NO_CACHE: HeaderValue = HeaderValue::from_static("no-cache");

/// Serves the embedded frontend: build assets under `/fe`, and index.html for
/// any other path the rest of the router does not handle.
pub fn router<S: Clone + Send + Sync + 'static>() -> Router<S> {
    // No index.html fallback under /fe: a chunk this build does not have must
    // 404, not come back as HTML the browser then rejects as a module script.
    let assets = ServiceBuilder::new()
        .layer(SetResponseHeaderLayer::if_not_present(
            header::CACHE_CONTROL,
            immutable_if_found,
        ))
        .service(ServeDir::with_backend(ASSETS_DIR, EmbedBackend));
    Router::new().nest_service("/fe", assets).fallback_service(
        ServeDir::with_backend("", EmbedBackend).fallback(spa_index.into_service()),
    )
}

fn immutable_if_found<B>(response: &Response<B>) -> Option<HeaderValue> {
    matches!(
        response.status(),
        StatusCode::OK | StatusCode::PARTIAL_CONTENT | StatusCode::NOT_MODIFIED
    )
    .then_some(IMMUTABLE)
}

#[derive(Embed)]
#[folder = "dist"]
struct Assets;

#[derive(Clone, Copy)]
struct EmbedMetadata {
    len: u64,
    last_modified: Option<u64>,
}

impl FsMetadata for EmbedMetadata {
    fn is_dir(&self) -> bool {
        false
    }

    fn modified(&self) -> io::Result<SystemTime> {
        self.last_modified
            .map(|secs| UNIX_EPOCH + Duration::from_secs(secs))
            .ok_or_else(|| io::Error::other("embedded file has no last-modified timestamp"))
    }

    fn len(&self) -> u64 {
        self.len
    }
}

struct EmbedFile {
    cursor: std::io::Cursor<std::borrow::Cow<'static, [u8]>>,
    metadata: EmbedMetadata,
}

impl AsyncRead for EmbedFile {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        Pin::new(&mut self.cursor).poll_read(cx, buf)
    }
}

impl AsyncSeek for EmbedFile {
    fn start_seek(mut self: Pin<&mut Self>, position: io::SeekFrom) -> io::Result<()> {
        Pin::new(&mut self.cursor).start_seek(position)
    }

    fn poll_complete(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<u64>> {
        Pin::new(&mut self.cursor).poll_complete(cx)
    }
}

impl FsFile for EmbedFile {
    type Metadata = EmbedMetadata;
    type MetadataFuture<'a> = std::future::Ready<io::Result<EmbedMetadata>>;

    fn metadata(&self) -> Self::MetadataFuture<'_> {
        std::future::ready(Ok(self.metadata))
    }
}

#[derive(Clone, Copy)]
struct EmbedBackend;

impl Backend for EmbedBackend {
    type File = EmbedFile;
    type Metadata = EmbedMetadata;
    type OpenFuture = std::future::Ready<io::Result<EmbedFile>>;
    type MetadataFuture = std::future::Ready<io::Result<EmbedMetadata>>;

    fn open(&self, path: PathBuf) -> Self::OpenFuture {
        std::future::ready(lookup(&path).map(|file| {
            let metadata = embed_metadata(&file);
            EmbedFile {
                cursor: std::io::Cursor::new(file.data),
                metadata,
            }
        }))
    }

    fn metadata(&self, path: PathBuf) -> Self::MetadataFuture {
        std::future::ready(lookup(&path).map(|file| embed_metadata(&file)))
    }
}

fn lookup(path: &Path) -> io::Result<rust_embed::EmbeddedFile> {
    // `ServeDir::with_backend` prefixes requested paths with its `.` base,
    // while rust-embed stores paths relative to the embedded folder.
    let path = path.strip_prefix(".").unwrap_or(path);
    let key = path.to_string_lossy().replace('\\', "/");
    Assets::get(&key).ok_or_else(|| io::Error::from(io::ErrorKind::NotFound))
}

fn embed_metadata(file: &rust_embed::EmbeddedFile) -> EmbedMetadata {
    EmbedMetadata {
        len: file.data.len() as u64,
        last_modified: file.metadata.last_modified(),
    }
}

async fn spa_index() -> Response {
    match Assets::get(INDEX_HTML) {
        Some(content) => (
            [
                (
                    header::CONTENT_TYPE,
                    HeaderValue::from_static("text/html; charset=utf-8"),
                ),
                (header::CACHE_CONTROL, NO_CACHE),
            ],
            content.data,
        )
            .into_response(),
        None => (StatusCode::NOT_FOUND, "404").into_response(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::Body, http::Request};
    use tower::ServiceExt;

    async fn get(uri: &str) -> Response {
        router::<()>()
            .oneshot(Request::get(uri).body(Body::empty()).expect("request"))
            .await
            .expect("infallible")
    }

    #[test]
    fn lookup_accepts_serve_dir_relative_paths() {
        assert!(lookup(Path::new("./index.html")).is_ok());
    }

    #[tokio::test]
    async fn built_assets_are_immutable() {
        let asset = Assets::iter()
            .find(|path| path.starts_with("fe/"))
            .expect("frontend build has assets");
        let response = get(&format!("/{asset}")).await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers()[header::CACHE_CONTROL], IMMUTABLE);
    }

    #[tokio::test]
    async fn missing_assets_do_not_fall_back_to_index() {
        let response = get("/fe/missing-00000000.js").await;
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        assert!(response.headers().get(header::CACHE_CONTROL).is_none());
    }

    #[tokio::test]
    async fn app_routes_get_a_revalidated_index() {
        let response = get("/story/obt/main/level_main_00-01_beg").await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers()[header::CACHE_CONTROL], NO_CACHE);
        assert_eq!(
            response.headers()[header::CONTENT_TYPE],
            "text/html; charset=utf-8"
        );
    }
}

use crate::{
    api::{
        AppState,
        error::{WebError, WebResult},
    },
    database::launcher::{LauncherDiff, LauncherFile, LauncherRelease},
};
use axum::{
    Json,
    body::Body,
    extract::{Path, Query, State},
    http::header,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

#[derive(Debug, Deserialize, IntoParams)]
pub struct ReleaseQuery {
    pub before: Option<i64>,
    pub limit: Option<u32>,
}

#[derive(Debug, Deserialize, IntoParams)]
pub struct FileQuery {
    /// Return paths strictly after this path (the previous nextAfter).
    pub after: Option<String>,
    /// Literal, case-sensitive path substring.
    pub q: Option<String>,
    pub limit: Option<u32>,
}

#[derive(Debug, Deserialize, IntoParams)]
pub struct DownloadQuery {
    pub path: String,
}

#[derive(Debug, Deserialize, IntoParams)]
pub struct DiffQuery {
    pub from: i64,
    pub to: i64,
    pub after: Option<String>,
    pub limit: Option<u32>,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ReleasePage {
    pub releases: Vec<LauncherRelease>,
    pub next_before: Option<i64>,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct FilePage {
    pub files: Vec<LauncherFile>,
    pub next_after: Option<String>,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DiffPage {
    pub changes: Vec<LauncherDiff>,
    pub next_after: Option<String>,
}

fn page_limit(limit: Option<u32>) -> WebResult<usize> {
    let limit = limit.unwrap_or(100);
    if !(1..=200).contains(&limit) {
        return Err(WebError::BadRequest(
            "limit must be between 1 and 200".into(),
        ));
    }
    Ok(limit as usize)
}

async fn require_release(state: &AppState, id: i64) -> WebResult<LauncherRelease> {
    state
        .database
        .get_launcher_release(id)
        .await?
        .ok_or(WebError::NotFound)
}

#[utoipa::path(get, path = "/launcher/releases", tag = "launcher", params(ReleaseQuery), responses((status = 200, body = ReleasePage), (status = 400)))]
pub async fn list_releases(
    State(state): State<AppState>,
    Query(query): Query<ReleaseQuery>,
) -> WebResult<Json<ReleasePage>> {
    let limit = page_limit(query.limit)?;
    let mut releases = state
        .database
        .list_launcher_releases(query.before, i64::try_from(limit + 1).unwrap_or(201))
        .await?;
    let next_before = (releases.len() > limit).then(|| releases[limit - 1].id);
    releases.truncate(limit);
    Ok(Json(ReleasePage {
        releases,
        next_before,
    }))
}

#[utoipa::path(get, path = "/launcher/releases/{id}", tag = "launcher", responses((status = 200, body = LauncherRelease), (status = 404)))]
pub async fn get_release(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> WebResult<Json<LauncherRelease>> {
    Ok(Json(require_release(&state, id).await?))
}

#[utoipa::path(get, path = "/launcher/releases/{id}/files", tag = "launcher", params(FileQuery), responses((status = 200, body = FilePage), (status = 400), (status = 404)))]
pub async fn list_files(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Query(query): Query<FileQuery>,
) -> WebResult<Json<FilePage>> {
    let limit = page_limit(query.limit)?;
    require_release(&state, id).await?;
    let mut files = state
        .database
        .launcher_files(
            id,
            query.after.as_deref().unwrap_or(""),
            query.q.as_deref().unwrap_or(""),
            i64::try_from(limit + 1).unwrap_or(201),
        )
        .await?;
    let next_after = (files.len() > limit).then(|| files[limit - 1].path.clone());
    files.truncate(limit);
    Ok(Json(FilePage { files, next_after }))
}

#[utoipa::path(get, path = "/launcher/diff", tag = "launcher", params(DiffQuery), responses((status = 200, body = DiffPage), (status = 400), (status = 404)))]
pub async fn diff(
    State(state): State<AppState>,
    Query(query): Query<DiffQuery>,
) -> WebResult<Json<DiffPage>> {
    let limit = page_limit(query.limit)?;
    require_release(&state, query.from).await?;
    require_release(&state, query.to).await?;
    let mut changes = state
        .database
        .launcher_diff(
            query.from,
            query.to,
            query.after.as_deref().unwrap_or(""),
            i64::try_from(limit + 1).unwrap_or(201),
        )
        .await?;
    let next_after = (changes.len() > limit).then(|| changes[limit - 1].path.clone());
    changes.truncate(limit);
    Ok(Json(DiffPage {
        changes,
        next_after,
    }))
}

#[utoipa::path(get, path = "/launcher/releases/{id}/download", tag = "launcher", params(DownloadQuery), responses((status = 200, description = "Verified original bytes"), (status = 404), (status = 503, description = "File not archived yet")))]
pub async fn download(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Query(query): Query<DownloadQuery>,
) -> WebResult<Response> {
    let file = state
        .database
        .launcher_file(id, &query.path)
        .await?
        .ok_or(WebError::NotFound)?;
    let hash = file.sha256.ok_or_else(|| {
        WebError::Unavailable("file is still being archived; retry shortly".into())
    })?;
    let object = state.storage.download_launcher_blob(&hash).await?;
    Ok((
        [
            (header::CONTENT_TYPE, "application/octet-stream".to_string()),
            (header::CONTENT_LENGTH, file.size.to_string()),
            (header::CONTENT_DISPOSITION, "attachment".to_string()),
            (header::ETAG, format!("\"{hash}\"")),
        ],
        Body::from_stream(object.into_stream()),
    )
        .into_response())
}

#[utoipa::path(get, path = "/launcher/releases/{id}/manifest", tag = "launcher", responses((status = 200, description = "Original encrypted game_files"), (status = 404)))]
pub async fn manifest(State(state): State<AppState>, Path(id): Path<i64>) -> WebResult<Response> {
    let bytes = state
        .database
        .launcher_manifest(id)
        .await?
        .ok_or(WebError::NotFound)?;
    Ok((
        [
            (header::CONTENT_TYPE, "application/octet-stream"),
            (
                header::CONTENT_DISPOSITION,
                "attachment; filename=\"game_files\"",
            ),
        ],
        bytes,
    )
        .into_response())
}

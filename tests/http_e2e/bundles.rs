use crate::support::TestServer;
use ak_asset_storage::database::{
    Database,
    row::{AssetMappingStatus, BundleRow, FileRow},
};
use reqwest::StatusCode;
use serde_json::Value;
use sqlx::PgPool;

async fn file(database: &Database, hash: char) -> i32 {
    database
        .create_file(FileRow {
            id: None,
            hash: hash.to_string().repeat(64),
            size: 7,
        })
        .await
        .unwrap()
}

async fn bundle(database: &Database, version: i32, file: i32, path: &str) -> i32 {
    database
        .create_bundle(BundleRow {
            id: None,
            path: path.to_owned(),
            version_id: version,
            file_id: file,
        })
        .await
        .unwrap()
}

#[sqlx::test]
#[ignore = "requires PostgreSQL in DATABASE_URL and plocate; runs in CI"]
async fn pagination_preserves_ties_and_crosses_versions(pool: PgPool) {
    let env = TestServer::start(&pool).await;
    let old = env.version("v1", AssetMappingStatus::Ready).await;
    let new = env.version("v2", AssetMappingStatus::Ready).await;
    let first_file = file(&env.database, 'a').await;
    let second_file = file(&env.database, 'b').await;
    let old_id = bundle(&env.database, old, first_file, "arts/amiya#1.ab").await;
    let first = bundle(&env.database, new, first_file, "arts/amiya#1.ab").await;
    let tie = bundle(&env.database, new, second_file, "arts/amiya#1.ab").await;
    let last = bundle(&env.database, new, first_file, "arts/z.ab").await;
    bundle(&env.database, new, second_file, "other/file.ab").await;

    let mut cursor = String::new();
    let mut first_cursor = String::new();
    let mut seen = Vec::new();
    // A one-row page splits equal version/path keys. The last page is
    // exactly full and crosses the version boundary; neither may lose rows.
    for (limit, expected) in [
        ("1", vec![first]),
        ("1", vec![tie]),
        ("2", vec![last, old_id]),
    ] {
        let response = env
            .get(
                "/api/v1/bundle",
                &[
                    ("path", " arts/ "),
                    ("hash", " "),
                    ("limit", limit),
                    ("cursor", &cursor),
                ],
            )
            .await;
        assert_eq!(response.status(), StatusCode::OK);
        let page: Value = response.json().await.unwrap();
        let ids: Vec<_> = page["bundles"]
            .as_array()
            .unwrap()
            .iter()
            .map(|row| row["id"].as_i64().unwrap())
            .collect();
        assert_eq!(ids, expected.into_iter().map(i64::from).collect::<Vec<_>>());
        seen.extend(ids);
        if seen.len() == 4 {
            assert!(page["nextCursor"].is_null(), "{page}");
        } else {
            cursor = page["nextCursor"]
                .as_str()
                .expect("non-final page needs a cursor")
                .to_owned();
            if first_cursor.is_empty() {
                first_cursor.clone_from(&cursor);
            }
        }
    }
    assert_eq!(
        seen,
        vec![
            i64::from(first),
            i64::from(tie),
            i64::from(last),
            i64::from(old_id)
        ]
    );

    // A normalized equivalent filter can reuse the cursor and change size.
    let response = env
        .get(
            "/api/v1/bundle",
            &[("path", "arts/"), ("limit", "2"), ("cursor", &first_cursor)],
        )
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let page: Value = response.json().await.unwrap();
    assert_eq!(page["bundles"][0]["id"], tie);
    assert_eq!(page["bundles"][1]["id"], last);

    for changed in [
        vec![("path", "other/".to_owned())],
        vec![("path", "arts/".to_owned()), ("version", new.to_string())],
        vec![
            ("path", "arts/".to_owned()),
            ("file", first_file.to_string()),
        ],
        vec![("path", "arts/".to_owned()), ("hash", "a".repeat(64))],
    ] {
        let mut query: Vec<_> = changed
            .iter()
            .map(|(key, value)| (*key, value.as_str()))
            .collect();
        query.push(("cursor", &first_cursor));
        let response = env.get("/api/v1/bundle", &query).await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST, "{query:?}");
        let error: Value = response.json().await.unwrap();
        assert!(
            error["detail"]
                .as_str()
                .unwrap()
                .contains("cursor does not match"),
            "{error}"
        );
    }
    env.close().await;
}

#[sqlx::test]
#[ignore = "requires PostgreSQL in DATABASE_URL and plocate; runs in CI"]
async fn filters_match_literal_paths_and_reject_invalid_pages(pool: PgPool) {
    let env = TestServer::start(&pool).await;
    let version = env.version("v1", AssetMappingStatus::Ready).await;
    let file = file(&env.database, 'a').await;
    let path = r"arts/100%_complete\amiya#1.ab";
    let expected = bundle(&env.database, version, file, path).await;
    bundle(&env.database, version, file, "arts/100XXcompleteamiya#1.ab").await;
    for needle in [path, "100%", "_complete", r"complete\"] {
        let response = env.get("/api/v1/bundle", &[("path", needle)]).await;
        assert_eq!(response.status(), StatusCode::OK);
        let page: Value = response.json().await.unwrap();
        assert_eq!(
            page["bundles"].as_array().unwrap().len(),
            1,
            "{needle}: {page}"
        );
        assert_eq!(page["bundles"][0]["id"], expected);
        assert_eq!(page["bundles"][0]["path"], path);
        assert!(page["nextCursor"].is_null());
    }

    for query in [
        vec![("path", " \t"), ("hash", " ")],
        vec![("path", "arts/"), ("limit", "0")],
        vec![("path", "arts/"), ("limit", "201")],
        vec![("path", "arts/"), ("cursor", "not-a-cursor!")],
        // Valid base64url JSON, but not a bundle cursor.
        vec![("path", "arts/"), ("cursor", "e30")],
    ] {
        let response = env.get("/api/v1/bundle", &query).await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST, "{query:?}");
        let error: Value = response.json().await.unwrap();
        assert!(
            error["detail"]
                .as_str()
                .unwrap()
                .starts_with("Bad Request:"),
            "{error}"
        );
    }
    let response = env
        .get("/api/v1/bundle", &[("path", "missing"), ("limit", "200")])
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let page: Value = response.json().await.unwrap();
    assert_eq!(page["bundles"], serde_json::json!([]));
    assert!(page["nextCursor"].is_null());
    env.close().await;
}

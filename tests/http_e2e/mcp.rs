use crate::support::TestServer;
use ak_asset_storage::database::row::AssetMappingStatus;
use reqwest::StatusCode;
use serde_json::{Value, json};
use sqlx::PgPool;

#[sqlx::test]
#[ignore = "requires PostgreSQL in DATABASE_URL and plocate; runs in CI"]
async fn stateless_transport_lists_tools_and_distinguishes_errors(pool: PgPool) {
    let env = TestServer::start(&pool).await;
    let initialized = env
        .rpc(
            "initialize",
            json!({
                "protocolVersion": "2025-03-26",
                "capabilities": {},
                "clientInfo": {"name": "http-e2e", "version": "1.0.0"}
            }),
        )
        .await;
    assert!(initialized.get("error").is_none(), "{initialized}");
    assert_eq!(initialized["result"]["protocolVersion"], "2025-03-26");
    assert!(initialized["result"]["capabilities"]["tools"].is_object());

    let listing = env.rpc("tools/list", json!({})).await;
    let tools = listing["result"]["tools"].as_array().unwrap();
    for name in [
        "list_versions",
        "get_version",
        "search_bundles",
        "list_files",
        "search_files",
    ] {
        assert!(
            tools.iter().any(|tool| tool["name"] == name),
            "missing {name}: {listing}"
        );
    }
    assert_eq!(
        env.tool("list_versions", json!({})).await["results"],
        json!([])
    );

    for (name, arguments) in [
        ("search_bundles", json!({"path": " ", "hash": "\t"})),
        ("list_files", json!({"path": "../outside"})),
        ("list_files", json!({"path": "/etc"})),
        ("list_files", json!({"path": "raw", "limit": 0})),
        (
            "list_files",
            json!({"path": "raw", "cursor": "not-a-cursor!"}),
        ),
    ] {
        let body = env
            .rpc("tools/call", json!({"name": name, "arguments": arguments}))
            .await;
        assert_eq!(body["error"]["code"], -32602, "{body}");
        assert!(body.get("result").is_none(), "{body}");
    }
    let missing = env
        .rpc(
            "tools/call",
            json!({
                "name": "get_version", "arguments": {"version_id": i32::MAX}
            }),
        )
        .await;
    assert!(missing.get("error").is_none(), "{missing}");
    assert_eq!(missing["result"]["isError"], true);
    assert!(
        missing["result"]["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("not found")
    );
    // A rejected call must not poison subsequent independent requests.
    assert_eq!(env.tool("list_versions", json!({})).await["total"], 0);
    env.close().await;
}

#[sqlx::test]
#[ignore = "requires PostgreSQL in DATABASE_URL and plocate; runs in CI"]
async fn file_pages_return_downloadable_urls_and_scope_cursors(pool: PgPool) {
    let env = TestServer::start(&pool).await;
    std::fs::create_dir_all(env.assets.join("raw/0-dir")).unwrap();
    env.write_asset("raw/z.txt", "ordinary asset");
    env.write_asset("raw/阿米娅 #1%?.txt", "带有特殊字符的资源");
    env.write_asset("gamedata/v1/excel/table.json", r#"{"name":"阿米娅"}"#);
    let mut cursor = Value::Null;
    let mut first_cursor = Value::Null;
    for (index, (name, content)) in [
        ("0-dir", None),
        ("z.txt", Some("ordinary asset")),
        ("阿米娅 #1%?.txt", Some("带有特殊字符的资源")),
    ]
    .into_iter()
    .enumerate()
    {
        let page = env
            .tool(
                "list_files",
                json!({"path": "raw", "limit": 1, "cursor": cursor}),
            )
            .await;
        assert_eq!(page["total"], 3);
        assert_eq!(page["results"].as_array().unwrap().len(), 1);
        let entry = &page["results"][0];
        assert_eq!(entry["name"], name);
        assert_eq!(entry["path"], format!("raw/{name}"));
        assert_eq!(entry["is_dir"], content.is_none());
        if let Some(content) = content {
            let url = reqwest::Url::parse(entry["url"].as_str().unwrap()).unwrap();
            assert!(
                url.as_str()
                    .starts_with(&format!("{}/assets/", env.base_url))
            );
            assert!(url.fragment().is_none());
            assert!(url.query().is_none());
            let response = env.client.get(url).send().await.unwrap();
            assert_eq!(response.status(), StatusCode::OK);
            assert_eq!(response.text().await.unwrap(), content);
        } else {
            assert!(
                entry.get("url").is_none(),
                "directory must not advertise a download URL"
            );
        }
        assert_eq!(page["truncated"], index < 2);
        cursor = page["next_cursor"].clone();
        if index < 2 {
            assert!(cursor.is_string());
            if index == 0 {
                first_cursor = cursor.clone();
            }
        } else {
            assert!(cursor.is_null());
        }
    }
    let changed = env
        .rpc(
            "tools/call",
            json!({
                "name": "list_files",
                "arguments": {"path": "gamedata/v1/excel", "cursor": first_cursor}
            }),
        )
        .await;
    assert_eq!(changed["error"]["code"], -32602, "{changed}");
    assert!(
        changed["error"]["message"]
            .as_str()
            .unwrap()
            .contains("cursor does not match")
    );

    let page = env
        .tool("list_files", json!({"path": "gamedata/v1/excel"}))
        .await;
    let url = page["results"][0]["url"].as_str().unwrap();
    assert_eq!(
        url,
        format!("{}/gamedata/v1/excel/table.json", env.base_url)
    );
    let response = env.client.get(url).send().await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.json::<Value>().await.unwrap(),
        json!({"name": "阿米娅"})
    );
    let empty = env.tool("list_files", json!({"path": "raw/0-dir"})).await;
    assert_eq!(
        empty,
        json!({"results": [], "total": 0, "truncated": false, "next_cursor": null})
    );
    env.close().await;
}

#[sqlx::test]
#[ignore = "requires PostgreSQL in DATABASE_URL and plocate; runs in CI"]
async fn gamedata_readiness_filters_before_limit_and_updates_after_marker(pool: PgPool) {
    let env = TestServer::start(&pool).await;
    let missing = env.version("v1", AssetMappingStatus::Ready).await;
    let ready = env.version("v2", AssetMappingStatus::Pending).await;
    let unmarked = env.version("v3", AssetMappingStatus::Ready).await;
    env.write_asset("gamedata/v2/.gamedata-ready.json", "{}");
    env.write_asset("gamedata/v3/excel/table.json", "{}");

    let versions = env.tool("list_versions", json!({})).await;
    let rows = versions["results"].as_array().unwrap();
    assert_eq!(rows.len(), 3);
    for (row, (id, status)) in rows.iter().zip([
        (unmarked, "unmarked"),
        (ready, "ready"),
        (missing, "missing"),
    ]) {
        assert_eq!(row["id"], id);
        assert_eq!(row["isReady"], true);
        assert_eq!(row["gamedataStatus"], status);
        let detail = env.tool("get_version", json!({"version_id": id})).await;
        assert_eq!(detail["gamedataStatus"], status);
        assert_eq!(detail["hotUpdateSummary"]["abInfos"]["count"], 1);
        assert!(detail.get("hotUpdateList").is_none());
    }
    // The newest version has downloaded bundles but incomplete gamedata.
    // Filtering must happen before limit=1, and not require a ready manifest.
    let filtered = env
        .tool(
            "list_versions",
            json!({"gamedata_ready_only": true, "limit": 1}),
        )
        .await;
    assert_eq!(filtered["results"].as_array().unwrap().len(), 1);
    assert_eq!(filtered["results"][0]["id"], ready);
    assert_eq!(filtered["total"], 1);
    assert_eq!(filtered["truncated"], false);
    let both = env
        .tool(
            "list_versions",
            json!({"gamedata_ready_only": true, "ready_only": true}),
        )
        .await;
    assert_eq!(both["results"], json!([]));

    env.write_asset("gamedata/v3/.gamedata-ready.json", "{}");
    let updated = env
        .tool(
            "list_versions",
            json!({"gamedata_ready_only": true, "limit": 1}),
        )
        .await;
    assert_eq!(updated["results"][0]["id"], unmarked);
    assert_eq!(updated["results"][0]["gamedataStatus"], "ready");
    assert_eq!(updated["total"], 2);
    assert_eq!(updated["truncated"], true);
    let detail = env.tool("get_version", json!({"res_version": "v3"})).await;
    assert_eq!(detail["id"], unmarked);
    assert_eq!(detail["gamedataStatus"], "ready");
    env.close().await;
}

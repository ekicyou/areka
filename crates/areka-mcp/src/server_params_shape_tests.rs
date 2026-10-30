//! 実ソケットのテスト: `params` がオブジェクトでない要求への受け口の答え
//! （spec: mcp-strict-errors 要件 6.1〜6.4・6.6・6.7・7.7）。
//!
//! 5 つの版の話し方で送る。旧式の 4 版は `MCP-Protocol-Version` を付け、`2026-07-28` は
//! `Mcp-Method` も付ける（`params` がオブジェクトでないので `_meta` は入れられない）。
//! 差の一覧（`doc/ssp-mcp/transport-diff-areka.md`）に写す値は各 assert に具体の値で固定する。
//! 測った版は rmcp 3.5.0。

use std::net::SocketAddr;

use serde_json::{Value, json};

use super::server_protocol_tests::{SUPPORTED_VERSIONS, speak};
use crate::ToolRegistry;
use crate::testkit::{Response, post_rpc, serve};

const STATELESS: &str = "2026-07-28";
/// rmcp が読めない本文に返す 415 の平文（JSON として読めた 1 件の形でないとき）。
const RMCP_415_SHAPE: &str = "fail to deserialize request body data did not match any variant of untagged enum JsonRpcMessage";
const SCALAR_MESSAGE: &str = "Invalid Request: params must be an array or an object";
const ARRAY_MESSAGE: &str = "Invalid params: params must be an object";

/// 本文 `body` を版 `version` の話し方で送る（`method` は `2026-07-28` の `Mcp-Method` に使う）。
fn send(addr: SocketAddr, version: &str, method: &str, body: &str) -> Response {
    let mut headers = vec![("MCP-Protocol-Version", version)];
    if version == STATELESS {
        headers.push(("Mcp-Method", method));
    }
    post_rpc(addr, &headers, body)
}

fn request(id: &Value, method: &str, params: &Value) -> String {
    json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params }).to_string()
}

/// (要件 6.1・6.2・6.4・6.6) 数・文字列・真偽値は `-32600`、配列は `-32602` の、要求の `id`
/// （数・文字列）を付けた 1 件（`application/json`）。知らない名前のメソッドでも同じ。
/// HTTP の状態は旧式の 4 版が 200、`2026-07-28` は `-32600` が 200・`-32602` が 400。
/// 答えた後に同じサーバへ送る `tools/list` はいつもどおり通る。
#[test]
fn non_object_params_answer_with_the_id_in_every_version() {
    let (_server, addr) = serve(ToolRegistry::default());
    let cases = [
        (json!(7), -32600, SCALAR_MESSAGE),
        (json!(1.5), -32600, SCALAR_MESSAGE),
        (json!("x"), -32600, SCALAR_MESSAGE),
        (json!(true), -32600, SCALAR_MESSAGE),
        (json!(false), -32600, SCALAR_MESSAGE),
        (json!([1]), -32602, ARRAY_MESSAGE),
        (json!([]), -32602, ARRAY_MESSAGE),
    ];
    let mut n = 0u64;
    for version in SUPPORTED_VERSIONS {
        for method in ["tools/list", "no/such"] {
            for (params, code, message) in &cases {
                n += 1;
                let id = if n % 2 == 0 {
                    json!(n)
                } else {
                    json!(format!("s{n}"))
                };
                let response = send(addr, version, method, &request(&id, method, params));
                let expected_status = if version == STATELESS && *code == -32602 {
                    400
                } else {
                    200
                };
                let at = format!("{version} {method} {params}");
                assert_eq!(response.status, expected_status, "{at}: {response:?}");
                assert_eq!(
                    response.header("content-type"),
                    Some("application/json"),
                    "{at}"
                );
                assert_eq!(
                    response.json(),
                    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } }),
                    "{at}"
                );
                let next = speak(addr, version, n, "tools/list", None);
                assert_eq!(next.status, 200, "{at}: {next:?}");
                assert_eq!(next.json()["result"]["tools"], json!([]), "{at}: {next:?}");
            }
        }
    }
}

/// (要件 6.7) 当たらないものは今のまま: 通知・`id` が `null` の要求・JSON でない本文は rmcp の
/// 415 の平文、`params` が `null` の要求は rmcp の答え（旧式は成功・`2026-07-28` は `_meta` の欠落の
/// 400・`-32602`）。
#[test]
fn requests_outside_the_shape_keep_current_answers_in_every_version() {
    let (_server, addr) = serve(ToolRegistry::default());
    for version in SUPPORTED_VERSIONS {
        let plain_415 = [
            (
                "tools/list",
                json!({ "jsonrpc": "2.0", "method": "tools/list", "params": 7 }).to_string(),
                RMCP_415_SHAPE,
            ),
            (
                "notifications/initialized",
                json!({ "jsonrpc": "2.0", "method": "notifications/initialized", "params": [1] })
                    .to_string(),
                RMCP_415_SHAPE,
            ),
            (
                "tools/list",
                request(&Value::Null, "tools/list", &json!(7)),
                RMCP_415_SHAPE,
            ),
            (
                "ping",
                "{".to_owned(),
                "fail to deserialize request body EOF while parsing an object at line 1 column 1",
            ),
        ];
        for (method, body, text) in plain_415 {
            let response = send(addr, version, method, &body);
            assert_eq!(response.status, 415, "{version} {body}: {response:?}");
            assert_eq!(response.header("content-type"), None, "{version} {body}");
            assert_eq!(
                String::from_utf8_lossy(&response.body),
                text,
                "{version} {body}"
            );
        }
        let response = send(
            addr,
            version,
            "tools/list",
            &request(&json!(1), "tools/list", &Value::Null),
        );
        if version == STATELESS {
            assert_eq!(response.status, 400, "{response:?}");
            assert_eq!(response.json()["error"]["code"], -32602, "{response:?}");
        } else {
            assert_eq!(response.status, 200, "{version}: {response:?}");
            assert_eq!(response.json()["result"]["tools"], json!([]), "{version}");
        }
    }
}

/// (要件 6.3・6.7) rmcp が読める要求は rmcp の答えのまま（`tools/list` は成功、知らない名前は `-32601`）。
#[test]
fn rmcp_readable_requests_keep_rmcp_answers_in_every_version() {
    let (_server, addr) = serve(ToolRegistry::default());
    for version in SUPPORTED_VERSIONS {
        let list = speak(addr, version, 1, "tools/list", Some(json!({})));
        assert_eq!(list.status, 200, "{version}: {list:?}");
        assert_eq!(list.json()["result"]["tools"], json!([]), "{version}");
        let unknown = speak(addr, version, 2, "no/such", Some(json!({})));
        let expected_status = if version == STATELESS { 404 } else { 200 };
        assert_eq!(unknown.status, expected_status, "{version}: {unknown:?}");
        assert_eq!(
            unknown.json()["error"],
            json!({ "code": -32601, "message": "no/such" }),
            "{version}"
        );
    }
}

/// (要件 6.4) 状態の写し（`shape_error_status`）が rmcp とずれていない: 同じ版・同じメソッドで
/// rmcp 自身が返す `-32602`（`2026-07-28` は `params: null` の `_meta` の欠落、旧式は未登録の名前の
/// `tools/call`）と、areka の配列の `params` への `-32602` が同じ HTTP の状態。
/// rmcp の版を上げて対応が変わればここが赤になる。
#[test]
fn invalid_params_status_matches_rmcp_own_in_every_version() {
    let (_server, addr) = serve(ToolRegistry::default());
    for version in SUPPORTED_VERSIONS {
        let rmcp_own = if version == STATELESS {
            send(
                addr,
                version,
                "tools/list",
                &request(&json!(1), "tools/list", &Value::Null),
            )
        } else {
            speak(
                addr,
                version,
                1,
                "tools/call",
                Some(json!({ "name": "no_such_tool", "arguments": {} })),
            )
        };
        assert_eq!(
            rmcp_own.json()["error"]["code"],
            -32602,
            "{version}: {rmcp_own:?}"
        );
        let areka = send(
            addr,
            version,
            "tools/list",
            &request(&json!(2), "tools/list", &json!([1])),
        );
        assert_eq!(
            areka.json()["error"]["code"],
            -32602,
            "{version}: {areka:?}"
        );
        assert_eq!(areka.status, rmcp_own.status, "{version}");
    }
}

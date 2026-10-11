# 名乗りが食い違うとテストが赤になることの確かめの記録（タスク 6.4）

- 日付: 2026-10-10
- 机: 開発者の机（x64）。HEAD は `1a30cbb4`。
- 確かめたこと: 受け口の名乗りと、中継が探す名乗りが食い違ったら、本物の受け口を相手にしたテスト（`crates/areka-mcp-bridge/tests/real_receiver_test.rs`）が赤になる（要件 8.2）。

## 一時的に変えたこと

- 変えたのは 1 か所だけ: 受け口が自分の情報を組む関数 `ArekaHandler` の `get_info`（`crates/areka-mcp/src/handler.rs`）で、`Implementation::new` に渡す名前を、共有の定数 `SERVER_NAME` から文字列 `"areka-mcp-server-x"` に替えた。
- 共有の定数 `SERVER_NAME`（同じファイル。値は `areka-mcp-server`）は動かしていない。中継はこの定数を読んで相手を確かめるので、定数を動かすと受け口と中継が一緒に動いて赤にならない。
- 変える前の写しを `target\task64\handler.rs.bak` に取った（SHA256 `d27f3a45b47a1438ab650045cc405100c446228daa2d6b1d88e12210c5f79525`）。

## 赤

- 回したもの: `cargo test -p areka-mcp-bridge --locked --test real_receiver_test`
- 結果: 終了コード 101。5 つの版のテストが 5 本とも赤（`test result: FAILED. 0 passed; 5 failed`）。
- 赤になった場所: 5 本とも、関数 `compare` の中の、中継ごしの答えと直接の答えを比べる `assert_eq!`（文「中継ごしの答え（左）が直接の答え（右）と違う」）。どの版も最初の行（最初の問い合わせ）で止まった。
- 中継ごしの答え（左）は、名乗りの違う相手を飛ばした「居ない」エラー。直接の答え（右）は、替えた名乗りの入った結果。印字された行（長い欄は略）:

  ```text
  test v2026_07_28_bridged_answers_equal_direct_answers ... FAILED
  test v2025_03_26_bridged_answers_equal_direct_answers ... FAILED
  test v2025_06_18_bridged_answers_equal_direct_answers ... FAILED
  test v2025_11_25_bridged_answers_equal_direct_answers ... FAILED
  test v2024_11_05_bridged_answers_equal_direct_answers ... FAILED

  assertion `left == right` failed: 2025-11-25 {"at":"/result/protocolVersion","id":1,"is":"2025-11-25","method":"initialize","params":{…}}: 中継ごしの答え（左）が直接の答え（右）と違う
    left: Object {"error": Object {"code": Number(-1), "message": String("Server not available: areka-mcp-server not found on 127.0.0.1 ports 50113; port 50113 answered as \"areka-mcp-server-x\"")}, "id": Number(1), "jsonrpc": String("2.0")}
   right: Object {"id": Number(1), "jsonrpc": String("2.0"), "result": Object {…, "protocolVersion": String("2025-11-25"), "serverInfo": Object {"name": String("areka-mcp-server-x"), "version": String("0.0.2")}}}

  assertion `left == right` failed: 2026-07-28 {"at":"/result/_meta/io.modelcontextprotocol~1serverInfo/name","id":1,"is":"areka-mcp-server","method":"server/discover"}: 中継ごしの答え（左）が直接の答え（右）と違う
    left: Object {"error": Object {"code": Number(-1), "message": String("Server not available: areka-mcp-server not found on 127.0.0.1 ports 50109; port 50109 answered as \"areka-mcp-server-x\"")}, "id": Number(1), "jsonrpc": String("2.0")}

  test result: FAILED. 0 passed; 5 failed; 0 ignored; 0 measured; 0 filtered out
  ```

- 番号（50113・50109）は、テストが起こした本物の受け口に OS が割り当てたもの（机の 9801〜9830 へはつないでいない）。

## 戻し

- 写しから `handler.rs` を戻し、ファイルの時刻を更新して建て直した。戻した後の SHA256 は上の値と同じで、`git diff --stat -- crates/areka-mcp` は空。
- 戻した後: `cargo test -p areka-mcp-bridge --locked --test real_receiver_test` は 5 本とも緑（受け口と中継の建て直しが走った）、`cargo test -p areka-mcp --locked` も緑。

# ギャップ分析: areka-P0-mcp-server-core

> 2026-10-03・本ブランチ（main `76e17654` と同じコード）で、確定した requirements.md と今のコードの間の差を調べた。コードは「何の定義か」（関数名・型名・定数名＋ファイルパス）で指し、行番号では指さない。
> rmcp は **crates.io の 3.5.0（リリース版）** を docs.rs の文書と docs.rs のソース表示で読み、さらに**作業用の一時フォルダに rmcp 3.5.0 だけを依存に持つ空のクレートを作って `cargo tree` と `cargo deny check licenses`（本リポジトリの `deny.toml` を複製）を実際に回した**（リポジトリの `Cargo.lock` には触れていない）。
> 本文書は「情報と選択肢」を出すもので、最終の決定は設計の段で行う。

## 1. 結論の要約

- **本番のサーバは 0・前例は 3 つ。** HTTP も非同期ランタイムも本番には無い。使える前例は ⑴ 空きポートを OS に割り当てさせる偽サーバ（`crates/areka-update/src/winhttp_real_tests.rs` の `serve`＝`TcpListener::bind("127.0.0.1:0")`）、⑵ 裏方のスレッドを起動時に立てて終了直前に畳む取っ手（`crates/areka/src/perf_thread_report.rs` の `start`／`ReportHandle::stop_and_report_final`）、⑶ 環境変数を読まない純粋な読み解き（`crates/areka/src/main.rs` の `smoke_exit_ms_from`・`perf_thread_report.rs` の `period_from_env_value`）。
- **rmcp 3.5.0 は要件の中心（無状態・JSON の単発・`Host` 検査・`Origin` の許可表・415・202・`server/discover`）をそのまま持つ。** ただし要件の 2 行は rmcp の現物と食い違う見込みがある＝ **未知メソッドの応答コード（要件 3.7 の `-32601`）** と **JSON-RPC エラーのときの HTTP の状態コード（`-32602` は 400・`-32601` は 404）**。どちらも「差の一覧」で扱えるが、要件 3.7 の文面は要件ディスカッションで確かめる（§7 議題 1）。
- **依存の重さは実測で新しいクレート 30 個（hyper を直に使う形）・axum を使うと 38 個。ライセンスの門は両方とも緑**（`cargo deny check licenses` が `licenses ok`・新しいクレートは全部 MIT か MIT OR Apache-2.0、rmcp 本体は Apache-2.0）。`deny.toml` の許可の表に足すものは **0**。
- **推奨の形**: 新しい葉クレート `areka-mcp` が ⑴ 呼び出し側のスレッドで同期に `std::net::TcpListener` を束ね（失敗はその場で `error!`）、⑵ `areka_actor::spawn_actor("mcp", …)` で起こした 1 本のスレッドの中に tokio の `current_thread` ランタイムを立て、⑶ hyper 1 を直に使った小さな振り分け（`POST /api/mcp/v1` → rmcp の `StreamableHttpService`・`GET /api/mcp/help` → 自前・それ以外 404）を回し、⑷ `Origin`／`Host` の検査は自前の純粋関数 1 つで両方のパスに掛ける（help は rmcp の外なので自前が要る＝1 か所に寄せる）。main.rs は「立てる 1 行・畳む 1 行」で済み、畳む側は `Drop` にして `down?` の早い戻りの罠を避ける。
- **設計で調べ残し（Research Needed）は 5 件**（§6）。最大のものは「Claude Code が 400／404 に載った JSON-RPC エラーをどう扱うか」で、本 spec の登録・`initialize`・`tools/list`・`ping` には関係しないが、後続 `mcp-tool-entrances` のエラーの見え方を決める。

## 2. 今のコードにあるもの（現状の調べ）

### 2.1 サーバ・非同期・HTTP

| 項目 | 実測 |
|---|---|
| 本番の `std::net`・tokio ランタイム・HTTP | **無い**（brief の 10-02 再測定と同じ）。tokio 1.53.1 は `Cargo.lock` に在るが wasm の任意依存で、x64 のビルドには入っていない |
| テストの偽 HTTP サーバ | `crates/areka-update/src/winhttp_real_tests.rs` の `serve`（`TcpListener::bind("127.0.0.1:0")` → `local_addr()` で番号を知る）と `answer`（HTTP/1.1 を手で書く）。**本 spec の決定論テストの型そのもの**。ただしこのテストは `#[ignore]`（本物の WinHTTP を使うため）で、本 spec は常時テストに置く |
| 実プロセスの smoke | `crates/areka/tests/smoke_boot_loop_exit.rs`（`CARGO_BIN_EXE_areka` を子プロセスで起こし、終了コードとログの行で判定）。実機の確かめ（要件 9.5）の補助に使える型 |
| `serde_json` | lock に 1.0.151（dola・budouy・ukadoc-survey 経由）。本番にも入っている |

### 2.2 起動と終了の配線（`crates/areka/src/main.rs`・`fn main()`）

- 裏方を立てる前例は `perf_thread_report::start()`（`thread_roles::install()` の直後）。返り値 `Option<ReportHandle>` を終了直前まで持ち回り、`finish_after_run` の閉包の最後で `stop_and_report_final()` を呼ぶ。
- **罠**: その閉包の中に `down?` があり、降ろす処理が失敗すると `?` で早く戻って perf の後始末は飛ぶ（本文のコメントにも「②③ の失敗で早く戻る経路では最後の 1 枚が出ない」と明記）。MCP を畳む処理をここに置くと同じ罠を踏む。
- `main.rs` は 946 行（1,000 行の番人 `crates/log-capture-kit/tests/file_length_guard_test.rs` の射程）。足せるのは数行。
- `WinApp::with_exit_policy(ExitPolicy::Explicit)` の後に `register_systems` → `install_boot_context` → `open_ghost_windows`（失敗なら `return Err`）→ `boot_first_ghost` → `app.run()`。要件 1.1「ゴーストの起動の成否によらない」は、`open_ghost_windows` の失敗（プロセスが終わる）ではなく `boot_first_ghost` の fallback の話なので、立てる位置は `boot_first_ghost` より前ならどこでもよい。

### 2.3 スレッドと名簿

- `areka_actor::spawn_actor(name, body)`（`crates/areka-actor/src/spawn.rs`）: 名前付きスレッドを起こし、走り始めに `thread_roles` のフックが wintf の名簿へ `actor:<name>` で登録する。`areka-mcp` は wintf に依存できない（葉）が、`areka-actor` への依存は `areka-sylphya` に前例がある。
- tokio の `current_thread` ランタイムはスレッドを増やさない。`spawn_blocking` を使うと tokio の blocking pool が別スレッドを起こし、それは名簿に載らず `unregistered_rest` に出る（brief の指摘のとおり。本 spec では使わない）。

### 2.4 環境変数の読み解きの前例

- `smoke_exit_ms_from(Option<&str>) -> Option<u64>`（main.rs）: 未設定・空・数でない・負・溢れ → `None`。
- `period_from_env_value(Option<&str>) -> Duration`（perf_thread_report.rs）: 読めない値は **`warn!` を残して既定へ倒す**。**要件 2.5 の形（`warn!`＋既定 9821）はこちらの前例と同じ**。`0` を「待ち受けない」に使う点だけが新しい。
- 非 UTF-8 の値は `read_period_env` が `warn!` を残して未設定扱い。同じ扱いが要る。

### 2.5 ログの捕捉

- `log_capture_kit::capture(f)` は**呼び出しスレッドで同期に出た**イベントだけを集める（`crates/log-capture-kit/src/capture.rs` の doc）。別スレッドのイベントは `install_global_capture_all`（`global.rs`・プロセスに 1 回・番人の例外表に載せる必要あり）でしか拾えない。
- **設計への含み**: 要件 9.3（待受の失敗で `error!` が 1 件）を `capture` で固定したいなら、**束ねる処理を呼び出し側のスレッドで同期に行う**（tokio のスレッドの中で束ねない）。そうすれば `start()` が束ねの成否を同期に返せ、help ページに載せる実際のポート番号もその場で分かる（テストの `127.0.0.1:0` でも `local_addr()` で実番号が取れる）。

### 2.6 ライセンスの門と告知の生成

- `deny.toml`: 許可 9 種・`[graph] all-features = true`（**ワークスペースの crate の**全機能を有効にして評価する。依存先 rmcp の任意機能〔`auth`・`reqwest` など〕までは有効にならない＝`areka-mcp` が機能フラグを持たなければ増えない）。
- `about.toml`: 同じ 9 種・`ignore-dev-dependencies = true`。`tools/test-all.ps1 -License` が `cargo deny check` → `cargo about generate --workspace about.hbs -o THIRD-PARTY-NOTICES.md` を直列で回し、`THIRD-PARTY-NOTICES.md` の差分を黄色で知らせる（手で直さない）。
- 道具は手元に在る: cargo-deny 0.20.2・cargo-about 0.9.2・cargo 1.99.0（rmcp 3.5.0 の `rust-version = 1.88` を満たす）。

### 2.7 新しいクレートの型（`Cargo.toml`）

- `crates/areka-update/Cargo.toml` が手本: `workspace = "../.."`・`version.workspace = true`・`publish = false`・依存はクレート自身に書き、根の `[workspace.dependencies]` は変えない。`publish` を開けるかは C2 の `crates-io-publish` が決める（roadmap の C2 の行）。
- ワークスペースの見張り（`log-capture-kit/tests/`）は `crates/**` を自動で走査する＝新クレートは何もしなくても 1,000 行の番人・捕捉先の迂回検知・一時パスの迂回検知の対象になる。

### 2.8 文書

- `doc/ssp-mcp/survey.md` §2 の表は 14 行＋§1 の手打ちフォーム＝要件 5.1 の 15 行。`doc/ssp-mcp/` には survey と `tools-list-ssp-2.9.05.json` の 2 つだけ（差の一覧は新しい 1 文書）。
- `dist/README.txt` に `AREKA_` の環境変数の記述は今 **0 件**。要件 2.7 は「help と `dist/README.txt` 以外には書かない」と言うが、roadmap の C1 の行（⑦ の触るファイル）に `dist/README.txt` は**載っていない**（C2 の `crates-io-publish`・C3 の winget・C4 の `install-live-target-hazards` が分け合う）。§7 議題 7。
- `doc/CONSTITUTION.md`「MCP 採用方針」は本 spec 群への案内が済んでいる（brief で直した）。

## 3. rmcp 3.5.0 の現物（確かめた事実）

docs.rs の `StreamableHttpServerConfig`・`StreamableHttpService`・`ServerHandler`・`ToolRouter`・`NeverSessionManager` の文書と、docs.rs のソース表示（`transport/streamable_http_server/tower.rs`・`handler/server.rs`・`handler/server/router/tool.rs`・`model.rs`）で読んだ。**要件の文面との突き合わせ**を右の列に書く。

| 項目 | rmcp 3.5.0 の現物 | 要件との関係 |
|---|---|---|
| `StreamableHttpServerConfig` の既定 | `legacy_session_mode: true`・`json_response: false`・`allowed_hosts: ["localhost","127.0.0.1","::1"]`・`allowed_origins: []`（空＝検査しない）・`max_request_body_bytes: 4 MiB`・`stateless_protocol_metadata_required: false`・`sse_keep_alive: 15s`・`cancellation_token` | 要件 3.9・3.10 は `with_legacy_session_mode(false)`＋`with_json_response(true)` で選ぶ（requirements §「brief を引き直して改めた点」1 と一致） |
| 無状態のときの `initialize`（旧式の版） | `legacy_session_mode == false` なら `initialize` も `ping` も **セッションを作らず** `serve_negotiated_request_directly`／`OneshotTransport` で単発に処理する。`Mcp-Session-Id` は発行も要求もしない。セッション管理は `NeverSessionManager`（全操作を `ErrorSessionManagementNotSupported` で断る）を渡す | 要件 3.1・3.9 を満たす見込み。**設計で最初に書く決定論テスト（`initialize` 5 版→`ping`）で裏を取る** |
| `json_response: true` | 「`legacy_session_mode` が false のとき、結果かエラーで終わる要求へ `application/json` を返す。途中で通知や要求を出す handler なら `text/event-stream` に倒れる」（doc の逐語）。本 spec のツールは 0 本なので SSE に倒れる経路は無い | 要件 3.10 |
| 通知（`id` 無し） | `accepted_response()`＝**202・本文なし** | 要件 3.3 |
| JSON でない本文 | `expect_json` が `Content-Type` に `application/json` が無ければ **415**。本文が JSON として壊れていれば **400＋JSON-RPC の `-32600`（Invalid Request）** | 要件 3.8 は「4xx か `-32700`」としか決めていないので満たす。差の一覧には「SSP は 400・`-32700`、areka は 415 または 400・`-32600`」と書く |
| **未知のメソッド** | 本文を `ClientJsonRpcMessage` へ読む段で、知らないメソッド名は読み解きに失敗し **400＋`-32600`（Invalid Request）** になる見込み。`-32601` を返す別の経路は見当たらない | **要件 3.7 の `-32601` と食い違う見込み**（§7 議題 1） |
| `tools/call`（未知の名前） | `ServerHandler::call_tool` の既定は `method_not_found`（`-32601`）。**`ToolRouter::call` なら `invalid_params("tool not found")`＝`-32602`** | 要件 3.6・7.3 は **ToolRouter を通す形でだけ**満たせる（既定の `call_tool` のままだと `-32601`） |
| JSON-RPC エラー → HTTP の状態 | `jsonrpc_http_status`: `INVALID_PARAMS`・`UNSUPPORTED_PROTOCOL_VERSION`・`HEADER_MISMATCH`・`MISSING_REQUIRED_CLIENT_CAPABILITY` → **400**、`METHOD_NOT_FOUND` → **404**、それ以外 → 200 | SSP は常に 200。要件 3.6・3.7 は「HTTP の状態は rmcp のまま」なので差の一覧行き。クライアントが困るかは §6 R1 |
| `Host` の検査 | 既定で有効。`host_is_allowed`: 表の項目にポートが無ければどのポートでも通す。失敗は **403**・本文 `Forbidden: Host header is not allowed`・`tracing::warn!` 1 件 | 要件 4.4 を既定のまま満たす |
| `Origin` の検査 | 表が空なら検査しない。`enforce_origin_validation()` で空の表でも検査（＝全部拒む）。項目は `scheme://host[:port]` か `null`。`:*` でポートの任意、ポート無しも任意。失敗は **403**・本文 `Forbidden: Origin header is not allowed` | 要件 4.1〜4.3 を rmcp の表で表せる（`http://localhost:*`・`https://localhost:*`・`http://127.0.0.1:*`・`https://127.0.0.1:*`・`http://[::1]:*`・`https://[::1]:*` の 6 件）。**ただし help ページは rmcp の外**（§5 B） |
| `MCP-Protocol-Version` ヘッダ | 無ければ `2025-03-26` と見なす。`KNOWN_VERSIONS`（5 版）以外は **400**・`Unsupported MCP-Protocol-Version: …` | 要件 3.13。SSP の「未知の値は無状態版の検査へ」とは違う＝差の一覧 |
| 版の交渉 | `ProtocolVersion`: `V_2024_11_05`〜`V_2026_07_28` の 5 つ・`LATEST = 2026-07-28`・`LATEST_WITH_INITIALIZE = 2025-11-25`。`initialize` の既定実装は `negotiate_protocol_version` に委ねる。**未知の版で何を返すかは文書から読み切れなかった**（§6 R2） | 要件 3.2 |
| `server/discover` | `DiscoverResult { supported_versions, capabilities, instructions, server_info, _meta }`。SSP の `ttlMs`・`cacheScope`・`resultType` は無い | 要件 3.12・差の一覧 |
| `GET /api/mcp/v1` | 無状態＋event store 無しのとき許すメソッドは `POST` だけ＝GET／DELETE は **405**（`Allow: POST`） | 要件 3.11（フォーム 0 ページ）を rmcp がそのまま満たす |
| 本文の上限 | 4 MiB 超は 413 | 要件に無し。差の一覧に 1 行足してよい（任意） |
| `serverInfo` | `Implementation { name, version, title?, icons?, website_url? }`。`from_build_env()` は **コンパイルした crate の** `CARGO_PKG_NAME`／`VERSION` を使うので、名前は明示の文字列 `areka-mcp-server` を書く。版はワークスペースの `0.0.1`（`areka-mcp` も `version.workspace = true` なら areka と同じ値） | 要件 3.1 |
| ツール登録の口 | `ToolRouter<S>`: `ToolRoute { call: Arc<DynCallToolHandler<S>>, attr: Tool }`・`ToolRoute::new_dyn(attr: impl Into<Tool>, closure)`・`add_route`・`list_all()`（有効な定義を名前順で返す）・`call(ToolCallContext)`。`Tool` は name・title・description・input_schema・annotations を逐語で持てる。`#[tool_router]` マクロは要らない（`macros` 機能を切れる） | 要件 7.1〜7.4 |
| `Service` の形 | `tower_service::Service<Request<B>>`・`Response<BoxBody<Bytes, Infallible>>`・失敗しない。hyper へ渡すには `hyper_util::service::TowerToHyperService`（`hyper-util` の `service` 機能）で包む | §5 A |
| 必要な機能フラグ | `server`（既定・`schemars`＋`uuid`＋`transport-async-rw` を含む）＋`transport-streamable-http-server`（`server-side-http`＝`http`・`http-body`・`bytes`・`sse-stream`・`tower-service`・`rand`・`tokio-stream`）。`macros`・`base64`（既定）は切れる | §4 |
| 常に入る依存 | `chrono`（`serde`＋`now`）・`schemars`・`indexmap`・`futures`・`tokio`（`sync`・`macros`・`rt`・`time`・`io-util`）・`tokio-util`・`uuid`（`v4`） | §4 |

## 4. 依存の重さ（実測）

作業用の空クレートに次を書いて `cargo tree -e normal` と `cargo deny check licenses` を回した: `rmcp = "=3.5.0"`（`default-features = false`・`server`＋`transport-streamable-http-server`）、`tokio`（`rt`・`net`・`macros`・`sync`）、`hyper`（`server`・`http1`）、`hyper-util`（`tokio`）、`http-body-util`。axum の比較は機能フラグで `axum 0.8`（`http1`・`tokio`）を足した。

| 形 | 本番の依存の総数 | **`Cargo.lock` に無い新しいクレート** | ライセンスの門 |
|---|---|---|---|
| hyper を直に使う | 71 | **30**: async-trait・base64 0.23・bytes・chrono・dyn-clone・futures・futures-executor・futures-macro・futures-sink・http・http-body・http-body-util・httparse・httpdate・hyper・hyper-util・mio・pastey・ref-cast・ref-cast-impl・rmcp・schemars・schemars_derive・serde_derive_internals・socket2・sse-stream・tokio-macros・tokio-stream・tokio-util・tower-service | **緑**（`licenses ok`） |
| axum 0.8 を使う | 79 | **38**（上の 30 ＋ axum・axum-core・matchit・mime・percent-encoding・sync_wrapper・tower・tower-layer） | **緑** |

- 新しい 30 個のライセンスは全部 **MIT** か **MIT OR Apache-2.0**、rmcp 本体だけ **Apache-2.0**（表の中）。`deny.toml`・`about.toml` に足すものは 0（要件 8.3 の「承認を得てから足す」は発動しない見込み）。
- 既に lock に在るものとの版の重複: `uuid` が lock 1.26.1 に対し新規解決は 1.27.0（`^1` なので lock の 1.26.1 のまま通る見込み・重複にならない）。`hashbrown` は lock に 0.16.1 と 0.17.1 が既に両方ある。`windows-sys` は同じ 0.61.2。`syn` の 2 版の重複は今も出ている警告（`[bans] multiple-versions = "warn"`）。
- `tokio` は lock に在る 1.53.1 がそのまま使われ、x64 のビルドに初めて入る。
- 推移依存の `chrono`（rmcp が無条件に要求）は暦の読み書きの土台だが、本 spec の経路では使わない。
- **測っていないもの**: リリースの `areka.exe` の増分（`opt-level='z'`・`lto`）。設計で 1 度測って signoff に書く（§6 R5）。

## 5. 要件 → 資産の対応と選択肢

### 要件ごとの対応表

| 要件 | 既存の資産 | 足りないもの | 印 |
|---|---|---|---|
| 1 待受・終了 | `perf_thread_report` の取っ手・`spawn_actor`・`winhttp_real_tests::serve` の束ね方 | tokio ランタイム・hyper の受付ループ・畳む合図・`Drop` での後始末 | Missing |
| 2 ポートと環境変数 | `period_from_env_value` の型（`warn!`＋既定） | `AREKA_MCP_PORT` の読み解き（`0`＝止める・空白許容・範囲） | Missing（型は在る） |
| 3 MCP の応答 | 無し | rmcp の組み込み・`ServerHandler`・設定の 2 つの切替 | Missing／**未知メソッドのコードは Unknown** |
| 4 `Origin`・`Host` | 無し | 自前の純粋関数 か rmcp の許可表（help にも掛けるので自前が 1 つ要る） | Missing |
| 5 差の一覧 | `doc/ssp-mcp/survey.md` §2 の 15 行 | 実ソケットで測る道具（テストと兼ねる）・新しい 1 文書 | Missing |
| 6 help | 無し | 日本語 HTML 1 枚（実番号を埋める）・404／405 の振り分け | Missing |
| 7 登録の口 | rmcp の `ToolRouter` | rmcp の型を外へ出さない包み（`serde_json::Value` で名前・title・説明・inputSchema と結果を渡す）・テストで 1 本登録 | Missing |
| 8 依存とライセンス | `deny.toml`・`about.toml`・`tools/test-all.ps1 -License`・`areka-update/Cargo.toml` の手本 | `crates/areka-mcp/Cargo.toml`・`tech.md`／`structure.md` の登記 | Constraint（門は緑の見込み） |
| 9 テストと実機 | `TcpListener::bind("127.0.0.1:0")`・`log_capture_kit::capture`・`smoke_boot_loop_exit.rs` の型 | HTTP を手で打つ小さな助け（`winhttp_real_tests::answer` の逆向き）・signoff.md | Missing |

### 選択肢 A: HTTP の土台

| 案 | 中身 | 利点 | 難点 |
|---|---|---|---|
| **A1 hyper 1 を直に** | `tokio::net::TcpListener::from_std` で受け、接続ごとに `hyper::server::conn::http1::Builder::serve_connection(TokioIo::new(stream), svc)`。振り分けは `(method, path)` の `match` 1 つ（`POST /api/mcp/v1` → `TowerToHyperService(StreamableHttpService)`・`GET /api/mcp/help` → 自前・他 404・help に GET 以外 405） | 新しいクレート 30 で最小。振り分けが 3 分岐しか無いのでルータの道具が要らない | 受付ループ（accept → spawn）を自分で書く（10 行ほど） |
| A2 axum 0.8 | `Router::new().route_service("/api/mcp/v1", svc).route("/api/mcp/help", get(help))`＋`axum::serve` | 受付ループと振り分けが既製 | 新しいクレート +8（38）・告知の一覧も伸びる。rmcp の例は axum だが本 spec の規模では利点が薄い |
| A3 hyper-util の `auto`（HTTP/1＋2） | A1 の http1 を auto に | HTTP/2 も通る | クライアントは HTTP/1.1 で来る。要らない |

### 選択肢 B: `Origin`／`Host` の検査

| 案 | 中身 | 利点 | 難点 |
|---|---|---|---|
| B1 rmcp の表だけ | `with_allowed_origins([6 件]).enforce_origin_validation()`＋`Host` は既定 | 書く量が最小 | **help ページは rmcp の外なので検査が掛からない**（要件 6.3 に反する）。要件 4.5 の「生の値を使わない判断を決定論テストで固定」は rmcp の関数が私有なので実ソケット越しにしか測れない。`warn!` の文面と target は rmcp のもの |
| **B2 自前の純粋関数 1 つを両方のパスの前に** | `fn gate(origin: Option<&str>, host: Option<&str>) -> Result<(), Reject>` を振り分けの手前で呼び、拒んだら 403＋`warn!`（値つき・要件 4.3）。rmcp の `Host` 検査は既定のまま残す（二重でも害は無い。`Origin` の表は空のまま＝rmcp 側は検査しない） | 要件 4.3（`warn!` に値）・4.5（純粋関数のテスト）・6.3（help にも同じ検査）を 1 か所で満たす。rmcp の版上げで振る舞いが動かない | rmcp と同じことを 20 行ほど書く。`Origin` の読み方（`null`・host 無し・`[::1]`）は自分で決めて固定する |
| B3 両方 | B2 ＋ rmcp の表も入れる | 二重の守り | 拒む理由の `warn!` が 2 件出る経路ができる（要件 4.3 は 1 件） |

### 選択肢 C: tokio の閉じ込め方と畳み方

| 案 | 中身 | 利点 | 難点 |
|---|---|---|---|
| **C1 `spawn_actor("mcp")` の中で `current_thread`** | 呼び出し側で `std::net::TcpListener` を束ね（失敗はここで `error!`・要件 1.3／9.3）、非ブロッキングにして actor のスレッドへ渡し、そこで `Builder::new_current_thread().enable_io().enable_time()` を作って `block_on(受付ループ)`。畳む合図は `tokio::sync::oneshot` か `CancellationToken`（rmcp の設定にも渡す）。終わりは `runtime.shutdown_background()`（開いた接続を待たない＝要件 1.8） | 名簿に `actor:mcp` で載る（要件の adjacent expectation）・tokio の型は crate の外へ出ない・束ねの失敗が同期に分かるので `start()` が実番号を返せる | `spawn_blocking` を将来使うと blocking pool のスレッドが名簿に載らない（`mcp-tool-entrances` で決める） |
| C2 `std::thread` 直 | C1 と同じで `spawn_actor` を使わない | 依存が 1 つ減る | 名簿に載らず `unregistered_rest` に出る（要件が名簿を求める） |
| C3 multi_thread ランタイム | — | — | 名簿に載らないスレッドが増える（brief が却下済み） |

### 選択肢 D: `fn main()` への配線（2 か所）

| 案 | 中身 | 利点 | 難点 |
|---|---|---|---|
| **D1 `Drop` で畳む取っ手** | `let _mcp = areka_mcp::start(areka_mcp::port_from_env(...));` を `boot_first_ghost` より前（たとえば `perf_report` の直後）に置く。取っ手の `Drop` が合図を送って畳み、`info!` を 1 件残す（要件 1.9）。`main` のどの `return` でも畳まれる | **`down?` の早い戻りの罠を踏まない**・main.rs へ足すのは 1〜2 行・取っ手を持ち回る必要が無い | `Drop` の中で待てる時間は上限を切る（perf の `FINAL_WAIT` と同じ考え） |
| D2 perf と同じく閉包の最後で明示に畳む | `finish_after_run` の閉包の `down?` より前に `stop()` | 前例と同じ形 | 早い戻りの罠は位置で避けるしかない。閉包へ値を運ぶ分だけ行が増える |
| D3 `register_systems` の後に World の NonSend へ入れる | 取っ手を World に預ける | 他の結線と同じ置き場 | 畳む時機が World の drop に縛られる。MCP は World を知らなくてよいので過剰 |

`start()` の失敗（束ねられない）は `error!` を残して「待ち受けていない取っ手」を返す形にすれば、呼び出し側の分岐が 0 で済む（要件 1.3・1.4）。`AREKA_MCP_PORT=0` も同じく「待ち受けていない取っ手」（`info!`・要件 2.4）。

### 選択肢 E: ツール登録の口の形（要件 7）

| 案 | 中身 | 利点 | 難点 |
|---|---|---|---|
| **E1 自前の小さな型で受け、中で `ToolRoute::new_dyn` に写す** | `ToolSpec { name, title, description, input_schema: serde_json::Value }`＋`handler: Arc<dyn Fn(serde_json::Value) -> ToolOutcome + Send + Sync>`（`ToolOutcome { content: Vec<Content>, is_error }`・`Content` は text／image の 2 種）。`ToolRegistry` に `register(spec, handler)` | rmcp の型が外へ出ない（brief の境界）・後続 spec は `tools-list-ssp-2.9.05.json` の定義を逐語で詰められる・`serde_json` は既存依存 | handler が同期（`Fn`）だと、後続で World へ問い合わせる間 tokio のスレッドが止まる＝1 本ずつしか処理できない。非同期にするなら `Pin<Box<dyn Future>>` を返す形にして今から決めておく（§7 議題 5） |
| E2 rmcp の `Tool`／`ToolRoute` をそのまま公開 | — | 包みが 0 | rmcp の版上げが後続 10 spec の全部に波及する |
| E3 `#[tool_router]` マクロ | — | 短い | 逐語の定義（SSP の inputSchema）を詰めにくい・`macros` 機能と `rmcp-macros` が要る |

### 選択肢 F: 要件 5 の「測る」道具

- 決定論テスト（要件 9.2）がそのまま 15 行の実測器になる。差の一覧（`doc/ssp-mcp/<新文書>.md`）にはテスト名を書く（要件 5.3）。別の計測スクリプトは作らない（テストと一覧の二重化を避ける）。

### 推奨の組み合わせ

**A1 ＋ B2 ＋ C1 ＋ D1 ＋ E1。** 新しいクレートは 30、自前で書くのは「受付ループ・3 分岐の振り分け・`Origin`／`Host` の純粋関数・ポートの読み解き・登録の包み・help の HTML」の 6 片で、どれも 100 行に満たない見込み。

## 6. 設計へ送る調べ残し（Research Needed）

| # | 何を調べるか | なぜ要るか | 調べ方 |
|---|---|---|---|
| R1 | **Claude Code（TS SDK の `StreamableHTTPClientTransport`）が、HTTP 400／404 に載った JSON-RPC エラー本文を「要求のエラー」として受け取るか、輸送の失敗として投げるか** | rmcp は `-32602` を 400・`-32601` を 404 で返す（SSP は 200）。本 spec の登録・`initialize`・`tools/list`・`ping` には出ないが、`mcp-tool-entrances` のエラーの見え方が変わる。要件 5.2 の「困る」の物差しに当たるかの判定材料 | TS SDK の `src/client/streamableHttp.ts` を読む（今回は GitHub の場所が動いていて読めなかった）か、実機で `tools/call` に未知の名前を打って Claude Code の表示を見る |
| R2 | rmcp の `negotiate_protocol_version` が**未知の版**に何を返すか（`LATEST`＝`2026-07-28` か `LATEST_WITH_INITIALIZE`＝`2025-11-25` か） | 要件 3.2 は「rmcp が決め、一覧に書く」なので設計の障害ではないが、差の一覧の 1 行に書く値 | 決定論テスト（`1999-01-01` を送る）で測る |
| R3 | 未知のメソッドの実際のコード（`-32600` か `-32601` か）と、`Origin` に host の無い値・`null`・`http://localhost.evil.example` を送ったときの rmcp 側の扱い | 要件 3.7・4.3 の文面の裏付け。B2（自前）なら `Origin` は自分で決められる | 同上 |
| R4 | `Drop` の中で `shutdown_background()` を呼んだとき、束ねたソケットが OS に返るまでの時間（終了直後に同じポートへ接続できないこと＝要件 1.7・9.4） | Windows では `TIME_WAIT` と `SO_REUSEADDR` の挙動が Linux と違う。「接続できない」の判定が安定するか | 決定論テストで畳んだ直後に `TcpStream::connect` を打つ |
| R5 | リリースの `areka.exe` の増分（配布 zip の大きさ） | 配布物の説明（`dist/README.txt`）と signoff に書く | `tools/package-alpha.ps1` の前後で比べる |

## 7. 要件ディスカッションの議題（答えで作業が変わるもの）

1. **要件 3.7「未知のメソッドは `-32601`」の文面。** rmcp 3.5.0 は知らないメソッド名を本文の読み解きの段で弾き、**`-32600`（Invalid Request）・HTTP 400** を返す見込み（`-32601` の経路は見当たらない）。選べるのは ⑴ 文面を「JSON-RPC のエラー（コードは rmcp のまま＝差の一覧に書く）」へ緩める、⑵ areka の HTTP 層で本文の `method` を先に覗いて、知らない名前なら自分で `-32601` を返す（メソッドの表を areka が持つ＝rmcp の版上げで表がずれる危険）。推奨は ⑴（クライアントはどちらでも「失敗」としか見ない）。
2. **`Origin`／`Host` の検査を自前にするか（B2）、rmcp の表に任せるか（B1）。** help ページには rmcp の検査が掛からないので、要件 6.3 を満たすには自前が最低 1 か所要る。推奨は B2（自前 1 つを両方のパスの手前に置き、rmcp の `Host` 既定は残す）。これで要件 4.5 の純粋関数のテストも成り立つ。
3. **HTTP の土台は hyper を直に（A1・新規 30）か axum（A2・新規 38）か。** 振り分けは 3 分岐なので推奨は A1。
4. **畳み方は `Drop` の取っ手（D1）か、perf と同じ閉包の明示の畳み（D2）か。** `down?` の早い戻りの罠を構造で避けられる D1 を推奨。main.rs への追加は 1〜2 行。
5. **登録の口の handler を同期にするか非同期にするか（E1 の形）。** 本 spec のツールは 0 本だが、後続がここへ詰める。同期（`Fn(Value) -> ToolOutcome`）だと tokio のスレッドが問い合わせの間止まり、要求が 1 本ずつになる（要件 1.5 は「他の接続の要求に答える」を求めるが、ツール 0 本の今は影響しない）。非同期（`Pin<Box<dyn Future>>` を返す）にしておけば後続が `spawn_blocking`（名簿に載らないスレッドが増える）か async の返事口のどちらでも選べる。推奨は非同期の形で口を切っておき、本 spec のテストの 1 本は `ready()` で返す。
6. **`macros`・`base64` の既定機能を切るか。** `default-features = false` で `server`＋`transport-streamable-http-server` だけにすると `rmcp-macros` が入らない（登録は `ToolRoute::new_dyn` で足りる）。推奨は切る（依存 30 の数字はこの形で測った）。
7. **`dist/README.txt` に `AREKA_MCP_PORT` を書くか。** 要件 2.7 は書いてよい場所に挙げるが、roadmap の C1 の行（⑦ の触るファイル）には無く、C2・C3・C4 の配布の spec が分け合う。推奨は本 spec では書かず help だけに載せ、`mcp-stdio-bridge`（Desktop の設定例を足す回）でまとめて書く。
8. **要件 3.8 の「JSON でない本文」の期待を 2 段に分けるか。** rmcp は `Content-Type` が違えば 415、JSON として壊れていれば 400・`-32600`。要件は「4xx か `-32700`」で両方を許すので変えなくてよいが、決定論テストは 2 本（415 と 400）に分けると差の一覧が正確になる。
9. **（任意）本文の上限 4 MiB を差の一覧に載せるか。** SSP の上限は未測定。載せるなら「違うが困らない」の行が 1 つ増える（要件 5.1 の 15 行の外）。

## 8. 規模とリスク

- **規模: M（brief の見立て 11〜14 タスクと同じ）。** 新しい土台の組み込み（tokio・hyper・rmcp）と、15 行の実測と文書、実機の 4 項目があるため S ではない。自前で書く量は少ない。
- **リスク: 中。** 根拠 ⑴ rmcp の無状態の経路は文書と docs.rs のソース表示で確かめたが、**手元で動かしてはいない**（最初の決定論テストで裏を取る）。⑵ 未知メソッドのコード（議題 1）と Claude Code の 400／404 の扱い（R1）が未確定。⑶ ライセンスの門は実測で緑なので、依存の面の不確かさは低い。⑷ 終了の経路は `Drop` にすれば構造で守れる。

## 9. 次の段へ

- 要件ディスカッションで §7 の 1・2・4・5・7 を決める（3・6・8・9 は設計で決めてよい）。

## 10. 要件ディスカッションの結果（2026-10-03）

### 要件へ当てた自明な修正（§7 の 1・7・8 はここで片付いた）

- §7-1（未知メソッドのコード）: 要件 3.7 を「JSON-RPC のエラー・コードは rmcp のまま・測った値を差の一覧に書く」へ緩めた。areka がメソッドの表を持つ案は取らない（brief「細部は rmcp に従う」のとおり）。
- §7-7（`dist/README.txt`）: 要件 2.7 から外した。本 spec は help だけに載せ、`mcp-stdio-bridge` が README に書く（要件の Adjacent expectations にも書いた）。
- §7-8（JSON でない本文）: 要件 3.8 を 415／400 の 2 段で書き、要件 9.2 のテストを 2 本に分けた。
- ほか: rmcp のライセンスは MIT でなく **Apache-2.0**（要件の Introduction を直した）・`AREKA_MCP_PORT` の UTF-8 でない値を要件 2.5 に足した・`MCP-Protocol-Version` の未知の値（400）を要件 3.13 と 9.2 に足した。

### 開発者が確定した議題（2 件）

- 議題 1: 既定ポート **9821・既定で有効・待受の失敗は記録だけ**（別のポートは試さない）＝要件 2.1・2.2・1.3 のまま。
- 議題 2: `serverInfo` の名前は **`areka-mcp-server`**・版は areka の Cargo の版＝要件 3.1 のまま。

### 設計へ送る判断（`/kiro-spec-design` で決める）

| # | 判断 | 本文書の推奨 | 要件側の縛り |
|---|---|---|---|
| B-1 | `Origin`／`Host` の検査を自前（B2）か rmcp の表（B1）か | B2（自前 1 つを両方のパスの手前に・rmcp の `Host` 既定は残す） | 要件 4.3（`warn!` に値）・4.5（純粋関数のテスト）・6.3（help にも同じ検査）を満たすこと |
| B-2 | HTTP の土台（A1 hyper 直／A2 axum） | A1 | 要件 8（新しいクレートの数と告知の量が変わる） |
| B-3 | 畳み方（D1 `Drop`／D2 閉包の明示） | D1 | 要件 1.7〜1.9・`main.rs` へ足すのは数行（brief） |
| B-4 | 登録口の handler を同期にするか非同期にするか（E1） | 非同期の形で口を切り、本 spec のテストの 1 本は `ready()` | 要件 7.4（逐語で渡せる形）・1.5（他の接続を止めない）・後続 `mcp-tool-entrances` が従う |
| B-5 | rmcp の既定機能 `macros`・`base64` を切るか | 切る（`server`＋`transport-streamable-http-server`） | 要件 8.2（版の固定） |
| B-6 | `tools/call` の `-32602` は `ToolRouter` を通す形でだけ出る（既定の `call_tool` は `-32601`） | `ToolRouter` を通す | 要件 3.6・7.3 |
| B-7 | 本文の上限 4 MiB を差の一覧に載せるか（任意） | 載せる（「違うが困らない」1 行） | 要件 5.1 の 15 行の外 |
| R1〜R5 | §6 の調べ残し | 設計の最初のタスク（空の `areka-mcp`＋`initialize` 5 版→`ping` のテスト）で R2・R3・R4 を測る | — |
- 設計の最初のタスクは「`areka-mcp` を空で建てて `cargo deny check` と `-License` を通し、`initialize`（5 版）→`ping` の決定論テストを 1 本緑にする」＝§6 の R2・R3 がここで同時に測れる。

## 11. 設計の段の調べと決定（2026-10-03・`/kiro-spec-design`）

### 11.1 調べの範囲
- **軽い発見（Extension）**: 既存の前例の確認（§2 のとおり・変更なし）と、**rmcp 3.5.0 の現物**（手元の cargo レジストリ `rmcp-3.5.0`・`hyper-util-0.1.21` のソース）の読み直し。§3 は docs.rs の表示から書いたので「見込み」が 3 か所あった。現物で引き直した結果を下に書き、設計はこちらに従う。
- Web の調べは行っていない（依存の版・ライセンスは §4 で実測済み。rmcp は現物で読めた）。

### 11.2 §3 の「見込み」を現物で引き直した点（設計の根拠）

| 項目 | §3 の記述 | 現物（`rmcp-3.5.0`） | 設計への含み |
|---|---|---|---|
| 未知のメソッド | 本文の読み解きで弾かれ **400＋`-32600`** の見込み | 知らないメソッド名は `ClientRequest::CustomRequest` として**読める**。`ServerHandler::on_custom_request` の既定が **`-32601`**（`handler/server.rs`）。旧式の経路では HTTP 200 | 要件 3.7 はそのまま満たす（SSP と同じ 200・`-32601`）。areka がメソッドの表を持つ必要は無い |
| 壊れた JSON | 400＋`-32600` | `expect_json`（`transport/common/server_side_http.rs`）は `serde_json::from_slice` の失敗を **415**・平文 `fail to deserialize request body …` で返す（同ファイルのテスト `expect_json_returns_415_for_invalid_json_under_limit` が固定している）。`Content-Type` 違いも 415 | 要件 3.8 の括弧書き「400 と JSON-RPC のエラー」は現物と違う。主文「4xx で答え、落とさない」は満たす。**設計ディスカッションで要件 3.8 の括弧書きを 1 行直す**（設計の判断 B-10） |
| JSON-RPC エラーのときの HTTP 状態 | `-32602` は 400・`-32601` は 404 | `jsonrpc_http_status` が効くのは `serve_negotiated_request_directly`（本文の `_meta` に版が入る **2026-07-28 の per-request 経路**と `server/discover`）だけ。旧式の 4 版の要求は `handle_post` の無状態の腕で `json_response` のとき **`StatusCode::OK` 固定** | 今の Claude Code・Cursor が使う経路では SSP と同じ 200。R1 の大半はここで解けた（残るのは 2026-07-28 の経路の 400／404＝差の一覧に書く） |
| `Accept` ヘッダ（§3 に無かった事実） | — | `handle_post` の先頭で `Accept` に `application/json` **と** `text/event-stream` の両方が無ければ **406**。素の `curl`（`Accept: */*`）は 406 | 差の一覧に 1 行（違うが困らない）。signoff と差の一覧の `curl` 例は `-H "Accept: application/json, text/event-stream"` を付ける（設計の判断 B-8） |
| 未知の版の交渉（R2） | 文書から読み切れず | `negotiate_protocol_version`（`service/server.rs`）: 要求の版が `initialize` を持ち（`2026-07-28` 未満）かつ対応表に在ればそれ、無ければ `get_info` の `protocol_version`（既定 `LATEST`＝`2026-07-28`＝`initialize` を持たない）→ `initialize` を持つ最新＝**`2025-11-25`** | SSP と同じ値になる見込み。テスト `initialize_unknown_version_falls_back` で測って差の一覧へ |
| `server/discover` の形 | `ttlMs`・`cacheScope`・`resultType` は無い | `DiscoverResult::from_server_info` は `result_type: complete`・`ttl_ms: 0`・`cache_scope: private`・`_meta` に serverInfo を**載せる** | 差の一覧の行を「SSP は `ttlMs: 3600000`、areka は `0`」の形で書く（違うが困らない） |
| `ToolRouter` の API | `new_dyn`・`add_route`・`list_all`・`call` | 同じ。`call` の未登録は `invalid_params("tool not found")`＝`-32602`。`new_dyn` の閉包は `Result<CallToolResponse, ErrorData>` を返し、`CallToolResult` は `From` で `CallToolResponse::Complete` へ写せる（`model/mrtr.rs`） | B-6 の裏付け |
| `get_info` の型名 | `ServerInfo` | 3.5.0 では `ServerConfig`（`InitializeResult` の別名。`ServerInfo` は非推奨の別名） | 設計は `ServerConfig` で書いた |
| `Service` の呼び方 | `TowerToHyperService` で包む | `StreamableHttpService` は `Clone` で `poll_ready` が常に Ready。`service_fn` の中で複製して `tower_service::Service::call` を直に呼べる（`hyper-util` の `service` 機能は要らない） | 直接の依存に `tower-service`（新しいクレートは増えない） |

### 11.3 設計の判断（B-1〜B-7 の結論と、現物で増えた B-8〜B-10）

| # | 判断 | 結論 |
|---|---|---|
| B-1 | `Origin`／`Host` の検査 | 自前の純粋関数 `gate::check` を振り分けの手前に 1 つ。rmcp の `Host` 既定は残し `allowed_origins` は空のまま（二重に鳴らない＝自前が先に拒む） |
| B-2 | HTTP の土台 | hyper 1 を直に（http1 のみ・`TokioTimer` を設定・受付ループ＋3 分岐の `match`） |
| B-3 | 畳み方 | 取っ手 `McpServer` の `Drop`。`fn main()` では `resolve_boot` の直後・`WinApp` 構築の前に `let _mcp = areka_mcp::start(…)` の 1 行 |
| B-4 | 登録口の handler | 非同期（`Arc<dyn Fn(Value) -> Pin<Box<dyn Future<Output = ToolOutcome> + Send>> + Send + Sync>`） |
| B-5 | rmcp の既定機能 | 切る（`server`＋`transport-streamable-http-server`） |
| B-6 | `tools/call` の `-32602` | `ToolRouter::call` へ委ねる |
| B-7 | 4 MiB | 差の一覧に 1 行。上限の定数は `MAX_BODY_BYTES` 1 つ（`dispatch` の読み取りと rmcp の設定の両方へ） |
| B-8 | `Accept` の 406 | 直さない・差の一覧に 1 行・`curl` 例にヘッダを付ける |
| B-9 | JSON-RPC エラーの HTTP 状態 | 旧式の経路は 200（同じ）・2026-07-28 の経路は 400／404（違うが困らない）と書き分ける |
| B-10 | 要件 3.7・3.8 と現物 | 3.7 はそのまま（`-32601`）。3.8 の括弧書きは現物（415・平文）に合わせて設計し、要件の文面は設計ディスカッションで直す |

### 11.4 統合の 3 つの見方（design-synthesis）
- **一般化**: 「環境変数を読まない純粋な判断＋読む 1 か所」（port）と「生の値を使わない純粋な判断」（gate）は同じ型で、どちらも `period_from_env_value` の前例の写し。新しい抽象は作らない。
- **作るか採るか**: プロトコル・無状態の経路・`Host` 検査・版の交渉・`ToolRouter` は rmcp を採る。自前で書くのは「受付ループ・3 分岐・`Origin`／`Host` の判断・ポートの読み解き・登録の包み・help の HTML」の 6 片（各 100 行未満）。axum（+8 クレート）・reqwest／ureq（テストの HTTP 用）は採らない。
- **簡素化**: ルータの道具・走行中のツールの追加削除・設定の型（`McpConfig` のような構造体）・HTTP/2・`spawn_blocking` は入れない。`spawn_actor` の inbox は使わない（合図は `CancellationToken` 1 つ）。

### 11.5 §6 の調べ残しの状態
| # | 状態 |
|---|---|
| R1 | **大半は解けた**（11.2 の 3 行目）。残り＝2026-07-28 の per-request 経路で Claude Code が 400／404 をどう見せるか。本 spec の 4 操作には出ない。差の一覧に「未測」と書く |
| R2 | 現物の読みで `2025-11-25` の見込み。テストで測る |
| R3 | 未知メソッドは `-32601`（解けた）。`Origin` の `null`・host 無し・`localhost.evil.example` は自前の `gate` が拒む（rmcp は関与しない） |
| R4 | 受付の口（listener）を `block_on` の直後に落とす設計で「終了直後の `connect` が失敗する」の判定を安定させる見込み。テスト `drop_closes_port` で確かめる |
| R5 | 実装の段で測って signoff.md に書く |

### 11.6 残るリスク
- rmcp の無状態＋`json_response` の経路を**手元で動かすのは実装の最初のタスク**（空の `areka-mcp`＋`initialize` 5 版→`ping`）。ここで赤なら設定の組み方を直す（設計の形は変えない見込み）。
- hyper の http1 のヘッダ読みの時間切れは `TokioTimer` を設定して有効にする（設定しないと時間切れが効かないか、版によっては失敗する）。
- `Drop` の中の待ち（`SHUTDOWN_WAIT` 2 秒）は perf の `FINAL_WAIT` と同じ考え。待ちきれなければ `warn!` で切り離す。

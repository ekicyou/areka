# 設計レビュー: areka-P0-mcp-server-core

> 2026-10-03・本ブランチ（`f7de5767`・設計生成の直後）。対象は `design.md`・`requirements.md`・`research.md`（§10・§11）・`brief.md`・`doc/ssp-mcp/survey.md`・steering（`roadmap.md` C1 の行 ⑦・`tech.md`・`structure.md`・`logging.md`）。設計の主張のうち安く確かめられるものは、実際のコード（`crates/areka/src/main.rs`・`perf_thread_report.rs`・`crates/areka-actor/src/spawn.rs`・`crates/log-capture-kit`・`deny.toml`・`about.toml`・`tools/test-all.ps1`）と、手元の cargo レジストリにある **rmcp 3.5.0 の現物**（`rmcp-3.5.0/src/…`）で裏を取った。非対話で行ったので、開発者への質問は無い。

## レビューの要約

設計は「葉クレート `areka-mcp` ＋ 取っ手 `McpServer` の `Drop` で畳む」という小さな形にまとまっており、既存の前例（`perf_thread_report` の上限つきの待ち・`spawn_actor` の名簿・環境変数を読まない純粋な判断）をそのまま写している。rmcp 3.5.0 の現物を読み直して、ギャップ分析の「見込み」3 点（壊れた JSON は 415・未知メソッドは `-32601`・旧式の経路の HTTP 状態は 200）を正した点は確かで、こちらでも同じ箇所を読んで一致を確かめた。**ただし 1 点、要件 3.1「5 版はいずれも要求どおり」が rmcp の現物と食い違う**（`2026-07-28` を要求した `initialize` は `2025-11-25` に倒れる）。設計の handler 節はこの事実を知っているのに、追跡表とテスト表は要件の文面のまま書かれており、このまま実装するとテストが 1 本赤になる。残りは実装の段で片付く規模の指摘で、設計の形を変えるものは無い。

## 裏を取った主張（設計の記述 → 実物）

| 設計の主張 | 確かめた場所 | 結果 |
|---|---|---|
| `StreamableHttpServerConfig` の既定（セッション有り・JSON 単発でない・`allowed_hosts` は `localhost`／`127.0.0.1`／`::1`・`allowed_origins` 空・本文 4 MiB）・`with_cancellation_token` が在る | `rmcp-3.5.0/src/transport/streamable_http_server/tower.rs` の `impl Default for StreamableHttpServerConfig`・`with_cancellation_token` | 一致 |
| `Accept` に `application/json` と `text/event-stream` の両方が無い `POST` は 406（B-8） | 同ファイル `handle_post` の先頭 | 一致（`header.contains(JSON) && header.contains(EVENT_STREAM)`） |
| `Content-Type` 違いと壊れた JSON は 415・平文（B-10） | `handle_post` の `Content-Type` 検査・`transport/common/server_side_http.rs` のテスト `expect_json_returns_415_for_invalid_json_under_limit` | 一致 |
| 未知メソッドは `on_custom_request` の既定で `-32601` | `handler/server.rs` の `fn on_custom_request`（`ErrorCode::METHOD_NOT_FOUND`） | 一致 |
| `ToolRouter::call` の未登録は `invalid_params("tool not found")`＝`-32602`（B-6） | `handler/server/router/tool.rs` の `pub async fn call` | 一致 |
| 無状態＋JSON 単発の腕は HTTP 200 固定（B-9） | `tower.rs` の無状態の腕（`NegotiatingStatelessHttpService` → `json_response` → `StatusCode::OK`） | 一致 |
| rmcp の `Host` 検査は `[::1]` の角括弧を外して比べる（自前の gate と二重に鳴らない） | `tower.rs` の `fn normalize_host` | 一致 |
| `fn main()` は 946 行・`down?` の早い戻りが `stop_and_report_final` の前に在る | `crates/areka/src/main.rs` の `finish_after_run` の閉包 | 一致（1 行＋コメントを足しても 1,000 行の内側） |
| `_mcp` を `app` より先に宣言すれば `app` の後に落ちる | `crates/wintf/src/runtime/mod.rs` の `pub fn run(&self)`（`app` は `main` の末尾まで生きる） | 一致（宣言の逆順で落ちる） |
| 根の `Cargo.toml` は触らない | 根の `Cargo.toml` の `members = ["crates/*"]` | 一致（新クレートは自動で入る） |
| `spawn_actor` の取っ手は落としても join しない（Drop の待ちが無限にならない） | `crates/areka-actor/src/spawn.rs` の冒頭（「非 RAII（detach）な join ハンドル」） | 一致 |
| `log_capture_kit::capture` は呼び出しスレッドだけ | `crates/log-capture-kit/src/capture.rs` の `pub fn capture` の doc | 一致。全スレッド捕捉 `install_global_capture_all` も在るが、番人の例外表への登記が要る（`tests/with_default_guard_test.rs` の `every_all_thread_capture_exception_still_has_a_real_hit`） |
| hyper 1 の http1 は `timer()` を持ち、ヘッダ読みの時間切れの既定は 30 秒 | `hyper-1.11.1/src/server/conn/http1.rs` の `pub fn timer`・`h1_header_read_timeout: Dur::Default(Some(30s))` | 一致（`TokioTimer` を設定する判断は正しい） |
| ライセンスの門は `-License` で `cargo deny check`＋`cargo about generate`・許可は 9 つ | `tools/test-all.ps1`・`deny.toml`・`about.toml` | 一致 |
| **`initialize` は 5 版とも要求どおりの版を返す（要件 3.1・テスト `initialize_returns_requested_version_for_each_of_five`）** | `rmcp-3.5.0/src/service/server.rs` の `fn negotiate_protocol_version`・`model.rs` の `has_initialize`（`V_2026_07_28` は偽） | **食い違い**（下の指摘 1） |

## 重大な指摘（最大 3 件）

### 指摘 1: 要件 3.1 の「5 版とも要求どおり」は rmcp の現物と食い違う（`2026-07-28` は `2025-11-25` に倒れる）

- **問題**: rmcp 3.5.0 の `negotiate_protocol_version` は「要求の版が `initialize` を持ち、かつ対応表に在るときだけ要求どおり」を返す。`2026-07-28` は `initialize` を持たない版（`ProtocolVersion::NO_INITIALIZE`）なので、`initialize` で `2026-07-28` を要求すると `2025-11-25`（`LATEST_WITH_INITIALIZE`）が返る。設計の handler 節は「既定の `protocol_version` が `2026-07-28`＝`initialize` を持たない版」と事実を書いているが、追跡表（3.1）と統合テストの表（`initialize_returns_requested_version_for_each_of_five`「5 版で `protocolVersion` が要求どおり」）は要件 3.1 の文面のままになっている。
- **影響**: このまま実装するとテストが 1 本赤になり、要件 3.1 を満たせない。rmcp に手を入れない方針（Out of scope）なので、areka 側で直す道は無い。Claude Code・Cursor は旧式の版で `initialize` するので利用者は困らない（「違うが困らない」の差）。
- **提案**: B-10 と同じ扱いで設計ディスカッションの議題にし、要件 3.1 を「旧式 4 版（`2025-11-25`・`2025-06-18`・`2025-03-26`・`2024-11-05`）は要求どおり。`2026-07-28` は `initialize` を持たない版なので rmcp が `2025-11-25` へ倒す（差の一覧に書く）」へ改める。テストは `initialize_returns_requested_version_for_each_of_four` と `initialize_with_no_initialize_version_falls_back_to_2025_11_25` の 2 本に分け、追跡表の 3.1・3.2 と差の一覧の「版の交渉」の行を揃える。survey §2 の SSP の行（「既知の版を渡せばその版を返す」）との差もその行に書く。
- **要件**: 3.1・3.2・5.1・9.2
- **根拠**: design.md「MCP › handler」（版の交渉の段落）・「Requirements Traceability」3.1・「Integration Tests」の表、research.md §11.2「未知の版の交渉（R2）」

### 指摘 2: 統合テスト 30 本を `server_tests.rs` 1 ファイルに置く計画は 1,000 行の番人の射程

- **問題**: 「Integration Tests」の表は 30 本すべてを `server_tests.rs` に置く。1 本 20〜30 行として 600〜900 行で、手書きの HTTP の往復や 5 版の繰り返しを含むと上限 1,000 行に届きうる。設計に分割の計画が無く、番人（`crates/log-capture-kit/tests/file_length_guard_test.rs`）の例外表はどの spec も触れない約束。
- **影響**: 実装の最後に `tools/test-all.ps1` が赤になり、その場しのぎの分割で名前の規則（`structure.md` の `<stem>_<テーマ>.rs`）を崩しやすい。
- **提案**: 最初からテーマで 3 つに分ける: `server_tests.rs`（待受・束ねの失敗・終了＝要件 1・2・9.3・9.4）・`server_protocol_tests.rs`（`initialize`〜`server/discover`・`Accept`・本文の上限＝要件 3・5）・`server_gate_help_tests.rs`（`Origin`／`Host`・help・404／405・登録 1 本の往復＝要件 4・6・7）。共有の部品は `testkit.rs`（`areka-update` の前例・`structure.md` に「テスト専用の部品」として登記済み）で足りる。接続宣言は `#[cfg(test)] #[path = "…"] mod …;` の形。
- **要件**: 9.2（テストの置き場）・steering `structure.md`「1 ファイル 1,000 行以下の目安は本番ファイル・テストファイルの双方に適用」
- **根拠**: design.md「File Structure Plan」（`server_tests.rs` 1 行）・「Integration Tests」の表（30 行）

### 指摘 3: mcp スレッドの中の失敗経路の書き方が 3 か所で薄い

- **問題**: ⑴ `accept` の失敗は「`warn!` を残して続ける」だけ。持続する失敗（ハンドル枯渇など）だと間を置かずに回り続け、`warn!` の嵐と CPU の空回りになる。⑵ 接続ごとの `serve_connection` が `Err` を返す場合（クライアントの切断・不正な HTTP）の記録の方針が無い（「ログ無しの失敗経路を作らない」と「雑音を増やさない」のどちらに倒すか未記載）。⑶ `Drop` は `done_rx.recv_timeout` の `Err` を一律「待ちきれず切り離す」の `warn!` にしているが、ランタイムの作成に失敗してスレッドが早く終わると `done_tx` が落ちて `Disconnected` が即座に返り、「待ちきれず」の文言が実態と違う。
- **影響**: ⑴ は実機で稀にしか起きないが起きると記録が読めなくなる。⑵ は実装者ごとに判断が割れ、レビューで戻る。⑶ は終了の記録が誤解を招く（要件 1.9 の `info!` の意味がぶれる）。
- **提案**: ⑴ `accept` の失敗は `warn!` のあと `tokio::time::sleep`（`time` 機能は既に入れる）で短く（100 ms 程度）待ってから続ける。⑵ 接続の `Err` は `debug!` 1 件に固定し、`dispatch` の `debug!` と同じ `areka_mcp` の target で読めるようにする（`error!`／`warn!` にはしない）。⑶ `RecvTimeoutError::Disconnected` は「スレッドは既に終わっている」として `info!`（閉じた）側へ倒し、`Timeout` だけを `warn!` にする。いずれも server.rs の中の数行で、公開の契約は変わらない。
- **要件**: 1.3・1.8・1.9・steering `logging.md`（レベルの選び方）・開発規律「ログ無しの失敗経路の禁止」
- **根拠**: design.md「待受 › server」の Responsibilities（`accept` の失敗・畳み）・「Error Handling › Error Strategy」

## 重大でないが設計ディスカッションで触れてよい点

- **干渉台帳と roadmap C1 の行 ⑦**: 設計が触るコードのファイルは roadmap の許可（新規 `crates/areka-mcp/`・`crates/areka/Cargo.toml` 1 行・`main.rs` の `fn main()`・`Cargo.lock`・`THIRD-PARTY-NOTICES.md`）の内側で、`main.rs` は「2 か所」の許可に対し 1 か所（`Drop` で畳むため）。許可の表に無いのは `.kiro/steering/tech.md`・`structure.md`・新規 `doc/ssp-mcp/transport-diff-areka.md`・spec 自身の `verification/signoff.md` だが、前 2 つは要件 8.5、3 つ目は要件 5.1 が求めるもので、C1 の他 spec と同じ節を触る見込みは無い（`emo-text-file-split` が `structure.md` の emo-text の節を直す可能性はあるが、節が違えば git が併合する）。致命ではない。
- **`dispatch::handle` の状態の渡し方**: 「`service_fn(dispatch::handle)`」と書かれているが、`handle` は実番号（help 用）・rmcp の `StreamableHttpService`・`CancellationToken` を要る。他の部品は Rust の署名を載せているのに dispatch だけ無い。タスク生成の前に `pub(crate) fn handle(state: Arc<State>, req: Request<Incoming>) -> …` の形を 1 行書いておくと実装者が迷わない。
- **要件 4.3 の `warn!`・3.14 の `debug!` を常時テストで数えない選択**: mcp スレッドで出るので `capture` は見えない。`install_global_capture_all` は在るが、番人の例外表への登記と「同じテストバイナリの `enabled!` が全部真になる」の両立条件が要るので、実機（要件 9.6）へ回す判断は妥当。設計の「別スレッドのイベントは常時テストでは数えない」の 1 文に、この窓口を使わない理由（例外表）を添えると後で問われない。
- **`read_port_env_non_unicode_falls_back_with_warn`**: edition 2024 では `std::env::set_var` が `unsafe`。`perf_thread_report` の同種のテストの書き方に合わせる（実装の細部）。

## 設計の強み

1. **現物で裏を取って見込みを正した**: ギャップ分析が docs.rs の表示から書いた 3 点（壊れた JSON は 400 でなく 415・未知メソッドは `-32600` でなく `-32601`・JSON-RPC エラーの HTTP 状態は旧式の経路で常に 200）を rmcp 3.5.0 のソースで引き直し、`Accept` の 406（B-8）という survey にも brief にも無かった事実を signoff の `curl` 例にまで反映している。こちらで同じ箇所を読んで全部一致した。
2. **畳み方と記録の捕捉が構造で成り立つ**: 取っ手の `Drop` を `resolve_boot` の直後・`WinApp` の前に置くことで `down?` の早い戻り・`?` のどの経路でも畳まれ、待ちは `recv_timeout(SHUTDOWN_WAIT)` 2 秒で頭打ち（`perf_thread_report` の `FINAL_WAIT` と同じ）、join はしない（`ActorHandle` は detach）。束ねを呼び出し側で同期に行うので、`capture` がスレッド局所でも要件 9.3 の `error!`・1.2 の `info!`・1.9 の `info!` を決定論テストで数えられる。この 2 つが噛み合っているのが本設計の芯。

## 最終判断

**GO**（条件つき: 指摘 1 を設計ディスカッションで要件 3.1 の文面とテスト 2 本の形に落としてから `/kiro-spec-tasks` へ進む）。

- 根拠: 既存の層と依存の向き（`areka` → `areka-mcp` → `areka-actor`・rmcp と tokio の型は外へ出ない）に食い違いは無く、要件はすべて追跡表で部品へ結ばれ、実装の道筋（最初のタスクで空のクレート＋`initialize`→`ping` の 1 本を緑にして rmcp の経路を確かめる）も明確。指摘 1 は要件の 1 文とテスト 1 本の分割で解け、設計の形は変わらない。指摘 2・3 は実装の段で数行で済む。
- 次の手順: ⑴ 設計ディスカッションで指摘 1（要件 3.1）と B-10（要件 3.8 の括弧書き）を併せて要件の文面を直す。⑵ 指摘 2 の 3 分割と指摘 3 の 3 点を design.md の該当節に 1〜2 行ずつ足す。⑶ `/kiro-spec-tasks areka-P0-mcp-server-core` でタスクを生成する。

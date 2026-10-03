# 設計の検証: areka-P0-mcp-tool-entrances

> 2026-10-03・本ブランチ（`eb7312dd`）で、`design.md` の主張を実物と突き合わせて書いた。対話なしの検証で、開発者への質問は無い。指摘は設計ディスカッションの議題になる。

## 判定

**GO**（指摘 3 件は設計ディスカッションで扱う。1 件目は 3 段目の並走に響くので、タスクを切る前に決める）。

## 要約

設計が前提にしている既存コードと rmcp・tokio-util の振る舞いは、読んだ範囲ですべて実物と合っていた。待ちの形・一覧の並び・`-32602` の出し方・終了の途中の扱いは、宣言済みの依存だけで組めて、取りこぼしや競合は見つからなかった。弱いのは「3 段目が自分のファイルだけを触る」約束で、後から答えるツールが毎フレーム返事を覗く場所を、共有ファイルを触らずに持てない。

## 実物で確かめたこと

| 設計の主張 | 確かめた場所 | 結果 |
|---|---|---|
| `ToolRouter::list_all` は名前の辞書順に並べ替える | rmcp 3.5.0 `src/handler/server/router/tool.rs` の `list_all`（`tools.sort_by(…name…)`） | 合っている。`ArekaHandler` が登録順の列を別に持って返せば SSP の並びになる。`get_tool`・`call` は `ToolRouter` のままで足りる |
| 処理が返した `Err(invalid_params)` は JSON-RPC のエラーのまま出る | 同ファイルの `ToolRouter::call` と `into_tool_argument_error`、`ToolRoute::new_dyn` の型（`Result<CallToolResponse, ErrorData>`） | 合っている。`isError` の結果へ変わるのは `message` が `failed to deserialize parameters:` で始まるときだけ。未知の名前と同じ道を通るので、既存の `unregistered_name_is_invalid_params` が旧式の経路の実証になっている |
| 無状態版の経路は `-32602` を HTTP 400 で返す見込み | `src/transport/streamable_http_server/tower.rs` の `jsonrpc_http_status` | 合っている（`INVALID_PARAMS` → `BAD_REQUEST`。旧式の経路はこの関数を通らず 200） |
| 4 つの欄だけの定義が出せる | `src/model/tool.rs` の `Tool`（`title`・`outputSchema`・`annotations`・`icons`・`_meta` は値が無ければ出さない） | 合っている |
| 合図（`CancellationToken`）は宣言済みの依存だけで使える | `crates/areka-mcp/Cargo.toml`（tokio `rt`・`net`・`time`、tokio-util 0.7）・tokio-util 0.7.19 の `Cargo.toml`（tokio を `sync` つきで無条件に引く）と `src/lib.rs`（`pub mod sync` に機能の条件なし） | 合っている。`tokio::sync` を自分で書かずに済む |
| 合図の取りこぼしが無い | tokio-util `cancellation_token.rs` の `WaitForCancellationFuture::poll`（毎回 `is_cancelled` を先に見る） | 合っている。待ち始める前に合図が立っていても即座に抜ける |
| 返事・上限・手放しを見分けられる | `crates/areka-actor/src/reply.rs` の `try_recv`（`Ok(Some)`・`Ok(None)`・`Err(Dropped)`） | 合っている。`ReplyTo` の `Drop` が「送り手を落とす → 合図」の順なら、合図で起きた時点で `Ok(None)` にはならない。上限と返事が同時でも、返事が届いていれば返事を返す |
| 処理のフューチャは `Send`、登録する処理は `Send + Sync` | std の `mpsc::Sender`（`Sync`）・`Receiver`（`Send` だが `Sync` でない） | 設計の注意書き（合図の写しを取り出して待つ）を守れば成り立つ |
| 受け口を落とすと溜まった要求も落ちる | std の mpsc（受け手の破棄で残りの値を捨てる）・`ghost_switch.rs` の `remove_non_send` の前例 | 合っている。`mcp::close` の 1 行で、溜まった要求は即座に「shutting down」になる |
| `main.rs` は 954 行・起動と終了の順 | `crates/areka/src/main.rs` の `fn main()`（`areka_mcp::start` → `WinApp::with_exit_policy` → `register_systems` → `app.run()` → `exit_wait::begin_close`） | 合っている。12 行足しても 966 行。`exit_wait.rs` は触らずに済む |
| 毎フレーム汲む定石・置き場・実行系の有無 | `ghost_switch.rs` の `ChangeRx`・`drain_change_requests`・`take_down`、`ghost_session.rs` の `GhostSlot`・`runtime()`・`names()`・`ghost_dir()`・`for_test` | 合っている |
| 記録の捕捉は呼んだスレッドだけ | `crates/log-capture-kit/src/capture.rs`（`with_default`） | 合っている。橋のフューチャをテストのスレッドの tokio で回す形なら件数を数えられる |
| 既存の `echo_args` のテストは検査を足しても通る | `server_gate_help_tests.rs` の `echo_spec`（`text` が必須の文字列）と `registered_tool_roundtrip`（毎回 `text` を渡す） | 合っている |
| `Cargo.toml` は変えなくてよい | `crates/areka-mcp/Cargo.toml`・`crates/areka/Cargo.toml` | 合っている。アプリ本体側は JSON も tokio も要らない形になっている |
| 要件の番号の網羅 | `design.md` の Requirements Traceability | 1.1〜8.5 の 51 項目がすべて載っている（欠け 0） |

## 指摘（3 件）

### 指摘 1: 後から答えるツールが、共有ファイルを触らずに「毎フレーム返事を覗く」場所を持てない

- **何が問題か**: 設計は「別スレッドのアクターに問うなら `ReplyTo` を持たせて後から送る」と書くが、kanade と sylphya の要求は `areka_actor::ReplySender<具体の型>` を受ける形で（`crates/areka-kanade/src/msg.rs` の `reply` の欄）、どちらのクレートも `areka-mcp` に依存していない。つまりアクターは `ReplyTo` を持てない。実際の形は「`ReplyReceiver` と `ReplyTo` の対を World に置き、毎フレーム `try_recv` で覗く」になり（`try_recv` の説明もこの使い方を名指ししている）、その系を登録する場所は `mcp/mod.rs` の `register` か `ghost_session.rs` しか無い。走っている最中の `Input` 段へ `handle` の中から系を足すこともできない。
- **響くところ**: `mcp-kanade-tools`・`mcp-get-property`・`mcp-reload`・`mcp-dump-images` がそろって `mcp/mod.rs` を触ることになり、「3 段目は共有ファイルを触らない・重なり 0」が崩れる。加えて、3 段目が自分の資源に抱えた `ReplyTo` は `mcp::close` では落ちないので、終了の途中に上限まで待つ要求が出る。
- **直し方の案**: 本 spec の `mcp/mod.rs` に「後で答える」置き場を 1 つ持つ。例: `later(world, 覗く関数, reply)` で `(Box<dyn FnMut(&mut World) -> Option<ToolOutcome>>, ReplyTo)` を積み、`drain` が毎フレーム全件を覗いて、答えが出たものを送って外す。`close` はこの置き場も落とす。別案は、各ツールのファイルに空の `register(world)` を置き、`mcp::register` が 10 本ぶん呼ぶ形。どちらでも 3 段目は自分のファイルだけで済む。
- **要件**: 6.4・6.6・7.1・7.3
- **設計の場所**: 「設計の決定」の「処理が返事を持つ形」・「mcp/<ツール>.rs」の約束・「3 段目の spec が触るファイル」

### 指摘 2: 「起動中のゴースト」を読む配線の、成り立つ側のテストが無い

- **何が問題か**: `resolve::active` のテストは「置き場が空」「実行系の無い単位」の 2 つ（どちらも `None`）だけ。実行系のある単位で `Some` になり、名前と絶対パスが入ること、LogSink へ倒れた単位も数えること（要件 3.8 の裁定）は、実機確認でしか確かめない。判断を「LogSink へ倒れていない」に書き違えても、どのテストも赤にならない。受け口から汲む系までを本物のゴーストで通すテストも無い。
- **響くところ**: 要件 3.8・4.1 の中心の枝が決定論テストで固定されない。
- **直し方の案**: 既存のテスト用の起こし方（`emo2_boot/ghost_switch_test_support.rs` の `boot`。偽の SHIORI で実行系つきの単位を作り、`register_systems` も通す）を使い、`mcp_tests.rs` に 1 本足す: ゴーストを起こす → `ToolRequest::new(GetActiveGhostList)` を送る → 1 フレーム回す → 答えが descript の `name`。LogSink へ倒れた単位は `ghost_session_strict_tests.rs` に作り方の前例がある。
- **要件**: 3.8・4.1・4.5・6.1
- **設計の場所**: Testing Strategy の `mcp_tests.rs`「`active`（3.8）」

### 指摘 3: 無状態版の経路の HTTP 400 が、実際のクライアントでどう見えるかを確かめる項目が無い

- **何が問題か**: Claude Code は無状態版の経路でつなぐ。その経路では、必須の欄の欠落や型違いの `-32602` が HTTP 400 で返る（rmcp の `jsonrpc_http_status`）。設計は値をテストで測って差の一覧に書くが、実機確認の手順に「引数を誤って呼んだとき、エージェントに理由の文（`missing required argument: …`）が見えるか」が無い。
- **響くところ**: 要件 2 の狙い（エージェントが自分の呼び方の誤りだと分かって直せる）。400 を接続の失敗として扱うクライアントだと、理由が届かない。要件 2.7 は rmcp の値のままでよいとしているので、設計の変更は要らない。
- **直し方の案**: `verification/signoff.md` に 1 項目足す（必須の欄を抜いた `sakurascript` を Claude Code から呼び、見えた文を書き残す）。理由が見えなければ、差の一覧に書いて別の spec へ送る。
- **要件**: 2.2・2.7・8.3・8.5
- **設計の場所**: 「設計の決定」の 1・「実機確認（8.5）」

## 小さな気付き（議題にしなくてよい）

- 処理が `ReplyTo` を誤って落とすと `NG:areka is shutting down` と答える。要件 6.4 の字面どおりで、設計も Risks に書いている。指摘 1 の置き場を持てば、落とす道そのものが減る。
- `as_integer` が `1.0` を整数として受けるのは要件 2.4（拒むのは「整数でない数」）の範囲内。`check_tests.rs` に載っている。

## 良い点

- **待ちの形**: 宣言済みの依存だけで、スレッドを足さず、短い間隔で覗くこともせず、返事・上限・手放しを見分けている。「送り手を落としてから合図」の順まで書いてあり、実物で確かめても穴が無かった。
- **既存を変えない線引き**: 検査を `handler.rs` の写しの中に置いたので、`ToolHandler`・`ToolSpec`・`ToolOutcome` の型と `mcp-server-core` のテストが 0 行のまま。共有テストがダミーの文言を見ない工夫も、3 段目の並走に効く。

## 次の一手

1. 設計ディスカッションで指摘 1 の形（「後で答える」置き場か、ツールごとの `register` か）を決め、`design.md` の `mcp/mod.rs` と「3 段目の spec が触るファイル」に反映する。
2. 指摘 2・3 はテストと実機確認の項目を 1 つずつ足すだけで済む。
3. その後 `/kiro-spec-tasks areka-P0-mcp-tool-entrances`。

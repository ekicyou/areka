# ギャップ分析: areka-P0-mcp-receiver-bugs

- 日付: 2026-10-18（main `2d86dbff` を取り込んだブランチ `claude/areka-p0-mcp-receiver-bugs-799171`）
- 対象: 確定した `requirements.md`（要件 1〜3）と、今のコード・rmcp `=3.5.0` の作り
- やり方: コードと rmcp の元のソースを読む静的な調べに加えて、件 1 の今の答えを**実際に測った**（下の「測った今の答え」）。実機の areka・SSP・開発者の記憶には触っていない。
- 測るのに使った道具: `target\gap-probe\`（ワークツリーの `target\` の下・git の外）。本番の 11 本のツールを登録した受け口（`areka_mcp::start`）を空きポートに起こし、5 つの版のそれぞれの話し方（`real_receiver_test.rs` の `post`／`request` と同じ札の付け方）で 33 通りの要求を直接送った。ビルドの成果物は消した（ソースと `Cargo.lock` だけ残す）。設計の段で測り直すときは `cargo run --offline` で同じ表が出る。

---

## 1. 分析の要約

- **件 1 の原因は brief のとおり**。rmcp 3.5.0 の `ClientRequest` は `#[serde(untagged)]` の列で、型の合う変種が無いと最後の `CustomRequest`（型のない要求）として読む。`ArekaHandler` は `on_custom_request` を上書きしていないので、rmcp の既定（`-32601`・`message` はメソッド名・`data` なし）が返る。`on_custom_request` を 1 つ書けば `tools/call` の `{"name":7}`・`{}`・`params` なし・`"arguments":7` はどれも直る（5 つの版とも、ヘッダや `_meta` の検査を通った後に handler へ届くことを測って確かめた）。
- **ただし、`on_custom_request` に届かない「形の違い」がある**。⑴ `tools/list`・`ping`・`prompts/list`・`resources/list`・`resources/templates/list`・`server/discover` は、rmcp の型が形の違う `params` を黙って受け流し（`tools/list` の `{"cursor":7}` は成功の答え）、handler には「`params` なし」として届く。⑵ `params` がオブジェクトでない要求（`7`・`[1]`・`"x"`）は rmcp が読めず、HTTP 415 の平文で答える（JSON-RPC のエラーではない）。要件 1.1 は例に `tools/list` を挙げ、要件 2.2 は「形の違う `params` は中継ごしでも `-32602`」と書くので、この 2 つの扱いを要件の討議で決める必要がある（下の議題 1・2）。
- **「受け付けるメソッド」の一覧の出どころ**は 3 案ある（rmcp の名前の定数で自分の表を持つ・areka が自分で答えるメソッドだけに絞る・メソッドごとに rmcp の型で読み直して理由の文も取る）。実際に `on_custom_request` へ届いて「受け付ける」側に入るのは `tools/call`・`initialize`・`completion/complete` の 3 つだけで、版による違い（`2026-07-28` に `ping` が無い）は rmcp の型の側で先に決まる。
- **件 2 は静的には止まる場所が見当たらない**。今の `dump_balloon` は UI スレッドで GPU を待たず（`Map` は `DO_NOT_WAIT`・積むときに `Flush` 済み）、毎フレーム 1 段ずつ進め、PNG と base64 は別のスレッドで作る。答えが無ければ受け口の側が 10 秒で `NG:areka did not respond within 10 seconds` を返す作り。残る仕事は実機の確かめで、その道具（並走中の `real-run-mcp-kit` には頼れない）と、バルーンを出す手段（`sakurascript` はまだ `NG:not implemented yet`）が設計の段の調べもの。
- 規模は **S**（件 1＝3〜5 タスク・件 2＝確かめだけなら 1〜2）。危うさは件 1 が**低**、件 2 が**中**（再現の有無が実機で決まる・debug 版と i686 の helper の準備が要る）。

---

## 2. 今のコードの姿

### 2.1 受け口（`crates/areka-mcp/`）

| 場所 | 中身 | 本 spec との関わり |
|---|---|---|
| `src/handler.rs`（144 行） | `ArekaHandler`＝rmcp の `ServerHandler`。`get_info`・`list_tools`（`2026-07-28` 以降は `ttlMs`・`cacheScope` を付ける）・`call_tool`（`ToolRouter::call` に任せる＝未登録の名前は `-32602 "tool not found"`）・`get_tool`。登録の写しの中で `arguments` を `inputSchema` に照らし、合わなければ `debug!(tool, reason, "MCP: 引数が inputSchema に合わない")` を 1 件残して `-32602` | **`on_custom_request` が無い**＝件 1 の直し場所。頭の説明（`-32602` の決まり）も書き足す所 |
| `src/dispatch.rs` | rmcp の `StreamableHttpService` の設定を 1 か所で組む（`with_legacy_session_mode(false)`＝無状態・`with_json_response(true)`・本文の上限 4 MiB） | 変えない。無状態の経路なので、答えの HTTP の状態は rmcp の `jsonrpc_http_status` が決める（下の 3.2） |
| `src/server_protocol_tests.rs`（446 行） | 実ソケットのテスト。`tools_call_any_name_is_invalid_params`（登録 0 本で `-32602`）・`unknown_method_is_jsonrpc_error_from_rmcp`（`{"code":-32601,"message":"no/such"}`）・`ping`・`server/discover`・ヘッダの検査など。版ごとの話し方の組み立て（`stateless_meta`・`LEGACY_VERSIONS`・`SUPPORTED_VERSIONS`）を持つ | 要件 2.3 の決定論テストの置き場の候補。1,000 行の目安まで余裕がある |
| `src/testkit.rs` | テスト専用の HTTP/1.1 クライアント（`serve`・`rpc`・`post_rpc`・`request`） | 新しいテストもこれを使える |
| `handler_tests.rs` | **無い** | 判断を純粋な関数に切り出すなら新しく作る兄弟（`structure.md` の兄弟の規則） |
| 記録のテスト | `log-capture-kit` の `capture` は**呼んだスレッドの記録だけ**を拾う（`server_tests.rs` の頭の説明）。handler は `mcp` スレッドの tokio の上で動く | 要件 1.5 の `debug` 1 件を実ソケットのテストで数えるには `install_global_capture_all` が要る。純粋な関数をテストのスレッドで呼べば `capture` で足りる（議題 6） |

既存の記録の実例: `arguments` の検査の `debug!` 1 件には、今テストが無い（数えているのは `server_tests.rs` の起動と終了の記録だけ）。

### 2.2 中継（`crates/areka-mcp-bridge/`）

- 中継は標準入力の行を rmcp の `ClientJsonRpcMessage` で読み、rmcp のクライアントで受け口へ送り直す。型のない要求（`CustomRequest`）も `method` と `params` のまま書き直して届く。HTTP の状態に関わらず本文の JSON をそのまま返す（`http_tests.rs` の `a_json_body_comes_back_whatever_the_http_status`）。
- `tests/real_receiver_test.rs`（320 行）: 5 つの版それぞれで 8 通りの要求を「直接の答え」と「中継ごしの答え」で比べる。**196〜197 行目の説明**と **208 行目** `{"id": 8, "method": "tools/call", "params": {"name": 7}, "at": "/error/code", "is": -32601}` が今の答えを固定している。比べ方は応答の全体（`id`・`error` の欄）で、HTTP の状態は見ない（`direct` は本文だけを読む）。
- 未登録の名前の行（`id` が 7・`-32602`）は今も 5 つの版で緑＝`2026-07-28` で受け口が HTTP 400 を返しても、中継は同じ本文を返している。直した後の `-32602`（400）も同じ道を通るので、**中継のコードは直さずに済む見込みが高い**（要件 2.2）。

### 2.3 差の記録（`doc/ssp-mcp/`）

- `bridge-diff-areka.md` **93 行目**「形の違う `params` の `tools/call`（`name` が数 など）」の areka の欄に `-32601` と「spec `areka-P0-mcp-receiver-bugs` の件 1 で直す」が書いてある（要件 2.5 の直し先）。
- 同じ文書の 37 行目（応答の行）は「5 つの版 × 8 通りの要求（…形の違う `params`）」と書くだけで番号を書いていない＝直さずに正しいまま。
- `transport-diff-areka.md`（受け口と SSP の差の一覧）には形の違う `params` の行が無い。この文書は「差の一覧に写す値は各テストの assert に具体の値で固定する」（`server_protocol_tests.rs` の頭）という組で、要件は足すことを求めていない（議題 8）。
- `structure.md` 357 行目の `handler.rs` の説明（「引数の検査を呼んで `-32602` に」）は、完了の時に `on_custom_request` の一言を足す所。

### 2.4 `dump_balloon`（`crates/areka/src/mcp/`）

| 場所 | 中身 |
|---|---|
| `mod.rs`（152 行） | `drain` が Input 段（`dispatch_pointer_events` の後）で毎フレーム、溜まった要求を振り分けてから `poll_later` で預かった組を全部覗く。`later` は覗く関数と返事を預ける。待つ側が去った組（`is_abandoned`）は覗かずに捨てる |
| `dump_balloon.rs`（163 行） | `answer`: 結線状態が無ければ「窓が無い」・装着の相がまだなら `None`（次のフレームでやり直し）・背景の写しを取り、文字の面が在れば `begin_read_back` で写しを積んで `Step::Read` |
| `dump_surface.rs`（437 行）の `start`・`Wait` | 2 本のツールが分け合う段の機械。`Attaching`（毎回ゴーストが替わっていないか確かめる）→ `Reading`（`try_finish` を 1 度だけ呼ぶ・待たない）→ `Encoding`（`mcp-encode` のスレッドの答えを `try_recv` で覗く） |
| `areka-emo-text/src/surface.rs` | `begin_read_back` は `CopyResource` の後に `Flush`。`try_finish` は `Map(READ, DO_NOT_WAIT)` で、`DXGI_ERROR_WAS_STILL_DRAWING` なら「まだ」 |
| `areka-mcp/src/tools/bridge.rs` | 受け口の側の待ちの上限 `REPLY_WAIT`＝10 秒。過ぎたら `NG:areka did not respond within 10 seconds` |
| テスト | `dump_balloon_tests.rs`（60 行）・`dump_balloon_overlay_tests.rs`・`dump_balloon_gpu_tests.rs`（587 行・x64 だけ・本物の GPU で段の機械を通す）。`mcp_tests.rs`（447 行）は `later`／`poll_later` の決まり |

静的に見て、答えが返らなくなりうる所は 3 つ（どれも今は 10 秒の上限で `NG` になる＝「黙って返らない」にはならない）:

1. `Attaching` のまま: 装着の相が終わらない（`wiring.attached()` が偽のまま）。要件 3.1 は「バルーンを出した状態で」呼ぶので、装着は済んでいるはず。
2. `Reading` のまま: GPU の写しが終わらない（`try_finish` がずっと「まだ」）。`Flush` 済みなので、普通は数フレームで終わる。
3. 覗かれない: `drain` が回らない（フレームが進まない）。今の作りではフレームは vsync で進む。

10-06 の "task cancelled" は、当時 UI スレッドで GPU を待っていた（その間フレームが止まる）作りでの、**呼び出し側**の時間切れだった。今の作りでは、areka の側の 10 秒より前に呼び出し側が切れることは考えにくい（中継の応答の上限は 30 秒）。

---

## 3. rmcp 3.5.0 の作り（件 1 に効く所）

### 3.1 型のない要求に落ちる道

- `ClientRequest` は `PingRequest`・`InitializeRequest`・`DiscoverRequest`・`CompleteRequest`・`SetLevelRequest`・`GetPromptRequest`・`ListPromptsRequest`・`ListResourcesRequest`・`ListResourceTemplatesRequest`・`ReadResourceRequest`・`SubscriptionsListenRequest`・`SubscribeRequest`・`UnsubscribeRequest`・`CallToolRequest`・`ListToolsRequest`・`GetTaskRequest`・`UpdateTaskRequest`・`CancelTaskRequest`・`CustomRequest` の untagged の列（`model.rs` 4633 行）。
- 落ち方は `params` の型の種類で決まる:
  - **`params` が必須の型（`Request<M, R>`）**: `initialize`・`server/discover`・`completion/complete`・`logging/setLevel`・`prompts/get`・`resources/read`・`resources/subscribe`・`resources/unsubscribe`・`subscriptions/listen`・`tools/call`・`tasks/*`。`params` が無い・中の型が合わないと `CustomRequest` に落ちる。
  - **`params` が任意の型（`RequestOptionalParam`）**: `tools/list`・`prompts/list`・`resources/list`・`resources/templates/list`。`params` の中身は serde の `flatten` した `Option` なので、型が合わないと**黙って「`params` なし」**になる（`{"cursor":7}` が成功する理由）。`CustomRequest` には落ちない。
  - **`params` を読まない型（`RequestNoParam`）**: `ping`。オブジェクトなら中身を問わない。
  - `server/discover` の `DiscoverRequestParams` は `deny_unknown_fields` だが、`flatten` の下なので余分な欄も通った（測った）。
  - **`params` がオブジェクトでない**（数・配列・文字列）: `CustomRequest` も読めず、要求全体が読めない＝**HTTP 415 の平文**（`fail to deserialize request body …`）。handler には届かない。
- `CustomRequest` は `_meta` を `params` から分けて持つ（`serde_impl.rs` 431 行）ので、`2026-07-28` の `_meta` の検査は型のない要求にも効く。`on_custom_request` が受ける `params` には `_meta` が入らない。

### 3.2 handler より前の検査と、答えの HTTP の状態

- 無状態の経路（`tower.rs` 2228 行〜）は、handler の前に `MCP-Protocol-Version` の値・`Mcp-Method`／`Mcp-Name`／`Mcp-Param-*`（`2026-07-28` だけ）・`_meta` と札の食い違いを検査する。`Mcp-Name` は `params.name` が**文字列のときだけ**求める（`mcp_headers.rs` の `extract_name`）ので、`{"name":7}` や `name` なしの `tools/call` は `Mcp-Method: tools/call` だけで handler へ届く（測った）。
- `_meta` に版を持つ要求（`2026-07-28`）の答えは `jsonrpc_http_status` で状態を決める: `-32602`・`-32020` 等は **400**、`-32601` は **404**、ほかは 200。旧式の 4 版は常に 200。→ `on_custom_request` が `ErrorData::invalid_params(…, None)` を返せば、`2026-07-28` でも未登録の名前と同じ **400** になり、要件 1.4 の「同じ HTTP の状態」は rmcp が満たす。
- `on_custom_request` の中で版は `context.protocol_version()` で分かる（旧式で札が無いときは `2025-03-26` と見なされる）。rmcp が `ping` を「旧式だけ」と決める `uses_legacy_lifecycle` は `pub(crate)` で外から呼べない。外から使えるのは `ProtocolVersion::has_initialize()`・`ProtocolVersion::NO_INITIALIZE`（＝`2026-07-28`）。
- メソッドの名前の定数は `rmcp::model::ConstString` の `VALUE`（`CallToolRequestMethod::VALUE == "tools/call"` など）で外から読める。

### 3.3 rmcp の既定の答え（areka が上書きしていないもの）

`complete`・`list_prompts`・`list_resources`・`list_resource_templates` は既定で**成功**（空の結果）を返す。areka は `prompts`・`resources`・`completions` の能力を名乗っていないが、形の正しい要求には成功で答えている（今の振る舞い・要件 1.6 で変えない側）。`set_level`・`get_prompt`・`read_resource`・`subscribe`・`unsubscribe` と `tasks/*`（能力なし）は既定で `-32601`。

---

## 4. 測った今の答え（5 つの版・本番の 11 本を登録した受け口に直接送った）

`2026-07-28` は `_meta` と `Mcp-Method`（`name` が文字列なら `Mcp-Name` も）を付けた。旧式の 4 版（`2024-11-05`・`2025-03-26`・`2025-06-18`・`2025-11-25`）は 4 つとも同じ答えだったので 1 列にまとめる。

| 要求 | 旧式の 4 版 | `2026-07-28` | 要件で直す側か |
|---|---|---|---|
| `tools/call` `{"name":7}` | 200・`-32601` `"tools/call"` | 404・`-32601` `"tools/call"` | 直す（1.2） |
| `tools/call` `{}`（`name` なし） | 200・`-32601` | 404・`-32601` | 直す（1.2） |
| `tools/call` `params` なし | 200・`-32601` | 404・`-32601` | 直す見込み（1.1） |
| `tools/call` `"arguments": 7` | 200・`-32601` | 404・`-32601` | 直す見込み（1.1。`arguments` が**オブジェクトで**定義に合わないときの検査とは別） |
| `tools/call` 未登録の名前（比べる相手） | 200・`-32602` `"tool not found"` | 400・`-32602` `"tool not found"` | 変えない（1.6） |
| `tools/call` `params` が `7`・`[1]`・`"x"` | 415・平文 | 415・平文 | **handler に届かない**（議題 2） |
| `tools/list` `{"cursor":7}` | 200・**成功**（一覧） | 200・**成功** | **handler に届かない**（議題 1） |
| `initialize` `{"protocolVersion":7}`・`{}` | 200・`-32601` `"initialize"` | 400・`-32020`（下の注） | 直す見込み（1.1） |
| `initialize` 形の正しいもの | 200・成功 | 200・成功（`2025-11-25` へ倒れる） | 変えない |
| `server/discover` `{}`・`{"x":7}` | 400・`-32602`（`_meta` の欠落・handler の前） | 200・成功（余分な欄は通る） | 届かない（実質） |
| `ping` なし／`{"x":7}` | 200・`{}` | 404・`-32601` `"ping"`（型の道で決まる） | 変えない（1.3） |
| `completion/complete` 形の正しいもの | 200・成功（空） | 200・成功（空） | 変えない |
| `completion/complete` `{"ref":7}` | 200・`-32601` | 404・`-32601` | **1.1 の字面では直す側**（議題 4） |
| `prompts/list`・`resources/list` `{"cursor":7}` | 200・成功 | 200・成功 | 届かない |
| `prompts/get`・`resources/read`・`logging/setLevel`（形の正しいもの・違うもの） | 200・`-32601` | 404・`-32601`（`resources/read` の正しい形は `Mcp-Name` を付けないと `-32020`） | 変えない（受け付けないメソッド） |
| `resources/subscribe`・`subscriptions/listen`・`tasks/get` | 200・`-32601` | 404・`-32601` か `-32020`（`Mcp-Name` の欠落） | 変えない |
| `nope/unknown` `{"name":7}` | 200・`-32601` `"nope/unknown"` | 404・`-32601` | 変えない（1.3） |

注: `initialize` の `2026-07-28` の列は、測り方（`initialize` には `MCP-Protocol-Version` を付けず `_meta` だけ付けた）のせいで handler の前の検査（`-32020`）が先に答えた。札と `_meta` を揃えて送れば handler（`on_custom_request`）まで届く作り（`tower.rs` の検査の順から読める・測ってはいない）。`2026-07-28` のクライアントは `initialize` を送らないので、実用の差は出ない。

ここから分かること:

- **`on_custom_request` に届いて「受け付ける」側に入るのは、実際には `tools/call`・`initialize`・`completion/complete` の 3 つ**。受け付けない側で届くのは知らない名前と `logging/setLevel`・`prompts/get`・`resources/read`・`resources/subscribe`・`resources/unsubscribe`・`subscriptions/listen`・`tasks/*`。
- 「`2026-07-28` に `ping` が無い」は rmcp の型の道（`PingRequest` に読んだ後の分岐）で決まり、`on_custom_request` には `ping` が届かない。表に版の分け方を持つのは、rmcp を上げたときの備え（議題 3）。

---

## 5. 要件ごとの対応表

| 要件 | 今あるもの | ギャップ | 種別 |
|---|---|---|---|
| 1.1 受け付けるメソッドの形の違いは `-32602` | 型の誤りの `-32602` は `arguments` と未登録の名前だけ | `on_custom_request` が無い。`tools/list` などは rmcp が受け流すので届かない・`params` がオブジェクトでないものは 415 | Missing（`tools/call` 等）／**Constraint**（`tools/list`・非オブジェクト） |
| 1.2 `{"name":7}`・`name` なし | なし（`-32601`） | 上と同じ 1 か所で直る（測って届くことを確かめた） | Missing |
| 1.3 受け付けないメソッドは `-32601`（文は名前） | rmcp の既定のまま正しい | 一覧の作り方を誤ると壊れる（`ping` を版に関わらず「知っている」に入れる等）。今は届かないので守られるが、テストで固定する | Constraint |
| 1.4 未登録の名前と同じ状態と欄の組 | 未登録の名前は `{"code":-32602,"message":"tool not found"}`・`data` なし・旧式 200／`2026-07-28` 400 | `ErrorData::invalid_params(文, None)` で揃う。`message` の文は未定（議題 5） | Missing（文は Unknown） |
| 1.5 `debug` の記録 1 件・メソッドの名前つき | `arguments` の検査の `debug!` が手本（テストは無い） | 記録の文と、どう数えるか（スレッドの壁） | Missing／Unknown（議題 6） |
| 1.6 ほかの答えは前と同じ | 既存のテスト（`server_protocol_tests.rs`・`real_receiver_test.rs` の 7 行） | 影響を受けうるのは `on_custom_request` に届くものだけ＝今 `-32601` の答えの一部だけ。形の正しい要求は届かない | Constraint |
| 2.1 5 つの版で同じ | 版ごとの話し方の組み立てがテストに在る | `2026-07-28` は状態が 400 になる（旧式は 200）＝「同じ」は番号と欄の組の意味で読む | Constraint |
| 2.2 中継ごしでも `-32602` | 中継は型のない要求をそのまま届け、本文を返す | コードの変更は要らない見込み。`params` がオブジェクトでない行は中継が自分で `-32600` を返す（受け口の 415 とも違う） | Constraint（議題 2） |
| 2.3 受け口の決定論テスト（1.1・1.2・1.3 を 5 つの版で） | `server_protocol_tests.rs` の部品（`stateless_meta` 等） | テストを足す。`tools/list` を例にするかで中身が変わる（議題 1） | Missing |
| 2.4 中継の比較テストの 1 行と説明 | 208 行目・196〜197 行目 | 期待を `-32602` に、説明を新しい答えに | Missing |
| 2.5 差の記録の 1 行 | 93 行目 | areka の欄を `-32602` に、「本 spec で直す」を外す | Missing |
| 2.6 既存のテストは緑のまま | — | 期待が変わるのは 2.4 の 1 行だけの見込み。`unknown_method_is_jsonrpc_error_from_rmcp`・`ping`（`2026-07-28`）の行は届かない／名前が一覧に無いので変わらない | Constraint |
| 2.7 実機で中継ごしに 1 回 | 実機の道具は並走中の `real-run-mcp-kit`（C9-⑥・未着地） | 件 2 と同じ起動で送る手順が要る | Unknown（議題 9） |
| 3.1〜3.3 debug 版で 10 回の確かめと記録 | 実走のテストの手本（`crates/areka/tests/mcp_get_log_real_run.rs`＝検体・`AREKA_MCP_PORT`・`AREKA_APP_SMOKE_EXIT_MS`・i686 の helper を自前で揃える） | 手順・道具・記録の置き場。バルーンを出す手段（`sakurascript`・`raise_event` はまだ `NG:not implemented yet`） | Unknown（議題 9・10） |
| 3.4〜3.7 返らなかったときの直し | 段の機械と 10 秒の上限・`dump_balloon_gpu_tests.rs` | 再現したときだけ。止まり場所の候補は 2.4 の 3 つ。`dump_surface.rs` の `start`／`Wait` に原因があれば 3.6 | Unknown（再現しだい） |
| 3.8 `target\` の下だけ・i686 の helper | 手本のテストが自前で揃える | 中継の控えの置き場も `target\` の下へ向ける必要（下の 7.4） | Constraint |

---

## 6. 件 1 のやり方の案

### 案 A: 自分の表（rmcp の名前の定数で綴る）を `handler.rs` に持つ

- `on_custom_request` を書き、`request.method` が表に在れば `ErrorData::invalid_params(…, None)`、無ければ rmcp の既定と同じ `ErrorData::new(METHOD_NOT_FOUND, method, None)`。
- 表は `CallToolRequestMethod::VALUE`・`InitializeResultMethod::VALUE`・`CompleteRequestMethod::VALUE`・`ListToolsRequestMethod::VALUE`・`DiscoverRequestMethod::VALUE`・`PingRequestMethod::VALUE`（旧式だけ＝`context.protocol_version()` が `has_initialize()` のとき）などで綴る。字を手で書かないので綴りの誤りが起きない。
- 判断を `fn` 1 つ（メソッド名と版 → 受け付けるか）に切り出せば、`handler_tests.rs` で純粋に確かめられる。
- ✅ 小さい・rmcp の作りに沿う・版の分け方を表の 1 行で持てる
- ❌ rmcp が既定で成功を返すメソッド（`completion/complete` など）を表に入れるかを決める必要（議題 4）。rmcp を上げたとき、新しいメソッドは表に足すまで `-32601` のまま（安全な側に倒れる）

### 案 B: areka が自分で答えるメソッドだけ（`tools/call`・`tools/list`・`initialize`・`server/discover`・旧式の `ping`）

- 表の作りは案 A と同じで、中身を「areka が名乗る能力（`tools`）と生き死にの手続き」に絞る。
- ✅ 「areka が知っている」の意味が `get_info` の名乗りと揃う
- ❌ `completion/complete` の形の違いが `-32601` のまま残る。要件 1.1 の定義（「形の正しい `params` を送れば `-32601` 以外で答えるメソッド」）では `completion/complete` は受け付ける側なので、字面と食い違う（要件の言い換えが要る）

### 案 C: メソッドごとに rmcp の型で読み直す（案 A の表＋理由の文）

- 表を「メソッド名 → rmcp の `params` の型」にし、`request.params_as::<CallToolRequestParams>()` などで読み直して、serde の誤りの文（例: `invalid type: integer `7`, expected a string`）を `message` にする。
- ✅ クライアントが何を直せばよいか分かる（要件の Objective「送り方を直せる」に近い）・型の名前で表が自分を確かめる（読み直しが通ってしまったら想定外として記録できる）
- ❌ 型を 3〜4 個引く分だけ長い。文が rmcp／serde の版で変わる（テストで文を逐語に固定すると上げるたびに直す）

### 案 D（参考・採りにくい）: rmcp の手前（`dispatch.rs`）で本文を読んで判定

- `tools/list` の `{"cursor":7}` や非オブジェクトの `params` まで拾える唯一の道。
- ❌ JSON-RPC を自前で読み直すことになり、「プロトコルは公式 SDK に任せる」（`tech.md`・開発者の決まり）に反する。中継は rmcp の型で書き直して届けるので、`{"cursor":7}` は中継ごしでは `{}` に化けて届き、要件 2.2（中継ごしでも同じ）を満たせない。

### 決めずに残すこと（設計の段へ）

どの案でも触るのは `handler.rs` の 1 か所と、テスト（新しい兄弟 1 つか `server_protocol_tests.rs` への追記）・`real_receiver_test.rs` の 1 行と説明・`bridge-diff-areka.md` の 1 行。`dispatch.rs`・中継のコードは触らない。

---

## 7. 件 2 の確かめのやり方の案

### 7.1 案 R1: 手で回す（コードを足さない）

- debug 版の `areka.exe` と i686 の helper と中継の exe をワークツリーの `target\` に建て、PowerShell から areka を起こし、中継の標準入力へ JSON の行を書く小さな台本で `initialize`（または `server/discover`）→ 形の違う `tools/call` 1 回（要件 2.7）→ `dump_balloon` 5 回、をゴーストごとに送り、各回の時間と答えを記録する。
- ✅ 要件 3.3 の「再現しなければコードを変えない」とそのまま合う・`real-run-mcp-kit` と重ならない
- ❌ 記録は手の作業。台本は spec の記録に残すだけ（後で回し直しにくい）

### 7.2 案 R2: `#[ignore]` の実走のテストのファイルを新しく足す

- `mcp_get_log_real_run.rs` の起こし方を写した新しいファイル（例: `crates/areka/tests/mcp_dump_balloon_real_run.rs`）。
- ✅ 回し直せる
- ❌ 並走中の `real-run-mcp-kit`（C9-⑥）がまさに実走の起こし方を 1 つの道具にまとめる spec で、写しを 1 つ増やすことになる。⑥ の持ち物の 3 ファイルには触らないが、⑥ の着地後に寄せ直す仕事が残る。要件 3.5 の「決定論テスト 1 本」は再現して直したときの話で、確かめそのものにテストは求めていない

### 7.3 案 R3: `real-run-mcp-kit` の着地を待つ

- brief の棚卸㉗の再測定は「窓口の後に回せば、その試験が残る」と書くが、⑥ は未着地で、本 spec は⑥に頼れない（指示どおり）。採らない。

### 7.4 どの案でも要る注意

- **中継の控えの置き場**: 中継は `areka.exe` が隣に在ると areka と同じ決め方（`areka-user-data`）で記憶の置き場を決める＝`%LOCALAPPDATA%\areka` 側に書きうる。`AREKA_PROFILE_DIR` を `target\` の下へ向けて起こす（areka 本体も同じ）。Claude アプリの Bash では `%LOCALAPPDATA%` が箱へ逸れるので、起こすのは PowerShell から。
- **バルーンを出す手段**: MCP の `sakurascript`・`raise_event` は今 `NG:not implemented yet`（`mcp-kanade-tools` が並走で本物にする）。起動の台詞（`OnBoot`）でバルーンが出た後に呼ぶ。台詞の前に呼ぶと「the balloon of this scope is not ready」（`isError: true`）＝要件 3.1 では断りの答えとして数えられるが、「バルーンを出した状態」の条件を満たさない。バルーンが出たことの見分け方（areka の記録の行・`get_log`）を手順に書く。
- **呼び出し側の時間切れ**: 中継の応答の上限は 30 秒、areka の側は 10 秒。記録には両方の値と各回の所要時間を残す（要件 3.2）。
- **記録の段**: 返らなかったときは「その分岐が見える段まで開けた」記録が要る（要件 3.4）。`dump_surface.rs`／`dump_balloon.rs` の記録は `debug` の `[mcp] 答えられない`・`[mcp] 絵を返す` 等で、段の移り（`Attaching` → `Reading` → `Encoding`）そのものの記録は無い。再現したときに足すかは設計で。
- **i686 の helper**: claudia（32bit の SHIORI）は debug 版の `areka.exe` の隣に i686 の helper が要る（`dev-helper-x64-clobber` の罠）。手本のテストが自前で揃える手順を持つ。

---

## 8. 規模と危うさ

- **規模: S**（件 1＝`handler.rs` に 1 つの関数とその判断・テスト 1 ファイル・既存の 2 行の差し替えで 3〜5 タスク。件 2＝確かめだけなら 1〜2 タスク、再現すれば 4〜7 タスク）。
- **危うさ: 件 1 は低**（rmcp の作りを読み、届く道を測って確かめた・既存の部品の範囲）。**件 2 は中**（再現するかは実機で決まる・debug 版の建て方と i686 の helper・バルーンを出す手段が限られる）。

---

## 9. 設計の段への申し送り

### 決める候補（要件の討議に回す）

1. **`tools/list` など、rmcp が形の違いを受け流すメソッドの扱い**: 要件 1.1 は `tools/list` を例に挙げるが、`{"cursor":7}` は成功で答え、`on_custom_request` に届かない。中継も rmcp の型で書き直して `{}` に化かす。⑴ 要件 1.1 を「rmcp が型の合わない要求として読んだもの（＝`params` が必須のメソッド）」に言い換え、受け流しは今のまま（成功）と書く ⑵ `dispatch.rs` で本文を読んで判定する（案 D・SDK に任せる決まりと要件 2.2 に反する）。
2. **`params` がオブジェクトでない要求（`7`・`[1]`・`"x"`）**: 受け口は 415 の平文、中継は自分で `-32600` を返し、どちらも `-32602` にならない。要件 2.2 の「形の違う `params`」から外す（範囲外か既知の限界と書く）か。
3. **「受け付けるメソッド」の表に版の分け方を持つか**: 今は rmcp の型の道が先に `2026-07-28` の `ping` を `-32601` にするので、表が版を見なくても答えは変わらない。rmcp を上げたときの備えとして版を見る（`has_initialize()`）か、見ない（短い）か。
4. **rmcp が既定で成功を返すメソッド（実際に届くのは `completion/complete`）を「受け付ける」に入れるか**: 要件 1.1 の定義の字面では入る（案 A）。areka が名乗る能力に揃えるなら入れない（案 B・要件の言い換えが要る）。
5. **`-32602` の `message` の文**: ⑴ メソッド名（rmcp の `-32601` と同じ形・例 `"tools/call"`）⑵ serde の理由の文（案 C・例 `invalid type: integer `7`, expected a string`）⑶ 決まった英文。テストで文を逐語に固定するか、番号と欄の組だけを固定するか。
6. **要件 1.5 の記録をどう確かめるか**: handler は `mcp` スレッドで動き、`capture` は呼んだスレッドだけを拾う。⑴ 判断を純粋な関数に出してテストのスレッドで `capture` ⑵ `install_global_capture_all` で実ソケットのテストから数える ⑶ 記録はテストせず目で確かめる（今の `arguments` の検査の記録と同じ扱い）。
7. **テストの置き場**: `server_protocol_tests.rs`（446 行）に足すか、新しい兄弟（`server_params_tests.rs`・判断を出すなら `handler_tests.rs`）か。中継の比較テストに `name` なしの行も足すか（要件は `name` が数の 1 行の差し替えだけ）。
8. **`doc/ssp-mcp/transport-diff-areka.md` に行を足すか**: 受け口と SSP の差の一覧に「形の違う `params`」の行が無い。SSP の答えは測らない（範囲外）ので、足すなら SSP の欄は未実測。
9. **件 2 の確かめの道具**: 案 R1（手で回す・コードを足さない）か案 R2（`#[ignore]` の実走のテストを 1 本足す・⑥ と写しが重複）か。要件 2.7 の中継ごしの 1 回も同じ道具で送る。
10. **バルーンを出した状態の作り方と見分け方**: `sakurascript`・`raise_event` が未実装なので起動の台詞を待つ。何を見て「出た」とするか（記録の行・`get_log`・`dump_balloon` が絵を返したこと自体）を手順に書く。

### 要件の討議の結論（2026-10-18・全議題を推奨案で承認）

- 議題 1 → 要件 1.1 を「`params` が欠かせないメソッド（`tools/call`・`initialize`・`completion/complete`）」に言い換え、受け流す側（`tools/list` など）は今のまま（要件 1.6・範囲外）。案 D は採らない。
- 議題 2 → `params` がオブジェクトでない要求は範囲外の既知の限界（要件の Out of scope）。
- 議題 4 → `completion/complete` は受け付ける側に入れる（案 A）。
- 議題 9 → 案 R1（手で回す台本・コードとテストのファイルを足さない・要件 3.2）。
- 議題 3・5・6・7・8・10 は設計の段で決める。

### 調べもの（Research Needed）

- `2026-07-28` の札と `_meta` を揃えた形の違う `initialize` が `on_custom_request` に届くこと（静的には届く・測っていない）。
- debug 版で claudia と emo2 の起動の台詞までにかかる時間（呼ぶ時刻の決め方）。
- 再現したときの止まり場所の切り分けに要る記録（段の移りの記録が今は無い）と、`RUST_LOG` の開け方。
- 実機の確かめの間の中継の控えの置き場（`AREKA_PROFILE_DIR`）と、areka 本体の記憶の置き場を `target\` の下へ向ける具体の手順。

### 並走との重なり（C9）

- 件 1 が触る `crates/areka-mcp/src/handler.rs` を同じウェーブで触る spec は無い（`mcp-strict-errors` は後のウェーブ）。`mcp-kanade-tools`（③）・`mcp-reload`（④）は `crates/areka-mcp/**` と `crates/areka/src/mcp/mod.rs` に触らない約束。
- 件 2 で直しが要ったときの `crates/areka/src/mcp/dump_balloon.rs`・`dump_surface.rs`・`mod.rs` も、③ ④ の約束の外（③ が触るのは `get_status`・`sakurascript`・`raise_event` の 3 ファイル）。
- `real-run-mcp-kit`（⑥）の持ち物の 3 ファイル（`crates/areka/tests/{emo2_real_run,mcp_get_log_real_run,smoke_boot_loop_exit}.rs`）には触らない。

### 次の段

`/kiro-requirements-discussion areka-P0-mcp-receiver-bugs` で上の議題 1・2・4（要件の字面に関わるもの）を先に決め、その後 `/kiro-design areka-P0-mcp-receiver-bugs`。

---

## 10. 設計の段の調べと決めたこと（2026-10-18）

- 調べの種類: **軽い調べ（既存の作りへの足し）**。外の資料は引かず、rmcp 3.5.0 の元のソース（`c:\rust\cargo\registry\src\…\rmcp-3.5.0`）と今のコードを読み、1 点だけ測った。
- 使ったスキルの決まり: `kiro-spec-design` の `design-principles.md`・`design-discovery-light.md`・`design-synthesis.md`・`design-review-gate.md`。

### 10.1 測ったこと: `2026-07-28` の札と `_meta` を揃えた形の違う `initialize`

- 道具: `target\gap-probe\src\bin\init_probe.rs`（ワークツリーの `target\` の下・git の外）。本番の 11 本を登録した受け口に、`MCP-Protocol-Version: 2026-07-28`・`Mcp-Method: initialize`・`params._meta` を揃えて送った。`cargo run --offline -q --bin init_probe` で同じ結果が出る。
- 結果:

| 要求 | 答え |
|---|---|
| `initialize` `{"protocolVersion":7}`（札と `_meta` を揃える） | 404・`{"code":-32601,"message":"initialize"}`（rmcp の既定の `on_custom_request` の答え＝handler まで届いている） |
| `initialize` `{}`（同） | 404・同じ |
| `initialize` 形の正しいもの（同） | 200・成功（`protocolVersion` は `2025-11-25`） |
| 旧式（札も `_meta` も無し）の `{"protocolVersion":7}` | 200・`-32601` `"initialize"` |
| `2026-07-28` の未登録の名前の `tools/call`（比べる相手） | 400・`{"code":-32602,"message":"tool not found"}` |
| `2026-07-28` の `completion/complete` `{"ref":7}` | 404・`-32601` `"completion/complete"` |

- 分かったこと: 3 つのメソッドはどれも、5 つの版のどの話し方でも `on_custom_request` に届く。直した後の `-32602` は rmcp の `jsonrpc_http_status`（`tower.rs` 646 行）で `2026-07-28` は 400、旧式は 200 になり、未登録の名前の答えと同じ状態になる（要件 1.4）。

### 10.2 rmcp の公開面（使うもの）

- `ServerHandler::on_custom_request(&self, CustomRequest, RequestContext<RoleServer>) -> Result<CustomResult, ErrorData>`（`handler/server.rs` 553 行）。既定は `ErrorData::new(ErrorCode::METHOD_NOT_FOUND, method, None)`。
- 名前の定数: `CallToolRequestMethod::VALUE == "tools/call"`（4238 行）・`InitializeResultMethod::VALUE == "initialize"`（1055 行）・`CompleteRequestMethod::VALUE == "completion/complete"`（3631 行）。トレイト `ConstString`（79 行）が `VALUE` を持つ。
- 通知は `on_custom_notification`（594 行）の道で、`on_custom_request` には来ない。

### 10.3 預かった議題の結論（理由の詳細は design.md の「設計で決めたこと」）

- 議題 3 → 表は版を見ない（3 つの名前だけ）。
- 議題 5 → 文は `invalid params for <メソッドの名前>`・`data` なし。テストは答えの全体を逐語に固定する。
- 議題 6 → 判断と `debug` の記録を純粋な関数 `custom_request_error` に置き、テストのスレッドで `capture` で数える。
- 議題 7 → 実ソケットのテストは `server_protocol_tests.rs` に追記、純粋な関数のテストは新しい兄弟 `handler_tests.rs`。中継の比較テストは 1 行の差し替えだけ。
- 議題 8 → `transport-diff-areka.md` には足さない。
- 議題 10 → 起動の台詞でバルーンが出るのを待ち、`dump_balloon` が初めて絵を返したことで「出た」と見る。台本は `verification/debug-dump-balloon.ps1`、記録は `verification/debug-dump-balloon.md`。

### 10.4 まとめ方の 3 つの見方（synthesis）

- 一般化: 3 つのメソッドの直しは「型のない要求への答え」1 か所に集まる。表 1 つと関数 1 つで足り、メソッドごとの処理は作らない。
- 作るか使うか: rmcp の拡張の口（`on_custom_request`）・名前の定数・HTTP の状態の決め方をそのまま使う。本文を自分で読み直す道（案 D）と、serde の理由の文を取り直す道（案 C）は採らない。
- 削ったもの: 表の版の分け方・`params` の中身を読む処理・実ソケットのテストの新しいファイル・`transport-diff-areka.md` の行・実走のテストのファイル。どれも今の要件に無い備え。

### 10.5 危うさと手当て

- rmcp を上げて型のない要求へ落ちる道が変わる → 追記したテストが 5 つの版で赤になる。`Cargo.toml` の注記どおり `server_*tests.rs` を全部通す。
- 起動の台詞でバルーンが出ないと件 2 の条件が作れない → 台本が終了コード 2 で分ける。
- debug 版は遅く、他のセッションの重いビルドでフレームが遅れうる → 上限の答えが出たら 3.4 の調べで、記録から止まった段を読み分ける。
- 自動終了を待つので、1 体あたり約 6〜7 分かかる → 回す前に伝える。

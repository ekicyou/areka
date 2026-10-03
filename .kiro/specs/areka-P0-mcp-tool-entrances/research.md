# ギャップ分析: areka-P0-mcp-tool-entrances

> 2026-10-03・本ブランチ（`9c2c5a08`＝main `d4f9e93d` の上に spec の初期化 1 コミット）で実物を読んで書いた。コードは「何の定義か」（型名・関数名＋ファイル）で指す。
> 本書は情報と選択肢を並べるもので、どれを採るかは要件ディスカッションと設計で決める。

## 1. 要約

- **土台はほぼ揃っている。** `areka-mcp` に登録口（`ToolRegistry`・`ToolSpec`・`ToolHandler`・`ToolOutcome`）、アプリ本体側に「受け口を World に置いて毎フレーム汲む」定石（`ChangeRx`／`drain_change_requests`）と「返事の器」（`areka_actor::reply_channel`）がある。足りないのは、10 本の定義・引数の検査・名前の解決・橋（要求の種類の列挙と待ち）・一覧の並びの 5 つ。
- **登録口の形を広げずに `-32602` を出せる道がある。** rmcp 3.5.0 は、ツールの処理が `Err(ErrorData::invalid_params(..))` を返すとそのまま JSON-RPC の `-32602` にする（文言が `failed to deserialize parameters:` で始まるときだけ `isError` の結果へ変える）。`handler.rs` の写しの中で、登録した `inputSchema` を見て検査すれば、`ToolHandler` の型も既存テストも変えずに済む。
- **一覧の並びは `handler.rs` の `list_tools` 1 か所で直せる。** `ToolRouter::list_all` が名前の辞書順に並べ替えるので、登録順の `Tool` の列を `ArekaHandler` が別に持って返す。
- **待ちの形が最大の分かれ目。** MCP 側の tokio に宣言されている機能は `rt`・`net`・`time` だけ（`sync` は rmcp 経由で入っているが自分では宣言していない）。`Cargo.toml` を触れないので、「`spawn_blocking`＋`recv_timeout`」「`try_recv` を短い間隔で覗く」「rmcp 経由の `tokio::sync::oneshot`」のどれかを選ぶ。
- **規模 M・リスク 中。** 新しい技術は無いが、起動の順（MCP が World より先）・終了の途中・待ちの上限を 10 秒待たずに確かめる形・`main.rs` の行数（954 行・上限 1,000 行）に気を配る。

## 2. いまあるもの（実物）

### 2.1 `areka-mcp`（2,393 行・14 ファイル）

| もの | 何をしているか | 本 spec との関係 |
|---|---|---|
| `registry.rs` の `ToolSpec` | name・title・description・inputSchema（`serde_json::Map`）を逐語で持つ | 10 本の定義をそのまま詰められる（要件 1.2・1.4） |
| `registry.rs` の `ToolHandler` | `Arc<dyn Fn(serde_json::Value) -> ToolFuture + Send + Sync>`。返すのは `ToolOutcome`（`content` と `is_error`）だけ | JSON-RPC のエラーを返す道が無い（要件 2）。§4.1 の選択肢で扱う |
| `registry.rs` の `ToolContent::Image` | `data`（base64 済みの文字列）と `mime_type` | 要件 7.2 ⑷ の画像の形は作れる。**base64 にする道具はワークスペースに無い**（§3.3） |
| `handler.rs` の `ArekaHandler::new` | 登録表を `ToolRouter` へ 1 度だけ写す。`ToolRoute::new_dyn` の処理の中で `ctx.arguments`（無ければ `{}`）を渡す | 引数の検査を差し込むならここ（§4.1 案 A） |
| `handler.rs` の `list_tools` | `self.router.list_all()` をそのまま返し、`2026-07-28` 以降には `ttlMs: 0`・`cacheScope: private` を足す | 並べ替わりを直すのはここ 1 か所（要件 1.3・1.5） |
| `handler.rs` の `INSTRUCTIONS` | 「this build registers none, so tools/list is empty」 | 書き換える（要件 8.4）。既存テストは「空でない」と「定数と一致」しか見ないので、文を変えても赤にならない |
| `server.rs` の `start` / `run` | `spawn_actor("mcp")` の中で tokio の `current_thread` を `enable_all()` で作り、接続ごとに `tokio::spawn` | 接続ごとに別のタスク＝1 件の待ちを非同期にすれば他の接続は止まらない（要件 6.3） |
| `testkit.rs`（`#[cfg(test)]`） | 手書きの HTTP/1.1 クライアント・`serve(registry)`・`post_rpc` | 実ソケットのテスト（要件 1.6・2.8・5.4・6.8）はこれで書ける。**クレートの外（`crates/areka`）からは使えない** |
| `Cargo.toml` | rmcp `=3.5.0`（`macros`・`base64` を切る）・tokio `rt`/`net`/`time`・`serde_json` | 変えられない（要件 7.5） |

### 2.2 rmcp 3.5.0 の振る舞い（`C:\rust\cargo\registry\src\index.crates.io-…\rmcp-3.5.0` を読んだ）

- `ToolRouter::list_all`（`src/handler/server/router/tool.rs`）: `tools.sort_by(|a, b| a.name.cmp(&b.name))`＝名前の辞書順。
- `ToolRouter::call`: 未登録の名前は `ErrorData::invalid_params("tool not found", None)`＝`-32602`。処理が `Err` を返したときは `into_tool_argument_error` を通り、**`code == -32602` かつ `message` が `failed to deserialize parameters:` で始まるときだけ** `CallToolResult::error`（`isError: true`）へ変える。それ以外の `Err` はそのまま JSON-RPC のエラーになる。
- `Tool`（`src/model/tool.rs`）: `title`・`outputSchema`・`annotations`・`icons`・`_meta` は `skip_serializing_if = "Option::is_none"`＝値が無ければ出ない（要件 1.4 は今の写し方のままで満たせる）。
- `ContentBlock::image(data, mime_type)`: base64 の文字列をそのまま運ぶ（符号化はしない）。
- 無状態版の `tools/call` は `Mcp-Name` ヘッダの照合と `Mcp-Param-*` の検査に `get_tool` を使う（今の `ArekaHandler::get_tool` のまま）。

### 2.3 アプリ本体（`crates/areka`）

| もの | どこ | 本 spec との関係 |
|---|---|---|
| 起動の順 | `main.rs` の `fn main()`: `resolve_boot` → **`areka_mcp::start(.., ToolRegistry::default())`** → `WinApp::with_exit_policy` → `ghost_session::register_systems` → … → `boot_first_ghost` → `app.run()` | MCP は World より先に立つ。登録表と送り口は `start` の時点で要る（要件 6.5） |
| `_mcp` の落ちる順 | `app` より先に宣言＝`app` の後に落ちる（コメントに明記） | 終了の途中（`run()` の後〜`app` の破棄まで）は MCP が受け付けたまま World の系が回らない（§3.5） |
| `main.rs` の行数 | 954 行（`tests/file_length_guard_test.rs` の上限 1,000 行） | `main.rs` に足すのは数行まで。組み立ては `crates/areka/src/mcp/` の関数へ寄せる必要がある |
| 汲む系の登録 | `ghost_session.rs` の `register_systems`（`ghost_switch::register_change_drain` など 10 本を並べる・866 行） | ここへ 1 行足す形が既にある（要件 7.4） |
| 毎フレーム汲む定石 | `emo2_boot/ghost_switch.rs` の `ChangeRx(Receiver<…>)`（`insert_non_send`）と `drain_change_requests`（`try_iter` で全件取り出し・`Input` 段の `dispatch_pointer_events` の後） | 本 spec の「汲む仕組み」の手本。受け口が無ければ無操作 |
| ゴーストの置き場 | `ghost_session.rs` の `GhostSlot(pub(crate) Option<GhostSession>)` | 「起動中のゴースト」の読み元 |
| 名前 | `GhostSession::names()` → `Option<&GhostNames>`（実行系が無ければ `None`）。`GhostNames` は `areka-parsers/src/package/model.rs`（`name`・`sakura_name`・`sakura_name2`・`kero_name`、どれも `Option<String>`） | 要件 3.2・4.1・4.2 |
| ルートフォルダ | `GhostSession::ghost_dir()` → `&Path`（`ghost/<フォルダ名>`）。元は `boot_config` の根（`absolute` で絶対化・`canonicalize` しない） | 要件 3.3・4.2。絶対パスであることは設計で確かめる |
| LogSink へ倒れた起動 | `ghost_session.rs` の倒れ先の `GhostSession`: 窓への結線は無いが実行系（`ghost`）は `Some` のことがある | 要件 3.8 の定義（実行系が起きている）では「起動中」に数わる |
| テスト用の組み立て | `GhostSession::for_test(kanade, ghost_dir)`: 実行系は `None`＝`names()` は常に `None` | World 越しに「名前あり」を作れない（§3.6） |
| 返事の器 | `areka-actor/src/reply.rs`: `reply_channel`（std の mpsc 1 本）・`ReplySender::send`（1 回だけ）・`ReplyReceiver::{recv, recv_timeout, try_recv}`・落とすと `ReplyError::Dropped` | 要件 6.4 の「手放した」は `Dropped` でそのまま分かる |
| UI スレッドへ即時に届ける器 | `areka-actor/src/ui.rs` の `spawn_ui`／`UiSender`（async-channel・UI スレッドの executor で汲む）。使っているのは `areka-emo-text/src/actor.rs` の `spawn_emo_text` | 届くのは速いが、受け手は World を持たない（§4.3 案 2） |
| World を借りる非同期の前例 | `main.rs` の煙の自動終了（`spawn_local`＋`Rc::downgrade(&app.world())`）・`menu/trigger.rs` の `finish`（`try_borrow_mut`・借りられなければ記録して終える） | メニュー表示中も tick は入れ子で回る（`finish` の doc）＝World を外から借りる形は「借りられない」場合を必ず持つ |
| tick の門 | `wintf/src/ecs/world/tick_gate.rs`・`tick_wake.rs`。既定は無効（毎フレーム回る）。`AREKA_TICK_GATE=1` で有効にすると旗が無い回は省くが、心拍（`TICK_HEARTBEAT_FRAMES = 30`）で必ず回る | 毎フレーム汲む形の最悪の遅れは、門が無効なら 1 フレーム、有効でも約 30 フレーム（60Hz で約 0.5 秒）＝10 秒の上限には十分収まる |
| 依存 | `crates/areka/Cargo.toml` に `serde_json`・`async-channel`・`tokio`・base64 は無い。`areka-mcp`・`areka-actor`・`async-io` はある | アプリ本体側は JSON の値を扱えない＝引数は型の付いた値（`String`・`i64`・`bool`・`Vec<String>`）で渡す必要がある |

### 2.4 保存した SSP の定義（`doc/ssp-mcp/tools-list-ssp-2.9.05.json`）

`{"tools": [...]}` の 10 本。各ツールの欄は `name`・`title`・`description`・`inputSchema` の 4 つだけ。並びと引数:

| 並び | 名前 | 引数（型） | `required` |
|---|---|---|---|
| 1 | `get_active_ghost_list` | なし | なし |
| 2 | `get_status` | `ghost_name`（string） | なし |
| 3 | `get_expression_table` | `ghost_name`（string） | `["ghost_name"]` |
| 4 | `get_property` | `property_name`・`ghost_name`（string） | `["property_name"]` |
| 5 | `get_log` | `log_type`・`ghost_name`（string）・`since_id`・`max_count`（integer） | なし |
| 6 | `sakurascript` | `script`・`ghost_name`（string）・`strict`（boolean） | `["script"]` |
| 7 | `raise_event` | `event`（string）・`references`（array）・`ghost_name`（string）・`strict`（boolean） | `["event"]` |
| 8 | `reload` | `target`・`ghost_name`（string） | `["target"]` |
| 9 | `dump_surface` | `scope`・`surface`（integer）・`ghost_name`（string） | なし |
| 10 | `dump_balloon` | `scope`（integer）・`ghost_name`（string） | なし |

型は string・integer・boolean・array（要素は string）の 4 種だけ＝小さな検査で足りる。

## 3. 要件ごとの対応表（足りないもの・分からないもの・縛り）

凡例: **足りない**＝新たに作る／**分からない**＝設計で調べる／**縛り**＝既存の約束から来る制約

| 要件 | いまあるもの | ギャップ |
|---|---|---|
| 1.1〜1.4 定義の逐語 | `ToolSpec` が逐語を持てる・rmcp は空の欄を出さない | **足りない**: 10 本の定義。**縛り**: 本番は `doc/` を読めない（7.6）＝定義をクレートの中に持つ（Rust の文字列か、クレート内に置いた JSON の写しを `include_str!`）。`include_str!("../../../doc/…")` はクレートの外を読むので不可（`crates-io-publish` で包に入らない） |
| 1.3・1.5 並び | `list_tools` が `list_all`（辞書順）を返す | **足りない**: 登録順の列を `ArekaHandler` に持たせて返す（1 か所） |
| 1.6 一致のテスト | `testkit.rs` の実ソケット | **足りない**: テストだけが `doc/ssp-mcp/tools-list-ssp-2.9.05.json` を読んで比べる（`CARGO_MANIFEST_DIR` から辿る） |
| 2.1 未知の名前 | rmcp が `-32602` | 足りている（10 本を登録すれば残りの名前は rmcp が拒む） |
| 2.2〜2.7 欠落・型違い | なし | **足りない**: 検査。**縛り**: `ToolHandler` は `ToolOutcome` しか返せない。§4.1 |
| 2.3 `get_expression_table` の例外 | なし | **足りない**: 「`ghost_name` の欠落は `-32602` にしない」の 1 規則。`ghost_name` を `required` に持つのは 10 本中この 1 本だけなので、検査の規則として「`ghost_name` は常に欠落を許す」と書けば名前で分岐しなくて済む |
| 2.7 HTTP の状態 | 旧式の経路は 200（`unregistered_name_is_invalid_params` で測った） | **分からない**: 無状態版の経路で `-32602` が何番の状態で返るか（測って差の一覧へ・8.3） |
| 3.1〜3.9 名前の解決 | 名前（`names()`）とルートフォルダ（`ghost_dir()`）は読める | **足りない**: 解決の判断（純粋な関数）。**分からない**: SSP の「フルパス」に末尾の区切りが付くか（要件は差を同じとみなすので影響は小さい）。`ghost_dir()` が常に絶対パスか |
| 3.8 起動中の定義 | 実行系の有無は `names()`／`runtime()` で分かる | **縛り**: LogSink へ倒れた起動は実行系があれば「起動中」に数わる（窓は無い）。意図どおりかは要件ディスカッションで確かめてもよい |
| 4.1〜4.5 一覧 | 同上 | **足りない**: 一覧の処理。**縛り**: `for_test` では名前ありの単位を作れない（§3.6） |
| 5.1〜5.4 ダミー | なし | **足りない**: 9 本の処理（アプリ本体側）。**分からない**: 実ソケットから World のダミーまで通す試験をどのクレートに置くか（§3.7） |
| 6.1〜6.8 橋 | 毎フレーム汲む定石・返事の器 | **足りない**: 要求の種類の列挙（10 本）・送り口・汲む系・待ち・上限・終了の扱い。**縛り**: tokio の `sync` を宣言していない（§3.2）・`test-util` が無いので時計を止められない（§3.4） |
| 6.5 準備の前 | MCP は World より先に立つ | std の mpsc なら、受け口を World に置く前に送った要求は溜まり、置いた後の最初のフレームで汲まれる＝自然に満たせる。`spawn_ui` は送り口が UI スレッドの準備の後にしかできない（§4.3） |
| 6.4 終了の途中 | `ReplyError::Dropped` | **分からない**: `run()` が戻ってから `app` が落ちるまで（降ろす・印・告知）は系が回らないので、要求は上限まで待つか、World が落ちて `Dropped` になるまで待つ。§3.5 |
| 7.2 結果の 4 形 | `ToolContent`・`ToolOutcome` | **足りない**: 4 つの小さな関数。**分からない**: ⑷ の画像を base64 済みの文字列で受けるか PNG のバイト列で受けるか（§3.3） |
| 7.3 干渉台帳 | roadmap の C2・C3 の行と台帳に「`mcp-tool-entrances` の設計が固定する」と書かれている | **足りない**: 設計で 7 spec 分の触るファイルを決めて書く |
| 7.4 触るファイル | — | **縛り**: `main.rs` は 954 行。`ghost_session.rs` は 1 行。`handler.rs`・`registry.rs` を触るのは「登録口と handler の改め」の範囲 |
| 8.2 既存テスト | `registry_tests.rs`・`server_gate_help_tests.rs` は `ToolHandler`／`ToolOutcome`／`ToolSpec` を構造体の字面で作る | **縛り**: これらの型に欄を足す・型を変えると、既存テストがコンパイルできなくなる（§4.1 で避けられる） |
| 8.3 差の一覧 | 「必須引数の欠落は本 spec では測れない」と書いてある行 | **足りない**: 測り直して書き換える |
| 8.5 実機 | `mcp-server-core` の実機確認の手順（`claude mcp add --transport http`・`info!` の実番号） | 手順は流用できる |

### 3.1 定義を持つ場所

保存した JSON の逐語をクレートの中に持つ方法は 2 つ。

- **(a) ツールのファイルごとに Rust の生文字列（`r#"{…}"#`）で持ち、起動時に `serde_json` で読む。** 3 段目の各 spec は自分のファイルだけを見る。ただし定義は本 spec で確定し、3 段目は定義を変えない（SSP の逐語なので）。
- **(b) クレートの中に JSON の写しを 1 つ置き（例 `crates/areka-mcp/src/tools/ssp-2.9.05.json`）、`include_str!` で読む。** 並びも JSON の順がそのまま使える。`doc/` の原本と写しが一致することはテストが確かめる。

どちらでも 3 段目の spec は定義に触らない。(b) は並びの正本が 1 つで済み、(a) は「ツールのファイルを開けば定義も引数の型も見える」。

### 3.2 tokio の機能の縛り

`crates/areka-mcp/Cargo.toml` の tokio は `rt`・`net`・`time` だけ。`tokio::sync::oneshot` は `sync` が要る。rmcp 3.5.0 が tokio を `sync`・`macros`・`rt`・`time` で引くので、機能の合算で今は使えるが、**自分で宣言していない機能に頼る**ことになる（rmcp の版を上げて依存の形が変わると壊れうる）。`Cargo.toml` を触らない約束なので、宣言を足す道は無い。

### 3.3 base64

rmcp の `base64` 機能は切ってあり、ワークスペースの直接の依存にも base64 の箱は無い。要件 7.2 ⑷ の「画像が続く形」を本 spec で作るとき、関数が受け取るのが base64 済みの文字列なら符号化は要らない（`ToolContent::Image` と同じ）。PNG のバイト列を受けるなら、本 spec か `mcp-dump-images` が符号化を持つ（依存を足すなら `Cargo.toml`、足さないなら数十行の自前の符号化）。

### 3.4 上限を 10 秒待たずに確かめる

tokio の `test-util`（時計を止める機能）は宣言されていない。上限の長さを橋を組むときの引数にして、本番は 10 秒・テストは数十ミリ秒を渡す形が要る（要件 6.8）。

### 3.5 終了の途中

`main.rs` の終了の流れは `app.run()` → `exit_wait::begin_close` → `after_run`（置き場から取り出して降ろす）→ … → `app` の破棄 → `_mcp` の破棄。

- `run()` が戻った後は World の系が回らないので、汲む系に頼る形では、その間に届いた要求は「上限まで待つ」か「World が落ちて受け口が落ち、`Dropped` になる」まで待つ。
- 要件 6.4 は「終了の途中で受けられない」ときに上限を待たないことを求める。満たすには、終了を始めたところ（`begin_close` の前後）で受け口を落とすか、閉じた印を送り口側に立てる手当てが要る。手当ての場所が `main.rs` か `exit_wait.rs` か `crates/areka/src/mcp/` の中かで、触るファイル（要件 7.4）に響く。

### 3.6 World 越しの試験の縛り

`GhostSession::for_test` は実行系を持たない＝`names()` が常に `None`。名前ありの場合を World 越しに試すには、`for_test` を広げる（`ghost_session.rs` を 1 行より多く触る）か、解決と一覧の判断を「名前とルートフォルダの組」を受ける純粋な関数に閉じ、World から組を作る部分は薄い配線に留めて、名前ありの場合は純粋な関数の側で試す（要件 3.9 がこの形を求めている）。

### 3.7 端から端までの試験の置き場

ダミーの処理はアプリ本体側（`crates/areka/src/mcp/`）にある。一方、実ソケットの手書きクライアント（`testkit.rs`）は `areka-mcp` の中のテスト専用。

- **(a)** `areka-mcp` のテストは「偽の受け手」（テストの中で要求を受けて決まった返事をするスレッド）で橋を確かめ、`crates/areka` のテストは World と汲む系とダミーをソケット無しで確かめる（登録した処理の `ToolFuture` を直接呼ぶ）。2 つを合わせて端から端までとみなす。
- **(b)** `crates/areka` の側にも小さな HTTP の送り手を置いて、実ソケットからダミーまで通す（手書きのクライアントが 2 つになる）。
- 要件 5.4 の「9 本それぞれについて実ソケットの `tools/call` が `NG:not implemented yet` を返す」をどちらで満たすかは設計で決める。

## 4. 作り方の選択肢

### 4.1 `-32602` を出す道（要件 2）

| 案 | 中身 | 良い点 | 気になる点 |
|---|---|---|---|
| **A. `handler.rs` の写しの中で、登録した `inputSchema` を見て検査する** | `ArekaHandler::new` の `ToolRoute::new_dyn` の処理で、`ctx.arguments` を `spec.input_schema` に照らし、違えば `Err(ErrorData::invalid_params(..))` を返す。検査は「`required` の欠落 → `properties` の型」の 2 段（名前は rmcp が先に見る）。`ghost_name` の欠落は常に許す | `ToolHandler`・`ToolOutcome`・`ToolSpec` の型が変わらない＝既存テストは無改変（`echo_args` の試験は `text` を渡し、余計な欄は無視されるので通る）。定義の逐語が検査の正本を兼ねる | 検査が汎用になり、登録したどのツールにも効く（今の `echo_args` にも効く）。`message` を `failed to deserialize parameters:` で始めないこと（始めると rmcp が `isError` の結果へ変える） |
| B. 登録口に「検査つきの登録」を足す | `ToolRegistry::register_checked(spec, check, handler)` のように別の口を足し、`check` が `Result<型の付いた引数, 理由>` を返す | ツールごとに型の付いた引数を作るところまでを 1 か所で書ける | 登録口が 2 つになる。`entries()` の形が変わり `registry_tests.rs` の `entries()[0]` の分解が壊れうる |
| C. `ToolHandler` の返り値を `Result<ToolOutcome, InvalidParams>` に変える | 処理の中で検査して `Err` を返す | 素直 | `registry_tests.rs`・`server_gate_help_tests.rs` の処理の字面が全部変わる＝要件 8.2 に反する（変えるならテスト名と理由を設計に書く） |

A と B の組（検査は A の汎用、型の付いた引数への詰め替えは各ツールのファイル）もありうる。A なら検査を通った後の詰め替えは失敗しないので、各ツールのファイルは「JSON の値 → 型の付いた引数」を数行で書ける。

### 4.2 返事を待つ形（要件 6.2・6.3）

MCP の tokio は 1 本のスレッドの上で接続ごとにタスクを回している。待ちを非同期にしないと他の接続まで止まる。

| 案 | 中身 | 良い点 | 気になる点 |
|---|---|---|---|
| **a. `tokio::task::spawn_blocking` の中で `ReplyReceiver::recv_timeout(上限)`** | 宣言済みの `rt` だけで足りる | 既存の器（`reply_channel`）をそのまま使う・`Timeout` と `Dropped` を区別できる | 待つ間は tokio の待機用スレッドを 1 本使う（`spawn_actor` を通らないので wintf のスレッド名簿に載らない）。終了の取り消しの後も、待っているスレッドは上限まで残る |
| b. `ReplyReceiver::try_recv` を短い間隔（例 5〜10 ms）で覗き、`tokio::time::sleep` で間を空ける | 宣言済みの `time` だけで足りる・スレッドを足さない | 単純・取り消しで即座に止まる | 返事の遅れが覗く間隔だけ増える。間隔ごとに起きる |
| c. `tokio::sync::oneshot`＋`tokio::time::timeout` | 送り手を要求に同梱し、UI 側が `send` する | 最も素直な非同期の待ち | §3.2 の「宣言していない `sync` に頼る」。`areka-actor` の `ReplySender` の流儀（std の mpsc）と別の器になる。アプリ本体側は tokio を依存に持たないが、送り手の型（`tokio::sync::oneshot::Sender`）を `areka-mcp` の公開面に出すことになる（「公開面に tokio の型は出さない」という `lib.rs` の方針に反する）＝包む型が要る |

### 4.3 アプリ本体へ届ける形（要件 6.1・6.5・6.6）

| 案 | 中身 | 良い点 | 気になる点 |
|---|---|---|---|
| **1. std の mpsc を `main.rs` で作り、受け口を World の資源（`NonSend`）に置いて、`Input` 段の系が毎フレーム `try_iter` で汲む**（`ChangeRx` と同じ形） | 送り口は `areka_mcp::start` に渡す登録表の処理が握る | 既存の定石そのもの。World の借用は系の中なので借りられない場合が無い。World の準備の前に届いた要求は溜まって、置いた後に汲まれる（6.5 が自然に満たせる）。メニュー表示中も tick は回る | 遅れは最大 1 フレーム（門が有効なら心拍まで約 0.5 秒）。終了の途中は系が回らない（§3.5） |
| 2. `areka_actor::spawn_ui` の受け手が `Rc<RefCell<EcsWorld>>` の `Weak` を握り、届くたびに `try_borrow_mut` で World を借りる | 煙の自動終了・メニューの `finish` と同じ借り方 | 届いてすぐ処理する | 送り口（`UiSender`）は UI スレッドで `WinApp` の後にしか作れない＝MCP が先に立つので、「後から差し込む送り口」が要る。借りられないとき（入れ子の tick の最中など）の扱いを決める必要がある（待たずに捨てると誤報、積み直す口は無い） |
| 3. 1 と 2 の組: 受け口は 1 と同じ std の mpsc、汲むのは系 | — | — | 1 と変わらない |

### 4.4 ファイルの分け方（要件 7.1・7.3・7.4）

プロトコル側とアプリ本体側でツールごとにファイルを分ける形の一例（設計で固定する）:

- **`crates/areka-mcp/src/tools/`**（新規）
  - `mod.rs`: 10 本の表（並び・定義の読み込み・登録表の組み立て）・要求の種類の列挙（10 の変種と、各変種が持つ型の付いた引数と返事の送り手）・結果の 4 形の関数・橋（送る・待つ・上限・`Dropped`）。
  - 検査（汎用・§4.1 案 A なら `handler.rs` 側）と名前の解決の判断（純粋な関数。アプリ本体側に置いてもよい）。
  - `get_active_ghost_list.rs` ほか 10 本: 各ツールの「型の付いた引数」と「JSON の値からの詰め替え」。3 段目は値の範囲の検査（`log_type` の未知の値など）をここに足せる。
- **`crates/areka/src/mcp/`**（新規）
  - `mod.rs`: 受け口の資源・汲む系・`register`（`ghost_session::register_systems` から 1 行で呼ばれる）・`main.rs` から呼ぶ組み立て（送り口と登録表を返す）。
  - `resolve.rs`（または `areka-mcp` 側）: World から「起動中のゴースト」の組（名前・ルートフォルダ）を作る薄い配線。
  - `get_active_ghost_list.rs`・`get_status.rs` ほか 10 本: アプリ本体側の処理（9 本はダミー）。3 段目は自分のファイルの中身だけを書く。
- 要求の種類の列挙を `areka-mcp` に置く理由: `areka-mcp` は `crates/areka` に依存できない（向きが逆）。アプリ本体側は `areka-mcp` の型を `match` するだけ。
- 3 段目の spec ごとの対応（設計で台帳へ）: `mcp-get-property`＝`get_property` の 2 ファイル＋`areka-ghost/src/runtime.rs`／`mcp-kanade-tools`＝`get_status`・`sakurascript`・`raise_event` の 6 ファイル＋kanade／`mcp-expression-table`＝`get_expression_table` の 2 ファイル＋表情の表／`mcp-log-history`＝`get_log` の 2 ファイル＋tracing の履歴／`mcp-reload`＝`reload` の 2 ファイル＋読み直しの経路／`mcp-dump-images`＝`dump_surface`・`dump_balloon` の 4 ファイル＋emo の読み戻し／`mcp-strict-errors`＝`sakurascript`・`raise_event` の 2 ツールのファイル（`mcp-kanade-tools` の後なので直列）。

### 4.5 全体の案

- **案 A（既存を広げる）**: `handler.rs` に検査と並びを入れ、`registry.rs` は変えず、橋とツールは `tools/` に、アプリ本体側は `crates/areka/src/mcp/` に置く。届ける形は 4.3 の 1（毎フレーム汲む）、待ちは 4.2 の a か b。
  - 良い点: 既存テストは無改変・既存の定石のまま・`Cargo.toml` 0 行。
  - 気になる点: 検査が汎用＝今後どのツールにも効く（意図どおりなら良い）。
- **案 B（新しい登録口）**: `registry.rs` に検査つきの登録を足し、ツールごとの型の付いた検査を各ファイルに持つ。
  - 良い点: ツールごとの型が 1 か所で閉じる。
  - 気になる点: 登録口が 2 つ・既存テストの書き換えが出うる・検査の規則がツールのファイルに散らばり、3 段目が検査を触る余地が増える（要件 7.1 の「検査を本 spec で揃える」と逆向き）。
- **案 C（組み合わせ）**: 検査は案 A の汎用（定義が正本）、詰め替えはツールごとのファイル、届ける形は毎フレーム汲む、待ちは a か b。今の材料ではこれが最も筋が良く見えるが、決めるのは設計。

## 5. 規模とリスク

- **規模: M（3〜7 日・brief の見立て 13〜17 タスク）**。新しい仕組みは小さな検査と橋だけで、残りは既存の定石の写し。10 本ぶんのファイルとテストの数が多い。
- **リスク: 中**。技術は既知だが、⑴ 待ちの形（tokio の機能の縛り）、⑵ MCP が World より先に立つ順と終了の途中、⑶ `main.rs` の行数の余り（46 行）、⑷ 実ソケットからダミーまでの試験の置き場、の 4 つで設計の判断が要る。

## 6. 設計へ持ち越す調べもの

1. 無状態版（`2026-07-28`）の経路で、`tools/call` の `-32602` が何番の HTTP 状態で返るか（要件 2.7・8.3。実ソケットで測る）。
2. `ghost_dir()` が常に絶対パスか（`boot_config` の `absolute` を通った根から作られるか）。SSP の「フルパス」に末尾の区切りが付くか（照合は差を同じとみなすので影響は小さいが、一覧に出す形は決める）。
3. 4.2 の a〜c のどれで待つか。a ならスレッド名簿に載らない待機用スレッドを許すか、b なら覗く間隔、c なら宣言していない `sync` に頼ることと公開面の包み方。
4. 終了の途中（`run()` の後）の要求に、上限を待たず `NG:areka is shutting down` を返す手当ての場所（§3.5）。
5. 端から端までの試験の置き場（§3.7）。
6. 結果の 4 形のうち画像の形が受け取るもの（base64 の文字列か PNG のバイト列か・§3.3）。
7. tick の門を有効にしたときの遅れ（心拍で約 0.5 秒）を許すか、送ったときに旗を立てるか（旗を立てる場合は wintf の旗の持ち主の一覧と、それを照合する `tick_gate_config_producers_tests.rs` に響く＝要件 7.4 の範囲の外）。

## 7. 要件ディスカッションへ送る議題の候補

要件の暫定の裁定（`requirements.md` の表 13 行）で覆す理由になりうるものだけを挙げる。

1. **終了の途中の扱い（暫定の裁定 12・要件 6.4）**: 「上限を待たずに `NG:areka is shutting down`」を満たすには、終了を始める所（`main.rs`／`exit_wait.rs`）にも手が入りうる。要件 7.4 の触るファイルの一覧に `exit_wait.rs` が無い。手当ての場所が `crates/areka/src/mcp/` の中で閉じない場合、要件 7.4 を広げるか、終了の途中は「World が落ちるまで待つ（最長でも上限）」で許すかを決める。
2. **LogSink へ倒れた起動は「起動中」か（要件 3.8）**: 窓への結線が無くても実行系が起きていれば一覧に出て、ツールの処理が届く。3 段目のツール（`dump_surface` など）は窓が無いと答えられないが、それは各 spec が `NG:` で返せばよい、で済むか。
3. **待ちの上限を 10 秒・10 本共通とする裁定（暫定の裁定 1）**: 橋の作りとしては上限を引数にできる（テストのためにも必要）。3 段目でツールごとに変える余地を橋の形として先に残すかどうか（残すなら要求の種類ごとの上限の表を本 spec で持つ）。
4. **検査を定義（inputSchema）から汎用に引く形でよいか（要件 2）**: 汎用にすると、`ghost_name` の欠落の例外（要件 2.3）は「`ghost_name` は常に欠落を許す」という 1 規則になり、名前での分岐が要らない。ツールごとの手書きの検査にする場合より 3 段目が検査を触る余地が減る（要件 7.1 の狙いに沿う）。利用者から見える振る舞いは同じ。
5. **要件 5.4 の「実ソケットで 9 本」**の範囲: 実ソケットの試験が「偽の受け手」までで、アプリ本体のダミーはソケット無しの試験で確かめる形（§3.7 (a)）で要件を満たしたとみなせるか。満たさないなら、`crates/areka` 側に HTTP の送り手を持つ（試験の部品が 2 つになる）。
6. **結果の 4 形の画像（要件 7.2 ⑷）**: 本 spec の関数が base64 済みの文字列を受ける形でよいか（符号化は `mcp-dump-images` が持つ）。PNG のバイト列を受けるなら、本 spec で符号化を持つ（`Cargo.toml` を触らないので自前の数十行）。

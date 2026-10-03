# Requirements Document

> 本文の実測は **2026-10-03・本ブランチ**（main `d4f9e93d`＝`status-execution-states` の PR#220 のコミット。`mcp-server-core`〔PR#219〕の着地の後）のもの。コードは「何の定義か」（関数名・型名・定数名＋ファイルパス）で指し、行番号では指さない。
> 「要件の段での暫定の裁定」の表は、brief が「要件で決める」とした議題と、要件を書く途中で答えが要った点に対する**推奨案による暫定の確定**であり、要件ディスカッションで覆せる（覆したら該当要件も改める）。答えで作業が変わる議題は見込み 2 件（待ちの上限・ダミーの文言）。

## Project Description (Input)

**誰の何が困っているか**: SSP MCP の移植の 3 段目（ツールの中身の 7 spec＝`mcp-get-property`・`mcp-kanade-tools`・`mcp-expression-table`・`mcp-log-history`・`mcp-reload`・`mcp-dump-images`、その後の `mcp-strict-errors`）を並走させたい。並走できるかは「各 spec が自分のファイルだけを触る」形が先にできているかで決まる。ツールの表・引数の検査・`ghost_name` の解決・アプリ本体への橋を各 spec がそれぞれ足すと、同じファイル（振り分けの表・要求の種類の列挙）を全員が触って毎回 rebase になる。AI エージェント（Claude Code など）の利用者から見ても、いまの areka は `tools/list` が空で、SSP と同じツールの名前を呼べない。

**今の状態**: `mcp-server-core` の着地で、areka は `127.0.0.1` で MCP の受け口を開き、`tools/list` は 0 本・`tools/call` はどの名前でも `-32602` を返す。ツールを登録する口（`crates/areka-mcp/src/registry.rs` の `ToolRegistry`）だけがあり、`fn main()` は空の登録表を渡している。アプリ本体（UI スレッドの World）へ問い合わせて返事を待つ道は無い。

**何を変えるか**: `tools/list` が SSP 2.9.05 と同じ 10 本（名前・title・description・inputSchema が逐語で一致）を返す。`tools/call` は引数の型と必須の欄を検査し（違えば `-32602`）、`ghost_name`（ゴースト名またはルートフォルダのフルパス）を起動中のゴーストへ解決し（失敗は SSP と同じ `NG:` の文言）、アプリ本体へ要求を届けて返事を待つ（上限を超えたら `NG:` で返してログを 1 行）。`get_active_ghost_list` だけは本物で、残り 9 本は `NG:not implemented yet` を返すダミー。3 段目の各 spec が「自分のツールのファイル」と「自分の触るエンジン」だけを触れば中身を書ける形にし、その配置を roadmap の干渉台帳に書く。

> 起票: 2026-09-29 `/kiro-discovery`（開発者指示「基本実装 → 空のダミー関数を置いて入り口だけ全部整備 → 個別のコマンド実装」の 2 段目）。事実の正本は [doc/ssp-mcp/survey.md](../../../doc/ssp-mcp/survey.md)（SSP 2.9.05 の実測）、ツール定義の逐語は [doc/ssp-mcp/tools-list-ssp-2.9.05.json](../../../doc/ssp-mcp/tools-list-ssp-2.9.05.json)、SSP と areka の輸送の差は [doc/ssp-mcp/transport-diff-areka.md](../../../doc/ssp-mcp/transport-diff-areka.md)。全体の並びは `.kiro/steering/roadmap.md`「SSP MCP の移植」節（本 spec は **C2-④**）。

## Introduction

### 誰が困っているか

- **AI エージェントでゴーストを作る人**: SSP では `get_active_ghost_list` で起動中のゴーストを知り、その名前を `ghost_name` に渡して他のツールを呼ぶ。areka は `tools/list` が空なので、エージェントは何も呼べない。本 spec で中身が入るのは `get_active_ghost_list` だけだが、10 本の名前と引数の形は SSP と同じになり、未実装のものは「未実装」と分かる文言で返る。
- **3 段目の spec の実装者**: 自分のツールの中身を書くために、表・検査・名前の解決・アプリ本体への橋を毎回自分で足すことになる。並走する他の spec と同じファイルを奪い合う。

### いま何が起きているか（2026-10-03 実測）

- **登録口の形。** `crates/areka-mcp/src/registry.rs` の `ToolSpec`（name・title・description・inputSchema を逐語で持つ）と `ToolHandler`（引数の JSON を受けて `ToolOutcome`＝`content` と `isError` を返す非同期の関数）。`crates/areka-mcp/src/handler.rs` の `ArekaHandler::new` が登録表を rmcp の `ToolRouter` へ 1 度だけ写す。**実装の関数は `ToolOutcome` しか返せず、JSON-RPC のエラー（`-32602`）を返す道が無い**＝引数の検査の `-32602`（要件 2）は登録口の形を広げないと出せない（設計の議題）。
- **一覧の並び。** rmcp 3.5.0 の `ToolRouter::list_all` は名前の辞書順に並べ替える（`rmcp-3.5.0/src/handler/server/router/tool.rs` の `list_all`）。登録表に積んだ順でも SSP の順でもない（要件 1.3 で SSP の順を求める）。rmcp の `Tool` は `outputSchema`・`annotations`・`icons`・`_meta` を値が無いとき書き出さない（同じファイル `model/tool.rs` の `Tool` の `skip_serializing_if`）＝4 つの欄だけの定義は作れる。
- **`instructions` の文。** `crates/areka-mcp/src/handler.rs` の `INSTRUCTIONS` は「this build registers none, so tools/list is empty」と書いており、本 spec の着地で嘘になる（要件 8.4）。
- **アプリ本体の起動の順。** `crates/areka/src/main.rs` の `fn main()` は `areka_mcp::start` を `WinApp`（World）を作るより**前**に呼ぶ。MCP の要求は World ができる前にも届きうる（要件 6.5）。
- **ゴーストは 1 体だけ。** `crates/areka/src/ghost_session.rs` の `GhostSlot(Option<GhostSession>)`。名前は `GhostSession::names()`（`areka-parsers` の `GhostNames`＝descript の `name`・`sakura.name`・`sakura.name2`・`kero.name`。どれも `Option`）、ルートフォルダは `GhostSession::ghost_dir()`（`ghost/<フォルダ名>`）。実行系が無い（起動に失敗した）ときは `names()` が `None`。
- **アプリ本体へ届けて返事を待つ定石**（`areka-actor` の慣行）: ⑴ mpsc の受け口を World の資源に置き、登録したシステムが毎フレーム汲む（`crates/areka/src/emo2_boot/ghost_switch.rs` の `ChangeRx`・`drain_change_requests`、`crates/areka/src/install/desk.rs` の `InstallDesk::drain`）。⑵ `areka_actor::spawn_ui`／`UiSender`（UI スレッドへ即時に届ける・`crates/areka/src/placement/follow.rs` ほか）。返事は `areka_actor::reply_channel` の `ReplySender`／`ReplyReceiver`（落とすと `ReplyError::Dropped`・待ちは `recv_timeout`）。`ReplyReceiver` は待つとスレッドを塞ぐ＝MCP のスレッドの tokio（`current_thread` 1 本）の中で待つと他の要求まで止まる（要件 6.3）。どれを使うかは設計の議題。
- **依存の形。** `crates/areka/Cargo.toml` に `serde_json` は無い（`areka-mcp` の 1 行と `areka-actor` はある）。`crates/areka-mcp/Cargo.toml` の rmcp は既定機能（`macros` を含む）を切っている。本 spec は `Cargo.toml` を触らない（要件 7.5）。

### brief の記述を実物で引き直して改めた点

1. brief の「定義を rmcp の `#[tool]` マクロで一致させられるかは design で確かめる」は、本 spec では選べない。`areka-mcp` の rmcp は `macros` 機能を切っており、入れるには `Cargo.toml` を変える必要がある（本 spec は `Cargo.toml` を触らない約束＝同じ C2 の `crates-io-publish` が全部の `Cargo.toml` を触る）。定義は保存した JSON の逐語を手で詰める（登録口はもともと逐語を渡せる形＝`mcp-server-core` 要件 7.4）。
2. brief と survey に無かった事実: rmcp の一覧は名前の辞書順に並べ替わる。SSP の順にするには一覧を areka 側で返す必要がある（要件 1.3・設計の議題）。
3. brief の「必須引数の欠落は `-32602`」と「名前の要るツールで省略すれば `NG:Specified ghost is not active`」は、`get_expression_table` で衝突する（inputSchema では `ghost_name` が `required` なのに、SSP は省略を `-32602` でなく `NG:` で返した〔survey §3〕）。SSP の実測に合わせ、**`ghost_name` の欠落だけは検査の `-32602` から外して名前の解決の `NG:` へ回す**（要件 2.3・3.4）。
4. brief の「`ghost_name` の解決」は全ツール共通と読めるが、`get_log` の `ghost_name` は「このゴーストの記録だけ」という**絞り込み**で、description も「target」と書かない（もう起動していないゴーストの記録を絞る使い方もありうる）。本 spec では `get_log` の `ghost_name` を解決せず、型だけ検査してそのまま中身へ渡す（要件 3.7・意味は `mcp-log-history` が決める）。

### 要件の段での暫定の裁定（要件ディスカッションで覆せる）

| # | 議題 | 暫定の裁定 | 根拠 | 載せた要件 |
|---|---|---|---|---|
| 1 | 返事を待つ上限（brief の議題） | **10 秒・10 本で共通**。超えたら `NG:areka did not respond within 10 seconds`（`isError: true`）と `warn!` 1 件 | UI スレッドはゴーストの切替などで数秒塞がりうるので短すぎると誤報が出る。MCP のクライアントの待ちはこれより長い。3 段目でツールごとに長さを変える要が出たら、その spec の要件で改める | 6.2・6.3 |
| 2 | ダミーの文言（brief の議題） | **`NG:not implemented yet`**（`isError: true`） | brief の案のまま。SSP の文言の型（`NG:` ＋英文）に揃える | 5.1 |
| 3 | 定義を逐語から手で詰めるかマクロか（brief の議題） | **手で詰める**（マクロは選べない＝改めた点 1） | `Cargo.toml` を触らない約束 | 1.2・7.5 |
| 4 | 汲み方（毎フレームか `spawn_ui` か）（brief の議題） | **要件では決めない**（設計の議題）。要件は「上限の内に返事が来る・他の要求を止めない・UI を止めない」だけを求める | 利用者から見える振る舞いが変わらない | 6 |
| 5 | 一覧の並び | **SSP の並び**（保存した JSON の順＝`get_active_ghost_list` が先頭） | SSP の description は「先に `get_active_ghost_list` を使え」と書き、並びもそれに沿う。並べ替える手間は小さい | 1.3 |
| 6 | `ghost_name` の照合 | **descript の `name` と完全一致**（大文字小文字も区別）**か、ルートフォルダ（`ghost/<フォルダ名>`）のフルパスと一致**（パスは大文字小文字・区切り〔`\` と `/`〕・末尾の区切りの差を同じとみなす）。`sakura.name` などの別名では照合しない | SSP の description は「ghost name or full path of its root folder」の 2 つだけを挙げる（ukadoc のプロパティシステムの `activeghostlist(…)` は本体側名でも引けるが、MCP の約束はそれより狭い） | 3.2・3.3 |
| 7 | `ghost_name` が空の文字列 | **省略と同じ**に扱う | 空の名前のゴーストは無い | 3.4・3.5 |
| 8 | 引数の値が `null` | **省略と同じ**に扱う（必須の欄が `null` なら欠落＝`-32602`） | 寛容に読み、意味のある値の型違いだけを拒む | 2.4 |
| 9 | inputSchema に無い欄 | **無視する**（拒まない） | SSP の inputSchema は `additionalProperties` を書いていない＝規格上は許される | 2.5 |
| 10 | ゴーストが起動していないときの一覧 | **空の本文**（`isError: false`） | 一覧に偽の名前を載せない。SSP の振る舞いは未測（survey §5） | 4.3 |
| 11 | descript に `name` が無いゴースト | 一覧に**ルートフォルダのフルパス**を出す | 一覧は「`ghost_name` にそのまま渡せる値」の約束（description）なので、渡せる値を出す | 4.2 |
| 12 | アプリ本体が終了の途中で答えられない | **上限を待たずに** `NG:areka is shutting down`（`isError: true`）と `warn!` 1 件 | 終了の途中で 10 秒待たせない | 6.4 |
| 13 | `get_log` の `ghost_name` | **本 spec では解決しない**（型だけ検査して渡す） | 改めた点 4 | 3.7 |

## Boundary Context

- **In scope**:
  - `tools/list` の 10 本（SSP 2.9.05 の逐語・SSP の並び）と、保存した JSON との一致のテスト。
  - `tools/call` の引数の検査（ツール名・必須の欄・型）と `-32602`。
  - `ghost_name` の解決（ゴースト名またはルートフォルダのフルパス）と、SSP と同じ失敗の文言。
  - アプリ本体（UI スレッドの World）へ要求を届けて返事を待つ道（要求の種類を 10 本ぶん揃える・汲む仕組み・待ちの上限・終了の途中の扱い）。
  - 結果の共通の形（素の値・`OK:`・`NG:`・画像）を作る小さな関数。
  - `get_active_ghost_list` の中身と、残り 9 本のダミー。
  - `instructions` の文の改め（ツール 0 本と書かない）と、SSP との差の一覧（`doc/ssp-mcp/transport-diff-areka.md`）の「必須引数の欠落」の行の測り直し。
  - 3 段目の各 spec が触るファイルの配置を設計で固定し、roadmap の干渉台帳に書くこと。
- **Out of scope**:
  - 残り 9 本の中身（`get_status`・`get_expression_table`・`get_property`・`get_log`・`sakurascript`・`raise_event`・`reload`・`dump_surface`・`dump_balloon`）＝3 段目の各 spec。値の範囲や列挙の検査（`log_type` の未知の値・`reload` の未知の `target`・存在しない `scope`／`surface` など、SSP が `NG:Unknown …` を返すもの）も 3 段目。
  - kanade・sylphya・emo・tracing の履歴への新しい問い合わせ口（3 段目の各 spec）。
  - 複数ゴーストの同時起動（予約「多重ゴースト」）。一覧は最大 1 体。
  - `strict` の働き（`mcp-strict-errors`）。本 spec は `strict` の型だけを検査する。
  - Claude Desktop 用の中継（`mcp-stdio-bridge`）・待受・ポート・`Origin`／`Host`・help（`mcp-server-core` の振る舞いは変えない）。
  - SSP の欠陥 2 件（survey §4）＝中身を書く 3 段目の spec が扱う。
  - `Cargo.toml` の変更（0 行）。
- **Adjacent expectations**:
  - `mcp-server-core` の決定論テスト（`crates/areka-mcp/src/` の `server_*_tests.rs`・`gate_tests.rs`・`port_tests.rs`・`registry_tests.rs`・`help_tests.rs`）は、登録口の形を広げても変えずに緑のまま（空の登録表で 0 本を返す振る舞いもテストの中で残る）。登録口の形を変えてテストの書き換えが要るなら、その行を設計で名指しする。
  - ゴーストの切替（`emo2_boot/ghost_switch.rs`）と `GhostSession` の中身は読むだけで、切替の経路を変えない。同じ C2 の `shell-balloon` が `emo2_boot/` を触るので、`ghost_session.rs` で触るのは汲む仕組みの登録の 1 か所だけにとどめる。
  - 同じ C2 の `crates-io-publish` が `areka-mcp` の `publish` を決める。`areka-mcp` の本番のビルドはクレートの外のファイル（`doc/ssp-mcp/` の JSON など）を読まない（テストは読んでよい）。
  - 3 段目の各 spec は、本 spec が固定した「自分のツールのファイル」と「自分の触るエンジン」だけを触り、表・検査・名前の解決・橋・要求の種類の列挙を触らない（触る要が出たら、その spec の要件で理由を書く）。
  - `get_log` の `ghost_name` の意味（絞り込みの照合の仕方）は `mcp-log-history` が決める。

## Requirements

### Requirement 1: `tools/list` が SSP と同じ 10 本を返す

**Objective:** As a AI エージェントでゴーストを作る人, I want areka の `tools/list` が SSP と同じ名前・説明・引数の形の 10 本を返すこと, so that SSP 向けに書いた手順やプロンプトがそのまま areka でも通じる

#### Acceptance Criteria

1. When `tools/list` が届く, the areka shall 10 本のツール（`get_active_ghost_list`・`get_status`・`get_expression_table`・`get_property`・`get_log`・`sakurascript`・`raise_event`・`reload`・`dump_surface`・`dump_balloon`）を返す（11 本目 0 本・欠け 0 本）。
2. The areka shall 各ツールの `name`・`title`・`description`・`inputSchema` を `doc/ssp-mcp/tools-list-ssp-2.9.05.json` の値と JSON の値として一致させる（description の英文・`"type": "integer"` の欄・`required` の並びを含めて逐語。object の欄の順は問わない）。
3. The areka shall 10 本を `doc/ssp-mcp/tools-list-ssp-2.9.05.json` と同じ並びで返す（先頭は `get_active_ghost_list`）。
4. The areka shall 各ツールに `name`・`title`・`description`・`inputSchema` の 4 つ以外の欄を付けない（`outputSchema`・`annotations`・`icons`・`_meta` は 0 個）。
5. When 無状態版（`2026-07-28`）の `tools/list` が届く, the areka shall `mcp-server-core` 要件 3.15 のとおり `ttlMs`・`cacheScope` を付け、10 本の中身と並びは旧式の版の `tools/list` と同じにする。
6. The areka shall 1.1〜1.4 を、保存した JSON を読んで実ソケットの `tools/list` の応答と比べる決定論テストで固定する（1 本でも違えば赤）。

### Requirement 2: 引数の検査（`-32602`）

**Objective:** As a AI エージェントの利用者, I want 名前の誤りや引数の欠け・型違いがツールの結果でなく規格どおりのエラーで返ること, so that エージェントが自分の呼び方の誤りだと分かって直せる

#### Acceptance Criteria

1. If `tools/call` の名前が 10 本のどれでもない, then the areka shall JSON-RPC のエラー `-32602`（結果の `isError` ではない）で答え、アプリ本体へ要求を届けない。
2. If 必須の欄（`get_property` の `property_name`・`sakurascript` の `script`・`raise_event` の `event`・`reload` の `target`）が無い, then the areka shall JSON-RPC のエラー `-32602` で答え、アプリ本体へ要求を届けない。
3. When `get_expression_table` の `ghost_name` が無い, the areka shall それを引数の欠落（`-32602`）とせず、名前の解決（要件 3.4）へ回す（inputSchema の `required` に載っているが、SSP の実測〔survey §3〕に合わせる）。
4. If 欄の値の型が inputSchema の型と違う（`"string"` の欄に文字列でない値・`"integer"` の欄に整数でない数〔`1.5` など〕や数でない値・64 ビットの符号付き整数に収まらない数・`"boolean"` の欄に真偽でない値・`"array"` の欄〔`references`〕に配列でない値や文字列でない要素）, then the areka shall JSON-RPC のエラー `-32602` で答え、アプリ本体へ要求を届けない。値が `null` の欄は省略と同じに扱う（必須の欄が `null` なら 2.2 の欠落）。
5. When inputSchema に無い欄が届く, the areka shall その欄を無視し、拒まない。
6. When `arguments` が無い, the areka shall 空の引数として扱う（必須の欄のあるツールでは 2.2 の欠落）。
7. The areka shall 検査を「名前 → 必須の欄 → 型」の順に行い、最初に見つけた誤りで `-32602` を返す（エラーの `message` は rmcp の形のままでよく、SSP の文言〔`Invalid params`〕との差は差の一覧に書く）。HTTP の状態コードは rmcp のまま（旧式の経路は 200・無状態版の経路は rmcp が決める値）とし、測った値を差の一覧に書く。
8. The areka shall 検査の通す／拒む条件を、10 本それぞれについて「欄が全部ある・必須の欄が無い・型違い（欄ごとに 1 つ以上）・`null`・余計な欄」の組で、ソケットを通さない決定論テストに固定する。実ソケットでも、未知の名前・必須の欄の欠落・型違いの 3 つを 1 本ずつ固定する。

### Requirement 3: `ghost_name` の解決

**Objective:** As a AI エージェントの利用者, I want `get_active_ghost_list` で得た名前（またはゴーストのフォルダのフルパス）で対象のゴーストを指せること, so that SSP と同じ呼び方でゴーストを選べ、誤った名前は SSP と同じ文言で分かる

#### Acceptance Criteria

1. When `ghost_name` を受けるツール（`get_status`・`get_expression_table`・`get_property`・`sakurascript`・`raise_event`・`reload`・`dump_surface`・`dump_balloon`）の `tools/call` が検査（要件 2）を通る, the areka shall `ghost_name` を起動中のゴーストへ解決してから、そのツールの処理へ届ける。
2. When `ghost_name` が起動中のゴーストの descript の `name` と完全に一致する（大文字小文字も区別）, the areka shall そのゴーストへ解決する。
3. When `ghost_name` が起動中のゴーストのルートフォルダ（`ghost/<フォルダ名>`）のフルパスと一致する（大文字小文字・区切り〔`\` と `/`〕・末尾の区切りの有無の差は同じとみなす）, the areka shall そのゴーストへ解決する。`sakura.name`・`kero.name` などの別名・フォルダ名だけ・相対パスでは解決しない。
4. If `get_expression_table` の `ghost_name` が無い・空の文字列である, then the areka shall 本文 `NG:Specified ghost is not active`・`isError: true` の結果で答え、ツールの処理へ届けない。
5. When `ghost_name` が任意のツール（3.1 のうち `get_expression_table` 以外の 7 本）で `ghost_name` が無い・空の文字列である, the areka shall 起動中のゴースト（1 体）へ解決する。ゴーストが起動していなければ、本文 `NG:Specified ghost is not active`・`isError: true` で答え、ツールの処理へ届けない。
6. If `ghost_name` が空でなく、起動中のどのゴーストにも 3.2・3.3 のとおり一致しない（ゴーストが起動していない場合を含む）, then the areka shall 本文 `NG:Cannot find active ghost from specified name`・`isError: true` の結果で答え、ツールの処理へ届けない。
7. The areka shall `get_log` の `ghost_name` を解決せず（3.4〜3.6 の `NG:` を出さない）、型を検査した値のままツールの処理へ届ける（意味は `mcp-log-history` が決める）。
8. The areka shall 「起動中のゴースト」を、ゴーストの実行系が起きているあいだのゴーストとする。起動に失敗して既定ゴーストへも戻れなかったとき、切替の途中で前のゴーストを降ろし次のゴーストがまだ起きていないあいだは、起動中のゴーストは 0 体である。
9. The areka shall 解決の判断を、ゴーストの名前とルートフォルダの値を受け取る（World や実行系を要しない）判断として持ち、次の各場合を決定論テストで固定する: 名前の一致・大文字小文字だけ違う名前（不一致）・フルパスの一致・大文字小文字と区切りと末尾の区切りの違うフルパス（一致）・フォルダ名だけ（不一致）・`sakura.name`（不一致）・空の文字列・省略・ゴーストが 0 体（省略と名前ありの両方）・`get_expression_table` の省略。

### Requirement 4: `get_active_ghost_list` が起動中のゴーストを返す

**Objective:** As a AI エージェントの利用者, I want 起動中のゴーストの名前を一覧で得られること, so that その名前を他のツールの `ghost_name` にそのまま渡せる

#### Acceptance Criteria

1. When `get_active_ghost_list` が呼ばれ、ゴーストが起動している, the areka shall 本文をそのゴーストの descript の `name`（1 行・末尾の改行なし）・`isError: false` の結果で答える（例 `Emily/Phase4.5`。`OK:` を付けない素の値＝survey §3 の形）。
2. If 起動中のゴーストの descript に `name` が無い, then the areka shall 本文にそのゴーストのルートフォルダのフルパスを載せる（`ghost_name` にそのまま渡せる値）。
3. When `get_active_ghost_list` が呼ばれ、ゴーストが起動していない（要件 3.8）, the areka shall 空の本文・`isError: false` で答える。
4. The areka shall 一覧に載せた値を `ghost_name` に渡すと、要件 3 の解決で同じゴーストへ解決されるようにする（一覧と解決は同じ「起動中のゴースト」を見る）。
5. The areka shall 4.1〜4.4 を決定論テストで固定する（名前あり・`name` 無し・0 体・一覧の値で解決できること）。

### Requirement 5: 残り 9 本はダミーとして答える

**Objective:** As a AI エージェントの利用者, I want 中身の無いツールを呼んだとき「未実装」と分かる結果が返ること, so that 失敗の理由を取り違えず、エージェントが別の手を選べる

#### Acceptance Criteria

1. When 検査（要件 2）と名前の解決（要件 3）を通った `get_status`・`get_expression_table`・`get_property`・`get_log`・`sakurascript`・`raise_event`・`reload`・`dump_surface`・`dump_balloon` の `tools/call` が届く, the areka shall アプリ本体のそのツールの処理へ届け、本文 `NG:not implemented yet`・`isError: true` の結果で答える。
2. The areka shall ダミーの処理でゴーストに何もさせない（台本の再生・イベントの送出・読み直し・画像の取得は 0 回）。
3. The areka shall ダミーの処理に、検査を通った引数（型の付いた値）と解決したゴーストを渡す（3 段目は処理の中身だけを書けばよい形）。
4. The areka shall 9 本それぞれについて、実ソケットの `tools/call` が `NG:not implemented yet`・`isError: true` を返すことを決定論テストで固定する。

### Requirement 6: アプリ本体へ届けて返事を待つ

**Objective:** As a areka を常駐させている利用者, I want MCP の要求がゴーストの描画や会話を止めず、areka が答えられないときも決まった時間で返事が来ること, so that エージェントが固まらず、ゴーストもいつもどおり動く

#### Acceptance Criteria

1. When 10 本のどれかの `tools/call` が検査を通る, the areka shall 要求をアプリ本体（UI スレッドのゴーストの状態）へ届け、その返事で答える（ゴーストの状態を MCP のスレッドから直接読まない＝ゴーストの切替で古い状態を掴まない）。
2. If 要求を届けてから 10 秒のうちに返事が来ない, then the areka shall 本文 `NG:areka did not respond within 10 seconds`・`isError: true` の結果で答え、`warn!` を 1 件残す（ツール名と上限を載せる）。上限を過ぎてから届いた返事は捨てる（応答を 2 度返さない）。
3. While 1 件の `tools/call` が返事を待っている, the areka shall 他の接続の要求（`ping`・`tools/list`・別の `tools/call`）に待たずに答える（MCP のスレッドを返事の待ちで塞がない）。
4. If アプリ本体が終了の途中で要求を受けられない・受けた要求に答えずに手放した, then the areka shall 10 秒を待たずに本文 `NG:areka is shutting down`・`isError: true` で答え、`warn!` を 1 件残す。
5. When アプリ本体の準備ができる前（起動の直後）に要求が届く, the areka shall 準備ができてから処理して答える（準備が 10 秒のうちに済まなければ 6.2 のとおり）。
6. While 要求を処理している, the areka shall ゴーストの描画・台詞の再生・メニュー・クリック透過の付け外しを止めない（UI スレッドで返事を待たない＝UI スレッドがするのは、ゴーストの状態の読み取り・名前の解決・ツールの処理の呼び出しと返事の送り出しだけ）。
7. When `tools/call` に答える, the areka shall `debug!` を 1 件残し、ツール名・解決の結果（解決したゴーストの名前か、失敗の文言）・`isError` を載せる。
8. The areka shall 6.2〜6.5 を決定論テストで固定する（返事をしない受け手で上限の `NG:` と `warn!` 1 件・待ちの間の `ping` が答えること・受け手を落とすと上限を待たずに `NG:areka is shutting down`・準備の前に届いた要求が準備の後に答えること）。上限の長さを実際に 10 秒待たずに確かめられる形にする。

### Requirement 7: 3 段目の各 spec が並走できる形

**Objective:** As a 3 段目の spec の実装者, I want 自分のツールのファイルと自分の触るエンジンだけを触れば中身を書けること, so that 並走する他の spec と同じファイルを奪い合わずに済む

#### Acceptance Criteria

1. The areka shall 10 本それぞれについて、プロトコル側（定義・引数の型）とアプリ本体側（処理）のファイルをツールごとに分け、表・検査・名前の解決・橋・要求の種類の列挙を本 spec で全 10 本ぶん揃える（3 段目がこれらを触らずに済む）。
2. The areka shall 結果の共通の形を作る小さな関数を持ち、次の 4 つの形を作れるようにする: ⑴ 素の値（`OK:` なし・`isError: false`）、⑵ `OK:` ＋付言（`isError: false`）、⑶ `NG:` ＋理由（`isError: true`）、⑷ 本文の後に画像（`type: "image"`・base64 の PNG・`mimeType: "image/png"`）が続く形。4 つを決定論テストで固定する。
3. The areka shall 設計の段で、3 段目の 6 spec（`mcp-get-property`・`mcp-kanade-tools`・`mcp-expression-table`・`mcp-log-history`・`mcp-reload`・`mcp-dump-images`）と `mcp-strict-errors` が触るファイルをツールごとに固定し、`.kiro/steering/roadmap.md` の干渉台帳（C3 の照合の要点を含む）に書く。
4. The areka shall 本 spec で触るファイルを、`crates/areka-mcp/src/`（ツールのファイル・登録口と handler の改め・定義の一致のテスト）・新しい `crates/areka/src/mcp/` の下・`crates/areka/src/ghost_session.rs`（汲む仕組みの登録の 1 か所）・`crates/areka/src/main.rs`（登録表と送り口を渡す）・`doc/ssp-mcp/transport-diff-areka.md`・`.kiro/steering/roadmap.md` に限る（ほかを触る要が出たら止めて報告する）。
5. The areka shall `Cargo.toml`（根・`crates/areka-mcp`・`crates/areka` のいずれも）を変えない（変える行 0・新しい依存 0）。
6. The areka shall `areka-mcp` の本番のビルドでクレートの外のファイルを読まない（保存した JSON を読むのはテストだけ）。

### Requirement 8: 決定論テスト・差の一覧・実機確認

**Objective:** As a 3 段目の spec の実装者と後で rmcp の版を上げる開発者, I want 入り口の振る舞いがテストで固定され、SSP との差が一覧に残ること, so that 中身を足したときや版を上げたときに何が変わったかが赤で分かる

#### Acceptance Criteria

1. The areka shall 常時テスト（`cargo test`・`tools/test-all.ps1`）でネットへ出ず（ループバックのみ・OS が割り当てる空きポート）、9801・9821 その他の固定の番号を束ねない（0 本）。
2. The areka shall `mcp-server-core` の決定論テストを変えずに緑のまま通す（変える要が出たら、そのテスト名と理由を設計に書く）。
3. The areka shall `doc/ssp-mcp/transport-diff-areka.md` の「未知のツール名・必須引数の欠落」の行に、本 spec で測った必須の欄の欠落と型違いの振る舞い（エラーの番号・`message`・HTTP の状態コード〔旧式の経路と無状態版の経路〕）と測ったテストの名前を書き、判定を改める（空欄 0）。
4. The areka shall `initialize`・`server/discover` の `instructions` を、ツールが 0 本だと書かない英文（1〜3 文）に改める（内容は設計で決める）。
5. The areka shall 実機で確かめ、結果を `.kiro/specs/areka-P0-mcp-tool-entrances/verification/signoff.md` に残す: ⑴ 配布形の `areka.exe` を既定ゴースト（emo2）で起動し、Claude Code（`claude mcp add --transport http`）で登録して `tools/list` に 10 本が出る、⑵ `get_active_ghost_list` が emo2 の descript の `name` を返す、⑶ その名前とルートフォルダのフルパスの両方を `ghost_name` に渡した `get_status` が `NG:not implemented yet` を返す、⑷ 存在しない名前で `NG:Cannot find active ghost from specified name` が返る、⑸ 呼んでいる間もゴーストの描画と会話が止まらない、⑹ `RUST_LOG` を要件 6.7 の `debug!` が出る階層まで開け、記録に各呼び出しの行が残る。

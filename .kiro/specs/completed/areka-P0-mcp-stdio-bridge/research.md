# ギャップ分析: areka-P0-mcp-stdio-bridge

> 2026-10-10・本ブランチ（main `414d43eb` の上・コミット `5b8a709b`）で、確定済みの `requirements.md` と今のコードを突き合わせた記録。
> 1〜10 章（ギャップ分析と要件ディスカッション）でやったのはファイルを読むことだけ（ビルド・テスト・SSP の中継や偽のサーバーの実行は 0 回）。SSP の中継の振る舞いは `requirements.md` の実測の表と、`target\mcp-stdio-bridge-req\` に残っている入力と出力の記録を読んだ。11 章以降（設計の段）は試しのクレートを git の外に建てて動かした（SSP の中継と机の 9801〜9830 には触れていない）。
> コードは「何の定義か」（関数名・定数名＋ファイル）で指す。rmcp と hyper の振る舞いは、`Cargo.lock` が固定している版（rmcp 3.5.0・hyper 1.11.1）のソースを読んで確かめた。
> この文書は選択肢と根拠を並べるもので、決定ではない。「見立て」と書いた所も、要件ディスカッションと設計で覆せる。

## 1. まとめ

- **中継そのものは 0 から作る。** `crates/` に中継のクレートは無く、標準入力を読んで HTTP で送る本番のコードも無い。使い回せるのは、候補の並びと名乗りの定数（`crates/areka-mcp/src/port.rs`・`handler.rs`）、テスト用の手書きの HTTP クライアント（`crates/areka-mcp/src/testkit.rs`）の書き方、本物の受け口を空きポートで起こす公開の入口（`areka_mcp::start`）、exe の隣のファイルを求める書き方（`crates/areka/src/boot_config.rs` の `default_helper_exe_path`）、配布スクリプトの検査の型（`tools/package.ps1`）。
- **「写すだけ」では 5 つの版のうち `2026-07-28` が通らない（rmcp のソースで確認）。** 本文に版（`params._meta` の `io.modelcontextprotocol/protocolVersion`）を持つ要求は、HTTP の札 `MCP-Protocol-Version` が無いと rmcp が 400・`-32020` で断る。札の版が `2026-07-28` なら `Mcp-Method`（`tools/call` では `Mcp-Name` も）が要る。stdio には札が無いので、中継が行の中身（`method`・`params.name`・`params._meta`）を読んで札を作る必要がある。これが「MCP の中身の解釈に当たるか（＝公式 SDK を使う話になるか）」は開発者の判断が要る（議題 1）。
- **同じウェーブの約束（`main.rs`・`dispatch.rs`・`server.rs`・`lib.rs` に触らない）を守ると、形が変わる所が 2 つある。** ⑴ help が中継の置き場を知る手段（help の中で exe の場所を自分で求めると、areka 本体が持つ「リンクを辿る」決まりを通らない）、⑵ 名乗りの定数 `SERVER_NAME` の共有（`lib.rs` が外へ出していない）。本質の形で設計するなら触るファイルを 4 章と 5 章に挙げた。
- **areka が居ないときの速さが未確認。** 候補は 20 個あり、誰も居ない番号への接続が Windows でどれだけで断られるかを測っていない。SSP の中継は 1 つの番号だけを見て約 0.1 秒で答える。20 個を順に当たる中継が同じ速さで答えられるかは、設計の最初に測る必要がある（調べ残し 1）。
- **規模は M（3〜7 日）・リスクは中。** 新しい外部のクレートは、手書きの HTTP と `serde_json` で組む案なら 0。公式 SDK の両側（stdio と HTTP クライアント）で組む案は `Cargo.lock` に無い依存が増える。

## 2. いまあるもの

### 2.1 MCP の受け口（完了 `mcp-server-core`）

| 見たもの | 実物 |
|---|---|
| 候補の並び | `crates/areka-mcp/src/port.rs` の `DEFAULT_PORTS`（9801・9821）・`FALLBACK_STEPS`（9）・`PORT_ENV`（`AREKA_MCP_PORT`）は公開（`lib.rs` の `pub use`）。20 個を組む `default_candidates` は非公開だが、公開の `candidates_from_env_value(None)` が同じ 20 個を返す |
| 環境変数の読み方 | `candidates_from_env_value`: 未設定は 20 個・`0` は空の列（待ち受けない）・1〜65535 はその 1 つ（前後の空白は許す）・読めない値は `warn!` を 1 件残して 20 個。記録は `tracing` で出す（受け手が無いプロセスでは何も出ない） |
| 名乗り | `crates/areka-mcp/src/handler.rs` の `SERVER_NAME`（`areka-mcp-server`）。`pub(crate)` で、`lib.rs` は外へ出していない |
| 立て方 | `crates/areka-mcp/src/server.rs` の `start(candidates, registry)` が候補を順に束ね、最初に束ねた 1 つで待ち受ける。要素 `0` は OS に空きポートを割り当てさせる（テスト用）。呼び手は `crates/areka/src/main.rs` の 1 行（`areka_mcp::start(&areka_mcp::read_port_candidates(), mcp_tools)`） |
| 接続の扱い | `server.rs` の `accept_loop` は接続ごとに hyper の `http1::Builder::new()` に `TokioTimer` を渡して回す。hyper 1.11.1 の既定は keep-alive が有効・ヘッダを読む時間切れが 30 秒（`http1::Builder::new` の初期値） |
| 検査 | `crates/areka-mcp/src/gate.rs` の `check`: `Host` はループバック（`127.0.0.1`・`localhost`・`[::1]`）だけ通し、無しも 403。`Origin` は無しかループバックだけ |
| 本文の上限 | `crates/areka-mcp/src/dispatch.rs` の `MAX_BODY_BYTES`（4 MiB）。超えると 413 の平文 |
| ツールが答えるまでの上限 | `crates/areka-mcp/src/tools/mod.rs` の `REPLY_WAIT`（10 秒）。`main.rs` がこの値で受け口を組む |
| ツールの定義に札の印（`x-mcp-header`）| `crates/` に 0 件（全文検索）。あると rmcp は `tools/call` に `Mcp-Param-*` の札も求める |

### 2.2 help

- `crates/areka-mcp/src/help.rs` の `help_html(port)` は番号だけを受け取り、Claude Desktop の節は「中継は今後の版で用意します」の 1 文。説明の注釈は「埋め込む値は数と定数なので HTML のエスケープは要らない」と書いている（パスを載せるとこの前提が崩れる）。
- 呼び手は `crates/areka-mcp/src/dispatch.rs` の `help(port)` の 1 か所で、番号は `State` の `port`（`start` が束ねた実番号）。
- `crates/areka-mcp/src/help_tests.rs` の `help_html_lists_five_items_with_actual_port` は、`claude_desktop_config` の字が**無い**こと・URL がちょうど 3 か所であること・既定の番号（9801・9821）の `127.0.0.1:<番号>` が無いことを確かめている。1 つ目は要件 8.3 で書き換える。残り 2 つは、設定例に URL を載せなければそのまま通る。
- exe の隣を求める前例は `crates/areka/src/boot_config.rs` の `default_helper_exe_path`。これは `exe_location`（起動した exe のリンクを `follow_exe_links` で辿ってから親フォルダを取る・プロセスで 1 回だけ）を使う。`areka-mcp` は `areka` に依存される側なので、この関数を呼べない。

### 2.3 配布

`tools/package.ps1` で中継を入れるために変わる所は **5 か所**（brief は 4 か所と書いていた。組み立ての段の写しが抜けていた）:

1. `$ALLOWED_EXECUTABLES`（zip に入れてよい実行ファイル）。
2. 本体を建てる段（`build $a areka` の `cargo build --locked --release -p areka --target …`）。`-p` を 1 つ足せば、同じ `RUSTFLAGS`（`+crt-static`）で中継も建つ。
3. 段 `check static linking` の `$exes` の表（機械種別と、VC++ ランタイムの DLL を読んでいないことの検査）。
4. `Test-ZipContent` の 3 つの表: 必須の項目（判定 1）・最上位の許可表（判定 2）・機械種別の表 `$machines`（判定 5・6。`*.exe` は取り込み表の検査も受ける＝要件 7.3 の「追加のランタイムを要しない」はここで判定できる）。
5. 段 `$a assemble` の `Copy-Item`（`areka.exe` と同じく CPU 種別ごとの成果物を写す）。

そのほかの事実:

- `.github/workflows/release.yml` は `tools/package.ps1 -Arch all` を呼ぶだけで、実行ファイルの一覧を持たない（変える所 0）。
- `tools/crates-io.ps1` の `Test-Reason` は、出さないクレートの `Cargo.toml` に行頭から `publish = false # 理由` の行を求める。
- 新しいクレートは根の `Cargo.toml` の `members = ["crates/*"]` で自動で入る（根は 0 行）。`THIRD-PARTY-NOTICES.md` はワークスペースのクレートも `- areka-mcp 0.0.2` の形で載せているので、新しいクレートで 1 行増える（`tools/test-all.ps1 -License` の `cargo about generate --workspace` が作り直す）。
- `tools/test-all.ps1` の x64 のテストは `cargo test --workspace`。新しいクレートのテストは自動で入る。
- `dist/README.txt` に「MCP」「9801」「`AREKA_MCP_PORT`」の字は 0 件。本体の見出しは「◆ areka 本体（areka.exe・shiori-host32-helper.exe）」。
- 中継の名前を一覧に持つ他の文書: `release-code-signing` の brief（署名する exe は 2 本と書いている）・`winget-manifest-submission` の brief（入れ子の実行ファイルに中継を入れるかを本 spec からの申し送り待ちにしている）。

### 2.4 テストの型と、新しいクレートに掛かる常設の見張り

- **手書きの HTTP クライアント**: `crates/areka-mcp/src/testkit.rs` の `request`（`Connection: close` で 1 本送り、閉じるまで読んで、`Content-Length` と `Transfer-Encoding: chunked` を解く）。テスト専用（`#[cfg(test)]`）で、ほかのクレートからは使えない。
- **本物の exe を起こす統合テスト**: `crates/areka/tests/mcp_get_log_real_run.rs` が `CARGO_BIN_EXE_areka` を起こし、標準出力と標準エラーをファイルへ向け、置き場を `CARGO_TARGET_TMPDIR`（ワークツリーの `target\tmp`）の下に取っている。
- **本物の受け口をテストで起こす**: `areka_mcp::start(&[0], ToolRegistry::default())` と `McpServer::local_addr` は公開。中継のテストが `areka-mcp` を開発専用の依存にすれば、偽物でない areka の受け口（5 つの版の振る舞いそのもの）を空きポートで相手にできる。
- **一時フォルダ**: `crates/temp-path-kit` の `TempPath::under_target`。`std::env::temp_dir` を窓口の外で呼ぶと、常設の見張り `crates/log-capture-kit/tests/temp_path_guard_test.rs` が赤にする（本番のコードも走査の対象）。
- **1,000 行の番人**（`file_length_guard_test.rs`）と**記録の捕捉の迂回の見張り**（`with_default_guard_test.rs`）もワークスペース全体を走査する。`log-capture-kit` を開発専用の依存に足すクレートは、後者の `KNOWN_DEV_DEPENDENTS` の表に載せる必要がある。
- **テストの置き方**（`.kiro/steering/structure.md`）: テストの本体は本番ファイルの中に書かず、隣の `<stem>_tests.rs` へ置いて `#[path]` でつなぐ。
- **記録**（`.kiro/steering/logging.md`）: 記録は `tracing`、実行ファイルは `tracing-subscriber`。一方で配布の補助 exe（`crates/shiori-host32-helper/src/main.rs`）は `eprintln!` で標準エラーへ書いている。

### 2.5 rmcp 3.5.0 のソースで確かめたこと（札の決まり）

`transport/streamable_http_server/tower.rs` と `transport/common/mcp_headers.rs` を読んだ。

| 送り方 | areka（rmcp）の答え | 出どころ |
|---|---|---|
| 札なし・本文に `_meta` の版なし（旧式の 4 つの版の送り方） | 通る（札が無ければ `2025-03-26` として扱う） | `validate_protocol_version_header`・`is_legacy_request` |
| `initialize`・札なし | 通る（札が在れば本文の版と一致が要る） | `validate_header_matches_init_body` |
| 本文の `_meta` に版が在る・札 `MCP-Protocol-Version` なし | **400・`-32020`**「request _meta protocolVersion requires MCP-Protocol-Version header」 | `validate_request_protocol_version_meta` |
| `server/discover`・`_meta` なし | 400・`-32602`（`_meta` の欠落） | 同上 |
| 札の版が `2026-07-28`・`Mcp-Method` なし | 400・`-32020`「missing required Mcp-Method header」 | `validate_standard_headers` → `validate_request_headers` |
| 札の版が `2026-07-28`・`tools/call`・`Mcp-Name` なし | 400・`-32020`（`params.name` と同じ値が要る。ASCII でない値は `=?base64?…?=` の形でも受ける） | `validate_request_headers`・`extract_name` |
| 道具の定義に `x-mcp-header` の印が在る `tools/call` | `Mcp-Param-*` の札も要る（areka の定義には今 0 件） | `param_header_annotations` |

- 札を**組む**側の関数 `standard_request_headers` は rmcp の中だけの関数（`pub(crate)`・機能 `client-side-sse` の下）で、外からは呼べない。公式 SDK で札を付けるには、HTTP クライアントの輸送（機能 `transport-streamable-http-client`）ごと使うことになる。その輸送は HTTP クライアントの実装を別に要し、付属の実装は `reqwest`（機能 `transport-streamable-http-client-reqwest`）。**`reqwest` は `Cargo.lock` に無い**。stdio の側は機能 `transport-io`（tokio の `io-std`）。
- **増える外部のクレートの数を測った**（2026-10-10・要件ディスカッションの中で。`target\mcp-stdio-bridge-req\deps-probe-*` に、ワークスペースの `Cargo.lock` を写した試しのクレートを置き、`cargo tree -e normal --target x86_64-pc-windows-msvc` の名前を `Cargo.lock` の名前と比べた。建ててはいない）:
  - rmcp の `server`・`client`・`transport-io`・`transport-streamable-http-client-reqwest`（付属の `reqwest` の HTTP クライアント）: `Cargo.lock` に無い名前が **34 個**（`reqwest`・`url`・`idna`・`icu_*` の族・`tower`・`tower-http`・`want` など）。
  - rmcp の `server`・`client`・`transport-io`・`transport-streamable-http-client`（HTTP を送る部分だけ自前で、rmcp の `StreamableHttpClient` の口に合わせて書く）: `Cargo.lock` に無い名前は **0 個**。この形で rmcp の口を満たせるか・exe の大きさは未確認。
- 受け口は JSON の単発応答を優先するが、ツールの処理が途中で通知や要求を出すと SSE に切り替わる（`tower.rs` の無状態の経路の注釈）。areka のツールは今は出さない（0 件）。出すようになると、JSON 1 件だけを読む中継には「JSON でない本文」に見える。

### 2.6 SSP の中継の実測の記録（読んだだけ）

- `target\mcp-stdio-bridge-req\d-nosaved.out.txt`: 9801 に何も居ないとき、入力から出力まで約 0.11 秒（`initialize` は 23 ms に入れて 134 ms に出た・`ping` は 3091 ms に入れて 3193 ms に出た）。
- `target\mcp-stdio-bridge-req\requests.log`: SSP の中継が送る札は `Host`・`Content-Type`・`Accept`・`Content-Length`・`Connection: close` の 5 つだけ。
- 要件の「いま何が起きているか」と実測の表は、この記録と食い違っていない。

## 3. 要件ごとの対応表

印: **無い**＝今のコードに無く作る／**縛り**＝今の実物が設計を縛る／**未確認**＝調べ残し（8 章）。

| 要件 | 使えるもの | 足りないもの・縛り |
|---|---|---|
| 1.1・1.2・1.3（行を届けて応答を返す） | `testkit.rs` の書き方（本番では使えない） | **無い**: 標準入力の読み手・HTTP の送り手・標準出力の書き手。**縛り**: `Host` はループバックの綴り・`Accept` は 2 つとも・`Content-Type: application/json`・`Origin` は付けない（`gate.rs`・要件の「受け口の約束」） |
| 1.4（改行入りを 1 行に） | — | **無い**。やり方が 2 つある（議題 9）。areka は今は改行なしで返す |
| 1.5〜1.7（空の行・JSON でない行・オブジェクトでない行） | `serde_json`（`Cargo.lock` に在る） | **無い**。areka へ送る前に中継が JSON として読む＝どの案でも行を 1 度は読む |
| 1.8（上限を設けない） | — | **縛り**: areka の側の上限は 4 MiB（`MAX_BODY_BYTES`）。超えた要求は 413 の平文が返り、要件 4.2 のエラーになる（中継の上限ではない）。応答は画像つきで大きい＝読み切りの作りと待つ時間に効く |
| 1.9（標準入力が閉じたら終わる） | — | **無い**。1 件ずつ順に扱う作りなら自然に満たす（議題 8） |
| 1.10（5 つの版） | `areka_mcp::start` を相手にしたテストで、直接つないだときと中継ごしを比べられる | **縛り**: `2026-07-28` は札を作らないと通らない（2.5）。**未確認**: Claude Desktop が話す版 |
| 2.1（20 候補を同じ順） | `DEFAULT_PORTS`・`FALLBACK_STEPS`・`candidates_from_env_value` | 共有の手段を選ぶ（議題 5）。**未確認**: 誰も居ない番号を見切るまでの時間（調べ残し 1） |
| 2.2・2.3（名乗りの確かめ） | `SERVER_NAME`（外へ出ていない）。`initialize` の答えの `result.serverInfo.name`、`server/discover` の答えの `result._meta."io.modelcontextprotocol/serverInfo".name` | **無い**。どの要求で確かめるか・いつ確かめ直すか（議題 3） |
| 2.4〜2.6・2.8（`AREKA_MCP_PORT`） | `candidates_from_env_value` と同じ読み方 | **縛り**: areka 本体の読み手は記録を `tracing` で出す。中継は標準エラーに 1 件残す（要件 2.5）ので、読み手を使い回すなら記録の受け手を据えるか、読み方を写す |
| 2.7（`127.0.0.1` だけ） | — | 作りで満たす（宛先の機械を変える設定 0 個） |
| 2.9（返事をしない相手） | — | **無い**。値は設計（議題 7） |
| 2.10・2.11（起動し直し・番号を別のものが取った） | — | **無い**。接続の持ち方で満たし方が変わる（議題 3）。**縛り**: 要件 4.5 の「送り直さない」と両立させる |
| 3.1〜3.5・3.9（控えを取る・返す） | — | **無い**。置き場と形（議題 4）。`id` の差し替えは JSON を読み直して書く |
| 3.6〜3.8（エラー・自前の `ping`・通知） | — | **無い**。行から `method` と `id` を読む |
| 3.10・3.11（読めない・書けない・同時に書く） | — | **無い**。置き場が書けない場合がある（展開先が書き込めない場所のとき）＝要件 3.10 がそのまま当たる |
| 4.1〜4.6（応答がおかしい） | — | **無い**。**縛り**: areka が SSE に切り替えた応答は 4.2 の扱いになる（今は起きない） |
| 5.1〜5.4（プロセスの作法） | `shiori-host32-helper` の `eprintln!`、または `tracing-subscriber`（`Cargo.lock` に在る） | **縛り**: `tracing-subscriber` の既定の書き先は標準出力。使うなら書き先を標準エラーへ向けないと要件 5.1 を破る |
| 6.1・6.2（設定例と絶対パス） | `help_html`・`serde_json`（`areka-mcp` の依存に在る） | **無い**: パスを求める手段（議題 6）・JSON と HTML の 2 段のエスケープ（今の `help.rs` はエスケープなし） |
| 6.3・6.4（`env` の有無） | `candidates_from_env_value(None)` で「既定の 20 個か」を判定できる | **無い**（`port.rs` に触らずに書ける） |
| 6.5・6.6 | 今の 5 節 | Desktop の節だけ書き換える |
| 7.1・7.2（zip と検査） | `tools/package.ps1` の 5 か所（2.3） | **無い** |
| 7.3（単体で動く） | 段 `check static linking` と `Test-ZipContent` の判定 6 | 表に足せば判定される |
| 7.4（説明書） | `dist/README.txt` の「◆ areka 本体」の見出し・「■ 記憶の置き場」の節の書き方 | **無い**（MCP の節ごと） |
| 7.5（全体テストが緑） | `publish = false # 理由`・`cargo about` の作り直し | 新しいクレートの `Cargo.toml` の 1 行と、`THIRD-PARTY-NOTICES.md`・`Cargo.lock` の作り直し |
| 8.1（決定論テスト） | `CARGO_BIN_EXE_*`・`CARGO_TARGET_TMPDIR`・`TempPath::under_target`・`areka_mcp::start(&[0], …)` | **縛り**: 「固定の番号を束ねない」と「既定の 20 個を当たる」を両立させるには、候補の列をテストから差し込める作りが要る（議題 10） |
| 8.2（食い違いで赤） | `candidates_from_env_value(None)`（公開）・本物の受け口の名乗り | 手段を選ぶ（議題 5） |
| 8.3（help のテスト） | `help_tests.rs` | 期待を書き換える |
| 8.4（差の記録） | `doc/ssp-mcp/transport-diff-areka.md` の書き方（行ごと・測ったテスト名つき） | **無い**（新しい文書 1 つと `survey.md` §6 の 1 行） |
| 8.5（実機） | 配布スクリプトの `-Check` | **未確認**: Desktop の話す版・Desktop が中継を起こすときの書き込みの扱い（調べ残し 3・4） |

## 4. 全体の作り方の案

### 案 A: `areka-mcp` に実行ファイルを足す（既存を広げる）

`crates/areka-mcp/` に bin の的（`src/bin/areka-mcp-bridge.rs`）を足す。新しいクレートは作らない。

- 良い点: `Cargo.lock` にパッケージの行が増えず、`THIRD-PARTY-NOTICES.md` も変わらない。候補の定数は同じクレートの公開の物をそのまま使える。
- 悪い点: 中継が `areka-mcp` の依存（rmcp・tokio・hyper・`areka-actor`）ごと建つ（使わない分は最適化で落ちる見込みだが、建てる時間と exe の中身が本体の部品に引きずられる）。`SERVER_NAME` は `pub(crate)` なので、`lib.rs` に `pub use` を足すことになる（同じウェーブの約束の外）。「areka 本体の部品」（`Cargo.toml` の `publish = false` の理由）に配布の exe が同居する。roadmap のウェーブ C5 の表は「新しいクレート `crates/areka-mcp-bridge/`」と書いている。
- 触るファイル: `crates/areka-mcp/src/bin/**`（新規）・`crates/areka-mcp/src/lib.rs`・`help.rs`・`help_tests.rs`・`tools/package.ps1`・`dist/README.txt`・文書。

### 案 B: 新しいクレート（手書きの HTTP と `serde_json`）

`crates/areka-mcp-bridge/` を作る。本番の依存は `serde_json` だけ（`Cargo.lock` に在る版）。HTTP は `std::net` で手書きする（`testkit.rs` と同じ大きさ）。テストから差し込めるよう、中身をライブラリ（候補の列・控えの置き場・待つ時間・入出力を引数に取る）にし、`main.rs` は環境変数と標準入出力をつなぐだけにする。`areka-mcp` は開発専用の依存にして、本物の受け口を相手にしたテストと、候補・名乗りの一致の検査に使う。

- 良い点: 外部のクレートが増えない。exe が小さく、tokio も rmcp も入らない。行の中身をバイトのまま届けられる。同じウェーブの約束を守れる（`lib.rs` に触らない）。
- 悪い点: 行から `method`・`id`・`params.name`・`params._meta` の版を読み、札を組み、`serverInfo.name` を見比べるコードを自分で持つ（議題 1 の判断が要る）。HTTP/1.1 の応答の読み手（`Content-Length`・chunked・途中で切れた接続）を本番のコードとして持つ。rmcp の版を上げたとき、札の決まりが変わっていないかは中継のテスト（本物の受け口を相手にしたもの）でしか分からない。
- 触るファイル: `crates/areka-mcp-bridge/**`（新規・`Cargo.toml` に `publish = false # 理由`）・`Cargo.lock`・`THIRD-PARTY-NOTICES.md`（作り直し）・`crates/areka-mcp/src/help.rs`・`help_tests.rs`・`tools/package.ps1`・`dist/README.txt`・`doc/ssp-mcp/` の新しい文書と `survey.md` §6 の 1 行・`.kiro/steering/tech.md`（依存の一覧に足すなら）。

### 案 C: 新しいクレート（公式 SDK の stdio と HTTP クライアント）

rmcp の `transport-io`（stdio の側）と `transport-streamable-http-client`（areka の側）で、両側を公式 SDK で終端する。

- 良い点: 札（`Mcp-Method`・`Mcp-Name`・`Mcp-Param-*`・版）と版の交渉を SDK が持つ。「MCP の中身を解釈するなら公式 SDK」の方針に字義どおり合う。
- 悪い点: HTTP クライアントの実装が要る。付属の実装は `reqwest` で、`Cargo.lock` に無い依存が増える（要件の「外部のクレートは増やさない見込み」「SDK が要ると分かったら止めて報告」に当たる）。自前で `StreamableHttpClient` を実装する道もあるが、hyper のクライアントの機能を足すことになる。両側を終端すると、行をいったん SDK の型に読んで書き直すので「行の中身は書き換えない」（細部 4）が字義どおりには保てない。控え・自前の `ping`・areka でない相手を飛ばす・起動し直した areka へのつなぎ直しは、SDK の「1 本のセッション」の考え方の外にあり、結局その上に自分のコードが要る。tokio が中継に入る（「tokio は MCP のスレッドに閉じる」という `tech.md` の決まりは本体の話だが、別の exe に広げることになる）。exe が大きくなる。
- 触るファイル: 案 B と同じ＋`deny.toml`（新しい依存のライセンスの確かめ）・`tech.md` の依存の一覧。

### 3 案の比べ

| 見る点 | 案 A | 案 B | 案 C |
|---|---|---|---|
| `Cargo.lock` に増える外部のクレート | 0 | 0 | 有り（`reqwest` とその先、または hyper のクライアントの機能の先） |
| 同じウェーブの約束 | `lib.rs` に触る | 守れる | 守れる |
| 札の決まりの持ち主 | 自分 | 自分 | SDK |
| 行をバイトのまま届ける | できる | できる | できない（読んで書き直す） |
| 要件 2・3（探す・控え）の作り | 自分 | 自分 | 自分（SDK の上に） |
| 規模の見立て | M | M | M〜L |

## 5. 議題ごとの選択肢

### 議題 1: 行の中身を読んで札を作ることは「MCP の中身の解釈」か（開発者の判断）

- **事実**: 要件 1.10（5 つの版のどれでも直接つないだときと同じ結果）を満たすには、`2026-07-28` の要求に札 `MCP-Protocol-Version`・`Mcp-Method`・`Mcp-Name` を付ける必要がある（2.5）。札の元は行の中の `params._meta` の版・`method`・`params.name`。SSP の中継の中にも同じ 3 つの札の字が在る（`survey.md` §6）。
- 要件の裁定 1・2 で既に行の中身を読む所が 3 つある（`method` と `id` を読む・`serverInfo.name` を読む・控えの `id` を差し替える）。札で 2 つ増える（`params.name`・`params._meta` の版）。
- **選択肢**:
  - ア. 「欄を読んで札へ写すだけで、MCP の意味は作らない」と見て、手書きで持つ（案 B）。札の決まりが rmcp と食い違えば、本物の受け口を相手にしたテストが赤になる形で守る。
  - イ. 「解釈に当たる」と見て、公式 SDK で組む（案 C）。付属の `reqwest` を使うと外部のクレートが 34 個増える。HTTP を送る部分だけ自前で書けば 0 個（2.5 の実測）だが、その部分（HTTP/1.1 の応答の読み手）は手書きのまま残る。brief の Approach は「写すだけで足りず MCP を解釈する必要が出たら、rmcp の stdio と HTTP クライアントで組む」と書いている。
  - ウ. `2026-07-28` を中継の範囲から外す（札を作らない＝要件 1.10 を 4 つの版に狭める）。Claude Desktop が `2026-07-28` で話すなら使えなくなるので、Desktop の話す版（調べ残し 3）が分かるまで選べない。
- **札を作るときの細部**（ア を選んだ場合に設計で決める）: 札の値に使えない字（ASCII の外・前後の空白）が `params.name` に在るときは rmcp と同じ `=?base64?…?=` の形にするか（`base64` は `Cargo.lock` に在るが、手書きでも数十行）、札を付けずに送って areka の `-32020` をそのまま返すか。`Mcp-Param-*` は areka の定義に印が 0 件なので作らず、「印が 0 件であること」をテストで見張るか。

### 議題 2: 全体の作り方（4 章の案 A・B・C）

議題 1 の答えで決まる。ア なら案 B（または案 A）、イ なら案 C。

### 議題 3: 相手が areka かをどの要求で確かめ、いつ確かめ直すか

**どの要求で**

| 選択肢 | 中身 | 良い点 | 悪い点 |
|---|---|---|---|
| ア. 旧式の `initialize` | 札なしで送り、答えの `result.serverInfo.name` を見る | 本文が短い。areka も SSP も札なしで答える（実測と `transport-diff-areka.md`）。areka は無状態なので、受け口に何も残らない | 相手が SSP のとき、SSP に `initialize` が 1 回届く（Desktop の要求ではないので要件 2.3 には触れない） |
| イ. `server/discover` | 札 3 つと `_meta` を付けて送り、`result._meta` の下の名乗りを見る | 新しい版の形 | 本文と札が長い。名乗りの場所が深い。古い相手は答えない |
| ウ. help の頁の題 | `GET /api/mcp/help` の題を見る | MCP を送らない | 要件 2.2 が「`serverInfo` の `name`」と決めているので、要件を変えない限り選べない |

**いつ確かめ直すか**（要件 2.11「areka でない相手へ 1 つも届けない」と要件 4.5「送り直さない」の両方に効く）

| 選択肢 | 中身 | 良い点 | 悪い点 |
|---|---|---|---|
| ア. 要求ごとに、同じ接続で先に確かめる | 接続を開く → 確かめの `initialize` を送って名乗りを見る → 同じ接続で Desktop の要求を送って閉じる | 確かめた相手と送る相手が同じ接続＝必ず同じプロセス。接続を持ち越さないので、切れた接続へ書く場合が無く、「届いたか分からない」が起きにくい。起動し直した areka には次の要求で自然につながる | 要求ごとに往復が 2 回（ループバックなので小さい見込み・未計測）。areka の受け口が 1 本の接続で 2 つの要求を受けること（keep-alive）に頼る（hyper の既定は有効） |
| イ. 接続をつなぎっぱなしにして、接続ごとに 1 度だけ確かめる | 接続が生きている間は確かめ直さない。切れたら探し直す | 往復が 1 回で済む | 受け口が使っていない接続を閉じる時間が在りうる（hyper のヘッダを読む時間切れは 30 秒・使っていない接続にも効くかは未確認＝調べ残し 2）。閉じられた直後に書いた要求は「届いたか分からない」になり、送り直せない（要件 4.5）ので、areka は元気なのにエラーを返す場合が出る |
| ウ. 別の接続で確かめてから、新しい接続で送る | SSP の中継と同じ「1 要求 1 接続」に、確かめの 1 接続を足す | 作りが単純 | 確かめた後・送る前に相手が入れ替わる隙が残る＝要件 2.11 の「1 つも届けず」を厳密には満たせない |
| エ. 見つけた番号を覚えて、切れるまで確かめない | 最初の 1 回だけ確かめる | 最も軽い | areka が去った番号を別のものが取ったとき、最初の要求が届いてしまう＝要件 2.11 に反する |

- 探し直しの頻度: areka につながっている間は ア でも探し直しは起きない（覚えた番号へ先に確かめに行き、名乗りが合えば使う、という作りにできる）。ただし要件 2.1 は「同じ順で当たる」と書いているので、「覚えた番号を先に当たる」が順を変えることに当たるかは要件ディスカッションで確かめたい。SSP が 9801・areka が 9821 の机で毎回 9801 から当たると、要求のたびに SSP へ確かめの `initialize` が 1 回届く。

### 議題 4: 控えの置き場と形

**置き場**

| 選択肢 | 中身 | 良い点 | 悪い点 |
|---|---|---|---|
| ア. 中継の exe の隣（areka の記憶と同じ木） | 例: `profile\areka\` の下に中継の控え用の場所 | SSP と同じ考え方（SSP は自分のフォルダの `data\profile\`）。`dist/README.txt` の「■ 記憶の置き場」にそのまま 1 行足せる。フォルダを消せば初めての状態に戻る。テストは exe を一時フォルダへ写して起こせば、環境変数を足さずに置き場を一時フォルダにできる | 展開先が書き込めない場所だと書けない（要件 3.10 のとおり控えなしで続く）。exe をリンクごしに起こされると「隣」がリンクの置き場になる（areka 本体は `follow_exe_links` で辿るが、その関数は `crates/areka` の中） |
| イ. 利用者ごとの場所（`%LOCALAPPDATA%` の下） | 例: `%LOCALAPPDATA%\areka\…` | 展開先が書き込めなくても書ける | areka は今まで利用者ごとの場所に記憶を置いていない（新しい決まりになる）。Claude Desktop はパッケージ版（`survey.md` §6 の `WindowsApps` の下）で、そこから起こされた子が `%LOCALAPPDATA%` へ書いた物が別の場所へ振り替えられるかは未確認（調べ残し 4）。説明書に書いた場所と実際の場所がずれうる |
| ウ. 一時フォルダ（`%TEMP%`） | — | — | 消されうるので「Desktop を先に起動してもツールが並ぶ」が頼りなくなる。本番のコードが `std::env::temp_dir` を呼ぶと常設の見張りが赤にする（例外表の編集が要る） |

- 置き場を変える環境変数を足すか: areka 本体は `AREKA_PROFILE_DIR` で記憶の置き場を変えられる（`boot_config.rs` の `default_app_profile_dir`）。中継も同じ変数を読めば、テストは環境変数だけで一時フォルダへ向けられ、Desktop の設定の `env` で利用者も変えられる。要件 2.8 が 1 つに絞っているのは「つなぐ先を変える手段」なので、置き場の変数は要件に反しない。足さない場合は、テストが exe を写して起こす。

**形**

- 種類ごとに 1 ファイル（`initialize` と `tools/list`）にするか、1 ファイルにまとめるか。まとめると、2 つの中継が別の種類を同時に書いたときに片方が消える（読んで足して書く間に競う）。種類ごとなら、要件 3.11 は「同じフォルダに一時の名前（プロセス番号つき）で書いてから改名で置き換える」で満たせる（`std::fs::rename` は Windows でも既存のファイルを置き換える）。鍵のファイルや名前つきの排他は要らない見込み。
- 中身は、areka の応答の本文そのもの（SSP と同じ）にするか、`result` だけにするか。返すときに `id` を差し替えるので、どちらでも JSON として読み直す。`serde_json` の `Value` で読んで書くと、欄の順がアルファベット順になる（`Cargo.lock` の `serde_json` の依存に `indexmap` が無い＝順を保つ機能が効いていない）。意味は変わらない（要件 3.4 は「ほかの中身は控えのまま」＝意味の話）が、字の並びまで保ちたいなら、`result` の字をそのまま挟む組み立てにする。
- 読めない控え（壊れている・途中まで）の判定は「JSON として読めて、`result` を持つ」で足りる。

### 議題 5: 候補の並びと名乗りを areka 本体と共有する手段

| 選択肢 | 中身 | 良い点 | 悪い点 |
|---|---|---|---|
| ア. 写して、テストで一致を判定する | 中継が定数を自分で持つ。中継のテスト（`areka-mcp` を開発専用の依存にする）が、並びを `areka_mcp::candidates_from_env_value(None)` と比べ、名乗りは `areka_mcp::start(&[0], …)` で起こした本物の受け口に中継をつないで確かめる | 配る exe に `areka-mcp` の依存が入らない。`lib.rs` に触らない。名乗りは定数どうしでなく、実際の応答で確かめるので、rmcp の版上げで応答の形が変わっても赤になる | 定数が 2 か所に在る（食い違いは全体テストで赤＝要件 8.2 の形） |
| イ. 中継が `areka-mcp` に本番の依存で頼る | 定数を直接使う | 定数は 1 か所 | 中継が rmcp・tokio・hyper・`areka-actor` ごと建つ。`SERVER_NAME` を外へ出すために `lib.rs` に触る（約束の外）。`candidates_from_env_value` の記録は `tracing` なので、標準エラーに出すには受け手を据える |
| ウ. 定数だけの小さなクレートを作り、両方が頼る | 例: 番号・変数名・名乗り・受け口のパスを持つクレート | 定数は 1 か所で、中継に余計な依存が入らない。本質の形 | 新しいクレートが 2 つになる。`areka-mcp` の `port.rs`・`handler.rs`・`Cargo.toml` に触る（`handler.rs` は roadmap の「MCP の 3 段目が触らない共有ファイル」）。同じウェーブの約束の表に無いファイルなので、選ぶなら止めて報告 |

### 議題 6: help が中継の置き場を知る手段

| 選択肢 | 中身 | 良い点 | 悪い点 |
|---|---|---|---|
| ア. `help.rs` の中で `std::env::current_exe()` の親フォルダを求める | `help_html(port)` の形は変えず、中で場所を求める。判断は「場所と番号を受けて節を組む」純粋な関数に分け、テストはそちらを呼ぶ | 同じウェーブの約束を守れる（`dispatch.rs`・`server.rs`・`lib.rs`・`main.rs` は 0 行） | areka 本体の「リンクを辿る」決まり（`boot_config.rs` の `exe_location`）を通らない。リンクごしに起こした areka では、help に載るパスがリンクの置き場の隣になり、そこに中継は無い。winget の portable 形式（同じウェーブの `winget-manifest-submission`）はリンクで起こす入れ方を持つので、この場合が現実になりうる（調べ残し 5） |
| イ. `main.rs` から渡す | `start` に中継の置き場（または exe のフォルダ）を足し、`State` が持ち、`help` が `help_html` へ渡す。値は `main.rs` が `exe_location` から求める（`default_helper_exe_path` と同じ形） | areka が根・補助 exe・記憶の置き場に使っているのと同じ場所になる。本質の形 | `crates/areka-mcp/src/{lib,server,dispatch}.rs` と `crates/areka/src/main.rs`（950 行・上限 1,000）に触る。`main.rs` は同じウェーブの `baseware-root-list` も触る。`start` の引数が変わるので、`start(&[0], …)` を呼ぶテスト全部に波及する。約束を破るので、選ぶなら止めて報告 |
| ウ. リンクを辿る関数を共有の場所へ移す | `follow_exe_links` と `exe_location` を `areka` の外（共有のクレート）へ出し、`help.rs` と中継の両方が使う | help も中継（控えの置き場が exe の隣の場合）も同じ決まりになる | `boot_config.rs` は同じウェーブの `baseware-root-list` が作り直すファイル。触る範囲が最も広い |

- どの選択肢でも要る細部: 場所が取れなかったとき（`current_exe` の失敗）に何を載せるか。隣に中継が実在しないとき（開発中に `areka` だけを建てた場合など）にも載せるか、注意を添えるか。
- エスケープは 2 段（要件 6.2）: パスを JSON の文字列にし（`\` を重ねる・`serde_json` で書ける）、その結果を HTML の本文として `&`・`<`・`>` を置き換える。テストは「頁から節を取り出す → HTML の置き換えを戻す → JSON として読む → `command` がパスと同じ」で固定できる。
- 設定を書くファイルの案内（要件 6.5）: Claude Desktop がパッケージ版だと、設定ファイルの実際の置き場が版ごとに違いうる。パスを書かずに「ファイルの名前」と「Desktop の設定の画面から開く」案内にするかは文面の話（要件は名前だけを求めている）。

### 議題 7: 待つ時間

| 何の時間 | 事実 | 選択肢・考える点 |
|---|---|---|
| 誰も居ない番号を見切るまで | 未計測（調べ残し 1）。SSP の中継は 1 つの番号で約 0.1 秒 | 接続に上限の時間を付ける（`TcpStream::connect_timeout`）。20 個を順に当たると、上限 × 20 が areka の居ないときの 1 要求の時間になる。短くできなければ、並べて当たる・居なかった結果を短い間だけ覚える、の手がある（どちらも要件 2.1 の「同じ順」と「最初に名乗った相手」を保てる作りにする） |
| 接続は受けるが返事をしない相手（要件 2.9） | — | 確かめの応答を待つ上限。短いほど次の候補へ早く進むが、起動の途中の areka を取りこぼす |
| 応答を待つ上限（要件 4.4） | areka のツールが答えるまでの上限は今 `REPLY_WAIT`（10 秒）。画像を返すツール（`dump_surface`・`dump_balloon`）は応答が大きい | 10 秒より長くする。後の spec（`mcp-kanade-tools`・`mcp-user-response` など）で長く待つツールが増えると、この値も見直しになる＝値をどこに書き、どのテストで「areka の上限より長い」を見張るか。Claude Desktop 自身が 1 要求を待つ上限は未確認（調べ残し 3） |

テストは本物の時間を待たないよう、3 つの値を差し込める作りにする（議題 10）。

### 議題 8: 要求を 1 件ずつ扱うか、並べて扱うか

要件は決めていない（要件 1.9 に「扱っている途中の要求」の語が在るだけ）。

- ア. 1 件ずつ（読む → 送る → 書く）。作りが最も小さく、応答の行が混ざらず、要件 1.9・4.6 を自然に満たす。長いツール（最大 10 秒）の間は、次の要求（Desktop の `ping` など）が待つ。
- イ. 並べる（要求ごとにスレッド・標準出力は鍵で 1 行ずつ）。待たせないが、接続の持ち方・控えの書き込み・探し直しの競いを全部考えることになる。
- SSP の中継がどちらかは未実測。

### 議題 9: 改行入りの応答を 1 行にするやり方（要件 1.4）

- ア. 本文を JSON として確かめ（読むだけ）、生の CR・LF のバイトを取り除く。JSON の文字列の中に生の改行は書けない決まりなので、取り除いても意味は変わらず、ほかのバイトは areka の応答のまま。
- イ. `serde_json` の `Value` で読んで書き直す。欄の順が変わり、数の書き方が変わりうる（意味は同じ）。

### 議題 10: テストから差し込める作り（要件 8.1 を満たす前提）

- 要件 8.1 は「固定の番号を束ねるテスト 0 本」、要件 2.1 は「既定の 20 個を順に当たる」。`AREKA_MCP_PORT` は 1 つの番号しか渡せないので、「areka と名乗らない相手を飛ばして次の候補へ」を exe ごしに確かめる手段が無い。
- 「`AREKA_MCP_PORT` が未設定」を exe ごしに確かめると、開発者の机の本物の 9801〜9830 を当たる（SSP や本物の areka が居ればそこへ確かめの要求が届く）。未設定の場合は「値から候補の列を作る」純粋な関数の段で固定し、exe ごしのテストは番号を指定した場合だけにするのが安全。
- 見立て: 中身をライブラリにして、候補の列・控えの置き場・待つ時間・入力と出力を引数に取る。`main.rs` は環境変数を読んで渡すだけ。exe ごしのテストは、終了コード・標準出力に応答の行しか無いこと・環境変数の読み方（番号・`0`・読めない値）に絞る。
- 記録の出し方（`tracing` と `tracing-subscriber` で標準エラーへ／`eprintln!`）もここで決まる。記録の文は、Desktop の記録ファイルで文字コードに左右されないよう ASCII（英語）にするかも合わせて決める（要件のエラーの文は英語と決まっている）。

### 議題 11: 配布スクリプトと説明書

- `tools/package.ps1` の 5 か所（2.3）に加えて、説明の注釈（「許可表の 3 本」「zip の中身の 9 行」「補助 exe と pasta.dll は arm64 の zip でも 32 ビット」）と `SCRIPT_VERSION` を合わせるか。
- `-Check`（起動の確かめ）に中継の 1 手（例: 標準入力を閉じて起こし、終了コード 0 を見る）を足すか。要件 8.5 ⑹ は「`-Check` が緑」だけを求めている。足さなければ、中継が起動できることは機械種別と取り込み表の検査（要件 7.2・7.3）と実機確認で見る。
- `dist/README.txt`: 同じウェーブの `baseware-root-list`・`winget-manifest-submission` の先の spec も自分の行を触る。本 spec の行は「◆ areka 本体」の見出しの 1 行と、新しい節（Claude Desktop からつなぐ・`AREKA_MCP_PORT`・控えの置き場）。既知の制限の「ほかのアプリからゴーストへ話しかける仕組み（SSTP）はありません」は MCP の節と並ぶと読み手が迷いうる（直すかは要件の外・気付いた点として残す）。

## 6. brief・要件の記述と実物の食い違い

1. brief（棚卸㉑〜㉓）の「同梱で変える 4 か所」は 5 か所（組み立ての段の `Copy-Item`）。
2. brief の「help の中で『areka.exe の隣』と求める形なら 4 ファイルに触らずに済む」は正しいが、areka 本体の「隣」（`exe_location`）とは同じにならない場合がある（リンクごしの起動）。要件の Boundary Context の「中継の置き場は help の文面の側で `areka.exe` の隣から導く」も同じ（議題 6）。
3. 要件の細部 4「HTTP に載せるのに要る札を行の中身から写すかどうかは設計」は、rmcp のソースでは「写さないと `2026-07-28` が通らない」と確かめられた（議題 1）。
4. 要件の「brief の記述を改めた点」7（「受け取った答えの出し入れと名乗りの見比べで足り、MCP の SDK は要らない見込み」）は、札を作る分が勘定に入っていない（議題 1）。
5. brief の「外部依存なしを目標・JSON は `serde_json` を共有」: `serde_json` は根の `[workspace.dependencies]` には無く、`crates/areka-mcp/Cargo.toml` が `serde_json = "1"` と直接書いている。中継も同じく自分の `Cargo.toml` に書く（`Cargo.lock` に新しい版は増えない）。

要件の本文（要件 1〜8 の受け入れ基準）と実物の食い違いは見つからなかった。

## 7. 規模とリスク

- **規模: M（3〜7 日）**。中継の本体（読む・送る・書く）は小さいが、探す・確かめる・控え・時間切れ・札の 5 つが重なり、要件 8.1 のテストの項目が約 40 ある。help・配布・説明書・差の記録・実機確認が続く。brief の見積もり（8〜12 タスク）の上の方、設計しだいで少し超える。
- **リスク: 中**。知らない技術は無い。読めていないのは ⑴ areka が居ないときの速さ（Windows の接続の断られ方）、⑵ Claude Desktop の話す版と待つ上限、⑶ パッケージ版の Desktop から起こされた中継の書き込み、の 3 つで、どれも設計の最初か実機確認で測れば決まる。議題 1 で公式 SDK の案になると、依存の追加で規模が M〜L に上がる。

## 8. 調べ残し

1. **誰も居ない番号への接続が断られるまでの時間**（Windows・`127.0.0.1`）。上限の時間を付けた接続で 20 個を当たったときの合計。測るのは本 spec の中継の試作で足りる（SSP の中継は動かさない）。
2. **areka の受け口が、使っていない接続をどれだけ保つか**。`server.rs` は hyper の `http1::Builder` に時計を渡しているので、ヘッダを読む時間切れ（既定 30 秒）が使っていない接続にも効くかどうか。議題 3 の イ を選ぶときだけ要る。
3. **Claude Desktop が話す版**（`initialize` で始めるか `server/discover` で始めるか）・**同時に要求を送るか**・**1 要求を待つ上限**。要件 8.5 ⑵ の実機確認で記録するが、議題 1 の ウ と議題 7・8 に効くので、設計の前に 1 度見られると良い。
4. **パッケージ版の Claude Desktop から起こされた中継の書き込み**。exe の隣への書き込みがそのまま通るか・`%LOCALAPPDATA%` への書き込みが別の場所へ振り替えられるか。議題 4 の置き場に効く。
5. **リンクごしに起こした areka で `current_exe()` が返すパス**（winget の portable 形式の入れ方を含む）。議題 6 の ア を選ぶときに、help に載るパスが正しいかを決める。
6. **SSP の中継の未実測の 3 点**（要件に記載: 返事をしない相手をどれだけ待つか・`2026-07-28` での送り方・控えが無いときの `tools/list`）。差の記録（要件 8.4）の行を埋めるのに要るが、SSP の中継を動かすのは開発者の了承の下で。

## 9. 設計の段への申し送り

- **見立て（決定ではない）**: 議題 1 が ア なら、案 B（新しいクレート・ライブラリと小さな `main.rs`・本番の依存は `serde_json` だけ・`areka-mcp` は開発専用の依存）が、要件の「外部のクレートを増やさない」と同じウェーブの約束の両方に合う。確かめは議題 3 の「旧式の `initialize`」を「要求ごとに同じ接続で先に」が、要件 2.11 と 4.5 を最も素直に満たす。
- **開発者の答えが要るもの**（答えで作業が変わる）: 議題 1（札を作ることと公式 SDK の線引き）・議題 4（控えの置き場）・議題 6（help のパスを本質の形〔`main.rs` から渡す〕で作って約束を解くか、help の中で求めるか）・議題 3 の探し直しの頻度（覚えた番号を先に当たることが要件 2.1 の「同じ順」に反するか）。
- **設計の最初に測るもの**: 調べ残し 1（20 個を当たる時間）。値しだいで議題 7 の作り（順に・並べて・覚える）が変わる。
- **完了のときの申し送り先**（要件の Adjacent expectations のとおり）: `release-code-signing`（署名する exe が 3 本になる）・`winget-manifest-submission`（入れ子の実行ファイルに中継を入れるか・リンクごしの起動で help のパスがどう見えるか）。
- **後の spec への縛りとして残す事実**: ⑴ areka のツールが途中で通知や要求を出すと応答が SSE になり、中継は「JSON でない本文」として扱う。⑵ ツールの定義に `x-mcp-header` の印を足すと、`2026-07-28` の `tools/call` が中継ごしに通らなくなる。⑶ `REPLY_WAIT` より長く待つツールを足すときは、中継の応答の上限も見直す。⑷ rmcp の版を上げるときは、中継のテスト（本物の受け口を相手にしたもの）も通す。

## 10. 要件ディスカッションの結果（2026-10-10）

- **議題 1 は決着: 公式 SDK（rmcp）で組む**（開発者の裁定 3・`requirements.md`）。4 章の案 C の変形で、付属の `reqwest` は使わず、HTTP を送る部分だけ自前で書く（`Cargo.lock` に無い名前 0 個＝2.5 の実測）。案 A・案 B と、議題 1 の ア・ウ は採らない。
- 裁定の前に rmcp 3.5.0 のソースで確かめたこと: ⑴ HTTP を送る口 `StreamableHttpClient` は公開の trait で、メソッドは 4 つ（`post_message`・`post_message_with_max_sse_event_size`・`delete_session`・`get_stream`）。⑵ `Service` は要求を種類ごとでなく 1 つの口（`handle_request`）で受けられ、未知のメソッド用の型（`CustomRequest`）が在る＝areka に機能が増えても中継に転送の行を足さずに済む見込み。⑶ stdio の輸送（`transport/async_rw.rs`）とクライアント（`service/client.rs`）に `2026-07-28` の扱いが入っている。
- **設計の最初に確かめる 3 点**（崩れたら止めて報告）: 自前の HTTP の送り手で rmcp の口を実際に満たせるか・「確かめた相手と同じ接続で送る」（要件 2.11）を rmcp のクライアントの上で保てるか・exe の大きさ。
- 裁定 3 で変わる議題:
  - 議題 2（全体の作り）: 新しいクレート・rmcp の `client`・`transport-io`・`transport-streamable-http-client`（と受ける側の `server`）・tokio。tokio を中継のどこに置くかは設計。
  - 議題 3（名乗りの確かめ）: rmcp のクライアントが areka と交わす最初のやりとりで得るサーバーの情報の `name` を見る形が素直。いつ確かめ直すかは上の 2 点目と一緒に決める。
  - 議題 4（控えの形）: 控えるのは「サーバーの情報」と「ツールの一覧」（rmcp の型で読み書きできる）。`id` の差し替えは要らなくなった。
  - 議題 5（定数の共有）: 中継にも rmcp と tokio が入るので、「`areka-mcp` に頼ると rmcp・tokio を引き込む」という悪い点は軽くなった（`areka-actor` は今も引き込む）。
  - 議題 9（1 行にするやり方）: rmcp の stdio の輸送が 1 行で書く。
  - 新しい議題: rmcp の stdio の輸送が JSON でない行・オブジェクトでない行をどう扱うか（要件 1.6・1.7）。
- **9 章の「後の spec への縛り」のうち消える見込みのもの**: ⑴ 応答が SSE になる場合（rmcp のクライアントが読む）・⑵ `x-mcp-header` の印（rmcp が札を作る）。設計で確かめてから消す。⑶ 待つ上限・⑷ rmcp の版上げのときに中継のテストも通す、は残る。
- 自明な修正で要件へ入れたもの: 「つなぐ先を決めた後は先頭から当たり直さない」（3 章の議題 3 の「同じ順に反するか」の答え）・「同じウェーブの約束は設計の形を縛らない」（議題 5 のウ・議題 6 のイを選ぶなら設計で挙げて諮る）・「机の本物の 9801〜9830 へ接続するテスト 0 本」。

## 11. 設計の段の確かめ（2026-10-10・裁定 3 の 3 点＋調べ残し 1）

> やり方: rmcp 3.5.0・hyper 1.11.1・sse-stream 0.2.6 のソース（cargo の登録簿 `C:\rust\cargo\registry\src\…`）を読み、試しのクレート `target\mcp-stdio-bridge-design\probe\`（git の外・ワークスペースの `Cargo.lock` を写して建てた）で、自前の送り手・rmcp の `Service<RoleServer>` の直接の実装・自前の stdio の輸送を書いて、偽の受け口（名乗り `areka-mcp-server`・`target\mcp-stdio-bridge-design\fake-areka.ps1`・番号 47123）を相手に 10 行を流した。開発者の SSP・`C:\wintools\ssp\data\profile\*`・机の 9801〜9830 には触れていない（接続の計測は束ねてすぐ放した空きポートと 47123・9・1）。

### 11.1 ⒜ 自前の HTTP の送り手で rmcp の口 `StreamableHttpClient` を満たせるか → **満たせる（建って動いた）**

- `StreamableHttpClient`（`src/transport/streamable_http_client.rs`）は `Clone + Send + 'static` の trait で、**実装が要るのは 3 つ**: `post_message`・`delete_session`・`get_stream`。`post_message_with_max_sse_event_size`・`get_stream_with_max_sse_event_size` は既定の実装（前者へ委ねる）を持つ。要件の「4 つ」は 1 つ多い。
- 署名（`impl Future … + Send + '_` の形）:
  - `post_message(&self, uri: Arc<str>, message: ClientJsonRpcMessage, session_id: Option<Arc<str>>, auth_header: Option<String>, custom_headers: HashMap<HeaderName, HeaderValue>) -> Result<StreamableHttpPostResponse, StreamableHttpError<Self::Error>>`。`StreamableHttpPostResponse` は `Accepted`（202）・`Json(ServerJsonRpcMessage, Option<String>)`・`Sse(BoxStream, Option<String>)`。
  - `delete_session(&self, uri, session_id: Arc<str>, auth_header, custom_headers) -> Result<(), …>`。
  - `get_stream(&self, uri, session_id: Option<Arc<str>>, last_event_id: Option<String>, auth_header, custom_headers) -> Result<BoxStream<'static, Result<sse_stream::Sse, SseError>>, …>`。戻り値の型 `Sse` は rmcp が外へ出していない（`SseError` だけ出している）ので、`sse-stream` 0.2.6（`Cargo.lock` に在る）を直接の依存にして型の名前を書く。
- **areka（無状態・JSON の単発）相手に実際に呼ばれるのは `post_message` だけ**: rmcp のクライアントの働き手（`StreamableHttpClientWorker::run`）は、`initialize` の応答に `Mcp-Session-Id` が無く `allow_stateless`（既定 `true`）なら `session_id = None` にし、`get_stream`（共通の SSE）は `session_id` が在るときだけ起こし、`delete_session` は片付けの情報が在るとき（＝セッション在り）だけ呼ぶ。試しでは `delete_session` を `Ok(())`、`get_stream` を `Err(ServerDoesNotSupportSse)` にして一度も呼ばれなかった。
- 送り手の中身は tokio の `TcpStream`＋`httparse`（1.10.1・`Cargo.lock` に在る）で、`Content-Length`・chunked・閉じるまで、の 3 通りの読み取り。**hyper の `client` 機能は使えない**: hyper 1.11.1 の `Cargo.toml` で `client = ["dep:want", "dep:pin-project-lite", "dep:smallvec"]` と `want`（と `try-lock`）を要し、`want` は `Cargo.lock` に無い（名前の数 0 を破る）。
- **`Cargo.lock` に無い名前の数**: 試しの `cargo tree -e normal --target x86_64-pc-windows-msvc` の名前（71 個）を `Cargo.lock` の名前と比べて **0 個**。`areka-mcp` を path の依存に足した形（101 個）でも **0 個**。
- 試しで動いたこと（10 行の入力・標準出力 9 行・終了コード 0・合計 0.44 秒）: `initialize`（areka の名乗り・できること・版を返す）・`notifications/initialized`（出力なし）・`ping`・文字列の `id` の `tools/list`・`tools/call`（本文の `\r\n` と日本語がそのまま 1 行）・JSON でない行 → `-32700`・`[1,2]` → `-32600`・未知のメソッド `x/pretty` が `CustomRequest` として素通り（改行入りの応答が 1 行に）・平文の 415 → `-1`・空の本文 → `-1`。偽の受け口が受けた方法の並びも記録した（`initialize`・`notifications/initialized`・`ping`・`tools/list`・`tools/call`・`x/pretty`・`x/415`・（作り直しの）`initialize`・`notifications/initialized`・`x/empty`）。

### 11.2 ⒝ 確かめた相手と同じ接続で送れるか・手作りの名乗りの確かめと rmcp の手順の関係 → **送れる（接続は送り手の持ち物）**

- rmcp のクライアントの輸送はセッションの番号（`Mcp-Session-Id`）を持ち回るだけで、**TCP の接続は `StreamableHttpClient` の実装が全部決める**（rmcp は `post_message` を要求・通知ごとに 1 回呼ぶ。接続の使い回しも切断の検知も rmcp には無い）。areka はセッションを返さないので `session_id` は常に `None`。
- したがって「確かめた相手と同じ接続で送る」は送り手の中で閉じる: 接続を開く → 固定の旧式の `initialize` を送って `result.serverInfo.name` を見る → 同じ接続で本当の要求を送る（`Connection: close`）。rmcp の手順の最初の `initialize` も `post_message` を通るので、その答えの名乗りで同じ確かめができる（2 度送らない）。
- rmcp の手順（`serve_client` の既定 `ClientLifecycleMode::Initialize`）: `initialize` → `notifications/initialized` を送ってから `RunningService` を返し、`peer_info()` に `ServerPeerInfo`（版・できること・`server_info`・`instructions`・`_meta`）が入る。`server/discover` から始める手順（`Auto`／`Discover`）も在るが、areka は旧式の `initialize` を受けるので既定でよい。
- 働き手の `post_message` の失敗はその要求の `responder` へ返るだけで働き手は死なない（`Event::PostResult` の処理）。`Peer::send_request` には `ServiceError::TransportSend(DynamicTransportError)` として届き、`DynamicTransportError.error`（`Box<dyn Error + Send + Sync>`）を `StreamableHttpError<自前の型>` へ `downcast_ref` できる＝「書く前／後」の読み分けができる。
- hyper 1.11.1 の受け口（areka）は、使っていない接続も `header_read_timeout`（既定 30 秒・`http1::Builder::new` の初期値・`Conn::poll_read_head` が次の要求の頭を待つときに時計を走らせる）で閉じる。接続を持ち越す形だと 30 秒後の最初の要求が「届いたか分からない」になるので、要求ごとに接続する形を選んだ（design D-4）。

### 11.3 ⒞ exe の大きさ → **2,469,888 バイト（約 2.36 MiB）・`areka-mcp` を依存に足すと 2,504,704 バイト（＋34,816）**

- 試しのクレート（rmcp `server`＋`client`＋`transport-io`＋`transport-streamable-http-client`・tokio・自前の送り手・`Service` の直接の実装・自前の stdio の輸送・`tracing-subscriber`）を、ワークスペースと同じ `[profile.release]`（`opt-level = 'z'`・`lto = true`・`codegen-units = 1`・`panic = 'unwind'`・`strip = false`）で `--target x86_64-pc-windows-msvc` に建てた。`+crt-static` は付けていない（配布の建て方では少し増える）。
- 本物の中継は控え・探索・記録の分が足されるので、見立ては 2.6〜3 MiB。配布の zip に入れて困る大きさではない（`areka.exe` と同じ木に入る）。

### 11.4 調べ残し 1: 誰も居ない番号への接続が断られるまでの時間 → **開発者の机では約 2 秒（即座に断られない）**

- 束ねてすぐ放した空きポート・47123・9・1 の 4 つへ、tokio の `TcpStream::connect` と std の `TcpStream::connect_timeout` で各 20 回（上限 1 秒）: **80 回とも時間切れ・断られた回数 0**。.NET の `TcpClient.Connect` では **2,035 ms 後に ConnectionRefused**。この机では、閉じた `127.0.0.1` の番号への SYN がすぐには RST で返らず、再送の後に断られる（Windows Defender の壁の 3 つの構成は有効）。
- SSP の中継が「居ないとき約 0.1 秒」で答えた（要件の実測）のは、1 つの番号を短い上限で見切っているため（断られるのを待っていない）。
- 含意: 20 候補を順に当たると「上限 × 20」になる。設計は全候補への接続を並べて開き、短い上限（`CONNECT_WAIT` 300 ms）で見切り、開いた接続を候補の順に確かめる形（design D-5）。areka が居ないときの 1 回の探索は約 0.3 秒。

### 11.5 ほかに rmcp 3.5.0 のソースで確かめたこと

- **stdio の輸送（`transport/async_rw.rs` の `AsyncRwTransport::receive`）は、JSON として読めない行（`Syntax`／`Eof`）を黙って捨てる**（`debug!` だけ・issue #938 の判断）。形が違う JSON（`Data`）には `-32600`・`id` なしを返す。要件 1.6（`-32700` を返す）を満たすには輸送を自前で持つ（60 行ほど・試しで動いた）。書き手（`JsonRpcMessageCodec::encode`）は `serde_json::to_writer`＋`\n`（改行を含まない 1 行＝要件 1.4）。
- **`serve_server`（`serve_server_with_ct_inner`）は、最初の問い合わせが失敗すると `Err` を返してループに入らない**（`report_initialize_failure`）。`initialize` の前の `ping` には自分で `{}` を返す。要件 4.6 と「控えが無いときの `initialize` はエラー」（裁定 1）を両立させるには `serve_directly_with_ct`（手順を飛ばして最初から `serve_inner`）を使い、`initialize`・`server/discover`・`ping` も `handle_request` で扱う。試しで `serve_directly` に `Service<RoleServer>` を直接実装した型を渡して動いた（`ServerHandler` の包括実装とは重ならない）。版の交渉（`negotiate_protocol_version`）は `serve_server` の中の関数で外から呼べないので、中継が同じ答え（要求の版が `initialize` を持つ 4 版ならその版・それ以外は `2025-11-25`）を自分で組む。
- **`Service<RoleServer>`** の 3 つ: `handle_request(&self, ClientRequest, RequestContext) -> Result<ServerResult, ErrorData>`・`handle_notification`・`get_info() -> ServerConfig`（＝`InitializeResult` の別名）。`supported_protocol_versions()` は既定で `ProtocolVersion::KNOWN_VERSIONS`（5 版）。`ClientRequest` は既知の 18 種＋`CustomRequest { method, params, extensions }`、`ServerResult` も `CustomResult(Value)` を持つ＝未知のメソッドが素通りする（試しの `x/pretty`）。
- **rmcp の受け手は要求ごとに仕事を起こす**（`serve_inner` の `tokio::spawn`）: 応答の行の順は要求の順と同じとは限らない（試しで `-32700` の行が先の要求の応答より先に出た）。標準入力が閉じると `QuitReason::Closed` で、書きかけの応答を **5 秒**まで掃き出してから終わる（`drain_timeout`）。
- **`Peer::send_request_with_option(request, PeerRequestOptions)`** の `timeout` で応答の上限を付けられる（`ServiceError::Timeout`）。`PeerRequestOptions` は `non_exhaustive` なので `no_options()` から欄を書く。
- **札**: クライアントの働き手の `build_request_headers`／`request_version_headers` が `MCP-Protocol-Version`・`Mcp-Method`・`Mcp-Name`・`Mcp-Param-*`（`tools/list` の答えから `x-mcp-header` の印を覚える `cache_tools_from_response`）を組んで `custom_headers` に入れて渡す。自前の送り手はそれを写すだけ。
- **`DiscoverResult::from_server_info(supported_versions, ServerConfig)`** で控えのサーバーの情報から `server/discover` の答えを組める（`_meta` の `io.modelcontextprotocol/serverInfo` 入り・`ttlMs: 0`・`cacheScope: private`）。`ServerPeerInfo` は `serde` で読み書きできる（控えの形にそのまま使える）。

## 12. 設計の決定（`design.md` の D-1〜D-16 の根拠の要約）

| 決定 | 選んだ形 | 退けた形と理由 |
|---|---|---|
| 全体の作り（議題 2） | 新しいクレート・rmcp 両側・HTTP の送り手だけ自前（裁定 3） | 案 A・案 B・`reqwest`（34 個増える）・hyper の `client` 機能（`want` が増える） |
| Desktop の側の受け方 | `serve_directly_with_ct`＋`Service<RoleServer>` の直接の実装 | `serve_server`＋`ServerHandler`: 最初の問い合わせの失敗で終わる（4.6 に反する）・`initialize` の答えを `get_info` の同期の関数で返す形になり、areka へ問いに行けない |
| 名乗りの確かめ（議題 3） | 接続ごとに、その接続の最初の要求で確かめてから同じ接続で送る（ア） | イ（つなぎっぱなし）: hyper の 30 秒の閉じで「届いたか分からない」が出る・ウ（別の接続）: 入れ替わりの隙・エ（最初だけ）: 2.11 に反する |
| 探し方（議題 7） | 並べて開き・順に確かめ・短い上限で見切る | 順に当たる: 居ないときに上限 × 20（机では断られるまで 2 秒） |
| 控え（議題 4） | exe の隣（`profile\areka\mcp-bridge\`）・`AREKA_PROFILE_DIR` で変えられる・種類ごとに 1 ファイル・一時の名前から `rename` | `%LOCALAPPDATA%`（areka に無い決まり・パッケージ版の振り替えが未確認）・`%TEMP%`（消される・見張りが赤） |
| 定数の共有（議題 5） | `areka-mcp` を本番の依存に（受け口の部品は呼ばない・exe は +34,816 バイト） | 写して検査（判断の関数を 2 か所に持つ）・定数だけのクレート（クレートが 2 つ・`handler.rs`／`port.rs` の移動） |
| help の置き場（議題 6） | `main.rs` から `exe_location` で求めて `start` の引数で渡す（イ＝本質の形） | ア（`help.rs` の中で `current_exe`）: リンクごしで本当の隣にならない。代案として design に残す |
| 入力の切り方・1 行にする（議題 9） | 自前の輸送が 1 行 1 要求で読み、rmcp の型を `serde_json` で 1 行に書く | rmcp の stdio の輸送: JSON でない行を黙って捨てる |
| 1 件ずつか並べてか（議題 8） | rmcp の受け手が並べる・接続は要求ごと | 1 件ずつに戻す: rmcp の受け手の形に逆らう・長いツールの間 `ping` が待つ |
| 通知 | つながっていれば `initialized` 以外を届ける・居なければ何もしない | 全部捨てる: 「意味を変えずに届ける」に反する理由が無い |
| 時間 | `CONNECT_WAIT` 300 ms・`VERIFY_WAIT` 3 秒・`REPLY_WAIT` 30 秒（`tools::REPLY_WAIT` 10 秒より長い・比べるテスト） | — |
| 窓 | `#![windows_subsystem = "windows"]`（areka 本体と同じ） | 端末のアプリ: Desktop の起こし方によっては窓が出る |

## 13. 9 章の「後の spec への縛り」の整理

- ⑴ SSE の応答: **残る**（自前の送り手は JSON 専用・SSE は 4.2 のエラー）。10 章の「消える見込み」は外れた。
- ⑵ `x-mcp-header` の札: **消える**（rmcp のクライアントが作る）。
- ⑶ `REPLY_WAIT` より長いツール: 残る（比べるテストで赤）。
- ⑷ rmcp の版上げで中継のテストも通す: 残る（`tech.md` に書く）。
- 新しく足す: 標準入力が閉じた後の掃き出しは rmcp の 5 秒（`REPLY_WAIT` より短い）。areka が居ないときの探索は Desktop の要求ごとに約 0.3 秒（覚えは持たない）。`dist/README.txt` の既知の制限「SSTP はありません」は MCP の節と並ぶが、要件の外なので触らない（気付いた点）。

## 14. 調べ残し（実装・実機確認で埋める）

1. Claude Desktop が話す版・同時に要求を送るか・1 要求を待つ上限（要件 8.5 ⑵）。
2. パッケージ版の Desktop から起こされた中継が exe の隣へ書けるか（8.5 ⑷ で分かる・書けなければ 3.10 の経路で控えなしになる）。
3. SSP の中継の未実測 3 点（返事をしない相手をどれだけ待つか・`2026-07-28` での送り方・控えが無いときの `tools/list`）は差の記録に「未実測」と書く（SSP の中継を動かすのは開発者の了承の下で）。
4. 配布の建て方（`+crt-static`）での exe の大きさ（11.3 は付けずに測った）。

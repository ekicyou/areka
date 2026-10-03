# Requirements Document

> 本文の実測は **2026-10-03・本ブランチ**（main `76e17654`＝棚卸⑳の PR#211 のコミット。brief の 10-02 の再測定〔main `03e8d7d6`〕からコードの変更は 0 件で、変わったのは roadmap だけ）のもの。コードは「何の定義か」（関数名・型名・定数名＋ファイルパス）で指し、行番号では指さない。
> 「要件の段での暫定の裁定」の表は、要件を書く途中で答えが要った点に対する**推奨案による暫定の確定**であり、要件ディスカッションで覆せる（覆したら該当要件も改める）。答えで作業が変わる議題は見込み 2 件（既定ポートと既定で有効にするか・`serverInfo` の名前）で、どちらも表に載せた。
> **2026-10-03・実機確認 6.2 で判明した改訂**: 「Claude Code・Cursor は旧式の 4 版の経路（`initialize`→`tools/list`→`ping`）でつなぐ」という前提は Claude Code について誤りだった。Claude Code 2.1.283 は `initialize` を送らず、無状態版（`2026-07-28`）の `server/discover`→`tools/list` でつなぎ、`tools/list` の結果に `ttlMs`・`cacheScope` が無いので接続に失敗した（Cursor の送り方は未確認）。いま何が起きているかの rmcp の段・要件 3 の Objective・要件 5.2 の物差し・要件 9.2 を改め、要件 3.15 を足した。
> **2026-10-03・開発者裁定（実機確認 6.2 の ⑷ で、開発者の机の SSP が 9801 と 9821 の両方で待ち受けており、既定 9821 を束ねられなかった）**: areka は SSP と同じ種類のベースウェアなので、同じ既定の番号を SSP と取り合う＝**早い者勝ち**。既定の候補は 9801 → 9821 の順で、先に束ねられた 1 つだけで待ち受ける（両方で待ち受ける SSP の形は取らない）。両方とも使用中なら隣の番号へ逃げる（9802 → 9822 → 9803 → 9823 → … → 9810 → 9830 の計 20 候補）。`AREKA_MCP_PORT` で番号を指定したときはその 1 つだけを試す。これは要件ディスカッション議題 1（既定 9821・9801 は SSP が使うので避ける・失敗は記録だけで別のポートを試さない）を覆す。要件 1.2・1.3・2（Objective・2.1〜2.6）・6.1・6.2・9.1・9.3・9.5 と Boundary・裁定の表 1〜3 を改め、要件 1.10・2.8・2.9 を足した。

## Project Description (Input)

**誰の何が困っているか**: AI エージェント（Claude Code・Cursor など）でゴーストを作る人は、SSP なら内蔵の MCP サーバ経由で台本を流し・イベントを起こし・絵を撮り・ログを読めるが、areka には受け口が無く、同じ開発の回し方ができない。areka の開発そのものも、動いている areka への確かめを `AREKA_APP_SMOKE_EXIT_MS` とログの grep に頼っている。

**今の状態**: areka 本体にサーバの類は 0（本番の `std::net`・tokio・HTTP の土台いずれも無し）。MCP の受け口も、ツールも、登録の案内も無い。

**何を変えるか**: areka が起動すると `127.0.0.1:<port>`（既定は 9801 → 9821 の早い者勝ち・どちらも使用中なら隣の番号へ・`AREKA_MCP_PORT` で変更・`0` で待ち受けない）で HTTP を受け、`POST /api/mcp/v1` が MCP サーバ（公式 Rust SDK `rmcp`・無状態・JSON の単発応答）として `initialize`→`tools/list`→`ping` に応える。ツールはまだ 0 本で、後続 spec がツールを足すための登録口だけを持つ。`GET /api/mcp/help` で登録手順を示し、`Origin` がループバック以外なら拒否する。待受の失敗はログに理由を 1 件残してアプリは動き続ける。SSP の輸送の癖との差は一覧にし、クライアントが困る行だけ直す。

> 起票: 2026-09-29 `/kiro-discovery`（開発者指示「ssp mcp tool の完全移植のための spec 群を立ち上げて。実装は α リリースの後。areka の 127.0.0.1 の適当なポートでサーバを開く形。基本実装 → 空のダミー関数を置いて入り口だけ全部整備 → 個別のコマンド実装」）。SSP MCP 移植の **1 段目（基本実装）**。事実の正本は [doc/ssp-mcp/survey.md](../../../doc/ssp-mcp/survey.md)（SSP 2.9.05 の実測）。全体の並びは `.kiro/steering/roadmap.md`「SSP MCP の移植」節。

## Introduction

### 誰が困っているか

- **AI エージェントでゴーストを作る人**: ukadoc の「その他の機能」は、SSP の内蔵 MCP サーバを「AI エージェントでゴーストを作りたい（Vibe Coding）」の土台として案内している。Claude Code・Cursor は HTTP の MCP サーバを直接登録できる（`claude mcp add --transport http <名> <URL>`）。areka には登録する先が無い。
- **areka の開発者**: 実機の確かめは有界の自動終了とログの grep だけで、動いている areka に「今どうなっているか」を問えない。本 spec でツールは増えないが、受け口ができれば後続の spec（`get_status`・`get_log` など）がそれを埋める。

### いま何が起きているか（2026-10-03 実測）

- **本番のサーバは 0。** brief の 10-02 の再測定のとおり（`crates/**/src/` の本番に `std::net`・tokio のランタイム・HTTP の土台は無い）。テストには前例が 1 つ: `crates/areka-update/src/winhttp_real_tests.rs` が `std::net::TcpListener::bind("127.0.0.1:0")` で偽の HTTP サーバを立てている（OS に空きポートを割り当てさせる形）。
- **起動と終了の配線の前例。** `crates/areka/src/main.rs` の `fn main()` は 1 つで、裏方を立てて畳む前例は `perf_thread_report::start()`／`stop_and_report_final()`。brief が注意するとおり、終了の閉包の中の早い `return` で後始末が飛びうるので、畳む処理は早い戻りより前か `Drop` に置く（設計の話＝ここでは要件 1 の「終了で待受が閉じる」だけを求める）。
- **環境変数の前例。** `AREKA_APP_SMOKE_EXIT_MS`（`crates/areka/src/main.rs` の `SMOKE_EXIT_ENV` と、env を読まない純粋な読み解きの関数）は「未設定・空・数でない・負・溢れ → 無効（発火しない）」。`AREKA_ROOT`（`crates/areka/src/boot_config.rs` の `resolve_root_from`）は「空でも『設定あり』として扱い、exe の隣へ倒さない」。本 spec の `AREKA_MCP_PORT` は、`0` を「待ち受けない」の明示の値に使うので、数でない値を「無効」へ倒すと `0` と区別が付かなくなる＝数でない値は既定へ倒して `warn!` で知らせる（裁定 2）。
- **ライセンスの門。** `deny.toml` の許可の表は 9 つ（MIT・Apache-2.0・Apache-2.0 WITH LLVM-exception・Unlicense・Zlib・0BSD・BSD-2-Clause・BSD-3-Clause・Unicode-3.0）。表に無いライセンスの推移依存が 1 つでも入ると `cargo deny check` が赤になる。rmcp（Apache-2.0）・tokio（MIT）・hyper／axum／tower（MIT）は表の中。ギャップ分析で rmcp 3.5.0 だけを依存に持つ空のクレートに `cargo deny check licenses` を回し、新しく入る 30 クレート（hyper を直に使う形）は全部 MIT か MIT OR Apache-2.0 で**門は緑・表に足すものは 0** と測った（research.md §4）。
- **rmcp の現物（2026-10-03・crates.io の最新は 3.5.0〔2026-09-28〕・rust-sdk の `main` の `crates/rmcp/src/transport/streamable_http_server/tower.rs` を読んだ）。** `StreamableHttpServerConfig` に ⑴ `legacy_session_mode`（**既定 `true`＝セッション有り**。`false` で無状態）、⑵ `json_response`（既定 `false`。`true` かつ無状態のとき、結果かエラーで終わる要求へ `application/json` の単発で答える・通知は `202 Accepted`）、⑶ `allowed_hosts`（**既定 `localhost`・`127.0.0.1`・`::1`**＝それ以外の `Host` は 403）、⑷ `allowed_origins`＋`enforce_origin_validation()`（既定は検査なし。有効にすると許可の表に無い `Origin` は 403・ポートのワイルドカード `:*` 可）、⑸ `stateless_protocol_metadata_required`（既定 `false`＝2026-07-28 の `_meta` を強いない）、⑹ `max_request_body_bytes`（既定 4 MiB）がある。`server/discover` は版の交渉の対象として扱われる。`Content-Type` が JSON でない本文も JSON として壊れた本文も **415・平文**（JSON-RPC のエラーではない）、`MCP-Protocol-Version` の無い要求は `2025-03-26` と見なし、5 版のいずれでもない値は **400**。これらは設計の段で手元の cargo レジストリにある **3.5.0 のソース**で確かめた（research.md §11.2）。未知のメソッド名は `CustomRequest` として読まれ、`on_custom_request` の既定が **`-32601`** を返す（旧式の経路では HTTP 200＝SSP と同じ）。JSON-RPC のエラーのとき HTTP の状態コードが変わる（`-32602` は 400・`-32601` は 404）のは **2026-07-28 の per-request 経路だけ**で、旧式の 4 版の経路では常に 200（SSP と同じ）。Claude Code 2.1.283 がつなぐのは前者（無状態版の `server/discover`→`tools/list`・`initialize` なし）で、Cursor は未確認（2026-10-03・実機確認 6.2）。**`initialize` の版の交渉**: `2026-07-28` は `initialize` を持たない版（規格が `server/discover` に置き換えた）なので、`initialize` で `2026-07-28` を要求すると rmcp は `initialize` を持つ最新の `2025-11-25` へ倒す（要件 3.1）。

### brief の記述を実物で引き直して改めた点

1. brief の「無状態の既定と `with_json_response(true)`」は半分違う。rmcp の既定は**セッション有り**（`legacy_session_mode = true`）で、無状態は明示して選ぶ。JSON の単発応答も明示して選ぶ。要件 3 は「無状態・JSON の単発」を利用者から見える形（セッション ID を要求しない・応答が `application/json`）で求め、選び方は設計に任せる。
2. brief の「`Origin` 検査（rmcp に同等の設定があればそれを使う）」は、rmcp に許可の表の形で在る。要件 4 は通す／拒む条件だけを決め、rmcp の表で足りるか自前かは設計で決める。
3. brief と survey に無かった事実: rmcp は既定で **`Host` も検査する**（ループバック以外は 403）。SSP は `Host` を検査しない（survey §2）。areka は rmcp の既定を採って `Host` も拒む（裁定 6）＝SSP との差として一覧に載せる。
4. brief の「JSON でない本文 → 400・`-32700`」（SSP の振る舞い）は、rmcp では 415。クライアントは JSON 以外を送らないので差として受け入れる（要件 5）。
5. brief の「無状態版で `_meta` が無ければ 400」（SSP）は、rmcp の既定では起きない（`stateless_protocol_metadata_required = false`）。クライアントに優しい側の差なので受け入れる。

### 要件の段での暫定の裁定（要件ディスカッションで覆せる）

| # | 議題（brief） | 暫定の裁定 | 根拠 | 載せた要件 |
|---|---|---|---|---|
| 1 | 既定ポートと既定で有効にするか | **既定は 9801 → 9821 の早い者勝ち・先に束ねられた 1 つだけで待ち受ける・既定で有効**（SSP と同じく常に待ち受ける）＝**2026-10-03 開発者裁定**（同日の要件ディスカッション議題 1「既定 9821・9801 は避ける」を覆した） | areka は SSP と同じ種類のベースウェアで、同じ既定の番号を取り合う（先に起きた方が取る）。実機確認 6.2 で SSP が 9801 と 9821 の両方で待ち受ける机があり、9821 決め打ちでは SSP と並べたとき待ち受けられなかった。両方で待ち受ける形（SSP の形）は取らない。既定で無効だと「登録したのにつながらない」が最初の体験になる | 2.1・2.2・2.8 |
| 2 | `AREKA_MCP_PORT` が数でない・範囲外のとき | **`warn!` 1 件を残して既定の候補（9801 → 9821 → 隣）で待ち受ける**（無効へ倒さない・1 つの番号へ倒さない）＝2026-10-03 開発者裁定で「既定 9821」から改めた | `0` が「待ち受けない」の明示の値なので、書き損じで黙って止まる形を避ける | 2.5 |
| 3 | 待受に失敗したら記録だけか別のポートを試すか | **既定の候補のときだけ隣へ逃げる**（9801・9821 がどちらも使用中なら 9802 → 9822 → … → 9810 → 9830 の計 20 候補を順に試す。全部だめなら `error!` 1 件で待ち受けない）。**`AREKA_MCP_PORT` で指定した番号は逃げない**（その 1 つだけ・だめなら `error!` 1 件）＝**2026-10-03 開発者裁定**（議題 1「記録だけ・別のポート 0 回」を覆した） | 早い者勝ちで後から起きた方も待ち受けられるようにする。実際の番号は `info!` の URL と help に必ず出るので、移ったことは分かる（登録済みのクライアントの URL が別のベースウェアを指しうる点は、早い者勝ちの代わりに受け入れる＝`serverInfo` の名前で見分けられる）。指定した番号から逃げると利用者の意図に反する | 1.3・1.10・2.3・2.9 |
| 4 | `serverInfo` の名前と版 | **`areka-mcp-server`・版は areka の Cargo の版**（今は `0.0.1`）＝**議題 2 で開発者が確定** | SSP の `ssp-mcp-server`／`2.9.05` の型に倣う | 3.1 |
| 5 | `GET /api/mcp/v1` の手打ちフォーム | **作らない**（0 ページ） | `curl` で足りる。SSP のフォームは手打ち用の便利機能で、クライアントは使わない | 3.11・Out of scope |
| 6 | `Host` の検査 | **rmcp の既定どおりループバック以外を 403 で拒む** | DNS 再束縛への備えが `Origin` だけの SSP より堅く、正規のクライアントは `127.0.0.1:<port>` を書くので困らない | 4.4 |
| 7 | `Origin` が無いとき | **通す** | `curl`・Claude Code・Cursor は `Origin` を付けない。SSP も通す | 4.1 |
| 8 | rmcp の版 | **crates.io の最新（2026-10-03 時点 3.5.0）を `=` で固定**し、上げるときは本 spec の決定論テストを通す | brief の制約。MCP の新しい版への追随は rmcp の版上げで行う | 8.2 |
| 9 | 差の一覧の置き場 | **`doc/ssp-mcp/` の下に 1 文書**（survey §2 の表の各行に areka の列を足した形） | 事実の正本の隣に置く。survey 本文は SSP の記録なので書き換えない | 5.1 |
| 10 | `instructions` と help の言語 | **`instructions` は英文（1〜3 文）・help は日本語**（コマンド例はそのまま） | `instructions` の読み手は LLM（SSP も英文）。help の読み手は伺かの利用者 | 3.1・6.1 |
| 11 | HTTP の土台（axum か hyper を直か）・`Origin` 検査を rmcp の表で済ますか自前か・tokio の閉じ込め方 | **要件では決めない**（設計の議題） | 利用者から見える振る舞いが変わらない | — |

## Boundary Context

- **In scope**:
  - `127.0.0.1` の 1 つのポートで HTTP を待ち受けること（ポートの決め方・環境変数・失敗の記録・終了で閉じること）。
  - `POST /api/mcp/v1` が MCP サーバとして応えること（`initialize` の版の交渉・`tools/list`〔0 本〕・`ping`・通知・無状態版の `server/discover`・エラーの形）。
  - ツールを登録する口（本 spec の登録は 0 本。テストで 1 本登録して口が通ることを確かめる）。
  - `Origin`（と `Host`）の検査。
  - `GET /api/mcp/help`（登録手順の案内）。
  - SSP の輸送の癖（survey §2 の表）との差の一覧と、クライアントが困る行の是正。
  - 新しい依存（rmcp・tokio・HTTP の土台とその推移依存）の追加と、ライセンスの門（`cargo deny check`・`THIRD-PARTY-NOTICES.md`）の通過。
  - 決定論テスト（実ソケット・ループバック・OS が割り当てる空きポート）と実機確認（Claude Code からの登録）。
  - `.kiro/steering/tech.md`（意図的依存追加の登記）・`structure.md`（新クレートの節）の更新。
- **Out of scope**:
  - ツールの定義と中身（`tools/list` の 10 本・引数の検査・`ghost_name` の解決・アプリ本体への橋）＝後続 `mcp-tool-entrances` と 3 段目の各 spec。本 spec の `tools/call` は、どの名前でもエラーで答える。
  - Claude Desktop 用の stdio ⇔ HTTP の中継 exe と、help の Desktop 用の設定例＝後続 `mcp-stdio-bridge`。本 spec の help は「Desktop は中継が要る」と書くだけ（例 0 件）。
  - 認証・TLS・リモート接続（`127.0.0.1` 以外での待受は 0 件）。
  - rmcp の中身に手を入れること（フォーク・パッチ 0 件）。差はクライアントが困る行だけ areka 側（自分の `ServerHandler` の実装・HTTP の層の前後）で直す。
  - `GET /api/mcp/v1` の手打ちフォーム（裁定 5）。
  - SSTP とその HTTP 経路。本 spec が 9801 で待ち受けても、受けるのは `/api/mcp/v1` と `/api/mcp/help` だけ（SSTP は 0 件）。SSTP を同じ口へ同居させるかは SSTP の spec が決める。
  - ゴーストが MCP クライアントになる構想（`doc/CONSTITUTION.md`「MCP 採用方針」の片側）。
  - SSP の欠陥 2 件（survey §4＝表情表の文字化け・script ログの JSON エスケープ漏れ）はツールの話で、本 spec では扱わない（0 件）。
  - `resources`・`prompts` の能力（SSP は空の配列を返すが実体が無い。areka は `capabilities` に載せず、差の一覧に書く）。
- **Adjacent expectations**:
  - 後続 `mcp-tool-entrances` は、本 spec の登録口を通してツール 10 本を足す。待受・ポート・`Origin`・help の振る舞いは変えない（本 spec の決定論テストが変わらず緑であること）。
  - 後続 `mcp-stdio-bridge` は、本 spec の help に Desktop 用の設定例を足し、`AREKA_MCP_PORT` を同じ意味で読む（既定は 9801 → 9821 → 隣の 20 候補。areka がどの候補で待ち受けているかの見つけ方は `mcp-stdio-bridge` の設計で決める）。`dist/README.txt` への `AREKA_MCP_PORT` の記述もそちらで書く（本 spec は書かない＝要件 2.7）。
  - 後続 `mcp-tool-entrances` は、本 spec の登録口の handler の形（同期か非同期か＝設計で決める）に従う。問い合わせの間に他の接続の要求を止めない（要件 1.5）のは後続でも変えない。
  - `perf_thread_report`（`crates/areka/src/perf_thread_report.rs`）の名簿に、本 spec が起こすスレッドが載ること（載らないスレッドは性能の報告に「名簿外」として出る）。設計で `areka_actor::spawn_actor` の作法に合わせる。
  - 依存を足す spec は 1 ウェーブに 1 本（`Cargo.lock`・`THIRD-PARTY-NOTICES.md`・`tech.md` が重なる）＝`animated-image-decode`・`mcp-stdio-bridge` と同じウェーブに置かない（roadmap の干渉台帳）。
  - 完了 spec の文書・`doc/ssp-mcp/survey.md` の本文は書き換えない。

## Requirements

### Requirement 1: areka が起動すると `127.0.0.1` で HTTP を待ち受ける

**Objective:** As a AI エージェントでゴーストを作る人, I want areka を起動するだけで MCP の受け口が開いていること, so that 設定を足さずに登録してすぐ使える

#### Acceptance Criteria

1. When areka が起動する（ゴーストの起動の成否・既定ゴーストへの切り戻しによらない）, the areka shall `127.0.0.1` の要件 2 で決まるポートで HTTP の待受を始める。待ち受けるのは IPv4 のループバック 1 つだけ（`0.0.0.0`・`::1`・LAN のアドレスでは受けない＝0 件）。
2. When 待受が始まる, the areka shall `info!` を 1 件残し、待ち受けている URL（`http://127.0.0.1:<実際の番号>/api/mcp/v1`）を載せる。束ねた番号が最初の候補でないときは、同じ 1 行に「先の候補が使用中なので移った」旨を載せる（行は増やさない＝1 件のまま）。
3. If 要件 2 で決まる候補のどれでも待受に失敗する（既定の 20 候補がすべて使用中・`AREKA_MCP_PORT` で指定した 1 つが使用中＝同じ番号で 2 つ目の areka を起動した・SSP と同じ番号を指定した、権限、など）, then the areka shall `error!` を 1 件残し（試した番号の範囲と最後の OS の理由を載せる）、ゴーストの起動・会話・メニュー・終了を妨げずに動き続ける。候補の外の番号は試さず（0 回）、同じ候補をもう一度試すこともせず（再試行 0 回）、利用者への画面の告知は出さない（0 件）。
4. The areka shall 待受の成否で終了コードを変えない（終了コードは待受と無関係）。
5. While 待受中, the areka shall 2 本以上の接続を同時に受け付け、1 本の接続が開いたままでも他の接続の要求に答える。
6. While 待受中, the areka shall 要求の処理でゴーストの描画・台詞の再生・メニュー・クリック透過の付け外しを止めない（要求の処理は UI のスレッドの時間を使わない）。
7. When アプリが終了する（メニューの終了・OS の終了・強制退避・自動終了のいずれでも）, the areka shall 待受を閉じ、終了の後にそのポートへ新しい接続ができない状態にする。
8. When アプリが終了するときに接続が開いたままである, the areka shall その接続を待たずに終了する（終了の手順が待受の後始末で止まらない）。
9. When 待受を閉じる, the areka shall `info!` を 1 件残す。
10. When 候補の 1 つを束ねられず次の候補へ進む, the areka shall `debug!` を 1 件残し、その番号と OS の理由を載せる（黙って飛ばさない）。

### Requirement 2: ポートの決め方と環境変数 `AREKA_MCP_PORT`

**Objective:** As a areka と SSP を同じ机で動かす開発者, I want 既定の番号を SSP と早い者勝ちで分け合い・ポートを環境変数で選べ・止めたいときは止められること, so that 後から起きた方も隣の番号で待ち受けられ、要らない机では口を閉じられる

#### Acceptance Criteria

1. The areka shall 既定の候補を 9801 → 9821 の順とし、その中で最初に束ねられた 1 つだけで待ち受ける（両方で待ち受けない＝待受は 1 つ）。両方とも束ねられなければ隣の番号へ逃げ、候補の全体を 9801・9821・9802・9822・9803・9823・…・9810・9830 の 20 個（9801〜9810 と 9821〜9830 を交互に）とする。
2. When `AREKA_MCP_PORT` が未設定である, the areka shall 既定の候補（要件 2.1）で待ち受ける（既定で有効）。
3. When `AREKA_MCP_PORT` が 1〜65535 の整数である（前後の空白は許す）, the areka shall その番号だけを試す（隣へ逃げない。束ねられなければ要件 1.3 の `error!` 1 件で待ち受けない）。
4. When `AREKA_MCP_PORT` が `0` である, the areka shall 待ち受けない（接続 0 件・スレッド 0 本）。`info!` を 1 件残し、待ち受けないことと理由（環境変数が `0`）を載せる。
5. If `AREKA_MCP_PORT` が空・空白だけ・数でない・負・65536 以上・溢れる値・UTF-8 でない値である, then the areka shall `warn!` を 1 件残し（その値を載せる。UTF-8 でなければその旨）、既定の候補（要件 2.1）で待ち受ける（1 つの番号へは倒さない）。
6. The areka shall 「値 → 試す候補の列」の対応を、環境変数を読まない判断として持ち、次の各値について決定論テストで固定する: 未設定 → 既定の候補・`0` → 空（待ち受けない）・`9821` → 9821 だけ・`65535` → 65535 だけ・`65536` → `warn!`＋既定の候補・`-1` → `warn!`＋既定の候補・`abc` → `warn!`＋既定の候補・空 → `warn!`＋既定の候補・` 9000 `（空白付き）→ 9000 だけ。
7. The areka shall 環境変数の名前を `AREKA_MCP_PORT` の 1 つだけとし（`AREKA_` の冠・新しい名前 0 個）、help の本文（要件 6）以外に別名を書かない。`dist/README.txt` には本 spec では書かない（roadmap の C1 の行で本 spec が触るファイルの外＝配布の spec 群と分け合うファイル。後続 `mcp-stdio-bridge` が Desktop の設定例と一緒に書く）。
8. The areka shall 既定の候補の 20 個の並び（要件 2.1）を決定論テストで固定する（個数・先頭の 9801 と 9821・末尾の 9810 と 9830・交互の順）。
9. The areka shall 「使用中の候補を飛ばして次の候補で待ち受ける」振る舞いを、OS が割り当てる空きポートだけを使う実ソケットの決定論テストで固定する（先に空きポートを 1 つ占め、その番号と「OS に任せる」の 2 つを候補に渡すと、占めた番号でない番号で待ち受け、飛ばした分の `debug!` が 1 件出る）。候補をすべて占めたときに `error!` 1 件で待ち受けないことも同じく固定する。テストは 9801・9821 その他の固定の番号を束ねない（0 本）。

### Requirement 3: `POST /api/mcp/v1` が MCP サーバとして応える（ツール 0 本）

**Objective:** As a Claude Code・Cursor の利用者, I want `claude mcp add --transport http` で登録した areka が規格どおりに答えること（旧式の版は `initialize`→`tools/list`→`ping`、無状態版〔`2026-07-28`〕は `server/discover`→`tools/list`。Claude Code 2.1.283 は無状態版でつなぐ）, so that 登録が通り、後続の spec でツールが増えたときそのまま使える

#### Acceptance Criteria

1. When `initialize` が届く, the areka shall 成功の結果を返し、`protocolVersion` は旧式の 4 版（`2025-11-25`・`2025-06-18`・`2025-03-26`・`2024-11-05`）なら要求どおり、`2026-07-28`（`initialize` を持たない版）なら rmcp が倒す `2025-11-25`（差の一覧に書く。SSP は `2026-07-28` をそのまま返す）、`capabilities` は `tools` を含み `resources`・`prompts` を含まず、`serverInfo` は `{"name":"areka-mcp-server","version":<areka の Cargo の版>}`、`instructions` は空でない英文（1〜3 文・内容は設計で決める）とする。
2. When `initialize` が未知の版（例 `1999-01-01`）を要求する, the areka shall 失敗にせず、サーバが対応する版の 1 つを `protocolVersion` に返す（どの版かは rmcp が決め、差の一覧〔要件 5〕に書く）。
3. When `notifications/initialized`（`id` の無い通知）が届く, the areka shall `202 Accepted`・本文なしで答える。
4. When `tools/list` が届く, the areka shall `tools` が空の配列（0 本）の結果を返す。
5. When `ping` が届く, the areka shall `result: {}` を返す。
6. When `tools/call` が届く（どの名前・どの引数でも）, the areka shall JSON-RPC のエラー（結果の `isError` ではない）`-32602`・`Invalid params` で答える（ツールが 0 本なので全部が未知の名前）。
7. When 未知のメソッド（例 `no/such`）が届く, the areka shall JSON-RPC のエラーで答える（コードは rmcp のまま＝`-32601` の見込み・HTTP の状態コードも rmcp のまま＝旧式の経路では 200。SSP は `-32601`・200＝測った値を差の一覧に書く。areka がメソッドの表を持って `-32601` を作ることはしない）。
8. If 本文が JSON でない, then the areka shall HTTP の 4xx で答え（`Content-Type` が JSON でない場合も本文が JSON として壊れている場合も rmcp のまま＝415・平文の見込み。SSP は 400・`-32700`＝差の一覧に書く）、接続と待受を落とさず次の要求に答える。
9. The areka shall セッション ID（`Mcp-Session-Id`）を要求せず、`initialize` を経ていない接続からの `ping`・`tools/list` にも答える（無状態）。
10. The areka shall 結果かエラーで終わる要求へ `Content-Type: application/json` の単発の応答で答え、SSE（`text/event-stream`）を使わない（0 本）。
11. The areka shall `GET /api/mcp/v1` に手打ちフォームを出さない（SSP のフォームは移植しない＝0 ページ。応答は rmcp のまま＝差の一覧に書く）。
12. When 無状態版の `server/discover` が届く, the areka shall `supportedVersions`・`capabilities`・`instructions`・`serverInfo` を含む結果を返す（SSP の `ttlMs`・`cacheScope`・`resultType` の有無は rmcp のまま＝差の一覧に書く）。
13. When `MCP-Protocol-Version` ヘッダが旧式の 4 版のいずれか・または無い, the areka shall `ping`・`tools/list` に要件 3.4・3.5 のとおり答える（ヘッダの有無で結果を変えない）。5 版のいずれでもない値への応答は rmcp のまま（400 の見込み。SSP は無状態版の検査へ回す＝差の一覧に書く）。
14. When 要求を 1 件処理する, the areka shall `debug!` を 1 件残し、メソッド名・`id`・HTTP の状態コードを載せる（`RUST_LOG` で絞って実機の確かめに使える）。
15. When 無状態版（`2026-07-28`）の `tools/list` が届く, the areka shall 結果に `ttlMs`・`cacheScope`・`resultType` を含める（`ttlMs: 0`・`cacheScope: "private"`。旧式の版の `tools/list` の結果は変えない）。

### Requirement 4: `Origin` と `Host` の検査（ループバック以外は拒む）

**Objective:** As a areka を常駐させている利用者, I want ブラウザで開いた他所のページから areka の口を叩けないこと, so that 待ち受けていても外から悪用されない

#### Acceptance Criteria

1. When `Origin` ヘッダが無い, the areka shall 要求を通す（`curl`・Claude Code・Cursor は `Origin` を付けない）。
2. When `Origin` の host が `localhost`・`127.0.0.1`・`[::1]` のいずれか（ポートの有無と番号を問わない・`http` と `https` の両方）である, the areka shall 要求を通す。
3. If `Origin` がそれ以外（`http://evil.example`・`null`・ループバックでないアドレス・host の無い値）である, then the areka shall HTTP `403` で拒み、MCP の処理に入らない（`initialize` でも `ping` でも `GET /api/mcp/help` でも同じ）。拒んだことを `warn!` に 1 件残し、その `Origin` の値を載せる。
4. If `Host` ヘッダの host が `localhost`・`127.0.0.1`・`[::1]` のいずれでもない, then the areka shall HTTP `403` で拒む（`Host: 127.0.0.1:<port>`・`Host: localhost:<port>` は通る）。SSP は `Host` を検査しない＝差の一覧（要件 5）に「困らない差」として書く。
5. The areka shall 検査の通す／拒む条件を、要求の生の値を使わない判断として持ち、`Origin` の無し・`http://localhost`・`http://localhost:3000`・`http://127.0.0.1:9821`・`https://localhost`・`http://evil.example`・`null`・`http://localhost.evil.example` の各値について決定論テストで固定する。

### Requirement 5: SSP の輸送の癖との差を一覧にする

**Objective:** As a SSP から areka へ移るゴーストの作者と後続 spec の実装者, I want SSP と areka の振る舞いの違いが 1 枚で分かること, so that 「SSP ではこうだった」を調べ直さずに済む

#### Acceptance Criteria

1. The areka shall `doc/ssp-mcp/survey.md` §2 の表の **15 行**（応答・通知・版の交渉・`MCP-Protocol-Version` ヘッダ・無状態版・`ping`・未知メソッド・JSON でない本文・バッチ・未知のツール名・エラーの形・`Origin`・`Host`・`serverInfo`・`GET /api/mcp/v1` のフォーム〔§1〕）の各行について、areka の振る舞いを実ソケットで測り、`doc/ssp-mcp/` の下の 1 文書に「同じ／違うが困らない／困るので直した」のいずれかを行ごとに書く（空欄 0 行）。
2. The areka shall 「困る」の物差しを「Claude Code（`claude mcp add --transport http`）または Cursor が登録・`initialize`（無状態版では `server/discover`）・`tools/list`・`ping` のいずれかに失敗する差」とし、物差しに当たる行だけを areka 側で直す（rmcp の中身には手を入れない）。
3. When 差を直す, the areka shall 直した行ごとに決定論テストを 1 本以上置き、一覧にそのテストの名前を書く。
4. The areka shall 一覧に、本 spec で測った rmcp の版と測った日を書く（版を上げたときに測り直す目印）。

### Requirement 6: `GET /api/mcp/help` が登録手順を案内する

**Objective:** As a はじめて areka に AI エージェントをつなぐ人, I want ブラウザで 1 ページ開けば登録のコマンドが分かること, so that 説明書を探さずに登録できる

#### Acceptance Criteria

1. When `GET /api/mcp/help` が届く, the areka shall HTTP `200`・`Content-Type: text/html; charset=utf-8` の日本語のページを返し、次の 5 つを載せる: ⑴ 今まさに待ち受けている URL（`http://127.0.0.1:<port>/api/mcp/v1`・`<port>` は実際の番号）、⑵ Claude Code の登録コマンド `claude mcp add --transport http areka http://127.0.0.1:<port>/api/mcp/v1`、⑶ Cursor 向けの設定の断片（`mcpServers` の `url` の形）、⑷ ポートの決まり方と変え方（既定は 9801 → 9821 の早い者勝ち・どちらも使用中なら隣の番号〔9802・9822 …〕へ・`AREKA_MCP_PORT`・`0` で待ち受けない）、⑸ Claude Desktop は HTTP を直接書けないので中継が要ること（中継は後続 spec＝設定例 0 件）。
2. When 待ち受けている番号が既定の最初の候補でない（隣へ逃げた・`AREKA_MCP_PORT` で選んだ）, the areka shall help の URL とコマンド例に実際の番号を載せる（9801・9821 を固定で書かない）。
3. The areka shall help のページに `Origin` の検査を同じく適用する（要件 4。ブラウザのアドレス欄から開く要求には `Origin` が無いので通る）。
4. If `/api/mcp/v1` と `/api/mcp/help` 以外のパス（`/`・`/api/mcp/`・`/api/mcp/help/x` など）に要求が届く, then the areka shall HTTP `404` で答える（他のページ 0 枚）。
5. If `/api/mcp/help` に `GET` 以外のメソッドが届く, then the areka shall HTTP の 4xx で答え、ページを返さない。

### Requirement 7: ツールを登録する口（本 spec の登録は 0 本）

**Objective:** As a 後続 `mcp-tool-entrances` の実装者, I want 待受やプロトコルの実装に触らずにツールを足せる口があること, so that 3 段目の spec 群が自分のファイルだけを触って並走できる

#### Acceptance Criteria

1. The areka shall ツールの一覧（`tools/list`）と呼び出し（`tools/call`）を、ツールを登録する 1 つの口に登録された内容から答える。本番の登録は 0 本。
2. When テストが口に 1 本のツール（名前・説明・引数の形・実装）を登録する, the areka shall `tools/list` にその 1 本を返し、その名前の `tools/call` をその実装へ届けて実装の返した結果（`content` と `isError`）をそのまま返す。
3. When テストが口に登録した名前以外で `tools/call` が届く, the areka shall 要件 3.6 のとおり `-32602` で答える。
4. The areka shall 口の形（登録に要る情報）を、MCP のツール定義の名前・title・description・inputSchema を逐語で渡せる形にする（後続 spec が SSP の定義を逐語で詰められる＝自動生成に縛らない）。

### Requirement 8: 依存とライセンスの門

**Objective:** As a areka を MIT で配り続ける開発者, I want 新しい依存を入れてもライセンスの門が緑のままであること, so that 配布物の告知が機械で作り直せる

#### Acceptance Criteria

1. The areka shall rmcp・tokio・HTTP の土台とその推移依存を足したうえで、`pwsh -NoProfile -File tools/test-all.ps1 -Format -License`（`cargo deny check`・`cargo about generate` を含む）を通す。
2. The areka shall rmcp の版を `=` で固定し（2026-10-03 時点の最新 3.5.0）、固定した版を `Cargo.toml`（`crates/areka-mcp`）に書く。
3. If 推移依存に `deny.toml` の許可の表（9 つ）に無いライセンスが現れる, then the areka shall 黙って表へ足さず、そのクレート名とライセンスを要件ディスカッションか設計の議題として開発者へ示し、承認を得てから表へ足す。
4. The areka shall `THIRD-PARTY-NOTICES.md` を `tools/test-all.ps1 -License` で作り直す（手で直さない＝手の差分 0 行）。
5. The areka shall 新しい依存を `.kiro/steering/tech.md` の Key Libraries に登記し（rmcp・tokio の用途と「tokio は MCP のスレッドに閉じる」の旨）、新クレートの節を `structure.md` に足す。
6. The areka shall 依存を `crates/areka-mcp/Cargo.toml` と `crates/areka/Cargo.toml`（`areka-mcp` の 1 行）だけに書き、根の `Cargo.toml` の `[workspace.dependencies]` は rmcp・tokio のために変えない（変える行 0）。

### Requirement 9: 決定論テストと実機確認

**Objective:** As a 後で rmcp の版を上げる開発者, I want 受け口の振る舞いが実ソケットのテストで固定されていること, so that 版上げで何が変わったかがテストの赤で分かる

#### Acceptance Criteria

1. The areka shall 常時テスト（`cargo test`・`tools/test-all.ps1`）でネットへ出ない（ループバックのみ）。ポートは OS が割り当てる空きポート（`127.0.0.1:0`）をテストごとに取り、固定の番号を使わない（9801・9821 を束ねるテスト 0 本）。
2. The areka shall 実ソケットで次を決定論テストに持つ: `initialize` の旧式 4 版が要求どおり（3.1）・`2026-07-28` が `2025-11-25` へ倒れる（3.1）・未知の版（3.2）・`notifications/initialized` の 202（3.3）・`tools/list` が 0 本（3.4）・`ping`（3.5）・`tools/call` の `-32602`（3.6）・未知メソッドのエラー（3.7）・JSON でない本文（3.8＝`Content-Type` 違いと壊れた JSON の 2 本）・5 版のいずれでもない `MCP-Protocol-Version`（3.13）・`initialize` 無しの `ping`（3.9）・`application/json` の単発（3.10）・`server/discover`（3.12）・無状態版の `tools/list` の `ttlMs`・`cacheScope`（3.15）・悪い `Origin` の 403（4.3）・悪い `Host` の 403（4.4）・help の 200 と本文（6.1・6.2）・未知のパスの 404（6.4）・登録 1 本の往復（7.2・7.3）。
3. The areka shall 待受の失敗を決定論テストで踏む: 先に `std::net::TcpListener` で空きポートを占め、その番号だけを候補に待受を始めると `error!` が 1 件出てアプリ側の処理が続く（1.3）。占めた番号の次に「OS に任せる」を置くと飛ばして待ち受け、`debug!` 1 件と移った旨の `info!` が出る（1.2・1.10・2.9）。ログの捕捉は `log-capture-kit` を通す。
4. The areka shall 終了を決定論テストで踏む: 待受を畳んだ後に同じポートへ接続できない（1.7）。接続を開いたまま畳んでも畳む側が有限の時間で戻る（1.8）。
5. The areka shall 実機で確かめ、結果を `.kiro/specs/areka-P0-mcp-server-core/verification/signoff.md` に残す: ⑴ 配布形の `areka.exe` を起動して `info!`（または help）が示す URL で `claude mcp add --transport http areka http://127.0.0.1:<実際の番号>/api/mcp/v1` と登録し、`claude mcp list` が接続できたことを示す、⑵ `curl` で `initialize`→`tools/list`→`ping` が通る、⑶ `AREKA_MCP_PORT=0` で起動すると接続できず `info!` が出る、⑷ SSP が 9801 と 9821 で待ち受けている机で areka を起動すると、9802（またはその先の空いた候補）で待ち受け、そこへの `ping` が 200 で答え、help がその番号を示し、`info!` が移った旨を載せる。
6. The areka shall 実機確認の走行で `RUST_LOG` に要件 3.14 の `debug!` が出る階層まで開け、拒んだ要求（4.3）の `warn!` が記録に残ることを確かめる。

# Requirements Document

> 本文の実測は **2026-10-03・本ブランチ**（main `76e17654`＝棚卸⑳の PR#211 のコミット。brief の 10-02 の再測定〔main `03e8d7d6`〕からコードの変更は 0 件で、変わったのは roadmap だけ）のもの。コードは「何の定義か」（関数名・型名・定数名＋ファイルパス）で指し、行番号では指さない。
> 「要件の段での暫定の裁定」の表は、要件を書く途中で答えが要った点に対する**推奨案による暫定の確定**であり、要件ディスカッションで覆せる（覆したら該当要件も改める）。答えで作業が変わる議題は見込み 2 件（既定ポートと既定で有効にするか・`serverInfo` の名前）で、どちらも表に載せた。

## Project Description (Input)

**誰の何が困っているか**: AI エージェント（Claude Code・Cursor など）でゴーストを作る人は、SSP なら内蔵の MCP サーバ経由で台本を流し・イベントを起こし・絵を撮り・ログを読めるが、areka には受け口が無く、同じ開発の回し方ができない。areka の開発そのものも、動いている areka への確かめを `AREKA_APP_SMOKE_EXIT_MS` とログの grep に頼っている。

**今の状態**: areka 本体にサーバの類は 0（本番の `std::net`・tokio・HTTP の土台いずれも無し）。MCP の受け口も、ツールも、登録の案内も無い。

**何を変えるか**: areka が起動すると `127.0.0.1:<port>`（既定 9821・`AREKA_MCP_PORT` で変更・`0` で待ち受けない）で HTTP を受け、`POST /api/mcp/v1` が MCP サーバ（公式 Rust SDK `rmcp`・無状態・JSON の単発応答）として `initialize`→`tools/list`→`ping` に応える。ツールはまだ 0 本で、後続 spec がツールを足すための登録口だけを持つ。`GET /api/mcp/help` で登録手順を示し、`Origin` がループバック以外なら拒否する。待受の失敗はログに理由を 1 件残してアプリは動き続ける。SSP の輸送の癖との差は一覧にし、クライアントが困る行だけ直す。

> 起票: 2026-09-29 `/kiro-discovery`（開発者指示「ssp mcp tool の完全移植のための spec 群を立ち上げて。実装は α リリースの後。areka の 127.0.0.1 の適当なポートでサーバを開く形。基本実装 → 空のダミー関数を置いて入り口だけ全部整備 → 個別のコマンド実装」）。SSP MCP 移植の **1 段目（基本実装）**。事実の正本は [doc/ssp-mcp/survey.md](../../../doc/ssp-mcp/survey.md)（SSP 2.9.05 の実測）。全体の並びは `.kiro/steering/roadmap.md`「SSP MCP の移植」節。

## Introduction

### 誰が困っているか

- **AI エージェントでゴーストを作る人**: ukadoc の「その他の機能」は、SSP の内蔵 MCP サーバを「AI エージェントでゴーストを作りたい（Vibe Coding）」の土台として案内している。Claude Code・Cursor は HTTP の MCP サーバを直接登録できる（`claude mcp add --transport http <名> <URL>`）。areka には登録する先が無い。
- **areka の開発者**: 実機の確かめは有界の自動終了とログの grep だけで、動いている areka に「今どうなっているか」を問えない。本 spec でツールは増えないが、受け口ができれば後続の spec（`get_status`・`get_log` など）がそれを埋める。

### いま何が起きているか（2026-10-03 実測）

- **本番のサーバは 0。** brief の 10-02 の再測定のとおり（`crates/**/src/` の本番に `std::net`・tokio のランタイム・HTTP の土台は無い）。テストには前例が 1 つ: `crates/areka-update/src/winhttp_real_tests.rs` が `std::net::TcpListener::bind("127.0.0.1:0")` で偽の HTTP サーバを立てている（OS に空きポートを割り当てさせる形）。
- **起動と終了の配線の前例。** `crates/areka/src/main.rs` の `fn main()` は 1 つで、裏方を立てて畳む前例は `perf_thread_report::start()`／`stop_and_report_final()`。brief が注意するとおり、終了の閉包の中の早い `return` で後始末が飛びうるので、畳む処理は早い戻りより前か `Drop` に置く（設計の話＝ここでは要件 1 の「終了で待受が閉じる」だけを求める）。
- **環境変数の前例。** `AREKA_APP_SMOKE_EXIT_MS`（`crates/areka/src/main.rs` の `SMOKE_EXIT_ENV` と、env を読まない純粋な読み解きの関数）は「未設定・空・数でない・負・溢れ → 無効（発火しない）」。`AREKA_ROOT`（`crates/areka/src/boot_config.rs` の `resolve_root_from`）は「空でも『設定あり』として扱い、exe の隣へ倒さない」。本 spec の `AREKA_MCP_PORT` は、`0` を「待ち受けない」の明示の値に使うので、数でない値を「無効」へ倒すと `0` と区別が付かなくなる＝数でない値は既定へ倒して `warn!` で知らせる（裁定 2）。
- **ライセンスの門。** `deny.toml` の許可の表は 9 つ（MIT・Apache-2.0・Apache-2.0 WITH LLVM-exception・Unlicense・Zlib・0BSD・BSD-2-Clause・BSD-3-Clause・Unicode-3.0）。表に無いライセンスの推移依存が 1 つでも入ると `cargo deny check` が赤になる。rmcp（MIT）・tokio（MIT）・hyper／axum／tower（MIT）は表の中だが、推移依存の全部は実装で入れてみるまで分からない。
- **rmcp の現物（2026-10-03・crates.io の最新は 3.5.0〔2026-09-28〕・rust-sdk の `main` の `crates/rmcp/src/transport/streamable_http_server/tower.rs` を読んだ）。** `StreamableHttpServerConfig` に ⑴ `legacy_session_mode`（**既定 `true`＝セッション有り**。`false` で無状態）、⑵ `json_response`（既定 `false`。`true` かつ無状態のとき、結果かエラーで終わる要求へ `application/json` の単発で答える・通知は `202 Accepted`）、⑶ `allowed_hosts`（**既定 `localhost`・`127.0.0.1`・`::1`**＝それ以外の `Host` は 403）、⑷ `allowed_origins`＋`enforce_origin_validation()`（既定は検査なし。有効にすると許可の表に無い `Origin` は 403・ポートのワイルドカード `:*` 可）、⑸ `stateless_protocol_metadata_required`（既定 `false`＝2026-07-28 の `_meta` を強いない）、⑹ `max_request_body_bytes`（既定 4 MiB）がある。`server/discover` は版の交渉の対象として扱われる。JSON でない本文は **415**、`MCP-Protocol-Version` の無い要求は `2025-03-26` と見なす。これらが 3.5.0 に入っているかは、設計で版を固定するときに `Cargo.lock` の現物で確かめる。

### brief の記述を実物で引き直して改めた点

1. brief の「無状態の既定と `with_json_response(true)`」は半分違う。rmcp の既定は**セッション有り**（`legacy_session_mode = true`）で、無状態は明示して選ぶ。JSON の単発応答も明示して選ぶ。要件 3 は「無状態・JSON の単発」を利用者から見える形（セッション ID を要求しない・応答が `application/json`）で求め、選び方は設計に任せる。
2. brief の「`Origin` 検査（rmcp に同等の設定があればそれを使う）」は、rmcp に許可の表の形で在る。要件 4 は通す／拒む条件だけを決め、rmcp の表で足りるか自前かは設計で決める。
3. brief と survey に無かった事実: rmcp は既定で **`Host` も検査する**（ループバック以外は 403）。SSP は `Host` を検査しない（survey §2）。areka は rmcp の既定を採って `Host` も拒む（裁定 6）＝SSP との差として一覧に載せる。
4. brief の「JSON でない本文 → 400・`-32700`」（SSP の振る舞い）は、rmcp では 415。クライアントは JSON 以外を送らないので差として受け入れる（要件 5）。
5. brief の「無状態版で `_meta` が無ければ 400」（SSP）は、rmcp の既定では起きない（`stateless_protocol_metadata_required = false`）。クライアントに優しい側の差なので受け入れる。

### 要件の段での暫定の裁定（要件ディスカッションで覆せる）

| # | 議題（brief） | 暫定の裁定 | 根拠 | 載せた要件 |
|---|---|---|---|---|
| 1 | 既定ポート 9821・既定で有効にするか | **9821・既定で有効**（SSP と同じく常に待ち受ける） | 9801 は開発者の机で SSP が使い、将来の SSTP の口。既定で無効だと「登録したのにつながらない」が最初の体験になる | 2.1・2.2 |
| 2 | `AREKA_MCP_PORT` が数でない・範囲外のとき | **`warn!` 1 件を残して既定 9821 で待ち受ける**（無効へ倒さない） | `0` が「待ち受けない」の明示の値なので、書き損じで黙って止まる形を避ける | 2.5 |
| 3 | 待受に失敗したら記録だけか別のポートを試すか | **記録だけ**（`error!` 1 件・再試行 0 回・別のポート 0 回） | 別のポートへ逃げると help の URL と登録済みのクライアントの URL が食い違う。`AREKA_MCP_PORT` で選び直せる | 1.3 |
| 4 | `serverInfo` の名前と版 | **`areka-mcp-server`・版は areka の Cargo の版**（今は `0.0.1`） | SSP の `ssp-mcp-server`／`2.9.05` の型に倣う | 3.1 |
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
  - SSTP（9801）とその HTTP 経路。MCP を SSTP の口へ同居させるかは SSTP の spec が決める。本 spec は 9801 を開かない。
  - ゴーストが MCP クライアントになる構想（`doc/CONSTITUTION.md`「MCP 採用方針」の片側）。
  - SSP の欠陥 2 件（survey §4＝表情表の文字化け・script ログの JSON エスケープ漏れ）はツールの話で、本 spec では扱わない（0 件）。
  - `resources`・`prompts` の能力（SSP は空の配列を返すが実体が無い。areka は `capabilities` に載せず、差の一覧に書く）。
- **Adjacent expectations**:
  - 後続 `mcp-tool-entrances` は、本 spec の登録口を通してツール 10 本を足す。待受・ポート・`Origin`・help の振る舞いは変えない（本 spec の決定論テストが変わらず緑であること）。
  - 後続 `mcp-stdio-bridge` は、本 spec の help に Desktop 用の設定例を足し、`AREKA_MCP_PORT` を同じ意味で読む（既定 9821）。
  - `perf_thread_report`（`crates/areka/src/perf_thread_report.rs`）の名簿に、本 spec が起こすスレッドが載ること（載らないスレッドは性能の報告に「名簿外」として出る）。設計で `areka_actor::spawn_actor` の作法に合わせる。
  - 依存を足す spec は 1 ウェーブに 1 本（`Cargo.lock`・`THIRD-PARTY-NOTICES.md`・`tech.md` が重なる）＝`animated-image-decode`・`mcp-stdio-bridge` と同じウェーブに置かない（roadmap の干渉台帳）。
  - 完了 spec の文書・`doc/ssp-mcp/survey.md` の本文は書き換えない。

## Requirements

### Requirement 1: areka が起動すると `127.0.0.1` で HTTP を待ち受ける

**Objective:** As a AI エージェントでゴーストを作る人, I want areka を起動するだけで MCP の受け口が開いていること, so that 設定を足さずに登録してすぐ使える

#### Acceptance Criteria

1. When areka が起動する（ゴーストの起動の成否・既定ゴーストへの切り戻しによらない）, the areka shall `127.0.0.1` の要件 2 で決まるポートで HTTP の待受を始める。待ち受けるのは IPv4 のループバック 1 つだけ（`0.0.0.0`・`::1`・LAN のアドレスでは受けない＝0 件）。
2. When 待受が始まる, the areka shall `info!` を 1 件残し、待ち受けている URL（`http://127.0.0.1:<port>/api/mcp/v1`）を載せる。
3. If 待受に失敗する（ポートが使用中＝同じポートで 2 つ目の areka を起動した・SSP と同じ番号を指定した、権限、など）, then the areka shall `error!` を 1 件残し（ポート番号と OS の理由を載せる）、ゴーストの起動・会話・メニュー・終了を妨げずに動き続ける。別のポートは試さず（0 回）、再試行もせず（0 回）、利用者への画面の告知は出さない（0 件）。
4. The areka shall 待受の成否で終了コードを変えない（終了コードは待受と無関係）。
5. While 待受中, the areka shall 2 本以上の接続を同時に受け付け、1 本の接続が開いたままでも他の接続の要求に答える。
6. While 待受中, the areka shall 要求の処理でゴーストの描画・台詞の再生・メニュー・クリック透過の付け外しを止めない（要求の処理は UI のスレッドの時間を使わない）。
7. When アプリが終了する（メニューの終了・OS の終了・強制退避・自動終了のいずれでも）, the areka shall 待受を閉じ、終了の後にそのポートへ新しい接続ができない状態にする。
8. When アプリが終了するときに接続が開いたままである, the areka shall その接続を待たずに終了する（終了の手順が待受の後始末で止まらない）。
9. When 待受を閉じる, the areka shall `info!` を 1 件残す。

### Requirement 2: ポートの決め方と環境変数 `AREKA_MCP_PORT`

**Objective:** As a areka と SSP を同じ机で動かす開発者, I want ポートを環境変数で選べ・止めたいときは止められること, so that 9801 の SSP と衝突せず、要らない机では口を閉じられる

#### Acceptance Criteria

1. The areka shall 既定のポートを 9821 とする（9801 は使わない）。
2. When `AREKA_MCP_PORT` が未設定である, the areka shall 既定の 9821 で待ち受ける（既定で有効）。
3. When `AREKA_MCP_PORT` が 1〜65535 の整数である（前後の空白は許す）, the areka shall その番号で待ち受ける。
4. When `AREKA_MCP_PORT` が `0` である, the areka shall 待ち受けない（接続 0 件・スレッド 0 本）。`info!` を 1 件残し、待ち受けないことと理由（環境変数が `0`）を載せる。
5. If `AREKA_MCP_PORT` が空・空白だけ・数でない・負・65536 以上・溢れる値である, then the areka shall `warn!` を 1 件残し（その値を載せる）、既定の 9821 で待ち受ける。
6. The areka shall 「値 → ポート／待ち受けない／既定へ倒す」の対応を、環境変数を読まない判断として持ち、未設定・`0`・`9821`・`65535`・`65536`・`-1`・`abc`・空・` 9000 `（空白付き）の各値について決定論テストで固定する。
7. The areka shall 環境変数の名前を `AREKA_MCP_PORT` の 1 つだけとし（`AREKA_` の冠・新しい名前 0 個）、help の本文（要件 6）と `dist/README.txt` 以外の文書に別名を書かない。

### Requirement 3: `POST /api/mcp/v1` が MCP サーバとして応える（ツール 0 本）

**Objective:** As a Claude Code・Cursor の利用者, I want `claude mcp add --transport http` で登録した areka が規格どおりに `initialize`→`tools/list`→`ping` に答えること, so that 登録が通り、後続の spec でツールが増えたときそのまま使える

#### Acceptance Criteria

1. When `initialize` が届く, the areka shall 成功の結果を返し、`protocolVersion` は要求の版（`2026-07-28`・`2025-11-25`・`2025-06-18`・`2025-03-26`・`2024-11-05` の 5 版はいずれも要求どおり）、`capabilities` は `tools` を含み `resources`・`prompts` を含まず、`serverInfo` は `{"name":"areka-mcp-server","version":<areka の Cargo の版>}`、`instructions` は空でない英文（1〜3 文・内容は設計で決める）とする。
2. When `initialize` が未知の版（例 `1999-01-01`）を要求する, the areka shall 失敗にせず、サーバが対応する版の 1 つを `protocolVersion` に返す（どの版かは rmcp が決め、差の一覧〔要件 5〕に書く）。
3. When `notifications/initialized`（`id` の無い通知）が届く, the areka shall `202 Accepted`・本文なしで答える。
4. When `tools/list` が届く, the areka shall `tools` が空の配列（0 本）の結果を返す。
5. When `ping` が届く, the areka shall `result: {}` を返す。
6. When `tools/call` が届く（どの名前・どの引数でも）, the areka shall JSON-RPC のエラー（結果の `isError` ではない）`-32602`・`Invalid params` で答える（ツールが 0 本なので全部が未知の名前）。
7. When 未知のメソッド（例 `no/such`）が届く, the areka shall JSON-RPC のエラー `-32601`（`Method not found`）で答える（HTTP の状態コードは rmcp のまま＝差の一覧に書く）。
8. If 本文が JSON でない, then the areka shall エラーで答え（HTTP の 4xx か JSON-RPC の `-32700`。どちらかは rmcp のまま＝差の一覧に書く）、接続と待受を落とさず次の要求に答える。
9. The areka shall セッション ID（`Mcp-Session-Id`）を要求せず、`initialize` を経ていない接続からの `ping`・`tools/list` にも答える（無状態）。
10. The areka shall 結果かエラーで終わる要求へ `Content-Type: application/json` の単発の応答で答え、SSE（`text/event-stream`）を使わない（0 本）。
11. The areka shall `GET /api/mcp/v1` に手打ちフォームを出さない（SSP のフォームは移植しない＝0 ページ。応答は rmcp のまま＝差の一覧に書く）。
12. When 無状態版の `server/discover` が届く, the areka shall `supportedVersions`・`capabilities`・`instructions`・`serverInfo` を含む結果を返す（SSP の `ttlMs`・`cacheScope`・`resultType` の有無は rmcp のまま＝差の一覧に書く）。
13. When `MCP-Protocol-Version` ヘッダが旧式の 4 版のいずれか・または無い, the areka shall `ping`・`tools/list` に要件 3.4・3.5 のとおり答える（ヘッダの有無で結果を変えない）。
14. When 要求を 1 件処理する, the areka shall `debug!` を 1 件残し、メソッド名・`id`・HTTP の状態コードを載せる（`RUST_LOG` で絞って実機の確かめに使える）。

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
2. The areka shall 「困る」の物差しを「Claude Code（`claude mcp add --transport http`）または Cursor が登録・`initialize`・`tools/list`・`ping` のいずれかに失敗する差」とし、物差しに当たる行だけを areka 側で直す（rmcp の中身には手を入れない）。
3. When 差を直す, the areka shall 直した行ごとに決定論テストを 1 本以上置き、一覧にそのテストの名前を書く。
4. The areka shall 一覧に、本 spec で測った rmcp の版と測った日を書く（版を上げたときに測り直す目印）。

### Requirement 6: `GET /api/mcp/help` が登録手順を案内する

**Objective:** As a はじめて areka に AI エージェントをつなぐ人, I want ブラウザで 1 ページ開けば登録のコマンドが分かること, so that 説明書を探さずに登録できる

#### Acceptance Criteria

1. When `GET /api/mcp/help` が届く, the areka shall HTTP `200`・`Content-Type: text/html; charset=utf-8` の日本語のページを返し、次の 5 つを載せる: ⑴ 今まさに待ち受けている URL（`http://127.0.0.1:<port>/api/mcp/v1`・`<port>` は実際の番号）、⑵ Claude Code の登録コマンド `claude mcp add --transport http areka http://127.0.0.1:<port>/api/mcp/v1`、⑶ Cursor 向けの設定の断片（`mcpServers` の `url` の形）、⑷ ポートの変え方（`AREKA_MCP_PORT`・`0` で待ち受けない）、⑸ Claude Desktop は HTTP を直接書けないので中継が要ること（中継は後続 spec＝設定例 0 件）。
2. When `AREKA_MCP_PORT` で既定以外の番号を選んでいる, the areka shall help の URL とコマンド例にその番号を載せる（9821 を固定で書かない）。
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

1. The areka shall 常時テスト（`cargo test`・`tools/test-all.ps1`）でネットへ出ない（ループバックのみ）。ポートは OS が割り当てる空きポート（`127.0.0.1:0`）をテストごとに取り、固定の番号を使わない（9821 を使うテスト 0 本）。
2. The areka shall 実ソケットで次を決定論テストに持つ: `initialize` の 5 版（3.1）・未知の版（3.2）・`notifications/initialized` の 202（3.3）・`tools/list` が 0 本（3.4）・`ping`（3.5）・`tools/call` の `-32602`（3.6）・未知メソッドの `-32601`（3.7）・JSON でない本文（3.8）・`initialize` 無しの `ping`（3.9）・`application/json` の単発（3.10）・`server/discover`（3.12）・悪い `Origin` の 403（4.3）・悪い `Host` の 403（4.4）・help の 200 と本文（6.1・6.2）・未知のパスの 404（6.4）・登録 1 本の往復（7.2・7.3）。
3. The areka shall 待受の失敗を決定論テストで踏む: 先に `std::net::TcpListener` で空きポートを占め、その番号で待受を始めると `error!` が 1 件出てアプリ側の処理が続く（1.3）。ログの捕捉は `log-capture-kit` を通す。
4. The areka shall 終了を決定論テストで踏む: 待受を畳んだ後に同じポートへ接続できない（1.7）。接続を開いたまま畳んでも畳む側が有限の時間で戻る（1.8）。
5. The areka shall 実機で確かめ、結果を `.kiro/specs/areka-P0-mcp-server-core/verification/signoff.md` に残す: ⑴ 配布形の `areka.exe` を起動して `claude mcp add --transport http areka http://127.0.0.1:9821/api/mcp/v1` で登録し、`claude mcp list` が接続できたことを示す、⑵ `curl` で `initialize`→`tools/list`→`ping` が通る、⑶ `AREKA_MCP_PORT=0` で起動すると接続できず `info!` が出る、⑷ SSP が 9801 で動いている机で areka が 9821 で同時に待ち受ける。
6. The areka shall 実機確認の走行で `RUST_LOG` に要件 3.14 の `debug!` が出る階層まで開け、拒んだ要求（4.3）の `warn!` が記録に残ることを確かめる。

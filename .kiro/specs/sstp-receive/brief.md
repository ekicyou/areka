# Brief: sstp-receive

> 起票: 2026-10-23（棚卸㉙・覚え書きの「SSTP の受信」から格上げ）。
> 版の段は「いつか」・区分 C（正典の受け口が無いだけで、今ある働きは壊れていない）。

## Problem

- **利用者・ほかのアプリ**: SSTP を送る道具（SSTP の送り手・ブラウザの jsstp・ほかのゴーストのツール）から areka のゴーストに台詞を出させたり、イベントを送ったりできない。SSP ならできる。
- **作者**: SSTP で届いた台詞の印 `sstpmarker` と送り主の表示 `sstpmessage`（`balloon-markers`）が、受け口が無いので画面に出ない。`Option: nobreak`・`SenderType: sstp`・`OnSSTPBreak` など SSTP を前提にした決まりが確かめられない。

## Current State

- **正典（ukadoc・2026-10-23 に MCP で引いた）**:
  - 「SSTP/1.x - request」: 例 `SEND SSTP/1.0`・`Charset`・`Sender`・`Script`・`Option`。共通のヘッダ: `Charset`・`Sender`・`SecurityLevel`（`local`／`external`。「DirectSSTP、または127.0.0.1等明らかにローカルから来たリクエストのみ有効。SSTP over HTTPは、localhostから配信されたhtml等から送られた場合を除き、常にexternalとみなされる」）・`SecurityOrigin`（2.6.59）・`Option`（`nodescript`＝バルーンの SSTP の表示を消す・`notranslate`・`nobreak`＝「現在実行中のスクリプトを中断せず、終わるまで待つ」・`strict`〔2.8.94〕）・`ID`（Owned SSTP）・`X-SSTP-PassThru-*`・`HWnd`（DirectSSTP だけ）・`ReceiverGhostHWnd`／`ReceiverGhostName`（Socket の SSTP だけ・見つからなければ「404 Not Found」）。
  - 「SSTP/1.x - response」: 例 `SSTP/1.4 200 OK`・`Charset`（省いたときは要求と同じ文字コードとみなす）・`X-SSTP-PassThru-*`。
  - メソッドごとの節（SEND・NOTIFY・COMMUNICATE・EXECUTE・GIVE・INSTALL）と、待ち受けの番号の節は、ukadoc の MCP の写しには入っていない（引けるのは上の 2 節だけ）＝要件の段で ukadoc の原文（`spec_sstp.html`）を読み直す。
  - SSP ヘルプ「その他の機能」: SSP の MCP は「127.0.0.1:9801/api/mcp/help」＝SSP は SSTP と MCP を同じ番号で待ち受ける。
  - descript の「sstp.allowunspecifiedsend,数値」（0 でゴーストを名指ししない SSTP を受けない）・「sstp.allowcommunicate,数値」（0 で COMMUNICATE を受けない）・「sstp.alwaystranslate,数値」。イベント「OnSSTPBreak」（SSTP が中断されたとき・Reference0〜2）・「OnSSTPBlacklisting」。SHIORI の要求の `SenderType` に `sstp`。
- **areka**:
  - SSTP の SEND と NOTIFY の「中身」の口は kanade に既にある: 依頼 `KanadeMsg::SstpSend`・`SstpNotify`（`crates/areka-kanade/src/msg.rs` 269・277 行目）、受け付けの判定と腕（`crates/areka-kanade/src/schedule/sstp.rs` 150 行の `accepts`・`on_send`・`on_notify`）、結果の型と検査（`crates/areka-kanade/src/sstp.rs` 53 行）、返事（`crates/areka-kanade/src/actor_sstp.rs` 107 行）。使うのは MCP の `sakurascript`・`raise_event`（`crates/areka/src/mcp/`）だけで、送り手の名前は `areka` と `MCP` の 2 つ（`crates/areka-kanade/src/request_origin.rs` 12 行目）。SEND は必ず割り込む（完了 `mcp-kanade-tools` の裁定）。
  - 待ち受けは MCP の 1 つだけ: `crates/areka-mcp/src/server.rs`（207 行）の `accept_loop`（164 行目）が、受けた接続を全部 HTTP/1.1（hyper）へ渡す。`SEND SSTP/1.4` で始まる要求は HTTP として読めずに切れる。番号は `crates/areka-mcp/src/port.rs` の `DEFAULT_PORTS = [9801, 9821]`（16 行目・SSP と早い者勝ち・両方塞がりで隣の番号）。起動の引数 `--sstp-listen` は MCP の待ち受けの番号になる（`crates/areka/src/cli/listen.rs`・127.0.0.1 だけ）。
  - SSTP の要求の読み書き・DirectSSTP（窓への `WM_COPYDATA`）・ゴーストの一覧を外に見せる共有の領域（FMO）は、`crates/areka/` のどこにも無い。
  - `balloon-markers` は、台詞の出どころの印を「いつも SSTP でない」で流す縮退の口を置く予定（同 brief の Approach）。
  - 網羅台帳 `doc/ukadoc-coverage/ledger/shiori.toml` の `spec_sstp` の 2 行（16313・16352 行目）は absent・持ち主なし。`assets.toml` の `sstp.allow*` の 3 行（3633・3649・3665 行目）。

## Desired Outcome

- Socket の SSTP（127.0.0.1 の番号）で `SEND` と `NOTIFY` を受け、kanade の今の口へ渡し、正典の形で返事を返す。
- SSTP で始まった台詞は「SSTP で届いた」の印と送り主の名前を持ち、`balloon-markers` の縮退の口が本物の値を受ける。
- `Option`（`notranslate`・`nobreak`・`strict`・`nodescript`）・`SecurityLevel`・`SenderType: sstp`・`X-SSTP-PassThru-*`・`ReceiverGhostName`・descript の `sstp.allowunspecifiedsend` が正典どおりに効く。
- MCP と SSTP が同じ番号で両方受けられる（SSP と同じ形）。areka が 9821 などへ逃げたときも、SSTP と MCP は同じ番号にそろう。
- 決定論のテストで、要求の読み（文字コード・ヘッダ・壊れた要求）・返事の組み立て・番号の共有の振り分けを固定する。実機で SSTP の送り手から台詞を出して確かめる。

## Approach

- 待ち受けは 1 つのまま、接続の最初の行で振り分ける（`<メソッド> SSTP/1.x` なら SSTP の読み手へ、ほかは今の HTTP へ）。番号の決まり（早い者勝ち・`--sstp-listen`）は今の MCP のものを使う。
- SSTP の読み手は文字コードの判定（`Charset`）とヘッダの読みだけを持つ純粋な関数にし、中身は kanade の `SstpSend`・`SstpNotify` へ渡す。送り手の名前の種類に SSTP を足す。
- `nobreak` の「終わるまで待つ」は、`shiori4-api` が作る待ちの列（同 brief の「待ちの列を作る」）に乗る。列が先に無ければ、本 spec で作るか待つかを要件の段で決める。

## Scope

- **In**: Socket の SSTP の受け口（番号の共有）・SSTP/1.x の要求の読みと返事の組み立て・`SEND`・`NOTIFY`・上の `Option` と共通のヘッダ・`SenderType: sstp`・`OnSSTPBreak`・`sstp.allowunspecifiedsend`・`ReceiverGhostName`（今は 1 体なので「名前が違えば 404」）・`balloon-markers` の縮退の口へ本物の値を渡すこと・網羅台帳の行・決定論のテスト・実機の確かめ。
- **Out**（分けて起票する候補・下の測りの分割の案）: `COMMUNICATE`（`sstp.allowcommunicate`・`OnCommunicate`）・`EXECUTE`・`GIVE`・`INSTALL`・SSTP over HTTP・DirectSSTP と FMO・Owned SSTP の `ID`（SHIORI の `uniqueid` が要る）・`OnSSTPBlacklisting`・127.0.0.1 の外からの SSTP（受けない）。

## Boundary Candidates

- 待ち受けと最初の行の振り分け（`crates/areka-mcp/src/server.rs` の受け口）
- SSTP/1.x の読み手と返事の組み立て（新しいファイル）
- kanade の SSTP の口と送り手の名前（`crates/areka-kanade/src/{sstp.rs, schedule/sstp.rs, request_origin.rs, msg.rs}`）
- 台詞の出どころの印を `balloon-markers` の口へ渡す所

## Out of Boundary

- MCP の道具の答え方（`mcp-strict-errors` ほか）は変えない。MCP の `sakurascript` が必ず割り込む決まりも変えない。

## Upstream / Downstream

- **Upstream**: 完了 `mcp-kanade-tools`（kanade の SSTP の口）・完了 `script-security-level`（`SecurityLevel` を運ぶ）。`mcp-shiori-query`（`X-SSTP-PassThru-*` を SHIORI へ運ぶ仕組み・同 brief が「将来の SSTP の受信も…この口を使う」と書く）。`shiori4-api`（`nobreak` の待ちの列）。
- **Downstream**: `balloon-markers`（`sstpmarker`／`sstpmessage` が画面に出る）・`makoto-dll-host`（`sstp.alwaystranslate`）・`startup-install-option`（同 brief の Out「起動中の areka へインストールを任せる仕組み（SSTP の受け口…）」）・`multi-ghost-runtime`（名指しの無い SSTP をどのゴーストへ渡すか）。

## Existing Spec Touchpoints

- **Extends**: 完了 `mcp-kanade-tools` の kanade の口に、出どころを 1 つ足す。
- **Adjacent**: `mcp-strict-errors`（kanade の `schedule/sstp.rs`・`msg.rs`）・`mouse-click-wheel-events`（`schedule/sstp.rs`・`msg.rs`）・`shiori2-protocol`（`schedule/sstp.rs`・`msg.rs`）・`shiori4-api`（同じ口と待ちの列）・`balloon-markers`。

## Constraints

- 待ち受けは 127.0.0.1 だけ（今の MCP と同じ）。番号の決まり（9801→9821 の早い者勝ち・明示の番号は逃げない）は変えない。
- tokio は MCP の専用スレッドに閉じたまま。プロトコルの読み書きは純粋な関数にして決定論のテストで固める。
- 失敗の道には必ず記録を残す（黙って捨てない）。
- 1 ファイル 1,000 行。

## 2026-10-23 棚卸㉙の測り

- **触るファイル**: `crates/areka-mcp/src/server.rs`（207）と新しい SSTP の読み手のファイル・`crates/areka-kanade/src/{sstp.rs 53, schedule/sstp.rs 150, request_origin.rs 114, msg.rs 544, actor_sstp.rs 107}` と兄弟のテスト・`OnSSTPBreak` を送るなら `crates/areka-kanade/src/schedule/events.rs`（888）・台詞の出どころの印を運ぶ所（`balloon-markers` の口・emo-text の側は同 spec の物）・`crates/areka/src/cli/listen.rs`（148・名前と説明の文）・台帳 `shiori.toml`・`assets.toml` の数行。
- **規模**: L。全部を 1 本にすると 24〜30 タスク＝20 を超える。**分割の案**: ① 本 spec＝受け口・番号の共有・SSTP/1.x の読み書き・`SEND`・`NOTIFY`・`Option`・共通のヘッダ・`OnSSTPBreak`・`balloon-markers` への受け渡し（14〜17）。② 仮の名前 `sstp-receive-methods`＝`COMMUNICATE`・`EXECUTE`・`GIVE`・`INSTALL`・SSTP over HTTP（10〜13・① の後）。DirectSSTP と FMO は、ほかのアプリがゴーストを見つけるための共有の領域を作る別の仕事なので、さらに別に起票するかを議題にする。
- **先に要るもの**: 働きの上の前提は満たした（`mcp-kanade-tools`・`script-security-level` は完了）。`X-SSTP-PassThru-*` を SHIORI へ運ぶのは `mcp-shiori-query` の後（先に着手するなら、運ぶ所だけ本 spec で作るかを要件の段で決める）。`nobreak` は `shiori4-api` の待ちの列の後が無駄が無い。同じウェーブに置けない相手（v0.1.0）: `mcp-strict-errors`・`mouse-click-wheel-events`・`shiori2-protocol`（kanade の `schedule/sstp.rs`・`msg.rs`）・`shell-scale-linkage`（`msg.rs`・`events.rs`）・`balloon-communicatebox`（`events.rs`）。`consumer_ledger.rs` の列・emo-text・`input_events/`・パーサの `balloon/` には触らない。実機の確かめは SSTP の送り手を AI が回す（SSP を落とさなくてよい番号で）。
- **優先度の区分**: C。
- **要件定義のモデル**: Opus。ただし `shiori4-api` の待ちの列より先に着手し、`nobreak` の列を本 spec で作ることになったら、新しい順序の設計なので Fable。
- **議題の候補**: ⑴ 番号の共有の形（同じ待ち受けで最初の行を見て振り分ける〔推奨・SSP と同じ〕／SSTP に別の番号）。⑵ `nobreak` を待ちの列が無いうちにどうするか（列を作る／`shiori4-api` を待つ）。⑶ 名指しの無い SSTP の宛先（今は 1 体＝そのゴースト。`multi-ghost-runtime` の後で決め直す）。⑷ DirectSSTP と FMO を別に起票するか。⑸ SSTP で届いた台詞が流れている最中のクリックで `OnSSTPBreak` を送るか（今の中断の決まりとの合わせ方）。

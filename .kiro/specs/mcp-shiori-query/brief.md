# Brief: mcp-shiori-query

> 起票: 2026-10-05 `/kiro-discovery`（同日の「MCP の areka 独自ツールと影響の段」の続き）。開発者「今の MCP ツールって、SHIORI への情報要求が無いですよね。ライズイベントはトークになるだけで MCP 応答にはならないですし」。同日、正典の具体例（ghost_terminal の `ShioriEcho` 系）を確かめたうえで、開発者が「正典寄りの形」に決めた。続けて開発者「`X-SSTP-PassThru-*` ではなく `X-MCP-PassThru-*` の方が良い。プロトコル的には `X-` で始まるヘッダは任意に命名可能だったはず」＝MCP の経路では名前を `X-MCP-PassThru-*` にする。

## Problem

エージェントがゴースト（SHIORI）に「情報」を問い、答えをデータとして受け取る道が無い。

- `raise_event`（SSP 互換）は SSTP NOTIFY 相当。返った台本を再生し、結果の本文にも台本を入れる（SSP の定義「The returned script is included in the result」）。けれど、台本は喋るためのもので、データの口ではない。引数も SSP の定義で `event`・`references`・`ghost_name`・`strict` に固定されている。
- `get_property` はベースウェアのプロパティを読むもので、SHIORI の中には届かない。
- `\![embed]`・`\![get,property]` は、SHIORI の答えを台本の中へ戻すだけ。エージェントへは戻らない。

## 正典の作法（外とゴーストのデータの往復）

ukadoc の SHIORI/3.0 は、外のアプリとゴーストがデータを往復させる口として `X-SSTP-PassThru-*` を定めている。

- **要求側**（SSP 2.5.05〜）: SSTP の要求ヘッダのうち `X-SSTP-PassThru-` で始まるものが、そのまま SHIORI へ通知される。SSTP 以外のときは付かない。
- **応答側**（SSP 2.5.03〜）: SHIORI が応答に付けた `X-SSTP-PassThru-*` は、SSTP の応答にそのまま足される。複数返すときはヘッダ名を変える。旧名 `X-SSTP-Return-` は廃止予定。
- **SSTP の応答**: SSTP の応答の仕様には「NOTIFYなどゴースト側との通信の際に、SHIORIから直接返されてきたヘッダ」とあり、名前と中身はそのまま中継される。

実際の使い手は ghost_terminal（ukadoc の外部アプリの拡張イベント `ShioriEcho` 系）。

- `ShioriEcho.GetResult` では、ゴーストは台本（Value）を返さず、`X-SSTP-PassThru-Result`（と `-Type`）で結果を返す。
- `ShioriEcho.TabPress` では、Reference で入力中のコマンドとカーソルの位置を受け、`X-SSTP-PassThru-Command`・`-InsertIndex` で補完の結果を返す。

**データはヘッダで返し、台本を空にすれば何も喋らない**。これが正典の作法で、再生を止める特別な決まりは要らない。

外のアプリが SHIORI リソース（`GET` の ID＝`username` など・Value がデータ）を直接問う正典の道は無い（SSTP にも SSP の MCP にも無い）。

## ヘッダの名前（開発者裁定・2026-10-05）

MCP の経路では `X-SSTP-PassThru-*` ではなく **`X-MCP-PassThru-*`** を使う。作法（要求のヘッダを SHIORI へそのまま通し、SHIORI の応答のヘッダをそのまま返す・データはヘッダで台本は空なら喋らない）は `X-SSTP-PassThru-*` を写し、名前の頭だけを経路に合わせる。

- 正典の `X-SSTP-PassThru-*` は「SSTPからイベント通知が来た場合に限り」付き、応答のものは「SSTP以外の時は無視される」。MCP は SSTP ではないので、同じ名前を使うと正典の定義からはみ出す。
- 名前で経路が分かる。ゴーストは、ghost_terminal（`X-SSTP-PassThru-*`）向けの答えとエージェント向けの答えを書き分けられる。
- `X-` で始まるヘッダを拡張に使うのは伺かの慣わし（`X-SSTP-PassThru-*` 自体が SSP の拡張）。ただし、ukadoc のスナップショットには SHIORI/3.0 で「`X-` は任意に命名してよい」と書いた原文が見つからなかった。要件の段で原文を当たり、無ければ「慣わし」として記録する。
- MCP の経路では、ゴーストの応答の `X-SSTP-PassThru-*` は返さない（正典の「SSTP以外の時は無視される」に揃える）。将来 areka が SSTP を受けるときは、同じ運搬の仕組みで `X-SSTP-PassThru-*` を扱う＝頭の文字列を経路ごとに替えるだけの作りにする。
- 各 SHIORI（YAYA・里々・蒼空・Pasta など）が、任意の名前の要求ヘッダを読めるか、任意の名前の応答ヘッダを返せるかは、要件の段で確かめる（`X-SSTP-PassThru-*` に答えられるゴーストなら同じ仕組みで答えられる見込み）。

## Current State

- `X-SSTP-PassThru-*` は、要求側も応答側も扱っていない。SHIORI への要求には付けず、応答のものは読み飛ばしている（`crates/shiori-host32-host/src/shiori3.rs` の注記・網羅台帳 `doc/ukadoc-coverage/ledger/shiori.toml` の群 11 で `absent`）。
- kanade には、汎用の通知の入口（`KanadeMsg::RaiseEvent`・許可表 `ALLOWED_EVENT_IDS`）と、SHIORI リソースの照会（`KanadeMsg::ResourceQuery`・許可表に固定）がある。
- `raise_event` の中身は `mcp-kanade-tools` が持つ（許可表の迂回・返った台本を結果に含める）。

## Desired Outcome

areka 独自のツール 1 本（仮名 `query_shiori`）。**SSTP NOTIFY と同じ振る舞いに、`X-MCP-PassThru-*` の往復を足したもの**（作法は `X-SSTP-PassThru-*` を写す）。

- 入力は次の 4 つ。
  - イベント ID
  - Reference 列
  - 要求の `X-MCP-PassThru-*`（名前の後半と値の組）
  - `ghost_name`
- ゴーストへの要求には、渡された組を `X-MCP-PassThru-（名前の後半）` として、中身を変えずに付ける。
- ゴーストが台本（Value）を返したら、NOTIFY と同じく再生する。空なら何も喋らない（正典どおり・areka 独自の「再生しない」は持たない）。
- 結果には次の 3 つを入れる。
  - 応答の状態（200・204・失敗）
  - 応答の `X-MCP-PassThru-*` の全部（名前も中身もそのまま）
  - 返った台本（`raise_event` と揃える）
- 結果の形（行の並び・JSON か平文か）は要件で決める。エージェントが読みやすく、ヘッダの名前と値の区切りが曖昧にならない形にする。

## Approach

- kanade の汎用の通知の入口を、パススルーのヘッダ（頭の文字列は経路ごと）を運べる形へ広げる。許可表の扱いは `mcp-kanade-tools` の `raise_event` と揃え、一度で設計する。
- SHIORI のプロトコル層で、要求への付加と応答からの取り出しを持つ。経路は host32-host の `build_request` と応答の読み取り、in-proc の経路の両方。将来の SSTP の受信も、頭を `X-SSTP-PassThru-` に替えてこの口を使う。
- SHIORI リソースを外から直接読む形は正典に無い。採るかどうかは要件の議題とし、起票時の推しは「採らない」。ゴーストが答えたい情報は、ゴースト側で決めたイベントに `X-MCP-PassThru-*` で答える作法で足りる。
- 出どころは MCP（`script-security-level` が着地していれば `SenderType` などに反映）。影響の段は「低」（ゴーストに聞くだけ）。

## Scope

- **In**:
  - ツール 1 本。
  - `X-MCP-PassThru-*` の要求への付加と応答からの取り出し（運搬の仕組みは頭の文字列を差し替えられる作り）。
  - kanade の通知の入口の拡張。
  - 網羅台帳の該当行（要求側・応答側の `X-SSTP-PassThru-*` の行に「運搬の仕組みはある・SSTP の受信は無い」を書く）。
  - help・サーバーの指示文の案内。
- **Out**:
  - `raise_event` の定義と振る舞い（SSP 互換・`mcp-kanade-tools`）。
  - SSTP の受信。
  - `\![embed]`（`sakura-embed-directive`）。
  - `\![get,property]` などのプロパティの照会経路（`property-query-channels`）。
  - ghost_terminal の `ShioriEcho` 系イベントを areka が送ること。

## Boundary Candidates

- パススルーのヘッダの運搬（SHIORI のプロトコル層・頭の文字列は経路ごと）。
- 通知の入口の拡張（kanade）。
- ツールの定義と結果の形（`mcp-author-tools` の登録口へ足す）。

## Out of Boundary

- ゴースト側でどのイベントにどんなヘッダで答えるかの約束（ゴースト作者・エージェント向けの文書の側）。

## Upstream / Downstream

- **Upstream**: `mcp-author-tools`（独自ツールの登録口）。
- **Downstream**:
  - 将来の SSTP の受信（運搬の仕組みを共有し、頭を `X-SSTP-PassThru-` にする）。
  - エージェント向けのスキル・文書。

## Existing Spec Touchpoints

- **Extends**: なし。
- **Adjacent**:
  - `mcp-kanade-tools`（`raise_event`・通知の入口と許可表＝kanade の列）。
  - `script-security-level`（`shiori3.rs` の `build_request` を同じく触る＝同じウェーブに置かない）。
  - `property-query-channels`・`sakura-embed-directive`（SHIORI への問い合わせの別経路）。

## Constraints

- 正典の出典:
  - https://ssp.shillest.net/ukadoc/manual/spec_shiori3.html （要求側・応答側の `X-SSTP-PassThru-*`・メソッド GET／NOTIFY）
  - https://ssp.shillest.net/ukadoc/manual/spec_sstp.html （SSTP の応答の `X-SSTP-PassThru-*` の中継）
  - https://ssp.shillest.net/ukadoc/manual/list_shiori_event_ex.html （`ShioriEcho` 系＝ghost_terminal の実例）
- SSTP NOTIFY が SHIORI へ `GET` で届くこと（返った台本を再生するため）は推定。要件の段で SSP の実測か ukadoc の原文で確かめる。
- MCP の返事の待ちは最長 10 秒。SHIORI が遅いときは `mcp::later` に預ける。
- 規模の見立て: M（10〜14 タスク）。

## 2026-11-01 棚卸㉝の再測定（main 13b80ab1）
- **前提の変化**: 働きの上の 2 本は着地した（✅ `mcp-kanade-tools` 10-19・✅ `script-security-level` 10-23）＝先頭（kanade の列でも host32-host の列でも次）。
  - 外から任意の名前のイベントを GET で送る口ができた＝`KanadeMsg::SstpNotify`（`crates/areka-kanade/src/msg.rs` 308 行目）→ 運行表の腕 `schedule/sstp.rs`（149）→ 呼び出しの組み立て `schedule/events.rs` の `sstp_notify`（828 行目）→ 結果 `SstpNotifyOutcome`（`crates/areka-kanade/src/sstp.rs`・`Script`／`NoScript`／`NotAccepted`／`Failed`）。MCP の `raise_event`（`crates/areka/src/mcp/raise_event.rs`）はこれを使い、返事を `later` に預ける。本文 Constraints の「SSTP NOTIFY が SHIORI へ GET で届くかは推定」は、この口が GET で送る形で決着した。
  - 要求の追加のヘッダの入れ物ができた＝`crates/shiori-host32-host/src/request_headers.rs`（70・`RequestHeaders`・説明に「後続の spec は欄を足す（例 `X-MCP-PassThru-*`）」）と、呼び出しの口 `ShioriBackend` の既定の実装つきのメソッド `get_with_headers`・`notify_with_headers`（`crates/areka-kanade/src/shiori/real.rs` 92・105 行目）。応答の側は今もヘッダを運ばない（`shiori3.rs` の `ParsedResponse`・kanade の `ShioriOutcome`・`SstpNotifyOutcome` に欄が無い）＝応答から `X-MCP-PassThru-*` を取り出して運ぶのはこの spec の仕事。
  - ✅ `shell-scale-linkage`（11-01）は `msg.rs` に `PropertyEvent` と `EventId::Property` を足しただけ（11 行）で、この spec の道には触れていない。`msg.rs` 586・`actor.rs` 852（棚卸㉓・㉔の 930／901 は古い）。
- **触るファイル**（行数）: 新規＝`crates/areka-mcp/src/tools/<ツール>.rs`・`crates/areka/src/mcp/<ツール>.rs` と兄弟のテスト／手順の表（`doc/ssp-mcp/areka-tools.md` 306 の ⑹）のとおり `crates/areka-mcp/src/tools/mod.rs`（241）・`handler.rs`（178）・`tools/tools_own_tests.rs`（55）・`tools_own_socket_tests.rs`（272）・`crates/areka/src/mcp/mod.rs`（152）／運ぶ道＝`request_headers.rs`（70）・`shiori3.rs`（879）・`client.rs`（**954＝上限の近く・足すなら先に分ける**）・`crates/areka-ghost/src/shiori_inproc.rs`（660）・kanade の `msg.rs`（586）・`actor.rs`（852）・`shiori/real.rs`（555）・`sstp.rs`（53）・`schedule/sstp.rs`（149）・`schedule/types.rs`・`schedule/events.rs`（943・`sstp_notify` に欄を足すなら）／台帳 `shiori.toml` の `X-SSTP-PassThru` の 2 行。依存の追加・emo-text は無い。
- **規模**: 10〜14 タスク（入れ物ができた分、棚卸㉓の見立ての下の幅・`client.rs` を先に分ける 1 つを含む）。
- **先に要るもの**: 無し。同じウェーブに置けない相手: MCP の独自のツールを足す `mcp-user-response`（手順の表のファイル）・`areka-tools.md` を触る `script-impact-tiers`・kanade の `msg.rs` を触る spec（`shell-tooltip` ほか）・`schedule/events.rs` を触る spec（`balloon-communicatebox`・`inputbox-user-input`）。
- **`balloon-communicatebox` との重なり**: **`schedule/events.rs` で重なる見込み**（要求に欄を足すと、`ShioriCall::Get` を組む `sstp_notify` に手が入る）。避けるなら `ShioriCall` の今の形を変えず、パススルーつきの呼び出しを `schedule/sstp.rs` の中で組む形を設計で取る（`events.rs` は 943 行で、どのみち足す余地が少ない）。
- **人の手・机**: 実機の確かめは `X-MCP-PassThru-*` に答える検体のゴーストが要る（emo2 の写しの台本で作れる見込み）。人の待ち・静かな机・前後の全体テストの比べは要らない。各 SHIORI（YAYA・里々・蒼空・Pasta）が任意の名前のヘッダを読み書きできるかは要件の段の調べ（待ちではない）。
- **優先度の区分**: A のまま（開発者「今の MCP ツールって、SHIORI への情報要求が無いですよね」・ヘッダの名前も開発者の裁定）。
- **要件定義のモデル**: **Opus に改める**。要求の側は入れ物の先例（`request_headers.rs`）に欄を足すだけ、応答の側も同じ形を写す。並行や時刻の新しい約束も無い（返事は `later` の今の道）。`X-` の原文の確認と各 SHIORI の読み書きは調べ物で、決まりのどれにも当たらない。
- **分割の案**: 無し。
- **見つけた穴・古くなった記述**: 棚卸㉒の「実装 19 か所」・棚卸㉓の「20 か所」は古い（今は `ShioriBackend for` が 28 か所＝既定の実装つきのメソッドを足す形なら波及しない）。roadmap の台帳の行（398 行目）の「`script-security-level` と同じウェーブに置かない」は古い（着地済み）。

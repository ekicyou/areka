# Brief: areka-P0-mcp-shiori-query

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

## 2026-10-05 棚卸㉒の再測定（main `f26aa1c1`・C3 の着地の後）

- 規模（タスク数）と切るかどうか: M（10〜14）。ただし下の「SHIORI の呼び出しの口の形」を変えると影響の範囲が大きい＝M〜L（12〜16）に上振れの見込み。切らない。
- 前提の状態: **待ち**。`mcp-author-tools`（独自ツールの登録口）が未着手。C3 は本 spec の触るファイル（`crates/shiori-host32-host/`・`crates/areka-kanade/src/shiori/`）を変えていない（`shiori_inproc.rs` は 4 行だけ・別の所）。
- 崩れた前提／古くなった位置:
  - SHIORI への要求の組み立ては `crates/shiori-host32-host/src/shiori3.rs` の `build_request` が `SecurityLevel: local` を直に書き、`SenderType`・`SecurityOrigin`・`X-SSTP-PassThru` は送らない（同関数の説明）。応答は同ファイルの `parse_response` が読む。
  - **運ぶ道が長い**: kanade の `ShioriCall`（`msg.rs` の列挙の定義・`Get`／`Notify` は `id`・`references`・`status` だけ）→ `src/shiori/real.rs` の `handle_call` → 呼び出しの口 `ShioriBackend`（同ファイルのトレイトの定義・`get`／`notify` は `id`・`references`・`status` だけ）→ `areka-ghost/src/shiori_inproc.rs` と `shiori-host32-host/src/client.rs` → `build_request`。応答の側は `ShioriOutcome`（`Value`／`NoContent`／`Notified`／`Failed`）がヘッダを運ばない。**`ShioriBackend` を実装している型は 19 か所**（本物 2・テストの偽物 17）＝`get`／`notify` の引数を変えると全部に波及する。既定の実装つきの新しいメソッドを足す形なら波及を止められる。
  - 同じ道（要求に欄を足す）を `script-security-level`（`SecurityLevel`・`SenderType`）と `property-query-channels`（`SenderType: property`）も通る＝**3 本が同じ口を広げる**。入れ物（要求に添える追加のヘッダ）は先に着手した 1 本が作り、残りはそれに値を足すだけにする。
- 触るファイル（並走の照合用・見込み）: `crates/areka-kanade/src/{msg.rs, actor.rs, shiori/real.rs}`・`crates/shiori-host32-host/src/{shiori3.rs, client.rs}`・`crates/areka-ghost/src/shiori_inproc.rs`・（引数を変えるなら）`ShioriBackend` の実装 19 か所・新規のツールのファイル（`mcp-author-tools` の登録口）・台帳 `shiori.toml` の `X-SSTP-PassThru` の 2 行。
- 議題（答えで作業が変わるものだけ）: 要求に添える追加のヘッダの入れ物を、`script-security-level`・`property-query-channels` のどれが先に作るか（＝3 本の順）。
- 見つけた穴: なし。すぐ直せる軽微な修正: この brief の末尾の前に道具の残りかす `</content>`・`</invoke>` の 2 行が紛れている（消してよい）。

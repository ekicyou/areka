# Brief: areka-P0-mcp-shiori-query

> 起票: 2026-10-05 `/kiro-discovery`（同日の「MCP の areka 独自ツールと影響の段」の続き）。開発者「今の MCP ツールって、SHIORI への情報要求が無いですよね。ライズイベントはトークになるだけで MCP 応答にはならないですし」。

## Problem

エージェントがゴースト（SHIORI）に「情報」を問う道が無い。

- **`raise_event`（SSP 互換）では答えにならない**: 返った台本を再生する。SSP の定義は「The returned script is included in the result」で、結果の本文に台本は入るが、再生されてしまう。しかも中身は喋るための台本で、データではない。
- **`get_property` では届かない**: ベースウェアのプロパティを読むもので、SHIORI の中には届かない。
- **さくらスクリプトでも届かない**: `\![embed]`・`\![get,property]` は SHIORI の答えを台本の中へ戻すだけで、エージェントへは戻らない。

伺かの正典には、外から SHIORI と情報をやり取りする作法がある。

- **SHIORI リソース**: ベースウェアが `GET` で ID（`username`・`homeurl`・`sakura.recommendsites`・`getaistate` など）を問い、SHIORI は Value に**データ**を返す。
- **`X-SSTP-PassThru-*`**:
  - 要求側: SSTP の要求に付けた `X-SSTP-PassThru-*` は、名前も中身もそのまま SHIORI へ渡る。
  - 応答側: SHIORI が応答に付けた `X-SSTP-PassThru-*` は、SSTP の応答へそのまま中継される（「NOTIFYなどゴースト側との通信の際に、SHIORIから直接返されてきたヘッダ」）。外のアプリとゴーストがデータを往復させる、正典の口。
- **メソッドの別**: `GET` は Value で何かを返す前提、`NOTIFY` は値を返さない前提。

AI と一緒にゴーストを作る作者にも、ゴーストと協働するエージェントにも、「ゴーストに聞く」道は要る。例えば次のような使い方がある。

- ゴーストの記憶（利用者の呼び名・好感度）を読む。
- ゴースト側で決めた独自の問い合わせに答えてもらう。

## Current State

- kanade に `KanadeMsg::ResourceQuery`（`crates/areka-kanade/src/msg.rs`・`actor_resources` が殻でその場で答える）がある。照会できる ID は**許可表に固定**（`&'static str`）で、許可外は `Failed`。会話できない状態では全件 `NoContent`。
- `X-SSTP-PassThru-*` は、要求側も応答側も扱っていない（`crates/shiori-host32-host/src/shiori3.rs` の注記・網羅台帳 `shiori.toml` の群 11 で `absent`）。
- `raise_event` は `mcp-kanade-tools` が持つ（許可表 `ALLOWED_EVENT_IDS` の迂回と、返った台本を結果に含めることが範囲）。

## Desired Outcome

areka 独自のツール（仮名 `query_shiori`）で、次のことができる。

- **SHIORI リソースを読む**: ID を 1 つ以上渡すと、Value を**データとして**返す。応答の状態（200・204・失敗）も返す。許可表に縛られない（ukadoc の SHIORI リソースの一覧に無い ID も、ゴースト独自の問い合わせとして通す）。
- **イベントで問う**: イベント ID・Reference 列・要求の `X-SSTP-PassThru-*` を渡す。SHIORI の応答の Value（台本）と応答の `X-SSTP-PassThru-*` を返す。返った台本を再生するかどうかは要件で決める（起票時の推しは、既定では再生しない＝情報の問い合わせに徹する。再生したいなら `raise_event` がある）。
- 応答の `X-SSTP-PassThru-*` は、名前も中身も変えずに返す。

## Approach

- 照会の経路は `ResourceQuery` の型を広げるか、並べて新しい要求の型を置く（設計で決める）。会話できない状態の扱い（`NoContent` で返すか待つか）も設計で決める。
- `X-SSTP-PassThru-*` を SHIORI の要求と応答で運ぶのは、host32-host の `build_request` と応答の読み取り、in-proc の経路の両方。将来の SSTP の受信も同じ口を使う。
- 出どころは MCP（`script-security-level` が着地していれば `SenderType` などに反映）。影響の段は「低」（ゴーストに聞くだけ）。

## Scope

- **In**:
  - ツール 1 本（リソースとイベントの 2 つの形）。
  - SHIORI リソースの任意 ID の照会。
  - `X-SSTP-PassThru-*` の要求と応答の運搬。
  - 網羅台帳の該当行。
- **Out**:
  - `raise_event` の振る舞い（SSP 互換・`mcp-kanade-tools`）。
  - SSTP の受信。
  - `\![embed]`（`sakura-embed-directive`）。
  - `\![get,property]` などのプロパティの照会経路（`property-query-channels`）。

## Boundary Candidates

- 照会の要求の型と kanade の殻の応答（`actor_resources` の隣）。
- `X-SSTP-PassThru-*` のヘッダの運搬（SHIORI のプロトコル層）。
- ツールの定義と結果の形（`mcp-author-tools` の登録口へ足す）。

## Out of Boundary

- ゴースト側でどんな ID に答えるかの約束（ゴースト作者・エージェント向けの文書の側）。

## Upstream / Downstream

- **Upstream**: `mcp-author-tools`（独自ツールの登録口）。
- **Downstream**:
  - 将来の SSTP の受信（`X-SSTP-PassThru-*` の口を共有）。
  - エージェント向けのスキル・文書。

## Existing Spec Touchpoints

- **Extends**: なし。
- **Adjacent**:
  - `mcp-kanade-tools`（`raise_event`・kanade の列）。
  - `script-security-level`（`shiori3.rs` の `build_request` を同じく触る＝同じウェーブに置かない）。
  - `property-query-channels`・`sakura-embed-directive`（SHIORI への問い合わせの別経路）。

## Constraints

- 正典の出典:
  - https://ssp.shillest.net/ukadoc/manual/spec_shiori3.html （メソッド GET／NOTIFY・`X-SSTP-PassThru-*` の要求側と応答側）
  - https://ssp.shillest.net/ukadoc/manual/spec_sstp.html （応答の `X-SSTP-PassThru-*` の中継）
  - https://ssp.shillest.net/ukadoc/manual/list_shiori_resource.html （SHIORI リソースの一覧）
- MCP の返事の待ちは最長 10 秒。SHIORI が遅いときは `mcp::later` に預ける。
- 規模の見立て: M〜L（12〜18 タスク）。
</content>
</invoke>

# Brief: areka-P0-mcp-user-response

> 起票: 2026-10-05 `/kiro-discovery`（開発者「エージェント目線でこんなツールがあったらいいな、面白いかも」「デスクトップマスコットのすそ野を広げる」）。

## Problem

エージェントは `sakurascript` で、ゴーストに選択肢（`\q`）や入力欄（`\![open,inputbox]` など）を出させることはできる。けれど、**利用者が何を選び、何を入力したかを知る道が無い**。なでる・クリックした、も同じ。選択の結果は SHIORI の `OnChoiceSelect`／`OnUserInput` へ行き、エージェントには戻らない。さくらスクリプトでは届かない穴で、SSP の MCP にも無い（押し出しも購読も無い）。

よその動向（Codex Pets・OpenPets・Vibe Island）は、「見せるだけ」から「その場で答えを返す」へ進んでいる。端末から離れた開発者が、ゴーストの選択肢でエージェントに答えを返せれば、デスクトップマスコットが「エージェントの体」になる。伺かを知らない Claude Code の利用者を呼び込む入口になる。

## Current State

- choice は kanade が扱う（`crates/areka-kanade/src/schedule/choice.rs`）。選ばれた結果は SHIORI へのイベントになるだけ。
- MCP には `get_log`（5 種別・通し番号 `since_id`）がある。利用者の操作は記録していない。
- MCP の返事を待つのは最長 10 秒（`REPLY_WAIT`）。後から答える仕組み `mcp::later` はある（まだ使う側は無い）。

## Desired Outcome

- エージェントが出した選択肢・入力欄の答えと、ゴーストへの操作（なでる・クリック・ダブルクリック・ドラッグなど）を、ツール 1 本（仮名 `wait_user_response`）で受け取れる。
  - `since_id` で続きから読める。
  - 指定した時間まで待てる（上限は 10 秒の壁の中で設計が決める）。
- エージェントが出した選択肢の答えでゴーストを煩わせるか、areka が先に受け取るかの区別がある（要件で決める）。
- 今ある `sakurascript` と組むと、「ゴーストに問わせて、利用者の答えでエージェントが続きを決める」流れが成り立つ。

## Approach

- 利用者の応答を、種別と通し番号付きで短い履歴に積む（`log_history` と同じ型）。ツールはそれを `since_id` で返す。
- 待つ指定があるときは、`mcp::later` に預けて、届くか期限で答える。
- 選択肢の ID の名前空間で、エージェント由来の選択を見分けるかは要件で決める。`\q[タイトル,ID]` の ID は正典では SHIORI への Reference になる。

## Scope

- **In**:
  - 利用者の応答の履歴。
  - 受け取るツール 1 本。
  - エージェント由来の選択の見分け方。
- **Out**:
  - サーバーからの押し出し（Claude Code の channels は研究プレビュー・MCP の購読は後で検討）。
  - 選択肢を出すツール（`sakurascript` で足りる）。
  - 同意の窓（`script-impact-tiers`）。

## Boundary Candidates

- 応答の履歴（kanade の choice・入力欄・マウスのイベントから積む）。
- ツールと待ち（`mcp::later`）。

## Out of Boundary

- ゴーストの台本の書き方（エージェントのスキル・文書の側）。

## Upstream / Downstream

- **Upstream**:
  - `mcp-author-tools`（独自ツールの登録口）
  - `mcp-kanade-tools`（`sakurascript` が実際に再生される）
  - `mouse-drag-events`（ドラッグの出来事）
- **Downstream**: エージェントの状態や約束ごとを配るスキル・文書（覚え書き）。

## Existing Spec Touchpoints

- **Extends**: なし。
- **Adjacent**:
  - kanade の列（`mouse-drag-events`・`mcp-kanade-tools`・`balloon-lifecycle-events`）。choice とマウスのイベントの配線を触るので、同じウェーブに置かない。

## Constraints

- 選択肢の正典: `\q`・`OnChoiceSelect`・`OnChoiceSelectEx`・`\![open,inputbox]`・`OnUserInput`・`OnUserInputCancel`。出典は https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html と https://ssp.shillest.net/ukadoc/manual/list_shiori_event.html 。
- クリック待ちを自動で作らない（開発者裁定「クリック待ちは台本の明示 `\x` だけ」）。このツールは、待つ主体がエージェントの側にあり、ゴーストの再生は止めない。
- 規模の見立て: M〜L（12〜18 タスク）。
</content>
</invoke>

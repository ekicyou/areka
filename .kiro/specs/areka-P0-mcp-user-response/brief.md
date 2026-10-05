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

## 2026-10-05 棚卸㉒の再測定（main `f26aa1c1`・C3 の着地の後）

- 規模（タスク数）と切るかどうか: M〜L（12〜18）。今は切らない。ただし下の 2 つの穴を本 spec で埋めるなら 20 を超えうる＝埋めずに別 spec にする前提で見積もる。
- 前提の状態: **待ち**。`mouse-drag-events`（PR#240）は着地。`mcp-author-tools`（独自ツールの登録口）と `mcp-kanade-tools`（`sakurascript` が実際に再生される）が未着手。
- 崩れた前提／古くなった位置:
  - 「後から答える仕組み `mcp::later` はある（まだ使う側は無い）」は古い＝`dump_surface`・`dump_balloon` が使っている（`crates/areka/src/mcp/dump_surface.rs` の `handle`）。
  - 履歴の手本 `crates/areka/src/log_history.rs`（`get_log` の 5 種別・通し番号）は C3 で着地した。
  - **入力欄は areka に無い**: `\![open,inputbox]`・`OnUserInput`・`OnUserInputCancel` は網羅台帳でどれも `absent`・`owner` が空（`sakura-script.toml`・`shiori.toml`）。コードにも 0 件。持ち主の spec が無い。
  - **単押しのクリックは SHIORI へ送っていない**: kanade の送るマウスの出来事は `OnMouseMove`・`OnMouseDoubleClick`・`OnMouseDragStart`・`OnMouseDragEnd` の 4 つだけ（`schedule/events.rs`）。`OnMouseClick`・`OnMouseClickEx`・`OnMouseWheel` は台帳で `absent`・`owner` が空。
  - 選択の結果は UI 側の `crates/areka/src/input_events/choice_drain.rs` が `KanadeMsg::Choice` で kanade へ送る。ドラッグは `input_events/drag.rs`、移動とダブルクリックは `input_events/mod.rs` から `KanadeMsg::Mouse`。**履歴を UI 側（送る所）で積めば kanade に触らずに済む**＝kanade の進行の列から外せる見込み。
- 触るファイル（並走の照合用・見込み）: 新規の応答の履歴のファイル（`crates/areka/src/` の下）・新規のツールのファイル（`mcp-author-tools` の登録口へ）・`input_events/{choice_drain.rs, mod.rs, drag.rs}`（積む 1 行ずつ）。エージェント由来の選択を kanade で見分けるなら `crates/areka-kanade/src/schedule/choice.rs`（346 行）も。
- 議題（答えで作業が変わるものだけ）:
  - 入力欄（`\![open,inputbox]`）と単押しのクリック（`OnMouseClick`）を本 spec の前に別 spec で作るか、本 spec の範囲を「選択肢・ダブルクリック・なでる（移動）・ドラッグ」に絞るか。
  - 履歴を UI 側で積むか（kanade に触らない）、kanade が SHIORI へ送った出来事で積むか。
- 見つけた穴: 入力欄と単押しのクリックの持ち主が無い（上記）。開発者の決まり「実機で未対応のためにうまくいかない件は範囲外でもすべて起票」に当たる＝`/kiro-discovery` で起票の候補。すぐ直せる軽微な修正: この brief の末尾の前に道具の残りかす `</content>`・`</invoke>` の 2 行が紛れている（消してよい）。

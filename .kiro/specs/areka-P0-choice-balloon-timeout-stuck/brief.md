# Brief: areka-P0-choice-balloon-timeout-stuck

> 起票: 2026-10-05（`areka-P0-balloon-lifecycle-events` の設計で発見。範囲の外の問題として `/kiro-discovery` の決まりで起票）。ソースの読みで確かめた。実機ではまだ確かめていない。

## Problem

選択肢を含む台詞のバルーンが、選んだ後も、選択肢の時間切れで解除された後も、時間切れで消えない。利用者には、用の済んだ選択肢つきのバルーンが次のトークまで出たままに見える。ゴーストの作者には、その台詞について `OnBalloonTimeout` が届かない。

## Current State

2026-10-05 時点（`balloon-lifecycle-events` のブランチ・main `04565148` を取り込んだ後）:

- 時間切れの待ちを止める条件の 1 つ「選択肢が表示中か」は、文字の層の `TextLayerRuntime::choice_active`（`crates/areka-emo-text/src/actor.rs`）が答える。中身は「そのスコープのどこかの場所に選択肢の行が残っているか」。
- 選択肢の行は内容の消去（`\c`・次のトークの冒頭の全消去）でしか消えない。選んだこと・選択肢の時間切れで解除したことでは消えない。
- そのため、選択の応答が台本を返さなかったとき（204）や、選択肢の時間切れの応答が 204 のとき、抑止が効き続ける。可視性の相（`crates/areka/src/emo2_boot/balloon_visibility_phase.rs` が `choice_active` を観測へ写す所）は時間切れを成立させない。
- `balloon-lifecycle-events` は抑止の条件を範囲の外に置いた。実機の確認の検体は選択肢の無い台詞で組む。

## Desired Outcome

選択が済んだ（選んだ・時間切れで解除された・中断された）台詞のバルーンは、選択肢の無い台詞と同じ決まりで時間切れになり、`OnBalloonTimeout` が送られる。選択を待っている間は今までどおり消えない。

## Approach

「選択肢が表示中か」を「行が残っているか」でなく「その選択がまだ生きているか」で答える形にする。候補は 2 つあり、要件・設計で選ぶ。

- 表示の側で、選択が済んだ合図（選んだ・解除された）を受けたら「選択待ち」の印を下ろす（行は残してよい）。
- 選択が済んだら選択肢の行を選べない字へ変える（見た目をどうするかは ukadoc に記述があるかを先に調べる）。

## Scope

- **In**: 選択が済んだ後の抑止の解除・それを固定する決定論のテスト・実機での確認。
- **Out**: 選択肢の時間切れそのもの（完了 `choice-timeout-directive`）・時間切れの判断の他の規則・3 つのイベントの送り方（`balloon-lifecycle-events`）。

## Boundary Candidates

- 文字の層の「選択待ち」の持ち方／可視性の相が読む観測／kanade の選択の帳簿との対応。

## Out of Boundary

- 選択肢の見た目の変更（`choice-marker-styling`）・範囲の選択肢（`range-choice-tag`）。

## Upstream / Downstream

- **Upstream**: `areka-P0-balloon-lifecycle-events`（計測の成立条件を「トークの終わりが届いてから」に変える。その後の形の上で直す）。
- **Downstream**: 選択肢を使う実ゴーストの適合。

## Existing Spec Touchpoints

- **Extends**: 完了 `areka-P0-balloon-visibility`（抑止の条件）・完了 `areka-P0-choice-timeout-directive`。
- **Adjacent**: 文字とバルーンの列（`crates/areka-emo-text/src/actor*.rs`・`state.rs`）＝同じ列の spec と同時に走らせない。

## Constraints

- 要件で決める未知: 選択が済んだ合図を表示の側がどこから受け取るか（選んだときは UI が自分で知っている。時間切れの解除と中断は kanade が止める）。
- 規模の見込み: S（4〜7 タスク）。
- 1 ファイル 1,000 行未満・決定論のテスト必達・1 フレーム遅らせる解は取らない。

## 2026-10-10 棚卸㉓の再測定（main `ee3af616`・C4 の着地の後）

- **前提の変化**:
  - `balloon-lifecycle-events` が着地した（10-08）。時間切れの計測は「トークの終わりが表示の側へ届いてから」だけ始まる（`crates/areka/src/emo2_boot/balloon_visibility_wait.rs` の `decide_timeout`）。
  - kanade は選択待ちを進行中のトークに結び付けているので、選択を待つ間はトークが終わらない見立て（設計で確かめる）。そうなら、待っている間は「選択肢が表示中」の抑止が無くても計測は始まらず、この抑止が効くのはトークが終わった後＝本 brief の不具合の場面だけになる。
  - 不具合そのものは残っている。`crates/areka-emo-text/src/actor.rs` の `TextLayerRuntime::choice_active` は今も「選択肢の行が残っているか」で答える。
- **触るファイル**:
  - 案 A（今回見えた直し方）＝抑止の数え方だけを直す: `crates/areka/src/emo2_boot/balloon_visibility_wait.rs`（289 行・抑止を数える `observe_suppression`）と可視性の検査（`balloon_visibility_timeout_suppression_cases.rs` ほか）。emo-text には触らない。
  - 案 B（本 brief の 2 案）＝文字の層の「選択待ち」の持ち方を直す: `crates/areka-emo-text/src/{actor.rs, state.rs}` と、同じ問いを使う入力の 4 か所（`crates/areka/src/input_events/` の `balloon_moved.rs`・`balloon_pressed.rs`・`balloon_exit.rs`・`shell_box_handler.rs`）。
- **規模**: S（案 A 3〜5・案 B 5〜7 タスク）。
- **先に要るもの**: `balloon-lifecycle-events`（着地済み）。
- **優先度の区分**: B（バグ。用の済んだバルーンが消えず、`OnBalloonTimeout` も届かない）。
- **要件定義のモデル**: Fable（合図の届く順と時刻の話・どの層で直すかの分かれ目）。
- **分割の案**: 切らない。
- **見つけた穴・古くなった記述**:
  - 済んだ選択肢の行は、マウスを乗せると光り続け、押すと選択の知らせが kanade へ飛んで警告 1 行（`choice_rejected_no_wait`・`crates/areka-kanade/src/schedule/steady.rs` の `on_choice`）で捨てられる。案 A だけではこれが残る＝要件で扱うかを決める。
  - 案 A なら `anchor-tag-canon` と触るファイルの重なり 0＝同じウェーブに置ける。案 B は `actor.rs`・`state.rs` が重なる＝前か後。
  - roadmap の「文字とバルーンの列」という置き場所は、案 A なら当たらない。`clippy-199-lints` が `balloon_visibility_*` に触る。実機ではまだ確かめていない。

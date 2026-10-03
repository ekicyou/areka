# Brief: areka-P0-sakura-time-critical

> 2026-10-03 `areka-P0-emo-text-file-split` の完了フロー（未解決問題の棚卸）で起票。出どころは同 spec の実機の確かめで開発者が見た「メニューを出した後に後続のトークが止まってないときがある」（`completed/areka-P0-emo-text-file-split/verification/notes.md` §3「持ち越しの議題」）。調べた結果、観測そのものは**正典どおり**で、正典から外れているのは本 spec の `\t` だけだった。file:line は起票時値（ブランチ `claude/areka-p0-emo-text-split-dc4e04`・HEAD `b6f22e53`）。

## Problem

ゴーストの作者は、台本に `\t`（タイムクリティカルセクション）を書くと「この先は台本の終わりまで、なでなでやクリックの反応で話を割り込ませない」と指定できる。メニューや大事な台詞を最後まで見せたいときに使う正典の手段である。areka は `\t` を読まないので、`\t` を書いたゴーストでも、話の最中にマウスを動かすだけでなでなでの返事に置き換わる。

## Current State

**観測（2026-10-03・正典どおりと判定）**: 体のダブルクリックでメニュー（選択肢あり・`OnMouseDoubleClick` の返事）が出た 0.5 秒後、体の上のマウスの動きで `OnMouseMove` の返事が届き、`steady_talk_replace` でメニューが置き換わった。

- 正典の根拠: ukadoc の `\t`（`https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_5ct:1`）は「スクリプトブレーク(例:選択肢を選ぶ、バルーンダブルクリックによる中断)か\eまでの間、マウス系などのイベント通知を行わない」。止めたい区間を台本で明示する作り＝既定では話の最中（選択待ちを含む）もマウス系は届き、返事の台本は今の話を中断して再生される。`Status` には `talking`／`choosing`／`timecritical` があり（`spec_shiori3.html#Status_20_5bSSP_62e1_5f35_5d:1`）、SHIORI 側が状態を見て黙る前提（里々の既定「トーク中・誘導中・…時間クリティカル中は反応しません」が SHIORI 側の自衛の例）。
- areka の裁定は既にこの形: 完了 `areka-P0-input-events` の DD-IE-1（話の最中のマウスも常に GET）・DD-IE-2（話の最中に届いた返事は置換・根拠は SSP 2.3.86 の通信記録と「`\t` が防ぐ側の opt-in であること自体が既定＝中断の証左」）。完了 `areka-P0-choice-select-events` 要件 4.3 も選択待ちを同じ単一 slot 調停に従わせる。
- 置換の述語は `crates/areka-kanade/src/schedule/events.rs` の `value_replaces_active_talk`（`OnSecondChange` 以外は置換）、調停は `steady.rs` の `on_reply`。`OnSecondChange` は話の最中は NOTIFY・Reference3=0 で出て返事は捨てる（ukadoc どおり）。
- **`\t` は未実装**: `areka-sakura` に `\t` を読む処理が無い。`crates/areka-kanade/src/status.rs` の `ExecutionState::TimeCritical` は語彙だけあり、出どころの無い差し替え口（常に無効）のまま。`\t` の区間でマウス系の通知を止める仕組みも無い。
- 隣の `areka-P0-status-execution-states`（C1・バグ）の表に「timecritical | `\t` 区間中 | sakura 再生（`\t`）」の行があるが、受け持つのは `Status` に載せることだけで、出どころ（`\t` の区間）は「まだ無い」と書いている。

## Desired Outcome

- 台本の `\t` を読み、その位置から「台本の終わり（`\e`）か、スクリプトブレーク（選択肢を選ぶ・バルーンのダブルクリックでの中断）」までをタイムクリティカルの区間として持つ。
- 区間の間は、マウス系などのイベント通知を SHIORI へ送らない（どの語を止めるかは正典の「マウス系など」を ukadoc で引き直して要件で確定する）。
- 区間の間、`Status` に `timecritical` を載せる（`status-execution-states` の差し替え口へ出どころを渡す）。
- `\t` の無い台本の振る舞いは 1 つも変えない（DD-IE-1／DD-IE-2 の置換はそのまま）。

## Approach

`\t` を台本の位置つきの合図として cue へ写し（`\![enter/leave,nouserbreakmode]` を `user_break_cue.rs` 経由で届けている完了 `balloon-break` の形が先例）、kanade が区間の開始・終了（台本の終わり・中断・選択の確定）を知って、マウス系の入口（`schedule/mod.rs` の `Input::Mouse` を Steady 相で受ける所）で通知を止める。`Status` の `timecritical` は同じ区間の旗から導く。

## Scope

- **In**: `\t` の字句・cue への写し／区間の開始と終了（`\e`・スクリプトブレーク・選択の確定・中断・置換）／区間の間のマウス系などの通知の抑止／`Status` の `timecritical` の出どころ／決定論テスト（区間の内外・終わり方ごと）
- **Out**: `\t` の無い台本の調停の変更（DD-IE-2 の置換は正典どおりで変えない）／ゴースト（emo2 の辞書）の作りの修正（なでなでの返事が `Status` を見て黙るかはゴースト側の領分）／`OnSecondChange` の扱い（既に正典どおり）

## Boundary Candidates

- 台本の側: `\t` の字句と cue の合図（`areka-sakura` の lexer・compile）
- 運行の側: 区間の旗とイベントの抑止（`areka-kanade` の `schedule/`）
- 状態の側: `Status` の `timecritical`（`areka-kanade/src/status.rs`・`status-execution-states` の差し替え口）

## Out of Boundary

- 話の最中の返事の置換の規則そのもの（完了 `input-events` DD-IE-2・`choice-select-events` 4.3 の正本）
- `quicksection`・`balloonwait` などの時間の指令（`sakura-time-directives`）
- 中断禁止 `\![enter,nouserbreakmode]`（完了 `balloon-break`）

## Upstream / Downstream

- **Upstream**: `areka-P0-status-execution-states`（`Status` の差し替え口を実値化する枠組み・C1）／完了 `input-events`・`choice-select-events`・`balloon-break`
- **Downstream**: `\t` を使う実ゴーストの適合

## Existing Spec Touchpoints

- **Extends**: なし（新しい境界）
- **Adjacent**: `status-execution-states`（`timecritical` の行を本 spec が出どころとして満たす）・`sakura-time-directives`（同じ compile の時間まわりだが別の語）・台本のコンパイルの列（`compile.rs` を共有する spec と同時に走らせない）・kanade の進行の列（`schedule/` を共有する spec と同時に走らせない）

## Constraints

- 正典は ukadoc。`\t` の終わり方と止めるイベントの範囲は要件の段で ukadoc を引き直して確定する（SSP 実測主義は取らない）。
- 決定論テスト必達。`\t` の無い台本の既存テストは 1 本も変えない。
- 規模の見立て: S〜M（6〜10 タスク）。段は「その他」（正典の穴で、`\t` を使うゴーストが現れるまで利用者に見える害は無い）。

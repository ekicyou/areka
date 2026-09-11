# Brief: areka-P0-kanade-boot-talkdone-drop

> **起票 2026-09-11**（`/kiro-discovery`・開発者「起票候補はすべて起票せよ」）。出所: `areka-P0-emo2-conformance-e2e` の決定論層の作業（タスク 5.5・2026-09-06）が構造から見つけた製品欠陥。登記: 同 spec `verification/acceptance-record.md` §13.2 行 4・design D9。**走行では発現していない**（点灯語 `event=boot_input_ignored` は 2026-09-10 の一周で全走行 0 行）。

## Problem

kanade が起動系列の `BootVersion` に滞在している間に、起動挨拶の再生完了（`TalkDone{Ended}`）が届くと、その通知が**捨てられる**。トーク枠が定常相（`Steady{Some}`）へ漏れたまま残り、以後の終了の握手（`OnClose` → 挨拶 → `\-` → `StopSelf`）が二度と始まらない。利用者から見ると「終了指示を出しても終了挨拶が流れず、窓が閉じない」。

## Current State

- `crates/areka-kanade/src/schedule/mod.rs` の `step` は `Input::TalkDone` を `on_talk_done`（`:514`）で横断的に扱うが、起動系列中は `dispatch_phase`（`:646` 付近）が `BootVersion` を `boot::step` へ渡し、`crates/areka-kanade/src/schedule/boot.rs` の「上記以外」の分岐（`:33-36`）が `warn!(event="boot_input_ignored")` を書いて捨てる。突合の側（`mod.rs` の `current_talk_id`・`:681-694`）は `BootVersion{Some}` を突合対象に含めて防御しているのに、委譲先が捨てる。
- 発現の条件: 起動挨拶の再生完了が `basewareversion` の応答より先に届くこと。`boot.rs:241` が再生の起動を応答の要求より先に積み、実機の起動挨拶は数秒・応答は数ミリ秒なので、実機では窓が開かない。決定論のハーネスでは起動記録トークが空になり得るため再現できる（e2e タスク 5.5 の経緯＝Implementation Notes「⑹ の真の機序」）。

## Desired Outcome

1. `BootVersion` 滞在中に届いた `TalkDone` を捨てず、`on_talk_done` の突合（`talk_id`）へ渡す。突合が一致すればトーク枠を空にし、以後の握手が成立する。
2. 決定論テスト: 「`BootVersion{Some}` で `TalkDone{Ended}` → 枠が空・`boot_input_ignored` 0 行・その後の `CloseRequest` で `OnClose` GET が出る」を固定（直す前は赤）。既存の boot 系列の試験は不変。
3. `boot_input_ignored` は「本当に無関係な入力」（Tick 等）にだけ残す。

## Approach

- `boot::step` の「上記以外」から `TalkDone` を分け、`mod.rs` の横断遷移へ委譲する（腕 1 本の追加・ponytail）。あるいは `dispatch_phase` が `TalkDone` を相に依らず先に `on_talk_done` へ回す。どちらが既存の順序規律（DD-IT-12「アクティブな talk を運ぶ相」）に合うかは設計で決める。

## Scope

- **In**: `crates/areka-kanade/src/schedule/{boot.rs, mod.rs}` と兄弟試験。
- **Out**: 起動系列の順序そのもの・areka 側の配線（`spine.rs`）。

## Boundary Candidates

- 相ごとの入力の委譲規則（`dispatch_phase`）／`TalkDone` の横断扱い。1 spec・S 規模。

## Out of Boundary

- 終了の握手の配線（e2e 6.9 で着地済み）・ホスト窓スレッドの pump（別 spec）。

## Upstream / Downstream

- **Upstream**: `areka-P0-emo2-conformance-e2e`（発見と登記・決定論一周テストが検出器）。
- **Downstream**: なし（e2e の完成判定は待たない）。

## Existing Spec Touchpoints

- **Extends**: kanade の起動系列（完了 spec `areka-P0-kanade-*` の DD-IT 規律）。
- **Adjacent**: `emo2-conformance-e2e`（`spine_conformance_*` の檻が非回帰の検出器）。

## Constraints

- 決定論テストで RED 先行。`cargo test -p areka-kanade` と `cargo test -p areka --bin areka` 緑。ログ語彙（`boot_input_ignored`）は e2e 手順書 §5.7 の点灯語なので綴りを変えない。

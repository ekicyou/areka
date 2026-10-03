# Brief: areka-P0-drag-cancel-borrow-miss

> 2026-10-03 `/kiro-discovery` で起票（`drag-click-without-move` の完了の手順の中で見つけた、以前からの穴）。**種別はバグ**（実機では未観測・静的に確かめたもの）。本文のソースの指し先は起票時（`fc45d5fe`）の実物＝着手時に引き直すこと。

## Problem

- **利用者**: まれな条件で、ドラッグを取り消したときに位置が保存されない。その直後に動かさずにクリックすると、1 回だけ位置が保存される。
- 条件は「画面更新（tick）が World を借りている最中に、同期で届くメッセージ（再入）」で、実機ではまだ見ていない。

## Current State

- `crates/wintf/src/ecs/window_proc/keyboard.rs` の 4 つのハンドラ（`WM_KEYDOWN` の ESC・`WM_CANCELMODE`・`WM_CAPTURECHANGED`・`WM_ACTIVATE`）は、`world.try_borrow()` が失敗すると `DragAccumulatorResource` へ `DragTransition::Ended` を積まない。そのまま `crate::ecs::drag::cancel_dragging()` へ進むので、ドラッグの状態は休む状態へ移る。
  - `crates/wintf/src/ecs/window_proc/mouse_click.rs` の左ボタンを離したときの枝にも `world.try_borrow()` の失敗で終了を積まない形がある。同じ穴かは着手時に確かめる。
- そのため累積器の `current_dragging_entity` が `Some` のまま残り、次の 2 つが起こりうる。
  1. 閾値を越えたドラッグの取り消しで終了の知らせが落ちる。「閾値を越えた後の取り消しでも、その時点の位置を保存する」（`drag-click-without-move` 要件 2.5 が今どおりと確かめた振る舞い）が、この条件では崩れる。
  2. 次の閾値前のクリックの `Ended` が、`drag-click-without-move` で入れた累積器の入口の判断を 1 回だけ通ってしまう。入口の判断は `crates/wintf/src/ecs/drag/accumulator.rs` の `set_transition` にあり、開始を積んでいない `Ended` を捨てる。通った結果、動かさないクリックで位置が保存されうる。
- 以前（入口の判断が入る前）から 1. の穴はあった。2. は入口の判断が `current_dragging_entity` を見るようになって生まれた形。

## Desired Outcome

- World の借用が取れなくても、取り消し・離しで累積器の「ドラッグ中の対象」が置き去りにならない。
- 閾値を越えたドラッグの取り消しは、この条件でも今どおり 1 件保存される（または、落とすならそれを決めて記録に残す）。
- 動かさないクリックは、どの条件でも位置を保存しない。
- 直す前に赤になる決定論のテストがある。借用中の World へハンドラを呼ぶ形で、置き去りと次のクリックの保存を見る。

## Approach

設計で決める。候補は次の 2 つ。
- 累積器を World の借用なしで触れる置き場へ移す（wndproc と ECS の間の箱という役目に合う）。
- 借用が取れないときの扱いを決める（例: 次の `start_preparing` で累積器の対象を片づける）。

## Scope

- **In**: wintf の wndproc のハンドラが World を借りられないときの、終了の種と累積器の対象の扱い。決定論のテスト。
- **Out**: ESC・`WM_CANCELMODE` が受け取った窓とドラッグ中の窓の一致を確かめない件（`drag-click-without-move` の research で範囲の外と記したもの）。areka の受け手の縮退。

## Boundary Candidates

- `crates/wintf/src/ecs/window_proc/{keyboard,mouse_click}.rs` の借用の失敗の扱い
- `crates/wintf/src/ecs/drag/accumulator.rs` の置き場と対象の片づけ

## Out of Boundary

- ドラッグの状態機械（`crates/wintf/src/ecs/drag/state/`）の移り変わり
- areka の `on_char_drag_end`・`on_balloon_drag_end`

## Upstream / Downstream

- **Upstream**: `drag-click-without-move`（累積器の入口の判断と待ち行列・完了）
- **Downstream**: なし

## Existing Spec Touchpoints

- **Extends**: なし（完了 spec `drag-click-without-move`・`event-drag-system` の約束を、再入の条件でも守らせる）
- **Adjacent**: `wintf-drag-state-rest-contract`（完了・休む状態の決まり）

## Constraints

- 1 フレーム遅らせる解は取らない。状態の持ち方を変えて 0 フレームで解く。
- 実機の根と一時フォルダはワークツリーの `target\` の下だけ。

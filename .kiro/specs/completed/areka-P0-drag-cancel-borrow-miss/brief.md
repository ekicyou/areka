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


---

## 2026-10-04 棚卸㉑の再測定（main `634032f6`・C2 の着地の後）

- 規模: S（4〜7 タスク）。切る: なし。
- 前提の状態: 上流 `drag-click-without-move` は着地済み（PR#216）。起票の後に `crates/wintf/src/ecs/window_proc/` と `crates/wintf/src/ecs/drag/` を触ったコミットは 0＝brief の指し先はそのまま。
- 崩れた前提／古くなった位置: なし。確かめたこと:
  - `keyboard.rs` の `WM_KEYDOWN`（ESC）・`WM_CANCELMODE`・`WM_ACTIVATE`（非活性化の枝）・`WM_CAPTURECHANGED` の 4 つは、どれも `world.try_borrow()` が取れたときだけ `DragAccumulatorResource` へ `DragTransition::Ended` を積み、取れなくても `cancel_dragging()` へ進む。
  - `mouse_click.rs` の左ボタンを離したとき: hit_test の枝は `try_borrow_mut()` の中にあり、取れなければフォールバックの枝へ落ちる。フォールバックの枝は `Dragging` なら窓の一致だけで `should_end` を真にし、その中の `try_borrow()` が取れないと終了を積まずに `end_dragging()` を呼ぶ＝**同じ穴がある**（brief の「着手時に確かめる」はこれで済み）。
  - `mouse_move.rs` の開始（`Started` を積む所）は、取り出したスナップショットの処理全体が World の借用の中にあり、取れなければ状態も動かさない＝穴ではない。
  - `DragAccumulatorResource` は `Arc<Mutex<DragAccumulator>>` を包む `Clone` の型で、World へ入れるのは `crates/wintf/src/ecs/world/mod.rs` の 1 か所（`insert_resource`）。読むのは `drag/dispatch.rs` の flush だけ。brief の案 1（World の借用なしで触れる置き場へ移す）は、この複製を wndproc の側（`drag/state/mod.rs` の `thread_local!` と同じ並び）にも持たせるだけで済む見込み。
- 触るファイル（並走の照合用）:
  - `crates/wintf/src/ecs/window_proc/keyboard.rs`・`keyboard_tests.rs`
  - `crates/wintf/src/ecs/window_proc/mouse_click.rs`・`mouse_click_tests.rs`
  - `crates/wintf/src/ecs/drag/accumulator.rs`・`accumulator_tests.rs`
  - 案 1 なら `crates/wintf/src/ecs/drag/state/mod.rs` か `drag/mod.rs`（複製の置き場）と `crates/wintf/src/ecs/world/mod.rs`（置き場へ渡す 1 行）
  - 共有しうる相手: `mouse-drag-events`（C2-⑦）は wintf の開始・終了の知らせを使う側で、触るのは kanade と areka の `input_events/mod.rs`＝重なり 0（同 brief の Adjacent も「触るファイルは重ならない」）。wintf を触る他の未完了 spec は C3 までに無い。`host32-testdll-marker-race` とも重なり 0。
- 議題（答えで作業が変わるものだけ）: なし。
- 見つけた穴（候補・着手時に確かめる）: `mouse_click.rs` のフォールバックの枝で、状態が `Preparing`／`JustStarted` のときは窓の一致を World の借用で調べるため、借用が取れないと `should_end` が偽になり、`end_dragging()` も呼ばれない＝**ボタンを離した後も準備の状態が残る**。`mouse_move.rs` は左ボタンが押されているかを見ずに閾値を比べるので、その後の移動で閾値を越えると、押していないのに開始が積まれうる。同じ「借用が取れないときの扱い」の話なので本 spec の範囲で一緒に見る。

## 2026-10-04 ウェーブ C3-②（棚卸㉑）

- 段は「バグ」。C3 は 11 本並走（`roadmap.md`「ウェーブ編成」の C3 の行が正本）。着手は最新の main から。
- 同じウェーブの約束: 触るのは `crates/wintf/src/runtime/wndproc_bridge.rs`（10-04 の要件ディスカッションの議題 1 で追加）と `crates/wintf/src/ecs/` の `window_proc/{keyboard,mouse_click}.rs`・`drag/`・`world/mod.rs` の 1 行とそのテストだけ。`mouse-drag-events`（C3-④）は wintf に触らない。

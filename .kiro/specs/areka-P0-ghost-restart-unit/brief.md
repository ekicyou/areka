# Brief: areka-P0-ghost-restart-unit

> 2026-09-24 `/kiro-discovery` 再入（棚卸⑯）で起票（台帳 #58）。`areka-P0-ghost-shell-balloon-switch`（台帳 #13）の再測定で想定タスクが 19〜22 本に膨らみ、1 spec 20 本の上限を超える恐れが出たため、**振る舞いを変えない括り出しだけ**を前に切り出した。切る場所は「kanade に触るかどうか」——本仕様は `crates/areka` の bin だけを触り、kanade には触らない。
> 本文のコードは「何の定義か」（関数名・型名＋ファイルパス）で指す。事実は**起票時の実測**（2026-09-24・main `0b01f654`）。着手時に必ず引き直すこと。

## Problem

**誰の何が困っているか**: ゴースト切替（#13）・シェルとバルーンの切替（#50）・インストール直後の切替（#15）を作る側。今日の areka は「1 プロセスに 1 回だけ起動して、終わったら終了する」形で組まれており、**同じプロセスの中でゴーストを降ろして起こし直す単位が無い**。

具体的には次の 4 つが、1 回きりを前提にしている。

1. **終了順序が `fn main`（`crates/areka/src/main.rs`）にべた書き。** `app.run()?` の後ろに、loop ticker の Close → `GhostRuntime::shutdown(CloseReason::User{scope:0})` → seriko の join → perf の最終報告が並ぶ。#49 も #12 もここを関数にしていない。
2. **「1 回だけスケジュールへ登録する」前提が 8 か所。** `wire_emo2_boot`（`emo2_boot/mod.rs`）・`menu::wire_menu_with`・`readme::wire_readme`・`input_events::user_break::wire_user_break`・`input_events::choice_drain::wire_choice_drain`・`input_events::balloon::wire_balloon_choice`・`placement::spawn::wire_zorder_pair`・`main.rs` の `open_startup_window`。うち readme・user_break・choice_drain の 3 か所はコメントで「`main` から 1 度しか呼ばれない」を前提と明記している。系（system）の登録と、ゴーストごとに差し替える状態が同じ関数に同居しているので、2 度呼ぶと系が二重に登録される。
3. **`wire_emo2_boot` は `&WinApp` を取り、中で `app.world().borrow_mut()` する。** フレームの系の中（World を借りている最中）から呼ぶと二重借用で落ちる。`open_startup_window` も「1 回だけの系の登録」と「窓を作る」が 1 つの関数に同居している。
4. **全窓を消す部品 `despawn_app_windows`（`crates/areka/src/app_exit.rs`）は私有**で、完了 `app-lifetime-separation` 要件 3.6 により単独では呼べない。「窓を閉じるが終了しない」操作が無い。

## Current State

- 寿命の分離は済んでいる（完了 #49）: `WinApp::with_exit_policy(ExitPolicy::Explicit)` で `run()` は `quit_app` の指示でしか戻らない。窓 0 の瞬間があってもアプリは落ちない。
- 起動成功時の後処理 `main.rs` の `on_boot_ok`（永続の結線と記憶の書き込み）は、繰り返し呼べる単位の芯になる（完了 #12）。
- ゴーストごとに差し替えが要る窓ごとの状態は 5 つ: `MouseWiring`・`MenuWiring`・choice_drain の kanade 送り口・`PersistWiring`・readme の経路。
- `main.rs` は 774 行・`emo2_boot/mod.rs` 727 行・`frame.rs` 459 行・`frame/wiring.rs` 352 行・`app_exit.rs` 154 行。

## Desired Outcome

完了時に次が真になっている。**利用者から見える振る舞いは 1 つも変わらない。**

1. `fn main` の終了順序が、繰り返し呼べる 1 つの関数になっている（終了理由を引数に取る）。
2. 8 か所の `wire_*` が「系の登録（プロセスに 1 回）」と「ゴーストごとの状態の載せ替え」に分かれ、後者を 2 度呼んでも系が二重登録されない。
3. `wire_emo2_boot` 相当が `&mut World`（または `CommandSender` 経由）で呼べ、フレームの系の外から実行できる。`open_startup_window` の「登録」と「窓を作る」が分かれている。
4. 「全窓を閉じるが終了しない」操作が `app_exit.rs` に在り、`quit_app` を通らない（完了 #49 の design の申し送り「切替は `quit_app` を通らない」と一致）。要件 3.6 の意図（単独で窓を消して寿命を狂わせない）は、その操作が「直後に起こし直す」呼び手にだけ公開される形で守る。
5. 既存の決定論テストが全て緑のまま（配線の再テストはしない・記憶 test-only-decision-branches-not-proven-wiring）。加えて「**同じプロセスで降ろして起こし直す**」決定論テストが 1 本ある（偽の SHIORI・偽の資産で 2 周し、系の登録数と窓ごとの状態が 1 周目と同じであること）。
6. 相乗り候補（要件で決める）: 台帳 #57「起動後の途中終了で記憶の確定が飛ぶ」——`app.run()` が `Err` を返すと終了順序を通らない。1 の関数が在れば `Err` の腕でも同じ関数を通すだけで閉じる（1〜2 タスク）。#55 が先に採っていれば本仕様は触らない。

## Approach

**選んだ形**: 振る舞いを固定したまま、構造だけを「1 回」から「n 回」へ広げる。正典の語彙は持たない（`\![change,…]`・`OnGhostChang*` は #13）。

| 段 | 中身 | 検証 |
|---|---|---|
| ① 終了順序 | `fn main` の `app.run()` の後ろを関数へ | 既存 smoke 3 方向が緑 |
| ② 登録と状態の分離 | 8 か所を「登録」と「載せ替え」に分ける・5 つの窓ごとの状態を載せ替えの単位に | 既存テスト緑＋2 周の決定論テスト |
| ③ 借用の形 | `wire_emo2_boot` を `&mut World` 化・`open_startup_window` の分割 | コンパイルが通る＝借用の証明 |
| ④ 窓だけ閉じる | `app_exit.rs` に操作を 1 つ | 決定論: 閉じたあと `AppExit` が立っていない |

**取らない形**: `Drop` で自動 shutdown（終了理由を持てない）／`GhostBootOptions` に欄を足す（構造体リテラルが 27 か所・16 ファイルに波及）——起動元の情報を渡す口は #13 が `boot_with_kanade_stop` 型の派生関数で作る。

## Scope

- **In**: 上の ①〜④・2 周の決定論テスト・`doc/COMPAT_ARCHITECTURE.md` §8 への 1 行（必要なら）・#57 の相乗り（要件で決める）
- **Out**: 切替の語彙とイベント・名前解決・kanade の握手・メニュー登記（#13）／シェル・バルーンの差し替え（#50）／告知（#55）／`GhostRuntime` の中身（`crates/areka-ghost`）

## Boundary Candidates

- 終了順序（`main.rs` の 1 関数）
- 系の登録と窓ごとの状態（`emo2_boot/mod.rs`・`frame/wiring.rs`・`input_events/*`・`readme.rs`・`placement/spawn.rs`・`menu/mod.rs` の `wire_menu_with`）
- 窓だけ閉じる操作（`app_exit.rs`）

## Out of Boundary

- 切替そのもの（#13・#50）／終了コードと告知（#55）

## Upstream / Downstream

- **Upstream**: 完了 `areka-P0-app-lifetime-separation`（`ExitPolicy::Explicit`・`quit_app`・`despawn_app_windows`）・完了 `areka-P0-baseware-root-layout`（`on_boot_ok`）・完了 `completed/areka-P0-shiori-fault-notice`（#55・2026-09-25 完了。`run()` の後は wintf がフレームを回さない守りと `finish_after_run` を入れた＝design の Revalidation Triggers を見よ・`main.rs` と `app_exit.rs` を共有するので**先に着地**）。
- **Downstream**: `areka-P0-ghost-shell-balloon-switch`（#13・本仕様の上に kanade の握手を建てる）・`areka-P0-shell-balloon-switch`（#50）・`areka-P0-ghost-install`（#15）。

## Existing Spec Touchpoints

- **Extends**: なし（構造の括り出し）。完了 `app-lifetime-separation` 要件 3.6 の「単独で呼べない」は守ったまま、呼び手を限定した操作を足す＝上書きではない（要件で確認し、変えるなら §8 に記す）。
- **Adjacent**: #55（`main.rs`・`app_exit.rs`・`frame.rs` を共有＝直列）／#13（`emo2_boot/mod.rs`・`frame.rs` を共有＝直列）。

## Constraints

- 規模 **M**（タスク 6〜7 本）。段は **α**（#13 の前提）。
- **要件と設計は Fable**（括り出しの形＝登録と状態の切り分け・借用の形が、後ろの切替 3 本の建て方を決める。完了 #49 と同じ種類の基盤）。
- 1 ファイル 1,000 行（`main.rs` 774・`emo2_boot/mod.rs` 727 は分割で減る方向）。
- 決定論テスト網羅は必達。ただし檻に入れるのは「2 周しても同じ」の判断分岐だけで、配線は再テストしない。
- 文書と報告では「同じプロセスの中でゴーストを降ろして起こし直せる形にする」と平易に書く（記憶 no-project-jargon-in-user-facing-docs）。

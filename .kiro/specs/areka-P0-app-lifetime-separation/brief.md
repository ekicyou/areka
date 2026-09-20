# Brief: areka-P0-app-lifetime-separation

> 2026-09-20 `/kiro-discovery` 再入（棚卸⑮）で起票。`areka-P0-ghost-shell-balloon-switch`（台帳 #13・規模 L）の Approach ①「寿命の分離」を**単独の spec として切り出した**（台帳 #49）。
> 切り出しの理由は 2 つ。⑴ #13 の想定タスクが 30〜40 本で 1 spec の上限（20 本）を大きく超えた。⑵ この段は列挙も記憶も使わないので `areka-P0-baseware-root-layout` を待たずに着手でき、α の直列経路を 1 段短くできる。
> 本文の file:line は**起票時の実測値**（2026-09-20・main `fe157df1`）。着手時に必ず引き直すこと。

## Problem

**誰の何が困っているか**: ゴースト・シェル・バルーンを切り替えたい利用者（の手前に居る、切替を実装する開発者）。

今日の areka は**窓が 1 枚も無くなった瞬間にアプリが終わる**。切替の途中では、古いゴーストの窓を全部閉じてから新しいゴーストの窓を開くので、窓が 0 枚の瞬間が必ず来る。このままでは切替の途中でプロセスが終わる。

**#13 の brief が見落としていた所在**（2026-09-20 実測）: 「窓 0 で終了」を決めているのは `crates/areka/src/main.rs` ではなく **wintf** である。登録表が空になった瞬間に終了の合図を撃つ仕掛けが `crates/wintf/src/runtime/mod.rs` の `wire_shutdown_hook` と `crates/wintf/src/runtime/window_registry.rs` に在り、`main.rs` は `app.run()?` を呼んで戻りを待つだけである。#13 の brief と roadmap の A2 行は wintf を 1 文字も挙げていなかった。

## Current State

「窓 0 で終了」に乗っている箇所（実測）:

- `crates/areka/src/emo2_boot/frame.rs` の `run_ghost_quit_phase`（終了の握手のあと全窓を破棄し、あとは wintf の仕掛けに任せる）
- `crates/areka/src/input_events/mod.rs` の強制退避（Ctrl＋Shift＋左ダブルクリック）
- `crates/areka/src/main.rs` の `on_dummy_pressed`（ゴーストの根が無いときのダミー窓）と smoke のクロージャ
- `crates/areka/tests/smoke_boot_loop_exit.rs`
- `WinApp::new` と `run()` を呼ぶ wintf の example 7 本

終了の経路そのものは、`areka-P0-balloon-break`（PR#164）のあと次の形になっている: メニューの `request_close` → `MouseWiring::send_close_request` → `KanadeMsg::CloseRequest` → `schedule/close.rs` → `StopSelf` → `notify_stop` → `KanadeStopped{cause}` → `run_ghost_quit_phase` → 全窓破棄 → wintf の終了の仕掛け。中断のあとの終了は新設の `schedule/user_break.rs` が引き受ける。

`AppExit` に当たる語は `crates/` に **0 件**（`git grep -l "BasewareRoot\|SwitchRequest\|AppExit" -- 'crates/*.rs'` ＝ 0。同じ書式の検索が `OnClose` で 63 ファイルに当たることを確かめてある）。

## Desired Outcome

完了時に次が真になっている。

1. **areka のアプリは「終了の指示」で終わり、窓の数では終わらない。** 窓が 0 枚の瞬間があってもプロセスは生き続ける。
2. **今日の終了操作はどれも、これまでと同じ見え方で終わる。** メニューの「終了」・別れの台詞のあとの終了・中断のあとの終了・強制退避・ダミー窓・smoke の自動終了。違いは「全窓破棄のあと、明示の終了の指示を 1 つ送る」ことだけで、利用者からは区別が付かない。
3. **wintf の既定は変わらない。** example 7 本と wintf 単体の利用者は、今までどおり窓 0 で終わる。切り替えるのは areka だけである。
4. 「窓 0 でも生きている」ことと「終了の指示で必ず終わる」ことを、それぞれ決定論テストが赤にできる。

## Approach

wintf に「窓 0 で終了するか」を利用側が選べる口を 1 つ足し、areka がそれを切る。終了の指示の送り手は areka の既存の終了経路の末尾（`run_ghost_quit_phase` の全窓破棄の直後ほか、上の一覧の各所）に置く。

要件段階で決めること（裁定候補）:

- **口の形**。⑴ `WinApp` の構築時の設定値／⑵ 実行中に切り替えられる資源／⑶ 終了の仕掛けを利用側が差し替える、のどれか。推すのは ⑴＋明示の終了 API 1 本——切替の最中だけ止める ⑵ は「戻し忘れ」という状態を新しく作る（記憶 no-frame-delay-fixes-change-the-state-shape の趣旨＝状態を増やさず形を変える）。
- **終了の指示を送り忘れた経路の扱い**。窓 0 のままプロセスが残り続けるのが最悪の壊れ方である（利用者からは見えず、タスクマネージャでしか止められない）。送り忘れを構造で防ぐ形（全窓破棄と終了の指示を 1 つの関数にまとめ、片方だけを呼べなくする）を推す。

## Scope

- **In**: wintf の終了の仕掛けの選択口と明示の終了 API／areka の終了経路（上の一覧）への終了の指示の結線／smoke テストの追随／「窓 0 でも生きている」「終了の指示で終わる」の決定論テスト。
- **Out**: 切替そのもの（`areka-P0-ghost-shell-balloon-switch`・`areka-P0-shell-balloon-switch`）／`KanadeStopCause` へ「切替」の値を足すこと（ゴースト切替の仕事）／ダミー窓の経路を消すこと（`areka-P0-baseware-root-layout` の仕事。本仕様は今在る経路に終了の指示を足すだけ）／トレイアイコン等の「窓が無いときの操作の入口」（棚卸⑭の裁定候補 ⑷＝含めないを推す）。

## Boundary Candidates

- wintf の終了の仕掛け（`crates/wintf/src/runtime/mod.rs`）
- areka の終了経路の末尾（`emo2_boot/frame.rs`・`input_events/mod.rs`・`main.rs` の 2 関数）

## Out of Boundary

- kanade の終了の握手（`schedule/close.rs`・`user_break.rs`）。合図を受けた**あと**だけを変える。握手の決定論テストは 1 本も変えない。
- `crates/areka-ghost/src/runtime.rs` の `shutdown`（変えない）。

## Upstream / Downstream

- **Upstream**: なし。完了 `areka-P0-popup-menu-minimal`（終了の入口）と完了 `areka-P0-balloon-break`（終了の握手の今の形）の上に建つ。**`baseware-root-layout` を待たない。**
- **Downstream**: `areka-P0-ghost-shell-balloon-switch`（ゴースト切替。本仕様が必須）。`areka-P0-shell-balloon-switch`（キャラ窓が残る設計なら不要・バルーン窓だけが一度 0 になる設計なら要る）。

## Existing Spec Touchpoints

- **Extends**: なし。
- **Adjacent**: `areka-P0-baseware-root-layout`——**`crates/areka/src/main.rs`（958 行）の別の関数と `crates/areka/tests/smoke_boot_loop_exit.rs` を共有する**。本仕様が触るのは `on_dummy_pressed`・smoke のクロージャ・`app.run()` の後ろ、相手が触るのは `main()` の根の解決と代替経路。関数は別だが同じファイルなので**実装は直列**にする（本仕様が先・相手は要件と設計だけ先行してよい）。`main.rs` は 1,000 行の上限が近く、後着の側が切り出しを負う。

## Constraints

- 規模 **S**（タスク 5〜7 本）。ただし終了経路の形を決める仕事なので、要件と設計は Fable で行う（記憶 canonical-not-minimal-lifecycle＝終了経路は正規実装・小細工禁止）。
- wintf の既定の振る舞いを変えない。example 7 本を 1 本も書き換えずに済む形にする。
- 既存の終了経路の決定論テストを 1 本も落とさない。配線の再テストはしない（記憶 test-only-decision-branches-not-proven-wiring）。足すのは判断の分かれ目 2 つだけ。
- 実機確認は有界の自動終了＋ログの絞り込みで足りる（記憶 areka-real-machine-signoff-bounded-auto-exit）。終了後にプロセスが残っていないことを必ず見る——**止めてよいのは自分が起こしたと確認できたプロセスだけ**（記憶 never-kill-processes-by-assumption）。

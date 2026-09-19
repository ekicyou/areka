# Brief: areka-P0-wintf-drag-state-rest-contract

起票: 2026-09-19（`areka-P0-popup-menu-minimal` の実機確認 9.3 と最終検証が発見・開発者指示「推奨タスクはツールチップにせず spec に起票」）

## Problem

wintf のドラッグの状態（スレッドごとの値・`crates/wintf/src/ecs/drag/state/mod.rs`）を読む側は、「待機（`Idle`）でなければドラッグ中」と読むと間違える。説明と実装が食い違っているからである。

- 説明: `DragState::JustEnded` の doc は「ドラッグ終了直後（1フレームのみ）」、`reset_to_idle` の doc は「ドラッグ状態を Idle にリセット（dispatch_drag_events 後）」。
- 実装: `reset_to_idle` の呼び手は**製品コードに 1 つも無い**（呼ぶのは `drag/state/tests.rs` と `clickthrough/controller_tests.rs` だけ）。左ボタンを 1 度押して離すと（`window_proc/mouse_click.rs` の `start_preparing` → `end_dragging`）、状態は `JustEnded` のまま次の左押下まで休み続ける。つまり製品では、起動後の最初の左クリック以降 `Idle` へは二度と戻らない。

実害は 2026-09-19 に 1 件出た。`crates/areka/src/menu/trigger.rs` の解放ハンドラが「`Idle` でなければドラッグ中」と読み、**左クリックを 1 度した後は右クリックメニューが二度と出なくなった**（実機ログ: 以後の解放がすべて `[menu] ignored release reason="dragging"`）。決定論テストは「ドラッグ中」の代役に `JustEnded` を置いていた（捕捉の持ち主 `CaptureGuard` が要らない唯一の非待機状態だった）ので、欠陥そのものを合格条件に固定しており、実機でしか見つからなかった。

利用側は同日是正した（コミット `b7968925`・`JustEnded` を待機と同じに扱う）。しかし**契約そのものは偽のまま**で、次にこの状態を読む人が同じ罠を踏む。

## Current State

- wintf の内側の読み手 3 か所は、すでに `JustEnded` を「ドラッグ中ではない」と読んでいる: `clickthrough/controller.rs` の `resolve_transition`（`Idle | Preparing | JustEnded` を非ドラッグの写像へ・doc に「`JustEnded` 再収束（R5.2）」）、`start_preparing`（`JustEnded` からの新しい押下を許す）、`window_proc/keyboard.rs`（`Idle / JustEnded: 何もしない`）。
- `runtime/mod.rs` の透過制御の再評価の doc も `JustEnded` の再収束（R5.2）に触れている＝「終了直後の 1 周で正しい透過状態へ戻す」ことに `JustEnded` が使われている。
- areka 側の読み手は `menu/trigger.rs` の 1 か所だけ（`DragStateSnapshot::Idle | JustEnded { .. }` を休みと読む・是正済み）。
- `DragStateSnapshot` に「いまドラッグ中か」を答える述語は無く、読み手はそれぞれ自分で variant を並べている。

## Desired Outcome

- 「ドラッグの状態がどこで休むか」の説明と実装が一致している。
- 読み手が variant を並べずに「左ボタンを押している間か」を 1 つの述語で聞ける（次の読み手が同じ罠を踏まない）。
- 契約が後退したら赤くなる決定論テストが 1 本ある（手で組んだ状態ではなく、製品と同じ関数 `start_preparing` → `end_dragging` を踏む）。

## Approach

直し方は 2 通りあり、要件段階で選ぶ。`menu/trigger.rs` の是正はどちらでも動き続ける。

- **案 A（説明を実装に合わせる・推奨の出発点）**: `JustEnded` の doc を「終了後はここで休む（次の左押下まで）」へ直し、`reset_to_idle` は製品の呼び手が無いことを明記するか、テスト専用へ格下げする。`DragStateSnapshot` に述語（例 `is_button_held()`＝`Preparing | JustStarted | Dragging`）を足し、areka の `menu/trigger.rs` と wintf 内の読み手をそれへ寄せる。挙動は 1 ビットも変えない。
- **案 B（実装を説明に合わせる）**: フレームの終わりで `reset_to_idle` を呼ぶ。ただし透過制御の R5.2（`JustEnded` の周で再収束する）が `JustEnded` を 1 周ぶん観測できることに依存しているかを先に確かめる必要がある。**1 フレーム遅らせて辻褄を合わせる解は取らない**（開発規律）。

## Scope

- **In**: `crates/wintf/src/ecs/drag/state/mod.rs` の doc と述語・`reset_to_idle` の扱い・契約の決定論テスト 1 本・読み手（wintf 内 3 か所＋`crates/areka/src/menu/trigger.rs`）を述語へ寄せる。
- **Out**: ドラッグの挙動そのもの（閾値・捕捉・窓の移動）・透過制御の判定規則の変更・右ボタンや中ボタンのドラッグ。

## Boundary Candidates

- wintf のドラッグ状態の契約（doc・述語・テスト）
- 読み手の寄せ替え（wintf 内／areka の `menu/trigger.rs`）

## Out of Boundary

- `areka-P0-popup-menu-minimal` のメニューの振る舞い（是正済み・本 spec は読み方を述語へ置き換えるだけ）
- 透過制御（clickthrough）の要件の改訂

## Upstream / Downstream

- **Upstream**: なし（wintf 単独で閉じる）。`areka-P0-popup-menu-minimal` が main へ入った後に着手する（`menu/trigger.rs` が main に在ること）。
- **Downstream**: ドラッグの状態を今後読む全 spec（メニューのサブメニュー登記・D&D を扱う `areka-P0-ghost-install` など）。

## Existing Spec Touchpoints

- **Extends**: なし
- **Adjacent**: `areka-P0-popup-menu-minimal`（tasks.md 完了記録 9.3 に発見の経緯）・`areka-P0-ghost-install`（`WM_DROPFILES` はドラッグの状態を読まない見込みだが要確認）

## Constraints

- wintf は 32bit 可搬性の適用範囲外（x64＋arm64）。
- テストは実装と同じディレクトリの兄弟ファイル・1 ファイル 1,000 行以内。
- 規模の見立て: S（案 A なら doc＋述語 1 つ＋テスト 1 本＋読み手 4 か所）。

# Brief: wintf-winmsg-executor

> **種別**: 本坑（main・完成品）spec。wintf 基盤リファクタ。
> **go-gate**: `_Depends(confirmed): pilot/wintf-winmsg-executor`
> **状態**: READY（先進坑 `pilot/wintf-winmsg-executor` の go 判定取得済み＝開発者承認 2026-06-29・二坑モデル要件 6.1）。`/kiro-start wintf-winmsg-executor` で着手可。
> **roadmap 帰属**: M1 emo2-boot ユニットではない（wintf 基盤＝⑥render/window の土台）。M1専用 `roadmap.md` には載せない（更地化規律の保全）。go-gate traceability は本 brief ＋ 先進坑 README の双方向名指しで担保。

## Problem

wintf のメッセージループとウィンドウ起動は**ほぼ自作**である。Windows 低レベルレイヤーは罠が多く、自作コードは「開発者の思い付き」由来の不正確な挙動を抱えるリスクがある。また UI スレッド上での非同期コード実行が現状は `async-executor::Executor` ＋ 別 `bevy_tasks::TaskPool` ＋ `mpsc` の手組みで、利用上のトラブル余地が残る。wintf のコードベースがこれ以上膨らむ前に、洗練された基盤へ置き換えるべきと判断した。

## Current State

調査済み（subagent 探索・本 brief の正本ではなく地図）。要点：

- **メッセージポンプ**: `WinThreadMgr::run()`（[crates/wintf/src/win_thread_mgr.rs:202](../../../crates/wintf/src/win_thread_mgr.rs)）が `PeekMessageW` ＋ `TranslateMessage`/`DispatchMessageW` の自作ループ。
- **60Hz ECS tick 駆動**: 専用 **VSync スレッド**が `DwmFlush()`（≈16.67ms）→ `PostMessageW(WM_VSYNC)` をメッセージ専用ウィンドウへ送信 → メインスレッドが pop → `try_tick_on_vsync()` → 13本スケジュールの ECS tick を実行（`win_thread_mgr.rs:329` / `ecs/world/mod.rs:505`）。**＝ユーザ指摘の「起動メッセージを pop する」方式**。再入ガード `IS_TICK_FLUSH_IN_PROGRESS`（`ecs/world/vsync.rs`）。
- **ウィンドウ生成**: `process_singleton` が `GetModuleHandleW(None)` で HINSTANCE 取得、2クラス（legacy `wndproc` ／ `ecs_wndproc`）を `RegisterClassExW`。`create_window()` が `CreateWindowExW`、ハンドラを `lpParam`→`GWLP_USERDATA` に手詰め。
- **wndproc 2層**: legacy `wndproc`（`#[deprecated]`・Arc ハンドラ）／ `ecs_wndproc`（現役・`GWLP_USERDATA` に Entity ID を保存し World へ dispatch、`ecs/window_proc/`）。
- **UI スレッド async**: `async-executor::Executor`（同期 tick ベース）＋ `bevy_tasks::TaskPool`（背景スレッド）＋ `mpsc` を Input スケジュールで drain。
- **COM 初期化**: `CoInitializeEx(COINIT_MULTITHREADED)` を `WinThreadMgr::new()` で実施。

## Desired Outcome

wintf のメッセージループ・ウィンドウ起動・UI スレッド async が `wintf-winmsg-executor`（フォーク元 `winmsg-executor`・Windows 低レベル知見に基づく洗練版）ベースに置き換わり、自作ポンプ／`GWLP_USERDATA` 手詰め／別 executor の手組みが撤去され、挙動が現状以上に正しく・トラブル余地が縮小していること。emo2-boot を含む既存 examples が回帰なく動くこと。

## Approach

`wintf-winmsg-executor` v0.0.3（要バージョン pin）を採用し、3 層を写像する：

1. **メッセージポンプ** `WinThreadMgr::run()` → `block_on` / `MessageLoop::run(filter)`。`quit`/`quit_when_idle` で `PostQuitMessage` 経路を引き継ぐ。
2. **ウィンドウ生成** 自作 `create_window` ＋ `GWLP_USERDATA` 手詰め → `util::Window<S>`（状態 S とクロージャ wndproc `Fn(Pin<&S>, WindowMessage)->Option<LRESULT>` をライブラリが束ねる）。**`new_ex`** で `WS_EX_NOREDIRECTIONBITMAP` を渡せる＝DComp 経路に直結。`get_instance_handle`（次版で `pub util::get_instance_handle()` 化予定）。
3. **UI スレッド async** 手組み executor → `spawn_local` / `block_on`（tokio 非依存・futures は `Send`/`Sync` 不要）。背景重処理の `bevy_tasks::TaskPool` は別件として残置検討。

**60Hz ECS tick の起床（採用決定＝event_listener ブリッジ）**: DwmFlush 60Hz スレッドが `event_listener::Event` を notify → UI スレッドで `spawn_local` した async tick タスクが `listener.await` で起床 → 1 フレーム実行 → 再 await。C# の `TaskCompletionSource` 相当のスレッド跨ぎ起床を tokio 無しで実現。`event_listener` クレートを tech.md に追加する。

> 設計判断の正本は本坑 design 側に置き、先進坑の**検証結果**は先進坑 README を参照して二重化しない（二坑モデル要件 3.5）。

## Scope

- **In**:
  - wintf のメッセージポンプを `wintf-winmsg-executor` の `MessageLoop`/`block_on` へ置換。
  - ウィンドウ生成を `util::Window`（`new_ex`/`new_checked_ex`）へ移行。HINSTANCE/クラス登録の責務整理。
  - `ecs_wndproc` の Entity dispatch を新基盤の wndproc クロージャ上へ再構築（World ハンドル＋Entity を capture）。
  - UI スレッド async を `spawn_local`/`block_on` へ移行。
  - 60Hz ECS tick を event_listener ブリッジ＋ async tick タスクへ移行。`IS_TICK_FLUSH_IN_PROGRESS` 再入ガードと executor の nested-message 処理（`new_checked` の `RefCell`）の整合。
  - deprecated レガシー（`winproc` / `win_thread_mgr` / `win_message_handler`）の撤去（既に `#[deprecated]`）。
  - 既存 examples（emo2-boot 系含む）の回帰確認。
- **Out**:
  - 背景重処理用 `bevy_tasks::TaskPool` の廃止（必要なら別 spec。UI スレッド async とは別レイヤ）。
  - 透過合成方式（ULW/DComp 切替）のロジック自体の変更。`new_ex` の ex-style 受け渡し口を使うのみ。
  - ECS スケジュール（13本）の構成・順序の変更。
  - emo2 互換機能の新規実装（M1 emo2-boot ユニット側の領分）。

## Boundary Candidates

- **メッセージループ層**（pump＝`MessageLoop`/`block_on`、quit 経路）
- **ウィンドウ生成・wndproc 層**（`util::Window<S>`、クラス登録、Entity dispatch 結線）
- **UI スレッド async 層**（`spawn_local`/`block_on`）
- **60Hz tick 起床ブリッジ層**（event_listener ↔ async tick task ↔ ECS 再入ガード）

## Out of Boundary

- 背景スレッドプール（`bevy_tasks::TaskPool`）の設計。
- ECS world のスケジュール内容・描画ロジック。
- 透過・DComp/ULW の合成方式そのもの。

## Upstream / Downstream

- **Upstream**: `wintf-winmsg-executor` v0.0.3（crates.io・要 pin）、`event_listener` クレート、windows 0.62。先進坑 `pilot/wintf-winmsg-executor` の **go 判定**（前提依存）。
- **Downstream**: wintf 上の全 examples、M1 emo2-boot トラック ⑥render/window（UI スレッド固定・message pump の土台）。`areka-P0-window-placement` 等の窓生成系。

## Existing Spec Touchpoints

- **Extends**: なし（wintf 基盤の横断リファクタ。新規 spec）。
- **Adjacent**: M1 roadmap の ⑥render-engine トラック（`areka-P0-window-placement` / `areka-P0-surface-engine`）。これらが窓生成に `WinThreadMgr`/`create_window` を使うため、移行後 API へ追従が要る点に注意。

## Constraints

- Rust 2024・マルチクレート。32bit 可搬性を崩さない（host-32 は別プロセスゆえ本件 UI スレッドとは別系統）。
- **tokio 非依存**（採用決定）。UI スレッド async は `wintf-winmsg-executor`、スレッド跨ぎ起床は `event_listener`。
- 採用クレートは v0.0.3（極めて初期）。API 安定性リスクありゆえバージョン pin。`get_instance_handle` は現状非公開（次版で `pub` 化予定）＝当面は `util::Window::new_ex`/`new_checked_ex` 経由で回避。
- 二坑モデル: 本坑は先進坑 `pilot/wintf-winmsg-executor` の **go 判定（人間判断）**まで BLOCKED。先進坑コードのコピペ流用は禁止（README 知見を見てクリーンに掘り直す・要件 5.3）。

## 先進坑ゲート（go 判定の合否基準）

先進坑 `crates/pilot/examples/wintf-winmsg-executor/` が以下を検証し、開発者が go/違う/直す を判定する：

1. **tick 起床**: DwmFlush 60Hz スレッド → `event_listener::Event` notify → UI スレッドの `spawn_local` async タスクが `block_on`/`MessageLoop::run` 下で約 16.67ms 周期に安定起床し、フレーム処理を回せること。
2. **再入整合**: tick 中に発生するメッセージ（`SetWindowPos`→`WM_WINDOWPOSCHANGED` 等）で executor の nested-message 処理（`new_checked` `RefCell`）と ECS 再入ガードが衝突せず、デッドロック/二重 tick が起きないこと。
3. **窓＋wndproc**: `util::Window::new_ex` で `WS_EX_NOREDIRECTIONBITMAP` 付き窓を生成し、wndproc クロージャから共有状態（World 相当）へ安全にアクセスでき、`GWLP_USERDATA` 手詰め無しで Entity dispatch 相当が成立すること。
4. **終了経路**: `PostQuitMessage`/`quit_when_idle` で清掃終了し panic しないこと（`block_on` は loop 先行終了時 panic 仕様ゆえ要確認）。

合否は**開発者の人間判断**（二坑モデル要件 6.3）。**→ 2026-06-29 go 取得済み**（4 基準＋縦 3 窓
並行ストレスすべて PASS・可視動作を開発者が目視確認）。本坑 BLOCKED 解除。詳細は先進坑 README の
検証結果を参照（二重化しない・要件 3.5）。

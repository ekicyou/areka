# Brief: areka-P0-instant-to-interrupt-ticks

> 2026-10-15 `/kiro-discovery` で起票（開発者「低優先度の起票。dola の時刻精度を、f64 の秒から、QueryUnbiasedInterruptTimePrecise を起点とした 100ns の u64 に変更し、プロセス間で完全同期アニメーションを行うためのベースを作成する。関連する時刻情報について、すべて、QueryUnbiasedInterruptTimePrecise から取得することを前提とし、必要な spec を起票せよ」）。「時刻の土台」の 3 本の 3 本目（roadmap「テーマ別の決めごと」の「時刻の土台」）。

## Problem

アニメの時刻（dola）と台詞の時刻（ghost・kanade・seriko）を割り込み時刻へ寄せても、ほかの「今」はまだ `std::time::Instant`（中身は QPC）で読まれている。刻みを起こす締め切り・入力の時刻・吹き出しの表示の締め切りなどがそれで、アニメの時刻と同じ物差しで比べられない。新しいコードが `Instant::now()` を足せば、黙って 2 つ目の時計が増える。

## Current State

（2026-10-15・main `574bf0a6` の実測・テストを除く本番のコード）

- **wintf**:
  - 刻みの締め切り: `tick_wake::arm_deadline(Instant)`／`take(Instant)`（`crates/wintf/src/ecs/world/tick_wake.rs`）・`EcsWorld::decide_tick(now: Instant)`（`world/mod.rs`・`runtime/tick_bridge.rs`）。
  - 刻みの診断: `TickStart`・`tick_diag.rs`・`transition_diag.rs`。
  - 入力と UI: ポインタの標本の時刻（`ecs/pointer/types/mod.rs`・`window_proc/mouse_move.rs`）・ドラッグ（`drag/state/mod.rs`・`drag/dispatch.rs`）・ツールチップの遅れ（`ecs/tooltip/mod.rs`・`system.rs`）。
- **areka**: 吹き出しの表示の締め切り（`emo2_boot/balloon_visibility_phase.rs`）・メニューの引き金（`menu/trigger.rs`）・終了の待ち（`exit_wait.rs`）・計測とログの所要時間（`frame.rs`・`ghost_switch.rs`・`switch_assets.rs`・`mcp/dump_*.rs`・`perf_thread_report.rs`・`session_end.rs`・`main.rs`）。
- **そのほか**: `areka-emo-present` の `presenter/timing.rs`（所要時間）・`shiori-host32-host` のプロセスの待ちの締め切り（`job.rs`・`lifecycle.rs`・`parent_window.rs`・`process_host.rs`）・`areka-nar` の `install.rs`。
- 表示の周期の待ちは `DwmFlush()`（専用のスレッド）。DXGI／DirectComposition のフレームの統計・`WM_TIMER`・待ち可能なタイマーは使っていない。
- テストの決定論の口: `decide_tick_with(now)`・`tick_wake::take(now)`・`spine_wait::wait_until_with(Instant::now, sleep, …)`。
- 「今」を読む関数を絞る機械の番人は無い（ワークスペースに `clippy.toml` が無い）。

## Desired Outcome

- 本番のコードで「今」を読む所は、`dola-interrupt-ticks` の時計（`QueryUnbiasedInterruptTimePrecise`）だけ。締め切り・入力の時刻・所要時間が同じ物差しで、アニメの時刻と直接比べられる。
- 機械の番人が、本番のコードに `Instant::now`・`SystemTime::now`・`GetTickCount64`・`QueryPerformanceCounter` などの別の時計が入るのを赤で止める。暦の時刻が要る所（ログの日時・作業フォルダの名前など）だけは名指しで許す。
- 待つ API（`WaitForSingleObject` のミリ秒・`SendMessageTimeoutW` の時間切れなど）に渡す「長さ」はそのままでよい（「今」を読んでいないので）。

## Approach

推す形（要件の段で覆してよい）:

- 番人は clippy の `disallowed_methods`（`clippy.toml`）で書く。新しい道具を作らない。許す所は `#[allow]` に理由を添える。
- 締め切りの型を `Instant` から新しい時刻の型へ替える。`std::thread::sleep` などに渡す所でだけ `Duration` へ変える。
- テストのコード（待ちの上限の `Instant::now()` など）は番人の外にする（アプリの時刻ではない）。テストの偽の時計は今までどおり差し込む。
- 計測の道具（`perf-*`・`areka-impl-watch` など、アプリの外の道具）は範囲外。

**要件の段で決める議題**:

1. 所要時間のログ（計測と診断）も割り込み時刻へ寄せるか。寄せると、眠りを挟んだ所要時間から眠りの分が消える（計測としてはむしろ正しい）。
2. 32bit の補助プロセス（`shiori-host32-helper`／`host`）も同じ番人に入れるか。同じ関数は 32bit でも呼べる（Windows 10 から）。

## Scope

- **In**: wintf・areka・`areka-emo-present`・`shiori-host32-host`・`areka-nar` の本番のコードの「今」の読み取りを割り込み時刻の時計へ。締め切りの型の付け替え。clippy の番人と許す所の一覧。`tech.md`（時計は 1 本の決まり）。
- **Out**: dola の型と時計そのもの（`dola-interrupt-ticks`）。ghost の刻み・kanade・seriko の時計（`talk-anchor-interrupt-ticks`）。暦の時刻（`GetLocalTime`・tracing の日時・作業フォルダの名前）。刻みを起こす仕組みの変更（`tick-gate-adoption`）。アプリの外の道具。

## Boundary Candidates

- wintf の刻みの締め切りと入力の時刻（`tick_wake.rs`・`world/mod.rs`・`tick_bridge.rs`・ポインタ・ドラッグ・ツールチップ）。
- areka の締め切りと計測（`emo2_boot`・`menu`・`exit_wait.rs`・`mcp`）。
- 番人（`clippy.toml` と許す所）。

## Out of Boundary

- 待ちの振る舞い（いつ起きるか・何秒待つか）は変えない。時計を替えるだけ。
- 表示の周期（`DwmFlush`）の待ちは変えない。

## Upstream / Downstream

- **Upstream**: `dola-interrupt-ticks`（型と時計）。`talk-anchor-interrupt-ticks` が先に着地していれば、番人で `GetTickCount64` も止められる（後なら、番人の禁止の一覧に `GetTickCount64` を載せるのはその spec の着地の時）。
- **Downstream**: なし（以後、新しいコードは番人に従う）。

## Existing Spec Touchpoints

- **Extends**: なし（新しい境界）。
- **Adjacent**:
  - `tick-gate-adoption`（保留・区分 D）: 同じ `tick_wake.rs`・`tick_bridge.rs` の締め切りに触る＝同じ時に走らせない。
  - `present-emit-tail-latency`（区分 B）: `show.rs` まわりの所要時間の計測に触る＝同じ時に走らせない。
  - `exit_wait.rs`・`menu/trigger.rs`・ツールチップ・ドラッグを触る spec（`popup-menu-residue`・`shell-tooltip` など）とは、触るファイルを要件の段で照らす。

## Constraints

- 段は**据え置き**・区分 D（開発者の指定「低優先度」）。ウェーブへの組み入れは次の棚卸で決める。
- 時刻は正確に扱う。3 つ目の時計を作らない。外の crate は足さない。
- 1 ファイル 1,000 行の番人に触れない。
- 規模の見込み: M（8〜12 タスク）。
- 要件定義のモデル: Opus で足りる（付け替えが主・判断は番人の許す所の一覧と議題 2 つ）。

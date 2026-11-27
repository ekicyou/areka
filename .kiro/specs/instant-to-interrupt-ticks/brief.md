# Brief: instant-to-interrupt-ticks

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

## 2026-11-27 棚卸㊹の再測定（main ccd790fc）

- **前提の変化**: `dola-interrupt-ticks` が着地し、時刻の型と時計は dola の中の `dola::time`（`crates/dola/src/time/`・`point.rs`・`clock.rs`）に置かれた。dola の外へ小さい crate として切り出してはいない。ゴースト・kanade・seriko のミリ秒の時計もその時計へ替わり、本番のコードの `GetTickCount64` は 0 になった（`talk-anchor-interrupt-ticks` の範囲もその spec に入った）＝番人の禁止の一覧に `GetTickCount64` を最初から載せてよい。`Instant` の読み取りには手を付けていない（刻みの締め切り `tick_wake::arm_deadline` は今も `Instant` を受ける）。見直しの spec が、ポインタの時刻（`ecs/pointer/`・`mouse_move.rs`）、wintf の `com/wuc.rs`、32 ビットの補助の `shiori_proxy.rs` から `Instant` を消した。
- **触るファイル**（テストとテストの手助けを除く本番のもの・43 ファイル）: wintf 16＝`ecs/world/{tick_wake.rs, mod.rs 670, tick_diag.rs}`・`runtime/tick_bridge.rs`・`ecs/tooltip/{mod.rs, system.rs, turn.rs}`・`ecs/drag/{state/mod.rs, dispatch.rs, accumulator.rs, systems.rs}`・`ecs/window/{command.rs 945, transition_diag.rs}`・`ecs/window_proc/window_pos.rs`・`ecs/clickthrough/monitor.rs`・`ecs/graphics/systems/render.rs`。実行系 `crates/areka-runtime/src/` 6＝`balloon_visibility_phase.rs`・`frame.rs`・`frame/scale_text.rs`・`frame/switch.rs`・`skin_switch.rs`・`switch_assets.rs`。`crates/areka-session/src/` 4＝`exit_wait.rs`・`session_end.rs`・`ghost_session_runtime.rs`・`ghost_switch.rs`（970＝上限の近く）。本体 `crates/areka/src/` 8＝`main.rs`・`menu/trigger.rs`・`perf_thread_report.rs`・`file_drop.rs`・`install/worker.rs`・`mcp/{check_script.rs, dump_balloon.rs, dump_surface.rs}`。入力 `crates/areka-input/src/{lib.rs, char_watch.rs}`。そのほか各 1＝`areka-actor/src/reply.rs`・`areka-emo-present/src/presenter/timing.rs`・`areka-kanade/src/shiori/probe.rs`（SHIORI の応答の予算）・`areka-nar/src/install.rs`・`areka-sylphya/src/persist/io.rs`（書き込みの再試行の締め切り）・`areka-user-data/src/copy.rs`・`shiori-host32-host/src/parent_window.rs`。ほかに新しい `clippy.toml` と `tech.md`。`real-run-kit` はテストの道具なので番人の外。
- **依存の席**: 時計が dola の中にあるので、dola を引いていない `areka-actor`・`areka-emo-present`・`areka-input`・`areka-nar`・`areka-sylphya`・`areka-user-data`・`shiori-host32-host` に dola を足す（各 `Cargo.toml` と `Cargo.lock` が変わる＝1 ウェーブに 1 本だけの席を使う）。dola は serde・rand・interpolation を連れてくる。最下層の `areka-sylphya` と 32 ビットの補助の側に、これを持ち込んでよいか（時刻の型だけを小さい crate へ分けるか）は要件の段の議題。
- **規模**: 12〜16 タスク（crate が増えた分と依存の足し方の議題で上げる）。20 を超えないので切らない。
- **先に要るもの**: 働きの前提は無い（`dola-interrupt-ticks` は着地済み）。見直しと crate 分けは全部済んだ。
- **同じウェーブに置けない相手**: ウェーブ C28 の 6 本のうち 4 本と重なる＝`balloon-text-visual-tree`（`frame/scale_text.rs`・本体 `mcp/dump_balloon.rs`）・`ghost-switch-balloon-fallback`（`ghost_switch.rs`）・`talk-thread-count-growth`（`perf_thread_report.rs`）・`sylphya-sqlite-kvs`（`persist/io.rs` と依存の席）。番人は全部の「今」の読み取りを替えてはじめて入れられるので、触らない約束で狭めると形が崩れる＝C28 には入れず、後のウェーブで単独に近い形で走らせる。後の相手は `tick-gate-adoption`・`background-result-tick-wake`（`tick_wake.rs`・`world/mod.rs`・`tick_bridge.rs`）、`compose-f16-precision`（`mcp/dump_surface.rs`・`dump_balloon.rs`）、`menu/trigger.rs` を触る spec（`link-context-copy`・`menu-thumbnail-preview`）、`exit_wait.rs` の `install-live-target-hazards`、ツールチップの `shell-tooltip`・`mouse-hover-event`、ドラッグの `window-snap`。
- **優先度の区分**: D（開発者「低優先度」・据え置き）。
- **要件定義のモデル**: Opus（付け替えが主・型と時計は前例どおり。依存の足し方の議題は前の spec が決めた置き場をそのまま使うか分けるかで、骨組みの分かれ目ではない）。
- **分割の案**: 無し。
- **見つけた穴・古くなった記述**: Current State の一覧の `emo2_boot/` の道は全部 `crates/areka-runtime/src/` と `crates/areka-session/src/` へ移った。`ecs/pointer/types/mod.rs`・`mouse_move.rs`・`shiori-host32-host` の `job.rs`・`lifecycle.rs`・`process_host.rs`（今はテストの中だけ）・`install/{overwrite.rs, fetch_url.rs}`（`fetch_url.rs` は暦の `SystemTime` だけ）は付け替えの相手から外れた。Upstream の「`talk-anchor-interrupt-ticks` が先なら」は済んだ（`GetTickCount64` は今止めてよい）。

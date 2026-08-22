# Gap Analysis: areka-P0-draw-load-parity

- 作成日: 2026-08-22（要件確定コミット `717d89b0` 直後・現行ツリー＝`f6b81078` 系統＝Bevy 0.19.1 更新後）
- 目的: 確定済み requirements.md（Req 1〜9）と現行コードの差を洗い出し、設計フェーズの選択肢と調査項目を整理する
- 方針: 本書は判断材料を並べるものであり、最終決定は行わない。file:line は**すべて現行ツリーで読んで確認した**（brief の旧番号は採用していない）
- 表記: 決定論テスト＝実機無しで走るテスト。「テスト間の状態汚染」＝並列実行する別テストの書き換えが見えてしまう問題

---

## 1. 現況調査（Current State）

### 1.1 フレーム駆動の実形（Req 1・4 の土台）

| 要素 | 所在 | 確認した挙動 |
|---|---|---|
| vblank 検出スレッド | `crates/wintf/src/runtime/tick_bridge.rs:114-134` | `DwmFlush()` 待ち → `event.notify(usize::MAX)`。DWM 失敗時のみ 15ms sleep（:127-131） |
| UI スレッドの tick ループ | 同 `:218-236` `run_async_tick` | listen → `upgrade()` → await → `tick_one_frame`。**スキップ判断は無い** |
| 1 フレーム実行 | 同 `:187-210` `tick_one_frame` | 再入ガード → `try_borrow_mut` → `try_tick_world()` → `flush_window_pos_commands()`（:206） |
| 13 本の固定順実行 | `crates/wintf/src/ecs/world/mod.rs:488-566` `try_tick_world` | フレーム番号 +1（:517-524）・`TickStart`（:525-534）・`FrameTime`（:536-541）・ポインタ転送（:545）・`try_run_schedule` ×13（:548-560）・NCHITTEST キャッシュ消去（:563）。**早期脱出はシステム未登録時のみ**（:493） |
| 周期の注記 | `tick_bridge.rs:142-146` | 「固定周期ではない。実効のフレーム周期は画面の更新周期」 |
| 旧経路 | `ecs/world/vsync.rs:17-22`・`world/mod.rs:580-607` | `try_tick_on_vsync` は生産者撤去済みで常に tick しない。`window_proc/window_pos.rs:282-291` の `WM_WINDOWPOSCHANGED` ハンドラが ②で呼び ③で `flush_window_pos_commands()` を無条件に呼ぶ（:290） |
| 既存の観測 | `world/mod.rs:461-484` `measure_and_log_framerate` | 10 秒ごとに `trace!` で fps／平均フレーム時間（target は既定＝`wintf::ecs::world`）。スケジュール別の所要は**どこにも無い** |
| 既存テスト | `world/mod.rs:617-745` | 13 本の順序不変（:657）と 2 周の安定（:707）。`FrameCount` が 1 tick で +1 になることを前提にしたテストが他にもある（`layout/systems/monitor_systems_transition_tests.rs:377`・`window/transition_diag_tests.rs:788`） |

**実行器の構成（brief に無い新事実）**: ルート `Cargo.toml:48-56` が `bevy_ecs` に `multi_threaded` を有効化しているため、`Schedule::new` の既定実行器は多スレッドである（`bevy_ecs-0.19.1/src/schedule/executor/mod.rs:49-64`・`schedule.rs:405-411`）。`world/mod.rs:104-160` は **UISetup／GraphicsSetup／PreRenderSurface／RenderSurface／Composition／CommitComposition の 6 本だけ** `SingleThreadedExecutor`（:117,135,141,146,151,156）へ固定し、**Input／Update／PreLayout／Layout／PostLayout／Draw／FrameFinalize の 7 本は多スレッド実行器のまま**である。多スレッド実行器は `run` のたびに `ComputeTaskPool::get_or_init(TaskPool::default).scope_with_executor(...)` を開く（`executor/multi_threaded.rs:274`。システム 0 本なら :242-244 で即 return）。`TaskPool::default` のスレッド数は `available_parallelism`（`bevy_tasks-0.19.1/src/task_pool.rs:165-167`）＝測定機なら 22 本。別に `WintfTaskPool` が `TaskPool::new()` でもう 1 組持つ（`ecs/widget/bitmap_source/task_pool.rs:44`）。brief の「スレッド数 83」の大半はこの 2 組の待機スレッドで説明がつく可能性が高い（**Research Needed R-1**）。

⚠ この 7 本を単スレッドへ落とす案には**既存テストが 2 本、字面で反対している**: `monitor_systems_transition_tests.rs:362-369` と `transition_diag_tests.rs:774-781` は「本番の World 構築が `schedules.insert(Schedule::new(Update));` のままである」ことを `assert!` し、Update が既定の多スレッド実行器で走る前提を固定している（理由: ログ捕捉に依らない検証が空振りしていないことの保証）。さらに `areka/src/emo2_boot/frame_harness_tests.rs:397` はテスト用スケジュールが単スレッドであることを字面で固定する。実行器を変えるなら、これらのテストの**前提の書き換え**が要る（削除ではなく改訂）。

### 1.2 1 tick で毎回走るもの（変化が無いときの費用の所在）

`world/mod.rs` 登録分（:162-373）を関数定義で読み、変化検知（`Changed<>`／`Added<>`／`RemovedComponents`）で空振りするか、毎回本体が走るかを分類した。

| スケジュール | 実行器 | 毎回本体が走るもの（ゲート無し） | 変化検知で空振りするもの |
|---|---|---|---|
| Input | 多 | `drain_task_pool_commands`（チャネル try 読み）・`dispatch_pointer_events`・`dispatch_drag_events`・`cleanup_drag_state`・（debug のみ 2 本）＋ areka の `clear_balloon_hover_on_leave`（`input_events/balloon.rs:891-894`）・`drain_choice_selections`（`input_events/choice_drain.rs:128-131`） | — |
| Update | 多 | `detect_display_change_system`（フラグ確認のみ・`monitor_systems.rs:208-221`）・`update_monitor_layout_system`・`invalidate_dependent_components`（`GraphicsCore` 有効確認のみ）・`update_typewriters`（areka に該当 entity 無し＝空走） | — |
| PreLayout | 多 | `init_graphics_core` | — |
| Layout | 多 | `cleanup_removed_entities_system` | `build_taffy_styles_system`／`sync_taffy_tree_system`／`compute_taffy_layout_system`／`update_arrangements_system`（`taffy_systems.rs:34,57-61,121-123,199`） |
| PostLayout | 多 | — | 5 本すべて（`window_pos_systems.rs:21,133`・`arrangement_systems.rs:13,33,52`）。`propagate_global_arrangements` は仕事があるときだけ `ComputeTaskPool` の scope を開く（`common/tree_system.rs:130-145`） |
| UISetup | 単 | `create_windows` | `apply_window_pos_changes`（`Changed<WindowPos>`・`graphics/systems/window_pos.rs:31`） |
| GraphicsSetup | 単 | — | `init_window_graphics` |
| Draw | 多 | `resolve_inherited_brushes`（`With<BrushInherit>`＝マーカー除去後は空走）・typewriter 3 本（entity 無し） | `draw_rectangles`／`draw_labels`／`draw_bitmap_sources`／`generate_alpha_mask_system` |
| PreRenderSurface | 単 | `visual_hierarchy_sync_system`（**全 `VisualGraphics` を毎回走査**・`visual_sync.rs:25-70`） | 他 4 本（`visual_manager.rs:76,120`・`surface.rs:30,81,280`） |
| RenderSurface | 単 | — | `render_surface`（`Changed<SurfaceGraphicsDirty>`・`render.rs:25-34`） |
| Composition | 単 | — | 3 本（`visual_sync.rs:210`・`clip_sync.rs:30`） |
| CommitComposition | 単 | （システム 0 本・no-op・`world/mod.rs:341-344`） | — |
| FrameFinalize | 多 | `clear_transient_pointer_state`（全 `PointerState` を毎回書き換え・`pointer/systems.rs:17-31`）・`Messages::update` ×3（排他）・wintf `reconcile_window_registry`（`runtime/mod.rs:208-211`）・areka **`emo2_frame_system`**（`emo2_boot/mod.rs:465-467`）・areka `establish_owner_links`＋`apply_zorder_pair_maintenance`（`placement/spawn.rs:640-643`） | areka `register_ghost_windows_click_through`（`Added<WindowHandle>`・`placement/spawn.rs:681-684`） |

**FrameFinalize の中身は大半が areka 側である。** `emo2_frame_system`（`crates/areka/src/emo2_boot/frame.rs:158-233`）は毎フレーム、作業領域同期→attach→dpi→drain→バルーン可視性→窓寸照合→move drain→resnap→連鎖確定→連鎖再解決→文字層 k 追従→**文字層の提示**を順に駆動する。多くの相は冒頭で早期 return する（`frame/drain_resnap.rs:50-58,79-88`・`frame/attach.rs:158-161`・`frame/scale_text.rs:144-147`）が、次の 2 つは定常状態でも本体が走る:

- **文字層の提示 `present_frame`**（`frame/scale_text.rs:255-275` → `crates/areka-emo-text/src/actor.rs:596-640`）: talk の時刻起点が一度確立すると毎フレーム `present_actor`（:744-805）が**可視グリフ数の再計算・行レイアウト `LayoutEngine::layout_with_cursor_warn`・注釈・装飾・`executor.render`** を実行する。swap chain の `Present` は変化があったフレームだけ（doc :592-593）だが、レイアウト計算は毎回である。更新前実測の「FrameFinalize 182µs」の相当部分がここである可能性がある（**R-2**）。
- **バルーン可視性の観測**（`emo2_boot/balloon_visibility_phase.rs:64-95`）: 毎フレーム scope を列挙し観測を集める。

⚠ 境界: `areka-emo-text/src/actor.rs` と `emo2_boot/frame/*.rs` は要件の In-scope（「wintf の runtime／ecs::world」「上位スケジュールの内訳解析と、変化が無いときの是正」）に**名指しされていない**。内訳が areka 側に出た場合は Req 1.8／9.5 により担当の実在確認と再裁定が要る（**決定事項 D-4**）。

### 1.3 tick の外で周期的に動くもの（Req 1.8 の「最大項がフレーム駆動以外」の候補）

| 源 | 周期 | 所在 |
|---|---|---|
| vblank 検出スレッド `wintf-vsync` | 画面更新周期（120/s） | `tick_bridge.rs:65-68,114-134` |
| クリック透過の中継＋評価 | 120/s（vblank 中継）＋カーソル移動 | `runtime/mod.rs:307-328`（中継）・`clickthrough/controller.rs:416-457`（評価）・`controller.rs:145-215` `evaluate_targets`（差分適用・更新前実測 5.2µs） |
| カーソル監視ワーカ | 12ms（約 83/s） | `clickthrough/monitor.rs:34` `POLL_INTERVAL` |
| SERIKO ループ ticker → seriko アクター | 16ms（62.5/s） | `areka-ghost/src/ticker.rs:250-266,283-318`・結線 `emo2_boot/mod.rs:470-480`・受信 `areka-seriko/src/actor.rs:108,243` |
| dispatcher／kanade ticker | 50ms／1000ms | `ticker.rs:57-65,165-242` |
| bevy タスクプール 2 組（待機スレッド） | イベント駆動 | 上記 1.1 |

CPU は `invoke-perf-run.ps1` がプロセス単位で採る（`% Processor Time`）ので、これらは全部「areka の CPU」に入る。**スレッド別の帰属**は現状どの道具も出さない（**R-3**）。

### 1.4 catch-up（Req 3）の実形

- 発行元は 3 系統とも `crates/areka-ghost/src/ticker.rs`（dispatcher :203-206・kanade :223-226・loop :305-308）。`BoundarySchedule::poll`（:125-143）が「前回デッドラインから 2 境界以上進んだ」ときに `catch_up=true` を返す。
- ticker は **UI スレッドではなく自前のアクタースレッド**で `recv_timeout(remaining)` を回す（:188-199,293-302）。つまり catch-up は「ticker スレッドが 1 周期以上遅く起きた」ことを意味し、UI スレッドの負荷は**直接には**境界判定に効かない。効くとすれば CPU 競合による起床遅延（間接）である。Req 3.2 の因果検証はこの構造を前提に組むこと。
- ループ ticker の 16ms は Windows 既定のタイマ分解能（約 15.6ms）に近く、`recv_timeout` の起床遅れだけで 2 境界を跨ぎ得る（**R-4**：`timeBeginPeriod` の有無・実測分布の確認）。
- ログ行は `target = "dispatcher"` のように **`target` という名前のフィールド**で系統を名乗っている（`:204,224`。`target:` ではない）。`loop_ticker` は文言自体が違う。判定スクリプトは loop／ticker を文言で区別する（`judge-perf.py:676`）が、dispatcher／kanade の別は見ていない。Req 3.1 の系統別計数には `target=` フィールドの読み取り追加が要る。

### 1.5 一括 flush と `command.rs`（Req 4.5／4.6／7.4／7.6・同居裁定）

| 項目 | 所在（現行） |
|---|---|
| `SELF_INITIATED_DEPTH`（`AtomicI32`・プロセス共有） | `crates/wintf/src/ecs/window/command.rs:49` |
| テスト用の錠 `lock_self_initiated_for_test`（`#[cfg(test)]`） | `command.rs:76-79`。使用箇所 **14**（`command.rs:921,933`・`command_batch_tests.rs:322,402,466,542,637`・`command_transition_tests.rs:302,372,408,426`・`window_proc/window_pos_tests.rs:44,284,318`） |
| `is_self_initiated`／ガード | `command.rs:86-88`・`:94-111`・`guarded_set_window_pos` `:129-155` |
| バッチ投入 `apply_as_batch`（ガードはバッチ全体） | `command.rs:386-447` |
| 縮退 `apply_sequentially` | `command.rs:453-490` |
| 合流（`is_coalescible` 3 連言・仕切り・後勝ち） | `command.rs:501-518,527-542,548-576,588-615` |
| `enqueue`（合流後に観測） | `command.rs:657-679` |
| `flush`（前置ガード→1 バッチ→観測 3 種） | `command.rs:723-806`・便利関数 `:810-812`・テスト用取り出し `:829-831` |
| 既存テスト | `command_coalesce_tests.rs`（21 本）・`command_batch_tests.rs`（8）・`command_transition_tests.rs`（16）・`window_proc/window_pos_transition_tests.rs`（9）・`areka/src/emo2_boot/frame_transition_atomicity_tests.rs`（4） |

`SELF_INITIATED_DEPTH` のスレッド局所化は技術的には `thread_local! { static ...: Cell<i32> }` への置換 1 手で、`SetWindowPosGuard::new／drop`・`is_self_initiated` の 3 箇所が読み書きの全てである。`EndDeferWindowPos` も呼出スレッド上で同期送達するので意味論は保たれる。着地後は錠 14 箇所が退役対象になる（cage が rebase で受ける裁定）。**判断点は「錠の退役までを本 spec で行うか、錠は残して cage に任せるか」**（**D-9**）。

### 1.6 見た目の追随（Req 5）が依存している前提

- `runtime/mod.rs:229-236` が二重起床の理由 (b)「VSync tick 毎の再評価」を記し、`:307-328` の中継タスクが vblank ごとに `click_wake` を叩く（`:312-323`）。評価ループ（`controller.rs:416-457`）は **tick とは独立に** vblank 起床で settled World を読む。**tick を省略しても中継と評価は変わらず動く**ので、Req 5.1／5.2 は「中継を触らない」で満たせる可能性が高い（αマスクが変わるのは tick が回ったときだけなので、省略フレームで再評価しても結果は同じ）。
- ドラッグ（Req 5.3）: `dispatch_drag_events`／`WindowDragging`・`apply_window_pos_changes`（`Changed<WindowPos>`）・`WM_WINDOWPOSCHANGED` 再入 flush。ドラッグ中は入力があるので「変化あり」に自然に入る。
- DPI 遷移（Req 5.4）: `frame_transition_atomicity_tests.rs`（4 本）と `dpi-transition-atomicity` の決定論 8 遷移は `FrameHarness`（`frame_test_support.rs:707-716` `advance_frame` が `FrameCount`／`TickStart` を tick と同じ規律で進める）で回る。tick のスキップ判断を `try_tick_world` の外（`tick_one_frame`）に置けば、これらのハーネスには影響しない。

### 1.7 観測・測定の道具（Req 1・6・8）

| 道具 | 現況 |
|---|---|
| `wintf::transition` 観測チャネル | `transition_diag.rs:54`（target）・`:624-627` `is_enabled`（`tracing::enabled!` の前置ガード）・`:633-635` `emit_line`・`KIND_ALL` :81・`win_kind` :167。フィールド名の重複禁止は `transition_diag_tests.rs:386` `no_line_repeats_a_field_name` |
| `perf(apply_show)` 行 | `areka-emo-present/src/presenter/timing.rs:56`（文言）・`:201-222`（`debug!`・末尾 `frame`） |
| 判定スクリプト | `tools/perf/judge-perf.py`（`SCRIPT_VERSION` :106・`WARMUP_EXCLUDE_SEC` :364・`IDLE_CPU_MAX_RELEASE_PCT` :380・`LONG_RUN_MIN_SPAN_SEC` :396・`parse_fields` :588・`J_CATCHUP_*` :451-452・`J_REQUIRED_LOG_KINDS` :466・`judge_catchup` :2257・`judge_idle_cpu` :2521） |
| 自己較正 | `tools/perf/fixtures/` 17 ケース＋`generate.py`。各 `case.txt` に `mode`／`exit`／`note` |
| 採取ランナー | `tools/perf/invoke-perf-run.ps1:102-105`（7 分＝420,000ms・25 分＝1,500,000ms・**`RUST_LOG_VALUE = 'info,areka_emo_present=debug'`**）。新設する tick 観測を点灯させるにはこの値の改訂（＝採取側較正値の版上げ）が要る |
| README | `tools/perf/README.md` §3（2 水準）・§8（較正値の所在）・§11「数字の較正について、まだ埋まっていない穴」(:487-498) に **SSP の採取方法が未記録**と明記。SSP 配置手順はリポジトリのどこにも無い（`emo2-perf` の語は本 spec の brief／requirements にしか現れない） |
| COMPAT 台帳 | `doc/COMPAT_ARCHITECTURE.md` に性能・フレームレート・アイドル CPU の項目は無い（見出し §7「未決・要設計」:113／§8「沈黙ルール対応表」:122 が登記先の候補） |
| ビルド設定 | `Cargo.toml:96-102` release は `opt-level='z'`・`lto=true`・`codegen-units=1`（先行 spec の裁定どおり） |
| ukadoc | `descript_shell_surfaces` の `scaling` に「ウインドウの拡大縮小表示はコストがかなり大きいので濫用しないこと」の注記あり（MCP で確認） |

---

## 2. 要件ごとの充足状況と不足（Requirement-to-Asset Map）

| Req | 既存資産 | 不足（Missing）／未知（Unknown）／制約（Constraint） |
|---|---|---|
| 1.1〜1.4 ベースライン再計測 | 採取ランナー・判定スクリプト・fps の 10 秒ログ | **Missing**: tick 1 回の所要・回数/秒・13 本の内訳・「変化なし tick」割合・クリック透過評価回数の計測行。**Unknown**: 壁時計と CPU 時間の分離手段（1.3）。**Constraint**: 既定 OFF・費用 0 の前置ガード（4.8／8.4） |
| 1.5〜1.7 測定規律 | README §2／§3・`-ConfirmQuiet` | 交互取得（1.7）の手順は README に無い（Missing・文書） |
| 1.8 最大項が tick 外 | — | 1.3 の tick 外周期群の帰属が測れない（R-3） |
| 2.1 SSP 描画解像度 | — | **Unknown**: 実測方法（R-5）。**Missing**: SSP 配置手順（8.1） |
| 2.2〜2.6 目標の置き方 | `IDLE_CPU_MAX_RELEASE_PCT = 3.0` | 裁定待ち。(B) なら正規化式・SSP 実測値・採取日を較正値バナーへ（judge-perf.py 冒頭）。COMPAT への登記先が無い（Missing） |
| 3.1〜3.4 catch-up | ticker 3 系統・判定式⑵ | **Missing**: dispatcher／kanade の別（`target=` フィールド読取）・時刻突合の形。**Unknown**: ticker 起床遅延の主因（R-4） |
| 4.1〜4.3 変化なし tick の省略 | `Changed<>` ゲート（1.2 表）・`transition_diag` の前置ガード作法 | **Missing**: 「変化の有無」を判定する仕組み（世界全体の変化フラグは bevy に無い）・判定入力の列挙・省略経路。**Constraint**: `FrameCount`／`FrameTime`／`TickStart` の進め方（省略フレームで進めるか） |
| 4.4 順序不変 | `tick_order_tests`（:657,:707） | 省略経路でも 13 本の順序検査が通る形にする（7.3） |
| 4.5／4.6 Z 指令・未強制の前提 | 合流 3 連言・21 本のテスト | 4.6 の文書化先が未定（Missing・文書） |
| 4.7 大改造の裁定 | — | 実行器変更（1.1 ⚠）を「大改造」と見なすかは裁定事項（D-1） |
| 4.8／8.3／8.4 観測の費用 0 | `transition_diag::is_enabled` の作法 | 新 target の名前・行の語彙・集約周期が未定（D-5） |
| 5.1〜5.7 見た目の追随 | 中継と評価の独立（1.6）・既存テスト群 | 5.7 のサインオフ手順（自動終了＋ログ照合）が未設計 |
| 6.1〜6.9 機械判定 | judge-perf.py 0.3.2・fixture 17 | 6.5（発話の頂の説明可能比）の較正値が無い。6.9 の `WARMUP_EXCLUDE_SEC` 見直しは根拠欄と fixture 追加が要る |
| 7.1〜7.8 決定論テスト | `EcsWorld::new()` headless tick（`world/mod.rs:617-745` の型）・`FrameHarness` | 判定の純関数化が前提（Missing）。7.6 は 1.5 の置換＋錠退役の可否 |
| 8.1〜8.6 常設化 | README・fixture 台帳 | SSP 手順・新観測行の README 節・交互取得手順（Missing） |
| 9.1〜9.6 境界 | roadmap 追記(81)・cage／pwc／bod brief | 申し送り先の実在: `areka-P0-test-cage-determinism`・`areka-P0-present-write-coherence`・`areka-P0-emo2-conformance-e2e` はいずれも `.kiro/specs/` 直下に実在（確認済み） |

---

## 3. 実装アプローチの選択肢

### 3.1 「変化が無いときにフレーム駆動が仕事をしない」（Req 4）の実現形

bevy には「World 全体で何か変わったか」を O(1) で引く口が無い（変化検知はシステムのクエリ単位）。よって判定の入力は**生産者側が明示する**か、**既存の資源・キューの状態から組む**かのどちらかになる。

| 案 | 内容 | 触る場所 | 利点 | 難点 |
|---|---|---|---|---|
| **A. tick 入口の門**（`tick_one_frame`／`try_tick_world` の手前で `should_run(inputs)` を評価） | 入力＝ポインタ転送バッファの有無・窓書込キューの有無・ドラッグ中・アニメ境界の跨ぎ（seriko→presenter の指令到着）・表示 1 コマ適用・DPI／作業領域変更・Z 順要求・`WM_*` 受理の有無。偽なら 13 本を回さない | `tick_bridge.rs:187-210`・`world/mod.rs:488-566`＋生産者（wndproc・presenter 配送・ticker 配送）に「変化あり」を立てる 1 行ずつ | 上限が最大（変化なし tick の費用を「門の評価＋vblank 起床」まで落とせる）・既存システムの中身は不変・順序不変（7.3）が保ちやすい | 判定漏れの危険（4.3「疑わしいときは回す」）。生産者が wintf の外（areka／emo-text／seriko）にも居るので依存方向を崩さずに旗を立てる口の設計が要る。`FrameCount`／`FrameTime` の扱い（D-2） |
| **B. スケジュール単位の省略**（13 本それぞれに「回す条件」を持たせ、偽なら `try_run_schedule` しない） | 例: Draw／PreRenderSurface／RenderSurface／Composition は「描画系の変化検知が 1 件でも真」のときだけ | `world/mod.rs:548-560` の周り＋条件の供給源 | A より細かく、部分的に回せる（Input だけ毎回など） | 条件の供給源が結局 A と同じ。13 本の順序不変テストが「全部回った」ことを数えている（:694-702）ので期待値の再定義が要る |
| **C. 実行器の見直し**（多スレッド 7 本を単スレッドへ） | 多スレッド実行器の `run` ごとのタスクプール scope 費用を消す | `world/mod.rs:108-112,138,159`（7 箇所）＋前提テスト 2 本の改訂（1.1 ⚠） | A／B と独立に効く（変化あり tick も速くなる）。既存の意味論（UI スレッド固定）を強める方向 | 「大改造」か否かの裁定（4.7）。多スレッドで走っていた `propagate_global_arrangements` の並列伝播はそのまま（別経路）。効果量は測るまで不明（R-1） |
| **D. 起床の間引き**（変化なしが N 回続いたら vblank を K 回に 1 回だけ拾う） | `run_async_tick` で listener を K 回 await | `tick_bridge.rs:218-236` | 門の評価すら減る | 反映の遅れが最大 K フレームになり 4.2（1 画面更新周期以内）と衝突。クリック透過の中継（vblank 直結）は影響を受けないが、変化を拾う側が遅れる。**4.2 を守るには A／B で門を 120/s に置いたまま**が安全 |
| **E. 内訳の個別是正**（1.2 で毎回走るもの） | 文字層の提示でレイアウト計算を「可視グリフ数・入力が前回と同じなら省略」・`visual_hierarchy_sync_system` の全走査を `Added`／`Changed` へ・`clear_transient_pointer_state` の毎回書込を条件付きへ | `areka-emo-text/src/actor.rs:744-805`（**境界要確認**）・`graphics/systems/visual_sync.rs:25-70`・`pointer/systems.rs:17-31` | 変化あり tick でも効く。A と組み合わせ可 | `actor.rs` は In-scope に名指しが無い（D-4）。個別是正の積み上げだけでは「桁で小さく」（4.1）に届かない見込み |

**組み合わせの見立て**: A（門）＋C（実行器）を軸に、E は境界の裁定次第。B は A の粒度調整として後から足せる。D は 4.2 と両立しにくいので採らない方向の材料が多い。

### 3.2 「変化の有無」の判定入力（案 A／B 共通）

| 入力 | 取り方の候補 | 所在 |
|---|---|---|
| 入力（ポインタ・ドラッグ） | wndproc 側バッファが空か／`WindowDragging` が 1 つでも在るか | `pointer/buffers.rs`（`transfer_buffers_to_world`）・`drag/` |
| 窓ジオメトリ・Z 順 | 窓書込キュー非空／`ReassertZOrder` 要求の有無／`WM_WINDOWPOSCHANGED`・`WM_DPICHANGED`・`WM_DISPLAYCHANGE` 受理 | `command.rs:224-227`（キュー）・`zorder_pair_maintain.rs`・`window_proc/` |
| 表示 1 コマ・アニメ境界 | presenter への配送チャネルに指令が届いた（`wiring.rx`／`move_rx`／`lifecycle_rx`）・emo-text の cue 到着 | `emo2_boot/frame/drain_resnap.rs:50-58,79-88`・`balloon_visibility_phase.rs`（`lifecycle_rx`） |
| DPI・作業領域 | `App::display_configuration_changed()`・`Changed<DPI>`・モニタ表の差 | `monitor_systems.rs:208-221`・`emo2_boot/frame/work_area_sync` |
| 文字層の進行（タイプライタ） | talk の時刻起点が確立し、可視グリフ数が変わり得る間は「変化あり」と扱う（安全側） | `scale_text.rs:255-275`・`actor.rs:744-748` |
| GPU 側・デバイスロスト | `GraphicsCore::is_valid()` が偽 | `graphics/systems/window_pos.rs:132-150` |

どの入力も「wintf の外の生産者」を含むため、**wintf が旗の置き場（資源または thread_local の 1 ビット）を提供し、生産者が立て、tick 入口が読んで倒す**形が依存方向（wintf ← areka）を保つ。旗を立て忘れた生産者は 4.3 の「判定漏れ」になるので、7.1 の全組合せテストに「旗を立てる生産者の一覧」の構造検査を添える案がある（`include_str!` で字面を見張る既存の流儀・`frame_harness_tests.rs:397` など）。

### 3.3 ベースライン計測（Req 1・8.3・8.4）の取り方

| 案 | 内容 | 長所 | 短所 |
|---|---|---|---|
| **M-1. tick 内訳の観測行**（新 target・既定 OFF） | `try_tick_world` の各 `try_run_schedule` を `Instant` で挟み、**N tick ごとに集約 1 行**（合計・最大・13 本の和）を出す。`tracing::enabled!` の前置ガードを `transition_diag::is_enabled` と同じ作法で置く。フィールドは `frame` を先頭に、名前重複なし・末尾追加のみ | 実機で同じ走行から tick・apply・catch-up を突合できる（1.2／3.1）。判定スクリプトの `parse_fields` で読める | 120 行/秒を素で出すとログ自体が負荷＝集約が必須。`RUST_LOG_VALUE` の改訂（採取側較正値）と judge-perf.py の読み口追加・fixture 追加（8.5） |
| **M-2. CPU 時間の分離**（1.3） | UI スレッドの `QueryThreadCycleTime`／`GetThreadTimes` を集約窓ごとに併記 | 壁時計との差で待ち時間が見える | `GetThreadTimes` は 100ns 刻みだが更新粒度が粗い（集約窓でしか意味を持たない）。`QueryThreadCycleTime` はサイクル数（周波数で割る） |
| **M-3. スレッド別 CPU の帰属**（1.8 の切り分け） | 採取ランナー終了時に `Get-Process` のスレッド `TotalProcessorTime` を run-meta に書く／または ETW（`wpr`）を別途 | tick 外の周期群（1.3）を切り分けられる | スレッド名が取れない（ID と開始時刻のみ）。ETW は道具の導入コストが大きい |
| **M-4. 既存の 10 秒 fps ログ** | `world/mod.rs:461-484`（trace） | 追加コード 0 | tick 回数/秒しか出ない。内訳は出ない |

### 3.4 SSP 比較（Req 2・8.1・8.2）

- SSP の実描画解像度（2.1）の確定手段候補: (a) 200% 設定の SSP 窓を画面取り込みし、バルーン文字の縁を 100% 表示と比べる（引き伸ばしなら 2×2 の同色ブロックまたはぼかしが出る）／(b) SSP 側の設定項目（拡大率の描画方式）の有無を確認／(c) ukadoc の `scaling` 注記（コストが大きい）を参考値とする。いずれも**実測の手順を README に書く**ことが 8.1 の要件（**R-5**）。
- 配置手順は現状どこにも残っていない（1.7）。SSP 側の採取は `invoke-perf-run.ps1` が areka 実行体前提（`-GhostRoot` 検証・`AREKA_APP_SMOKE_EXIT_MS`）なので、SSP には**別の採取口**（同じカウンタ名・同じ間隔・同じ長さの CSV を出す小スクリプト、またはランナーへの `-TargetProcess` 追加）が要る（**D-8**）。

### 3.5 Effort／Risk

| 区分 | 見積り | 根拠 |
|---|---|---|
| 計測基盤（M-1〜M-3・README・fixture） | **M**（3〜5 日） | 既存の観測作法（`transition_diag`）を写せる。実機走行は 7 分×交互＋25 分×2 の待ち時間が支配的 |
| 是正（A＋C を軸） | **M〜L**（5〜8 日） | 門の判定入力が wintf 外に跨る。実行器変更は既存テスト 2 本の改訂を伴う。効果は測るまで不明 |
| `command.rs` の 1 行＋錠の扱い | **S**（0.5〜1 日） | 置換 1 手。錠 14 箇所の退役まで含めると +0.5 日 |
| 全体 | **L** | 測定→是正→再測の往復と開発者裁定（Req 2・4.7）が直列に入る |
| Risk | **Medium〜High** | wintf 中核の駆動に手を入れる。判定漏れは見た目の遅れとして出る（4.3）。SSP 比較の前提が未確定のまま目標が動く |

---

## 4. 設計フェーズへ持ち越す判断事項（決定事項 D）と調査事項（R）

### 決定事項（要件ディスカッションの議題候補）

1. **D-1 実行器**: 多スレッド 7 本（Input／Update／PreLayout／Layout／PostLayout／Draw／FrameFinalize）を単スレッドへ寄せる案を「大改造」（4.7）と見るか。採る場合、前提テスト 2 本（`monitor_systems_transition_tests.rs:362-369`・`transition_diag_tests.rs:774-781`）の改訂方針。
2. **D-2 省略フレームの番号と時刻**: 省略した tick で `FrameCount`／`FrameTime`／`TickStart` を進めるか。`transition_diag` の刻印・`perf` 行末尾の `frame`・`FrameHarness::advance_frame` が同じ規律を共有しているので、どちらに決めても全員へ波及する。
3. **D-3 省略時の flush**: 省略フレームでも `flush_window_pos_commands()` を呼ぶか（空なら `trace!` 1 行で安い）。wndproc 経路（`window_pos.rs:290`）は自前で flush するので必須ではないが、4.5 の「Z 指令の適用順不変」を崩さない最小は「常に呼ぶ」。
4. **D-4 FrameFinalize の中身が areka 側だったときの境界**（**✅ 要件ディスカッション議題 1 で解決＝調査は無制限・担当者不在の場所は本 spec で是正可・requirements 改訂欄参照**）: `emo2_frame_system` の文字層提示（`areka-emo-text/src/actor.rs:744-805`）が上位項に出た場合、本 spec で触るか（9.5 の担当確認→即報告）。`frame.rs` は atom（完了）の産物、`show.rs` は pwc、`placement/follow` は bod の所有。
5. **D-5 観測行の形**: 新 target 名（例 `wintf::tick`）・集約の周期（N tick または 1 秒）・フィールド語彙（`frame` 先頭・`kind=tick`・13 本の名前）・`RUST_LOG_VALUE` の改訂と `SCRIPT_VERSION` の版上げ・judge-perf.py では必須種（`J_REQUIRED_LOG_KINDS`）にせず任意種として読む（既存 fixture を判定不能にしないため）。
6. **D-6 CPU 時間の測り方**（1.3）: `GetThreadTimes`／`QueryThreadCycleTime`／ETW のどれで「壁時計」と「CPU」を分けるか。スレッド別の帰属（1.8）をランナー側に足すか。
7. **D-7 アイドル時の起床方針**: 120/s の vblank 起床は維持して門で落とす（4.2 を守る）か、起床そのものを間引く（4.2 と衝突）か。
8. **D-8 SSP 側の採取口**（議題 2 の裁定で**優先度低**＝再採取は要件外。手順の文書化のみ）: ランナーにプロセス指定を足すか、別スクリプトか。配置先・削除の記録（8.2）の形。
9. **D-9 `SELF_INITIATED_DEPTH`**: `Cell<i32>` 化と同時に錠 14 箇所を退役させるか、錠は残して cage へ渡すか（同居裁定は「着地形を cage へ申し送る」まで）。
10. **D-10 目標の置き方**（**✅ 要件ディスカッション議題 2 で解決＝CPU 絶対値 3.0% 未満・SSP の描画方式は調べない・SSP 再採取は要件外。requirements 改訂欄参照**）（旧 Req 2.2）: (A) 絶対値 3.0% ／ (B) 画素あたり。SSP 解像度の実測結果が出るまで保留。(B) の場合の正規化式の置き場（judge-perf.py バナー）と COMPAT の登記先（§7 か §8 か新節か）。
11. **D-11 catch-up の系統別計数**（3.1）: `target=` フィールドを判定スクリプトで読む改訂を本 spec で行うか。判定式⑵ 自体は変えない。
12. **D-12 4.6 の文書化先**: 「ウィンドウプロシージャ側が Z 指令を積まない」前提をどこに書くか（`command.rs` の module doc・`window_pos.rs`・design.md）。

**要件ディスカッション（2026-08-22）での分類**: 上の 12 件のうち **D-4（FrameFinalize の中身が areka 側だったときの境界）と D-10（目標の置き方）は開発者裁定の議題**として要件ディスカッションで扱い、結果は requirements.md へ反映する。**残り 10 件（D-1・D-2・D-3・D-5・D-6・D-7・D-8・D-9・D-11・D-12）は how の判断であり設計フェーズ（`/kiro-spec-design`）で解決する**。補足: D-1（実行器）は R-1 の効果量が出るまで採否を決めず、採る場合は requirements 4.7 の関門（大改造なら開発者裁定）を通す。D-7 は requirements 4.2（反映の遅れは 1 画面更新周期以内）が既に起床間引き（案 D）を排する向きに効いている。

### 調査事項（Research Needed）

- **R-1** 多スレッド実行器の `run` 1 回あたりの固定費（タスクプール scope・起床）と、単スレッドへ落とした場合の差。`ComputeTaskPool` のスレッド数（22）と `WintfTaskPool` の重複がスレッド数 83 をどこまで説明するか。
- **R-2** 更新後ベースラインで FrameFinalize／Draw の内訳を areka 側（`emo2_frame_system` 各相・文字層レイアウト）と wintf 側に分けて測る。
- **R-3** tick 外の周期群（vsync スレッド・カーソル監視 12ms・ループ ticker 16ms・dispatcher 50ms）のスレッド別 CPU。
- **R-4** catch-up の主因: ticker スレッドの起床遅延分布（Windows タイマ分解能・`timeBeginPeriod` の有無）と、UI スレッド負荷との時間的重なり（3.2）。
- ~~**R-5** SSP の 200% 表示が 100% 描画の引き伸ばしかどうかの確定手段と、SSP の CPU 採取を同一物差しで行う手順。~~（**議題 2 の裁定で不要**＝SSP の描画方式は調べない・再採取は要件外。README には 08-15 の参考値と採取条件を登記するのみ）
- **R-6** bevy 0.19.1 で `Schedule::run` をスキップした場合の `Messages<T>::update`（FrameFinalize の排他システム 3 本）や `RemovedComponents` の取りこぼし——省略フレームを挟んでも次に回った tick で `Changed`／`Removed` が拾えるか（bevy の変化ティックは World 単位で進むので拾える見込みだが、`RemovedComponents` は 2 回の `update` で消える点を確認）。

---

## 5. 推奨（設計フェーズへの申し送り）

- **まず測る**（Req 1）: M-1（tick 内訳の集約観測行・既定 OFF）＋M-2（UI スレッド CPU 時間）を最小で入れ、release／dev を 25 分で採る。内訳が wintf 側か areka 側かで D-4 の要否が決まる。
- **是正の軸は A（tick 入口の門）＋C（実行器）**。D（起床間引き）は 4.2 と衝突する材料が多い。E は内訳の結果次第。
- **判定の入力は純関数 `should_run(inputs) -> Decision` に閉じ、7.1 の全組合せテストで固定**。旗を立てる生産者の一覧は字面検査で見張る。
- **`command.rs` の 1 行は flush に触るついでに実施し、錠の退役可否を cage へ申し送る**（D-9）。
- **SSP 側は 2026-08-15 の参考値を登記するのみ**（議題 2 の裁定）。再採取は要件外。目標は CPU 絶対値 3.0% 未満。

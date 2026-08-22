# Gap Analysis: areka-P0-draw-load-parity

- 作成日: 2026-08-22（要件確定コミット `717d89b0` 直後・現行ツリー＝`f6b81078` 系統＝Bevy 0.19.1 更新後）
- 目的: 確定済み requirements.md（Req 1〜9）と現行コードの差を洗い出し、設計フェーズの選択肢と調査項目を整理する
- 方針: 本書は判断材料を並べるものであり、最終決定は行わない。file:line は**すべて現行ツリーで読んで確認した**（brief の旧番号は採用していない）
- 表記: 決定論テスト＝実機無しで走るテスト。「テスト間の状態汚染」＝並列実行する別テストの書き換えが見えてしまう問題


> **📌 2026-08-22 要件改訂後の読み替え**: 開発者指示で spec の目的が「自走改善ループの仕組みを作って回す」へ再定義され、requirements.md の番号が振り直された（新 1＝ループ・新 2＝計測手段・新 3＝是正の規則〔旧 4〕・新 4＝見た目〔旧 5〕・新 5＝目標と判定〔旧 2＋6〕・新 6＝テスト〔旧 7〕・新 7＝常設化〔旧 8〕・新 8＝境界〔旧 9〕）。本書の §1〜§4 に出る「Req N.M」は**旧番号**である。新旧の対応は requirements.md の改訂欄を正とし、下の §6 が新要件 1・2 に対する補足分析である。

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

---

## 6. 自走改善ループと「重い場所の突き止め方」（2026-08-22 要件改訂＝新 Requirement 1・2 の補足分析）

### 6.1 現状に無いもの

| 新要件 | 現況 | 不足 |
|---|---|---|
| 1.1 目標定義ファイル | 判定スクリプトの較正値（`judge-perf.py` 冒頭）と README に散在 | 1 ファイルに集約した目標定義（判定式・閾値・測定水準・スクリプト版）が無い |
| 1.2／1.6 ループの起動口 `/goal` | 開発者指示＝**Claude Code の組み込みスキルとして存在する**（本セッションのスキル一覧・`ListSkills`・`SearchSkills`・npm 配下の検索には現れず、受け取り方は未確認）。`/loop` は間隔実行のみで目標判定を持たない | `/goal` に渡す目標文と、1 周「計測→解析→実装→テスト→再計測→採否→記録」を 1 コマンドで回す道具・停止条件の判定。`/goal` は新設しない |
| 1.3 台帳 | 先行 spec は `remeasure-YYYY-MM-DD.md` を手書き | 周ごとの追記形式（機械で追記できる表）が無い |
| 2.1〜2.4 4 段の帰属 | プロセス全体のみ（`invoke-perf-run.ps1` の `% Processor Time`） | スレッド別・関数別・相別のどれも無い |
| 2.4 CPU サンプリング | **測定マシンに Windows Performance Toolkit が実在**（`C:\Windows\System32\wpr.exe`・`C:\Program Files (x86)\Windows Kits\Windows Performance Toolkit\{wpr,xperf}.exe`・2026-08-22 `where.exe` で確認） | 採取→記号解決→上位スタック一覧をテキストで出す 1 コマンド化。release に PDB が無い（`Cargo.toml:96-102` に `debug` 指定無し・`target/release/*.pdb` 不在）＝**`CARGO_PROFILE_RELEASE_DEBUG=line-tables-only`（環境変数）でビルド**すれば `Cargo.toml` 非接触で関数名が引ける。`lto=true`・`opt-level='z'` のインライン化でスタックが浅くなる点は読み方の注意として README へ |
| 2.8 静寂確認の自動化 | `-ConfirmQuiet` は**人の確認**を前提にしたスイッチ | 採取前後の全体 CPU・既知プロセス（`cargo`・`rustc`・`python`・別 `areka.exe`・`claude`）の有無を記録し、閾値超で再採取する小スクリプト |
| 4.7 見た目の追随をエージェント自身が確認 | 先行 spec の実機サインオフは開発者立会い＋ログ grep | 有界 auto-exit（`AREKA_APP_SMOKE_EXIT_MS`）＋ログ照合の判定をスクリプト化し、クリック透過は別プロセス窓へのクリック結果をログで読む形に |

### 6.2 ループ 1 周の実現形（候補）

| 段 | 候補 | 備考 |
|---|---|---|
| ⒜ 計測 | 既存ランナー（7 分・交互）＋同時に ETW サンプリング（`wpr -start CPU` 相当の軽量プロファイル・または `xperf -on PROC_THREAD+LOADER+PROFILE -stackwalk Profile`）＋tick 内訳観測行（既定 OFF を `RUST_LOG` で点灯） | サンプリング自体の費用（約 1kHz の割込）が対象の CPU を押し上げるので、**合否の採取とプロファイル採取は別走行**にする（合否は素の走行・順位付けはプロファイル走行） |
| ⒝ 解析 | ETL → `xperf -i ... -o ... -a`（または `wpa` のエクスポート／`tracerpt`）→ 自前 Python で「スレッド別 %・上位スタック（自己時間・包含時間）・モジュール別」を順位表に。tick 観測行は `judge-perf.py` の `parse_fields` で読み相別の順位表 | 出力は決定論的フォーマット（新 Req 2.10）。既知ログの fixture で自己較正（新 Req 6.7） |
| ⒞ 選択 | 順位表の最上位で、かつ Out of scope・別 spec 稼働中・既試行（台帳）に当たらないもの | 選ばなかった理由を台帳へ |
| ⒟⒠ 実装・テスト | 通常の実装＋`cargo test --workspace`＋見た目の追随テスト | 失敗なら ⒢ で戻す |
| ⒡ 再計測 | 変更前後のバイナリを 2 つ用意し A→B→A→B の交互 7 分 | 「ばらつき」は A→A の差で較正（新 Req 1.7） |
| ⒢ 採否 | 改善がばらつきを超え、テスト緑、見た目の追随 PASS → コミット。さもなくば `git revert`／`git checkout` で戻す | 1 周＝1 コミット（新 Req 1.8） |
| ⒣ 記録 | 台帳へ 1 行追記 | 周番号・仮説・差分ファイル・前後の値・採否・所要 |

### 6.3 停止条件と「頭打ち」の定義

- 目標達成＝25 分 release で判定式⑴〜⑷b すべて PASS（かつ dev で ⑴〜⑶ PASS）。
- 頭打ち＝連続 3 周「採用なし」。ただし順位表の最上位が Out of scope（合成本体・発話実装）に達した時点でも停止し、残る最大項を報告。
- 計測失敗＝サンプリングが空・記号解決が全滅・自己較正が赤。道具を直してから再開（人は呼ばない）。

### 6.4 設計フェーズへ持ち越す判断（新要件分）

13. **D-13 組み込み `/goal` との接続**（**✅ 2026-08-22 裁定＝`/goal` 採用・§6.6 参照**）: 残る設計事項＝目標文テンプレートの文言（判定役 Haiku が会話の表示行だけで判定できる形）・1 周のプロジェクトスキルの形・目標定義ファイル（YAML/TOML/JSON）と駆動スクリプトの分担・`CLAUDE_CODE_GOAL_CHECKIN_MINUTES` の値・context 肥大への対策（重い解析はサブエージェントへ・台帳を正本に）。
14. **D-14 プロファイル採取の道具**: `wpr`（プロファイル指定が簡単・解析は `wpa`/`xperf -i` 依存）か `xperf`（採取と `-i` 解析が同じ道具・スタックウォーク指定が明示的）か。記号解決の `_NT_SYMBOL_PATH` と PDB の置き場。
15. **D-15 スレッド別 CPU の取り方**: ETW のコンテキストスイッチ（`CSWITCH`）から算出／areka 側で `GetThreadTimes` を終了時に記録／両方。スレッドの役割名をどう付けるか（`SetThreadDescription` を areka 側で呼ぶ案＝wintf・areka-ghost のスレッド生成点に 1 行ずつ）。
16. **D-16 合否の採取とプロファイル採取の分離**: 同一セッション内で「素の走行（合否）」と「プロファイル走行（順位付け）」をどの順で何回回すか（交互 A/B × 2 系統で 1 周あたり 7 分×4〜6＝30〜45 分）。
17. **D-17 静寂確認の閾値**: マシン全体 CPU の上限・既知プロセス名の一覧・再採取の上限回数。

18. **D-18 モデルの使い分けの実装形**（新 Req 1.12・1.13）: ⒜ `.claude/agents/perf-measure.md`・`perf-analyze.md`・`perf-implement.md`・`perf-review.md`（frontmatter `model: opus`・tools 最小化）の定義内容。⒝ `kiro-impl` 改修の置き場所（SKILL.md の「Dispatch via Agent tool」3 箇所＋最終検証）と自己モデル判別の文言（システムプロンプト「You are powered by the model named …」を読む・判別不能なら `opus` へ倒す）。⒞ 起動時に各サブエージェントへ自分のモデル名を 1 行印字させる検査（`model` 指定が効かない環境で黙って Fable を継承しないため）。⒟ 既存リポジトリに `.claude/agents/` は無い（2026-08-22 確認）・`kiro-impl` の Agent 呼出は現状 `model` 無し＝メインを継承。

### 6.5 推奨（改訂）

- **最初の 1 周は「道具作り」**: 4 段の帰属が順位表として出るまで、是正は始めない（新 Req 2.1・2.10）。順位表が出れば、以後は機械的に回る。
- **合否は素の走行・順位付けはプロファイル走行**に分け、同一セッション内で交互に採る。
- **記号はビルド時の環境変数で付け、`Cargo.toml` には触れない**（新 Req 8.6）。
- 是正の候補（新 Req 3.2〜3.3）は §3.1 の A〜E と同じ。順位表の結果で選ぶ。

### 6.6 起動口の裁定（2026-08-22・`/goal` 対 `/loop` 対「使わない」・Opus 5 で実行する前提）

公式文書で確認した事実（`https://code.claude.com/docs/en/goal.md`・`https://code.claude.com/docs/en/scheduled-tasks.md`）:

| | `/goal` | `/loop` | 使わない（1 ターンで回す） |
|---|---|---|---|
| 次のターンが始まる契機 | 前のターンが終わったとき（背景作業があれば終了結果が新ターンとして届く） | 時間間隔（固定 cron または自己ペース 1〜60 分） | 無し（1 ターン内で背景タスクの通知を待つ） |
| 止まる条件 | 判定役が「達成」または「不可能」と判定／`/goal clear`／直さないと消えないエラー | 人が止める／Claude が終わったと判断／**7 日で失効** | ターンが終わったとき |
| 達成判定 | **あり**（毎ターン・小型モデル既定 Haiku が会話に現れた内容だけで判定・条件は 4,000 字以内） | **無し** | 無し（自前） |
| 長い計測（7〜25 分）との相性 | 背景実行中は判定を後回し・終了で自動再開・30 分で check-in（`CLAUDE_CODE_GOAL_CHECKIN_MINUTES` で変更） | 固定間隔と計測長が合わない・自己ペースは「終わった」と誤って止め得る | 可（背景通知で続く）が文脈が肥大 |
| 中断からの再開 | 再開時に条件を復元（v2.1.239 以降は全経路） | 7 日以内なら復元 | 無し |
| 主モデルの制約 | 無し（判定役は `ANTHROPIC_DEFAULT_HAIKU_MODEL`・主は Opus 5 で可） | 無し | 無し |
| 前提 | hooks が有効（`disableAllHooks` 無し＝本リポジトリは可）・auto mode で無人化 | — | — |

**裁定＝`/goal` を採用。** 理由: 目標が機械判定できる形で置かれる本 spec では「条件で止まる」「背景計測の終了で自動再開」「再開で条件復元」が決定的で、`/loop` にはどれも無い。「使わない」は文脈肥大と中断時の再開が弱い。**Opus 5 で回すための帰結**: ⒜ 1 周の手順をプロジェクトスキルとして明文化し（主モデルが毎ターン同じ順で回す）、⒝ 判定に要る事実を決まった書式で会話に表示する（判定役はファイルを読まない）、⒞ 台帳を正本にして会話の記憶に頼らない（要約・再開後も続く）、⒟ 重い解析はサブエージェントに逃がして主文脈を小さく保つ、⒠ `CLAUDE_CODE_GOAL_CHECKIN_MINUTES` を計測長より長く置く。`/loop` は fallback としても採らない（`/goal` が使えない環境では同じスキルを人が 1 周ずつ呼ぶ）。

---

## 7. 設計フェーズの調査記録（2026-08-22・`/kiro-spec-design`＝既存系への拡張として光の発見プロセスを適用）

- **Discovery Scope**: Extension（既存の計測資産 `tools/perf/`・wintf フレーム駆動・kiro スキル群への拡張）。外部依存の新規追加は無し（WPT は測定マシンに実在・Python 標準ライブラリ・PowerShell 7）。
- **Key Findings**（設計を動かした 3 点）:
  1. **`/goal` は「条件文＝最初のターンの指示」であり、判定役は会話だけを見る。背景タスク（subagent・background Bash）が走っている間は判定を飛ばし、終了結果を新しいターンとして届ける。** よって 1 周は「ターン単位で再入できる相の列」として設計し、各相の終わりに決まった書式の STATUS 行を会話へ印字する（§7.2）。
  2. **変化の有無は World から O(1) で引けない（bevy の変化検知はクエリ単位）ので、生産者が旗を立てる形にする。** 生産者は wintf の外（areka の `PresentBridge`・`MoveCueSink`・lifecycle 送信端）にも居るが、いずれも areka 側に実装があり wintf へ依存できる（依存方向は保てる）。表示層が「まだ仕事がある」ときは自分で次フレームを予約する（self-rearm）形が、タイプライタ・バルーン可視性の待ち時間・dola アニメを 1 つの形で覆う（§7.3）。
  3. **スレッドの役割名は既に OS に届いている。** `wintf-vsync`（`tick_bridge.rs:65-66`）・`wintf-cursor-monitor`（`clickthrough/monitor.rs:87-88`）・アクタースレッド（`areka-actor/src/spawn.rs:48-49`・`ticker`／`loop-ticker` は `ticker.rs:179,289`）は `thread::Builder::name` で名付けられ、bevy のタスクプールは `TaskPool (N)` で名付く（`bevy_tasks-0.19.1/src/task_pool.rs:174-177`）。Rust の std は Windows で `SetThreadDescription` を呼ぶので、プロセス内から `GetThreadDescription` で読める＝**スレッド別 CPU の表は実行体の内側で組める**（ETW に頼らない）。

### 7.1 `/goal` の受け取り方（公式文書 `code.claude.com/docs/en/goal.md`・2026-08-22 再確認）

- 構文: `/goal <条件文>` で設定（**設定した瞬間に条件文を指示として最初のターンが始まる**）・引数無しで状態表示・`/goal clear` で解除。条件文は 4,000 字以内。1 セッションに 1 つ。
- 判定役: 設定済みの小型モデル（Claude API では既定 Haiku・`ANTHROPIC_DEFAULT_HAIKU_MODEL` で変更）に**条件文と会話全体**を渡す。**道具を呼ばず、ファイルも読まない**。毎ターン終了時に「未達／達成／不可能」のいずれかと理由を返す。達成・不可能で目標は解除される。
- 背景作業: 「subagent または background shell command が走っているターン末は判定を飛ばし、背景作業が無いターン末で判定する」「背景作業が終わると結果を新しいターンとして届ける」。
- check-in: 背景作業で 30 分待つと「走っているタスクを列挙して出力を読み、進んでいれば待ち、止まっていれば直すか止める」指示が届く。`CLAUDE_CODE_GOAL_CHECKIN_MINUTES` で間隔を変える（0 で無効）。以後は 2 倍ずつ伸びる（上限 4 倍）。
- 無進捗の番人: 「道具を使わないターンが数回続くと停止して制御を返す」——本ループは毎ターン道具を使うので該当しない。
- 再開: `--continue`／`--resume`／セッションピッカーのいずれでも条件を復元（周回数・時間・トークンはリセット）。`-p` 非対話でも完走する。hooks が無効（`disableAllHooks`）だと使えない（本リポジトリの `.claude/settings.json` に同設定は無い＝使える）。権限は変わらないので**無人で回すには auto mode で起動**する。
- 良い条件文: 「1 つの計測できる終端状態」「その証明のしかた」「途中で守る制約」を含め、「`or stop after 20 turns`」のような上限節を入れる。判定役は会話に現れたものしか見ないので、ファイルに書いただけでは判定されない。
- `/loop`（`scheduled-tasks.md`）: 間隔駆動のみ・目標判定無し・7 日で失効。採らない（要件 1.6 の裁定どおり）。

### 7.2 ループ 1 周の実現形（設計決定）

| 論点 | 決定 | 根拠 |
|---|---|---|
| ターンの粒度 | **相（phase）単位で再入**する状態機械。台帳の `状態` ブロックに `iteration`／`phase`／`pending_run` を持ち、スキルは毎ターン「状態を読む → 相を 1 つ進める → STATUS 行を印字」だけを行う | 計測（7〜29 分）は background Bash で走らせてターンを終える。終了が新ターンとして届くので、相単位でなければ続きが回らない（要件 1.10「会話の記憶に頼らない」） |
| 目標定義ファイル | `tools/perf/goals/draw-load-parity.toml`（機械読取）＋同 `draw-load-parity.goal.md`（`/goal` へ渡す条件文） | 要件 1.1・1.6（汎用の形）。判定式・閾値・判定スクリプト版・水準・停止条件・静寂閾値・check-in 値を 1 箇所に |
| 台帳 | `.kiro/specs/areka-P0-draw-load-parity/loop-ledger.md` 1 ファイル（`状態` ブロック＋周ごとの固定キー行）。`perf-ledger.py` が追記・読取・STATUS 行生成を担う | 要件 1.3（1 ファイル・誰が読んでも同じ結論）・1.9（決まった書式） |
| STATUS 行 | `PERF-LOOP STATUS iter=… phase=… judge=… idle_cpu=… delta=… noise=… verdict=… streak=…/3 next=…` と終端の `PERF-LOOP FINAL: GOAL_MET …`／`PERF-LOOP FINAL: STOPPED reason=…`。条件文はこの 2 行の字面で達成／不可能を判定する形に書く | 判定役は会話しか見ない（§7.1） |
| 合否の採取と順位付けの採取 | **別走行**。順位付け用 1 本（tick 観測・スレッド報告を点灯＋CPU サンプリング同時）、採否用は A→B→A→B の 4 本（素の走行） | サンプリングと観測行は対象の CPU を押し上げる。A→A の差がばらつきの物差し（要件 1.7） |
| A／B の実行体 | 周の冒頭で現在の release 実行体（＋PDB・32bit helper）を走行ディレクトリへ複製して A とし、変更後のビルドを B とする。ランナーに `-BinDir` を足す | 同一セッション内の交互取得には両実行体が同時に要る |
| 記号 | ループの release ビルドは常に環境変数 `CARGO_PROFILE_RELEASE_DEBUG=line-tables-only` で行い、PDB を得る（`Cargo.toml` 非接触＝要件 8.6）。合否の走行も同じ実行体を使う | 最適化は変わらず PDB だけが増える。A と B の条件を揃える |
| 採否 | `perf-compare.py` が前後 2 本ずつから差とばらつきを出し、`ADOPTED`／`NO_DIFF`／`WORSE` を返す。採用の必要条件＝⒜ ワークスペース全テスト緑 ⒝ 見た目の追随チェック全 PASS ⒞ 主指標（定常アイドル CPU）の改善が `max(ばらつき, 床値)` を超え、副指標（⑴ p95・⑵ catch-up・⑶ 確保）が悪化しない | 要件 1.7・3.6・4.7 |
| 戻し方 | 台帳に記録したファイル一覧を `git restore --source=HEAD -- <files>`（新規ファイルは削除）。破壊的 reset は使わない | 要件 1.8（採用した変更だけがコミットとして残る） |
| 停止 | ⒜ 25 分最終判定で全 PASS ⒝ 連続 3 周採用なし ⒞ 安全（テスト赤で戻せない・道具が壊れた）⒟ 周数上限（既定 30・目標定義ファイル）。いずれも FINAL 行を印字し、到達点と残る最大項を台帳と会話へ | 要件 1.4 |
| check-in | `CLAUDE_CODE_GOAL_CHECKIN_MINUTES=60`（最長の背景コマンド＝交互 4 本 約 29 分＜60） | 要件 1.11 |
| モデル | メインはセッション設定（README に推奨「Fable で起動」・Opus 5 でも同じ手順で回る）。重い作業は `.claude/agents/perf-{measure,analyze,implement,review}.md`（`model: opus`）へ。各エージェントは最初の行に自分のモデル名を印字する（`model` 指定が効かない環境で黙って継承しないため） | 要件 1.10・1.12 |

### 7.3 「変化が無いときに回さない」の実現形（D-2・D-3・D-7 の決定）

- **旗の置き場**: wintf に `ecs/world/tick_wake.rs`（プロセス共有の `AtomicU32` ビット集合＋期限 `AtomicU64`）。生産者は `mark(bit)`／`arm_deadline(instant)` を呼び、tick 入口が `take()` で読んで倒す。ビット＝ポインタ入力／ドラッグ中／窓書込指令の積み上げ／Z 順要求／幾何系メッセージ受理（`WM_WINDOWPOSCHANGED`・`WM_DPICHANGED`・`WM_DISPLAYCHANGE` 等）／表示指令の到着（`PresentBridge`・`MoveCueSink`・lifecycle 送信端）／次フレーム予約（self-rearm）／グラフィックス無効。
- **判定は純関数** `tick_gate::should_run(&TickGateInputs) -> TickDecision`。入力は上のビット＋`deadline_due`＋`frames_since_run`（心拍）＋`warmup`（起動直後）。心拍 `TICK_HEARTBEAT_FRAMES=30`（120Hz で約 4 回/秒）と起動直後 `TICK_GATE_WARMUP_FRAMES=600` は「旗を立て忘れた生産者」に対する安全側の網（要件 3.2「疑わしいときは回す」・3.7）。
- **D-2（省略フレームの番号と時刻）**: 省略した tick では `FrameCount`／`FrameTime`／`TickStart` を**進めない**。フレーム番号は「回った tick」の系列のまま。`FrameHarness::advance_frame`（`frame_test_support.rs:710-716`）は常に回す側なので影響なし。
- **D-3（省略時の flush）**: `flush_window_pos_commands()` は**常に呼ぶ**（空なら安い）。wndproc 経路の再入 flush（`window_pos.rs:290`）と Z 指令の適用順は不変。
- **D-7（起床）**: vblank 起床は維持し、門で落とす（起床間引き案 D は要件 3.2 の 1 画面更新周期以内と衝突するので採らない）。クリック透過の中継（`runtime/mod.rs:307-328`）は触らない。
- **tick を省略しても壊れないことの根拠（R-6）**: bevy の変化ティックは World 単位で進むので、次に回った tick で `Changed`／`Added` は拾える。`RemovedComponents` は `Messages` と同じ二重バッファで、tick を挟まない限り消えない（省略中は `update` が走らない）。`Messages::update` の 3 本は FrameFinalize（回った tick のみ）。
- **テスト**: `should_run` の全組合せ（ビット 2^10 × 期限 2 × 心拍 2 × 起動直後 2）を決定論テストで固定し、「省略の直後に変化→次 tick で反映」を `EcsWorld` の headless tick で固定する。生産者の一覧は `include_str!` の字面検査で見張る。

### 7.4 4 段の計測手段（D-5・D-6・D-14・D-15 の決定）

| 段 | 決定 | 出力 |
|---|---|---|
| ① プロセス | 既存ランナー（`% Processor Time`）と `judge-perf.py` をそのまま。**ランナーの `-ConfirmQuiet` は残し、`-AutoQuiet`（`check-quiet.ps1` を前後に走らせる）を足す** | `cpu.csv`・`run-meta.txt`（版 1.1.0） |
| ② スレッド | **実行体の内側**で `CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD)`→`GetThreadTimes`＋`GetThreadDescription` を 60 秒ごとと終了時に採り、`perf(thread)` 行（target `areka::perf`・既定 OFF・`tracing::enabled!` の前置ガード）で出す。併せて `perf(process)` 行に壁時計と `GetProcessTimes` を併記 | 役割名の写像は純関数（`wintf-vsync`→vblank 検出・`wintf-cursor-monitor`→カーソル監視・`ticker`→ticker(dispatcher/kanade)・`loop-ticker`→SERIKO ループ・`TaskPool (n)`→タスクプール・main thread id→UI・その他アクター名） |
| ③ 関数 | **`xperf`** を第一候補（採取と解析が同じ道具）: `xperf -on PROC_THREAD+LOADER+PROFILE -stackwalk Profile -f` → `xperf -d` → `xperf -i … -symbols -a dumper -o dump.txt`（`_NT_SYMBOL_PATH` に `target/release` と MS 公開シンボルサーバ）→ `perf-rank.py` が `SampledProfile`／`Stack` 行を集計（自己時間・包含時間・モジュール別・スレッド別）。Rust の legacy mangling（`_ZN…17h<hash>E`）は解析側で機械的に展開する。**記号解決が 1 フレームも `areka.exe!` を出さなければ計測失敗**（要件 2.11）。**代替**: `wpaexporter` ＋ 版管理した `.wpaProfile` | `stacks-rank.txt` |
| ④ 相 | wintf に `ecs/world/tick_diag.rs`（target `wintf::tick`・既定 OFF・前置ガード）。`try_tick_world` の各 `try_run_schedule` を `Instant` で挟み、**1 秒窓で集約 1 行**（tick 回数・省略回数・心拍回数・壁時計合計/最大・UI スレッド CPU〔`GetThreadTimes` 差分〕・13 本別 µs）。フィールド名は 1 行に重複なし | `[tick] kind=window …` 行。`RUST_LOG_VALUE` は変えず、ランナーの `-RustLogExtra` で点灯 |

- **合否の走行では ②〜④ を点灯しない**（素の走行）。順位付けの走行だけで点灯し、サンプリングも同時に採る。
- D-16（分離と回数）: 1 周＝順位付け 1 本（7 分）＋採否 4 本（各 7 分）。ベースライン（周 0）と最終判定は 25 分 × release/dev。
- D-17（静寂の閾値）: マシン全体 `% Processor Time (_Total)` を 20 秒・1 秒刻みで採り平均 10% 未満、既知の重いプロセス名（`cargo`・`rustc`・`rust-analyzer`・`msbuild`・`link`・`cl`・対象以外の `areka`・対象以外の `python`）が無いこと。超えたら 60 秒待って再確認（上限 3 回）。前後の結果を走行ディレクトリへ残す。

### 7.5 見た目の追随をエージェント自身が確かめる形（要件 4.7）

- `invoke-followup-checks.ps1`（有界 120 秒の実走・`AREKA_APP_SMOKE_EXIT_MS`）の中から PowerShell（`Add-Type` で user32）が順に操作し、`judge-followup.py` がログと操作記録を突合する: ⒜ クリック透過＝キャラ窓の角（透明）と足元中央（不透明）へ `SetCursorPos` → `GetWindowLongPtr(GWL_EXSTYLE)` の `WS_EX_TRANSPARENT` が立つ／落ちること（OS の実状態）と `clickthrough: ex-style トグル適用`（`controller.rs:212`）の記録 ⒝ ドラッグ＝`SendInput` で足元から +80px 引く → `WM_WINDOWPOSCHANGED` の `[transition] kind=msg` とキャラ・バルーンの `kind=write` の位置差が不変 ⒞ DPI＝`SetWindowPos` で別 DPI のモニタへ移す → `WM_DPICHANGED` の受理と表示成立点 `k=` の更新、戻す ⒟ バルーン追従＝⒝⒞ の前後で `win_kind=balloon` の位置がキャラ窓相対で一致。
- 2 モニタ混在 DPI が前提（測定マシンの実形）。満たせないときは ⒞ を判定不能にし、**判定不能は採用しない**（安全側）。

### 7.6 設計決定の台帳（研究テンプレート「Design Decisions」相当）

| # | 決定 | 代替案 | 採った理由 | 追認点 |
|---|---|---|---|---|
| DD-1 | 1 周を相単位の状態機械にし台帳を正本にする | 1 ターンで 1 周を完走 | `/goal` は背景作業でターンを切る。要約・再開に強い | 相の遷移表を決定論テストで固定 |
| DD-2 | 判定材料を STATUS 行 1 本に集約し、条件文はその字面で書く | 判定役にファイルを読ませる | 判定役はファイルを読まない（公式） | 条件文テンプレートの字面とスクリプトの出力を同じ定数から出す |
| DD-3 | 旗方式の tick 門＋心拍＋起動直後の全走 | スケジュール単位の省略（案 B）／起床間引き（案 D） | 上限が最大・順序不変を保ちやすい・3.2 と両立 | 旗を立てる生産者の一覧を字面検査で固定 |
| DD-4 | スレッド別 CPU は実行体の内側で採る | ETW の CSWITCH 集計 | 名前付きで決定論的に出る・道具の導入無し | `GetThreadTimes` の粒度（約 15.6ms）を README に注記 |
| DD-5 | 関数別は `xperf` の dumper 出力を自前で集計 | `wpaexporter`＋`.wpaProfile`／`wpr` | 採取と解析が同じ道具・テキスト出力・記号解決の失敗を検出できる | 周 0（道具作り）で既知ケースを作り `--selftest` に載せる。代替は `wpaexporter` |
| DD-6 | `SELF_INITIATED_DEPTH` は `thread_local! Cell<i32>` へ置換し、錠 `lock_self_initiated_for_test` は**残す**（退役可を cage へ申し送る） | 錠 21 箇所も本 spec で退役 | 同居裁定（roadmap 追記(81)）＝錠の退役は cage が rebase で受ける | スレッド隔離の決定論テストを足す |
| DD-7 | 7 本の実行器見直し（案 C）は**候補**として持ち、順位表の結果で選ぶ。採る場合は前提テスト 2 本の字面検査を新しい構築形へ改訂 | 設計で一律に単スレッド化 | 効果量は測るまで不明（R-1） | `single_threaded(label)` 補助関数を構築側に置き、検査対象文字列を 1 つにする |
| DD-8 | 目標定義・スキル・エージェント・道具は性能以外の目標でも使える汎用の形（goal 名で切替） | 本 spec 専用 | 要件 1.6 | `goals/<name>.toml` の読取を決定論テストで固定 |
| DD-9 | kiro-impl の改修＝Preflight に「派遣モデルの決定」を 1 節足し、3 箇所の Agent 派遣と最終検証（kiro-validate-impl の subagent 派遣）へ `model: "opus"` を渡す規則を置く | Agent ツールの既定を変える（不可） | 要件 1.13 | 判別不能は `opus` 側へ倒す |

### 7.7 リスクと緩和

- 旗の立て忘れ＝見た目の遅れ → 心拍・起動直後の全走・生産者一覧の字面検査・実走の追随チェック（4 項目）。
- サンプリングの記号解決が環境依存で失敗 → 失敗を「計測失敗」として止める（黙って続けない）・代替 `wpaexporter`。
- 実行器見直しで既存テスト 2 本が赤 → 改訂（削除ではない）を候補の実装手順に含める。
- `/goal` の check-in が計測中に割り込む → 60 分に設定・check-in の指示は「待つ」で答える形をスキルに書く。
- 台帳の手書き汚染 → 機械が読む `- key: value` 行の書式を固定し、`perf-ledger.py` の読取を fixture で固定する。

### 7.8 参照

- `https://code.claude.com/docs/en/goal.md`・`https://code.claude.com/docs/en/scheduled-tasks.md`（2026-08-22 取得）
- `bevy_tasks-0.19.1/src/task_pool.rs:174-177`（スレッド名 `TaskPool (N)`）・`bevy_ecs-0.19.1/src/schedule/executor/multi_threaded.rs:274`
- 本書 §1〜§6（現況の file:line）・requirements.md 改訂欄

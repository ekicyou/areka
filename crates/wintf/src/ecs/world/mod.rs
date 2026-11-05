//! ECSワールド管理
//!
//! - schedule_labels: フレームカウンタとスケジュールラベル定義
//! - vsync: tick+flush の再入防止ガードと錠つきの窓の書き込みの流し

mod vsync;

pub mod schedule_labels;
pub mod thread_registry;
pub mod tick_diag;
pub mod tick_gate;
pub mod tick_wake;

pub use schedule_labels::*;
pub use vsync::*;

use bevy_ecs::message::Messages;
use bevy_ecs::prelude::*;
use bevy_ecs::schedule::*;
use bevy_ecs::system::*;
use std::cell::RefCell;
use std::rc::Weak;
use std::time::Instant;
use tracing::{error, info};

/// 単一スレッドで回すスケジュール（WUC に触る係と窓を作る係はここに置く）。
/// `intern()` が const でないため、定数の代わりに固定の配列を返す関数にする。
///
/// WUC（Windows.UI.Composition）はスレッド親和 API であり、かつ DispatcherQueue は
/// 「メッセージポンプが回るスレッド」に居ないと合成 commit が永遠に flush されない。
/// try_tick_world は UI スレッド（AsyncTickTask の spawn_local）から呼ばれるため、
/// WUC オブジェクトに触る schedule を SingleThreaded executor に固定して
/// 「全 WUC 呼び出し＝UI スレッド」を構造的に保証する。
///
/// 背景（2026-07-02 実測）: DComp は free-threaded で MultiThreaded executor
/// （worker スレッド実行）でも動作していたが、WUC 移行でこの前提が崩れた。
/// worker スレッドで CreateDispatcherQueueController(DQTYPE_THREAD_CURRENT) が走ると
/// DispatcherQueue がポンプの無い worker に紐づき、エラーを一切出さないまま
/// 画面へ何も合成されない（dcomp_demo: 窓は visible・SetBrush/SetRoot 全成功・
/// 内容だけ出ない／ECS 非依存でメインスレッド生成の wuc_spike: 表示される、で切り分け確定）。
pub(crate) fn single_threaded_schedules() -> [InternedScheduleLabel; 6] {
    [
        UISetup.intern(),
        GraphicsSetup.intern(),
        PreRenderSurface.intern(),
        RenderSurface.intern(),
        Composition.intern(),
        CommitComposition.intern(),
    ]
}

/// 外側 `Rc<RefCell<EcsWorld>>` への弱参照を保持する NonSend リソース（task 4.3）。
///
/// `create_windows`（排他システム）は bevy の `&mut World` しか持たないが、ウィンドウ生成
/// ファクトリ（runtime 層 `EcsWindowFactory`）は wndproc クロージャの `WndState` 用に
/// **外側**の `Weak<RefCell<EcsWorld>>`（共有 World ハンドルの弱参照）を必要とする。
///
/// 本リソースを **ECS 層**で定義することで `create_windows` が読むのは ECS 型のまま
/// （ecs→runtime の上向き依存を作らない）。`Weak` を保持する `!Send` 型ゆえ bevy が
/// 自動的に NonSend として扱う（`Send` 偽装はしない）。
///
/// 注入は `WinApp`（runtime）が `new()`/`run()` 開始時に `Rc::downgrade(&self.world)` を
/// insert する（facade から下向き）。`create_windows` はこのリソースの有無で新経路
/// （ファクトリ生成）/ 旧 `WinThreadMgr` 経路（no-op）を分岐する（task 4.3 解釈 (1)/(2)）。
pub struct EcsWorldSelfRef(pub Weak<RefCell<EcsWorld>>);

/// ECSワールドのラッパー
/// 初期化ロジックや拡張機能をここに集約
pub struct EcsWorld {
    world: World,
    /// フレーム駆動の相別観測（既定 OFF・`RUST_LOG` に `wintf::tick=debug` で点く）。
    tick_diag: tick_diag::TickDiag,
    /// tick の門が有効か（既定 **無効**＝門を入れる前と同じ挙動）。
    ///
    /// 既定値は周 1 の A/B（既定 ON 対 既定 OFF）で決める（設計 C16）。areka は起動時に
    /// `AREKA_TICK_GATE=1|0` を読んで [`set_tick_gate`](Self::set_tick_gate) で上書きできる。
    tick_gate_enabled: bool,
    /// 起動からの画面更新の回数（**省略した回も含む**・起動直後かどうかの物差し）。
    frames_since_boot: u32,
    /// 前回**回した** tick からの画面更新の回数（心拍の物差し）。
    frames_since_run: u32,
    /// 直前の判断が心拍だったか（観測行の `heartbeat=` に載せるための印）。
    ///
    /// 判断が置き、`try_tick_world` が読んで倒す。門を通さない呼び出し（旧経路・テスト）が
    /// 前回の印を引き継がないよう、読んだ側で必ず倒す。
    last_run_was_heartbeat: bool,
}

impl EcsWorld {
    /// 新しいEcsWorldを作成
    pub fn new() -> Self {
        let mut world = World::new();

        // リソースの初期化
        world.insert_resource(crate::ecs::app::App::new());
        world.insert_resource(FrameCount::default());
        world.insert_resource(crate::ecs::layout::taffy::TaffyLayoutResource::default());
        // Surface生成統計リソース（Req 5.3）
        world.insert_resource(crate::ecs::graphics::SurfaceCreationStats::default());

        // WicCore初期化（Device Lostの影響を受けない独立リソース）
        // 生成失敗は無言スキップにせず error! で記録する（Req 9.4・log-first）。
        // factory 無しでも縮退継続する（画像デコード不可のまま return しない）。
        match crate::ecs::widget::bitmap_source::WicCore::new() {
            Ok(wic_core) => {
                world.insert_resource(wic_core);
            }
            Err(e) => {
                error!(
                    error = %e,
                    "WicCore の生成に失敗（WIC factory 未取得）。画像デコード機能を欠いたまま継続する"
                );
            }
        }

        // FrameTime初期化（dola::runtime::clock::now() の値を保持）
        world.insert_resource(crate::ecs::graphics::FrameTime(dola::runtime::clock::now()));

        // TickStart初期化（遷移観測の刻印の時刻基準・D1）
        // 最初の tick で FrameCount と同じ点から更新されるが、tick 前に World を借りる
        // 観測点が資源を引けるよう構築時に置いておく。
        world.insert_resource(crate::ecs::window::transition_diag::TickStart(
            Instant::now(),
        ));

        // WintfTaskPool初期化（非同期タスク実行基盤）
        world.insert_resource(crate::ecs::widget::bitmap_source::WintfTaskPool::new());

        // LayoutRootとMonitor階層を初期化（Window spawnより前に必要）
        crate::ecs::layout::initialize_layout_root(&mut world);

        // ドラッグ累積器の登録
        crate::ecs::drag::install_drag_accumulator(&mut world);

        // ドラッグコンテキスト（ECS→wndprocスレッド間転送用）の登録
        world.insert_resource(crate::ecs::drag::WindowDragContextResource::new());

        // イベントの登録
        world.init_resource::<Messages<crate::ecs::drag::DragStartEvent>>();
        world.init_resource::<Messages<crate::ecs::drag::DragEvent>>();
        world.init_resource::<Messages<crate::ecs::drag::DragEndEvent>>();

        // スケジュールの登録
        {
            world.init_resource::<Schedules>();
            let mut schedules = world.resource_mut::<Schedules>();

            schedules.insert(Schedule::new(Input));
            schedules.insert(Schedule::new(Update));
            schedules.insert(Schedule::new(PreLayout));
            schedules.insert(Schedule::new(Layout));
            schedules.insert(Schedule::new(PostLayout));
            schedules.insert(Schedule::new(UISetup));
            schedules.insert(Schedule::new(GraphicsSetup));
            schedules.insert(Schedule::new(Draw));
            schedules.insert(Schedule::new(PreRenderSurface));
            schedules.insert(Schedule::new(RenderSurface));
            schedules.insert(Schedule::new(Composition));
            schedules.insert(Schedule::new(CommitComposition));
            schedules.insert(Schedule::new(FrameFinalize));

            // WUC に触る係と窓を作る係は UI スレッドで回す（理由は single_threaded_schedules）。
            for label in single_threaded_schedules() {
                match schedules.get_mut(label) {
                    Some(sc) => {
                        sc.set_executor(SingleThreadedExecutor::new());
                    }
                    None => error!(?label, "単一スレッドにするスケジュールが登録されていない"),
                }
            }
        }

        // デフォルトシステムの登録
        {
            let mut schedules = world.resource_mut::<Schedules>();

            // Inputスケジュール: 非同期タスク完了コマンドのドレイン
            schedules.add_systems(
                Input,
                crate::ecs::widget::bitmap_source::systems::drain_task_pool_commands,
            );

            // Inputスケジュール: ポインターイベントディスパッチ
            // transfer_buffers_to_world()でWorldに直接データ投入済み
            schedules.add_systems(
                Input,
                crate::ecs::pointer::dispatch_pointer_events
                    .after(crate::ecs::widget::bitmap_source::systems::drain_task_pool_commands),
            );

            // 注: process_pointer_buffersは廃止
            // WndProcスレッドのthread_localバッファは、try_tick_world()内の
            // transfer_buffers_to_world()で直接Worldに転送される

            // Inputスケジュール: ドラッグイベントディスパッチ
            schedules.add_systems(
                Input,
                crate::ecs::drag::dispatch_drag_events
                    .after(crate::ecs::pointer::dispatch_pointer_events),
            );

            // Inputスケジュール: ドラッグ状態クリーンアップ（dispatch_drag_eventsの後）
            schedules.add_systems(
                Input,
                crate::ecs::drag::cleanup_drag_state.after(crate::ecs::drag::dispatch_drag_events),
            );

            // Inputスケジュール: ポインターデバッグ監視（デバッグビルドのみ）
            #[cfg(debug_assertions)]
            schedules.add_systems(
                Input,
                (
                    crate::ecs::pointer::debug_pointer_state_changes,
                    crate::ecs::pointer::debug_pointer_leave,
                )
                    .after(crate::ecs::pointer::dispatch_pointer_events),
            );

            // UISetupスケジュール：ウィンドウ作成とWindowPos反映
            schedules.add_systems(UISetup, crate::ecs::window::window_system::create_windows);
            // on_window_handle_addedとon_window_handle_removedはフックで代替

            // WindowPos変更をSetWindowPosに反映（メインスレッド固定が必要）
            schedules.add_systems(UISetup, crate::ecs::graphics::apply_window_pos_changes);

            // Updateスケジュールにディスプレイ構成変更検知とモニター階層管理を登録
            schedules.add_systems(
                Update,
                (
                    // ディスプレイ構成変更検知
                    crate::ecs::layout::detect_display_change_system,
                    // Monitorレイアウト更新（detect_display_change_systemの後）
                    crate::ecs::layout::update_monitor_layout_system
                        .after(crate::ecs::layout::detect_display_change_system),
                    // Typewriter更新（アニメーション進行）
                    crate::ecs::widget::text::update_typewriters
                        .after(crate::ecs::layout::update_monitor_layout_system),
                )
                    .chain(),
            );

            // Layoutスケジュールにtaffyレイアウトシステムを登録
            schedules.add_systems(
                Layout,
                (
                    crate::ecs::layout::build_taffy_styles_system,
                    crate::ecs::layout::sync_taffy_tree_system
                        .after(crate::ecs::layout::build_taffy_styles_system),
                    crate::ecs::layout::compute_taffy_layout_system
                        .after(crate::ecs::layout::sync_taffy_tree_system),
                    crate::ecs::layout::update_arrangements_system
                        .after(crate::ecs::layout::compute_taffy_layout_system),
                    crate::ecs::layout::cleanup_removed_entities_system,
                )
                    .chain(),
            );

            // PostLayoutスケジュール: 論理計算系（Arrangement伝播まで）
            // Note: Arrangementは各コンポーネントのon_addフックで自動挿入されるため、
            //       init_window_arrangementシステムは廃止されました。
            schedules.add_systems(
                PostLayout,
                (
                    crate::ecs::layout::sync_window_arrangement_from_window_pos,
                    crate::ecs::layout::sync_simple_arrangements,
                    crate::ecs::layout::mark_dirty_arrangement_trees
                        .after(crate::ecs::layout::sync_simple_arrangements),
                    crate::ecs::layout::propagate_global_arrangements
                        .after(crate::ecs::layout::mark_dirty_arrangement_trees),
                    crate::ecs::layout::window_pos_sync_system
                        .after(crate::ecs::layout::propagate_global_arrangements),
                )
                    .chain(),
            );

            // GraphicsSetupスケジュール: WUC（Windows.UI.Composition）合成スタック初期化
            // 失いの確かめ → 無効化 → 装置 → WUC の資源 → 窓の順（無効化を作り直しより先に置かないと、
            // 作り直された装置を見て何も倒さなくなる）。作る係は作れる回だけ回し、変化の印を食べない。
            {
                use crate::ecs::graphics::*;
                schedules.add_systems(
                    GraphicsSetup,
                    (
                        check_device_removed,
                        invalidate_dependent_components,
                        init_graphics_core,
                        init_wuc_resource.run_if(graphics_core_ready),
                        init_window_graphics.run_if(wuc_ready),
                    )
                        .chain(),
                );
            }

            // Drawスケジュール: ウィジェット描画システム（D2D の命令の列を作るだけで WUC に触らない）
            // chainはgenerate_alpha_mask_systemまで維持
            schedules.add_systems(
                Draw,
                (
                    // Brush継承解決（描画システムより前に実行）
                    crate::ecs::graphics::resolve_inherited_brushes,
                    crate::ecs::widget::shapes::rectangle::draw_rectangles
                        .after(crate::ecs::graphics::resolve_inherited_brushes),
                    crate::ecs::widget::text::draw_labels
                        .after(crate::ecs::graphics::resolve_inherited_brushes),
                    // Typewriter: Arrangement変更で無効化 → LayoutCache初期化 → 描画の順
                    crate::ecs::widget::text::invalidate_typewriter_layout_on_arrangement_change
                        .after(crate::ecs::graphics::resolve_inherited_brushes),
                    crate::ecs::widget::text::init_typewriter_layout
                        .after(crate::ecs::widget::text::invalidate_typewriter_layout_on_arrangement_change),
                    crate::ecs::widget::text::draw_typewriters
                        .after(crate::ecs::widget::text::init_typewriter_layout),
                    // 空トーク時の背景描画（draw_typewritersの後）
                    crate::ecs::widget::text::draw_typewriter_backgrounds
                        .after(crate::ecs::widget::text::draw_typewriters),
                    crate::ecs::widget::bitmap_source::draw_bitmap_sources
                        .after(crate::ecs::graphics::resolve_inherited_brushes),
                    // αマスク生成（draw_bitmap_sourcesの後、BitmapSourceResource追加検出時に実行）
                    crate::ecs::widget::bitmap_source::generate_alpha_mask_system
                        .after(crate::ecs::widget::bitmap_source::draw_bitmap_sources),
                )
                    .chain(),
            );

            // PreRenderSurface: WUC の見た目（Visual）と描画面（Surface）の部品の管理
            schedules.add_systems(
                PreRenderSurface,
                (
                    // 後から窓へつないだ部品に見た目の部品を入れる（4 番）。同じ回に作られるよう先頭。
                    crate::ecs::graphics::insert_graphics_parts_on_window_join.before(
                        crate::ecs::graphics::visual_manager::visual_resource_management_system,
                    ),
                    crate::ecs::graphics::visual_manager::visual_resource_management_system
                        .run_if(crate::ecs::graphics::wuc_ready),
                    crate::ecs::graphics::deferred_surface_creation_system
                        .run_if(crate::ecs::graphics::wuc_ready)
                        .after(
                            crate::ecs::graphics::visual_manager::visual_resource_management_system,
                        ),
                    crate::ecs::graphics::mark_dirty_surfaces
                        .after(crate::ecs::graphics::deferred_surface_creation_system),
                    crate::ecs::graphics::cleanup_surface_on_commandlist_removed
                        .after(crate::ecs::graphics::mark_dirty_surfaces),
                    crate::ecs::graphics::visual_manager::window_visual_integration_system
                        .after(crate::ecs::graphics::cleanup_surface_on_commandlist_removed),
                ),
            );

            // RenderSurface: WUC の描画面へ命令の列を描く
            schedules.add_systems(RenderSurface, crate::ecs::graphics::render_surface);

            // Compositionスケジュール: WUC Visual階層同期（合成 commit は WUC 側で暗黙反映）
            // visual_hierarchy_sync → visual_property_sync → clip_sync
            schedules.add_systems(
                Composition,
                (
                    crate::ecs::graphics::visual_hierarchy_sync_system,
                    crate::ecs::graphics::visual_property_sync_system
                        .after(crate::ecs::graphics::visual_hierarchy_sync_system),
                    crate::ecs::graphics::clip_sync_system
                        .after(crate::ecs::graphics::visual_property_sync_system),
                ),
            );

            // CommitComposition: 現状システム無し（空スケジュール）
            // Note: WUC への合成 commit は暗黙反映のため専用システム不要（要件 7.1）。旧 ULW 画面転送
            // システムは撤去済み。schedule ラベルと try_run_schedule 呼び出しは tick 構成不変
            // （要件 4.5 / 13 本固定順）を保つため残置し、zero-system の no-op として回す。

            // FrameFinalizeスケジュール: 一時的ポインター状態クリア
            schedules.add_systems(
                FrameFinalize,
                crate::ecs::pointer::clear_transient_pointer_state,
            );

            // FrameFinalizeスケジュール: ドラッグ中は次の画面更新を予約（tick の末尾）
            schedules.add_systems(FrameFinalize, crate::ecs::drag::rearm_tick_while_dragging);

            // FrameFinalizeスケジュール: Messagesの更新
            schedules.add_systems(
                FrameFinalize,
                (
                    |world: &mut World| {
                        world
                            .resource_mut::<Messages<crate::ecs::drag::DragStartEvent>>()
                            .update()
                    },
                    |world: &mut World| {
                        world
                            .resource_mut::<Messages<crate::ecs::drag::DragEvent>>()
                            .update()
                    },
                    |world: &mut World| {
                        world
                            .resource_mut::<Messages<crate::ecs::drag::DragEndEvent>>()
                            .update()
                    },
                ),
            );
        }

        // ツールチップ（資源と FrameFinalize の判定）の登録
        crate::ecs::tooltip::install(&mut world);

        Self {
            world,
            tick_diag: tick_diag::TickDiag::default(),
            tick_gate_enabled: false,
            frames_since_boot: 0,
            frames_since_run: 0,
            last_run_was_heartbeat: false,
        }
    }

    /// 指定したスケジュールにシステムを追加
    ///
    /// # Example
    /// ```ignore
    /// world.add_systems(Update, my_system);
    /// ```
    pub fn add_systems<M>(
        &mut self,
        schedule: impl ScheduleLabel,
        systems: impl IntoScheduleConfigs<ScheduleSystem, M>,
    ) {
        self.world
            .resource_mut::<Schedules>()
            .add_systems(schedule, systems);
    }

    /// 内部のWorldへの参照を取得
    pub fn world(&self) -> &World {
        &self.world
    }

    /// 内部のWorldへの可変参照を取得
    pub fn world_mut(&mut self) -> &mut World {
        &mut self.world
    }

    /// 非同期タスクを生成（WintfTaskPool経由）
    ///
    /// タスク内で`CommandSender`を使ってECSコマンドを送信できる。
    /// コマンドはInputスケジュールで自動的にWorldに適用される。
    ///
    /// # Example
    /// ```ignore
    /// ecs_world.spawn(|tx| async move {
    ///     let result = some_async_work().await;
    ///     let cmd: BoxedCommand = Box::new(move |world: &mut World| {
    ///         // worldを操作
    ///     });
    ///     let _ = tx.send(cmd);
    /// });
    /// ```
    pub fn spawn<F, Fut>(&self, f: F)
    where
        F: FnOnce(crate::ecs::widget::bitmap_source::CommandSender) -> Fut + Send + 'static,
        Fut: std::future::Future<Output = ()> + Send,
    {
        if let Some(task_pool) = self
            .world
            .get_resource::<crate::ecs::widget::bitmap_source::WintfTaskPool>()
        {
            task_pool.spawn(f);
        }
    }

    // ────────────────────────────────────── tick の門（設計 C16・要件 3.2/3.7）

    /// tick の門の有効を切り替える（起動時に 1 度・`AREKA_TICK_GATE` の上書き口）。
    ///
    /// 無効なら判断そのものを行わず必ず回す（＝門を入れる前と同じ挙動）。切り替えは
    /// 起動時の 1 回を想定しているので、呼ばれたことを 1 行残す。
    pub fn set_tick_gate(&mut self, enabled: bool) {
        self.tick_gate_enabled = enabled;
        info!("[set_tick_gate] tick gate enabled={enabled}");
    }

    /// tick の門が有効か。
    pub fn tick_gate_enabled(&self) -> bool {
        self.tick_gate_enabled
    }

    /// 数えている 2 つの回数（起動から, 前回回した tick から）。
    ///
    /// 観測と決定論テストのための読み出し口で、判断には使わない。
    pub fn tick_gate_counters(&self) -> (u32, u32) {
        (self.frames_since_boot, self.frames_since_run)
    }

    /// 直前の判断が心拍だったか（観測行の `heartbeat=` へ載る印）。
    #[cfg(test)]
    pub(crate) fn last_run_was_heartbeat(&self) -> bool {
        self.last_run_was_heartbeat
    }

    /// 画面更新 1 回ぶんの判断（旗を読んで倒す → 純関数 → 数を数える）。
    ///
    /// 旗の読み取りは原子的な swap（[`tick_wake::take`]）で、**失敗しうる経路が無い**
    /// ——錠を取らず、OS も呼ばず、`Result` も返さない。したがってここに `Err` の腕は
    /// 存在しない。将来ここへ失敗しうる経路（別プロセスからの読み出し・OS 問い合わせ
    /// など）を足すときは、要件 3.7 に従い **`error!` を残して「回す」側**
    /// （[`TickDecision::Run`](tick_gate::TickDecision::Run)`(`[`RunReason::Disabled`](tick_gate::RunReason::Disabled)`)`）
    /// へ倒すこと——黙って省略へ倒してはならない。作り物の `Result` は返さない。
    pub fn decide_tick(&mut self, now: Instant) -> tick_gate::TickDecision {
        self.decide_tick_with(tick_wake::take(now))
    }

    /// 読み取り済みの旗から判断する（[`decide_tick`](Self::decide_tick) の中身）。
    ///
    /// 旗を注入できるので、プロセス共有の実体に触れずに決定論テストが書ける。
    /// 判断そのものは純関数 [`tick_gate::should_run`] に閉じており、ここがするのは
    /// 入力を組むことと、答えに応じて 2 つの回数を進めることだけである。
    pub fn decide_tick_with(
        &mut self,
        snapshot: tick_wake::WakeSnapshot,
    ) -> tick_gate::TickDecision {
        let inputs = tick_gate::TickGateInputs::from_snapshot(
            &snapshot,
            self.frames_since_run,
            self.frames_since_boot,
            self.tick_gate_enabled,
        );
        let decision = tick_gate::should_run(&inputs);

        // 起動からの回数は省略した回も含めて進む（起動直後の物差しなので頭打ちで十分）。
        self.frames_since_boot = self.frames_since_boot.saturating_add(1);
        match decision {
            tick_gate::TickDecision::Run(reason) => {
                self.frames_since_run = 0;
                self.last_run_was_heartbeat = matches!(reason, tick_gate::RunReason::Heartbeat);
            }
            tick_gate::TickDecision::Skip => {
                self.frames_since_run = self.frames_since_run.saturating_add(1);
            }
        }

        decision
    }

    /// 省略した画面更新を 1 件記録する（門が [`Skip`](tick_gate::TickDecision::Skip) を返した回）。
    ///
    /// `FrameCount`／`FrameTime`／`TickStart` は**進めない**し、スケジュールも 1 本も
    /// 回さない。観測が点いているときだけ省略の数を積み、窓が閉じていれば 1 行出す
    /// ——省略が長く続く間も窓は閉じる（`ticks=0` の窓は正当な観測結果である）。
    /// 観測が消えているときは時刻を 1 度も取らない（要件 3.8）。
    pub fn note_skipped_tick(&mut self) {
        if !tick_diag::is_enabled() {
            return;
        }
        let now = Instant::now();
        self.tick_diag.record_skipped(now);
        if let Some(line) = self.tick_diag.maybe_close(now) {
            tick_diag::emit_line(&line);
        }
    }

    /// 13 本のスケジュールを 1 回回す（いつも回すので `true` を返す）
    pub fn try_tick_world(&mut self) -> bool {
        // 相別観測の前置ガード（要件 3.8）。tick 1 回につきここで 1 度だけ評価し、
        // 以後はこの bool を配る。偽のときこの観測のために時刻を取らず、行も組まない
        // （兄弟テストが「ガードは最初の時刻取得より前」を本文の並びで固定する）。
        let diag_on = tick_diag::is_enabled();

        // FrameCount をインクリメント、FrameTime を更新
        //
        // 整数境界（W7b-V 点検）: FrameCount(u32) は毎 tick +1 で増加し、~60Hz では
        // 2^32 到達に約828日の連続 tick を要する。到達時 debug ビルドは加算オーバーフローで
        // panic、release はラップする。FrameCount.0 の消費は (1) 多数の tracing ログの
        // `frame=` フィールドと (2) graphics/systems/surface.rs::mark_dirty_surfaces での
        // `dirty.requested_frame = frame_count.0 as u64`（Changed<SurfaceGraphicsDirty>
        // 検出をトリガーするための「変化ノンス」用途で、値自体は本番では読まれない。本番読み取りは
        // 皆無で grep 上 graphics/tests.rs のみ）のみ。後者はラップ MAX→0 でも「前回値と異なる」
        // 性質を満たすため Changed 検出は継続し、いずれの消費も算術上の大小比較・配列添字には
        // 用いられないため、ラップしても観測挙動はログ値が 0 に戻るだけで正当性に影響しない。
        // debug panic 化を避ける堅牢化（saturating/wrapping_add）は debug 挙動を変えるため
        // P66 として記録。
        //
        // 遷移観測の刻印（D1）: フレーム番号の権威はここで進む 1 系列だけである。
        // 同一点で Resource `TickStart` を更新し、同じ値をスレッド局所の写しへ配る
        // （`transition_diag::begin_tick`）。World を借りられる観測点は資源から、
        // 借りられない観測点（一括 flush・wndproc）だけが写しから刻印を組む。
        // スケジュール構成（13 本の順序）はここでは変えない。
        let tick_start = Instant::now();
        let frame = if let Some(mut frame_count) = self.world.get_resource_mut::<FrameCount>() {
            frame_count.0 += 1;
            frame_count.0
        } else {
            // FrameCount を持たない World（`EcsWorld::new` 以外で組まれた場合）では
            // 番号を進める先が無いので 0 のまま配る（資源と写しが食い違わない）。
            0
        };
        match self
            .world
            .get_resource_mut::<crate::ecs::window::transition_diag::TickStart>()
        {
            Some(mut start) => start.0 = tick_start,
            None => self
                .world
                .insert_resource(crate::ecs::window::transition_diag::TickStart(tick_start)),
        }
        crate::ecs::window::transition_diag::begin_tick(frame, tick_start);

        if let Some(mut frame_time) = self
            .world
            .get_resource_mut::<crate::ecs::graphics::FrameTime>()
        {
            frame_time.0 = dola::runtime::clock::now();
        }

        // WndProcスレッドのthread_localバッファからWorldに直接データを投入
        // これにより、マルチスレッドで実行されるシステムでもデータにアクセス可能になる
        crate::ecs::pointer::transfer_buffers_to_world(&mut self.world);

        // 相別観測（既定 OFF）: 点いているときだけ UI スレッドのハンドルを 1 度確保する。
        if diag_on {
            self.tick_diag.ensure_ui_thread();
        }

        // 各Scheduleを順番に実行
        //
        // 13 本の呼び出しと順序は不変（要件 3.4／既存の順序テスト 2 本）。間に挟まる
        // `phase.lap()` は相別観測の区切りで、OFF のときは何もしない（時刻も取らない）。
        let mut phase = tick_diag::PhaseTimer::start(diag_on);
        let _ = self.world.try_run_schedule(Input);
        phase.lap();
        let _ = self.world.try_run_schedule(Update);
        phase.lap();
        let _ = self.world.try_run_schedule(PreLayout);
        phase.lap();
        let _ = self.world.try_run_schedule(Layout);
        phase.lap();
        let _ = self.world.try_run_schedule(PostLayout);
        phase.lap();
        let _ = self.world.try_run_schedule(UISetup);
        phase.lap();
        let _ = self.world.try_run_schedule(GraphicsSetup);
        phase.lap();
        let _ = self.world.try_run_schedule(Draw);
        phase.lap();
        let _ = self.world.try_run_schedule(PreRenderSurface);
        phase.lap();
        let _ = self.world.try_run_schedule(RenderSurface);
        phase.lap();
        let _ = self.world.try_run_schedule(Composition);
        phase.lap();
        let _ = self.world.try_run_schedule(CommitComposition);
        phase.lap();
        let _ = self.world.try_run_schedule(FrameFinalize);
        phase.lap();

        // Layout スケジュール実行後のタイミングで NCHITTEST キャッシュをクリア
        crate::ecs::pointer::nchittest_cache::clear_nchittest_cache();

        // 心拍で回した回かどうかは、門の判断（`decide_tick_with`）が置いた印を読む。
        // 読んだら倒す——門を通さない呼び出し（旧経路・テスト）が前回の印を引き継がない。
        let heartbeat = std::mem::take(&mut self.last_run_was_heartbeat);

        // 相別観測の集計と、1 秒窓が閉じたときの 1 行（要件 2.5／2.6）。
        if diag_on {
            let ended = Instant::now();
            let wall_us = ended.saturating_duration_since(tick_start).as_micros() as u64;
            self.tick_diag
                .record_run(ended, frame, wall_us, phase.per_schedule(), heartbeat);
            if let Some(line) = self.tick_diag.maybe_close(ended) {
                tick_diag::emit_line(&line);
            }
        }

        true
    }
}

impl Default for EcsWorld {
    fn default() -> Self {
        Self::new()
    }
}

/// プロセス共有の旗（[`tick_wake`]）を触る決定論テストが取る錠（テスト専用）。
///
/// 旗は 1 つしかない実体をプロセス全体で共有するため、並列に走るテストは互いの旗を
/// 読み倒してしまう。共有の旗（[`tick_wake::mark`]／[`tick_wake::take`]）に触る検査は、
/// **この 1 つ**の錠を取ってから触ること。
///
/// **錠は 1 つでなければ意味がない。** 守る対象が 1 つしか無いのに錠が 2 つあれば、
/// どちらを取っても相手を締め出せず、直列化は成立しない。共有の旗に触る検査は現在
/// 4 箇所——`world_tick_gate_tests.rs`（共有経由の判断）・`tick_wake_tests.rs`
/// （共有の入口の往復）・`runtime/tick_bridge.rs`（門が無効な 1 フレーム）・
/// `ecs/drag/systems.rs`（ドラッグ中の予約）——で、いずれもここを参照する。新たに
/// 共有の旗へ触る検査を足すときも、自前の錠を作らずここを使う。
///
/// **錠でも防げないものがある。** 旗を立てるのは本番の経路（窓書込指令の積み上げ・
/// ウィンドウメッセージの配送・Z 順の要求・ポインタ入力の投入）であり、そちらは錠を
/// 知らない。ゆえに「旗が立っていない**はず**」＝省略になるはず、を共有の旗の上で
/// 主張してはならない。省略の側を確かめる検査は、読み取り結果を直に渡す差し替え口
/// （`runtime::tick_bridge::tick_one_frame_with`）を使うこと。共有の旗の上で主張して
/// よいのは「立てた旗が含まれる」「回る側になる」——余分な旗で結論が変わらない側だけ
/// である。
///
/// 取るときは毒化に耐えること（`lock().unwrap_or_else(|p| p.into_inner())`）——
/// 錠を持ったまま 1 本が落ちたときに、無関係な検査まで連鎖して赤くしないため。
#[cfg(test)]
pub(crate) static TICK_WAKE_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[cfg(test)]
#[path = "world_tick_gate_tests.rs"]
mod tick_gate_wiring_tests;

#[cfg(test)]
#[path = "world_schedule_tests.rs"]
mod schedule_tests;

impl std::fmt::Debug for EcsWorld {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EcsWorld").finish_non_exhaustive()
    }
}

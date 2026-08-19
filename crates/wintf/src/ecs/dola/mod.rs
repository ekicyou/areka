//! DolaAnimator — dola ランタイムの ECS Component ラッパー。
//!
//! `DolaRuntime` をエンティティごとに所有し、フレーム単位で一括 tick する。
//! `tick_dola_animators` System により全エンティティを `Res<FrameTime>` で駆動する。
//!
//! # 安全性
//!
//! `DolaRuntime` 内部に `Rc` を含むため `Send + Sync` を手動実装する。
//! 健全性は `Query<&mut DolaAnimator>` の排他（`&mut`）アクセスにより、ある
//! `DolaAnimator` を同時に触れるのが常に高々1スレッドであることに依拠する。
//! なお wintf は単一 UI スレッドだが ECS スケジュールは既定でマルチスレッド実行であり
//! （`multi_threaded` feature 有効・`UISetup` のみ `SingleThreaded` 固定）、本型は現状
//! どのプロダクションスケジュールにも未配線（到達不能）である。実スケジュール構成の
//! 裏取り結果と `Sync` 側の潜在ハザードの詳細は `unsafe impl` 直前の SAFETY 注記を参照。
//!
//! # balloon06 との関係
//!
//! balloon06 の `DolaBridgeResource` 設計を本 `DolaAnimator` Component 設計で上書きする。
//! `DolaBridgeResource` は `Resource`（ワールドに一つ）であったのに対し、
//! `DolaAnimator` は `Component`（エンティティごと）で、複数アニメーションの独立管理が可能。

use bevy_ecs::component::Component;
use bevy_ecs::system::{Query, Res};
use dola::runtime::{DolaRuntime, UpdateResult};

use super::graphics::FrameTime;

/// DolaRuntime の ECS Component ラッパー。
///
/// エンティティごとに独立した `DolaRuntime` を保持し、
/// `tick_dola_animators` System で一括更新される。
///
/// # 消費者パターン
///
/// ```rust,ignore
/// fn my_consumer(query: Query<&DolaAnimator>) {
///     for animator in query.iter() {
///         let result = animator.last_result();
///         for change in &result.changes {
///             // 変化した変数に応じて ECS Component を更新
///         }
///     }
/// }
/// ```
#[derive(Component)]
pub struct DolaAnimator {
    runtime: DolaRuntime,
}

// SAFETY 条件: DolaRuntime は内部に Rc を含む（timeline_manager の ObjectInternPool
// = HashMap<DynamicValue, Rc<DynamicValue>>（dola runtime/interpolator/mod.rs:19-20）
// および UpdateResult.changes 経由の EvaluatedValue::Object(Rc<DynamicValue>)
// （dola runtime/types.rs:24・144-146）。Rc が !Send + !Sync であることにより
// DolaRuntime も自動では Send/Sync を導出できない。一方 bevy_ecs の Component は
// Send + Sync を要求するため、この手動 unsafe impl は冗長ではなく必須（load-bearing）である。
//
// 健全性の根拠（W8-V が実コードで裏取りした実スケジュール構成）:
//   1. tick_dola_animators / DolaAnimator は現状プロダクションのいずれのスケジュールにも
//      登録・spawn されていない（ワークスペース全域 grep: 出現は本ファイルと ecs/mod.rs の
//      再エクスポート・tests/ecs/dola_animator_test.rs のみ。ecs/world/mod.rs の
//      schedules.add_systems 群に tick_dola_animators は不在）。よって本 unsafe impl は
//      現状到達不能（reachable な跨スレッド共有経路が存在しない）。
//   2. ただし wintf は単一 UI スレッドではあるが、ECS スケジュールは単一スレッド実行ではない:
//      ワークスペースは bevy_ecs の "multi_threaded" feature を有効化しており
//      （ルート Cargo.toml [workspace.dependencies.bevy_ecs] features）、Schedule の
//      既定 ExecutorKind は MultiThreaded（bevy_ecs schedule/executor/mod.rs:65-66）。
//      ecs/world/mod.rs では UISetup のみ set_executor(SingleThreadedExecutor) で固定し、
//      Input/Update 等の他スケジュールは既定 = マルチスレッドエグゼキュータで走る。
//      Send である本型を読むシステム（Query<&mut DolaAnimator>）は is_send==true と
//      判定され（function_system.rs:84 / multi_threaded.rs:545）メインスレッドに固定されず
//      ワーカースレッド上で実行され得る。
//   3. それでも健全である理由: Query<&mut DolaAnimator> は排他（&mut）アクセスであり、
//      bevy のアクセス競合スケジューリングにより、ある DolaAnimator を同時に触れるのは
//      常に高々1スレッド（内部 Rc の参照カウント操作・clone もその単一システム内で完結）。
//      tick_dola_animators の iter_mut() は逐次（par_iter_mut ではない）であり、複数
//      DolaAnimator 間にもデータ共有はない。
//
// 留意（Sync 側の潜在ハザード・W8-V 警告）: Sync は「複数の読み取り専用システムが
// 跨スレドで &DolaAnimator を同時共有する」ことを許す。last_result() が返す
// UpdateResult は EvaluatedValue::Object(Rc<DynamicValue>) を含み得るため、もし将来
// 消費者システム（Query<&DolaAnimator>）が並列に走り内部 Rc を clone すると、非アトミックな
// 参照カウントにデータ競合が生じ得る。現状は消費者システム・spawn が皆無で到達不能だが、
// 本型をプロダクションのマルチスレッドスケジュールへ配線する際は (a) tick も消費も
// SingleThreaded スケジュールへ載せる／(b) DolaRuntime の Rc を Arc 化する 等の対策が要る
// （挙動変更を伴うため本ループ非適用＝proposals P68 記録）。
unsafe impl Send for DolaAnimator {}
unsafe impl Sync for DolaAnimator {}

impl DolaAnimator {
    /// 新しい DolaAnimator を生成する。
    pub fn new() -> Self {
        Self {
            runtime: DolaRuntime::new(),
        }
    }

    /// 既存の DolaRuntime で DolaAnimator を生成する。
    pub fn with_runtime(runtime: DolaRuntime) -> Self {
        Self { runtime }
    }

    /// 現在時刻まで内部状態を進行する。
    ///
    /// 通常は `tick_dola_animators` System から呼び出される。
    /// 消費者が直接呼び出す必要はない。
    pub fn tick(&mut self, current_time: f64) {
        self.runtime.tick(current_time);
    }

    /// 直前の `tick()` 結果を読み取り専用で返す。
    ///
    /// 次の `tick()` まで何度でも参照可能（冪等）。
    pub fn last_result(&self) -> &UpdateResult {
        self.runtime.last_result()
    }

    /// 内部 DolaRuntime への読み取り参照。
    ///
    /// ドキュメントロード状態やサブスクリプション確認に使用する。
    pub fn runtime(&self) -> &DolaRuntime {
        &self.runtime
    }
}

impl Default for DolaAnimator {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for DolaAnimator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DolaAnimator")
            .field("has_runtime", &true)
            .finish()
    }
}

/// 全 DolaAnimator を一括 tick する System。
///
/// Update スケジュール先頭に配置し、消費者システムは
/// `.after(tick_dola_animators)` で順序依存を宣言する。
///
/// # スケジュール配置
///
/// ```rust,ignore
/// app.add_systems(Update, tick_dola_animators);
/// app.add_systems(Update, my_consumer.after(tick_dola_animators));
/// ```
pub fn tick_dola_animators(
    mut query: Query<&mut DolaAnimator>,
    frame_time: Res<FrameTime>,
) {
    for mut animator in query.iter_mut() {
        animator.tick(frame_time.0);
    }
}

#[cfg(test)]
mod tests {
    //! W8-T2: `DolaAnimator` Component ラッパーと `tick_dola_animators` System の
    //! ECS 統合契約（dola 統合）を特性化する。
    //!
    //! dola ランタイム本体（補間・インスタンス管理・`UpdateResult` 生成）の正当性は
    //! `dola` クレート側（D1/D2/D3 領域）で検証済み。本 in-source テストは wintf 側の
    //! 統合点 ＝ (a) Component ラッパーの委譲契約（new/default/with_runtime/tick/
    //! last_result/runtime/Debug）と (b) `tick_dola_animators` System が `Res<FrameTime>`
    //! で全エンティティを駆動する配線、をデバイス非依存に固定する。
    //!
    //! いずれも bevy `World`/`Schedule` 上の決定論的処理（Win32/GPU 非依存）であり、
    //! 注入時刻（`FrameTime`）に対して完全に再現可能。

    use super::*;
    use bevy_ecs::schedule::Schedule;
    use bevy_ecs::world::World;
    use dola::runtime::{DolaRuntime, EvaluatedValue};
    use dola::{
        AnimationVariableDef, DolaDocumentBuilder, StoryboardBuilder, StoryboardEntry,
        TransitionDef, TransitionRef, TransitionValue,
    };

    /// `opacity` を 0.0→1.0 へ 1 秒でフェードさせる単純なアニメーションをロードした
    /// `DolaRuntime` を返すヘルパー（`start` は未実行 ＝ 呼び出し側で開始する）。
    fn loaded_runtime() -> DolaRuntime {
        let doc = DolaDocumentBuilder::new("1.0")
            .variable(
                "opacity",
                AnimationVariableDef::Float {
                    initial: 0.0,
                    min: None,
                    max: None,
                },
            )
            .transition(
                "fade_t",
                TransitionDef {
                    from: Some(TransitionValue::Scalar(0.0)),
                    to: Some(TransitionValue::Scalar(1.0)),
                    duration: Some(1.0),
                    ..Default::default()
                },
            )
            .storyboard(
                "fade_in",
                StoryboardBuilder::new()
                    .entry(StoryboardEntry {
                        variable: Some("opacity".into()),
                        transition: Some(TransitionRef::Named("fade_t".into())),
                        ..Default::default()
                    })
                    .build(),
            )
            .build()
            .unwrap();

        let mut rt = DolaRuntime::new();
        rt.subscribe("opacity");
        rt.load_document(doc).unwrap();
        rt
    }

    /// 0.0 時点で `fade_in` を開始済みの `DolaAnimator` を返すヘルパー。
    fn started_animator() -> DolaAnimator {
        let mut rt = loaded_runtime();
        rt.start("fade_in", 0.0).unwrap();
        DolaAnimator::with_runtime(rt)
    }

    // -----------------------------------------------------------------
    // DolaAnimator ラッパーの委譲契約
    // -----------------------------------------------------------------

    #[test]
    fn new_animator_has_empty_last_result() {
        // tick 未実行の新規 animator は changes/triggered ともに空
        // （DolaRuntime::new の last_update_result 初期値が委譲される）。
        let animator = DolaAnimator::new();
        let result = animator.last_result();
        assert!(result.changes.is_empty(), "new() must start with no changes");
        assert!(
            result.triggered.is_empty(),
            "new() must start with no triggered results"
        );
    }

    #[test]
    fn default_animator_matches_new_empty_result() {
        // Default::default は new() に委譲されるため last_result も同様に空。
        let animator = DolaAnimator::default();
        let result = animator.last_result();
        assert!(
            result.changes.is_empty() && result.triggered.is_empty(),
            "default() must delegate to new() and have an empty result"
        );
    }

    #[test]
    fn with_runtime_preserves_supplied_runtime_state() {
        // with_runtime に渡したロード済みランタイムが animator 内部に保持され、
        // runtime() 経由で評価可能であること（購読済み変数が tick で変化する）。
        let animator = started_animator();
        // 渡したランタイムが保持されている → tick すると opacity の変化が出る。
        let mut animator = animator;
        animator.tick(0.5);
        assert!(
            !animator.last_result().changes.is_empty(),
            "with_runtime must retain the loaded/started runtime (opacity should change at t=0.5)"
        );
    }

    #[test]
    fn tick_updates_last_result_from_empty_to_changes() {
        // tick 前は空、tick(0.5) 後は opacity の差分変化が現れる
        // （tick が runtime.tick へ委譲し last_result を更新する契約）。
        let mut animator = started_animator();
        assert!(
            animator.last_result().changes.is_empty(),
            "before tick(): last_result must be empty"
        );

        animator.tick(0.5);
        assert!(
            !animator.last_result().changes.is_empty(),
            "after tick(0.5): opacity change must be present"
        );
    }

    #[test]
    fn last_result_is_idempotent_across_repeated_reads() {
        // tick 後の last_result は次の tick まで何度読んでも同一（冪等）。
        let mut animator = started_animator();
        animator.tick(0.5);

        let first_len = animator.last_result().changes.len();
        let second_len = animator.last_result().changes.len();
        let third_len = animator.last_result().changes.len();
        assert_eq!(
            first_len, second_len,
            "last_result() must be stable across repeated reads"
        );
        assert_eq!(second_len, third_len, "last_result() must remain idempotent");
        assert!(first_len > 0, "midpoint tick must produce at least one change");
    }

    #[test]
    fn tick_advancing_time_replaces_previous_result() {
        // last_result は最新 tick の差分へ「置換」される（累積ではない）。
        // tick(1.0) で opacity=1.0 を配信した後、tick(2.0) では値が 1.0 のまま
        // （終端で安定）なので差分は空になる。これは前回の非空結果が最新 tick の
        // 空差分へ置換された証跡（蓄積なら 1.0 の変化が残り続けるはず）。
        // 観測値（probe）: start=0 → t=1.0:[(0,1.0)] / t=2.0:[]。
        let mut animator = started_animator();

        animator.tick(1.0);
        assert_eq!(
            animator.last_result().changes.len(),
            1,
            "tick(1.0) must deliver the opacity end value once"
        );

        animator.tick(2.0);
        assert!(
            animator.last_result().changes.is_empty(),
            "tick(2.0) settles at the same value: last_result is replaced by the latest (empty-diff) tick, not accumulated"
        );
    }

    #[test]
    fn repeated_tick_at_same_value_yields_empty_diff_after_first_delivery() {
        // 差分検出は「前回配信値」基準（subscription_manager::diff_and_update）。
        // 開始前時刻でも初回 tick は初期値 0.0 を「初回配信」として 1 件報告し、
        // 同一時刻（=同一値）の再 tick は差分ゼロになる。tick が時刻をそのまま
        // runtime へ渡し last_result が前回配信との差分を反映する契約を固定する。
        // 観測値（probe）: start=1 → t=0.5:[(0,0.0)] / 同値再 tick:[]。
        let mut rt = loaded_runtime();
        rt.start("fade_in", 1.0).unwrap(); // 1.0 開始（0.5 はまだ開始前 ＝ 初期値 0.0）
        let mut animator = DolaAnimator::with_runtime(rt);

        animator.tick(0.5);
        assert_eq!(
            animator.last_result().changes.len(),
            1,
            "first tick delivers the initial value (0.0) once, even before start_time"
        );

        animator.tick(0.5); // 同一時刻 → 同一値 → 前回配信と一致
        assert!(
            animator.last_result().changes.is_empty(),
            "re-ticking the same time yields no diff (value unchanged from last delivery)"
        );
    }

    #[test]
    fn runtime_accessor_returns_inner_runtime_reference() {
        // runtime() は内部 DolaRuntime への読み取り参照を返す。
        // 同一インスタンスを指す（last_result の同一性で確認）。
        let animator = started_animator();
        let inner = animator.runtime();
        // runtime().last_result と animator.last_result は同じ初期状態（空）を指す。
        assert!(
            std::ptr::eq(inner.last_result(), animator.last_result()),
            "runtime() must expose the same inner runtime backing last_result()"
        );
    }

    #[test]
    fn debug_format_reports_has_runtime_true() {
        // Debug 実装は型名と has_runtime:true を含む（中身は伏せる固定様式）。
        let animator = DolaAnimator::new();
        let debug = format!("{animator:?}");
        assert!(
            debug.contains("DolaAnimator"),
            "Debug must include the type name: {debug}"
        );
        assert!(
            debug.contains("has_runtime") && debug.contains("true"),
            "Debug must report has_runtime: true: {debug}"
        );
    }

    // -----------------------------------------------------------------
    // tick_dola_animators System の配線（実 System を Schedule で駆動）
    // -----------------------------------------------------------------

    /// `tick_dola_animators` のみを実行する Schedule を構築する。
    fn tick_schedule() -> Schedule {
        let mut s = Schedule::default();
        s.add_systems(tick_dola_animators);
        s
    }

    #[test]
    fn tick_system_on_empty_world_is_noop() {
        // DolaAnimator エンティティゼロでも FrameTime さえあれば System は
        // パニックせず走る（Query が空でループ本体が実行されないだけ）。
        let mut world = World::new();
        world.insert_resource(FrameTime(0.5));

        let mut schedule = tick_schedule();
        schedule.run(&mut world); // パニックしないこと自体が契約
                                  // 観測可能な状態変化はない（エンティティ不在）。
        assert_eq!(
            world.query::<&DolaAnimator>().iter(&world).count(),
            0,
            "empty world must remain empty after the tick system"
        );
    }

    #[test]
    fn tick_system_drives_all_animators_from_frame_time() {
        // 実 tick_dola_animators System が複数エンティティすべてを
        // Res<FrameTime> の時刻で一括 tick することを特性化する
        // （既存統合テストは loop を手書きしていたが、本テストは実 System を駆動）。
        let mut world = World::new();
        world.insert_resource(FrameTime(0.5));

        let e1 = world.spawn(started_animator()).id();
        let e2 = world.spawn(started_animator()).id();

        let mut schedule = tick_schedule();
        schedule.run(&mut world);

        for (label, e) in [("e1", e1), ("e2", e2)] {
            let animator = world.get::<DolaAnimator>(e).unwrap();
            assert!(
                !animator.last_result().changes.is_empty(),
                "{label} must be ticked by the system at FrameTime(0.5)"
            );
        }
    }

    #[test]
    fn tick_system_uses_injected_frame_time_value() {
        // System が tick へ渡す時刻は Res<FrameTime>.0 そのもの。
        // start=0 のフェードに対し FrameTime(0.5) を注入すると、配信される
        // opacity 値は「その注入時刻での補間値」= 0.5 になることで時刻反映を固定する
        // （別の時刻なら別の値になるため、注入時刻が tick へ流れている証跡）。
        // 観測値（probe）: start=0 → t=0.5:[(0, Float(0.5))]。
        let mut world = World::new();
        world.insert_resource(FrameTime(0.5));
        let e = world.spawn(started_animator()).id();

        let mut schedule = tick_schedule();
        schedule.run(&mut world);

        let changes = &world.get::<DolaAnimator>(e).unwrap().last_result().changes;
        assert_eq!(changes.len(), 1, "one opacity change expected at t=0.5");
        let (var_id, value) = &changes[0];
        assert_eq!(*var_id, 0, "subscribed opacity has variable_id 0");
        match value {
            EvaluatedValue::Float(v) => assert!(
                (*v - 0.5).abs() < 1e-9,
                "FrameTime(0.5) must interpolate opacity to 0.5 (system forwards injected time), got {v}"
            ),
            other => panic!("expected Float opacity value, got {other:?}"),
        }
    }
}

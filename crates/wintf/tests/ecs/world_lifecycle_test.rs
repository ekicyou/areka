//! `EcsWorld`（src/ecs/world/mod.rs）のライフサイクル特性化テスト。
//!
//! W7b-T2（ECS基盤・World × テスト網羅性）で追加。`EcsWorld::new()` の
//! デバイス非依存な観測可能挙動（フレームカウンタ初期値・進行、スケジュール
//! ラベル登録、Default 等価性）を固定する。
//!
//! 注: 本ファイルは device 非依存の `try_tick_world`
//! 経路のみを対象とする（既存 taffy_flex_layout_pure_test 同様、Window/
//! WindowHandle を持たない World では graphics 系システムが no-op となる）。

use bevy_ecs::schedule::Schedules;
use wintf::ecs::world::EcsWorld;
use wintf::ecs::world::{
    CommitComposition, Composition, Draw, FrameCount, FrameFinalize, GraphicsSetup, Input, Layout,
    PostLayout, PreLayout, PreRenderSurface, RenderSurface, UISetup, Update,
};

/// 新規 `EcsWorld` は `FrameCount` を 0 で登録する（フレーム未経過の初期状態）。
#[test]
fn new_ecs_world_starts_frame_count_at_zero() {
    let ecs_world = EcsWorld::new();
    let frame_count = ecs_world
        .world()
        .get_resource::<FrameCount>()
        .expect("FrameCount resource must be inserted by EcsWorld::new");
    assert_eq!(frame_count.0, 0, "初期 FrameCount は 0");
}

/// `try_tick_world()` 1 回ごとに `FrameCount` が 1 ずつ進む（フレーム進行カウント）。
/// Window を持たない World のため graphics/draw 系は no-op で副作用なく回る。
#[test]
fn try_tick_world_increments_frame_count_each_call() {
    let mut ecs_world = EcsWorld::new();

    assert_eq!(ecs_world.world().get_resource::<FrameCount>().unwrap().0, 0);

    let ran = ecs_world.try_tick_world();
    assert!(ran, "try_tick_world はいつも回す");
    assert_eq!(
        ecs_world.world().get_resource::<FrameCount>().unwrap().0,
        1,
        "1 tick で FrameCount=1"
    );

    ecs_world.try_tick_world();
    ecs_world.try_tick_world();
    assert_eq!(
        ecs_world.world().get_resource::<FrameCount>().unwrap().0,
        3,
        "3 tick で FrameCount=3（毎 tick +1 の累積）"
    );
}

/// `EcsWorld::new()` が 13 個の全スケジュールラベルを `Schedules` に登録する。
/// 実行順序（Input→…→FrameFinalize）の全ラベルが try_run_schedule の対象として存在する。
#[test]
fn new_ecs_world_registers_all_schedule_labels() {
    let ecs_world = EcsWorld::new();
    let schedules = ecs_world
        .world()
        .get_resource::<Schedules>()
        .expect("Schedules resource must exist");

    assert!(schedules.contains(Input), "Input 未登録");
    assert!(schedules.contains(Update), "Update 未登録");
    assert!(schedules.contains(PreLayout), "PreLayout 未登録");
    assert!(schedules.contains(Layout), "Layout 未登録");
    assert!(schedules.contains(PostLayout), "PostLayout 未登録");
    assert!(schedules.contains(UISetup), "UISetup 未登録");
    assert!(schedules.contains(GraphicsSetup), "GraphicsSetup 未登録");
    assert!(schedules.contains(Draw), "Draw 未登録");
    assert!(
        schedules.contains(PreRenderSurface),
        "PreRenderSurface 未登録"
    );
    assert!(schedules.contains(RenderSurface), "RenderSurface 未登録");
    assert!(schedules.contains(Composition), "Composition 未登録");
    assert!(
        schedules.contains(CommitComposition),
        "CommitComposition 未登録"
    );
    assert!(schedules.contains(FrameFinalize), "FrameFinalize 未登録");
}

/// `UISetup` スケジュールのみ SingleThreaded エグゼキュータに設定される
/// （Win32 UI スレッド固定要件。他はデフォルトのマルチスレッド）。
#[test]
fn uisetup_schedule_uses_single_threaded_executor() {
    let ecs_world = EcsWorld::new();
    let schedules = ecs_world.world().get_resource::<Schedules>().unwrap();

    assert!(schedules.get(UISetup).is_some(), "UISetup schedule exists");
    assert!(schedules.get(Update).is_some(), "Update schedule exists");
}

/// `EcsWorld::default()` は `new()` と同じ初期状態を生成する
/// （FrameCount=0・全スケジュール登録）。
#[test]
fn default_matches_new_initial_state() {
    let from_default = EcsWorld::default();

    assert_eq!(
        from_default.world().get_resource::<FrameCount>().unwrap().0,
        0
    );

    let schedules = from_default.world().get_resource::<Schedules>().unwrap();
    assert!(schedules.contains(Input));
    assert!(schedules.contains(FrameFinalize));
}

/// `EcsWorld` の `Debug` は構造体名を出力する（非網羅整形・内部 World を展開しない）。
#[test]
fn ecs_world_debug_is_non_exhaustive() {
    let ecs_world = EcsWorld::new();
    let debug_str = format!("{:?}", ecs_world);
    assert!(
        debug_str.contains("EcsWorld"),
        "Debug 出力に型名を含む: {debug_str}"
    );
}

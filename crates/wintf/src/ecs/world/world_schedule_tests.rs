//! 単一スレッドで回すスケジュールの一覧のテスト（audit 35 番・要件 3.1/3.2）。
//!
//! bevy 0.19 には実行器を読む公開の口が無いので、`set_executor` の 1 行を消しても
//! 実行のテストは緑のまま通る。そこで一覧の中身と、`EcsWorld::new` の本文が一覧から
//! `set_executor` を回していること（本文を読む構造の検査・`tick_diag_tests.rs` と同じやり方）を固定する。

use super::*;

/// 一覧は窓を作る `UISetup` と WUC に触る 5 本の 6 本で、この順。
#[test]
fn single_threaded_schedules_are_the_six_wuc_and_window_schedules() {
    let expected = [
        UISetup.intern(),
        GraphicsSetup.intern(),
        PreRenderSurface.intern(),
        RenderSurface.intern(),
        Composition.intern(),
        CommitComposition.intern(),
    ];
    assert_eq!(single_threaded_schedules(), expected);
}

/// 一覧のスケジュールは全部 `EcsWorld::new` が入れている（一覧が空振りしない）。
#[test]
fn every_single_threaded_schedule_is_registered() {
    let ecs = EcsWorld::new();
    let schedules = ecs.world().resource::<Schedules>();
    for label in single_threaded_schedules() {
        assert!(schedules.contains(label), "{label:?} が登録されていない");
    }
}

/// `EcsWorld::new` は `set_executor` を一覧からの 1 か所でだけ呼ぶ（個別の呼び出しへ戻らない）。
#[test]
fn ecs_world_new_sets_executor_only_from_the_list() {
    let src = include_str!("mod.rs");
    let at = src
        .find("pub fn new() -> Self")
        .expect("mod.rs に EcsWorld::new が無い");
    let rest = &src[at..];
    let end = rest[1..]
        .find("pub fn ")
        .expect("EcsWorld::new の後ろに次の関数が無い");
    let body = &rest[..end];

    assert_eq!(
        body.matches("set_executor(").count(),
        1,
        "set_executor は一覧から回す 1 か所だけに在るべき"
    );
    let list = body
        .find("in single_threaded_schedules()")
        .expect("EcsWorld::new が一覧を回していない");
    let set = body.find("set_executor(").unwrap();
    assert!(list < set, "set_executor は一覧の繰り返しの中に在るべき");
}

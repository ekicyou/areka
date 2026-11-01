//! 表示成立点の倍率の記録（`presenter/scale_change.rs`）の檻（GPU・`shell-scale-linkage` 要件 3.3・
//! 3.4・4.2・4.4・5.6・5.7）。
//!
//! | 檻 | 固定する性質 |
//! |---|---|
//! | 初回の表示 | 前が無い記録が 1 件（k と窓の DPI） |
//! | 同じ DPI の再表示 | 記録なし |
//! | DPI の替わり（`refresh_scale`） | 前のある記録が 1 件 |
//! | 作者 DPI の違う差し替え | 前が無い記録が 1 件（起点は引き継がない） |
//! | 失敗した表示 | 記録なし（絵にかからなかった k は出ない） |
//! | 取り出しの 2 回目 | 空 |

use super::*;

use std::time::Duration;

use super::test_support::{
    build_target_assets, make_world_with_gpu, set_window_dpi, show_ok, spawn_window_with_dpi,
};

/// 資産の面の番号（`build_target_assets` が作る唯一の面）。
const FACE: u32 = 1000;

fn scale(n: u32, d: u32, dpi: u16) -> AppliedScale {
    AppliedScale {
        ratio: ScaleRatio::new(n, d).unwrap(),
        dpi_x: Some(dpi),
    }
}

/// 窓の DPI `dpi`・作者 DPI `author_dpi` の対象 0 を装着した表示の主と窓。
fn attached(world: &mut World, dpi: u16, author_dpi: u16) -> (EmoPresenter, Entity) {
    let window = spawn_window_with_dpi(world, dpi);
    let (emo_world, atlas, _) = build_target_assets(3, 2, 0x51);
    let mut presenter = EmoPresenter::new();
    presenter
        .attach_target(world, TargetId(0), window, emo_world, atlas, author_dpi)
        .expect("attach_target 失敗");
    (presenter, window)
}

#[test]
fn first_show_records_a_change_without_before() {
    let mut world = make_world_with_gpu();
    let (mut presenter, _) = attached(&mut world, 144, 96);
    show_ok(&mut presenter, &mut world, TargetId(0), FACE);
    assert_eq!(
        presenter.take_scale_changes(),
        vec![ScaleChange {
            target: TargetId(0),
            before: None,
            after: scale(3, 2, 144),
        }],
        "装着して初めての表示は前が無い記録"
    );
}

#[test]
fn reshow_at_the_same_dpi_records_nothing() {
    let mut world = make_world_with_gpu();
    let (mut presenter, _) = attached(&mut world, 96, 96);
    show_ok(&mut presenter, &mut world, TargetId(0), FACE);
    assert_eq!(presenter.take_scale_changes().len(), 1);
    show_ok(&mut presenter, &mut world, TargetId(0), FACE);
    assert!(
        presenter.take_scale_changes().is_empty(),
        "同じ（k・DPI）の再表示は記録しない"
    );
}

#[test]
fn dpi_change_records_a_change_with_before() {
    let mut world = make_world_with_gpu();
    let (mut presenter, window) = attached(&mut world, 96, 96);
    show_ok(&mut presenter, &mut world, TargetId(0), FACE);
    presenter.take_scale_changes();
    set_window_dpi(&mut world, window, 192);
    presenter.refresh_scale(&mut world, TargetId(0));
    assert_eq!(
        presenter.take_scale_changes(),
        vec![ScaleChange {
            target: TargetId(0),
            before: Some(scale(1, 1, 96)),
            after: scale(2, 1, 192),
        }],
        "DPI の替わりは起点を前に持つ記録"
    );
}

#[test]
fn replacement_with_another_author_dpi_records_a_change_without_before() {
    let mut world = make_world_with_gpu();
    let (mut presenter, _) = attached(&mut world, 192, 96);
    show_ok(&mut presenter, &mut world, TargetId(0), FACE);
    presenter.take_scale_changes();

    let (emo_world, atlas, _) = build_target_assets(3, 2, 0x52);
    let (tx, rx) = reply_channel::<PresentOutcome>();
    presenter.apply(
        &mut world,
        PresentCommand::ReplaceTarget {
            target: TargetId(0),
            emo_world: Box::new(emo_world),
            atlas,
            author_dpi: 192,
            show: Some((FACE, BindSet::default())),
            reply: Some(tx),
        },
    );
    assert!(matches!(
        rx.recv_timeout(Duration::from_secs(10)),
        Ok(Ok(()))
    ));
    assert_eq!(
        presenter.take_scale_changes(),
        vec![ScaleChange {
            target: TargetId(0),
            before: None,
            after: scale(1, 1, 192),
        }],
        "差し替えは起点を引き継がない＝前が無い記録"
    );
}

#[test]
fn failed_show_records_nothing() {
    let mut world = make_world_with_gpu();
    let (mut presenter, window) = attached(&mut world, 96, 96);
    show_ok(&mut presenter, &mut world, TargetId(0), FACE);
    presenter.take_scale_changes();
    // k の変わる DPI で、無い面を出す＝表示成立点より前で失敗する。
    set_window_dpi(&mut world, window, 192);
    let (tx, rx) = reply_channel::<PresentOutcome>();
    presenter.apply(
        &mut world,
        PresentCommand::ShowSurface {
            target: TargetId(0),
            surface_id: 4242,
            binds: BindSet::default(),
            pattern: PatternState::default(),
            reply: Some(tx),
        },
    );
    assert!(matches!(
        rx.recv_timeout(Duration::from_secs(10)),
        Ok(Err(_))
    ));
    assert!(
        presenter.take_scale_changes().is_empty(),
        "絵にかからなかった k は記録に出ない"
    );
    assert_eq!(
        presenter.applied_ratio(TargetId(0)),
        Some(ScaleRatio::ONE),
        "失敗は前の k を保つ"
    );
}

#[test]
fn second_take_is_empty() {
    let mut world = make_world_with_gpu();
    let (mut presenter, _) = attached(&mut world, 96, 96);
    show_ok(&mut presenter, &mut world, TargetId(0), FACE);
    assert_eq!(presenter.take_scale_changes().len(), 1);
    assert!(
        presenter.take_scale_changes().is_empty(),
        "取り出しの 2 回目は空"
    );
}

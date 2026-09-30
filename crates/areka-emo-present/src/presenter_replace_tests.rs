//! 装着の置き換え（`presenter/replace.rs`）の檻（GPU・要件 4.3・4.6・11.9）。
//!
//! | 檻 | 固定する性質 |
//! |---|---|
//! | 登録を消す口 | 表から外れ・装着の子 2 つが消え・引き継ぐ値（窓・可視性の持ち主・適用済みの k・素の大きさ・保留中の窓寸の要求）がそのまま返る |
//! | 未登録 | `error!` 1 件と `Err(TargetNotAttached)`・他の登録と窓の子は不変 |
//! | 置き換え（`ReplaceTarget`） | 古い子が 0・新しい子が 1 組・外部所有の引き継ぎで隠れたまま・同じ物理寸なら窓寸の要求なし／違えば要求あり（k=1 と k=2 の同じ手順）・作者 DPI は新しい値・無い面は `error!` 1 件で表示なし・未登録は `Err` で表は不変 |

use super::*;

use std::time::Duration;

use bevy_ecs::hierarchy::Children;
use wintf::ecs::Visual;

use super::test_support::{
    CapturedEvent, build_target_assets, capture, make_world_with_gpu, mount_entities, show_ok,
    spawn_window_with_dpi,
};

/// 表示を確立する面（`build_target_assets` の唯一の面）。
const FACE: u32 = 1000;
/// 面の素の大きさ。
const FACE_SIZE: (u32, u32) = (4, 3);

fn child_count(world: &World, window: Entity) -> usize {
    world.get::<Children>(window).map_or(0, |c| c.len())
}

/// 窓 DPI `dpi`・作者 DPI 96 で target を登録する（表示はまだ確立しない）。
fn attached_target(
    world: &mut World,
    presenter: &mut EmoPresenter,
    target: TargetId,
    dpi: u16,
) -> Entity {
    let window = spawn_window_with_dpi(world, dpi);
    let (emo_world, atlas, _) = build_target_assets(FACE_SIZE.0, FACE_SIZE.1, 0);
    presenter
        .attach_target(world, target, window, emo_world, atlas, 96)
        .expect("attach_target");
    window
}

/// 要件 4.6: 登録を消す口は表から外し、装着の子 2 つを消し、引き継ぐ値をそのまま返す。
///
/// 外部所有・k=2 の表示で、`pending_resize` は取り出さずに残す（引き継ぐ値がどれも既定値と
/// 違う形にして、「既定値を返しているだけ」で緑にならないようにする）。
#[test]
fn detach_target_unregisters_despawns_children_and_returns_the_carried_values() {
    let mut world = make_world_with_gpu();
    let mut presenter = EmoPresenter::new();
    let target = TargetId(0);
    let window = attached_target(&mut world, &mut presenter, target, 192);
    presenter
        .set_visibility_ownership(target, VisibilityOwnership::External)
        .expect("ownership");
    show_ok(&mut presenter, &mut world, target, FACE);
    let (surface, slot) = mount_entities(&presenter, target);
    assert_eq!(child_count(&world, window), 2, "較正: 表示の後は窓の子が 2");

    let state = presenter
        .detach_target(&mut world, target)
        .expect("登録済みの target は外せる");

    assert_eq!(state.window, window);
    assert_eq!(state.ownership, VisibilityOwnership::External);
    assert_eq!(state.applied, Some(ScaleRatio::new(2, 1).unwrap()));
    assert_eq!(state.native_size, Some(FACE_SIZE));
    assert_eq!(
        state.pending_resize,
        Some((FACE_SIZE.0 * 2, FACE_SIZE.1 * 2)),
        "取り出していない窓寸の要求は引き継ぐ"
    );

    assert_eq!(presenter.target_visible(target), None, "表から外れていない");
    assert_eq!(child_count(&world, window), 0, "窓の子が残っている");
    assert!(world.get_entity(surface).is_err());
    assert!(world.get_entity(slot).is_err());
}

/// 要件 4.6: 一度も表示していない（装着の子が無い）target も外せる。子を消す対象は無い。
#[test]
fn detach_target_without_a_mount_returns_the_registration_defaults() {
    let mut world = make_world_with_gpu();
    let mut presenter = EmoPresenter::new();
    let target = TargetId(0);
    let window = attached_target(&mut world, &mut presenter, target, 96);

    let state = presenter.detach_target(&mut world, target).expect("外せる");

    assert_eq!(state.window, window);
    assert_eq!(state.ownership, VisibilityOwnership::CommandDriven);
    assert_eq!(state.applied, None);
    assert_eq!(state.native_size, None);
    assert_eq!(state.pending_resize, None);
    assert_eq!(presenter.target_visible(target), None);
}

/// 要件 4.6: 未登録の target は `error!` 1 件と `Err(TargetNotAttached)`。表は変わらない
/// （他の登録の表示・窓の子はそのまま）。同じ target を 2 度外すのも未登録として扱う。
#[test]
fn detach_target_of_an_unregistered_target_fails_and_leaves_the_table_unchanged() {
    let mut world = make_world_with_gpu();
    let mut presenter = EmoPresenter::new();
    let kept = TargetId(0);
    let window = attached_target(&mut world, &mut presenter, kept, 96);
    show_ok(&mut presenter, &mut world, kept, FACE);
    let (surface, slot) = mount_entities(&presenter, kept);

    let missing = TargetId(9);
    let (result, events) = capture(|| presenter.detach_target(&mut world, missing));

    assert!(
        matches!(result, Err(PresentError::TargetNotAttached(t)) if t == missing),
        "未登録は TargetNotAttached: {result:?}"
    );
    let errors = events
        .iter()
        .filter(|e| e.level == tracing::Level::ERROR)
        .count();
    assert_eq!(errors, 1, "未登録の error! はちょうど 1 件: {events:?}");

    assert_eq!(presenter.target_visible(kept), Some(true));
    assert_eq!(presenter.current_surface_id(kept), Some(FACE));
    assert_eq!(mount_entities(&presenter, kept), (surface, slot));
    assert_eq!(child_count(&world, window), 2, "他の登録の窓の子は不変");

    // 2 度目は未登録。
    presenter
        .detach_target(&mut world, kept)
        .expect("1 度目は外せる");
    assert!(matches!(
        presenter.detach_target(&mut world, kept),
        Err(PresentError::TargetNotAttached(t)) if t == kept
    ));
}

// ── 置き換え（`PresentCommand::ReplaceTarget`・要件 4.1〜4.3・4.5・4.6・5.6・11.9）──────────────

/// 面 [`FACE`] だけを持つ大きさ `size` の新しい資産で `target` を置き換え、返信を返す。
fn replace(
    presenter: &mut EmoPresenter,
    world: &mut World,
    target: TargetId,
    size: (u32, u32),
    author_dpi: u16,
    show: Option<u32>,
) -> PresentOutcome {
    let (emo_world, atlas, _) = build_target_assets(size.0, size.1, 7);
    let (tx, rx) = reply_channel::<PresentOutcome>();
    presenter.apply(
        world,
        PresentCommand::ReplaceTarget {
            target,
            emo_world: Box::new(emo_world),
            atlas,
            author_dpi,
            show: show.map(|id| (id, BindSet::default())),
            reply: Some(tx),
        },
    );
    rx.recv_timeout(Duration::from_secs(10))
        .expect("ReplaceTarget は同じ呼び出しの中で返信する")
}

/// 本クレート（present）が出した `error!` の件数。
///
/// 無い面の合成では合成器（`areka_emo_compose`）も自分の `error!` を今日から 1 件出す。それは
/// present の外の既存の記録なので数えず、present が残す件数だけを固定する。
fn present_error_count(events: &[CapturedEvent]) -> usize {
    events
        .iter()
        .filter(|e| e.level == tracing::Level::ERROR && e.target.starts_with("areka_emo_present"))
        .count()
}

/// 要件 4.1・4.6: 置き換えの後、窓の子は新しい 1 組だけ（古い 2 つは消えている）で、新しい面が
/// 見えている（指令駆動の target）。
#[test]
fn replace_target_leaves_only_the_new_pair_of_children() {
    let mut world = make_world_with_gpu();
    let mut presenter = EmoPresenter::new();
    let target = TargetId(0);
    let window = attached_target(&mut world, &mut presenter, target, 96);
    show_ok(&mut presenter, &mut world, target, FACE);
    let (old_surface, old_slot) = mount_entities(&presenter, target);

    let outcome = replace(&mut presenter, &mut world, target, (6, 5), 96, Some(FACE));

    assert!(matches!(outcome, Ok(())), "置き換えは Ok: {outcome:?}");
    assert!(world.get_entity(old_surface).is_err(), "古い面が残っている");
    assert!(
        world.get_entity(old_slot).is_err(),
        "古いスロットが残っている"
    );
    let (surface, slot) = mount_entities(&presenter, target);
    let children: Vec<Entity> = world
        .get::<Children>(window)
        .map_or_else(Vec::new, |c| c.iter().copied().collect());
    assert_eq!(children.len(), 2, "窓の子は新しい 1 組だけ: {children:?}");
    assert!(children.contains(&surface) && children.contains(&slot));
    assert_eq!(presenter.target_visible(target), Some(true));
    assert_eq!(presenter.current_surface_id(target), Some(FACE));
    assert_eq!(presenter.target_physical_size(target), Some((6, 5)));
}

/// 要件 4.3: 外部所有（バルーン窓）の持ち主は引き継がれ、置き換えの直後も隠れたまま（新しい子は
/// entity の段でも不可視）。
#[test]
fn replace_target_keeps_external_ownership_and_stays_hidden() {
    let mut world = make_world_with_gpu();
    let mut presenter = EmoPresenter::new();
    let target = TargetId(1);
    attached_target(&mut world, &mut presenter, target, 96);
    presenter
        .set_visibility_ownership(target, VisibilityOwnership::External)
        .expect("ownership");
    show_ok(&mut presenter, &mut world, target, FACE);

    let outcome = replace(&mut presenter, &mut world, target, (6, 5), 96, Some(FACE));

    assert!(matches!(outcome, Ok(())), "置き換えは Ok: {outcome:?}");
    assert_eq!(presenter.target_visible(target), Some(false), "隠れたまま");
    assert_eq!(
        presenter.current_surface_id(target),
        Some(FACE),
        "確立はする"
    );
    let (surface, slot) = mount_entities(&presenter, target);
    for entity in [surface, slot] {
        assert!(
            !world.get::<Visual>(entity).expect("Visual").is_visible,
            "新しい子が可視で作られた"
        );
    }
    // 引き継いだ持ち主は新しい登録に載っている（もう 1 度外すと External が返る）。
    let carried = presenter.detach_target(&mut world, target).expect("外せる");
    assert_eq!(carried.ownership, VisibilityOwnership::External);
}

/// 要件 4.3・4.5・11.9: 窓寸の要求は引き継いだ前回の物理寸と比べて積まれる——同じ物理寸なら
/// 要求なし、違えば新しい物理寸の要求あり。拡大率 1（96）と 2（192）で同じ手順が同じ答えを出す。
#[test]
fn replace_target_requests_resize_only_when_the_physical_size_changes() {
    for (dpi, k) in [(96u16, 1u32), (192, 2)] {
        let mut world = make_world_with_gpu();
        let mut presenter = EmoPresenter::new();
        let target = TargetId(0);
        attached_target(&mut world, &mut presenter, target, dpi);
        show_ok(&mut presenter, &mut world, target, FACE);
        assert_eq!(
            presenter.take_pending_resize(target),
            Some((FACE_SIZE.0 * k, FACE_SIZE.1 * k)),
            "較正: 初回表示は要求を積む（dpi {dpi}）"
        );

        // 同じ物理寸: 要求なし。
        let outcome = replace(
            &mut presenter,
            &mut world,
            target,
            FACE_SIZE,
            96,
            Some(FACE),
        );
        assert!(matches!(outcome, Ok(())), "dpi {dpi}: {outcome:?}");
        assert_eq!(
            presenter.take_pending_resize(target),
            None,
            "dpi {dpi}: 同じ物理寸で窓寸の要求が積まれた"
        );
        assert_eq!(
            presenter.applied_ratio(target),
            Some(ScaleRatio::new(k, 1).unwrap()),
            "dpi {dpi}: 拡大率が違う"
        );

        // 違う物理寸: 新しい物理寸の要求あり。
        let outcome = replace(&mut presenter, &mut world, target, (6, 5), 96, Some(FACE));
        assert!(matches!(outcome, Ok(())), "dpi {dpi}: {outcome:?}");
        assert_eq!(
            presenter.take_pending_resize(target),
            Some((6 * k, 5 * k)),
            "dpi {dpi}: 物理寸が変わったのに要求が無い"
        );
        assert_eq!(presenter.target_physical_size(target), Some((6 * k, 5 * k)));
    }
}

/// 要件 4.3: 取り出されていない窓寸の要求は、同じ物理寸の置き換えを越えて生き残る（引き継ぎ）。
#[test]
fn replace_target_carries_an_untaken_resize_request() {
    let mut world = make_world_with_gpu();
    let mut presenter = EmoPresenter::new();
    let target = TargetId(0);
    attached_target(&mut world, &mut presenter, target, 192);
    show_ok(&mut presenter, &mut world, target, FACE);

    let outcome = replace(
        &mut presenter,
        &mut world,
        target,
        FACE_SIZE,
        96,
        Some(FACE),
    );

    assert!(matches!(outcome, Ok(())), "{outcome:?}");
    assert_eq!(
        presenter.take_pending_resize(target),
        Some((FACE_SIZE.0 * 2, FACE_SIZE.1 * 2)),
        "未消費の要求が置き換えで失われた"
    );
}

/// design「Present」手順 3・4: `show` が無ければ登録だけ（古い子は消え、新しい子はまだ無い）。
/// 拡大政策は新しい作者 DPI から作る（窓 192・新しい作者 192 → 次の表示の k は 1）。
#[test]
fn replace_target_without_show_only_registers_with_the_new_author_dpi() {
    let mut world = make_world_with_gpu();
    let mut presenter = EmoPresenter::new();
    let target = TargetId(0);
    let window = attached_target(&mut world, &mut presenter, target, 192);
    show_ok(&mut presenter, &mut world, target, FACE);
    assert_eq!(
        presenter.applied_ratio(target),
        Some(ScaleRatio::new(2, 1).unwrap()),
        "較正: 古い作者 96 で k=2"
    );

    let outcome = replace(&mut presenter, &mut world, target, FACE_SIZE, 192, None);

    assert!(matches!(outcome, Ok(())), "{outcome:?}");
    assert_eq!(child_count(&world, window), 0, "登録だけで子が作られた");
    assert_eq!(presenter.target_visible(target), Some(false));
    assert_eq!(presenter.current_surface_id(target), None);

    show_ok(&mut presenter, &mut world, target, FACE);
    assert_eq!(
        presenter.applied_ratio(target),
        Some(ScaleRatio::ONE),
        "作者 DPI の政策が新しい値から作られていない"
    );
    assert_eq!(child_count(&world, window), 2);
}

/// 要件 2.6（今日の「無い面」の扱い）・5.6: 新しいシェルに面が無ければ `error!` 1 件で、その
/// target は表示なしのまま。登録は済んでいるので返信は `Ok`（置き換えの失敗は未登録だけ）。
#[test]
fn replace_target_with_a_missing_face_logs_one_error_and_shows_nothing() {
    let mut world = make_world_with_gpu();
    let mut presenter = EmoPresenter::new();
    let target = TargetId(0);
    let window = attached_target(&mut world, &mut presenter, target, 96);
    show_ok(&mut presenter, &mut world, target, FACE);

    let missing = 3000;
    let (outcome, events) = capture(|| {
        replace(
            &mut presenter,
            &mut world,
            target,
            (6, 5),
            96,
            Some(missing),
        )
    });

    assert!(matches!(outcome, Ok(())), "登録は済んでいる: {outcome:?}");
    assert_eq!(
        present_error_count(&events),
        1,
        "無い面の error! は 1 件: {events:?}"
    );
    assert_eq!(
        child_count(&world, window),
        0,
        "古い子が残ったか新しい子が出た"
    );
    assert_eq!(presenter.target_visible(target), Some(false));
    assert_eq!(presenter.current_surface_id(target), None);
}

/// 要件 4.6・5.6: 未登録の target の置き換えは `error!` 1 件と `Err(TargetNotAttached)`。表は
/// 変わらない（他の登録の表示・窓の子はそのまま）。
#[test]
fn replace_target_of_an_unregistered_target_fails_and_leaves_the_table_unchanged() {
    let mut world = make_world_with_gpu();
    let mut presenter = EmoPresenter::new();
    let kept = TargetId(0);
    let window = attached_target(&mut world, &mut presenter, kept, 96);
    show_ok(&mut presenter, &mut world, kept, FACE);
    let entities = mount_entities(&presenter, kept);

    let missing = TargetId(9);
    let (outcome, events) =
        capture(|| replace(&mut presenter, &mut world, missing, (6, 5), 96, Some(FACE)));

    assert!(
        matches!(outcome, Err(PresentError::TargetNotAttached(t)) if t == missing),
        "未登録は TargetNotAttached: {outcome:?}"
    );
    assert_eq!(
        present_error_count(&events),
        1,
        "未登録の error! は 1 件: {events:?}"
    );
    assert_eq!(presenter.target_visible(missing), None, "未登録のまま");
    assert_eq!(presenter.target_visible(kept), Some(true));
    assert_eq!(mount_entities(&presenter, kept), entities);
    assert_eq!(child_count(&world, window), 2);
}

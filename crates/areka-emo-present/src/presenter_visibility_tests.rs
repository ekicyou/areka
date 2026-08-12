//! 可視性の所有権と、可視化を伴わない表示確立の檻（`areka-P0-balloon-visibility` Requirement 1.1・
//! 1.3・6.1・6.2・6.6・6.8・6.9／design §Testing Strategy > Unit Tests 5「presenter 所有権」）。
//!
//! 檻の軸は 4 つである——⑴ 外部所有では表示指令が可視化を伴わず、配置先（text 層スロット）と面が
//! 確立すること、⑵ 不可視のまま面切替・ループ由来の再指令を何度受けても可視にならず結果だけが
//! 更新されること、⑶ 専用入口 `show_target` を呼んだときにだけ可視になり、その際 k が**現在の**窓 DPI
//! から導出し直されること、⑷ 非表示側の指令は所有権に依らず即時に効くこと（可視化側だけが所有権で
//! ゲートされる非対称）。
//!
//! 既定所有権（`CommandDriven`）の檻を対で置くことで、「ゲートが常に閉じている（＝どの target も
//! 可視化されない）」という恒真化を検出できる形にしてある。

use super::*;

use std::time::Duration;

use areka_actor::reply_channel;

use wintf::ecs::{HitTest, HitTestMode, Visual};

use super::test_support::{
    attach_hit_target, build_target_assets, build_two_face_assets, make_world_with_gpu,
    scaled_golden, set_window_dpi, show_ok, spawn_window_with_dpi,
};

// ── 檻の補助（装着実体の実値を読む）────────────────────────────────────────────────────
// presenter の私有状態（`visible`）だけを見ると「照会は false を返すが entity は可視」という
// 見かけ倒しを見逃す。可視性の主張は必ず **entity の実 component 値**でも裏を取る。

/// 装着済み target の (surface entity, text 層スロット entity)。
fn mount_entities(presenter: &EmoPresenter, target: TargetId) -> (Entity, Entity) {
    let mount = presenter
        .targets
        .get(&target)
        .expect("装着済み target")
        .mount
        .as_ref()
        .expect("表示確立後は mount が生成済み");
    (mount.surface_entity(), mount.text_slot())
}

/// 枠の面・文字層スロットの双方が不可視かつポインタ透過であること（Requirement 1.2/1.4/1.8）。
fn assert_entities_hidden(world: &World, presenter: &EmoPresenter, target: TargetId, ctx: &str) {
    let (surface, slot) = mount_entities(presenter, target);
    assert!(
        !world.get::<Visual>(surface).expect("surface に Visual").is_visible,
        "{ctx}: 枠の面 entity が可視のまま"
    );
    assert!(
        !world.get::<Visual>(slot).expect("slot に Visual").is_visible,
        "{ctx}: 文字層スロット entity が可視のまま"
    );
    assert_eq!(
        world.get::<HitTest>(surface).expect("surface に HitTest").mode,
        HitTestMode::None,
        "{ctx}: 枠の面がポインタを受け続けている"
    );
    assert_eq!(
        world.get::<HitTest>(slot).expect("slot に HitTest").mode,
        HitTestMode::None,
        "{ctx}: 文字層スロットがポインタを受け続けている"
    );
}

/// 枠の面・文字層スロットの双方が可視かつ従来の判定へ復帰していること。
fn assert_entities_visible(world: &World, presenter: &EmoPresenter, target: TargetId, ctx: &str) {
    let (surface, slot) = mount_entities(presenter, target);
    assert!(
        world.get::<Visual>(surface).expect("surface に Visual").is_visible,
        "{ctx}: 枠の面 entity が不可視のまま"
    );
    assert!(
        world.get::<Visual>(slot).expect("slot に Visual").is_visible,
        "{ctx}: 文字層スロット entity が不可視のまま"
    );
    assert_eq!(
        world.get::<HitTest>(surface).expect("surface に HitTest").mode,
        HitTestMode::AlphaMask,
        "{ctx}: 枠の面が αマスク判定へ復帰していない"
    );
    assert_eq!(
        world.get::<HitTest>(slot).expect("slot に HitTest").mode,
        HitTestMode::Bounds,
        "{ctx}: 文字層スロットが矩形判定へ復帰していない"
    );
}

/// `Hide` を適用し、reply が `Ok(())` であることを確認する。
fn hide_ok(presenter: &mut EmoPresenter, world: &mut World, target: TargetId) {
    let (tx, rx) = reply_channel::<PresentOutcome>();
    presenter.apply(
        world,
        PresentCommand::Hide {
            target,
            reply: Some(tx),
        },
    );
    assert!(
        matches!(rx.recv_timeout(Duration::from_secs(10)), Ok(Ok(()))),
        "Hide が Ok でない"
    );
}

/// 外部所有の target を 1 つ組み、初回 `ShowSurface` まで済ませた状態を返す。
fn external_target_after_first_show(
    dpi: u16,
    salt: u8,
) -> (World, Entity, EmoPresenter, TargetId, Vec<u8>) {
    let mut world = make_world_with_gpu();
    let window = spawn_window_with_dpi(&mut world, dpi);
    let (emo_world, atlas, golden) = build_target_assets(3, 2, salt);

    let mut presenter = EmoPresenter::new();
    presenter
        .attach_target(&mut world, TargetId(0), window, emo_world, atlas, 96)
        .expect("attach_target 失敗");
    presenter
        .set_visibility_ownership(TargetId(0), VisibilityOwnership::External)
        .expect("装着済み target への所有権設定は Ok");

    show_ok(&mut presenter, &mut world, TargetId(0), 1000);

    (world, window, presenter, TargetId(0), golden)
}

// ── ⑴ 外部所有: 表示指令は確立のみ（Requirement 1.1/1.3）──────────────────────────────

/// Requirement 1.1／1.3: `External` の target へ `ShowSurface` を与えても**不可視のまま**、配置先
/// （text 層スロット）と面・供給面・実適用 k・物理寸がすべて確立する。
///
/// 1.3 が要求するのは「可視化を伴わずに配置先を確保する」ことである。ゆえに檻は `target_visible` が
/// `Some(false)` であることと、`text_slot_view` が `Some`（＝スロット・native 原寸・実適用 k が
/// 揃っている）ことを**同時に**主張する。片方だけでは「出ないが文字も流せない」縮退を見逃す。
///
/// 供給面のバイト（readback）まで golden 一致を見るのは、可視化手順を落とした結果として
/// アップロードやマスク同期まで一緒に落ちていないこと（design「それ以外の全手順は所有権に依らず共通」）
/// を実行で確かめるためである。
#[test]
fn external_show_establishes_display_without_making_it_visible() {
    let (world, window, presenter, target, golden) = external_target_after_first_show(96, 0x21);

    assert_eq!(
        presenter.target_visible(target),
        Some(false),
        "外部所有の target が表示指令だけで可視になっている（Requirement 1.1）"
    );
    assert_entities_hidden(&world, &presenter, target, "外部所有の初回 ShowSurface 後");

    let view = presenter
        .text_slot_view(target)
        .expect("不可視でも配置先は確立していなければならない（Requirement 1.3）");
    assert_eq!(view.window(), window, "スロットは装着先の窓に属する");
    assert_eq!(view.surface_size(), (3, 2), "native 原寸が確立していない");
    assert_eq!(view.physical_size(), (3, 2), "k=1/1 の物理寸は native 原寸");
    assert_eq!(view.scale(), 1.0, "実適用 k が確立していない");

    assert_eq!(
        presenter.current_surface_id(target),
        Some(1000),
        "面 id の確立は可視性と直交する（Requirement 6.8）"
    );
    assert_eq!(
        presenter.applied_ratio(target),
        Some(ScaleRatio::ONE),
        "実適用 k は所有権に依らず表示成立点で確定する"
    );
    assert_eq!(
        presenter.target_physical_size(target),
        Some((3, 2)),
        "物理寸の照会が確立していない"
    );

    let rb = presenter.read_back(target).expect("read_back 失敗");
    assert_eq!(
        rb, golden,
        "不可視確立でも供給面へのアップロードは走る（合成・アップロードは所有権に依らず共通）"
    );
}

/// 較正（恒真化の検出）: 既定所有権（`CommandDriven`）では `ShowSurface` が従来どおり可視化する。
///
/// 上の檻だけでは「可視化手順そのものを壊した（どの target も可視にならない）」実装が緑になる。
/// 既定所有権の可視化が生きていることを対で固定して、ゲートが**所有権で分岐している**ことを主張する。
#[test]
fn command_driven_target_is_still_visualized_by_show_surface() {
    let mut world = make_world_with_gpu();
    let window = spawn_window_with_dpi(&mut world, 96);
    let (emo_world, atlas, _golden) = build_target_assets(3, 2, 0x22);

    let mut presenter = EmoPresenter::new();
    presenter
        .attach_target(&mut world, TargetId(0), window, emo_world, atlas, 96)
        .expect("attach_target 失敗");
    // 所有権を設定しない＝既定（CommandDriven）。
    show_ok(&mut presenter, &mut world, TargetId(0), 1000);

    assert_eq!(
        presenter.target_visible(TargetId(0)),
        Some(true),
        "既定所有権の target が表示指令で可視にならない（従来挙動の退行）"
    );
    assert_entities_visible(
        &world,
        &presenter,
        TargetId(0),
        "既定所有権の ShowSurface 後",
    );
}

// ── ⑵ 不可視のままの面切替・ループ由来指令（Requirement 6.2/6.9）─────────────────────────

/// Requirement 6.2／6.9: 不可視の外部所有 target へ面切替（`\b[ID]` 相当）と、同一面の再指令
/// （面切替側のアニメーション反復再生由来の指令に相当）を重ねても、**可視にならず結果だけが更新**される。
///
/// 反復再生由来の指令を「同一面の再 `ShowSurface`」で代表させているのは、可視化のゲートが面 id の
/// 変化や指令の発行元ではなく**所有権**だけを見ていること（＝経路を問わず可視化しない）を主張するため。
#[test]
fn external_reshow_updates_result_without_becoming_visible() {
    let mut world = make_world_with_gpu();
    let window = spawn_window_with_dpi(&mut world, 96);
    let (emo_world, atlas, golden_1000, golden_3000) = build_two_face_assets(4, 3);

    let mut presenter = EmoPresenter::new();
    presenter
        .attach_target(&mut world, TargetId(0), window, emo_world, atlas, 96)
        .expect("attach_target 失敗");
    presenter
        .set_visibility_ownership(TargetId(0), VisibilityOwnership::External)
        .expect("所有権設定は Ok");

    show_ok(&mut presenter, &mut world, TargetId(0), 1000);
    assert_eq!(presenter.target_visible(TargetId(0)), Some(false));
    assert_eq!(
        presenter.read_back(TargetId(0)).expect("read_back 失敗"),
        golden_1000
    );

    // 面切替（`\b[ID]` 相当）。
    show_ok(&mut presenter, &mut world, TargetId(0), 3000);
    assert_eq!(
        presenter.target_visible(TargetId(0)),
        Some(false),
        "面切替が可視性を変えている（Requirement 6.2）"
    );
    assert_eq!(
        presenter.current_surface_id(TargetId(0)),
        Some(3000),
        "面切替の結果が保持されていない"
    );
    assert_eq!(
        presenter.read_back(TargetId(0)).expect("read_back 失敗"),
        golden_3000,
        "不可視でも供給面の中身は新しい面へ入れ替わる"
    );

    // ループ由来の再指令相当（同一面を繰り返し指令する）。
    for _ in 0..3 {
        show_ok(&mut presenter, &mut world, TargetId(0), 3000);
        assert_eq!(
            presenter.target_visible(TargetId(0)),
            Some(false),
            "反復再生由来の指令で可視へ復帰している（Requirement 6.9）"
        );
    }
    assert_entities_hidden(
        &world,
        &presenter,
        TargetId(0),
        "面切替・反復再生の指令を重ねた後",
    );
    assert_eq!(
        presenter.current_surface_id(TargetId(0)),
        Some(3000),
        "反復指令後も最後に確立した面 id が正"
    );
}

// ── ⑶ 専用入口による可視化と k の再導出（Requirement 6.6）───────────────────────────────

/// Requirement 6.6: `show_target` は**現在の**窓 DPI から k を導出し直した上で可視化する。
///
/// 不可視の期間中に窓 DPI が 96 → 192 へ変わった状況を作る。可視化が「不可視化した時点の絵をそのまま
/// 出す」実装なら k=1/1 の native 寸のまま出てしまい、非表示期間中に生じた変化を取りこぼす。ゆえに檻は
/// 可視になったことに加えて、実適用 k・物理寸・表示バイトがすべて**新しい** DPI 由来であることを見る。
#[test]
fn show_target_makes_visible_and_rederives_scale_from_current_dpi() {
    let (mut world, window, mut presenter, target, _golden) =
        external_target_after_first_show(96, 0x23);
    assert_eq!(
        presenter.applied_ratio(target),
        Some(ScaleRatio::ONE),
        "前提: 96dpi／author 96 の確立は k=1/1"
    );

    // 不可視の期間中にモニタ跨ぎ相当の DPI 変化が起きる。
    set_window_dpi(&mut world, window, 192);

    // 同一入力から k=2/1 の golden を独立に再現する（presenter の内部値の追認にしない）。
    let (probe_world, probe_atlas, _) = build_target_assets(3, 2, 0x23);
    let k2 = ScaleRatio::new(2, 1).expect("非ゼロ比");
    let (scaled_bytes, native_size, scaled_size) =
        scaled_golden(&probe_world, &probe_atlas, 1000, k2);
    assert_eq!(native_size, (3, 2));
    assert_eq!(scaled_size, (6, 4), "前提: 2 水準の物理寸は異なる");

    presenter
        .show_target(&mut world, target)
        .expect("確立済みの外部所有 target の可視化は Ok");

    assert_eq!(
        presenter.target_visible(target),
        Some(true),
        "専用入口を呼んでも可視にならない（Requirement 1.1 の裏返し）"
    );
    assert_entities_visible(&world, &presenter, target, "show_target 後");
    assert_eq!(
        presenter.applied_ratio(target),
        Some(k2),
        "可視化時に k が現在の窓 DPI から導出し直されていない（Requirement 6.6）"
    );
    assert_eq!(
        presenter.target_physical_size(target),
        Some(scaled_size),
        "物理寸が新しい k を反映していない"
    );
    assert_eq!(
        presenter.read_back(target).expect("read_back 失敗"),
        scaled_bytes,
        "表示バイトが新しい k のリサンプル結果と一致しない（不可視期間中の変化の取りこぼし）"
    );
    assert_eq!(
        presenter.take_pending_resize(target),
        Some(scaled_size),
        "可視化に伴う物理寸の変化が窓寸 reconcile 要求として積まれていない"
    );
}

/// `show_target` は同一漏斗を再通過するため、k が変わらないときも可視化だけが成立する
/// （表示バイト・面 id は不変）。
#[test]
fn show_target_without_dpi_change_only_adds_visibility() {
    let (mut world, _window, mut presenter, target, golden) =
        external_target_after_first_show(96, 0x24);

    presenter
        .show_target(&mut world, target)
        .expect("確立済み target の可視化は Ok");

    assert_eq!(presenter.target_visible(target), Some(true));
    assert_entities_visible(&world, &presenter, target, "k 不変の show_target 後");
    assert_eq!(
        presenter.current_surface_id(target),
        Some(1000),
        "可視化が面 id を変えてはならない"
    );
    assert_eq!(
        presenter.read_back(target).expect("read_back 失敗"),
        golden,
        "可視化が表示バイトを変えてはならない"
    );
}

// ── ⑷ 非表示側は所有権に依らず即時（Requirement 6.1）────────────────────────────────────

/// Requirement 6.1: `\b[-1]` 相当の `Hide` は所有権に依らず**即時**に効く（可視化側だけが所有権で
/// ゲートされる非対称を保つ）。外部所有・既定所有権の双方で同一の結果になることを見る。
#[test]
fn hide_is_immediate_regardless_of_ownership() {
    // 外部所有: show_target で可視にした後の Hide。
    let (mut world, _window, mut presenter, target, _golden) =
        external_target_after_first_show(96, 0x25);
    presenter
        .show_target(&mut world, target)
        .expect("可視化は Ok");
    assert_eq!(presenter.target_visible(target), Some(true), "前提: 可視");

    hide_ok(&mut presenter, &mut world, target);
    assert_eq!(
        presenter.target_visible(target),
        Some(false),
        "外部所有の target で Hide が即時に効いていない（Requirement 6.1）"
    );
    assert_eq!(
        presenter.current_surface_id(target),
        None,
        "Hide は現サーフェス無しへ落とす（従来契約）"
    );
    assert_entities_hidden(&world, &presenter, target, "外部所有の Hide 後");

    // 既定所有権: 従来どおり。
    let mut world2 = make_world_with_gpu();
    let window2 = spawn_window_with_dpi(&mut world2, 96);
    let (emo_world, atlas, _g) = build_target_assets(3, 2, 0x26);
    let mut presenter2 = EmoPresenter::new();
    presenter2
        .attach_target(&mut world2, TargetId(1), window2, emo_world, atlas, 96)
        .expect("attach_target 失敗");
    show_ok(&mut presenter2, &mut world2, TargetId(1), 1000);
    assert_eq!(presenter2.target_visible(TargetId(1)), Some(true));

    hide_ok(&mut presenter2, &mut world2, TargetId(1));
    assert_eq!(
        presenter2.target_visible(TargetId(1)),
        Some(false),
        "既定所有権の Hide が退行している"
    );
    assert_entities_hidden(&world2, &presenter2, TargetId(1), "既定所有権の Hide 後");
}

/// 外部所有の target は `Hide` の後でも `show_target` で復帰できる（`last_show` は Hide で消えない）。
#[test]
fn external_target_can_be_shown_again_after_hide() {
    let (mut world, _window, mut presenter, target, golden) =
        external_target_after_first_show(96, 0x27);
    presenter.show_target(&mut world, target).expect("可視化 Ok");
    hide_ok(&mut presenter, &mut world, target);
    assert_eq!(presenter.target_visible(target), Some(false));

    presenter
        .show_target(&mut world, target)
        .expect("Hide 後も最後に確立した内容で再可視化できる");
    assert_eq!(presenter.target_visible(target), Some(true));
    assert_eq!(
        presenter.current_surface_id(target),
        Some(1000),
        "再可視化で最後に確立した面 id が戻る"
    );
    assert_eq!(
        presenter.read_back(target).expect("read_back 失敗"),
        golden,
        "再可視化の絵が最後に確立した内容と一致しない"
    );
}

// ── 失敗経路（log-first・沈黙する失敗を作らない）────────────────────────────────────────
// いずれも `attach_target` が World にも GPU にも触れない性質を使い、素の `World` で決定論固定する。

/// 未装着 target への所有権設定は `Err`（`error!` 済み・沈黙しない）。
#[test]
fn set_visibility_ownership_on_unattached_target_is_err() {
    let mut presenter = EmoPresenter::new();
    let result = presenter.set_visibility_ownership(TargetId(7), VisibilityOwnership::External);
    assert!(
        matches!(result, Err(PresentError::TargetNotAttached(TargetId(7)))),
        "未装着 target への所有権設定が Err でない: {result:?}"
    );
}

/// 未装着 target の可視化は `Err`。
#[test]
fn show_target_on_unattached_target_is_err() {
    let mut world = World::new();
    let mut presenter = EmoPresenter::new();
    let result = presenter.show_target(&mut world, TargetId(7));
    assert!(
        matches!(result, Err(PresentError::TargetNotAttached(TargetId(7)))),
        "未装着 target の可視化が Err でない: {result:?}"
    );
}

/// 一度も表示が確立していない target の可視化は `Err`（描くものが無いまま可視にしない）。
#[test]
fn show_target_before_any_established_display_is_err() {
    let mut world = World::new();
    let mut presenter = EmoPresenter::new();
    attach_hit_target(&mut presenter, &mut world, TargetId(0));
    presenter
        .set_visibility_ownership(TargetId(0), VisibilityOwnership::External)
        .expect("所有権設定は Ok");

    let result = presenter.show_target(&mut world, TargetId(0));
    assert!(
        matches!(result, Err(PresentError::TargetNotAttached(TargetId(0)))),
        "未確立 target の可視化が Err でない: {result:?}"
    );
    assert_eq!(
        presenter.target_visible(TargetId(0)),
        Some(false),
        "失敗した可視化が可視状態を書き換えている"
    );
}

/// 所有権の設定そのものは可視状態を変えない（「以後の指令が可視化してよいか」だけを決める）。
#[test]
fn setting_ownership_does_not_change_current_visibility() {
    let mut world = make_world_with_gpu();
    let window = spawn_window_with_dpi(&mut world, 96);
    let (emo_world, atlas, _g) = build_target_assets(3, 2, 0x28);

    let mut presenter = EmoPresenter::new();
    presenter
        .attach_target(&mut world, TargetId(0), window, emo_world, atlas, 96)
        .expect("attach_target 失敗");
    show_ok(&mut presenter, &mut world, TargetId(0), 1000);
    assert_eq!(presenter.target_visible(TargetId(0)), Some(true));

    presenter
        .set_visibility_ownership(TargetId(0), VisibilityOwnership::External)
        .expect("所有権設定は Ok");
    assert_eq!(
        presenter.target_visible(TargetId(0)),
        Some(true),
        "所有権の設定が可視状態を消している（設定は宣言であって指令ではない）"
    );
    assert_entities_visible(&world, &presenter, TargetId(0), "所有権設定の直後");
}

/// 未装着 target の可視状態照会は `None`（「不可視」と区別する）。
#[test]
fn target_visible_is_none_for_unattached_target() {
    let presenter = EmoPresenter::new();
    assert_eq!(
        presenter.target_visible(TargetId(9)),
        None,
        "未装着 target の照会は None（不可視 Some(false) と混同しない）"
    );
}

/// 装着直後（未表示）は `Some(false)`（skeleton の初期可視性）。
#[test]
fn target_visible_is_false_right_after_attach() {
    let mut world = World::new();
    let mut presenter = EmoPresenter::new();
    attach_hit_target(&mut presenter, &mut world, TargetId(0));
    assert_eq!(
        presenter.target_visible(TargetId(0)),
        Some(false),
        "装着直後は不可視（skeleton 登録のみ）"
    );
}

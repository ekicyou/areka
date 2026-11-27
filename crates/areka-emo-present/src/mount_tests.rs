//! `mount.rs` の構造・配置・当たり判定・変更検知を固定する GPU 非依存の檻。
//!
//! | 檻 | 固定する性質 |
//! |---|---|
//! | 論理配置 | 論理配置（寸＝原寸・係数＝k・原点 0）と、`Visual::on_add` の連鎖挿入が実際に起きること |
//! | 連鎖挿入の対照 | 親が窓でなければ連鎖挿入は起きないこと（片方だけでは恒真） |
//! | 同値書き込み | 同値の書き込みでは変更検知が立たないこと |
//!
//! z 順の檻は本ファイル、非表示切替の檻は `mount_visibility_tests.rs` に残る。

use super::*;

use bevy_ecs::hierarchy::Children;
use wintf::ecs::widget::BrushInherit;
use wintf::ecs::{
    GraphicsCommandList, HitTestMode, SurfaceGraphics, SurfaceGraphicsDirty, VisualGraphics,
};

use super::test_support::{attach_fixture, attach_fixture_under_window};

/// attach 後、text-layer slot が surface entity の**兄弟**かつ**上位 z**（`Children` 先頭）で
/// 存在し、`Name("emo-text-layer-slot")` を持ち、内容（描画リソース）を持たない予約 seam であること。
///
/// z 典拠: `visual_hierarchy_sync_system` は `Children` を前方反復し `InsertAtBottom` するため
/// 先頭の子ほど最上（描画で前面）。ゆえに index(slot) < index(surface) が「slot が上位 z」を意味する。
#[test]
fn text_slot_is_higher_z_sibling_with_name() {
    let (world, window, mount) = attach_fixture(3, 2);

    let children = world
        .get::<Children>(window)
        .expect("窓に Children（子 visual）が無い");
    let idx_slot = children
        .iter()
        .position(|e| e == mount.text_slot())
        .expect("text-layer slot が窓の子でない");
    let idx_surface = children
        .iter()
        .position(|e| e == mount.surface_entity())
        .expect("surface entity が窓の子でない");

    // 兄弟（同一親 window の子）であることは両 position の成立で担保。
    assert!(
        idx_slot < idx_surface,
        "text-layer slot は surface より上位 z（Children 先頭＝最上）でなければならない: \
         idx_slot={idx_slot} idx_surface={idx_surface}"
    );

    let name = world
        .get::<Name>(mount.text_slot())
        .expect("text-layer slot に Name が無い");
    assert_eq!(name.as_str(), "emo-text-layer-slot");

    // 予約スロットは Visual のみ・内容なし（表示記録も VisualGraphics も持たない）。
    assert!(
        world.get::<Visual>(mount.text_slot()).is_some(),
        "text-layer slot は Visual を持つ"
    );
    assert!(
        world.get::<VisualGraphics>(mount.text_slot()).is_none(),
        "予約スロットは内容（VisualGraphics/brush）を持たない seam であること"
    );
    assert!(
        world
            .get::<GraphicsCommandList>(mount.text_slot())
            .is_none(),
        "予約スロットは表示記録を持たない seam であること"
    );
}

/// 窓の子として装着したとき、surface entity が
///
/// 1. 論理配置（`Arrangement` の寸＝原寸・係数＝k・**原点 0**）と表示側の一式
///    （`OwnSurface`・`HitTest::surface_alpha`）を持ち、根の命令の列を持たず、
/// 2. wintf の `Visual::on_add`（`graphics/visual.rs` `on_visual_add`）の**連鎖挿入**で
///    `VisualGraphics`／`SurfaceGraphics`／`SurfaceGraphicsDirty` が**既定値のまま**入る
///
/// ことを固定する。既定値であること（`!is_valid()`・`requested_frame == 0`）は、本モジュールが
/// COM 資源を自前で作らず wintf の面生成系（`visual_resource_management_system`／
/// `deferred_surface_creation_system`）へ委ねている証拠である（GPU 不要）。
///
/// 原点 0 は不変条件（[`logical_arrangement`] の doc）——`visual_property_sync_system` の
/// 「offset × 自身の累積スケール」と `impl Mul<Arrangement> for GlobalArrangement` の
/// 「offset × 親スケール」は offset が 0 のときだけ一致する。
///
/// 対照は [`chain_insertion_requires_owner_window`]。
#[test]
fn arrangement_is_logical_native_with_scale_k_and_on_add_chain_runs() {
    let k = ScaleRatio::new(5, 4).expect("5/4");
    let (mut world, _window, mount) = attach_fixture_under_window(382, 547, k);
    let surface = mount.surface_entity();

    // --- 1. 論理配置（寸＝原寸・係数＝k・原点 0）。
    let arr = world
        .get::<Arrangement>(surface)
        .expect("surface entity に Arrangement が無い");
    assert_eq!(
        (arr.size.width, arr.size.height),
        (382.0, 547.0),
        "装着時 Arrangement 寸は原寸（論理 px）"
    );
    assert_eq!(
        (arr.scale.x, arr.scale.y),
        (1.25, 1.25),
        "装着時 Arrangement の係数は k（5/4）"
    );
    assert_eq!(
        (arr.offset.x, arr.offset.y),
        (0.0, 0.0),
        "原点は窓クライアント 0,0（不変条件）"
    );

    // --- 2. 表示側の一式。
    assert!(
        world.get::<OwnSurface>(surface).is_some(),
        "surface entity は描画面の持ち主の目印を持つ"
    );
    assert!(
        world.get::<GraphicsCommandList>(surface).is_none(),
        "surface entity は根の命令の列を持たない"
    );
    assert_eq!(
        world.get::<HitTest>(surface).unwrap().mode,
        HitTestMode::SurfaceAlpha,
        "surface entity は入れ物の α の判定"
    );

    // --- 3. `Visual::on_add` の連鎖挿入（owner Window 配下でのみ起きる 3 component）。
    let visual_graphics = world
        .get::<VisualGraphics>(surface)
        .expect("on_add 連鎖で VisualGraphics が入る（窓配下）");
    assert!(
        !visual_graphics.is_valid(),
        "VisualGraphics は既定値（COM 未生成）——自前生成ではなく wintf の面生成系に委ねる"
    );
    let surface_graphics = world
        .get::<SurfaceGraphics>(surface)
        .expect("on_add 連鎖で SurfaceGraphics が入る（窓配下）");
    assert!(
        !surface_graphics.is_valid(),
        "SurfaceGraphics は既定値（面未生成）"
    );
    assert_eq!(
        surface_graphics.size,
        (0, 0),
        "面寸は未確定（deferred_surface_creation_system が後で決める）"
    );
    let dirty = world
        .get::<SurfaceGraphicsDirty>(surface)
        .expect("on_add 連鎖で SurfaceGraphicsDirty が入る（窓配下）");
    assert_eq!(
        dirty.requested_frame, 0,
        "SurfaceGraphicsDirty も既定値のまま"
    );

    // --- 4. 原寸と k の変更（DPI 変化・面切替の経路）でも同じ形を保つ。
    let k2 = ScaleRatio::new(2, 1).expect("2/1");
    mount.set_layout(&mut world, (7, 5), k2);
    let arr = world
        .get::<Arrangement>(surface)
        .expect("set_layout 後も Arrangement を持つ");
    assert_eq!(
        (arr.size.width, arr.size.height),
        (7.0, 5.0),
        "set_layout 後 Arrangement 寸が新しい原寸を反映"
    );
    assert_eq!((arr.scale.x, arr.scale.y), (2.0, 2.0), "set_layout 後の k");
    assert_eq!(
        (arr.offset.x, arr.offset.y),
        (0.0, 0.0),
        "set_layout 後も原点 0,0"
    );
}

/// 対照（上の檻の較正）: 親が `Window` を持たなければ `Visual::on_add` の
/// `VisualGraphics`／`SurfaceGraphics`／`SurfaceGraphicsDirty` は**挿さらない**。
///
/// 上の檻だけでは「そもそもフックが動いていない」場合を排除できず、逆に本テストだけでは
/// 「何も挿さらない」だけを見ることになる。対で置いて初めて「窓配下でのみ挿さる」が固定される。
///
/// フック自体は走っていること（＝窓判定の分岐だけが落ちたこと）は、owner Window に依らず常に
/// 挿さる `BrushInherit` の存在で示す。
#[test]
fn chain_insertion_requires_owner_window() {
    let (world, _window, mount) = attach_fixture(3, 2);
    let surface = mount.surface_entity();

    // フックは走っている（owner Window に依らず挿さる側）。
    assert!(
        world.get::<BrushInherit>(surface).is_some(),
        "Visual::on_add は走っている（BrushInherit は窓判定に依らず挿さる）"
    );
    // 窓判定に依る 3 component は入らない。
    assert!(
        world.get::<VisualGraphics>(surface).is_none(),
        "窓でない親の下では VisualGraphics は挿さらない"
    );
    assert!(
        world.get::<SurfaceGraphics>(surface).is_none(),
        "窓でない親の下では SurfaceGraphics は挿さらない"
    );
    assert!(
        world.get::<SurfaceGraphicsDirty>(surface).is_none(),
        "窓でない親の下では SurfaceGraphicsDirty は挿さらない"
    );
}

/// surface entity の唯一の子は原寸のまとめの節（`SOURCE_OVER`・原点 0・倍率 1・寸＝原寸）で、
/// 描画面の持ち主にならない節である。拡大は surface entity の係数 k だけが持つ。
#[test]
fn the_only_child_of_the_surface_is_a_native_size_source_over_group() {
    let k = ScaleRatio::new(5, 4).expect("5/4");
    let (world, _window, mount) = attach_fixture_under_window(382, 547, k);
    let children = world
        .get::<Children>(mount.surface_entity())
        .expect("surface entity は子を持つ");
    assert_eq!(&children[..], &[mount.group()], "唯一の子はまとめの節");

    let group = mount.group();
    assert_eq!(
        world.get::<VisualGroup>(group),
        Some(&VisualGroup {
            mix: MixMode::default()
        }),
        "まとめの重ね方は SOURCE_OVER"
    );
    let arr = world.get::<Arrangement>(group).expect("まとめの配置");
    assert_eq!(
        (arr.offset.x, arr.offset.y, arr.scale.x, arr.scale.y),
        (0.0, 0.0, 1.0, 1.0),
        "まとめは原点 0・倍率 1（k は持ち主の係数だけ）"
    );
    assert_eq!(
        (arr.size.width, arr.size.height),
        (382.0, 547.0),
        "まとめの寸は原寸"
    );
    assert!(
        world.get::<VisualGraphics>(group).is_none(),
        "まとめは描画面の持ち主にならない（節）"
    );
}

/// `set_layout` は同値の書き込みで変更検知を立てない（surface entity とまとめの節の両方）。
///
/// `Changed<Arrangement>` は `GlobalArrangement` → 面の寸合わせ → 全面再描画の連鎖を引くため、
/// 変化のない適用で毎コマ立ててはならない。恒真にならないよう、同じテスト内で
/// 「違う値なら立つ」側（陽性対照）も置く。
#[test]
fn same_value_writes_do_not_trigger_change_detection() {
    let k = ScaleRatio::new(5, 4).expect("5/4");
    let (mut world, _window, mount) = attach_fixture_under_window(382, 547, k);
    let surface = mount.surface_entity();
    let group = mount.group();
    let changed = |world: &World, e: Entity| {
        world
            .entity(e)
            .get_ref::<Arrangement>()
            .expect("Arrangement")
            .is_changed()
    };

    // 追跡を切り、以後の書き込みだけが検知されるようにする。
    world.clear_trackers();
    world.increment_change_tick();

    // --- 同値の書き込み。
    mount.set_layout(&mut world, (382, 547), k);
    assert!(
        !changed(&world, surface) && !changed(&world, group),
        "同値の set_layout で Changed<Arrangement> を立ててはならない"
    );

    // --- 陽性対照 1: 違う k なら持ち主だけが立つ。
    let k2 = ScaleRatio::new(2, 1).expect("2/1");
    mount.set_layout(&mut world, (382, 547), k2);
    assert!(
        changed(&world, surface) && !changed(&world, group),
        "k だけが変われば持ち主の係数だけが動く"
    );

    // --- 陽性対照 2: 原寸が変わればまとめの寸も動く。
    world.clear_trackers();
    world.increment_change_tick();
    mount.set_layout(&mut world, (100, 50), k2);
    assert!(changed(&world, group), "原寸が変わればまとめの寸も動く");
    let arr = world.get::<Arrangement>(group).expect("まとめの配置");
    assert_eq!((arr.size.width, arr.size.height), (100.0, 50.0));
}

/// 装着の子を消す口（`despawn`）の後、窓の子は 0 で、面と文字層スロットの entity は
/// どちらも World に残らない。親を実 `Window` にして `Visual::on_add` の連鎖挿入が起きた装着で踏む。
///
/// 較正: 消す前に窓の子が 2（面とスロット）であることを先に確かめる（「最初から 0」で恒真に
/// ならないため）。
#[test]
fn despawn_removes_both_children_from_the_window() {
    let (mut world, window, mount) = attach_fixture_under_window(3, 2, ScaleRatio::ONE);
    let (surface, group, slot) = (mount.surface_entity(), mount.group(), mount.text_slot());
    assert_eq!(
        world.get::<Children>(window).map_or(0, |c| c.len()),
        2,
        "較正: 装着の直後は窓の子が 2（面とスロット）"
    );

    mount.despawn(&mut world);

    assert_eq!(
        world.get::<Children>(window).map_or(0, |c| c.len()),
        0,
        "despawn の後は窓の子が 0"
    );
    assert!(
        world.get_entity(surface).is_err(),
        "面の entity が残っている"
    );
    assert!(world.get_entity(group).is_err(), "まとめの節が残っている");
    assert!(
        world.get_entity(slot).is_err(),
        "文字層スロットが残っている"
    );
    assert!(world.get_entity(window).is_ok(), "窓そのものは消さない");
}

/// 面と文字層の entity が先に消えた装着へ 3 つの口（`set_visible`・`set_layout`・
/// `despawn`）を呼ぶと、装着契約の破綻の記録 4 つがそれぞれ 1 件以上残る
/// （沈黙する失敗経路を作らない）。
#[test]
fn operations_on_a_mount_whose_entities_are_gone_each_leave_a_warn() {
    let (mut world, _window, mount) = attach_fixture(3, 2);
    world.despawn(mount.surface_entity());
    world.despawn(mount.text_slot());

    let ((), events) = log_capture_kit::capture(|| {
        mount.set_visible(&mut world, false);
        mount.set_layout(&mut world, (3, 2), ScaleRatio::ONE);
        mount.despawn(&mut world);
    });

    for want in [
        "set_visible: Visual が無い（装着が未完了）",
        "set_visible: HitTest が無い（装着が未完了・ポインタ透過が成立しない）",
        "set_layout: 装着の実体に Arrangement が無い（装着が未完了）",
        "despawn: 装着の子が既に居ない",
    ] {
        assert!(
            events
                .iter()
                .any(|e| e.level == tracing::Level::WARN && e.message() == want),
            "記録「{want}」が warn で残っていない: {events:?}"
        );
    }
}

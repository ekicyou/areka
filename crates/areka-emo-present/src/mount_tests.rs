//! `mount.rs` の構造・配置・当たり判定・変更検知を固定する GPU 非依存の檻。
//!
//! | 檻 | 固定する性質 |
//! |---|---|
//! | T-N2 | 論理配置（寸＝原寸・係数＝k・原点 0）と、`Visual::on_add` の連鎖挿入が実際に起きること |
//! | T-N9 | T-N2 の対照: 親が窓でなければ連鎖挿入は起きないこと（片方だけでは恒真） |
//! | T-N4 | 当たり判定で点が **1 回だけ** 縮約されること（二重縮約なし） |
//! | T-N5 | 同値の書き込みでは変更検知が立たないこと |
//!
//! z 順・非表示切替の檻は本ファイルと `mount_visibility_tests.rs` に分かれて残る。

use super::*;

use bevy_ecs::hierarchy::Children;
use wintf::ecs::widget::BrushInherit;
use wintf::ecs::widget::bitmap_source::AlphaMask;
use wintf::ecs::{
    GlobalArrangement, HitTestMode, Point, PointF, Rect, SurfaceGraphics, SurfaceGraphicsDirty,
    VisualGraphics, WindowPos, hit_test_in_window,
};

// 注意: `wintf::ecs::PhysicalPoint` は pointer 側の別名（整数 `Point`）へ解決される。
// ヒットテストが取るのは `layout::hit_test` 側の別名（浮動小数 `PointF`）なので、
// 取り違えを避けるため本ファイルでは `PointF` を直接使う。

use super::test_support::{attach_fixture, attach_fixture_under_window};

/// R1.4: attach 後、text-layer slot が surface entity の**兄弟**かつ**上位 z**（`Children` 先頭）で
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

/// R3.3: 非表示切替で surface entity の `HitTest` が `None`（当たり判定停止）＋ `Visual` 不可視へ、
/// 再表示で `AlphaMask`（α判定）＋可視へ戻ること。表示記録/キャッシュは呼び手保持ゆえ触れない。
#[test]
fn hide_toggle_switches_hittest_and_visibility() {
    let (mut world, _window, mount) = attach_fixture(4, 4);

    // 初期状態: 可視 ＋ αマスクヒットテスト。
    assert!(
        world
            .get::<Visual>(mount.surface_entity())
            .unwrap()
            .is_visible,
        "装着直後は可視"
    );
    assert_eq!(
        world.get::<HitTest>(mount.surface_entity()).unwrap().mode,
        HitTestMode::AlphaMask,
        "装着直後は αマスクヒットテスト"
    );

    // 非表示へ。
    mount.set_visible(&mut world, false);
    assert!(
        !world
            .get::<Visual>(mount.surface_entity())
            .unwrap()
            .is_visible,
        "非表示後は Visual 不可視"
    );
    assert_eq!(
        world.get::<HitTest>(mount.surface_entity()).unwrap().mode,
        HitTestMode::None,
        "非表示後は当たり判定停止（HitTest::none）"
    );

    // 再表示へ。
    mount.set_visible(&mut world, true);
    assert!(
        world
            .get::<Visual>(mount.surface_entity())
            .unwrap()
            .is_visible,
        "再表示後は可視へ復帰"
    );
    assert_eq!(
        world.get::<HitTest>(mount.surface_entity()).unwrap().mode,
        HitTestMode::AlphaMask,
        "再表示後は αマスクヒットテストへ復帰"
    );
}

/// **T-N2**: 窓の子として装着したとき、surface entity が
///
/// 1. 論理配置（`Arrangement` の寸＝原寸・係数＝k・**原点 0**）と表示側の一式
///    （`GraphicsCommandList`・`HitTest::alpha_mask`・`AlphaMaskResource`）を持ち、
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
/// 対照は [`chain_insertion_requires_owner_window`]（T-N9）。
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
        world.get::<GraphicsCommandList>(surface).is_some(),
        "surface entity は表示記録（GraphicsCommandList）を持つ"
    );
    assert_eq!(
        world.get::<HitTest>(surface).unwrap().mode,
        HitTestMode::AlphaMask,
        "surface entity は αマスク判定"
    );
    assert!(
        world.get::<AlphaMaskResource>(surface).is_some(),
        "surface entity は αマスクの供給口を持つ"
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

/// **T-N9**（T-N2 の較正・対照）: 親が `Window` を持たなければ `Visual::on_add` の
/// `VisualGraphics`／`SurfaceGraphics`／`SurfaceGraphicsDirty` は**挿さらない**。
///
/// T-N2 だけでは「そもそもフックが動いていない」場合を排除できず、逆に本テストだけでは
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

/// 4×4 の既知マスク（PBGRA32・α は 0 か 255）。`true` の升だけ不透明。
///
/// ```text
///        x=0    x=1    x=2    x=3
/// y=0  [  T      F      F      T  ]
/// y=1  [  F      T      F      F  ]
/// y=2  [  F      F      T      F  ]
/// y=3  [  T      F      F      T  ]
/// ```
///
/// 対角と四隅を混ぜてあるので、÷k を 1 回しか掛けない写像と 2 回掛ける写像とで
/// 判定が食い違う点（例: 原寸換算 (3,3) と (1,1)）が存在する。
const KNOWN_MASK_4X4: [[bool; 4]; 4] = [
    [true, false, false, true],
    [false, true, false, false],
    [false, false, true, false],
    [true, false, false, true],
];

/// [`KNOWN_MASK_4X4`] を `AlphaMask`（原寸 4×4）へ焼く。
fn known_mask() -> AlphaMask {
    let mut pixels = Vec::with_capacity(4 * 4 * 4);
    for row in KNOWN_MASK_4X4 {
        for opaque in row {
            let a = if opaque { 255u8 } else { 0u8 };
            pixels.extend_from_slice(&[0, 0, 0, a]);
        }
    }
    AlphaMask::from_pbgra32(&pixels, 4, 4, 4 * 4)
}

/// **T-N4**: 原寸 4×4 のマスクを k=2 の非等倍で配置したとき、クリック透過の判定
/// （`hit_test_in_window` → `alpha_mask_hit`）が「点を **1 回だけ** 縮約した表」と一致する。
///
/// 経路は本番と同じ `evaluate_targets` → `hit_test_in_window` → `hit_test` → `hit_test_entity` →
/// `bounds.contains` → `alpha_mask_hit`。`alpha_mask_hit` は境界に対する比例写像
/// `⌊(p − left) / (native × k) × native⌋ = ⌊(p − left) / k⌋` を掛けるので、emo 側が点を
/// 縮約していれば ÷k が二重に効いて判定がずれる（設計 §÷k の写像・要件 4.2／4.7）。
///
/// 期待値は原寸マスクを `⌊client / k⌋` で 1 回だけ読む式で組む。式そのものが誤っていないことは、
/// 1 回縮約と 2 回縮約とで答えが変わる 2 点をベタ書きで押さえて較正する。
#[test]
fn hit_test_reduces_the_point_by_k_exactly_once() {
    let k = ScaleRatio::new(2, 1).expect("2/1");
    let native = (4u32, 4u32);
    // 窓はスクリーン (100, 200) に居る（client → screen の平行移動を経路に含める）。
    let window_origin = Point { x: 100, y: 200 };

    let mut world = World::new();
    let window = world
        .spawn(WindowPos {
            position: Some(window_origin),
            ..Default::default()
        })
        .id();

    // surface entity: 論理配置は本番と同じ [`logical_arrangement`]。
    let surface = world
        .spawn((
            logical_arrangement(native, k),
            HitTest::alpha_mask(),
            ChildOf(window),
        ))
        .id();
    world.flush();

    // 窓のスクリーン位置に置いた `GlobalArrangement`（scale 1.0）× 論理配置＝本番と同じ式。
    let window_global = GlobalArrangement {
        bounds: Rect {
            left: window_origin.x as f32,
            top: window_origin.y as f32,
            right: window_origin.x as f32,
            bottom: window_origin.y as f32,
        },
        ..GlobalArrangement::default()
    };
    let surface_global = window_global * logical_arrangement(native, k);
    assert_eq!(
        (surface_global.width(), surface_global.height()),
        (8.0, 8.0),
        "境界は原寸 4×4 の k=2 倍＝物理 8×8"
    );

    let mut mask_resource = AlphaMaskResource::new();
    mask_resource.set(known_mask());
    world
        .entity_mut(surface)
        .insert((surface_global, mask_resource));

    // --- 較正: 1 回縮約 ⌊c/2⌋ と 2 回縮約 ⌊c/4⌋ とで答えが変わる 2 点をベタ書きで押さえる。
    // 期待値を後段のループと同じ式から導いていると、式そのものが誤っていても通ってしまう。
    assert_eq!(
        hit_test_in_window(&world, window, PointF::new(6.0, 0.0)),
        Some(surface),
        "client (6,0)＝原寸 (3,0) は不透明ゆえヒット。÷k が二重なら原寸 (1,0)＝透明で外れる"
    );
    assert_eq!(
        hit_test_in_window(&world, window, PointF::new(4.0, 6.0)),
        None,
        "client (4,6)＝原寸 (2,3) は透明ゆえ外れ。÷k が二重なら原寸 (1,1)＝不透明で当たってしまう"
    );

    // --- 表: client 物理 px の格子全点で「1 回だけ縮約した表」と一致すること。
    for iy in 0..=16u32 {
        for ix in 0..=16u32 {
            // 0.0, 0.5, 1.0, … 8.0（境界 8.0 と半画素を含む）。
            let (cx, cy) = (ix as f32 * 0.5, iy as f32 * 0.5);
            // ÷k は **ここ 1 回だけ**。範囲外（境界ちょうどの 8.0）はマスク外＝外れ。
            let (mx, my) = ((cx / k.as_f32()) as usize, (cy / k.as_f32()) as usize);
            let hit = mx < 4 && my < 4 && KNOWN_MASK_4X4[my][mx];
            let expected = if hit { Some(surface) } else { None };

            let actual = hit_test_in_window(&world, window, PointF::new(cx, cy));
            assert_eq!(
                actual, expected,
                "client ({cx},{cy}) → 原寸 ({mx},{my}) の表と一致すべき（÷k は 1 回だけ）"
            );
        }
    }

    // --- 境界の外は当たらない（bounds.contains の早期リターン）。
    for (cx, cy) in [(-0.5f32, 4.0f32), (8.5, 4.0), (4.0, -0.5), (4.0, 8.5)] {
        assert_eq!(
            hit_test_in_window(&world, window, PointF::new(cx, cy)),
            None,
            "境界（物理 8×8）の外 ({cx},{cy}) は当たらない"
        );
    }
}

/// **T-N5**: `set_layout`／`set_display` は同値の書き込みで変更検知を立てない。
///
/// `Changed<Arrangement>` は `GlobalArrangement` → 面の寸合わせ → 全面再描画の連鎖を引くため、
/// 変化のない適用で毎コマ立ててはならない。恒真にならないよう、同じテスト内で
/// 「違う値なら立つ」側（陽性対照）も置く。
///
/// `GraphicsCommandList` の陽性対照は **component 不在 → 挿入** で取る。値の異なる 2 つ目の
/// `GraphicsCommandList` は実 `ID2D1CommandList`（GPU）を要し、GPU 非依存の檻では作れないためで、
/// 「無条件 return ではなく現値を見て分岐している」ことはこの対照で足りる。
#[test]
fn same_value_writes_do_not_trigger_change_detection() {
    let k = ScaleRatio::new(5, 4).expect("5/4");
    let (mut world, _window, mount) = attach_fixture_under_window(382, 547, k);
    let surface = mount.surface_entity();
    let display = GraphicsCommandList::empty();

    // 追跡を切り、以後の書き込みだけが検知されるようにする。
    world.clear_trackers();
    world.increment_change_tick();

    // --- 同値の書き込み。
    mount.set_layout(&mut world, (382, 547), k);
    mount.set_display(&mut world, &display);
    assert!(
        !world
            .entity(surface)
            .get_ref::<Arrangement>()
            .expect("Arrangement")
            .is_changed(),
        "同値の set_layout で Changed<Arrangement> を立ててはならない"
    );
    assert!(
        !world
            .entity(surface)
            .get_ref::<GraphicsCommandList>()
            .expect("GraphicsCommandList")
            .is_changed(),
        "同値の set_display で Changed<GraphicsCommandList> を立ててはならない"
    );

    // --- 陽性対照 1: 違う k なら立つ。
    let k2 = ScaleRatio::new(2, 1).expect("2/1");
    mount.set_layout(&mut world, (382, 547), k2);
    assert!(
        world
            .entity(surface)
            .get_ref::<Arrangement>()
            .expect("Arrangement")
            .is_changed(),
        "値が変われば Changed<Arrangement> は立つ（同値スキップが恒真でない証拠）"
    );

    // --- 陽性対照 2: 表示記録が不在なら挿さる。
    world.entity_mut(surface).remove::<GraphicsCommandList>();
    world.clear_trackers();
    world.increment_change_tick();
    mount.set_display(&mut world, &display);
    assert!(
        world
            .entity(surface)
            .get_ref::<GraphicsCommandList>()
            .expect("不在なら set_display が挿す")
            .is_changed(),
        "現値と異なれば set_display は挿す（無条件 return ではない証拠）"
    );
}

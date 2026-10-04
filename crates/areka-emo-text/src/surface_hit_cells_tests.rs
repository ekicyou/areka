//! 箱の文字の面の当たり判定＝表示されている字の矩形の集まり の檻（task 13・要件 9.4）。
//!
//! GPU を使わない: 面の entity が装着の時に持つ当たり判定（[`box_hit_components`]）と、提示が
//! 字の矩形を入れ替える口（[`set_child_hit_cells`]）を、窓・絵・箱の 3 entity を手で組んだ World に
//! 当て、wintf の当たり判定の入口 `hit_test_in_window`（クリック透過の切り替えと窓のメッセージの
//! 振り分けが共に呼ぶ入口）で引く。座標は窓の位置 (0,0) のクライアント物理 px。

use super::*;

use windows_numerics::Matrix3x2;
use wintf::ecs::{GlobalArrangement, Point, PointF, Rect, SizeI, WindowPos, hit_test_in_window};

/// シェルの窓の大きさ（物理 px）。
const WINDOW: (u32, u32) = (200, 120);
/// 箱の文字の面の左上と大きさ（物理 px）。左 80 px は絵の透明な所・右 40 px は不透明な所に掛かる。
const BOX_AT: (f32, f32) = (20.0, 20.0);
const BOX_SIZE: (u32, u32) = (120, 40);

fn arranged(left: f32, top: f32, size: (u32, u32)) -> GlobalArrangement {
    GlobalArrangement {
        transform: Matrix3x2::translation(left, top),
        bounds: Rect {
            left,
            top,
            right: left + size.0 as f32,
            bottom: top + size.1 as f32,
        },
    }
}

fn cell(left: f32, top: f32, right: f32, bottom: f32) -> HitRectPx {
    HitRectPx {
        left,
        top,
        right,
        bottom,
    }
}

/// 絵のマスク: x < 100 は透明（α 0）・x ≥ 100 は不透明（α 255）。
fn picture_mask() -> AlphaMask {
    let (w, h) = WINDOW;
    let mut px = vec![0u8; (w * h * 4) as usize];
    for y in 0..h {
        for x in 100..w {
            px[((y * w + x) * 4 + 3) as usize] = 255;
        }
    }
    AlphaMask::from_pbgra32(&px, w, h, w * 4)
}

/// 窓（当たり判定なし）の子に、製品と同じ並び「箱 → 絵」で 2 entity を組む。返り値: (窓, 箱, 絵)。
fn shell_with_box() -> (World, Entity, Entity, Entity) {
    let mut world = World::new();
    let window = world
        .spawn((
            arranged(0.0, 0.0, WINDOW),
            HitTest::none(),
            WindowPos {
                position: Some(Point { x: 0, y: 0 }),
                size: Some(SizeI {
                    width: WINDOW.0 as i32,
                    height: WINDOW.1 as i32,
                }),
                ..Default::default()
            },
        ))
        .id();
    let mut picture_res = AlphaMaskResource::new();
    picture_res.set(picture_mask());
    let picture = world
        .spawn((
            arranged(0.0, 0.0, WINDOW),
            HitTest::alpha_mask(),
            picture_res,
        ))
        .id();
    let box_entity = world
        .spawn((
            arranged(BOX_AT.0, BOX_AT.1, BOX_SIZE),
            box_hit_components(BOX_SIZE),
        ))
        .id();
    world
        .entity_mut(window)
        .add_children(&[box_entity, picture]);
    (world, window, box_entity, picture)
}

fn hit(world: &World, window: Entity, x: f32, y: f32) -> Option<Entity> {
    hit_test_in_window(world, window, PointF::new(x, y))
}

/// 装着したばかりの箱（字の矩形 0 個）は、箱の四角の中でも何も受けない（矩形全体へ縮退しない）。
#[test]
fn freshly_attached_box_receives_nothing() {
    let (world, window, box_entity, picture) = shell_with_box();
    assert_eq!(
        world.get::<HitTest>(box_entity).copied(),
        Some(HitTest::alpha_mask()),
        "箱の面の当たり判定はマスク（字の矩形の集まり）で決まる"
    );
    assert_eq!(hit(&world, window, 25.0, 25.0), None, "透明な絵の上は通す");
    assert_eq!(
        hit(&world, window, 130.0, 30.0),
        Some(picture),
        "不透明な絵の上は今までどおり絵が受ける"
    );
}

/// 字の矩形の内は、絵が透明でも受ける。字の間・字の無い所で絵が透明なら通す。絵が不透明なら
/// 今までどおり受ける（要件 9.4）。字の矩形を空にすると（`\c` など）、また通す。
#[test]
fn glyph_cells_receive_over_transparent_picture_and_gaps_pass_through() {
    let (mut world, window, box_entity, picture) = shell_with_box();
    // 面の左上を原点とする字の矩形 2 つ（20×20）。間に 20 px の隙間。
    let cells = [cell(0.0, 0.0, 20.0, 20.0), cell(40.0, 0.0, 60.0, 20.0)];
    set_child_hit_cells(&mut world, box_entity, BOX_SIZE, &cells);

    assert_eq!(
        hit(&world, window, 25.0, 25.0),
        Some(box_entity),
        "1 字目の矩形（絵は透明）"
    );
    assert_eq!(
        hit(&world, window, 65.0, 25.0),
        Some(box_entity),
        "2 字目の矩形（絵は透明）"
    );
    assert_eq!(
        hit(&world, window, 50.0, 25.0),
        None,
        "字の間の隙間（絵は透明）は通す"
    );
    assert_eq!(
        hit(&world, window, 25.0, 50.0),
        None,
        "箱の四角の中でも字の無い所（絵は透明）は通す"
    );
    assert_eq!(
        hit(&world, window, 130.0, 30.0),
        Some(picture),
        "箱の四角の中でも絵が不透明なら今までどおり受ける"
    );
    assert_eq!(hit(&world, window, 10.0, 100.0), None, "箱の外の透明な絵");
    assert_eq!(
        hit(&world, window, 150.0, 100.0),
        Some(picture),
        "箱の外の不透明な絵"
    );

    set_child_hit_cells(&mut world, box_entity, BOX_SIZE, &[]);
    assert_eq!(
        hit(&world, window, 25.0, 25.0),
        None,
        "字の矩形が無くなれば、前に字のあった所も通す"
    );
}

/// マスクの画素の内外: 矩形 `[left, right) × [top, bottom)` を外側へ丸めた画素が内。面の外へ
/// はみ出す矩形は面の端で切り、面の外の矩形は何も足さない（panic しない）。
#[test]
fn mask_rounds_cells_outward_and_clips_to_surface() {
    let mask = hit_cells_mask(
        (10, 6),
        &[
            cell(0.5, 0.5, 2.5, 1.2),
            cell(-5.0, 4.0, 1.0, 9.0),
            cell(20.0, 20.0, 30.0, 30.0),
        ],
    );
    let inside = |x, y| mask.is_hit(x, y);
    for (x, y) in [(0, 0), (2, 0), (0, 1), (2, 1), (0, 4), (0, 5)] {
        assert!(inside(x, y), "({x},{y}) は内");
    }
    for (x, y) in [(3, 0), (0, 2), (1, 4), (9, 5), (5, 3)] {
        assert!(!inside(x, y), "({x},{y}) は外");
    }
}

//! 節の下の手動の持ち主と、`VisualTransform` を持つ持ち主の画素の一致（WARP）。
//!
//! 同じ木を、持ち主を節にした形（全部を親の描画面の中で D2D が描く）と、持ち主の形（親の描画面から
//! 持ち主が抜け、持ち主の描画面を WUC の写しの値で重ねる）とで描いて比べる。写しの値は本番と同じ道で
//! 出す: 置き場は 1 回の判断（`run_round`）が書き、位置・行列・不透明度は `owner_props`、切り抜きは
//! `placed_key`→`placed_clip_px`（`CompositionClip.TransformMatrix` を含む）。
//! 回転は持ち主の描画面を線形で写すので、縁から 2 画素より内側だけを画素で比べ（軸に平行な写しは画像全体）、
//! 位置・向き・大きさのずれは画像全体の赤の量・重心・重心まわりの広がりで見る。

use super::super::super::{placed_clip_px, placed_key};
use super::super::placement::{OwnerProps, PlacementKind, WucTransform, owner_props};
use super::super::promotion_round::{PromotionData, RoundContext, run_round};
use super::super::world_visual::ReportedIssues;
use super::super::*;
use super::emit_test_support::*;
use crate::ecs::graphics::ClipShape;
use crate::ecs::layout::Arrangement;
use crate::numerics::Transform2D;
use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::system::SystemState;
use std::collections::HashSet;
use windows_numerics::Matrix3x2;

const ROOT_DIP: f32 = 40.0;
const SCALES: [f32; 2] = [1.0, 1.25];
const GREEN: [u8; 4] = [0, 255, 0, 255];

/// 持ち主 `m` の置き方。
#[derive(Clone, Copy)]
struct Case {
    /// 節 N（不透明度 0.5・矩形の切り抜き）の下に置くか（偽なら根の直下）。
    under_node: bool,
    transform: Option<Transform2D>,
}

/// 根 → [背景の青 (2,2) 36×36, (節 N (6,5) 20×18 → m (8,7) 16×14)]。N は左上に小さい緑の中身を持つ
/// （m と重ならない）。m の赤は下の 16×10 の帯だけ（基準点 (8,7) について対称でなく細長いので、基準点まわりの
/// 回転・拡大の誤りが重心と広がりに出る）。`manual` なら m は手動の持ち主（`OwnSurface`）、偽なら節。返すのは (根, m)。
fn tree(world: &mut World, case: Case, manual: bool) -> (Entity, Entity) {
    let root = world
        .spawn((Visual::default(), at(0.0, 0.0, ROOT_DIP, ROOT_DIP)))
        .id();
    world.spawn((
        Visual::default(),
        at(2.0, 2.0, 36.0, 36.0),
        solid(rect(0.0, 0.0, 36.0, 36.0), BLUE),
        ChildOf(root),
    ));
    let parent = if case.under_node {
        world
            .spawn((
                Visual {
                    opacity: 0.5,
                    clip: Some(ClipShape::Rectangle),
                    ..Default::default()
                },
                at(6.0, 5.0, 20.0, 18.0),
                solid(rect(0.0, 0.0, 3.0, 3.0), GREEN),
                ChildOf(root),
            ))
            .id()
    } else {
        root
    };
    let mut m = world.spawn((
        Visual::default(),
        at(8.0, 7.0, 16.0, 14.0),
        solid(rect(0.0, 4.0, 16.0, 14.0), RED),
        ChildOf(parent),
    ));
    if let Some(t) = case.transform {
        m.insert(VisualTransform(t));
    }
    if manual {
        m.insert(OwnSurface);
    }
    (root, m.id())
}

fn affine(t: WucTransform) -> Matrix3x2 {
    match t {
        WucTransform::Offset(o) => Matrix3x2::translation(o.X, o.Y),
        WucTransform::Matrix(m) => Matrix3x2 {
            M11: m.M11,
            M12: m.M12,
            M21: m.M21,
            M22: m.M22,
            M31: m.M41,
            M32: m.M42,
        },
    }
}

/// 持ち主の形の画面: 親の描画面（持ち主は抜ける）に、1 回の判断が書いた置き場から出した WUC の写しの値で
/// 持ち主の描画面を重ねる。
fn owner_form(world: &mut World, root: Entity, m: Entity, s: f32) -> Pixels {
    let side = (ROOT_DIP * s).ceil() as u32;
    let scale = Matrix3x2::scale(s, s);
    let parent = render_tree_to_pixels(world, root, side, side, scale);

    let scan = tree_scan(world, root);
    let mut state: SystemState<(Commands, Query<PromotionData>)> = SystemState::new(world);
    let (mut commands, mut states) = state.get_mut(world).unwrap();
    let ctx = RoundContext {
        now: None,
        device_ready: true,
        scale: s,
        host_px: (side, side),
    };
    let mut reported = ReportedIssues::default();
    run_round(
        &ctx,
        &scan,
        &mut states,
        &mut commands,
        &mut HashSet::new(),
        &mut reported,
    );
    state.apply(world);
    let placement = world.get::<SurfacePlacement>(m).map(|p| p.0.clone());

    // 性質の同期・切り抜きの同期と同じ局所の行列（VisualTransform × 平行移動(Arrangement.offset)）。
    let arrangement = *world.get::<Arrangement>(m).unwrap();
    let local = world
        .get::<VisualTransform>(m)
        .map_or(Matrix3x2::identity(), |t| t.to_affine())
        * Matrix3x2::from(arrangement.offset);
    let OwnerProps {
        transform, opacity, ..
    } = owner_props(local, 1.0, s, placement.as_ref());
    let size = (arrangement.size.width, arrangement.size.height);
    let none = (0.0, 0.0, 0.0, None, None, false);
    let clip = placed_key(none, None, placement.as_ref(), local, size)
        .4
        .map(|placed| {
            let (o, sz, mat) = placed_clip_px(&placed, s);
            (
                rect(o.X, o.Y, o.X + sz.X, o.Y + sz.Y),
                mat.unwrap_or(Matrix3x2::identity()),
            )
        });
    if !matches!(placement, Some(PlacementKind::Embedded { .. })) {
        assert!(
            clip.is_none(),
            "根の直下の持ち主は置き場の切り抜きを持たない"
        );
    }

    let (w, h) = ((size.0 * s).ceil() as u32, (size.1 * s).ceil() as u32);
    let own = render_tree_to_pixels(world, m, w, h, scale);
    composite_layers(
        side,
        side,
        &[
            Layer {
                pixels: &parent,
                transform: Matrix3x2::identity(),
                opacity: 1.0,
                clip: None,
            },
            Layer {
                pixels: &own,
                transform: affine(transform),
                opacity,
                clip,
            },
        ],
    )
}

/// 違う画素（座標・期待の値・持ち主の形の値）。
type Mismatch = (u32, u32, [u8; 4], [u8; 4]);

/// 縁から `margin` 画素の内側（期待の絵の (2·margin+1)² の近傍が ±1 で揃う画素）で、±1 を超えて違う画素と、
/// 比べた画素のうち赤の画素の数。`margin` 0 は画像全体。
fn compare(want: &Pixels, got: &Pixels, margin: i32) -> (Vec<Mismatch>, usize) {
    let h = want.bytes.len() as i32 / (want.width as i32 * 4);
    let w = want.width as i32;
    let near = |a: [u8; 4], b: [u8; 4]| {
        a.iter()
            .zip(b)
            .all(|(&u, v)| (u as i16 - v as i16).abs() <= 1)
    };
    let mut bad = Vec::new();
    let mut red = 0;
    for y in 0..h {
        for x in 0..w {
            let c = want.at(x as u32, y as u32);
            let flat = (-margin..=margin).all(|dy| {
                (-margin..=margin).all(|dx| {
                    let (nx, ny) = (x + dx, y + dy);
                    nx < 0 || ny < 0 || nx >= w || ny >= h || near(want.at(nx as u32, ny as u32), c)
                })
            });
            if !flat {
                continue;
            }
            let g = got.at(x as u32, y as u32);
            if !near(c, g) {
                bad.push((x as u32, y as u32, c, g));
            }
            if c[2] > 0 && c[2] >= c[0] {
                red += 1;
            }
        }
    }
    (bad, red)
}

/// 画像全体の赤の量 ΣR、R で重みをつけた重心 (Σx·R/ΣR, Σy·R/ΣR)（物理ピクセル）と、重心まわりの広がり
/// [Σ(x−cx)²R, Σ(y−cy)²R, Σ(x−cx)(y−cy)R]/ΣR（物理ピクセルの 2 乗・向きと大きさを量る）。
/// 青の背景と N の緑は R が 0 なので、持ち主の写りだけを量る。
fn red_moments(p: &Pixels) -> (f64, f64, f64, [f64; 3]) {
    let reds = || {
        let r = p.bytes.iter().skip(2).step_by(4).enumerate();
        r.map(|(i, &b)| {
            let (x, y) = ((i as u32 % p.width) as f64, (i as u32 / p.width) as f64);
            (x, y, b as f64)
        })
    };
    let (mut sum, mut sx, mut sy) = (0.0, 0.0, 0.0);
    for (x, y, r) in reds() {
        sum += r;
        sx += x * r;
        sy += y * r;
    }
    let (cx, cy) = (sx / sum, sy / sum);
    let mut spread = [0.0; 3];
    for (x, y, r) in reds() {
        spread[0] += (x - cx).powi(2) * r / sum;
        spread[1] += (y - cy).powi(2) * r / sum;
        spread[2] += (x - cx) * (y - cy) * r / sum;
    }
    (sum, cx, cy, spread)
}

fn assert_owner_matches(name: &str, case: Case, margin: i32) {
    for s in SCALES {
        let side = (ROOT_DIP * s).ceil() as u32;
        let mut nodes = World::new();
        let (root, _) = tree(&mut nodes, case, false);
        let want = render_tree_to_pixels(&mut nodes, root, side, side, Matrix3x2::scale(s, s));

        let mut owners = World::new();
        let (root, m) = tree(&mut owners, case, true);
        let got = owner_form(&mut owners, root, m, s);
        let (bad, red) = compare(&want, &got, margin);
        assert!(
            bad.is_empty(),
            "{name} 拡大 {s}: ±1 を超える画素 {} 個（先頭 {:?}）",
            bad.len(),
            &bad[..bad.len().min(4)]
        );
        // 較正: 比べた画素に持ち主の赤が十分に在る（持ち主が抜けたまま・全部が縁、を通さない）。
        assert!(
            red >= 20,
            "{name} 拡大 {s}: 比べた赤の画素が {red} 個しかない"
        );
        // 縁の内側だけの比べは 2 画素までの位置のずれを見ない。写りの量と重心は画像全体で、
        // 線形の写しの揺れ（量 1%・重心 0.25 物理ピクセル）より大きいずれを落とす。
        let (want_sum, want_x, want_y, want_spread) = red_moments(&want);
        let (got_sum, got_x, got_y, got_spread) = red_moments(&got);
        assert!(
            (got_sum - want_sum).abs() <= want_sum * 0.01,
            "{name} 拡大 {s}: 赤の量 {got_sum} が期待 {want_sum} から 1% を超えてずれる"
        );
        let shift = ((got_x - want_x).powi(2) + (got_y - want_y).powi(2)).sqrt();
        assert!(
            shift <= 0.25,
            "{name} 拡大 {s}: 赤の重心 ({got_x:.3}, {got_y:.3}) が期待 ({want_x:.3}, {want_y:.3}) から {shift:.3} 物理ピクセルずれる"
        );
        // 向きと大きさ: 重心まわりの広がりの 3 つの値。線形の写しの揺れは 0.1 未満。
        let spread = (0..3)
            .map(|i| (got_spread[i] - want_spread[i]).abs())
            .fold(0.0, f64::max);
        assert!(
            spread <= 0.15,
            "{name} 拡大 {s}: 赤の広がり {got_spread:?} が期待 {want_spread:?} から {spread:.3} ずれる"
        );
    }
}

fn rotated() -> Transform2D {
    Transform2D {
        rotation: 0.3,
        anchor: windows_numerics::Vector2 { X: 8.0, Y: 7.0 },
        offset: windows_numerics::Vector2 { X: 1.5, Y: -2.0 },
        ..Transform2D::identity()
    }
}

#[test]
fn owner_under_node_matches_with_ancestor_opacity_and_rect_clip() {
    // 置き場の不透明度 0.5 と、N の矩形の切り抜き（m の右下を切る）。拡大 1.25 では m の世界の位置
    // (17.5, 15) と N の切り抜きの縁 (7.5, 6.25)–(32.5, 28.75) が小数になるが、軸に平行な写しは期待と
    // ±1 で揃うので画像全体で比べる（拡大 1.0 は全部が整数）。
    let case = Case {
        under_node: true,
        transform: None,
    };
    assert_owner_matches("節の下", case, 0);
}

#[test]
fn owner_with_translated_visual_transform_matches() {
    // 位置 (12, 8) は拡大 1.25 でも整数の物理ピクセル（小数の位置の持ち主の線形の写しは縁がにじむ）。
    let t = Transform2D {
        offset: windows_numerics::Vector2 { X: 4.0, Y: 1.0 },
        ..Transform2D::identity()
    };
    let case = Case {
        under_node: false,
        transform: Some(t),
    };
    assert_owner_matches("平行移動", case, 0);
}

#[test]
fn owner_with_rotated_visual_transform_matches_inside_edges() {
    let case = Case {
        under_node: false,
        transform: Some(rotated()),
    };
    assert_owner_matches("回転", case, 2);
}

#[test]
fn rotated_owner_under_node_matches_with_transformed_clip_inside_edges() {
    // 回した持ち主に N の矩形の切り抜きを `CompositionClip.TransformMatrix`（逆行列）で当てる形。
    let case = Case {
        under_node: true,
        transform: Some(rotated()),
    };
    assert_owner_matches("節の下の回転", case, 2);
}

/// 持ち主の `VisualTransform` を変えても持ち主の版は上がらない（version.rs の約束）。
#[test]
fn owner_visual_transform_change_does_not_bump_its_version() {
    let mut world = World::new();
    let case = Case {
        under_node: true,
        transform: Some(rotated()),
    };
    let (root, m) = tree(&mut world, case, true);
    let mut frames = TreeFrames::new(&mut world);
    frames.commands(&mut world, root, None);
    frames.commands(&mut world, m, None);
    let before = world.get::<VisualVersion>(m).unwrap().0;
    world.get_mut::<VisualTransform>(m).unwrap().rotation = 0.6;
    frames.commands(&mut world, root, None);
    frames.commands(&mut world, m, None);
    assert_eq!(world.get::<VisualVersion>(m).unwrap().0, before);
}

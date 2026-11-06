use super::super::*;
use super::command::*;
use crate::ecs::graphics::clip::ClipShape;
use crate::ecs::layout::Arrangement;
use crate::numerics::*;
use std::hash::{Hash, Hasher};
use windows::Win32::Graphics::Direct2D::Common::D2D1_COMPOSITE_MODE;
use windows::Win32::Graphics::Direct2D::*;
use windows_numerics::*;

/// 平らにする係が 1 つの節から読む部品（`fill_world_visuals` が埋めた後の値）。
/// 可視と持ち主の境は `fill_world_visuals` の訪れた順の列が決めているので読み直さない。
pub(crate) type NodeData = (
    &'static Visual,
    &'static Arrangement,
    &'static WorldVisual,
    Option<&'static VisualDrawContent>,
    Option<&'static VisualVersion>,
);

/// 切り抜きの計画。
enum ClipKind {
    /// 切り抜いた後の有効な矩形が空＝部分木ごと出さない（push も pop も出さない）。
    Culled,
    NoClip,
    Clip {
        push: DrawCommand,
        pop: DrawCommand,
        new_eff: Aabb,
    },
}

/// `fill_world_visuals` の訪れた順の列 `visited`（前順・深さつき・見える節だけ）を画家の順の
/// `DrawCommand` の列にする。再帰しない: 深さが戻ったところで積んだ pop を出す。
///
/// - `out`: 先頭で `clear()` する（容量は使い回す）。
/// - `unit_rect`: (0,0)-(1,1) の矩形の図形。回転・せん断した矩形の切り抜きを図形へ上げるとき共有する。
/// - `viewport`: 根の `Arrangement.size` の矩形（根からの DIP）。これと切り抜きの外の節は間引く。
///   無限の矩形も渡せる（交わりにだけ使い、変形しない）。
///
/// 間引きは性能のため（D2D の切り抜きが既に正しく切る）。間引いた列は viewport に依る。
#[allow(dead_code, reason = "`update_draw_trees`（7.1）が呼ぶまで使い手が無い")]
pub(crate) fn build(
    out: &mut Vec<DrawCommand>,
    unit_rect: &ID2D1Geometry,
    viewport: Aabb,
    nodes: &Query<NodeData>,
    visited: &[(Entity, usize)],
) {
    out.clear();
    // 開いている切り抜き: (深さ, 閉じる命令, その下の有効な矩形)。
    let mut open: Vec<(usize, DrawCommand, Aabb)> = Vec::new();
    // 間引いた節の深さ。これより深い間は子孫なので飛ばす。
    let mut culled_at: Option<usize> = None;
    for &(entity, depth) in visited {
        if let Some(d) = culled_at {
            if depth > d {
                continue;
            }
            culled_at = None;
        }
        while open.last().is_some_and(|(d, ..)| *d >= depth) {
            out.push(open.pop().unwrap().1);
        }
        let eff = open.last().map_or(viewport, |(.., e)| *e);
        let Ok((visual, arrangement, world, content, version)) = nodes.get(entity) else {
            culled_at = Some(depth); // 消えた実体
            continue;
        };
        if eff.is_empty() {
            culled_at = Some(depth);
            continue;
        }
        // shortcut: 角丸も Arrangement の大きさの矩形として切る（角を落とさない）、5.3 で path geometry へ。
        let clip_rect = Aabb::new(0.0, 0.0, arrangement.size.width, arrangement.size.height);
        let new_eff = match plan_clip(visual.clip.as_ref(), &clip_rect, unit_rect, world, eff) {
            ClipKind::Culled => {
                culled_at = Some(depth);
                continue;
            }
            ClipKind::NoClip => eff,
            ClipKind::Clip { push, pop, new_eff } => {
                out.push(push);
                open.push((depth, pop, new_eff));
                new_eff
            }
        };
        if let Some(c) = content
            && !world.content_world_aabb.intersect(new_eff).is_empty()
        {
            out.push(DrawCommand::Draw(DrawItem {
                hash: hash_draw(
                    entity,
                    version.map_or(0, |v| v.0),
                    c.interpolation_mode,
                    c.composite_mode,
                    &world.world_mat,
                ),
                entity,
                world_mat: world.world_mat,
                world_aabb: world.content_world_aabb,
                command_list: c.command_list.clone(),
                interpolation_mode: c.interpolation_mode,
                composite_mode: c.composite_mode,
            }));
        }
    }
    while let Some((_, pop, _)) = open.pop() {
        out.push(pop);
    }
}

/// 切り抜きを計画する。どの分岐でも切り抜きの世界の外接矩形を `eff` と交え、空なら `Culled`。
/// 切り抜きの無い節は `Culled` にしない（自分が見えなくても子は見え得る）。
///
/// 矩形は世界の行列が軸に沿えば `PushClipRect`、回転・せん断なら単位矩形＋実効の変換の
/// `PushClipGeometry` へ上げる（図形は `unit_rect` を共有する）。
fn plan_clip(
    clip: Option<&ClipShape>,
    clip_rect: &Aabb,
    unit_rect: &ID2D1Geometry,
    world: &WorldVisual,
    eff: Aabb,
) -> ClipKind {
    let Some(clip) = clip else {
        return ClipKind::NoClip;
    };
    let (push, pop, world_aabb) = match clip {
        ClipShape::Rectangle
        | ClipShape::RoundedRectangle { .. }
        | ClipShape::RoundedRectangleIndividual { .. } => {
            if world.axis_aligned {
                let world_aabb = clip_rect.transform(world.world_mat);
                (
                    DrawCommand::PushClipRect(ClipRect { world_aabb }),
                    DrawCommand::PopClipRect,
                    world_aabb,
                )
            } else {
                let m = unit_to_rect(clip_rect) * world.world_mat;
                let world_aabb = geometry_bounds(unit_rect, &m);
                (
                    DrawCommand::PushClipGeometry(ClipGeometry {
                        world_mat: m,
                        world_aabb,
                        geometry: unit_rect.clone(),
                    }),
                    DrawCommand::PopClipGeometry,
                    world_aabb,
                )
            }
        }
        ClipShape::Geometry(g) => {
            let world_aabb = geometry_bounds(g, &world.world_mat);
            (
                DrawCommand::PushClipGeometry(ClipGeometry {
                    world_mat: world.world_mat,
                    world_aabb,
                    geometry: g.clone(),
                }),
                DrawCommand::PopClipGeometry,
                world_aabb,
            )
        }
    };
    let new_eff = eff.intersect(world_aabb);
    if new_eff.is_empty() {
        return ClipKind::Culled;
    }
    ClipKind::Clip { push, pop, new_eff }
}

/// 図形を `m` で置いた世界の外接矩形。
// shortcut: 失敗を 0 の矩形に読み替えている（結果は間引き）、5.3 で error! と部分木の飛ばしへ。
fn geometry_bounds(g: &ID2D1Geometry, m: &Matrix3x2) -> Aabb {
    let r = unsafe { g.GetBounds(Some(m as *const _)) }.unwrap_or_default();
    Aabb::new_rect(r)
}

/// 描く 1 件の同一性（4.5）。実体・見た目の版・補間・重ね方・世界の行列から取り、COM の番地は使わない。
/// 小数は `CalcHash` で正規化する。`DefaultHasher::new()` は鍵が固定なので回をまたいで同じ値になる。
pub(crate) fn hash_draw(
    entity: Entity,
    version: u64,
    interpolation: D2D1_INTERPOLATION_MODE,
    composite: D2D1_COMPOSITE_MODE,
    world: &Matrix3x2,
) -> u64 {
    let mut state = std::collections::hash_map::DefaultHasher::new();
    entity.hash(&mut state);
    version.hash(&mut state);
    interpolation.0.hash(&mut state);
    composite.0.hash(&mut state);
    world.calc_hash(&mut state);
    state.finish()
}

/// 単位矩形 (0,0)-(1,1) を局所の矩形 `r` へ写す変換。
pub(crate) fn unit_to_rect(r: &Aabb) -> Matrix3x2 {
    Matrix3x2 {
        M11: r.width(),
        M12: 0.0,
        M21: 0.0,
        M22: r.height(),
        M31: r.left(),
        M32: r.top(),
    }
}

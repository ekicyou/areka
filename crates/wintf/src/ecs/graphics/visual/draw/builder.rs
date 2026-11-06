use super::super::*;
use super::clip_geometry;
use super::command::*;
use crate::ecs::graphics::clip::ClipShape;
use crate::ecs::graphics::systems::format_entity_name;
use crate::ecs::layout::Arrangement;
use crate::numerics::*;
use std::hash::{Hash, Hasher};
use tracing::error;
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
    Option<&'static Name>,
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
/// - `factory`: 角丸の path geometry を作る工場（`GraphicsCore::d2d_factory()`）。
/// - `unit_rect`: (0,0)-(1,1) の矩形の図形。回転・せん断した矩形の切り抜きを図形へ上げるとき共有する。
/// - `viewport`: 根の `Arrangement.size` の矩形（根からの DIP）。これと切り抜きの外の節は間引く。
///   無限の矩形も渡せる（交わりにだけ使い、変形しない）。
/// - `root`・`root_name`: 記録に載せる根（描画面の持ち主）。
///
/// 切り抜きの図形を作れないか外接矩形を取れない節は `error!`（実体・HRESULT）を残し、子孫ごと飛ばす（5.1, 5.2）。
///
/// 間引きは性能のため（D2D の切り抜きが既に正しく切る）。間引いた列は viewport に依る。
pub(crate) fn build(
    out: &mut Vec<DrawCommand>,
    factory: &ID2D1Factory,
    unit_rect: &ID2D1Geometry,
    viewport: Aabb,
    nodes: &Query<NodeData>,
    root: Entity,
    root_name: Option<&Name>,
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
        let Ok((visual, arrangement, world, content, version, name)) = nodes.get(entity) else {
            error!(
                root = %format_entity_name(root, root_name),
                entity = ?entity,
                "[build] 埋めた後に節が消えたので、その部分木を飛ばす"
            );
            culled_at = Some(depth);
            continue;
        };
        if eff.is_empty() {
            culled_at = Some(depth);
            continue;
        }
        let size = &arrangement.size;
        let plan = plan_clip(factory, visual.clip.as_ref(), size, unit_rect, world, eff);
        let new_eff = match plan {
            Err(e) => {
                error!(
                    root = %format_entity_name(root, root_name),
                    entity = %format_entity_name(entity, name),
                    error = ?e,
                    hresult = format_args!("0x{:08X}", e.code().0),
                    "[build] 切り抜きの図形を作れないか外接矩形を取れないので、その部分木を飛ばす"
                );
                culled_at = Some(depth);
                continue;
            }
            Ok(ClipKind::Culled) => {
                culled_at = Some(depth);
                continue;
            }
            Ok(ClipKind::NoClip) => eff,
            Ok(ClipKind::Clip { push, pop, new_eff }) => {
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
/// `PushClipGeometry` へ上げる（図形は `unit_rect` を共有する）。角丸は `size` の path geometry、
/// 任意の図形はそのまま `PushClipGeometry`。図形の作成と外接矩形の失敗は `Err`（0 の矩形に化かさない）。
fn plan_clip(
    factory: &ID2D1Factory,
    clip: Option<&ClipShape>,
    size: &crate::ecs::Size,
    unit_rect: &ID2D1Geometry,
    world: &WorldVisual,
    eff: Aabb,
) -> windows::core::Result<ClipKind> {
    let Some(clip) = clip else {
        return Ok(ClipKind::NoClip);
    };
    let clip_rect = Aabb::new(0.0, 0.0, size.width, size.height);
    let geometry_push = |geometry: ID2D1Geometry, world_mat: Matrix3x2| {
        let world_aabb = geometry_bounds(&geometry, &world_mat)?;
        windows::core::Result::Ok((
            DrawCommand::PushClipGeometry(ClipGeometry {
                world_mat,
                world_aabb,
                geometry,
            }),
            DrawCommand::PopClipGeometry,
            world_aabb,
        ))
    };
    let rounded = |tl: f32, tr: f32, br: f32, bl: f32| {
        let g = clip_geometry::rounded_rect(factory, size.width, size.height, tl, tr, br, bl)?;
        geometry_push(g, world.world_mat)
    };
    let (push, pop, world_aabb) = match clip {
        ClipShape::RoundedRectangle { radius: r } => rounded(*r, *r, *r, *r)?,
        &ClipShape::RoundedRectangleIndividual {
            top_left,
            top_right,
            bottom_right,
            bottom_left,
        } => rounded(top_left, top_right, bottom_right, bottom_left)?,
        ClipShape::Rectangle => {
            if world.axis_aligned {
                let world_aabb = clip_rect.transform(world.world_mat);
                (
                    DrawCommand::PushClipRect(ClipRect { world_aabb }),
                    DrawCommand::PopClipRect,
                    world_aabb,
                )
            } else {
                geometry_push(
                    unit_rect.clone(),
                    unit_to_rect(&clip_rect) * world.world_mat,
                )?
            }
        }
        ClipShape::Geometry(g) => geometry_push(g.clone(), world.world_mat)?,
    };
    let new_eff = eff.intersect(world_aabb);
    if new_eff.is_empty() {
        return Ok(ClipKind::Culled);
    }
    Ok(ClipKind::Clip { push, pop, new_eff })
}

/// 図形を `m` で置いた世界の外接矩形。
fn geometry_bounds(g: &ID2D1Geometry, m: &Matrix3x2) -> windows::core::Result<Aabb> {
    Ok(Aabb::new_rect(unsafe { g.GetBounds(Some(m as *const _)) }?))
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

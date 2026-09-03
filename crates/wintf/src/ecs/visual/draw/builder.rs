use super::super::*;
use super::command::*;
use crate::numerics::*;
use bevy_ecs::prelude::*;
use windows::Win32::Graphics::Direct2D::*;
use windows_numerics::*;

/// visit が 1 ノードから読み取る不変ビュー（すべて read-only）。
///
/// - `VisualTransform`   : 無ければ identity 扱い。
/// - `VisualDrawContent` : 無ければ描画コマンドを出さない。
/// - `VisualClip`        : 無ければ clip push/pop なし。
/// - `Children`          : 無ければ葉ノード。
///
/// 4 つを 1 本に束ねることで、1 エンティティ = 1 回の `Query::get`
/// ルックアップで済む（クエリを分けると get が複数回になる）。
type NodeData = (
    Option<&'static VisualTransform>,
    Option<&'static VisualDrawContent>,
    Option<&'static VisualClip>,
    Option<&'static Children>,
);

/// `root` を起点に pre-order DFS（ペインターズ順）で `DrawCommand` 列を構築する。
///
/// - `out`       : 出力累積器。先頭で `clear()` され、capacity は再利用される。
/// - `unit_rect` : Rect→Geometry 昇格時に共有する単位矩形 (0,0)-(1,1)。ポインタ安定用。
/// - `root_mat`  : ルートに与える親変換（通常は identity か DPI 変換）。
/// - `nodes`     : read-only クエリ。ノードは `get` でランダムアクセスする。
/// - `root`      : 走査の起点 entity。
#[allow(dead_code)]
pub(crate) fn build(
    out: &mut Vec<DrawCommand>,
    unit_rect: &ID2D1Geometry,
    root_mat: Matrix3x2,
    nodes: &Query<NodeData>,
    root: Entity,
) {
    out.clear();
    visit(out, unit_rect, nodes, root, root_mat);
}

enum ClipKind {
    PopClipRect,
    PopClipGeometry,
}

/// 1 ノードを訪問する。clip push → 自分を Draw → 子を再帰 → clip pop の順。
/// この順序により clip は自ノードと部分木の両方を囲み、`Children` の順序が Z 順になる。
///
/// `&Query` は共有借用なので再帰で何度渡しても read 借用が重なるだけで合法。
fn visit(
    out: &mut Vec<DrawCommand>,
    unit_rect: &ID2D1Geometry,
    nodes: &Query<NodeData>,
    entity: Entity,
    parent_world: Matrix3x2,
) {
    // despawn 済み / クエリ非該当は静かにスキップ（部分木ごと出力しない）。
    let Ok((transform, draw, clip, children)) = nodes.get(entity) else {
        return;
    };

    // world = local * parent（行ベクトル規約: v' = v * local * parent）。
    let local = transform
        .map(|t| t.0.into())
        .unwrap_or(Matrix3x2::identity());
    let world_mat = local * parent_world;

    // push clip
    let pop_clip = push_clip(out, unit_rect, clip, entity, world_mat);

    // 自ノードの描画
    if let Some(content) = draw {
        out.push(DrawCommand::Draw(DrawItem {
            hash: hash_draw(content, &world_mat),
            world_mat,
            world_aabb: transform_aabb(&content.local_aabb, &world_mat),
            entity,
        }));
    }

    // 子を宣言順に再帰（= 描画順 = Z 順）。
    if let Some(children) = children {
        for child in children.iter() {
            visit(out, unit_rect, nodes, child, world_mat);
        }
    }

    // pop clip
    match pop_clip {
        Some(ClipKind::PopClipRect) => out.push(DrawCommand::PopClipRect),
        Some(ClipKind::PopClipGeometry) => out.push(DrawCommand::PopClipGeometry),
        None => {}
    }
}

/// clip を push し、対応する Pop のための種別を返す。
///
/// `VisualClip::Rect` は world が矩形保存なら `PushClipRect`、回転/せん断があれば
/// 単位矩形 + 実効変換に正規化して `PushClipGeometry` へ昇格する（geometry 実体は
/// `unit_rect` を共有し、差分のポインタ同一性を保つ）。
fn push_clip(
    out: &mut Vec<DrawCommand>,
    unit_rect: &ID2D1Geometry,
    clip: Option<&VisualClip>,
    entity: Entity,
    world_mat: Matrix3x2,
) -> Option<ClipKind> {
    let clip = clip?;
    match clip {
        VisualClip::Rect(local) => {
            if is_rect_preserving(&world_mat) {
                // 矩形保存 → そのまま world 矩形へ焼き込み。
                out.push(DrawCommand::PushClipRect(ClipRect {
                    world_aabb: transform_aabb(local, &world_mat),
                }));
                Some(ClipKind::PopClipRect)
            } else {
                // 昇格: 単位矩形→local矩形 に world を合成した実効変換に畳む。
                let eff = unit_to_rect(local) * world_mat;
                out.push(DrawCommand::PushClipGeometryRect(ClipGeometryRect {
                    world_mat: eff,
                    world_aabb: geometry_bounds(unit_rect, &eff),
                    geometry: unit_rect.clone(), // 共有 → ポインタ安定（clone は AddRef のみ）
                }));
                Some(ClipKind::PopClipGeometry)
            }
        }
        VisualClip::Geometry(local_geom) => {
            // local geometry は遅延適用（emit 側で SetTransform(world_mat)）。
            out.push(DrawCommand::PushClipGeometryEntity(ClipGeometryEntity {
                world_mat,
                world_aabb: geometry_bounds(local_geom, &world_mat),
                entity,
            }));
            Some(ClipKind::PopClipGeometry)
        }
    }
}

/// geometry を `m` で変換した後の world AABB。オブジェクト生成なし（GetBounds 一発）。
fn geometry_bounds(g: &ID2D1Geometry, m: &Matrix3x2) -> Aabb {
    let r = unsafe { g.GetBounds(Some(m as *const _)) }.unwrap_or_default();
    Aabb::new_rect(r)
}

/// hash 入力: content ポインタ同一性 + 補間/合成モード + world_mat。
/// `local_aabb`/`world_aabb` はメタ・導出値なので除外（ピクセルに無関係）。
fn hash_draw(c: &VisualDrawContent, world: &Matrix3x2) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut state = std::collections::hash_map::DefaultHasher::new();
    c.hash(&mut state);
    world.calc_hash(&mut state);
    state.finish()
}

/// 単位矩形 (0,0)-(1,1) を local 矩形 `r` へ写す変換。
fn unit_to_rect(r: &Aabb) -> Matrix3x2 {
    Matrix3x2 {
        M11: r.width(),
        M12: 0.0,
        M21: 0.0,
        M22: r.height(),
        M31: r.left(),
        M32: r.top(),
    }
}

/// 軸整列を保つ変換か（`PushAxisAlignedClip` 可否）。
/// 回転なし（せん断なし）、または 90/270 度回転を許容。
fn is_rect_preserving(m: &Matrix3x2) -> bool {
    const EPS: f32 = 1e-4;
    let no_skew = m.M12.abs() < EPS && m.M21.abs() < EPS;
    let quarter = m.M11.abs() < EPS && m.M22.abs() < EPS;
    no_skew || quarter
}

/// AABB の 4 隅を変換して外接 AABB を得る（任意アフィン対応）。
fn transform_aabb(a: &Aabb, m: &Matrix3x2) -> Aabb {
    let corners = [
        (a.left(), a.top()),
        (a.right(), a.top()),
        (a.right(), a.bottom()),
        (a.left(), a.bottom()),
    ];
    let (mut left, mut top, mut right, mut bottom) = (
        f32::INFINITY,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NEG_INFINITY,
    );
    for (x, y) in corners {
        let wx = x * m.M11 + y * m.M21 + m.M31;
        let wy = x * m.M12 + y * m.M22 + m.M32;
        left = left.min(wx);
        top = top.min(wy);
        right = right.max(wx);
        bottom = bottom.max(wy);
    }
    Aabb::new(left, top, right, bottom)
}

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

/// クリップ計画の 4 値。`plan_clip` が有効 AABB `eff` とノードのクリップから算出する。
///
/// - `Culled`   : クリップ適用後に有効範囲が空 → 部分木ごとキャンセル（push も pop も出さない）。
/// - `NoClip`   : クリップ無し → 有効範囲は据え置き、push/pop なし。
/// - `Rect`     : 軸整列 Rect クリップ。push を積み、末尾で PopClipRect。
/// - `Geometry` : Geometry クリップ（昇格 Rect / Entity geometry）。末尾で PopClipGeometry。
///
/// `Rect`/`Geometry` は push コマンドと絞り込んだ有効 AABB (`new_eff`) を保持する。
/// pop 種別は variant により一意に決まる（Rect→PopClipRect, Geometry→PopClipGeometry）。
enum ClipKind {
    Culled,
    NoClip,
    Rect { push: DrawCommand, new_eff: Aabb },
    Geometry { push: DrawCommand, new_eff: Aabb },
}

/// `root` を起点に pre-order DFS（ペインターズ順）で `DrawCommand` 列を構築する。
///
/// 有効 AABB（`viewport ∩ 全祖先クリップ`）でカリングしながら走査する。
///
/// - `out`       : 出力累積器。先頭で `clear()` され、capacity は再利用される。
/// - `unit_rect` : Rect→Geometry 昇格時に共有する単位矩形 (0,0)-(1,1)。ポインタ安定用。
/// - `root_mat`  : ルートに与える親変換（通常は identity か DPI 変換）。
/// - `viewport`  : ルートの有効 AABB。`root_mat` と同じワールド座標系の矩形。
///                 これより外は 1 ピクセルも見えないという保証領域。
/// - `nodes`     : read-only クエリ。ノードは `get` でランダムアクセスする。
/// - `root`      : 走査の起点 entity。
///
/// # カリング前提
/// D2D 側は clip で既に視覚的に切り抜いている。ここでのカリングは *正しさ* ではなく
/// *性能*（コマンド発行 / hash / diff / 直列 emit の負荷削減）のための刈り込み。
/// カリングを入れると `DrawCommand` 列は viewport 依存になるので、hash 差分更新と
/// 併用する場合は viewport を差分キーに含めること。
#[allow(dead_code)]
pub(crate) fn build(
    out: &mut Vec<DrawCommand>,
    unit_rect: &ID2D1Geometry,
    root_mat: Matrix3x2,
    viewport: Aabb,
    nodes: &Query<NodeData>,
    root: Entity,
) {
    out.clear();
    visit(out, unit_rect, nodes, root, root_mat, viewport);
}

/// 1 ノードを訪問する。clip push → 自分を Draw → 子を再帰 → clip pop の順。
/// この順序により clip は自ノードと部分木の両方を囲み、`Children` の順序が Z 順になる。
///
/// `eff` は有効 AABB（祖先クリップ ∩ viewport）。カリングの基準領域。
/// `&Query` は共有借用なので再帰で何度渡しても read 借用が重なるだけで合法。
fn visit(
    out: &mut Vec<DrawCommand>,
    unit_rect: &ID2D1Geometry,
    nodes: &Query<NodeData>,
    entity: Entity,
    parent_world: Matrix3x2,
    eff: Aabb,
) {
    // 入り口: 祖先クリップで既に有効領域が空なら部分木ごと不可視 → 丸ごとスキップ。
    if aabb_is_empty(&eff) {
        return;
    }

    // despawn 済み / クエリ非該当は静かにスキップ（部分木ごと出力しない）。
    let Ok((transform, draw, clip, children)) = nodes.get(entity) else {
        return;
    };

    // world = local * parent（行ベクトル規約: v' = v * local * parent）。
    let local = transform
        .map(|t| t.0.into())
        .unwrap_or(Matrix3x2::identity());
    let world_mat = local * parent_world;

    // ① クリップ計画（4 値）。push はここでは積まない。
    // ③ Culled は push を出す *前* に return するので bracket（push/pop 対）は均衡を保つ。
    let (new_eff, pop) = match plan_clip(clip, unit_rect, entity, world_mat, eff) {
        ClipKind::Culled => return,
        ClipKind::NoClip => (eff, None),
        ClipKind::Rect { push, new_eff } => {
            out.push(push);
            (new_eff, Some(DrawCommand::PopClipRect))
        }
        ClipKind::Geometry { push, new_eff } => {
            out.push(push);
            (new_eff, Some(DrawCommand::PopClipGeometry))
        }
    };

    // ② 自ノードの描画（有効 AABB と交差する時だけ発行）。
    // world_aabb は DrawItem 用にどのみち計算する値なので、追加コストは交差判定のみ。
    if let Some(content) = draw {
        let world_aabb = transform_aabb(&content.local_aabb, &world_mat);
        if aabb_intersects(&world_aabb, &new_eff) {
            out.push(DrawCommand::Draw(DrawItem {
                hash: hash_draw(content, &world_mat),
                world_mat,
                world_aabb,
                entity,
            }));
        }
    }

    // 子を宣言順に再帰（= 描画順 = Z 順）。new_eff を引き継ぐ。
    // クリップの無いノードでも new_eff == eff なので再帰は止めない
    //（子は親の描画範囲を超えて広がり得るため）。
    if let Some(children) = children {
        for child in children.iter() {
            visit(out, unit_rect, nodes, child, world_mat, new_eff);
        }
    }

    // pop clip（Rect/Geometry のときだけ）。
    if let Some(cmd) = pop {
        out.push(cmd);
    }
}

/// クリップを計画して 4 値 (`ClipKind`) を返す。push はまだ積まない。
///
/// `VisualClip::Rect` は world が矩形保存なら `PushClipRect`、回転/せん断があれば
/// 単位矩形 + 実効変換に正規化して `PushClipGeometry` へ昇格する（geometry 実体は
/// `unit_rect` を共有し、差分のポインタ同一性を保つ）。
///
/// いずれの分岐でも、クリップの world AABB を `eff` と交差させて `new_eff` を求め、
/// 空になれば `Culled` を返す。geometry クリップの world AABB は geometry の
/// バウンディングボックス（保守的な上位集合）なので、それで刈っても実クリップより
/// 広く残るだけで健全。
///
/// クリップの無いノードは決して `Culled` を返さない（`NoClip`）。これにより
/// 「自ノードが不可視でも子は見え得る」という部分木カリングの健全性が保たれる。
fn plan_clip(
    clip: Option<&VisualClip>,
    unit_rect: &ID2D1Geometry,
    entity: Entity,
    world_mat: Matrix3x2,
    eff: Aabb,
) -> ClipKind {
    let Some(clip) = clip else {
        return ClipKind::NoClip;
    };

    match clip {
        VisualClip::Rect(local) => {
            if is_rect_preserving(&world_mat) {
                // 矩形保存 → そのまま world 矩形へ焼き込み。world_aabb は厳密。
                let world_aabb = transform_aabb(local, &world_mat);
                let new_eff = aabb_intersect(&eff, &world_aabb);
                if aabb_is_empty(&new_eff) {
                    return ClipKind::Culled;
                }
                ClipKind::Rect {
                    push: DrawCommand::PushClipRect(ClipRect { world_aabb }),
                    new_eff,
                }
            } else {
                // 昇格: 単位矩形→local矩形 に world を合成した実効変換に畳む。
                let m = unit_to_rect(local) * world_mat;
                let world_aabb = geometry_bounds(unit_rect, &m);
                let new_eff = aabb_intersect(&eff, &world_aabb);
                if aabb_is_empty(&new_eff) {
                    return ClipKind::Culled;
                }
                ClipKind::Geometry {
                    push: DrawCommand::PushClipGeometryRect(ClipGeometryRect {
                        world_mat: m,
                        world_aabb,
                        geometry: unit_rect.clone(), // 共有 → ポインタ安定（clone は AddRef のみ）
                    }),
                    new_eff,
                }
            }
        }
        VisualClip::Geometry(local_geom) => {
            // local geometry は遅延適用（emit 側で SetTransform(world_mat)）。
            // 有効 AABB は geometry のバウンディングボックスで保守的に絞る。
            let world_aabb = geometry_bounds(local_geom, &world_mat);
            let new_eff = aabb_intersect(&eff, &world_aabb);
            if aabb_is_empty(&new_eff) {
                return ClipKind::Culled;
            }
            ClipKind::Geometry {
                push: DrawCommand::PushClipGeometryEntity(ClipGeometryEntity {
                    world_mat,
                    world_aabb,
                    entity,
                }),
                new_eff,
            }
        }
    }
}

/// 2 つの AABB の交差領域。空になり得る（`aabb_is_empty` で判定）。
fn aabb_intersect(a: &Aabb, b: &Aabb) -> Aabb {
    Aabb::new(
        a.left().max(b.left()),
        a.top().max(b.top()),
        a.right().min(b.right()),
        a.bottom().min(b.bottom()),
    )
}

/// 退化（幅 or 高さ ≤ 0）なら空とみなす。ゼロ面積クリップもここで捕捉される。
fn aabb_is_empty(a: &Aabb) -> bool {
    a.right() <= a.left() || a.bottom() <= a.top()
}

/// 2 つの AABB が正の面積で重なるか（境界接触のみは非交差扱い）。
fn aabb_intersects(a: &Aabb, b: &Aabb) -> bool {
    a.left() < b.right() && b.left() < a.right() && a.top() < b.bottom() && b.top() < a.bottom()
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

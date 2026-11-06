use super::super::*;
use super::command::*;
use crate::ecs::graphics::clip::ClipShape;
use crate::ecs::layout::Arrangement;
use crate::numerics::*;
use windows::Win32::Graphics::Direct2D::*;
use windows_numerics::*;

/// visit が 1 ノードから読み取る不変ビュー（すべて read-only）。
///
/// - `VisualTransform`   : 無ければ identity 扱い。
/// - `VisualDrawContent` : 無ければ描画コマンドを出さない。
/// - `Visual`            : `is_visible` が false なら部分木ごと出さない。`clip` が無ければ push/pop なし。
/// - `Arrangement`       : 矩形・角丸の切り抜きの大きさ。無ければ大きさ 0。
/// - `Children`          : 無ければ葉ノード。
///
/// 5 つを 1 本に束ねることで、1 エンティティ = 1 回の `Query::get`
/// ルックアップで済む（クエリを分けると get が複数回になる）。
type NodeData = (
    Option<&'static VisualTransform>,
    Option<&'static VisualDrawContent>,
    Option<&'static Visual>,
    Option<&'static Arrangement>,
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
#[allow(
    dead_code,
    reason = "`wintf-visual-tree` が本番へ上げるまで使い手が無い"
)]
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
    if eff.is_empty() {
        return;
    }

    // despawn 済み / クエリ非該当は静かにスキップ（部分木ごと出力しない）。
    let Ok((transform, draw, visual, arrangement, children)) = nodes.get(entity) else {
        return;
    };
    // 見えない節は子孫ごと出さない（1.7）。
    if visual.is_some_and(|v| !v.is_visible) {
        return;
    }
    let clip = visual.and_then(|v| v.clip.as_ref());
    // shortcut: 矩形も角丸も Arrangement の大きさの矩形として切る（角丸は角を落とさない）、5.2・5.3 で本来の形へ。
    let clip_rect = arrangement.map_or(Aabb::new(0.0, 0.0, 0.0, 0.0), |a| {
        Aabb::new(0.0, 0.0, a.size.width, a.size.height)
    });

    // world = local * parent（行ベクトル規約: v' = v * local * parent）。
    let local = transform
        .map(|t| t.0.into())
        .unwrap_or(Matrix3x2::identity());
    let world_mat = local * parent_world;

    // ① クリップ計画（4 値）。push はここでは積まない。
    // ③ Culled は push を出す *前* に return するので bracket（push/pop 対）は均衡を保つ。
    let (new_eff, pop) = match plan_clip(clip, &clip_rect, unit_rect, world_mat, eff) {
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
        let world_aabb = content.local_aabb.transform(world_mat);
        if !world_aabb.intersect(new_eff).is_empty() {
            out.push(DrawCommand::Draw(DrawItem {
                hash: hash_draw(content, &world_mat),
                world_mat,
                world_aabb,
                entity,
                command_list: content.command_list.clone(),
                interpolation_mode: content.interpolation_mode,
                composite_mode: content.composite_mode,
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
/// 矩形・角丸（`clip_rect` に読み替え済み）は world が矩形保存なら `PushClipRect`、回転/せん断があれば
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
    clip: Option<&ClipShape>,
    clip_rect: &Aabb,
    unit_rect: &ID2D1Geometry,
    world_mat: Matrix3x2,
    eff: Aabb,
) -> ClipKind {
    let Some(clip) = clip else {
        return ClipKind::NoClip;
    };

    match clip {
        ClipShape::Rectangle
        | ClipShape::RoundedRectangle { .. }
        | ClipShape::RoundedRectangleIndividual { .. } => {
            let local = clip_rect;
            if world_mat.axis_aligned() {
                // 矩形保存 → そのまま world 矩形へ焼き込み。world_aabb は厳密。
                let world_aabb = local.transform(world_mat);
                let new_eff = eff.intersect(world_aabb);
                if new_eff.is_empty() {
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
                let new_eff = eff.intersect(world_aabb);
                if new_eff.is_empty() {
                    return ClipKind::Culled;
                }
                ClipKind::Geometry {
                    push: DrawCommand::PushClipGeometry(ClipGeometry {
                        world_mat: m,
                        world_aabb,
                        geometry: unit_rect.clone(), // 共有 → ポインタ安定（clone は AddRef のみ）
                    }),
                    new_eff,
                }
            }
        }
        ClipShape::Geometry(local_geom) => {
            // 局所の図形はそのまま持ち、世界への変換は world_mat で渡す。
            // 有効 AABB は geometry のバウンディングボックスで保守的に絞る。
            let world_aabb = geometry_bounds(local_geom, &world_mat);
            let new_eff = eff.intersect(world_aabb);
            if new_eff.is_empty() {
                return ClipKind::Culled;
            }
            ClipKind::Geometry {
                push: DrawCommand::PushClipGeometry(ClipGeometry {
                    world_mat,
                    world_aabb,
                    geometry: local_geom.clone(),
                }),
                new_eff,
            }
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

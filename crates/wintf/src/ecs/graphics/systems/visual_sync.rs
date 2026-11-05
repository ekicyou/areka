use super::init::format_entity_name;
use crate::ecs::common::ancestors;
use crate::ecs::graphics::{Visual, VisualGraphics};
use crate::ecs::layout::{Arrangement, GlobalArrangement};
use crate::ecs::window::Window;
use bevy_ecs::hierarchy::{ChildOf, Children};
use bevy_ecs::name::Name;
use bevy_ecs::prelude::*;
use tracing::{debug, error};
use windows::UI::Composition::{ContainerVisual, Visual as WucVisual};
use windows::core::Interface;

/// 同じ WUC の見た目か（控えは `VisualGraphics::visual()` の複製なので同じポインタになる）。
fn same_visual(a: Option<&WucVisual>, b: &WucVisual) -> bool {
    a.is_some_and(|a| a.as_raw() == b.as_raw())
}

/// ECS の ChildOf/Children の木を WUC の見た目の木へ写すシステム (R3, R6, R7)
///
/// 未同期の見分けは「控えの親（`parent_visual`）が、いま居るべき親と違う」。居るべき親は、
/// - 親が `VisualGraphics` を持ち見た目がある: その見た目
/// - 親が `VisualGraphics` を持つが見た目がまだ無い: 保留（何もしない）
/// - 親が `VisualGraphics` を持たない（LayoutRoot など）: 根（自分自身の見た目を控えに置く）
///
/// 付け替え（ChildOf の変更）では、古い親の子から外してから新しい親を同期し直す。
/// 親が後から見た目を持てば、根の印と違うので同期し直される。
///
/// 重要: 親→子の順序で挿入する必要がある。深さでソートして親が先に処理されるようにする。
/// 兄弟間のZ-orderはChildrenコンポーネントの順序を権威的ソースとする。
///
/// 係には `Err` を返す先が無いので、失敗は記録だけにする（外す失敗は親が先に消えた場合が
/// あるので `debug!`、子の全部を外す失敗は `error!`）。
pub fn visual_hierarchy_sync_system(
    mut vg_queries: ParamSet<(
        // ChildOf + VisualGraphics を持つ全Entity
        Query<(Entity, &ChildOf, &mut VisualGraphics, Option<&Name>)>,
        // VisualGraphics を持つ全Entity（読むだけ）
        Query<(Entity, &VisualGraphics, Option<&ChildOf>, Option<&Name>)>,
    )>,
    child_of_query: Query<&ChildOf>,
    // Children コンポーネント（兄弟順序の権威的ソース）
    children_query: Query<&Children>,
) {
    // Phase 1: 未同期の子を持つ親と、古い親から外す見た目を集める
    let mut affected_parents = std::collections::HashSet::new();
    // 深さ付き親情報: (parent_entity, depth) — depth は親の深さ（浅い順に処理するため）
    let mut parent_depths: std::collections::HashMap<Entity, usize> =
        std::collections::HashMap::new();
    // (古い親の見た目, 子の見た目)
    let mut detach: Vec<(WucVisual, WucVisual)> = Vec::new();
    {
        let query = vg_queries.p1();
        for (_entity, child_vg, child_of, _name) in query.iter() {
            let (Some(child_of), Some(own)) = (child_of, child_vg.visual()) else {
                continue;
            };
            let parent_entity = child_of.parent();
            let expected = match query.get(parent_entity) {
                Ok((_, parent_vg, _, _)) => match parent_vg.visual() {
                    Some(v) => v,
                    None => continue, // 親の見た目がまだ無い: 保留
                },
                Err(_) => own, // 親が見た目の部品を持たない: 根
            };
            let cached = child_vg.parent_visual();
            if same_visual(cached, expected) {
                continue;
            }
            if let Some(old) = cached.filter(|old| !same_visual(Some(own), old)) {
                detach.push((old.clone(), own.clone()));
            }
            affected_parents.insert(parent_entity);
            parent_depths.entry(parent_entity).or_insert_with(|| {
                ancestors(parent_entity, |e| {
                    child_of_query.get(e).ok().map(|c| c.parent())
                })
                .count()
            });
        }
    }

    if affected_parents.is_empty() {
        return;
    }

    // Phase 2: 付け替えた子を古い親の子から外す（親が先に消えた場合があるので debug!）
    for (old_parent, child) in &detach {
        if let Err(e) = old_parent
            .cast::<ContainerVisual>()
            .and_then(|cv| cv.Children())
            .and_then(|c| c.Remove(child))
        {
            debug!(error = ?e, "[visual_hierarchy_sync] Remove from old parent failed");
        }
    }

    // Phase 3: affected_parents を深さでソート（浅い親が先 → 親→子の順序で処理）
    let mut sorted_parents: Vec<Entity> = affected_parents.into_iter().collect();
    sorted_parents.sort_by_key(|e| parent_depths.get(e).copied().unwrap_or(0));

    debug!(
        count = sorted_parents.len(),
        "[visual_hierarchy_sync] Processing affected parents (sorted by depth)"
    );

    // Phase 4: 各 affected_parent について Children 順で子 Visual を再配置
    for parent_entity in sorted_parents {
        let parent_depth = parent_depths.get(&parent_entity).copied().unwrap_or(0);

        // 親の Visual と名前（ログ用）を取得。None = 親が見た目の部品を持たない（根）
        let (parent_visual, parent_name) = {
            let parent_query = vg_queries.p1();
            match parent_query.get(parent_entity) {
                Ok((_, pv, _, name)) => match pv.visual() {
                    Some(v) => (Some(v.clone()), name.map(|n| n.to_string())),
                    None => continue,
                },
                Err(_) => (None, None),
            }
        };
        let parent_fallback = format!("Entity({:?})", parent_entity);
        let parent_display = parent_name.as_deref().unwrap_or(&parent_fallback);

        if let Some(ref parent_visual) = parent_visual {
            // 親の VisualCollection を取得（基底 Visual は Children を持たないため
            // ContainerVisual へ cast する）。
            let parent_children = match parent_visual
                .cast::<ContainerVisual>()
                .and_then(|cv| cv.Children())
            {
                Ok(c) => c,
                Err(e) => {
                    error!(
                        parent = %parent_display,
                        error = ?e,
                        "[visual_hierarchy_sync] Children() failed"
                    );
                    continue;
                }
            };

            // 親の既存 Visual 子をすべて削除して Z-order をリセット
            if let Err(e) = parent_children.RemoveAll() {
                error!(
                    parent = %parent_display,
                    error = ?e,
                    "[visual_hierarchy_sync] RemoveAll failed"
                );
                continue;
            }

            // Children コンポーネントの順序で子 Visual を再追加。
            //
            // Z順序等価性（要件 5.2）: DComp の `add_visual(insertabove=false, ref=None)` は
            // 「子リストの先頭（＝全兄弟の下・最初に描画）へ挿入」。Children を [A,B,C] の順に
            // 反復して毎回この呼び出しを行うと、最終スタック（下→上）は C,B,A（最後に反復した
            // 子が最下・最初の子が最上）になる。WUC で同一スタックを再現するには、同じ反復順で
            // `InsertAtBottom` を呼ぶ（InsertAtBottom(A)→[A], InsertAtBottom(B)→[B,A],
            // InsertAtBottom(C)→[C,B,A]）。ゆえに InsertAtTop ではなく InsertAtBottom を選ぶ。
            if let Ok(children) = children_query.get(parent_entity) {
                for child_entity in children.iter() {
                    let mut child_query = vg_queries.p0();
                    if let Ok((_, _, mut child_vg, child_name)) = child_query.get_mut(child_entity)
                    {
                        let child_visual = match child_vg.visual() {
                            Some(v) => v.clone(),
                            None => continue,
                        };

                        let child_name_str = child_name.map(|n| n.to_string());
                        let child_fallback = format!("Entity({:?})", child_entity);
                        let child_display = child_name_str.as_deref().unwrap_or(&child_fallback);

                        if let Err(e) = parent_children.InsertAtBottom(&child_visual) {
                            error!(
                                child = %child_display,
                                depth = parent_depth + 1,
                                parent = %parent_display,
                                error = ?e,
                                "[visual_hierarchy_sync] InsertAtBottom failed"
                            );
                        } else {
                            debug!(
                                child = %child_display,
                                depth = parent_depth + 1,
                                parent = %parent_display,
                                "[visual_hierarchy_sync] InsertAtBottom success"
                            );
                        }

                        // 控えが同じなら書かない（変化の印を立てない）
                        if !same_visual(child_vg.parent_visual(), parent_visual) {
                            child_vg.set_parent_visual(Some(parent_visual.clone()));
                        }
                    }
                }
            }
        } else {
            // 親が VisualGraphics を持たない場合: 各子を Visual 階層のルートとしてマーク
            if let Ok(children) = children_query.get(parent_entity) {
                for child_entity in children.iter() {
                    let mut child_query = vg_queries.p0();
                    if let Ok((_, _, mut child_vg, child_name)) = child_query.get_mut(child_entity)
                    {
                        let child_visual = match child_vg.visual() {
                            Some(v) => v.clone(),
                            None => continue,
                        };
                        if same_visual(child_vg.parent_visual(), &child_visual) {
                            continue; // 既に根の印
                        }

                        let child_name_str = child_name.map(|n| n.to_string());
                        let child_fallback = format!("Entity({:?})", child_entity);
                        let child_display = child_name_str.as_deref().unwrap_or(&child_fallback);

                        debug!(
                            name = %child_display,
                            depth = parent_depth + 1,
                            "[visual_hierarchy_sync] Visual hierarchy root"
                        );
                        // 自分自身の Visual を設定して「根」と印す
                        child_vg.set_parent_visual(Some(child_visual));
                    }
                }
            }
        }
    }
}

/// Visual プロパティ同期システム (R8)
///
/// ArrangementまたはOpacity変更、または見た目の作り直しを検知してVisualのプロパティを同期する。
/// - Arrangement.offset → Visual.SetOffsetX/SetOffsetY
/// - Opacity → Visual.SetOpacity
///
/// DirectCompositionのVisual Offsetは物理ピクセル単位で指定する必要がある。
/// Arrangement.offset（論理座標）にGlobalArrangement.transform（累積DPIスケール）を適用して物理座標に変換。
///
/// Compositionスケジュールで実行。
pub fn visual_property_sync_system(
    changed_entities: Query<
        (
            Entity,
            &Arrangement,
            &GlobalArrangement,
            &Visual,
            &VisualGraphics,
            Option<&Name>,
            Has<Window>,
        ),
        Or<(
            Changed<Arrangement>,
            Changed<GlobalArrangement>,
            Changed<Visual>,
            // 見た目が作り直された部品にも写す
            Changed<VisualGraphics>,
        )>,
    >,
) {
    for (entity, arrangement, global_arrangement, visual_component, vg, name, is_window) in
        changed_entities.iter()
    {
        let Some(visual_com) = vg.visual() else {
            continue;
        };

        let entity_name = format_entity_name(entity, name);

        // Offset同期: Arrangementのoffset（論理座標）にGlobalArrangementのscale（累積DPIスケール）を適用
        // ただし、WindowエンティティはWin32がCompositionTargetを通じて位置を管理するため、
        // offsetを設定すると二重にオフセットが適用されてしまう。Windowはスキップする。
        if !is_window {
            // GlobalArrangementから累積スケールを取得（親からのDPIスケールを含む）
            let scale_x = global_arrangement.scale_x();
            let scale_y = global_arrangement.scale_y();
            // 論理座標 × 累積スケール = 物理ピクセル座標
            let offset_x = arrangement.offset.x * scale_x;
            let offset_y = arrangement.offset.y * scale_y;

            // WUC は SetOffset(Vector3) 1 回で x,y を設定（DComp は SetOffsetX2/Y2 の 2 回）。
            // z は DComp 非設定に合わせ 0.0。
            let offset = windows_numerics::Vector3 {
                X: offset_x,
                Y: offset_y,
                Z: 0.0,
            };
            if let Err(e) = visual_com.SetOffset(offset) {
                error!(
                    entity = %entity_name,
                    error = ?e,
                    "[visual_property_sync] SetOffset failed"
                );
            }
        }

        // Opacity同期: Visual.opacityを読み取り、is_visible=falseなら0.0を設定
        let opacity_value = if !visual_component.is_visible {
            0.0
        } else {
            visual_component.clamped_opacity()
        };
        if let Err(e) = visual_com.SetOpacity(opacity_value) {
            error!(
                entity = %entity_name,
                error = ?e,
                "[visual_property_sync] SetOpacity failed"
            );
        }
    }
}

#[cfg(test)]
#[path = "visual_sync_tests.rs"]
mod visual_sync_tests;

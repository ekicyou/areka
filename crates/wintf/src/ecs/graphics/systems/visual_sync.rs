use super::init::format_entity_name;
use crate::ecs::graphics::{Visual, VisualGraphics};
use crate::ecs::layout::{Arrangement, GlobalArrangement};
use crate::ecs::window::Window;
use bevy_ecs::hierarchy::{ChildOf, Children};
use bevy_ecs::name::Name;
use bevy_ecs::prelude::*;
use tracing::{debug, error};
use windows::core::Interface;

/// ChildOf変更またはVisualGraphics追加を検出してVisual階層を同期するシステム (R3, R6, R7)
///
/// ECSのChildOf/Children階層とDirectCompositionのVisual階層を同期する。
/// Phase 6改善: parent_visual.is_none()で未同期を検出し、毎フレーム処理可能に。
/// - ChildOf追加/変更時: 子VisualGraphicsを親VisualGraphicsに追加
/// - VisualGraphics追加時: 既存のChildOf関係を元に親Visualに追加
/// - parent_visualがNone: 階層未構築なので同期実行
/// - 親がVisualGraphicsを持たない場合（LayoutRootなど）: Visual階層のルートとして処理済みマーク
///
/// 重要: 親→子の順序でAddVisualを呼ぶ必要がある（DirectComposition要件）。
/// 深さでソートして親が先に処理されるようにする。
/// 兄弟間のZ-orderはChildrenコンポーネントの順序を権威的ソースとする。
///
/// 注意: エラーは無視する（親が先に削除されている場合など）
pub fn visual_hierarchy_sync_system(
    mut vg_queries: ParamSet<(
        // ChildOf + VisualGraphics を持つ全Entity
        Query<(Entity, &ChildOf, &mut VisualGraphics, Option<&Name>)>,
        // 親エンティティクエリ
        Query<(&VisualGraphics, Option<&Name>)>,
    )>,
    child_of_query: Query<&ChildOf>,
    // Children コンポーネント（兄弟順序の権威的ソース）
    children_query: Query<&Children>,
) {
    // Phase 1: 未同期（parent_visual==None）のエンティティと親情報を収集
    // affected_parents: 未同期の子を持つ親エンティティの集合
    let mut affected_parents = std::collections::HashSet::new();
    // 深さ付き親情報: (parent_entity, depth) — depth は親の深さ（浅い順に処理するため）
    let mut parent_depths: std::collections::HashMap<Entity, usize> =
        std::collections::HashMap::new();
    {
        let child_query = vg_queries.p0();
        for (_entity, child_of, child_vg, _child_name) in child_query.iter() {
            // parent_visualがNoneなら未同期
            // NOTE(W3b-V): この検出は「キャッシュ未設定」のみを未同期と見なすため、
            // 既に親 Visual を持つ子の ChildOf 変更（再ペアレント）は検出されない。
            // 再ペアレント後は旧親の DComp Visual に接続されたまま ECS 階層と乖離し、
            // 後で旧親側が別の未同期子により再同期されると remove_all_visuals で
            // 切り離されたまま新親へ再接続されない（tests/visual/
            // hierarchy_reparent_gap_test.rs で現行挙動を固定。修正は挙動変更のため
            // P47 に記録）。
            if child_vg.parent_visual().is_none() {
                let parent_entity = child_of.parent();
                affected_parents.insert(parent_entity);

                // 親の深さを計算（まだ計算していない場合）
                // NOTE(W3b-V): この走査は ChildOf チェーンの終端到達を前提とする
                // （間接巡回 A→B→A があると終了しない。巡回ガードは P48 参照）。
                if !parent_depths.contains_key(&parent_entity) {
                    let mut depth = 0;
                    let mut current = parent_entity;
                    while let Ok(co) = child_of_query.get(current) {
                        depth += 1;
                        current = co.parent();
                    }
                    parent_depths.insert(parent_entity, depth);
                }
            }
        }
    }

    if affected_parents.is_empty() {
        return;
    }

    // Phase 2: affected_parents を深さでソート（浅い親が先 → 親→子の順序で処理）
    let mut sorted_parents: Vec<Entity> = affected_parents.into_iter().collect();
    sorted_parents.sort_by_key(|e| parent_depths.get(e).copied().unwrap_or(0));

    debug!(
        count = sorted_parents.len(),
        "[visual_hierarchy_sync] Processing affected parents (sorted by depth)"
    );

    // Phase 3: 各 affected_parent について Children 順で子 Visual を再配置
    for parent_entity in sorted_parents {
        let parent_depth = parent_depths.get(&parent_entity).copied().unwrap_or(0);

        // 親の Visual と名前（ログ用）を取得
        let (parent_visual, parent_name) = {
            let parent_query = vg_queries.p1();
            match parent_query.get(parent_entity) {
                Ok((pv, name)) => (pv.visual().cloned(), name.map(|n| n.to_string())),
                Err(_) => (None, None),
            }
        };
        let parent_fallback = format!("Entity({:?})", parent_entity);
        let parent_display = parent_name.as_deref().unwrap_or(&parent_fallback);

        if let Some(ref parent_visual) = parent_visual {
            // 親の VisualCollection を取得（WUC。基底 Visual は Children を持たないため
            // ContainerVisual/SpriteVisual へ cast する必要があるが、Children() は
            // ICompositor経由で cast 済みのため Visual からは呼べない。SpriteVisual へ cast）。
            let parent_children = match parent_visual
                .cast::<windows::UI::Composition::ContainerVisual>()
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
            let _ = parent_children.RemoveAll();

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

                        // parent_visual キャッシュを更新
                        child_vg.set_parent_visual(Some(parent_visual.clone()));
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
                        if child_vg.parent_visual().is_some() {
                            continue; // 既に同期済み
                        }
                        let child_visual = match child_vg.visual() {
                            Some(v) => v.clone(),
                            None => continue,
                        };

                        let child_name_str = child_name.map(|n| n.to_string());
                        let child_fallback = format!("Entity({:?})", child_entity);
                        let child_display = child_name_str.as_deref().unwrap_or(&child_fallback);

                        debug!(
                            name = %child_display,
                            depth = parent_depth + 1,
                            "[visual_hierarchy_sync] Visual hierarchy root"
                        );
                        // 自分自身の Visual を設定して「処理済み」とマーク
                        child_vg.set_parent_visual(Some(child_visual.clone()));
                    }
                }
            }
        }
    }
}

/// Visual プロパティ同期システム (R8)
///
/// ArrangementまたはOpacity変更を検知してVisualのプロパティを同期する。
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

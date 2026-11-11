//! Taffyレイアウトエンジン連携システム

use bevy_ecs::hierarchy::{ChildOf, Children};
use bevy_ecs::name::Name;
use bevy_ecs::prelude::*;
use tracing::{debug, error, trace, warn};

use super::super::metrics::{LayoutScale, Offset, Size};
use super::super::taffy::{TaffyComputedLayout, TaffyLayoutResource, TaffyStyle};
use super::super::{Arrangement, ArrangementTreeChanged, BoxStyle, Dimension, LayoutRoot};
use crate::ecs::graphics::format_entity_name;
use crate::ecs::window::DPI;
use taffy::prelude::*;

/// BoxStyleからTaffyStyleを構築するシステム（統合後）
///
/// # 変更点（BoxStyle統合）
/// - 旧: 8コンポーネント（BoxSize, BoxMargin, BoxPadding, BoxPosition, BoxInset, FlexContainer, FlexItem, LayoutRoot）
/// - 新: BoxStyle + LayoutRoot の2コンポーネントのみ
///
/// # 動作
/// 1. BoxStyleまたはLayoutRootを持ち、TaffyStyleがないエンティティにTaffyStyleを自動挿入
/// 2. BoxStyleが変更されたエンティティのTaffyStyleを更新
/// 3. LayoutRootのみでBoxStyleがないエンティティにはBoxStyle::default()相当のスタイルを適用
/// 4. BoxStyleが外れたエンティティは、LayoutRootなら既定のスタイルへ戻し、
///    そうでなければTaffyStyleと計算結果を外す（Arrangementは最後の値のまま残す）
pub fn build_taffy_styles_system(
    mut commands: Commands,
    // LayoutRootまたはBoxStyleがあるがTaffyStyleがないエンティティ
    // LayoutRootのみの場合はBoxStyle::default()相当のスタイルを適用
    without_style: Query<
        (Entity, Option<&BoxStyle>),
        (Or<(With<LayoutRoot>, With<BoxStyle>)>, Without<TaffyStyle>),
    >,
    // BoxStyleが変更されたエンティティ
    mut changed: Query<(&BoxStyle, &mut TaffyStyle), Changed<BoxStyle>>,
    // BoxStyleが外れたエンティティ（同じ回に付け直したものは除く）
    mut removed_box_styles: RemovedComponents<BoxStyle>,
    mut unstyled: Query<(Has<LayoutRoot>, Option<&mut TaffyStyle>), Without<BoxStyle>>,
    // 外した部品の子と名前（子が配置から外れる warn に使う）
    parts: Query<(Option<&Children>, Option<&Name>)>,
    styled: Query<(), With<TaffyStyle>>,
) {
    for entity in removed_box_styles.read() {
        let Ok((is_root, taffy_style)) = unstyled.get_mut(entity) else {
            continue; // 消えた部品か、もうBoxStyleを付け直した部品
        };
        if is_root {
            if let Some(mut taffy_style) = taffy_style {
                taffy_style.0 = taffy::Style::default();
            }
        } else {
            // 子の節は taffy の木から切れ、この節が BoxStyle を持ち直すまでその子は配置されない
            if let Ok((Some(children), name)) = parts.get(entity)
                && children.iter().any(|child| styled.contains(child))
            {
                warn!(
                    entity = %format_entity_name(entity, name),
                    "[build_taffy_styles] BoxStyle removed while children still have TaffyStyle; they are not laid out until it gets a BoxStyle again"
                );
            }
            commands
                .entity(entity)
                .try_remove::<(TaffyStyle, TaffyComputedLayout)>();
        }
    }

    // TaffyStyle自動挿入
    for (entity, box_style) in without_style.iter() {
        // BoxStyleがない場合（LayoutRootのみ）はデフォルトスタイル
        let taffy_style: taffy::Style = box_style.map(|s| s.into()).unwrap_or_default();
        commands.entity(entity).insert((
            TaffyStyle(taffy_style),
            TaffyComputedLayout::default(),
            ArrangementTreeChanged,
        ));
    }

    // 変更反映
    for (box_style, mut taffy_style) in changed.iter_mut() {
        taffy_style.0 = box_style.into();
    }
}

/// TaffyツリーをECS階層と同期
pub fn sync_taffy_tree_system(
    mut taffy_res: ResMut<TaffyLayoutResource>,
    // TaffyStyleが付いたばかりのエンティティ（ノードが無ければ作る・親子をつなぎ直す）
    new_entities: Query<(Entity, Option<&ChildOf>, Has<Children>), Added<TaffyStyle>>,
    // TaffyStyleが変更されたエンティティ
    changed_styles: Query<(Entity, &TaffyStyle), Changed<TaffyStyle>>,
    // 階層が変更されたエンティティ
    changed_hierarchy: Query<&ChildOf, Changed<ChildOf>>,
    // ChildOfが削除されたエンティティ
    mut removed_hierarchy: RemovedComponents<ChildOf>,
    // TaffyStyleが外れたエンティティ（節を消すのは計算の後の掃除の係）
    mut removed_styles: RemovedComponents<TaffyStyle>,
    // Children コンポーネント（兄弟順序の権威的ソース）と、記録に出す名前（warn のときだけ引く）
    hierarchy: Query<(Option<&Children>, Option<&Name>)>,
) {
    // 係には Err を返す先が無いので、taffy の失敗は記録だけにする（以下同じ）。
    // 新規エンティティにtaffyノードを作成し、その親と自分（子を持つなら）を影響親に足す
    // （節が前から在る部品も、後から BoxStyle を付けた部品も、ここで親子がつながる）
    let mut affected_parents = std::collections::HashSet::new();
    for (entity, child_of, has_children) in new_entities.iter() {
        if taffy_res.get_node(entity).is_none()
            && let Err(e) = taffy_res.create_node(entity)
        {
            error!(?entity, error = ?e, "[sync_taffy_tree] Taffy node creation failed");
        }
        if let Some(child_of) = child_of {
            affected_parents.insert(child_of.parent());
        }
        if has_children {
            affected_parents.insert(entity);
        }
    }
    // 親に節が無い部品は、親が BoxStyle を持つまで木につながらず配置されない（同じ回に親の節ができたなら出さない）
    for (entity, child_of, _) in new_entities.iter() {
        if let Some(child_of) = child_of
            && taffy_res.get_node(child_of.parent()).is_none()
        {
            let parent = child_of.parent();
            warn!(
                entity = %format_entity_name(entity, hierarchy.get(entity).ok().and_then(|h| h.1)),
                parent = %format_entity_name(parent, hierarchy.get(parent).ok().and_then(|h| h.1)),
                "[sync_taffy_tree] Parent has no taffy node; this node is not laid out until the parent gets a BoxStyle"
            );
        }
    }

    // TaffyStyleの変更をtaffyツリーに反映
    for (entity, style) in changed_styles.iter() {
        if let Some(node_id) = taffy_res.get_node(entity) {
            if let Err(e) = taffy_res.taffy_mut().set_style(node_id, style.0.clone()) {
                error!(?entity, error = ?e, "[sync_taffy_tree] Taffy set_style failed");
            }
        }
    }

    // 階層変更を処理: Children コンポーネントの順序を権威的ソースとして使用
    // Step 1: changed_hierarchy から影響を受けた親エンティティを収集（重複排除）
    for child_of in changed_hierarchy.iter() {
        affected_parents.insert(child_of.parent());
    }

    // Step 2: 各影響親について Children の順序で taffy 子ノードを一括設定
    for parent_entity in affected_parents {
        if let Some(parent_node) = taffy_res.get_node(parent_entity) {
            if let Ok((Some(children), _)) = hierarchy.get(parent_entity) {
                // Children.iter() の順序で NodeId リストを構築
                // taffy ノード未作成のエンティティはスキップ
                let ordered_node_ids: Vec<taffy::NodeId> = children
                    .iter()
                    .filter_map(|child_entity| taffy_res.get_node(child_entity))
                    .collect();
                if let Err(e) = taffy_res
                    .taffy_mut()
                    .set_children(parent_node, &ordered_node_ids)
                {
                    error!(entity = ?parent_entity, error = ?e, "[sync_taffy_tree] Taffy set_children failed");
                }
            }
        }
    }

    // ChildOfかTaffyStyleが削除された場合の処理: taffy の親から外す
    // （掃除の係は計算の後に並ぶので、ここで外さないとその回の計算が消える前の節を含む）
    for entity in removed_hierarchy.read().chain(removed_styles.read()) {
        if let Some(node_id) = taffy_res.get_node(entity)
            && let Some(parent_node) = taffy_res.taffy().parent(node_id)
            && let Err(e) = taffy_res.taffy_mut().remove_child(parent_node, node_id)
        {
            error!(?entity, error = ?e, "[sync_taffy_tree] Taffy remove_child failed");
        }
    }
}

/// Taffyレイアウト計算を実行
pub fn compute_taffy_layout_system(
    mut taffy_res: ResMut<TaffyLayoutResource>,
    // LayoutRootマーカーを持つエンティティをレイアウトルートとして扱う
    roots: Query<(Entity, Option<&BoxStyle>), With<LayoutRoot>>,
    // 変更検知用
    changed_styles: Query<(), Changed<TaffyStyle>>,
    changed_box_styles: Query<(), Changed<BoxStyle>>,
    changed_hierarchy: Query<(), Changed<ChildOf>>,
    // 削除検知用（係ごとに読み位置を持つので、掃除の係と同じ記録を読める）
    mut removed_hierarchy: RemovedComponents<ChildOf>,
    mut removed_styles: RemovedComponents<TaffyStyle>,
    // TaffyComputedLayoutを書き込むクエリ
    mut all_taffy_entities: Query<(Entity, &mut TaffyComputedLayout), With<TaffyStyle>>,
) {
    // 消えた記録は毎回 2 つとも読み切る（is_empty は読み位置を進めず、次の回もまた計算してしまう）
    let removed = removed_hierarchy.read().count() + removed_styles.read().count();
    // BoxStyleまたはTaffyStyleの変更、階層の変更・削除のいずれかで再計算
    let has_changes = removed > 0
        || !changed_styles.is_empty()
        || !changed_box_styles.is_empty()
        || !changed_hierarchy.is_empty();

    // Changed検知時にレイアウト計算を実行
    if has_changes {
        for (root_entity, box_style) in roots.iter() {
            if let Some(root_node) = taffy_res.get_node(root_entity) {
                // LayoutRootのBoxStyleからavailable_spaceを構築
                // Px指定の軸のみDefinite、それ以外（Percent/Auto/未指定）はMaxContent
                let to_available = |d: Option<Dimension>| match d {
                    Some(Dimension::Px(px)) => AvailableSpace::Definite(px),
                    _ => AvailableSpace::MaxContent,
                };
                let root_size = box_style.and_then(|style| style.size);
                let available_space = taffy::Size {
                    width: to_available(root_size.and_then(|s| s.width)),
                    height: to_available(root_size.and_then(|s| s.height)),
                };

                if let Err(e) = taffy_res
                    .taffy_mut()
                    .compute_layout(root_node, available_space)
                {
                    error!(entity = ?root_entity, error = ?e, "[compute_taffy_layout] Taffy compute_layout failed");
                } else {
                    // 計算成功時、全エンティティにTaffyComputedLayoutを書き込む
                    for (entity, mut computed) in all_taffy_entities.iter_mut() {
                        if let Some(node_id) = taffy_res.get_node(entity) {
                            match taffy_res.taffy().layout(node_id) {
                                Err(e) => {
                                    error!(?entity, error = ?e, "[compute_taffy_layout] Taffy layout read failed");
                                }
                                Ok(layout) => {
                                    let new_layout = TaffyComputedLayout(*layout);
                                    // 値比較で変更検知を抑制
                                    if *computed != new_layout {
                                        tracing::debug!(
                                            entity = ?entity,
                                            old_width = computed.0.size.width,
                                            old_height = computed.0.size.height,
                                            new_width = new_layout.0.size.width,
                                            new_height = new_layout.0.size.height,
                                            "[compute_taffy_layout] Layout changed"
                                        );
                                        *computed = new_layout;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// TaffyComputedLayoutまたはDPIの変更をArrangementに反映
///
/// # Triggers
/// - TaffyComputedLayoutの追加・変更
/// - DPIコンポーネントの変更
///
/// # Behavior
/// どちらのトリガーでも全フィールド（offset, size, scale）を再計算
pub fn update_arrangements_system(
    mut commands: Commands,
    mut query: Query<
        (
            Entity,
            &TaffyComputedLayout,
            Option<&mut Arrangement>,
            Option<&Name>,
            Option<Ref<DPI>>,
            Option<&crate::ecs::window::Window>,
        ),
        (
            Or<(Changed<TaffyComputedLayout>, Changed<DPI>)>,
            With<TaffyStyle>,
        ),
    >,
) {
    for (entity, computed_layout, arrangement, name, dpi, window) in query.iter_mut() {
        let layout = &computed_layout.0;

        // Window エンティティの判定（scale と offset の両方で使用）
        let is_window = window.is_some();

        // Window エンティティの場合、Arrangement.scale に DPI スケールを設定する。
        // これにより変換伝播（GlobalArrangement = parent_GA * Arrangement）で
        // GA.bounds が物理ピクセル単位に正しく変換される。
        // Window 以外のエンティティは Taffy が論理 px で計算するため (1.0, 1.0) を維持。
        let scale = if is_window {
            if let Some(ref d) = dpi {
                LayoutScale {
                    x: d.scale_x(),
                    y: d.scale_y(),
                }
            } else {
                LayoutScale::default() // DPI コンポーネントなし → フォールバック
            }
        } else {
            LayoutScale::default() // 非 Window → (1.0, 1.0)
        };

        // デバッグ: DPI変更検知の確認
        if let Some(ref d) = dpi {
            if d.is_changed() {
                debug!(
                    entity = ?entity,
                    dpi_x = d.dpi_x,
                    dpi_y = d.dpi_y,
                    scale_x = scale.x,
                    scale_y = scale.y,
                    is_window = is_window,
                    "[update_arrangements] DPI changed, scale applied"
                );
            }
        }

        // Window エンティティの場合、offset は sync_window_arrangement_from_window_pos
        // で管理されるため、taffy の layout.location で上書きしない。
        // サイズとスケールのみ更新する。

        let new_offset = if is_window {
            // 既存の offset を維持、まだ存在しない場合は (0,0)
            arrangement
                .as_ref()
                .map_or(Offset { x: 0.0, y: 0.0 }, |arr| arr.offset)
        } else {
            Offset {
                x: layout.location.x,
                y: layout.location.y,
            }
        };

        let new_arrangement = Arrangement {
            offset: new_offset,
            scale,
            size: Size {
                width: layout.size.width,
                height: layout.size.height,
            },
        };

        // 名前はその記録が点いているときだけ作る（配置が変わるたびに通る道）
        let entity_name = || format_entity_name(entity, name);
        trace!(
            entity = %entity_name(),
            location_x = layout.location.x,
            location_y = layout.location.y,
            size_width = layout.size.width,
            size_height = layout.size.height,
            "[update_arrangements] TaffyLayout"
        );
        trace!(
            entity = %entity_name(),
            offset_x = new_arrangement.offset.x,
            offset_y = new_arrangement.offset.y,
            scale_x = new_arrangement.scale.x,
            scale_y = new_arrangement.scale.y,
            size_width = new_arrangement.size.width,
            size_height = new_arrangement.size.height,
            "[update_arrangements] Arrangement"
        );

        if let Some(mut arr) = arrangement {
            arr.set_if_neq(new_arrangement);
        } else {
            commands.entity(entity).insert(new_arrangement);
        }
    }
}

/// 削除されたエンティティのTaffyノードをクリーンアップ
pub fn cleanup_removed_entities_system(
    mut taffy_res: ResMut<TaffyLayoutResource>,
    mut removed: RemovedComponents<TaffyStyle>,
) {
    for entity in removed.read() {
        if let Err(e) = taffy_res.remove_node(entity) {
            error!(?entity, error = ?e, "[cleanup_removed_entities] Taffy node removal failed");
        }
    }
}

#[cfg(test)]
#[path = "taffy_systems_tests.rs"]
mod tests;

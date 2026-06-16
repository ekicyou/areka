// ========== Brush継承解決システム ==========

use crate::ecs::widget::{Brush, BrushInherit, Brushes, DEFAULT_BACKGROUND, DEFAULT_FOREGROUND};
use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::prelude::*;
use tracing::trace;

/// BrushInheritマーカーを持つエンティティのBrushesを解決するシステム
///
/// # 処理フロー
/// 1. With<BrushInherit>でクエリ（未解決のみ）
/// 2. Brushesがあれば Inherit フィールドのみ解決
/// 3. Brushesがなければ親から継承して新規挿入
/// 4. ルートに到達したらデフォルト値を適用
/// 5. BrushInheritマーカーを除去
///
/// # Requirements
/// - Req 4.4: resolve_inherited_brushesシステム
/// - Req 4.5: デフォルト色適用
pub fn resolve_inherited_brushes(
    mut commands: Commands,
    query: Query<(Entity, Option<&Brushes>), With<BrushInherit>>,
    parent_query: Query<&ChildOf>,
    all_brushes: Query<&Brushes>,
) {
    for (entity, brushes_opt) in query.iter() {
        // 親を辿って解決済みのBrushesを探す
        let parent_brushes = find_parent_brushes(entity, &parent_query, &all_brushes);

        // 解決後のBrushesを計算
        let resolved = match brushes_opt {
            Some(brushes) => {
                // 既存のBrushesがある場合、Inheritフィールドのみ解決
                resolve_brush_fields(brushes, parent_brushes.as_ref())
            }
            None => {
                // Brushesがない場合、親から継承または新規作成
                match parent_brushes {
                    Some(parent) => parent.clone(),
                    None => Brushes {
                        foreground: DEFAULT_FOREGROUND,
                        background: DEFAULT_BACKGROUND,
                    },
                }
            }
        };

        // Brushesを挿入/更新し、BrushInheritマーカーを除去
        commands
            .entity(entity)
            .insert(resolved)
            .remove::<BrushInherit>();

        trace!(
            entity = ?entity,
            "[resolve_inherited_brushes] Brushes resolved"
        );
    }
}

/// 親を辿って解決済みのBrushesを探す
fn find_parent_brushes(
    entity: Entity,
    parent_query: &Query<&ChildOf>,
    all_brushes: &Query<&Brushes>,
) -> Option<Brushes> {
    let mut current = entity;

    // NOTE(W3b-V): この走査は ChildOf チェーンの終端到達を前提とする
    // （間接巡回 A→B→A があると終了しない。巡回ガードは P48 参照）。
    while let Ok(child_of) = parent_query.get(current) {
        let parent = child_of.parent();

        // 親がBrushesを持っていて、両フィールドが解決済みならそれを返す
        if let Ok(parent_brushes) = all_brushes.get(parent) {
            if !parent_brushes.foreground.is_inherit() && !parent_brushes.background.is_inherit() {
                return Some(parent_brushes.clone());
            }
        }

        current = parent;
    }

    None
}

/// Brushesの各フィールドを解決する
fn resolve_brush_fields(brushes: &Brushes, parent_brushes: Option<&Brushes>) -> Brushes {
    let foreground = if brushes.foreground.is_inherit() {
        parent_brushes
            .and_then(|p| p.foreground.as_color())
            .map(Brush::Solid)
            .unwrap_or(DEFAULT_FOREGROUND)
    } else {
        brushes.foreground.clone()
    };

    let background = if brushes.background.is_inherit() {
        parent_brushes
            .and_then(|p| p.background.as_color())
            .map(Brush::Solid)
            .unwrap_or(DEFAULT_BACKGROUND)
    } else {
        brushes.background.clone()
    };

    Brushes {
        foreground,
        background,
    }
}

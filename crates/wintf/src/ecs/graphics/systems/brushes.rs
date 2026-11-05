// ========== Brush継承解決システム ==========

use crate::ecs::common::ancestors;
use crate::ecs::widget::{Brush, BrushInherit, Brushes, DEFAULT_BACKGROUND, DEFAULT_FOREGROUND};
use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::prelude::*;
use std::collections::HashMap;
use tracing::trace;

/// BrushInheritマーカーを持つエンティティのBrushesを解決するシステム
///
/// # 処理フロー
/// 1. With<BrushInherit>でクエリ（未解決のみ）し、深さの浅い順に並べる
/// 2. 前景・背景を別々に、決まった色を持つ最も近い祖先から取る
///    （この回に解決した色の表を先に見るので、同じ回に作った親子でも親の色が届く）
/// 3. 決まった祖先が無ければデフォルト値を適用
/// 4. Brushesを挿入し、BrushInheritマーカーを除去
///
/// 祖先の走査は上限つき（`ancestors`）なので、親の輪があっても終わる。
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
    let parent_of = |e: Entity| parent_query.get(e).ok().map(|c| c.parent());

    let mut pending: Vec<(usize, Entity, Option<&Brushes>)> = query
        .iter()
        .map(|(entity, brushes)| (ancestors(entity, parent_of).count(), entity, brushes))
        .collect();
    pending.sort_by_key(|(depth, _, _)| *depth);

    let mut resolved_this_run: HashMap<Entity, Brushes> = HashMap::new();
    for (_, entity, brushes_opt) in pending {
        let own = brushes_opt.cloned().unwrap_or_default();
        let lookup = |e: Entity| {
            resolved_this_run
                .get(&e)
                .cloned()
                .or_else(|| all_brushes.get(e).ok().cloned())
        };
        let resolved = Brushes {
            foreground: resolve_field(&own.foreground, entity, parent_of, &lookup, |b| {
                &b.foreground
            })
            .unwrap_or(DEFAULT_FOREGROUND),
            background: resolve_field(&own.background, entity, parent_of, &lookup, |b| {
                &b.background
            })
            .unwrap_or(DEFAULT_BACKGROUND),
        };

        // Brushesを挿入/更新し、BrushInheritマーカーを除去
        commands
            .entity(entity)
            .insert(resolved.clone())
            .remove::<BrushInherit>();
        resolved_this_run.insert(entity, resolved);

        trace!(
            entity = ?entity,
            "[resolve_inherited_brushes] Brushes resolved"
        );
    }
}

/// 1 つの色を解決する。自分で決まっていればそれ、Inherit なら決まった色を持つ最も近い祖先の色。
fn resolve_field(
    own: &Brush,
    entity: Entity,
    parent_of: impl FnMut(Entity) -> Option<Entity>,
    lookup: &impl Fn(Entity) -> Option<Brushes>,
    field: impl Fn(&Brushes) -> &Brush,
) -> Option<Brush> {
    if !own.is_inherit() {
        return Some(own.clone());
    }
    ancestors(entity, parent_of).find_map(|a| {
        lookup(a)
            .map(|b| field(&b).clone())
            .filter(|b| !b.is_inherit())
    })
}

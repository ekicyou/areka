//! 階層配置伝播システム（Arrangement → GlobalArrangement）

use crate::ecs::common::tree_system::{
    NodeQuery, WorkQueue, mark_dirty_trees, propagate_parent_transforms, sync_simple_transforms,
};
use bevy_ecs::hierarchy::{ChildOf, Children};
use bevy_ecs::prelude::*;
use tracing::trace;

use super::super::{Arrangement, ArrangementTreeChanged, GlobalArrangement};

/// 階層に属していないEntity（ルートWindow）のGlobalArrangementを更新
pub fn sync_simple_arrangements(
    query: ParamSet<(
        Query<
            (&Arrangement, &mut GlobalArrangement),
            (
                Or<(Changed<Arrangement>, Added<GlobalArrangement>)>,
                Without<ChildOf>,
                Without<Children>,
            ),
        >,
        Query<(Ref<Arrangement>, &mut GlobalArrangement), (Without<ChildOf>, Without<Children>)>,
    )>,
    orphaned: RemovedComponents<ChildOf>,
) {
    sync_simple_transforms::<Arrangement, GlobalArrangement, ArrangementTreeChanged>(
        query, orphaned,
    );
}

/// 「ダーティビット」を階層の祖先に向かって伝播
pub fn mark_dirty_arrangement_trees(
    changed: Query<
        Entity,
        Or<(
            Changed<Arrangement>,
            Changed<ChildOf>,
            Added<GlobalArrangement>,
        )>,
    >,
    orphaned: RemovedComponents<ChildOf>,
    transforms: Query<(Option<&ChildOf>, &mut ArrangementTreeChanged)>,
) {
    let changed_count = changed.iter().count();
    if changed_count > 0 {
        // Note: 頻繁に呼ばれるためログは抑制
        // tracing::info!(
        //     changed_count,
        //     "[mark_dirty_arrangement_trees] Detected changed arrangements"
        // );
    }
    mark_dirty_trees::<Arrangement, GlobalArrangement, ArrangementTreeChanged>(
        changed, orphaned, transforms,
    );
}

/// 親から子へGlobalArrangementを伝播
pub fn propagate_global_arrangements(
    queue: Local<WorkQueue>,
    roots: Query<
        (Entity, Ref<Arrangement>, &mut GlobalArrangement, &Children),
        (Without<ChildOf>, Changed<ArrangementTreeChanged>),
    >,
    nodes: NodeQuery<Arrangement, GlobalArrangement, ArrangementTreeChanged>,
) {
    // デバッグログ: ルートエンティティの情報
    for (entity, arr, global, _) in roots.iter() {
        trace!(
            entity = ?entity,
            offset_x = arr.offset.x,
            offset_y = arr.offset.y,
            scale_x = arr.scale.x,
            scale_y = arr.scale.y,
            "[propagate_global_arrangements] Root Entity Arrangement"
        );
        trace!(
            entity = ?entity,
            m11 = global.transform.M11,
            m12 = global.transform.M12,
            m31 = global.transform.M31,
            m32 = global.transform.M32,
            bounds_left = global.bounds.left,
            bounds_top = global.bounds.top,
            bounds_right = global.bounds.right,
            bounds_bottom = global.bounds.bottom,
            "[propagate_global_arrangements] Root Entity GlobalArrangement"
        );
    }

    propagate_parent_transforms::<Arrangement, GlobalArrangement, ArrangementTreeChanged>(
        queue, roots, nodes,
    );
}

//! テストが、配られたドラッグの知らせを受け手の呼ばれた数で数えるための受け手と溜め。
//!
//! [`watch`] で entity へ 3 つの受け手を付け、配った後に [`take`] で読み出す。受け手は
//! 泡の段で 1 件ずつ控えるので、見張りは 1 つの経路に 1 か所だけ付ける（窓か、窓の無い対象）。

use bevy_ecs::prelude::*;

use super::{OnDrag, OnDragEnd, OnDragStart};
use crate::ecs::pointer::Phase;

#[derive(Resource)]
struct Seen<E: Send + Sync + 'static>(Vec<E>);

fn record<E: Clone + Send + Sync + 'static>(
    world: &mut World,
    _sender: Entity,
    _entity: Entity,
    ev: &Phase<E>,
) -> bool {
    if let Phase::Bubble(e) = ev {
        world
            .get_resource_or_insert_with(|| Seen::<E>(Vec::new()))
            .0
            .push(e.clone());
    }
    false
}

/// `entity` に、始まり・途中・終わりの知らせを控える受け手を付ける。
pub(crate) fn watch(world: &mut World, entity: Entity) {
    world
        .entity_mut(entity)
        .insert((OnDragStart(record), OnDrag(record), OnDragEnd(record)));
}

/// 控えた知らせ `E` を取り出す（取り出した分は消える）。
pub(crate) fn take<E: Clone + Send + Sync + 'static>(world: &mut World) -> Vec<E> {
    world
        .get_resource_mut::<Seen<E>>()
        .map(|mut seen| std::mem::take(&mut seen.0))
        .unwrap_or_default()
}

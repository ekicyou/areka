//! EntityRegistry — ActorKey + CueTarget から Entity を解決する統合レジストリ。
//!
//! 名前空間の統合:
//! - Actor(ActorKey, CueTarget): アクターの特定スロット
//! - Spot(String): 物理スポットエンティティ (P1 拡張)
//! - Balloon(String): 物理バルーンエンティティ (P1 拡張)

use std::collections::HashMap;

use bevy_ecs::entity::Entity;
use bevy_ecs::resource::Resource;

use super::command::EntityKey;
use super::{ActorKey, CueTarget};

/// ActorKey + CueTarget から Entity を解決する統合レジストリ。
/// O(1) 解決、型安全な名前空間統合。
#[derive(Resource, Default, Debug)]
pub struct EntityRegistry {
    map: HashMap<EntityKey, Entity>,
}

impl EntityRegistry {
    /// 新しいレジストリを生成する。
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }

    /// アクター登録（ショートカット）
    pub fn register_actor(
        &mut self,
        actor: impl Into<ActorKey>,
        target: CueTarget,
        entity: Entity,
    ) {
        self.map
            .insert(EntityKey::Actor(actor.into(), target), entity);
    }

    /// アクター解決（ショートカット）
    pub fn resolve_actor(&self, actor: &ActorKey, target: &CueTarget) -> Option<Entity> {
        self.map
            .get(&EntityKey::Actor(actor.clone(), target.clone()))
            .copied()
    }

    /// 指定アクターの全ルーティングスロットを返却
    pub fn routes_for_actor(&self, actor: &ActorKey) -> Vec<(CueTarget, Entity)> {
        self.map
            .iter()
            .filter_map(|(key, &entity)| {
                if let EntityKey::Actor(a, target) = key {
                    if a == actor {
                        return Some((target.clone(), entity));
                    }
                }
                None
            })
            .collect()
    }

    /// 汎用登録
    pub fn register(&mut self, key: EntityKey, entity: Entity) {
        self.map.insert(key, entity);
    }

    /// 汎用解決
    pub fn resolve(&self, key: &EntityKey) -> Option<Entity> {
        self.map.get(key).copied()
    }

    /// 登録数
    pub fn len(&self) -> usize {
        self.map.len()
    }

    /// 空判定
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }
}

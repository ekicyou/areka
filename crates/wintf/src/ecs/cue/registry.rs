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

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_ecs::world::World;

    fn spawn2() -> (Entity, Entity) {
        let mut world = World::new();
        (world.spawn_empty().id(), world.spawn_empty().id())
    }

    /// 同一キーへの再登録は後勝ち（last-write-wins）で len は増えない。
    #[test]
    fn register_actor_same_key_overwrites_last_wins() {
        let (e1, e2) = spawn2();
        let mut registry = EntityRegistry::new();

        registry.register_actor("sakura", CueTarget::Shell, e1);
        assert_eq!(registry.len(), 1);
        // 同一 (actor, target) を再登録 → 上書き
        registry.register_actor("sakura", CueTarget::Shell, e2);
        assert_eq!(registry.len(), 1, "same key must not grow the map");

        let key = ActorKey::from("sakura");
        assert_eq!(registry.resolve_actor(&key, &CueTarget::Shell), Some(e2));
    }

    /// 汎用 `register`/`resolve` の往復と後勝ち上書き。
    #[test]
    fn generic_register_resolve_roundtrip_and_overwrite() {
        let (e1, e2) = spawn2();
        let mut registry = EntityRegistry::new();

        let key = EntityKey::Spot("s0".into());
        assert_eq!(registry.resolve(&key), None);

        registry.register(key.clone(), e1);
        assert_eq!(registry.resolve(&key), Some(e1));

        // 後勝ち上書き
        registry.register(key.clone(), e2);
        assert_eq!(registry.resolve(&key), Some(e2));
        assert_eq!(registry.len(), 1);
    }

    /// 未登録アクターの `routes_for_actor` は空 Vec を返す。
    #[test]
    fn routes_for_unknown_actor_is_empty() {
        let (e1, _e2) = spawn2();
        let mut registry = EntityRegistry::new();
        registry.register_actor("sakura", CueTarget::Shell, e1);

        let routes = registry.routes_for_actor(&ActorKey::from("ghost"));
        assert!(routes.is_empty());
    }

    /// `routes_for_actor` は Spot/Balloon 名前空間のエントリを含めない（Actor のみ抽出）。
    #[test]
    fn routes_for_actor_excludes_non_actor_namespaces() {
        let mut world = World::new();
        let shell = world.spawn_empty().id();
        let spot = world.spawn_empty().id();
        let balloon = world.spawn_empty().id();

        let mut registry = EntityRegistry::new();
        registry.register_actor("sakura", CueTarget::Shell, shell);
        registry.register(EntityKey::Spot("sakura".into()), spot);
        registry.register(EntityKey::Balloon("sakura".into()), balloon);

        // routes_for_actor は Actor バリアントのみを返す（Spot/Balloon は除外）
        let routes = registry.routes_for_actor(&ActorKey::from("sakura"));
        assert_eq!(routes.len(), 1);
        assert_eq!(routes[0], (CueTarget::Shell, shell));
    }

    /// `new` と `default` は等価（空レジストリ）。
    #[test]
    fn new_and_default_are_both_empty() {
        let a = EntityRegistry::new();
        let b = EntityRegistry::default();
        assert!(a.is_empty());
        assert!(b.is_empty());
        assert_eq!(a.len(), b.len());
    }
}

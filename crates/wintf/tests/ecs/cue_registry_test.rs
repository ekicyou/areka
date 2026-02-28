//! ユニットテスト: EntityRegistry の検証（Task 4.7）
//!
//! - register_actor() / resolve_actor() の登録・解決
//! - routes_for_actor() の全スロット取得
//! - EntityKey 名前空間の分離
//! - 未登録キーの解決が None を返す

use bevy_ecs::world::World;
use wintf::ecs::cue::*;

fn make_entity() -> bevy_ecs::entity::Entity {
    let mut world = World::default();
    world.spawn_empty().id()
}

#[test]
fn register_and_resolve_actor() {
    let entity = make_entity();
    let mut registry = EntityRegistry::default();

    registry.register_actor("sakura", CueTarget::Shell, entity);

    let key = ActorKey::from("sakura");
    let resolved = registry.resolve_actor(&key, &CueTarget::Shell);
    assert_eq!(resolved, Some(entity));
}

#[test]
fn resolve_actor_returns_none_for_unregistered() {
    let registry = EntityRegistry::default();
    let key = ActorKey::from("sakura");
    assert_eq!(registry.resolve_actor(&key, &CueTarget::Shell), None);
}

#[test]
fn routes_for_actor_returns_all_slots() {
    let mut world = World::default();
    let shell_entity = world.spawn_empty().id();
    let balloon_entity = world.spawn_empty().id();

    let mut registry = EntityRegistry::new();
    registry.register_actor("sakura", CueTarget::Shell, shell_entity);
    registry.register_actor("sakura", CueTarget::Balloon, balloon_entity);

    let key = ActorKey::from("sakura");
    let routes = registry.routes_for_actor(&key);
    assert_eq!(routes.len(), 2);

    // Shell と Balloon の両方が存在
    let targets: Vec<&CueTarget> = routes.iter().map(|(t, _)| t).collect();
    assert!(targets.contains(&&CueTarget::Shell));
    assert!(targets.contains(&&CueTarget::Balloon));
}

#[test]
fn routes_for_actor_doesnt_return_other_actors() {
    let mut world = World::default();
    let e1 = world.spawn_empty().id();
    let e2 = world.spawn_empty().id();

    let mut registry = EntityRegistry::new();
    registry.register_actor("sakura", CueTarget::Shell, e1);
    registry.register_actor("unyu", CueTarget::Shell, e2);

    let sakura_key = ActorKey::from("sakura");
    let routes = registry.routes_for_actor(&sakura_key);
    assert_eq!(routes.len(), 1);
    assert_eq!(routes[0].1, e1);
}

#[test]
fn entity_key_namespace_separation() {
    let mut world = World::default();
    let actor_entity = world.spawn_empty().id();
    let spot_entity = world.spawn_empty().id();

    let mut registry = EntityRegistry::new();

    // Actor(sakura, Shell) と Spot("sakura") は別の名前空間
    registry.register(
        EntityKey::Actor(ActorKey::from("sakura"), CueTarget::Shell),
        actor_entity,
    );
    registry.register(
        EntityKey::Spot("sakura".into()),
        spot_entity,
    );

    let resolved_actor = registry.resolve(&EntityKey::Actor(
        ActorKey::from("sakura"),
        CueTarget::Shell,
    ));
    let resolved_spot = registry.resolve(&EntityKey::Spot("sakura".into()));

    assert_eq!(resolved_actor, Some(actor_entity));
    assert_eq!(resolved_spot, Some(spot_entity));
    assert_ne!(actor_entity, spot_entity);
}

#[test]
fn registry_len_and_is_empty() {
    let mut registry = EntityRegistry::new();
    assert!(registry.is_empty());
    assert_eq!(registry.len(), 0);

    let entity = make_entity();
    registry.register_actor("sakura", CueTarget::Shell, entity);
    assert!(!registry.is_empty());
    assert_eq!(registry.len(), 1);
}

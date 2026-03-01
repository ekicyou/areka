//! 統合テスト: CueSheet 配送 E2E の検証
//!
//! Task 4.8 — CueSheet → PendingCueSheet → dispatch → CueQueue 配送を E2E で検証する。

use bevy_ecs::prelude::*;
use bevy_ecs::system::SystemState;
use wintf::ecs::cue::dispatch::dispatch_cue_sheet_internal;
use wintf::ecs::cue::*;

/// テスト用 World を構築するヘルパー。
/// アクター "sakura" と "unyu" を Shell + Balloon の 2 スロットずつ登録する。
fn setup_world() -> (World, Entity, Entity, Entity, Entity) {
    let mut world = World::new();
    world.init_resource::<EntityRegistry>();

    let sakura_shell = world.spawn(CueQueue::new()).id();
    let sakura_balloon = world.spawn(CueQueue::new()).id();
    let unyu_shell = world.spawn(CueQueue::new()).id();
    let unyu_balloon = world.spawn(CueQueue::new()).id();

    {
        let mut registry = world.resource_mut::<EntityRegistry>();
        registry.register_actor("sakura", CueTarget::Shell, sakura_shell);
        registry.register_actor("sakura", CueTarget::Balloon, sakura_balloon);
        registry.register_actor("unyu", CueTarget::Shell, unyu_shell);
        registry.register_actor("unyu", CueTarget::Balloon, unyu_balloon);
    }

    (
        world,
        sakura_shell,
        sakura_balloon,
        unyu_shell,
        unyu_balloon,
    )
}

/// E2E: CueSheet を dispatch_cue_sheet_internal で配送し、CueQueue に届くことを検証
#[test]
fn dispatch_distributes_commands_to_correct_queues() {
    let (mut world, sakura_shell, sakura_balloon, _unyu_shell, _unyu_balloon) = setup_world();

    let sheet = CueSheet::new(vec![
        Cue {
            actor: ActorKey::from("sakura"),
            start_time: 0.0,
            payload: CueCommand::Text("hello".into()).into(),
        },
        Cue {
            actor: ActorKey::from("sakura"),
            start_time: 1.0,
            payload: CueCommand::Clear.into(),
        },
    ]);

    // dispatch
    let tracker_entity = world.spawn_empty().id();

    // Use manual dispatch via SystemState
    let handle = {
        let mut system_state: SystemState<(ResMut<EntityRegistry>, Query<&mut CueQueue>)> =
            SystemState::new(&mut world);
        let (mut registry, mut queue_query) = system_state.get_mut(&mut world);
        dispatch_cue_sheet_internal(
            &sheet,
            100.0,
            &mut registry,
            &mut queue_query,
            tracker_entity,
        )
    };

    // 配送先に sakura の Shell + Balloon が含まれる
    assert!(
        handle.targets.len() >= 2,
        "Expected at least 2 targets for sakura (Shell + Balloon), got {}",
        handle.targets.len()
    );

    // sakura_shell の CueQueue を検証
    {
        let queue = world.entity(sakura_shell).get::<CueQueue>().unwrap();
        assert_eq!(queue.len(), 2, "sakura_shell should have 2 commands");
    }

    // sakura_balloon の CueQueue を検証
    {
        let queue = world.entity(sakura_balloon).get::<CueQueue>().unwrap();
        assert_eq!(queue.len(), 2, "sakura_balloon should have 2 commands");
    }
}

/// E2E: 複数演者への配送 + 独立消費を検証
#[test]
fn dispatch_multiple_actors_independent_consumption() {
    let (mut world, sakura_shell, _sakura_balloon, unyu_shell, _unyu_balloon) = setup_world();

    let sheet = CueSheet::new(vec![
        Cue {
            actor: ActorKey::from("sakura"),
            start_time: 0.0,
            payload: CueCommand::Text("sakura says hi".into()).into(),
        },
        Cue {
            actor: ActorKey::from("unyu"),
            start_time: 0.5,
            payload: CueCommand::Text("unyu says hello".into()).into(),
        },
    ]);

    let tracker_entity = world.spawn_empty().id();

    {
        let mut system_state: SystemState<(ResMut<EntityRegistry>, Query<&mut CueQueue>)> =
            SystemState::new(&mut world);
        let (mut registry, mut queue_query) = system_state.get_mut(&mut world);
        dispatch_cue_sheet_internal(
            &sheet,
            10.0,
            &mut registry,
            &mut queue_query,
            tracker_entity,
        );
    }

    // sakura_shell: 1 command (Text at t=10.0)
    {
        let queue = world.entity(sakura_shell).get::<CueQueue>().unwrap();
        assert!(
            queue.len() >= 1,
            "sakura_shell should have at least 1 command"
        );
    }

    // unyu_shell: 1 command (Text at t=10.5)
    {
        let queue = world.entity(unyu_shell).get::<CueQueue>().unwrap();
        assert!(
            queue.len() >= 1,
            "unyu_shell should have at least 1 command"
        );
    }

    // 独立消費: sakura だけ消費しても unyu に影響しない
    {
        let mut queue = world.get_mut::<CueQueue>(sakura_shell).unwrap();
        let sakura_cmds = queue.pop_ready(10.0);
        assert_eq!(sakura_cmds.len(), 1);
    }
    {
        let queue = world.entity(unyu_shell).get::<CueQueue>().unwrap();
        assert_eq!(queue.len(), 1, "unyu_shell should still have 1 command");
    }
}

/// E2E: ルーティングコマンドは CueQueue に入らず EntityRegistry のみ更新
#[test]
fn routing_commands_update_registry_only() {
    let (mut world, sakura_shell, sakura_balloon, _unyu_shell, _unyu_balloon) = setup_world();

    // 新しい balloon entity を作成
    let new_balloon = world.spawn(CueQueue::new()).id();

    // EntityRegistry に Balloon("new_b") として登録
    {
        let mut registry = world.resource_mut::<EntityRegistry>();
        registry.register(EntityKey::Balloon("new_b".into()), new_balloon);
    }

    let sheet = CueSheet::new(vec![
        // ルーティングコマンド: sakura の Balloon スロットを切替
        Cue {
            actor: ActorKey::from("sakura"),
            start_time: 0.0,
            payload: RoutingCommand::RouteSwitch {
                target: CueTarget::Balloon,
                to: EntityKey::Balloon("new_b".into()),
            }
            .into(),
        },
        // 切替後にテキストを配送
        Cue {
            actor: ActorKey::from("sakura"),
            start_time: 0.1,
            payload: CueCommand::Text("after switch".into()).into(),
        },
    ]);

    let tracker_entity = world.spawn_empty().id();

    {
        let mut system_state: SystemState<(ResMut<EntityRegistry>, Query<&mut CueQueue>)> =
            SystemState::new(&mut world);
        let (mut registry, mut queue_query) = system_state.get_mut(&mut world);
        dispatch_cue_sheet_internal(&sheet, 0.0, &mut registry, &mut queue_query, tracker_entity);
    }

    // RouteSwitchはCueQueueに入らない — sakura_shell の queue にはルーティングコマンドなし
    {
        let queue = world.entity(sakura_shell).get::<CueQueue>().unwrap();
        // Shell には "after switch" Text が入る
        assert!(queue.len() >= 1);
    }

    // 旧 sakura_balloon には "after switch" が入らない（切替済み）
    {
        let queue = world.entity(sakura_balloon).get::<CueQueue>().unwrap();
        assert_eq!(
            queue.len(),
            0,
            "old balloon should have no commands after RouteSwitch"
        );
    }

    // 新 balloon には "after switch" が入る
    {
        let queue = world.entity(new_balloon).get::<CueQueue>().unwrap();
        assert_eq!(
            queue.len(),
            1,
            "new balloon should have 1 command after RouteSwitch"
        );
    }
}

/// E2E: ブロードキャスト — 非ルーティングコマンドが全スロットに配送される
#[test]
fn non_routing_commands_broadcast_to_all_slots() {
    let (mut world, sakura_shell, sakura_balloon, _unyu_shell, _unyu_balloon) = setup_world();

    let sheet = CueSheet::new(vec![Cue {
        actor: ActorKey::from("sakura"),
        start_time: 0.0,
        payload: CueCommand::Text("broadcast test".into()).into(),
    }]);

    let tracker_entity = world.spawn_empty().id();

    {
        let mut system_state: SystemState<(ResMut<EntityRegistry>, Query<&mut CueQueue>)> =
            SystemState::new(&mut world);
        let (mut registry, mut queue_query) = system_state.get_mut(&mut world);
        dispatch_cue_sheet_internal(&sheet, 0.0, &mut registry, &mut queue_query, tracker_entity);
    }

    // Shell と Balloon の両方にコマンドが配送される
    {
        let shell_queue = world.entity(sakura_shell).get::<CueQueue>().unwrap();
        assert_eq!(
            shell_queue.len(),
            1,
            "Shell should receive the broadcast command"
        );
    }
    {
        let balloon_queue = world.entity(sakura_balloon).get::<CueQueue>().unwrap();
        assert_eq!(
            balloon_queue.len(),
            1,
            "Balloon should receive the broadcast command"
        );
    }
}

/// E2E: 未登録 ActorKey の場合、他の演者は正常に配送される
#[test]
fn unregistered_actor_skipped_others_delivered() {
    let (mut world, sakura_shell, sakura_balloon, _unyu_shell, _unyu_balloon) = setup_world();

    let sheet = CueSheet::new(vec![
        // "ghost" は未登録
        Cue {
            actor: ActorKey::from("ghost"),
            start_time: 0.0,
            payload: CueCommand::Text("I am ghost".into()).into(),
        },
        Cue {
            actor: ActorKey::from("sakura"),
            start_time: 0.0,
            payload: CueCommand::Text("I am sakura".into()).into(),
        },
    ]);

    let tracker_entity = world.spawn_empty().id();

    {
        let mut system_state: SystemState<(ResMut<EntityRegistry>, Query<&mut CueQueue>)> =
            SystemState::new(&mut world);
        let (mut registry, mut queue_query) = system_state.get_mut(&mut world);
        dispatch_cue_sheet_internal(&sheet, 0.0, &mut registry, &mut queue_query, tracker_entity);
    }

    // sakura は正常に配送される
    {
        let queue = world.entity(sakura_shell).get::<CueQueue>().unwrap();
        assert_eq!(queue.len(), 1, "sakura_shell should have 1 command");
    }
    {
        let queue = world.entity(sakura_balloon).get::<CueQueue>().unwrap();
        assert_eq!(queue.len(), 1, "sakura_balloon should have 1 command");
    }
}

/// E2E: 絶対時刻変換が正しいことを検証
#[test]
fn dispatch_converts_relative_to_absolute_time() {
    let (mut world, sakura_shell, _sakura_balloon, _unyu_shell, _unyu_balloon) = setup_world();

    let sheet = CueSheet::new(vec![Cue {
        actor: ActorKey::from("sakura"),
        start_time: 0.5, // 相対 0.5 秒
        payload: CueCommand::Text("delayed".into()).into(),
    }]);

    let tracker_entity = world.spawn_empty().id();
    let sheet_start_time = 100.0;

    {
        let mut system_state: SystemState<(ResMut<EntityRegistry>, Query<&mut CueQueue>)> =
            SystemState::new(&mut world);
        let (mut registry, mut queue_query) = system_state.get_mut(&mut world);
        dispatch_cue_sheet_internal(
            &sheet,
            sheet_start_time,
            &mut registry,
            &mut queue_query,
            tracker_entity,
        );
    }

    // 100.0 では消費されない（100.5 が到達時刻）
    {
        let mut queue = world.get_mut::<CueQueue>(sakura_shell).unwrap();
        let cmds = queue.pop_ready(100.0);
        assert!(cmds.is_empty(), "Should not consume at t=100.0");
    }

    // 100.5 では消費される
    {
        let mut queue = world.get_mut::<CueQueue>(sakura_shell).unwrap();
        let cmds = queue.pop_ready(100.5);
        assert_eq!(cmds.len(), 1, "Should consume at t=100.5");
    }
}

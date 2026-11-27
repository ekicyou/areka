//! テストの World へ置き場のゴーストの seriko を据える道具（写しを作らない）。

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use areka_actor::ActorHandle;
use areka_seriko::{
    BindResolver, DefaultBinds, DisplayCommand, MockSurfaceOutput, SerikoLoopConfig, SerikoSink,
    SurfaceResolver, spawn_seriko,
};
use bevy_ecs::prelude::Resource;
use bevy_ecs::world::World;

use super::{NO_BODY, SessionHooks};

#[derive(Resource)]
struct TestSeriko(SerikoSink);

/// 定義の空な seriko を立て、本体の口の `seriko_sink` がその受け口を返す World。
pub(crate) struct SerikoWorld {
    pub world: World,
    sink: SerikoSink,
    actor: ActorHandle,
    records: Arc<Mutex<Vec<DisplayCommand>>>,
}

impl SerikoWorld {
    pub(crate) fn new() -> Self {
        let out = MockSurfaceOutput::new();
        let records = out.records();
        let (sink, actor) = spawn_seriko(
            SurfaceResolver::new(BTreeMap::new()),
            DefaultBinds::uniform(crate::shell_carry::test_support::set(&[])),
            BindResolver::default(),
            SerikoLoopConfig::disabled(),
            out,
        );
        let mut world = World::new();
        world.insert_resource(TestSeriko(sink.clone()));
        world.insert_resource(SessionHooks {
            seriko_sink: |w| w.get_resource::<TestSeriko>().map(|t| &t.0),
            ..NO_BODY
        });
        Self {
            world,
            sink,
            actor,
            records,
        }
    }

    /// seriko を閉じて止め、表示層へ出した指令を返す。
    pub(crate) fn finish(self) -> Vec<DisplayCommand> {
        drop(self.world);
        self.sink.close().expect("seriko の受け口が在る");
        self.actor.join().expect("seriko が止まる");
        std::mem::take(&mut *self.records.lock().expect("記録の鍵"))
    }
}

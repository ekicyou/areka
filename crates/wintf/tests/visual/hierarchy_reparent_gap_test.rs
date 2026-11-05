//! visual_hierarchy_sync_system の付け替えと「親が後から見た目を持つ」のテスト
//!
//! 未同期の見分けは「控えの親が、いま居るべき親と違う」。付け替えた子は古い親の WUC の
//! 子から外れて新しい親の子に入り、先に根の印を付けられた子も、親が後から見た目を持てば
//! つながる。WUC の親は `Visual::Parent()` で読み、同じ物かは `IUnknown` で比べる。

use std::sync::mpsc;
use std::time::Duration;

use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::prelude::*;
use windows::UI::Composition::{ContainerVisual, Visual as WucVisual};
use windows::core::{IUnknown, Interface, Result};
use wintf::ecs::{Visual, VisualGraphics, visual_hierarchy_sync_system};

use super::common::{WucVisualFactory, add_child, setup_graphics};

fn child_count(parent: &WucVisual) -> Result<i32> {
    parent.cast::<ContainerVisual>()?.Children()?.Count()
}

fn same(a: &impl Interface, b: &impl Interface) -> bool {
    match (a.cast::<IUnknown>(), b.cast::<IUnknown>()) {
        (Ok(a), Ok(b)) => a.as_raw() == b.as_raw(),
        _ => false,
    }
}

/// `child` の WUC の親が `parent` か。
fn is_wuc_child_of(child: &WucVisual, parent: &WucVisual) -> bool {
    child.Parent().map(|p| same(&p, parent)).unwrap_or(false)
}

fn cached_parent(world: &World, entity: Entity) -> Option<WucVisual> {
    world
        .get::<VisualGraphics>(entity)
        .unwrap()
        .parent_visual()
        .cloned()
}

fn run_sync(world: &mut World) {
    let mut schedule = Schedule::default();
    schedule.add_systems(visual_hierarchy_sync_system);
    schedule.run(world);
}

/// 付け替え（ChildOf A→B）: 子は B の WUC の子に入り、A から外れる
#[test]
fn reparent_moves_child_to_new_parent() -> Result<()> {
    crate::gpu::on_gpu_owner_thread(|| -> Result<()> {
        let graphics = setup_graphics()?;
        let factory = WucVisualFactory::new(&graphics)?;

        let mut world = World::new();
        world.insert_resource(graphics);

        let visual_a = factory.create_visual()?;
        let parent_a = world
            .spawn((Visual::default(), VisualGraphics::new(visual_a.clone())))
            .id();
        let visual_b = factory.create_visual()?;
        let parent_b = world
            .spawn((Visual::default(), VisualGraphics::new(visual_b.clone())))
            .id();

        // 子: A に同期済み（控え = A、WUC でも A の子）
        let visual_c = factory.create_visual()?;
        let mut child_vg = VisualGraphics::new(visual_c.clone());
        child_vg.set_parent_visual(Some(visual_a.clone()));
        add_child(&visual_a, &visual_c)?;
        let child = world
            .spawn((Visual::default(), child_vg, ChildOf(parent_a)))
            .id();

        world.entity_mut(child).insert(ChildOf(parent_b));
        run_sync(&mut world);

        assert!(
            !is_wuc_child_of(&visual_c, &visual_a),
            "古い親に残る: 付け替えた子が A の WUC の子のまま"
        );
        assert_eq!(child_count(&visual_a)?, 0, "古い親に残る: A の子の数");
        assert!(
            is_wuc_child_of(&visual_c, &visual_b),
            "付け替えた子が新しい親 B の WUC の子に入っていない"
        );
        let cached = cached_parent(&world, child).expect("控えが空");
        assert!(same(&cached, &visual_b), "古い親に残る: 控えが A のまま");
        Ok(())
    })
}

/// 付け替えの後に古い親が別の子で同期し直されても、移した子は新しい親に居る
#[test]
fn old_parent_resync_keeps_moved_child_in_new_parent() -> Result<()> {
    crate::gpu::on_gpu_owner_thread(|| -> Result<()> {
        let graphics = setup_graphics()?;
        let factory = WucVisualFactory::new(&graphics)?;

        let mut world = World::new();
        world.insert_resource(graphics);

        let visual_a = factory.create_visual()?;
        let parent_a = world
            .spawn((Visual::default(), VisualGraphics::new(visual_a.clone())))
            .id();
        let visual_b = factory.create_visual()?;
        let parent_b = world
            .spawn((Visual::default(), VisualGraphics::new(visual_b.clone())))
            .id();

        let visual_c = factory.create_visual()?;
        let mut child_vg = VisualGraphics::new(visual_c.clone());
        child_vg.set_parent_visual(Some(visual_a.clone()));
        add_child(&visual_a, &visual_c)?;
        let moved_child = world
            .spawn((Visual::default(), child_vg, ChildOf(parent_a)))
            .id();

        world.entity_mut(moved_child).insert(ChildOf(parent_b));

        // 新しい未同期の子 D を A の下に足す → A も同期し直される
        let visual_d = factory.create_visual()?;
        let new_child = world
            .spawn((
                Visual::default(),
                VisualGraphics::new(visual_d.clone()),
                ChildOf(parent_a),
            ))
            .id();

        run_sync(&mut world);

        assert!(is_wuc_child_of(&visual_d, &visual_a), "D が A の子にない");
        let d_cache = cached_parent(&world, new_child).expect("D の控えが空");
        assert!(same(&d_cache, &visual_a));
        assert_eq!(
            child_count(&visual_a)?,
            1,
            "古い親に残る: A の子は D だけのはず"
        );
        assert!(
            is_wuc_child_of(&visual_c, &visual_b),
            "移した子が新しい親 B の WUC の子に入っていない"
        );
        let c_cache = cached_parent(&world, moved_child).expect("C の控えが空");
        assert!(same(&c_cache, &visual_b), "古い親に残る: 控えが A のまま");
        Ok(())
    })
}

/// 子を先に作り（親は見た目を持たない＝根の印）、次の回に親が見た目を持てば子がつながる
#[test]
fn child_joins_when_parent_gets_visual_later() -> Result<()> {
    crate::gpu::on_gpu_owner_thread(|| -> Result<()> {
        let graphics = setup_graphics()?;
        let factory = WucVisualFactory::new(&graphics)?;

        let mut world = World::new();
        world.insert_resource(graphics);

        let parent = world.spawn_empty().id();
        let visual_c = factory.create_visual()?;
        let child = world
            .spawn((
                Visual::default(),
                VisualGraphics::new(visual_c.clone()),
                ChildOf(parent),
            ))
            .id();

        run_sync(&mut world);
        let root_mark = cached_parent(&world, child).expect("根の印が付いていない");
        assert!(
            same(&root_mark, &visual_c),
            "親が見た目を持たない間は根の印"
        );

        // 親が見た目の部品を持ったが、見た目はまだ作られていない → 保留（印は変えない）
        world.entity_mut(parent).insert(VisualGraphics::default());
        run_sync(&mut world);
        let pending = cached_parent(&world, child).expect("控えが空");
        assert!(same(&pending, &visual_c), "見た目が作られる前は保留");

        // 親の見た目ができた
        let visual_p = factory.create_visual()?;
        *world.get_mut::<VisualGraphics>(parent).unwrap() = VisualGraphics::new(visual_p.clone());
        run_sync(&mut world);

        assert!(
            is_wuc_child_of(&visual_c, &visual_p),
            "つながらない: 親が後から見た目を持っても子が WUC の子に入らない"
        );
        let cached = cached_parent(&world, child).expect("控えが空");
        assert!(same(&cached, &visual_p), "つながらない: 控えが根の印のまま");
        Ok(())
    })
}

/// 輪の親（A→B→A）を持つ World でも、木の同期の係は終わる
#[test]
fn hierarchy_sync_terminates_on_parent_cycle() {
    let (tx, rx) = mpsc::channel::<std::result::Result<(), String>>();
    std::thread::spawn(move || {
        let run = crate::gpu::on_gpu_owner_thread(|| -> Result<()> {
            let graphics = setup_graphics()?;
            let factory = WucVisualFactory::new(&graphics)?;
            let mut world = World::new();
            world.insert_resource(graphics);

            let a = world.spawn_empty().id();
            let b = world.spawn(ChildOf(a)).id();
            world.entity_mut(a).insert(ChildOf(b));
            let visual_c = factory.create_visual()?;
            world.spawn((Visual::default(), VisualGraphics::new(visual_c), ChildOf(a)));

            run_sync(&mut world);
            Ok(())
        });
        let _ = tx.send(run.map_err(|e| format!("{e:?}")));
    });
    match rx.recv_timeout(Duration::from_secs(30)) {
        Ok(Ok(())) => {}
        Ok(Err(e)) => panic!("準備に失敗: {e}"),
        Err(mpsc::RecvTimeoutError::Timeout) => {
            panic!("輪の親で木の同期の係が 30 秒で返らない")
        }
        Err(mpsc::RecvTimeoutError::Disconnected) => panic!("係のスレッドが落ちた"),
    }
}

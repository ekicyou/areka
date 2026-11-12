//! 描く骨組み: 失敗の後の空の列（10 番）・立ち直りの後の描き直し（実 GPU）

use super::*;
use crate::com::d2d::D2D1DeviceContextExt;
use crate::ecs::graphics::{GraphicsCore, invalidate_dependent_components};
use bevy_ecs::system::RunSystemOnce;
use windows::Win32::Foundation::E_FAIL;

/// 骨組みを試す部品。`fail` が真なら描く所が必ず失敗する。
#[derive(Component)]
struct Probe {
    fail: bool,
}

/// 係が部品を描こうとした回数。
#[derive(Resource, Default)]
struct Attempts(Vec<Entity>);

/// 本番の描く係と同じ形（変化か命令の列の無さで回し、骨組みを呼ぶ）の試しの係。
#[allow(clippy::type_complexity)]
fn draw_probes(
    mut commands: Commands,
    query: Query<
        (Entity, &Probe, Option<&GraphicsCommandList>, Option<&Name>),
        Or<(Changed<Probe>, Without<GraphicsCommandList>)>,
    >,
    graphics_core: Res<GraphicsCore>,
    mut attempts: ResMut<Attempts>,
) {
    let dc = graphics_core.device_context().expect("device context");
    for (entity, probe, list, name) in query.iter() {
        attempts.0.push(entity);
        let recorded = record_command_list(dc, |dc| {
            if probe.fail {
                return Err(E_FAIL.into());
            }
            let brush = dc.create_solid_color_brush(&TRANSPARENT, None)?;
            dc.fill_rectangle(
                &windows::Win32::Graphics::Direct2D::Common::D2D_RECT_F {
                    left: 0.0,
                    top: 0.0,
                    right: 4.0,
                    bottom: 4.0,
                },
                &brush,
            );
            Ok(())
        });
        store_command_list(
            &mut commands,
            entity,
            name,
            list.is_some(),
            recorded,
            "probe",
        );
    }
}

fn new_world() -> (World, Schedule) {
    let mut world = World::new();
    world.insert_resource(GraphicsCore::new().expect("GraphicsCore"));
    world.init_resource::<Attempts>();
    let mut schedule = Schedule::default();
    schedule.add_systems(draw_probes);
    (world, schedule)
}

fn list_of(world: &World, entity: Entity) -> Option<Option<ID2D1CommandList>> {
    world
        .get::<GraphicsCommandList>(entity)
        .map(|l| l.command_list().cloned())
}

fn attempts_of(world: &World, entity: Entity) -> usize {
    world
        .resource::<Attempts>()
        .0
        .iter()
        .filter(|e| **e == entity)
        .count()
}

/// 必ず失敗する記録で、列の無い部品には空の列が入って 2 回目の係で対象にならない。
/// 前から列を持つ部品は、失敗しても列が入れ替わらない（絵が残る）。
#[test]
fn failed_record_inserts_empty_list_only_without_one_and_is_not_retried() {
    let (mut world, mut schedule) = new_world();
    let fresh = world.spawn(Probe { fail: true }).id();
    let drawn = world.spawn(Probe { fail: false }).id();

    schedule.run(&mut world);
    assert_eq!(list_of(&world, fresh), Some(None), "空の列が入る");
    let picture = list_of(&world, drawn)
        .flatten()
        .expect("成功した部品には命令の列が入る");

    // 前から列を持つ部品を失敗させる
    world.get_mut::<Probe>(drawn).unwrap().fail = true;
    schedule.run(&mut world);
    assert_eq!(attempts_of(&world, fresh), 1, "空の列の部品はやり直さない");
    assert_eq!(attempts_of(&world, drawn), 2);
    assert_eq!(
        list_of(&world, drawn).flatten(),
        Some(picture),
        "一時の失敗で前の絵を消さない"
    );

    schedule.run(&mut world);
    assert_eq!(attempts_of(&world, fresh), 1);
    assert_eq!(attempts_of(&world, drawn), 2);
}

/// 空の列が入った部品も、立ち直り（命令の列を外す係）の後に描き直される。
#[test]
fn empty_list_entity_is_redrawn_after_recovery() {
    let (mut world, mut schedule) = new_world();
    let entity = world.spawn(Probe { fail: true }).id();
    schedule.run(&mut world);
    assert_eq!(list_of(&world, entity), Some(None));

    // 失敗の原因が消えても、変化の印は立てない（立ち直りだけで描き直されることを見る）
    world
        .get_mut::<Probe>(entity)
        .unwrap()
        .bypass_change_detection()
        .fail = false;

    // 装置を失い、立ち直りの係が命令の列を外す
    world.resource_mut::<GraphicsCore>().invalidate();
    world
        .run_system_once(invalidate_dependent_components)
        .expect("invalidate_dependent_components");
    assert!(world.get::<GraphicsCommandList>(entity).is_none());

    // 作り直した装置で描く係を回す
    world.insert_resource(GraphicsCore::new().expect("GraphicsCore"));
    schedule.run(&mut world);
    assert_eq!(attempts_of(&world, entity), 2);
    assert!(
        list_of(&world, entity).flatten().is_some(),
        "立ち直りの後は描かれる"
    );
}

/// 描く所が失敗しても `EndDraw` を呼ぶので、同じ文脈で次の記録ができる。
#[test]
fn failed_draw_still_ends_so_next_record_succeeds() {
    let core = GraphicsCore::new().expect("GraphicsCore");
    let dc = core.device_context().expect("device context");
    let failed = record_command_list(dc, |_| Err(E_FAIL.into()));
    assert_eq!(failed.unwrap_err().code(), E_FAIL, "描く所の失敗を先に返す");
    record_command_list(dc, |_| Ok(())).expect("次の記録は通る");
}

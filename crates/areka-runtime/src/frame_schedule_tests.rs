// =============================================================================
// 毎フレームの相（`frame_system`）の**登録先**の決定論テスト
//
// 登録先は 1 巡の末尾の段（`FrameFinalize`）ではなく上流の `Update` である。相がそこで挿した
// 描画命令と配置を、同じ巡の伝播（`PostLayout`）・面の生成（`PreRenderSurface`）・描画
// （`RenderSurface`）が拾う——末尾の段に載せると絵の着地が次の巡へずれ、文字だけが 1 コマ
// 先に出る。
//
// この登録先は挙動の檻に映らない。相の本体は `RuntimeWiring` が挿さっていない World では丸ごと
// 無操作であり、実窓を持たない檻から見ると「絵が 1 コマ遅れる」は原理的に観測できない。
// ゆえに本番の登録の関数 [`register_frame_system`](crate::register_frame_system) を空の World の
// `Schedules` へ直に呼び、当の相が `Update` に載り末尾の段には載らないという**構造**
// （GPU も窓も要らない）を見る。対照として、登録の前は `Update` に無い。
// 登録の字面と段の並びは `zorder_wiring_tests.rs` の `t_zwi06` が受け持つ。
// =============================================================================

use bevy_ecs::prelude::*;
use bevy_ecs::schedule::{IntoSystemSet, ScheduleLabel, Schedules};
use wintf::ecs::{FrameFinalize, Update};

use super::frame::frame_system;
use areka_placement::spawn::wire_zorder_pair;

// ---------------------------------------------------------------- 道具立て

/// 指定の段に当の相が載っているか。
///
/// 判定は system の**型の集合**（bevy が関数 system ごとに自動で作る集合）が段のグラフに
/// 在るかどうかで行う。表示名の文字列は `bevy_ecs` の `debug` 機能が消えると
/// 「機能が無効」の定型文に化けるため、檻の判定材料にはしない。
fn schedule_holds_the_frame_system(world: &World, label: impl ScheduleLabel) -> bool {
    world
        .get_resource::<Schedules>()
        .and_then(|schedules| schedules.get(label))
        .is_some_and(|schedule| {
            schedule
                .graph()
                .system_sets
                .contains(frame_system.into_system_set())
        })
}

/// 鎖の適用系（末尾の段の 4 本）を**先に**載せた World。
///
/// 本番の登録の入口（`ghost_session::register_systems`）も相の登録と `wire_zorder_pair` の両方を載せる。
/// 末尾の段を空のままにすると「載らない」の主張が「段そのものが無い」で通ってしまうので、
/// ここで実際に 4 本を載せておく。
fn world_with_the_chain_apply_wired() -> World {
    let mut world = World::new();
    world.init_resource::<Schedules>();
    wire_zorder_pair(&mut world);
    world
}

// ---------------------------------------------------------------------------
// 構造——本番の登録は `Update` に載り、末尾の段には載らない
// ---------------------------------------------------------------------------

/// 本番の登録の関数を呼ぶと、当の相は `Update` に載り、鎖の適用系が居る末尾の段には載らない。
///
/// 末尾の段は空ではない（適用系ほか 4 本が先に載っている）ので、「載らない」の主張は
/// 「段そのものが無い」で成り立っているのではない。
#[test]
fn t_n10_the_registration_lands_the_frame_system_in_update_only() {
    let mut world = world_with_the_chain_apply_wired();

    crate::register_frame_system(&mut world);

    assert!(
        schedule_holds_the_frame_system(&world, Update),
        "本番の登録をしたのに相が `Update` に載っていない"
    );
    assert!(
        !schedule_holds_the_frame_system(&world, FrameFinalize),
        "相が 1 巡の末尾の段にも載っている（絵の着地が次の巡へずれる経路が残っている）"
    );

    let schedules = world.resource::<Schedules>();
    assert_eq!(
        schedules
            .get(Update)
            .expect("登録後は `Update` の段が在る")
            .systems_len(),
        1,
        "`Update` へ載るのは当の相ちょうど 1 本（順序指定の相手まで載ると相順の前提が変わる）"
    );
    assert_eq!(
        schedules
            .get(FrameFinalize)
            .expect("末尾の段は結線済み（確立系・帯の系・ペア維持系・鎖の適用系）")
            .systems_len(),
        4,
        "末尾の段が空になっている＝「相が載らない」の主張が空虚（対照が成立していない）"
    );
}

/// 対照——登録の前は `Update` に何も無い。
///
/// これが無いと上の檻は「何をしても載っていると言う」形と区別が付かない。
#[test]
fn t_n10_before_the_registration_the_update_stage_holds_nothing() {
    let world = world_with_the_chain_apply_wired();

    assert!(
        !schedule_holds_the_frame_system(&world, Update),
        "登録の前から相が `Update` に載っている（判定が恒真）"
    );
    assert!(
        !world.resource::<Schedules>().contains(Update),
        "前提: 鎖の結線は `Update` の段を作らない（この World は wintf の既定システムを持たない）"
    );
}

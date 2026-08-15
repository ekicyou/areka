use super::*;
use super::test_support::{headless_emo2_wiring, runtime_with_active_choice, spawn_balloon_leave_child};
use bevy_ecs::world::World;
use std::sync::mpsc;

fn sample() -> ChoiceSelection {
    ChoiceSelection {
        id: "q0".to_string(),
        label: "はい".to_string(),
        scope: 0,
        references: vec!["ref0".to_string(), "ref1".to_string()],
    }
}

#[test]
fn identical_field_contents_compare_equal() {
    let a = sample();
    let b = sample();
    assert_eq!(a, b, "同一フィールド内容の ChoiceSelection は等価であるべき");
}

#[test]
fn differing_field_contents_compare_unequal() {
    let base = sample();

    let mut different_id = sample();
    different_id.id = "q1".to_string();
    assert_ne!(base, different_id, "id が異なれば非等価であるべき");

    let mut different_refs = sample();
    different_refs.references = vec!["ref0".to_string()];
    assert_ne!(base, different_refs, "references が異なれば非等価であるべき");
}

#[test]
fn clone_equals_original_and_debug_is_usable() {
    let original = sample();
    let cloned = original.clone();
    assert_eq!(original, cloned, "clone は元と等価であるべき（Clone 導出の証跡）");

    let rendered = format!("{original:?}");
    assert!(!rendered.is_empty(), "Debug 出力は非空であるべき（Debug 導出の証跡）");
}

/// NonSend 挿入＋シーム観測檻（2.2・design「NonSend 資源」）: `BalloonWiring` を `World` へ
/// **NonSend 挿入**でき、`ChoiceSelectionInbox` の `Receiver` 経由で発行シンクへ送った
/// `ChoiceSelection` を一度だけ観測できる（送信値と等価・2 度目は `Empty`）。
///
/// mpsc `Sender`/`Receiver` は `!Sync` ゆえ NonSend 資源として挿入する（`insert_non_send_resource`）。
/// 受信処理は M1 未消費（下流 W6 `choice-select-events` が置換する seam・5.3）。ここでは
/// 発行が mpsc 上で観測できることのみを固定し、`resolve_choice` は一切呼ばない（5.4）。
#[test]
fn wiring_inserts_non_send_and_selection_observed_via_inbox() {
    let (tx, rx) = mpsc::channel::<ChoiceSelection>();
    let wiring = BalloonWiring::new(tx);

    let mut world = World::new();
    world.insert_non_send_resource(wiring);
    world.insert_non_send_resource(ChoiceSelectionInbox(rx));

    assert!(
        world.get_non_send_resource::<BalloonWiring>().is_some(),
        "BalloonWiring は NonSend 挿入されている"
    );

    // 発行シンク経由で ChoiceSelection を送る。
    let sel = sample();
    let sent = world
        .get_non_send_resource::<BalloonWiring>()
        .expect("直上で存在確認済み")
        .send_selection(sel.clone());
    assert!(sent, "Receiver 生存中の発行は成功する（Err にならない・5.3）");

    // seam の Receiver 経由で送信値を観測する。
    let inbox = world
        .get_non_send_resource::<ChoiceSelectionInbox>()
        .expect("ChoiceSelectionInbox は NonSend 挿入されている");
    let received = inbox.0.try_recv().expect("発行した ChoiceSelection が届く");
    assert_eq!(received, sel, "受信値は送信値と等価（task 2.1 の PartialEq 再利用）");
    assert!(
        inbox.0.try_recv().is_err(),
        "発行は一度きり（2 度目の try_recv は Empty・2.4）"
    );
}

/// hover 自前追跡（B-2・R3.4）: `set_hover`/`hover` で scope 別の last-injected ordinal を
/// 記録・回収でき、消滅時整合の `None` 上書きが反映される。
#[test]
fn hover_tracks_last_injected_ordinal_per_scope() {
    let (tx, _rx) = mpsc::channel::<ChoiceSelection>();
    let mut wiring = BalloonWiring::new(tx);

    assert_eq!(wiring.hover(0), None, "未注入 scope の hover は None");

    wiring.set_hover(0, Some(2));
    wiring.set_hover(1, Some(5));
    assert_eq!(wiring.hover(0), Some(2), "scope 0 の last-injected を回収");
    assert_eq!(wiring.hover(1), Some(5), "scope 1 は独立に保持");

    // 消滅時整合（R3.4）: None 上書きで自前状態を整える。
    wiring.set_hover(0, None);
    assert_eq!(wiring.hover(0), None, "None 上書きが反映される");
    assert_eq!(wiring.hover(1), Some(5), "他 scope は影響を受けない");
}

// -------------------------------------------------------------------------
// post-spawn 装着・NonSend 結線・スケジュール登録檻（task 6.1・design
// 「attach_balloon_pointer_handlers / wire_balloon_choice」＋Integration Test 7・
// R4.3/4.4/5.5/6.6）
//
// (1) 配線存在檻（6.6/4.3）: bare World → spawn_ghost_windows → attach_char（キャラ窓 baseline）
//     → attach_balloon_pointer_handlers で、全バルーン窓に OnPointerMoved＋OnPointerPressed が
//     装着され、キャラ窓のハンドラ集合は不変であること（非退行・donor attach 檻同型）。
// (2) NonSend 結線檻: wire_balloon_choice 後に BalloonWiring＋ChoiceSelectionInbox が NonSend
//     資源として存在すること（donor wire_mouse_input 同型）。
// (3) スケジュール登録檻（Integration Test 7）: wire_balloon_choice が clear_balloon_hover_on_leave を
//     Input スケジュールへ登録すること——構造（systems_len）と行動（Input 実行で hover 解除）の双方で
//     固定する。登録漏れは高速離脱時の hover 残置として実機目視でしか検出できないため（1.3, 6.6）。
// -------------------------------------------------------------------------

use bevy_ecs::schedule::Schedules;
use wintf::ecs::Input;
use wintf::ecs::pointer::{OnPointerMoved, OnPointerPressed};

use crate::input_events::attach_char_pointer_handlers;
use crate::placement::resolver::{Anchor, PointPx, ScopePlacement, SizePx};
use crate::placement::source::GhostTitles;
use crate::placement::spawn::{CharWindowMarker, spawn_ghost_windows};

/// emo2 相当 2 スコープぶんの解決済み配置（spawn.rs 檻の two_scope_placements と同値）。
fn two_scopes() -> Vec<ScopePlacement> {
    vec![
        ScopePlacement {
            scope: 0,
            char_pos: PointPx { x: 1483, y: 733 },
            char_size: SizePx { w: 434, h: 687 },
            balloon_pos: PointPx { x: 1071, y: 708 },
            balloon_size: SizePx { w: 223, h: 158 },
            balloon_offset: PointPx { x: -412, y: -25 },
            // windowposition-limit: 正典既定（有効）。本檻は limit の判定を対象にしない。
            balloon_limit: true,
            anchor: Anchor::Bottom,
            balloon_keyword_base: None,
        },
        ScopePlacement {
            scope: 1,
            char_pos: PointPx { x: 1049, y: 1063 },
            char_size: SizePx { w: 278, h: 357 },
            balloon_pos: PointPx { x: 1334, y: 1044 },
            balloon_size: SizePx { w: 223, h: 158 },
            balloon_offset: PointPx { x: 285, y: -19 },
            // windowposition-limit: 正典既定（有効）。本檻は limit の判定を対象にしない。
            balloon_limit: true,
            anchor: Anchor::Bottom,
            balloon_keyword_base: None,
        },
    ]
}

fn ghost_titles() -> GhostTitles {
    GhostTitles::from_scope_titles([(0, "a".to_string()), (1, "b".to_string())])
}

fn balloon_window_entities(world: &mut World) -> Vec<Entity> {
    world
        .query_filtered::<Entity, With<BalloonWindowMarker>>()
        .iter(world)
        .collect()
}

fn char_window_entities(world: &mut World) -> Vec<Entity> {
    world
        .query_filtered::<Entity, With<CharWindowMarker>>()
        .iter(world)
        .collect()
}

fn has_pointer_handlers(world: &World, e: Entity) -> bool {
    world.get::<OnPointerMoved>(e).is_some() && world.get::<OnPointerPressed>(e).is_some()
}

/// 配線存在檻（6.6/4.3）: 全バルーン窓へハンドラ装着・キャラ窓集合不変（非退行）。
#[test]
fn attach_installs_handlers_on_all_balloon_windows_and_leaves_char_unchanged() {
    let mut world = World::new();
    spawn_ghost_windows(&mut world, &two_scopes(), &ghost_titles());
    // キャラ窓へ donor ハンドラを装着（R4.3 非退行の対象 baseline を非自明にする）。
    attach_char_pointer_handlers(&mut world);

    let balloons = balloon_window_entities(&mut world);
    let chars = char_window_entities(&mut world);
    assert_eq!(balloons.len(), 2, "2 スコープぶんのバルーン窓が spawn される");
    assert_eq!(chars.len(), 2, "2 スコープぶんのキャラ窓が spawn される");

    // 装着前: バルーン窓はハンドラ未装着（spawn.rs はバルーンに付けない・DD-IE-12）。
    for &e in &balloons {
        assert!(
            !has_pointer_handlers(&world, e),
            "attach 前のバルーン窓はポインタハンドラ未装着"
        );
    }
    // キャラ窓は donor で装着済み（baseline）。
    for &e in &chars {
        assert!(
            has_pointer_handlers(&world, e),
            "キャラ窓は donor（attach_char）でハンドラ装着済み（baseline）"
        );
    }

    attach_balloon_pointer_handlers(&mut world);

    // 装着後: 全バルーン窓に OnPointerMoved＋OnPointerPressed。
    for &e in &balloons {
        assert!(
            has_pointer_handlers(&world, e),
            "attach 後は全バルーン窓に OnPointerMoved＋OnPointerPressed が装着される（6.6）"
        );
    }
    // キャラ窓のハンドラ集合は不変（非退行・R4.3）。
    for &e in &chars {
        assert!(
            has_pointer_handlers(&world, e),
            "キャラ窓のハンドラ集合は attach_balloon で不変（非退行・R4.3）"
        );
    }
}

/// NonSend 結線檻（R5.5）: wire_balloon_choice が BalloonWiring＋ChoiceSelectionInbox を NonSend 挿入。
#[test]
fn wire_inserts_both_non_send_resources() {
    let mut world = World::new();
    world.init_resource::<Schedules>(); // wire は schedule 登録も行うため Schedules 既在が前提。

    assert!(
        world.get_non_send_resource::<BalloonWiring>().is_none(),
        "挿入前は BalloonWiring 不在"
    );
    assert!(
        world.get_non_send_resource::<ChoiceSelectionInbox>().is_none(),
        "挿入前は ChoiceSelectionInbox 不在"
    );

    wire_balloon_choice(&mut world);

    assert!(
        world.get_non_send_resource::<BalloonWiring>().is_some(),
        "wire 後は BalloonWiring が NonSend 挿入されている"
    );
    assert!(
        world.get_non_send_resource::<ChoiceSelectionInbox>().is_some(),
        "wire 後は ChoiceSelectionInbox が NonSend 挿入されている（発行 seam・5.3）"
    );
}

/// スケジュール登録檻・構造（Integration Test 7・R6.6）: wire が clear_balloon_hover_on_leave を
/// Input スケジュールへ 1 件登録する。
#[test]
fn wire_registers_leave_system_into_input_schedule() {
    let mut world = World::new();
    world.init_resource::<Schedules>();

    wire_balloon_choice(&mut world);

    let schedules = world.resource::<Schedules>();
    assert!(schedules.contains(Input), "Input スケジュールが存在する");
    let input = schedules.get(Input).expect("Input schedule は存在する");
    assert_eq!(
        input.systems_len(),
        1,
        "Input スケジュールに 1 システム（clear_balloon_hover_on_leave）が登録される（6.6）"
    );
}

/// スケジュール登録檻・行動（Integration Test 7・R1.3/6.6）: 登録済みシステムが Input 実行で走り、
/// バルーン所有 leave の hover を解除する（登録が「clear_balloon_hover_on_leave」であることの行動的証明・
/// debug feature 非依存）。
#[test]
fn registered_leave_system_runs_in_input_schedule_and_clears_balloon_hover() {
    let mut world = World::new();
    world.init_resource::<Schedules>();

    // バルーン所有 leave（scope 0）＋表示中 choice の実 runtime。
    spawn_balloon_leave_child(&mut world, 0);
    let runtime = runtime_with_active_choice("0");
    world.insert_non_send_resource(headless_emo2_wiring(Rc::clone(&runtime)));

    // wire で BalloonWiring／Inbox 挿入＋leave system 登録。
    wire_balloon_choice(&mut world);
    // wire が挿入した BalloonWiring に前回注入値 Some(2) を仕込む（解除対象）。
    world
        .get_non_send_resource_mut::<BalloonWiring>()
        .expect("wire で BalloonWiring 挿入済み")
        .set_hover(0, Some(2));
    assert_eq!(
        world.get_non_send_resource::<BalloonWiring>().unwrap().hover(0),
        Some(2),
        "前提: hover[0]=Some(2)"
    );

    // Input スケジュールを実行 → 登録済み clear_balloon_hover_on_leave が走り hover を解除する。
    world.run_schedule(Input);

    assert_eq!(
        world.get_non_send_resource::<BalloonWiring>().unwrap().hover(0),
        None,
        "Input 実行で登録済み clear_balloon_hover_on_leave が走り hover を解除する（登録の行動的証明）"
    );
}

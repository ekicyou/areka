//! 手動の送りの公開の口の檻（design.md「公開の口（`actor_scroll_tests.rs`）」・要件 8.1〜8.3・8.5・8.8）。
//!
//! 消去・隠した・面の切り替え・箱を外したで手動が捨てられること、場所ごとに別々なこと、
//! 手動の位置で選択肢の当たりの写しが戻した分だけずれること（`committed` を通る）を固定する。
//! 「手動か」は私有の表を `marker_visibility(..).down`（＝手動の位置が在る）で読む。

use areka_sakura::contract::{ActorKey, CueCommand, TalkCue};
use bevy_ecs::entity::Entity;
use bevy_ecs::prelude::World;

use super::box_sync_tests::{fixture, text};
use super::test_support::{choice_cue, com_world, cue, geo_model};
use super::{ResolvedBalloonText, TextLayerRuntime, TextSlotBinding, present_frame};
use crate::layout::{FixedMetrics, VisibleWindow};
use crate::place::PlaceKey;
use crate::scroll_marker::{
    PlaceScroll, ScrollChange, ScrollDir, StopReason, WheelLines, marker_visibility, window_for,
};
use crate::state::TextLayerConfig;

/// 小さいバルーン（既定の 12px の字で 2〜3 行しか入らない）。
const IMAGE: (u32, u32) = (120, 40);
/// 行の数（あふれて古い行が押し出される）。
const LINES: usize = 8;
/// 全部の字が見えている時刻。
const DONE: f64 = 100.0;

/// 1 行に 1 つずつ選択肢を置き、改行で区切った台本（`LINES` 行）。
fn choice_lines(actor: &str) -> Vec<TalkCue> {
    let mut cues = Vec::new();
    for i in 0..LINES {
        let at = i as f64;
        cues.push(choice_cue(actor, at, &format!("On{i}"), "はい", &[]));
        if i + 1 < LINES {
            cues.push(cue(actor, at + 0.5, CueCommand::NewLine { ratio: 1.0 }));
        }
    }
    cues
}

/// 装着してあふれる台本を流し、1 フレーム提示した実行時。
fn presented(world: &mut World, window: Entity, slot: Entity, actors: &[&str]) -> TextLayerRuntime {
    let mut rt = TextLayerRuntime::new(TextLayerConfig::default());
    for actor in actors {
        rt.register_actor(
            ActorKey::from(*actor),
            TextSlotBinding::new(slot, window, 1.0, IMAGE, IMAGE),
            ResolvedBalloonText::resolve(&geo_model(), IMAGE),
        );
        for c in choice_lines(actor) {
            rt.apply_cue(&c);
        }
    }
    present_frame(&mut rt, world, DONE).expect("提示フレーム");
    rt
}

/// 場所が手動の位置にいるか。
fn is_manual(rt: &TextLayerRuntime, place: &PlaceKey) -> bool {
    let any = VisibleWindow {
        first_visible_line: 0,
        block_offset: 0.0,
    };
    rt.scrolls
        .get(place)
        .is_some_and(|s| marker_visibility(&any, s).down)
}

/// 手動の位置にいる状態（GPU なし: 決まった字幅で本番と同じ配置の手順を通す）。
fn manual_scroll() -> PlaceScroll {
    let mut rt = TextLayerRuntime::new(TextLayerConfig::default());
    let actor = ActorKey::from("9");
    rt.register_actor(
        actor.clone(),
        TextSlotBinding::new(Entity::PLACEHOLDER, Entity::PLACEHOLDER, 1.0, IMAGE, IMAGE),
        ResolvedBalloonText::resolve(&geo_model(), IMAGE),
    );
    for c in choice_lines("9") {
        rt.apply_cue(&c);
    }
    let place = PlaceKey::balloon(&actor);
    let resolved = rt.layout_input[&place].clone();
    let lines = rt
        .arrange_for_test(&place, &FixedMetrics, DONE)
        .expect("行の列");
    let mut s = PlaceScroll::default();
    window_for(&lines, &resolved.region, resolved.mode, &mut s);
    assert_eq!(s.step(ScrollDir::Older, 2), ScrollChange::Started);
    s
}

#[test]
fn nothing_presented_cannot_scroll() {
    let mut rt = TextLayerRuntime::new(TextLayerConfig::default());
    let place = PlaceKey::balloon(&ActorKey::from("0"));
    assert!(!rt.has_hidden_lines_at(&place));
    assert_eq!(
        rt.scroll_manual_at(&place, ScrollDir::Older, 2),
        ScrollChange::Unchanged
    );
    assert_eq!(
        rt.wheel_at(&place, 120, WheelLines::Lines(3)),
        ScrollChange::Unchanged
    );
}

/// 手動の位置で選択肢の当たりの写しが、戻した 2 行の分だけ新しい側へずれる（要件 8.5）。
/// 新しい側へ戻すと自動の位置へ帰り、写しも元に戻る（要件 5.4）。
#[test]
fn manual_position_shifts_choice_hit_rows_by_the_lines_scrolled_back() {
    let (mut world, window, slot) = com_world();
    let mut rt = presented(&mut world, window, slot, &["0"]);
    let actor = ActorKey::from("0");
    let place = PlaceKey::balloon(&actor);
    assert!(
        rt.has_hidden_lines_at(&place),
        "あふれて古い行が押し出されている"
    );
    let before = rt.choice_hit_rows(&actor).to_vec();
    assert_eq!(before.len(), LINES);
    let pitch = before[1].rect.top - before[0].rect.top;
    assert!(pitch > 0.0);

    assert_eq!(
        rt.scroll_manual_at(&place, ScrollDir::Older, 2),
        ScrollChange::Started
    );
    present_frame(&mut rt, &mut world, DONE).expect("手動の位置の提示");
    let after = rt.choice_hit_rows(&actor).to_vec();
    for (b, a) in before.iter().zip(&after) {
        assert_eq!(a.ordinal, b.ordinal);
        let shift = a.rect.top - b.rect.top;
        assert!(
            (shift - 2.0 * pitch).abs() <= 1.0,
            "2 行戻した分だけ下へずれる: shift={shift} pitch={pitch}"
        );
    }

    assert_eq!(
        rt.scroll_manual_at(&place, ScrollDir::Newer, 2),
        ScrollChange::Stopped(StopReason::ReachedAuto)
    );
    present_frame(&mut rt, &mut world, DONE).expect("自動へ戻した提示");
    assert_eq!(rt.choice_hit_rows(&actor), before.as_slice());
}

/// 消去（`\c`・台詞の頭）で手動を捨てる（要件 8.2）。
#[test]
fn clear_and_clear_all_forget_manual() {
    let (mut world, window, slot) = com_world();
    let mut rt = presented(&mut world, window, slot, &["0", "1"]);
    let p0 = PlaceKey::balloon(&ActorKey::from("0"));
    let p1 = PlaceKey::balloon(&ActorKey::from("1"));
    rt.scroll_manual_at(&p0, ScrollDir::Older, 2);
    rt.scroll_manual_at(&p1, ScrollDir::Older, 2);
    assert!(is_manual(&rt, &p0) && is_manual(&rt, &p1));

    rt.apply_cue(&cue("0", DONE, CueCommand::Clear));
    assert!(!is_manual(&rt, &p0), "`\\c` は今の行き先の手動を捨てる");
    assert!(is_manual(&rt, &p1), "ほかのスコープの手動は残る");

    rt.apply_cue(&cue("0", DONE, CueCommand::ClearAll));
    assert!(!is_manual(&rt, &p1), "台詞の頭は全部の場所の手動を捨てる");
}

/// 見える字が増えたら、そのフレームで手動を捨てて自動の位置で描く（要件 8.1）。
#[test]
fn new_visible_text_forgets_manual_in_the_same_frame() {
    let (mut world, window, slot) = com_world();
    let mut rt = presented(&mut world, window, slot, &["0"]);
    let actor = ActorKey::from("0");
    let place = PlaceKey::balloon(&actor);
    rt.scroll_manual_at(&place, ScrollDir::Older, 2);
    present_frame(&mut rt, &mut world, DONE).expect("手動の位置の提示");
    assert!(is_manual(&rt, &place));

    rt.apply_cue(&cue("0", DONE + 1.0, CueCommand::Text("あ".into())));
    present_frame(&mut rt, &mut world, DONE + 10.0).expect("字が増えたフレーム");
    assert!(!is_manual(&rt, &place), "見える字が増えたので手動を捨てた");
}

/// 隠した（`Hidden`）は場所ごと、面の切り替え（`FaceChanged`）はアクターの全部の場所の手動を捨てる。
/// 別のアクターの場所は動かない（要件 8.3・8.6・8.8）。
#[test]
fn hidden_and_face_change_forget_per_place_and_per_actor() {
    let mut rt = TextLayerRuntime::new(TextLayerConfig::default());
    let a0 = ActorKey::from("0");
    let balloon0 = PlaceKey::balloon(&a0);
    let f = fixture();
    let box0 = f.key(0, "a");
    let balloon1 = PlaceKey::balloon(&ActorKey::from("1"));
    for place in [&balloon0, &box0, &balloon1] {
        rt.scrolls.insert(place.clone(), manual_scroll());
    }

    rt.forget_manual_at(&box0, StopReason::Hidden);
    assert!(!is_manual(&rt, &box0), "隠した箱の手動を捨てた");
    assert!(
        is_manual(&rt, &balloon0),
        "同じアクターの普通のバルーンは残る"
    );

    rt.scrolls.insert(box0.clone(), manual_scroll());
    rt.forget_manual_of(&a0, StopReason::FaceChanged);
    assert!(!is_manual(&rt, &balloon0) && !is_manual(&rt, &box0));
    assert!(is_manual(&rt, &balloon1), "別のアクターは残る");
}

/// 箱の登録を外すと、その箱の手動を捨てる（`BoxRemoved`）。
#[test]
fn unregistering_a_box_forgets_its_manual() {
    let mut f = fixture();
    f.show("0");
    f.rt.apply_cue(&text("0", "あ"));
    f.sync(Some(1.0));
    let key = f.key(0, "a");
    assert!(f.rt.box_sites.contains_key(&key));
    f.rt.scrolls.insert(key.clone(), manual_scroll());
    assert!(is_manual(&f.rt, &key));

    f.picture = None;
    f.sync(Some(1.0));
    assert!(!f.rt.box_sites.contains_key(&key));
    assert!(!is_manual(&f.rt, &key), "箱を外したので手動を捨てた");
}

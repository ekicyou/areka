//! 手動の位置にいる普通のバルーンの文字の層を実 GPU で作る足場。
//!
//! `frame_test_support.rs` が 1 ファイル 1,000 行の上限に近いので、兄弟に置く。

use areka_emo_text::state::TextLayerConfig;

use super::*;

/// 普通のバルーンの場所が手動の位置にいる文字の層。
///
/// 手動の位置の表は提示で作られるので、実 GPU（本番の UI スレッドと同じ MTA）で小さいバルーンに
/// あふれる行を流して 1 フレーム提示し、古い側へ 1 行送る。返す World は提示の資源を持つ。
/// GPU の装置の許可は組の先頭で返す（束縛は後ろから捨てられるので、許可は装置より後に返る）。
pub fn manual_balloon_runtime(
    actor: &str,
) -> (
    crate::spine::GpuPermit,
    World,
    Rc<RefCell<TextLayerRuntime>>,
) {
    use areka_emo_text::actor::{ResolvedBalloonText, TextSlotBinding};
    use areka_emo_text::place::PlaceKey;
    use areka_emo_text::scroll_marker::{ScrollChange, ScrollDir};
    use bevy_ecs::hierarchy::ChildOf;
    use dola::cue::{CueCommand, TalkCue};
    use dola::{TimePoint, TimeSpan};
    use wintf::ecs::Visual;

    // 途中で panic しても許可が装置より後に捨てられるよう、許可を先に束ねる。
    let (permit, mut world) = {
        let mut world = World::new();
        let permit = super::test_support::gpu_on(&mut world);
        (permit, world)
    };
    let window = world.spawn_empty().id();
    let slot = world.spawn((Visual::default(), ChildOf(window))).id();
    world.flush();

    let image = (120, 40);
    let key = ActorKey::from(actor);
    let mut rt = TextLayerRuntime::new(TextLayerConfig::default());
    rt.register_actor(
        key.clone(),
        TextSlotBinding::new(slot, window, 1.0, image, image),
        ResolvedBalloonText::resolve(&areka_parsers::balloon::parse_str("", None), image),
    );
    let cue = |at_ms: u32, command: CueCommand| {
        TalkCue::scheduled(
            TimePoint::ZERO,
            TimeSpan::from_millis(at_ms),
            key.clone(),
            command,
            TimeSpan::ZERO,
        )
    };
    for i in 0..8 {
        rt.apply_cue(&cue(i * 1_000, CueCommand::Text("あ".into())));
        rt.apply_cue(&cue(i * 1_000 + 500, CueCommand::NewLine { ratio: 1.0 }));
    }
    present_frame(&mut rt, &mut world, TimeSpan::from_millis(100_000)).expect("提示フレーム");
    assert_eq!(
        rt.scroll_manual_at(&PlaceKey::balloon(&key), ScrollDir::Older, 1),
        ScrollChange::Started,
        "前提: 古い行が押し出されていて手動の位置に入る"
    );
    (permit, world, Rc::new(RefCell::new(rt)))
}

/// 普通のバルーンの場所が手動の位置にいるか（新しい側へ 1 行送って、自動なら何も起きない）。
///
/// 手動なら 1 行送った結果で自動へ戻る（続けて 2 回呼ぶと 2 回目は偽）。手動へ入れ直すには
/// [`restart_manual`] を使う。
pub fn balloon_is_manual(rt: &RefCell<TextLayerRuntime>, actor: &str) -> bool {
    use areka_emo_text::place::PlaceKey;
    use areka_emo_text::scroll_marker::{ScrollChange, ScrollDir};
    rt.borrow_mut().scroll_manual_at(
        &PlaceKey::balloon(&ActorKey::from(actor)),
        ScrollDir::Newer,
        1,
    ) != ScrollChange::Unchanged
}

/// 普通のバルーンの場所を手動の位置へ入れ直す（[`balloon_is_manual`] が手動を自動へ戻すので、次の段の前に使う）。
pub fn restart_manual(rt: &RefCell<TextLayerRuntime>, actor: &str) {
    use areka_emo_text::place::PlaceKey;
    use areka_emo_text::scroll_marker::{ScrollChange, ScrollDir, StopReason};
    let mut rt = rt.borrow_mut();
    let place = PlaceKey::balloon(&ActorKey::from(actor));
    rt.forget_manual_at(&place, StopReason::Cleared);
    assert_eq!(
        rt.scroll_manual_at(&place, ScrollDir::Older, 1),
        ScrollChange::Started
    );
}

//! 手動の位置にいる普通のバルーンの文字の層を実 GPU で作る足場（balloon-scroll-arrows task 5.3）。
//!
//! `frame_test_support.rs` が 1 ファイル 1,000 行の上限に近いため、中身を変えずに分けた
//! （areka-test-binary-stability・task 1.2・R10.4）。

use areka_emo_text::state::TextLayerConfig;

use super::*;

/// 普通のバルーンの場所が手動の位置にいる文字の層（balloon-scroll-arrows task 5.3 の檻）。
///
/// 手動の位置の表は提示で作られるので、実 GPU（本番の UI スレッドと同じ MTA）で小さいバルーンに
/// あふれる行を流して 1 フレーム提示し、古い側へ 1 行送る。返す World は提示の資源を持つ。
pub(crate) fn manual_balloon_runtime(actor: &str) -> (World, Rc<RefCell<TextLayerRuntime>>) {
    use areka_emo_text::actor::{ResolvedBalloonText, TextSlotBinding};
    use areka_emo_text::place::PlaceKey;
    use areka_emo_text::scroll_marker::{ScrollChange, ScrollDir};
    use bevy_ecs::hierarchy::ChildOf;
    use dola::cue::{CueCommand, TalkCue};
    use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx};
    use wintf::ecs::{GraphicsCore, Visual, WucGraphicsResource};

    // SAFETY: このスレッドの COM を MTA で初期化するだけ（既に初期化済みなら失敗を捨てる）。
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
    }
    let core = GraphicsCore::new().expect("GraphicsCore");
    let wuc = WucGraphicsResource::new(core.d2d_device().expect("d2d_device"))
        .expect("WucGraphicsResource");
    let mut world = World::new();
    let window = world.spawn_empty().id();
    let slot = world.spawn((Visual::default(), ChildOf(window))).id();
    world.flush();
    world.insert_resource(core);
    world.insert_resource(wuc);

    let image = (120, 40);
    let key = ActorKey::from(actor);
    let mut rt = TextLayerRuntime::new(TextLayerConfig::default());
    rt.register_actor(
        key.clone(),
        TextSlotBinding::new(slot, window, 1.0, image, image),
        ResolvedBalloonText::resolve(&areka_parsers::balloon::parse_str("", None), image),
    );
    let cue = |at: f64, command: CueCommand| TalkCue {
        at,
        actor: key.clone(),
        command,
        duration: 0.0,
    };
    for i in 0..8 {
        rt.apply_cue(&cue(i as f64, CueCommand::Text("あ".into())));
        rt.apply_cue(&cue(i as f64 + 0.5, CueCommand::NewLine { ratio: 1.0 }));
    }
    present_frame(&mut rt, &mut world, 100.0).expect("提示フレーム");
    assert_eq!(
        rt.scroll_manual_at(&PlaceKey::balloon(&key), ScrollDir::Older, 1),
        ScrollChange::Started,
        "前提: 古い行が押し出されていて手動の位置に入る"
    );
    (world, Rc::new(RefCell::new(rt)))
}

/// 普通のバルーンの場所が手動の位置にいるか（新しい側へ 1 行送って、自動なら何も起きない）。
pub(crate) fn balloon_is_manual(rt: &RefCell<TextLayerRuntime>, actor: &str) -> bool {
    use areka_emo_text::place::PlaceKey;
    use areka_emo_text::scroll_marker::{ScrollChange, ScrollDir};
    rt.borrow_mut().scroll_manual_at(
        &PlaceKey::balloon(&ActorKey::from(actor)),
        ScrollDir::Newer,
        1,
    ) != ScrollChange::Unchanged
}

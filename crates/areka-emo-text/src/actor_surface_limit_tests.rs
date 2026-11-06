//! # 文字の面の切り・大きさの上限・作れない面の覚え（audit-emo-text-state-actor 指摘 3・実 GPU）
//!
//! - 画像より大きい validrect の面は、画像の右と下で切った大きさになる（x だけ・y だけ・両方）。
//! - 物理寸の上限（一辺・面積）を越える面は作らず、`error!` は 10 フレームで 1 回のまま。
//! - 資源の無いフレームの失敗は覚えない＝資源が入ると同じ入力で字が出る。
//! - スクロールの後も、切った面の画素は切らない面（今までの形）の左上と同じ。
//! - 箱の文字の面と印の面は「箱の大きさとシェルの画像の重なり」を越えない（シェルの大きさが変われば作り直す）。
//! - 印の面も、装置が無効なフレームの失敗は覚えず、上限越えの覚えは WUC の資源が作り直されると試し直す。
//! - 作れない文字の面の覚えは、WUC の資源の作り直しと装置の作り直しで試し直す。
//! - 一辺が上限を越える矢印の絵は描かず、`error!` は 10 フレームで 1 回のまま。

use std::sync::Arc;

use areka_emo_present::balloon_marker::{MarkerImage, MarkerPair};
use areka_parsers::balloon::{BalloonModel, Font, FontColor, Origin, ValidRect};
use areka_sakura::contract::{ActorKey, CueCommand, TalkCue};
use bevy_ecs::change_detection::DetectChangesMut;
use log_capture_kit::capture;
use wintf::ecs::{GraphicsCore, WucGraphicsResource};

use super::marker::ARROW_LIMIT_MESSAGE;
use super::present::{
    MARKER_LIMIT_MESSAGE, MAX_SURFACE_AREA, MAX_SURFACE_SIDE, SURFACE_LIMIT_MESSAGE,
};
use super::test_support::{
    REVEAL_INTERVAL, balloon, balloon_model_of, box_layout, box_name, com_world, cue, emote,
    fake_resolver, level_count, opaque_count, physical, read_back, spawn_reserved_slot, text,
};
use super::{ResolvedBalloonText, TextLayerRuntime, TextSlotBinding, present_frame};
use crate::place::{PlaceKey, TextPlace};
use crate::state::TextLayerConfig;

/// バルーンの画像の大きさ（image px）。
const IMAGE: (u32, u32) = (200, 100);

/// validrect（上・下・左・右）と字の丈だけを決めたバルーン。
fn model(validrect: ValidRect, font_height: Option<u32>) -> BalloonModel {
    balloon_model_of(
        Origin::new(None, None),
        validrect,
        Font::new(None, font_height, FontColor::new(None, None, None)),
        None,
        None,
    )
}

/// 字を積んだ runtime を、画像 `image`・拡大率 `k` の差し込み口へ `model` で登録する。
fn runtime(
    world: &mut bevy_ecs::prelude::World,
    cues: &[TalkCue],
    model: &BalloonModel,
    image: (u32, u32),
    k: f32,
) -> TextLayerRuntime {
    let (window, slot) = spawn_reserved_slot(world);
    let mut rt = TextLayerRuntime::new(TextLayerConfig::default());
    for c in cues {
        rt.apply_cue(c);
    }
    rt.register_actor(
        ActorKey::from("0"),
        TextSlotBinding::new(slot, window, k, physical(image, k), image),
        ResolvedBalloonText::resolve(model, IMAGE),
    );
    rt
}

#[test]
fn large_validrect_surface_does_not_exceed_the_image() {
    let (mut world, _, _) = com_world();
    let actor = ActorKey::from("0");
    let cues = [cue("0", 0.0, CueCommand::Text("あ".into()))];
    // x だけ・y だけ・両方（片方の切りを外すと、その軸の case が落ちる）。
    let cases = [
        ("x", ValidRect::new(None, None, None, Some(5000))),
        ("y", ValidRect::new(None, Some(4000), None, None)),
        ("xy", ValidRect::new(None, Some(4000), None, Some(5000))),
    ];
    for k in [1.0, 1.5] {
        for (axis, vr) in &cases {
            let mut rt = runtime(&mut world, &cues, &model(*vr, None), IMAGE, k);
            present_frame(&mut rt, &mut world, 1.0).expect("提示");
            let size = rt.surface(&actor).expect("面").size();
            assert_eq!(
                size,
                physical(IMAGE, k),
                "{axis}・k={k}: 面は画像の寸で切る"
            );
            assert!(
                opaque_count(&read_back(&rt, &actor)) > 0,
                "{axis}・k={k}: 字は出る"
            );
        }
    }
}

#[test]
fn over_limit_surface_logs_once_in_ten_frames() {
    let (mut world, _, _) = com_world();
    let actor = ActorKey::from("0");
    let cues = [cue("0", 0.0, CueCommand::Text("あ".into()))];
    // 一辺が x だけ越える・y だけ越える・一辺は内で面積だけ越える（左と上へ大きく負の validrect）。
    let cases = [
        ("x", ValidRect::new(None, None, Some(-100_000), None)),
        ("y", ValidRect::new(Some(-100_000), None, None, None)),
        ("area", ValidRect::new(Some(-6000), None, Some(-6000), None)),
    ];
    for (name, vr) in &cases {
        let mut rt = runtime(&mut world, &cues, &model(*vr, None), IMAGE, 1.0);
        let (results, events) = capture(|| {
            (0..10)
                .map(|i| present_frame(&mut rt, &mut world, 1.0 + f64::from(i) * 0.1))
                .collect::<Vec<_>>()
        });
        assert_eq!(
            level_count(&events, tracing::Level::ERROR),
            1,
            "{name}: ERROR は 10 フレームで 1 回: {events:?}"
        );
        assert!(
            events.iter().any(|e| e.message() == SURFACE_LIMIT_MESSAGE),
            "{name}: {events:?}"
        );
        assert!(results.iter().all(Result::is_ok), "{name}: {results:?}");
        assert!(rt.surface(&actor).is_none(), "{name}: 面は作らない");
        let place = balloon("0");
        let (size, _, _, _) = rt.surface_failed[&place];
        assert!(
            size.0 > MAX_SURFACE_SIDE
                || size.1 > MAX_SURFACE_SIDE
                || u64::from(size.0) * u64::from(size.1) > MAX_SURFACE_AREA,
            "{name}: 覚えたのは上限を越える寸 {size:?}"
        );
    }
}

#[test]
fn text_appears_once_resources_arrive_after_a_frame_without_them() {
    let (mut world, _, _) = com_world();
    let actor = ActorKey::from("0");
    let cues = [cue("0", 0.0, CueCommand::Text("あいう".into()))];
    let vr = ValidRect::new(None, None, None, None);
    let mut rt = runtime(&mut world, &cues, &model(vr, None), IMAGE, 1.0);
    let core = world
        .remove_resource::<GraphicsCore>()
        .expect("GraphicsCore");
    assert!(
        present_frame(&mut rt, &mut world, 1.0).is_err(),
        "資源が無いフレームは作れない"
    );
    assert!(rt.surface(&actor).is_none());
    // 同じ装置を戻す（入力も WUC の資源も同じ）: 一時の失敗を覚えていなければ作れる。
    world.insert_resource(core);
    present_frame(&mut rt, &mut world, 1.1).expect("資源が入ったフレーム");
    assert!(
        opaque_count(&read_back(&rt, &actor)) > 0,
        "同じ入力で字が出る"
    );
}

#[test]
fn rows_of_a_validrect_larger_than_the_surface_draw_the_same_pixels_after_scroll() {
    let (mut world, _, _) = com_world();
    let actor = ActorKey::from("0");
    let place = balloon("0");
    // validrect は画像より右と下へ広い（0,0)-(400,300)。丈 40 の行を 12 行＝面の外まで送る。
    let vr = ValidRect::new(Some(0), Some(300), Some(0), Some(400));
    let m = model(vr, Some(40));
    let mut cues = Vec::new();
    let mut t = 0.0;
    for i in 0..12 {
        let line = format!("行{i}あいう");
        cues.push(cue("0", t, CueCommand::Text(line.clone())));
        t += line.chars().count() as f64 * REVEAL_INTERVAL;
        cues.push(cue("0", t, CueCommand::NewLine { ratio: 1.0 }));
    }
    for k in [1.0, 2.0] {
        // 切る面（画像 200×100）と、切らない面（今までの形＝画像を validrect の大きさにした）。
        let mut clipped = runtime(&mut world, &cues, &m, IMAGE, k);
        let mut full = runtime(&mut world, &cues, &m, (400, 300), k);
        let mut frame = 0.0;
        while frame <= t + 0.1 {
            present_frame(&mut clipped, &mut world, frame).expect("切る面");
            present_frame(&mut full, &mut world, frame).expect("切らない面");
            let (cw, ch) = clipped.surface(&actor).expect("面").size();
            let fw = full.surface(&actor).expect("面").size().0;
            assert_eq!((cw, ch), physical(IMAGE, k));
            let (a, b) = (read_back(&clipped, &actor), read_back(&full, &actor));
            for y in 0..ch as usize {
                let row = |px: &[u8], w: u32| px[y * w as usize * 4..][..cw as usize * 4].to_vec();
                assert!(
                    row(&a, cw) == row(&b, fw),
                    "k={k}・t={frame}: 行 {y} の画素が切らない面と違う"
                );
            }
            frame += 0.07;
        }
        let committed = clipped.surfaces[&place].executor.scroll_state().committed;
        assert!(committed != 0, "k={k}: スクロールが起きている");
        assert!(
            opaque_count(&read_back(&clipped, &actor)) > 0,
            "k={k}: 送った後も字が見える"
        );
    }
}

#[test]
fn text_appears_after_reregistering_on_a_new_slot_with_the_same_size() {
    let (mut world, _, _) = com_world();
    let actor = ActorKey::from("0");
    let cues = [cue("0", 0.0, CueCommand::Text("あいう".into()))];
    let m = model(ValidRect::new(None, None, None, None), None);
    let mut rt = runtime(&mut world, &cues, &m, IMAGE, 1.0);
    // 古い差し込み口が消えた結線: 装着は失敗する（寸と位置は同じのまま）。
    let old_slot = rt.routing[&balloon("0")].slot;
    world.despawn(old_slot);
    assert!(
        present_frame(&mut rt, &mut world, 1.0).is_err(),
        "古い差し込み口"
    );
    // 新しい差し込み口で登録し直す（画像も validrect も同じ）: 次のフレームで字が出る。
    let (window, slot) = spawn_reserved_slot(&mut world);
    rt.register_actor(
        actor.clone(),
        TextSlotBinding::new(slot, window, 1.0, IMAGE, IMAGE),
        ResolvedBalloonText::resolve(&m, IMAGE),
    );
    present_frame(&mut rt, &mut world, 1.1).expect("新しい差し込み口");
    assert!(
        opaque_count(&read_back(&rt, &actor)) > 0,
        "同じ寸でも字が出る"
    );
}

/// シェルの画像（image px）と、箱を置く位置。
const SHELL: (u32, u32) = (300, 200);
const SITE: (u32, u32) = (50, 40);

/// 定義 `def`（ブレスの中身）の箱 `big` を絵 0 の `SITE` に置き、矢印の絵と `lines` 行の字を積んだ runtime。
fn box_runtime(
    world: &mut bevy_ecs::prelude::World,
    def: &str,
    lines: usize,
) -> (TextLayerRuntime, PlaceKey) {
    let actor = ActorKey::from("0");
    let layout = box_layout(&format!(
        "balloon.big
{{
{def}
}}
surface0
{{
element1,balloon,big,{},{}
}}
",
        SITE.0, SITE.1
    ));
    let place = PlaceKey {
        actor: actor.clone(),
        place: TextPlace::Box(box_name(&layout, 0, "big")),
    };
    let mut rt = TextLayerRuntime::new(TextLayerConfig::default());
    rt.set_box_layout(world, layout, fake_resolver(), vec![]);
    let dot = Arc::new(MarkerImage {
        width: 4,
        height: 4,
        bgra: vec![255; 64],
    });
    let pair = MarkerPair {
        up: Some(Arc::clone(&dot)),
        down: Some(dot),
    };
    rt.set_balloon_markers(actor, pair);
    rt.apply_cue(&emote("0", "0"));
    for i in 0..lines {
        rt.apply_cue(&text("0", "あ"));
        rt.apply_cue(&cue("0", i as f64, CueCommand::NewLine { ratio: 1.0 }));
    }
    (rt, place)
}

/// シェル `shell`・拡大率 1 で箱を同期する。
fn sync(rt: &mut TextLayerRuntime, world: &mut bevy_ecs::prelude::World, shell: (u32, u32)) {
    let (window, slot) = {
        let b = rt.routing.values().next().copied();
        b.map(|b| (b.window, b.slot))
            .unwrap_or_else(|| spawn_reserved_slot(world))
    };
    let binding = TextSlotBinding::new(slot, window, 1.0, shell, shell);
    rt.sync_box_bindings(world, &[(ActorKey::from("0"), Some((binding, 0)))]);
}

/// 装置の変化の印を立てずに `GraphicsCore` を差し替える（`with_graphics_core` と同じ戻し方）。
fn swap_core(world: &mut bevy_ecs::prelude::World, core: GraphicsCore) -> GraphicsCore {
    let before = world
        .get_resource_change_ticks::<GraphicsCore>()
        .expect("GraphicsCore")
        .changed;
    let old = world
        .remove_resource::<GraphicsCore>()
        .expect("GraphicsCore");
    world.insert_resource(core);
    world
        .resource_mut::<GraphicsCore>()
        .set_last_changed(before);
    old
}

/// audit-emo-text-state-actor 指摘 3: 印の面を作るフレームに装置が無効でも、その失敗は覚えない＝装置が戻ると同じ寸で作る。
#[test]
fn marker_surface_appears_once_the_device_is_valid_again() {
    let (mut world, _, _) = com_world();
    let (mut rt, place) = box_runtime(&mut world, "size,200,100", 40);
    sync(&mut rt, &mut world, SHELL);
    // まだあふれない時刻で文字の面だけを作る。
    present_frame(&mut rt, &mut world, 0.0).expect("あふれる前の提示");
    assert!(rt.surfaces[&place].markers.is_none(), "まだ矢印は出ない");
    let mut dead = GraphicsCore::new().expect("GraphicsCore");
    dead.invalidate();
    let live = swap_core(&mut world, dead);
    let _ = present_frame(&mut rt, &mut world, 1000.5);
    assert!(
        rt.surfaces[&place].markers.is_none(),
        "無効な装置では作れない"
    );
    swap_core(&mut world, live);
    present_frame(&mut rt, &mut world, 1001.0).expect("装置が戻った提示");
    let markers = rt.surfaces[&place].markers.as_ref();
    assert_eq!(
        markers.map(|m| m.surface().size()),
        Some((200, 100)),
        "同じ寸で印の面ができる"
    );
}

/// audit-emo-text-state-actor 指摘 3: 上限を越える印の面は 10 フレームで記録 1 回、WUC の資源が作り直されると試し直す（記録が
/// もう 1 回）。文字の面は描画範囲で小さく、印の面だけが重なり（17000×100）で一辺の上限を越える。
#[test]
fn over_limit_marker_surface_logs_once_until_wuc_resources_change() {
    let (mut world, _, _) = com_world();
    let (mut rt, place) = box_runtime(
        &mut world,
        "size,17000,100
validrect.right,-16800",
        40,
    );
    sync(&mut rt, &mut world, (20000, 200));
    let count = |rt: &mut TextLayerRuntime, world: &mut bevy_ecs::prelude::World, t: f64| {
        let (_, events) = capture(|| {
            for i in 0..10 {
                let _ = present_frame(rt, world, t + f64::from(i) * 0.1);
            }
        });
        events
            .iter()
            .filter(|e| e.level == tracing::Level::ERROR && e.message() == MARKER_LIMIT_MESSAGE)
            .count()
    };
    assert_eq!(count(&mut rt, &mut world, 1000.0), 1, "10 フレームで 1 回");
    assert!(rt.surfaces[&place].markers.is_none(), "印の面は作らない");
    assert_eq!(
        count(&mut rt, &mut world, 1002.0),
        0,
        "同じ入力では試さない"
    );
    world.resource_mut::<WucGraphicsResource>().set_changed();
    assert_eq!(
        count(&mut rt, &mut world, 1004.0),
        1,
        "WUC の資源が変わると試し直す"
    );
}

/// audit-emo-text-state-actor 指摘 3: 箱の大きさが 16000 でも、箱の文字の面と印の面は「箱とシェルの画像の重なり」
/// （`min(箱の幅, シェルの幅 − X)`・高さも同じ）で切る。x だけ・y だけ・両方を大きくし（片方の切りを
/// 外すと、その軸の case が落ちる）、シェルの画像の大きさが変わると次の提示で切り直す。
#[test]
fn large_box_surfaces_do_not_exceed_the_overlap_with_the_shell_image() {
    let (mut world, window, slot) = com_world();
    let actor = ActorKey::from("0");
    let cases = [
        ("x", (16000, 100)),
        ("y", (100, 16000)),
        ("xy", (16000, 16000)),
    ];
    for k in [1.0, 1.5] {
        for (axis, size) in cases {
            // 箱の丈（100・16000）より多い行＝押し出された行ができて上の矢印が出る。
            let (mut rt, place) = box_runtime(
                &mut world,
                &format!(
                    "size,{},{}
font.height,40",
                    size.0, size.1
                ),
                500,
            );
            for (shell, t) in [(SHELL, 1000.0), ((400, 260), 1001.0)] {
                let binding = TextSlotBinding::new(slot, window, k, physical(shell, k), shell);
                rt.sync_box_bindings(&mut world, &[(actor.clone(), Some((binding, 0)))]);
                // 矢印はあふれた次の提示で出る。
                present_frame(&mut rt, &mut world, t).expect("提示");
                present_frame(&mut rt, &mut world, t + 0.5).expect("次の提示");
                let overlap = (size.0.min(shell.0 - SITE.0), size.1.min(shell.1 - SITE.1));
                let render = &rt.surfaces[&place];
                assert_eq!(
                    render.surface.size(),
                    physical(overlap, k),
                    "{axis}・k={k}・シェル {shell:?}: 文字の面は重なりで切る"
                );
                let markers = render.markers.as_ref().expect("矢印が出て印の面がある");
                assert_eq!(
                    markers.surface().size(),
                    physical(overlap, k),
                    "{axis}・k={k}・シェル {shell:?}: 印の面も重なりで切る"
                );
            }
        }
    }
}

/// 2.4・3.3: 作れなかった文字の面の覚えは、WUC の資源の作り直し（鍵の時刻が変わる）と装置の作り直し
/// （覚えを消す）で、入力が同じでも試し直す（上限越えの記録がそれぞれもう 1 回）。
#[test]
fn remembered_surface_failure_is_retried_after_wuc_or_device_recreation() {
    let (mut world, _, _) = com_world();
    let cues = [cue("0", 0.0, CueCommand::Text("あ".into()))];
    let vr = ValidRect::new(None, None, Some(-100_000), None);
    let mut rt = runtime(&mut world, &cues, &model(vr, None), IMAGE, 1.0);
    let mut count = |world: &mut bevy_ecs::prelude::World| {
        let (_, events) = capture(|| {
            for i in 0..3 {
                let _ = present_frame(&mut rt, world, 1.0 + f64::from(i) * 0.1);
            }
        });
        events
            .iter()
            .filter(|e| e.message() == SURFACE_LIMIT_MESSAGE)
            .count()
    };
    assert_eq!(count(&mut world), 1, "初めの 3 フレームで 1 回");
    assert_eq!(count(&mut world), 0, "同じ入力では試さない");
    // WUC の資源だけを作り直す（覚えの鍵の WUC の時刻が変わる）。
    world.resource_mut::<WucGraphicsResource>().set_changed();
    assert_eq!(count(&mut world), 1, "WUC の資源を作り直すと試し直す");
    assert_eq!(count(&mut world), 0, "作り直した後も同じ入力では試さない");
    // 装置だけを作り直す（WUC の資源の変わった時刻は覚えの鍵のまま）。
    world.resource_mut::<GraphicsCore>().set_changed();
    assert_eq!(count(&mut world), 1, "装置を作り直すと試し直す");
}

/// 一辺が上限（16,384）を越える矢印の絵は GPU へ移せない（入力で決まる失敗）: 10 フレームで `error!` は
/// 1 回・提示は `Err` にしない（毎フレーム描き直して記録し続けない）。
#[test]
fn over_limit_arrow_image_logs_once_in_ten_frames() {
    let (mut world, _, _) = com_world();
    let (mut rt, place) = box_runtime(&mut world, "size,200,100", 40);
    let big = Arc::new(MarkerImage {
        width: MAX_SURFACE_SIDE + 1,
        height: 4,
        bgra: vec![255; (MAX_SURFACE_SIDE as usize + 1) * 16],
    });
    let pair = MarkerPair {
        up: Some(Arc::clone(&big)),
        down: Some(big),
    };
    rt.set_balloon_markers(ActorKey::from("0"), pair);
    sync(&mut rt, &mut world, SHELL);
    let (results, events) = capture(|| {
        (0..10)
            .map(|i| present_frame(&mut rt, &mut world, 1000.0 + f64::from(i) * 0.1))
            .collect::<Vec<_>>()
    });
    assert_eq!(
        level_count(&events, tracing::Level::ERROR),
        1,
        "10 フレームで 1 回: {events:?}"
    );
    assert!(
        events.iter().any(|e| e.message() == ARROW_LIMIT_MESSAGE),
        "矢印の絵の上限の記録: {events:?}"
    );
    assert!(results.iter().all(Result::is_ok), "{results:?}");
    assert!(rt.surfaces[&place].markers.is_some(), "印の面はある");
}

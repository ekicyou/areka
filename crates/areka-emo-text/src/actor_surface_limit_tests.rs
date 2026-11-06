//! # 文字の面の切り・大きさの上限・作れない面の覚え（指摘 3・実 GPU）
//!
//! - 画像より大きい validrect の面は、画像の右と下で切った大きさになる（x だけ・y だけ・両方）。
//! - 物理寸の上限（一辺・面積）を越える面は作らず、`error!` は 10 フレームで 1 回のまま。
//! - 資源の無いフレームの失敗は覚えない＝資源が入ると同じ入力で字が出る。
//! - スクロールの後も、切った面の画素は切らない面（今までの形）の左上と同じ。

use areka_parsers::balloon::{
    BalloonModel, Font, FontColor, Origin, ValidRect, WindowPosition, WordWrapPoint,
};
use areka_sakura::contract::{ActorKey, CueCommand, TalkCue};
use log_capture_kit::capture;
use wintf::ecs::GraphicsCore;

use super::present::{MAX_SURFACE_AREA, MAX_SURFACE_SIDE, SURFACE_LIMIT_MESSAGE};
use super::test_support::{
    REVEAL_INTERVAL, balloon, com_world, cue, opaque_count, read_back, spawn_reserved_slot,
};
use super::{ResolvedBalloonText, TextLayerRuntime, TextSlotBinding, present_frame};
use crate::state::TextLayerConfig;

/// バルーンの画像の大きさ（image px）。
const IMAGE: (u32, u32) = (200, 100);

/// validrect（上・下・左・右）と字の丈だけを決めたバルーン。
fn model(validrect: ValidRect, font_height: Option<u32>) -> BalloonModel {
    BalloonModel::new(
        WindowPosition::new(None, None),
        Origin::new(None, None),
        WordWrapPoint::new(None, None),
        validrect,
        Font::new(None, font_height, FontColor::new(None, None, None)),
        None,
        None,
    )
}

fn physical(size: (u32, u32), k: f32) -> (u32, u32) {
    (
        (size.0 as f32 * k).ceil() as u32,
        (size.1 as f32 * k).ceil() as u32,
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
        let errors: Vec<_> = events
            .iter()
            .filter(|e| e.level == tracing::Level::ERROR)
            .collect();
        assert_eq!(
            errors.len(),
            1,
            "{name}: ERROR は 10 フレームで 1 回: {errors:?}"
        );
        assert_eq!(errors[0].message(), SURFACE_LIMIT_MESSAGE, "{name}");
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

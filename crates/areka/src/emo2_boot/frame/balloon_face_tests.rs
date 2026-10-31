//! 今の定義の唯一の書き手のテスト（balloon-scroll-arrows task 5.3・要件 1.6・1.7・8.6・
//! design「面の切り替え（`balloon_face_tests.rs`）」）。
//!
//! 面 0 → 2 → 0 で今の定義（モデル）と矢印の絵が替わり手動が捨てられること、動く絵のコマ（同じ
//! 面の番号）では差し替えないことを固定する。束は一時フォルダ（ワークツリーの `target\` の下）の
//! 設定ファイルから本番と同じ `load_scope_balloon_definitions` で作り、絵は偽の読み手が返す。
//! 手動の位置は実 GPU で提示した文字の層で作る（`manual_balloon_runtime`）。

use std::collections::HashMap;
use std::sync::mpsc;

use areka_emo_atlas::{AlphaParams, MemoryDecoder, UseSelfAlpha};
use areka_emo_present::TargetId;
use areka_emo_present::balloon::resolve_balloon_faces;
use areka_emo_present::balloon_definitions::{
    ScopeBalloonDefinitions, load_scope_balloon_definitions,
};
use bevy_ecs::world::World;
use log_capture_kit::{CapturedEvent, capture};
use temp_path_kit::TempPath;

use super::{FaceCause, after_drain, changed_faces, select_balloon_face};
use crate::emo2_boot::frame::test_support::{
    balloon_is_manual, headless_wiring_with, manual_balloon_runtime, zero_clock,
};
use crate::emo2_boot::frame::{Emo2Wiring, balloon_target, shell_target};

/// 書き手が文字の層へ矢印の絵を入れたときに文字の層が残す 1 行（`actor_scroll.rs`）。
const MARKERS_MESSAGE: &str = "普通のバルーンの矢印の絵を受け取った";
/// 書き手が文字の層へ行の中の印の絵を入れたときに文字の層が残す 1 行（`actor_decoration.rs`）。
const INLINE_MARKER_MESSAGE: &str = "行の中の印の絵を受け取った";

/// 面 0 は矢印の絵を持たず、面 2 は自分の設定ファイル（書体の大きさ・`arrow.filename,two`）と
/// `two0.png` を持つバルーンの束（scope 0）。
fn two_face_definitions() -> ScopeBalloonDefinitions {
    let dir = TempPath::under_target("balloon-face");
    let mut dec = MemoryDecoder::new();
    let write = |name: &str, text: &str| std::fs::write(dir.child(name), text).expect("ファイル");
    write("descript.txt", "font.height,10\r\n");
    write(
        "balloons2s.txt",
        "font.height,22\r\narrow.filename,two\r\nmarker.filename,mk\r\n",
    );
    for name in ["balloons0.png", "balloons2.png", "two0.png", "mk.png"] {
        write(name, "");
    }
    dec.insert(dir.child("two0.png"), 2, 2, 8, vec![0xff; 16], true);
    dec.insert(dir.child("mk.png"), 3, 3, 12, vec![0xff; 36], true);
    let faces = resolve_balloon_faces(dir.path(), 0).expect("面 0 は在る");
    let alpha = AlphaParams {
        use_self_alpha: UseSelfAlpha::On,
    };
    let defs = load_scope_balloon_definitions(dir.path(), 0, &faces, &dec, alpha);
    assert_ne!(
        defs.for_face(2),
        defs.face0(),
        "前提: 面 2 は面 0 と違う定義"
    );
    assert!(defs.face0().arrows.up.is_none() && defs.for_face(2).arrows.up.is_some());
    assert!(defs.inline_marker_for_face(0).is_none() && defs.inline_marker_for_face(2).is_some());
    defs
}

/// 手動の位置にいる文字の層と束（scope 0）を持つ結線。World は提示の資源ごと返す。
fn rig() -> (World, Emo2Wiring, ScopeBalloonDefinitions) {
    let (world, runtime) = manual_balloon_runtime("0");
    let mut wiring = headless_wiring_with(mpsc::channel().1, zero_clock());
    wiring.runtime = runtime;
    let defs = two_face_definitions();
    wiring.balloon_definitions.insert(0, defs.clone());
    (world, wiring, defs)
}

/// 書き手を 1 回呼び、文字の層が受け取った上向きの矢印の絵の有無を返す。
fn select(wiring: &mut Emo2Wiring, world: &mut World, face: u32, cause: FaceCause) -> Vec<bool> {
    let ((), events) = capture(|| select_balloon_face(wiring, world, 0, face, cause));
    markers_up(&events)
}

fn markers_up(events: &[CapturedEvent]) -> Vec<bool> {
    events
        .iter()
        .filter(|e| e.message().contains(MARKERS_MESSAGE))
        .map(|e| e.field("up") == Some("true"))
        .collect()
}

/// 面 0 → 2 → 0 で今の定義と矢印の絵が替わり、面が替わるたびに手動が捨てられる（要件 1.7・8.6）。
/// 装着（前の面が無い）は手動を捨てない。
#[test]
fn face_0_2_0_replaces_the_definition_and_markers_and_forgets_manual() {
    let (mut world, mut wiring, defs) = rig();

    assert_eq!(
        select(&mut wiring, &mut world, 0, FaceCause::Attach),
        vec![false]
    );
    assert_eq!(wiring.balloon_models[&0], defs.face0().model);
    assert_eq!(wiring.balloon_faces[&0], 0);
    assert!(
        balloon_is_manual(&wiring.runtime, "0"),
        "装着は手動を捨てない"
    );

    restart_manual(&wiring);
    assert_eq!(
        select(&mut wiring, &mut world, 2, FaceCause::FaceChange),
        vec![true]
    );
    assert_eq!(wiring.balloon_models[&0], defs.for_face(2).model);
    assert_eq!(wiring.balloon_faces[&0], 2);
    assert!(
        !balloon_is_manual(&wiring.runtime, "0"),
        "面 0→2 で手動を捨てる"
    );

    restart_manual(&wiring);
    assert_eq!(
        select(&mut wiring, &mut world, 0, FaceCause::FaceChange),
        vec![false]
    );
    assert_eq!(wiring.balloon_models[&0], defs.face0().model);
    assert_eq!(wiring.balloon_faces[&0], 0);
    assert!(
        !balloon_is_manual(&wiring.runtime, "0"),
        "面 2→0 で手動を捨てる"
    );
}

/// 書き手が面の行の中の印の絵を文字の層へ入れる: 印の絵のある面 2 は絵、無い面 0 は `None`
/// （＝既定の丸・script-small-tags 要件 5.1・5.2）。面 2 → 0 で絵を外す。
#[test]
fn face_switch_hands_the_inline_marker_of_the_face() {
    let (mut world, mut wiring, _) = rig();
    let inline = |wiring: &mut Emo2Wiring, world: &mut World, face| {
        let ((), events) =
            capture(|| select_balloon_face(wiring, world, 0, face, FaceCause::FaceChange));
        events
            .iter()
            .filter(|e| e.message().contains(INLINE_MARKER_MESSAGE))
            .map(|e| {
                (
                    e.field("actor").map(str::to_string),
                    e.field("image").map(str::to_string),
                )
            })
            .collect::<Vec<_>>()
    };
    let got = |image: &str| vec![(Some("0".to_string()), Some(image.to_string()))];
    assert_eq!(inline(&mut wiring, &mut world, 0), got("None"));
    assert_eq!(inline(&mut wiring, &mut world, 2), got("Some((3, 3))"));
    assert_eq!(inline(&mut wiring, &mut world, 0), got("None"));
}

/// 同じ面の番号で書き手を通っても（バルーンの切り替えなど）手動は捨てない。
#[test]
fn same_face_keeps_manual() {
    let (mut world, mut wiring, _) = rig();
    select(&mut wiring, &mut world, 2, FaceCause::Attach);
    restart_manual(&wiring);
    select(&mut wiring, &mut world, 2, FaceCause::BalloonSwitch);
    assert!(balloon_is_manual(&wiring.runtime, "0"));
}

/// 手動の位置へ入り直す（`balloon_is_manual` の確かめは手動を自動へ戻すので、次の段の前に戻す）。
fn restart_manual(wiring: &Emo2Wiring) {
    use areka_emo_text::place::PlaceKey;
    use areka_emo_text::scroll_marker::{ScrollChange, ScrollDir};
    use areka_sakura::ActorKey;
    let mut rt = wiring.runtime.borrow_mut();
    let place = PlaceKey::balloon(&ActorKey::from("0"));
    rt.forget_manual_at(&place, areka_emo_text::scroll_marker::StopReason::Cleared);
    assert_eq!(
        rt.scroll_manual_at(&place, ScrollDir::Older, 1),
        ScrollChange::Started
    );
}

/// drain の後の拾い: このフレームで ShowSurface を受けた balloon の target だけを、presenter の
/// 今の面と覚えた面で比べる。動く絵のコマ（同じ面の番号）・シェルの target・受けていない target・
/// presenter が面を持たない（隠した）target は差し替えない。
#[test]
fn changed_faces_picks_only_shown_balloon_targets_whose_face_changed() {
    let faces = HashMap::from([(0, 0), (1, 2)]);
    let presenter = |t: TargetId| match t {
        t if t == balloon_target(0) => Some(2),
        t if t == balloon_target(1) => Some(2),
        _ => Some(9),
    };
    let both = [balloon_target(0), balloon_target(1)];
    assert_eq!(changed_faces(&both, &faces, presenter), vec![(0, 2)]);
    assert_eq!(
        changed_faces(&[balloon_target(1)], &faces, presenter),
        vec![],
        "コマ送り（同じ面の番号）は差し替えない"
    );
    assert_eq!(changed_faces(&[shell_target(0)], &faces, presenter), vec![]);
    assert_eq!(changed_faces(&[], &faces, presenter), vec![]);
    assert_eq!(changed_faces(&both, &faces, |_| None), vec![]);
}

/// presenter が面を持たない（未装着・隠した）target では drain の後に何も書き換えない。
#[test]
fn after_drain_without_presenter_face_changes_nothing() {
    let mut world = World::new();
    let mut wiring = headless_wiring_with(mpsc::channel().1, zero_clock());
    wiring.balloon_faces.insert(0, 0);
    after_drain(&mut wiring, &mut world, &[balloon_target(0)]);
    assert_eq!(wiring.balloon_faces[&0], 0);
    assert!(wiring.balloon_models.is_empty());
}

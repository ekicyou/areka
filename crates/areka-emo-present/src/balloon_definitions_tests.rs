//! 面ごとの定義の束 [`load_scope_balloon_definitions`] のテスト（要件 1.3・1.5・1.7・11.1）。
//!
//! 一時フォルダ（ワークツリーの `target\` の下）に設定ファイルと名前だけの絵を置き、絵は偽の読み手
//! （[`MemoryDecoder`]）が返す。

use super::*;

use std::sync::Arc;

use areka_emo_atlas::MemoryDecoder;
use areka_parsers::balloon::MarkerCoord;
use tracing::Level;

use crate::balloon::resolve_balloon_faces;
use crate::balloon::test_support::{ALPHA, CapturedEvent, FieldUnquoted, TempDir, size};

fn load(dir: &TempDir, dec: &MemoryDecoder) -> (ScopeBalloonDefinitions, Vec<CapturedEvent>) {
    let faces = resolve_balloon_faces(dir.path(), 0).expect("面 0 は在る");
    log_capture_kit::capture(|| load_scope_balloon_definitions(dir.path(), 0, &faces, dec, ALPHA))
}

/// 指摘 20: 空の面列を渡すと、面 0 の定義が無い `error!` を 1 行出して空の定義を返す
/// （panic しない・面 0 必在の契約を破った呼び手でも続ける）。
#[test]
fn empty_faces_record_one_error_and_give_empty_definition() {
    let dir = TempDir::new();
    let dec = MemoryDecoder::new();

    let (defs, events) = log_capture_kit::capture(|| {
        load_scope_balloon_definitions(dir.path(), 0, &[], &dec, ALPHA)
    });
    let errors: Vec<&CapturedEvent> = events.iter().filter(|e| e.level == Level::ERROR).collect();
    assert_eq!(errors.len(), 1, "{events:?}");
    assert!(
        errors[0].message().contains("面 0 の定義が無い"),
        "{errors:?}"
    );
    let empty = FaceDefinition {
        model: areka_parsers::balloon::parse_str("", None),
        arrows: MarkerPair::default(),
    };
    assert_eq!(defs, ScopeBalloonDefinitions::single(0, empty));
}

/// 面 2 の設定ファイルの `validrect`・書体・矢印の鍵は面 2 の定義にだけ入る（要件 1.3・1.7）。
#[test]
fn face_override_lands_only_in_its_face() {
    let dir = TempDir::new();
    let mut dec = MemoryDecoder::new();
    dir.write(
        "descript.txt",
        "validrect.top,1\r\nfont.height,10\r\narrow0.x,5\r\n",
    );
    dir.write(
        "balloons2s.txt",
        "validrect.top,20\r\nfont.height,22\r\narrow0.x,-7\r\narrow.filename,two\r\n",
    );
    for name in ["balloons0.png", "balloons1.png", "balloons2.png"] {
        dir.touch(name);
    }
    dir.image(&mut dec, "arrow0.png", 3, 3);
    dir.image(&mut dec, "arrow1.png", 4, 4);
    dir.image(&mut dec, "two0.png", 6, 6);
    dir.image(&mut dec, "two1.png", 7, 7);

    let (defs, _) = load(&dir, &dec);

    let f0 = defs.face0();
    assert_eq!(f0.model.validrect().top(), Some(1));
    assert_eq!(f0.model.font().height(), Some(10));
    assert_eq!(
        f0.model.arrow_keys().arrow0.x,
        Some(MarkerCoord::FromStart(5))
    );
    assert_eq!(f0.model.arrow_keys().filename, None);
    assert_eq!(size(&f0.arrows.up), Some((3, 3)));

    let f2 = defs.for_face(2);
    assert_eq!(f2.model.validrect().top(), Some(20));
    assert_eq!(f2.model.font().height(), Some(22));
    assert_eq!(
        f2.model.arrow_keys().arrow0.x,
        Some(MarkerCoord::FromEnd(-7))
    );
    assert_eq!(f2.model.arrow_keys().filename.as_deref(), Some("two"));
    assert_eq!(size(&f2.arrows.up), Some((6, 6)));
    assert_eq!(size(&f2.arrows.down), Some((7, 7)));

    assert_eq!(
        defs.for_face(1),
        f0,
        "設定ファイルを持たない面は面 0 と同じ"
    );
    assert_eq!(defs.for_face(9), f0, "定義の無い面は面 0");
}

/// 面 0 以外の設定ファイルを持たないバルーンでは全部の面が面 0 と同じ（要件 1.6）。
#[test]
fn faces_without_own_settings_equal_face0() {
    let dir = TempDir::new();
    let mut dec = MemoryDecoder::new();
    dir.write("descript.txt", "validrect.top,1\r\n");
    dir.write("balloons0s.txt", "validrect.top,3\r\nfont.height,12\r\n");
    for name in ["balloons0.png", "balloons1.png", "balloons2.png"] {
        dir.touch(name);
    }
    dir.image(&mut dec, "arrow0.png", 3, 3);

    let (defs, _) = load(&dir, &dec);

    assert_eq!(defs.face0().model.validrect().top(), Some(3));
    for face in [1, 2] {
        assert_eq!(defs.for_face(face), defs.face0(), "面 {face}");
    }
}

/// 矢印の絵は `arrow.filename` ごとに 1 回だけ解き、面どうしで共有する（設計 MarkerFamilies の Risks）。
/// 面 0・2・3 は名前なし、面 4・5 は `two` → 解決は 2 回・名前の無い絵の warn も 1 回だけ。
#[test]
fn arrow_images_resolved_once_per_name_and_shared() {
    let dir = TempDir::new();
    let mut dec = MemoryDecoder::new();
    dir.write("descript.txt", "validrect.top,1\r\n");
    for face in 0..=5 {
        dir.touch(&format!("balloons{face}.png"));
    }
    dir.write("balloons2s.txt", "validrect.top,2\r\n");
    dir.write("balloons3s.txt", "validrect.top,3\r\n");
    dir.write("balloons4s.txt", "arrow.filename,two\r\n");
    dir.write("balloons5s.txt", "arrow.filename,two\r\n");
    dir.image(&mut dec, "arrow0.png", 3, 3); // 片方だけ → 解決ごとに warn が出る
    dir.image(&mut dec, "two0.png", 6, 6);
    dir.image(&mut dec, "two1.png", 7, 7);

    let (defs, events) = load(&dir, &dec);

    assert_eq!(
        count(&events, Level::DEBUG, "balloon_marker: 印の絵を解いた"),
        2,
        "名前ごとに 1 回"
    );
    assert_eq!(
        count(&events, Level::WARN, "balloon_marker: 印の絵が片方しか無い"),
        1,
        "warn は重複しない"
    );
    for face in [2, 3] {
        assert!(Arc::ptr_eq(
            defs.for_face(face).arrows.up.as_ref().unwrap(),
            defs.face0().arrows.up.as_ref().unwrap()
        ));
    }
    assert!(Arc::ptr_eq(
        defs.for_face(4).arrows.down.as_ref().unwrap(),
        defs.for_face(5).arrows.down.as_ref().unwrap()
    ));
}

/// 読めない座標の値は (鍵, 値) ごとにスコープにつき 1 回だけ warn（要件 1.5）。
#[test]
fn unreadable_coordinate_warns_once_per_key_and_value() {
    let dir = TempDir::new();
    let dec = MemoryDecoder::new();
    dir.write("descript.txt", "arrow0.x,abc\r\n");
    dir.write("balloons2s.txt", "arrow1.y,zz\r\n");
    dir.write("balloons3s.txt", "arrow1.y,zz\r\n");
    for name in ["balloons0.png", "balloons2.png", "balloons3.png"] {
        dir.touch(name);
    }

    let (defs, events) = load(&dir, &dec);

    assert_eq!(defs.for_face(2).model.arrow_keys().arrow1.y, None);
    let mut warns: Vec<(String, String)> = events
        .iter()
        .filter(|e| e.level == Level::WARN && e.message().starts_with("balloon_definitions:"))
        .map(|e| {
            let f = |n: &str| e.field_unquoted(n).expect(n).to_string();
            (f("key"), f("value"))
        })
        .collect();
    warns.sort();
    assert_eq!(
        warns,
        vec![
            ("arrow0.x".to_string(), "abc".to_string()),
            ("arrow1.y".to_string(), "zz".to_string()),
        ]
    );
}

/// 定義の無い面の番号は面 0 の定義を返し debug を残す。
#[test]
fn missing_face_falls_back_to_face0_with_debug() {
    let dir = TempDir::new();
    let dec = MemoryDecoder::new();
    dir.write("descript.txt", "");
    dir.touch("balloons0.png");
    let (defs, _) = load(&dir, &dec);

    let (def, events) = log_capture_kit::capture(|| defs.for_face(5).clone());

    assert_eq!(&def, defs.face0());
    assert!(events.iter().any(|e| {
        e.level == Level::DEBUG
            && e.message()
                .starts_with("balloon_definitions: 面の定義が無い")
            && e.field("face") == Some("5")
    }));
}

fn count(events: &[CapturedEvent], level: Level, prefix: &str) -> usize {
    events
        .iter()
        .filter(|e| e.level == level && e.message().starts_with(prefix))
        .count()
}

/// 面ごとの `marker.filename` で印の絵を選び、設定ファイルを持たない面・番号の無い面は面 0 の値
/// （script-small-tags 要件 5.2）。同じ名前は 1 回だけ読み、面どうしで共有する。
#[test]
fn inline_marker_per_face_and_face0_fallback() {
    let dir = TempDir::new();
    let mut dec = MemoryDecoder::new();
    dir.write("descript.txt", "marker.filename,mine\r\n");
    for face in 0..=3 {
        dir.touch(&format!("balloons{face}.png"));
    }
    dir.write("balloons2s.txt", "marker.filename,two\r\n");
    dir.write("balloons3s.txt", "marker.filename,two\r\n");
    dir.image(&mut dec, "mine.png", 5, 5);
    dir.image(&mut dec, "two.png", 8, 8);
    dir.image(&mut dec, "marker.png", 2, 2);

    let (defs, events) = load(&dir, &dec);

    assert_eq!(size(&defs.inline_marker_for_face(0)), Some((5, 5)));
    assert_eq!(
        size(&defs.inline_marker_for_face(1)),
        Some((5, 5)),
        "設定ファイルを持たない面は面 0 の値"
    );
    assert_eq!(size(&defs.inline_marker_for_face(2)), Some((8, 8)));
    assert_eq!(
        size(&defs.inline_marker_for_face(9)),
        Some((5, 5)),
        "番号の無い面は面 0 の値"
    );
    assert!(Arc::ptr_eq(
        defs.inline_marker_for_face(2).as_ref().unwrap(),
        defs.inline_marker_for_face(3).as_ref().unwrap()
    ));
    assert_eq!(
        count(&events, Level::WARN, "balloon_definitions: 書いた印"),
        0
    );
    assert_eq!(
        count(&events, Level::INFO, "balloon_definitions: 印の絵が無い"),
        0
    );
}

/// 書いた名前の絵が無ければ warn（名前・スコープ・フォルダ）を 1 行出し、連鎖の残りで探す（要件 5.3）。
#[test]
fn inline_marker_missing_explicit_warns_and_falls_back() {
    let dir = TempDir::new();
    let mut dec = MemoryDecoder::new();
    dir.write("descript.txt", "marker.filename,nothere\r\n");
    dir.touch("balloons0.png");
    dir.image(&mut dec, "markers.png", 4, 4);

    let (defs, events) = load(&dir, &dec);

    assert_eq!(size(&defs.inline_marker_for_face(0)), Some((4, 4)));
    let warns: Vec<&CapturedEvent> = events
        .iter()
        .filter(|e| {
            e.level == Level::WARN && e.message().starts_with("balloon_definitions: 書いた印")
        })
        .collect();
    assert_eq!(warns.len(), 1);
    let w = warns[0];
    assert_eq!(w.field_unquoted("name"), Some("nothere"));
    assert_eq!(w.field("scope"), Some("0"));
    assert!(w.field("balloon_dir").is_some());
}

/// 絵が 1 枚も無いバルーンは「絵なし」で、info の記録は面と名前がいくつあっても 1 回だけ（要件 5.4）。
#[test]
fn inline_marker_none_records_info_once() {
    let dir = TempDir::new();
    let dec = MemoryDecoder::new();
    dir.write("descript.txt", "");
    for face in 0..=2 {
        dir.touch(&format!("balloons{face}.png"));
    }
    dir.write("balloons1s.txt", "validrect.top,1\r\n");
    dir.write("balloons2s.txt", "marker.filename,other\r\n");

    let (defs, events) = load(&dir, &dec);

    for face in 0..=2 {
        assert_eq!(defs.inline_marker_for_face(face), None, "面 {face}");
    }
    assert_eq!(
        count(&events, Level::INFO, "balloon_definitions: 印の絵が無い"),
        1,
        "名前の違う面があっても読み込み 1 回につき 1 回"
    );
    assert_eq!(
        count(&events, Level::WARN, "balloon_definitions: 書いた印"),
        1,
        "面 2 の書いた名前 other が無い"
    );
}

/// 矢印でない数の鍵の読めない値も同じ `warn!` の輪で (鍵, 値) ごとに 1 回（audit-parsers-sakura 指摘 18）。
#[test]
fn unreadable_numeric_key_warns_once() {
    let dir = TempDir::new();
    let dec = MemoryDecoder::new();
    dir.write("descript.txt", "font.height,28px\r\n");
    dir.write("balloons0.png", "");

    let (_, events) = load(&dir, &dec);

    let warns: Vec<(String, String)> = events
        .iter()
        .filter(|e| e.level == Level::WARN && e.message().starts_with("balloon_definitions:"))
        .map(|e| {
            let f = |n: &str| e.field(n).expect(n).trim_matches('"').to_string();
            (f("key"), f("value"))
        })
        .collect();
    assert_eq!(warns, vec![("font.height".to_string(), "28px".to_string())]);
}

/// 指摘 40: `descript.txt` の無いバルーンで上書きを持つ面が 2 つあっても、descript は読み込み 1 回に
/// つき 1 回だけ読むので、読めない `warn!` も 1 件だけ（面ごとに読み直さない）。
#[test]
fn missing_descript_warns_once_across_overriding_faces() {
    let dir = TempDir::new();
    let dec = MemoryDecoder::new();
    for face in 0..=2 {
        dir.touch(&format!("balloons{face}.png"));
    }
    dir.write("balloons1s.txt", "validrect.top,1\r\n");
    dir.write("balloons2s.txt", "validrect.top,2\r\n");

    let (defs, events) = load(&dir, &dec);

    assert_eq!(
        count(
            &events,
            Level::WARN,
            "balloon: バルーン既定設定（descript.txt）の読取に失敗"
        ),
        1,
        "{events:?}"
    );
    assert_eq!(defs.for_face(1).model.validrect().top(), Some(1));
    assert_eq!(defs.for_face(2).model.validrect().top(), Some(2));
}

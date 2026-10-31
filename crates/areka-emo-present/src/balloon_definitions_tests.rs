//! 面ごとの定義の束 [`load_scope_balloon_definitions`] のテスト（要件 1.3・1.5・1.7・11.1）。
//!
//! 一時フォルダ（ワークツリーの `target\` の下）に設定ファイルと名前だけの絵を置き、絵は偽の読み手
//! （[`MemoryDecoder`]）が返す。

use super::*;

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use areka_emo_atlas::{MemoryDecoder, UseSelfAlpha};
use areka_parsers::balloon::MarkerCoord;
use log_capture_kit::CapturedEvent;
use tracing::Level;

use crate::balloon::resolve_balloon_faces;
use crate::balloon_marker::MarkerImage;

static COUNTER: AtomicU32 = AtomicU32::new(0);

struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Self {
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../target/test-tmp")
            .join(format!("balloon-definitions-{}-{n}", std::process::id()));
        std::fs::create_dir_all(&path).expect("一時フォルダ");
        TempDir(path)
    }

    fn write(&self, name: &str, text: &str) {
        std::fs::write(self.0.join(name), text).expect("ファイル");
    }

    /// 名前だけのファイルを置き、偽の読み手に `w`×`h` の絵を登録する。
    fn image(&self, dec: &mut MemoryDecoder, name: &str, w: u32, h: u32) {
        self.write(name, "");
        let bgra = vec![0x80; (w * h * 4) as usize];
        dec.insert(self.0.join(name), w, h, w * 4, bgra, true);
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

const ALPHA: AlphaParams = AlphaParams {
    use_self_alpha: UseSelfAlpha::On,
};

fn load(dir: &TempDir, dec: &MemoryDecoder) -> (ScopeBalloonDefinitions, Vec<CapturedEvent>) {
    let faces = resolve_balloon_faces(&dir.0, 0).expect("面 0 は在る");
    log_capture_kit::capture(|| load_scope_balloon_definitions(&dir.0, 0, &faces, dec, ALPHA))
}

fn size(img: &Option<Arc<MarkerImage>>) -> Option<(u32, u32)> {
    img.as_ref().map(|i| (i.width, i.height))
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
        dir.write(name, "");
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
        dir.write(name, "");
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
        dir.write(&format!("balloons{face}.png"), "");
    }
    dir.write("balloons2s.txt", "validrect.top,2\r\n");
    dir.write("balloons3s.txt", "validrect.top,3\r\n");
    dir.write("balloons4s.txt", "arrow.filename,two\r\n");
    dir.write("balloons5s.txt", "arrow.filename,two\r\n");
    dir.image(&mut dec, "arrow0.png", 3, 3); // 片方だけ → 解決ごとに warn が出る
    dir.image(&mut dec, "two0.png", 6, 6);
    dir.image(&mut dec, "two1.png", 7, 7);

    let (defs, events) = load(&dir, &dec);

    let count = |level: Level, prefix: &str| {
        events
            .iter()
            .filter(|e| e.level == level && e.message().starts_with(prefix))
            .count()
    };
    assert_eq!(
        count(Level::DEBUG, "balloon_marker: 印の絵を解いた"),
        2,
        "名前ごとに 1 回"
    );
    assert_eq!(
        count(Level::WARN, "balloon_marker: 印の絵が片方しか無い"),
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
        dir.write(name, "");
    }

    let (defs, events) = load(&dir, &dec);

    assert_eq!(defs.for_face(2).model.arrow_keys().arrow1.y, None);
    let mut warns: Vec<(String, String)> = events
        .iter()
        .filter(|e| e.level == Level::WARN && e.message().starts_with("balloon_definitions:"))
        .map(|e| {
            let f = |n: &str| e.field(n).expect(n).trim_matches('"').to_string();
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
    dir.write("balloons0.png", "");
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

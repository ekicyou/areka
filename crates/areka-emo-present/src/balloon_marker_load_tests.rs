//! 印の絵の読み込みの入口 [`resolve_marker_pair`] のテスト（要件 2.3・2.4・2.5・9.1・9.4・11.1）。
//!
//! 一時フォルダ（ワークツリーの `target\` の下）に名前だけのファイルを置き、絵は偽の読み手
//! （[`MemoryDecoder`]）が返す。戻り・片方だけ・0 枚・壊れた絵の答えと記録の行を固定する。

use super::*;

use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};

use areka_emo_atlas::{MemoryDecoder, UseSelfAlpha};
use log_capture_kit::CapturedEvent;
use tracing::Level;

static COUNTER: AtomicU32 = AtomicU32::new(0);

/// ワークツリーの `target\` の下の一時フォルダ（Drop で消す）。
struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Self {
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../target/test-tmp")
            .join(format!("balloon-marker-{}-{n}", std::process::id()));
        std::fs::create_dir_all(&path).expect("一時フォルダ");
        TempDir(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }

    /// 名前だけのファイルを置く（中身は偽の読み手が返すので空）。
    fn touch(&self, name: &str) {
        std::fs::File::create(self.0.join(name)).expect("ファイル");
    }

    /// 名前だけのファイルを置き、偽の読み手に `w`×`h` の絵を登録する。
    fn image(&self, dec: &mut MemoryDecoder, name: &str, w: u32, h: u32) {
        self.touch(name);
        dec.insert(
            self.0.join(name),
            w,
            h,
            w * 4,
            vec![0x80; (w * h * 4) as usize],
            true,
        );
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

fn size(img: &Option<Arc<MarkerImage>>) -> Option<(u32, u32)> {
    img.as_ref().map(|i| (i.width, i.height))
}

fn marker_events(events: &[CapturedEvent], level: Level) -> Vec<&CapturedEvent> {
    events
        .iter()
        .filter(|e| e.level == level && e.message().starts_with("balloon_marker:"))
        .collect()
}

fn field<'a>(e: &'a CapturedEvent, name: &str) -> &'a str {
    e.field(name).expect(name).trim_matches('"')
}

fn resolve(
    dir: &TempDir,
    scope: u32,
    explicit: Option<&str>,
    dec: &MemoryDecoder,
) -> (MarkerPair, Vec<CapturedEvent>) {
    log_capture_kit::capture(|| {
        resolve_marker_pair(&[dir.path()], &ARROW_FAMILY, scope, explicit, dec, ALPHA)
    })
}

/// 番号ごとの選びと読み込み、解決 1 回につき debug 1 行（要件 2.1・9.1）。
#[test]
fn resolves_each_number_and_logs_one_debug_line() {
    let dir = TempDir::new();
    let mut dec = MemoryDecoder::new();
    dir.image(&mut dec, "arrows0.png", 12, 10);
    dir.image(&mut dec, "arrow1.png", 8, 6);

    let (pair, events) = resolve(&dir, 0, None, &dec);

    assert_eq!(size(&pair.up), Some((12, 10)));
    assert_eq!(size(&pair.down), Some((8, 6)));
    assert_eq!(pair.up.as_ref().unwrap().bgra.len(), 12 * 10 * 4);
    assert!(marker_events(&events, Level::WARN).is_empty());
    let debugs = marker_events(&events, Level::DEBUG);
    assert_eq!(debugs.len(), 1);
    assert_eq!(field(debugs[0], "scope"), "0");
    assert_eq!(field(debugs[0], "chain"), "arrowp0def>arrows>arrow");
    assert!(field(debugs[0], "up").ends_with("arrows0.png 12x10"));
    assert!(field(debugs[0], "down").ends_with("arrow1.png 8x6"));
}

/// 書かれた名前の絵が無い番号は既定の連鎖へ落ち、番号ごとに warn 1 回（要件 2.3）。
#[test]
fn explicit_name_missing_falls_back_with_warn_per_number() {
    let dir = TempDir::new();
    let mut dec = MemoryDecoder::new();
    dir.image(&mut dec, "myarrow0.png", 5, 5);
    dir.image(&mut dec, "arrow0.png", 7, 7);
    dir.image(&mut dec, "arrow1.png", 9, 9);

    let (pair, events) = resolve(&dir, 1, Some("myarrow"), &dec);

    assert_eq!(size(&pair.up), Some((5, 5)), "書かれた名前を先に採る");
    assert_eq!(
        size(&pair.down),
        Some((9, 9)),
        "無い番号は既定の連鎖へ落ちる"
    );
    let warns = marker_events(&events, Level::WARN);
    assert_eq!(warns.len(), 1);
    assert_eq!(field(warns[0], "name"), "myarrow");
    assert_eq!(field(warns[0], "number"), "1");
    assert_eq!(field(warns[0], "scope"), "1");
    let debugs = marker_events(&events, Level::DEBUG);
    assert_eq!(debugs.len(), 1);
    assert_eq!(
        field(debugs[0], "chain"),
        "myarrow>arrowp1def>arrowk>arrowp0def>arrows>arrow"
    );
}

/// 片方だけなら見つかった方だけ・無い番号を warn 1 回（要件 2.5）。
#[test]
fn only_one_side_warns_missing_number() {
    let dir = TempDir::new();
    let mut dec = MemoryDecoder::new();
    dir.image(&mut dec, "arrow0.png", 4, 4);

    let (pair, events) = resolve(&dir, 0, None, &dec);

    assert_eq!(size(&pair.up), Some((4, 4)));
    assert_eq!(pair.down, None);
    let warns = marker_events(&events, Level::WARN);
    assert_eq!(warns.len(), 1);
    assert_eq!(field(warns[0], "missing"), "1");
    let debugs = marker_events(&events, Level::DEBUG);
    assert_eq!(debugs.len(), 1);
    assert_eq!(field(debugs[0], "down"), "none");
}

/// 1 枚も無ければ両方無し・warn 無し・debug 1 行（要件 2.4）。
#[test]
fn no_images_is_default_pair_with_debug_only() {
    let dir = TempDir::new();
    let dec = MemoryDecoder::new();
    dir.touch("balloons0.png");

    let (pair, events) = resolve(&dir, 0, None, &dec);

    assert_eq!(pair, MarkerPair::default());
    assert!(marker_events(&events, Level::WARN).is_empty());
    let debugs = marker_events(&events, Level::DEBUG);
    assert_eq!(debugs.len(), 1);
    assert_eq!(field(debugs[0], "up"), "none");
    assert_eq!(field(debugs[0], "down"), "none");
}

/// 壊れた絵は場所と理由を warn し、その番号だけ無しで続ける（要件 9.4）。
#[test]
fn broken_image_warns_path_and_reason_and_drops_only_that_number() {
    let dir = TempDir::new();
    let mut dec = MemoryDecoder::new();
    dir.image(&mut dec, "arrow0.png", 3, 3);
    dir.touch("arrow1.png");
    dec.insert_corrupt(dir.path().join("arrow1.png"), "bad crc");

    let (pair, events) = resolve(&dir, 0, None, &dec);

    assert_eq!(size(&pair.up), Some((3, 3)));
    assert_eq!(pair.down, None);
    let warns = marker_events(&events, Level::WARN);
    assert_eq!(warns.len(), 1);
    assert!(field(warns[0], "path").ends_with("arrow1.png"));
    assert!(field(warns[0], "error").contains("bad crc"));
    let debugs = marker_events(&events, Level::DEBUG);
    assert_eq!(debugs.len(), 1);
    assert!(field(debugs[0], "down").ends_with("arrow1.png unreadable"));
}

/// フォルダの列は番号ごとに先頭から探す（先のフォルダに無い番号だけ次のフォルダ・要件 7.3）。
#[test]
fn dirs_are_searched_in_order_per_number() {
    let first = TempDir::new();
    let second = TempDir::new();
    let mut dec = MemoryDecoder::new();
    first.image(&mut dec, "arrow0.png", 2, 2);
    second.image(&mut dec, "arrows0.png", 6, 6);
    second.image(&mut dec, "arrows1.png", 5, 5);

    let pair = resolve_marker_pair(
        &[first.path(), second.path()],
        &ARROW_FAMILY,
        0,
        None,
        &dec,
        ALPHA,
    );

    assert_eq!(size(&pair.up), Some((2, 2)), "先のフォルダの旧名が勝つ");
    assert_eq!(size(&pair.down), Some((5, 5)));
}

/// 行の詰め物（stride の余り）は除いて原寸の BGRA にする。
#[test]
fn stride_padding_is_removed() {
    let dir = TempDir::new();
    let mut dec = MemoryDecoder::new();
    dir.touch("arrow0.png");
    let mut bgra = vec![0u8; 12 * 2];
    bgra[..8].copy_from_slice(&[1, 2, 3, 255, 4, 5, 6, 255]);
    bgra[12..20].copy_from_slice(&[7, 8, 9, 255, 10, 11, 12, 255]);
    dec.insert(dir.path().join("arrow0.png"), 2, 2, 12, bgra, true);

    let (pair, _) = resolve(&dir, 0, None, &dec);

    assert_eq!(
        pair.up.unwrap().bgra,
        [1, 2, 3, 255, 4, 5, 6, 255, 7, 8, 9, 255, 10, 11, 12, 255]
    );
}

//! 記録を出さない入口 [`load_shell_target_quiet`] の檻（spec: placement-measure-bake-once
//! 要件 3.1・3.4・3.5・3.6）。
//!
//! 見ているのは 3 つ——⑴ 読み込みの記録を出す検体（重複・入れ子・箱・相手の無いコマ・`.pna`・
//! 画像だけ・descript が読めない）で、出す入口はその記録を出し、出さない入口は読み込みの記録
//! （宛先 `shell_target`・`self_alpha`）を 0 行にする ⑵ 戻りの [`ShellTarget`] が両入口で等しい
//! ⑶ 読み込みを `Err` で終える 3 つの失敗は、出さない入口でも `error!` を 1 行だけ出す。
//!
//! 出す入口の記録の中身（文言・欄・数）は `shell_target_load_tests.rs`・
//! `shell_target_nesting_tests.rs`・`shell_target_boxes_tests.rs` ほかが今のまま持つ。

use super::*;

use std::path::PathBuf;

use areka_emo_atlas::{ElementId, MemoryDecoder};
use temp_path_kit::TempPath;

use super::test_support::{CapturedEvent, capture_events, write_descript};

/// 読み込みの記録の宛先（シェルの権威と、透過の宣言の記録）。
const LOAD_TARGETS: [&str; 2] = [
    "areka_emo_present::shell_target",
    "areka_emo_present::self_alpha",
];

/// 読み込みの記録だけを集める（面の表の構築・焼く段の既存の記録は数えない）。
fn load_records(events: &[CapturedEvent]) -> Vec<&CapturedEvent> {
    events
        .iter()
        .filter(|e| LOAD_TARGETS.contains(&e.target.as_str()))
        .collect()
}

/// 不透明 1×1 PBGRA の絵を `names` の名前で入れた復号器。
fn decoder_with(dir: &Path, names: &[&str]) -> MemoryDecoder {
    let mut dec = MemoryDecoder::new();
    for name in names {
        dec.insert(dir.join(name), 1, 1, 4, vec![10u8, 20, 30, 255], true);
    }
    dec
}

/// 戻り値の比べられる写し（索引表は `HashMap` を持ち `Debug` の順が定まらないので鍵の並びで写す）。
fn fingerprint(target: &ShellTarget) -> String {
    let keys: Vec<&areka_emo_atlas::AtlasKey> = (0..target.atlas.len() as u32)
        .map(|i| target.atlas.key(ElementId(i)))
        .collect();
    let bake_errors: Vec<String> = target.bake_errors.iter().map(|e| e.to_string()).collect();
    format!(
        "{:?}|{:?}|{keys:?}|{bake_errors:?}|{:?}|{:?}|{:?}|{:?}|{}|{:?}|{}|{:?}|{:?}|{}",
        target.shell,
        target.images,
        target.base_images,
        target.dangling,
        target.boxes,
        target.box_report,
        target.box_surfaces,
        target.nest_report,
        target.ignored_pna,
        target.cursor_braces,
        target.use_self_alpha,
        target.world.is_some(),
    )
}

/// 両入口で読み、戻り値が等しいこと・出す入口に `needle` の記録が在ること・出さない入口の
/// 読み込みの記録が 0 行であることを確かめる。
fn assert_quiet_matches_emit(dir: &Path, dec: &MemoryDecoder, needle: &str) {
    let (emit, emit_events) =
        capture_events(|| load_shell_target(dir, dec).expect("出す入口で読める"));
    let (quiet, quiet_events) =
        capture_events(|| load_shell_target_quiet(dir, dec).expect("出さない入口で読める"));

    assert!(
        load_records(&emit_events)
            .iter()
            .any(|e| e.message().contains(needle)),
        "出す入口に `{needle}` の記録が無い（検体が事象を起こしていない）: {emit_events:?}"
    );
    let quiet_records = load_records(&quiet_events);
    assert!(
        quiet_records.is_empty(),
        "出さない入口が読み込みの記録を出した: {quiet_records:?}"
    );
    assert_eq!(
        fingerprint(&quiet),
        fingerprint(&emit),
        "戻り値が両入口で違う"
    );
}

/// 重複・桁溢れ・使わなかった画像・相手の無いコマ・焼く段で落ちた絵を同居させた検体。
#[test]
fn records_fixture_is_silent_and_equal() {
    let dir = TempPath::new("shell-quiet-records");
    std::fs::write(
        dir.child("surfaces.txt"),
        concat!(
            "charset,UTF-8\n",
            "surface0\n{\nelement0,overlay,base0.png,0,0\n}\n",
            "surface1\n{\nelement1,overlay,parts1.png,0,0\n",
            "animation0.interval,random,4\n",
            "animation0.pattern0,overlay,777,0,0,0\n}\n",
        ),
    )
    .expect("記述ファイル作成");
    write_descript(dir.path());
    for name in [
        "surface0.png",
        "surface0000.png",
        "surface1.png",
        "surface99999999999.png",
        "base0.png",
        "parts1.png",
    ] {
        std::fs::File::create(dir.child(name)).expect("プレースホルダ作成");
    }
    // `parts1.png` を入れないので、その 1 枚が焼く段で落ちる（`error!` も黙らせる側）。
    let dec = decoder_with(dir.path(), &["base0.png", "surface1.png"]);

    assert_quiet_matches_emit(dir.path(), &dec, "同じ番号");
    assert_quiet_matches_emit(dir.path(), &dec, "コマの相手");
    assert_quiet_matches_emit(dir.path(), &dec, "脱落");
}

/// 入れ子の検体（無い番号・循環・descript.txt が無い）。
#[test]
fn nesting_fixture_is_silent_and_equal() {
    let dir: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("areka-emo-compose")
        .join("tests")
        .join("fixtures")
        .join("surface-nesting");
    let dec = decoder_with(
        &dir,
        &[
            "body.png",
            "eye.png",
            "eye_closed.png",
            "mouth.png",
            "mouth_open.png",
            "part.png",
            "surface11.png",
            "surface2.png",
        ],
    );
    assert_quiet_matches_emit(&dir, &dec, "element定義が指したサーフェスが無い");
}

/// 箱の検体（箱の数の `info!`）と、誤りのある箱（各件の `warn!`）。
#[test]
fn box_fixtures_are_silent_and_equal() {
    let dir: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("areka-emo-text")
        .join("tests")
        .join("fixtures")
        .join("shell-balloon");
    let dec = decoder_with(&dir, &["body0.png", "body1.png", "body2.png", "body3.png"]);
    assert_quiet_matches_emit(&dir, &dec, "箱を読んだ");

    let bad = TempPath::new("shell-quiet-bad-box");
    std::fs::write(
        bad.child("surfaces.txt"),
        concat!(
            "charset,UTF-8\n",
            "balloon.nosize\n{\nfont.height,12\n}\n",
            "surface0\n{\nelement0,overlay,base.png,0,0\n}\n",
        ),
    )
    .expect("記述ファイル作成");
    write_descript(bad.path());
    let dec = decoder_with(bad.path(), &["base.png"]);
    assert_quiet_matches_emit(bad.path(), &dec, "size の無い");
}

/// `.pna` が添えてある絵（数つきの `warn!`）と、透過の宣言の `info!`（宛先 `self_alpha`）。
#[test]
fn pna_fixture_is_silent_and_equal() {
    let dir = TempPath::new("shell-quiet-pna");
    std::fs::write(
        dir.child("surfaces.txt"),
        "charset,UTF-8\nsurface0\n{\nelement0,overlay,base.png,0,0\n}\n",
    )
    .expect("記述ファイル作成");
    write_descript(dir.path());
    let mut dec = decoder_with(dir.path(), &["base.png"]);
    dec.insert_pna(dir.child("base.png"));
    assert_quiet_matches_emit(dir.path(), &dec, ".pna");
    assert_quiet_matches_emit(dir.path(), &dec, "透過");
}

/// `surfaces.txt` が無く画像だけで組むシェル、と descript.txt が読めないシェル。
#[test]
fn image_only_and_no_descript_are_silent_and_equal() {
    let dir = TempPath::new("shell-quiet-image-only");
    std::fs::File::create(dir.child("surface0.png")).expect("プレースホルダ作成");
    let dec = decoder_with(dir.path(), &["surface0.png"]);
    assert_quiet_matches_emit(dir.path(), &dec, "画像だけで");
    assert_quiet_matches_emit(dir.path(), &dec, "descript.txt が読めない");
}

/// 出さない入口で読み、`Err` が返ることと、読み込みの記録が `needle` を含む `error!` 1 行だけで
/// あることを確かめる（要件 3.6）。
fn assert_quiet_failure(dir: &Path, needle: &str) -> ShellLoadError {
    let dec = MemoryDecoder::new();
    let (result, events) = capture_events(|| load_shell_target_quiet(dir, &dec));
    let records = load_records(&events);
    assert_eq!(records.len(), 1, "失敗の記録だけが 1 行: {records:?}");
    assert_eq!(records[0].level, tracing::Level::ERROR, "{records:?}");
    assert!(records[0].message().contains(needle), "{records:?}");
    result.expect_err("失敗は Err で返る")
}

/// フォルダの一覧に失敗しても、出さない入口は `error!` を 1 行出して `List` を返す。
#[test]
fn quiet_list_failure_still_records_the_error() {
    let dir = PathBuf::from(r"C:\areka-test\shell-quiet-no-dir");
    let error = assert_quiet_failure(&dir, "一覧に失敗");
    assert!(matches!(error, ShellLoadError::List { .. }), "{error:?}");
}

/// `surfaces.txt` が読めなくても、出さない入口は `error!` を 1 行出して `Read` を返す。
#[test]
fn quiet_read_failure_still_records_the_error() {
    let dir = TempPath::new("shell-quiet-unreadable-surfaces-txt");
    std::fs::create_dir(dir.child("surfaces.txt")).expect("フォルダ作成");
    let error = assert_quiet_failure(dir.path(), "読み取りに失敗");
    assert!(matches!(error, ShellLoadError::Read { .. }), "{error:?}");
}

/// 面が 1 つも無くても、出さない入口は `error!` を 1 行出して `Empty` を返す。
#[test]
fn quiet_empty_failure_still_records_the_error() {
    let dir = TempPath::new("shell-quiet-empty-surfaces-txt");
    std::fs::write(dir.child("surfaces.txt"), "charset,UTF-8\n").expect("記述ファイル作成");
    let error = assert_quiet_failure(dir.path(), "面が 1 つも無い");
    assert!(matches!(error, ShellLoadError::Empty { .. }), "{error:?}");
}

/// 焼く段（`areka_emo_atlas`）の 0 寸の記録の文言。
const ZERO_SIZED: &str = "bake: element の元画像が 0 寸です";
/// 畳み込み（`areka_emo_compose`）の追記の相手の無い面の記録の文言。
const APPEND_NO_TARGET: &str = "surface.append 対象 id が未存在";

/// `needle` を文言に含む記録の行の数（宛先まるごとでなく文言ごとに数える）。
fn count_message(events: &[CapturedEvent], needle: &str) -> usize {
    events
        .iter()
        .filter(|e| e.message().contains(needle))
        .count()
}

/// spec: emo-bake-log-overlap 要件 1.1・2.1・3.3: 出す読み込みは焼く段と畳み込みの記録を 1 行ずつ
/// 出し、出さない読み込みはその文言が 0 行・戻り値は等しい。
#[test]
fn quiet_load_silences_bake_and_fold_records() {
    let dir = TempPath::new("shell-quiet-bake-fold");
    std::fs::write(
        dir.child("surfaces.txt"),
        concat!(
            "charset,UTF-8\n",
            "surface0\n{\nelement0,overlay,base.png,0,0\nelement1,overlay,zero.png,0,0\n}\n",
            "surface.append5\n{\nelement2,overlay,base.png,0,0\n}\n",
        ),
    )
    .expect("記述ファイル作成");
    write_descript(dir.path());
    let mut dec = decoder_with(dir.path(), &["base.png"]);
    dec.insert(dir.child("zero.png"), 0, 0, 0, Vec::new(), true);

    let (emit, emit_events) =
        capture_events(|| load_shell_target(dir.path(), &dec).expect("出す入口で読める"));
    let (quiet, quiet_events) =
        capture_events(|| load_shell_target_quiet(dir.path(), &dec).expect("出さない入口で読める"));

    assert_eq!(
        count_message(&emit_events, ZERO_SIZED),
        1,
        "{emit_events:?}"
    );
    assert_eq!(
        count_message(&emit_events, APPEND_NO_TARGET),
        1,
        "{emit_events:?}"
    );
    assert_eq!(
        count_message(&quiet_events, ZERO_SIZED),
        0,
        "{quiet_events:?}"
    );
    assert_eq!(
        count_message(&quiet_events, APPEND_NO_TARGET),
        0,
        "{quiet_events:?}"
    );
    assert_eq!(
        fingerprint(&quiet),
        fingerprint(&emit),
        "戻り値が両入口で違う"
    );
}

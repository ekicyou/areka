//! 記録を出さない入口 [`resolve_balloon_faces_quiet`]・[`load_balloon_use_self_alpha_quiet`] の檻
//! （spec: placement-measure-bake-once 要件 3.2・3.4・3.5・3.6）。
//!
//! 見ているのは 3 つ——⑴ 読み込みの記録を出す検体（相方側の系列を持たず縮退するバルーン・
//! `.pna` を持つバルーン・descript が読めないバルーン）で、出す入口はその記録を出し、出さない
//! 入口は読み込みの記録（宛先 `balloon`・`self_alpha`）を 0 行にする ⑵ 戻り値が両入口で等しい
//! ⑶ 読み込みを `Err` で終える失敗（面 0 が無い・フォルダが無い）は、出さない入口でも `error!` を
//! 1 行だけ出す。
//!
//! 出す入口の記録の中身（文言・欄・数）は `balloon_series_tests.rs`・`balloon_alpha_tests.rs` が
//! 今のまま持つ。

use super::*;

use super::test_support::{CapturedEvent, TempDir, capture_events};

/// 読み込みの記録の宛先（バルーンの権威と、透過の宣言の記録）。
const LOAD_TARGETS: [&str; 2] = [
    "areka_emo_present::balloon",
    "areka_emo_present::self_alpha",
];

/// 読み込みの記録だけを集める。
fn load_records(events: &[CapturedEvent]) -> Vec<&CapturedEvent> {
    events
        .iter()
        .filter(|e| LOAD_TARGETS.contains(&e.target.as_str()))
        .collect()
}

/// 出す入口の記録に `needle` が在り（検体が事象を起こしている）、出さない入口の読み込みの
/// 記録が 0 行であることを確かめる。
fn assert_silenced(emit_events: &[CapturedEvent], quiet_events: &[CapturedEvent], needle: &str) {
    assert!(
        load_records(emit_events)
            .iter()
            .any(|e| e.message().contains(needle)),
        "出す入口に `{needle}` の記録が無い（検体が事象を起こしていない）: {emit_events:?}"
    );
    let quiet_records = load_records(quiet_events);
    assert!(
        quiet_records.is_empty(),
        "出さない入口が読み込みの記録を出した: {quiet_records:?}"
    );
}

/// 系列の解決を両入口で行い、戻り値が等しく、出さない入口の記録が 0 行であることを確かめる。
fn assert_resolve_quiet_matches_emit(dir: &Path, scope: u32, needle: &str) {
    let (emit, emit_events) = capture_events(|| resolve_balloon_faces(dir, scope));
    let (quiet, quiet_events) = capture_events(|| resolve_balloon_faces_quiet(dir, scope));
    assert_silenced(&emit_events, &quiet_events, needle);
    assert_eq!(
        quiet.expect("出さない入口で解決できる"),
        emit.expect("出す入口で解決できる"),
        "戻り値が両入口で違う"
    );
}

/// 透過の宣言の読みを両入口で行い、戻り値が等しく、出さない入口の記録が 0 行であることを確かめる。
fn assert_alpha_quiet_matches_emit(dir: &Path, needle: &str) {
    let (emit, emit_events) = capture_events(|| load_balloon_use_self_alpha(dir));
    let (quiet, quiet_events) = capture_events(|| load_balloon_use_self_alpha_quiet(dir));
    assert_silenced(&emit_events, &quiet_events, needle);
    assert_eq!(quiet, emit, "戻り値が両入口で違う");
}

/// 相方側の系列を持たないバルーンを scope 1 で解決すると全面が本体側へ縮退する。出さない入口は
/// 縮退の `warn!` も解決の完了の `info!` も出さず、採用面列は出す入口と等しい。
#[test]
fn degraded_scope1_resolution_is_silent_and_equal() {
    let dir = TempDir::new();
    dir.touch("balloons0.png");
    dir.touch("balloons1.png");
    assert_resolve_quiet_matches_emit(dir.path(), 1, "縮退した");
    assert_resolve_quiet_matches_emit(dir.path(), 1, "解決が完了");
    assert_resolve_quiet_matches_emit(dir.path(), 0, "解決が完了");
}

/// `.pna` を持つバルーン: 出さない入口は `.pna` の `warn!` も透過の宣言の記録も出さない。
#[test]
fn pna_balloon_alpha_is_silent_and_equal() {
    let dir = TempDir::new();
    dir.write("descript.txt", "charset,UTF-8\nuse_self_alpha,1\n");
    dir.touch("balloons0.png");
    dir.touch("balloons0.pna");
    assert_alpha_quiet_matches_emit(dir.path(), ".pna");
    assert_alpha_quiet_matches_emit(dir.path(), "透過の扱い");
}

/// descript.txt が読めないバルーン: 出さない入口は読めない `warn!` を出さない。
#[test]
fn unreadable_descript_alpha_is_silent_and_equal() {
    let dir = TempDir::new();
    dir.touch("balloons0.png");
    assert_alpha_quiet_matches_emit(dir.path(), "descript.txt");
}

/// 出さない入口の読み込みの記録が `needle` を含む `error!` 1 行だけであることを確かめる（要件 3.6）。
fn assert_single_error(events: &[CapturedEvent], needle: &str) {
    let records = load_records(events);
    assert_eq!(records.len(), 1, "失敗の記録だけが 1 行: {records:?}");
    assert_eq!(records[0].level, tracing::Level::ERROR, "{records:?}");
    assert!(records[0].message().contains(needle), "{records:?}");
}

/// 面 0 が無くても、出さない入口は `error!` を 1 行出して `Err` を返す。
#[test]
fn quiet_missing_face_zero_still_records_the_error() {
    let dir = TempDir::new();
    dir.touch("balloons1.png");
    let (result, events) = capture_events(|| resolve_balloon_faces_quiet(dir.path(), 0));
    assert_single_error(&events, "面 ID 0 が連鎖のどの接頭辞でも解決できない");
    assert!(
        matches!(
            result,
            Err(PresentError::Compose(ComposeError::EmptyComposition(0)))
        ),
        "{result:?}"
    );
}

/// フォルダが無くても、出さない入口は走査の失敗の `error!` を 1 行出す（系列の解決は `Err`・
/// 透過の宣言の読みは宣言なしで続ける）。
#[test]
fn quiet_missing_folder_still_records_the_error() {
    let dir = TempDir::new();
    let missing = dir.path().join("no-such-balloon-dir");

    let (result, events) = capture_events(|| resolve_balloon_faces_quiet(&missing, 0));
    assert_single_error(&events, "走査に失敗");
    assert!(
        matches!(
            result,
            Err(PresentError::Compose(ComposeError::EmptyComposition(0)))
        ),
        "{result:?}"
    );

    let (alpha, events) = capture_events(|| load_balloon_use_self_alpha_quiet(&missing));
    assert_single_error(&events, "走査に失敗");
    assert_eq!(alpha, UseSelfAlpha::Undeclared);
}

/// 焼く段（`areka_emo_atlas`）の抜き色の記録の文言。
const KEY_COLOR: &str = "bake: element を抜き色（左上の 1 画素と同じ色）で透過しました";
/// 焼く段の脱落の `error!` の文言。
const BAKE_FAILED: &str = "balloon: 面画像の bake に失敗";

/// `needle` を文言に含む記録の行の数（宛先まるごとでなく文言ごとに数える）。
fn count_message(events: &[CapturedEvent], needle: &str) -> usize {
    events
        .iter()
        .filter(|e| e.message().contains(needle))
        .count()
}

/// 組んだ組の比べられる写し（索引表の鍵と配置・面の表の面の番号）。
fn target_fingerprint((world, table): &(EmoWorld, AtlasTable)) -> String {
    let entries: Vec<String> = (0..table.len() as u32)
        .map(|i| {
            let id = areka_emo_atlas::ElementId(i);
            format!("{:?}={:?}", table.key(id), table.entry(id).placement)
        })
        .collect();
    let surfaces: Vec<u32> = world.surface_ids().collect();
    format!("{entries:?}|{surfaces:?}")
}

/// spec: emo-bake-log-overlap 要件 1.4・6.4: 抜き色の面 0 で、出す入口は抜き色の記録を 1 行出し、
/// 出さない入口はその文言が 0 行・戻り値は等しい。
#[test]
fn quiet_faces_build_silences_key_color_and_is_equal() {
    let dir = TempDir::new();
    dir.touch("balloons0.png");
    let faces = resolve_balloon_faces(dir.path(), 0).expect("面 0 を解決できる");
    let mut dec = areka_emo_atlas::MemoryDecoder::new();
    // 左上と違う色の画素を 1 つ持たせ、抜いた後も全透明にならない絵にする。
    dec.insert(
        dir.path().join("balloons0.png"),
        2,
        1,
        8,
        vec![10u8, 20, 30, 255, 40, 50, 60, 255],
        false,
    );

    let (emit, emit_events) = capture_events(|| {
        build_balloon_target_from_faces(dir.path(), &dec, &faces, UseSelfAlpha::Off)
            .expect("出す入口で組める")
    });
    let (quiet, quiet_events) = capture_events(|| {
        build_balloon_target_from_faces_quiet(dir.path(), &dec, &faces, UseSelfAlpha::Off)
            .expect("出さない入口で組める")
    });

    assert_eq!(count_message(&emit_events, KEY_COLOR), 1, "{emit_events:?}");
    assert_eq!(
        count_message(&quiet_events, KEY_COLOR),
        0,
        "{quiet_events:?}"
    );
    assert_eq!(
        target_fingerprint(&quiet),
        target_fingerprint(&emit),
        "戻り値が両入口で違う"
    );
}

/// spec: emo-bake-log-overlap 要件 3.5・6.4: 面 0 を読めない絵にすると、出さない入口でも
/// 「面画像の bake に失敗」の `error!` が 1 行出て `Err` を返す。
#[test]
fn quiet_faces_build_still_records_the_bake_failure() {
    let dir = TempDir::new();
    dir.touch("balloons0.png");
    let faces = resolve_balloon_faces(dir.path(), 0).expect("面 0 を解決できる");
    // 絵を登録しない＝読めない面 0。
    let dec = areka_emo_atlas::MemoryDecoder::new();

    let (result, events) = capture_events(|| {
        build_balloon_target_from_faces_quiet(dir.path(), &dec, &faces, UseSelfAlpha::Off)
    });
    let failures: Vec<&CapturedEvent> = events
        .iter()
        .filter(|e| e.message().contains(BAKE_FAILED))
        .collect();
    assert_eq!(failures.len(), 1, "{events:?}");
    assert_eq!(failures[0].level, tracing::Level::ERROR, "{failures:?}");
    assert!(
        matches!(
            result,
            Err(PresentError::Compose(ComposeError::EmptyComposition(0)))
        ),
        "{:?}",
        result.err()
    );
}

//! 2 層マージの記録の行の数の檻。
//!
//! 起動の準備 1 回（窓の位置の読みを出さない入口で通し、資産の組み立てと同じ scope 0・1 の定義の読みを
//! 通す）で、2 層マージの 4 つの文言が（文言, scope, ファイル）ごとに何行出るかを期待の表と比べる。

use std::collections::BTreeMap;
use std::fs;

use areka_emo_atlas::{AlphaParams, MemoryDecoder, UseSelfAlpha};
use areka_emo_present::balloon::{load_scope_balloon_model_quiet, resolve_balloon_faces};
use areka_emo_present::balloon_definitions::load_scope_balloon_definitions;
use log_capture_kit::{CapturedEvent, capture};

use super::shared_test_support::TempDir;
use super::*;

const BASE_UNREADABLE: &str = "balloon: バルーン既定設定（descript.txt）の読取に失敗（空層で継続）";
const OVERRIDE_ABSENT: &str = "balloon: 面別上書き設定が無いため既定設定のみを用いる（正常縮退）";
const OVERRIDE_UNREADABLE: &str = "balloon: 面別上書き設定の読取に失敗（既定設定のみで継続）";
const MERGED: &str = "balloon: scope 別バルーン定義の 2 層マージが確定";

type Counts = BTreeMap<(&'static str, String, String), usize>;

/// 2 層マージの 4 つの文言だけを（文言, scope, ファイル）ごとに数える。欄が無ければ `-`。
fn merge_record_counts(events: &[CapturedEvent]) -> Counts {
    let mut counts = Counts::new();
    for e in events {
        let Some(message) = [
            BASE_UNREADABLE,
            OVERRIDE_ABSENT,
            OVERRIDE_UNREADABLE,
            MERGED,
        ]
        .into_iter()
        .find(|m| e.message() == *m) else {
            continue;
        };
        let field = |name| e.field(name).unwrap_or("-").to_string();
        *counts
            .entry((message, field("scope"), field("file")))
            .or_default() += 1;
    }
    counts
}

fn counts_of(rows: &[(&'static str, &str, &str, usize)]) -> Counts {
    rows.iter()
        .map(|&(m, scope, file, n)| ((m, scope.to_string(), file.to_string()), n))
        .collect()
}

fn startup_prepare_counts(dir: &Path) -> Counts {
    let ((), events) = capture(|| {
        load_scope_windowpositions_with(dir, &[0, 1], load_scope_balloon_model_quiet);
        for scope in [0, 1] {
            let faces = resolve_balloon_faces(dir, scope).expect("面 0 は在る");
            load_scope_balloon_definitions(
                dir,
                scope,
                &faces,
                &MemoryDecoder::new(),
                AlphaParams {
                    use_self_alpha: UseSelfAlpha::Off,
                },
            );
        }
    });
    merge_record_counts(&events)
}

fn touch(dir: &Path, name: &str) {
    fs::write(dir.join(name), b"").expect("検体の絵");
}

fn write(dir: &Path, name: &str, text: &str) {
    fs::write(dir.join(name), text).expect("検体の設定ファイル");
}

fn base_unreadable() -> TempDir {
    let dir = TempDir::new();
    touch(dir.path(), "balloons0.png");
    dir
}

fn override_unreadable() -> TempDir {
    let dir = TempDir::new();
    write(dir.path(), "descript.txt", "validrect.top,7\n");
    touch(dir.path(), "balloons0.png");
    fs::create_dir(dir.path().join("balloons0s.txt")).expect("同名のフォルダ");
    dir
}

fn override_on_scope0_only() -> TempDir {
    let dir = TempDir::new();
    write(dir.path(), "descript.txt", "windowposition.x,10\n");
    write(dir.path(), "balloons0s.txt", "windowposition.x,111\n");
    touch(dir.path(), "balloons0.png");
    touch(dir.path(), "balloonk0.png");
    dir
}

fn face1_with_own_settings() -> TempDir {
    let dir = TempDir::new();
    write(dir.path(), "descript.txt", "windowposition.x,10\n");
    write(dir.path(), "balloons0s.txt", "windowposition.x,111\n");
    write(dir.path(), "balloons1s.txt", "windowposition.x,222\n");
    touch(dir.path(), "balloons0.png");
    touch(dir.path(), "balloons1.png");
    dir
}

#[test]
fn startup_prepare_with_unreadable_base_layer() {
    let dir = base_unreadable();
    assert_eq!(
        startup_prepare_counts(dir.path()),
        counts_of(&[
            (BASE_UNREADABLE, "-", "-", 2),
            (OVERRIDE_ABSENT, "0", "balloons0s.txt", 1),
            (OVERRIDE_ABSENT, "1", "balloons0s.txt", 1),
            (MERGED, "0", "balloons0s.txt", 1),
            (MERGED, "1", "balloons0s.txt", 1),
        ])
    );
}

#[test]
fn startup_prepare_with_override_layer_unreadable_for_another_reason() {
    let dir = override_unreadable();
    assert_eq!(
        startup_prepare_counts(dir.path()),
        counts_of(&[
            (OVERRIDE_UNREADABLE, "0", "balloons0s.txt", 1),
            (OVERRIDE_UNREADABLE, "1", "balloons0s.txt", 1),
            (MERGED, "0", "balloons0s.txt", 1),
            (MERGED, "1", "balloons0s.txt", 1),
        ])
    );
}

#[test]
fn startup_prepare_with_override_layer_on_scope0_only() {
    let dir = override_on_scope0_only();
    assert_eq!(
        startup_prepare_counts(dir.path()),
        counts_of(&[
            (OVERRIDE_ABSENT, "1", "balloonk0s.txt", 1),
            (MERGED, "0", "balloons0s.txt", 1),
            (MERGED, "1", "balloonk0s.txt", 1),
        ])
    );
}

#[test]
fn startup_prepare_with_another_face_having_its_own_settings() {
    let dir = face1_with_own_settings();
    assert_eq!(
        startup_prepare_counts(dir.path()),
        counts_of(&[
            (MERGED, "0", "balloons0s.txt", 1),
            (MERGED, "0", "balloons1s.txt", 1),
            (MERGED, "1", "balloons0s.txt", 1),
            (MERGED, "1", "balloons1s.txt", 1),
        ])
    );
}

fn shell_switch_counts(dir: &Path) -> Counts {
    let (_, events) = capture(|| load_scope_windowpositions(dir, &[0, 1]));
    merge_record_counts(&events)
}

fn lines_with(events: &[CapturedEvent], needle: &str, scope: Option<&str>) -> usize {
    events
        .iter()
        .filter(|e| {
            e.message().contains(needle) && scope.is_none_or(|s| e.field("scope") == Some(s))
        })
        .count()
}

fn assert_failures_still_recorded(dir: &Path, resolution_failure: &str) {
    let (positions, events) =
        capture(|| load_scope_windowpositions_with(dir, &[0], load_scope_balloon_model_quiet));
    assert!(positions.is_empty());
    assert_eq!(
        lines_with(&events, "バルーン系列の解決に失敗", Some("0")),
        1
    );
    assert_eq!(lines_with(&events, resolution_failure, None), 1);
    assert!(merge_record_counts(&events).is_empty());
}

#[test]
fn quiet_path_still_records_a_missing_balloon_folder() {
    let dir = TempDir::new();
    assert_failures_still_recorded(&dir.path().join("absent"), "走査に失敗");
}

#[test]
fn quiet_path_still_records_a_balloon_without_face0() {
    let dir = TempDir::new();
    touch(dir.path(), "balloons1.png");
    assert_failures_still_recorded(dir.path(), "面 ID 0 が連鎖のどの接頭辞でも解決できない");
}

#[test]
fn shell_switch_read_records_the_merge_once_per_scope() {
    assert_eq!(
        shell_switch_counts(base_unreadable().path()),
        counts_of(&[
            (BASE_UNREADABLE, "-", "-", 2),
            (OVERRIDE_ABSENT, "0", "balloons0s.txt", 1),
            (OVERRIDE_ABSENT, "1", "balloons0s.txt", 1),
            (MERGED, "0", "balloons0s.txt", 1),
            (MERGED, "1", "balloons0s.txt", 1),
        ])
    );
    assert_eq!(
        shell_switch_counts(override_unreadable().path()),
        counts_of(&[
            (OVERRIDE_UNREADABLE, "0", "balloons0s.txt", 1),
            (OVERRIDE_UNREADABLE, "1", "balloons0s.txt", 1),
            (MERGED, "0", "balloons0s.txt", 1),
            (MERGED, "1", "balloons0s.txt", 1),
        ])
    );
    assert_eq!(
        shell_switch_counts(override_on_scope0_only().path()),
        counts_of(&[
            (OVERRIDE_ABSENT, "1", "balloonk0s.txt", 1),
            (MERGED, "0", "balloons0s.txt", 1),
            (MERGED, "1", "balloonk0s.txt", 1),
        ])
    );
}

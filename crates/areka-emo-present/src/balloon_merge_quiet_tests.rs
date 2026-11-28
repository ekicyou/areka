//! 2 層マージの記録を出さない入口 [`load_scope_balloon_model_quiet`] の檻。
//!
//! 見ているのは 2 つ——⑴ 記録を起こす検体で、出す入口は対応する記録を出し、出さない入口は宛先
//! `balloon` の記録を 0 行にして戻り値が等しい ⑵ 窓の位置の読みと同じ手順の模型が、資産の組み立ての
//! 手順の面 0 の模型と丸ごと等しい。

use super::*;

use super::test_support::{ALPHA, CapturedEvent, TempDir};
use crate::balloon_definitions::load_scope_balloon_definitions;
use areka_emo_atlas::MemoryDecoder;
use log_capture_kit::capture;

const TARGET: &str = "areka_emo_present::balloon";

fn balloon_records(events: &[CapturedEvent]) -> Vec<&CapturedEvent> {
    events.iter().filter(|e| e.target == TARGET).collect()
}

fn face0_quiet(dir: &Path, scope: u32) -> ResolvedFace {
    resolve_balloon_faces_quiet(dir, scope)
        .expect("面 0 は在る")
        .into_iter()
        .find(|f| f.surface_id == 0)
        .expect("面 0 必在契約")
}

fn assert_quiet_matches_emit(dir: &Path, scope: u32, needle: &str) {
    let face0 = face0_quiet(dir, scope);
    let (emit, emit_events) = capture(|| load_scope_balloon_model(dir, scope, &face0));
    let (quiet, quiet_events) = capture(|| load_scope_balloon_model_quiet(dir, scope, &face0));
    assert!(
        balloon_records(&emit_events)
            .iter()
            .any(|e| e.message().contains(needle)),
        "出す入口に `{needle}` の記録が無い（検体が事象を起こしていない）: {emit_events:?}"
    );
    let quiet_records = balloon_records(&quiet_events);
    assert!(
        quiet_records.is_empty(),
        "出さない入口が記録を出した: {quiet_records:?}"
    );
    assert_eq!(quiet, emit, "戻り値が両入口で違う");
}

#[test]
fn unreadable_base_layer_is_silent_and_equal() {
    let dir = TempDir::new();
    dir.touch("balloons0.png");
    assert_quiet_matches_emit(dir.path(), 0, "バルーン既定設定");
}

#[test]
fn unreadable_override_layer_is_silent_and_equal() {
    let dir = TempDir::new();
    dir.write("descript.txt", "validrect.top,7\n");
    dir.touch("balloons0.png");
    std::fs::create_dir(dir.path().join("balloons0s.txt")).expect("同名ディレクトリ作成");
    assert_quiet_matches_emit(dir.path(), 0, "面別上書き設定の読取に失敗");
}

fn dir_with_override_on_scope0_only() -> TempDir {
    let dir = TempDir::new();
    dir.write(
        "descript.txt",
        "windowposition.x,10\nwindowposition.y,20\nvalidrect.top,1\nvalidrect.bottom,2\n",
    );
    dir.write("balloons0s.txt", "windowposition.x,111\nvalidrect.top,9\n");
    dir.touch("balloons0.png");
    dir.touch("balloonk0.png");
    dir
}

#[test]
fn missing_override_layer_is_silent_and_equal() {
    let dir = dir_with_override_on_scope0_only();
    assert_quiet_matches_emit(dir.path(), 1, "面別上書き設定が無い");
}

#[test]
fn face0_with_override_layer_is_silent_and_equal() {
    let dir = dir_with_override_on_scope0_only();
    assert_quiet_matches_emit(dir.path(), 0, "2 層マージが確定");
}

#[test]
fn placement_path_face0_model_equals_asset_build_face0_model() {
    let dir = dir_with_override_on_scope0_only();
    let model_of = |scope: u32| {
        let placement =
            load_scope_balloon_model_quiet(dir.path(), scope, &face0_quiet(dir.path(), scope));
        let faces = resolve_balloon_faces(dir.path(), scope).expect("面 0 は在る");
        let defs =
            load_scope_balloon_definitions(dir.path(), scope, &faces, &MemoryDecoder::new(), ALPHA);
        assert_eq!(
            placement,
            defs.face0().model,
            "scope {scope} の面 0 の模型が 2 つの道で違う"
        );
        placement
    };
    assert_ne!(
        model_of(0),
        model_of(1),
        "scope 0 と scope 1 の模型が同じでは比べが空虚"
    );
}

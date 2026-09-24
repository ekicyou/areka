//! 無いときの告知の決定論テスト（要件 6.1・6.2・6.5・6.7）。
//!
//! 確かめること: 4 場面（根なし／ゴーストなし／バルーンなし／起動窓を開けない）のそれぞれが
//! 抑止ありで `error!` をちょうど 1 件残し、本文に置くべき場所の絶対パスが入ること・
//! 本文の冒頭の文言・抑止の環境変数の値の読み方。
//!
//! 抑止なしの呼び出し（`MessageBoxW`）はモーダルで止まるので決定論テストに入れず、
//! 実機確認 ③ へ回す。ここでは `raise` を必ず `suppressed = true` で呼ぶ。

use std::path::{Path, PathBuf};

use log_capture_kit::{CapturedEvent, capture};

use super::*;
use crate::boot_config::{RootError, RootSource};

// ---------------------------------------------------------------- 道具立て

/// 実在を問わない絶対パス（本文に綴られることだけを見る）。
fn abs(leaf: &str) -> PathBuf {
    PathBuf::from(r"C:\areka-alert-tests").join(leaf)
}

/// 抑止ありで `raise` を呼び、捕まえた記録を全件返す。
fn raise_suppressed(scene: &AlertScene) -> Vec<CapturedEvent> {
    let ((), events) = capture(|| raise(scene, true));
    events
}

/// 記録がちょうど 1 件の `error!(event = "alert")` であることを確かめ、その本文を返す。
fn only_alert_body(events: &[CapturedEvent]) -> String {
    assert_eq!(
        events.len(),
        1,
        "告知の記録はちょうど 1 件（要件 6.7）: {events:?}"
    );
    let ev = &events[0];
    assert_eq!(
        ev.level,
        tracing::Level::ERROR,
        "告知の記録は error!（要件 6.2）"
    );
    assert_eq!(ev.field_str("event"), Some("alert"));
    ev.field_str("body").expect("本文を載せる").to_owned()
}

fn assert_contains_path(body: &str, path: &Path) {
    assert!(path.is_absolute());
    let shown = path.display().to_string();
    assert!(
        body.contains(&shown),
        "本文に絶対パス {shown} が入る: {body}"
    );
}

// ---------------------------------------------------------------- 4 場面の記録

/// 根なし: 決まった根が実在しない。本文に根の絶対パスが入る（要件 1.4・6.1）。
#[test]
fn root_missing_logs_one_error_with_absolute_root() {
    let dir = abs("no-such-root");
    let scene = AlertScene::RootMissing(RootError::NotADirectory {
        dir: dir.clone(),
        source: RootSource::EnvVar,
    });
    let body = only_alert_body(&raise_suppressed(&scene));
    assert!(body.starts_with("根が決まりません"), "{body}");
    assert_contains_path(&body, &dir);
}

/// ゴーストなし: 本文に格納フォルダ `<根>/ghost` の絶対パスと置くものの形が入る（要件 4.7・6.1）。
#[test]
fn ghost_missing_logs_one_error_with_absolute_store() {
    let ghost_store = abs("root").join("ghost");
    let scene = AlertScene::GhostMissing {
        ghost_store: ghost_store.clone(),
        argv: None,
    };
    let body = only_alert_body(&raise_suppressed(&scene));
    assert!(body.starts_with("ゴーストが見つかりません"), "{body}");
    assert_contains_path(&body, &ghost_store);
    assert!(
        body.contains(r"ghost\master\descript.txt"),
        "置くものの形を示す: {body}"
    );
}

/// バルーンなし: 本文に格納フォルダ `<根>/balloon` の絶対パスが入る（要件 5.8・6.1）。
#[test]
fn balloon_missing_logs_one_error_with_absolute_store() {
    let balloon_store = abs("root").join("balloon");
    let scene = AlertScene::BalloonMissing {
        balloon_store: balloon_store.clone(),
    };
    let body = only_alert_body(&raise_suppressed(&scene));
    assert!(body.starts_with("バルーンが見つかりません"), "{body}");
    assert_contains_path(&body, &balloon_store);
    assert!(body.contains("descript.txt"), "置くものの形を示す: {body}");
}

/// 起動窓を開けない: 本文に失敗の内容が入る（要件 6.4）。
#[test]
fn startup_window_logs_one_error_with_reason() {
    let scene = AlertScene::StartupWindow {
        reason: "モニタ列挙に失敗: 0 台".to_owned(),
    };
    let body = only_alert_body(&raise_suppressed(&scene));
    assert!(body.starts_with("起動窓を開けません"), "{body}");
    assert!(body.contains("モニタ列挙に失敗: 0 台"), "{body}");
}

// ---------------------------------------------------------------- 文面

/// 格納フォルダの場面は「何が無いか」「置くべき場所」「置くものの形」の 3 行。
#[test]
fn store_scenes_have_three_lines() {
    let ghost = AlertScene::GhostMissing {
        ghost_store: abs("root").join("ghost"),
        argv: None,
    };
    let balloon = AlertScene::BalloonMissing {
        balloon_store: abs("root").join("balloon"),
    };
    for scene in [ghost, balloon] {
        let (title, body) = alert_text(&scene);
        assert!(!title.is_empty());
        assert_eq!(body.lines().count(), 3, "{body}");
    }
}

/// argv 起動のゴーストなしは、渡されたパスも本文に入る（要件 4.8）。
#[test]
fn ghost_missing_from_argv_names_the_given_path() {
    let given = abs("not-a-ghost");
    let scene = AlertScene::GhostMissing {
        ghost_store: abs("root").join("ghost"),
        argv: Some(given.clone()),
    };
    let (_, body) = alert_text(&scene);
    assert!(body.starts_with("ゴーストが見つかりません"), "{body}");
    assert_contains_path(&body, &given);
}

/// 空の `AREKA_ROOT` は空のパスを綴らず「空です」と告げる（絶対化できない唯一の場合）。
#[test]
fn root_missing_with_empty_env_says_it_is_empty() {
    let scene = AlertScene::RootMissing(RootError::NotADirectory {
        dir: PathBuf::new(),
        source: RootSource::EnvVar,
    });
    let (_, body) = alert_text(&scene);
    assert!(body.starts_with("根が決まりません"), "{body}");
    assert!(body.contains("環境変数 AREKA_ROOT が空です"), "{body}");
}

/// exe の場所が取れないときは `AREKA_ROOT` で根を渡す道を示す（要件 1.4）。
#[test]
fn root_missing_without_exe_location_points_to_env() {
    let (_, body) = alert_text(&AlertScene::RootMissing(RootError::ExeLocationUnavailable));
    assert!(body.starts_with("根が決まりません"), "{body}");
    assert!(body.contains("AREKA_ROOT"), "{body}");
}

// ---------------------------------------------------------------- 抑止

/// 未設定・空・空白だけ・"0" は出す（抑えない）、それ以外は抑える（要件 6.5）。
#[test]
fn suppressed_from_reads_the_value() {
    assert!(!suppressed_from(None), "未設定は出す");
    assert!(!suppressed_from(Some("")), "空は出す");
    assert!(!suppressed_from(Some("   ")), "空白だけは出す");
    assert!(!suppressed_from(Some("0")), "\"0\" は出す");
    assert!(!suppressed_from(Some(" 0 ")), "trim 後の \"0\" は出す");
    assert!(suppressed_from(Some("1")), "\"1\" は抑える");
    assert!(suppressed_from(Some(" 1 ")), "\" 1 \" は抑える");
}

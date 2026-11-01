//! 無いときの告知の決定論テスト（要件 6.1・6.2・6.5・6.7）。
//!
//! 確かめること: 4 場面（根なし／ゴーストなし／バルーンなし／起動窓を開けない）のそれぞれが
//! 抑止ありで `error!` をちょうど 1 件残すこと・根なし／ゴーストなし／バルーンなしの本文を
//! 複数の置き場を並べた形で丸ごと固定すること（baseware-root-list 要件 9.4〜9.8・12.6）・
//! 抑止の環境変数の値の読み方。
//!
//! 抑止なしの呼び出し（タスクダイアログ）はモーダルで止まるので決定論テストに入れず、
//! 実機確認 ③ へ回す。ここでは `raise` を必ず `suppressed = true` で呼ぶ。

use std::path::PathBuf;

use log_capture_kit::{CapturedEvent, capture};

use super::*;
use crate::roots::{RootError, RootSkip, RootsOrigin};
use areka_kanade::{ShioriFault, ShioriFaultKind};

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

/// 実行ファイルの場所が取れない失敗の値。
fn exe_unavailable() -> RootError {
    RootError::ExeLocationUnavailable {
        tried: Vec::new(),
        origin: RootsOrigin::Default,
        skipped: Vec::new(),
        memory_fell_back: false,
    }
}

// ---------------------------------------------------------------- 置き場・ゴースト・バルーンの文面（要件 9.4〜9.8・12.6）

/// 2 つの置き場（上から A・B）。
fn two_roots() -> [PathBuf; 2] {
    [abs("root-a"), abs("root-b")]
}

/// 抑止ありで告げ、記録の本文が `alert_text` の本文と同じで、それが `expected` であることを確かめる（要件 9.8）。
fn assert_alert_body(scene: &AlertScene, expected: &str) {
    let (title, body) = alert_text(scene);
    assert_eq!(title, "areka を起動できません");
    assert_eq!(body, expected);
    assert_eq!(only_alert_body(&raise_suppressed(scene)), expected);
}

/// 根が決まらない（どの置き場も見つからない）: 試した場所を出どころつきで 1 行ずつ・最後に `;` の案内（要件 9.4・9.5）。
#[test]
fn root_missing_lists_every_tried_root_with_origin() {
    let [a, b] = two_roots();
    for (origin, from) in [
        (RootsOrigin::EnvVar, "環境変数 AREKA_ROOT"),
        (RootsOrigin::Memory, "アプリの記憶"),
        (RootsOrigin::Default, "既定（実行ファイルの隣）"),
    ] {
        let scene = AlertScene::RootMissing(RootError::NoneFound {
            tried: vec![a.clone(), b.clone()],
            origin,
            skipped: Vec::new(),
            memory_fell_back: false,
        });
        let expected = format!(
            "置き場（ゴーストとバルーンを置くフォルダ）が見つかりません。\n\
             試した場所: {}（{from}）\n\
             試した場所: {}（{from}）\n\
             置くもの: ghost フォルダと balloon フォルダを持つフォルダ\n\
             環境変数 AREKA_ROOT に、置き場のフォルダの絶対パスを ; で区切って複数渡せます。",
            a.display(),
            b.display()
        );
        assert_alert_body(&scene, &expected);
    }
}

/// 空の `AREKA_ROOT` は 1 行目を理由にし、空のパスを綴らない（要件 9.4・9.5）。
#[test]
fn root_missing_with_empty_env_says_it_is_empty() {
    let scene = AlertScene::RootMissing(RootError::EnvAllEmpty {
        skipped: vec![RootSkip::Empty { index: 0 }, RootSkip::Empty { index: 1 }],
        memory_fell_back: false,
    });
    assert_alert_body(
        &scene,
        "置き場が決まりません: 環境変数 AREKA_ROOT が空です。\n\
         置くもの: ghost フォルダと balloon フォルダを持つフォルダ\n\
         環境変数 AREKA_ROOT に、置き場のフォルダの絶対パスを ; で区切って複数渡せます。",
    );
}

/// 実行ファイルの場所が取れないときは 1 行目を理由にし、`AREKA_ROOT` で渡す道を示す（要件 9.4・9.5）。
#[test]
fn root_missing_without_exe_location_says_why() {
    assert_alert_body(
        &AlertScene::RootMissing(exe_unavailable()),
        "置き場が決まりません: 実行ファイルの場所が取れません。\n\
         置くもの: ghost フォルダと balloon フォルダを持つフォルダ\n\
         環境変数 AREKA_ROOT に、置き場のフォルダの絶対パスを ; で区切って複数渡せます。",
    );
}

/// ゴーストが無い: 各置き場の `ghost\` を 1 行ずつ絶対パスで（要件 9.6）。
#[test]
fn ghost_missing_lists_every_ghost_store() {
    let [a, b] = two_roots().map(|r| r.join("ghost"));
    let scene = AlertScene::GhostMissing {
        ghost_stores: vec![a.clone(), b.clone()],
        argv: None,
    };
    let expected = format!(
        "ゴーストが見つかりません。\n\
         探した場所: {}\n\
         探した場所: {}\n\
         置くもの: ghost\\master\\descript.txt を持つゴーストのフォルダ",
        a.display(),
        b.display()
    );
    assert_alert_body(&scene, &expected);
}

/// argv 起動のゴーストなしは、1 行目に渡されたパスを入れ、探した場所は同じ形（要件 9.6）。
#[test]
fn ghost_missing_from_argv_names_the_given_path() {
    let given = abs("not-a-ghost");
    let [a, b] = two_roots().map(|r| r.join("ghost"));
    let scene = AlertScene::GhostMissing {
        ghost_stores: vec![a.clone(), b.clone()],
        argv: Some(given.clone()),
    };
    let expected = format!(
        "ゴーストが見つかりません: 渡されたフォルダ {} はゴーストではありません。\n\
         探した場所: {}\n\
         探した場所: {}\n\
         置くもの: ghost\\master\\descript.txt を持つゴーストのフォルダ",
        given.display(),
        a.display(),
        b.display()
    );
    assert_alert_body(&scene, &expected);
}

/// バルーンが無い: 各置き場の `balloon\` を 1 行ずつ絶対パスで（要件 9.7）。
#[test]
fn balloon_missing_lists_every_balloon_store() {
    let [a, b] = two_roots().map(|r| r.join("balloon"));
    let scene = AlertScene::BalloonMissing {
        balloon_stores: vec![a.clone(), b.clone()],
    };
    let expected = format!(
        "バルーンが見つかりません。\n\
         探した場所: {}\n\
         探した場所: {}\n\
         置くもの: descript.txt を持つバルーンのフォルダ",
        a.display(),
        b.display()
    );
    assert_alert_body(&scene, &expected);
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

// ---------------------------------------------------------------- SHIORI が動かなくなった

/// 題名が起動失敗と別であることを、既存 4 場面の側からも確かめる（要件 1.3・5.2）。
#[test]
fn startup_scenes_keep_the_startup_title() {
    let scenes = [
        AlertScene::RootMissing(exe_unavailable()),
        AlertScene::GhostMissing {
            ghost_stores: vec![abs("root").join("ghost")],
            argv: None,
        },
        AlertScene::BalloonMissing {
            balloon_stores: vec![abs("root").join("balloon")],
        },
        AlertScene::StartupWindow {
            reason: "x".to_owned(),
        },
    ];
    for scene in scenes {
        assert_eq!(alert_text(&scene).0, "areka を起動できません", "{scene:?}");
    }
}

/// SHIORI の失敗の告知を組み、文面と記録を確かめる。`phrase` は種類の 5 語をそのまま書く
/// （`fault_kind_text` を通さない＝写しの取り違えが赤になる）（要件 1.3・1.4・1.5・1.9・3.5・7.1）。
fn assert_shiori_fault(kind: ShioriFaultKind, phrase: &str, name: Option<&str>) {
    let root = abs("root").join("ghost").join("emily");
    let reason = "shiori handshake failure: 接続が拒否されました";
    let scene = AlertScene::ShioriFault {
        ghost_name: name.map(str::to_owned),
        ghost_root: root.clone(),
        fault: ShioriFault {
            kind,
            reason: reason.to_owned(),
        },
    };
    let who = match name {
        Some(n) => format!("ゴースト: {n}（{}）", root.display()),
        None => format!("ゴースト: {}", root.display()),
    };
    let expected = format!("{who}\n失敗の種類: {phrase}\n理由: {reason}");

    let (title, body) = alert_text(&scene);
    assert_eq!(title, "SHIORI が動かなくなりました");
    assert_eq!(body, expected);

    // 告知と同じ題名・本文が error! の 1 件に載る（抑止しても記録は残る）。
    let events = raise_suppressed(&scene);
    assert_eq!(only_alert_body(&events), expected);
    assert_eq!(
        events[0].field_str("title"),
        Some("SHIORI が動かなくなりました")
    );
}

#[test]
fn shiori_fault_connect_failed_with_name() {
    assert_shiori_fault(
        ShioriFaultKind::ConnectFailed,
        "SHIORI に接続できなかった",
        Some("エミリ"),
    );
}

#[test]
fn shiori_fault_connect_failed_without_name() {
    assert_shiori_fault(
        ShioriFaultKind::ConnectFailed,
        "SHIORI に接続できなかった",
        None,
    );
}

#[test]
fn shiori_fault_timeout_with_name() {
    assert_shiori_fault(
        ShioriFaultKind::Timeout,
        "SHIORI の応答が期限内に返らなかった",
        Some("エミリ"),
    );
}

#[test]
fn shiori_fault_timeout_without_name() {
    assert_shiori_fault(
        ShioriFaultKind::Timeout,
        "SHIORI の応答が期限内に返らなかった",
        None,
    );
}

#[test]
fn shiori_fault_disconnected_with_name() {
    assert_shiori_fault(
        ShioriFaultKind::Disconnected,
        "SHIORI との通信が切れた",
        Some("エミリ"),
    );
}

#[test]
fn shiori_fault_disconnected_without_name() {
    assert_shiori_fault(
        ShioriFaultKind::Disconnected,
        "SHIORI との通信が切れた",
        None,
    );
}

#[test]
fn shiori_fault_internal_with_name() {
    assert_shiori_fault(
        ShioriFaultKind::Internal,
        "areka 側の内部の失敗",
        Some("エミリ"),
    );
}

#[test]
fn shiori_fault_internal_without_name() {
    assert_shiori_fault(ShioriFaultKind::Internal, "areka 側の内部の失敗", None);
}

#[test]
fn shiori_fault_unknown_with_name() {
    assert_shiori_fault(ShioriFaultKind::Unknown, "原因不明", Some("エミリ"));
}

#[test]
fn shiori_fault_unknown_without_name() {
    assert_shiori_fault(ShioriFaultKind::Unknown, "原因不明", None);
}

// ---------------------------------------------------------------- はい／いいえ（ghost-install 要件 4.4・4.7）

/// 抑止ありなら画面を出さずに `Suppressed` を返し、`info!(event = "ask")` をちょうど 1 件残す。
/// 抑止なしの呼び出しはモーダルで止まるので決定論テストに入れない。
#[test]
fn ask_yes_no_suppressed_returns_suppressed_and_logs_one_info() {
    let (answer, events) = capture(|| ask_yes_no("利用条件", "本文", true));
    assert_eq!(answer, YesNo::Suppressed);
    assert_eq!(events.len(), 1, "記録はちょうど 1 件: {events:?}");
    let ev = &events[0];
    assert_eq!(ev.level, tracing::Level::INFO);
    assert_eq!(ev.field_str("event"), Some("ask"));
    assert_eq!(ev.field_str("title"), Some("利用条件"));
    assert_eq!(ev.field("suppressed"), Some("true"));
}

// ---------------------------------------------------------------- OK だけの警告（emily-ghost-verification 要件 1.12）

/// 抑止ありなら画面を出さず、`warn!(event = "install_warning_dialog")` をちょうど 1 件残す
/// （欄は題名・本文・抑えたか）。抑止なしの呼び出しはモーダルで止まるので決定論テストに入れない。
#[test]
fn warn_ok_suppressed_shows_nothing_and_logs_one_warn() {
    let ((), events) = capture(|| warn_ok("インストールの警告 - X", "書庫: x.nar\n本文", true));
    assert_eq!(events.len(), 1, "記録はちょうど 1 件: {events:?}");
    let ev = &events[0];
    assert_eq!(ev.level, tracing::Level::WARN);
    assert_eq!(ev.field_str("event"), Some("install_warning_dialog"));
    assert_eq!(ev.field_str("title"), Some("インストールの警告 - X"));
    assert_eq!(ev.field_str("body"), Some("書庫: x.nar\n本文"));
    assert_eq!(ev.field("suppressed"), Some("true"));
}

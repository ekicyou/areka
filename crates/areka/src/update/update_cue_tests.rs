//! `UpdateCueSink` と `parse_update_command` の決定論テスト。
//!
//! 確かめること: 入口の 4 つの形（`updatebymyself`・`update,all`・`update,shell+balloon`・
//! `updateother,--balloon=B,--shell=S`）が台本の文字列から期待の生の要求 1 件になること・
//! 更新オプション付き・`platform`・`--plugin=` だけ・`--option=` 混じりが要求 0 と `warn!` 1 件に
//! なること・`--shell=S,--plugin=P` が `P` を読み飛ばして `S` だけの要求になること・担当外は 0 件で
//! 警告しないこと・開けない自分宛の荷物と受信端の落ちた送出は `warn!` 1 件で台本を殺さないこと。

use super::*;
use std::sync::mpsc::{Receiver, channel};

use areka_ghost::BasewareRoot;
use bevy_ecs::world::World;
use dola::cue::CueSink;
use log_capture_kit::{CapturedEvent, capture};
use temp_path_kit::TempPath;

use areka_runtime::consumer_ledger::test_support::{broken_custom_cue, carrier_cue};

use crate::session_tests::write_descript;
use crate::session_tests::{events_named, play_into, warns};
use crate::update::TargetKind::{self, Balloon, Ghost, Shell};
use crate::update::desk;
use crate::update::update_test_support::slot_world;

// ---------------------------------------------------------------- 道具立て

fn sink() -> (UpdateCueSink, Receiver<RawUpdateRequest>) {
    let (tx, rx) = channel();
    (UpdateCueSink::new(tx), rx)
}

/// 素の受信端を渡した受け口へ台本の文字列を配る。
fn play_script(script: &str) -> (Vec<RawUpdateRequest>, Vec<CapturedEvent>) {
    let (sink, rx) = sink();
    let ((), events) = capture(|| play_into(script, sink));
    (rx.try_iter().collect(), events)
}

/// 窓口を据え、起動の文脈と置き場のゴースト（ゴースト `emo`・名前 `Emo`）を置いた World。
///
/// kanade の受信端はすぐ落とす: 受けた依頼で起きる背景スレッドは最初のリソースの照会で
/// 「kanade が居ない」として依頼をやめる（取得へ進まない＝ネットへ出ない）。受付の判定は
/// UI スレッドの記録で見る。
fn desk_world() -> (World, TempPath) {
    let tmp = TempPath::new("areka-update-cue-desk");
    let root = BasewareRoot::new(tmp.path().to_path_buf());
    write_descript(
        &root.ghost_dir("emo").join("ghost").join("master"),
        "name,Emo",
    );
    let (world, _) = slot_world(tmp.path(), "emo", "kaku");
    (world, tmp)
}

/// 台本の文字列を窓口の送出端を借りた受け口へ配り、取り出しを 1 回回す。
fn play_to_desk(script: &str) -> Vec<CapturedEvent> {
    let (mut world, _tmp) = desk_world();
    let sink = UpdateCueSink::new(desk::raw_sender(&world));
    let ((), events) = capture(|| {
        play_into(script, sink);
        desk::drain(&mut world);
    });
    events
}

fn emit_one(name: &str, tokens: &[&str]) -> (Vec<RawUpdateRequest>, Vec<CapturedEvent>) {
    let (mut sink, rx) = sink();
    let ((), events) = capture(|| sink.emit(carrier_cue(name, tokens)));
    (rx.try_iter().collect(), events)
}

fn other(pairs: &[(TargetKind, &str)]) -> RawUpdateRequest {
    RawUpdateRequest::Other(pairs.iter().map(|(k, n)| (*k, n.to_string())).collect())
}

// ---------------------------------------------------------------- 入口の 4 つの形

/// 4 つの入口の形が、台本の文字列から期待の生の要求 1 件になり、警告を残さない。
#[test]
fn four_entry_forms_become_the_expected_request() {
    let cases = [
        (
            r"\![updatebymyself]\e",
            RawUpdateRequest::Current(vec![Ghost, Shell, Balloon]),
        ),
        (
            r"\![update,all]\e",
            RawUpdateRequest::Current(vec![Ghost, Shell, Balloon]),
        ),
        (
            r"\![update,shell+balloon]\e",
            RawUpdateRequest::Current(vec![Shell, Balloon]),
        ),
        (
            r"\![updateother,--balloon=B,--shell=S]\e",
            other(&[(Balloon, "B"), (Shell, "S")]),
        ),
    ];
    for (script, expected) in cases {
        let (sent, events) = play_script(script);
        assert_eq!(sent, vec![expected], "{script}");
        assert!(warns(&events).is_empty(), "{script}: 警告なし {events:?}");
    }
}

/// `update` の対象は並んだ順（逆順・1 つだけも）で運び、重複はそのまま・`all` は 3 つに開く。
#[test]
fn update_targets_keep_their_order() {
    let cases: [(&[&str], Vec<TargetKind>); 4] = [
        (&["balloon+ghost"], vec![Balloon, Ghost]),
        (&["ghost"], vec![Ghost]),
        (&["shell+ghost+balloon"], vec![Shell, Ghost, Balloon]),
        (&["all+ghost"], vec![Ghost, Shell, Balloon, Ghost]),
    ];
    for (params, expected) in cases {
        assert_eq!(
            parse_update_command("update", params),
            Ok(Parsed {
                request: RawUpdateRequest::Current(expected),
                ignored: vec![],
            }),
            "{params:?}"
        );
    }
}

// ---------------------------------------------------------------- 読み飛ばし

/// `--shell=S,--plugin=P` は `P` を `warn!` 1 件で読み飛ばし、`S` だけの要求になる。
#[test]
fn unknown_selector_is_skipped_with_one_warning() {
    let (sent, events) = play_script(r"\![updateother,--shell=S,--plugin=P]\e");
    assert_eq!(sent, vec![other(&[(Shell, "S")])]);
    assert_eq!(warns(&events), vec![Some("update_cue_selector_ignored")]);
    assert_eq!(
        parse_update_command("updateother", &["--shell=S", "--plugin=P"]),
        Ok(Parsed {
            request: other(&[(Shell, "S")]),
            ignored: vec!["--plugin=P".into()],
        })
    );
}

// ---------------------------------------------------------------- 断る

/// オプション付き・`platform`・`--plugin=` だけ・`--option=` 混じりは要求 0 と `warn!` 1 件になる。
#[test]
fn refused_forms_send_nothing_and_warn_once() {
    let scripts = [
        r"\![updatebymyself,checkonly]\e",
        r"\![update,all,testonly]\e",
        r"\![update,ghost,recovery]\e",
        r"\![update,platform]\e",
        r"\![update,ghost+platform]\e",
        r"\![updateother,--plugin=P]\e",
        r"\![updateother,--shell=S,--option=x]\e",
        r"\![updateother,--shell=S,checkonly]\e",
    ];
    for script in scripts {
        let (sent, events) = play_script(script);
        assert!(sent.is_empty(), "{script}: 要求 0");
        assert_eq!(
            warns(&events),
            vec![Some("update_cue_refused")],
            "{script}: 警告 1 件"
        );
    }
}

/// 断りの種類（純粋な解析）。
#[test]
fn refusal_kinds() {
    let option = |s: &str| Err(Refusal::Option { found: s.into() });
    assert_eq!(
        parse_update_command("updatebymyself", &["checkonly"]),
        option("checkonly")
    );
    assert_eq!(
        parse_update_command("update", &["all", "--option=x"]),
        option("--option=x")
    );
    assert_eq!(
        parse_update_command("update", &["platform"]),
        Err(Refusal::UnknownTarget {
            found: "platform".into()
        })
    );
    assert_eq!(
        parse_update_command("update", &[]),
        Err(Refusal::UnknownTarget { found: "".into() })
    );
    assert_eq!(
        parse_update_command("updateother", &["--shell=S", "--option=x", "--plugin=P"]),
        option("--option=x")
    );
    assert_eq!(
        parse_update_command("updateother", &["--shell=S", "foo"]),
        option("foo"),
        "`--` で始まらない語は読めない引数として断る"
    );
    assert_eq!(
        parse_update_command("updateother", &["--plugin=P"]),
        Err(Refusal::NoTargets)
    );
    assert_eq!(
        parse_update_command("updateother", &[]),
        Err(Refusal::NoTargets)
    );
}

// ---------------------------------------------------------------- 担当外

/// 他の名前（似た名前・`execute` を含む）は 0 件で警告しない。
#[test]
fn not_ours_is_benign_skip_without_warning() {
    let cases: [(&str, &[&str]); 3] = [
        ("execute", &["install", "path", r"C:\x.nar"]),
        ("updatex", &[]),
        ("reload", &["ghost"]),
    ];
    for (name, tokens) in cases {
        let (sent, events) = emit_one(name, tokens);
        assert!(sent.is_empty(), "{name}: 送らない");
        assert!(warns(&events).is_empty(), "{name}: 警告しない");
    }
}

/// 開封できない自分宛の荷物は `warn!` 1 件で送らない。
#[test]
fn unopenable_own_cue_warns_and_sends_nothing() {
    for name in ["updatebymyself", "update", "updateother"] {
        let (mut sink, rx) = sink();
        let ((), events) = capture(|| sink.emit(broken_custom_cue("0", name)));
        assert_eq!(rx.try_iter().count(), 0, "{name}");
        assert_eq!(
            warns(&events),
            vec![Some("update_cue_unopenable")],
            "{name}"
        );
    }
}

// ---------------------------------------------------------------- 送出の失敗

/// 受信端が落ちていても落ちず、送れなかったことを `warn!` 1 件で残す。
#[test]
fn dropped_receiver_is_logged_and_does_not_panic() {
    let (mut sink, rx) = sink();
    drop(rx);
    let ((), events) = capture(|| sink.emit(carrier_cue("updatebymyself", &[])));
    assert_eq!(warns(&events), vec![Some("update_cue_send_failed")]);
}

// ---------------------------------------------------------------- 窓口まで

/// 台本の文字列から受け口 → 窓口の取り出し → 受付まで通すと、出どころ「台本」の依頼がちょうど
/// 1 件受け付けられ、警告は無い。
///
/// 背景スレッドへ渡した依頼の中身は `update` の外から読めないので、受付が渡したときに残す
/// `info!(update_order_started)`（出どころと対象の名前）で判定する。
#[test]
fn script_string_reaches_the_desk_and_is_accepted() {
    let events = play_to_desk(r"\![update,ghost]\e");
    let started = events_named(&events, "update_order_started");
    assert_eq!(started.len(), 1, "依頼はちょうど 1 件: {events:?}");
    assert_eq!(started[0].level, tracing::Level::INFO);
    assert_eq!(
        started[0].field_str("origin"),
        Some("script"),
        "出どころは台本"
    );
    assert_eq!(
        started[0].field("targets"),
        Some(r#"["Emo"]"#),
        "対象は今のゴースト"
    );
    // 背景の `update` の担い手は窓を引き継ぐが、閉じる前に記録が届くかは間合い次第なので数えない
    let on_path_warns: Vec<_> = events
        .iter()
        .filter(|e| e.level == tracing::Level::WARN && e.target != "areka::update::procedure")
        .collect();
    assert!(on_path_warns.is_empty(), "断りも警告もない: {events:?}");
}

/// 断りの形は窓口まで通しても依頼 0 件・`warn!` 1 件（受け口の断りだけ）。
#[test]
fn refused_form_reaches_no_order() {
    let events = play_to_desk(r"\![updatebymyself,checkonly]\e");
    assert!(
        events_named(&events, "update_order_started").is_empty(),
        "依頼は 0 件: {events:?}"
    );
    assert_eq!(warns(&events), vec![Some("update_cue_refused")]);
}

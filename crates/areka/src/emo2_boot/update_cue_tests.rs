//! `UpdateCueSink` と `parse_update_command` の決定論テスト（要件 1.5〜1.8・9.4・10.8）。
//!
//! 確かめること: 入口の 4 つの形（`updatebymyself`・`update,all`・`update,shell+balloon`・
//! `updateother,--balloon=B,--shell=S`）が台本の文字列から期待の生の要求 1 件になること・
//! 更新オプション付き・`platform`・`--plugin=` だけ・`--option=` 混じりが要求 0 と `warn!` 1 件に
//! なること・`--shell=S,--plugin=P` が `P` を読み飛ばして `S` だけの要求になること・担当外は 0 件で
//! 警告しないこと・開けない自分宛の荷物と受信端の落ちた送出は `warn!` 1 件で台本を殺さないこと。

use super::*;
use std::sync::mpsc::{Receiver, channel};

use areka_ghost::BasewareRoot;
use areka_sakura::sysvar::SystemVarSnapshot;
use bevy_ecs::schedule::Schedules;
use bevy_ecs::world::World;
use dola::DynamicValue;
use dola::cue::{ActorKey, CueCommand, CuePlayer, CueSink, TalkCue};
use log_capture_kit::{CapturedEvent, capture};
use temp_path_kit::TempPath;

use crate::boot_config::{BootContext, ConfigInputs, CurrentGhost};
use crate::boot_resolve::{BalloonDecision, BalloonRoute, GhostDecision, GhostRoute};
use crate::ghost_session::{GhostSession, GhostSlot};
use crate::update::TargetKind::{self, Balloon, Ghost, Shell};
use crate::update::{self as update, desk};

// ---------------------------------------------------------------- 道具立て

/// `\![name,tokens...]` の汎用キャリア cue を組む。
fn carrier_cue(name: &str, tokens: &[&str]) -> TalkCue {
    TalkCue {
        at: 0.0,
        actor: ActorKey::from("0"),
        command: CueCommand::command_carrier(name, tokens.iter().map(|s| s.to_string()).collect()),
        duration: 0.0,
    }
}

fn sink() -> (UpdateCueSink, Receiver<RawUpdateRequest>) {
    let (tx, rx) = channel();
    (UpdateCueSink::new(tx), rx)
}

/// 台本の文字列を本番と同じ解析と組み立てで cue にし、受け口へ最後まで配る。
fn play_into(script: &str, sink: UpdateCueSink) {
    let instructions = areka_parsers::sakura::parse(script);
    let compiled = areka_sakura::compile(&instructions, &SystemVarSnapshot::default());
    let mut player = CuePlayer::from_sheet(&compiled.sheet);
    player.register_sink(Box::new(sink));
    player.tick(compiled.sheet.absolute_end_time());
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
    let master = root.ghost_dir("emo").join("ghost").join("master");
    std::fs::create_dir_all(&master).expect("検体のフォルダを作る");
    std::fs::write(master.join("descript.txt"), "charset,UTF-8\r\nname,Emo\r\n")
        .expect("descript.txt を書く");
    let mut world = World::new();
    world.init_resource::<Schedules>();
    update::register(&mut world);
    world.insert_resource(BootContext {
        root: root.clone(),
        app_profile_dir: tmp.path().join("profile"),
        helper_exe: tmp.path().join("helper.exe"),
        argv_session: false,
        current: CurrentGhost {
            cfg: ConfigInputs {
                ghost_root: root.ghost_dir("emo"),
                balloon_root: root.balloon_dir("kaku"),
            },
            ghost: GhostDecision {
                route: GhostRoute::Default,
                dir: root.ghost_dir("emo"),
                folder: Some("emo".to_owned()),
            },
            balloon: BalloonDecision {
                route: BalloonRoute::Default,
                dir: root.balloon_dir("kaku"),
                folder: Some("kaku".to_owned()),
            },
        },
    });
    let (kanade, _) = channel();
    world.insert_non_send(GhostSlot(Some(GhostSession::for_test(
        Some(kanade),
        root.ghost_dir("emo"),
    ))));
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

fn named<'a>(events: &'a [CapturedEvent], name: &str) -> Vec<&'a CapturedEvent> {
    events
        .iter()
        .filter(|e| e.field_str("event") == Some(name))
        .collect()
}

fn emit_one(name: &str, tokens: &[&str]) -> (Vec<RawUpdateRequest>, Vec<CapturedEvent>) {
    let (mut sink, rx) = sink();
    let ((), events) = capture(|| sink.emit(carrier_cue(name, tokens)));
    (rx.try_iter().collect(), events)
}

fn warns(events: &[CapturedEvent]) -> Vec<Option<&str>> {
    events
        .iter()
        .filter(|e| e.level == tracing::Level::WARN)
        .map(|e| e.field_str("event"))
        .collect()
}

fn other(pairs: &[(TargetKind, &str)]) -> RawUpdateRequest {
    RawUpdateRequest::Other(pairs.iter().map(|(k, n)| (*k, n.to_string())).collect())
}

// ---------------------------------------------------------------- 入口の 4 つの形

/// 4 つの入口の形が、台本の文字列から期待の生の要求 1 件になり、警告を残さない
/// （要件 1.5・1.6・1.7・9.4）。
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

/// `update` の対象は並んだ順（逆順・1 つだけも）で運び、重複はそのまま・`all` は 3 つに開く（要件 1.6）。
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

/// `--shell=S,--plugin=P` は `P` を `warn!` 1 件で読み飛ばし、`S` だけの要求になる（要件 1.7・9.4）。
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

/// オプション付き・`platform`・`--plugin=` だけ・`--option=` 混じりは要求 0 と `warn!` 1 件
/// （要件 1.8・9.4・10.8）。
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
        let ((), events) = capture(|| {
            sink.emit(TalkCue {
                at: 0.0,
                actor: ActorKey::from("0"),
                command: CueCommand::Custom {
                    command: name.into(),
                    params: DynamicValue::Null,
                },
                duration: 0.0,
            })
        });
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
/// 1 件受け付けられ、警告は無い（要件 1.1・1.5・1.6）。
///
/// 背景スレッドへ渡した依頼の中身は `update` の外から読めないので、受付が渡したときに残す
/// `info!(update_order_started)`（出どころと対象の名前）で判定する。
#[test]
fn script_string_reaches_the_desk_and_is_accepted() {
    let events = play_to_desk(r"\![update,ghost]\e");
    let started = named(&events, "update_order_started");
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
    assert!(warns(&events).is_empty(), "断りも警告もない: {events:?}");
}

/// 断りの形は窓口まで通しても依頼 0 件・`warn!` 1 件（受け口の断りだけ・要件 1.8・9.4）。
#[test]
fn refused_form_reaches_no_order() {
    let events = play_to_desk(r"\![updatebymyself,checkonly]\e");
    assert!(
        named(&events, "update_order_started").is_empty(),
        "依頼は 0 件: {events:?}"
    );
    assert_eq!(warns(&events), vec![Some("update_cue_refused")]);
}

//! `InstallCueSink` の自己選別と送り出しの決定論テスト。

use super::*;
use crate::session_tests::events_named;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, TryRecvError, channel};

use areka_kanade::online::OnlineCounter;
use areka_sakura::sysvar::SystemVarSnapshot;
use bevy_ecs::schedule::Schedules;
use bevy_ecs::world::World;
use dola::DynamicValue;
use dola::cue::{ActorKey, CueCommand, CuePlayer, CueSink, TalkCue};
use log_capture_kit::{CapturedEvent, capture};
use temp_path_kit::TempPath;

use areka_runtime::consumer_ledger::test_support::carrier_cue;

use crate::install::fetch_url::FetchTools;
use crate::install::fetch_url_test_support::FakeFetch;
use crate::install::{self, InstallOrder, InstallOrigin, desk};
use areka_runtime::spine::{Progress, wait_until};

// ---------------------------------------------------------------- 道具立て

/// 絶対パスの書庫（どのテストも fs を読まない＝在る必要はない）。
const ABSOLUTE: &str = r"C:\areka-install-cue-test\in\hana.nar";
/// `url` の腕の URL（偽の取得口だけが答える＝ネットへ出ない）。
const URL: &str = "https://example.com/files/hana.nar";
const BODY: &[u8] = b"PK\x03\x04 nar body";

/// 読み取りだけでない受け口（取りに行くのは窓口なので、受け口だけではネットへ出ない）。
fn sink() -> (InstallCueSink, Receiver<RawInstallRequest>) {
    let (tx, rx) = channel();
    (InstallCueSink::with_readonly(tx, false), rx)
}

/// 1 つの cue を流し、送られた要求と記録を返す。
fn emit_one(name: &str, tokens: &[&str]) -> (Vec<RawInstallRequest>, Vec<CapturedEvent>) {
    let (mut sink, rx) = sink();
    let ((), events) = capture(|| sink.emit(carrier_cue(name, tokens)));
    (rx.try_iter().collect(), events)
}

/// 台本の文字列を本番と同じ解析と組み立てで cue にし、受け口へ最後まで配る。
fn play_script(script: &str, sink: InstallCueSink) {
    let instructions = areka_parsers::sakura::parse(script);
    let compiled = areka_sakura::compile(&instructions, &SystemVarSnapshot::default());
    let mut player = CuePlayer::from_sheet(&compiled.sheet);
    player.register_sink(Box::new(sink));
    player.tick(compiled.sheet.absolute_end_time());
}

fn warns(events: &[CapturedEvent]) -> Vec<Option<&str>> {
    events
        .iter()
        .filter(|e| e.level == tracing::Level::WARN)
        .map(|e| e.field_str("event"))
        .collect()
}

fn script_raw(path: &str) -> RawInstallRequest {
    RawInstallRequest::Path {
        path: PathBuf::from(path),
        origin: InstallOrigin::Script,
    }
}

// ---------------------------------------------------------------- 受理

/// 台本の文字列の `\![execute,install,path,絶対パス]` は、パスを無変形で運ぶ出どころ「台本」の
/// 生の要求 1 件になる。
#[test]
fn script_string_becomes_one_script_request_with_the_path() {
    let (sink, rx) = sink();
    let ((), events) =
        capture(|| play_script(&format!(r"\![execute,install,path,{ABSOLUTE}]\e"), sink));
    assert_eq!(
        rx.try_iter().collect::<Vec<_>>(),
        vec![script_raw(ABSOLUTE)]
    );
    assert_eq!(warns(&events), Vec::<Option<&str>>::new(), "警告なし");
}

/// 台本の文字列から受け口 → 窓口の取り出し → 受付まで通すと、窓口の待ち行列に出どころ「台本」・
/// 書庫 1 本の依頼がちょうど 1 件入る。
#[test]
fn script_string_reaches_the_desk_queue_as_one_script_order() {
    let mut world = World::new();
    world.init_resource::<Schedules>();
    install::register(&mut world);
    desk::insert_busy_desk_for_test(&mut world);
    let sink = InstallCueSink::new(desk::raw_sender(&world));

    let ((), events) = capture(|| {
        play_script(&format!(r"\![execute,install,path,{ABSOLUTE}]\e"), sink);
        desk::drain(&mut world);
    });

    assert_eq!(
        install::queued_orders(&world),
        vec![InstallOrder {
            archives: vec![PathBuf::from(ABSOLUTE)],
            origin: InstallOrigin::Script,
        }]
    );
    assert_eq!(
        warns(&events),
        Vec::<Option<&str>>::new(),
        "断りも警告もない"
    );
}

/// パスより後ろの引数は読まず `warn!` を 1 件残す（要求は出す）。
#[test]
fn arguments_after_the_path_are_ignored_with_one_warning() {
    let (sent, events) = emit_one("execute", &["install", "path", ABSOLUTE, "--extra"]);
    assert_eq!(sent, vec![script_raw(ABSOLUTE)], "パスまでは読む");
    assert_eq!(warns(&events), vec![Some("install_cue_extra_ignored")]);
}

// ---------------------------------------------------------------- 断る

/// 相対パス・空・パスの欠けは `install_cue_bad_path` の `warn!` 1 件で送らない。
#[test]
fn relative_or_empty_path_warns_once_and_sends_nothing() {
    let cases: [&[&str]; 3] = [
        &["install", "path", r"Downloads\hana.nar"],
        &["install", "path", ""],
        &["install", "path"],
    ];
    for tokens in cases {
        let (sent, events) = emit_one("execute", tokens);
        assert!(sent.is_empty(), "{tokens:?}: 送らない");
        assert_eq!(
            warns(&events),
            vec![Some("install_cue_bad_path")],
            "{tokens:?}: 警告 1 件"
        );
    }
}

/// 台本の文字列の相対パスも、窓口まで通して依頼 0 件・`warn!` 1 件。
#[test]
fn relative_path_in_a_script_string_reaches_no_order() {
    let mut world = World::new();
    world.init_resource::<Schedules>();
    install::register(&mut world);
    let sink = InstallCueSink::new(desk::raw_sender(&world));

    let ((), events) = capture(|| {
        play_script(r"\![execute,install,path,in\hana.nar]\e", sink);
        desk::drain(&mut world);
    });

    assert!(
        events_named(&events, "install_order_queued").is_empty(),
        "依頼は 0 件: {events:?}"
    );
    assert_eq!(warns(&events), vec![Some("install_cue_bad_path")]);
}

/// 2 番目の引数が `path`／`url` 以外（大文字の `URL`・知らない語・無し）は `install_cue_unsupported`
/// の `warn!` 1 件で送らない。
#[test]
fn not_path_warns_once_and_sends_nothing() {
    let cases: [&[&str]; 3] = [
        &["install", "URL", "https://example.com/hana.nar"],
        &["install", "file", ABSOLUTE],
        &["install"],
    ];
    for tokens in cases {
        let (sent, events) = emit_one("execute", tokens);
        assert!(sent.is_empty(), "{tokens:?}: 送らない");
        assert_eq!(
            warns(&events),
            vec![Some("install_cue_unsupported")],
            "{tokens:?}: 警告 1 件"
        );
    }
}

// ---------------------------------------------------------------- url の腕

/// 台本の文字列の `\![execute,install,url,URL,nar]` は、受け口から窓口へ URL の要求として届き、
/// 窓口が偽の取得口を差した取得のスレッドで落とし終えると、出どころ「台本」の依頼がちょうど 1 件
/// 積まれる。取得を抜けた後は通信中の数が 0 に戻っている。
#[test]
fn url_arm_starts_one_fetch_and_one_script_request_arrives() {
    static ONLINE: OnlineCounter = OnlineCounter::new();
    let tmp = TempPath::new("install-cue-url");
    let dir = tmp.child("download");
    let mut world = World::new();
    desk::insert_busy_desk_for_test(&mut world);
    let fake = FakeFetch::new().serve(URL, BODY);
    desk::set_fetch_tools_for_test(
        &mut world,
        FetchTools {
            dir: dir.clone(),
            make: Arc::new(move || Ok(Box::new(fake.clone()) as Box<dyn areka_update::Fetch>)),
            online: &ONLINE,
        },
    );
    let sink = InstallCueSink::new(desk::raw_sender(&world));

    let ((), events) =
        capture(|| play_script(&format!(r"\![execute,install,url,{URL},nar]\e"), sink));
    wait_until(
        "取得した書庫が依頼になる",
        Progress::Unknown,
        || {
            desk::drain(&mut world);
            !install::queued_orders(&world).is_empty()
        },
    )
    .expect("依頼が積まれる");
    // 依頼は通信中の数を立てたまま送られるので、数が戻るのは送った後。
    wait_until("通信中の数が戻る", Progress::Unknown, || {
        !ONLINE.is_online()
    })
    .expect("取得を抜けたら数は 0");
    let orders = install::queued_orders(&world);
    let [order] = orders.as_slice() else {
        panic!("依頼は 1 件だけ: {orders:?}");
    };
    assert_eq!(order.origin, InstallOrigin::Script);
    let [path] = order.archives.as_slice() else {
        panic!("書庫は 1 本: {order:?}");
    };
    assert_eq!(path.parent(), Some(dir.as_path()));
    assert_eq!(std::fs::read(path).expect("読める"), BODY);
    assert_eq!(warns(&events), Vec::<Option<&str>>::new(), "警告なし");
}

/// 種別の省略も `nar` と同じく URL の要求 1 件になる。種別より後ろの引数は `warn!` 1 件を残して読まない。
#[test]
fn url_arm_with_no_kind_or_trailing_arguments_starts_the_fetch() {
    let cases: [(&[&str], Vec<Option<&str>>); 2] = [
        (&["install", "url", URL], vec![]),
        (
            &["install", "url", URL, "nar", "--extra"],
            vec![Some("install_cue_extra_ignored")],
        ),
    ];
    for (tokens, expected_warns) in cases {
        let (sent, events) = emit_one("execute", tokens);
        assert_eq!(
            sent,
            vec![RawInstallRequest::Url(URL.to_owned())],
            "{tokens:?}"
        );
        assert_eq!(warns(&events), expected_warns, "{tokens:?}");
    }
}

/// 空・`http://`／`https://` で始まらない URL は `install_cue_bad_url`、`nar` 以外の種別は
/// `install_cue_unsupported_kind` の `warn!` 1 件で、取得 0・依頼 0。
#[test]
fn url_refusals_warn_once_and_start_nothing() {
    let cases: [(&[&str], &str); 7] = [
        (&["install", "url", ""], "install_cue_bad_url"),
        (
            &["install", "url", "ftp://example.com/hana.nar"],
            "install_cue_bad_url",
        ),
        (
            &["install", "url", URL, "feed"],
            "install_cue_unsupported_kind",
        ),
        (
            &["install", "url", URL, "homeurl"],
            "install_cue_unsupported_kind",
        ),
        (
            &["install", "url", URL, "ical"],
            "install_cue_unsupported_kind",
        ),
        (
            &["install", "url", URL, "ssf"],
            "install_cue_unsupported_kind",
        ),
        (
            &["install", "url", URL, "zip"],
            "install_cue_unsupported_kind",
        ),
    ];
    for (tokens, expected) in cases {
        let (sent, events) = emit_one("execute", tokens);
        assert!(sent.is_empty(), "{tokens:?}: 取得も依頼も 0");
        assert_eq!(
            warns(&events),
            vec![Some(expected)],
            "{tokens:?}: 警告 1 件"
        );
    }
}

/// 読み取りだけの受け口の `url` の腕は、要求を送らず、`readonly_refused`（`what = install_url`）の
/// info を 1 件残す。受け口を落とすと受信端は閉じる＝送り物 0。
#[test]
fn readonly_fetch_refuses_the_url_without_sending_or_warning() {
    let (tx, rx) = channel();
    let mut sink = InstallCueSink::with_readonly(tx, true);
    let ((), events) =
        capture(|| sink.emit(carrier_cue("execute", &["install", "url", URL, "nar"])));
    drop(sink);

    assert_eq!(rx.try_recv(), Err(TryRecvError::Disconnected), "依頼 0");
    let refused: Vec<_> = events_named(&events, "readonly_refused");
    assert_eq!(refused.len(), 1, "断りの記録は 1 件");
    assert_eq!(refused[0].level, tracing::Level::INFO);
    assert_eq!(refused[0].field_str("what"), Some("install_url"));
    assert_eq!(refused[0].field_str("url"), Some(URL));
    assert_eq!(warns(&events), Vec::<Option<&str>>::new(), "警告なし");
}

// ---------------------------------------------------------------- 担当外

/// `execute` の他の第 1 引数・裸の `execute`・他の名前は 0 件で警告も出さない。
#[test]
fn not_ours_is_benign_skip_without_warning() {
    let cases: [(&str, &[&str]); 3] = [
        ("execute", &["http-get", "https://example.com"]),
        ("execute", &[]),
        ("open", &["install", "path", ABSOLUTE]),
    ];
    for (name, tokens) in cases {
        let (sent, events) = emit_one(name, tokens);
        assert!(sent.is_empty(), "{name} {tokens:?}: 担当外は送らない");
        assert!(warns(&events).is_empty(), "{name} {tokens:?}: 警告しない");
    }
}

/// 開封できない自分宛（`execute`）の荷物は警告を残し、送らない。
#[test]
fn unopenable_execute_warns_and_sends_nothing() {
    let (mut sink, rx) = sink();
    let ((), events) = capture(|| {
        sink.emit(TalkCue {
            at: 0.0,
            actor: ActorKey::from("0"),
            command: CueCommand::Custom {
                command: "execute".into(),
                params: DynamicValue::Null,
            },
            duration: 0.0,
        })
    });
    assert_eq!(rx.try_iter().count(), 0);
    assert_eq!(warns(&events), vec![Some("install_cue_unopenable")]);
}

// ---------------------------------------------------------------- 送出の失敗

/// 受信端が落ちていても落ちず、送れなかったことを記録する。
#[test]
fn dropped_receiver_is_logged_and_does_not_panic() {
    let (mut sink, rx) = sink();
    drop(rx);
    let ((), events) =
        capture(|| sink.emit(carrier_cue("execute", &["install", "path", ABSOLUTE])));
    assert_eq!(warns(&events), vec![Some("install_cue_send_failed")]);
}

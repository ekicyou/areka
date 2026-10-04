//! 翻訳（`OnTranslate`）の殻の結合テスト（areka-P0-translate-pipeline タスク 3.6・要件 3.6・4.5・
//! 5.3・5.4・5.6・8.1 ⑺）。
//!
//! 本物の殻（`spawn_kanade_translating`）に、SHIORI と再生側の両方の受け口を 1 本のスレッドで
//! 受ける偽物（[`spawn_rig`]）をつなぐ。偽物は SHIORI の要求を 1 件受けるたびに、その前に届いて
//! いた再生側の指示をすべて記録へ移してから要求を記録する。kanade は 1 本のスレッドで「再生の
//! 指示を送る → 次の往復を送る」の順に送るので、記録の並びは kanade が送った順と一致する
//! （再生の開始と、その後の SHIORI の要求の前後を 1 本の列で比べられる）。
//!
//! 偽物は最初の `OnTranslate` を、テストが放すまで握れる。握っている間は kanade が応答待ちで
//! 止まっている。放したときに偽物の受け口へ次の要求が積まれていないことを確かめてから応答する
//! （＝待ちの間に SHIORI へ何も送られていない・要件 5.3）。再生側は TalkDone を返さないので、
//! 始めた台詞はテストが TalkDone を送るまで再生中のまま（今日の「再生中」の規則で入力が処理される）。
//!
//! 見出し（要件 3.6）は、kanade が SHIORI へ渡す呼出（GET・ID・Reference・Status）までをこの層で
//! 見て、線の見出しは本物の組み立て（`shiori_host32_host::build_request`）に同じ呼出を通して比べる。
//! 本番の線の層（`shiori::real` の `handle_call` → `Shiori3Client::get` → `build_request`）は ID で
//! 分岐しないので、同じ形の呼出からは同じ見出しが出る。

use std::collections::HashMap;
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::thread;

use areka_kanade::{
    ChangeHandoff, ChangeOrigin, ChangeRequest, ChangeTarget, CloseReason, ExecutionSnapshot,
    KanadeConfig, KanadeMsg, KanadeNotice, KanadeStopCause, KanadeStopped, MonotonicMs,
    MouseButton, MouseEventKind, MouseInput, ShioriCall, ShioriFailure, ShioriFault, ShioriMethod,
    ShioriMsg, ShioriOutcome, TalkCommand, TalkDone, TalkEndReason, TalkId, TranslateSeams, events,
    spawn_kanade_translating,
};
use shiori_host32_host::{Charset, Method, ShioriRequest, build_request};

use super::common::{
    CallMethod, DEFAULT_TIMEOUT, RecordedCall, expected_call, expected_translate, expected_unload,
    join_bounded,
};

/// ダブルクリックが返す台詞。
const MOUSE_SCRIPT: &str = r"\0\s[0]なでなで\e";

/// 偽物が記録する 1 件（SHIORI の要求か、再生側への指示か）。
#[derive(Debug, Clone, PartialEq, Eq)]
enum Seen {
    Call(RecordedCall),
    /// 再生側への指示（`start:<talk_id>:<台詞>` など）。
    Talk(String),
}

/// 偽物の応答の決め方。
struct WireSpec {
    /// GET の ID → 返す台詞（無い ID は 204）。
    scripts: HashMap<&'static str, String>,
    /// `OnTranslate` が返す台詞（`None` なら 204）。
    translated: Option<String>,
    /// この ID の最初の GET に輸送路の失敗（`Ipc`）を返す。
    fail: Option<&'static str>,
    /// 最初の `OnTranslate` を放すまで握るか。
    hold: bool,
}

impl WireSpec {
    fn new(scripts: &[(&'static str, &str)]) -> Self {
        WireSpec {
            scripts: scripts.iter().map(|(id, s)| (*id, s.to_string())).collect(),
            translated: None,
            fail: None,
            hold: false,
        }
    }
}

/// 偽物を止めた後に返る記録。
struct WireLog {
    seen: Vec<Seen>,
    /// `OnTranslate` を握っている間に次の要求が積まれていた。
    sent_while_held: bool,
}

/// 偽物と kanade をつないだ一式。
struct Rig {
    kanade: Sender<KanadeMsg>,
    handle: areka_actor::ActorHandle,
    /// `OnTranslate` を握ったら 1 件届く。
    held: Receiver<()>,
    /// 送ると握った `OnTranslate` に応答する。
    release: Sender<()>,
    done: Receiver<WireLog>,
    notices: Receiver<KanadeNotice>,
}

fn describe(command: TalkCommand) -> String {
    match command {
        TalkCommand::Start(start) => format!("start:{}:{}", start.talk_id.0, start.script),
        TalkCommand::ResolveChoice { talk_id, id } => format!("resolve:{}:{id}", talk_id.0),
        TalkCommand::CancelChoice { talk_id } => format!("cancel:{}", talk_id.0),
    }
}

fn record(call: &ShioriCall) -> RecordedCall {
    let (method, id, references, status) = match call {
        ShioriCall::Get {
            id,
            references,
            status,
        } => (CallMethod::Get, id, references, status),
        ShioriCall::Notify {
            id,
            references,
            status,
        } => (CallMethod::Notify, id, references, status),
    };
    RecordedCall {
        method,
        id: id.as_str().to_string(),
        references: references.clone(),
        status: status.render(),
    }
}

/// 偽物を起こし、素通しの口つきの kanade（停止通知の受け口あり）をつなぐ。
fn spawn_rig(spec: WireSpec) -> Rig {
    let (shiori_tx, shiori_rx) = mpsc::channel::<ShioriMsg>();
    let (talk_tx, talk_rx) = mpsc::channel::<TalkCommand>();
    let (held_tx, held) = mpsc::channel::<()>();
    let (release, release_rx) = mpsc::channel::<()>();
    let (done_tx, done) = mpsc::channel::<WireLog>();
    thread::spawn(move || {
        let WireSpec {
            scripts,
            translated,
            mut fail,
            mut hold,
        } = spec;
        let mut seen = Vec::new();
        let mut sent_while_held = false;
        while let Ok(msg) = shiori_rx.recv() {
            // この要求より前に kanade が送った再生の指示を先に記録する。
            while let Ok(command) = talk_rx.try_recv() {
                seen.push(Seen::Talk(describe(command)));
            }
            match msg {
                ShioriMsg::Request { call, reply } => {
                    let recorded = record(&call);
                    let id = recorded.id.clone();
                    let outcome = if recorded.method == CallMethod::Notify {
                        ShioriOutcome::Notified
                    } else if fail == Some(id.as_str()) {
                        fail = None;
                        ShioriOutcome::Failed(ShioriFailure::Ipc("helper gone".into()))
                    } else if id == "OnTranslate" {
                        if std::mem::take(&mut hold) {
                            let _ = held_tx.send(());
                            let _ = release_rx.recv_timeout(DEFAULT_TIMEOUT);
                            sent_while_held =
                                !matches!(shiori_rx.try_recv(), Err(TryRecvError::Empty));
                        }
                        translated
                            .clone()
                            .map_or(ShioriOutcome::NoContent, ShioriOutcome::Value)
                    } else {
                        scripts
                            .get(id.as_str())
                            .cloned()
                            .map_or(ShioriOutcome::NoContent, ShioriOutcome::Value)
                    };
                    seen.push(Seen::Call(recorded));
                    let _ = reply.send(outcome);
                }
                ShioriMsg::Unload { reply } => {
                    seen.push(Seen::Call(expected_unload()));
                    let _ = reply.send(ShioriOutcome::Unloaded);
                }
                ShioriMsg::Close => break,
            }
        }
        // kanade が止まって送り手を落とすまでの指示を拾う。
        seen.extend(talk_rx.iter().map(|c| Seen::Talk(describe(c))));
        let _ = done_tx.send(WireLog {
            seen,
            sent_while_held,
        });
    });
    let (notice_tx, notices) = mpsc::channel::<KanadeNotice>();
    let (kanade, handle) = spawn_kanade_translating(
        KanadeConfig::new("master", "1.0.0"),
        shiori_tx,
        talk_tx,
        Box::new(|_, _| {}),
        Some(notice_tx),
        TranslateSeams::passthrough(),
    );
    Rig {
        kanade,
        handle,
        held,
        release,
        done,
        notices,
    }
}

impl Rig {
    fn send(&self, msg: KanadeMsg) {
        self.kanade.send(msg).expect("kanade inbox");
    }

    /// 握った `OnTranslate` が偽物へ届くのを待つ。
    fn wait_held(&self) {
        self.held
            .recv_timeout(DEFAULT_TIMEOUT)
            .expect("OnTranslate が偽物へ届いて握られる");
    }

    fn release(&self) {
        self.release.send(()).expect("偽物は握ったまま待っている");
    }

    /// kanade を止めて（まだ動いていれば `Close`）、記録と停止通知を返す。
    fn finish(self) -> (WireLog, Option<KanadeStopped>) {
        let _ = self.kanade.send(KanadeMsg::Close);
        drop(self.kanade);
        join_bounded("kanade translate join", DEFAULT_TIMEOUT, self.handle)
            .expect("kanade は期限内に止まる");
        let log = self
            .done
            .recv_timeout(DEFAULT_TIMEOUT)
            .expect("偽物は kanade が止まった後に記録を返す");
        let stopped = self.notices.try_iter().find_map(|n| match n {
            KanadeNotice::Stopped(stopped) => Some(stopped),
            _ => None,
        });
        (log, stopped)
    }
}

/// 再生中の状態（`Status: talking`）。
fn talking() -> ExecutionSnapshot {
    ExecutionSnapshot {
        talk_active: true,
        ..ExecutionSnapshot::INACTIVE
    }
}

fn double_click() -> KanadeMsg {
    KanadeMsg::Mouse(MouseInput {
        scope: 0,
        x: 10,
        y: 20,
        region: Some("Head".to_string()),
        kind: MouseEventKind::DoubleClick {
            button: MouseButton::Left,
        },
    })
}

/// [`double_click`] が送る GET（`snapshot` はそのときの状態）。
fn double_click_call(snapshot: &ExecutionSnapshot) -> ShioriCall {
    events::on_mouse_double_click(10, 20, 0, Some("Head"), MouseButton::Left, snapshot)
}

fn mouse_move() -> KanadeMsg {
    KanadeMsg::Mouse(MouseInput {
        scope: 0,
        x: 11,
        y: 21,
        region: None,
        kind: MouseEventKind::Move,
    })
}

fn change_request() -> ChangeRequest {
    ChangeRequest {
        target: ChangeTarget {
            sakura_name: "次".to_string(),
            name: "next".to_string(),
            dir: r"C:\ghost\next".to_string(),
        },
        origin: ChangeOrigin::Manual,
        raise_event: true,
    }
}

fn talk_done(id: u64) -> KanadeMsg {
    KanadeMsg::TalkDone(TalkDone {
        talk_id: TalkId(id),
        reason: TalkEndReason::Ended,
        quit_reserved: false,
    })
}

fn start(id: u64, script: &str) -> Seen {
    Seen::Talk(format!("start:{id}:{script}"))
}

fn call(call: ShioriCall) -> Seen {
    Seen::Call(expected_call(call))
}

/// 起動（すべて 204）の後、`id` の GET からの記録。
fn from_first<'a>(seen: &'a [Seen], id: &str) -> &'a [Seen] {
    let at = seen
        .iter()
        .position(|s| matches!(s, Seen::Call(c) if c.id == id))
        .unwrap_or_else(|| panic!("{id} が記録に無い: {seen:#?}"));
    &seen[at..]
}

fn calls_of<'a>(seen: &'a [Seen], id: &str) -> Vec<&'a RecordedCall> {
    seen.iter()
        .filter_map(|s| match s {
            Seen::Call(c) if c.id == id => Some(c),
            _ => None,
        })
        .collect()
}

/// 待ちの間に届いた入力の 1 行（名前, 握っている間に送る入力, 放した後に期待する列の続き）。
type WaitCase = (&'static str, fn() -> Vec<KanadeMsg>, fn() -> Vec<Seen>);

/// 要件 5.3・5.4・8.1 ⑺: `OnTranslate` を握っている間にマウス・毎秒の時刻・終了の要求・切替の
/// 要求・外からの依頼を送る。握っている間は SHIORI へ何も送られず、放した後に再生の開始が先に
/// 出て、その後で各入力が今日の「再生中」の規則で処理される。
///
/// 今日の「再生中」の規則: マウスと外からの依頼は `Status: talking` の GET、毎秒の時刻は
/// Ref3=0 の NOTIFY。終了と切替の要求は台詞が終わるまで保留され、TalkDone の後で握手に入る
/// （終了は `OnClose`、切替は `OnGhostChanging` → 切替の `OnClose`）。
#[test]
fn inputs_during_translate_wait_run_after_start_under_talking_rules() {
    let cases: [WaitCase; 5] = [
        (
            "mouse",
            || vec![mouse_move()],
            || vec![call(events::on_mouse_move(11, 21, 0, None, &talking()))],
        ),
        (
            "tick",
            || {
                vec![KanadeMsg::Tick {
                    now: MonotonicMs(1_000),
                }]
            },
            || {
                vec![call(events::on_second_change(
                    MonotonicMs(1_000),
                    &talking(),
                ))]
            },
        ),
        (
            "close",
            || {
                let reason = CloseReason::User { scope: 0 };
                vec![KanadeMsg::CloseRequest { reason }, talk_done(1)]
            },
            || {
                let reason = CloseReason::User { scope: 0 };
                let idle = ExecutionSnapshot::INACTIVE;
                vec![
                    call(events::on_close(reason, &idle)),
                    Seen::Call(expected_unload()),
                ]
            },
        ),
        (
            "change",
            || vec![KanadeMsg::ChangeGhost(change_request()), talk_done(1)],
            || {
                let idle = ExecutionSnapshot::INACTIVE;
                vec![
                    call(events::on_ghost_changing(&change_request(), &idle)),
                    call(events::on_close(CloseReason::System, &idle)),
                    Seen::Call(expected_unload()),
                ]
            },
        ),
        (
            "raise",
            || {
                vec![KanadeMsg::RaiseEvent {
                    id: "OnInstallBegin".to_string(),
                    references: vec!["x".to_string()],
                    method: ShioriMethod::Get,
                    reply: None,
                }]
            },
            || {
                let refs = vec!["x".to_string()];
                vec![call(events::raise(
                    "OnInstallBegin",
                    refs,
                    ShioriMethod::Get,
                    &talking(),
                ))]
            },
        ),
    ];
    for (name, inputs, tail) in cases {
        let mut spec = WireSpec::new(&[("OnMouseDoubleClick", MOUSE_SCRIPT)]);
        spec.hold = true;
        let rig = spawn_rig(spec);
        rig.send(KanadeMsg::Boot);
        rig.send(double_click());
        rig.wait_held();
        for msg in inputs() {
            rig.send(msg);
        }
        rig.release();
        let (log, _) = rig.finish();

        assert!(
            !log.sent_while_held,
            "{name}: 握っている間に SHIORI へ要求が積まれた"
        );
        let source = double_click_call(&ExecutionSnapshot::INACTIVE);
        let mut expected = vec![
            call(double_click_call(&ExecutionSnapshot::INACTIVE)),
            Seen::Call(expected_translate(MOUSE_SCRIPT, &source, &talking())),
            start(1, MOUSE_SCRIPT),
        ];
        expected.extend(tail());
        assert_eq!(
            from_first(&log.seen, "OnMouseDoubleClick"),
            expected.as_slice(),
            "{name}: 再生の開始が先・その後に今日の「再生中」の規則で処理される"
        );
    }
}

/// 要件 5.6・8.1 ⑷: 台詞 1 つにつき `OnTranslate` は 1 回だけ。204 の元のイベント（起動・毎秒の
/// 時刻）と中身が空の 200（マウス移動）では 0 回。
#[test]
fn one_on_translate_per_script_and_none_for_no_content_or_empty() {
    let rig = spawn_rig(WireSpec::new(&[
        ("OnMouseDoubleClick", MOUSE_SCRIPT),
        ("OnMouseMove", ""),
    ]));
    rig.send(KanadeMsg::Boot);
    rig.send(KanadeMsg::Tick {
        now: MonotonicMs(1_000),
    });
    rig.send(mouse_move());
    rig.send(talk_done(1)); // 空の台詞の再生（talk 1）を終えて定常へ戻す。
    rig.send(double_click());
    rig.send(double_click()); // 再生中のダブルクリックの台詞は今の台詞を置き換える（2 つ目の台詞）。
    let (log, _) = rig.finish();

    let gets: Vec<&str> = log
        .seen
        .iter()
        .filter_map(|s| match s {
            Seen::Call(c) if c.method == CallMethod::Get => Some(c.id.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(
        gets,
        [
            "username",
            "OnFirstBoot",
            "OnBoot",
            "OnSecondChange",
            "OnMouseMove",
            "OnMouseDoubleClick",
            "OnTranslate",
            "OnMouseDoubleClick",
            "OnTranslate",
        ],
        "OnTranslate は台詞を返した GET の直後に 1 回ずつだけ: {:#?}",
        log.seen
    );
    let starts: Vec<&Seen> = log
        .seen
        .iter()
        .filter(|s| matches!(s, Seen::Talk(_)))
        .collect();
    assert_eq!(
        starts,
        [
            &start(1, ""),
            &start(2, MOUSE_SCRIPT),
            &start(3, MOUSE_SCRIPT)
        ],
        "空の台詞は翻訳せずに再生し、台詞を返した 2 回はそれぞれ翻訳の後に再生する"
    );
}

/// 線の見出し（要求行・値を除いた見出しの名前の並び・Status の行）。Reference の番号の行は
/// 中身がイベントごとに違うので名前の並びから除く。
fn header_shape(recorded: &RecordedCall) -> Vec<String> {
    let encoded = build_request(&ShioriRequest {
        method: Method::Get,
        id: &recorded.id,
        references: &recorded.references,
        sender: "areka",
        status: recorded.status.as_deref(),
        charset: Charset::UTF_8,
    });
    let text = String::from_utf8(encoded.bytes).expect("UTF-8");
    text.split("\r\n")
        .filter(|line| !line.starts_with("Reference"))
        .map(|line| match line.split_once(": ") {
            Some(("Status", _)) => line.to_string(),
            Some((name, _)) => name.to_string(),
            None => line.to_string(),
        })
        .collect()
}

/// 要件 3.6・設計の論点 9: `OnTranslate` の Status は再生を始める時点の状態（`talking` を含む）で
/// 届く。定常（再生なし）から始まる台詞では元の GET に Status の行が無くても `OnTranslate` は
/// `talking` を持つ。見出しは同じ時点に送った他の GET（再生中のダブルクリック）と同じ。
#[test]
fn on_translate_status_is_the_playback_start_state_and_headers_match_other_gets() {
    let rig = spawn_rig(WireSpec::new(&[("OnMouseDoubleClick", MOUSE_SCRIPT)]));
    rig.send(KanadeMsg::Boot);
    rig.send(double_click());
    rig.send(double_click());
    let (log, _) = rig.finish();

    let sources = calls_of(&log.seen, "OnMouseDoubleClick");
    let translates = calls_of(&log.seen, "OnTranslate");
    assert_eq!((sources.len(), translates.len()), (2, 2), "{:#?}", log.seen);
    assert_eq!(
        sources[0].status, None,
        "定常から送る元の GET に Status の行は無い"
    );
    let first_source = double_click_call(&ExecutionSnapshot::INACTIVE);
    assert_eq!(
        *translates[0],
        expected_translate(MOUSE_SCRIPT, &first_source, &talking()),
        "Status は再生を始める時点の状態（talking）"
    );
    assert_eq!(translates[0].status.as_deref(), Some("talking"));

    // 再生中に送った 2 つ目の元の GET と、その台詞の OnTranslate の見出しを比べる。
    assert_eq!(*sources[1], expected_call(double_click_call(&talking())));
    assert_eq!(header_shape(translates[1]), header_shape(sources[1]));
    assert_eq!(
        header_shape(translates[1]),
        [
            "GET SHIORI/3.0",
            "Charset",
            "Sender",
            "Status: talking",
            "ID",
            "SecurityLevel",
            "",
            "",
        ],
        "見出しの並びは他の GET と同じ"
    );
}

/// 停止通知と、失敗した GET から後の記録（`fail` の ID の GET を輸送路の失敗にする）。
fn run_failing(fail: &'static str, inputs: Vec<KanadeMsg>) -> (Vec<Seen>, Option<KanadeStopped>) {
    let mut spec = WireSpec::new(&[
        ("OnMouseDoubleClick", MOUSE_SCRIPT),
        ("OnGhostChanging", r"\0送り出し\e"),
    ]);
    spec.fail = Some(fail);
    let rig = spawn_rig(spec);
    rig.send(KanadeMsg::Boot);
    for msg in inputs {
        rig.send(msg);
    }
    let (log, stopped) = rig.finish();
    (from_first(&log.seen, fail).to_vec(), stopped)
}

/// 要件 4.5: `OnTranslate` の輸送路の失敗で止まるときの停止の原因と後の運びは、元のイベントの
/// GET が同じ輸送路の失敗をした今日の故障と同じ（再生は始まらず、降ろして止まる）。
#[test]
fn transport_failure_on_translate_stops_like_todays_fault() {
    let (after_translate, stopped) = run_failing("OnTranslate", vec![double_click()]);
    let (after_source, today) = run_failing("OnMouseDoubleClick", vec![double_click()]);

    let fault = KanadeStopCause::Fault(ShioriFault::from_failure(&ShioriFailure::Ipc(
        "helper gone".into(),
    )));
    let expected = Some(KanadeStopped {
        cause: fault,
        handoff: None,
    });
    assert_eq!(stopped, expected, "OnTranslate の輸送路の失敗は故障");
    assert_eq!(stopped, today, "停止の原因は元の GET の故障と同じ");
    assert_eq!(after_translate[1..], [Seen::Call(expected_unload())]);
    assert_eq!(
        after_source[1..],
        after_translate[1..],
        "再生を始めずに降ろす"
    );
}

/// 3.5 の申し送り（要件 6.2・4.5）: 停止通知の切替の中身は、`OnGhostChanging` の台詞なら翻訳の
/// 後の台詞を運ぶ。切替の送り出しの翻訳で輸送路が失敗したら、中身は空（`None`）で、原因は
/// `OnGhostChanging` そのものが同じ失敗をした今日の故障と同じ。
#[test]
fn change_handoff_carries_translated_script_or_none_on_transport_failure() {
    let translated = r"\0訳した送り出し\e";
    let mut spec = WireSpec::new(&[("OnGhostChanging", r"\0送り出し\e")]);
    spec.translated = Some(translated.to_string());
    let rig = spawn_rig(spec);
    rig.send(KanadeMsg::Boot);
    rig.send(KanadeMsg::ChangeGhost(change_request()));
    rig.send(talk_done(1));
    let (log, stopped) = rig.finish();
    assert!(
        log.seen.contains(&start(1, translated)),
        "再生するのは翻訳の後の台詞: {:#?}",
        log.seen
    );
    assert_eq!(
        stopped.map(|s| s.handoff),
        Some(Some(ChangeHandoff {
            script: Some(translated.to_string()),
        })),
        "停止通知の切替の中身は翻訳の後の台詞"
    );

    let change = || vec![KanadeMsg::ChangeGhost(change_request())];
    let (after_translate, stopped) = run_failing("OnTranslate", change());
    let (_, today) = run_failing("OnGhostChanging", change());
    let fault = KanadeStopCause::Fault(ShioriFault::from_failure(&ShioriFailure::Ipc(
        "helper gone".into(),
    )));
    assert_eq!(
        stopped,
        Some(KanadeStopped {
            cause: fault,
            handoff: Some(ChangeHandoff { script: None }),
        }),
        "表示しなかった送り出しの台詞は次のゴーストへ渡さない"
    );
    assert_eq!(stopped, today, "原因と中身は OnGhostChanging の故障と同じ");
    assert_eq!(after_translate[1..], [Seen::Call(expected_unload())]);
}

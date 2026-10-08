//! 時計の先行の檻（areka-P0-ghost-session-test-load-flake タスク 3.3・檻 8・要件 2.6）。
//!
//! 足場の時刻の注入（[`SwitchRig::inject_talk_tick`]）の送り先が dispatcher の `Tick` だけであることを、
//! 相手が進めない間に時計だけが先へ行く場面で固定する。A の台本で B への切替を始め、切替の握手の
//! `GET OnGhostChanging` を [`Stall`] で止める（相手が進めない状態を、時間でなく固まりで作る）。止まって
//! いる間に足場の時計を kanade の合成の締切（`close_talk_deadline_ms` ＝ 30,000 ms）の 3 倍以上進め、
//! 解いた後に B が定常に着くこと・A と B の呼び出しの並びが止めない回と同じであること・error の記録が
//! 0 件であることを見る。
//!
//! # 締切の error（`change_deadline_exceeded`）をどう判定するか
//! その `error!` は kanade のスレッドで出るので、呼び手のスレッドだけを捕える `log_capture_kit::capture`
//! には映らない（全スレッドの捕捉は、このテストバイナリの全テストの記録を溜め続けるので使わない＝
//! `session_end_deadline_tests.rs` と同じ判断）。代わりに締切が切れた証を呼び出しの記録で見る: 締切が
//! 切れると kanade は送り出しの台詞の完了を待たずに UNLOAD する。解いた直後、足場の時計をそれ以上
//! 進める前に A の kanade へ状態の問い合わせを送り、その返事（または kanade が止まって返事が落ちたこと）を
//! 待つ。kanade は受け口を届いた順に捌くので、返事の時点で、止まっていた間に届いていた入力は全部
//! 捌き終えている。台詞の時計は 1 つも進んでいないので、そこで A に UNLOAD があれば締切が切れている。
//! 呼び手のスレッドで出る error（切替の失敗など）は捕捉窓で数える。
//!
//! # 非空虚性
//! タスク 3.3 で変異を入れて確かめた。時刻の注入が kanade へも `KanadeMsg::Tick` を送るようにすると、
//! 定常の A が `OnSecondChange` を送って（台本に無い）落ち、止めない回から赤になる。止まっている間の
//! 時刻だけが kanade へ届く変異では、呼び出しの並びも捕捉窓の error も止めない回と同じまま、
//! 「解いた直後に UNLOAD していた」だけが赤になる（締切が切れたことを見ているのはこの観測だけ）。

use std::path::absolute;

use areka_kanade::{KanadeConfig, KanadeMsg};
use log_capture_kit::capture;
use wintf::ecs::widget::bitmap_source::WintfTaskPool;

use super::{FakeShiori, Stall, SwitchRig, TICK_STEP_MS, standard_script};
use crate::emo2_boot::ghost_switch::SwitchInFlight;
use crate::emo2_boot::spine::RecordedCall;
use crate::ghost_session::GhostSlot;

/// A の `OnBoot` の台本: 台詞の途中で B への切替（`OnGhostChanging` を送らせる）を命じる。
const A_TO_B: &str = "\\0A\\![change,ghost,B,--option=raise-event]\\e";
/// A の `OnGhostChanging` が返す送り出しの台本。
const A_SEND_OFF: &str = "\\0Bへ交代します\\e";
/// 止める切替の握手の GET。
const HANDSHAKE: &str = "OnGhostChanging";

/// 1 周の観測。
#[derive(Debug, PartialEq, Eq)]
struct Lap {
    /// A を起こして定常に着いたか。
    a_steady: bool,
    /// （止めた回だけ）止まる GET に入ったか。
    stalled: Option<bool>,
    /// （止めた回だけ）解いた直後、足場の時計を進める前に A が UNLOAD していたか（締切が切れた証）。
    unloaded_before_talk: Option<bool>,
    /// B が定常に着き、切替の予約が下りたか。
    b_welcomed: bool,
    /// A の呼び出しの並び（根のパスは `<root>` に置き換える）。
    a_calls: Vec<Vec<String>>,
    /// B の呼び出しの並び。
    b_calls: Vec<Vec<String>>,
    /// 呼び手のスレッドで出た error の記録。
    errors: Vec<String>,
    /// 降ろせたか。
    shutdown_ok: bool,
}

/// 呼び出しを「種類 名前 [Reference]」の 1 行へ写す（根のパスは回ごとに違うので `<root>` に置き換える）。
fn line(call: &RecordedCall, root: &str) -> String {
    let (kind, id, refs) = match call {
        RecordedCall::Get { id, references } => ("GET", id.as_str(), references.as_slice()),
        RecordedCall::Notify { id, references } => ("NOTIFY", id.as_str(), references.as_slice()),
        RecordedCall::Unload => ("UNLOAD", "", &[][..]),
        RecordedCall::Status => ("STATUS", "", &[][..]),
    };
    format!("{kind} {id} {:?}", refs.join("|").replace(root, "<root>"))
}

fn lines(rig: &SwitchRig, folder: &str) -> Vec<Vec<String>> {
    let root = absolute(rig.root.dir())
        .expect("根の絶対パス")
        .display()
        .to_string();
    rig.calls(folder)
        .iter()
        .map(|calls| calls.iter().map(|c| line(c, &root)).collect())
        .collect()
}

/// A の切替の 1 周。`stall` があれば握手の GET を止め、止まっている間に足場の時計を `advance_ms` 進めてから解く。
fn lap(stall: Option<Stall>, advance_ms: u64) -> Lap {
    let script = || {
        standard_script(A_TO_B)
            .get(HANDSHAKE, Ok(Some(A_SEND_OFF.to_owned())))
            .get("OnGhostChanged", Ok(None))
    };
    let a = match &stall {
        Some(stall) => FakeShiori::ScriptedStalled(Box::new(script), stall.clone()),
        None => FakeShiori::Scripted(Box::new(script)),
    };
    let b = FakeShiori::Scripted(Box::new(|| {
        standard_script("\\0B\\e").get("OnGhostChanged", Ok(None))
    }));
    let mut rig = SwitchRig::new(vec![("A", a), ("B", b)]);
    // 切替先の窓の準備が閉包を投函する先（`Input` の段に作業プールの取り出しの系は無いので走らない）。
    rig.world.insert_resource(WintfTaskPool::with_threads(1));
    rig.plant_boot_record("A");
    rig.plant_boot_record("B");

    let (mut lap, events) = capture(|| {
        rig.boot("A");
        let a_steady = rig.wait_steady();
        let (stalled, unloaded_before_talk) = match &stall {
            Some(stall) => {
                let stalled = rig.pump_talking_until(|rig| rig.exit_requested() || stall.entered());
                for _ in 0..advance_ms / TICK_STEP_MS {
                    rig.inject_talk_tick();
                }
                stall.release();
                (Some(stalled), Some(drained_then_unloaded(&rig)))
            }
            None => (None, None),
        };
        let b_welcomed = rig.pump_talking_until(|rig| {
            rig.exit_requested()
                || (!rig.calls("B").is_empty()
                    && rig.world.get_non_send::<SwitchInFlight>().is_none())
        }) && !rig.exit_requested();
        Lap {
            a_steady,
            stalled,
            unloaded_before_talk,
            b_welcomed,
            a_calls: lines(&rig, "A"),
            b_calls: lines(&rig, "B"),
            errors: Vec::new(),
            shutdown_ok: false,
        }
    });
    lap.errors = events
        .iter()
        .filter(|e| e.level == tracing::Level::ERROR)
        .map(|e| format!("{e:?}"))
        .collect();
    lap.shutdown_ok = rig.shutdown();
    lap
}

/// 置き場のゴースト（A）の kanade へ状態の問い合わせを送り、返事か、kanade が止まって返事が落ちたことを
/// 待ってから、A が UNLOAD していたかを返す（受け口は届いた順に捌かれる＝それまでに届いていた入力は
/// 捌き終えている）。
fn drained_then_unloaded(rig: &SwitchRig) -> bool {
    let kanade = rig
        .world
        .get_non_send::<GhostSlot>()
        .and_then(|slot| slot.0.as_ref())
        .and_then(|session| session.kanade().cloned())
        .expect("置き場に A の kanade がある");
    let (reply, rx) = areka_actor::reply_channel();
    if kanade.send(KanadeMsg::StatusQuery { reply }).is_ok() {
        assert!(
            rig.wait_for(|| !matches!(rx.try_recv(), Ok(None))),
            "A の kanade が状態の問い合わせに答えない"
        );
    }
    rig.calls("A")
        .last()
        .is_some_and(|calls| calls.contains(&RecordedCall::Unload))
}

/// 檻 8: 握手の GET を止めている間に足場の時計を締切の 3 倍以上進めても、解いた後の 1 周は止めない回と
/// 同じに終わる（B が定常に着く・呼び出しの並びが同じ・締切が切れない・error 0 件）。
#[test]
fn a_rig_clock_far_ahead_of_a_stalled_handshake_leaves_the_switch_unchanged() {
    let deadline_ms = KanadeConfig::new("", "").close_talk_deadline_ms;
    let advance_ms = deadline_ms * 3 + TICK_STEP_MS;
    let free = lap(None, 0);
    let held = lap(Some(Stall::at(HANDSHAKE)), advance_ms);

    assert_eq!(
        (
            free.a_steady,
            free.b_welcomed,
            free.errors.len(),
            free.shutdown_ok
        ),
        (true, true, 0, true),
        "止めない回が 1 周しない（前提）: {free:?}"
    );
    assert!(
        free.a_calls
            .iter()
            .flatten()
            .any(|c| c.starts_with(&format!("GET {HANDSHAKE} "))),
        "止めない回に握手の GET が無い（前提）: {free:?}"
    );
    assert_eq!(
        held,
        Lap {
            a_steady: true,
            stalled: Some(true),
            unloaded_before_talk: Some(false),
            b_welcomed: true,
            a_calls: free.a_calls.clone(),
            b_calls: free.b_calls.clone(),
            errors: Vec::new(),
            shutdown_ok: true,
        },
        "足場の時計を {advance_ms} ms 先へ進めた回が、止めない回と違う（締切 {deadline_ms} ms）"
    );
}

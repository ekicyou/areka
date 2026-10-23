//! drain フェーズ（[`drain_present_commands`]）の檻（areka-P0-pasta-in-windows-verification
//! task 5.6・要件 6.1）。
//!
//! rerun2（2026-10-23）で、送り手（seriko の ShowSurface・動く WebP で毎秒約 14 件）が送り続ける
//! あいだ drain が尽きず、1 フレームが終わらずに areka が答えなくなった。その再発を止める。

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use areka_actor::reply_channel;
use areka_emo_compose::{BindSet, PatternState};
use areka_emo_present::{PresentCommand, PresentOutcome, TargetId};

use super::super::test_support::{capture_logs, count_level};
use super::{MAX_DRAIN_PER_FRAME, drain_present_commands};

fn show(target: u32, surface_id: u32) -> PresentCommand {
    PresentCommand::ShowSurface {
        target: TargetId(target),
        surface_id,
        binds: BindSet::default(),
        pattern: PatternState::default(),
        reply: None,
    }
}

fn hide(target: u32) -> PresentCommand {
    PresentCommand::Hide {
        target: TargetId(target),
        reply: None,
    }
}

/// 適用された指令を、比べやすい形へ写す（`reply` は有無だけ）。
#[derive(Debug, PartialEq, Eq)]
enum Applied {
    Show {
        target: u32,
        surface_id: u32,
        reply: bool,
    },
    Hide {
        target: u32,
    },
    StageAck {
        target: u32,
        generation: u64,
    },
    Other,
}

fn record(cmd: &PresentCommand) -> Applied {
    match cmd {
        PresentCommand::ShowSurface {
            target,
            surface_id,
            reply,
            ..
        } => Applied::Show {
            target: target.0,
            surface_id: *surface_id,
            reply: reply.is_some(),
        },
        PresentCommand::Hide { target, .. } => Applied::Hide { target: target.0 },
        PresentCommand::StageAck { target, generation } => Applied::StageAck {
            target: target.0,
            generation: *generation,
        },
        _ => Applied::Other,
    }
}

/// ① 送り手が適用より速く送り続けても、drain は戻る（フレームが必ず終わる）。
///
/// 旧実装（`try_iter` を回しながら適用）は、適用 1 回が送りの間隔より長いと尽きずに戻らない。
/// 走者を別スレッドに置き、有限の待ちで「戻らない」を失敗として観測する（テストを固めない）。
#[test]
fn drain_returns_while_the_sender_keeps_sending() {
    let (tx, rx) = mpsc::channel::<PresentCommand>();
    // 届いている分を作っておく（drain が適用する物がある）。
    for n in 0..8 {
        tx.send(show(0, n)).expect("受信端は生きている");
    }
    let stop = Arc::new(AtomicBool::new(false));
    let sender = {
        let stop = Arc::clone(&stop);
        thread::spawn(move || {
            let mut n = 100;
            while !stop.load(Ordering::Relaxed) {
                if tx.send(show(0, n)).is_err() {
                    break;
                }
                n += 1;
                thread::sleep(Duration::from_millis(1));
            }
        })
    };
    let (done_tx, done_rx) = mpsc::channel::<usize>();
    thread::spawn(move || {
        let mut applied = 0usize;
        // 適用 1 回を送りの間隔より重くする（デバッグ版の合成と D2D の確保に当たる）。
        drain_present_commands(&rx, |_cmd| {
            applied += 1;
            thread::sleep(Duration::from_millis(10));
        });
        let _ = done_tx.send(applied);
    });
    let outcome = done_rx.recv_timeout(Duration::from_secs(3));
    stop.store(true, Ordering::Relaxed);
    sender.join().expect("送り手のスレッドが panic した");
    let applied =
        outcome.expect("送り手が送り続けるあいだ drain が戻らない（1 フレームが終わらない）");
    assert_eq!(
        applied, 1,
        "同じ target へ続く reply の無い ShowSurface は最後の 1 件だけ適用する"
    );
}

/// ② 同じ target へ続く reply の無い ShowSurface は最後の 1 件だけ適用し、ほかの指令の順番と
/// `reply` は保つ。ほかの指令（Hide・StageAck ほか）は区切りで、それをまたいで畳まない。
#[test]
fn consecutive_shows_to_one_target_collapse_to_the_last_and_keep_other_commands_in_order() {
    let (tx, rx) = mpsc::channel::<PresentCommand>();
    let (reply_tx, _reply_rx) = reply_channel::<PresentOutcome>();
    let sends = vec![
        show(0, 1),
        show(2, 50),
        show(0, 2),
        PresentCommand::ShowSurface {
            target: TargetId(0),
            surface_id: 3,
            binds: BindSet::default(),
            pattern: PatternState::default(),
            reply: Some(reply_tx),
        },
        show(0, 4),
        hide(0),
        show(0, 5),
        show(0, 6),
        PresentCommand::StageAck {
            target: TargetId(1),
            generation: 7,
        },
        show(1, 8),
        show(1, 9),
    ];
    for cmd in sends {
        tx.send(cmd).expect("受信端は生きている");
    }
    let mut applied = Vec::new();
    drain_present_commands(&rx, |cmd| applied.push(record(&cmd)));
    assert_eq!(
        applied,
        vec![
            Applied::Show {
                target: 2,
                surface_id: 50,
                reply: false
            },
            Applied::Show {
                target: 0,
                surface_id: 3,
                reply: true
            },
            Applied::Show {
                target: 0,
                surface_id: 4,
                reply: false
            },
            Applied::Hide { target: 0 },
            Applied::Show {
                target: 0,
                surface_id: 6,
                reply: false
            },
            Applied::StageAck {
                target: 1,
                generation: 7
            },
            Applied::Show {
                target: 1,
                surface_id: 9,
                reply: false
            },
        ]
    );
}

/// ③ 1 フレームに取り出すのは上限まで。残りは捨てずにチャネルに残し、次のフレームが順に
/// 取り出す。上限に当たったことはログに残す（記録の無い道を作らない）。
#[test]
fn drain_takes_at_most_the_cap_and_leaves_the_rest_in_order_with_a_warning() {
    let (tx, rx) = mpsc::channel::<PresentCommand>();
    let total = MAX_DRAIN_PER_FRAME + 5;
    for t in 0..total as u32 {
        tx.send(hide(t)).expect("受信端は生きている");
    }
    let mut first = Vec::new();
    let logs = capture_logs(|| drain_present_commands(&rx, |cmd| first.push(record(&cmd))));
    assert_eq!(first.len(), MAX_DRAIN_PER_FRAME);
    assert_eq!(count_level(&logs, "WARN"), 1, "{logs:?}");
    let mut second = Vec::new();
    drain_present_commands(&rx, |cmd| second.push(record(&cmd)));
    let expected: Vec<Applied> = (MAX_DRAIN_PER_FRAME as u32..total as u32)
        .map(|target| Applied::Hide { target })
        .collect();
    assert_eq!(second, expected, "残りは次のフレームで到着順に適用する");
}

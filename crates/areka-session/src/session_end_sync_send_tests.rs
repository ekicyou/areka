//! OS のセッションの終了の後始末が降ろす処理を待つ（join）間に、別スレッドから UI スレッドの窓へ
//! 同期の送信が重なっても止まらないことの再現。
//!
//! areka の中で join と重なって UI スレッドを待つ箇所は 0 件なので、代表の形（待ちの最中に
//! 別スレッドから UI の窓へ同期の送信を 1 通）を起こす。登場するスレッドは 4 本:
//! - UI 役（[`run_bounded`] が起こすスレッド）: message-only 窓を作り、`OnClose` で固まる偽の
//!   SHIORI で A を起こし、送信が自分のキューに届いたのを見てから、大きな上限で
//!   [`end_session_within`] を呼ぶ（見張りは発火しない）。後始末が戻ったら連番を取り、
//!   メッセージを 1 回だけ覗く（送られてきたメッセージが配られ、送信が返る）。
//! - 送り手: 「送る」の旗を立ててから UI 役の窓へ期限なしで同期に送る。返ったら連番を取る。
//! - 解き手: 旗と偽の SHIORI の固まりを見てから偽の SHIORI を解く（送信の返りは待たない）。
//! - ゴーストの実行系のスレッド（kanade・shiori ほか）: UI 役が join で待つ相手。
//!
//! 緑の意味: 後始末は送信に依らず戻り（連番で 後始末の戻り < 送信の戻り）、後始末が戻った時点で
//! 送信はまだ UI 役のキューで待っていて、UI 役が次にメッセージを覗いた時点で返る。見分けは順序だけで
//! 決め、時刻は使わない。join の最中に送られてきたメッセージを配る形に変わると、送信が後始末より先に
//! 返り、後始末の後のキューも空なので赤になる（受け手の説明「送られてきたメッセージは配らない」が
//! 崩れた）。join が送信を待つ形（輪）に変わると、送信も後始末も返らないので、全体を囲む
//! [`run_bounded`] が [`WHOLE`] で打ち切って赤になる。
//!
//! 送信に期限を付けない理由: 送信の期限は「輪になった」の見分けの代わりにならない。負荷の下で解き手の
//! スレッドの始まりが遅れると後始末が送信の期限より長くかかり、輪でないのに送信が期限で切れて後始末
//! より先に返った。固まりを切るのは外側の [`run_bounded`] の 1 つだけにする。
//!
//! 順序は sleep で作らない: 送信が UI 役のキューに届いたことは `GetQueueStatus`（配らずに読む）で、
//! 偽の SHIORI の固まりは `holding()` で見る。

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::thread;
use std::time::Duration;

use log_capture_kit::capture;
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, GetQueueStatus, HWND_MESSAGE, MSG, PM_NOREMOVE, PeekMessageW, QS_SENDMESSAGE,
    SendMessageW, WINDOW_EX_STYLE, WINDOW_STYLE, WM_NULL,
};
use windows::core::w;

use super::end_session_within;
use crate::ghost_switch_test_support::{FakeShiori, SwitchRig, standard_script};
use areka_runtime::spine::hold_support::HoldAt;
use areka_runtime::spine::{Progress, RecordedCall, run_bounded, wait_until};

/// 見張りが発火しない上限（後始末が戻るのは解き手が解いたときだけにする）。
const HOUR: Duration = Duration::from_secs(3600);
/// 再現の全体の上限（走行ごと固まらない・輪になったらここで赤）。
const WHOLE: Duration = Duration::from_secs(120);

/// UI 役のスレッドで見える結果。
#[derive(Debug, PartialEq, Eq)]
struct Observed {
    /// 後始末が戻った時点で、送信はまだ UI 役のキューで待っていたか（join の間は配らない）。
    pending_after_teardown: bool,
    /// 後始末の戻りが送信の戻りより先か（連番）。
    teardown_before_send: bool,
    /// 解く手が呼ばれた回数（解き手の 1 回だけ＝見張りは発火していない）。
    unblock_calls: usize,
    /// `OnClose` の NOTIFY の Reference0（届いた分だけ）。
    on_close_ref0: Vec<String>,
    /// `os_session_end_done` の `shiori_cut`（上限で切っていない）。
    shiori_cut: Option<String>,
}

/// 今のスレッドのキューに、別スレッドからの同期の送信が待っているか（配らずに読む）。
fn send_pending() -> bool {
    // SAFETY: キューの状態を読むだけで、送られてきたメッセージは配らない。上位語＝今キューにある種類。
    (unsafe { GetQueueStatus(QS_SENDMESSAGE) } >> 16) & QS_SENDMESSAGE.0 != 0
}

/// 今のスレッドに `STATIC`・`HWND_MESSAGE`（message-only）の窓を作り、取っ手を数で返す
/// （`HWND` はスレッドをまたいで渡せないので数にする）。
fn create_ui_window() -> isize {
    // SAFETY: 引数はすべて静的な文字列か空。親は HWND_MESSAGE（message-only）。
    let hwnd = unsafe {
        CreateWindowExW(
            WINDOW_EX_STYLE(0),
            w!("STATIC"),
            w!("session_end_sync_send"),
            WINDOW_STYLE(0),
            0,
            0,
            0,
            0,
            Some(HWND_MESSAGE),
            None,
            None,
            None,
        )
    }
    .expect("message-only 窓を作れる");
    hwnd.0 as isize
}

/// 窓 `hwnd` へ `WM_NULL` を期限なしで同期に送り、UI 役が受け取るまで戻らない。
fn send_null(hwnd: isize) {
    // SAFETY: ポインタを運ばない問い合わせ。宛先の窓は UI 役のスレッドが生かしている
    // （UI 役のスレッドが終われば窓も消え、送信は返る）。
    let _ = unsafe { SendMessageW(HWND(hwnd as *mut _), WM_NULL, None, None) };
}

/// 呼出列のうち `OnClose` の NOTIFY の Reference0（届いた分だけ）。
fn on_close_ref0(calls: &[RecordedCall]) -> Vec<String> {
    calls
        .iter()
        .filter_map(|c| match c {
            RecordedCall::Notify { id, references } if id == "OnClose" => {
                Some(references.first().cloned().unwrap_or_default())
            }
            _ => None,
        })
        .collect()
}

/// UI 役のスレッドの本体（モジュールの説明の手順）。
fn reproduce() -> Observed {
    let mut rig = SwitchRig::new(vec![(
        "A",
        FakeShiori::Scripted(Box::new(|| {
            standard_script("\\0A\\e").hold_at(HoldAt::Notify("OnClose"))
        })),
    )]);
    rig.plant_boot_record("A");
    rig.boot("A");
    assert!(rig.wait_steady(), "A が定常に着かない");
    let handle = rig.handle("A");
    let hwnd = create_ui_window();

    let seq = Arc::new(AtomicU8::new(0));
    let sending = Arc::new(AtomicBool::new(false));
    let sender = {
        let (seq, sending) = (Arc::clone(&seq), Arc::clone(&sending));
        thread::spawn(move || {
            sending.store(true, Ordering::SeqCst);
            send_null(hwnd);
            seq.fetch_add(1, Ordering::SeqCst)
        })
    };
    // 送信が届いてから後始末に入る（join の間に送信が待っていることを時刻に依らず作る）。
    assert!(
        rig.wait_for(send_pending),
        "送り手の送信が UI 役のキューに届かない"
    );
    let releaser = {
        let (sending, handle) = (Arc::clone(&sending), handle.clone());
        thread::spawn(move || {
            // 足場を持ち込めない別のスレッドなので、偽の SHIORI が受けた呼び出しの数を進みの目印にする。
            let waited = wait_until(
                "解き手: 送信の旗と偽の SHIORI の固まり",
                Progress::Count(&|| handle.call_count()),
                || sending.load(Ordering::SeqCst) && handle.holding(),
            );
            if let Err(failure) = &waited {
                eprintln!("{failure}");
            }
            let ready = waited.is_ok();
            if ready {
                handle.release();
            }
            ready
        })
    };

    let ((), events) = capture(|| end_session_within(&mut rig.world, HOUR));
    let teardown_seq = seq.fetch_add(1, Ordering::SeqCst);
    let pending_after_teardown = send_pending();
    let mut msg = MSG::default();
    // SAFETY: 今のスレッドのキューを 1 回覗くだけ（取り除かない）。待っている送信はここで配られる。
    let _ = unsafe { PeekMessageW(&mut msg, None, 0, 0, PM_NOREMOVE) };

    let send_seq = sender.join().expect("送り手は panic しない");
    assert!(
        releaser.join().expect("解き手は panic しない"),
        "解き手が送信の旗と偽の SHIORI の固まりを見られない"
    );
    Observed {
        pending_after_teardown,
        teardown_before_send: teardown_seq < send_seq,
        unblock_calls: handle.unblock_calls(),
        on_close_ref0: on_close_ref0(&rig.calls("A").last().cloned().unwrap_or_default()),
        shiori_cut: events
            .iter()
            .find(|e| e.field_str("event") == Some("os_session_end_done"))
            .and_then(|e| e.field_str("shiori_cut").or_else(|| e.field("shiori_cut")))
            .map(str::to_owned),
    }
}

/// join の最中に別スレッドから UI の窓へ同期の送信が重なっても、後始末は送信に依らず戻り、
/// 送信は後始末の後に UI 役が 1 回覗いた時点で返る。輪になったら全体の上限で赤。
/// `OnClose` は Ref0＝`system` で 1 件のまま。
#[test]
fn teardown_join_does_not_wait_for_a_sync_send_to_the_ui_window() {
    let (tx, rx) = std::sync::mpsc::channel();
    run_bounded(
        "join と同期の送信の重なりの再現",
        WHOLE,
        move || {
            let _ = tx.send(reproduce());
        },
    );
    assert_eq!(
        rx.recv().expect("再現の結果が届く"),
        Observed {
            pending_after_teardown: true,
            teardown_before_send: true,
            unblock_calls: 1,
            on_close_ref0: vec!["system".to_owned()],
            shiori_cut: Some("false".to_owned()),
        },
        "join と同期の送信が重なった（join の間に送られてきたメッセージを配った）"
    );
}

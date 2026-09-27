//! OS のセッションの終了の後始末が降ろす処理を待つ（join）間に、別スレッドから UI スレッドの窓へ
//! 同期の送信が重なっても止まらないことの再現（areka-P0-session-mark-residue task 6.1・要件 5.2・
//! 5.4・5.5・7.5）。
//!
//! `research.md` §6 の洗い出しでは、areka の中で join と重なって UI スレッドを待つ箇所は 0 件
//! だった。要件 5.2 に従い、代表の形（待ちの最中に別スレッドから UI の窓へ同期の送信を 1 通）を
//! 起こす。登場するスレッドは 4 本:
//! - UI 役（[`run_bounded`] が起こすスレッド）: message-only 窓を作り、`OnClose` で固まる偽の
//!   SHIORI で A を起こし、送信が自分のキューに届いたのを見てから、大きな上限で
//!   [`end_session_within`] を呼ぶ（見張りは発火しない）。後始末が戻ったら連番を取り、
//!   メッセージを 1 回だけ覗く（送られてきたメッセージが配られ、送信が返る）。
//! - 送り手: 「送る」の旗を立ててから UI 役の窓へ期限つきで同期に送る。返ったら連番を取る。
//! - 解き手: 旗と偽の SHIORI の固まりを見てから偽の SHIORI を解く（送信の返りは待たない）。
//! - ゴーストの実行系のスレッド（kanade・shiori ほか）: UI 役が join で待つ相手。
//!
//! 緑の意味: 後始末は送信に依らず戻り（連番で 後始末の戻り < 送信の戻り）、送信は UI 役が次に
//! メッセージを取り出した時点で期限内に返る。join が送信を待つ形（輪）に変わると、送信は期限で
//! 切れ、後始末はその後にしか戻れないので赤になる。join の最中に送られてきたメッセージを配る形に
//! 変わると、送信が後始末より先に返るので赤になる（受け手の説明「送られてきたメッセージは配らない」
//! が崩れた）。
//!
//! 順序は sleep で作らない: 送信が UI 役のキューに届いたことは `GetQueueStatus`（配らずに読む）で、
//! 偽の SHIORI の固まりは `holding()` で見る。走行ごと固まらないよう、全体を [`run_bounded`] で
//! 囲み、送信にも期限（[`SEND_TIMEOUT_MS`]）を付ける。

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::thread;
use std::time::Duration;

use log_capture_kit::capture;
use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, GetQueueStatus, HWND_MESSAGE, MSG, PM_NOREMOVE, PeekMessageW, QS_SENDMESSAGE,
    SMTO_NORMAL, SendMessageTimeoutW, WINDOW_EX_STYLE, WINDOW_STYLE, WM_NULL,
};
use windows::core::w;

use super::end_session_within;
use crate::emo2_boot::ghost_switch_test_support::{FakeShiori, SwitchRig, standard_script};
use crate::emo2_boot::spine::hold_support::HoldAt;
use crate::emo2_boot::spine::{RecordedCall, run_bounded, spin_wait_until};

/// 見張りが発火しない上限（後始末が戻るのは解き手が解いたときだけにする）。
const HOUR: Duration = Duration::from_secs(3600);
/// 送り手の同期の送信の期限（切れたら赤＝輪になった）。
const SEND_TIMEOUT_MS: u32 = 10_000;
/// 再現の全体の上限（走行ごと固まらない）。
const WHOLE: Duration = Duration::from_secs(120);

/// UI 役のスレッドで見える結果。
#[derive(Debug, PartialEq, Eq)]
struct Observed {
    /// 後始末が戻った時点で、送信はまだ UI 役のキューで待っていたか（join の間は配らない）。
    pending_after_teardown: bool,
    /// 後始末の戻りが送信の戻りより先か（連番）。
    teardown_before_send: bool,
    /// 送信が期限内に返ったか（`SendMessageTimeoutW` の戻り値が 0 でない）。
    send_ok: bool,
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

/// 窓 `hwnd` へ `WM_NULL` を期限つき・応答なし判定なし（`SMTO_NORMAL`）で同期に送り、
/// 戻り値が 0 でないか（期限内に返ったか）を返す。
fn send_null(hwnd: isize) -> bool {
    let mut result = 0usize;
    // SAFETY: ポインタを運ばない問い合わせ。宛先の窓は UI 役のスレッドが生かしている。
    let ret = unsafe {
        SendMessageTimeoutW(
            HWND(hwnd as *mut _),
            WM_NULL,
            WPARAM(0),
            LPARAM(0),
            SMTO_NORMAL,
            SEND_TIMEOUT_MS,
            Some(&mut result as *mut usize),
        )
    };
    ret.0 != 0
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
            let ok = send_null(hwnd);
            (ok, seq.fetch_add(1, Ordering::SeqCst))
        })
    };
    // 送信が届いてから後始末に入る（join の間に送信が待っていることを時刻に依らず作る）。
    assert!(
        spin_wait_until(send_pending),
        "送り手の送信が UI 役のキューに届かない"
    );
    let releaser = {
        let (sending, handle) = (Arc::clone(&sending), handle.clone());
        thread::spawn(move || {
            let ready = spin_wait_until(|| sending.load(Ordering::SeqCst) && handle.holding());
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

    let (send_ok, send_seq) = sender.join().expect("送り手は panic しない");
    assert!(
        releaser.join().expect("解き手は panic しない"),
        "解き手が送信の旗と偽の SHIORI の固まりを見られない"
    );
    Observed {
        pending_after_teardown,
        teardown_before_send: teardown_seq < send_seq,
        send_ok,
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
/// 送信は後始末の後に UI 役が 1 回覗いた時点で期限内に返る（要件 5.2・5.4・7.5）。
/// `OnClose` は Ref0＝`system` で 1 件のまま（要件 5.5）。
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
            send_ok: true,
            unblock_calls: 1,
            on_close_ref0: vec!["system".to_owned()],
            shiori_cut: Some("false".to_owned()),
        },
        "join と同期の送信が重なった（後始末が送信を待った・join の間に配った・送信が期限で切れた）"
    );
}

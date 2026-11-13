use super::*;
use crate::proxy_test_support::copy_testdll_into_unique_target_dir;
use std::rc::Rc;
use windows::Win32::System::DataExchange::COPYDATASTRUCT;

/// 受けた HELLO の helper HWND・RESPONSE の数・最後の RESPONSE・届いた順の全 RESPONSE。
type ParentRecord = (
    Rc<Cell<Option<u32>>>,
    Rc<Cell<u64>>,
    Rc<RefCell<Vec<u8>>>,
    Rc<RefCell<Vec<Vec<u8>>>>,
);

/// stand-in parent（HELLO / RESPONSE の受け皿）。受けたものを [`ParentRecord`] に記録する。
fn stand_in_parent() -> (Window<()>, ParentRecord) {
    let record: ParentRecord = Default::default();
    let (hello_helper_hwnd, responses, last_response, all_responses) = record.clone();
    let parent = Window::new(
        WindowType::MessageOnly,
        (),
        move |_state: Pin<&()>, msg: WindowMessage| -> Option<LRESULT> {
            if msg.msg != WM_COPYDATA {
                return None;
            }
            // SAFETY: WM_COPYDATA 契約。
            if let Some((dw, len, payload)) = unsafe { read_copydata(msg.lparam) } {
                match copydata_payload(dw, len, &payload) {
                    Ok((MsgTag::Hello, p)) if p.len() == 4 => {
                        let bytes = [p[0], p[1], p[2], p[3]];
                        hello_helper_hwnd.set(Some(shiori_host32_ipc::decode_hwnd_le(bytes)));
                    }
                    Ok((MsgTag::Response, p)) => {
                        responses.set(responses.get() + 1);
                        *last_response.borrow_mut() = p.to_vec();
                        all_responses.borrow_mut().push(p.to_vec());
                    }
                    _ => {}
                }
            }
            Some(LRESULT(0))
        },
    )
    .expect("stand-in parent 窓生成に失敗");
    (parent, record)
}

/// 単一 loopback テスト（HELLO 送出・未確立 REQUEST→500・LOAD・proxy 駆動 REQUEST(GET/NOTIFY)・
/// 不正フレーム記録・bounded 生存を同一窓で網羅）。
///
/// 窓は stand-in parent ＋ helper の 2 つ。
/// bounded: 有限メッセージを PostMessage で撒いた後に必ず抜ける（無限ループ禁止）。
#[test]
#[cfg_attr(
    not(target_arch = "x86"),
    ignore = "i686 専用: 32bit testdll(shiori.dll) を load するため x64 では BAD_EXE_FORMAT。`cargo test -p shiori-host32-helper --target i686-pc-windows-msvc` で実行"
)]
fn loopback_hello_request_proxy_driven_and_bounded_loop() {
    // --- stand-in parent（HELLO / RESPONSE 受け皿）。受領を Rc<Cell> に記録 ---
    let (parent, (hello_helper_hwnd, responses, last_response, _)) = stand_in_parent();
    let parent_hwnd_u32 = u32::from_le_bytes(encode_hwnd_le(parent.hwnd()));

    // --- helper 窓生成（HELLO は create 内で親へ送出）---
    // LOAD 経路検証のため、testdll(shiori.dll) を入れたテストの根を load_dir として helper を構築する。
    // dll_path = load_dir\shiori.dll（絶対）。testdll 解決は silent skip 禁止（resolve_testdll が panic）。
    // 根は helper より先に作るので、helper の後に消える。
    let (root, _) = copy_testdll_into_unique_target_dir("loop");
    let load_dir = root.path().to_path_buf();

    let helper =
        HelperMessageWindow::create(parent_hwnd_u32, load_dir.clone(), "shiori.dll".to_string())
            .expect("HelperMessageWindow 生成に失敗");
    let helper_hwnd_u32 = u32::from_le_bytes(encode_hwnd_le(helper.hwnd()));

    // HELLO 送出経路が動いた: 親が helper HWND を復号一致で受領（要件 3.1）。
    assert_eq!(
        hello_helper_hwnd.get(),
        Some(helper_hwnd_u32),
        "親が HELLO を復号一致で受領する（要件 3.1）"
    );
    assert_eq!(helper.shared().hellos_sent.get(), 1);

    // --- 未確立 REQUEST: proxy 未確立（LOAD 前）で REQUEST を送っても crash せず、識別可能な
    //     500 エラー RESPONSE を親が受領する（echo ではない・R3.1）---
    // Load-before-Request は構造的不変だが、helper は不変違反でも crash しない最小防御を持つ。
    let pre_load_req =
        b"GET SHIORI/3.0\r\nCharset: UTF-8\r\nSender: areka\r\nID: OnTestValue\r\n\r\n";
    send_copydata(
        helper.hwnd(),
        parent.hwnd(),
        MsgTag::Request,
        pre_load_req,
        REPLY_TIMEOUT,
    )
    .expect("未確立 REQUEST 送出に失敗");

    assert_eq!(
        helper.shared().requests_handled.get(),
        1,
        "WndProc が未確立 REQUEST も処理する（crash しない・R3.1）"
    );
    assert_eq!(
        helper.shared().responses_sent.get(),
        1,
        "未確立 REQUEST にも RESPONSE を 1 通返す（無応答でない・R3.1）"
    );
    assert_eq!(
        responses.get(),
        1,
        "親が未確立 REQUEST の RESPONSE を受領する"
    );
    {
        let text =
            String::from_utf8(last_response.borrow().clone()).expect("500 RESPONSE は UTF-8");
        assert!(
            text.contains("SHIORI/3.0 500 Internal Server Error"),
            "未確立 proxy では識別可能な 500 を返す（echo ではない・R3.1）: {text:?}"
        );
        assert_ne!(
            last_response.borrow().as_slice(),
            &pre_load_req[..],
            "RESPONSE は request の echo ではない（proxy 駆動結果／エラー）"
        );
    }

    // --- LOAD 経路: MsgTag::Load（空ペイロード）→ 未確立ゆえ proxy 確立 → ack[1]（要件 4.1/5.1）---
    // 親が受領した最新 RESPONSE を LOAD ack として観測する（load_dir\shiori.dll=testdll を確立）。
    send_copydata(
        helper.hwnd(),
        parent.hwnd(),
        MsgTag::Load,
        &[],
        REPLY_TIMEOUT,
    )
    .expect("LOAD 送出に失敗");

    assert_eq!(
        helper.shared().loads_attempted.get(),
        1,
        "WndProc が LOAD トリガを受領する（要件 4.1）"
    );
    assert_eq!(
        helper.shared().load_acks_ok.get(),
        1,
        "初回 LOAD は proxy 確立成功で ack[1] を送出する（要件 5.1/6.4）"
    );
    assert_eq!(
        helper.shared().load_acks_fail.get(),
        0,
        "成功 LOAD で ack[0] は送出しない"
    );
    assert!(
        helper.shared().proxy.borrow().is_some(),
        "確立成功した proxy が常設保持される（要件 4.3）"
    );
    assert_eq!(
        responses.get(),
        2,
        "親が LOAD ack を 1 通受領する（未確立 REQUEST 500 と合わせ 2）"
    );
    assert_eq!(
        &*last_response.borrow(),
        &[LOAD_ACK_OK],
        "LOAD ack は厳密 1 byte [1]（成功・要件 5.1）"
    );

    // --- proxy 駆動 REQUEST(GET): 確立済み proxy で REQUEST を送ると testdll が固定 200 応答を返し、
    //     それが echo でなく proxy 駆動結果として親へ RESPONSE 返送される（R4.7・Observable）---
    let get_req = b"GET SHIORI/3.0\r\nCharset: UTF-8\r\nSender: areka\r\nID: OnTestValue\r\n\r\n";
    send_copydata(
        helper.hwnd(),
        parent.hwnd(),
        MsgTag::Request,
        get_req,
        REPLY_TIMEOUT,
    )
    .expect("proxy 駆動 GET REQUEST 送出に失敗");

    assert_eq!(
        helper.shared().requests_handled.get(),
        2,
        "WndProc が proxy 駆動 GET REQUEST を処理する（要件 4.7）"
    );
    assert_eq!(
        responses.get(),
        3,
        "親が proxy 駆動 GET の RESPONSE を受領する（計 3）"
    );
    {
        let text =
            String::from_utf8(last_response.borrow().clone()).expect("GET RESPONSE は UTF-8");
        assert!(
            text.contains("SHIORI/3.0 200 OK"),
            "proxy 駆動 GET は testdll の固定 200 応答を返す（R4.7・Observable）: {text:?}"
        );
        assert!(
            text.contains("Value: \\0\\s[0]host32 request roundtrip ok\\e"),
            "proxy 駆動 GET の固定 Value 行（echo ではない・R4.7）: {text:?}"
        );
        assert_ne!(
            last_response.borrow().as_slice(),
            &get_req[..],
            "RESPONSE は request の echo ではない（proxy 駆動 SHIORI/3.0 応答）"
        );
    }

    // --- proxy 駆動 REQUEST(NOTIFY): 同経路で helper は proxy.request を駆動し DLL 戻り（204）を
    //     そのまま RESPONSE 返送する（GET と同一経路・host 側で破棄・R4.8 の helper 側）---
    let notify_req =
        b"NOTIFY SHIORI/3.0\r\nCharset: UTF-8\r\nSender: areka\r\nID: OnTestNotify\r\n\r\n";
    send_copydata(
        helper.hwnd(),
        parent.hwnd(),
        MsgTag::Request,
        notify_req,
        REPLY_TIMEOUT,
    )
    .expect("proxy 駆動 NOTIFY REQUEST 送出に失敗");

    assert_eq!(
        helper.shared().requests_handled.get(),
        3,
        "WndProc が proxy 駆動 NOTIFY REQUEST も同経路で処理する（R4.8）"
    );
    assert_eq!(
        responses.get(),
        4,
        "親が proxy 駆動 NOTIFY の RESPONSE を受領する（計 4）"
    );
    {
        let text =
            String::from_utf8(last_response.borrow().clone()).expect("NOTIFY RESPONSE は UTF-8");
        assert!(
            text.contains("SHIORI/3.0 204 No Content"),
            "proxy 駆動 NOTIFY は testdll の固定 204 応答を返す（helper は GET と同一駆動・R4.8）: {text:?}"
        );
    }

    // --- 冪等再 LOAD: もう一度 Load → load 再呼出なしで ack[1] 冪等返送（R2.4・無 panic）---
    send_copydata(
        helper.hwnd(),
        parent.hwnd(),
        MsgTag::Load,
        &[],
        REPLY_TIMEOUT,
    )
    .expect("再 LOAD 送出に失敗");

    assert_eq!(
        helper.shared().loads_attempted.get(),
        2,
        "再 LOAD も TriggerLoad として受領する"
    );
    assert_eq!(
        helper.shared().load_acks_ok.get(),
        2,
        "確立済み再 LOAD は load 再呼出なしで ack[1] を冪等返送する（R2.4）"
    );
    assert_eq!(
        helper.shared().load_acks_fail.get(),
        0,
        "冪等 LOAD でも ack[0] は送出しない"
    );
    assert_eq!(
        responses.get(),
        5,
        "親が冪等 LOAD ack を追加 1 通受領する（計 5）"
    );
    assert_eq!(
        &*last_response.borrow(),
        &[LOAD_ACK_OK],
        "冪等 LOAD ack も厳密 1 byte [1]（R2.4）"
    );

    // --- UNLOAD（正規正常終了経路のアーム機構・R5.1/R5.6・Task 3.2）---
    // MsgTag::Unload（空ペイロード）を helper へ送ると、helper は proxy を take して `close`
    // （unload → FreeLibrary）→ quit_requested セット → 既存 LOAD ack と同型の ack[1] を
    // close 完了後・ループ終了前の順序で返送する。proxy は未確立へ戻る。
    // （proxy を閉じるため、以降 proxy 確立を前提とする REQUEST 検証は置かない・R5.1）。
    // exit 0 の観測は x64 e2e（Task 5.1）の領分ゆえここでは扱わない。
    send_copydata(
        helper.hwnd(),
        parent.hwnd(),
        MsgTag::Unload,
        &[],
        REPLY_TIMEOUT,
    )
    .expect("UNLOAD 送出に失敗");

    assert_eq!(
        helper.shared().unloads_handled.get(),
        1,
        "WndProc が UNLOAD トリガを受領する（R5.6）"
    );
    assert!(
        helper.shared().quit_requested.get(),
        "UNLOAD 受領で終了要求フラグが立つ（R5.6・正規正常終了経路）"
    );
    assert!(
        helper.shared().proxy.borrow().is_none(),
        "UNLOAD で proxy を take→drop し未確立へ戻す（courtesy unload 実行・R5.1）"
    );
    assert_eq!(
        responses.get(),
        6,
        "親が UNLOAD ack を追加 1 通受領する（proxy drop 後・ループ終了前・計 6）"
    );
    assert_eq!(
        &*last_response.borrow(),
        &[LOAD_ACK_OK],
        "UNLOAD ack は既存 LOAD ack と同型の厳密 1 byte [1]（新契約を発明しない・R5.1）"
    );

    // --- R5.1: quit_requested が立った状態で、main と同型のフィルタ（quit_requested 検知→quit）を
    //     持つ MessageLoop を回すと、posted メッセージ契機でフィルタが flag を見てループを正常終了する
    //     （＝正規の正常終了経路のループ機構。プロセス exit 0 の観測は x64 e2e=Task 5.1）。この demo は
    //     UNLOAD セクションで既に quit_requested==true になった後に実行し、独自の MessageLoop を生成・
    //     消費する（前段の bounded 生存ループ・後段とは各 MessageLoop::run が返ってから次が始まるため
    //     干渉しない）。quit_seen==true は quit 配線が正しいことに依存する実回帰ガードであり（配線を
    //     外すとループが flag で抜けず、上の RED 実測で確認済み）、shell ではない。---
    // SAFETY: helper.hwnd() は有効。WM_NULL(0) は無害な起こし用メッセージ。
    unsafe {
        let _ = PostMessageW(Some(helper.hwnd()), 0, WPARAM(0), LPARAM(0));
    }
    {
        let quit_seen = Rc::new(Cell::new(false));
        let helper_ref = &helper;
        MessageLoop::run({
            let quit_seen = quit_seen.clone();
            move |msg_loop, _msg| {
                if helper_ref.shared().quit_requested.get() {
                    quit_seen.set(true);
                    msg_loop.quit();
                }
                FilterResult::Forward
            }
        });
        assert!(
            quit_seen.get(),
            "quit_requested 検知でメッセージループが正常終了する（R5.1・正規正常終了経路のループ機構）"
        );
    }

    // --- 不正フレーム: 未知タグを helper へ送っても crash せず記録のみ（要件 2.5）---
    // 未知タグ生値 0xFF を dwData に載せて自窓へ送る（copydata_payload が UnknownTag で弾く）。
    {
        let payload: &[u8] = b"";
        let cds = COPYDATASTRUCT {
            dwData: 0xFFusize,
            cbData: 0,
            lpData: payload.as_ptr() as *mut core::ffi::c_void,
        };
        // SAFETY: helper.hwnd() は有効。&cds は本呼び出し中生存。
        unsafe {
            let _ = windows::Win32::UI::WindowsAndMessaging::SendMessageW(
                helper.hwnd(),
                WM_COPYDATA,
                Some(windows::Win32::Foundation::WPARAM(parent.hwnd().0 as usize)),
                Some(LPARAM(&cds as *const COPYDATASTRUCT as isize)),
            );
        }
    }
    assert_eq!(
        helper.shared().bad_frames.get(),
        1,
        "不正フレームは crash させず記録のみ（要件 2.5）"
    );
    // 不正フレームでは RESPONSE を送らない（無応答）。
    // 未確立 REQUEST 500(1)＋LOAD ack(2)＋GET(3)＋NOTIFY(4)＋冪等 LOAD ack(5)＋UNLOAD ack(6) の計 6 のまま。
    assert_eq!(
        responses.get(),
        6,
        "不正フレームに応答しない（無応答・計 6 のまま）"
    );

    // --- bounded ループ生存: 有限個の WM_NULL を撒いてから quit し、必ず抜ける（無クラッシュ）---
    let pumped = Rc::new(Cell::new(0u32));
    const PUMP_N: u32 = 8;
    // SAFETY: helper.hwnd() は有効。WM_NULL(0) は無害な起こし用メッセージ。
    for _ in 0..PUMP_N {
        unsafe {
            let _ = PostMessageW(
                Some(helper.hwnd()),
                0, // WM_NULL
                windows::Win32::Foundation::WPARAM(0),
                LPARAM(0),
            );
        }
    }
    {
        let pumped = pumped.clone();
        MessageLoop::run(move |msg_loop, _msg| {
            pumped.set(pumped.get() + 1);
            if pumped.get() >= PUMP_N {
                msg_loop.quit();
            }
            FilterResult::Forward
        });
    }
    assert!(
        pumped.get() >= PUMP_N,
        "bounded ループが有限回で必ず抜ける（無限ループでない・要件 6.3）"
    );

    // 窓は Drop で破棄（helper Drop で proxy の courtesy unload → FreeLibrary が走る）。
    drop(helper);
    drop(parent);
    // 根（testdll の写し先）は Drop で消える。
    drop(root);
}

/// helper が始めに標準の取っ手から受け継ぎの印を落とす（design.md U5・指摘 4）。
///
/// 自分の標準エラーの取っ手に受け継ぎの印を立ててから `disinherit_std_handles()` を呼び、印が落ちた
/// ことを `GetHandleInformation` で見る（呼ばなければ印は立ったまま＝赤）。印は取っ手ごとの値で、
/// この crate で取っ手の印を触るテストはこの 1 本だけなので並走のテストと競らない。終わりに元の印へ戻す。
#[test]
#[cfg(target_arch = "x86")]
fn std_handles_are_not_inherited() {
    use windows::Win32::Foundation::{
        GetHandleInformation, HANDLE, HANDLE_FLAG_INHERIT, HANDLE_FLAGS, SetHandleInformation,
    };
    use windows::Win32::System::Console::{GetStdHandle, STD_ERROR_HANDLE};

    fn flags(h: HANDLE) -> u32 {
        let mut f = 0u32;
        // SAFETY: Win32 境界。生きている標準エラーの取っ手の印を読むだけ。
        unsafe { GetHandleInformation(h, &mut f) }.expect("GetHandleInformation に失敗");
        f
    }

    // SAFETY: Win32 境界。自分の標準エラーの取っ手を取るだけ。
    let h = unsafe { GetStdHandle(STD_ERROR_HANDLE) }.expect("GetStdHandle に失敗");
    assert!(
        !h.is_invalid() && !h.0.is_null(),
        "テストのプロセスに標準エラーの取っ手が無い"
    );
    let original = flags(h);
    // SAFETY: Win32 境界。自分の取っ手の受け継ぎの印を立てるだけ。
    unsafe { SetHandleInformation(h, HANDLE_FLAG_INHERIT.0, HANDLE_FLAG_INHERIT) }
        .expect("受け継ぎの印を立てられない");
    assert_ne!(flags(h) & HANDLE_FLAG_INHERIT.0, 0, "前提: 印が立っている");

    disinherit_std_handles();
    let after = flags(h);

    // SAFETY: Win32 境界。元の印へ戻す。
    unsafe {
        SetHandleInformation(
            h,
            HANDLE_FLAG_INHERIT.0,
            HANDLE_FLAGS(original & HANDLE_FLAG_INHERIT.0),
        )
    }
    .expect("元の印へ戻せない");
    assert_eq!(
        after & HANDLE_FLAG_INHERIT.0,
        0,
        "disinherit_std_handles の後も標準エラーに受け継ぎの印が残っている"
    );
}

/// `[helper]` で始まり `needle` を含む行が記録にあるか（記録の口 `helper_log`・指摘 9・12）。
fn assert_logged(log: &[String], needle: &str) {
    assert!(
        log.iter()
            .any(|l| l.starts_with("[helper]") && l.contains(needle)),
        "`{needle}` を含む [helper] の行が無い: {log:?}"
    );
}

/// 指摘 9・要件 10.11: `unload` が偽でも UNLOAD の返事は [1] のままで、失敗は `[helper]` の行に残る。
/// 偽の DLL を自分だけのフォルダへ写して読み、印の環境変数は差さないので錠は取らない。
#[test]
#[cfg(target_arch = "x86")]
fn unload_ack_stays_one_when_unload_returns_false() {
    let (parent, (_, responses, last_response, _)) = stand_in_parent();
    let parent_hwnd_u32 = u32::from_le_bytes(encode_hwnd_le(parent.hwnd()));
    let (root, _) = copy_testdll_into_unique_target_dir("unload-false");
    let helper = HelperMessageWindow::create(
        parent_hwnd_u32,
        root.path().to_path_buf(),
        "shiori.dll".to_string(),
    )
    .expect("HelperMessageWindow 生成に失敗");
    let send = |tag: MsgTag, payload: &[u8]| {
        send_copydata(helper.hwnd(), parent.hwnd(), tag, payload, REPLY_TIMEOUT)
            .unwrap_or_else(|e| panic!("{tag:?} の送出に失敗: {e:?}"));
    };

    send(MsgTag::Load, &[]);
    assert_eq!(&*last_response.borrow(), &[LOAD_ACK_OK], "LOAD は成功する");
    send(
        MsgTag::Request,
        b"NOTIFY SHIORI/3.0\r\nCharset: UTF-8\r\nID: OnTestUnloadFalse\r\n\r\n",
    );
    assert!(
        last_response.borrow().starts_with(b"SHIORI/3.0 204"),
        "OnTestUnloadFalse は 204"
    );

    take_helper_log();
    send(MsgTag::Unload, &[]);
    assert_eq!(responses.get(), 3, "UNLOAD の返事が 1 通届く");
    assert_eq!(
        &*last_response.borrow(),
        &[LOAD_ACK_OK],
        "unload が偽でも UNLOAD の返事は [1]（要件 10.11）"
    );
    assert!(helper.shared().proxy.borrow().is_none());
    assert_logged(&take_helper_log(), "UnloadReturnedFalse");

    drop(helper);
    drop(parent);
    drop(root);
}

/// 指摘 12: 自窓を起こす `PostMessageW` が失敗したら偽を返し、`[helper]` の行に残す。
/// 壊した窓の値を渡して失敗させる。
#[test]
fn wake_self_logs_a_post_failure() {
    let gone = Window::new(
        WindowType::MessageOnly,
        (),
        |_state: Pin<&()>, _msg: WindowMessage| -> Option<LRESULT> { None },
    )
    .expect("窓生成に失敗");
    let hwnd = gone.hwnd();
    drop(gone);

    take_helper_log();
    assert!(!wake_self(hwnd), "壊した窓へは起こしのメッセージを置けない");
    assert_logged(&take_helper_log(), "PostMessageW");
}

/// 指摘 12: `lparam` が空の `WM_COPYDATA` は不正なメッセージとして数え、`[helper]` の行に残す。
#[test]
fn an_empty_copydata_is_counted_as_bad() {
    let s = HelperShared::new(0, PathBuf::new(), String::new());
    let msg = WindowMessage {
        hwnd: HWND::default(),
        msg: WM_COPYDATA,
        wparam: WPARAM(0),
        lparam: LPARAM(0),
    };

    take_helper_log();
    assert_eq!(handle_message(&s, HWND::default(), &msg), Some(LRESULT(0)));
    assert_eq!(s.bad_frames.get(), 1, "空の lparam は不正なメッセージ");
    assert_logged(&take_helper_log(), "lparam");
}

/// 指摘 20: `OnTestNull` で DLL が null の応答を返しても、折り返しの REQUEST の腕は 500 を返す。
/// 偽の DLL を自分だけのフォルダへ写して読み、印の環境変数は差さないので錠は取らない。
#[test]
#[cfg(target_arch = "x86")]
fn a_null_response_answers_500() {
    let (parent, (_, _, last_response, _)) = stand_in_parent();
    let parent_hwnd_u32 = u32::from_le_bytes(encode_hwnd_le(parent.hwnd()));
    let (root, _) = copy_testdll_into_unique_target_dir("null-500");
    let helper = HelperMessageWindow::create(
        parent_hwnd_u32,
        root.path().to_path_buf(),
        "shiori.dll".to_string(),
    )
    .expect("HelperMessageWindow 生成に失敗");
    let send = |tag: MsgTag, payload: &[u8]| {
        send_copydata(helper.hwnd(), parent.hwnd(), tag, payload, REPLY_TIMEOUT)
            .unwrap_or_else(|e| panic!("{tag:?} の送出に失敗: {e:?}"));
    };

    send(MsgTag::Load, &[]);
    assert_eq!(&*last_response.borrow(), &[LOAD_ACK_OK], "LOAD は成功する");
    send(
        MsgTag::Request,
        b"GET SHIORI/3.0\r\nCharset: UTF-8\r\nID: OnTestNull\r\n\r\n",
    );
    assert_eq!(
        &*last_response.borrow(),
        REQUEST_ERROR_RESPONSE,
        "null の応答は 500"
    );

    drop(helper);
    drop(parent);
    drop(root);
}

/// 指摘 8 の再入の場: testdll を読んだ helper へ `OnTestReenter` を送り、DLL の `request` の中から
/// helper の窓へ `tag`（3＝REQUEST・5＝UNLOAD）の空の `WM_COPYDATA` を送り返させる。
/// 返すのはテストの根・helper（この順で受ければ panic でも helper が先に消える）・親に届いた全 RESPONSE（LOAD の ack を含む）・
/// REQUEST を送った後の `[helper]` の行。
#[cfg(target_arch = "x86")]
fn reenter(
    name: &str,
    tag: MsgTag,
) -> (
    crate::proxy_test_support::TestRoot,
    HelperMessageWindow,
    Vec<Vec<u8>>,
    Vec<String>,
) {
    let (parent, (_, _, _, all_responses)) = stand_in_parent();
    let parent_hwnd_u32 = u32::from_le_bytes(encode_hwnd_le(parent.hwnd()));
    let (root, _) = copy_testdll_into_unique_target_dir(name);
    let helper = HelperMessageWindow::create(
        parent_hwnd_u32,
        root.path().to_path_buf(),
        "shiori.dll".to_string(),
    )
    .expect("HelperMessageWindow 生成に失敗");
    send_copydata(
        helper.hwnd(),
        parent.hwnd(),
        MsgTag::Load,
        &[],
        REPLY_TIMEOUT,
    )
    .expect("LOAD の送出に失敗");
    take_helper_log();
    let req = format!(
        "GET SHIORI/3.0\r\nCharset: UTF-8\r\nID: OnTestReenter\r\nReference0: {}\r\nReference1: {}\r\n\r\n",
        helper.hwnd().0 as usize,
        tag as usize
    );
    send_copydata(
        helper.hwnd(),
        parent.hwnd(),
        MsgTag::Request,
        req.as_bytes(),
        REPLY_TIMEOUT,
    )
    .expect("REQUEST の送出に失敗");
    let log = take_helper_log();
    let all = all_responses.borrow().clone();
    drop(parent);
    (root, helper, all, log)
}

/// 指摘 8: DLL の呼び出しの最中の REQUEST は DLL を入れ子に呼ばずに 500、外の呼び出しは 204 のまま。
#[test]
#[cfg(target_arch = "x86")]
fn request_during_a_dll_call_gets_500() {
    let (root, helper, all, log) = reenter("reenter-request", MsgTag::Request);
    let all: Vec<String> = all
        .iter()
        .map(|r| String::from_utf8_lossy(r).into_owned())
        .collect();
    assert_eq!(
        all.len(),
        3,
        "LOAD の ack・最中の 500・外の応答の 3 通: {all:?}"
    );
    assert_eq!(all[0].as_bytes(), [LOAD_ACK_OK], "LOAD の ack");
    assert_eq!(
        all[1].as_bytes(),
        REQUEST_ERROR_RESPONSE,
        "最中の REQUEST は 500"
    );
    assert!(
        all[2].starts_with("SHIORI/3.0 204"),
        "外の REQUEST は 204: {all:?}"
    );
    assert_logged(&log, "最中に REQUEST");
    assert!(!helper.shared().in_dll_call.get(), "戻った後は印が下りる");
    assert!(helper.shared().proxy.borrow().is_some(), "proxy は残る");
    drop(helper);
    drop(root);
}

/// 指摘 8: DLL の呼び出しの最中の UNLOAD は ack [1] を先に返し、戻った後に遅れた応答を送らずに
/// 後始末（`close` → 終了要求）を行う（今は `proxy` の借りの衝突で panic して落ちる）。
#[test]
#[cfg(target_arch = "x86")]
fn unload_during_a_dll_call_defers_the_unload() {
    let (root, helper, all, log) = reenter("reenter-unload", MsgTag::Unload);
    assert_eq!(
        all,
        vec![vec![LOAD_ACK_OK], vec![LOAD_ACK_OK]],
        "LOAD の ack と UNLOAD の ack [1] だけで、遅れた RESPONSE は届かない"
    );
    assert_logged(&log, "最中に UNLOAD");
    assert_logged(&log, "遅れた RESPONSE は送らない");
    assert_eq!(helper.shared().unloads_handled.get(), 1);
    assert!(helper.shared().proxy.borrow().is_none(), "proxy は閉じた");
    assert!(
        helper.shared().quit_requested.get(),
        "後始末の後に終了要求が立つ"
    );
    drop(helper);
    drop(root);
}

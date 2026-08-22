//! ウィンドウメッセージ受理（`window_pos.rs`／`lifecycle.rs`）の**遷移観測**に対する
//! 決定論的テスト。
//!
//! 固定するのは 4 つである。
//!
//! - **受理の記録**——拡大率変更（`WM_DPICHANGED`）・窓位置変更（`WM_WINDOWPOSCHANGED`）・
//!   表示構成変更（`WM_DISPLAYCHANGE`）の 3 メッセージが、窓ハンドルと「窓書込の内側で
//!   受理されたか」（`in_swp`）・「一括書込の開始からの経過」（`since_flush_us`）つきで
//!   記録されること。
//! - **同期書込の段**——OS 提案位置を採用したときの書込が `stage=sync` で記録され、
//!   一括書込（`stage=flush`）と区別できること。**採否の判定自体は変えない**——同じ
//!   捕捉窓の中で既存の採否ログ（`applied=`）と並存することを見る。
//! - **既定 OFF**——観測 target を点けていない運転では 1 行も出ないこと。
//! - **前置ガード**——`WM_WINDOWPOSCHANGED` はドラッグ中も含めて絶えず走る経路であり
//!   （要件 10.6）、既定運転で行の組立・時刻読み・矩形の読み戻しを一切行わないこと。
//!   これは出力では示せない（`emit_line` の水準を subscriber が濾すため既定では
//!   どのみち無音になる）ので、本文の構造で固定する。
//!
//! # `in_swp` を見るテストと書込を撃つテストは直列化する（要件 7.7・7.1）
//!
//! `in_swp` の出所である `is_self_initiated()` は**プロセス共有**の `SELF_INITIATED_DEPTH`
//! （`window/command.rs`）を読む。スレッド局所ではないので、並列に走る別テストの
//! `guarded_set_window_pos` が持ち上げた値がそのまま見え、`in_swp=false` の検査が
//! 気まぐれに落ちる（是正前の実測: `cargo test -p wintf --lib` 60 周中 11 周が赤）。
//! よって ⑴ 値を読むテストと ⑵ 値を持ち上げるテスト（提案位置を採る配送・一括書込）の
//! 両方が [`lock_self_initiated_for_test`](crate::ecs::window::lock_self_initiated_for_test)
//! をテスト本体の先頭で取り、最後まで持つ。
//!
//! **錠は本体で取る（ヘルパでは取らない）**——`std::sync::Mutex` は再入不可であり、
//! ヘルパと本体の両方で取ると自分自身と競合して止まる。
//!
//! # 実行形態の明示（要件 7.6）
//!
//! 本ファイルの検証はすべて**テストスレッド上の直接呼出**である。wndproc のハンドラは
//! 呼び出しスレッドで走り、一括書込（`flush`）も同様なので、`tracing` のスレッド局所
//! subscriber（[`capture_under_filter`]）で確実に捕捉できる。多スレッド実行器の相での
//! 「出ないこと」は一切主張しない。
//!
//! 「出ない」を見るテストには、同じ捕捉窓の内側に必ず拾えるはずの**対照行**を置く
//! ——捕捉が死んでいるだけで緑になる形にしない。
//!
//! # 実窓を使う 1 本について
//!
//! `WM_WINDOWPOSCHANGED` の `in_swp=true`／`since_flush_us` が実際に立つのは
//! **`SetWindowPos` が同期送達したとき**だけである。これは実窓が要るが、`SetWindowPos`
//! は同一スレッドの窓へ**同期 send** するのでメッセージポンプは要らない——よって
//! 実機サインオフではなく決定論テストで再現できる（再観測 §5 の 24/24＝「各窓の最初の
//! 書込の内側で当該窓のメッセージが同期処理される」に対応する形）。

use std::cell::RefCell;
use std::rc::Rc;

use bevy_ecs::prelude::Entity;
use windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, RegisterClassExW, SET_WINDOW_POS_FLAGS,
    SWP_NOACTIVATE, SWP_NOSIZE, SWP_NOZORDER, WINDOW_EX_STYLE, WINDOWPOS, WM_DISPLAYCHANGE,
    WM_DPICHANGED, WM_WINDOWPOSCHANGED, WNDCLASSEXW, WS_EX_TOOLWINDOW, WS_POPUP,
};
use windows::core::{PCWSTR, w};

use crate::ecs::App;
use crate::ecs::test_support::capture_under_filter;
use crate::ecs::window::transition_diag::{
    self, KIND_MSG, KIND_WRITE, MISSING, MSG_DISPLAYCHANGE, MSG_DPICHANGED, MSG_WINDOWPOSCHANGED,
    ORIGIN_DPI_SUGGESTED, RECORD_PREFIX_TAG, TRANSITION_TARGET,
};
use crate::ecs::window::{DPI, DpiSuggestedRectPolicy, SetWindowPosCommand, WindowHandle};
use crate::ecs::world::EcsWorld;
use crate::executor::util::WindowMessage;

/// 実機サインオフが用いる directive のうち、本チャネルを点灯させるもの。
const SIGNOFF_DIRECTIVES: &str = "info,wintf::transition=debug";

/// 既定水準（観測チャネルを有効化していない通常運転）。
const DEFAULT_DIRECTIVES: &str = "info";

/// 捕捉が生きている証拠として同じ窓の中で必ず出す対照行。
const CONTROL_LINE: &str = "[transition-probe] alive";

/// 対照行を出す（`info` 水準ゆえ両 directive で通る）。
fn emit_control() {
    tracing::info!(target: TRANSITION_TARGET, "{CONTROL_LINE}");
}

/// テスト冒頭の初期化——キューの残り・刻印の写し・残置コンテキストを落とす（要件 7.7）。
fn clean_slate() {
    let _residue = crate::ecs::window::drain_window_pos_commands();
    let _context = crate::ecs::window::DpiChangeContext::take();
    transition_diag::reset_for_test();
}

/// 捕捉出力から観測チャネルの行だけを取り出す。
fn transition_lines(captured: &str) -> Vec<&str> {
    captured
        .lines()
        .filter(|line| line.contains(RECORD_PREFIX_TAG))
        .collect()
}

/// 種別語 `kind=<kind>` を持つ行だけを取り出す。
fn lines_of_kind<'a>(captured: &'a str, kind: &str) -> Vec<&'a str> {
    let needle = format!("kind={kind} ");
    transition_lines(captured)
        .into_iter()
        .filter(|line| line.contains(&needle))
        .collect()
}

/// 行に載るハンドル表現（`transition_diag` の `hwnd_field` と同じ書式）。
fn hwnd_field(hwnd: HWND) -> String {
    format!("hwnd=0x{:X}", hwnd.0 as usize)
}

// ---------------------------------------------------------------------------
// ヘッドレス配送（実 HWND 不要）
// ---------------------------------------------------------------------------

/// `WM_DPICHANGED` メッセージを組む（LOWORD=X DPI / HIWORD=Y DPI・LPARAM=提案矩形）。
fn dpichanged_message(hwnd: HWND, new_dpi: u16, suggested: &RECT) -> WindowMessage {
    WindowMessage {
        hwnd,
        msg: WM_DPICHANGED,
        wparam: WPARAM(((new_dpi as usize) << 16) | new_dpi as usize),
        lparam: LPARAM(suggested as *const RECT as isize),
    }
}

/// 政策 component の有無を変えて `WM_DPICHANGED` を 1 回配送し、観測出力を返す。
fn capture_dpichanged(directives: &str, policy: Option<DpiSuggestedRectPolicy>) -> String {
    clean_slate();
    let world = Rc::new(RefCell::new(EcsWorld::new()));
    let entity = {
        let mut w = world.borrow_mut();
        let mut e = w.world_mut().spawn(DPI::from_dpi(96, 96));
        if let Some(p) = policy {
            e.insert(p);
        }
        e.id()
    };

    let suggested = RECT {
        left: 3210,
        top: 140,
        right: 3810,
        bottom: 620,
    };
    let m = dpichanged_message(HWND(std::ptr::null_mut()), 192, &suggested);
    let out = capture_under_filter(directives, || {
        emit_control();
        let _ = crate::ecs::dispatch_window_message(&world, entity, &m);
    });
    clean_slate();
    out
}

/// `WM_WINDOWPOSCHANGED` を 1 回配送し、観測出力を返す（窓書込の**外側**での受理）。
fn capture_windowposchanged(directives: &str) -> String {
    clean_slate();
    let world = Rc::new(RefCell::new(EcsWorld::new()));
    let entity = world.borrow_mut().world_mut().spawn(()).id();

    let windowpos = WINDOWPOS {
        hwnd: HWND(std::ptr::null_mut()),
        hwndInsertAfter: HWND(std::ptr::null_mut()),
        x: 100,
        y: 200,
        cx: 640,
        cy: 480,
        flags: SET_WINDOW_POS_FLAGS(0),
    };
    let m = WindowMessage {
        hwnd: HWND(std::ptr::null_mut()),
        msg: WM_WINDOWPOSCHANGED,
        wparam: WPARAM(0),
        lparam: LPARAM(&windowpos as *const WINDOWPOS as isize),
    };
    let out = capture_under_filter(directives, || {
        emit_control();
        let _ = crate::ecs::dispatch_window_message(&world, entity, &m);
    });
    clean_slate();
    out
}

// ---------------------------------------------------------------------------
// メッセージ受理の記録
// ---------------------------------------------------------------------------

#[test]
fn dpichanged_receipt_is_recorded_with_handle_and_flush_context() {
    let _serialized = crate::ecs::window::lock_self_initiated_for_test();
    let captured = capture_dpichanged(SIGNOFF_DIRECTIVES, None);
    assert!(
        captured.contains(CONTROL_LINE),
        "対照が拾えていない＝捕捉が死んでいる: {captured}"
    );

    let msgs = lines_of_kind(&captured, KIND_MSG);
    assert_eq!(msgs.len(), 1, "受理 1 回につき 1 行: {captured}");
    let line = msgs[0];
    assert!(
        line.contains(&format!("msg={MSG_DPICHANGED} ")),
        "メッセージ名が載る: {line}"
    );
    assert!(
        line.contains(&hwnd_field(HWND(std::ptr::null_mut()))),
        "対象ハンドルが載る: {line}"
    );
    assert!(
        line.contains("in_swp=false"),
        "窓書込の外側で受理したので in_swp は偽: {line}"
    );
    assert!(
        line.contains("since_flush_us=-"),
        "一括書込の外で受理したので経過は番兵: {line}"
    );
}

#[test]
fn windowposchanged_receipt_is_recorded_outside_a_window_write() {
    let _serialized = crate::ecs::window::lock_self_initiated_for_test();
    let captured = capture_windowposchanged(SIGNOFF_DIRECTIVES);
    assert!(
        captured.contains(CONTROL_LINE),
        "対照が拾えていない＝捕捉が死んでいる: {captured}"
    );

    let msgs = lines_of_kind(&captured, KIND_MSG);
    assert_eq!(msgs.len(), 1, "受理 1 回につき 1 行: {captured}");
    assert!(
        msgs[0].contains(&format!("msg={MSG_WINDOWPOSCHANGED} ")),
        "メッセージ名が載る: {}",
        msgs[0]
    );
    assert!(
        msgs[0].contains("in_swp=false") && msgs[0].contains("since_flush_us=-"),
        "自前の窓書込を伴わない受理は両フィールドとも「外側」を示す: {}",
        msgs[0]
    );
}

#[test]
fn displaychange_receipt_is_recorded_and_still_delegates_to_defwindowproc() {
    clean_slate();
    let world = Rc::new(RefCell::new(EcsWorld::new()));
    let entity = {
        let mut w = world.borrow_mut();
        w.world_mut().insert_resource(App::new());
        w.world_mut().spawn(()).id()
    };

    let m = WindowMessage {
        hwnd: HWND(0x4321 as *mut _),
        msg: WM_DISPLAYCHANGE,
        wparam: WPARAM(0),
        lparam: LPARAM(0),
    };
    let mut ret: Option<LRESULT> = Some(LRESULT(1));
    let captured = capture_under_filter(SIGNOFF_DIRECTIVES, || {
        emit_control();
        ret = crate::ecs::dispatch_window_message(&world, entity, &m);
    });

    assert!(captured.contains(CONTROL_LINE), "捕捉が死んでいる");
    // 戻り値は不変（`None`＝`DefWindowProcW` へ委譲）。
    assert!(
        ret.is_none(),
        "観測を足したことで WM_DISPLAYCHANGE の戻り値が変わっている（既定手続きへの委譲が失われる）"
    );
    // 既存の業務ロジックも不変。
    assert!(
        world
            .borrow()
            .world()
            .resource::<App>()
            .display_configuration_changed(),
        "表示構成変更フラグが立っていない（既存の挙動を壊している）"
    );

    let msgs = lines_of_kind(&captured, KIND_MSG);
    assert_eq!(msgs.len(), 1, "受理 1 回につき 1 行: {captured}");
    assert!(
        msgs[0].contains(&format!("msg={MSG_DISPLAYCHANGE} "))
            && msgs[0].contains(&hwnd_field(HWND(0x4321 as *mut _))),
        "メッセージ名と対象ハンドルが載る: {}",
        msgs[0]
    );
    clean_slate();
}

// ---------------------------------------------------------------------------
// 採用時の同期書込（`stage=sync`）——採否の判定は変えない
// ---------------------------------------------------------------------------

#[test]
fn applied_suggested_position_is_recorded_as_a_synchronous_write() {
    // 書込を撃つ側なので錠を取る（他テストの `in_swp` 検査を汚さない）。
    let _serialized = crate::ecs::window::lock_self_initiated_for_test();
    // 政策未宣言＝従来どおり OS 提案位置を採る窓（`applied=true`）。
    let captured = capture_dpichanged(
        "info,wintf::ecs::window_proc=debug,wintf::transition=debug",
        None,
    );

    // 採否判定の**既存ログ**と並存すること（design Testing Strategy > Integration Tests 7）。
    assert!(
        captured.contains("[WM_DPICHANGED] suggested position write decision")
            && captured.contains("applied=true"),
        "採否判定の既存ログが失われている（判定を変えていないことの対照）: {captured}"
    );

    let writes = lines_of_kind(&captured, KIND_WRITE);
    assert_eq!(
        writes.len(),
        1,
        "採用した遷移では同期書込が 1 行記録される: {captured}"
    );
    let write = writes[0];
    assert!(
        write.contains(" stage=sync "),
        "一括書込（stage=flush）と区別できる段で記録する: {write}"
    );
    assert!(write.contains("seq=0"), "同期書込の通し番号は 0: {write}");
    assert!(
        write.contains("x=3210 y=140 cx=0 cy=0"),
        "採用した提案原点がそのまま載る（寸は SWP_NOSIZE ゆえ 0）: {write}"
    );
    assert!(
        write.contains(&format!("origin={ORIGIN_DPI_SUGGESTED} scope=- win_kind=-")),
        "経路語は wintf 自身が名乗る（判定側は `path_a_writes` をこの語で数える）。\
         キャラ番号と窓種別は areka の語彙ゆえ番兵: {write}"
    );
    assert_ne!(
        ORIGIN_DPI_SUGGESTED, MISSING,
        "経路語が番兵と同じでは「タグの付け忘れ」と区別が付かない"
    );
    assert!(
        write.contains("ok=false"),
        "null ハンドルへの書込は失敗する: {write}"
    );
    assert!(
        write.contains("ax=- ay=- aw=- ah=-"),
        "読み戻せない窓の書込後矩形は 4 フィールドとも番兵: {write}"
    );
    assert!(write.contains("call_us="), "1 回の所要が載る: {write}");
}

/// 経路語は 3 つとも互いに異なり、番兵とも異なる——判定側が `origin` で経路を数える
/// （`path_a_writes` は `origin=dpi-suggested`）ための前提。
#[test]
fn the_synchronous_write_origin_is_distinct_from_the_enqueue_site_origins() {
    use crate::ecs::window::transition_diag::{ORIGIN_WINDOW_POS, ORIGIN_ZORDER_PAIR};

    let words = [ORIGIN_DPI_SUGGESTED, ORIGIN_WINDOW_POS, ORIGIN_ZORDER_PAIR];
    let mut sorted = words.to_vec();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(sorted.len(), words.len(), "経路語が互いに異なっていない");
    for word in words {
        assert_ne!(word, MISSING, "経路語が番兵と同じ: {word}");
        assert!(
            !word.contains(' '),
            "経路語に空白があると `名前=値` の辞書化が壊れる: {word}"
        );
    }
}

#[test]
fn a_declined_suggested_position_records_the_receipt_but_no_write() {
    let captured = capture_dpichanged(
        "info,wintf::ecs::window_proc=debug,wintf::transition=debug",
        Some(DpiSuggestedRectPolicy::ExternalAuthority),
    );

    // 採否判定の既存ログは「書かない」のまま（判定を変えていない・Requirement 10.2）。
    assert!(
        captured.contains("applied=false"),
        "位置権威が外部にある窓の採否判定が変わっている（`dpi-window-vanish` の規約）: {captured}"
    );
    assert_eq!(
        lines_of_kind(&captured, KIND_MSG).len(),
        1,
        "書かない窓でも受理そのものは記録される: {captured}"
    );
    assert!(
        lines_of_kind(&captured, KIND_WRITE).is_empty(),
        "書かなかったのに書込レコードが出ている: {captured}"
    );
}

// ---------------------------------------------------------------------------
// 既定 OFF
// ---------------------------------------------------------------------------

#[test]
fn message_handlers_emit_nothing_under_the_default_directives() {
    // 提案位置を採る配送が書込経路を通るので錠を取る。
    let _serialized = crate::ecs::window::lock_self_initiated_for_test();
    for captured in [
        capture_dpichanged(DEFAULT_DIRECTIVES, None),
        capture_windowposchanged(DEFAULT_DIRECTIVES),
    ] {
        assert!(
            captured.contains(CONTROL_LINE),
            "対照が拾えていない＝捕捉が死んでいる: {captured}"
        );
        assert!(
            transition_lines(&captured).is_empty(),
            "観測 target を点けない運転では観測行は 1 行も出ない: {captured}"
        );
    }
}

// ---------------------------------------------------------------------------
// 実窓での再入（`SetWindowPos` の内側での同期受理）
// ---------------------------------------------------------------------------

thread_local! {
    /// テスト窓の wndproc が配送先とする `(World, Entity)`。未設定なら配送しない
    /// （`CreateWindowExW`／`DestroyWindow` が送るメッセージを拾わないため）。
    static DISPATCH_TARGET: RefCell<Option<(Rc<RefCell<EcsWorld>>, Entity)>> =
        const { RefCell::new(None) };
}

/// 本番と同じ配送表（`dispatch_window_message`）へ橋渡しするテスト窓の wndproc。
///
/// `runtime::wndproc_bridge::make_wndproc` と同じ規律（配送表は ECS 層が単一の真実の源・
/// `None` なら既定手続きへ委譲）を最小限で再現する。
unsafe extern "system" fn test_wndproc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    let target = DISPATCH_TARGET.with(|cell| cell.borrow().clone());
    if let Some((world, entity)) = target {
        let message = WindowMessage {
            hwnd,
            msg,
            wparam,
            lparam,
        };
        if let Some(result) = crate::ecs::dispatch_window_message(&world, entity, &message) {
            return result;
        }
    }
    // SAFETY: Win32 境界。既定のウィンドウ手続きへそのまま委譲する。
    unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
}

const TEST_CLASS: PCWSTR = w!("wintf-transition-msg-test");

/// テスト窓クラスを 1 度だけ登録する。
fn register_test_class() {
    use std::sync::Once;
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        // SAFETY: Win32 境界。自プロセスに固有名のクラスを 1 度だけ登録する。
        unsafe {
            let hinstance: HINSTANCE = GetModuleHandleW(None).expect("GetModuleHandleW").into();
            let class = WNDCLASSEXW {
                cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
                lpfnWndProc: Some(test_wndproc),
                hInstance: hinstance,
                lpszClassName: TEST_CLASS,
                ..Default::default()
            };
            assert_ne!(
                RegisterClassExW(&class),
                0,
                "テスト窓クラスの登録に失敗した"
            );
        }
    });
}

/// 0x0 の不可視の道具窓を作る（`SetWindowPos` は不可視窓にも
/// `WM_WINDOWPOSCHANGED` を**同期送達**するので表示は要らない）。
fn create_test_window() -> (HWND, HINSTANCE) {
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    register_test_class();
    // SAFETY: Win32 境界。自プロセス所有の 0x0 窓を生成する。
    unsafe {
        let hinstance: HINSTANCE = GetModuleHandleW(None).expect("GetModuleHandleW").into();
        let hwnd = CreateWindowExW(
            WINDOW_EX_STYLE(WS_EX_TOOLWINDOW.0),
            TEST_CLASS,
            w!("transition-msg"),
            WS_POPUP,
            10,
            10,
            0,
            0,
            None,
            None,
            Some(hinstance),
            None,
        )
        .expect("CreateWindowExW should create a test window");
        (hwnd, hinstance)
    }
}

/// **一括書込の内側での同期受理**が `in_swp=1` と非番兵の `since_flush_us` で残ること。
///
/// これが本仕様が追っている機序そのもの（再観測 §5＝各窓の最初の `SetWindowPos` の
/// 内側で当該窓の `WM_DPICHANGED` が同期処理される 24/24）の**測り口**である。
/// 実窓が要るがメッセージポンプは要らない——`SetWindowPos` は同一スレッドの窓へ
/// 同期 send するため、`flush` の呼出がそのまま handler の呼出になる。
///
/// あわせて **flush 区間の入れ子**（`command.rs::flush` の doc）を実配線で確かめる:
/// handler の手順③が `flush_window_pos_commands()` を無条件に呼ぶため区間は入れ子に
/// なるが、内側の解除が起点を消していれば `since_flush_us` が番兵に落ちて本檻が赤になる。
#[test]
fn windowposchanged_received_inside_a_window_write_carries_in_swp_and_flush_elapsed() {
    // 一括書込を撃ちながら `in_swp` を読む——持ち上げる側と読む側の両方であり、錠は必須。
    let _serialized = crate::ecs::window::lock_self_initiated_for_test();
    clean_slate();
    let (hwnd, instance) = create_test_window();

    let world = Rc::new(RefCell::new(EcsWorld::new()));
    let entity = world
        .borrow_mut()
        .world_mut()
        .spawn((
            WindowHandle { hwnd, instance },
            crate::ecs::window::WindowPos::default(),
            DPI::from_dpi(96, 96),
        ))
        .id();
    DISPATCH_TARGET.with(|cell| *cell.borrow_mut() = Some((world.clone(), entity)));

    let captured = capture_under_filter(SIGNOFF_DIRECTIVES, || {
        emit_control();
        SetWindowPosCommand::enqueue(SetWindowPosCommand::new(
            hwnd,
            30,
            40,
            0,
            0,
            SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
            None,
        ));
        crate::ecs::window::flush_window_pos_commands();
    });

    // 配送を止めてから窓を壊す（`DestroyWindow` のメッセージを拾わない）。
    DISPATCH_TARGET.with(|cell| *cell.borrow_mut() = None);
    // SAFETY: Win32 境界。自プロセスが生成した窓を破棄する。
    unsafe {
        let _ = DestroyWindow(hwnd);
    }
    clean_slate();

    assert!(captured.contains(CONTROL_LINE), "捕捉が死んでいる");

    // 探針の自己検査: 一括書込が実際に走っている（走っていなければ以下は空虚）。
    let lines = transition_lines(&captured);
    let begin_at = lines
        .iter()
        .position(|l| l.contains("kind=flush ") && l.contains("stage=begin"))
        .unwrap_or_else(|| panic!("一括書込の開始行が無い: {captured}"));
    let write_at = lines
        .iter()
        .position(|l| l.contains("kind=write "))
        .unwrap_or_else(|| panic!("書込レコードが無い: {captured}"));

    let msg_at = lines
        .iter()
        .position(|l| {
            l.contains("kind=msg ") && l.contains(&format!("msg={MSG_WINDOWPOSCHANGED} "))
        })
        .unwrap_or_else(|| {
            panic!("`SetWindowPos` が同期送達した受理が記録されていない: {captured}")
        });
    let line = lines[msg_at];

    assert!(
        line.contains(&hwnd_field(hwnd)),
        "受理行が対象の実窓を指していない: {line}"
    );
    assert!(
        line.contains("in_swp=true"),
        "`SetWindowPos` の内側での受理なのに in_swp が偽（同期送達の証跡が消えている）: {line}"
    );
    assert!(
        !line.contains("since_flush_us=-"),
        "一括書込の内側での受理なのに経過が番兵（入れ子の解除が外側の起点まで消している）: {line}"
    );

    // 時系列: 開始 → 受理（`SetWindowPos` の内側）→ 書込レコード。
    assert!(
        begin_at < msg_at && msg_at < write_at,
        "受理が一括書込の区間の内側に並んでいない（begin={begin_at} msg={msg_at} write={write_at}）: {captured}"
    );
}

// ---------------------------------------------------------------------------
// 前置ガード（構造で固定する——出力では示せない）
// ---------------------------------------------------------------------------

/// 説明文（`//` で始まる行）を落とし、コードだけの本文を返す。
fn code_only(source: &str) -> String {
    source
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn message_handlers_keep_the_front_guard_so_the_default_run_pays_no_observation_cost() {
    // 「既定の directive で 1 行も出ない」は前置ガードを外しても成立してしまう
    // ——`emit_line` の水準（debug）を subscriber が濾すからである。よって
    // **費用を払わないこと**（行の組立・`Instant` の読み取り・`GetWindowRect` の
    // 読み戻し）は出力では示せず、構造で固定するほかない。
    //
    // ガードそのものの真偽が directive に追随することは
    // `transition_diag_tests::is_enabled_follows_the_directive` が実濾過で示している。
    let code = code_only(include_str!("window_pos.rs"));

    // 記録を組む点は 3 つ（`WM_DPICHANGED` の受理・`WM_WINDOWPOSCHANGED` の受理・
    // 採用時の同期書込）。刻印を読む回数とガードの回数が一致していなければ、
    // どれかがガードの外に出ている。
    assert_eq!(
        code.matches("transition_diag::is_enabled()").count(),
        3,
        "前置ガードの数が記録点の数と食い違う——ガードされていない記録点がある"
    );
    assert_eq!(
        code.matches("transition_diag::stamp()").count(),
        3,
        "刻印を読む回数が記録点の数と食い違う"
    );
    assert_eq!(
        code.matches("transition_diag::since_flush_us()").count(),
        2,
        "受理レコードは 2 点（`WM_DPICHANGED`・`WM_WINDOWPOSCHANGED`）だけが経過を読む"
    );
    // 同期書込の計時と読み戻しはガード変数の内側だけ。
    assert!(
        code.contains("let observe = transition_diag::is_enabled();"),
        "同期書込の前置ガードが失われている——既定運転でも計時と読み戻しの費用を払う"
    );
    assert!(
        code.contains("observe.then(Instant::now)"),
        "同期書込の計時がガードの外で行われている"
    );
    assert_eq!(
        code.matches("Instant::now").count(),
        1,
        "本文に `Instant::now` が 2 度以上ある（ガードされていない計時が混ざり得る）"
    );
    assert_eq!(
        code.matches("read_back_window_rect(").count(),
        1,
        "書込後矩形の読み戻しの呼出が 1 箇所でない（ガードの外から呼ばれ得る）"
    );

    // 走査が中身を見ている対照——落とし過ぎていないこと。
    assert!(
        code.contains("pub(super) fn WM_WINDOWPOSCHANGED("),
        "説明文を落とす処理が本文まで落としている"
    );

    // 表示構成変更の受理も同じ形（ガードの内側で 1 回だけ組む）。
    let lifecycle = code_only(include_str!("lifecycle.rs"));
    assert_eq!(
        lifecycle.matches("transition_diag::is_enabled()").count(),
        1,
        "`WM_DISPLAYCHANGE` の記録点の前置ガードが 1 つでない"
    );
    assert_eq!(
        lifecycle.matches("transition_diag::stamp()").count(),
        1,
        "`WM_DISPLAYCHANGE` が刻印を読む回数が 1 でない"
    );
    assert!(
        lifecycle.contains("pub(super) fn WM_DISPLAYCHANGE("),
        "説明文を落とす処理が本文まで落としている"
    );
}

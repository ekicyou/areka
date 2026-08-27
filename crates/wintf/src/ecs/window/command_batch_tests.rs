//! 一括書込の**バッチ投入**（`Begin/Defer/EndDeferWindowPos`）に対する決定論テスト。
//!
//! 設計 C8 の候補 B-2b（task 7.1 で採用・実装は task 7.2）の対テストである。固定するのは
//! 5 つ。
//!
//! - **1 バッチであること**——1 区間の書込がすべて 1 回の `Begin`／`End` の内側で投入され、
//!   書込レコードが `in_batch=true` を名乗ること（設計 C8 の「7.3 の対テスト」列の決定論側）。
//! - **適用順と引数が変わらないこと**（要件 10.3）——Z 専用指令は合流の対象外だが、
//!   `DeferWindowPos` の per-window 引数として**同じ並びのまま同居**する。純関数
//!   [`window_pos_args`](super::window_pos_args) が 7 項目をそのまま運ぶことと、実窓に
//!   対する Z 指令の並びがバッチでも 1 本ずつでも**同じ最終順序**になることの両方を見る。
//! - **自発書込の判定が生きること**（要件 10.2）——`EndDeferWindowPos` が同期送達する
//!   `WM_WINDOWPOSCHANGED` の内側で `is_self_initiated()` が真であること。偽になると
//!   `dpi-window-vanish` が確立した位置権威が壊れる。
//! - **縮退経路**——バッチが使えないときは 1 本ずつの `SetWindowPos` へ落ち、`warn!` と
//!   `in_batch=false` の両方が残ること（無音の失敗経路を作らない）。
//! - **理由語**——縮退の 3 つの出所が別々の語で名乗ること。
//!
//! # 実窓を使う理由と作法
//!
//! `DeferWindowPos` は実ハンドルでしか成功しないので、バッチ側の主張には実窓が要る
//! （偽ハンドルの経路は「縮退が起きること」の側で使う）。窓は 0x0 の不可視の道具窓で、
//! メッセージポンプは要らない——`EndDeferWindowPos` は同一スレッドの窓へ同期 send する。
//!
//! 一括書込が持ち上げる `SELF_INITIATED_DEPTH` は**スレッドごとに独立**である
//! （`command.rs` の `thread_local!`）。持ち上げ・同期送達・判定・解放はすべて `flush` を
//! 呼んだこのテストスレッドの内側で閉じるので、並列に走る他テストの `is_self_initiated()`
//! ／`in_swp` 検査には見えない。よって本ファイルのテストは直列化の錠を取らない。
//!
//! # 「1 バッチ」を主張するときの対
//!
//! 「バッチで投入された」を見るテストには、同じテスト本体の内側に**縮退した側**を置く
//! ——札が定数で真になっているだけで緑になる形にしない。

use std::cell::RefCell;

use windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, GW_HWNDNEXT, GetTopWindow, GetWindow, IsWindow,
    RegisterClassExW, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER, SWP_SHOWWINDOW,
    WINDOW_EX_STYLE, WM_WINDOWPOSCHANGED, WNDCLASSEXW, WS_EX_TOOLWINDOW, WS_POPUP,
};
use windows::core::{PCWSTR, w};

use super::super::transition_diag::{self, KIND_FLUSH, KIND_WRITE, RECORD_PREFIX_TAG};
use super::{
    BatchDegrade, SetWindowPosCommand, WindowPosArgs, drain_window_pos_commands,
    flush_window_pos_commands, is_self_initiated, window_pos_args, with_forced_batch_begin_failure,
};
use crate::ecs::test_support::capture_under_filter;

/// 観測チャネルを点灯させる directive（実機サインオフと同じ）。
const SIGNOFF_DIRECTIVES: &str = "info,wintf::transition=debug";

/// 捕捉が生きている証拠として同じ窓の中で必ず出す対照行。
const CONTROL_LINE: &str = "[transition-probe] alive";

fn emit_control() {
    tracing::info!(target: transition_diag::TRANSITION_TARGET, "{CONTROL_LINE}");
}

/// テスト冒頭の初期化——キューの残りと刻印の写しを落とす（要件 7.7）。
fn clean_slate() {
    let _residue = drain_window_pos_commands();
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

/// 実在し得ない窓ハンドル（`command_transition_tests` と同じ作法）。
fn fake_hwnd(tag: u8) -> HWND {
    let value = 0xFFFF_FE00_usize | (usize::from(tag) << 1) | 1;
    let hwnd = HWND(value as *mut core::ffi::c_void);
    // SAFETY: `IsWindow` は任意のハンドル値に対して安全に真偽を返す読み取り専用 API。
    assert!(
        !unsafe { IsWindow(Some(hwnd)) }.as_bool(),
        "偽ハンドル 0x{value:X} が実窓を掴んでいる"
    );
    hwnd
}

// ---------------------------------------------------------------------------
// テスト窓（0x0・不可視・既定手続き）
// ---------------------------------------------------------------------------

thread_local! {
    /// `WM_WINDOWPOSCHANGED` を受けたときの `is_self_initiated()` の値（受理順）。
    static SELF_INITIATED_ON_POSCHANGED: RefCell<Vec<bool>> = const { RefCell::new(Vec::new()) };
    /// 上を記録するかどうか（窓の生成・破棄が送るメッセージを拾わないための札）。
    static RECORDING: RefCell<bool> = const { RefCell::new(false) };
}

/// 既定手続きへ委譲しつつ、`WM_WINDOWPOSCHANGED` の受理時点の自発判定だけを控える wndproc。
unsafe extern "system" fn test_wndproc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if msg == WM_WINDOWPOSCHANGED && RECORDING.with(|cell| *cell.borrow()) {
        let observed = is_self_initiated();
        SELF_INITIATED_ON_POSCHANGED.with(|cell| cell.borrow_mut().push(observed));
    }
    // SAFETY: Win32 境界。既定のウィンドウ手続きへそのまま委譲する。
    unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
}

const TEST_CLASS: PCWSTR = w!("wintf-command-batch-test");

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

/// 0x0 の不可視の道具窓を作る。
fn create_test_window() -> HWND {
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    register_test_class();
    // SAFETY: Win32 境界。自プロセス所有の 0x0 窓を生成する。
    unsafe {
        let hinstance: HINSTANCE = GetModuleHandleW(None).expect("GetModuleHandleW").into();
        CreateWindowExW(
            WINDOW_EX_STYLE(WS_EX_TOOLWINDOW.0),
            TEST_CLASS,
            w!("command-batch"),
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
        .expect("CreateWindowExW should create a test window")
    }
}

fn destroy(windows: &[HWND]) {
    for hwnd in windows {
        // SAFETY: Win32 境界。自プロセスが生成した窓を破棄する。
        unsafe {
            let _ = DestroyWindow(*hwnd);
        }
    }
}

/// 合流の対象になり得る素直なジオメトリ指令。
fn geometry_command(hwnd: HWND, x: i32, y: i32, w: i32, h: i32) -> SetWindowPosCommand {
    SetWindowPosCommand::new(hwnd, x, y, w, h, SWP_NOZORDER | SWP_NOACTIVATE, None)
}

/// Z のみを動かす指令（合流の対象外・要件 10.3）。
fn zorder_command(hwnd: HWND, after: HWND) -> SetWindowPosCommand {
    SetWindowPosCommand::new(
        hwnd,
        0,
        0,
        0,
        0,
        SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
        Some(after),
    )
}

// ---------------------------------------------------------------------------
// 引数は 1 つも作り替えない（要件 10.3・純関数）
// ---------------------------------------------------------------------------

#[test]
fn window_pos_args_carries_all_seven_arguments_verbatim() {
    let hwnd = fake_hwnd(0x10);
    let after = fake_hwnd(0x11);

    // ジオメトリ指令。
    let geometry = geometry_command(hwnd, 11, 22, 33, 44);
    assert_eq!(
        window_pos_args(&geometry),
        WindowPosArgs {
            hwnd,
            hwnd_insert_after: None,
            x: 11,
            y: 22,
            cx: 33,
            cy: 44,
            flags: SWP_NOZORDER | SWP_NOACTIVATE,
        }
    );

    // Z 専用指令——挿入位置と、合流できないフラグ組がそのまま運ばれる。
    let zorder = zorder_command(hwnd, after);
    assert_eq!(
        window_pos_args(&zorder),
        WindowPosArgs {
            hwnd,
            hwnd_insert_after: Some(after),
            x: 0,
            y: 0,
            cx: 0,
            cy: 0,
            flags: SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
        }
    );

    // 表示状態を変えるフラグも落とさない（知らないフラグを黙って捨てない）。
    let show = SetWindowPosCommand::new(hwnd, 1, 2, 3, 4, SWP_SHOWWINDOW, None);
    assert_eq!(window_pos_args(&show).flags, SWP_SHOWWINDOW);

    // 観測専用タグは引数へ 1 つも漏れない（design.md D3）。
    let tagged = geometry_command(hwnd, 11, 22, 33, 44).with_tag(transition_diag::WriteTag {
        origin: transition_diag::ORIGIN_ZORDER_PAIR,
        scope: Some(3),
        kind: "shell",
    });
    assert_eq!(
        window_pos_args(&tagged),
        window_pos_args(&geometry),
        "要求語彙タグが引数を動かしている"
    );
}

#[test]
fn the_queue_maps_to_the_same_arguments_in_the_same_order() {
    // 仕切りを含む並び——同一窓のジオメトリ・Z 専用・別窓のジオメトリ・同一窓の
    // ジオメトリ（合流では Z 指令が仕切りになる形）。
    let a = fake_hwnd(0x12);
    let b = fake_hwnd(0x13);
    let queue = [
        geometry_command(a, 1, 2, 3, 4),
        zorder_command(a, b),
        geometry_command(b, 5, 6, 7, 8),
        geometry_command(a, 9, 10, 11, 12),
    ];

    let applied: Vec<WindowPosArgs> = queue.iter().map(window_pos_args).collect();

    assert_eq!(applied.len(), queue.len(), "件数が変わっている");
    for (index, (args, cmd)) in applied.iter().zip(queue.iter()).enumerate() {
        assert_eq!(args.hwnd, cmd.hwnd, "{index} 番目の対象窓");
        assert_eq!(
            args.hwnd_insert_after, cmd.hwnd_insert_after,
            "{index} 番目の挿入位置"
        );
        assert_eq!(args.flags, cmd.flags, "{index} 番目のフラグ");
        assert_eq!(
            (args.x, args.y, args.cx, args.cy),
            (cmd.x, cmd.y, cmd.width, cmd.height),
            "{index} 番目の矩形"
        );
    }
    // Z 専用指令の位置が変わっていない（並べ替えの検出）。
    assert_eq!(
        applied
            .iter()
            .position(|args| args.hwnd_insert_after.is_some()),
        Some(1),
        "Z 専用指令が並びの中で動いている: {applied:?}"
    );
}

// ---------------------------------------------------------------------------
// 縮退の理由語
// ---------------------------------------------------------------------------

#[test]
fn each_degrade_reason_has_its_own_word() {
    let words = [
        BatchDegrade::BeginFailed.as_str(),
        BatchDegrade::DeferFailed { seq: 2 }.as_str(),
        BatchDegrade::EndFailed.as_str(),
    ];
    let mut unique = words.to_vec();
    unique.sort_unstable();
    unique.dedup();
    assert_eq!(
        unique.len(),
        words.len(),
        "縮退の理由語が重複している: {words:?}"
    );
}

// ---------------------------------------------------------------------------
// 1 バッチであること／縮退したこと（同一本体の対）
// ---------------------------------------------------------------------------

#[test]
fn a_flush_of_real_windows_is_submitted_as_one_batch_and_says_so() {
    clean_slate();
    let first = create_test_window();
    let second = create_test_window();

    let captured = capture_under_filter(SIGNOFF_DIRECTIVES, || {
        emit_control();
        SetWindowPosCommand::enqueue(geometry_command(first, 100, 110, 40, 30));
        SetWindowPosCommand::enqueue(geometry_command(second, 200, 210, 50, 60));
        SetWindowPosCommand::enqueue(zorder_command(second, first));
        flush_window_pos_commands();
    });

    // 縮退した側の対——同じ指令列を、バッチを開けない状態で流す。
    let degraded = capture_under_filter(SIGNOFF_DIRECTIVES, || {
        emit_control();
        SetWindowPosCommand::enqueue(geometry_command(first, 100, 110, 40, 30));
        SetWindowPosCommand::enqueue(geometry_command(second, 200, 210, 50, 60));
        SetWindowPosCommand::enqueue(zorder_command(second, first));
        with_forced_batch_begin_failure(flush_window_pos_commands);
    });

    destroy(&[first, second]);
    clean_slate();

    assert!(captured.contains(CONTROL_LINE), "捕捉が死んでいる");

    // 区間はちょうど 1 組（開始 1・終了 1）＝入れ子でない 1 バッチ。
    let flushes = lines_of_kind(&captured, KIND_FLUSH);
    assert_eq!(flushes.len(), 2, "区間は開始と終了の 2 行だけ: {captured}");
    assert!(
        flushes[0].contains("stage=begin") && flushes[0].contains("count=3"),
        "開始行の指令数: {}",
        flushes[0]
    );
    assert!(
        flushes[1].contains("stage=end") && flushes[1].contains("count=3"),
        "終了行の指令数: {}",
        flushes[1]
    );

    // 3 件とも 1 バッチで投入され、成功している。
    let writes = lines_of_kind(&captured, KIND_WRITE);
    assert_eq!(writes.len(), 3, "指令 1 件につき書込 1 行: {captured}");
    for (index, line) in writes.iter().enumerate() {
        assert!(
            line.contains(&format!("seq={index} ")),
            "{index} 番目の通し番号が並びと違う: {line}"
        );
        assert!(
            line.contains(" in_batch=true"),
            "1 バッチで投入した書込が in_batch=true を名乗っていない: {line}"
        );
        assert!(
            line.contains("ok=true"),
            "実窓への書込が失敗している: {line}"
        );
    }

    // 対: 縮退した側は同じ 3 件が in_batch=false で、理由が warn に残る。
    let degraded_writes = lines_of_kind(&degraded, KIND_WRITE);
    assert_eq!(degraded_writes.len(), 3, "縮退でも書込は 3 行: {degraded}");
    for line in &degraded_writes {
        assert!(
            line.contains(" in_batch=false"),
            "縮退した書込が in_batch=false を名乗っていない: {line}"
        );
        assert!(
            line.contains("ok=true"),
            "縮退でも実窓への書込は成功する: {line}"
        );
    }
    assert!(
        degraded.contains(BatchDegrade::BeginFailed.as_str()),
        "縮退が無音である（理由語が残っていない）: {degraded}"
    );
}

#[test]
fn an_invalid_handle_discards_the_batch_and_degrades_to_one_write_per_command() {
    clean_slate();
    let first = fake_hwnd(0x20);
    let second = fake_hwnd(0x21);

    let captured = capture_under_filter(SIGNOFF_DIRECTIVES, || {
        emit_control();
        SetWindowPosCommand::enqueue(geometry_command(first, 1, 1, 1, 1));
        SetWindowPosCommand::enqueue(geometry_command(second, 2, 2, 2, 2));
        flush_window_pos_commands();
    });
    clean_slate();

    assert!(captured.contains(CONTROL_LINE), "捕捉が死んでいる");
    assert!(
        captured.contains(BatchDegrade::DeferFailed { seq: 0 }.as_str()),
        "`DeferWindowPos` の失敗が記録されていない: {captured}"
    );
    assert!(
        !captured.contains(BatchDegrade::BeginFailed.as_str()),
        "確保は成功しているのに確保失敗の理由が出ている: {captured}"
    );

    let writes = lines_of_kind(&captured, KIND_WRITE);
    assert_eq!(
        writes.len(),
        2,
        "縮退しても指令の件数だけ書込が出る: {captured}"
    );
    for line in &writes {
        assert!(line.contains(" in_batch=false"), "縮退した書込の札: {line}");
        assert!(
            line.contains("ok=false"),
            "偽ハンドルへの書込は失敗する: {line}"
        );
    }
}

// ---------------------------------------------------------------------------
// 実施ログ（O9）の本数——1 指令につき 1 行
// ---------------------------------------------------------------------------

/// 診断手順書（`dpi-window-vanish` の O9）と `zorder_pair_*_tests.rs` の 16 本が
/// **本数**を数えている実施ログの水準。
const WRITE_LOG_DIRECTIVES: &str = "info,wintf::ecs::window=debug";

/// 実施ログ（O9）の行を数える。
fn write_log_lines<'a>(captured: &'a str, via: Option<&str>) -> Vec<&'a str> {
    captured
        .lines()
        .filter(|line| line.contains(super::WINDOW_WRITE_LOG_MESSAGE))
        .filter(|line| via.is_none_or(|via| line.contains(&format!("via=\"{via}\""))))
        .collect()
}

/// **実施していない書込を名乗らない**——O9 の行は経路によらず指令 1 件につき 1 行。
///
/// バッチ経路は積み上げ（`DeferWindowPos`）の時点ではまだ書いていない。積み上げのたびに
/// 実施ログを出すと、`Defer` が失敗して 1 本ずつへ縮退したときに**破棄されたバッチの指令が
/// 「書いた」と名乗り**、書き直した本番の行と二重に数えられる。この行の本数は既存 16 本の
/// 檻と診断手順 O9 が測っている量なので、狂っても実窓の檻は今日は赤にならない——
/// **静かに壊れる形**であり、本檻がその 1 点を押さえる。
#[test]
fn the_write_log_line_is_emitted_once_per_command_on_both_paths() {
    clean_slate();
    let first = create_test_window();
    let second = create_test_window();

    // ⑴ バッチが成る経路。
    let batched = capture_under_filter(WRITE_LOG_DIRECTIVES, || {
        SetWindowPosCommand::enqueue(geometry_command(first, 120, 130, 40, 30));
        SetWindowPosCommand::enqueue(geometry_command(second, 220, 230, 50, 60));
        flush_window_pos_commands();
    });

    // ⑵ バッチを開けずに縮退する経路（実窓なので書込そのものは成功する）。
    let degraded = capture_under_filter(WRITE_LOG_DIRECTIVES, || {
        SetWindowPosCommand::enqueue(geometry_command(first, 121, 131, 40, 30));
        SetWindowPosCommand::enqueue(geometry_command(second, 221, 231, 50, 60));
        with_forced_batch_begin_failure(flush_window_pos_commands);
    });

    destroy(&[first, second]);

    // ⑶ 積み上げの途中で失敗してバッチが**破棄**される経路（レビューが二重計上を見つけた形）。
    let discarded = capture_under_filter(WRITE_LOG_DIRECTIVES, || {
        SetWindowPosCommand::enqueue(geometry_command(fake_hwnd(0x30), 1, 1, 1, 1));
        SetWindowPosCommand::enqueue(geometry_command(fake_hwnd(0x31), 2, 2, 2, 2));
        flush_window_pos_commands();
    });
    clean_slate();

    assert_eq!(
        write_log_lines(&batched, None).len(),
        2,
        "バッチ経路の実施ログが指令数と違う: {batched}"
    );
    assert_eq!(
        write_log_lines(&batched, Some("DeferWindowPos")).len(),
        2,
        "バッチ経路の行が呼んだ API を名乗っていない: {batched}"
    );

    assert_eq!(
        write_log_lines(&degraded, None).len(),
        2,
        "縮退経路の実施ログが指令数と違う: {degraded}"
    );
    assert_eq!(
        write_log_lines(&degraded, Some("SetWindowPos")).len(),
        2,
        "縮退経路の行が呼んだ API を名乗っていない: {degraded}"
    );

    assert_eq!(
        write_log_lines(&discarded, None).len(),
        2,
        "破棄されたバッチの指令が「書いた」と名乗っている（O9 の本数が二重になる）: {discarded}"
    );
    assert_eq!(
        write_log_lines(&discarded, Some("DeferWindowPos")).len(),
        0,
        "適用されなかった積み上げが実施ログを残している: {discarded}"
    );

    // 探針の較正——`via=` の絞り込みが恒真でないこと（絞れば必ず減る側があること）。
    assert_eq!(
        write_log_lines(&batched, Some("SetWindowPos")).len(),
        0,
        "`via=` の絞り込みが効いていない（部分一致で両方拾っている）: {batched}"
    );
}

// ---------------------------------------------------------------------------
// 自発書込の判定（要件 10.2）
// ---------------------------------------------------------------------------

#[test]
fn the_batch_keeps_the_self_initiated_guard_over_the_messages_it_sends() {
    clean_slate();
    let hwnd = create_test_window();

    SELF_INITIATED_ON_POSCHANGED.with(|cell| cell.borrow_mut().clear());
    RECORDING.with(|cell| *cell.borrow_mut() = true);
    SetWindowPosCommand::enqueue(geometry_command(hwnd, 60, 70, 20, 25));
    flush_window_pos_commands();
    let batched = SELF_INITIATED_ON_POSCHANGED.with(|cell| cell.borrow().clone());

    // 対: 1 本ずつの経路でも同じ判定になる（バッチだけが特別ではない）。
    SELF_INITIATED_ON_POSCHANGED.with(|cell| cell.borrow_mut().clear());
    SetWindowPosCommand::enqueue(geometry_command(hwnd, 80, 90, 20, 25));
    with_forced_batch_begin_failure(flush_window_pos_commands);
    let sequential = SELF_INITIATED_ON_POSCHANGED.with(|cell| cell.borrow().clone());
    RECORDING.with(|cell| *cell.borrow_mut() = false);

    destroy(&[hwnd]);
    clean_slate();

    assert!(
        !batched.is_empty(),
        "`EndDeferWindowPos` が `WM_WINDOWPOSCHANGED` を送っていない（探針が空虚）"
    );
    assert!(
        batched.iter().all(|observed| *observed),
        "バッチの送るメッセージが外部由来と判定されている（位置権威が壊れる）: {batched:?}"
    );
    assert!(!sequential.is_empty(), "縮退側でも受理はある");
    assert!(
        sequential.iter().all(|observed| *observed),
        "1 本ずつの経路で自発判定が偽になった: {sequential:?}"
    );

    // 探針そのものの較正——ガードの外での受理は偽になる。
    assert!(
        !is_self_initiated(),
        "一括書込の外側なのに自発判定が真のまま（錠が効いていない）"
    );
}

// ---------------------------------------------------------------------------
// Z 順の結果が変わらない（要件 10.3・実窓）
// ---------------------------------------------------------------------------

/// 与えた窓集合だけを Z の上から下へ並べて返す。
///
/// 最前面から `GW_HWNDNEXT` で降りながら、集合に属する窓だけを拾う。**生の 1 歩では
/// 測らない**——既定の IME 窓のような不可視の隣が間に挟まるので、隣接ではなく順序で見る。
fn relative_z_order(windows: &[HWND]) -> Vec<HWND> {
    let mut result = Vec::new();
    // SAFETY: Win32 境界。デスクトップ配下の最前面窓を得る読み取り専用 API。
    let mut cursor = unsafe { GetTopWindow(None) }.ok();
    let mut steps = 0usize;
    while let Some(hwnd) = cursor {
        if hwnd.is_invalid() {
            break;
        }
        if windows.contains(&hwnd) && !result.contains(&hwnd) {
            result.push(hwnd);
            if result.len() == windows.len() {
                break;
            }
        }
        steps += 1;
        // 走査が終わらない事態（別プロセスの窓が増え続ける等）で固まらない保険。
        if steps > 100_000 {
            break;
        }
        // SAFETY: Win32 境界。窓ハンドルに対する読み取り専用の走査。
        cursor = unsafe { GetWindow(hwnd, GW_HWNDNEXT) }.ok();
    }
    result
}

/// 組内の並びを**添字の列**（上から下）で表す。
fn z_shape(set: &[HWND]) -> Vec<usize> {
    relative_z_order(set)
        .iter()
        .filter_map(|hwnd| set.iter().position(|w| w == hwnd))
        .collect()
}

/// 組の相対順を `0 → 1 → 2`（上から下）へ揃える。
///
/// 生成順に依る初期 Z を当てにしないための助走。**測る対象ではない**ので 1 本ずつ書く。
fn normalize_z(set: &[HWND]) {
    for index in 1..set.len() {
        SetWindowPosCommand::enqueue(zorder_command(set[index], set[index - 1]));
    }
    with_forced_batch_begin_failure(flush_window_pos_commands);
}

#[test]
fn a_batched_zorder_list_lands_in_the_same_order_as_one_write_per_command() {
    clean_slate();

    let batch_set: Vec<HWND> = (0..3).map(|_| create_test_window()).collect();
    let sequential_set: Vec<HWND> = (0..3).map(|_| create_test_window()).collect();
    normalize_z(&batch_set);
    normalize_z(&sequential_set);
    let batch_start = z_shape(&batch_set);
    let sequential_start = z_shape(&sequential_set);

    // **並べ替えに敏感な**指令列でなければ順序の主張にならない。
    // 初期 `0,1,2`（上→下）に対し ⑴「0 を 2 の後ろへ」→ `1,2,0` ⑵「1 を 0 の後ろへ」→ `2,0,1`。
    // 逆順に適用すると ⑵ が空振り（1 は既に 0 の直後ではない＝動く）して `1,2,0` に着き、
    // 結果が変わる——だから逆順の変異をこの檻が殺せる。
    const EXPECTED: [usize; 3] = [2, 0, 1];
    let program = |set: &[HWND]| {
        SetWindowPosCommand::enqueue(zorder_command(set[0], set[2]));
        SetWindowPosCommand::enqueue(zorder_command(set[1], set[0]));
    };

    program(&batch_set);
    flush_window_pos_commands();
    let batched = z_shape(&batch_set);

    program(&sequential_set);
    with_forced_batch_begin_failure(flush_window_pos_commands);
    let sequential = z_shape(&sequential_set);

    destroy(&batch_set);
    destroy(&sequential_set);
    clean_slate();

    // 助走の自己検査——始点が揃っていなければ以下の比較は空虚。
    assert_eq!(
        batch_start,
        vec![0, 1, 2],
        "バッチ側の始点が揃っていない: {batch_start:?}"
    );
    assert_eq!(
        sequential_start,
        vec![0, 1, 2],
        "1 本ずつ側の始点が揃っていない: {sequential_start:?}"
    );

    assert_eq!(
        batched,
        EXPECTED.to_vec(),
        "バッチ投入の最終 Z 順序が逐次適用の結果と違う（要件 10.3）"
    );
    assert_eq!(
        sequential,
        EXPECTED.to_vec(),
        "1 本ずつの書込の最終 Z 順序が期待と違う（対照が壊れている）"
    );
    assert_eq!(
        batched, sequential,
        "バッチ投入と 1 本ずつの書込で Z の最終順序が食い違う（要件 10.3）"
    );
}

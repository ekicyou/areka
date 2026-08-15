//! 窓書込指令（`command.rs`）の**遷移観測**に対する決定論的テスト。
//!
//! 固定するのは 4 つである。
//!
//! - **要求語彙タグ**——`SetWindowPosCommand` が `WriteTag` を運び、`new()` の 7 引数は
//!   変わらないこと。タグを付けない生成は番兵（`UNTAGGED`）になり、「付け忘れた経路」が
//!   行の上で見分けられること。
//! - **一括書込の 3 種**——`flush` が `flush stage=begin`／各指令の `write stage=flush`／
//!   `flush stage=end` を発行すること。**偽の窓ハンドル**（`SetWindowPos` が必ず失敗する）
//!   でも 3 種とも出て、`ok=false` になること。
//! - **検査用の口**——`drain_window_pos_commands` が積まれた指令を**実行せずに**そのまま
//!   返し、キューを空にすること。
//! - **既定 OFF**——観測 target を点けていない運転では `flush` が観測行を 1 行も出さず、
//!   読み戻し（`GetWindowRect`）も計時も行わないこと。
//!
//! # 実行形態の明示（要件 7.6）
//!
//! 本ファイルの検証はすべて**テストスレッド上の直接呼出**である。`flush` は bevy の
//! スケジュール実行器を介さないため、`tracing` のスレッド局所 subscriber
//! （[`capture_under_filter`]）で確実に捕捉できる。多スレッド実行器の相での「出ないこと」は
//! 一切主張しない。
//!
//! 加えて「出ない」を見るテストには、同じ捕捉窓の内側に必ず拾えるはずの**対照行**を置く
//! ——捕捉が死んでいるだけで緑になる形にしない。
//!
//! # 一括書込を呼ぶテストは直列化する（要件 7.7）
//!
//! `flush` は `guarded_set_window_pos` を通り、**プロセス共有**の `SELF_INITIATED_DEPTH` を
//! 一時的に持ち上げる。並列に走る他テストの `is_self_initiated()`／`in_swp` 検査がそれを
//! 読むと偽の失敗になるため、一括書込を呼ぶテストは
//! [`lock_self_initiated_for_test`](super::lock_self_initiated_for_test) を取る
//! （経緯と実測はその doc を参照）。
//!
//! # 偽の窓ハンドルの安全性
//!
//! `HWND` はプロセス横断の識別子であり、値を当てずっぽうに撃つと**他プロセスの実窓**を
//! 動かしかねない。よって偽ハンドルは ⑴ 64bit Windows で `HWND` が 32bit 値の符号拡張で
//! あることを利用して符号拡張されない値を選び、⑵ 使う直前に `IsWindow` で実窓でないことを
//! 毎回確かめる。⑵ が破れたときはテストが失敗する（黙って実窓を動かさない）。

use bevy_ecs::prelude::*;
use bevy_ecs::schedule::ExecutorKind;
use windows::Win32::Foundation::{HINSTANCE, HWND};
use windows::Win32::UI::WindowsAndMessaging::{
    IsWindow, SET_WINDOW_POS_FLAGS, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER,
};

use super::super::transition_diag::{
    self, KIND_ENQUEUE, KIND_FLUSH, KIND_WRITE, MISSING, ORIGIN_WINDOW_POS, ORIGIN_ZORDER_PAIR,
    RECORD_PREFIX_TAG, TRANSITION_TARGET, WriteTag,
};
use super::super::zorder_pair::InsertSpec;
use super::super::zorder_pair_maintain::pair_fix_command;
use super::super::{Window, WindowHandle, WindowPos};
use super::{SetWindowPosCommand, drain_window_pos_commands, flush_window_pos_commands};
use crate::ecs::test_support::capture_under_filter;
use crate::ecs::{Point, SizeI, apply_window_pos_changes};

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

/// テスト冒頭の初期化——キューの残りと刻印の写しを落とす（要件 7.7）。
fn clean_slate() {
    let _residue = drain_window_pos_commands();
    transition_diag::reset_for_test();
}

/// 実在し得ない窓ハンドルを作る。
///
/// 64bit Windows の `HWND` は 32bit 値の**符号拡張**であり、`0xFFFF_FF01` のように上位
/// 32bit が 0 のまま下位が負域にある値はハンドル表に存在し得ない。それでも他プロセスの
/// 実窓を掴んでいないことを `IsWindow` で毎回確かめる。
fn fake_hwnd(tag: u8) -> HWND {
    // 下位に識別子を載せ、必ず奇数にする（ハンドル値の粒度に載らない）。
    let value = 0xFFFF_FF00_usize | usize::from(tag) | 1;
    let hwnd = HWND(value as *mut core::ffi::c_void);
    // SAFETY: `IsWindow` は任意のハンドル値に対して安全に真偽を返す読み取り専用 API で
    // あり、無効値でも未定義動作を起こさない。
    assert!(
        !unsafe { IsWindow(Some(hwnd)) }.as_bool(),
        "偽ハンドル 0x{value:X} が実窓を掴んでいる——このまま書込を撃つと無関係の窓を動かす"
    );
    hwnd
}

/// 合流の対象になり得る素直なジオメトリ指令（Z も表示状態も動かさない）。
fn geometry_command(hwnd: HWND, x: i32, y: i32, w: i32, h: i32) -> SetWindowPosCommand {
    SetWindowPosCommand::new(hwnd, x, y, w, h, SWP_NOZORDER | SWP_NOACTIVATE, None)
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

// ---------------------------------------------------------------------------
// 要求語彙タグ
// ---------------------------------------------------------------------------

#[test]
fn new_defaults_to_the_untagged_write_tag() {
    let cmd = geometry_command(fake_hwnd(0x10), 1, 2, 3, 4);
    assert_eq!(
        cmd.tag,
        WriteTag::UNTAGGED,
        "既存の 7 引数生成はタグ無し＝番兵になる"
    );
}

#[test]
fn with_tag_replaces_the_tag_and_keeps_the_seven_construction_arguments() {
    let hwnd = fake_hwnd(0x11);
    let after = fake_hwnd(0x12);
    let tag = WriteTag {
        origin: ORIGIN_ZORDER_PAIR,
        scope: Some(1),
        kind: "balloon",
    };
    let cmd =
        SetWindowPosCommand::new(hwnd, 10, 20, 300, 400, SWP_NOACTIVATE, Some(after)).with_tag(tag);

    assert_eq!(cmd.tag, tag);
    // 7 引数の中身は 1 つも動かない。
    assert_eq!(cmd.hwnd, hwnd);
    assert_eq!(cmd.x, 10);
    assert_eq!(cmd.y, 20);
    assert_eq!(cmd.width, 300);
    assert_eq!(cmd.height, 400);
    assert_eq!(cmd.flags, SWP_NOACTIVATE);
    assert_eq!(cmd.hwnd_insert_after, Some(after));
}

#[test]
fn the_two_display_layer_enqueue_sites_carry_distinct_origins() {
    assert_ne!(
        ORIGIN_ZORDER_PAIR, ORIGIN_WINDOW_POS,
        "積み上げ元 2 点は行の上で区別できなければならない"
    );
    assert_ne!(ORIGIN_ZORDER_PAIR, MISSING);
    assert_ne!(ORIGIN_WINDOW_POS, MISSING);
}

#[test]
fn the_zorder_maintenance_site_tags_its_commands_without_changing_them() {
    let target = fake_hwnd(0x60);
    let cmd = pair_fix_command(target, InsertSpec::TopEdge);

    assert_eq!(cmd.tag.origin, ORIGIN_ZORDER_PAIR);
    assert_eq!(
        cmd.tag.scope, None,
        "スコープは表示基盤側では判らないので番兵"
    );
    assert_eq!(cmd.tag.kind, MISSING, "窓種別も同じく番兵");

    // 指令そのものは Z のみのまま——位置・寸法・活性化は伴わない（要件 10.3）。
    assert_eq!(cmd.hwnd, target);
    assert_eq!((cmd.x, cmd.y, cmd.width, cmd.height), (0, 0, 0, 0));
    assert_eq!(cmd.flags, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
}

#[test]
fn the_generic_window_pos_site_tags_the_command_it_enqueues() {
    clean_slate();
    let hwnd = fake_hwnd(0x61);

    let mut world = World::new();
    world.spawn((
        Window::default(),
        WindowHandle {
            hwnd,
            instance: HINSTANCE::default(),
        },
        WindowPos {
            position: Some(Point { x: 10, y: 20 }),
            size: Some(SizeI {
                width: 100,
                height: 50,
            }),
            ..Default::default()
        },
    ));

    // 単一スレッド実行に固定する——積み上げ先はスレッド局所のキューであり、
    // ワーカースレッドへ載ると本スレッドからは取り出せない（要件 7.6）。
    let mut schedule = Schedule::default();
    schedule.set_executor_kind(ExecutorKind::SingleThreaded);
    schedule.add_systems(apply_window_pos_changes);
    schedule.run(&mut world);

    let drained = drain_window_pos_commands();
    assert_eq!(drained.len(), 1, "位置反映は指令を 1 件積む");
    assert_eq!(drained[0].hwnd, hwnd);
    assert_eq!(drained[0].tag.origin, ORIGIN_WINDOW_POS);
    assert_eq!(drained[0].tag.scope, None);
    assert_eq!(drained[0].tag.kind, MISSING);
}

// ---------------------------------------------------------------------------
// 検査用の口（実行せずに取り出す）
// ---------------------------------------------------------------------------

#[test]
fn drain_returns_the_enqueued_commands_verbatim_without_executing_them() {
    clean_slate();

    let first = fake_hwnd(0x20);
    let second = fake_hwnd(0x21);
    let tag = WriteTag {
        origin: ORIGIN_WINDOW_POS,
        scope: Some(0),
        kind: "shell",
    };

    // 取り出しが書込を伴わないことを、観測を点けた窓の中で確かめる
    // （書いていれば `kind=write` が必ず出る）。
    let captured = capture_under_filter(SIGNOFF_DIRECTIVES, || {
        SetWindowPosCommand::enqueue(geometry_command(first, 1, 2, 3, 4).with_tag(tag));
        SetWindowPosCommand::enqueue(geometry_command(second, 5, 6, 7, 8));

        let drained = drain_window_pos_commands();
        assert_eq!(drained.len(), 2, "積んだ 2 件がそのまま返る");

        // 1 件目——タグも矩形も enqueue した値のまま。
        assert_eq!(drained[0].hwnd, first);
        assert_eq!(drained[0].x, 1);
        assert_eq!(drained[0].y, 2);
        assert_eq!(drained[0].width, 3);
        assert_eq!(drained[0].height, 4);
        assert_eq!(drained[0].flags, SWP_NOZORDER | SWP_NOACTIVATE);
        assert_eq!(drained[0].hwnd_insert_after, None);
        assert_eq!(drained[0].tag, tag);

        // 2 件目——順序は enqueue 順のまま、タグ無しは番兵。
        assert_eq!(drained[1].hwnd, second);
        assert_eq!(drained[1].x, 5);
        assert_eq!(drained[1].tag, WriteTag::UNTAGGED);

        // 取り出した後のキューは空。
        assert!(drain_window_pos_commands().is_empty());
        emit_control();
    });

    assert!(
        captured.contains(CONTROL_LINE),
        "対照が拾えていない＝捕捉が死んでいる: {captured}"
    );
    assert!(
        lines_of_kind(&captured, KIND_WRITE).is_empty(),
        "検査用の口は指令を実行しない（書込レコードが 1 行も出てはならない）: {captured}"
    );
}

#[test]
fn drain_and_reset_leave_no_residue_for_the_following_test() {
    clean_slate();
    SetWindowPosCommand::enqueue(geometry_command(fake_hwnd(0x22), 9, 9, 9, 9));

    clean_slate();

    assert!(
        drain_window_pos_commands().is_empty(),
        "初期化のあとにキューの残りがあってはならない（要件 7.7）"
    );
    assert_eq!(transition_diag::current_frame(), 0);
    assert!(transition_diag::since_flush_us().is_none());
}

// ---------------------------------------------------------------------------
// 一括書込の観測（begin / write / end）
// ---------------------------------------------------------------------------

#[test]
fn flush_emits_begin_write_and_end_for_a_fake_window_handle() {
    let _serialized = super::lock_self_initiated_for_test();
    clean_slate();
    let hwnd = fake_hwnd(0x30);

    let captured = capture_under_filter(SIGNOFF_DIRECTIVES, || {
        emit_control();
        SetWindowPosCommand::enqueue(geometry_command(hwnd, 11, 22, 33, 44).with_tag(WriteTag {
            origin: ORIGIN_ZORDER_PAIR,
            scope: Some(2),
            kind: "shell",
        }));
        SetWindowPosCommand::flush();
    });

    assert!(
        captured.contains(CONTROL_LINE),
        "対照が拾えていない＝捕捉が死んでいる: {captured}"
    );

    let flush_lines = lines_of_kind(&captured, KIND_FLUSH);
    assert_eq!(flush_lines.len(), 2, "開始と終了の 2 行: {captured}");
    assert!(
        flush_lines[0].contains("stage=begin") && flush_lines[0].contains("count=1"),
        "開始行に指令数が載る: {}",
        flush_lines[0]
    );
    assert!(
        flush_lines[0].contains("total_us=-"),
        "開始行の総所要は番兵: {}",
        flush_lines[0]
    );
    assert!(
        flush_lines[1].contains("stage=end") && !flush_lines[1].contains("total_us=-"),
        "終了行に総所要が載る: {}",
        flush_lines[1]
    );

    let write_lines = lines_of_kind(&captured, KIND_WRITE);
    assert_eq!(write_lines.len(), 1, "指令 1 件につき書込レコード 1 行");
    let write = write_lines[0];
    assert!(write.contains("stage=flush"), "経路は一括 flush: {write}");
    assert!(write.contains("seq=0"), "通し番号は 0 始まり: {write}");
    assert!(
        write.contains(&format!("hwnd=0x{:X}", hwnd.0 as usize)),
        "対象ハンドルが載る: {write}"
    );
    assert!(
        write.contains(&format!(
            "origin={ORIGIN_ZORDER_PAIR} scope=2 win_kind=shell"
        )),
        "要求語彙タグの 3 フィールドが載る: {write}"
    );
    assert!(
        write.contains("x=11 y=22 cx=33 cy=44"),
        "要求矩形が載る: {write}"
    );
    assert!(write.contains("flags=0x"), "フラグが載る: {write}");
    assert!(write.contains("call_us="), "1 回の所要が載る: {write}");
    assert!(
        write.contains("ok=false"),
        "偽ハンドルへの書込は失敗する: {write}"
    );
    assert!(
        write.contains("ax=- ay=- aw=- ah=-"),
        "読み戻せない窓の書込後矩形は 4 フィールドとも番兵: {write}"
    );
}

#[test]
fn flush_numbers_each_write_in_enqueue_order() {
    let _serialized = super::lock_self_initiated_for_test();
    clean_slate();
    let first = fake_hwnd(0x31);
    let second = fake_hwnd(0x32);

    let captured = capture_under_filter(SIGNOFF_DIRECTIVES, || {
        emit_control();
        SetWindowPosCommand::enqueue(geometry_command(first, 1, 1, 1, 1));
        SetWindowPosCommand::enqueue(geometry_command(second, 2, 2, 2, 2));
        flush_window_pos_commands();
    });

    assert!(captured.contains(CONTROL_LINE), "捕捉が死んでいる");
    let writes = lines_of_kind(&captured, KIND_WRITE);
    assert_eq!(writes.len(), 2, "2 件とも記録される: {captured}");
    assert!(
        writes[0].contains("seq=0")
            && writes[0].contains(&format!("hwnd=0x{:X}", first.0 as usize))
    );
    assert!(
        writes[1].contains("seq=1")
            && writes[1].contains(&format!("hwnd=0x{:X}", second.0 as usize)),
        "通し番号は enqueue 順（10.3 の適用順不変）: {}",
        writes[1]
    );

    let flush_lines = lines_of_kind(&captured, KIND_FLUSH);
    assert!(
        flush_lines[0].contains("count=2"),
        "開始行の指令数: {}",
        flush_lines[0]
    );
}

#[test]
fn flush_of_an_empty_queue_emits_no_records() {
    let _serialized = super::lock_self_initiated_for_test();
    clean_slate();

    let captured = capture_under_filter(SIGNOFF_DIRECTIVES, || {
        emit_control();
        SetWindowPosCommand::flush();
        flush_window_pos_commands();
    });

    assert!(captured.contains(CONTROL_LINE), "捕捉が死んでいる");
    assert!(
        transition_lines(&captured).is_empty(),
        "空の一括書込は 1 行も記録しない: {captured}"
    );
}

#[test]
fn flush_emits_nothing_under_the_default_directives() {
    let _serialized = super::lock_self_initiated_for_test();
    clean_slate();

    let captured = capture_under_filter(DEFAULT_DIRECTIVES, || {
        emit_control();
        SetWindowPosCommand::enqueue(geometry_command(fake_hwnd(0x33), 1, 2, 3, 4));
        SetWindowPosCommand::flush();
    });

    assert!(
        captured.contains(CONTROL_LINE),
        "対照が拾えていない＝捕捉が死んでいる: {captured}"
    );
    assert!(
        transition_lines(&captured).is_empty(),
        "観測 target を点けない運転では観測行は 1 行も出ない: {captured}"
    );
}

#[test]
fn a_nested_flush_epoch_restores_the_outer_one_instead_of_clearing_it() {
    // 一括 flush は**入れ子になる**。`guarded_set_window_pos` の中で `WM_WINDOWPOSCHANGED` が
    // 同期送達され、その処理の手順③（`window_proc/window_pos.rs:268`）が
    // `flush_window_pos_commands()` を無条件に呼ぶためである。内側の解除が起点を消すと、
    // 2 本目以降の書込のあいだ `since_flush_us()` が `None` になり、
    // 「`WM_DPICHANGED` が `SetWindowPos` の内側で受理された」証跡（task 2.4 の `msg`
    // レコード）が失われる——本仕様が追っている当の機序が測れなくなる。
    clean_slate();

    let outer = transition_diag::begin_flush();
    assert!(
        transition_diag::since_flush_us().is_some(),
        "外側の区間が開いていない"
    );
    {
        let _inner = transition_diag::begin_flush();
        assert!(
            transition_diag::since_flush_us().is_some(),
            "内側の区間でも起点はある"
        );
    }
    assert!(
        transition_diag::since_flush_us().is_some(),
        "入れ子の解除が外側の起点まで消している"
    );

    drop(outer);
    assert!(
        transition_diag::since_flush_us().is_none(),
        "最外の解除で起点が残ってはならない（要件 7.7）"
    );
}

#[test]
fn flush_keeps_the_front_guard_so_the_default_run_pays_no_observation_cost() {
    // 「既定の directive で 1 行も出ない」は前置ガードを外しても成立してしまう
    // ——`emit_line` の水準（debug）を subscriber が濾すからである。よって
    // **費用を払わないこと**（行の組立・`GetWindowRect` の読み戻し・`Instant` の読み取り）は
    // 出力では示せず、構造で固定するほかない（要件 10.4＝定常状態のアロケーション 0）。
    //
    // ガードそのものの真偽が directive に追随することは
    // `transition_diag_tests::is_enabled_follows_the_directive` が実濾過で示している。
    // 説明文（`//` で始まる行）を落として本文だけを見る——doc から関数名を参照しても
    // 数え方が狂わないようにするため。
    let code = code_only(include_str!("command.rs"));

    assert!(
        code.contains("let observe = transition_diag::is_enabled();"),
        "一括 flush の前置ガードが失われている——既定運転でも観測の費用を払う形になる"
    );
    // 時刻基準を開くのもガードの内側だけ。`begin_flush` を裸で呼ぶと、既定運転でも
    // flush のたびに `Instant::now()` を読む（module doc の主張と食い違う）。
    assert!(
        code.contains("observe.then(transition_diag::begin_flush)"),
        "flush 区間の時刻基準がガードの外で開かれている"
    );
    assert_eq!(
        code.matches("begin_flush").count(),
        1,
        "本文に `begin_flush` が 2 度以上ある（ガードされていない開き方が混ざり得る）"
    );
    // 読み戻しの呼出も 1 箇所だけ。本文には定義 1 つと呼出 1 つで合計 2 回現れる。
    //
    // これが示すのは「呼出が 1 箇所しかない」ことまでであり、その 1 箇所が `if observe` の
    // 内側にあることは示さない——そこは上の 2 つの assert と実際の本文が担う。
    assert_eq!(
        code.matches("read_back_window_rect(").count(),
        2,
        "読み戻しの定義 1 と呼出 1 以外の出現が増えている（ガードの外から呼ばれ得る）"
    );

    // 走査が中身を見ている対照——落とし過ぎていないこと。
    assert!(
        code.contains("pub fn flush()"),
        "説明文を落とす処理が本文まで落としている"
    );
}

/// 説明文（`//` で始まる行）を落とし、コードだけの本文を返す。
fn code_only(source: &str) -> String {
    source
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

// ---------------------------------------------------------------------------
// 積み上げの記録
// ---------------------------------------------------------------------------

#[test]
fn enqueue_records_the_command_with_the_merge_sentinel() {
    clean_slate();
    let hwnd = fake_hwnd(0x40);

    let captured = capture_under_filter(SIGNOFF_DIRECTIVES, || {
        emit_control();
        SetWindowPosCommand::enqueue(geometry_command(hwnd, 3, 4, 5, 6).with_tag(WriteTag {
            origin: ORIGIN_WINDOW_POS,
            scope: None,
            kind: "shell",
        }));
        let _drained = drain_window_pos_commands();
    });

    assert!(captured.contains(CONTROL_LINE), "捕捉が死んでいる");
    let enqueues = lines_of_kind(&captured, KIND_ENQUEUE);
    assert_eq!(enqueues.len(), 1, "積み上げ 1 件につき 1 行: {captured}");
    assert!(
        enqueues[0].contains(&format!(
            "origin={ORIGIN_WINDOW_POS} scope=- win_kind=shell"
        )),
        "タグの 3 フィールドが載る（scope を持たない窓は番兵）: {}",
        enqueues[0]
    );
    assert!(
        enqueues[0].contains("merged_into_seq=-"),
        "先着の枠へ畳んでいないので合流先は番兵: {}",
        enqueues[0]
    );
}

// ---------------------------------------------------------------------------
// 既存挙動の維持
// ---------------------------------------------------------------------------

#[test]
fn enqueue_keeps_every_command_in_order_including_two_for_the_same_window() {
    clean_slate();
    let hwnd = fake_hwnd(0x50);

    // 寸のみ → 位置のみ（同一窓）。積み上げは指令をそのまま保つ。
    SetWindowPosCommand::enqueue(SetWindowPosCommand::new(
        hwnd,
        0,
        0,
        100,
        200,
        SWP_NOMOVE | SWP_NOZORDER | SWP_NOACTIVATE,
        None,
    ));
    SetWindowPosCommand::enqueue(SetWindowPosCommand::new(
        hwnd,
        10,
        20,
        0,
        0,
        SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
        None,
    ));

    let drained = drain_window_pos_commands();
    assert_eq!(drained.len(), 2, "積み上げは指令を畳まない");
    assert_eq!(drained[0].width, 100);
    assert_eq!(drained[1].x, 10);
}

#[test]
fn zero_flags_command_still_round_trips_through_the_queue() {
    clean_slate();
    let hwnd = fake_hwnd(0x51);
    SetWindowPosCommand::enqueue(SetWindowPosCommand::new(
        hwnd,
        0,
        0,
        0,
        0,
        SET_WINDOW_POS_FLAGS(0),
        None,
    ));
    let drained = drain_window_pos_commands();
    assert_eq!(drained.len(), 1);
    assert_eq!(drained[0].flags, SET_WINDOW_POS_FLAGS(0));
    assert_eq!(drained[0].tag, WriteTag::UNTAGGED);
}

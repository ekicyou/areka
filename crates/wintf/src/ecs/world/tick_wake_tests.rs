//! `tick_wake` の決定論テスト。
//!
//! 旗はプロセス共有だが、判定の中身は [`Wake`] という素の構造体に閉じている。ほとんどの
//! テストは自分専用の [`Wake::new`] を作って調べるので、並列に走る他のテストの書き換えが
//! 見えることはない。プロセス共有の入口（[`mark`]／[`take`]）を通す 1 本だけは錠で直列化し、
//! 「立てた旗が含まれる」ことだけを見る（他所が同時に立てた旗まで否定しない）。
//!
//! 実時間の閾値は 1 つも使わない（要件 6.5）。期限の検査は「今」を引数で与える合成時刻
//! だけで組み、`sleep` を一切挟まない。

use super::*;

use std::collections::BTreeSet;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use windows::Win32::UI::Controls::WM_MOUSELEAVE;
use windows::Win32::UI::WindowsAndMessaging::{
    WM_DPICHANGED, WM_ERASEBKGND, WM_KEYDOWN, WM_MOUSEMOVE, WM_NCMOUSELEAVE, WM_PAINT, WM_TIMER,
    WM_USER, WM_WINDOWPOSCHANGED,
};

/// プロセス共有の旗を触るテストを互いに直列化する錠。
static GLOBAL_TEST_LOCK: Mutex<()> = Mutex::new(());

/// 合成時刻の基点。実時刻より十分に先へ置くことで、内部の基準時刻（プロセス開始時に
/// 一度だけ確定する）より必ず後になり、飽和（基準時刻より前は 0 に丸める）に当たらない。
fn base() -> Instant {
    Instant::now() + Duration::from_secs(3600)
}

fn ms(n: u64) -> Duration {
    Duration::from_millis(n)
}

// ---------------------------------------------------------------- 旗のビット

#[test]
fn ten_flags_are_distinct_single_bits_in_declaration_order() {
    let expected: [WakeBits; 10] = [
        POINTER,
        DRAG,
        WINDOW_CMD,
        ZORDER,
        WM_GEOMETRY,
        PRESENT,
        ANIM,
        REARM,
        GRAPHICS,
        FORCE,
    ];

    for (index, flag) in expected.iter().enumerate() {
        assert_eq!(
            flag.bits().count_ones(),
            1,
            "旗はビット 1 本のはず: index={index} bits={:#x}",
            flag.bits()
        );
        assert_eq!(
            flag.bits(),
            1u32 << index,
            "旗の並びは宣言順に bit 0..9 のはず: index={index}"
        );
    }

    let distinct: BTreeSet<u32> = expected.iter().map(|f| f.bits()).collect();
    assert_eq!(distinct.len(), 10, "10 本の旗はすべて異なるビットのはず");
}

#[test]
fn all_table_lists_every_flag_with_a_unique_name() {
    assert_eq!(ALL.len(), 10, "名前表は 10 件のはず");

    let names: BTreeSet<&str> = ALL.iter().map(|(name, _)| *name).collect();
    assert_eq!(names.len(), 10, "名前は重複しないはず: {ALL:?}");

    let bits: BTreeSet<u32> = ALL.iter().map(|(_, flag)| flag.bits()).collect();
    assert_eq!(bits.len(), 10, "名前表のビットは重複しないはず");

    assert_eq!(ALL[0].0, "POINTER", "名前表の先頭は POINTER のはず");
    assert_eq!(ALL[9].0, "FORCE", "名前表の末尾は FORCE のはず");
    for (index, (name, flag)) in ALL.iter().enumerate() {
        assert_eq!(
            flag.bits(),
            1u32 << index,
            "名前表の並びはビットの並びと一致するはず: {name}"
        );
    }
}

#[test]
fn none_is_empty_and_contains_is_a_subset_test() {
    assert_eq!(WakeBits::NONE.bits(), 0, "NONE は空のはず");

    let both = POINTER | DRAG;
    assert!(both.contains(POINTER), "OR した旗は両方を含むはず");
    assert!(both.contains(DRAG), "OR した旗は両方を含むはず");
    assert!(!both.contains(ZORDER), "立てていない旗は含まないはず");

    let mut acc = WakeBits::NONE;
    acc |= PRESENT;
    acc |= PRESENT;
    assert_eq!(acc, PRESENT, "同じ旗を 2 度立てても結果は同じはず（冪等）");
}

// ------------------------------------------------------------ 立てる／倒す

#[test]
fn mark_then_take_returns_the_bits_and_clears_them() {
    let wake = Wake::new();
    let now = base();

    wake.mark(PRESENT);
    wake.mark(ZORDER);

    let first = wake.take(now);
    assert_eq!(
        first.bits,
        (PRESENT | ZORDER).bits(),
        "立てた旗がそのまま読めるはず"
    );
    assert!(
        !first.deadline_due,
        "期限を入れていないので到来はしないはず"
    );

    let second = wake.take(now);
    assert_eq!(second.bits, 0, "一度読んだら倒れているはず");
    assert!(second.is_empty(), "旗も期限も無ければ空のはず");
}

#[test]
fn a_flag_marked_right_after_take_is_seen_by_the_following_take() {
    let wake = Wake::new();
    let now = base();

    wake.mark(ANIM);
    assert_eq!(wake.take(now).bits, ANIM.bits(), "倒す前の旗が読めるはず");

    wake.mark(POINTER);
    assert_eq!(
        wake.take(now).bits,
        POINTER.bits(),
        "倒した直後に立てた旗は次の読み取りで取れるはず"
    );
}

#[test]
fn a_flag_marked_on_another_thread_is_seen_by_the_next_take() {
    let wake = Wake::new();
    let now = base();

    std::thread::scope(|scope| {
        scope.spawn(|| {
            wake.mark(PRESENT);
        });
    });

    let snapshot = wake.take(now);
    assert_eq!(
        snapshot.bits,
        PRESENT.bits(),
        "別スレッドで立てた旗が次の読み取りで見えるはず"
    );
}

#[test]
fn every_flag_marked_from_its_own_thread_survives_into_one_take() {
    let wake = Wake::new();
    let now = base();
    let flags: Vec<WakeBits> = ALL.iter().map(|(_, flag)| *flag).collect();

    let shared = &wake;
    std::thread::scope(|scope| {
        for flag in &flags {
            scope.spawn(move || shared.mark(*flag));
        }
    });

    let snapshot = wake.take(now);
    assert_eq!(
        snapshot.bits,
        (1u32 << 10) - 1,
        "10 スレッドが同時に立てた旗は 1 本も落ちないはず"
    );
}

#[test]
fn the_process_wide_entry_points_round_trip_a_flag() {
    let _guard = GLOBAL_TEST_LOCK.lock().expect("錠は毒化しないはず");
    let now = base();

    let _ = take(now);
    mark(GRAPHICS);
    let snapshot = take(now);

    assert!(
        WakeBits(snapshot.bits).contains(GRAPHICS),
        "プロセス共有の入口でも立てた旗が読めるはず: bits={:#x}",
        snapshot.bits
    );
}

// -------------------------------------------------------------------- 期限

#[test]
fn no_deadline_is_never_due() {
    let wake = Wake::new();
    let snapshot = wake.take(base() + ms(1000));
    assert!(!snapshot.deadline_due, "期限が無ければ到来はしないはず");
}

#[test]
fn the_earliest_deadline_is_kept_and_fires_once() {
    let wake = Wake::new();
    let base = base();

    wake.arm_deadline(base + ms(50));
    wake.arm_deadline(base + ms(10));

    let before = wake.take(base + ms(9));
    assert!(!before.deadline_due, "期限より前では到来しないはず");

    let at = wake.take(base + ms(10));
    assert!(at.deadline_due, "期限ちょうどで到来するはず");

    let after = wake.take(base + ms(11));
    assert!(!after.deadline_due, "到来した期限は倒れているはず");
}

#[test]
fn a_later_deadline_does_not_push_back_an_earlier_one() {
    let wake = Wake::new();
    let base = base();

    wake.arm_deadline(base + ms(10));
    wake.arm_deadline(base + ms(50));

    assert!(
        wake.take(base + ms(10)).deadline_due,
        "後から入れた遅い期限に上書きされないはず"
    );
}

#[test]
fn a_pending_deadline_survives_a_take_that_is_too_early() {
    let wake = Wake::new();
    let base = base();

    wake.arm_deadline(base + ms(20));
    assert!(!wake.take(base).deadline_due, "まだ到来しないはず");
    assert!(!wake.take(base + ms(19)).deadline_due, "まだ到来しないはず");
    assert!(
        wake.take(base + ms(20)).deadline_due,
        "早すぎる読み取りで期限が消えてはいけない"
    );
}

#[test]
fn a_deadline_armed_before_the_reference_point_is_due_immediately() {
    let wake = Wake::new();

    // 基準時刻より前の時刻は 0 に丸める（飽和）ので、必ず到来済みとして扱われる。
    wake.arm_deadline(Instant::now() - Duration::from_secs(3600));
    assert!(
        wake.take(Instant::now()).deadline_due,
        "過ぎた期限は次の読み取りで到来として出るはず"
    );
}

#[test]
fn a_deadline_alone_makes_the_snapshot_non_empty() {
    let wake = Wake::new();
    let base = base();

    wake.arm_deadline(base);
    let snapshot = wake.take(base);
    assert_eq!(snapshot.bits, 0, "旗は立てていないはず");
    assert!(!snapshot.is_empty(), "期限の到来だけでも空ではないはず");
}

// ------------------------------------------------------------------ 写像表

#[test]
fn the_known_message_table_has_no_duplicate_messages_and_no_empty_bits() {
    let messages: BTreeSet<u32> = KNOWN_MESSAGE_TABLE.iter().map(|(msg, _)| *msg).collect();
    assert_eq!(
        messages.len(),
        KNOWN_MESSAGE_TABLE.len(),
        "既知メッセージの表に重複があってはいけない"
    );

    for (msg, expected) in KNOWN_MESSAGE_TABLE {
        assert_ne!(
            *expected,
            WakeBits::NONE,
            "既知メッセージが「旗なし」に落ちてはいけない: msg={msg:#x}"
        );
    }
}

#[test]
fn every_known_message_maps_to_the_bits_listed_in_the_table() {
    for (msg, expected) in KNOWN_MESSAGE_TABLE {
        assert_eq!(
            wake_bits_for_message(*msg),
            *expected,
            "写像表と純関数が食い違っている: msg={msg:#x}"
        );
        assert_ne!(
            wake_bits_for_message(*msg),
            FORCE,
            "既知メッセージが未知扱い（FORCE）に落ちてはいけない: msg={msg:#x}"
        );
    }
}

/// 表に載っていないメッセージが純関数側にだけ生えていないことを確かめる。
///
/// 純関数は `match`、[`KNOWN_MESSAGE_TABLE`] は並びで書かれており、片方だけ直すと食い違う。
/// 前のテストが「表 → 純関数」を見るのに対し、こちらは低位のメッセージ番号を総なめして
/// 「純関数 → 表」を見る（扱うメッセージは最大でも `WM_DPICHANGED`=0x02E0 なので、
/// 0x0000..=0x03FF で全件に届く）。
#[test]
fn no_message_outside_the_table_gets_a_flag_of_its_own() {
    let known: BTreeSet<u32> = KNOWN_MESSAGE_TABLE.iter().map(|(msg, _)| *msg).collect();

    for msg in 0u32..=0x03FF {
        if known.contains(&msg) {
            continue;
        }
        assert_eq!(
            wake_bits_for_message(msg),
            FORCE,
            "表に無いのに固有の旗へ落ちるメッセージがある（表と純関数の食い違い）: msg={msg:#x}"
        );
    }
}

#[test]
fn geometry_messages_map_to_wm_geometry() {
    for msg in [WM_DPICHANGED, WM_WINDOWPOSCHANGED] {
        assert_eq!(
            wake_bits_for_message(msg),
            WM_GEOMETRY,
            "幾何・DPI 系は WM_GEOMETRY のはず: msg={msg:#x}"
        );
    }
}

#[test]
fn pointer_messages_map_to_pointer() {
    for msg in [WM_MOUSEMOVE, WM_MOUSELEAVE, WM_NCMOUSELEAVE] {
        assert_eq!(
            wake_bits_for_message(msg),
            POINTER,
            "ポインタ系は POINTER のはず: msg={msg:#x}"
        );
    }
}

#[test]
fn unknown_messages_map_to_force() {
    for msg in [
        0u32,
        WM_PAINT,
        WM_ERASEBKGND,
        WM_KEYDOWN,
        WM_TIMER,
        WM_USER + 7,
        0xFFFF,
    ] {
        assert_eq!(
            wake_bits_for_message(msg),
            FORCE,
            "表に無いメッセージは FORCE（疑わしいときは回す）のはず: msg={msg:#x}"
        );
    }
}

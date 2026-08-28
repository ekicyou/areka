//! `tick_gate` の決定論テスト。
//!
//! 見るのは純関数 [`should_run`] ただ 1 つで、時計も乱数も共有状態も使わない
//! （実時間の閾値は 1 つも登場しない＝要件 6.5）。
//!
//! 主軸は 2 本立てである。
//!
//! ⒈ **全組合せ**——旗 2^10（1,024 通り）× 期限の有無 2 × 前回実行からのフレーム数 6 点
//!    （0・1・29・30・31・上限）× 起動からのフレーム数 6 点（0・1・599・600・601・上限）
//!    × 門の有効 2 ＝ 147,456 通りを総なめし、**本番とは別に書き下ろした対照実装**
//!    （[`oracle`]）と 1 件ずつ突き合わせる。対照実装は素直な if の連なりで、本番の
//!    書き方に引きずられないよう意図的に別の形にしてある。
//!
//! ⒉ **省略の必要十分条件**——同じ 147,456 通りに対して「省略になるのは、旗が 1 本も
//!    立たず・期限も来ておらず・心拍に届かず・起動直後でもなく・門が有効なとき、
//!    かつそのときだけ」を対照実装を経由せずに直にぶつける。⒈ が対照実装ごと間違って
//!    いても、この 1 本が独立に捕まえる。
//!
//! 残りは優先順位（5 つの理由の上下関係）・境界の値（599／600・29／30）・
//! `Wake` が運ぶ旗の中身・名前の綴り・[`TickGateInputs::from_snapshot`] の写しである。

use super::*;

use crate::ecs::world::tick_wake::{
    ANIM, DRAG, FORCE, GRAPHICS, POINTER, PRESENT, REARM, WINDOW_CMD, WM_GEOMETRY, WakeSnapshot,
    ZORDER,
};

// ------------------------------------------------------------ 総なめの目盛り

/// 前回実行からのフレーム数の検査点（心拍 30 の直前・ちょうど・直後を挟む）。
const SINCE_RUN_POINTS: [u32; 6] = [0, 1, 29, 30, 31, u32::MAX];

/// 起動からのフレーム数の検査点（起動直後 600 の直前・ちょうど・直後を挟む）。
const SINCE_BOOT_POINTS: [u32; 6] = [0, 1, 599, 600, 601, u32::MAX];

/// 旗の全組合せ（10 本ぶん）。
const BITS_COUNT: u32 = 1 << 10;

/// 総なめの一点を作る。
fn inputs(
    bits: u32,
    deadline_due: bool,
    frames_since_run: u32,
    frames_since_boot: u32,
    gate_enabled: bool,
) -> TickGateInputs {
    TickGateInputs {
        bits,
        deadline_due,
        frames_since_run,
        frames_since_boot,
        gate_enabled,
    }
}

/// 総なめの本体（同じ 147,456 通りを 2 つの検査が使う）。
fn for_each_case(mut visit: impl FnMut(TickGateInputs)) {
    for bits in 0..BITS_COUNT {
        for deadline_due in [false, true] {
            for frames_since_run in SINCE_RUN_POINTS {
                for frames_since_boot in SINCE_BOOT_POINTS {
                    for gate_enabled in [false, true] {
                        visit(inputs(
                            bits,
                            deadline_due,
                            frames_since_run,
                            frames_since_boot,
                            gate_enabled,
                        ));
                    }
                }
            }
        }
    }
}

/// 本番とは別に書き下ろした対照実装。
///
/// 優先順位を上から素直に並べただけの形にしてあり、本番の実装を参照しない。数値も
/// 本番の定数を借りずに 30／600 を直に書く（定数を書き換えたら、ここも赤くなって
/// 気づけるようにするため）。
fn oracle(i: &TickGateInputs) -> TickDecision {
    if !i.gate_enabled {
        return TickDecision::Run(RunReason::Disabled);
    }
    if i.frames_since_boot < 600 {
        return TickDecision::Run(RunReason::Warmup);
    }
    if i.bits != 0 {
        return TickDecision::Run(RunReason::Wake(i.bits));
    }
    if i.deadline_due {
        return TickDecision::Run(RunReason::Deadline);
    }
    if i.frames_since_run >= 30 {
        return TickDecision::Run(RunReason::Heartbeat);
    }
    TickDecision::Skip
}

// ================================================================ 全組合せ

#[test]
fn exhaustive_matches_independent_oracle() {
    let mut checked = 0u32;
    for_each_case(|i| {
        assert_eq!(
            should_run(&i),
            oracle(&i),
            "対照実装と食い違った: bits={:#x} deadline_due={} since_run={} since_boot={} enabled={}",
            i.bits,
            i.deadline_due,
            i.frames_since_run,
            i.frames_since_boot,
            i.gate_enabled
        );
        checked += 1;
    });

    // 目盛りを削ってしまったら気づけるように、通った件数そのものを固定する。
    assert_eq!(checked, 1024 * 2 * 6 * 6 * 2, "総なめの件数");
    assert_eq!(checked, 147_456, "総なめの件数（実数）");
}

#[test]
fn exhaustive_skip_holds_exactly_when_nothing_asks_for_a_run() {
    for_each_case(|i| {
        // 対照実装を通さずに、省略の必要十分条件を直にぶつける。
        let should_skip = i.bits == 0
            && !i.deadline_due
            && i.frames_since_run < TICK_HEARTBEAT_FRAMES
            && i.frames_since_boot >= TICK_GATE_WARMUP_FRAMES
            && i.gate_enabled;

        let decision = should_run(&i);
        assert_eq!(
            decision == TickDecision::Skip,
            should_skip,
            "省略の必要十分条件を外れた: bits={:#x} deadline_due={} since_run={} since_boot={} enabled={} decision={decision:?}",
            i.bits,
            i.deadline_due,
            i.frames_since_run,
            i.frames_since_boot,
            i.gate_enabled
        );
        assert_eq!(decision.is_run(), !should_skip, "is_run は Skip の裏返し");
    });
}

// ============================================================== 優先順位

#[test]
fn disabled_outranks_every_other_reason() {
    // 起動直後・旗あり・期限あり・心拍超過を全部同時に立てても、門が無効なら Disabled。
    let i = inputs(FORCE.bits(), true, u32::MAX, 0, false);
    assert_eq!(should_run(&i), TickDecision::Run(RunReason::Disabled));

    // 起動直後を抜けていても同じ。
    let i = inputs(POINTER.bits(), true, u32::MAX, u32::MAX, false);
    assert_eq!(should_run(&i), TickDecision::Run(RunReason::Disabled));

    // 回す理由が 1 つも無くても、門が無効なら省略にはならない。
    let i = inputs(0, false, 0, u32::MAX, false);
    assert_eq!(should_run(&i), TickDecision::Run(RunReason::Disabled));
}

#[test]
fn warmup_outranks_wake_deadline_and_heartbeat() {
    let i = inputs(PRESENT.bits(), true, u32::MAX, 0, true);
    assert_eq!(should_run(&i), TickDecision::Run(RunReason::Warmup));

    // 旗も期限も心拍も無くても、起動直後というだけで回る。
    let i = inputs(0, false, 0, 599, true);
    assert_eq!(should_run(&i), TickDecision::Run(RunReason::Warmup));
}

#[test]
fn wake_outranks_deadline_and_heartbeat() {
    let i = inputs(ANIM.bits(), true, u32::MAX, 600, true);
    assert_eq!(
        should_run(&i),
        TickDecision::Run(RunReason::Wake(ANIM.bits()))
    );
}

#[test]
fn deadline_outranks_heartbeat() {
    let i = inputs(0, true, u32::MAX, 600, true);
    assert_eq!(should_run(&i), TickDecision::Run(RunReason::Deadline));
}

#[test]
fn heartbeat_is_the_last_resort() {
    let i = inputs(0, false, TICK_HEARTBEAT_FRAMES, 600, true);
    assert_eq!(should_run(&i), TickDecision::Run(RunReason::Heartbeat));
}

// ================================================================ 境界

#[test]
fn warmup_boundary_is_599_in_and_600_out() {
    let just_inside = inputs(0, false, 0, TICK_GATE_WARMUP_FRAMES - 1, true);
    assert_eq!(
        should_run(&just_inside),
        TickDecision::Run(RunReason::Warmup),
        "599 フレーム目はまだ起動直後"
    );

    let just_outside = inputs(0, false, 0, TICK_GATE_WARMUP_FRAMES, true);
    assert_eq!(
        should_run(&just_outside),
        TickDecision::Skip,
        "600 フレーム目で起動直後は終わる"
    );
}

#[test]
fn heartbeat_boundary_is_29_skip_and_30_run() {
    let below = inputs(
        0,
        false,
        TICK_HEARTBEAT_FRAMES - 1,
        TICK_GATE_WARMUP_FRAMES,
        true,
    );
    assert_eq!(
        should_run(&below),
        TickDecision::Skip,
        "29 はまだ心拍に届かない"
    );

    let at = inputs(
        0,
        false,
        TICK_HEARTBEAT_FRAMES,
        TICK_GATE_WARMUP_FRAMES,
        true,
    );
    assert_eq!(
        should_run(&at),
        TickDecision::Run(RunReason::Heartbeat),
        "30 でちょうど心拍"
    );
}

#[test]
fn constants_are_fixed() {
    assert_eq!(TICK_HEARTBEAT_FRAMES, 30);
    assert_eq!(TICK_GATE_WARMUP_FRAMES, 600);
}

// ========================================================== 旗の運搬と名前

#[test]
fn wake_carries_the_exact_bits() {
    let bits = (POINTER | PRESENT | ZORDER).bits();
    let i = inputs(bits, false, 0, TICK_GATE_WARMUP_FRAMES, true);
    match should_run(&i) {
        TickDecision::Run(RunReason::Wake(carried)) => assert_eq!(carried, bits),
        other => panic!("Wake を期待した: {other:?}"),
    }
}

#[test]
fn wake_names_joins_flag_names_in_bit_order() {
    assert_eq!(wake_names((POINTER | PRESENT).bits()), "POINTER|PRESENT");
    // 並びはビットの若い順で、渡した順ではない。
    assert_eq!(wake_names((PRESENT | POINTER).bits()), "POINTER|PRESENT");
    assert_eq!(wake_names(FORCE.bits()), "FORCE");
    assert_eq!(
        wake_names((DRAG | WINDOW_CMD | WM_GEOMETRY | ANIM | REARM | GRAPHICS).bits()),
        "DRAG|WINDOW_CMD|WM_GEOMETRY|ANIM|REARM|GRAPHICS"
    );
}

#[test]
fn wake_names_covers_all_ten_flags_and_the_empty_case() {
    assert_eq!(wake_names(0), "NONE");
    assert_eq!(
        wake_names(BITS_COUNT - 1),
        "POINTER|DRAG|WINDOW_CMD|ZORDER|WM_GEOMETRY|PRESENT|ANIM|REARM|GRAPHICS|FORCE"
    );
}

#[test]
fn wake_names_shows_unknown_bits_instead_of_dropping_them() {
    // 表に無いビットを黙って捨てると、ログを読んだ人が「旗ゼロなのに回った」と誤読する。
    assert_eq!(wake_names(1 << 10), "0x400");
    assert_eq!(wake_names(POINTER.bits() | (1 << 11)), "POINTER|0x800");
}

// ============================================================ 入力の組み立て

#[test]
fn from_snapshot_copies_fields_verbatim() {
    let snapshot = WakeSnapshot {
        bits: (POINTER | ANIM).bits(),
        deadline_due: true,
    };
    let i = TickGateInputs::from_snapshot(&snapshot, 7, 1234, true);

    assert_eq!(i.bits, snapshot.bits);
    assert_eq!(i.deadline_due, snapshot.deadline_due);
    assert_eq!(i.frames_since_run, 7);
    assert_eq!(i.frames_since_boot, 1234);
    assert!(i.gate_enabled);

    let empty = WakeSnapshot {
        bits: 0,
        deadline_due: false,
    };
    let i = TickGateInputs::from_snapshot(&empty, 0, u32::MAX, false);
    assert_eq!(i.bits, 0);
    assert!(!i.deadline_due);
    assert_eq!(i.frames_since_run, 0);
    assert_eq!(i.frames_since_boot, u32::MAX);
    assert!(!i.gate_enabled);
}

// ================================================================ 名札

#[test]
fn reason_names_are_fixed() {
    assert_eq!(RunReason::Disabled.as_str(), "disabled");
    assert_eq!(RunReason::Warmup.as_str(), "warmup");
    assert_eq!(RunReason::Wake(POINTER.bits()).as_str(), "wake");
    assert_eq!(RunReason::Wake(0).as_str(), "wake");
    assert_eq!(RunReason::Deadline.as_str(), "deadline");
    assert_eq!(RunReason::Heartbeat.as_str(), "heartbeat");
}

#[test]
fn is_run_distinguishes_run_from_skip() {
    assert!(TickDecision::Run(RunReason::Disabled).is_run());
    assert!(TickDecision::Run(RunReason::Warmup).is_run());
    assert!(TickDecision::Run(RunReason::Wake(0)).is_run());
    assert!(TickDecision::Run(RunReason::Deadline).is_run());
    assert!(TickDecision::Run(RunReason::Heartbeat).is_run());
    assert!(!TickDecision::Skip.is_run());
}

// ================================================== 生産者一覧の字面検査

// ここから下は「旗を立てる側が現に在るか」の字面検査である。判断の中身は見ない
// ——判断の正しさは上の全組合せが見ており、ここが守るのは配線の抜けだけである
// （旗を立て忘れると、門は正しく判断したまま反応しない画面更新を作ってしまう）。

/// 旗を立てる側（wintf 内）の一覧。（見出し・中身・期待する旗の名前）。
///
/// 並びは見出しの辞書順。areka 側の生産者は別クレートなのでここからは読めず、
/// areka 側の同種の検査が受け持つ。
const WINTF_PRODUCERS: [(&str, &str, &str); 10] = [
    ("app.rs", include_str!("../app.rs"), "WM_GEOMETRY"),
    ("dola/mod.rs", include_str!("../dola/mod.rs"), "ANIM"),
    (
        "drag/systems.rs",
        include_str!("../drag/systems.rs"),
        "DRAG",
    ),
    (
        "graphics/systems/init.rs",
        include_str!("../graphics/systems/init.rs"),
        "GRAPHICS",
    ),
    (
        "pointer/buffers.rs",
        include_str!("../pointer/buffers.rs"),
        "POINTER",
    ),
    (
        "window/command.rs",
        include_str!("../window/command.rs"),
        "WINDOW_CMD",
    ),
    (
        "window/zorder_group_maintain.rs",
        include_str!("../window/zorder_group_maintain.rs"),
        "ZORDER",
    ),
    (
        "window/zorder_pair_maintain.rs",
        include_str!("../window/zorder_pair_maintain.rs"),
        "ZORDER",
    ),
    // 配送点は個別の旗ではなく写像表で立てる（旗の名前は次の検査が見る）。
    (
        "window_proc/mod.rs",
        include_str!("../window_proc/mod.rs"),
        "",
    ),
    (
        "window_proc/window_pos.rs",
        include_str!("../window_proc/window_pos.rs"),
        "ZORDER",
    ),
];

/// 註釈の行を落とす——説明文に書いてあるだけの綴りを「在る」と数えないため。
fn code_only(src: &str) -> String {
    src.lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn every_wintf_producer_marks_the_tick_wake() {
    for (label, src, flag) in WINTF_PRODUCERS {
        let code = code_only(src);
        assert!(
            code.contains("tick_wake::mark("),
            "{label}: 旗を立てる呼出（tick_wake::mark(）が無い"
        );
        if !flag.is_empty() {
            assert!(
                code.contains(&format!("tick_wake::{flag}")),
                "{label}: 期待する旗 {flag} を立てていない"
            );
        }
    }
}

/// `pointer/buffers.rs` は投入の入口が 6 つあり、旗を立てるのはその全部である。
///
/// 呼出は私設ヘルパー 1 本に束ねてあるので、`tick_wake::mark(` が在るだけでは
/// 「ヘルパーは残っているが誰も呼んでいない」形を見逃す。呼び元の数を直に数える。
#[test]
fn every_pointer_entry_point_calls_the_wake_helper() {
    let code = code_only(include_str!("../pointer/buffers.rs"));
    let call_sites = code.matches("wake_pointer();").count();
    assert!(
        call_sites >= 6,
        "pointer/buffers.rs: 投入の入口 6 つから旗を立てるはずが {call_sites} 箇所しかない"
    );
}

#[test]
fn window_proc_dispatch_maps_the_message_to_bits() {
    let code = code_only(include_str!("../window_proc/mod.rs"));
    assert!(
        code.contains("tick_wake::wake_bits_for_message("),
        "window_proc/mod.rs: 配送点がメッセージ→旗の写像を通っていない"
    );
}

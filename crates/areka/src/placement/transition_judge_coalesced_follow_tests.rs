//! 合流（C2）を通った随伴の追従を、判定器が**数に入れられる**ことの檻（task 6.6）。
//!
//! # 何が壊れていたか
//!
//! 判定器の裁定⑴ は当初「随伴の同一フレーム性はバルーンの `origin=BalloonFollow` の
//! **書込**で測る」であった。task 5.3 の合流が着地すると、同一 tick・同一窓のジオメトリ
//! 指令は**先着の枠へ畳まれ**、合流後の 1 本は先着の経路語を名乗る
//! （`crates/wintf/src/ecs/window/command.rs` の `merge_into`＝「タグは先着の値を保つ」）。
//! バルーン窓は寸の指令（`KeepPositionResize`）が先に積まれるので、後から積まれた随伴の
//! 位置指令は `kind=write` から消え、`origin=BalloonFollow` の書込が 1 本も残らない。
//!
//! 結果として `balloon_pairs_checked` が 0 になり、要件 4.3 の合否は「違反なし」ではなく
//! **未測定**（`Violation::Unmeasured(Quantity::BalloonSameFrame)`）として落ちた。2026-08-21 の
//! 実機再採取（7 遷移）では 5 遷移が `pairs=0`・残る 2 遷移も片側だけが測れており、キャラ窓
//! ごとに数えて**計 12 件**が落ちた。追従そのものは成立していた（記録票の突合＝offset が
//! 両拡大率で完全一致）ので、**製品ではなく測り方の欠陥**であった。
//!
//! # 直した形
//!
//! `kind=enqueue` は合流の有無に関わらず「随伴の位置指令がそのフレームで積まれた」事実を
//! 持ち、畳まれたときは `merged_into_seq` に**先着の通し番号**が載る。この番号は一括書込の
//! `kind=write` の `seq`（キュー内の位置）と同じ数え方なので、合流先の書込を引き当てて
//! フレームを読める（[`super::summarize`] の当該箇所）。**本番の観測語彙は 1 つも増やして
//! いない**。
//!
//! # 檻が欠陥を通した機序
//!
//! ここまでの檻は観測行を手で組んでおり、**合流で書込が消えた形を 1 度も作っていなかった**
//! （`test_support::enqueue` は `merged_into_seq` を番兵に固定していた）。ゆえに本モジュールは
//! ⑴ 合流された形と ⑵ 本当に遅れた形を**同一のテスト本体の内側で対にする**——⑴ だけを見る
//! 檻は「何でも測れたことにする」退行（`balloon_same_frame` を無条件に真と数える）で緑のまま
//! 通ってしまう。

use areka_emo_present::presenter::SURFACE_STAGE_VISUALIZE;
use wintf::ecs::window::transition_diag::{
    FIELD_KIND, FIELD_MERGED_INTO_SEQ, FIELD_ORIGIN, KIND_ENQUEUE, KIND_WRITE, MISSING,
    STAGE_BEGIN, STAGE_END, STAGE_FLUSH,
};

use super::super::diag::{PlacementRoute, WindowKind};
use super::test_support::{
    balloon_kind, char_kind, enqueue_merged_into, flush, ground, monitor, summarize_lines, surface,
    write,
};
use super::{
    Bounds, Quantity, TransitionSummary, Violation, WindowKey, judge, parse_transition_log,
    split_transitions, summarize,
};

/// 起点のフレーム番号（手組みの遷移で共有する）。
const ORIGIN_FRAME: u32 = 10;

/// バルーン窓の窓ハンドル（字面は判定に効かないが、実機と同じく窓ごとに別値を置く）。
const BALLOON_HWND: &str = "0x2";

/// キャラ窓の窓ハンドル。
const CHAR_HWND: &str = "0x1";

/// 当該遷移で「随伴の同一フレーム性が未測定」と咎められた窓（無ければ空）。
fn unmeasured_companions(summary: &TransitionSummary) -> Vec<WindowKey> {
    let Err(violations) = judge(summary, &Bounds::deterministic()) else {
        return Vec::new();
    };
    violations
        .into_iter()
        .filter_map(|violation| match violation {
            Violation::Unmeasured(Quantity::BalloonSameFrame(window)) => Some(window),
            _ => None,
        })
        .collect()
}

/// 当該遷移で「随伴が別フレームで書かれた」と咎められたか。
fn flagged_as_another_frame(summary: &TransitionSummary) -> bool {
    match judge(summary, &Bounds::deterministic()) {
        Ok(()) => false,
        Err(violations) => violations.contains(&Violation::BalloonWrittenInAnotherFrame),
    }
}

/// キャラ窓 1・バルーン窓 1 の遷移 1 本を組む。
///
/// バルーンは寸の指令（`KeepPositionResize`）を先に積み、随伴の位置指令を `follow_frame` で
/// 積む。`merged` が真ならその位置指令は寸の枠（`seq=0`）へ**畳まれ**、書込には先着の経路語
/// しか残らない（実機 `frame=14884` と同じ形）。`balloon_lands` が偽ならバルーン窓の書込が
/// 1 本も来ない（指令は積まれたが一括書込へ届かなかった形）——キャラ窓の書込は
/// どちらの場合も出るので、対の期待そのものは消えない。
fn transition_lines(follow_frame: u32, merged: bool, balloon_lands: bool) -> Vec<String> {
    // 合流されたなら書込が名乗るのは先着の寸の経路語、畳まれなければ随伴の経路語である。
    let landed_origin = if merged {
        PlacementRoute::KeepPositionResize.as_str()
    } else {
        PlacementRoute::BalloonFollow.as_str()
    };
    let mut lines = vec![
        monitor(ORIGIN_FRAME, 192, 96, 1704, 1752),
        surface(
            ORIGIN_FRAME,
            1_000,
            SURFACE_STAGE_VISUALIZE,
            0,
            "382",
            "547",
            MISSING,
            MISSING,
        ),
        surface(
            ORIGIN_FRAME,
            1_100,
            SURFACE_STAGE_VISUALIZE,
            1,
            "400",
            "224",
            MISSING,
            MISSING,
        ),
        ground(
            ORIGIN_FRAME,
            0,
            1752,
            1752,
            PlacementRoute::DpiReproject.as_str(),
        ),
        // バルーンの寸の指令（先着＝合流の枠になる）とキャラ窓の指令。
        enqueue_merged_into(
            ORIGIN_FRAME,
            1_200,
            BALLOON_HWND,
            PlacementRoute::KeepPositionResize.as_str(),
            "0",
            balloon_kind(),
            MISSING,
        ),
        enqueue_merged_into(
            ORIGIN_FRAME,
            1_250,
            CHAR_HWND,
            PlacementRoute::DpiReproject.as_str(),
            "0",
            char_kind(),
            MISSING,
        ),
    ];

    // 随伴の位置指令。**行の並びは時系列である**——起点フレームで積まれたなら起点フレームの
    // 一括書込より前に、次フレームで積まれたならその後に現れる。
    let follow_command = enqueue_merged_into(
        follow_frame,
        1_300,
        BALLOON_HWND,
        PlacementRoute::BalloonFollow.as_str(),
        "0",
        balloon_kind(),
        if merged { "0" } else { MISSING },
    );
    if follow_frame == ORIGIN_FRAME {
        lines.push(follow_command.clone());
    }

    // 起点フレームの一括書込。バルーンが書かれる形では、その枠が `seq=0` である
    // （合流先の通し番号と対応する）。書かれない形ではキャラ窓の 1 本だけが出る。
    let origin_writes = if balloon_lands { 2 } else { 1 };
    lines.push(flush(
        ORIGIN_FRAME,
        1_400,
        STAGE_BEGIN,
        origin_writes,
        MISSING,
    ));
    if balloon_lands {
        lines.push(write(
            ORIGIN_FRAME,
            1_500,
            STAGE_FLUSH,
            0,
            BALLOON_HWND,
            if follow_frame == ORIGIN_FRAME {
                landed_origin
            } else {
                PlacementRoute::KeepPositionResize.as_str()
            },
            "0",
            balloon_kind(),
            400,
        ));
    }
    lines.push(write(
        ORIGIN_FRAME,
        1_600,
        STAGE_FLUSH,
        origin_writes - 1,
        CHAR_HWND,
        PlacementRoute::DpiReproject.as_str(),
        "0",
        char_kind(),
        500,
    ));
    lines.push(flush(ORIGIN_FRAME, 1_700, STAGE_END, origin_writes, "300"));

    // 随伴が次フレームで積まれた形は、その次フレームの一括書込で着地する。
    if follow_frame != ORIGIN_FRAME {
        lines.push(follow_command);
        if balloon_lands {
            lines.push(flush(follow_frame, 1_800, STAGE_BEGIN, 1, MISSING));
            lines.push(write(
                follow_frame,
                1_900,
                STAGE_FLUSH,
                0,
                BALLOON_HWND,
                landed_origin,
                "0",
                balloon_kind(),
                400,
            ));
            lines.push(flush(follow_frame, 2_000, STAGE_END, 1, "300"));
        }
    }

    lines
}

#[test]
fn a_companion_follow_folded_into_another_command_is_measured_and_a_late_one_still_violates() {
    let balloon0 = WindowKey::of(0, WindowKind::Balloon);
    let char0 = WindowKey::of(0, WindowKind::Char);

    // ⑴ 合流で書込が消えた形（実機 `frame=14884` と同じ形）。書込の経路語は先着の
    //    `KeepPositionResize` 1 本だけで、`origin=BalloonFollow` の書込は 1 行も無い。
    //    それでも随伴は同一フレームで着地しているので、測れなければならない。
    let merged = summarize_lines(&transition_lines(ORIGIN_FRAME, true, true));
    assert!(
        merged.balloon_follow_windows.contains(&balloon0),
        "合流された随伴の指令も『随伴の位置書込を受けた窓』として数えなければならない"
    );
    assert_eq!(
        merged.balloon_pairs_checked, 1,
        "合流されても対は 1 組検査できる（実機 7 遷移中 5 遷移がこの形）"
    );
    assert!(
        merged.balloon_same_frame,
        "合流先の書込は起点と同一フレームなので随伴は遅れていない"
    );
    assert_eq!(
        unmeasured_companions(&merged),
        Vec::new(),
        "測れている対を『量が欠けている』と咎めてはならない"
    );
    assert!(!flagged_as_another_frame(&merged));

    // ⑵ 随伴が**本当に遅れた**形（次フレームで積まれ、次フレームの一括書込で着地する）。
    //    ⑴ を測れるようにした是正が「何でも測れたことにする」形へ倒れていれば、この主張が
    //    先に赤くなる。
    let late = summarize_lines(&transition_lines(ORIGIN_FRAME + 1, true, true));
    assert_eq!(
        late.balloon_pairs_checked, 1,
        "遅れた随伴も対としては検査できている（測れないのではなく、違反である）"
    );
    assert!(
        !late.balloon_same_frame,
        "随伴が次フレームで着地したことは要件 4.3 の違反として立たなければならない"
    );
    assert!(
        flagged_as_another_frame(&late),
        "違反は `BalloonWrittenInAnotherFrame` として judge から出る"
    );
    assert_eq!(
        unmeasured_companions(&late),
        Vec::new(),
        "遅れは『未測定』ではなく違反である（2 つを取り違えない）"
    );

    // 合流を通していない従来の形（`origin=BalloonFollow` の書込が残る）も、同じ 2 通りの
    // 答えを返す——是正が合流の側だけを見る形へ倒れていないことの対。
    let unmerged_same = summarize_lines(&transition_lines(ORIGIN_FRAME, false, true));
    assert_eq!(unmerged_same.balloon_pairs_checked, 1);
    assert!(unmerged_same.balloon_same_frame);
    let unmerged_late = summarize_lines(&transition_lines(ORIGIN_FRAME + 1, false, true));
    assert_eq!(unmerged_late.balloon_pairs_checked, 1);
    assert!(!unmerged_late.balloon_same_frame);

    // キャラ窓側の量は 4 通りとも従来どおり（除外や数え直しが広がっていないことの対）。
    for summary in [&merged, &late, &unmerged_same, &unmerged_late] {
        assert_eq!(summary.writes_per_window.get(&char0).copied(), Some(1));
    }
}

#[test]
fn a_companion_command_that_never_reached_a_write_is_still_unmeasured() {
    // 指令が積まれただけで一括書込に至らなければ、随伴は**着地していない**。合流を数に
    // 入れる是正が「enqueue があれば追従した」と読むと、書かれていない随伴を合格の根拠に
    // してしまう。
    //
    // この形ではキャラ窓の書込が `seq=0` に来る（バルーンの枠が無いので番号が繰り上がる）
    // ので、合流先を**通し番号だけ**で引き当てる実装はキャラ窓の書込を随伴の着地と
    // 取り違える。引き当ては窓ごとに行うこと。
    let never_written = summarize_lines(&transition_lines(ORIGIN_FRAME, true, false));
    assert!(
        !never_written
            .balloon_follow_windows
            .contains(&WindowKey::of(0, WindowKind::Balloon)),
        "書込に至っていない指令を『随伴の位置書込を受けた』と数えてはならない"
    );
    assert_eq!(never_written.balloon_pairs_checked, 0);
    assert_eq!(
        unmeasured_companions(&never_written),
        vec![WindowKey::of(0, WindowKind::Char)],
        "測れていないことは未測定として立つ（空虚な真で合格にしない）"
    );
}

#[test]
fn a_merge_target_written_before_the_companion_command_is_not_its_landing() {
    // `seq` は一括書込ごとに 0 から振り直される。合流先を通し番号だけで引き当てると、
    // **その指令が積まれるより前**の一括書込の同じ番号を掴み、遅れた随伴が「同一フレームで
    // 着地した」ことになる。引き当ては当該 `enqueue` より後の書込に限る。
    let mut lines = vec![
        monitor(ORIGIN_FRAME, 192, 96, 1704, 1752),
        surface(
            ORIGIN_FRAME,
            1_000,
            SURFACE_STAGE_VISUALIZE,
            0,
            "382",
            "547",
            MISSING,
            MISSING,
        ),
        ground(
            ORIGIN_FRAME,
            0,
            1752,
            1752,
            PlacementRoute::DpiReproject.as_str(),
        ),
        // 起点フレーム: バルーンの寸だけが seq=0 で書かれる（随伴の指令はまだ積まれていない）。
        flush(ORIGIN_FRAME, 1_100, STAGE_BEGIN, 2, MISSING),
        write(
            ORIGIN_FRAME,
            1_200,
            STAGE_FLUSH,
            0,
            BALLOON_HWND,
            PlacementRoute::KeepPositionResize.as_str(),
            "0",
            balloon_kind(),
            400,
        ),
        write(
            ORIGIN_FRAME,
            1_300,
            STAGE_FLUSH,
            1,
            CHAR_HWND,
            PlacementRoute::DpiReproject.as_str(),
            "0",
            char_kind(),
            500,
        ),
        flush(ORIGIN_FRAME, 1_400, STAGE_END, 2, "300"),
    ];
    // 次フレーム: 随伴の指令が積まれて seq=0 の枠へ畳まれる。**書込は 1 本も来ない。**
    lines.push(enqueue_merged_into(
        ORIGIN_FRAME + 1,
        1_500,
        BALLOON_HWND,
        PlacementRoute::BalloonFollow.as_str(),
        "0",
        balloon_kind(),
        "0",
    ));

    let summary = summarize_lines(&lines);
    assert_eq!(
        summary.balloon_pairs_checked, 0,
        "積まれる前の書込を合流先として掴んではならない"
    );
    assert_eq!(
        unmeasured_companions(&summary),
        vec![WindowKey::of(0, WindowKind::Char)]
    );
}

// ---------------------------------------------------------------------------
// 実機ログの回帰（2026-08-21 の再採取・遷移 #3）
// ---------------------------------------------------------------------------

/// 2026-08-21 の実機再採取（task 7.1）の**遷移 #3 の全 56 レコードを逐語で**埋め込んだもの。
///
/// 出所は `atom-71-recapture-1\atom-signoff.log`（リポジトリ外・書き換え禁止。記録票
/// `meta.txt` と対）で、`kind=monitor` の 3 本目の起点（`frame=14884`・192→96）から次の起点
/// （`frame=31750`）の直前までである。**判定に効かない `tracing` の接頭語（時刻・レベル・
/// target・span）だけを落として**あり、`[transition]` 以降は 1 文字も変えていない
/// （接頭語は [`super::parse_transition_line`] が読まないので `records` は 56 のまま）。
///
/// この遷移が本タスクの引受先そのものである——`frame=14884` の `kind=enqueue` に
/// `origin=BalloonFollow` が 3 本（scope=0 が 1 本・scope=1 が 2 本）あり、いずれも
/// `merged_into_seq` を持つ。対応する `kind=write` は先着の `KeepPositionResize` 1 本ずつで、
/// `origin=BalloonFollow` の書込は 1 行も無い。
///
/// 是正前はここで `balloon_pairs_checked=0` となり、記録票の
/// 「判定対象の量が欠けている: balloon_same_frame」2 件が立っていた。
const RECAPTURE_TRANSITION_3: &str = "\
[transition] frame=14884 t_us=4679 kind=monitor entity=1v0 old_dpi=192 new_dpi=96 old_wa=0,0,2880,1704 new_wa=0,0,2880,1752
[transition] frame=14884 t_us=5312 kind=snapshot monitors=2 m0=96:0,0,2880,1752 m1=144:-2560,195,0,1795
[transition] frame=14884 t_us=5365 kind=hold entity=5v0 scope=1 win_kind=balloon window_dpi=96 table_dpi=96 since_frame=14884 decision=proceed site=dpi
[transition] frame=14884 t_us=5376 kind=hold entity=3v0 scope=0 win_kind=balloon window_dpi=96 table_dpi=96 since_frame=14884 decision=proceed site=dpi
[transition] frame=14884 t_us=5383 kind=hold entity=4v0 scope=0 win_kind=char window_dpi=96 table_dpi=96 since_frame=14884 decision=proceed site=dpi
[transition] frame=14884 t_us=5391 kind=hold entity=6v0 scope=1 win_kind=char window_dpi=96 table_dpi=96 since_frame=14884 decision=proceed site=dpi
[transition] frame=14884 t_us=12711 kind=surface stage=upload target_id=3 w=288 h=203 resized=true reason=-
[transition] frame=14884 t_us=28729 kind=surface stage=visualize target_id=3 w=288 h=203 resized=- reason=-
[transition] frame=14884 t_us=28811 kind=enqueue hwnd=0x7609AA origin=KeepPositionResize scope=1 win_kind=balloon merged_into_seq=-
[transition] frame=14884 t_us=33332 kind=surface stage=upload target_id=1 w=400 h=224 resized=true reason=-
[transition] frame=14884 t_us=33366 kind=surface stage=visualize target_id=1 w=400 h=224 resized=- reason=-
[transition] frame=14884 t_us=33393 kind=enqueue hwnd=0xD70D6A origin=KeepPositionResize scope=0 win_kind=balloon merged_into_seq=-
[transition] frame=14884 t_us=42210 kind=surface stage=upload target_id=0 w=382 h=547 resized=true reason=-
[transition] frame=14884 t_us=42264 kind=surface stage=visualize target_id=0 w=382 h=547 resized=- reason=-
[transition] frame=14884 t_us=42351 kind=enqueue hwnd=0x750D40 origin=DpiReproject scope=0 win_kind=char merged_into_seq=-
[transition] frame=14884 t_us=42428 kind=ground scope=0 ground_y=1752 wa_bottom=1752 diff=0 route=DpiReproject
[transition] frame=14884 t_us=42453 kind=enqueue hwnd=0xD70D6A origin=BalloonFollow scope=0 win_kind=balloon merged_into_seq=1
[transition] frame=14884 t_us=42487 kind=chain stage=armed scopes=2 moved=0 reason=-
[transition] frame=14884 t_us=50282 kind=surface stage=upload target_id=2 w=336 h=400 resized=true reason=-
[transition] frame=14884 t_us=50325 kind=surface stage=visualize target_id=2 w=336 h=400 resized=- reason=-
[transition] frame=14884 t_us=50372 kind=enqueue hwnd=0x5704CE origin=DpiReproject scope=1 win_kind=char merged_into_seq=-
[transition] frame=14884 t_us=50415 kind=ground scope=1 ground_y=1752 wa_bottom=1752 diff=0 route=DpiReproject
[transition] frame=14884 t_us=50427 kind=enqueue hwnd=0x7609AA origin=BalloonFollow scope=1 win_kind=balloon merged_into_seq=0
[transition] frame=14884 t_us=50488 kind=enqueue hwnd=0x5704CE origin=ChainRealign scope=1 win_kind=char merged_into_seq=3
[transition] frame=14884 t_us=50503 kind=enqueue hwnd=0x7609AA origin=BalloonFollow scope=1 win_kind=balloon merged_into_seq=0
[transition] frame=14884 t_us=50518 kind=chain stage=realigned scopes=2 moved=1 reason=-
[transition] frame=14884 t_us=95250 kind=flush stage=begin count=4 since_tick_us=95250 total_us=-
[transition] frame=14884 t_us=141065 kind=msg msg=WM_WINDOWPOSCHANGED hwnd=0x7609AA in_swp=true since_flush_us=45815
[transition] frame=14884 t_us=142758 kind=msg msg=WM_DPICHANGED hwnd=0x7609AA in_swp=true since_flush_us=47509
[transition] frame=14884 t_us=155413 kind=msg msg=WM_WINDOWPOSCHANGED hwnd=0x7609AA in_swp=true since_flush_us=60164
[transition] frame=14884 t_us=157617 kind=write stage=flush seq=0 hwnd=0x7609AA origin=KeepPositionResize scope=1 win_kind=balloon x=2211 y=1202 cx=288 cy=203 flags=0x14 ax=2211 ay=1202 aw=288 ah=203 call_us=62339 ok=true
[transition] frame=14884 t_us=186437 kind=msg msg=WM_WINDOWPOSCHANGED hwnd=0xD70D6A in_swp=true since_flush_us=91187
[transition] frame=14884 t_us=202578 kind=msg msg=WM_DPICHANGED hwnd=0xD70D6A in_swp=true since_flush_us=107329
[transition] frame=14884 t_us=220804 kind=msg msg=WM_WINDOWPOSCHANGED hwnd=0xD70D6A in_swp=true since_flush_us=125555
[transition] frame=14884 t_us=225438 kind=write stage=flush seq=1 hwnd=0xD70D6A origin=KeepPositionResize scope=0 win_kind=balloon x=1987 y=947 cx=400 cy=224 flags=0x14 ax=1987 ay=947 aw=400 ah=224 call_us=67789 ok=true
[transition] frame=14884 t_us=247054 kind=msg msg=WM_WINDOWPOSCHANGED hwnd=0x750D40 in_swp=true since_flush_us=151804
[transition] frame=14884 t_us=247488 kind=msg msg=WM_DPICHANGED hwnd=0x750D40 in_swp=true since_flush_us=152239
[transition] frame=14884 t_us=259274 kind=msg msg=WM_WINDOWPOSCHANGED hwnd=0x750D40 in_swp=true since_flush_us=164025
[transition] frame=14884 t_us=261171 kind=write stage=flush seq=2 hwnd=0x750D40 origin=DpiReproject scope=0 win_kind=char x=2255 y=1205 cx=382 cy=547 flags=0x14 ax=2255 ay=1205 aw=382 ah=547 call_us=35697 ok=true
[transition] frame=14884 t_us=279756 kind=msg msg=WM_WINDOWPOSCHANGED hwnd=0x5704CE in_swp=true since_flush_us=184507
[transition] frame=14884 t_us=281914 kind=msg msg=WM_DPICHANGED hwnd=0x5704CE in_swp=true since_flush_us=186665
[transition] frame=14884 t_us=291891 kind=msg msg=WM_WINDOWPOSCHANGED hwnd=0x5704CE in_swp=true since_flush_us=196641
[transition] frame=14884 t_us=292364 kind=write stage=flush seq=3 hwnd=0x5704CE origin=DpiReproject scope=1 win_kind=char x=1919 y=1352 cx=336 cy=400 flags=0x14 ax=1919 ay=1352 aw=336 ah=400 call_us=31160 ok=true
[transition] frame=14884 t_us=292391 kind=flush stage=end count=4 since_tick_us=292391 total_us=197140
[transition] frame=14885 t_us=3657 kind=hold entity=5v0 scope=1 win_kind=balloon window_dpi=96 table_dpi=96 since_frame=14885 decision=proceed site=dpi
[transition] frame=14885 t_us=3685 kind=hold entity=3v0 scope=0 win_kind=balloon window_dpi=96 table_dpi=96 since_frame=14885 decision=proceed site=dpi
[transition] frame=14885 t_us=3691 kind=hold entity=4v0 scope=0 win_kind=char window_dpi=96 table_dpi=96 since_frame=14885 decision=proceed site=dpi
[transition] frame=14885 t_us=3695 kind=hold entity=6v0 scope=1 win_kind=char window_dpi=96 table_dpi=96 since_frame=14885 decision=proceed site=dpi
[transition] frame=14885 t_us=3702 kind=surface stage=skipped target_id=3 w=- h=- resized=- reason=k-unchanged
[transition] frame=14885 t_us=3707 kind=surface stage=skipped target_id=1 w=- h=- resized=- reason=k-unchanged
[transition] frame=14885 t_us=3712 kind=surface stage=skipped target_id=0 w=- h=- resized=- reason=k-unchanged
[transition] frame=14885 t_us=3717 kind=surface stage=skipped target_id=2 w=- h=- resized=- reason=k-unchanged
[transition] frame=31749 t_us=57951 kind=msg msg=WM_DISPLAYCHANGE hwnd=0x7609AA in_swp=false since_flush_us=-
[transition] frame=31749 t_us=66975 kind=msg msg=WM_DISPLAYCHANGE hwnd=0x5704CE in_swp=false since_flush_us=-
[transition] frame=31749 t_us=72235 kind=msg msg=WM_DISPLAYCHANGE hwnd=0xD70D6A in_swp=false since_flush_us=-
[transition] frame=31749 t_us=76923 kind=msg msg=WM_DISPLAYCHANGE hwnd=0x750D40 in_swp=false since_flush_us=-
";

/// 埋め込みログを 1 本の遷移として集計する。
fn recapture_transition_3() -> TransitionSummary {
    let records = parse_transition_log(RECAPTURE_TRANSITION_3);
    assert_eq!(
        records.len(),
        56,
        "接頭語を落としても解析されるレコード数は記録票の 56 のまま"
    );
    let transitions = split_transitions(&records);
    assert_eq!(transitions.len(), 1, "遷移 1 本ぶんの引用である");
    summarize(&transitions[0])
}

/// 当該レコード種別の行（`kind=` の値で選ぶ）。
fn lines_of_kind(kind: &str) -> Vec<&'static str> {
    RECAPTURE_TRANSITION_3
        .lines()
        .filter(|line| line.contains(&format!("{FIELD_KIND}={kind} ")))
        .collect()
}

/// 随伴の位置の経路語（`origin=BalloonFollow`）を名乗る行か。
fn names_the_follow_route(line: &str) -> bool {
    line.contains(&format!(
        "{FIELD_ORIGIN}={}",
        PlacementRoute::BalloonFollow.as_str()
    ))
}

#[test]
fn the_recaptured_transition_measures_both_companions_and_passes_the_deterministic_bounds() {
    // まず「合流された随伴」が実機ログの側に本当に在ることを固定する（無ければ以下の主張は
    // 何も測っていない）。
    let follow_enqueues: Vec<&str> = lines_of_kind(KIND_ENQUEUE)
        .into_iter()
        .filter(|line| names_the_follow_route(line))
        .collect();
    assert_eq!(follow_enqueues.len(), 3, "随伴の位置指令は 3 本ある");
    assert!(
        follow_enqueues
            .iter()
            .all(|line| !line.contains(&format!("{FIELD_MERGED_INTO_SEQ}={MISSING}"))),
        "3 本とも合流されている（だから書込に経路語が残らない）"
    );
    let writes = lines_of_kind(KIND_WRITE);
    assert_eq!(writes.len(), 4, "書込は 4 本（窓ごとに 1 本）");
    assert!(
        !writes.iter().any(|line| names_the_follow_route(line)),
        "`origin=BalloonFollow` の書込は 1 行も無い（是正前に空振りした当の形）"
    );

    let summary = recapture_transition_3();

    // 本タスクの観察可能な完了条件——2 スコープとも対が検査でき、随伴は同一フレームである。
    assert_eq!(
        summary.balloon_pairs_checked, 2,
        "scope 0／1 の両方で対を検査できる（是正前は 0）"
    );
    assert!(summary.balloon_same_frame);
    assert_eq!(
        unmeasured_companions(&summary),
        Vec::new(),
        "「量が欠けている: balloon_same_frame」は 1 件も立たない"
    );

    // 記録票「全 7 遷移で共通の量」の逐語再現（判定量そのものが変わっていないこと）。
    assert_eq!(summary.records, 56);
    assert_eq!(summary.frames_to_last_write, Some(0));
    assert_eq!(summary.writes, 4);
    assert_eq!(summary.path_a_writes, 0);
    assert_eq!(summary.sync_stage_writes, 0);
    assert_eq!(summary.holds, 0);
    assert_eq!(summary.chain_realigned, 1);
    assert_eq!(summary.ground_diff_max, Some(0));
    assert_eq!(summary.malformed_records, 0);
    assert!(!summary.frames_indeterminate);
    assert!(
        summary.skipped_windows.is_empty(),
        "`reason=k-unchanged` の 4 件は除外に使わない（裁定⑶）"
    );
    for scope in [0, 1] {
        for kind in [WindowKind::Char, WindowKind::Balloon] {
            let window = WindowKey::of(scope, kind);
            assert_eq!(summary.writes_per_window.get(&window).copied(), Some(1));
            assert_eq!(
                summary.mismatch_frames_per_window.get(&window).copied(),
                Some(0)
            );
        }
    }

    // 決定論系統は**全件合格**になる（是正前はこの遷移で 2 件の未測定が立っていた）。実機
    // 専用系統の µs 2 種は task 7.1 の持ち分なので、ここでは当てない。
    assert_eq!(judge(&summary, &Bounds::deterministic()), Ok(()));
}

#[test]
fn the_recaptured_transition_still_flags_a_companion_that_lands_a_frame_late() {
    // 実機ログの形のままでは「違反が立たないこと」しか測れない。合流先の書込（scope=1 の
    // バルーンの seq=0）を 1 フレーム遅らせると、同じ入力が従来どおり違反として立つ——
    // 是正が要件 4.3 の見張りを殺していないことの対である。
    let needle = "frame=14884 t_us=157617 kind=write";
    assert_eq!(
        RECAPTURE_TRANSITION_3.matches(needle).count(),
        1,
        "置換対象はちょうど 1 行にあるはず"
    );
    let late = RECAPTURE_TRANSITION_3.replace(needle, "frame=14885 t_us=157617 kind=write");

    let records = parse_transition_log(&late);
    let transitions = split_transitions(&records);
    assert_eq!(transitions.len(), 1);
    let summary = summarize(&transitions[0]);
    assert_eq!(
        summary.balloon_pairs_checked, 2,
        "対は 2 組とも検査できたままである（測れなくなったのではない）"
    );
    assert!(
        !summary.balloon_same_frame,
        "scope=1 の随伴が 1 フレーム遅れて着地したことは違反として立つ"
    );
    assert!(flagged_as_another_frame(&summary));
}

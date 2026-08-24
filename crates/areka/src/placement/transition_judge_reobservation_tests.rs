//! 第 1 段再観測 §3.1（遷移 1・192→96）を**新語彙へ整形した埋め込みログ**からの逐語再現と、
//! §3.2 の**6 遷移すべて**に対する判定の突合（task 3.2 で後半を追加）。
//!
//! 後半が要るのは、判定の上限をテストの作った形に合わせて書くと、レポートが「欠陥ではない」と
//! 明記した遷移を不合格にしてしまうからである（task 3.2 のレビューで 2 度起きた）。判定器を
//! 触ったら、まず 6 遷移すべての判定をレポートの記述と突き合わせること。
//!
//! **この治具を実機専用側（`Bounds::signoff`）の判定へ流用しないこと。** `t_us` の並びは忠実で
//! ないためである——最初の窓書込が `t_us=13,800`、`flush stage=begin` が `100,500` で、書込が
//! 一括書込の開始より前に現れる。決定論側の判定は `t_us` を判定語に使わないので無害だが、
//! `visualize_to_write_us`／`flush_total_us` を測る用途に使うなら先に時刻の並びを直すこと。
//!
//! 正本は `.kiro/specs/areka-P0-dpi-transition-atomicity/reobservation-2026-08-15.md` の §3.1
//! （全行を引用した代表例）である。当時のログにはフレーム番号が無く順序と遅れは時刻近似
//! だったので、整形にあたって次の 2 点を補った——どちらも再観測レポートの本文から導ける:
//!
//! - **フレーム番号は 1 つ**（`41231`）。6 回の窓書込はいずれも `+103〜255ms` に並ぶが、これは
//!   1 回の一括書込の内側で `SetWindowPos` が 1 本ずつ 60〜80ms を要しているためであり
//!   （§5「逐次区間の内側では `SetWindowPos` 1 回ごとに当該窓の `WM_DPICHANGED` 処理が同期で
//!   挟まる」）、区間全体が同一 tick に載る。design C8 の脚注「4.4 の決定論値
//!   `TRANSITION_FRAME_BOUND = 0` は現行コードで既に成立する」と同じ事実である。
//! - **`t_us` は起点を 0 とする相対 ms を µs へ引き伸ばした値**（`+13ms` → `13000`）。同一
//!   µs に 2 行が並ぶ箇所だけ、レポートの記述順に 100µs 刻みでずらしてある。
//!
//! # レポートに出所の無い値（判定量には使わない）
//!
//! 次の 3 種はレポートが持たないので**発明した**値である。いずれも本モジュールの逐語再現
//! （書込 6・経路 A 0・接地点差 −48px・フレーム量 0）には 1 つも効かない:
//!
//! - 起点行の `t_us=214`——モニタ表更新は tick の内側で起きるという以上の根拠は無い。
//! - 6 本の `call_us`——レポートは「`SetWindowPos` 1 回が 60〜80ms」（§5・内訳は未特定）と
//!   書くだけで 1 回ごとの値を持たない。行の間隔から辻褄の合う値を置いてある。
//! - `flush stage=begin`／`stage=end` の刻印と `total_us`——レポートに一括書込の区間の記録は
//!   無い（当時その観測点が無かった）。最初の書込の直前から最後の書込の直後までを覆う値。
//!
//! # task 4.2 で実測と突き合わせること
//!
//! 上の補い（フレーム番号 1 つ・`t_us` の引き伸ばし・発明した 3 種）は、**フレーム番号つきの
//! 再採取（task 4.2）が出る時点で必ず再検証する**。実測が「6 本の書込が複数フレームに
//! またがる」を示したなら、本モジュールの前提（同一 tick）と
//! `the_frame_unit_quantities_are_already_zero_before_any_correction` の主張は書き換えが要る。
//!
//! ここが押さえる要点は、フレーム単位の量が**是正前でも 0 になる**ことである
//! （`frames_to_last_write=0`・`mismatch_frames_per_window` が全窓 0）。同一 tick の内側の
//! 食い違いは µs でしか見えないので、実機サインオフ専用の量
//! （`visualize_to_write_us`・`flush_total_us_max`）が要る——設計討議 A-2 の裁定そのものである。

use areka_emo_present::presenter::{
    SURFACE_REASON_INVISIBLE, SURFACE_STAGE_SKIPPED, SURFACE_STAGE_VISUALIZE,
};
use wintf::ecs::window::transition_diag::{
    FIELD_STAGE, KIND_WRITE, MISSING, STAGE_BEGIN, STAGE_END, STAGE_FLUSH,
};

use super::super::diag::{PlacementRoute, WindowKind};
use super::test_support::{flush, ground, monitor, surface, write};
use super::{
    Bounds, Violation, WindowKey, judge, parse_transition_log, split_transitions, summarize,
};

/// 起点のフレーム番号（整形時に与えた 1 つの値）。
const ORIGIN_FRAME: u32 = 41231;

/// 再観測 §3.1（遷移 1・192→96）を新語彙へ整形した埋め込みログ。
///
/// `[transition]` を持たない 3 行（`info` の本文・`[diag.window_move]`・段階別計時）は
/// 実機ログの混ざり方をそのまま再現するために残してある。判定器はこれを 1 件も数えない。
///
/// 姉妹モジュール `transition_judge_verdict_tests`／`transition_judge_negative_tests` が
/// **同じ字面**を上限判定と負例の入力に使う（task 3.2）。複製すると片方だけが語彙の変更に
/// 追随しなくなるので、可視性を広げて 1 本を共有する。
pub(super) const REOBSERVATION_TRANSITION_1: &str = "\
2026-08-15T11:54:14.327101Z DEBUG wintf::transition: [transition] frame=41231 t_us=214 kind=monitor entity=2v0 old_dpi=192 new_dpi=96 old_wa=0,0,2880,1704 new_wa=0,0,2880,1752
2026-08-15T11:54:14.327400Z  INFO wintf::ecs::layout: Redriving window DPI from updated Monitor (no WM_DPICHANGED required) entity=5v0 192->96
[transition] frame=41231 t_us=13000 kind=surface stage=upload target_id=3 w=288 h=203 resized=true reason=-
[transition] frame=41231 t_us=13400 kind=surface stage=visualize target_id=3 w=288 h=203 resized=- reason=-
[transition] frame=41231 t_us=13500 kind=enqueue hwnd=0x702A0 origin=KeepPositionResize scope=1 win_kind=balloon merged_into_seq=-
[diag.window_move] route=KeepPositionResize entity=5v0 kind=balloon scope=1 x=1684 y=754 w=288 h=203
[transition] frame=41231 t_us=19000 kind=surface stage=upload target_id=2 w=336 h=400 resized=true reason=-
[transition] frame=41231 t_us=19300 kind=surface stage=visualize target_id=2 w=336 h=400 resized=- reason=-
[transition] frame=41231 t_us=19400 kind=ground scope=1 ground_y=1704 wa_bottom=1752 diff=-48 route=DpiReproject
[transition] frame=41231 t_us=19450 kind=enqueue hwnd=0xD0AEE origin=DpiReproject scope=1 win_kind=char merged_into_seq=-
[transition] frame=41231 t_us=19600 kind=enqueue hwnd=0x702A0 origin=BalloonFollow scope=1 win_kind=balloon merged_into_seq=-
perf apply target_id=2 t_total_us=6316 upload_us=2100 frame=41231
[transition] frame=41231 t_us=33000 kind=surface stage=upload target_id=0 w=382 h=547 resized=true reason=-
[transition] frame=41231 t_us=33600 kind=surface stage=visualize target_id=0 w=382 h=547 resized=- reason=-
[transition] frame=41231 t_us=33700 kind=ground scope=0 ground_y=1704 wa_bottom=1752 diff=-48 route=DpiReproject
[transition] frame=41231 t_us=33750 kind=enqueue hwnd=0x2109FA origin=DpiReproject scope=0 win_kind=char merged_into_seq=-
[transition] frame=41231 t_us=33900 kind=enqueue hwnd=0xE095A origin=BalloonFollow scope=0 win_kind=balloon merged_into_seq=-
[transition] frame=41231 t_us=38000 kind=surface stage=upload target_id=1 w=400 h=224 resized=true reason=-
[transition] frame=41231 t_us=38200 kind=surface stage=visualize target_id=1 w=400 h=224 resized=- reason=-
[transition] frame=41231 t_us=38400 kind=enqueue hwnd=0xE095A origin=KeepPositionResize scope=0 win_kind=balloon merged_into_seq=-
[transition] frame=41231 t_us=100500 kind=flush stage=begin count=6 since_tick_us=100500 total_us=-
[transition] frame=41231 t_us=103000 kind=write stage=flush seq=0 hwnd=0x702A0 origin=KeepPositionResize scope=1 win_kind=balloon x=1684 y=754 cx=288 cy=203 flags=0x14 ax=1684 ay=754 aw=288 ah=203 call_us=20500 ok=true
[transition] frame=41231 t_us=124000 kind=msg msg=WM_DPICHANGED hwnd=0x702A0 in_swp=true since_flush_us=23500
[transition] frame=41231 t_us=181000 kind=write stage=flush seq=1 hwnd=0xD0AEE origin=DpiReproject scope=1 win_kind=char x=1560 y=1304 cx=336 cy=400 flags=0x14 ax=1560 ay=1304 aw=336 ah=400 call_us=57200 ok=true
[transition] frame=41231 t_us=201000 kind=msg msg=WM_DPICHANGED hwnd=0xD0AEE in_swp=true since_flush_us=100500
[transition] frame=41231 t_us=216000 kind=write stage=flush seq=2 hwnd=0x702A0 origin=BalloonFollow scope=1 win_kind=balloon x=1852 y=1154 cx=0 cy=0 flags=0x15 ax=1852 ay=1154 aw=288 ah=203 call_us=34100 ok=true
[transition] frame=41231 t_us=218000 kind=write stage=flush seq=3 hwnd=0x2109FA origin=DpiReproject scope=0 win_kind=char x=2255 y=1157 cx=382 cy=547 flags=0x14 ax=2255 ay=1157 aw=382 ah=547 call_us=2100 ok=true
[transition] frame=41231 t_us=230000 kind=msg msg=WM_DPICHANGED hwnd=0x2109FA in_swp=true since_flush_us=129500
[transition] frame=41231 t_us=239000 kind=write stage=flush seq=4 hwnd=0xE095A origin=BalloonFollow scope=0 win_kind=balloon x=1987 y=899 cx=0 cy=0 flags=0x15 ax=1987 ay=899 aw=400 ah=224 call_us=20800 ok=true
[transition] frame=41231 t_us=247000 kind=msg msg=WM_DPICHANGED hwnd=0xE095A in_swp=true since_flush_us=146500
[transition] frame=41231 t_us=255000 kind=write stage=flush seq=5 hwnd=0xE095A origin=KeepPositionResize scope=0 win_kind=balloon x=1987 y=899 cx=400 cy=224 flags=0x14 ax=1987 ay=899 aw=400 ah=224 call_us=16200 ok=true
[transition] frame=41231 t_us=271500 kind=flush stage=end count=6 since_tick_us=100500 total_us=171000
";

/// 埋め込みログを 1 本の遷移として集計する。
fn summary() -> super::TransitionSummary {
    let records = parse_transition_log(REOBSERVATION_TRANSITION_1);
    let transitions = split_transitions(&records);
    assert_eq!(transitions.len(), 1, "§3.1 は遷移 1 回ぶんの引用である");
    summarize(&transitions[0])
}

#[test]
fn every_embedded_record_is_well_formed() {
    // 語彙が変わって必須フィールドが増減すれば、この 1 本が真っ先に赤くなる
    // （埋め込みログは字面のデータなので、追随の合図をここで受け取る）。
    let records = parse_transition_log(REOBSERVATION_TRANSITION_1);
    assert!(
        !records.is_empty(),
        "1 行も解析できていないと、この検査は空虚に緑になる"
    );
    for record in records {
        assert!(
            record.is_well_formed(),
            "埋め込みログの行が語彙の規約を破っている: kind={} defects={:?}",
            record.kind,
            record.defects
        );
    }
}

#[test]
fn lines_without_the_record_tag_are_not_counted() {
    let total_lines = REOBSERVATION_TRANSITION_1.lines().count();
    let records = parse_transition_log(REOBSERVATION_TRANSITION_1);
    assert_eq!(total_lines, 32, "埋め込みログの行数");
    assert_eq!(
        records.len(),
        29,
        "`info` 本文・`[diag.window_move]`・段階別計時の 3 行は数えない"
    );
}

#[test]
fn reproduces_six_writes_no_path_a_write_and_a_minus_48_ground_difference() {
    // task 3.1 の観察可能な完了条件そのもの（再観測 §3.2 の表・§6 の接地点）。
    let summary = summary();
    assert_eq!(summary.writes, 6, "再観測 §3.2「書込回数 6」");
    assert_eq!(summary.path_a_writes, 0, "再観測 §3.2「経路 A 0」");
    assert_eq!(
        summary.ground_diff_max,
        Some(-48),
        "再観測 §6「−48px（浮き）」"
    );
}

#[test]
fn reproduces_the_per_window_write_breakdown() {
    // 再観測 §3.2 の内訳: キャラ窓 1 回 ×2・バルーン窓 2 回 ×2。
    let summary = summary();
    for scope in [0, 1] {
        assert_eq!(
            summary
                .writes_per_window
                .get(&WindowKey::of(scope, WindowKind::Char)),
            Some(&1),
            "キャラ窓 scope={scope} は 1 回（DpiReproject の位置＋寸）"
        );
        assert_eq!(
            summary
                .writes_per_window
                .get(&WindowKey::of(scope, WindowKind::Balloon)),
            Some(&2),
            "バルーン窓 scope={scope} は 2 回（寸＋随伴位置）＝合流前の値"
        );
    }
    assert_eq!(summary.writes_per_window.len(), 4);
    assert_eq!(summary.sync_stage_writes, 0, "`stage=sync` の裏取りも 0");
}

#[test]
fn the_frame_unit_quantities_are_already_zero_before_any_correction() {
    let summary = summary();
    let origin = summary.origin.expect("起点は拡大率 192→96 のモニタ表更新");
    assert_eq!(origin.stamp.frame, ORIGIN_FRAME);
    assert_eq!((origin.old_dpi, origin.new_dpi), (192, 96));
    assert_eq!(summary.frames_to_last_write, Some(0));
    assert!(
        !summary.frames_indeterminate,
        "一様に 0 のフレーム系列ではないので判定可能"
    );
    for scope in [0, 1] {
        for kind in [WindowKind::Char, WindowKind::Balloon] {
            assert_eq!(
                summary
                    .mismatch_frames_per_window
                    .get(&WindowKey::of(scope, kind)),
                Some(&0),
                "フレーム差は是正前でも 0（scope={scope} {}）",
                kind.as_str()
            );
        }
    }
    assert_eq!(summary.balloon_pairs_checked, 2);
    assert!(summary.balloon_same_frame);
}

#[test]
fn the_same_tick_divergence_is_only_visible_in_microseconds() {
    // 「描画内容は +13〜47ms に新寸・窓矩形は +63〜309ms まで旧寸」（再観測 §4.1）は
    // 同一 tick の内側なので、実機サインオフ専用の量でしか測れない。
    let summary = summary();
    let expected = [
        (WindowKey::of(1, WindowKind::Balloon), 202_600_u64),
        (WindowKey::of(1, WindowKind::Char), 161_700),
        (WindowKey::of(0, WindowKind::Char), 184_400),
        (WindowKey::of(0, WindowKind::Balloon), 216_800),
    ];
    for (window, gap_us) in expected {
        assert_eq!(
            summary.visualize_to_write_us.get(&window),
            Some(&gap_us),
            "可視化から書込までの経過（scope={:?} {}）",
            window.scope,
            window.kind
        );
    }
    assert_eq!(summary.flush_total_us_max, Some(171_000));
}

#[test]
fn the_first_and_last_write_times_come_from_the_report_the_call_total_does_not() {
    let summary = summary();
    // ここだけがレポート由来（§3.1: 最初の書込 +103ms・最後の書込 +255ms）。
    assert_eq!(summary.wall.first_write_t_us, Some(103_000));
    assert_eq!(summary.wall.last_write_t_us, Some(255_000));
    // 総和は**発明した `call_us` の合計**であり、レポートに対応する数値は無い
    // （module doc「レポートに出所の無い値」）。固定するのは「6 本ぶんを足し漏れなく
    // 積む」という集計の性質であって、実機の所要ではない。
    assert_eq!(
        summary.wall.sum_call_us, 150_900,
        "20500+57200+34100+2100+20800+16200＝埋め込み値の総和"
    );
}

#[test]
fn nothing_was_held_realigned_or_skipped_in_this_transition() {
    let summary = summary();
    assert_eq!(summary.holds, 0, "整合待ちは未実装（task 5.4）ゆえ 0");
    assert_eq!(
        summary.chain_realigned, 0,
        "再観測 §7「`finalize_chain` の解き直しは 6 遷移で 0 件」"
    );
    assert!(
        summary.skipped_windows.is_empty(),
        "遷移 1 では見送り窓は無い"
    );
    assert_eq!(summary.malformed_records, 0);
    assert_eq!(summary.records, 29);
}

#[test]
fn the_visualize_stage_precedes_every_window_write_in_the_same_frame() {
    // 順序（サーフェス更新と窓書込の前後）は要件 7.1 の判定量である。
    let records = parse_transition_log(REOBSERVATION_TRANSITION_1);
    let last_visualize = records
        .iter()
        .filter(|record| record.raw_field(FIELD_STAGE) == Some(SURFACE_STAGE_VISUALIZE))
        .map(|record| record.stamp.t_us)
        .max()
        .expect("可視化の記録があるはず");
    let first_write = records
        .iter()
        .filter(|record| record.kind == KIND_WRITE)
        .map(|record| record.stamp.t_us)
        .min()
        .expect("窓書込の記録があるはず");
    assert!(
        last_visualize < first_write,
        "4 窓とも可視化が済んでから 1 枚目の窓書込が出る（{last_visualize} < {first_write}）"
    );
}

// ---------------------------------------------------------------------------
// 6 遷移すべての突合（再観測 §3.2 の一覧）
// ---------------------------------------------------------------------------
//
// 上の逐語再現は §3.1（遷移 1）だけを見る。判定の上限（task 3.2）を書くときに見なければ
// ならないのは**残り 5 遷移**であり、とりわけ §3.2 がただ 1 つ「Requirement 4.6 の現状維持
// 挙動＝欠陥ではない」と明記した遷移 2 である。判定器がそれを不合格にしたら、それは
// ゴーストの欠陥ではなく判定器の欠陥である——2 度の是正はどちらもこの突合を先にしていれば
// 防げた（判定を「テストの作った形」に合わせて書いたのが機序）。

/// 見送られた寸書込が届くまでのフレーム差（レポートの +649〜660ms ≒ 60Hz で 40 フレーム）。
const DEFERRED_RESIZE_FRAMES: u32 = 40;

/// §3.2 の 1 行ぶん（2 スコープ・書込 6 回）を新語彙で組む。
///
/// `deferred_balloon_scope` を与えると、そのスコープのバルーンは**遷移時点で不可視**として
/// 再表示を見送られ（`stage=skipped reason=invisible`）、寸書込だけが
/// [`DEFERRED_RESIZE_FRAMES`] 後に届く——随伴の位置書込は他の窓と同じフレームに残る。
/// これが遷移 2 の外れ値の形そのものである。
fn baseline_transition(
    frame: u32,
    old_dpi: u32,
    new_dpi: u32,
    deferred_balloon_scope: Option<u32>,
) -> Vec<String> {
    // 接地点は全 12 件で 1704 固定（§6）。作業領域下端は 96 で 1752・192 で 1704。
    let wa_bottom = if new_dpi == 96 { 1752 } else { 1704 };
    let old_bottom = if new_dpi == 96 { 1704 } else { 1752 };
    let mut lines = vec![monitor(frame, old_dpi, new_dpi, old_bottom, wa_bottom)];
    let mut t_us = 13_000_u64;
    for scope in [1_u32, 0] {
        let deferred = deferred_balloon_scope == Some(scope);
        lines.push(surface(
            frame,
            t_us,
            SURFACE_STAGE_VISUALIZE,
            scope * 2,
            "336",
            "400",
            MISSING,
            MISSING,
        ));
        t_us += 200;
        if deferred {
            lines.push(surface(
                frame,
                t_us,
                SURFACE_STAGE_SKIPPED,
                scope * 2 + 1,
                MISSING,
                MISSING,
                MISSING,
                SURFACE_REASON_INVISIBLE,
            ));
        } else {
            lines.push(surface(
                frame,
                t_us,
                SURFACE_STAGE_VISUALIZE,
                scope * 2 + 1,
                "288",
                "203",
                MISSING,
                MISSING,
            ));
        }
        t_us += 200;
        lines.push(ground(
            frame,
            scope,
            1704,
            wa_bottom,
            PlacementRoute::DpiReproject.as_str(),
        ));
    }
    lines.push(flush(frame, 100_500, STAGE_BEGIN, 6, MISSING));
    for scope in [1_u32, 0] {
        let deferred = deferred_balloon_scope == Some(scope);
        // バルーンの寸（要件 4.6 が先送りを許す側）。
        lines.push(write(
            if deferred {
                frame + DEFERRED_RESIZE_FRAMES
            } else {
                frame
            },
            t_us,
            STAGE_FLUSH,
            scope * 3,
            "0xB",
            PlacementRoute::KeepPositionResize.as_str(),
            &scope.to_string(),
            WindowKind::Balloon.as_str(),
            20_000,
        ));
        t_us += 60_000;
        // キャラ窓（位置＋寸を 1 回で）。
        lines.push(write(
            frame,
            t_us,
            STAGE_FLUSH,
            scope * 3 + 1,
            "0xC",
            PlacementRoute::DpiReproject.as_str(),
            &scope.to_string(),
            WindowKind::Char.as_str(),
            57_000,
        ));
        t_us += 35_000;
        // 随伴の位置（要件 4.3 が同一フレームで求める側）——見送られた遷移でも定刻に届く。
        lines.push(write(
            frame,
            t_us,
            STAGE_FLUSH,
            scope * 3 + 2,
            "0xB",
            PlacementRoute::BalloonFollow.as_str(),
            &scope.to_string(),
            WindowKind::Balloon.as_str(),
            34_000,
        ));
        t_us += 2_000;
    }
    lines.push(flush(frame, t_us, STAGE_END, 6, "171000"));
    lines
}

/// §3.2 の 6 遷移（方向と、遷移 2 の外れ値だけが違う）。
fn baseline_transitions() -> Vec<(usize, Vec<String>)> {
    [
        (1, 192, 96, None),
        (2, 96, 192, Some(1)),
        (3, 192, 96, None),
        (4, 96, 192, None),
        (5, 192, 96, None),
        (6, 96, 192, None),
    ]
    .into_iter()
    .enumerate()
    .map(|(index, (number, old_dpi, new_dpi, deferred))| {
        (
            number,
            baseline_transition(1_000 + index as u32 * 100, old_dpi, new_dpi, deferred),
        )
    })
    .collect()
}

/// 1 遷移ぶんを決定論の上限で判定する。
fn deterministic_verdict(lines: &[String]) -> Result<(), Vec<Violation>> {
    let records = parse_transition_log(&lines.join("\n"));
    let transitions = split_transitions(&records);
    assert_eq!(transitions.len(), 1);
    judge(&summarize(&transitions[0]), &Bounds::deterministic())
}

#[test]
fn every_baseline_transition_is_judged_exactly_as_the_report_describes_it() {
    // レポートが是正対象として記した量だけが出ること。6 遷移すべてでバルーン窓の書込 2 回
    // （§3.2「バルーン窓 2 回（`KeepPositionResize` 寸＋`BalloonFollow` 位置）」→ task 5.3）、
    // 192→96 の 3 遷移だけ接地点差 −48px（§6 の表・3/3 → task 5.1／5.2）。
    // フレーム量は是正前でも 0 なので、フレーム由来の違反は 1 件も出てはならない。
    for (number, lines) in baseline_transitions() {
        let violations = deterministic_verdict(&lines).expect_err("是正前なので合格しない");
        let shrinking = number % 2 == 1;
        let mut expected: Vec<Violation> = Vec::new();
        for scope in [0_u32, 1] {
            expected.push(Violation::WritesPerWindow {
                window: WindowKey::of(scope, WindowKind::Balloon),
                writes: 2,
                max: super::WRITES_PER_WINDOW_MAX,
            });
        }
        if shrinking {
            expected.push(Violation::GroundDiff {
                diff: -48,
                max: super::GROUND_DIFF_MAX,
            });
        }
        assert_eq!(
            violations, expected,
            "遷移 {number} の判定がレポートと食い違う"
        );
    }
}

#[test]
fn the_deferred_resize_of_transition_two_is_not_judged_as_a_defect() {
    // §3.2 の注「バルーン 1 …は遷移時点で不可視…だったため見送られ、+649ms に表示された
    // 時点で新寸へ（**Requirement 4.6 の現状維持挙動＝欠陥ではない**）」。判定器がこれを
    // 欠陥と呼ばないことは、同じ方向（96→192）の他の 2 遷移と**判定が一致する**ことで示す
    // ——「違反が少ない」ではなく「外れ値が判定に現れない」が主張である。
    let all = baseline_transitions();
    let outlier = deterministic_verdict(&all[1].1);
    for sibling in [3_usize, 5] {
        assert_eq!(
            outlier,
            deterministic_verdict(&all[sibling].1),
            "遷移 2 の外れ値（見送られた寸の遅延）が判定へ漏れている"
        );
    }
    let violations = outlier.expect_err("是正前なので合格しない");
    assert!(
        !violations.iter().any(|violation| matches!(
            violation,
            Violation::FramesToLastWrite { .. }
                | Violation::BalloonWrittenInAnotherFrame
                | Violation::Unmeasured(_)
        )),
        "先送りされた寸書込がフレーム量・随伴・未測定へ化けている: {violations:?}"
    );
}

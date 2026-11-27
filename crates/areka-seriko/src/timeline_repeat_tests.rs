//! `always` の繰り返しの計算（`lap_of`・`always_at`）の決定論テスト。
//!
//! 手書きの `always` の形＝作者のコマ列そのまま・周期は待ち時間の合計・回数なし。
//! 動く絵の子の形＝絵を指すコマ・pattern i の待ちはコマ i−1 の待ち時間（pattern0 は 0）・
//! 周期はコマの待ち時間の合計（最後のコマの待ち時間を含む）・回数はファイルの値。

use super::*;
use areka_emo_compose::ComposeMethod;
use dola::time::testing::ms;
use std::num::NonZeroU32;

// 手書きの `always` の 1 コマ（作者のサーフェスを指す）。
use crate::test_support::draw_frame as frame;

/// 手書きの `always` のコマ列と周期（＝待ち時間の合計）。
fn written(spec: &[(i64, u32)]) -> (Vec<LoopFrame>, TimeSpan) {
    let frames: Vec<LoopFrame> = spec.iter().map(|&(s, w)| frame(s, ms(w))).collect();
    let total: u32 = spec.iter().map(|&(_, w)| w).sum();
    (frames, ms(total))
}

/// 動く絵の子のコマ列と周期（コマの待ち時間 `delays` から作る・pattern i の待ち＝コマ i−1 の待ち時間）。
fn film(delays: &[u32]) -> (Vec<LoopFrame>, TimeSpan) {
    let frames: Vec<LoopFrame> = std::iter::once(0)
        .chain(delays[..delays.len() - 1].iter().copied())
        .enumerate()
        .map(|(i, wait_ms)| LoopFrame {
            surface_id: 0,
            picture: Some(i as u32),
            method: ComposeMethod::Overlay,
            act: crate::table::FrameAct::Draw,
            wait: ms(wait_ms),
            x: 0,
            y: 0,
        })
        .collect();
    let total: u32 = delays.iter().sum();
    (frames, ms(total))
}

fn laps(n: u32) -> Option<NonZeroU32> {
    Some(NonZeroU32::new(n).expect("正"))
}

use AlwaysView::{Frame, Nothing};

// ── lap_of ─────────────────────────────────────────────────────────────────

/// 何周目か（0 始まり）と周の頭からの経過。境目ちょうどは次の周の頭。
#[test]
fn lap_of_splits_elapsed_by_period() {
    let p = ms(150);
    assert_eq!(lap_of(ms(0), p), (0, ms(0)));
    assert_eq!(lap_of(ms(149), p), (0, ms(149)));
    assert_eq!(lap_of(ms(150), p), (1, ms(0)), "境目ちょうどは 2 周目の頭");
    assert_eq!(lap_of(ms(451), p), (3, ms(1)));
    let ticks_per_ms = ms(1).as_ticks();
    assert_eq!(
        lap_of(TimeSpan::MAX, ms(1)),
        (
            (TimeSpan::MAX.as_ticks() / ticks_per_ms) as u64,
            TimeSpan::try_from_ticks(TimeSpan::MAX.as_ticks() % ticks_per_ms).unwrap()
        ),
        "最大の経過でも丸めずに割る"
    );
}

// ── 待ち時間どおりの境目 ────────────────────────────────────────

/// 手書き: 待ち 50・30・20 → 境目は累積の 50・80・100 ちょうど（前後 1 ミリ秒も固定）。
#[test]
fn written_switches_exactly_at_cumulative_waits() {
    let (f, p) = written(&[(10, 50), (11, 30), (12, 20)]);
    let at = |e| always_at(&f, p, None, e);
    assert_eq!(at(ms(0)), Nothing, "1 周目の最初の待ちの前は何も出さない");
    assert_eq!(at(ms(49)), Nothing);
    assert_eq!(at(ms(50)), Frame(0));
    assert_eq!(at(ms(79)), Frame(0));
    assert_eq!(at(ms(80)), Frame(1));
    assert_eq!(at(ms(99)), Frame(1));
    // 100＝周の境目＝2 周目の最初の待ちの間は前の周の最後のコマ。
    assert_eq!(at(ms(100)), Frame(2));
    assert_eq!(at(ms(149)), Frame(2));
    assert_eq!(at(ms(150)), Frame(0));
}

/// 子: 待ち時間 40・60 → コマ 0 は 0〜39、コマ 1 は 40〜99、100 で頭へ（コマ 0 は経過 0 から見える）。
#[test]
fn film_switches_exactly_at_frame_delays() {
    let (f, p) = film(&[40, 60]);
    let at = |e| always_at(&f, p, None, e);
    assert_eq!(at(ms(0)), Frame(0), "子は経過 0 で 1 枚目");
    assert_eq!(at(ms(39)), Frame(0));
    assert_eq!(at(ms(40)), Frame(1));
    assert_eq!(at(ms(99)), Frame(1));
    assert_eq!(at(ms(100)), Frame(0), "最後のコマの待ち時間を終えたら頭へ");
}

/// 1 ミリ秒の待ちも丸めずそのまま使う（時刻は正確に）。
#[test]
fn one_ms_waits_are_not_rounded() {
    let (f, p) = film(&[1, 1, 1]);
    let at = |e| always_at(&f, p, None, e);
    assert_eq!(at(ms(0)), Frame(0));
    assert_eq!(at(ms(1)), Frame(1));
    assert_eq!(at(ms(2)), Frame(2));
    assert_eq!(at(ms(3)), Frame(0));
}

// ── 終わりなしの繰り返し ───────────────────────────────────────

/// 終わりなし: 何周しても同じ並びを繰り返し、2 周目以降の最初の待ちの間は前の周の最後のコマ。
#[test]
fn endless_repeats_and_holds_previous_last_before_first_wait() {
    let (f, p) = written(&[(10, 50), (11, 50)]);
    for lap in [1u32, 2, 1000] {
        let base = lap * 100;
        assert_eq!(
            always_at(&f, p, None, ms(base)),
            Frame(1),
            "{lap} 周目の頭は前の周の最後のコマ"
        );
        assert_eq!(always_at(&f, p, None, ms(base + 49)), Frame(1));
        assert_eq!(always_at(&f, p, None, ms(base + 50)), Frame(0));
        assert_eq!(always_at(&f, p, None, ms(base + 99)), Frame(0));
    }
}

/// 子の終わりなしも同じ 1 関数で頭へ戻る。
#[test]
fn endless_film_wraps_to_first_frame() {
    let (f, p) = film(&[100, 100]);
    assert_eq!(always_at(&f, p, None, ms(199)), Frame(1));
    assert_eq!(always_at(&f, p, None, ms(200)), Frame(0));
    assert_eq!(always_at(&f, p, None, ms(300)), Frame(1));
}

// ── 合計 N 回で最後のコマに止まる ───────────────────────────────────

/// 合計 2 回: 2 周目までは繰り返し、周期 × 2 以上は最後のコマのまま。
#[test]
fn finite_laps_stop_on_last_frame() {
    let (f, p) = film(&[40, 60]);
    let n = laps(2);
    assert_eq!(always_at(&f, p, n, ms(100)), Frame(0), "2 周目の頭");
    assert_eq!(always_at(&f, p, n, ms(199)), Frame(1), "2 周目の最後");
    assert_eq!(
        always_at(&f, p, n, ms(200)),
        Frame(1),
        "周期 × 2 で最後のコマに止まる"
    );
    assert_eq!(always_at(&f, p, n, ms(240)), Frame(1), "3 周目へ進まない");
    assert_eq!(always_at(&f, p, n, TimeSpan::MAX), Frame(1), "止まったまま");
}

/// 合計 1 回: 1 周で止まる（周期ちょうどで最後のコマ）。
#[test]
fn single_lap_stops_after_one_period() {
    let (f, p) = film(&[40, 60]);
    assert_eq!(always_at(&f, p, laps(1), ms(99)), Frame(1));
    assert_eq!(always_at(&f, p, laps(1), ms(100)), Frame(1));
    assert_eq!(
        always_at(&f, p, None, ms(100)),
        Frame(0),
        "終わりなしなら頭へ"
    );
}

// ── 待ち時間 0 のコマを飛ばす（検体 alpha.webp の 100・0・70・合計 3 回） ──────

/// alpha.webp: コマ 1 は待ち 0 ゆえ表示されない（同じ時刻のコマは後ろが勝つ）。
#[test]
fn zero_delay_frame_is_skipped_alpha_webp() {
    let (f, p) = film(&[100, 0, 70]);
    assert_eq!(p, ms(170));
    let n = laps(3);
    let seen: Vec<AlwaysView> = (0..510).map(|e| always_at(&f, p, n, ms(e))).collect();
    assert!(
        !seen.contains(&Frame(1)),
        "待ち 0 のコマ 1 は 1 ミリ秒も出ない"
    );
    assert_eq!(always_at(&f, p, n, ms(99)), Frame(0));
    assert_eq!(
        always_at(&f, p, n, ms(100)),
        Frame(2),
        "100 でコマ 1 を飛ばしてコマ 2"
    );
    assert_eq!(always_at(&f, p, n, ms(169)), Frame(2));
    assert_eq!(always_at(&f, p, n, ms(170)), Frame(0), "2 周目");
    assert_eq!(always_at(&f, p, n, ms(509)), Frame(2), "3 周目の最後");
    assert_eq!(
        always_at(&f, p, n, ms(510)),
        Frame(2),
        "合計 3 回で最後のコマに止まる"
    );
}

// ── 経過が 1 秒飛んだら 1 秒ぶん進む ────────────────────────────────

/// 手書きの形: 0 → 1000 で 1 秒ぶん進む（周期 150 の 6 周＋100 → コマ 1）。
#[test]
fn written_jump_of_one_second_advances_one_second() {
    let (f, p) = written(&[(10, 50), (11, 30), (12, 70)]);
    assert_eq!(
        always_at(&f, p, None, ms(1000)),
        Frame(1),
        "1000 = 150×6+100 → コマ 1"
    );
    assert_eq!(
        always_at(&f, p, None, ms(1050)),
        Frame(2),
        "1050 = 150×7 → 8 周目の最初の待ちの間＝前の周の最後のコマ 2"
    );
}

/// 子の形: 0 → 1000 で 1 秒ぶん進む（alpha.webp・終わりなしと合計 3 回の両方）。
#[test]
fn film_jump_of_one_second_advances_one_second() {
    let (f, p) = film(&[100, 0, 70]);
    // 1000 = 170×5+150 → コマ 2（終わりなし）。
    assert_eq!(always_at(&f, p, None, ms(1000)), Frame(2));
    // 1020 = 170×6 → 7 周目の頭＝コマ 0。
    assert_eq!(always_at(&f, p, None, ms(1020)), Frame(0));
    // 合計 3 回なら 510 以上は最後のコマ。
    assert_eq!(always_at(&f, p, laps(3), ms(1000)), Frame(2));
}

// ── 途中の -1 で消えて次で戻る ───────────────────────────────────────

/// 手書き: 途中の `-1` の間は何も出さず、次のコマで戻り、周を回っても続ける。
#[test]
fn negative_frame_hides_then_returns() {
    let (f, p) = written(&[(10, 50), (-1, 50), (12, 50)]);
    let at = |e| always_at(&f, p, None, e);
    assert_eq!(at(ms(50)), Frame(0));
    assert_eq!(at(ms(100)), Nothing, "-1 のコマに居る間は消える");
    assert_eq!(at(ms(149)), Nothing);
    // 150＝周の境目＝前の周の最後のコマ（12）で戻る。
    assert_eq!(at(ms(150)), Frame(2));
    assert_eq!(at(ms(200)), Frame(0), "2 周目も続ける");
    assert_eq!(at(ms(250)), Nothing);
}

/// 最後のコマが `-1` なら、2 周目以降の最初の待ちの間も何も出さない。
#[test]
fn negative_last_frame_shows_nothing_before_next_first_wait() {
    let (f, p) = written(&[(10, 50), (-1, 50)]);
    assert_eq!(always_at(&f, p, None, ms(100)), Nothing);
    assert_eq!(always_at(&f, p, None, ms(149)), Nothing);
    assert_eq!(always_at(&f, p, None, ms(150)), Frame(0));
}

/// 絵を指すコマ（子）は番号の符号に関わらず消えない（子の欄は「消えている」を持たない）。
#[test]
fn picture_frames_never_show_nothing() {
    let (mut f, p) = film(&[40, 60]);
    for fr in &mut f {
        fr.surface_id = -1;
    }
    for e in 0..300 {
        assert_ne!(always_at(&f, p, None, ms(e)), Nothing, "経過 {e}");
        assert_ne!(always_at(&f, p, laps(2), ms(e)), Nothing, "経過 {e}");
    }
}

// ── pattern0 の待ちが 0 の手書きの always の周の境目 ──────────────────────────

/// 待ち 0・100 の 2 コマ: 周の境目で最後のコマと次の周の pattern0 が同じ時刻になり後ろが勝つ
/// ＝最後のコマは 0 ミリ秒しか出ない（この帰結を固定する）。
#[test]
fn written_zero_first_wait_last_frame_shows_zero_ms() {
    let (f, p) = written(&[(10, 0), (11, 100)]);
    assert_eq!(p, ms(100));
    let seen: Vec<AlwaysView> = (0..1000).map(|e| always_at(&f, p, None, ms(e))).collect();
    assert!(
        seen.iter().all(|v| *v == Frame(0)),
        "コマ 1 は 1 ミリ秒も出ない"
    );
    assert_eq!(always_at(&f, p, None, ms(99)), Frame(0));
    assert_eq!(
        always_at(&f, p, None, ms(100)),
        Frame(0),
        "境目は次の周の pattern0"
    );
}

// ── 縮退 ──────────────────────────────────────────────────────────────────

/// コマ列が空なら何も出さない（表が採らない形でも panic しない）。
#[test]
fn empty_frames_show_nothing() {
    assert_eq!(always_at(&[], ms(100), None, ms(0)), Nothing);
    assert_eq!(always_at(&[], ms(100), laps(1), ms(500)), Nothing);
}

// ── 経過 0 の答えと合成の経過 0 のコマが一致する ────────────────

/// `always_at(…, 0)` は合成が描く経過 0 のコマ（`areka_emo_compose::rest_index` を同じ待ちへ当てたもの）
/// と同じ。手書きの形と子の形（待ち [0, d0, …]）の両方で。
#[test]
fn elapsed_zero_matches_compose_rest_index() {
    let cases: Vec<(&str, (Vec<LoopFrame>, TimeSpan))> = vec![
        ("手書き 0・0・100", written(&[(10, 0), (11, 0), (12, 100)])),
        (
            "手書き 50・50（先頭から待つ）",
            written(&[(10, 50), (11, 50)]),
        ),
        ("子 alpha.webp 100・0・70", film(&[100, 0, 70])),
        ("子 最初の待ち時間 0（0・50・30）", film(&[0, 50, 30])),
    ];
    for (name, (f, p)) in cases {
        let expected = match areka_emo_compose::rest_index(
            f.iter().map(|fr| u32::from(fr.wait != TimeSpan::ZERO)),
        ) {
            Some(i) => Frame(i),
            None => Nothing,
        };
        assert_eq!(always_at(&f, p, None, ms(0)), expected, "{name}");
        assert_eq!(
            always_at(&f, p, laps(1), ms(0)),
            expected,
            "{name}（回数つき）"
        );
    }
    // 値そのものも固定（合成の側の定義が変わったら気付く）。
    let (f, p) = written(&[(10, 0), (11, 0), (12, 100)]);
    assert_eq!(always_at(&f, p, None, ms(0)), Frame(1));
    let (f, p) = film(&[0, 50, 30]);
    assert_eq!(always_at(&f, p, None, ms(0)), Frame(1));
}

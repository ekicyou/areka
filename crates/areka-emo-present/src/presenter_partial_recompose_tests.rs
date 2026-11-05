//! コマ替えの矩形の重ね直し（`animated-surface-log-flood` 要件 3.1・3.3〜3.6・4.1・4.2・8.3・8.5）。
//!
//! 控えに外れて成立した適用ごとに、表示の行の直前に重ね直しの範囲の行が 1 行出る。動きのある面は、
//! 前に成立した適用が下敷き形の合成だったときだけ矩形（`partial`）で、それ以外はいちから（`full`）。
//! 動きの無い面は毎回いちから。どの適用でも絵は、同じ入力を新しい合成器でいちから合成した絵と同じ。
//! 数えて判定し、行が 0 件のまま緑にならないよう、どのテストも行の件数そのものを主張する。

use super::*;

use wintf::ecs::GraphicsCommandList;

use crate::display::DisplayFault;

use super::test_support::{
    CapturedEvent, INFO_LINE_MESSAGE, RANGE_LINE_MESSAGE, bake_world, capture, elem, gradient,
    invalidate_ok, make_world_with_gpu, mount_entities, native_golden_with, pattern_overlay_at,
    shell_of, show_ok, show_with_armed_fault, show_with_ok, spawn_window_with_dpi, surface,
};
use super::timing::PERF_LINE_MESSAGE;

const TARGET: TargetId = TargetId(0);
/// 下の面（動かない・16×12）。
const BASE_ID: u32 = 1000;
/// 同じ寸の 2 つ目の下の面（同じコマのまま面を替える回に使う・別のバイト）。
const OTHER_BASE_ID: u32 = 1001;
/// 動く絵のコマの面（4×3）。
const CELL_ID: u32 = 3000;
const BASE_SIZE: (u32, u32) = (16, 12);
const CELL_SIZE: (u32, u32) = (4, 3);
/// 動く絵の行（コマは `(x, CELL_Y)` に重なる）。
const CELL_Y: i64 = 2;

/// 下の面 2 つとコマの面の `(EmoWorld, AtlasTable)`（どれも全不透明・別のバイト）。
fn build_assets() -> (EmoWorld, AtlasTable) {
    let (bw, bh) = BASE_SIZE;
    let (cw, ch) = CELL_SIZE;
    bake_world(
        shell_of(vec![
            surface(BASE_ID, vec![elem("p.png", 0, 0)]),
            surface(CELL_ID, vec![elem("q.png", 0, 0)]),
            surface(OTHER_BASE_ID, vec![elem("r.png", 0, 0)]),
        ]),
        vec![
            ("p.png", bw, bh, gradient(bw, bh, 0x11)),
            ("q.png", cw, ch, gradient(cw, ch, 0x80)),
            ("r.png", bw, bh, gradient(bw, bh, 0x45)),
        ],
    )
}

/// コマ `x`（コマの面を `(x, CELL_Y)` に重ねる現在コマ）。
fn frame(x: i64) -> PatternState {
    pattern_overlay_at(2000, CELL_ID, x, CELL_Y)
}

fn setup() -> (World, EmoPresenter, EmoWorld, AtlasTable) {
    let mut world = make_world_with_gpu();
    let mut presenter = EmoPresenter::new();
    let window = spawn_window_with_dpi(&mut world, 96);
    let (emo_world, atlas) = build_assets();
    presenter.attach_target(TARGET, window, emo_world, atlas, 96);
    // 正解の絵を作るための同じ入力（attach は値を取るので作り直す）。
    let (emo_world, atlas) = build_assets();
    (world, presenter, emo_world, atlas)
}

fn show(presenter: &mut EmoPresenter, world: &mut World, pattern: PatternState) {
    show_with_ok(
        presenter,
        world,
        TARGET,
        BASE_ID,
        BindSet::default(),
        pattern,
    );
}

/// 表示中の絵が、同じ入力を新しい合成器でいちから合成した絵と同じこと（要件 3.3・4.1）。
fn assert_picture(presenter: &EmoPresenter, emo_world: &EmoWorld, atlas: &AtlasTable, x: i64) {
    let (golden, _) = native_golden_with(emo_world, atlas, BASE_ID, &BindSet::default(), &frame(x));
    assert_eq!(
        presenter.read_back(TARGET).expect("read_back"),
        golden,
        "コマ x={x} の絵がいちから合成と違う"
    );
}

/// 重ね直しの範囲の行。
#[derive(Debug, PartialEq)]
enum Range {
    Full,
    Partial { x: u32, y: u32, w: u32, h: u32 },
}

fn num(ev: &CapturedEvent, name: &str) -> u32 {
    ev.field(name)
        .unwrap_or_else(|| panic!("欄 {name} が無い: {ev:?}"))
        .parse()
        .unwrap_or_else(|_| panic!("欄 {name} が数でない: {ev:?}"))
}

/// 範囲の行を順に読む。どの範囲の行も直後が `cache_hit=false` の表示の行、その直後が perf の行
/// （要件 2.3・3.5）で、`cache_hit=false` の表示の行の数と範囲の行の数は同じ。
fn ranges(events: &[CapturedEvent]) -> Vec<Range> {
    let misses = events
        .iter()
        .filter(|e| e.message() == INFO_LINE_MESSAGE && e.field("cache_hit") == Some("false"))
        .count();
    let out: Vec<Range> = events
        .iter()
        .enumerate()
        .filter(|(_, e)| e.message() == RANGE_LINE_MESSAGE)
        .map(|(i, ev)| {
            assert_eq!(ev.level, tracing::Level::DEBUG, "範囲の行は DEBUG");
            let next = events.get(i + 1).expect("範囲の行の後に行が無い");
            assert_eq!(
                next.message(),
                INFO_LINE_MESSAGE,
                "範囲の行の直後が表示の行でない"
            );
            assert_eq!(next.field("cache_hit"), Some("false"));
            assert_eq!(
                events.get(i + 2).map(|e| e.message()),
                Some(PERF_LINE_MESSAGE),
                "表示の行の直後が perf の行でない"
            );
            let rect = (
                num(ev, "rect_x"),
                num(ev, "rect_y"),
                num(ev, "rect_w"),
                num(ev, "rect_h"),
            );
            match ev.field_str("recompose") {
                Some("full") => {
                    assert_eq!(rect, (0, 0, BASE_SIZE.0, BASE_SIZE.1), "full は全面の 4 値");
                    Range::Full
                }
                Some("partial") => Range::Partial {
                    x: rect.0,
                    y: rect.1,
                    w: rect.2,
                    h: rect.3,
                },
                other => panic!("recompose が full／partial でない: {other:?}"),
            }
        })
        .collect();
    assert_eq!(out.len(), misses, "範囲の行と外れの表示の行の数が違う");
    out
}

/// 要件 3.1・3.3・3.5・8.3: 動く絵の N 回のコマ替えで、範囲の行は最初の 1 回だけ `full`、残りは
/// `partial` で、矩形は動く絵の範囲（コマが重なりうる帯）に収まる。どの回の絵も正しい。
#[test]
fn animated_frame_changes_recompose_partially_after_the_first() {
    let (mut world, mut presenter, emo_world, atlas) = setup();
    let xs = [0i64, 2, 4, 6, 8, 10, 12];
    let ((), events) = capture(|| {
        for &x in &xs {
            show(&mut presenter, &mut world, frame(x));
            assert_picture(&presenter, &emo_world, &atlas, x);
        }
    });
    let got = ranges(&events);
    assert_eq!(got.len(), xs.len(), "外れの回ごとに範囲の行が 1 行");
    assert_eq!(got[0], Range::Full, "最初はいちから");
    for (i, r) in got.iter().enumerate().skip(1) {
        let Range::Partial { x, y, w, h } = *r else {
            panic!("{i} 回目が partial でない: {r:?}");
        };
        assert!(w > 0 && h > 0, "コマが替わったのに矩形が空: {r:?}");
        // 動く絵の範囲＝ x∈[0,16)・y∈[CELL_Y, CELL_Y+3)。
        assert!(
            x + w <= BASE_SIZE.0 && y >= CELL_Y as u32 && y + h <= CELL_Y as u32 + CELL_SIZE.1,
            "{i} 回目の矩形が動く絵の範囲の外: {r:?}"
        );
        // 前と今のコマの位置の和の内側（全面ではない）。
        let (prev, cur) = (xs[i - 1] as u32, xs[i] as u32);
        assert!(
            x <= prev && x + w >= cur + CELL_SIZE.0 && w < BASE_SIZE.0,
            "{i} 回目の矩形が前と今のコマの位置の和でない: {r:?}"
        );
    }
}

/// 要件 3.3・4.1: 下敷きの印が真のまま、同じコマで下の面を替えた回の絵も、新しい合成器でいちから
/// 合成した絵と同じ（矩形がコマの帯の外の下の面の画素を覆い損ねると前の面の画素が残る）。
#[test]
fn switching_the_surface_on_the_same_frame_matches_a_from_scratch_compose() {
    let (mut world, mut presenter, emo_world, atlas) = setup();
    let ((), events) = capture(|| {
        show(&mut presenter, &mut world, frame(0));
        show(&mut presenter, &mut world, frame(2));
        assert!(
            presenter.targets[&TARGET].partial_base,
            "前提: 下敷きの印が真"
        );
        show_with_ok(
            &mut presenter,
            &mut world,
            TARGET,
            OTHER_BASE_ID,
            BindSet::default(),
            frame(2),
        );
    });
    let (golden, _) = native_golden_with(
        &emo_world,
        &atlas,
        OTHER_BASE_ID,
        &BindSet::default(),
        &frame(2),
    );
    assert_eq!(
        presenter.read_back(TARGET).expect("read_back"),
        golden,
        "同じコマで面を替えた絵がいちから合成と違う"
    );
    let got = ranges(&events);
    assert_eq!(got.len(), 3, "外れは 3 回: {got:?}");
    // 空の道で緑にしない: 面を替えた回も下敷き形（`partial`）の道を通っている。
    assert!(
        matches!(got[2], Range::Partial { .. }),
        "面を替えた回が下敷き形の道を通らない: {got:?}"
    );
}

/// 要件 3.4: 動きの無い面（`pattern` が空）は毎回いちからで、合成の回数（外れの回数）は前と同じ
/// ——違う入力の数だけ。控えに当たる回は範囲の行を出さない。
#[test]
fn static_surface_always_recomposes_from_scratch() {
    let (mut world, mut presenter, _, _) = setup();
    let ((), events) = capture(|| {
        // 面 1000 → 面 3000 → 面 1000（当たり）→ 面 3000（当たり）。
        for id in [BASE_ID, CELL_ID, BASE_ID, CELL_ID] {
            show_ok(&mut presenter, &mut world, TARGET, id);
            // 動きの無い面の合成は下敷きの印を立てない（各回の直後に見る。当たりの回も印を立てない）。
            assert!(!presenter.targets[&TARGET].partial_base, "面 {id} の後");
        }
    });
    let misses: Vec<&CapturedEvent> = events
        .iter()
        .filter(|e| e.message() == RANGE_LINE_MESSAGE)
        .collect();
    assert_eq!(misses.len(), 2, "合成は違う入力の 2 回だけ");
    for ev in misses {
        assert_eq!(ev.field_str("recompose"), Some("full"));
    }
    let hits = events
        .iter()
        .filter(|e| e.message() == INFO_LINE_MESSAGE && e.field("cache_hit") == Some("true"))
        .count();
    assert_eq!(hits, 2, "控えに当たる回が 2 回");
}

/// 要件 3.1・3.5: 控えに当たった次の外れはいちから（当たりの回は下敷きの印を立てない）。
#[test]
fn miss_after_cache_hit_recomposes_from_scratch() {
    let (mut world, mut presenter, emo_world, atlas) = setup();
    let ((), events) = capture(|| {
        show(&mut presenter, &mut world, frame(0));
        show(&mut presenter, &mut world, frame(2));
        show(&mut presenter, &mut world, frame(0)); // 当たり
        show(&mut presenter, &mut world, frame(4));
        assert_picture(&presenter, &emo_world, &atlas, 4);
        show(&mut presenter, &mut world, frame(6));
        assert_picture(&presenter, &emo_world, &atlas, 6);
    });
    let got = ranges(&events);
    assert_eq!(got.len(), 4, "外れは 4 回: {got:?}");
    assert_eq!(got[0], Range::Full);
    assert!(matches!(got[1], Range::Partial { .. }));
    assert_eq!(got[2], Range::Full, "当たりの次の外れはいちから");
    assert!(
        matches!(got[3], Range::Partial { .. }),
        "その次は矩形へ戻る"
    );
}

/// 要件 3.1・4.2: 控えを捨てる指令の次はいちからで、絵が正しい。指令は下敷きの印を偽にする。
#[test]
fn miss_after_invalidate_recomposes_from_scratch() {
    let (mut world, mut presenter, emo_world, atlas) = setup();
    let ((), events) = capture(|| {
        show(&mut presenter, &mut world, frame(0));
        show(&mut presenter, &mut world, frame(2));
        assert!(presenter.targets[&TARGET].partial_base);
        invalidate_ok(&mut presenter, &mut world, TARGET);
        assert!(
            !presenter.targets[&TARGET].partial_base,
            "控えを捨てる指令で下敷きの印は偽"
        );
        show(&mut presenter, &mut world, frame(4));
        assert_picture(&presenter, &emo_world, &atlas, 4);
    });
    let got = ranges(&events);
    assert_eq!(got.len(), 3);
    assert!(matches!(got[1], Range::Partial { .. }));
    assert_eq!(got[2], Range::Full, "控えを捨てた次はいちから");
}

/// 要件 4.1・4.2・8.5: 表示の記録の失敗を注入した適用の次はいちからで、絵が正しい。失敗のあいだも
/// 前の表示（絵・表示記録・面・原寸）が残る。
///
/// 失敗した適用でも合成器の前の命令列は替わる（合成は成功している）。印を合成の前に偽にしないと、
/// 次の適用は前のコマの絵を、失敗したコマの命令列との違いで重ね直し、絵が崩れる。
#[test]
fn miss_after_display_record_failure_recomposes_from_scratch() {
    let (mut world, mut presenter, emo_world, atlas) = setup();
    let ((), events) = capture(|| {
        show(&mut presenter, &mut world, frame(0));
        show(&mut presenter, &mut world, frame(6));
        let (surface_entity, _) = mount_entities(&presenter, TARGET);
        let before = (
            presenter.read_back(TARGET).expect("read_back"),
            world.get::<GraphicsCommandList>(surface_entity).cloned(),
            presenter.current_surface_id(TARGET),
            presenter.targets[&TARGET].native_size,
            presenter.targets[&TARGET].last_show.clone(),
        );

        let outcome = show_with_armed_fault(
            &mut presenter,
            &mut world,
            TARGET,
            BASE_ID,
            BindSet::default(),
            frame(12),
            DisplayFault::CreateBitmap,
        );
        assert!(
            matches!(outcome, Err(PresentError::Device { .. })),
            "注入した失敗が返らない: {outcome:?}"
        );
        assert!(
            !presenter.targets[&TARGET].partial_base,
            "失敗した適用のあと下敷きの印は偽"
        );
        let after = (
            presenter.read_back(TARGET).expect("read_back"),
            world.get::<GraphicsCommandList>(surface_entity).cloned(),
            presenter.current_surface_id(TARGET),
            presenter.targets[&TARGET].native_size,
            presenter.targets[&TARGET].last_show.clone(),
        );
        assert_eq!(after, before, "失敗のあいだも前の表示が残る");

        show(&mut presenter, &mut world, frame(4));
        assert_picture(&presenter, &emo_world, &atlas, 4);
        show(&mut presenter, &mut world, frame(8));
        assert_picture(&presenter, &emo_world, &atlas, 8);
    });
    let got = ranges(&events);
    assert_eq!(
        got.len(),
        4,
        "成立した外れは 4 回（失敗した回は範囲の行を出さない）: {got:?}"
    );
    assert_eq!(got[0], Range::Full);
    assert!(matches!(got[1], Range::Partial { .. }));
    assert_eq!(got[2], Range::Full, "失敗の次はいちから");
    assert!(matches!(got[3], Range::Partial { .. }));
}

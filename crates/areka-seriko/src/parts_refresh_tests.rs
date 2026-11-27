//! 部品の時計の檻（出来事の直後の作り直し・時計を捨てる口・記録）。

use areka_emo_compose::{BindSet, PatternState};
use areka_sakura::ActorKey;
use dola::time::testing::{at_ms, ms};

use super::tests::{calls, cell_of, count, no_binds, overlay, refresh_at, tick, tick_at};
use super::{PartAnim, PartClocks};
use crate::looper::tests::counting_rng;
use crate::state::Slot;
use crate::test_support::{capture_logs, part_frames, part_playing, scope, table_of};

// ── 出来事の直後の作り直し・時計を捨てる口・記録 ──

/// 子 100 は 50ms で 101 → さらに 50ms で 102（`-1` の無い末尾＝保つ）。
const HOLD: &str = "surface0\n{\nelement0,overlay,100,0,0\n}\nsurface1\n{\n}\n\
    surface100\n{\nanimation0.interval,random,2\n\
    animation0.pattern0,overlay,101,50,0,0\n\
    animation0.pattern1,overlay,102,50,0,0\n}\n";

/// 出来事の直後の作り直しは抽選の乱数を呼ばず、抽選の animation の時計を変えない。末尾に着いた時刻を
/// 見ても `Residual` へ移さない（移すのは次の刻み）。何度呼んでも、次の刻みの結果は呼ばなかったときと
/// 同じ。
#[test]
fn peek_draws_no_rng_and_leaves_clocks_unchanged() {
    let table = table_of(HOLD);
    let b = no_binds();
    let top = PatternState::default();

    // 呼ばない側（比べる相手）。
    let (mut rng0, probe0) = counting_rng(&[0, 1]);
    let mut plain = PartClocks::default();
    tick(
        &mut plain,
        &table,
        0,
        &b,
        at_ms(1000),
        true,
        &mut rng0,
        &top,
    );
    let expect_1100 = tick(
        &mut plain,
        &table,
        0,
        &b,
        at_ms(1100),
        false,
        &mut rng0,
        &top,
    );
    let expect_2000 = tick(
        &mut plain,
        &table,
        0,
        &b,
        at_ms(2000),
        true,
        &mut rng0,
        &top,
    );

    // 呼ぶ側。
    let (mut rng, probe) = counting_rng(&[0, 1]);
    let mut clocks = PartClocks::default();
    // 時計が 1 本も無いうちは欄が空になる（前の写しの欄も消える）。
    let mut stale = PatternState::default();
    stale.set_part(100, 0, overlay(999));
    assert!(refresh_at(&mut clocks, &table, 0, &b, at_ms(900), &stale).is_empty());

    tick(
        &mut clocks,
        &table,
        0,
        &b,
        at_ms(1000),
        true,
        &mut rng,
        &top,
    );
    assert_eq!(calls(&probe), 1);
    let playing = part_playing(at_ms(1000));

    // 同じ時刻で 3 回ずつ呼ぶ（時刻は戻さない）。
    for _ in 0..3 {
        assert!(
            part_frames(
                &refresh_at(&mut clocks, &table, 0, &b, at_ms(1000), &top),
                100
            )
            .is_empty(),
            "Pending はコマ無し"
        );
        assert_eq!(clocks.clock(&scope(), 100, 0), playing);
    }
    for _ in 0..3 {
        assert_eq!(
            part_frames(
                &refresh_at(&mut clocks, &table, 0, &b, at_ms(1060), &stale),
                100
            ),
            vec![(0, 101)],
            "前の写しの欄は作り直される"
        );
        assert_eq!(clocks.clock(&scope(), 100, 0), playing);
    }
    for _ in 0..3 {
        // 末尾の時刻: コマは出すが時計は `Playing` のまま。
        assert_eq!(
            part_frames(
                &refresh_at(&mut clocks, &table, 0, &b, at_ms(1100), &top),
                100
            ),
            vec![(0, 102)]
        );
        // 見えない面では何も書かない。
        assert!(refresh_at(&mut clocks, &table, 1, &b, at_ms(1100), &stale).is_empty());
        assert_eq!(clocks.clock(&scope(), 100, 0), playing);
    }
    assert_eq!(
        calls(&probe),
        1,
        "出来事の直後の作り直しは抽選の乱数を呼ばない"
    );

    // 次の刻みの結果は、呼ばなかった側と同じ。
    assert_eq!(
        tick(
            &mut clocks,
            &table,
            0,
            &b,
            at_ms(1100),
            false,
            &mut rng,
            &top
        ),
        expect_1100
    );
    assert_eq!(
        clocks.clock(&scope(), 100, 0),
        Some(PartAnim::Residual { frame_index: 1 })
    );
    // 保っている間も同じ。
    for _ in 0..3 {
        assert_eq!(
            part_frames(
                &refresh_at(&mut clocks, &table, 0, &b, at_ms(1500), &top),
                100
            ),
            vec![(0, 102)]
        );
    }
    assert_eq!(
        tick(
            &mut clocks,
            &table,
            0,
            &b,
            at_ms(2000),
            true,
            &mut rng,
            &top
        ),
        expect_2000
    );
    assert_eq!(calls(&probe), calls(&probe0));
}

/// 出来事の直後の作り直しでも、有効でない `bind+random` のコマは欄に書かない。時計は消さない
/// （消すのは次の刻み）。
#[test]
fn peek_does_not_write_inactive_bind_random_frames() {
    let table = table_of(
        "surface0\n{\nelement0,overlay,100,0,0\n}\n\
         surface100\n{\nanimation5.interval,bind+random,2\n\
         animation5.pattern0,overlay,101,0,0,0\n\
         animation5.pattern1,overlay,102,100,0,0\n\
         animation6.interval,random,2\n\
         animation6.pattern0,overlay,103,0,0,0\n}\n",
    );
    let (mut rng, _probe) = counting_rng(&[0, 0]);
    let mut clocks = PartClocks::default();
    let top = PatternState::default();
    let on = BindSet::from_ids([5]);
    let off = no_binds();

    tick(
        &mut clocks,
        &table,
        0,
        &on,
        at_ms(1000),
        true,
        &mut rng,
        &top,
    );
    assert_eq!(
        part_frames(
            &refresh_at(&mut clocks, &table, 0, &on, at_ms(1050), &top),
            100
        ),
        vec![(5, 101), (6, 103)]
    );

    // 外した直後: 5 のコマは書かず、`random` の 6 は書く。5 の時計は残る。
    assert_eq!(
        part_frames(
            &refresh_at(&mut clocks, &table, 0, &off, at_ms(1050), &top),
            100
        ),
        vec![(6, 103)]
    );
    assert_eq!(clocks.clock(&scope(), 100, 5), part_playing(at_ms(1000)));

    // 次の刻みで消える。
    tick(
        &mut clocks,
        &table,
        0,
        &off,
        at_ms(1100),
        false,
        &mut rng,
        &top,
    );
    assert_eq!(clocks.clock(&scope(), 100, 5), None);
}

/// 時計を捨てる口は全スコープの時計を捨てる。
#[test]
fn clear_drops_clocks_of_every_scope() {
    let table = table_of(HOLD);
    let (mut rng, _probe) = counting_rng(&[0, 0]);
    let mut clocks = PartClocks::default();
    let b = no_binds();
    let other = ActorKey::from("1");

    tick(
        &mut clocks,
        &table,
        0,
        &b,
        at_ms(1000),
        true,
        &mut rng,
        &PatternState::default(),
    );
    tick_at(
        &mut clocks,
        &table,
        &other,
        0,
        &b,
        at_ms(1000),
        true,
        &mut rng,
        &PatternState::default(),
    );
    assert!(clocks.clock(&scope(), 100, 0).is_some());
    assert!(clocks.clock(&other, 100, 0).is_some());

    clocks.clear(Slot::Shell);
    assert_eq!(clocks.clock(&scope(), 100, 0), None);
    assert_eq!(clocks.clock(&other, 100, 0), None);
    assert!(
        refresh_at(
            &mut clocks,
            &table,
            0,
            &b,
            at_ms(1060),
            &PatternState::default()
        )
        .is_empty()
    );
}

/// `-1` 以外の負の番号は (スコープ, 部品, animation) ごとに初回だけ `warn!` を出し、その animation を
/// 止める。捨てた後は再び初回として扱う。`-1` では出さない。
#[test]
fn negative_id_other_than_minus_one_warns_once_per_scope_part_animation() {
    let table = table_of(
        "surface0\n{\nelement0,overlay,100,0,0\nelement1,overlay,200,0,0\n}\n\
         surface100\n{\nanimation0.interval,random,2\n\
         animation0.pattern0,overlay,101,0,0,0\n\
         animation0.pattern1,overlay,-2,50,0,0\n}\n\
         surface200\n{\nanimation0.interval,random,2\n\
         animation0.pattern0,overlay,201,0,0,0\n\
         animation0.pattern1,overlay,-1,50,0,0\n}\n",
    );
    let (mut rng, _probe) = counting_rng(&[0; 16]);
    let mut clocks = PartClocks::default();
    let b = no_binds();
    let top = PatternState::default();
    let other = ActorKey::from("1");

    let lines = capture_logs(|| {
        for base in [at_ms(1000), at_ms(2000), at_ms(3000)] {
            tick(&mut clocks, &table, 0, &b, base, true, &mut rng, &top);
            let p = tick(
                &mut clocks,
                &table,
                0,
                &b,
                base + ms(50),
                false,
                &mut rng,
                &top,
            );
            assert!(part_frames(&p, 100).is_empty(), "負の番号で止まる");
            assert_eq!(clocks.clock(&scope(), 100, 0), None);
        }
        // 別のスコープは別に数える。
        for base in [at_ms(1000), at_ms(2000)] {
            tick_at(
                &mut clocks,
                &table,
                &other,
                0,
                &b,
                base,
                true,
                &mut rng,
                &PatternState::default(),
            );
            tick_at(
                &mut clocks,
                &table,
                &other,
                0,
                &b,
                base + ms(50),
                false,
                &mut rng,
                &PatternState::default(),
            );
        }
    });
    let warn = ["level=WARN", "seriko: part", "part=100", "surface_id=-2"];
    assert_eq!(
        count(&lines, &warn),
        2,
        "スコープごとに初回だけ: {lines:#?}"
    );
    assert_eq!(count(&lines, &["level=WARN", "scope=\"0\""]), 1);
    assert_eq!(count(&lines, &["level=WARN", "scope=\"1\""]), 1);
    assert_eq!(
        count(&lines, &["level=WARN", "part=200"]),
        0,
        "`-1` では出さない"
    );
    assert_eq!(
        count(&lines, &["level=INFO", "seriko: part 停止", "part=100"]),
        5,
        "停止は毎回 info で残る（陽性対照）"
    );

    // 捨てた後は初回に戻る。
    clocks.clear(Slot::Shell);
    let lines = capture_logs(|| {
        tick(
            &mut clocks,
            &table,
            0,
            &b,
            at_ms(5000),
            true,
            &mut rng,
            &top,
        );
        tick(
            &mut clocks,
            &table,
            0,
            &b,
            at_ms(5050),
            false,
            &mut rng,
            &top,
        );
    });
    assert_eq!(count(&lines, &warn), 1, "{lines:#?}");
}

/// 部品の発火・末尾での保持・停止・着せ替えから外れた停止は `info!` で部品の番号の欄つきで残る。
/// 出来事の直後の作り直しは、抽選の発火・末尾での保持・停止の記録を出さない（出すのは `advance` だけ・
/// 同じ捕捉の中で刻みの行が出ることを陽性対照にする）。
#[test]
fn part_fire_hold_and_stop_lines_carry_the_part_number() {
    let table = table_of(
        "surface0\n{\nelement0,overlay,100,0,0\n}\n\
         surface100\n{\nanimation0.interval,random,2\n\
         animation0.pattern0,overlay,101,50,0,0\n\
         animation0.pattern1,overlay,102,50,0,0\n\
         animation3.interval,random,2\n\
         animation3.pattern0,overlay,103,0,0,0\n\
         animation3.pattern1,overlay,-1,50,0,0\n\
         animation5.interval,bind+random,2\n\
         animation5.pattern0,overlay,105,0,0,0\n}\n",
    );
    let (mut rng, _probe) = counting_rng(&[0, 0, 0]);
    let mut clocks = PartClocks::default();
    let top = PatternState::default();
    let on = BindSet::from_ids([5]);
    let off = no_binds();

    let lines = capture_logs(|| {
        tick(
            &mut clocks,
            &table,
            0,
            &on,
            at_ms(1000),
            true,
            &mut rng,
            &top,
        );
        for _ in 0..3 {
            refresh_at(&mut clocks, &table, 0, &on, at_ms(1100), &top);
            refresh_at(&mut clocks, &table, 0, &off, at_ms(1100), &top);
        }
        tick(
            &mut clocks,
            &table,
            0,
            &on,
            at_ms(1100),
            false,
            &mut rng,
            &top,
        );
        tick(
            &mut clocks,
            &table,
            0,
            &off,
            at_ms(1150),
            false,
            &mut rng,
            &top,
        );
        tick(
            &mut clocks,
            &table,
            0,
            &off,
            at_ms(1200),
            false,
            &mut rng,
            &top,
        );
    });
    let part_lines: Vec<&String> = lines
        .iter()
        .filter(|l| l.contains("seriko: part"))
        .collect();
    let fire = [
        "level=INFO",
        "seriko: part 抽選発火",
        "scope=\"0\"",
        "part=100",
    ];
    assert_eq!(count(&lines, &fire), 3, "{part_lines:#?}");
    for id in [0, 3, 5] {
        let id_field = format!("animation_id={id} ");
        let mut needles = fire.to_vec();
        needles.push(&id_field);
        assert_eq!(count(&lines, &needles), 1, "{part_lines:#?}");
    }
    assert_eq!(
        count(
            &lines,
            &[
                "level=INFO",
                "seriko: part 末尾残留",
                "part=100",
                "animation_id=0"
            ]
        ),
        1,
        "末尾での保持は移った刻みの 1 回だけ（出来事の直後の作り直しは保持の記録を出さない）: {part_lines:#?}"
    );
    assert_eq!(
        count(
            &lines,
            &[
                "level=INFO",
                "seriko: part 停止",
                "part=100",
                "animation_id=3"
            ]
        ),
        1,
        "{part_lines:#?}"
    );
    assert_eq!(
        count(
            &lines,
            &[
                "level=INFO",
                "seriko: part bind から外れた",
                "part=100",
                "animation_id=5"
            ]
        ),
        1,
        "外れた刻みの 1 回だけ（次の刻みでは時計が無いので出さない）: {part_lines:#?}"
    );
    // 発火 3・末尾残留 2（5 は待ち 0 の 1 コマなので発火の刻みで保つ）・停止 1・外れた停止 1。
    assert_eq!(part_lines.len(), 7, "{part_lines:#?}");
}

/// 呼ばれるだけ（純 `bind`）の部品の animation は、着せ替えに無くても在る時計を消さずに進め、
/// 自分では始めない。着せ替えの番人は `bind+random` だけ。
#[test]
fn call_only_part_keeps_and_advances_an_existing_clock_without_starting_itself() {
    let table = table_of(
        "surface0\n{\nelement0,overlay,100,0,0\n}\n\
         surface100\n{\nanimation0.interval,bind\n\
         animation0.pattern0,overlay,101,50,0,0\n\
         animation0.pattern1,overlay,102,50,0,0\n}\n",
    );
    let (mut rng, probe) = counting_rng(&[]);
    let mut clocks = PartClocks::default();
    let top = PatternState::default();
    let b = no_binds();

    // 境界を跨いでも自分では始めない（乱数も引かない）。
    let p = tick(
        &mut clocks,
        &table,
        0,
        &b,
        at_ms(1000),
        true,
        &mut rng,
        &top,
    );
    assert!(p.is_empty());
    assert_eq!(clocks.clock(&scope(), 100, 0), None);

    // 呼ばれて時計が在る（呼び出しの経路の代わりに手で置く）。
    clocks
        .clocks
        .get_mut(&scope())
        .and_then(|slots| slots.get_mut(&Slot::Shell))
        .expect("advance が入れ物を作る")
        .clocks
        .insert(
            (areka_emo_compose::PartKey::Surface(100), 0),
            PartAnim::Playing {
                started_at: at_ms(1000),
                acts_through: None,
            },
        );

    // 着せ替えに無くても消さずに経過から進める。
    let p = tick(
        &mut clocks,
        &table,
        0,
        &b,
        at_ms(1060),
        true,
        &mut rng,
        &top,
    );
    assert_eq!(part_frames(&p, 100), vec![(0, 101)]);
    assert_eq!(clocks.clock(&scope(), 100, 0), part_playing(at_ms(1000)));

    // 抽選も作り直しも同じ時計を読む（`refresh` も欄を書く）。
    let p = refresh_at(&mut clocks, &table, 0, &b, at_ms(1110), &top);
    assert_eq!(part_frames(&p, 100), vec![(0, 102)]);
    assert_eq!(
        calls(&probe),
        0,
        "呼ばれるだけの animation は乱数を引かない"
    );
}

/// 一番上 0 が子 40 を置き、2 は何も置かない。40 の animation0 は random,2 で 42（すぐ）→ 1000ms で `-1`。
const HIDDEN_END: &str = "surface0\n{\nelement0,overlay,40,0,0\n}\n\
    surface2\n{\n}\n\
    surface40\n{\nanimation0.interval,random,2\n\
    animation0.pattern0,overlay,42,0,0,0\n\
    animation0.pattern1,overlay,-1,1000,0,0\n}\n\
    surface42\n{\n}\n";

/// 見えない間に実は終わっていた再生は、見え直した境目の刻みで抽選に戻る（1 回分遅れない・
/// 時刻は正確に扱う）。
#[test]
fn play_that_ended_while_hidden_is_lotteried_on_the_reshow_tick() {
    let table = table_of(HIDDEN_END);
    let (mut rng, probe) = counting_rng(&[0, 0]);
    let mut clocks = PartClocks::default();
    let (b, top) = (BindSet::default(), PatternState::default());

    tick(
        &mut clocks,
        &table,
        0,
        &b,
        at_ms(1000),
        true,
        &mut rng,
        &top,
    );
    assert_eq!(clocks.clock(&scope(), 40, 0), part_playing(at_ms(1000)));
    tick(
        &mut clocks,
        &table,
        2,
        &b,
        at_ms(1100),
        false,
        &mut rng,
        &top,
    ); // 40 が見えなくなる。
    tick(
        &mut clocks,
        &table,
        2,
        &b,
        at_ms(2000),
        true,
        &mut rng,
        &top,
    ); // 実際の時間ではここで終わる。
    assert_eq!(
        clocks.clock(&scope(), 40, 0),
        part_playing(at_ms(1000)),
        "見えない間は残る"
    );

    let p = tick(
        &mut clocks,
        &table,
        0,
        &b,
        at_ms(3000),
        true,
        &mut rng,
        &top,
    );
    assert_eq!(probe.lock().unwrap().calls, 2, "見え直した刻みで抽選する");
    assert_eq!(clocks.clock(&scope(), 40, 0), part_playing(at_ms(3000)));
    assert_eq!(cell_of(&p, 40), Some(Some(42)), "新しい再生の頭のコマ");
}

/// 部品でも、`-1` と `-2` のすぐ後に待ち 0 の働きのコマがあるとき、warn の番号は止めた描くコマの番号:
/// `-1` は warn しない、`-2` は 1 回だけ `surface_id=-2` で warn する。続くのが待ち 0 の描くコマでも
/// 同じ。
#[test]
fn part_negative_then_zero_wait_action_frame_warns_with_the_drawn_number() {
    for tail in ["insert,0,0,0,0", "overlay,42,0,0,0"] {
        let table = table_of(
            &"surface0\n{\nelement0,overlay,40,0,0\n}\n\
         surface40\n{\nanimation0.interval,random,2\n\
         animation0.pattern0,overlay,42,0,0,0\n\
         animation0.pattern1,overlay,-1,50,0,0\n\
         animation0.pattern2,insert,0,0,0,0\n\
         animation1.interval,random,2\n\
         animation1.pattern0,overlay,42,0,0,0\n\
         animation1.pattern1,overlay,-2,50,0,0\n\
         animation1.pattern2,insert,0,0,0,0\n}\n\
         surface42\n{\n}\n"
                .replace("insert,0,0,0,0", tail),
        );
        let (mut rng, _probe) = counting_rng(&[0, 0]);
        let mut clocks = PartClocks::default();
        let (b, top) = (BindSet::default(), PatternState::default());
        let lines = capture_logs(|| {
            tick(
                &mut clocks,
                &table,
                0,
                &b,
                at_ms(1000),
                true,
                &mut rng,
                &top,
            );
            tick(
                &mut clocks,
                &table,
                0,
                &b,
                at_ms(1050),
                false,
                &mut rng,
                &top,
            );
        });
        assert_eq!(clocks.clock(&scope(), 40, 0), None, "-1 で止まる");
        assert_eq!(clocks.clock(&scope(), 40, 1), None, "-2 で止まる");
        let warned: Vec<_> = lines
            .iter()
            .filter(|l| l.contains("以外の負 surface"))
            .collect();
        assert_eq!(warned.len(), 1, "{warned:?}");
        assert!(warned[0].contains("animation_id=1"), "{warned:?}");
        assert!(warned[0].contains("surface_id=-2"), "{warned:?}");
    }
}

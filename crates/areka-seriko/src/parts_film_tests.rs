//! 動く絵の子の時計の檻（spec: areka-P0-animated-image-playback 要件 2.3・2.7・2.8・3.1〜3.5・6.6・
//! tasks.md 3.3）。
//!
//! seriko は読み込みの側（アトラス）に依存しないので、子の定義は分解が作る値と同じ形を手で組み、
//! 面の表の element の種類も分解と同じに `ElementKind::Film` へ替えて
//! [`AnimationTable::from_world_and_films`] へ渡す（`table_film_tests.rs` と同じ組み方）。

use std::num::NonZeroU32;

use areka_emo_compose::{
    BindSet, Cell, ElementKind, EmoWorld, FilmId, FilmSheet, PartKey, PatternState, SurfaceMaster,
};
use areka_sakura::ActorKey;
use log_capture_kit::{LineFormat, capture_lines};

use super::{PartAnim, PartClocks};
use crate::looper::tests::counting_rng;
use crate::state::Slot;
use crate::table::AnimationTable;
use crate::timeline::LoopRng;

/// 終わりなしの子（コマ 10・11・12、各 100ms・周期 300）。
pub(crate) const ENDLESS: FilmId = FilmId(10);
/// 合計 1 回の子（コマ 20・21、各 100ms・周期 200）。
pub(crate) const ONCE: FilmId = FilmId(20);

/// 0 と 1 は `a.png`（終わりなし）を置き、1 は `b.png`（合計 1 回）も置く。2 は何も置かない。
/// 3 は `b.png` だけ。4 は子 100 を置き、100 が `a.png` を置く。
const SHELL: &str = "surface0\n{\nelement0,overlay,a.png,0,0\n}\n\
    surface1\n{\nelement0,overlay,a.png,5,5\nelement1,overlay,b.png,0,0\n}\n\
    surface2\n{\n}\n\
    surface3\n{\nelement0,overlay,b.png,0,0\n}\n\
    surface4\n{\nelement0,overlay,100,0,0\n}\n\
    surface100\n{\nelement0,overlay,a.png,0,0\n}\n";

fn sheet(id: FilmId, delays_ms: &[u32], laps: Option<u32>) -> FilmSheet {
    FilmSheet {
        id,
        path: format!("{}.png", id.0),
        frames: (0..delays_ms.len() as u32).map(|i| id.0 + i).collect(),
        delays_ms: delays_ms.to_vec(),
        laps: laps.and_then(NonZeroU32::new),
        original: (2, 2),
        rest: 0,
    }
}

pub(crate) fn table() -> AnimationTable {
    let mut world = EmoWorld::build(&areka_parsers::shell::parse(SHELL));
    let w = world.world_mut();
    let mut q = w.query::<&mut SurfaceMaster>();
    for mut master in q.iter_mut(w) {
        for e in &mut master.elements {
            let film = match e.path.as_str() {
                "a.png" => ENDLESS,
                "b.png" => ONCE,
                _ => continue,
            };
            e.kind = ElementKind::Film(film);
        }
    }
    let sheets = [
        sheet(ENDLESS, &[100, 100, 100], None),
        sheet(ONCE, &[100, 100], Some(1)),
    ];
    AnimationTable::from_world_and_films(&world, sheets.iter(), &[])
}

fn scope() -> ActorKey {
    ActorKey::from("0")
}

/// `scope` の `slot` の面、一番上 `top` で刻みを 1 回回し、部品の欄を作り直した絵を返す。
#[allow(clippy::too_many_arguments)]
fn tick_at(
    clocks: &mut PartClocks,
    table: &AnimationTable,
    scope: &ActorKey,
    slot: Slot,
    top: u32,
    now_ms: u64,
    rng: &mut LoopRng,
) -> PatternState {
    let mut p = PatternState::default();
    clocks.advance(
        scope,
        slot,
        top,
        &BindSet::default(),
        table,
        now_ms,
        true,
        rng,
        &mut p,
    );
    p
}

fn tick(
    clocks: &mut PartClocks,
    table: &AnimationTable,
    top: u32,
    now_ms: u64,
    rng: &mut LoopRng,
) -> PatternState {
    tick_at(clocks, table, &scope(), Slot::Shell, top, now_ms, rng)
}

/// 子の欄の絵の番号（載っていなければ `None`＝経過 0）。
fn picture(p: &PatternState, film: FilmId) -> Option<u32> {
    match p.cell(Some(PartKey::Film(film)), 0) {
        Cell::Picture(id) => Some(id),
        Cell::Rest => None,
        other => panic!("子の欄に {other:?}"),
    }
}

fn clock(clocks: &PartClocks, slot: Slot, film: FilmId) -> Option<PartAnim> {
    clocks.clock_at(&scope(), slot, PartKey::Film(film), 0)
}

fn playing(at: u64) -> Option<PartAnim> {
    Some(PartAnim::Playing { started_at_ms: at })
}

/// 見えた刻みの時刻で、乱数を引かずに時計が生まれる。経過 0（1 枚目）は欄に載らない（要件 3.1・7.4）。
#[test]
fn film_clock_is_born_when_it_becomes_visible() {
    let table = table();
    let (mut rng, probe) = counting_rng(&[]);
    let mut clocks = PartClocks::default();

    let p = tick(&mut clocks, &table, 2, 500, &mut rng);
    assert!(p.is_empty());
    assert_eq!(clock(&clocks, Slot::Shell, ENDLESS), None);

    let p = tick(&mut clocks, &table, 0, 1000, &mut rng);
    assert_eq!(clock(&clocks, Slot::Shell, ENDLESS), playing(1000));
    assert!(p.is_empty(), "1 枚目は経過 0 と同じ");
    let p = tick(&mut clocks, &table, 0, 1100, &mut rng);
    assert_eq!(picture(&p, ENDLESS), Some(11));
    let p = tick(&mut clocks, &table, 0, 1300, &mut rng);
    assert!(p.is_empty(), "2 周目の頭は 1 枚目");
    assert_eq!(probe.lock().unwrap().calls, 0, "子は乱数を引かない");
}

/// 終わりなし: 同じ絵を置く別のサーフェスへ替えても続き。置いていないサーフェスへ行って戻っても、
/// 見えなかった間も進んでいる（要件 3.2・3.3）。
#[test]
fn endless_film_continues_across_surfaces_and_while_hidden() {
    let table = table();
    let (mut rng, _probe) = counting_rng(&[]);
    let mut clocks = PartClocks::default();

    tick(&mut clocks, &table, 0, 1000, &mut rng);
    let p = tick(&mut clocks, &table, 1, 1150, &mut rng);
    assert_eq!(
        picture(&p, ENDLESS),
        Some(11),
        "同じ絵を置く別のサーフェスで続き"
    );

    let p = tick(&mut clocks, &table, 2, 1160, &mut rng);
    assert!(p.is_empty());
    assert_eq!(
        clock(&clocks, Slot::Shell, ENDLESS),
        playing(1000),
        "終わりなしは捨てない"
    );
    let p = tick(&mut clocks, &table, 0, 1250, &mut rng);
    assert_eq!(picture(&p, ENDLESS), Some(12), "見えなかった間も進んでいる");
}

/// 子のサーフェスに置いた絵も、親の切り替えで巻き戻らない（要件 1.9・4.4）。
#[test]
fn film_in_child_surface_does_not_rewind() {
    let table = table();
    let (mut rng, _probe) = counting_rng(&[]);
    let mut clocks = PartClocks::default();

    let p = tick(&mut clocks, &table, 4, 1000, &mut rng);
    assert!(p.is_empty());
    assert_eq!(clock(&clocks, Slot::Shell, ENDLESS), playing(1000));
    let p = tick(&mut clocks, &table, 4, 1100, &mut rng);
    assert_eq!(picture(&p, ENDLESS), Some(11));
    let p = tick(&mut clocks, &table, 0, 1200, &mut rng);
    assert_eq!(picture(&p, ENDLESS), Some(12), "子から一番上へ移っても続き");
}

/// 回数つき: 見えなくなった評価で捨てられ、戻ると頭から（要件 2.3）。生まれた・捨てたは `debug!`。
#[test]
fn finite_film_is_dropped_when_hidden_and_restarts_from_head() {
    let table = table();
    let (mut rng, _probe) = counting_rng(&[]);
    let mut clocks = PartClocks::default();

    let (_, lines) = capture_lines(LineFormat::LevelTargetFields, || {
        tick(&mut clocks, &table, 3, 1000, &mut rng);
        let p = tick(&mut clocks, &table, 3, 1100, &mut rng);
        assert_eq!(picture(&p, ONCE), Some(21));

        let p = tick(&mut clocks, &table, 2, 1150, &mut rng);
        assert!(p.is_empty());
        assert_eq!(
            clock(&clocks, Slot::Shell, ONCE),
            None,
            "見えなくなって捨てた"
        );

        let p = tick(&mut clocks, &table, 3, 1500, &mut rng);
        assert_eq!(
            clock(&clocks, Slot::Shell, ONCE),
            playing(1500),
            "戻ると頭から"
        );
        assert!(p.is_empty(), "頭＝経過 0");
        let p = tick(&mut clocks, &table, 3, 1600, &mut rng);
        assert_eq!(picture(&p, ONCE), Some(21));
    });
    let debug = |needle: &str| {
        lines
            .iter()
            .filter(|l| {
                l.contains("level=DEBUG") && l.contains(needle) && l.contains("part=film:20")
            })
            .count()
    };
    assert_eq!(
        debug("seriko: part always の時計が生まれた"),
        2,
        "{lines:#?}"
    );
    assert_eq!(
        debug("seriko: part 回数つきの時計を捨てた"),
        1,
        "{lines:#?}"
    );
}

/// 回数つき: 続けて見えていれば（同じ絵を置く別のサーフェスへ替えても）始め直さず、止まっていれば
/// 止まったまま（要件 2.2・2.7・2.8）。
#[test]
fn finite_film_continuously_visible_keeps_going_and_stays_stopped() {
    let table = table();
    let (mut rng, _probe) = counting_rng(&[]);
    let mut clocks = PartClocks::default();

    tick(&mut clocks, &table, 3, 1000, &mut rng);
    let p = tick(&mut clocks, &table, 1, 1100, &mut rng);
    assert_eq!(picture(&p, ONCE), Some(21));
    assert_eq!(
        clock(&clocks, Slot::Shell, ONCE),
        playing(1000),
        "始め直さない"
    );
    let p = tick(&mut clocks, &table, 3, 5000, &mut rng);
    assert_eq!(picture(&p, ONCE), Some(21), "最後のコマで止まったまま");
    assert_eq!(clock(&clocks, Slot::Shell, ONCE), playing(1000));
}

/// スコープごと・面の種類ごとに別の時計。片方で捨てても他は残る（要件 3.5・6.6）。
#[test]
fn clocks_are_separate_per_scope_and_face_kind() {
    let table = table();
    let (mut rng, _probe) = counting_rng(&[]);
    let mut clocks = PartClocks::default();
    let other = ActorKey::from("1");

    tick_at(
        &mut clocks,
        &table,
        &scope(),
        Slot::Shell,
        3,
        1000,
        &mut rng,
    );
    tick_at(&mut clocks, &table, &other, Slot::Shell, 3, 1050, &mut rng);
    tick_at(
        &mut clocks,
        &table,
        &scope(),
        Slot::Balloon,
        3,
        1080,
        &mut rng,
    );
    assert_eq!(clock(&clocks, Slot::Shell, ONCE), playing(1000));
    assert_eq!(clock(&clocks, Slot::Balloon, ONCE), playing(1080));
    assert_eq!(
        clocks.clock_at(&other, Slot::Shell, PartKey::Film(ONCE), 0),
        playing(1050)
    );

    // シェルの面で見えなくなっても、バルーンの面と別のスコープの時計は残る。
    tick_at(
        &mut clocks,
        &table,
        &scope(),
        Slot::Shell,
        2,
        1100,
        &mut rng,
    );
    assert_eq!(clock(&clocks, Slot::Shell, ONCE), None);
    assert_eq!(clock(&clocks, Slot::Balloon, ONCE), playing(1080));
    assert_eq!(
        clocks.clock_at(&other, Slot::Shell, PartKey::Film(ONCE), 0),
        playing(1050)
    );
    let p = tick_at(
        &mut clocks,
        &table,
        &scope(),
        Slot::Balloon,
        3,
        1180,
        &mut rng,
    );
    assert_eq!(picture(&p, ONCE), Some(21));
}

/// 入れ物 1 つの回数つきの時計を全部捨てる口（面が隠れたとき用）。終わりなしと、ほかの入れ物は残す。
#[test]
fn drop_finite_drops_only_finite_clocks_of_that_container() {
    let table = table();
    let (mut rng, _probe) = counting_rng(&[]);
    let mut clocks = PartClocks::default();

    tick_at(
        &mut clocks,
        &table,
        &scope(),
        Slot::Shell,
        1,
        1000,
        &mut rng,
    );
    tick_at(
        &mut clocks,
        &table,
        &scope(),
        Slot::Balloon,
        1,
        1000,
        &mut rng,
    );
    assert_eq!(clock(&clocks, Slot::Shell, ONCE), playing(1000));
    assert_eq!(clock(&clocks, Slot::Shell, ENDLESS), playing(1000));

    clocks.drop_finite(&scope(), Slot::Shell, &table);
    assert_eq!(clock(&clocks, Slot::Shell, ONCE), None);
    assert_eq!(clock(&clocks, Slot::Shell, ENDLESS), playing(1000));
    assert_eq!(clock(&clocks, Slot::Balloon, ONCE), playing(1000));

    // 次に見えた刻みで頭から生まれる。
    tick_at(
        &mut clocks,
        &table,
        &scope(),
        Slot::Shell,
        1,
        1700,
        &mut rng,
    );
    assert_eq!(clock(&clocks, Slot::Shell, ONCE), playing(1700));
}

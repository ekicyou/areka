//! 面 1 つを時点 T まで進める口の檻（spec: areka-P0-offline-surface-dump 要件 4.1・4.7・4.9・12.2）。
//!
//! 表は research.md §10.1・§10.2 の SSP 2.9.23 の測りを写す。100・101・102 はコマの面、200 は
//! `always` のコマの頭。刻みの位相は書き出しの決め方（合図と同じ時刻 0 から 16ms ごと）。

use std::num::NonZeroU32;

use areka_emo_compose::{
    BindSet, Cell, ElementKind, EmoWorld, FilmId, FilmSheet, PartKey, PatternState, SurfaceMaster,
};

use super::{SETTLE_TICK_MS, settle_surface, settle_time};
use crate::table::AnimationTable;

/// 0: runonce 赤→緑→青（各 100）。1: 0 の最後に `-1`。2: always だけ（8 コマ×50）。
/// 3: runonce 3 コマ×1000＋`random,1`（T までに 1 秒の境を 3 度越える）。
/// 4: runonce 3 コマ×1000＋periodic,1（`-1` まで 10ms）。5: 0 の runonce＋2 の always。
/// 6: runonce 2 本（100 と 100＋200）。7: 部品 8 を置くだけ（8 は runonce 500 を持つ）。
/// 9: 動く絵の子（3 コマ×100・終わりなし）＋runonce 450。
const SHELL: &str = "surface0\n{\nanimation0.interval,runonce\n\
    animation0.pattern0,overlay,100,100,0,0\n\
    animation0.pattern1,overlay,101,100,0,0\n\
    animation0.pattern2,overlay,102,100,0,0\n}\n\
    surface1\n{\nanimation0.interval,runonce\n\
    animation0.pattern0,overlay,100,100,0,0\n\
    animation0.pattern1,overlay,101,100,0,0\n\
    animation0.pattern2,overlay,102,100,0,0\n\
    animation0.pattern3,overlay,-1,100,0,0\n}\n\
    surface2\n{\nanimation1.interval,always\n\
    animation1.pattern0,overlay,200,50,0,0\nanimation1.pattern1,overlay,201,50,0,0\n\
    animation1.pattern2,overlay,202,50,0,0\nanimation1.pattern3,overlay,203,50,0,0\n\
    animation1.pattern4,overlay,204,50,0,0\nanimation1.pattern5,overlay,205,50,0,0\n\
    animation1.pattern6,overlay,206,50,0,0\nanimation1.pattern7,overlay,207,50,0,0\n}\n\
    surface3\n{\nanimation0.interval,runonce\n\
    animation0.pattern0,overlay,100,1000,0,0\n\
    animation0.pattern1,overlay,101,1000,0,0\n\
    animation0.pattern2,overlay,102,1000,0,0\n\
    animation1.interval,random,1\n\
    animation1.pattern0,overlay,200,0,0,0\n}\n\
    surface4\n{\nanimation0.interval,runonce\n\
    animation0.pattern0,overlay,100,1000,0,0\n\
    animation0.pattern1,overlay,101,1000,0,0\n\
    animation0.pattern2,overlay,102,1000,0,0\n\
    animation1.interval,periodic,1\n\
    animation1.pattern0,overlay,101,0,0,0\n\
    animation1.pattern1,overlay,-1,10,0,0\n}\n\
    surface5\n{\nanimation0.interval,runonce\n\
    animation0.pattern0,overlay,100,100,0,0\n\
    animation0.pattern1,overlay,101,100,0,0\n\
    animation0.pattern2,overlay,102,100,0,0\n\
    animation1.interval,always\n\
    animation1.pattern0,overlay,200,50,0,0\nanimation1.pattern1,overlay,201,50,0,0\n\
    animation1.pattern2,overlay,202,50,0,0\nanimation1.pattern3,overlay,203,50,0,0\n\
    animation1.pattern4,overlay,204,50,0,0\nanimation1.pattern5,overlay,205,50,0,0\n\
    animation1.pattern6,overlay,206,50,0,0\nanimation1.pattern7,overlay,207,50,0,0\n}\n\
    surface6\n{\nanimation0.interval,runonce\n\
    animation0.pattern0,overlay,100,100,0,0\n\
    animation1.interval,runonce\n\
    animation1.pattern0,overlay,101,100,0,0\n\
    animation1.pattern1,overlay,102,200,0,0\n}\n\
    surface7\n{\nelement0,overlay,8,0,0\n}\n\
    surface8\n{\nanimation0.interval,runonce\n\
    animation0.pattern0,overlay,100,500,0,0\n}\n\
    surface9\n{\nelement0,overlay,a.png,0,0\n\
    animation0.interval,runonce\n\
    animation0.pattern0,overlay,100,450,0,0\n}\n\
    surface100\n{\n}\nsurface101\n{\n}\nsurface102\n{\n}\n";

/// 動く絵の子（コマの絵 10・11・12、各 100ms・周期 300）。
const FILM: FilmId = FilmId(10);

/// [`SHELL`] の表。`a.png` は分解と同じに子を置く element へ替える（`parts_film_tests.rs` と同じ組み方）。
fn table() -> AnimationTable {
    let mut world = EmoWorld::build(&areka_parsers::shell::parse(SHELL));
    let w = world.world_mut();
    let mut q = w.query::<&mut SurfaceMaster>();
    for mut master in q.iter_mut(w) {
        for e in &mut master.elements {
            if e.path.as_str() == "a.png" {
                e.kind = ElementKind::Film(FILM);
            }
        }
    }
    let sheet = FilmSheet {
        id: FILM,
        path: "a.png".to_string(),
        frames: vec![10, 11, 12],
        delays_ms: vec![100, 100, 100],
        laps: None::<NonZeroU32>,
        original: (2, 2),
        rest: 0,
    };
    AnimationTable::from_world_and_films(&world, [sheet].iter(), &[])
}

fn settle(table: &AnimationTable, surface_id: u32) -> PatternState {
    settle_surface(table, 0, &BindSet::default(), surface_id)
}

/// 一番上の animation `anim` の今のコマの面（出ていなければ `None`）。
fn top(p: &PatternState, anim: u32) -> Option<u32> {
    p.get(anim).map(|f| f.surface_id)
}

#[test]
fn tick_interval_is_the_production_16ms() {
    assert_eq!(SETTLE_TICK_MS, 16);
}

/// T は runonce の待ちの合計の最大。無ければ 0。部品の runonce は T を決めない。
#[test]
fn settle_time_is_the_longest_runonce() {
    let t = table();
    for (surface, want) in [
        (0, 300),
        (1, 400),
        (2, 0),
        (3, 3000),
        (4, 3000),
        (5, 300),
        (6, 300),
        (7, 0),
        (8, 500),
        (9, 450),
        (100, 0),
        (999, 0),
    ] {
        assert_eq!(settle_time(&t, surface), want, "面 {surface}");
    }
}

/// 面 0: runonce の最後のコマが残る（SSP は左が青）。
#[test]
fn runonce_keeps_its_last_frame() {
    assert_eq!(top(&settle(&table(), 0), 0), Some(102));
}

/// 面 1: 最後が `-1` なら土台に戻る（SSP は灰だけ）。
#[test]
fn runonce_ending_in_minus_one_returns_to_the_base() {
    assert!(settle(&table(), 1).is_empty());
}

/// 面 2: always だけは T=0 で何も出ない（コマ 0 は待ち 50 の後）。
#[test]
fn always_alone_shows_nothing_at_zero() {
    assert!(settle(&table(), 2).is_empty());
}

/// 面 3: 1 秒の境を越えても `random,1` は抽選が当たらないので出ない（SSP 2.9.23 と同じ）。
#[test]
fn random_never_starts() {
    let p = settle(&table(), 3);
    assert_eq!(top(&p, 0), Some(102));
    assert_eq!(top(&p, 1), None);
}

/// 面 4: periodic は T まで進んで出る。書き出しの位相 0 では時刻 3000 の刻みが 3 周目の頭に当たる
/// （画面は刻みの格子がずれうるので、この行は書き出しの決め方を固定する）。
#[test]
fn periodic_fires_with_phase_zero() {
    let p = settle(&table(), 4);
    assert_eq!(top(&p, 0), Some(102));
    assert_eq!(top(&p, 1), Some(101));
}

/// 面 5: runonce＋always は T=300 の時点の always のコマ 5（SSP は `(160,200,50)`＝コマ 5）。
#[test]
fn always_beside_runonce_shows_the_frame_at_t() {
    let p = settle(&table(), 5);
    assert_eq!(top(&p, 0), Some(102));
    assert_eq!(top(&p, 1), Some(205));
}

/// 面 6: 2 本の runonce は長いほうまで進む（どちらも最後のコマが残る）。
#[test]
fn two_runonces_settle_at_the_longer() {
    let p = settle(&table(), 6);
    assert_eq!(top(&p, 0), Some(100));
    assert_eq!(top(&p, 1), Some(102));
}

/// 面 9: 動く絵の子は T=450 の時点のコマで繰り返す（1 周 300 を越えて 2 周目の 2 枚目＝絵 11）。
#[test]
fn film_child_loops_to_the_frame_at_t() {
    let p = settle(&table(), 9);
    assert_eq!(p.cell(Some(PartKey::Film(FILM)), 0), Cell::Picture(11));
    assert_eq!(top(&p, 0), Some(100));
}

/// 2 度呼んで同じ状態（決定論・4.9）。
#[test]
fn settling_twice_gives_the_same_state() {
    let t = table();
    for surface in [0, 4, 5, 9] {
        assert_eq!(settle(&t, surface), settle(&t, surface), "面 {surface}");
    }
}

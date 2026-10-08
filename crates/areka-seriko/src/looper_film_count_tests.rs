//! 合成し直す回数（＝刻みが出す指令の数）を数える檻（spec: areka-P0-animated-image-playback
//! 要件 7.2・7.4・9.2・9.3・design「テスト」5・tasks.md 6.1）。
//!
//! 動く絵 1 つ（待ち 100ms のコマ 3 枚・終わりなし）だけの表で、16ms の刻みを与える。時刻は
//! `on_tick(now_ms)`、乱数は注入列だけで決定論（実際の時計・OS に依らない）。動く絵も `always` も
//! 無い表（`emo2` を含む）の列は `looper_parts_emo2_tests.rs` と `areka` の
//! `spine_seriko_loop_tests.rs` が書き換えずに固定している。

use std::num::NonZeroU32;

use areka_emo_compose::{BindSet, ElementKind, EmoWorld, FilmId, FilmSheet, SurfaceMaster};

use super::tests::{cfg, counting_rng};
use super::*;
use crate::resolve::SurfaceTarget;

const FILM: FilmId = FilmId(10);
/// 1 コマの待ち時間。
const DELAY_MS: u64 = 100;
/// 画面の更新の刻み。
const TICK_MS: u64 = 16;

/// 面 0 に動く絵 `a.png` を 1 つ置いた表（分解と同じ形を手で組む＝`parts_film_tests.rs` と同じ組み方）。
fn one_film_table() -> AnimationTable {
    let mut world = EmoWorld::build(&areka_parsers::shell::parse(
        "surface0\n{\nelement0,overlay,a.png,0,0\n}\n",
    ));
    let w = world.world_mut();
    let mut q = w.query::<&mut SurfaceMaster>();
    for mut master in q.iter_mut(w) {
        for e in &mut master.elements {
            e.kind = ElementKind::Film(FILM);
        }
    }
    let sheet = FilmSheet {
        id: FILM,
        path: "a.png".into(),
        frames: vec![10, 11, 12],
        delays_ms: vec![DELAY_MS as u32; 3],
        laps: None::<NonZeroU32>,
        original: (2, 2),
        rest: 0,
    };
    AnimationTable::from_world_and_films(&world, [sheet].iter(), &[])
}

/// 刻み 0 の後に面 0 を出す（アクターと同じ順）。出来事の時刻は直前の刻み＝0。
fn shown() -> (LoopRuntime, ScopeStates) {
    let (rng, _probe) = counting_rng(&[]);
    let mut rt = LoopRuntime::new(cfg(one_film_table(), rng));
    let mut states = ScopeStates::new(BindSet::default());
    let scope = ActorKey::from("0");
    assert!(rt.on_tick(0, &mut states).is_empty());
    let crate::state::ApplyOutcome::Changed(_) = states.apply(&scope, SurfaceTarget::Show(0))
    else {
        panic!("面 0 の表示の指令が出ない");
    };
    rt.on_surface_changed(&scope, Slot::Shell);
    rt.refresh(&scope, Slot::Shell, None, &mut states);
    (rt, states)
}

/// 面の表示の指令の直後の最初の刻みは 0 件（経過 0 の絵は表示の指令で出ている）。待ち 100ms の
/// コマに 16ms の刻みを 7 回与えると、コマが替わる 7 回目（112ms）の 1 件だけ（要件 7.4）。
#[test]
fn hundred_ms_frame_over_seven_16ms_ticks_emits_exactly_one_command() {
    let (mut rt, mut states) = shown();
    let counts: Vec<usize> = (1..=7)
        .map(|i| rt.on_tick(i * TICK_MS, &mut states).len())
        .collect();
    assert_eq!(counts, [0, 0, 0, 0, 0, 0, 1]);
}

/// 長く回しても、指令はコマが替わった刻みにだけ出る（3 秒・終わりなしの 3 枚の周回）。
#[test]
fn commands_are_emitted_only_on_ticks_where_the_frame_changes() {
    let (mut rt, mut states) = shown();
    let mut prev = 0;
    let mut now = TICK_MS;
    while now <= 3000 {
        let changed = now / DELAY_MS != prev / DELAY_MS;
        assert_eq!(
            rt.on_tick(now, &mut states).len(),
            usize::from(changed),
            "刻み {now}ms"
        );
        prev = now;
        now += TICK_MS;
    }
}

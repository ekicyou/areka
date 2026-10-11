//! 部品の `always` の時計の檻（spec: areka-P0-animated-image-playback 要件 3.1・4.4・4.5・4.7・7.4・
//! tasks.md 3.3）。
//!
//! 表は `surfaces.txt` の本文から解析 → 畳み込み → [`AnimationTable::from_world`] の実経路で組む。
//! 時刻は `advance`／`refresh` の時刻、乱数は呼ばれた回数を数える注入列で、どちらも決定論。

use areka_emo_compose::{Cell, EmoWorld, PartKey, PatternState};
use areka_sakura::ActorKey;
use log_capture_kit::{LineFormat, capture_lines};

use super::{PartAnim, PartClocks};
use crate::looper::tests::counting_rng;
use crate::state::Slot;
use crate::table::AnimationTable;
use crate::timeline::LoopRng;

fn table_of(text: &str) -> AnimationTable {
    AnimationTable::from_world(&EmoWorld::build(&areka_parsers::shell::parse(text)))
}

fn scope() -> ActorKey {
    ActorKey::from("0")
}

/// 一番上 `top` のシェルの面で刻みを 1 回回し、部品の欄を作り直した絵を返す。
fn tick(
    clocks: &mut PartClocks,
    table: &AnimationTable,
    top: u32,
    now_ms: u64,
    crossed: bool,
    rng: &mut LoopRng,
) -> PatternState {
    let mut p = PatternState::default();
    clocks.advance(
        &scope(),
        Slot::Shell,
        top,
        &areka_emo_compose::BindSet::default(),
        table,
        now_ms,
        crossed,
        true,
        None,
        rng,
        &mut p,
    );
    p
}

/// 部品 `part` の animation 0 の欄の読み（サーフェスを指すコマは番号）。
fn cell_of(p: &PatternState, part: u32) -> Option<Option<u32>> {
    match p.cell(Some(PartKey::Surface(part)), 0) {
        Cell::Rest => None,
        Cell::Frame(f) => Some(Some(f.surface_id)),
        Cell::Blank => Some(None),
        Cell::Picture(_) => unreachable!("作者のサーフェスの部品は絵を指さない"),
    }
}

fn playing(at: u64) -> Option<PartAnim> {
    Some(PartAnim::Playing { started_at_ms: at })
}

/// 一番上 0 と 1 が子 100 を置き、2 は何も置かない。100 の `always` は 101（待ち 0＝経過 0）→
/// 100ms で `-1`（消える）→ 100ms で 102 → 100ms で 103（周期 300・103 は 0 ミリ秒しか出ない）。
const ALWAYS: &str = "surface0\n{\nelement0,overlay,100,0,0\n}\n\
    surface1\n{\nelement0,overlay,100,5,5\n}\n\
    surface2\n{\n}\n\
    surface100\n{\nanimation0.interval,always\n\
    animation0.pattern0,overlay,101,0,0,0\n\
    animation0.pattern1,overlay,-1,100,0,0\n\
    animation0.pattern2,overlay,102,100,0,0\n\
    animation0.pattern3,overlay,103,100,0,0\n}\n\
    surface101\n{\n}\nsurface102\n{\n}\nsurface103\n{\n}\n";

/// 見えた刻みの時刻で乱数を引かずに時計が生まれる。経過 0 と同じコマは欄に載らず、`-1` の間は
/// 「消えている」、周の頭へ戻ると再び載らない（要件 3.1・4.1・4.5・7.4）。
#[test]
fn always_part_is_born_without_rng_and_rest_frame_is_not_written() {
    let table = table_of(ALWAYS);
    let (mut rng, probe) = counting_rng(&[]);
    let mut clocks = PartClocks::default();

    // 見えない間は時計が無い。
    let p = tick(&mut clocks, &table, 2, 500, true, &mut rng);
    assert!(p.is_empty());
    assert_eq!(clocks.clock(&scope(), 100, 0), None);

    let p = tick(&mut clocks, &table, 0, 1000, true, &mut rng);
    assert_eq!(
        clocks.clock(&scope(), 100, 0),
        playing(1000),
        "見えた時刻で生まれる"
    );
    assert_eq!(cell_of(&p, 100), None, "経過 0 と同じコマは載せない");
    assert!(p.is_empty());

    let p = tick(&mut clocks, &table, 0, 1100, false, &mut rng);
    assert_eq!(cell_of(&p, 100), Some(None), "`-1` の間は消えている");
    let p = tick(&mut clocks, &table, 0, 1250, true, &mut rng);
    assert_eq!(cell_of(&p, 100), Some(Some(102)));
    let p = tick(&mut clocks, &table, 0, 1300, false, &mut rng);
    assert_eq!(cell_of(&p, 100), None, "周の頭は経過 0 と同じ");
    assert!(p.is_empty());

    assert_eq!(probe.lock().unwrap().calls, 0, "always は乱数を引かない");
}

/// 親が替わっても（同じ子を置く別のサーフェス・置かないサーフェスを挟んで戻る）子の `always` は
/// 巻き戻らない（要件 4.4）。
#[test]
fn always_in_child_does_not_rewind_when_parent_switches() {
    let table = table_of(ALWAYS);
    let (mut rng, _probe) = counting_rng(&[]);
    let mut clocks = PartClocks::default();

    tick(&mut clocks, &table, 0, 1000, true, &mut rng);
    let p = tick(&mut clocks, &table, 1, 1210, false, &mut rng);
    assert_eq!(cell_of(&p, 100), Some(Some(102)), "別の親でも続き");
    assert_eq!(clocks.clock(&scope(), 100, 0), playing(1000));

    let p = tick(&mut clocks, &table, 2, 1220, false, &mut rng);
    assert!(p.is_empty());
    assert_eq!(
        clocks.clock(&scope(), 100, 0),
        playing(1000),
        "終わりなしは捨てない"
    );
    let p = tick(&mut clocks, &table, 0, 1350, false, &mut rng);
    assert_eq!(
        cell_of(&p, 100),
        None,
        "見えなかった間も進んでいる（1350 は 2 周目の頭）"
    );
    let p = tick(&mut clocks, &table, 0, 1410, false, &mut rng);
    assert_eq!(cell_of(&p, 100), Some(None));
    assert_eq!(clocks.clock(&scope(), 100, 0), playing(1000));
}

/// 出来事の直後の `refresh` は、見えている `always` の時計が無ければ出来事の時刻で作り（経過 0＝欄に
/// 載せない）、在れば今のコマを書く。時刻が分からない（`None`）ときは作らない（tasks.md 3.4）。
#[test]
fn refresh_creates_always_clock_at_event_time() {
    let table = table_of(ALWAYS);
    let (mut rng, probe) = counting_rng(&[]);
    let mut clocks = PartClocks::default();
    let b = areka_emo_compose::BindSet::default();

    let mut p = PatternState::default();
    clocks.refresh(&scope(), Slot::Shell, 0, &b, &table, None, true, &mut p);
    assert!(p.is_empty());
    assert_eq!(
        clocks.clock(&scope(), 100, 0),
        None,
        "時刻が分からなければ作らない"
    );

    let mut p = PatternState::default();
    clocks.refresh(
        &scope(),
        Slot::Shell,
        0,
        &b,
        &table,
        Some(1000),
        true,
        &mut p,
    );
    assert!(p.is_empty(), "経過 0 のコマは欄に載らない");
    assert_eq!(clocks.clock(&scope(), 100, 0), playing(1000));

    let mut p = PatternState::default();
    clocks.refresh(
        &scope(),
        Slot::Shell,
        1,
        &b,
        &table,
        Some(1200),
        true,
        &mut p,
    );
    assert_eq!(cell_of(&p, 100), Some(Some(102)), "在る時計は続き");
    assert_eq!(clocks.clock(&scope(), 100, 0), playing(1000));
    tick(&mut clocks, &table, 0, 1250, true, &mut rng);
    assert_eq!(probe.lock().unwrap().calls, 0, "乱数を引かない");
}

/// 同じ部品の `random` の抽選は、`always` が在っても同じ回数だけ乱数を引く（要件 4.7）。
#[test]
fn always_beside_random_does_not_change_the_draws() {
    let with = "surface0\n{\nelement0,overlay,100,0,0\n}\n\
        surface100\n{\nanimation0.interval,always\n\
        animation0.pattern0,overlay,101,0,0,0\n\
        animation0.pattern1,overlay,102,100,0,0\n\
        animation1.interval,random,2\n\
        animation1.pattern0,overlay,103,50,0,0\n}\n\
        surface101\n{\n}\nsurface102\n{\n}\nsurface103\n{\n}\n";
    let without = with.replace("animation0.interval,always", "animation0.interval,runonce");
    let draws = |text: &str| {
        let table = table_of(text);
        let (mut rng, probe) = counting_rng(&[1, 1, 1]);
        let mut clocks = PartClocks::default();
        for (i, at) in [1000u64, 1100, 1200].into_iter().enumerate() {
            tick(&mut clocks, &table, 0, at, i != 1, &mut rng);
        }
        probe.lock().unwrap().calls
    };
    assert_eq!(draws(with), 2);
    assert_eq!(draws(&without), 2);
}

/// 同じ刻みの後の回で見えなくなった部品を欄から外すときも、見えている部品の「消えている」は残る。
/// 100 が消える刻みに、200 のコマが経過 0 の先 300 を置き換えて 300 が見えなくなる。
#[test]
fn blank_survives_when_a_part_hidden_in_the_same_tick_is_dropped() {
    let text = "surface0\n{\nelement0,overlay,100,0,0\nelement1,overlay,200,0,0\n}\n\
        surface100\n{\nanimation0.interval,always\n\
        animation0.pattern0,overlay,101,0,0,0\n\
        animation0.pattern1,overlay,-1,100,0,0\n\
        animation0.pattern2,overlay,101,100,0,0\n}\n\
        surface200\n{\nanimation0.interval,always\n\
        animation0.pattern0,overlay,300,0,0,0\n\
        animation0.pattern1,overlay,201,100,0,0\n\
        animation0.pattern2,overlay,300,100,0,0\n}\n\
        surface101\n{\n}\nsurface201\n{\n}\nsurface300\n{\n}\n";
    let table = table_of(text);
    let (mut rng, _probe) = counting_rng(&[]);
    let mut clocks = PartClocks::default();

    tick(&mut clocks, &table, 0, 1000, true, &mut rng);
    let p = tick(&mut clocks, &table, 0, 1100, false, &mut rng);
    assert_eq!(cell_of(&p, 100), Some(None), "消えているが残る");
    assert_eq!(cell_of(&p, 200), Some(Some(201)));
}

/// 時計が生まれたときに `debug!` を 1 回出す（刻みごとには出さない）。
#[test]
fn always_clock_birth_is_debug_logged_once() {
    let table = table_of(ALWAYS);
    let (mut rng, _probe) = counting_rng(&[]);
    let mut clocks = PartClocks::default();
    let (_, lines) = capture_lines(LineFormat::LevelTargetFields, || {
        for at in [1000u64, 1100, 1200] {
            tick(&mut clocks, &table, 0, at, false, &mut rng);
        }
    });
    let born: Vec<&String> = lines
        .iter()
        .filter(|l| l.contains("level=DEBUG") && l.contains("seriko: part always の時計が生まれた"))
        .collect();
    assert_eq!(born.len(), 1, "{lines:#?}");
    assert!(born[0].contains("part=100"), "{born:?}");
}

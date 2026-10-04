//! 部品の時計の檻（spec: areka-P0-surface-element-nesting 要件 5.1〜5.5・5.7・5.11〜5.14・8.3・
//! tasks.md 7.2）。
//!
//! 表は `surfaces.txt` の本文から解析 → 畳み込み → [`AnimationTable::from_world`] の実経路で組む。
//! 刻みは `advance` の `now_ms`／`crossed`、乱数は呼ばれた回数を数える注入列で、どちらも決定論。
//! 一番上の進行は looper の仕事なので、ここでは一番上の欄のコマを手で置いて「一番上の進行を
//! 済ませた後の絵」を作る。

use std::sync::{Arc, Mutex};

use areka_emo_compose::{BindSet, ComposeMethod, EmoWorld, PatternFrame, PatternState};
use areka_sakura::ActorKey;

use super::{PartAnim, PartClocks};
use crate::looper::tests::{RngProbe, counting_rng};
use crate::table::AnimationTable;
use crate::timeline::LoopRng;

fn table_of(text: &str) -> AnimationTable {
    AnimationTable::from_world(&EmoWorld::build(&areka_parsers::shell::parse(text)))
}

fn scope() -> ActorKey {
    ActorKey::from("0")
}

fn no_binds() -> BindSet {
    BindSet::from_ids([])
}

fn calls(probe: &Arc<Mutex<RngProbe>>) -> usize {
    probe.lock().unwrap().calls
}

/// overlay・位置 0,0 のコマ（一番上の欄に手で置く用）。
fn overlay(surface_id: u32) -> PatternFrame {
    PatternFrame {
        surface_id,
        method: ComposeMethod::Overlay,
        x: 0,
        y: 0,
    }
}

/// 部品 `part` の欄を (animation の番号, コマの番号) の列で読む。
fn part_frames(pattern: &PatternState, part: u32) -> Vec<(u32, u32)> {
    pattern
        .part(part)
        .map(|(id, f)| (id, f.surface_id))
        .collect()
}

/// 部品の欄に載っている部品の番号（昇順）。
fn parts_in_field(pattern: &PatternState) -> Vec<u32> {
    (0..1000)
        .filter(|&p| pattern.part(p).next().is_some())
        .collect()
}

/// 刻み 1 回を回す（一番上 `top`・一番上の欄 `top_pattern` から始めて、部品の欄を作り直した結果を返す）。
#[allow(clippy::too_many_arguments)]
fn tick(
    clocks: &mut PartClocks,
    table: &AnimationTable,
    top: u32,
    binds: &BindSet,
    now_ms: u64,
    crossed: bool,
    rng: &mut LoopRng,
    top_pattern: &PatternState,
) -> PatternState {
    let mut pattern = top_pattern.clone();
    clocks.advance(
        &scope(),
        top,
        binds,
        table,
        now_ms,
        crossed,
        rng,
        &mut pattern,
    );
    pattern
}

/// 一番上 0 が子 100 を置き、一番上 1 は何も置かない。100 のまばたきは
/// 50ms 待って 101 → さらに 50ms で `-1`（停止）。
const BLINK: &str = "surface0\n{\nelement0,overlay,100,10,20\n}\n\
    surface1\n{\n}\n\
    surface100\n{\nanimation0.interval,random,2\n\
    animation0.pattern0,overlay,101,50,0,0\n\
    animation0.pattern1,overlay,-1,50,0,0\n}\n\
    surface101\n{\n}\n";

/// 見える刻みの境界でだけ抽選し、当たれば経過から 101 → 停止まで進む（要件 5.1・5.5）。
/// 境界を跨がない刻みは乱数を呼ばない。初めて見えた刻みの後の境界から抽選の対象になる。
#[test]
fn lottery_only_at_visible_boundary_ticks_then_progress() {
    let table = table_of(BLINK);
    let (mut rng, probe) = counting_rng(&[0]);
    let mut clocks = PartClocks::default();
    let top = PatternState::default();
    let b = no_binds();

    // 初めて見えた刻み（境界を跨がない）: 抽選しない・時計なし・欄は空。
    let p = tick(&mut clocks, &table, 0, &b, 1500, false, &mut rng, &top);
    assert_eq!(calls(&probe), 0, "境界を跨がない刻みは乱数を呼ばない");
    assert!(p.is_empty());
    assert_eq!(clocks.clock(&scope(), 100, 0), None);

    // 次の境界: 1 回だけ抽選して当たる。経過 0 は先頭の待ちの前なのでコマ無し。
    let p = tick(&mut clocks, &table, 0, &b, 2000, true, &mut rng, &top);
    assert_eq!(calls(&probe), 1);
    assert_eq!(
        clocks.clock(&scope(), 100, 0),
        Some(PartAnim::Playing {
            started_at_ms: 2000
        })
    );
    assert!(part_frames(&p, 100).is_empty(), "Pending はコマ無し");

    // 経過 60: 101。境界を跨がないので乱数は増えない。
    let p = tick(&mut clocks, &table, 0, &b, 2060, false, &mut rng, &top);
    assert_eq!(part_frames(&p, 100), vec![(0, 101)]);
    assert!(
        p.iter().next().is_none(),
        "一番上の欄には書かない（要件 5.10）"
    );

    // 経過 100: `-1` で停止 → 時計を消し、欄にも載らない。
    let p = tick(&mut clocks, &table, 0, &b, 2100, false, &mut rng, &top);
    assert!(p.is_empty());
    assert_eq!(clocks.clock(&scope(), 100, 0), None);
    assert_eq!(calls(&probe), 1, "抽選は境界の 1 回だけ");
}

/// 見えない部品には触らない: 境界を何度跨いでも乱数を呼ばず、欄にも書かない（要件 5.11）。
/// 渡された部品の欄は先に空にされる。
#[test]
fn invisible_part_draws_no_rng_and_writes_nothing() {
    let table = table_of(BLINK);
    let (mut rng, probe) = counting_rng(&[0, 0, 0]);
    let mut clocks = PartClocks::default();
    let mut top = PatternState::default();
    // 前の刻みの部品の欄が残った写し（作り直しで消えること）。
    top.set_part(100, 0, overlay(101));

    for (i, now) in [1000u64, 2000, 3000].into_iter().enumerate() {
        let p = tick(
            &mut clocks,
            &table,
            1,
            &no_binds(),
            now,
            true,
            &mut rng,
            &top,
        );
        assert!(p.is_empty(), "刻み {i}: 見えない部品の欄は空");
    }
    assert_eq!(calls(&probe), 0, "見えない部品のために乱数を呼ばない");
    assert_eq!(clocks.clock(&scope(), 100, 0), None);
}

/// 再生中に見えなくなり、戻った刻みのコマは経過から決まる（巻き戻らない・要件 5.7）。
/// 見えない間は時計に触らない（開始の時刻を持ち続ける）。
#[test]
fn playing_part_resumes_from_elapsed_after_being_hidden() {
    let table = table_of(
        "surface0\n{\nelement0,overlay,100,0,0\n}\nsurface1\n{\n}\n\
         surface100\n{\nanimation0.interval,random,2\n\
         animation0.pattern0,overlay,101,100,0,0\n\
         animation0.pattern1,overlay,102,100,0,0\n\
         animation0.pattern2,overlay,-1,100,0,0\n}\n",
    );
    let (mut rng, probe) = counting_rng(&[0]);
    let mut clocks = PartClocks::default();
    let top = PatternState::default();
    let b = no_binds();

    tick(&mut clocks, &table, 0, &b, 1000, true, &mut rng, &top);
    let p = tick(&mut clocks, &table, 0, &b, 1150, false, &mut rng, &top);
    assert_eq!(part_frames(&p, 100), vec![(0, 101)]);

    // 子を置いていない面へ: 触らない（時計はそのまま）。
    let p = tick(&mut clocks, &table, 1, &b, 1200, false, &mut rng, &top);
    assert!(p.is_empty());
    assert_eq!(
        clocks.clock(&scope(), 100, 0),
        Some(PartAnim::Playing {
            started_at_ms: 1000
        })
    );

    // 戻った刻み: 経過 250 → 102（101 からやり直さない）。
    let p = tick(&mut clocks, &table, 0, &b, 1250, false, &mut rng, &top);
    assert_eq!(part_frames(&p, 100), vec![(0, 102)]);
    assert_eq!(calls(&probe), 1);
}

/// 見えない間に境界を跨いでも、戻った刻みが境界を跨がなければ抽選しない。
/// `-1` の無い末尾は最後のコマを保ち、保っている間は次の境界で再び抽選の対象になる（要件 5.1・5.5）。
#[test]
fn residual_keeps_last_frame_and_is_drawn_again_at_next_visible_boundary() {
    let table = table_of(
        "surface0\n{\nelement0,overlay,100,0,0\n}\nsurface1\n{\n}\n\
         surface100\n{\nanimation0.interval,random,2\n\
         animation0.pattern0,overlay,101,0,0,0\n}\n",
    );
    let (mut rng, probe) = counting_rng(&[0, 1]);
    let mut clocks = PartClocks::default();
    let top = PatternState::default();
    let b = no_binds();

    // 見えない境界: 抽選なし。
    tick(&mut clocks, &table, 1, &b, 1000, true, &mut rng, &top);
    // 見えた刻み（境界でない）: 抽選なし。
    tick(&mut clocks, &table, 0, &b, 1040, false, &mut rng, &top);
    assert_eq!(calls(&probe), 0);

    // 境界: 当たり、待ち 0 の末尾 → 101 を保つ。
    let p = tick(&mut clocks, &table, 0, &b, 2000, true, &mut rng, &top);
    assert_eq!(part_frames(&p, 100), vec![(0, 101)]);
    assert_eq!(
        clocks.clock(&scope(), 100, 0),
        Some(PartAnim::Residual { frame_index: 0 })
    );
    let p = tick(&mut clocks, &table, 0, &b, 2500, false, &mut rng, &top);
    assert_eq!(part_frames(&p, 100), vec![(0, 101)], "保ったコマが出続ける");

    // 次の境界: 保っている間は抽選の対象（外れても保ったまま）。
    let p = tick(&mut clocks, &table, 0, &b, 3000, true, &mut rng, &top);
    assert_eq!(calls(&probe), 2);
    assert_eq!(part_frames(&p, 100), vec![(0, 101)]);
}

/// 子の中の `bind+random` は着せ替えが有効なときだけ抽選し、外れたら時計を消して止まる。
/// そのコマは有効な間だけ欄に載る（要件 5.12）。
#[test]
fn bind_random_in_child_fires_only_while_active_and_stops_when_removed() {
    let table = table_of(
        "surface0\n{\nelement0,overlay,100,0,0\n}\n\
         surface100\n{\nanimation5.interval,bind+random,2\n\
         animation5.pattern0,overlay,101,0,0,0\n\
         animation5.pattern1,overlay,102,100,0,0\n}\n",
    );
    let (mut rng, probe) = counting_rng(&[0, 0]);
    let mut clocks = PartClocks::default();
    let top = PatternState::default();
    let off = no_binds();
    let on = BindSet::from_ids([5]);

    // 無効: 境界でも乱数を呼ばない。
    let p = tick(&mut clocks, &table, 0, &off, 1000, true, &mut rng, &top);
    assert_eq!(calls(&probe), 0);
    assert!(p.is_empty());

    // 有効: 抽選して当たり、待ち 0 の先頭 101 が出る。
    let p = tick(&mut clocks, &table, 0, &on, 2000, true, &mut rng, &top);
    assert_eq!(calls(&probe), 1);
    assert_eq!(part_frames(&p, 100), vec![(5, 101)]);
    let p = tick(&mut clocks, &table, 0, &on, 2100, false, &mut rng, &top);
    assert_eq!(part_frames(&p, 100), vec![(5, 102)], "末尾を保つ");

    // 外す: 時計を消し、欄に載らない。境界でも抽選しない。
    let p = tick(&mut clocks, &table, 0, &off, 3000, true, &mut rng, &top);
    assert!(p.is_empty());
    assert_eq!(clocks.clock(&scope(), 100, 5), None);
    assert_eq!(calls(&probe), 1);

    // 戻しても、境界までは時計が無いのでコマも無い（外した時点で止まっている）。
    let p = tick(&mut clocks, &table, 0, &on, 3050, false, &mut rng, &top);
    assert_eq!(part_frames(&p, 100), Vec::<(u32, u32)>::new());
}

/// 部品のコマが別のサーフェスを指すと、その先も同じ刻みで部品として評価される（要件 5.2・5.13）。
/// 抽選の消費順は 繰り返しの回 → 部品の番号の昇順。
#[test]
fn frame_pointing_to_another_surface_is_evaluated_in_the_same_tick() {
    let table = table_of(
        "surface0\n{\nelement0,overlay,100,0,0\n}\n\
         surface100\n{\nanimation0.interval,random,2\n\
         animation0.pattern0,overlay,200,0,0,0\n\
         animation0.pattern1,overlay,-1,500,0,0\n}\n\
         surface200\n{\nanimation0.interval,random,2\n\
         animation0.pattern0,overlay,201,0,0,0\n\
         animation0.pattern1,overlay,-1,100,0,0\n}\n",
    );
    let (mut rng, probe) = counting_rng(&[0, 0]);
    let mut clocks = PartClocks::default();

    let p = tick(
        &mut clocks,
        &table,
        0,
        &no_binds(),
        1000,
        true,
        &mut rng,
        &PatternState::default(),
    );
    assert_eq!(calls(&probe), 2, "100 の後に、そのコマが指した 200 を引く");
    assert_eq!(part_frames(&p, 100), vec![(0, 200)]);
    assert_eq!(part_frames(&p, 200), vec![(0, 201)]);
    assert_eq!(
        clocks.clock(&scope(), 200, 0),
        Some(PartAnim::Playing {
            started_at_ms: 1000
        })
    );
}

/// element定義で置いた子と、一番上のコマ（pattern定義）が指した同じ番号は 1 つの時計を使う
/// （要件 5.13・5.14）。コマが消えても時計は捨てない。
#[test]
fn element_child_and_pattern_target_share_one_clock() {
    let table = table_of(BLINK);
    let (mut rng, probe) = counting_rng(&[0]);
    let mut clocks = PartClocks::default();
    let b = no_binds();

    // 一番上 0 で子 100 として発火。
    tick(
        &mut clocks,
        &table,
        0,
        &b,
        1000,
        true,
        &mut rng,
        &PatternState::default(),
    );
    assert_eq!(calls(&probe), 1);

    // 一番上 1 のコマが 100 を指す: 同じ時計の続き（経過 60 → 101）。抽選しない。
    let mut top1 = PatternState::default();
    top1.set(7, overlay(100));
    let p = tick(&mut clocks, &table, 1, &b, 1060, false, &mut rng, &top1);
    assert_eq!(part_frames(&p, 100), vec![(0, 101)]);
    assert_eq!(calls(&probe), 1);

    // コマが消えても時計は残る。
    tick(
        &mut clocks,
        &table,
        1,
        &b,
        1070,
        false,
        &mut rng,
        &PatternState::default(),
    );
    assert_eq!(
        clocks.clock(&scope(), 100, 0),
        Some(PartAnim::Playing {
            started_at_ms: 1000
        })
    );
}

/// 同じ子を 1 つの親の何か所にも、内側の子からも置いても、抽選は 1 回・欄は 1 つ（要件 5.2・5.4）。
#[test]
fn same_child_in_several_places_has_one_clock_and_one_field() {
    let table = table_of(
        "surface0\n{\nelement0,overlay,100,0,0\nelement1,overlay,100,50,0\n\
         element2,overlay,300,0,80\n}\n\
         surface300\n{\nelement0,overlay,100,5,5\n}\n\
         surface100\n{\nanimation0.interval,random,2\n\
         animation0.pattern0,overlay,101,0,0,0\n\
         animation0.pattern1,overlay,-1,100,0,0\n}\n",
    );
    let (mut rng, probe) = counting_rng(&[0, 0, 0]);
    let mut clocks = PartClocks::default();

    let p = tick(
        &mut clocks,
        &table,
        0,
        &no_binds(),
        1000,
        true,
        &mut rng,
        &PatternState::default(),
    );
    assert_eq!(calls(&probe), 1, "100 は 3 か所に在っても 1 回だけ引く");
    assert_eq!(part_frames(&p, 100), vec![(0, 101)]);
    assert_eq!(parts_in_field(&p), vec![100], "欄は部品 100 の 1 つだけ");
}

/// 刻みの途中で見えなくなった部品（親の着せ替えのコマが pattern0 を置き換えた先）のコマは、
/// 欄に残さない（部品の欄の番号は `visible_parts` の答えに含まれる・design.md「Data Models」）。
///
/// 既知の振る舞い（設計どおり・決定論）: 親の着せ替えのコマが pattern0 を置き換えている間も、
/// その pattern0 の先（ここでは 40）は刻みの 1 回目の `visible_parts` で見える扱いになり、
/// 毎刻み評価され、毎境界で抽選される。乱数の呼び出し回数をここで固定する。欄には載らない。
#[test]
fn part_hidden_by_a_later_frame_in_the_same_tick_is_not_left_in_the_field() {
    let table = table_of(
        "surface0\n{\nelement0,overlay,100,0,0\n}\n\
         surface100\n{\nanimation5.interval,bind+random,2\n\
         animation5.pattern0,overlay,40,0,0,0\n\
         animation5.pattern1,overlay,41,100,0,0\n}\n\
         surface40\n{\nanimation0.interval,random,2\n\
         animation0.pattern0,overlay,42,0,0,0\n\
         animation0.pattern1,overlay,-1,1000,0,0\n}\n\
         surface41\n{\n}\nsurface42\n{\n}\n",
    );
    let (mut rng, probe) = counting_rng(&[0, 0, 1, 0, 1]);
    let mut clocks = PartClocks::default();
    let on = BindSet::from_ids([5]);
    let top = PatternState::default();

    // 境界: 40（着せ替えの pattern0 の先）と 100 が当たる。100 のコマは 40。
    let p = tick(&mut clocks, &table, 0, &on, 1000, true, &mut rng, &top);
    assert_eq!(calls(&probe), 2);
    assert_eq!(part_frames(&p, 100), vec![(5, 40)]);
    assert_eq!(part_frames(&p, 40), vec![(0, 42)]);

    // 経過 100: 100 のコマが 41 になり、40 はもう絵に出ない → 40 のコマは欄に残さない。
    let p = tick(&mut clocks, &table, 0, &on, 1100, false, &mut rng, &top);
    assert_eq!(part_frames(&p, 100), vec![(5, 41)]);
    assert_eq!(
        parts_in_field(&p),
        vec![100],
        "見えない 40 のコマは載らない"
    );
    assert_eq!(calls(&probe), 2);

    // 境界 2000: 40 は再生中（Playing{1000}）なので抽選されず、経過 1000 の `-1` で止まって時計が消える。
    // 100 は末尾を保っている（Residual）ので抽選の対象（3 回目・外れ）。
    let p = tick(&mut clocks, &table, 0, &on, 2000, true, &mut rng, &top);
    assert_eq!(calls(&probe), 3);
    assert_eq!(clocks.clock(&scope(), 40, 0), None);
    assert_eq!(part_frames(&p, 100), vec![(5, 41)]);
    assert_eq!(parts_in_field(&p), vec![100]);

    // 境界 3000: 40 は絵に出ていないのに抽選される（設計どおりの既知の振る舞い）。欄を空にしてから回すので、
    // 1 回目の `visible_parts` では 100 のコマがまだ無く、着せ替えの pattern0 の先 40 が見える扱いになる。
    // 100 の着せ替えのコマが pattern0 を置き換えている間は、毎刻み 40 を評価し、毎境界で 40 を抽選する
    // （決定論。消費順は 40 → 100）。それでも 40 のコマは欄に載らない。
    let p = tick(&mut clocks, &table, 0, &on, 3000, true, &mut rng, &top);
    assert_eq!(
        calls(&probe),
        5,
        "40（4 回目・当たり）→ 100（5 回目・外れ）"
    );
    assert_eq!(
        clocks.clock(&scope(), 40, 0),
        Some(PartAnim::Playing {
            started_at_ms: 3000
        }),
        "絵に出ていない 40 の時計が動き出す"
    );
    assert_eq!(part_frames(&p, 100), vec![(5, 41)]);
    assert_eq!(parts_in_field(&p), vec![100], "40 のコマは欄に載らない");
}

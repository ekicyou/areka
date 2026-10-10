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
use log_capture_kit::{LineFormat, capture_lines};

use super::{PartAnim, PartClocks};
use crate::looper::tests::{RngProbe, counting_rng};
use crate::state::Slot;
use crate::table::AnimationTable;
use crate::timeline::LoopRng;

pub(super) fn table_of(text: &str) -> AnimationTable {
    AnimationTable::from_world(&EmoWorld::build(&areka_parsers::shell::parse(text)))
}

pub(super) fn scope() -> ActorKey {
    ActorKey::from("0")
}

pub(super) fn no_binds() -> BindSet {
    BindSet::from_ids([])
}

pub(super) fn calls(probe: &Arc<Mutex<RngProbe>>) -> usize {
    probe.lock().unwrap().calls
}

/// overlay・位置 0,0 のコマ（一番上の欄に手で置く用）。
pub(super) fn overlay(surface_id: u32) -> PatternFrame {
    PatternFrame {
        surface_id,
        method: ComposeMethod::Overlay,
        x: 0,
        y: 0,
    }
}

/// 部品 `part` の欄を (animation の番号, コマの番号) の列で読む。
pub(super) fn part_frames(pattern: &PatternState, part: u32) -> Vec<(u32, u32)> {
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
        Slot::Shell,
        top,
        binds,
        table,
        now_ms,
        crossed,
        true,
        None,
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

// ── 7.3: 時計を書き換えない口・時計を捨てる口・記録 ─────────────────────────────

/// 子 100 は 50ms で 101 → さらに 50ms で 102（`-1` の無い末尾＝保つ）。
const HOLD: &str = "surface0\n{\nelement0,overlay,100,0,0\n}\nsurface1\n{\n}\n\
    surface100\n{\nanimation0.interval,random,2\n\
    animation0.pattern0,overlay,101,50,0,0\n\
    animation0.pattern1,overlay,102,50,0,0\n}\n";

/// 時計を書き換えない口（出来事の直後の `refresh`・抽選の時計は書き換えない）で部品の欄を作り直した
/// 結果を返す。
fn peek_at(
    clocks: &mut PartClocks,
    table: &AnimationTable,
    top: u32,
    binds: &BindSet,
    now_ms: u64,
    top_pattern: &PatternState,
) -> PatternState {
    let mut pattern = top_pattern.clone();
    clocks.refresh(
        &scope(),
        Slot::Shell,
        top,
        binds,
        table,
        Some(now_ms),
        true,
        &mut pattern,
    );
    pattern
}

/// 時計を書き換えない口は乱数を呼ばず、時計を変えない。末尾に着いた時刻を見ても `Residual` へ
/// 移さない（移すのは次の刻み）。何度呼んでも、次の刻みの結果は呼ばなかったときと同じ（要件 5.6）。
#[test]
fn peek_draws_no_rng_and_leaves_clocks_unchanged() {
    let table = table_of(HOLD);
    let b = no_binds();
    let top = PatternState::default();

    // 呼ばない側（比べる相手）。
    let (mut rng0, probe0) = counting_rng(&[0, 1]);
    let mut plain = PartClocks::default();
    tick(&mut plain, &table, 0, &b, 1000, true, &mut rng0, &top);
    let expect_1100 = tick(&mut plain, &table, 0, &b, 1100, false, &mut rng0, &top);
    let expect_2000 = tick(&mut plain, &table, 0, &b, 2000, true, &mut rng0, &top);

    // 呼ぶ側。
    let (mut rng, probe) = counting_rng(&[0, 1]);
    let mut clocks = PartClocks::default();
    // 時計が 1 本も無いうちは欄が空になる（前の写しの欄も消える）。
    let mut stale = PatternState::default();
    stale.set_part(100, 0, overlay(999));
    assert!(peek_at(&mut clocks, &table, 0, &b, 900, &stale).is_empty());

    tick(&mut clocks, &table, 0, &b, 1000, true, &mut rng, &top);
    assert_eq!(calls(&probe), 1);
    let playing = Some(PartAnim::Playing {
        started_at_ms: 1000,
    });

    for _ in 0..3 {
        assert!(
            part_frames(&peek_at(&mut clocks, &table, 0, &b, 1000, &top), 100).is_empty(),
            "Pending はコマ無し"
        );
        assert_eq!(
            part_frames(&peek_at(&mut clocks, &table, 0, &b, 1060, &stale), 100),
            vec![(0, 101)],
            "前の写しの欄は作り直される"
        );
        // 末尾の時刻: コマは出すが時計は `Playing` のまま。
        assert_eq!(
            part_frames(&peek_at(&mut clocks, &table, 0, &b, 1100, &top), 100),
            vec![(0, 102)]
        );
        // 見えない面では何も書かない。
        assert!(peek_at(&mut clocks, &table, 1, &b, 1100, &stale).is_empty());
        assert_eq!(clocks.clock(&scope(), 100, 0), playing);
    }
    assert_eq!(calls(&probe), 1, "時計を書き換えない口は乱数を呼ばない");

    // 次の刻みの結果は、呼ばなかった側と同じ。
    assert_eq!(
        tick(&mut clocks, &table, 0, &b, 1100, false, &mut rng, &top),
        expect_1100
    );
    assert_eq!(
        clocks.clock(&scope(), 100, 0),
        Some(PartAnim::Residual { frame_index: 1 })
    );
    // 保っている間も同じ。
    for _ in 0..3 {
        assert_eq!(
            part_frames(&peek_at(&mut clocks, &table, 0, &b, 1500, &top), 100),
            vec![(0, 102)]
        );
    }
    assert_eq!(
        tick(&mut clocks, &table, 0, &b, 2000, true, &mut rng, &top),
        expect_2000
    );
    assert_eq!(calls(&probe), calls(&probe0));
}

/// 時計を書き換えない口でも、有効でない `bind+random` のコマは欄に書かない。時計は消さない
/// （消すのは次の刻み・要件 5.12）。
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

    tick(&mut clocks, &table, 0, &on, 1000, true, &mut rng, &top);
    assert_eq!(
        part_frames(&peek_at(&mut clocks, &table, 0, &on, 1050, &top), 100),
        vec![(5, 101), (6, 103)]
    );

    // 外した直後: 5 のコマは書かず、`random` の 6 は書く。5 の時計は残る。
    assert_eq!(
        part_frames(&peek_at(&mut clocks, &table, 0, &off, 1050, &top), 100),
        vec![(6, 103)]
    );
    assert_eq!(
        clocks.clock(&scope(), 100, 5),
        Some(PartAnim::Playing {
            started_at_ms: 1000
        })
    );

    // 次の刻みで消える。
    tick(&mut clocks, &table, 0, &off, 1100, false, &mut rng, &top);
    assert_eq!(clocks.clock(&scope(), 100, 5), None);
}

/// 時計を捨てる口は全スコープの時計を捨てる（要件 5.8）。
#[test]
fn clear_drops_clocks_of_every_scope() {
    let table = table_of(HOLD);
    let (mut rng, _probe) = counting_rng(&[0, 0]);
    let mut clocks = PartClocks::default();
    let b = no_binds();
    let other = ActorKey::from("1");

    let mut p = PatternState::default();
    clocks.advance(
        &scope(),
        Slot::Shell,
        0,
        &b,
        &table,
        1000,
        true,
        true,
        None,
        &mut rng,
        &mut p,
    );
    let mut p = PatternState::default();
    clocks.advance(
        &other,
        Slot::Shell,
        0,
        &b,
        &table,
        1000,
        true,
        true,
        None,
        &mut rng,
        &mut p,
    );
    assert!(clocks.clock(&scope(), 100, 0).is_some());
    assert!(clocks.clock(&other, 100, 0).is_some());

    clocks.clear(Slot::Shell);
    assert_eq!(clocks.clock(&scope(), 100, 0), None);
    assert_eq!(clocks.clock(&other, 100, 0), None);
    assert!(peek_at(&mut clocks, &table, 0, &b, 1060, &PatternState::default()).is_empty());
}

pub(super) fn capture_logs<F: FnOnce()>(f: F) -> Vec<String> {
    capture_lines(LineFormat::LevelTargetFields, f).1
}

pub(super) fn count(lines: &[String], needles: &[&str]) -> usize {
    lines
        .iter()
        .filter(|l| needles.iter().all(|n| l.contains(n)))
        .count()
}

/// `-1` 以外の負の番号は (スコープ, 部品, animation) ごとに初回だけ `warn!` を出し、その animation を
/// 止める。捨てた後は再び初回として扱う（要件 8.1）。`-1` では出さない。
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
        for base in [1000u64, 2000, 3000] {
            tick(&mut clocks, &table, 0, &b, base, true, &mut rng, &top);
            let p = tick(&mut clocks, &table, 0, &b, base + 50, false, &mut rng, &top);
            assert!(part_frames(&p, 100).is_empty(), "負の番号で止まる");
            assert_eq!(clocks.clock(&scope(), 100, 0), None);
        }
        // 別のスコープは別に数える。
        for base in [1000u64, 2000] {
            let mut p = PatternState::default();
            clocks.advance(
                &other,
                Slot::Shell,
                0,
                &b,
                &table,
                base,
                true,
                true,
                None,
                &mut rng,
                &mut p,
            );
            let mut p = PatternState::default();
            clocks.advance(
                &other,
                Slot::Shell,
                0,
                &b,
                &table,
                base + 50,
                false,
                true,
                None,
                &mut rng,
                &mut p,
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
        tick(&mut clocks, &table, 0, &b, 5000, true, &mut rng, &top);
        tick(&mut clocks, &table, 0, &b, 5050, false, &mut rng, &top);
    });
    assert_eq!(count(&lines, &warn), 1, "{lines:#?}");
}

/// 部品の発火・末尾での保持・停止・着せ替えから外れた停止は `info!` で部品の番号の欄つきで残る。
/// 時計を書き換えない口は何も記録しない（同じ捕捉の中で刻みの行が出ることを陽性対照にする）。
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
        tick(&mut clocks, &table, 0, &on, 1000, true, &mut rng, &top);
        for _ in 0..3 {
            peek_at(&mut clocks, &table, 0, &on, 1100, &top);
            peek_at(&mut clocks, &table, 0, &off, 1100, &top);
        }
        tick(&mut clocks, &table, 0, &on, 1100, false, &mut rng, &top);
        tick(&mut clocks, &table, 0, &off, 1150, false, &mut rng, &top);
        tick(&mut clocks, &table, 0, &off, 1200, false, &mut rng, &top);
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
        "末尾での保持は移った刻みの 1 回だけ（時計を書き換えない口は記録しない）: {part_lines:#?}"
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

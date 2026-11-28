//! ループ統括器（`LoopRuntime`）の抽選・進行・発行の決定論テスト。

use super::*;
use crate::resolve::SurfaceTarget;
use areka_emo_compose::{BindSet, EmoWorld, PatternState};
use areka_parsers::shell::{
    Animation, AppendTarget, DefRef, DrawMethod, Interval, Pattern, Shell, Surface,
};
use dola::time::testing::at_ms;
use std::sync::{Arc, Mutex};

// ── テスト用の表構築（from_world 経由＝実構築パスを通す） ──────────────────

/// コマ 1 本（overlay 固定・x/y=0）。
pub(crate) fn pat(index: u32, surface_id: i64, wait: u32) -> Pattern {
    Pattern {
        index,
        method: DrawMethod::new("overlay".to_string()),
        surface_id,
        target: surface_id.to_string(),
        wait,
        x: 0,
        y: 0,
    }
}

/// animation 1 本を持つ surface 1 件。
pub(crate) fn surface_with(id: u32, animations: Vec<Animation>) -> Surface {
    Surface {
        targets: vec![AppendTarget::Single(id)],
        elements: Vec::new(),
        collisions: Vec::new(),
        animations,
    }
}

pub(super) fn world_of(surfaces: Vec<Surface>) -> EmoWorld {
    let definitions = (0..surfaces.len()).map(DefRef::Surface).collect();
    let shell = Shell {
        surfaces,
        appends: Vec::new(),
        aliases: Vec::new(),
        animation_sort: None,
        collision_sort: None,
        definitions,
    };
    EmoWorld::build(&shell)
}

/// surfaces から shell 表を build する。
pub(crate) fn shell_table(surfaces: Vec<Surface>) -> AnimationTable {
    AnimationTable::from_world(&world_of(surfaces))
}

/// 単一 anim（id/interval/frames）を持つ surface から shell 表を build する。
pub(crate) fn table_single(
    surface_id: u32,
    anim_id: u32,
    interval: Interval,
    frames: &[(i64, u32)],
) -> AnimationTable {
    let patterns = frames
        .iter()
        .enumerate()
        .map(|(i, (sid, wait))| pat(i as u32, *sid, *wait))
        .collect();
    shell_table(vec![surface_with(
        surface_id,
        vec![Animation {
            id: anim_id,
            interval,
            patterns,
        }],
    )])
}

// ── 注入乱数（消費回数を計数する列 rng） ──────────────────────────────────

#[derive(Default)]
pub(crate) struct RngProbe {
    pub(crate) calls: usize,
    values: std::collections::VecDeque<u32>,
}

/// 指定した値列を順に返し、呼ばれた回数を計数する rng を作る（尽きたら 1 を返す＝非発火）。
/// 返す `Arc` から消費回数を照合できる（bind OFF での**非消費**の檻）。
pub(crate) fn counting_rng(values: &[u32]) -> (LoopRng, Arc<Mutex<RngProbe>>) {
    let probe = Arc::new(Mutex::new(RngProbe {
        calls: 0,
        values: values.iter().copied().collect(),
    }));
    let handle = Arc::clone(&probe);
    let rng: LoopRng = Box::new(move |_bound: u32| -> u32 {
        let mut p = probe.lock().unwrap();
        p.calls += 1;
        p.values.pop_front().unwrap_or(1)
    });
    (rng, handle)
}

/// 常に発火する rng（`should_fire` は `rng(k)==0` で発火）。
pub(crate) fn always_fire() -> LoopRng {
    Box::new(|_bound: u32| 0)
}

pub(super) fn shown_shell(sid: u32) -> (ScopeStates, ActorKey) {
    shown_shell_with_binds(sid, BindSet::from_ids([]))
}

pub(crate) fn shown_shell_with_binds(sid: u32, binds: BindSet) -> (ScopeStates, ActorKey) {
    let mut states = ScopeStates::new(crate::DefaultBinds::uniform(binds));
    let scope = ActorKey::from("0");
    states.apply(&scope, SurfaceTarget::Show(sid));
    (states, scope)
}

/// shell 表＋注入乱数から config を組む（バルーン表の写像は空＝全 scope 不活性）。
pub(crate) fn cfg(shell_table: AnimationTable, rng: LoopRng) -> SerikoLoopConfig {
    SerikoLoopConfig {
        shell_table,
        balloon_tables: BTreeMap::new(),
        rng,
        pick_rng: crate::seeded_rng(0),
        text_routing: crate::TextRouting::default(),
    }
}

/// スコープ `"0"` の `slot` の面の (部品, animation の番号) の時計（部品のテストが見る）。
pub(crate) fn part_clock(
    rt: &LoopRuntime,
    slot: Slot,
    part: u32,
    id: u32,
) -> Option<crate::parts::PartAnim> {
    rt.parts.clock_at(
        &ActorKey::from("0"),
        slot,
        areka_emo_compose::PartKey::Surface(part),
        id,
    )
}

// ── 抽選ゲート: 表示中×非再生中×bind ゲート ───────────────────────────────

/// bind OFF（BindRandom で bindgroup が current_binds に無い）→ **乱数を消費しない**・発火しない。表示中でも境界を跨いでも rng は 1 度も呼ばれない。
#[test]
fn bindrandom_off_does_not_consume_rng() {
    // BindRandom{4}・anim id 7・bind ゲート id は 7。binds は空 ⇒ OFF。
    let table = table_single(10, 7, Interval::BindRandom { k: 4 }, &[(1412, 0)]);
    let (rng, probe) = counting_rng(&[0, 0, 0]); // もし呼ばれれば発火してしまう値
    let mut rt = LoopRuntime::new(cfg(table, rng));
    let (mut states, _scope) = shown_shell(10); // binds 空 ⇒ id 7 は OFF

    assert!(
        rt.on_tick(at_ms(0), &mut states).is_empty(),
        "起動 tick は無発行"
    );
    let cmds = rt.on_tick(at_ms(1000), &mut states);
    assert!(cmds.is_empty(), "bind OFF は発火しない＝無発行");

    assert_eq!(
        probe.lock().unwrap().calls,
        0,
        "bind OFF で rng は非消費（要件 3.1）"
    );
}

/// bind ON（bindgroup が current_binds に在る）＋境界跨ぎ＋発火 rng → 発火し先頭コマを発行する。rng は 1 回だけ消費される。
#[test]
fn bindrandom_on_fires_and_consumes_rng_once() {
    let table = table_single(
        10,
        7,
        Interval::BindRandom { k: 4 },
        &[(1412, 0), (1411, 150)],
    );
    let (rng, probe) = counting_rng(&[0]); // 1 回目 0 ⇒ 発火
    let mut rt = LoopRuntime::new(cfg(table, rng));
    // binds に id 7 を含める ⇒ ON。
    let (mut states, scope) = shown_shell_with_binds(10, BindSet::from_ids([7]));

    assert!(rt.on_tick(at_ms(0), &mut states).is_empty());
    let cmds = rt.on_tick(at_ms(1000), &mut states); // 跨ぎ＋発火＋elapsed0 で先頭コマ 1412
    assert_eq!(probe.lock().unwrap().calls, 1, "ON では rng を 1 回消費");
    assert_eq!(cmds.len(), 1, "発火＋先頭コマで 1 指令");
    match &cmds[0] {
        DisplayCommand::Show {
            scope: s,
            surface_id,
            pattern,
            ..
        } => {
            assert_eq!(s, &scope);
            assert_eq!(*surface_id, 10, "表示中 surface は 10");
            let f = pattern.get(7).expect("anim7 の現在コマ");
            assert_eq!(f.surface_id, 1412, "先頭コマ 1412（要件 2.2 先頭から）");
        }
        other => panic!("Show を期待: {other:?}"),
    }
}

/// Random は無条件ゲート（bind に依らず発火する）。
#[test]
fn random_fires_unconditionally() {
    let table = table_single(10, 0, Interval::Random { k: 4 }, &[(2106, 0)]);
    let mut rt = LoopRuntime::new(cfg(table, always_fire()));
    let (mut states, _scope) = shown_shell(10); // binds 空でも Random は無条件

    rt.on_tick(at_ms(0), &mut states);
    let cmds = rt.on_tick(at_ms(1000), &mut states);
    assert_eq!(cmds.len(), 1, "Random は無条件で発火");
    match &cmds[0] {
        DisplayCommand::Show { pattern, .. } => {
            assert_eq!(pattern.get(0).unwrap().surface_id, 2106);
        }
        other => panic!("Show を期待: {other:?}"),
    }
}

/// 非表示 slot は評価対象ゼロ＝抽選も進行も走らず乱数も消費しない（表示中ゲート）。
#[test]
fn hidden_slot_consumes_no_rng() {
    let table = table_single(10, 0, Interval::Random { k: 4 }, &[(2106, 0)]);
    let (rng, probe) = counting_rng(&[0]);
    let mut rt = LoopRuntime::new(cfg(table, rng));
    // Show せず（未知 scope）＝表示中 slot が 1 つも無い。
    let mut states = crate::test_support::empty_states();

    rt.on_tick(at_ms(0), &mut states);
    let cmds = rt.on_tick(at_ms(1000), &mut states);
    assert!(cmds.is_empty());
    assert_eq!(
        probe.lock().unwrap().calls,
        0,
        "表示中 slot 皆無で rng 非消費"
    );
}

/// 再生中アニメは再抽選対象外＝次の境界跨ぎで乱数を消費しない。
#[test]
fn playing_anim_is_not_relotteried() {
    // 次の境界を越えて続く残留アニメ（末尾非負・1500ms 継続）。
    let table = table_single(10, 0, Interval::Random { k: 4 }, &[(2106, 0), (2110, 1500)]);
    let (rng, probe) = counting_rng(&[0]); // 最初の跨ぎで 1 回だけ発火
    let mut rt = LoopRuntime::new(cfg(table, rng));
    let (mut states, _scope) = shown_shell(10);

    rt.on_tick(at_ms(0), &mut states);
    rt.on_tick(at_ms(1000), &mut states); // 発火（rng 1 回消費）・再生中へ
    // 次の境界（2000）で再生中ゆえ再抽選しない＝rng 追加消費なし。
    rt.on_tick(at_ms(2000), &mut states);
    assert_eq!(
        probe.lock().unwrap().calls,
        1,
        "再生中アニメは再抽選されず rng は追加消費されない（要件 2.3）"
    );
}

// ── 終端意味論: Stopped（ベース復帰）／FinishedResidual（残留・再抽選可） ────

/// kero 型（末尾 `-1`）: `-1` 到達でコマ除去＋playback 除去＝ベース復帰し、以降の境界で再抽選できる（rng 再消費）。
#[test]
fn kero_negative_tail_restores_base_and_is_relotteriable() {
    // 2106(w0)/2110(w40)/-1(w80) ⇒ t=[0,40,120]。
    let table = table_single(
        10,
        0,
        Interval::Random { k: 4 },
        &[(2106, 0), (2110, 40), (-1, 80)],
    );
    let (rng, probe) = counting_rng(&[0, 0]); // 2 度の跨ぎで各 1 回発火
    let mut rt = LoopRuntime::new(cfg(table, rng));
    let (mut states, _scope) = shown_shell(10);

    rt.on_tick(at_ms(0), &mut states);
    let c1 = rt.on_tick(at_ms(1000), &mut states); // 発火・elapsed0 → 2106
    assert_eq!(pattern_of(&c1[0]).get(0).unwrap().surface_id, 2106);

    // 経過 120ms（1120）で -1 → Stopped → ベース復帰（空 pattern の Show）。
    let c2 = rt.on_tick(at_ms(1120), &mut states);
    assert_eq!(c2.len(), 1, "停止でベース復帰の再発行");
    assert!(
        pattern_of(&c2[0]).is_empty(),
        "-1 停止でコマ除去＝空 pattern（ベース復帰・要件 4.3）"
    );

    // 次の境界 2000 で再抽選できる（Idle へ戻っている）＝rng が再度消費される。
    rt.on_tick(at_ms(2000), &mut states);
    assert_eq!(
        probe.lock().unwrap().calls,
        2,
        "停止後は Idle ゆえ再抽選（rng 再消費）"
    );
}

/// sakura 型（末尾非負）: 末尾到達で最終コマを残したまま playback 除去＝IdleResidual。残留は保たれ、
/// 次の境界で再抽選できる。
#[test]
fn sakura_residual_keeps_frame_and_is_relotteriable() {
    // 1412(w0)/1411(w150)/1410(w22) ⇒ t=[0,150,172]。末尾非負。
    let table = table_single(
        10,
        7,
        Interval::BindRandom { k: 4 },
        &[(1412, 0), (1411, 150), (1410, 22)],
    );
    let (rng, probe) = counting_rng(&[0, 0]);
    let mut rt = LoopRuntime::new(cfg(table, rng));
    let (mut states, _scope) = shown_shell_with_binds(10, BindSet::from_ids([7]));

    rt.on_tick(at_ms(0), &mut states);
    rt.on_tick(at_ms(1000), &mut states); // 発火 → 1412
    // 経過 172ms（1172）で末尾 1410 残留。
    let c = rt.on_tick(at_ms(1172), &mut states);
    assert_eq!(c.len(), 1);
    assert_eq!(
        pattern_of(&c[0]).get(7).unwrap().surface_id,
        1410,
        "末尾コマ 1410 残留（要件 4.4）"
    );

    // さらに時間が進んでも残留は不変（無発行）。
    assert!(
        rt.on_tick(at_ms(1500), &mut states).is_empty(),
        "残留は恒久＝無発行（要件 6.2）"
    );

    // 次の境界 2000 で再抽選できる（IdleResidual は抽選対象）。
    rt.on_tick(at_ms(2000), &mut states);
    assert_eq!(
        probe.lock().unwrap().calls,
        2,
        "IdleResidual は再抽選対象（rng 再消費）"
    );
}

/// 残留→再発火の**即時クリア**: 先頭 wait>0 の合成テーブルで、末尾非負の残留状態から
/// 再発火した tick に残留コマが即消えベース復帰し、先頭コマ deadline で改めて表示される。
#[test]
fn residual_is_immediately_cleared_on_refire() {
    // 先頭 wait=50>0・末尾非負の 2 コマ: 700(w50)/701(w50) ⇒ t=[50,100]。
    let table = table_single(10, 0, Interval::Random { k: 4 }, &[(700, 50), (701, 50)]);
    let mut rt = LoopRuntime::new(cfg(table, always_fire()));
    let (mut states, _scope) = shown_shell(10);

    rt.on_tick(at_ms(0), &mut states);
    rt.on_tick(at_ms(1000), &mut states); // 発火（started_at=1000）・elapsed0 → Pending（wait50 未到達）＝無コマ
    // 経過 100ms（1100）で末尾 701 残留。
    let c1 = rt.on_tick(at_ms(1100), &mut states);
    assert_eq!(
        pattern_of(&c1[0]).get(0).unwrap().surface_id,
        701,
        "末尾 701 残留"
    );

    // 次の境界 2000 で再発火（always_fire）。elapsed0 → Pending → 残留 701 が即時クリア（ベース復帰）。
    let c2 = rt.on_tick(at_ms(2000), &mut states);
    assert_eq!(c2.len(), 1, "再発火 tick で残留クリアの再発行");
    assert!(
        pattern_of(&c2[0]).is_empty(),
        "再発火の瞬間に残留コマが即時クリア＝ベース露出"
    );

    // 先頭コマ deadline（started_at 2000＋50＝2050）で先頭コマ 700 が表示される。
    let c3 = rt.on_tick(at_ms(2050), &mut states);
    assert_eq!(
        pattern_of(&c3[0]).get(0).unwrap().surface_id,
        700,
        "先頭コマ 700 が deadline で表示"
    );
}

// ── on_surface_changed / 単調性ガード ──────────────────────────────────

/// `on_surface_changed` は当該 slot の playback を全除去する（surface 従属）。
/// 除去後は再抽選対象へ戻る（rng が再消費される）。
#[test]
fn on_surface_changed_removes_playback() {
    let table = table_single(10, 0, Interval::Random { k: 4 }, &[(2106, 0), (2110, 1500)]);
    let (rng, probe) = counting_rng(&[0, 0]);
    let mut rt = LoopRuntime::new(cfg(table, rng));
    let (mut states, scope) = shown_shell(10);

    rt.on_tick(at_ms(0), &mut states);
    rt.on_tick(at_ms(1000), &mut states); // 発火・再生中（rng 1 回）
    rt.on_surface_changed(&scope, Slot::Shell);
    // 次の境界で再抽選できる（playback が消えたので Idle）＝rng 再消費。
    rt.on_tick(at_ms(2000), &mut states);
    assert_eq!(
        probe.lock().unwrap().calls,
        2,
        "on_surface_changed 後は Idle ゆえ再抽選（rng 再消費）"
    );
}

/// 非単調 tick（now < 前回）は無視され、状態を変えず空を返す（防御）。
/// コマが替わった後に時刻が戻っても、前のコマへ巻き戻る指令を出さず、番人の時刻も戻さない。
#[test]
fn non_monotonic_tick_is_ignored() {
    // 2000 で発火し、コマ 2 枚目（2110）は 2100 から。2150 と 2050 の間にコマの替わり目がある。
    // 3 枚目は 2150 で再生がまだ終わっていないために置く（2 枚だと 2150 で再生が消え、2050 で
    // 何も出せず番人を外しても緑のまま）。
    let table = table_single(
        10,
        0,
        Interval::Random { k: 4 },
        &[(2106, 0), (2110, 100), (2120, 1000)],
    );
    let (rng, _probe) = counting_rng(&[0]);
    let mut rt = LoopRuntime::new(cfg(table, rng));
    let (mut states, _scope) = shown_shell(10);

    rt.on_tick(at_ms(1000), &mut states); // 初回（遅延初期化・境界 next=2000）
    assert!(
        !rt.on_tick(at_ms(2000), &mut states).is_empty(),
        "2000 で発火する"
    );
    assert!(
        !rt.on_tick(at_ms(2150), &mut states).is_empty(),
        "2150 でコマ 2 枚目へ替わる"
    );
    // 過去へ戻る tick は無視（前のコマ 2106 へ巻き戻る Show を出さない）。
    let cmds = rt.on_tick(at_ms(2050), &mut states);
    assert!(cmds.is_empty(), "非単調 tick は無発行: {cmds:?}");
    assert_eq!(
        rt.last_seen,
        Some(at_ms(2150)),
        "非単調 tick は番人の時刻を戻さない"
    );
}

/// bind 書込 API は一切呼ばない（read-only）: on_tick 前後で current_binds が不変であることの反証。
#[test]
fn on_tick_never_writes_binds() {
    let table = table_single(10, 7, Interval::BindRandom { k: 4 }, &[(1412, 0)]);
    let mut rt = LoopRuntime::new(cfg(table, always_fire()));
    let (mut states, scope) = shown_shell_with_binds(10, BindSet::from_ids([7]));

    let before = states.current_binds(&scope).clone();
    rt.on_tick(at_ms(0), &mut states);
    rt.on_tick(at_ms(1000), &mut states); // 発火（BindRandom ON）
    let after = states.current_binds(&scope).clone();
    assert_eq!(
        before, after,
        "on_tick は bind を書き込まない（read-only・要件 3.3）"
    );
}

/// `SerikoLoopConfig::disabled()` は空表＋ダミー乱数でループ完全不活性（常に無発行）。
#[test]
fn disabled_config_is_inert() {
    let mut rt = LoopRuntime::new(SerikoLoopConfig::disabled());
    let (mut states, _scope) = shown_shell(10);
    assert!(rt.on_tick(at_ms(0), &mut states).is_empty());
    assert!(
        rt.on_tick(at_ms(1000), &mut states).is_empty(),
        "空表は境界跨ぎでも無発行"
    );
    assert!(rt.on_tick(at_ms(5000), &mut states).is_empty());
}

/// 変化なし tick は無発行（冪等）: 再生中でも同一コマが続く間は指令を返さない。
#[test]
fn unchanged_tick_emits_nothing() {
    // 先頭コマが長く続く（w0 の 2106 が 150ms 継続）。
    let table = table_single(10, 0, Interval::Random { k: 4 }, &[(2106, 0), (2110, 150)]);
    let mut rt = LoopRuntime::new(cfg(table, always_fire()));
    let (mut states, _scope) = shown_shell(10);

    rt.on_tick(at_ms(0), &mut states);
    let c1 = rt.on_tick(at_ms(1000), &mut states); // 発火・2106
    assert_eq!(c1.len(), 1);
    // 同じ 2106 が続く中間 tick（境界も跨がない）は無発行。
    assert!(
        rt.on_tick(at_ms(1050), &mut states).is_empty(),
        "同一コマ継続は無発行（要件 6.2）"
    );
    assert!(rt.on_tick(at_ms(1100), &mut states).is_empty());
}

// ── 補助 ────────────────────────────────────────────────────────────────

pub(crate) fn pattern_of(cmd: &DisplayCommand) -> &PatternState {
    match cmd {
        DisplayCommand::Show { pattern, .. } | DisplayCommand::ShowBalloon { pattern, .. } => {
            pattern
        }
        other => panic!("pattern を持つ指令を期待: {other:?}"),
    }
}

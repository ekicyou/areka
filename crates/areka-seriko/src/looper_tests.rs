use super::*;
use crate::resolve::SurfaceTarget;
use areka_emo_compose::{BindSet, EmoWorld, PatternState};
use areka_parsers::shell::{
    Animation, AppendTarget, DefRef, DrawMethod, Interval, Pattern, Shell, Surface,
};
use std::sync::{Arc, Mutex};

// ── テスト用の表構築（from_world 経由＝実構築パスを通す） ──────────────────

/// コマ 1 本（overlay 固定・x/y=0）。
fn pat(index: u32, surface_id: i64, wait: u32) -> Pattern {
    Pattern {
        index,
        method: DrawMethod::new("overlay".to_string()),
        surface_id,
        wait,
        x: 0,
        y: 0,
    }
}

/// animation 1 本を持つ surface 1 件。
fn surface_with(id: u32, animations: Vec<Animation>) -> Surface {
    Surface {
        id,
        targets: vec![AppendTarget::Single(id)],
        elements: Vec::new(),
        collisions: Vec::new(),
        animations,
    }
}

fn world_of(surfaces: Vec<Surface>) -> EmoWorld {
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
fn shell_table(surfaces: Vec<Surface>) -> AnimationTable {
    AnimationTable::from_world(&world_of(surfaces))
}

/// 単一 anim（id/interval/frames）を持つ surface から shell 表を build する。
fn table_single(
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
struct RngProbe {
    calls: usize,
    values: std::collections::VecDeque<u32>,
}

/// 指定した値列を順に返し、呼ばれた回数を計数する rng を作る（尽きたら 1 を返す＝非発火）。
/// 返す `Arc` から消費回数・残量を照合できる（bind OFF での**非消費**の檻・要件 3.1）。
fn counting_rng(values: &[u32]) -> (LoopRng, Arc<Mutex<RngProbe>>) {
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
fn always_fire() -> LoopRng {
    Box::new(|_bound: u32| 0)
}

fn shown_shell(sid: u32) -> (ScopeStates, ActorKey) {
    shown_shell_with_binds(sid, BindSet::from_ids([]))
}

fn shown_shell_with_binds(sid: u32, binds: BindSet) -> (ScopeStates, ActorKey) {
    let mut states = ScopeStates::new(binds);
    let scope = ActorKey::from("0");
    states.apply(&scope, SurfaceTarget::Show(sid));
    (states, scope)
}

/// shell 表＋注入乱数から config を組む（バルーン表の写像は空＝全 scope 不活性）。
fn cfg(shell_table: AnimationTable, rng: LoopRng) -> SerikoLoopConfig {
    SerikoLoopConfig {
        shell_table,
        balloon_tables: BTreeMap::new(),
        rng,
    }
}

// ── 抽選ゲート: 表示中×非再生中×bind ゲート ───────────────────────────────

/// bind OFF（BindRandom で bindgroup が current_binds に無い）→ **乱数を消費しない**・発火しない
/// （要件 3.1・CRITICAL）。表示中でも境界を跨いでも rng は 1 度も呼ばれない。
#[test]
fn bindrandom_off_does_not_consume_rng() {
    // BindRandom{4}・anim id 7・bind ゲート id は 7。binds は空 ⇒ OFF。
    let table = table_single(10, 7, Interval::BindRandom { k: 4 }, &[(1412, 0)]);
    let (rng, probe) = counting_rng(&[0, 0, 0]); // もし呼ばれれば発火してしまう値
    let mut rt = LoopRuntime::new(cfg(table, rng));
    let (mut states, _scope) = shown_shell(10); // binds 空 ⇒ id 7 は OFF

    // 起動 tick（遅延初期化・非跨ぎ）→ 境界を跨ぐ tick。
    assert!(rt.on_tick(0, &mut states).is_empty(), "起動 tick は無発行");
    let cmds = rt.on_tick(1000, &mut states); // 境界跨ぎ
    assert!(cmds.is_empty(), "bind OFF は発火しない＝無発行");

    // 乱数は 1 度も消費されていない（ゲート不通過で should_fire 未呼出・要件 3.1）。
    assert_eq!(probe.lock().unwrap().calls, 0, "bind OFF で rng は非消費（要件 3.1）");
}

/// bind ON（bindgroup が current_binds に在る）＋境界跨ぎ＋発火 rng → 発火し先頭コマを発行する
/// （要件 3.2/2.2）。rng は 1 回だけ消費される。
#[test]
fn bindrandom_on_fires_and_consumes_rng_once() {
    let table = table_single(10, 7, Interval::BindRandom { k: 4 }, &[(1412, 0), (1411, 150)]);
    let (rng, probe) = counting_rng(&[0]); // 1 回目 0 ⇒ 発火
    let mut rt = LoopRuntime::new(cfg(table, rng));
    // binds に id 7 を含める ⇒ ON。
    let (mut states, scope) = shown_shell_with_binds(10, BindSet::from_ids([7]));

    assert!(rt.on_tick(0, &mut states).is_empty());
    let cmds = rt.on_tick(1000, &mut states); // 跨ぎ＋発火＋elapsed0 で先頭コマ 1412
    assert_eq!(probe.lock().unwrap().calls, 1, "ON では rng を 1 回消費");
    assert_eq!(cmds.len(), 1, "発火＋先頭コマで 1 指令");
    match &cmds[0] {
        DisplayCommand::Show { scope: s, surface_id, pattern, .. } => {
            assert_eq!(s, &scope);
            assert_eq!(*surface_id, 10, "表示中 surface は 10");
            let f = pattern.get(7).expect("anim7 の現在コマ");
            assert_eq!(f.surface_id, 1412, "先頭コマ 1412（要件 2.2 先頭から）");
        }
        other => panic!("Show を期待: {other:?}"),
    }
}

/// Random は無条件ゲート（bind に依らず発火する・要件 3.2）。
#[test]
fn random_fires_unconditionally() {
    let table = table_single(10, 0, Interval::Random { k: 4 }, &[(2106, 0)]);
    let mut rt = LoopRuntime::new(cfg(table, always_fire()));
    let (mut states, _scope) = shown_shell(10); // binds 空でも Random は無条件

    rt.on_tick(0, &mut states);
    let cmds = rt.on_tick(1000, &mut states);
    assert_eq!(cmds.len(), 1, "Random は無条件で発火");
    match &cmds[0] {
        DisplayCommand::Show { pattern, .. } => {
            assert_eq!(pattern.get(0).unwrap().surface_id, 2106);
        }
        other => panic!("Show を期待: {other:?}"),
    }
}

/// 非表示 slot は評価対象ゼロ＝抽選も進行も走らず乱数も消費しない（要件 2.1 の表示中ゲート）。
#[test]
fn hidden_slot_consumes_no_rng() {
    let table = table_single(10, 0, Interval::Random { k: 4 }, &[(2106, 0)]);
    let (rng, probe) = counting_rng(&[0]);
    let mut rt = LoopRuntime::new(cfg(table, rng));
    // Show せず（未知 scope）＝表示中 slot が 1 つも無い。
    let mut states = ScopeStates::new(BindSet::from_ids([]));

    rt.on_tick(0, &mut states);
    let cmds = rt.on_tick(1000, &mut states);
    assert!(cmds.is_empty());
    assert_eq!(probe.lock().unwrap().calls, 0, "表示中 slot 皆無で rng 非消費");
}

/// 再生中アニメは再抽選対象外＝次の境界跨ぎで乱数を消費しない（要件 2.3）。
#[test]
fn playing_anim_is_not_relotteried() {
    // 長い残留アニメ（末尾非負・150ms 継続）。
    let table = table_single(10, 0, Interval::Random { k: 4 }, &[(2106, 0), (2110, 150)]);
    let (rng, probe) = counting_rng(&[0]); // 最初の跨ぎで 1 回だけ発火
    let mut rt = LoopRuntime::new(cfg(table, rng));
    let (mut states, _scope) = shown_shell(10);

    rt.on_tick(0, &mut states);
    rt.on_tick(1000, &mut states); // 発火（rng 1 回消費）・再生中へ
    // 次の境界（2000）で再生中ゆえ再抽選しない＝rng 追加消費なし。
    rt.on_tick(2000, &mut states);
    assert_eq!(
        probe.lock().unwrap().calls,
        1,
        "再生中アニメは再抽選されず rng は追加消費されない（要件 2.3）"
    );
}

// ── 終端意味論: Stopped（ベース復帰）／FinishedResidual（残留・再抽選可） ────

/// kero 型（末尾 `-1`）: `-1` 到達でコマ除去＋playback 除去＝ベース復帰し、以降の境界で再抽選できる
/// （rng 再消費・要件 4.3/2.3）。
#[test]
fn kero_negative_tail_restores_base_and_is_relotteriable() {
    // 2106(w0)/2110(w40)/-1(w80) ⇒ t=[0,40,120]。
    let table = table_single(10, 0, Interval::Random { k: 4 }, &[(2106, 0), (2110, 40), (-1, 80)]);
    let (rng, probe) = counting_rng(&[0, 0]); // 2 度の跨ぎで各 1 回発火
    let mut rt = LoopRuntime::new(cfg(table, rng));
    let (mut states, _scope) = shown_shell(10);

    rt.on_tick(0, &mut states);
    let c1 = rt.on_tick(1000, &mut states); // 発火・elapsed0 → 2106
    assert_eq!(pattern_of(&c1[0]).get(0).unwrap().surface_id, 2106);

    // 経過 120ms（1120）で -1 → Stopped → ベース復帰（空 pattern の Show）。
    let c2 = rt.on_tick(1120, &mut states);
    assert_eq!(c2.len(), 1, "停止でベース復帰の再発行");
    assert!(pattern_of(&c2[0]).is_empty(), "-1 停止でコマ除去＝空 pattern（ベース復帰・要件 4.3）");

    // 次の境界 2000 で再抽選できる（Idle へ戻っている）＝rng が再度消費される。
    rt.on_tick(2000, &mut states);
    assert_eq!(probe.lock().unwrap().calls, 2, "停止後は Idle ゆえ再抽選（rng 再消費）");
}

/// sakura 型（末尾非負）: 末尾到達で最終コマを残したまま playback 除去＝IdleResidual。残留は保たれ、
/// 次の境界で再抽選できる（要件 4.4/2.3）。
#[test]
fn sakura_residual_keeps_frame_and_is_relotteriable() {
    // 1412(w0)/1411(w150)/1410(w22) ⇒ t=[0,150,172]。末尾非負。
    let table = table_single(10, 7, Interval::BindRandom { k: 4 }, &[(1412, 0), (1411, 150), (1410, 22)]);
    let (rng, probe) = counting_rng(&[0, 0]);
    let mut rt = LoopRuntime::new(cfg(table, rng));
    let (mut states, _scope) = shown_shell_with_binds(10, BindSet::from_ids([7]));

    rt.on_tick(0, &mut states);
    rt.on_tick(1000, &mut states); // 発火 → 1412
    // 経過 172ms（1172）で末尾 1410 残留。
    let c = rt.on_tick(1172, &mut states);
    assert_eq!(c.len(), 1);
    assert_eq!(pattern_of(&c[0]).get(7).unwrap().surface_id, 1410, "末尾コマ 1410 残留（要件 4.4）");

    // さらに時間が進んでも残留は不変（無発行）。
    assert!(rt.on_tick(1500, &mut states).is_empty(), "残留は恒久＝無発行（要件 6.2）");

    // 次の境界 2000 で再抽選できる（IdleResidual は抽選対象・要件 2.3/9.4）。
    rt.on_tick(2000, &mut states);
    assert_eq!(probe.lock().unwrap().calls, 2, "IdleResidual は再抽選対象（rng 再消費）");
}

/// 残留→再発火の**即時クリア**（討議 #2）: 先頭 wait>0 の合成テーブルで、末尾非負の残留状態から
/// 再発火した tick に残留コマが即消えベース復帰し、先頭コマ deadline で改めて表示される。
#[test]
fn residual_is_immediately_cleared_on_refire() {
    // 先頭 wait=50>0・末尾非負の 2 コマ: 700(w50)/701(w50) ⇒ t=[50,100]。
    let table = table_single(10, 0, Interval::Random { k: 4 }, &[(700, 50), (701, 50)]);
    let mut rt = LoopRuntime::new(cfg(table, always_fire()));
    let (mut states, _scope) = shown_shell(10);

    rt.on_tick(0, &mut states);
    rt.on_tick(1000, &mut states); // 発火（started_at=1000）・elapsed0 → Pending（wait50 未到達）＝無コマ
    // 経過 100ms（1100）で末尾 701 残留。
    let c1 = rt.on_tick(1100, &mut states);
    assert_eq!(pattern_of(&c1[0]).get(0).unwrap().surface_id, 701, "末尾 701 残留");

    // 次の境界 2000 で再発火（always_fire）。elapsed0 → Pending → 残留 701 が即時クリア（ベース復帰）。
    let c2 = rt.on_tick(2000, &mut states);
    assert_eq!(c2.len(), 1, "再発火 tick で残留クリアの再発行");
    assert!(
        pattern_of(&c2[0]).is_empty(),
        "再発火の瞬間に残留コマが即時クリア＝ベース露出（討議 #2）"
    );

    // 先頭コマ deadline（started_at 2000＋50＝2050）で先頭コマ 700 が表示される。
    let c3 = rt.on_tick(2050, &mut states);
    assert_eq!(pattern_of(&c3[0]).get(0).unwrap().surface_id, 700, "先頭コマ 700 が deadline で表示");
}

// ── 固定消費順（scope 昇順→Shell→Balloon→id 昇順・D-7） ─────────────────

/// 同一 surface に複数アニメ（id 昇順で消費）。注入列 [1,0] で id 昇順の 2 本目のみ発火することを示し、
/// 消費順が animation id 昇順であることを檻に入れる（要件 2.4・D-7）。
#[test]
fn lottery_consumes_in_animation_id_order() {
    // 同 surface 10 に anim id 5 と anim id 2（宣言順は 5→2＝id 昇順ではない）。
    let s = surface_with(
        10,
        vec![
            Animation { id: 5, interval: Interval::Random { k: 4 }, patterns: vec![pat(0, 5000, 0)] },
            Animation { id: 2, interval: Interval::Random { k: 4 }, patterns: vec![pat(0, 2000, 0)] },
        ],
    );
    let table = shell_table(vec![s]);
    // 消費列 [1, 0]: 1 本目（id 昇順の先頭＝id 2）は 1（非発火）、2 本目（id 5）は 0（発火）。
    let (rng, probe) = counting_rng(&[1, 0]);
    let mut rt = LoopRuntime::new(cfg(table, rng));
    let (mut states, _scope) = shown_shell(10);

    rt.on_tick(0, &mut states);
    let cmds = rt.on_tick(1000, &mut states);
    assert_eq!(probe.lock().unwrap().calls, 2, "2 アニメ分の rng を消費");
    assert_eq!(cmds.len(), 1, "id 5 のみ発火");
    // id 2 は非発火（コマ無し）・id 5 は発火（コマ 5000）＝消費順が id 昇順（2→5）である証拠。
    let p = pattern_of(&cmds[0]);
    assert!(p.get(2).is_none(), "id 2 は非発火（消費列 1 番目に 1 が割当）");
    assert_eq!(p.get(5).unwrap().surface_id, 5000, "id 5 は発火（消費列 2 番目に 0 が割当）");
}

/// scope 昇順で消費する（D-7）。scope "0"→"1" の順で乱数を消費し、注入列 [1,0] で scope1 のみ発火。
#[test]
fn lottery_consumes_in_scope_ascending_order() {
    // 同一 shell_table 上に surface10（scope0 表示）と surface11（scope1 表示）。
    let s10 = surface_with(
        10,
        vec![Animation { id: 0, interval: Interval::Random { k: 4 }, patterns: vec![pat(0, 1000, 0)] }],
    );
    let s11 = surface_with(
        11,
        vec![Animation { id: 0, interval: Interval::Random { k: 4 }, patterns: vec![pat(0, 1100, 0)] }],
    );
    let table = shell_table(vec![s10, s11]);
    // [1, 0]: 先頭消費（scope "0"）は非発火、2 番目（scope "1"）は発火。
    let (rng, probe) = counting_rng(&[1, 0]);
    let mut rt = LoopRuntime::new(cfg(table, rng));
    let mut states = ScopeStates::new(BindSet::from_ids([]));
    let s0 = ActorKey::from("0");
    let s1 = ActorKey::from("1");
    states.apply(&s0, SurfaceTarget::Show(10));
    states.apply(&s1, SurfaceTarget::Show(11));

    rt.on_tick(0, &mut states);
    let cmds = rt.on_tick(1000, &mut states);
    assert_eq!(probe.lock().unwrap().calls, 2);
    assert_eq!(cmds.len(), 1, "scope1 のみ発火");
    match &cmds[0] {
        DisplayCommand::Show { scope, pattern, .. } => {
            assert_eq!(scope, &s1, "発火したのは scope \"1\"（scope 昇順で \"0\" が先に消費された証拠）");
            assert_eq!(pattern.get(0).unwrap().surface_id, 1100);
        }
        other => panic!("Show を期待: {other:?}"),
    }
}

/// Shell を Balloon より先に消費する（同一 scope・D-7）。注入列 [0,1] で Shell が発火・Balloon が非発火。
#[test]
fn lottery_consumes_shell_before_balloon() {
    let shell_t = table_single(10, 0, Interval::Random { k: 4 }, &[(2106, 0)]);
    let balloon_t = table_single(20, 1, Interval::Random { k: 4 }, &[(3000, 0)]);
    // 消費列 [0, 1]: Shell(先) 発火・Balloon(後) 非発火。
    let (rng, probe) = counting_rng(&[0, 1]);
    let scope = ActorKey::from("0");
    let mut rt = LoopRuntime::new(SerikoLoopConfig {
        shell_table: shell_t,
        balloon_tables: BTreeMap::from([(scope.clone(), balloon_t)]),
        rng,
    });
    let mut states = ScopeStates::new(BindSet::from_ids([]));
    states.apply(&scope, SurfaceTarget::Show(10)); // Shell shown surface 10
    states.apply_balloon(&scope, SurfaceTarget::Show(20)); // Balloon shown surface 20

    rt.on_tick(0, &mut states);
    let cmds = rt.on_tick(1000, &mut states);
    assert_eq!(probe.lock().unwrap().calls, 2);
    assert_eq!(cmds.len(), 1, "Shell のみ発火");
    assert!(
        matches!(&cmds[0], DisplayCommand::Show { .. }),
        "発火したのは Shell（Show）＝Shell が Balloon より先に消費（D-7）"
    );
}

// ── scope キー表引き（バルーン表・檻 10・要件 5.6） ──────────────────────

/// バルーン表は scope ごとに独立に引かれる（要件 5.6）。同一 surface id・同一 animation id でも、
/// scope 0 は自 scope の表のコマ、scope 1 は自 scope の表のコマで駆動される
/// （ある scope が別 scope の系列由来の定義で駆動されないことの反証）。
#[test]
fn balloon_tables_are_looked_up_per_scope() {
    // 同じ surface 20・同じ anim id 1 でありながら、コマの surface_id が表ごとに異なる。
    let t0 = table_single(20, 1, Interval::Random { k: 4 }, &[(3000, 0)]);
    let t1 = table_single(20, 1, Interval::Random { k: 4 }, &[(4000, 0)]);
    let mut rt = LoopRuntime::new(SerikoLoopConfig {
        shell_table: AnimationTable::empty(),
        balloon_tables: BTreeMap::from([(ActorKey::from("0"), t0), (ActorKey::from("1"), t1)]),
        rng: always_fire(),
    });
    let mut states = ScopeStates::new(BindSet::from_ids([]));
    let s0 = ActorKey::from("0");
    let s1 = ActorKey::from("1");
    states.apply_balloon(&s0, SurfaceTarget::Show(20));
    states.apply_balloon(&s1, SurfaceTarget::Show(20));

    rt.on_tick(0, &mut states);
    let cmds = rt.on_tick(1000, &mut states);
    assert_eq!(cmds.len(), 2, "両 scope が自 scope の表で発火（scope 昇順）");
    match &cmds[0] {
        DisplayCommand::ShowBalloon { scope, pattern, .. } => {
            assert_eq!(scope, &s0);
            assert_eq!(
                pattern.get(1).expect("scope0 anim1 の現在コマ").surface_id,
                3000,
                "scope0 は scope0 の表のコマ（要件 5.6）"
            );
        }
        other => panic!("ShowBalloon を期待: {other:?}"),
    }
    match &cmds[1] {
        DisplayCommand::ShowBalloon { scope, pattern, .. } => {
            assert_eq!(scope, &s1);
            assert_eq!(
                pattern.get(1).expect("scope1 anim1 の現在コマ").surface_id,
                4000,
                "scope1 は scope1 の表のコマ＝scope0 の表とは独立に引かれる（要件 5.6）"
            );
        }
        other => panic!("ShowBalloon を期待: {other:?}"),
    }
}

/// 表を持たない scope のバルーンは**空表意味論＝不活性**（抽選対象ゼロ・**乱数非消費**・panic なし・
/// 要件 5.6）。他 scope が表を持っていても、その表が不在 scope へ流用されることはない。
#[test]
fn absent_scope_balloon_table_is_inert() {
    // 表は scope "0" だけが持ち、表示するのは表を持たない scope "1" のバルーンのみ。
    let t0 = table_single(20, 1, Interval::Random { k: 4 }, &[(3000, 0)]);
    let (rng, probe) = counting_rng(&[0, 0]); // もし引かれれば発火してしまう値
    let mut rt = LoopRuntime::new(SerikoLoopConfig {
        shell_table: AnimationTable::empty(),
        balloon_tables: BTreeMap::from([(ActorKey::from("0"), t0)]),
        rng,
    });
    let mut states = ScopeStates::new(BindSet::from_ids([]));
    let s1 = ActorKey::from("1");
    states.apply_balloon(&s1, SurfaceTarget::Show(20));

    rt.on_tick(0, &mut states);
    assert!(
        rt.on_tick(1000, &mut states).is_empty(),
        "不在 scope は抽選対象ゼロ＝無発行（要件 5.6）"
    );
    assert_eq!(
        probe.lock().unwrap().calls,
        0,
        "不在 scope は乱数を消費しない（空表意味論・要件 5.6）"
    );
    // さらに境界を跨いでも不活性のまま（panic せず無発行・非消費）。
    assert!(rt.on_tick(2000, &mut states).is_empty());
    assert_eq!(probe.lock().unwrap().calls, 0);
}

// ── on_surface_changed / 単調性ガード ──────────────────────────────────

/// `on_surface_changed` は当該 slot の playback を全除去する（surface 従属・要件 2.3）。
/// 除去後は再抽選対象へ戻る（rng が再消費される）。
#[test]
fn on_surface_changed_removes_playback() {
    let table = table_single(10, 0, Interval::Random { k: 4 }, &[(2106, 0), (2110, 150)]);
    let (rng, probe) = counting_rng(&[0, 0]);
    let mut rt = LoopRuntime::new(cfg(table, rng));
    let (mut states, scope) = shown_shell(10);

    rt.on_tick(0, &mut states);
    rt.on_tick(1000, &mut states); // 発火・再生中（rng 1 回）
    // surface 切替連動で playback 除去。
    rt.on_surface_changed(&scope, Slot::Shell);
    // 次の境界で再抽選できる（playback が消えたので Idle）＝rng 再消費。
    rt.on_tick(2000, &mut states);
    assert_eq!(
        probe.lock().unwrap().calls,
        2,
        "on_surface_changed 後は Idle ゆえ再抽選（rng 再消費）"
    );
}

/// 非単調 tick（now < 前回）は無視され、状態を変えず空を返す（rng も消費しない・防御・要件 1.2）。
#[test]
fn non_monotonic_tick_is_ignored() {
    let table = table_single(10, 0, Interval::Random { k: 4 }, &[(2106, 0)]);
    let (rng, probe) = counting_rng(&[0]);
    let mut rt = LoopRuntime::new(cfg(table, rng));
    let (mut states, _scope) = shown_shell(10);

    rt.on_tick(1000, &mut states); // 初回（遅延初期化・境界 next=2000）
    // 過去へ戻る tick は無視。
    let cmds = rt.on_tick(500, &mut states);
    assert!(cmds.is_empty(), "非単調 tick は無発行");
    // 抽選も走らない＝rng 非消費（初回 1000 は境界跨ぎしない＝next は 2000）。
    assert_eq!(probe.lock().unwrap().calls, 0, "非単調 tick は rng を消費しない");
}

/// bind 書込 API は一切呼ばない（read-only）: on_tick 前後で current_binds が不変であることの反証。
#[test]
fn on_tick_never_writes_binds() {
    let table = table_single(10, 7, Interval::BindRandom { k: 4 }, &[(1412, 0)]);
    let mut rt = LoopRuntime::new(cfg(table, always_fire()));
    let (mut states, scope) = shown_shell_with_binds(10, BindSet::from_ids([7]));

    let before = states.current_binds(&scope).clone();
    rt.on_tick(0, &mut states);
    rt.on_tick(1000, &mut states); // 発火（BindRandom ON）
    let after = states.current_binds(&scope).clone();
    assert_eq!(before, after, "on_tick は bind を書き込まない（read-only・要件 3.3）");
}

/// `SerikoLoopConfig::disabled()` は空表＋ダミー乱数でループ完全不活性（非退行・常に無発行）。
#[test]
fn disabled_config_is_inert() {
    let mut rt = LoopRuntime::new(SerikoLoopConfig::disabled());
    let (mut states, _scope) = shown_shell(10);
    assert!(rt.on_tick(0, &mut states).is_empty());
    assert!(rt.on_tick(1000, &mut states).is_empty(), "空表は境界跨ぎでも無発行");
    assert!(rt.on_tick(5000, &mut states).is_empty());
}

/// 変化なし tick は無発行（冪等・要件 6.2）: 再生中でも同一コマが続く間は指令を返さない。
#[test]
fn unchanged_tick_emits_nothing() {
    // 先頭コマが長く続く（w0 の 2106 が 150ms 継続）。
    let table = table_single(10, 0, Interval::Random { k: 4 }, &[(2106, 0), (2110, 150)]);
    let mut rt = LoopRuntime::new(cfg(table, always_fire()));
    let (mut states, _scope) = shown_shell(10);

    rt.on_tick(0, &mut states);
    let c1 = rt.on_tick(1000, &mut states); // 発火・2106
    assert_eq!(c1.len(), 1);
    // 同じ 2106 が続く中間 tick（境界も跨がない）は無発行。
    assert!(rt.on_tick(1050, &mut states).is_empty(), "同一コマ継続は無発行（要件 6.2）");
    assert!(rt.on_tick(1100, &mut states).is_empty());
}

// ── 進行相の bind 判定（再生中に外れた ID の停止・bindopt 7.3/7.4/7.5・D9-2） ──

/// 再生**途中**（末尾未到達）に bind から外した ID は、次の評価でコマも再生も消え、以後**復活しない**
/// （bindopt 7.3・D9-2）。
///
/// 実機の固着そのものの機構: 状態側の除去（`commit_bind` の `drop_residual_frames`・bindopt 7.1）だけでは
/// 「再生中に外れた」場合を塞げない——playback が残る限り次 tick の進行相が `frame_at` の結果でコマを
/// 置き直すため。進行相にも bind 判定が要る。
#[test]
fn playing_bind_anim_removed_from_binds_stops_and_does_not_revive() {
    // 1412(w0)/1411(w150)/1410(w22) ⇒ t=[0,150,172]。elapsed<150 は Active（末尾未到達＝再生途中）。
    let table = table_single(10, 7, Interval::BindRandom { k: 4 }, &[(1412, 0), (1411, 150), (1410, 22)]);
    let (rng, probe) = counting_rng(&[0, 0]);
    let mut rt = LoopRuntime::new(cfg(table, rng));
    let (mut states, scope) = shown_shell_with_binds(10, BindSet::from_ids([7]));

    rt.on_tick(0, &mut states);
    let c1 = rt.on_tick(1000, &mut states); // 発火 → 先頭コマ 1412（再生中）
    assert_eq!(pattern_of(&c1[0]).get(7).unwrap().surface_id, 1412, "再生中の先頭コマ");

    // 再生途中で bind から外す（状態側 bindopt 7.1 が保持コマをここで取り除く）。
    states.apply_bind(&scope, 7, false);
    assert!(
        states.current_pattern(&scope, Slot::Shell).get(7).is_none(),
        "状態側の除去（bindopt 7.1）で保持コマは一旦消える"
    );

    // 次の評価（境界を跨がない tick でも進行相は走る）。進行相に bind 判定が無いとここで復活する。
    let c2 = rt.on_tick(1100, &mut states);
    assert!(
        states.current_pattern(&scope, Slot::Shell).get(7).is_none(),
        "bind から外れた ID のコマは次の評価で復活しない（bindopt 7.3）"
    );
    assert!(c2.is_empty(), "復活しない＝pattern 不変ゆえ無発行（要件 6.2）");

    // playback も除去されている＝Idle へ戻っている反証: bind を戻すと次の境界で再抽選対象になる
    // （再生中のままなら再抽選されず rng は消費されない・要件 2.3）。
    states.apply_bind(&scope, 7, true);
    rt.on_tick(2000, &mut states); // 境界跨ぎ
    assert_eq!(
        probe.lock().unwrap().calls,
        2,
        "停止相当で playback も除去＝再抽選対象へ戻る（bindopt 7.3）"
    );
}

/// bind 種でないアニメ（純 `Random`）は進行相の bind 判定の影響を受けない（bindopt 7.4）。
///
/// 同一 surface に bind 種（id 7・BindRandom）と非 bind 種（id 9000・Random）を並べ、id 7 だけを
/// bind から外す。id 7 は停止するが、id 9000 は再生を続け末尾残留コマまで到達する。
#[test]
fn non_bind_anim_is_unaffected_by_progress_phase_bind_check() {
    let s = surface_with(
        10,
        vec![
            Animation {
                id: 7,
                interval: Interval::BindRandom { k: 4 },
                // 1412(w0)/1411(w150)/1410(w22) ⇒ t=[0,150,172]。
                patterns: vec![pat(0, 1412, 0), pat(1, 1411, 150), pat(2, 1410, 22)],
            },
            Animation {
                id: 9000,
                interval: Interval::Random { k: 4 },
                // 2106(w0)/2110(w150) ⇒ t=[0,150]。末尾非負＝到達後も残留する。
                patterns: vec![pat(0, 2106, 0), pat(1, 2110, 150)],
            },
        ],
    );
    let mut rt = LoopRuntime::new(cfg(shell_table(vec![s]), always_fire()));
    let (mut states, scope) = shown_shell_with_binds(10, BindSet::from_ids([7]));

    rt.on_tick(0, &mut states);
    let c1 = rt.on_tick(1000, &mut states); // 両方発火（id 7 は bind ON・id 9000 は無条件）
    let p1 = pattern_of(&c1[0]);
    assert_eq!(p1.get(7).unwrap().surface_id, 1412);
    assert_eq!(p1.get(9000).unwrap().surface_id, 2106);

    // bind 種の id 7 だけを外す（id 9000 は bind 集合に属さないアニメ）。
    states.apply_bind(&scope, 7, false);

    // 経過 200ms（1200）: id 7 は停止、id 9000 は末尾 2110 へ進み残留する。
    let c2 = rt.on_tick(1200, &mut states);
    assert_eq!(c2.len(), 1, "id 9000 のコマ進行で 1 指令");
    let p2 = pattern_of(&c2[0]);
    assert!(p2.get(7).is_none(), "bind から外れた bind 種は復活しない（bindopt 7.3）");
    assert_eq!(
        p2.get(9000).unwrap().surface_id,
        2110,
        "bind 非所属アニメは進行相の bind 判定に影響されない（bindopt 7.4）"
    );

    // さらに時間が進んでも id 9000 の残留は保たれる（末尾非負＝IdleResidual・要件 4.4）。
    assert!(rt.on_tick(1500, &mut states).is_empty(), "残留は恒久＝無発行");
    assert_eq!(
        states.current_pattern(&scope, Slot::Shell).get(9000).unwrap().surface_id,
        2110,
        "bind 非所属アニメの保持コマは除去されない（bindopt 7.4）"
    );
}

/// 進行相の停止は `info!` で痕跡を残す（bindopt 7.5・無言の状態変更を作らない）。文言と水準を固定する。
#[test]
fn progress_phase_bind_drop_emits_info_marker() {
    let table = table_single(10, 7, Interval::BindRandom { k: 4 }, &[(1412, 0), (1411, 150), (1410, 22)]);
    let mut rt = LoopRuntime::new(cfg(table, always_fire()));
    let (mut states, scope) = shown_shell_with_binds(10, BindSet::from_ids([7]));

    rt.on_tick(0, &mut states);
    rt.on_tick(1000, &mut states); // 発火・再生中
    states.apply_bind(&scope, 7, false);

    let logs = capture_logs(|| {
        rt.on_tick(1100, &mut states);
    });

    assert!(
        logs.contains("level=INFO") && logs.contains("seriko: loop bind から外れた ID の再生を停止"),
        "進行相の停止は実機の既定ログ水準（info）で grep 可能な固定文言を残す（bindopt 7.5）: {logs}"
    );
    assert!(
        logs.contains("animation_id=7"),
        "停止した ID を痕跡に載せる（bindopt 7.5）: {logs}"
    );
}

/// bind 種でも bind 集合に属したまま再生が続く間は停止しない（誤検出の不在・bindopt 7.3 の裏側）。
///
/// **不在主張には同一捕捉内の陽性対照を併置する**（`actor_dispatch_tests` の既存流儀）。捕捉区間に
/// 抽選境界を跨ぐ tick を含め、健全系では `loop 抽選発火` の痕跡が**必ず出る**ことを同時に主張する
/// ——捕捉が空でも通ってしまう恒真の檻にしないため。あわせて状態側（保持コマの残存）でも反証し、
/// ログ経路が壊れても判別能力が残るようにする。進行相のガード条件を反転させる変異
/// （`!contains` → `contains`）では、発火直後の進行相で id 7 が落とされ、停止痕跡の出現と
/// コマの消失の**両方**でこの檻が赤になる。
#[test]
fn progress_phase_emits_no_drop_marker_while_bind_holds() {
    let table = table_single(10, 7, Interval::BindRandom { k: 4 }, &[(1412, 0), (1411, 150), (1410, 22)]);
    let mut rt = LoopRuntime::new(cfg(table, always_fire()));
    let (mut states, scope) = shown_shell_with_binds(10, BindSet::from_ids([7]));

    rt.on_tick(0, &mut states);
    // 捕捉区間に境界跨ぎ（＝発火）を含める。bind は終始 ON のまま。
    let logs = capture_logs(|| {
        rt.on_tick(1000, &mut states); // 抽選発火 → 進行相（bind 所属のまま）
        rt.on_tick(1100, &mut states); // 再生継続（Active・末尾未到達）
    });

    // 陽性対照: この捕捉には現に痕跡が出ている（空捕捉に対する恒真の不在主張を禁じる）。
    assert!(
        logs.contains("seriko: loop 抽選発火"),
        "陽性対照: bind ON の発火痕跡が同一捕捉に現れる（捕捉が空でないことの反証）: {logs}"
    );
    // 不在主張: bind 所属のまま再生中の ID は停止相当へ落ちない。
    assert!(
        !logs.contains("seriko: loop bind から外れた ID の再生を停止"),
        "bind 所属のまま再生中の ID は停止しない（bindopt 7.3 の裏側）: {logs}"
    );
    // 状態側の反証（ログ経路に依らない判別）: 再生中のコマが保たれている。
    assert_eq!(
        states.current_pattern(&scope, Slot::Shell).get(7).map(|f| f.surface_id),
        Some(1412),
        "bind 所属のまま再生は継続し保持コマも保たれる（bindopt 7.3 の裏側）"
    );
}

// ── 補助 ────────────────────────────────────────────────────────────────

/// テスト専用 tracing 捕捉ハーネス（state/actor/table の同名ヘルパと同一流儀・スレッドローカル
/// `with_default` ゆえ並行テスト安全）。1 イベント 1 行へ level／target／各フィールド
/// （`name=value`）を整形し、改行連結で返す。
fn capture_logs<F: FnOnce()>(f: F) -> String {
    use tracing::field::{Field, Visit};
    use tracing_subscriber::prelude::*;

    #[derive(Clone, Default)]
    struct Capture(Arc<Mutex<Vec<String>>>);

    impl<S: tracing::Subscriber> tracing_subscriber::Layer<S> for Capture {
        fn on_event(&self, ev: &tracing::Event<'_>, _: tracing_subscriber::layer::Context<'_, S>) {
            let meta = ev.metadata();
            let mut line = format!("level={} target={}", meta.level(), meta.target());
            struct V<'a>(&'a mut String);
            impl Visit for V<'_> {
                fn record_debug(&mut self, f: &Field, v: &dyn std::fmt::Debug) {
                    use std::fmt::Write;
                    let _ = write!(self.0, " {}={:?}", f.name(), v);
                }
            }
            ev.record(&mut V(&mut line));
            self.0.lock().unwrap().push(line);
        }
    }

    // 並行実行下の callsite interest 毒化対策（`log_interest_probe` のモジュール doc 参照）。
    crate::log_interest_probe::ensure_interest_probes();

    let cap = Capture::default();
    let logs = cap.0.clone();
    let subscriber = tracing_subscriber::registry().with(cap);
    tracing::subscriber::with_default(subscriber, || {
        // probe 常駐前に焼かれた `never` の掃き残しを、窓が開いた後にもう一度潰す。
        tracing::callsite::rebuild_interest_cache();
        f()
    });
    let guard = logs.lock().unwrap();
    guard.join("\n")
}

fn pattern_of(cmd: &DisplayCommand) -> &PatternState {
    match cmd {
        DisplayCommand::Show { pattern, .. } | DisplayCommand::ShowBalloon { pattern, .. } => {
            pattern
        }
        other => panic!("pattern を持つ指令を期待: {other:?}"),
    }
}

//! SERIKO ループ統合テスト（task 10.1）— アクター全経路の決定論檻（要件 6.2/7.2/7.3/9.4）。
//!
//! looper.rs の単体テストは `LoopRuntime::on_tick` を**直接**呼んで純関数コアの分岐を檻に入れる。
//! 本ファイルはそれと**別の観測面**——seriko アクター**全体**を `SerikoSink::send_tick` と
//! `CueSink::emit`（表示中 surface の設定）で駆動し、`SerikoMsg::Tick` → `handle_message` Tick 腕 →
//! `LoopRuntime::on_tick` → `emit_display` 単一発行点 → `MockSurfaceOutput` の全経路を貫通させる。
//! 固定注入 tick 列＋固定注入乱数列に対して、`MockSurfaceOutput` の**発行 `DisplayCommand` 列**（各
//! `Show` が同梱する `PatternState` 込み）が期待列と**順序・全値で完全一致**することを主張する（7.2）。
//!
//! # 同期（sleep/polling ゼロ・要件 7.3）
//!
//! 全ステップ（cue と tick）は seriko の単一 inbox を FIFO 共有する。送り終えたら `close()`→`join()`
//! の 1 点のみで同期する。`join` 復帰時点で全ステップは単一スレッドで処理済みゆえ、`records()` の
//! 発行列を決定論的に照合できる（既存 cue_sequence.rs/bind_e2e.rs と同一技法）。
//!
//! # 表の構築（looper 単体・table.rs と同一経路）
//!
//! アニメ表は合成 `EmoWorld`（`Surface`＋`Animation`＋`Pattern`）から `AnimationTable::from_world`
//! で構築する（実構築パスを通す）。kero 型（末尾 `-1`）・sakura 型（末尾非負・bind ゲート）の実形と、
//! 討議 #2 の「先頭 wait>0」合成形、`-2`（他負 surface）を注入して全経路を網羅する。
//!
//! # 網羅する 9 檻
//!
//! 1. kero 型: 発火→先頭コマ→…→`-1`→停止→ベース復帰（PatternState 空へ）。
//! 2. sakura 型: bind ON→発火→フレーム→末尾非負残留（残留保持・playback 除去）。
//! 3. 再生中の非再抽選（R2.3）: 再生中の境界跨ぎで再抽選しない（乱数非消費）。
//! 4. bind OFF 乱数非消費（R3.1）: bind+random で bindgroup OFF は乱数を 1 度も消費せず無発行。
//! 5. bind ON 発火（R3.2）: 同一アニメが bindgroup ON で発火。
//! 6. 変化なし tick 無発行（R6.2）: PatternState 不変の tick は発行しない。
//! 7. surface 切替でコマ消滅: 表示 surface 切替で当該 slot の playback＋PatternState がクリアされ、
//!    後続 tick が陳腐コマを蘇生させない。
//! 8. 残留→再発火の即時クリア（討議 #2）: IdleResidual が再発火した瞬間に残留コマが即消えベース復帰し、
//!    先頭コマは自身のデッドラインで現れる。
//! 9. `-2`（他負 surface）warn-once（5.3 の 10.1 繰り延べ）: `-2` コマで自アニメのみ停止（`-1` と同様
//!    ベース復帰）し `warn!` が **1 回だけ**発火（2 度目の走査では dedup で再 warn しない）。かつ `-2` は
//!    **他アニメを停止させない**（R8.2）——同時再生の別アニメが停止を跨いで生存し続けることを発行列で示す。

use std::collections::{BTreeMap, VecDeque};
use std::sync::{Arc, Mutex};

use areka_emo_compose::{BindSet, ComposeMethod, EmoWorld, PatternFrame, PatternState};
use areka_parsers::shell::{
    Animation, AppendTarget, DefRef, DrawMethod, Interval, Pattern, Shell, Surface,
};
use areka_sakura::{ActorKey, CueCommand, CueSink, TalkCue};
use areka_seriko::{
    AnimationTable, BindResolver, DisplayCommand, LoopRng, MockSurfaceOutput, SerikoLoopConfig,
    SurfaceResolver, spawn_seriko,
};

// ─────────────────────────────────────────────────────────────────────────────
// 駆動ハーネス（cue と tick を FIFO 順に注入・Close→join 同期・sleep なし）
// ─────────────────────────────────────────────────────────────────────────────

/// 1 ステップ（表示 surface を確定する cue、または時間駆動 tick）。順序どおり単一 inbox へ注入する。
enum Step {
    Cue(TalkCue),
    Tick(u64),
}

/// 表示中 surface を確定する Shell 系 Emote cue（数値 key は数値枝で直接解決＝空 alias 表で足りる）。
fn emote(scope: &str, key: &str) -> Step {
    Step::Cue(TalkCue {
        at: 0.0,
        actor: ActorKey::from(scope),
        command: CueCommand::Emote { key: key.into() },
        duration: 0.0,
    })
}

/// アクターを起動し `steps` を FIFO 順に注入、`Close`→`join` で同期して全発行列を返す（sleep なし）。
///
/// `static_binds` は各 scope の既定 bind 集合（BindRandom ゲート源＝Show 同梱 binds）。`loop_config`
/// は実表＋注入乱数を運ぶ。resolver は空 alias 表（数値 key のみ使用）・bind resolver は空（`\![bind]`
/// を使わない）。
fn run(
    static_binds: BindSet,
    loop_config: SerikoLoopConfig,
    steps: Vec<Step>,
) -> Vec<DisplayCommand> {
    let out = MockSurfaceOutput::new();
    let records = out.records();
    let (mut sink, handle) = spawn_seriko(
        SurfaceResolver::new(BTreeMap::new()),
        static_binds,
        BindResolver::empty(),
        loop_config,
        out,
    );
    for step in steps {
        match step {
            Step::Cue(cue) => CueSink::emit(&mut sink, cue),
            Step::Tick(now) => sink.send_tick(now),
        }
    }
    sink.close().expect("Close を送れること");
    handle.join().expect("Close で正常終了する");
    let recorded = records.lock().expect("records mutex poisoned");
    recorded.clone()
}

// ─────────────────────────────────────────────────────────────────────────────
// 期待値ビルダ（Show / PatternState）
// ─────────────────────────────────────────────────────────────────────────────

/// overlay 固定・x/y=0 の現在コマ（合成テストと同一・emo2 実測は全 overlay）。
fn overlay(surface_id: u32) -> PatternFrame {
    PatternFrame {
        surface_id,
        method: ComposeMethod::Overlay,
        x: 0,
        y: 0,
    }
}

/// `(anim_id, surface_id)` 列から PatternState を組む（overlay 固定）。
fn pattern(frames: &[(u32, u32)]) -> PatternState {
    let mut p = PatternState::default();
    for &(id, sid) in frames {
        p.set(id, overlay(sid));
    }
    p
}

/// Shell 面 Show 指令の期待値（scope・surface・binds・pattern を全指定）。
fn show(scope: &str, surface_id: u32, binds: &BindSet, pattern: PatternState) -> DisplayCommand {
    DisplayCommand::Show {
        scope: ActorKey::from(scope),
        surface_id,
        binds: binds.clone(),
        pattern,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// 表ビルダ（table.rs / looper.rs 単体テストと同一の from_world 経路）
// ─────────────────────────────────────────────────────────────────────────────

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

/// `(surface_id, wait)` 列から 1 アニメを組む。
fn anim(id: u32, interval: Interval, frames: &[(i64, u32)]) -> Animation {
    let patterns = frames
        .iter()
        .enumerate()
        .map(|(i, (sid, wait))| pat(i as u32, *sid, *wait))
        .collect();
    Animation {
        id,
        interval,
        patterns,
    }
}

/// アニメ列を持つ surface 1 件。
fn surface_with(id: u32, animations: Vec<Animation>) -> Surface {
    Surface {
        id,
        targets: vec![AppendTarget::Single(id)],
        elements: Vec::new(),
        collisions: Vec::new(),
        animations,
    }
}

/// surface 列から shell 表を build する（from_world 実経路）。
fn shell_table(surfaces: Vec<Surface>) -> AnimationTable {
    let definitions = (0..surfaces.len()).map(DefRef::Surface).collect();
    let shell = Shell {
        surfaces,
        appends: Vec::new(),
        aliases: Vec::new(),
        animation_sort: None,
        collision_sort: None,
        definitions,
    };
    AnimationTable::from_world(&EmoWorld::build(&shell))
}

/// 単一 anim を持つ surface 1 件から shell 表を build する。
fn table_single(
    surface_id: u32,
    anim_id: u32,
    interval: Interval,
    frames: &[(i64, u32)],
) -> AnimationTable {
    shell_table(vec![surface_with(
        surface_id,
        vec![anim(anim_id, interval, frames)],
    )])
}

/// shell 表＋注入乱数から config を組む（バルーン表の写像は空＝emo2 データ事実・全 scope 不活性）。
fn cfg(shell_table: AnimationTable, rng: LoopRng) -> SerikoLoopConfig {
    SerikoLoopConfig {
        shell_table,
        balloon_tables: BTreeMap::new(),
        rng,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// 注入乱数（消費回数を計数する列 rng・bind OFF 非消費の檻に使う）
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Default)]
struct RngProbe {
    calls: usize,
    values: VecDeque<u32>,
}

/// 指定した値列を順に返し、呼ばれた回数を計数する rng（尽きたら 1＝非発火）。返す `Arc` から
/// 消費回数を照合できる（bind OFF での**非消費**・再生中の**非再抽選**の檻・要件 3.1/2.3）。
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

// ─────────────────────────────────────────────────────────────────────────────
// 檻 1: kero 型（末尾 `-1`）— 発火→フレーム進行→停止→ベース復帰（PatternState 空へ）
// ─────────────────────────────────────────────────────────────────────────────

/// kero 型（`interval,random,N`・末尾 `-1`）: 発火で先頭コマから再生し、`-1` 到達でコマ除去＋
/// ベース復帰（空 PatternState の Show）する。固定 tick 列に対する発行列＋各 PatternState を完全一致で檻る
/// （要件 2.2/4.1/4.3/6.1/7.2）。
#[test]
fn kero_negative_tail_restores_base_full_path() {
    let empty = BindSet::from_ids([]);
    // 2106(w0)/2110(w40)/-1(w80) ⇒ t=[0,40,120]。Random は無条件ゲート。
    let table = table_single(
        10,
        0,
        Interval::Random { k: 4 },
        &[(2106, 0), (2110, 40), (-1, 80)],
    );
    let (rng, probe) = counting_rng(&[0]); // 最初の境界で 1 回発火

    let records = run(
        empty.clone(),
        cfg(table, rng),
        vec![
            emote("0", "10"), // 表示 surface 10 を確定（初回 Show・空 pattern）
            Step::Tick(0),    // 遅延初期化（境界 next=1000）・非跨ぎ・無発行
            Step::Tick(1000), // 境界跨ぎ＋発火・elapsed0 → 先頭コマ 2106
            Step::Tick(1040), // elapsed40 → 2110
            Step::Tick(1120), // elapsed120 → -1 → 停止・ベース復帰（空 pattern）
        ],
    );

    assert_eq!(
        records,
        vec![
            show("0", 10, &empty, PatternState::default()), // Emote 初回 Show
            show("0", 10, &empty, pattern(&[(0, 2106)])),   // 発火・先頭コマ
            show("0", 10, &empty, pattern(&[(0, 2110)])),   // 次コマ
            show("0", 10, &empty, PatternState::default()), // -1 停止でベース復帰
        ],
        "kero 型は発火→2106→2110→(-1)ベース復帰の発行列と PatternState 列が完全一致すること（4.3）"
    );
    assert_eq!(
        probe.lock().unwrap().calls,
        1,
        "発火判定は最初の境界で 1 回のみ（-1 到達までは再生中で再抽選しない）"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// 檻 2: sakura 型（末尾非負・bind ゲート）— bind ON 発火→フレーム→末尾残留
// ─────────────────────────────────────────────────────────────────────────────

/// sakura 型（`interval,bind+random,N`・末尾非負）: bindgroup ON で発火し、末尾コマ到達で最終コマを
/// **残したまま** playback 除去（IdleResidual）する。残留は恒久（後続 tick は無発行）。固定 tick 列の
/// 発行列＋PatternState を完全一致で檻る（要件 3.2/4.4/6.2/9.4）。
#[test]
fn sakura_residual_tail_keeps_frame_full_path() {
    let on = BindSet::from_ids([7]); // anim id 7 の bindgroup を ON（BindRandom ゲート通過）
    // 1412(w0)/1411(w150)/1410(w22) ⇒ t=[0,150,172]。末尾非負＝残留。
    let table = table_single(
        10,
        7,
        Interval::BindRandom { k: 4 },
        &[(1412, 0), (1411, 150), (1410, 22)],
    );
    let (rng, _probe) = counting_rng(&[0]);

    let records = run(
        on.clone(),
        cfg(table, rng),
        vec![
            emote("0", "10"),
            Step::Tick(0),
            Step::Tick(1000), // 発火 → 1412
            Step::Tick(1150), // elapsed150 → 1411
            Step::Tick(1172), // elapsed172 → 末尾 1410 残留（playback 除去）
            Step::Tick(1500), // 残留は恒久＝無発行
        ],
    );

    assert_eq!(
        records,
        vec![
            show("0", 10, &on, PatternState::default()), // Emote 初回 Show（binds={7}）
            show("0", 10, &on, pattern(&[(7, 1412)])),   // 発火・先頭コマ
            show("0", 10, &on, pattern(&[(7, 1411)])),   // 次コマ
            show("0", 10, &on, pattern(&[(7, 1410)])),   // 末尾非負＝残留
        ],
        "sakura 型は bind ON 発火→1412→1411→1410 残留で終わり、以降 tick は無発行（残留恒久・4.4/6.2）"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// 檻 3: 再生中の非再抽選（R2.3）— 再生中の境界跨ぎで再抽選しない（乱数非消費）
// ─────────────────────────────────────────────────────────────────────────────

/// 再生中（先頭コマが長く継続）のまま次の 1000ms 境界を跨いでも、当該アニメは再抽選対象外＝
/// **乱数を追加消費しない**（要件 2.3）。同一コマ継続ゆえ発行も増えない（要件 6.2）。
#[test]
fn playing_anim_not_relotteried_across_boundary_full_path() {
    let empty = BindSet::from_ids([]);
    // 2106(w0)/2110(w3000) ⇒ 先頭 2106 が 3000ms 継続（境界 2000 でもまだ Active(0)）。
    let table = table_single(10, 0, Interval::Random { k: 4 }, &[(2106, 0), (2110, 3000)]);
    let (rng, probe) = counting_rng(&[0]); // 発火は最初の境界のみ。再抽選が起きれば追加消費が観測される。

    let records = run(
        empty.clone(),
        cfg(table, rng),
        vec![
            emote("0", "10"),
            Step::Tick(0),
            Step::Tick(1000), // 発火（rng 1 回）→ 2106
            Step::Tick(2000), // 境界跨ぎだが再生中ゆえ再抽選しない・同一コマ継続で無発行
        ],
    );

    assert_eq!(
        records,
        vec![
            show("0", 10, &empty, PatternState::default()),
            show("0", 10, &empty, pattern(&[(0, 2106)])),
        ],
        "再生中の境界跨ぎ tick は同一コマ継続で無発行（発行列は初回 Show＋発火コマの 2 件のみ・2.3/6.2）"
    );
    assert_eq!(
        probe.lock().unwrap().calls,
        1,
        "再生中アニメは 2000ms 境界で再抽選されず乱数は追加消費されない（要件 2.3）"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// 檻 4: bind OFF は乱数非消費（R3.1）
// ─────────────────────────────────────────────────────────────────────────────

/// `bind+random` で bindgroup が OFF のとき、境界を跨いでも judgment そのものが不発＝
/// **乱数を 1 度も消費しない**（要件 3.1・CRITICAL）。発行は初回 Show のみ。
#[test]
fn bindrandom_off_consumes_no_rng_full_path() {
    let empty = BindSet::from_ids([]); // anim id 7 の bindgroup は不在＝OFF
    let table = table_single(10, 7, Interval::BindRandom { k: 4 }, &[(1412, 0)]);
    let (rng, probe) = counting_rng(&[0, 0, 0]); // もし消費されれば発火してしまう値

    let records = run(
        empty.clone(),
        cfg(table, rng),
        vec![
            emote("0", "10"),
            Step::Tick(0),
            Step::Tick(1000), // 境界跨ぎでも bind OFF ゆえ should_fire 未呼出
            Step::Tick(2000),
        ],
    );

    assert_eq!(
        records,
        vec![show("0", 10, &empty, PatternState::default())],
        "bind OFF は発火せず初回 Show のみ（無発行・R3.1）"
    );
    assert_eq!(
        probe.lock().unwrap().calls,
        0,
        "bind OFF はゲート不通過で should_fire を呼ばず乱数を消費しない（要件 3.1）"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// 檻 5: bind ON で発火（R3.2）
// ─────────────────────────────────────────────────────────────────────────────

/// 檻 4 と同一アニメが bindgroup ON なら発火し、先頭コマ（単一コマ＝即残留）を発行する。乱数は
/// 1 回だけ消費される（要件 3.2）。
#[test]
fn bindrandom_on_fires_full_path() {
    let on = BindSet::from_ids([7]);
    let table = table_single(10, 7, Interval::BindRandom { k: 4 }, &[(1412, 0)]);
    let (rng, probe) = counting_rng(&[0]);

    let records = run(
        on.clone(),
        cfg(table, rng),
        vec![emote("0", "10"), Step::Tick(0), Step::Tick(1000)],
    );

    assert_eq!(
        records,
        vec![
            show("0", 10, &on, PatternState::default()),
            show("0", 10, &on, pattern(&[(7, 1412)])),
        ],
        "bind ON は発火し先頭コマ 1412 を発行（R3.2）"
    );
    assert_eq!(
        probe.lock().unwrap().calls,
        1,
        "bind ON では乱数を 1 回だけ消費（R3.2）"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// 檻 6: 変化なし tick は無発行（R6.2）
// ─────────────────────────────────────────────────────────────────────────────

/// PatternState が変化しない tick（境界も跨がず同一コマ継続）は単一発行点から発行しない（要件 6.2）。
#[test]
fn unchanged_tick_emits_nothing_full_path() {
    let empty = BindSet::from_ids([]);
    // 先頭 2106 が 150ms 継続（中間 tick は同一コマ）。
    let table = table_single(10, 0, Interval::Random { k: 4 }, &[(2106, 0), (2110, 150)]);

    let records = run(
        empty.clone(),
        cfg(table, always_fire()),
        vec![
            emote("0", "10"),
            Step::Tick(0),
            Step::Tick(1000), // 発火 → 2106
            Step::Tick(1050), // 同一コマ・境界非跨ぎ → 無発行
            Step::Tick(1100), // 同一コマ → 無発行
        ],
    );

    assert_eq!(
        records,
        vec![
            show("0", 10, &empty, PatternState::default()),
            show("0", 10, &empty, pattern(&[(0, 2106)])),
        ],
        "変化なし tick（1050/1100）は無発行＝発行列は初回 Show＋発火コマの 2 件のみ（要件 6.2）"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// 檻 7: surface 切替でコマ消滅（on_surface_changed リセット）
// ─────────────────────────────────────────────────────────────────────────────

/// 再生中に表示 surface を切り替えると、当該 slot の playback＋PatternState がクリアされ、後続 tick が
/// 陳腐コマを蘇生させない。切替 Show は空 pattern・以降のコマ発行は起きない（surface 従属・要件 2.3）。
#[test]
fn surface_switch_clears_playback_and_frame_full_path() {
    let empty = BindSet::from_ids([]);
    // surface 10 に長い再生アニメ（境界 2000 でも Active(0)）。surface 20 にはアニメなし。
    let table = shell_table(vec![
        surface_with(
            10,
            vec![anim(
                0,
                Interval::Random { k: 4 },
                &[(2106, 0), (2110, 3000)],
            )],
        ),
        surface_with(20, Vec::new()),
    ]);

    let records = run(
        empty.clone(),
        cfg(table, always_fire()),
        vec![
            emote("0", "10"), // surface 10 表示（初回 Show）
            Step::Tick(0),
            Step::Tick(1000), // 発火 → 2106（再生中）
            emote("0", "20"), // surface 切替 → pattern クリア＋playback 除去（空 pattern の Show{20}）
            Step::Tick(2000), // surface20 にアニメ無し・陳腐 2106 は蘇生しない → 無発行
        ],
    );

    assert_eq!(
        records,
        vec![
            show("0", 10, &empty, PatternState::default()), // Emote 10
            show("0", 10, &empty, pattern(&[(0, 2106)])),   // 発火コマ
            show("0", 20, &empty, PatternState::default()), // 切替＝空 pattern（コマ消滅）
        ],
        "surface 切替で当該 slot の playback＋PatternState がクリアされ後続 tick は陳腐コマを蘇生させない"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// 檻 8: 残留→再発火の即時クリア（討議 #2）
// ─────────────────────────────────────────────────────────────────────────────

/// 先頭 wait>0 の合成テーブルで、末尾非負の残留状態から再発火した瞬間に残留コマが**即消え**ベース
/// 復帰し（`Pending` 中はエントリなし＝表示は直前 PatternState に依存しない）、先頭コマは自身の
/// デッドラインで改めて現れる（討議 #2 裁定の檻・emo2 実測は wait=0 ゆえ合成 fixture で網羅）。
#[test]
fn residual_immediately_cleared_on_refire_full_path() {
    let empty = BindSet::from_ids([]);
    // 先頭 wait=50>0・末尾非負の 2 コマ: 700(w50)/701(w50) ⇒ t=[50,100]。
    let table = table_single(10, 0, Interval::Random { k: 4 }, &[(700, 50), (701, 50)]);

    let records = run(
        empty.clone(),
        cfg(table, always_fire()),
        vec![
            emote("0", "10"),
            Step::Tick(0),
            Step::Tick(1000), // 発火（started=1000）・elapsed0 → Pending（wait50 未到達）＝無コマ・無発行
            Step::Tick(1100), // elapsed100 → 末尾 701 残留
            Step::Tick(2000), // 再発火（always_fire）・elapsed0 → Pending → 残留 701 即時クリア（ベース復帰）
            Step::Tick(2050), // started=2000＋50 → 先頭コマ 700
        ],
    );

    assert_eq!(
        records,
        vec![
            show("0", 10, &empty, PatternState::default()), // Emote 初回（1000 の Pending は無発行）
            show("0", 10, &empty, pattern(&[(0, 701)])),    // 1100: 末尾 701 残留
            show("0", 10, &empty, PatternState::default()), // 2000: 再発火の瞬間に残留即時クリア＝ベース露出
            show("0", 10, &empty, pattern(&[(0, 700)])),    // 2050: 先頭コマ 700 が deadline で表示
        ],
        "残留は再発火の瞬間に即時クリアされ、表示は frame_at のみで決まる（討議 #2 裁定）"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// 檻 9: `-2`（他負 surface）warn-once ＋ 他アニメ非停止（R8.2）
// ─────────────────────────────────────────────────────────────────────────────

/// 全スレッド横断のログ捕捉（アクタースレッドで発火する warn! を捕捉するためのプロセス広域 subscriber）。
///
/// looper.rs/table.rs 単体は同スレッド `with_default` で捕捉するが、統合テストは warn! が**別スレッド**
/// （アクター）で発火するため、プロセス広域 default subscriber を 1 度だけ設置して横断捕捉する。共有
/// buffer から `-2` warn 行のみを filter するため、並行実行時の他ログ混入は判別に影響しない
/// （`-2` warn を出すのは本檻のみ）。
mod warn_capture {
    use std::sync::{Arc, Mutex, OnceLock};
    use tracing::field::{Field, Visit};
    use tracing_subscriber::prelude::*;

    static BUFFER: OnceLock<Arc<Mutex<Vec<String>>>> = OnceLock::new();

    #[derive(Clone)]
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

    /// プロセス広域 subscriber を 1 度だけ設置し、共有捕捉 buffer を返す。
    pub fn buffer() -> Arc<Mutex<Vec<String>>> {
        BUFFER
            .get_or_init(|| {
                let buf = Arc::new(Mutex::new(Vec::new()));
                let cap = Capture(Arc::clone(&buf));
                let subscriber = tracing_subscriber::registry().with(cap);
                // 本テストバイナリ内で default を設定するのは本モジュールのみ（他檻はログを主張しない）。
                tracing::subscriber::set_global_default(subscriber)
                    .expect("プロセス広域 subscriber を 1 度だけ設置できること");
                buf
            })
            .clone()
    }
}

/// `-2`（他負 surface）コマは自アニメのみ停止（`-1` と同様ベース復帰）し `warn!` を **1 回だけ**発火する
/// （2 度目の `-2` 走査では dedup で再 warn しない）。かつ `-2` は**他アニメを停止させない**——同時再生の
/// 別アニメ（id 1）が id 0 の停止・再発火・再停止を跨いで生存し続けることを発行列で示す（要件 8.2/9.4）。
///
/// 5.3 で timeline に繰り延べた `-2` の warn-once 分岐を、本統合檻がアクター全経路で確定させる。
#[test]
fn other_negative_surface_warns_once_and_spares_others_full_path() {
    let buffer = warn_capture::buffer(); // 設置はアクター起動前に済ませる（別スレッド warn! を捕捉）

    let empty = BindSet::from_ids([]);
    // anim id 0: 2106(w0)/-2(w40) ⇒ t=[0,40]。elapsed>=40 で `-2` → Stopped（自アニメ停止・warn once）。
    // anim id 1: 3000(w0)/3001(w5000) ⇒ t=[0,5000]。長く Active(0)=3000 を保つ（id 0 の停止を跨いで生存）。
    let table = shell_table(vec![surface_with(
        10,
        vec![
            anim(0, Interval::Random { k: 4 }, &[(2106, 0), (-2, 40)]),
            anim(1, Interval::Random { k: 4 }, &[(3000, 0), (3001, 5000)]),
        ],
    )]);

    let records = run(
        empty.clone(),
        cfg(table, always_fire()),
        vec![
            emote("0", "10"),
            Step::Tick(0),
            Step::Tick(1000), // 両アニメ発火 → {0:2106, 1:3000}
            Step::Tick(1040), // id0 が -2 で停止（warn 1 回目）・id1 は 3000 生存 → {1:3000}
            Step::Tick(2000), // 境界跨ぎ: id0 再発火（id1 は再生中で非再抽選）→ {0:2106, 1:3000}
            Step::Tick(2040), // id0 が再び -2 で停止（2 度目の走査・dedup で再 warn なし）→ {1:3000}
        ],
    );

    assert_eq!(
        records,
        vec![
            show("0", 10, &empty, PatternState::default()), // Emote 初回
            show("0", 10, &empty, pattern(&[(0, 2106), (1, 3000)])), // 両発火
            show("0", 10, &empty, pattern(&[(1, 3000)])),   // id0 停止・id1 生存（R8.2）
            show("0", 10, &empty, pattern(&[(0, 2106), (1, 3000)])), // id0 再発火・id1 継続
            show("0", 10, &empty, pattern(&[(1, 3000)])),   // id0 再停止・id1 なお生存（R8.2）
        ],
        "`-2` は自アニメのみ停止し他アニメ（id1）は停止を跨いで生存する（R8.2）"
    );

    // warn-once: `-2`（`-1` 以外の負 surface）warn は (scope, slot, anim) ごとに 1 回だけ。id0 は -2 を
    // 2 度走査するが dedup で warn は 1 行のみ。`-2` warn を出すのは本檻だけゆえ全 buffer を filter して数える。
    let negative_warns = buffer
        .lock()
        .unwrap()
        .iter()
        .filter(|line| line.contains("以外の負 surface"))
        .count();
    assert_eq!(
        negative_warns, 1,
        "`-2` の warn! は 2 度の走査でも dedup で 1 回だけ発火する（warn-once・要件 8.2）"
    );
}

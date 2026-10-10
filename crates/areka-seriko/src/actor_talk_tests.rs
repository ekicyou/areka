//! 受け口から口パクまで（spec: areka-P0-seriko-trigger-intervals 要件 2.6・4.8〜4.10・6.3・6.4・9.2・
//! 9.4・9.6・tasks.md 6.1・design「cue の受け口」）。
//!
//! アクターの受け口 `handle_message` へ cue と刻みを届いた順に流し、出てくる指令で確かめる。固定する
//! のは受け口の配線（届いた cue が種類を問わず文字の写しへ渡ること）と、スコープ・表示／非表示の
//! 振り分けだけ。区切りの計算・消去・刈り込みは `looper_talk_tests.rs` が固定している。
//!
//! 時計は手で進める偽物で、どのメッセージもその直前に時計を合わせる（観測を追い越さない）。cue には
//! 台本の秒 `at` を書き、「今」はアクターが届いたときに時計から読む。文字は 1 字 50 ms。刻みは文字の
//! 現れる時刻ちょうどを避けて置く。

use super::test_support::*;
use super::*;
use crate::looper::tests::{always_fire, cfg};
use crate::output::{DisplayCommand, MockSurfaceOutput};
use areka_emo_compose::{BindSet, EmoWorld};
use areka_sakura::{ActorKey, CueCommand, TalkCue};
use dola::cue::CueSink;
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

/// 面 0: 一番上に `talk,3` の口（animation 0＝601 → 40ms で 602 → 40ms で `-1`）。
/// 面 1: 一番上に `talk,3` の口（animation 0＝601 → 40ms で `-1`）と `runonce`（animation 1＝701 →
/// 40ms で `-1`）。シェルの表は全スコープで共有なので、`\0` も `\1` も同じ口を持つ。
const FACES: &str = "surface0\n{\n\
    animation0.interval,talk,3\n\
    animation0.pattern0,overlay,601,0,0,0\n\
    animation0.pattern1,overlay,602,40,0,0\n\
    animation0.pattern2,overlay,-1,40,0,0\n}\n\
    surface1\n{\n\
    animation0.interval,talk,3\n\
    animation0.pattern0,overlay,601,0,0,0\n\
    animation0.pattern1,overlay,-1,40,0,0\n\
    animation1.interval,runonce\n\
    animation1.pattern0,overlay,701,0,0,0\n\
    animation1.pattern1,overlay,-1,40,0,0\n}\n\
    surface601\n{\n}\nsurface602\n{\n}\nsurface701\n{\n}\n";

/// 口の animation の番号。
const MOUTH: u32 = 0;
/// `runonce` の animation の番号（面 1）。
const ONCE: u32 = 1;

fn config() -> SerikoLoopConfig {
    let world = EmoWorld::build(&areka_parsers::shell::parse(FACES));
    cfg(AnimationTable::from_world(&world), always_fire())
}

/// 同期 `handle_message` に cue と刻みを流す足場（統括器に偽の時計を注入する）。
struct Rig {
    resolver: SurfaceResolver,
    bind_resolver: BindResolver,
    states: ScopeStates,
    rt: LoopRuntime,
    out: MockSurfaceOutput,
    records: Arc<Mutex<Vec<DisplayCommand>>>,
    clock: Arc<AtomicU64>,
}

impl Rig {
    fn new() -> Self {
        let clock = Arc::new(AtomicU64::new(0));
        let read = Arc::clone(&clock);
        let out = MockSurfaceOutput::new();
        let records = out.records();
        Self {
            resolver: SurfaceResolver::new(BTreeMap::new()),
            bind_resolver: BindResolver::empty(),
            states: ScopeStates::new(BindSet::from_ids([])),
            rt: LoopRuntime::new(config())
                .with_clock(Some(Arc::new(move || read.load(Ordering::SeqCst)))),
            out,
            records,
            clock,
        }
    }

    /// 時計が `ms` のときに `msg` が届く。そのメッセージで出た指令を返す。
    fn send(&mut self, ms: u64, msg: SerikoMsg) -> Vec<DisplayCommand> {
        self.clock.store(ms, Ordering::SeqCst);
        let before = self.records.lock().unwrap().len();
        let flow = handle_message(
            &self.resolver,
            &self.bind_resolver,
            &mut self.states,
            &mut self.rt,
            &mut self.out,
            msg,
        );
        assert_eq!(flow, ControlFlow::Continue(()));
        self.records.lock().unwrap()[before..].to_vec()
    }

    fn tick(&mut self, ms: u64) -> Vec<DisplayCommand> {
        self.send(ms, SerikoMsg::Tick { now_ms: ms })
    }

    /// 時計が `ms` のときに cue が届く。
    fn hear(&mut self, ms: u64, cue: TalkCue) -> Vec<DisplayCommand> {
        self.send(ms, SerikoMsg::Cue(cue))
    }
}

fn cue(at: f64, scope: &str, command: CueCommand, duration: f64) -> TalkCue {
    TalkCue {
        at,
        actor: ActorKey::from(scope),
        command,
        duration,
    }
}

/// 台本の `at` 秒に始まる文字の cue（1 字 50 ms）。
fn text(at: f64, scope: &str, s: &str) -> TalkCue {
    let duration = s.chars().count() as f64 * 0.05;
    cue(at, scope, CueCommand::Text(s.into()), duration)
}

/// 台詞の頭に前置される全消去（台本の 0 秒）。
fn clear_all() -> TalkCue {
    cue(0.0, "0", CueCommand::ClearAll, 0.0)
}

/// 指令（どれも `Show`）ごとの（スコープ, animation `anim` の欄の絵）。
fn frames(cmds: &[DisplayCommand], anim: u32) -> Vec<(&str, Option<u32>)> {
    cmds.iter()
        .map(|c| match c {
            DisplayCommand::Show { scope, pattern, .. } => {
                (scope.as_str(), pattern.get(anim).map(|f| f.surface_id))
            }
            other => panic!("Show を期待: {other:?}"),
        })
        .collect()
}

/// 届いた文字で口のコマが出る。開始は刻みの時刻でなく文字が現れた時刻で、文字の数は待ちと塊を
/// またいで積み上がる（要件 4.1・4.3・4.5 の到着の側・6.3）。
#[test]
fn arriving_text_moves_the_mouth_from_the_glyph_reveal_time() {
    let mut rig = Rig::new();
    rig.tick(1000);
    assert_eq!(
        frames(&rig.hear(1000, emote_cue(0.0, "0", "0")), MOUTH),
        [("0", None)]
    );
    rig.hear(1000, clear_all());
    assert!(
        rig.hear(1000, text(0.0, "0", "あいう")).is_empty(),
        "文字の cue そのものは指令を出さない"
    );

    assert!(rig.tick(1090).is_empty(), "2 文字（1000・1050）ではまだ");
    assert_eq!(
        frames(&rig.tick(1120), MOUTH),
        [("0", Some(601))],
        "3 文字目（1100）が現れた刻みで口が動く"
    );
    assert_eq!(
        frames(&rig.tick(1145), MOUTH),
        [("0", Some(602))],
        "開始は 1100（刻みの 1120 から数えれば 1160 まで 601 のまま）"
    );
    rig.hear(1150, cue(0.15, "0", CueCommand::Wait, 0.5));
    assert_eq!(
        frames(&rig.tick(1190), MOUTH),
        [("0", None)],
        "`-1` で消える"
    );
    assert!(rig.tick(1640).is_empty(), "待ちの間は動かない");

    rig.hear(1650, text(0.65, "0", "えおか"));
    assert!(rig.tick(1710).is_empty(), "通算 5 文字（1650・1700）");
    assert_eq!(
        frames(&rig.tick(1795), MOUTH),
        [("0", Some(602))],
        "通算 6 文字目（1750）から 45ms の分だけ進んだコマを同じ刻みで出す"
    );
}

/// 文字でない cue も起点の見積もりを動かす（受け口は種類で振り分ける前に写しへ渡す・要件 6.4）。
///
/// 待ちの cue（振り分けでは「どの演者の担当でもない」腕へ行く）が起点を 1000 にする。続く文字の cue は
/// 10 ms 早く届く（「今 − cue の時刻」＝990）が、最大に負けて起点を戻さない。渡すのを文字の腕（または
/// 面の切り替えと文字の腕）だけにすると起点が 990 になり、3 文字目が 1190 に現れたことになって
/// 刻み 1195 で口が動く（この檻の最初の表明が赤になる）。
#[test]
fn cue_that_is_not_text_moves_the_time_origin() {
    let mut rig = Rig::new();
    rig.tick(900);
    rig.hear(900, emote_cue(0.0, "0", "0"));
    rig.hear(1000, cue(0.0, "0", CueCommand::Wait, 0.1));
    rig.hear(1090, text(0.1, "0", "あいう"));

    assert!(
        rig.tick(1195).is_empty(),
        "3 文字目は 1200（起点 1000＋台本の 0.2 秒）"
    );
    assert_eq!(
        frames(&rig.tick(1245), MOUTH),
        [("0", Some(602))],
        "1200 から 45ms"
    );
}

/// `\0` の文字で `\1` の口は動かない（要件 4.8）。`\1` も同じ口を持ち、自分の文字では動く。
#[test]
fn text_of_one_scope_does_not_move_the_other_scope() {
    let mut rig = Rig::new();
    rig.tick(1000);
    rig.hear(1000, emote_cue(0.0, "0", "0"));
    rig.hear(1000, emote_cue(0.0, "1", "0"));
    rig.hear(1000, clear_all());
    rig.hear(1000, text(0.0, "0", "あいう"));

    assert_eq!(
        frames(&rig.tick(1120), MOUTH),
        [("0", Some(601))],
        "指令は `\\0` の 1 件だけ"
    );
    assert_eq!(frames(&rig.tick(1190), MOUTH), [("0", None)]);

    // 陽性対照: `\1` の文字（1500・1550・1600）では `\1` の口だけが動く。
    rig.hear(1500, text(0.5, "1", "かきく"));
    assert_eq!(frames(&rig.tick(1610), MOUTH), [("1", Some(601))]);
}

/// 選択肢の文字は数える。改行・待ち・`\!` のコマンドは数えない（要件 4.10）。
#[test]
fn choice_text_counts_and_newline_wait_and_command_do_not() {
    let mut rig = Rig::new();
    rig.tick(1000);
    rig.hear(1000, emote_cue(0.0, "0", "0"));
    rig.hear(1000, clear_all());
    rig.hear(1000, text(0.0, "0", "あ"));
    rig.hear(
        1050,
        cue(0.05, "0", CueCommand::NewLine { ratio: 1.0 }, 0.0),
    );
    rig.hear(1050, cue(0.05, "0", CueCommand::Wait, 0.1));
    let raise = CueCommand::command_carrier("raise", vec!["OnTest".to_string()]);
    rig.hear(1150, cue(0.15, "0", raise, 0.0));
    assert!(
        rig.tick(1160).is_empty(),
        "文字は 1 つ（改行・待ち・`\\!` を数えれば 4 つで口が動く）"
    );

    let choice = CueCommand::Choice {
        id: "yes".into(),
        text: "はい".into(),
        references: Vec::new(),
    };
    rig.hear(1200, cue(0.2, "0", choice, 0.1));
    assert!(rig.tick(1210).is_empty(), "通算 2 文字（1000・1200）");
    assert_eq!(
        frames(&rig.tick(1295), MOUTH),
        [("0", Some(602))],
        "選択肢の 2 文字目（1250）が通算 3 文字目"
    );
}

/// 面が非表示（`\s[-1]`）のスコープでは、文字が届いても指令も記録も増えない（要件 4.9）。戻ると
/// `runonce` がもう 1 回鳴り（要件 2.6）、文字は戻った時点で現れていた次から数える。
#[test]
fn hidden_scope_stays_still_and_runonce_fires_again_on_return() {
    let mut rig = Rig::new();
    rig.tick(1000);
    let first = rig.hear(1000, emote_cue(0.0, "0", "1"));
    assert_eq!(
        frames(&first, ONCE),
        [("0", Some(701))],
        "前提: 最初の表示で鳴る"
    );
    assert_eq!(
        frames(&rig.tick(1050), ONCE),
        [("0", None)],
        "前提: 1 回で終える"
    );
    let hidden = rig.hear(1060, emote_cue(0.06, "0", "-1"));
    assert!(
        matches!(hidden[..], [DisplayCommand::Hide { .. }]),
        "{hidden:?}"
    );

    // 時計 2000 が新しい台詞の 0 秒。9 文字は 2000・2050・…・2400。
    let logs = capture_logs(|| {
        assert!(rig.hear(2000, clear_all()).is_empty());
        assert!(
            rig.hear(2000, text(0.0, "0", "あいうえおかきくけ"))
                .is_empty()
        );
        for now in [2060, 2120, 2190, 2240] {
            assert!(rig.tick(now).is_empty(), "{now}: 非表示の間は何も出ない");
        }
    });
    let ours: Vec<&str> = logs.lines().filter(|l| l.contains("seriko:")).collect();
    assert_eq!(
        ours.len(),
        2,
        "cue 1 件につき今までの読み飛ばしの 1 行だけ: {ours:?}"
    );
    assert!(
        ours.iter()
            .all(|l| l.contains("level=DEBUG") && l.contains("担当外（非 Shell）cue")),
        "{ours:?}"
    );

    // 2260 に戻る: 6 文字（2000〜2250）が現れている。そこから 3 文字目は 9 文字目（2400）。
    let back = rig.hear(2260, emote_cue(0.26, "0", "1"));
    assert_eq!(frames(&back, ONCE), [("0", Some(701))], "戻ると鳴る");
    assert_eq!(
        frames(&back, MOUTH),
        [("0", None)],
        "戻る前の文字で口は動かない"
    );
    assert_eq!(frames(&rig.tick(2310), ONCE), [("0", None)]);
    assert!(rig.tick(2395).is_empty(), "戻ってから 2 文字");
    assert_eq!(
        frames(&rig.tick(2410), MOUTH),
        [("0", Some(601))],
        "陽性対照: 表示中は同じ台詞の文字で口が動く"
    );
}

/// 本物のアクター（別スレッド・`CueSink` から届く cue）でも、届いた文字で口のコマが出る。時計は
/// 1000 に止めたままなので、cue はどれも 1000 に届いたことになる（刻みは自分の時刻を運ぶ）。
#[test]
fn spawned_actor_moves_the_mouth_for_text_arriving_through_the_sink() {
    let out = MockSurfaceOutput::new();
    let records = out.records();
    let (mut sink, handle) = spawn_seriko_clocked(
        SurfaceResolver::new(BTreeMap::new()),
        BindSet::from_ids([]),
        BindResolver::empty(),
        config(),
        out,
        Some(Arc::new(|| 1000)),
    );
    sink.send_tick(1000);
    CueSink::emit(&mut sink, emote_cue(0.0, "0", "0"));
    CueSink::emit(&mut sink, clear_all());
    CueSink::emit(&mut sink, text(0.0, "0", "あいう"));
    sink.send_tick(1120);
    sink.send_tick(1145);
    sink.close().expect("Close を送れること");
    handle.join().expect("Close で正常終了する");

    let records = records.lock().unwrap();
    assert_eq!(
        frames(&records, MOUTH),
        [("0", None), ("0", Some(601)), ("0", Some(602))]
    );
}

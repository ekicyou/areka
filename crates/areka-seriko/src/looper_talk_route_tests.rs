//! 合図を種類ごとに字の行き先と字の列へ当てる道を、`observe_cue` から刻みまで通す決定論テスト。
//!
//! シェルは面 0 が箱なし、面 10 に箱 `fuda`（element1）と `kami`（element2）。どちらの面も一番上に
//! `talk,3` の口（animation 0・601 → 40ms で 602 → 40ms で `-1`）を持つ。合図はどれも、刻みの前に
//! その時刻までに発火した物だけを渡す。期待の値は、場所ごとの前の字から 1 本の式で手で求めた固定の値。

use std::collections::BTreeMap;

use areka_emo_compose::{EmoWorld, SurfaceKeyOutcome, box_index, fold_boxes};
use areka_parsers::shell::{parse, parse_boxes};
use areka_sakura::CueCommand;
use dola::time::testing::{at_ms, ms};

use super::test_support::{started, switch};
use super::tests::{cfg, counting_rng};
use super::*;
use crate::resolve::{SurfaceResolver, SurfaceTarget};
use crate::test_support::{capture_logs, clear_all, cue, empty_states, scope, table_of, text};

const BOXES: &str = "\
balloon.fuda
{
size,100,50
}
balloon.kami
{
size,80,40
}
";

const SURFACES: &str = "\
surface0
{
animation0.interval,talk,3
animation0.pattern0,overlay,601,0,0,0
animation0.pattern1,overlay,602,40,0,0
animation0.pattern2,overlay,-1,40,0,0
}
surface10
{
element1,balloon,fuda,0,0
element2,balloon,kami,0,60
animation0.interval,talk,3
animation0.pattern0,overlay,601,0,0,0
animation0.pattern1,overlay,602,40,0,0
animation0.pattern2,overlay,-1,40,0,0
}
surface601
{
}
surface602
{
}
";

/// 面 10 に箱を置くが、どの面にも `talk` が無いシェル（門が閉じている）。
const SURFACES_WITHOUT_TALK: &str = "\
surface0
{
}
surface10
{
element1,balloon,fuda,0,0
element2,balloon,kami,0,60
}
";

fn shell_text(surfaces: &str) -> String {
    format!("{BOXES}{surfaces}")
}

fn routing() -> TextRouting {
    let text = shell_text(SURFACES);
    let world = EmoWorld::build(&parse(&text));
    let (layout, report) = fold_boxes(&parse_boxes(&text), &BTreeMap::new(), &world);
    assert_eq!(report.issues, vec![], "文面は誤りを持たない");
    TextRouting {
        boxes: box_index(&layout),
        surface_ids: [0, 10].into(),
    }
}

struct Rig {
    rt: LoopRuntime,
    states: ScopeStates,
    resolver: SurfaceResolver,
}

impl Rig {
    fn new(surfaces: &str) -> Rig {
        let (rng, _probe) = counting_rng(&[]);
        let mut config = cfg(table_of(&shell_text(surfaces)), rng);
        config.text_routing = routing();
        Rig {
            rt: LoopRuntime::new(config),
            states: empty_states(),
            resolver: SurfaceResolver::new(BTreeMap::new()),
        }
    }

    /// 時計 1000 に面 `sid` を出し、台本の 0 秒の全消去を届けた舞台。
    fn shown(surfaces: &str, sid: u32) -> Rig {
        let mut rig = Rig::new(surfaces);
        rig.tick(1000);
        switch(
            &mut rig.rt,
            &mut rig.states,
            SurfaceTarget::Show(sid),
            at_ms(1000),
        );
        rig.hear(1000, clear_all());
        rig
    }

    fn tick(&mut self, now_ms: i64) {
        self.rt.on_tick(at_ms(now_ms), &mut self.states);
    }

    /// 時刻 `now_ms` に発火した合図が届く（台本の起点は `now_ms − cue.at`）。
    fn hear(&mut self, now_ms: i64, cue: TalkCue) {
        let fired = TalkCue::scheduled(
            at_ms(now_ms) - cue.at,
            cue.at,
            cue.actor,
            cue.command,
            cue.duration,
        );
        self.rt.observe_cue(&fired, &self.resolver);
    }

    fn started(&self) -> Option<TimePoint> {
        started(&self.rt, Slot::Shell, 0)
    }
}

fn surface(key: &str) -> TalkCue {
    cue(
        TimeSpan::ZERO,
        "0",
        CueCommand::Emote { key: key.into() },
        TimeSpan::ZERO,
    )
}

fn balloon(key: &str) -> TalkCue {
    cue(
        TimeSpan::ZERO,
        "0",
        CueCommand::BalloonSurface { key: key.into() },
        TimeSpan::ZERO,
    )
}

fn marker(duration_ms: u32) -> TalkCue {
    cue(
        TimeSpan::ZERO,
        "0",
        CueCommand::command_carrier("*", Vec::new()),
        ms(duration_ms),
    )
}

/// 箱の在る面 10 で `\s[10]`（行き先＝先頭の箱 `fuda`）に「あい」（1000・1050）、`\s[0]`（行き先＝普通の
/// バルーン）に「うえお」（前の字が無いので塊の頭から 1000・1050・1100）。時刻の順の 3 字目は 1050。
/// 場所を分けなければ「うえお」は「い」に続いて 1100 から並び、3 字目は 1100 になる。
#[test]
fn surface_cue_moves_the_glyphs_to_their_own_place() {
    let mut rig = Rig::shown(SURFACES, 10);
    rig.hear(1000, surface("10"));
    rig.hear(1000, text(ms(0), "0", "あい"));
    rig.hear(1000, surface("0"));
    rig.hear(1000, text(ms(0), "0", "うえお"));

    rig.tick(1040);
    assert_eq!(rig.started(), None, "2 字（1000・1000）ではまだ");
    rig.tick(1060);
    assert_eq!(rig.started(), Some(at_ms(1050)), "3 字目は 1050");
}

/// 面 10 で `\b[kami]` は行き先を 2 つ目の箱へ替える: `fuda` の「あい」（1000・1050）に続く「うえお」は
/// `kami` の頭から 1000・1050・1100 に並び、3 字目は 1050。無い名前の `\b[nai]` は行き先を変えず、
/// 記録も出さない。
#[test]
fn name_form_balloon_cue_selects_a_box_and_a_missing_name_is_silent() {
    let mut rig = Rig::shown(SURFACES, 10);
    rig.hear(1000, surface("10"));
    rig.hear(1000, text(ms(0), "0", "あい"));
    rig.hear(1000, balloon("kami"));
    let logs = capture_logs(|| rig.hear(1000, balloon("nai")));
    assert_eq!(logs, Vec::<String>::new(), "無い名前でも記録しない");
    rig.hear(1000, text(ms(0), "0", "うえお"));

    rig.tick(1040);
    assert_eq!(rig.started(), None);
    rig.tick(1060);
    assert_eq!(rig.started(), Some(at_ms(1050)), "`kami` の字は頭から並ぶ");
}

/// `\c` は行き先の場所の字だけを捨てる: `fuda` の「あいう」（1000・1050・1100）と普通のバルーンの
/// 「かき」（1000・1050）の後、1020 の `\c`（行き先は普通のバルーン）は「き」（1050）だけを捨て、次の
/// 「くけこ」は頭から 1020・1070・1120。時刻の順は 1000・1000・1020・1050・1070・1100・1120 で、
/// 3 字目は 1020・6 字目は 1100。
#[test]
fn clear_cue_drops_only_the_destination_place() {
    let mut rig = Rig::shown(SURFACES, 10);
    rig.hear(1000, surface("10"));
    rig.hear(1000, text(ms(0), "0", "あいう"));
    rig.hear(1000, surface("0"));
    rig.hear(1000, text(ms(0), "0", "かき"));
    rig.tick(1010);
    rig.hear(
        1020,
        cue(TimeSpan::ZERO, "0", CueCommand::Clear, TimeSpan::ZERO),
    );
    rig.hear(1020, text(ms(0), "0", "くけこ"));

    rig.tick(1030);
    assert_eq!(rig.started(), Some(at_ms(1020)), "3 字目は 1020");
    rig.tick(1095);
    assert_eq!(rig.started(), Some(at_ms(1020)), "5 字ではまだ");
    rig.tick(1105);
    assert_eq!(rig.started(), Some(at_ms(1100)), "6 字目は `fuda` の 1100");
}

/// 印は前の字だけを進め、数えない: 「あ」（1000）の後の 200ms の印で前の字は 1200 になり、続く「いう」
/// は 1250・1300。3 字目は 1300。
#[test]
fn marker_cue_advances_the_previous_glyph_without_counting() {
    let mut rig = Rig::shown(SURFACES, 0);
    rig.hear(1000, text(ms(0), "0", "あ"));
    rig.hear(1000, marker(200));
    rig.hear(1000, text(ms(0), "0", "いう"));

    rig.tick(1260);
    assert_eq!(rig.started(), None, "印は数えない（2 字）");
    rig.tick(1310);
    assert_eq!(rig.started(), Some(at_ms(1300)), "3 字目は 1300");
}

/// どの表にも `talk` が無い間は、`\s`・`\b`・字のどれが届いても字の列も行き先も作らない。
#[test]
fn nothing_is_kept_while_no_table_has_talk() {
    let mut rig = Rig::shown(SURFACES_WITHOUT_TALK, 10);
    rig.hear(1000, surface("10"));
    rig.hear(1000, balloon("kami"));
    rig.hear(1000, text(ms(0), "0", "あい"));
    rig.hear(1000, marker(100));
    assert!(rig.rt.feeds.is_empty(), "字の列も行き先も作らない");
}

/// 門が閉じている間に `\s[10]` を受けた（控えは面 10）後、シェルの表の差し替えで門が開くと、行き先は
/// 控えから決め直される（面 10 の先頭の箱 `fuda`）。「あい」は `fuda` に 2000・2050、`\s[0]` の後の
/// 「うえお」は普通のバルーンの頭から 2000・2050・2100 で、3 字目は 2050。
#[test]
fn opening_the_gate_seeds_the_destination_from_the_kept_surface() {
    let mut rig = Rig::shown(SURFACES_WITHOUT_TALK, 10);
    rig.hear(1000, surface("10"));
    rig.states
        .note_text_surface(&scope(), SurfaceKeyOutcome::Show(10));

    assert!(rig.rt.replace_shell_table(table_of(&shell_text(SURFACES))));
    rig.rt.seed_talk_routes(&rig.states);
    rig.tick(2000);
    rig.hear(2000, clear_all());
    rig.hear(2000, text(ms(0), "0", "あい"));
    rig.hear(2000, surface("0"));
    rig.hear(2000, text(ms(0), "0", "うえお"));

    rig.tick(2040);
    assert_eq!(rig.started(), None);
    rig.tick(2060);
    assert_eq!(rig.started(), Some(at_ms(2050)), "行き先は控えの面 10 の箱");
}

/// actor の `\s` の経路は、門が閉じていても文字の層の規則の答えを控える（面の表に無い番号は控えない）。
#[test]
fn actor_surface_path_keeps_the_text_answer_regardless_of_the_gate() {
    let mut config = SerikoLoopConfig::disabled();
    config.text_routing = routing();
    let mut rt = LoopRuntime::new(config);
    let mut states = empty_states();
    let mut out = crate::output::MockSurfaceOutput::new();
    let resolver = SurfaceResolver::new(BTreeMap::new());
    for key in ["10", "7"] {
        let flow = crate::actor::handle_message(
            &resolver,
            &crate::BindResolver::empty(),
            &mut states,
            &mut rt,
            &mut out,
            crate::SerikoMsg::Cue(surface(key)),
        );
        assert_eq!(flow, std::ops::ControlFlow::Continue(()));
    }
    assert_eq!(
        states.text_surface(&scope()),
        Some(SurfaceKeyOutcome::Show(10)),
        "面 7 は面の表に無いので控えを変えない"
    );
    assert!(rt.feeds.is_empty(), "門が閉じているので字の列は作らない");
}

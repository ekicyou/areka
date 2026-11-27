//! 受け口から口パクまで。
//!
//! アクターの受け口 `handle_message` へ cue と刻みを届いた順に流し、出てくる指令で確かめる。固定する
//! のは受け口の配線（届いた cue が種類を問わず文字の列へ渡ること）と、スコープ・表示／非表示の
//! 振り分けだけ。区切りの計算・消去・刈り込みは `looper_talk_tests.rs` が固定している。
//!
//! 刻みと合図には時刻を書いて渡す。cue は
//! 届いた時刻に発火したものとして渡す（[`fired`]）。文字は 1 字 50 ms。刻みは文字の現れる時刻ちょうどを
//! 避けて置く。

use super::test_support::*;
use super::*;
use crate::looper::tests::{always_fire, cfg};
use crate::output::{DisplayCommand, MockSurfaceOutput};
use crate::test_support::table_of;
use areka_emo_compose::BindSet;
use areka_sakura::CueCommand;
use dola::cue::CueSink;
use dola::time::testing::{at_ms, ms};
use std::collections::BTreeMap;

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
    cfg(table_of(FACES), always_fire())
}

/// この檻の表で組んだ足場（本体は兄弟の足場 `ClockRig`）。
type Rig = ClockRig;

impl Rig {
    fn new() -> Self {
        Self::with_config(config())
    }
}

/// 届いた文字で口のコマが出る。開始は刻みの時刻でなく文字が現れた時刻で、文字の数は待ちと塊を
/// またいで積み上がる。
#[test]
fn arriving_text_moves_the_mouth_from_the_glyph_reveal_time() {
    let mut rig = Rig::new();
    rig.tick(at_ms(1000));
    assert_eq!(
        frames(
            &rig.hear_fired(at_ms(1000), emote_cue(ms(0), "0", "0")),
            MOUTH
        ),
        [("0", None)]
    );
    rig.hear_fired(at_ms(1000), clear_all());
    assert!(
        rig.hear_fired(at_ms(1000), text(ms(0), "0", "あいう"))
            .is_empty(),
        "文字の cue そのものは指令を出さない"
    );

    assert!(
        rig.tick(at_ms(1090)).is_empty(),
        "2 文字（1000・1050）ではまだ"
    );
    assert_eq!(
        frames(&rig.tick(at_ms(1120)), MOUTH),
        [("0", Some(601))],
        "3 文字目（1100）が現れた刻みで口が動く"
    );
    assert_eq!(
        frames(&rig.tick(at_ms(1145)), MOUTH),
        [("0", Some(602))],
        "開始は 1100（刻みの 1120 から数えれば 1160 まで 601 のまま）"
    );
    rig.hear_fired(at_ms(1150), cue(ms(150), "0", CueCommand::Wait, ms(500)));
    assert_eq!(
        frames(&rig.tick(at_ms(1190)), MOUTH),
        [("0", None)],
        "`-1` で消える"
    );
    assert!(rig.tick(at_ms(1640)).is_empty(), "待ちの間は動かない");

    rig.hear_fired(at_ms(1650), text(ms(650), "0", "えおか"));
    assert!(
        rig.tick(at_ms(1710)).is_empty(),
        "通算 5 文字（1650・1700）"
    );
    assert_eq!(
        frames(&rig.tick(at_ms(1795)), MOUTH),
        [("0", Some(602))],
        "通算 6 文字目（1750）から 45ms の分だけ進んだコマを同じ刻みで出す"
    );
}

/// 消去の cue も、振り分けの前に文字の列へ渡る。消去の発火時刻（1120）より後に現れるはずだった
/// 6 文字目（1250）では口が動かない（渡らなければ刻み 1260 で口が動く）。
#[test]
fn clear_cue_reaches_the_glyph_list_before_routing() {
    let mut rig = Rig::new();
    rig.tick(at_ms(900));
    rig.hear_fired(at_ms(900), emote_cue(ms(0), "0", "0"));
    rig.hear_fired(at_ms(1000), text(ms(0), "0", "あいうえおか"));
    rig.hear_fired(at_ms(1120), cue(ms(120), "0", CueCommand::Clear, ms(0)));

    assert_eq!(
        frames(&rig.tick(at_ms(1130)), MOUTH),
        [("0", Some(601))],
        "前提: 3 文字目（1100）"
    );
    assert_eq!(frames(&rig.tick(at_ms(1190)), MOUTH), [("0", None)]);
    assert!(
        rig.tick(at_ms(1260)).is_empty(),
        "消去の後に現れるはずだった 6 文字目では動かない"
    );
}

/// `\0` の文字で `\1` の口は動かない。`\1` も同じ口を持ち、自分の文字では動く。
#[test]
fn text_of_one_scope_does_not_move_the_other_scope() {
    let mut rig = Rig::new();
    rig.tick(at_ms(1000));
    rig.hear_fired(at_ms(1000), emote_cue(ms(0), "0", "0"));
    rig.hear_fired(at_ms(1000), emote_cue(ms(0), "1", "0"));
    rig.hear_fired(at_ms(1000), clear_all());
    rig.hear_fired(at_ms(1000), text(ms(0), "0", "あいう"));

    assert_eq!(
        frames(&rig.tick(at_ms(1120)), MOUTH),
        [("0", Some(601))],
        "指令は `\\0` の 1 件だけ"
    );
    assert_eq!(frames(&rig.tick(at_ms(1190)), MOUTH), [("0", None)]);

    // 陽性対照: `\1` の文字（1500・1550・1600）では `\1` の口だけが動く。
    rig.hear_fired(at_ms(1500), text(ms(500), "1", "かきく"));
    assert_eq!(frames(&rig.tick(at_ms(1610)), MOUTH), [("1", Some(601))]);
}

/// 選択肢の文字は数える。改行・待ち・`\!` のコマンドは数えない。
#[test]
fn choice_text_counts_and_newline_wait_and_command_do_not() {
    let mut rig = Rig::new();
    rig.tick(at_ms(1000));
    rig.hear_fired(at_ms(1000), emote_cue(ms(0), "0", "0"));
    rig.hear_fired(at_ms(1000), clear_all());
    rig.hear_fired(at_ms(1000), text(ms(0), "0", "あ"));
    rig.hear_fired(
        at_ms(1050),
        cue(ms(50), "0", CueCommand::NewLine { ratio: 1.0 }, ms(0)),
    );
    rig.hear_fired(at_ms(1050), cue(ms(50), "0", CueCommand::Wait, ms(100)));
    let raise = CueCommand::command_carrier("raise", vec!["OnTest".to_string()]);
    rig.hear_fired(at_ms(1150), cue(ms(150), "0", raise, ms(0)));
    assert!(
        rig.tick(at_ms(1160)).is_empty(),
        "文字は 1 つ（改行・待ち・`\\!` を数えれば 4 つで口が動く）"
    );

    let choice = CueCommand::Choice {
        id: "yes".into(),
        text: "はい".into(),
        references: Vec::new(),
    };
    rig.hear_fired(at_ms(1200), cue(ms(200), "0", choice, ms(100)));
    assert!(
        rig.tick(at_ms(1210)).is_empty(),
        "通算 2 文字（1000・1200）"
    );
    assert_eq!(
        frames(&rig.tick(at_ms(1295)), MOUTH),
        [("0", Some(602))],
        "選択肢の 2 文字目（1250）が通算 3 文字目"
    );
}

/// 面が非表示（`\s[-1]`）のスコープでは、文字が届いても指令も記録も増えない。戻ると
/// `runonce` がもう 1 回鳴り、文字は戻った時点で現れていた次から数える。
#[test]
fn hidden_scope_stays_still_and_runonce_fires_again_on_return() {
    let mut rig = Rig::new();
    rig.tick(at_ms(1000));
    let first = rig.hear_fired(at_ms(1000), emote_cue(ms(0), "0", "1"));
    assert_eq!(
        frames(&first, ONCE),
        [("0", Some(701))],
        "前提: 最初の表示で鳴る"
    );
    assert_eq!(
        frames(&rig.tick(at_ms(1050)), ONCE),
        [("0", None)],
        "前提: 1 回で終える"
    );
    let hidden = rig.hear_fired(at_ms(1060), emote_cue(ms(60), "0", "-1"));
    assert!(
        matches!(hidden[..], [DisplayCommand::Hide { .. }]),
        "{hidden:?}"
    );

    // 時計 2000 が新しい台詞の 0 秒。9 文字は 2000・2050・…・2400。
    let logs = capture_text(|| {
        assert!(rig.hear_fired(at_ms(2000), clear_all()).is_empty());
        assert!(
            rig.hear_fired(at_ms(2000), text(ms(0), "0", "あいうえおかきくけ"))
                .is_empty()
        );
        for now in [2060, 2120, 2190, 2240].map(at_ms) {
            assert!(rig.tick(now).is_empty(), "{now}: 非表示の間は何も出ない");
        }
    });
    let ours: Vec<&str> = logs.lines().filter(|l| l.contains("seriko:")).collect();
    assert_eq!(
        ours.len(),
        2,
        "cue 1 件につき読み飛ばしの 1 行だけ: {ours:?}"
    );
    assert!(
        ours.iter()
            .all(|l| l.contains("level=DEBUG") && l.contains("担当外（非 Shell）cue")),
        "{ours:?}"
    );

    // 2260 に戻る: 6 文字（2000〜2250）が現れている。そこから 3 文字目は 9 文字目（2400）。
    let back = rig.hear_fired(at_ms(2260), emote_cue(ms(260), "0", "1"));
    assert_eq!(frames(&back, ONCE), [("0", Some(701))], "戻ると鳴る");
    assert_eq!(
        frames(&back, MOUTH),
        [("0", None)],
        "戻る前の文字で口は動かない"
    );
    assert_eq!(frames(&rig.tick(at_ms(2310)), ONCE), [("0", None)]);
    assert!(rig.tick(at_ms(2395)).is_empty(), "戻ってから 2 文字");
    assert_eq!(
        frames(&rig.tick(at_ms(2410)), MOUTH),
        [("0", Some(601))],
        "陽性対照: 表示中は同じ台詞の文字で口が動く"
    );
}

/// 本物のアクター（別スレッド・`CueSink` から届く cue）でも、届いた文字で口のコマが出る。cue はどれも
/// 1000 に発火する（刻みは自分の時刻を運ぶ）。
#[test]
fn spawned_actor_moves_the_mouth_for_text_arriving_through_the_sink() {
    let out = MockSurfaceOutput::new();
    let records = out.records();
    let (mut sink, handle) = spawn_seriko(
        SurfaceResolver::new(BTreeMap::new()),
        crate::DefaultBinds::uniform(BindSet::from_ids([])),
        BindResolver::empty(),
        config(),
        out,
    );
    sink.send_tick(at_ms(1000));
    CueSink::emit(&mut sink, fired(at_ms(1000), emote_cue(ms(0), "0", "0")));
    CueSink::emit(&mut sink, fired(at_ms(1000), clear_all()));
    CueSink::emit(&mut sink, fired(at_ms(1000), text(ms(0), "0", "あいう")));
    sink.send_tick(at_ms(1120));
    sink.send_tick(at_ms(1145));
    sink.close().expect("Close を送れること");
    handle.join().expect("Close で正常終了する");

    let records = records.lock().unwrap();
    assert_eq!(
        frames(&records, MOUTH),
        [("0", None), ("0", Some(601)), ("0", Some(602))]
    );
}

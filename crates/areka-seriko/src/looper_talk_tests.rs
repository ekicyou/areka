//! 一番上の面の `talk` の檻（spec: areka-P0-seriko-trigger-intervals 要件 4.1・4.3〜4.5・4.8〜4.10・
//! 5.5・6.4・7.3・8.1・9.4・9.6・tasks.md 4.2）。
//!
//! 文字の到着は `observe_cue` の直呼び（偽の文字の到着）、刻みは `on_tick(now_ms)`、面の切り替えは
//! アクターと同じ順で踏む。時計は手で進める偽物で、どの呼び出しもその直前に時計を合わせる（観測を
//! 追い越さない）。文字は 1 字 50 ms（cue の再生時間＝文字数×50 ms）で、台本の 0 秒が時計の何 ms かは
//! 各テストの最初の cue が決める。刻みは文字の現れる時刻ちょうどを避けて置く（秒の端数に寄らない）。

use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use areka_emo_compose::BindSet;

use super::test_support::{capture_logs, cell, count, scope, started, switch, table_of};
use super::tests::{cfg, counting_rng};
use super::*;
use crate::resolve::SurfaceTarget;
use crate::state::StageNote;

/// 偽の時計つきのランタイムと、まだ何も表示していない状態。
struct Stage {
    rt: LoopRuntime,
    states: ScopeStates,
    clock: Arc<AtomicU64>,
}

impl Stage {
    fn new(config: SerikoLoopConfig) -> Stage {
        let clock = Arc::new(AtomicU64::new(0));
        let read = Arc::clone(&clock);
        Stage {
            rt: LoopRuntime::new(config)
                .with_clock(Some(Arc::new(move || read.load(Ordering::SeqCst)))),
            states: ScopeStates::new(BindSet::default()),
            clock,
        }
    }

    /// シェルの表が `text` のもの。
    fn shell(text: &str) -> Stage {
        let (rng, _probe) = counting_rng(&[]);
        Stage::new(cfg(table_of(text), rng))
    }

    fn tick(&mut self, ms: u64) -> Vec<DisplayCommand> {
        self.clock.store(ms, Ordering::SeqCst);
        self.rt.on_tick(ms, &mut self.states)
    }

    /// 時計が `ms` のときに cue が届く。
    fn hear(&mut self, ms: u64, cue: TalkCue) {
        self.clock.store(ms, Ordering::SeqCst);
        self.rt.observe_cue(&cue);
    }

    /// 時計が `ms` のときに `\0` のシェルの面が `sid` に切り替わる。
    fn show(&mut self, sid: u32, ms: u64) {
        self.clock.store(ms, Ordering::SeqCst);
        switch(&mut self.rt, &mut self.states, SurfaceTarget::Show(sid), ms).expect("面が変わる");
    }

    /// `\0` のシェルの口（animation 0）の開始の時刻。
    fn started(&self) -> Option<u64> {
        started(&self.rt, Slot::Shell, 0)
    }

    /// `\0` のシェルの口（animation 0）の欄の絵。
    fn mouth(&self) -> Option<u32> {
        cell(&self.states, Slot::Shell, 0)
    }

    /// `\0` の `slot` の面の文字の数え（数え始めの序数, 数え済みの序数）。
    fn counted(&self, slot: Slot) -> Option<(u64, u64)> {
        self.rt
            .armed
            .get(&(scope(), slot))
            .and_then(|state| state.talk_window_bounds())
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

const NINE: &str = "あいうえおかきくけ";

/// 面 0 と面 1 の一番上に `talk,3` の口（animation 0＝601 → 40ms で 602 → 40ms で `-1`・全部で 80ms）。
/// 面 2 は何も持たない。
const MOUTH: &str = "surface0\n{\n\
    animation0.interval,talk,3\n\
    animation0.pattern0,overlay,601,0,0,0\n\
    animation0.pattern1,overlay,602,40,0,0\n\
    animation0.pattern2,overlay,-1,40,0,0\n}\n\
    surface1\n{\n\
    animation0.interval,talk,3\n\
    animation0.pattern0,overlay,601,0,0,0\n\
    animation0.pattern1,overlay,602,40,0,0\n\
    animation0.pattern2,overlay,-1,40,0,0\n}\n\
    surface2\n{\n}\nsurface601\n{\n}\nsurface602\n{\n}\n";

/// 面 0 を出し、時計 1000 を台本の 0 秒にして 9 文字（1000・1050・…・1400 に現れる）を届けた舞台。
fn nine_glyphs_from_1000() -> Stage {
    let mut st = Stage::shell(MOUTH);
    st.tick(1000);
    st.show(0, 1000);
    st.hear(1000, clear_all());
    st.hear(1000, text(0.0, "0", NINE));
    st
}

/// 3 文字目・6 文字目・9 文字目が現れた刻みで口が動く。開始は刻みの時刻でなく文字が現れた時刻で、
/// 同じ刻みのうちに経過の分のコマが出る。口は 1 回流れて `-1` で消える（要件 4.1・4.5・5.1・5.2）。
#[test]
fn mouth_starts_at_every_third_glyph_from_its_reveal_time() {
    let mut st = nine_glyphs_from_1000();

    assert!(st.tick(1090).is_empty(), "2 文字（1000・1050）ではまだ");
    assert_eq!((st.started(), st.mouth()), (None, None));

    let cmds = st.tick(1120);
    assert_eq!(
        st.started(),
        Some(1100),
        "開始は 3 文字目が現れた 1100（刻みの 1120 ではない）"
    );
    assert_eq!(cmds.len(), 1, "同じ刻みで口のコマを出す");
    assert_eq!(st.mouth(), Some(601));
    st.tick(1145);
    assert_eq!(st.mouth(), Some(602), "経過 45");
    st.tick(1190);
    assert_eq!((st.started(), st.mouth()), (None, None), "`-1` で終える");

    st.tick(1240);
    assert_eq!(st.started(), None, "5 文字ではまだ");
    st.tick(1295);
    assert_eq!(st.started(), Some(1250), "6 文字目が現れた時刻");
    assert_eq!(
        st.mouth(),
        Some(602),
        "遅れた 45ms の分だけ進んだコマを同じ刻みで出す"
    );
    st.tick(1345);
    assert_eq!(st.started(), None);
    st.tick(1410);
    assert_eq!((st.started(), st.mouth()), (Some(1400), Some(601)));
}

/// 文字の数は待ちと塊をまたいで積み上がる: 2 文字 → 待ち → 2 文字 なら、通算 3 文字目（後ろの塊の
/// 1 文字目）で口が動く（要件 4.3）。
#[test]
fn glyph_count_accumulates_across_a_wait_and_chunks() {
    let mut st = Stage::shell(MOUTH);
    st.tick(1000);
    st.show(0, 1000);
    st.hear(1000, clear_all());
    st.hear(1000, text(0.0, "0", "あい"));
    st.tick(1060);
    st.hear(1100, cue(0.1, "0", CueCommand::Wait, 0.5));
    st.tick(1590);
    assert_eq!(st.started(), None, "待ちの間は 2 文字のまま");

    st.hear(1600, text(0.6, "0", "うえ"));
    st.tick(1610);
    assert_eq!(st.started(), Some(1600), "通算 3 文字目");
    assert_eq!(st.mouth(), Some(601));
}

/// 面が切り替わると、その時点で現れていた文字の次から数え直す（要件 4.4）。
#[test]
fn glyph_count_restarts_when_the_surface_changes() {
    let mut st = nine_glyphs_from_1000();
    st.tick(1060);
    st.show(1, 1060);
    assert_eq!(st.counted(Slot::Shell), Some((2, 2)), "2 文字が現れている");

    st.tick(1110);
    assert_eq!(st.started(), None, "古い面の数えなら 3 文字目で鳴っていた");
    st.tick(1160);
    assert_eq!(st.started(), None, "切り替えてから 2 文字");
    st.tick(1210);
    assert_eq!(
        st.started(),
        Some(1200),
        "切り替えてから 3 文字目（通算 5 文字目）"
    );
}

/// `\0` の文字で `\1` の口は動かない（要件 4.8）。
#[test]
fn text_of_one_scope_does_not_move_another_scope() {
    let mut st = nine_glyphs_from_1000();
    let other = ActorKey::from("1");
    st.states.apply(&other, SurfaceTarget::Show(0));
    st.rt.on_surface_changed(&other, Slot::Shell);
    st.rt
        .refresh(&other, Slot::Shell, Some(1000), &mut st.states);

    for now in [1060, 1120, 1190, 1295, 1410] {
        st.tick(now);
        assert!(
            !st.rt.playback.contains_key(&(other.clone(), Slot::Shell)),
            "{now}: `\\1` は鳴らない"
        );
        assert!(st.states.current_pattern(&other, Slot::Shell).is_empty());
    }
    assert_eq!(st.started(), Some(1400), "陽性対照: `\\0` は鳴っている");
    let counted = |key: &ActorKey| st.rt.armed[&(key.clone(), Slot::Shell)].talk_window_bounds();
    assert_eq!(counted(&other), Some((0, 0)), "`\\1` の文字は 0 のまま");
}

/// 面が非表示のスコープでは、文字が届いても状態も記録も増えない。戻ったら、その時点で現れていた
/// 文字の次から数える（要件 4.4・4.9）。
#[test]
fn hidden_scope_keeps_no_state_and_counts_from_its_return() {
    let mut st = Stage::shell(MOUTH);
    let logs = capture_logs(|| {
        st.tick(1000);
        st.hear(1000, clear_all());
        st.hear(1000, text(0.0, "0", NINE));
        for now in [1060, 1120, 1190, 1240] {
            assert!(st.tick(now).is_empty(), "{now}");
            assert!(st.rt.armed.is_empty(), "{now}: 引き金の状態は生まれない");
            assert!(st.rt.playback.is_empty(), "{now}");
        }
    });
    assert_eq!(count(&logs, "seriko:"), 0, "{logs:?}");
    let feed = &st.rt.feeds[&scope()];
    assert_eq!(feed.time_of(4), None, "現れた文字は列に溜めない");
    assert!(
        feed.time_of(5).is_some(),
        "陽性対照: まだ現れていない文字は在る"
    );

    // 1260 に面を出す: 6 文字（1000〜1250）が現れている。そこから 3 文字目は 9 文字目（1400）。
    st.show(0, 1260);
    st.tick(1310);
    assert_eq!(st.started(), None, "戻る前の文字は数えない");
    st.tick(1410);
    assert_eq!(st.started(), Some(1400));
}

/// バルーンの面の `talk`: 窓が閉じている間は数えも再生も持たず、文字の列も溜めない。開いたら
/// その時点で現れていた文字の次から数える（要件 4.9・5.5）。
#[test]
fn closed_balloon_window_counts_nothing_and_recounts_on_open() {
    let (rng, _probe) = counting_rng(&[]);
    let mut st = Stage::new(SerikoLoopConfig {
        shell_table: AnimationTable::empty(),
        balloon_tables: BTreeMap::from([(scope(), table_of(MOUTH))]),
        rng,
    });
    let note = |st: &mut Stage, open: bool, ms: u64| {
        st.clock.store(ms, Ordering::SeqCst);
        st.states.note_stage(&StageNote::Balloon {
            scope: scope(),
            open,
            face: 0,
            generation: 0,
        });
        st.rt
            .refresh(&scope(), Slot::Balloon, Some(ms), &mut st.states);
    };

    note(&mut st, false, 1000);
    st.tick(1000);
    st.hear(1000, clear_all());
    st.hear(1000, text(0.0, "0", NINE));
    for now in [1060, 1120, 1160] {
        st.tick(now);
        assert_eq!(st.counted(Slot::Balloon), None, "{now}: 数えを持たない");
        assert!(st.rt.playback.is_empty(), "{now}");
    }
    assert_eq!(st.rt.feeds[&scope()].time_of(3), None, "列は刈り込まれる");

    // 1160 に開く: 4 文字（1000〜1150）が現れている。そこから 3 文字目は 7 文字目（1300）。
    note(&mut st, true, 1160);
    assert_eq!(st.counted(Slot::Balloon), Some((4, 4)));
    st.tick(1260);
    assert_eq!(started(&st.rt, Slot::Balloon, 0), None);
    st.tick(1310);
    assert_eq!(started(&st.rt, Slot::Balloon, 0), Some(1300));

    note(&mut st, false, 1320);
    assert_eq!(st.counted(Slot::Balloon), None, "閉じたら数えを捨てる");
}

/// 面 0 の一番上に `runonce` だけ（`talk` はどこにも無い）。
const NO_TALK: &str = "surface0\n{\n\
    animation0.interval,runonce\n\
    animation0.pattern0,overlay,601,0,0,0\n\
    animation0.pattern1,overlay,-1,40,0,0\n}\n\
    surface601\n{\n}\n";

/// 表に `talk` が無ければ、届いた cue は何も変えない（起点も文字の列も数えも生まれない）。`talk` の
/// 在る表では同じ cue で起点と列が生まれる。どちらでも cue を写すこと自体は記録を出さない
/// （要件 4.9・7.3・8.1）。
#[test]
fn cues_change_nothing_when_the_table_has_no_talk() {
    let run = |table: &str| {
        let mut st = Stage::shell(table);
        st.tick(1000);
        st.show(0, 1000);
        let logs = capture_logs(|| {
            st.hear(1000, clear_all());
            st.hear(1000, text(0.0, "0", NINE));
            st.hear(1450, cue(0.45, "0", CueCommand::Clear, 0.0));
        });
        assert!(logs.is_empty(), "cue の写しは記録を出さない: {logs:?}");
        st
    };

    let plain = run(NO_TALK);
    assert!(plain.rt.feeds.is_empty(), "文字の列は生まれない");
    assert_eq!(plain.rt.epoch.talk_time(2000), None, "起点は生まれない");
    assert_eq!(plain.counted(Slot::Shell), None, "文字の数えは生まれない");

    let talking = run(MOUTH);
    assert!(talking.rt.feeds.contains_key(&scope()), "陽性対照");
    assert_eq!(talking.rt.epoch.talk_time(2000), Some(1.0), "陽性対照");
    assert_eq!(talking.counted(Slot::Shell), Some((0, 0)), "陽性対照");
}

/// 表の差し替えで「`talk` が在るか」を組み直す: 在る表に替えたら cue を写し始め、無い表に戻したら
/// 起点と文字の列を捨てて写さなくなる。
#[test]
fn table_replacement_rebuilds_the_talk_gate() {
    let mut st = Stage::shell(NO_TALK);
    st.tick(1000);
    st.hear(1000, text(0.0, "0", NINE));
    assert!(st.rt.feeds.is_empty(), "前提: 無い表では写さない");

    st.rt.replace_shell_table(table_of(MOUTH));
    st.hear(1000, text(0.0, "0", NINE));
    assert!(st.rt.feeds.contains_key(&scope()), "在る表に替えたら写す");

    st.rt.replace_shell_table(table_of(NO_TALK));
    assert!(st.rt.feeds.is_empty(), "無い表に戻したら捨てる");
    assert_eq!(st.rt.epoch.talk_time(2000), None);
    st.hear(1000, text(0.0, "0", NINE));
    assert!(st.rt.feeds.is_empty());
}

/// 文字が 1 つも届かないうち（起点の見積もりがまだ無いうち）に構えた面でも、最初の台詞の 1 文字目
/// から数える（最初の cue と 1 文字目は同じ刻みの間に届く）。
#[test]
fn surface_armed_before_any_cue_counts_from_the_first_glyph() {
    let mut st = Stage::shell(MOUTH);
    st.tick(1000);
    st.show(0, 1000);
    for now in [1016, 3000] {
        st.tick(now);
    }

    st.hear(5000, clear_all());
    st.hear(5000, text(0.0, "0", NINE));
    st.tick(5016);
    assert_eq!(st.started(), None, "1 文字目が現れただけ");
    st.tick(5110);
    assert_eq!(
        st.started(),
        Some(5100),
        "3 文字目（1 文字目を数え落とすと 5150）"
    );
}

/// 選択肢の文字は数える。改行・待ち・`\!` のコマンドは数えない（要件 4.10）。
#[test]
fn choice_text_counts_and_non_text_cues_do_not() {
    let mut st = Stage::shell(MOUTH);
    st.tick(1000);
    st.show(0, 1000);
    st.hear(1000, clear_all());
    st.hear(1000, text(0.0, "0", "あ"));
    st.hear(
        1050,
        cue(0.05, "0", CueCommand::NewLine { ratio: 1.0 }, 0.0),
    );
    st.hear(1050, cue(0.05, "0", CueCommand::Wait, 0.1));
    let raise = CueCommand::command_carrier("raise", vec!["OnTest".to_string()]);
    st.hear(1150, cue(0.15, "0", raise, 0.0));
    let choice = CueCommand::Choice {
        id: "yes".into(),
        text: "はい".into(),
        references: Vec::new(),
    };
    st.hear(1150, cue(0.15, "0", choice, 0.1));
    assert_eq!(
        st.rt.feeds[&scope()].revealed_until(f64::INFINITY),
        3,
        "文字 1＋選択肢 2 だけが列に積まれる"
    );

    st.tick(1160);
    assert_eq!(st.started(), None, "2 文字（1000・1150）");
    st.tick(1210);
    assert_eq!(st.started(), Some(1200), "選択肢の 2 文字目が通算 3 文字目");
}

/// 台詞の途中の消去: 現れなかった文字は数えず、消去の後の文字はそれまでに現れた文字に続けて数える
/// （要件 4.3・4.10）。
#[test]
fn clear_in_mid_talk_drops_unrevealed_glyphs_and_keeps_counting() {
    let mut st = Stage::shell(MOUTH);
    st.tick(1000);
    st.show(0, 1000);
    st.hear(1000, clear_all());
    st.hear(1000, text(0.0, "0", "あいうえおか"));
    st.tick(1120);
    assert_eq!(st.started(), Some(1100), "前提: 3 文字目");

    // 3 文字が現れたところで消去（残りの 3 文字は現れない）。続く 4 文字は 1120・1170・1220・1270。
    st.hear(1120, cue(0.12, "0", CueCommand::Clear, 0.0));
    st.hear(1120, text(0.12, "0", "きくけこ"));
    st.tick(1190);
    assert_eq!(st.started(), None, "通算 5 文字");
    st.tick(1230);
    assert_eq!(
        st.started(),
        Some(1220),
        "通算 6 文字目（消去の後の 3 文字目）"
    );
}

/// 区切りの途中（5 文字）で台詞が中断され、新しい台詞（頭に全消去・台本の 0 秒）が始まる。数えは
/// 現れた 5 文字に続くので、新しい台詞の 1 文字目（通算 6 文字目）で口が動く。現れなかった残りの
/// 4 文字は数えない。
#[test]
fn new_talk_after_an_interrupt_moves_the_mouth_at_its_first_boundary() {
    let mut st = nine_glyphs_from_1000();
    st.tick(1120);
    st.tick(1210);
    assert_eq!(
        st.counted(Slot::Shell),
        Some((0, 5)),
        "前提: 5 文字が現れた"
    );

    // 時計 2000 が新しい台詞の 0 秒。6 文字は 2000・2050・…・2250。
    st.hear(2000, clear_all());
    st.hear(2000, text(0.0, "0", "さしすせそた"));
    st.tick(2010);
    assert_eq!(st.started(), Some(2000), "新しい台詞の 1 文字目");
    st.tick(2110);
    assert_eq!(st.started(), None, "通算 8 文字");
    st.tick(2160);
    assert_eq!(st.started(), Some(2150), "新しい台詞の 4 文字目");
}

/// 刻みの間に面が切り替わり（その時点の 4 文字から数え始める）、次の刻みの前に新しい台詞の全消去が
/// 届く。前の刻みより後に現れた 2 文字は列から捨てられ序数が振り直されるが、数えは列に揃うので、
/// 新しい台詞の 3 文字目で口が動く。
#[test]
fn clear_all_right_after_arming_between_ticks_counts_the_new_talk_from_its_head() {
    let mut st = nine_glyphs_from_1000();
    st.tick(1060);
    st.show(1, 1160);
    assert_eq!(
        st.counted(Slot::Shell),
        Some((4, 4)),
        "前提: 4 文字が現れている"
    );

    // 時計 1170 が新しい台詞の 0 秒。6 文字は 1170・1220・1270・…。
    st.hear(1170, clear_all());
    st.hear(1170, text(0.0, "0", "さしすせそた"));
    st.tick(1230);
    assert_eq!(st.started(), None, "新しい台詞の 2 文字");
    st.tick(1280);
    assert_eq!(st.started(), Some(1270), "新しい台詞の 3 文字目");
}

/// 面 0 の一番上には何も無く、面 0 に置かれた部品（面 100）にだけ `talk,3` が在る。
const MOUTH_ON_PART: &str = "surface0\n{\nelement0,overlay,100,0,0\n}\n\
    surface100\n{\n\
    animation0.interval,talk,3\n\
    animation0.pattern0,overlay,601,0,0,0\n\
    animation0.pattern1,overlay,-1,40,0,0\n}\n\
    surface601\n{\n}\n";

/// 一番上に `talk` が無く部品にだけ在る面でも、その面の文字の数えは進む（部品が借りる窓の元）。
/// 一番上では何も始まらず、数え終えた文字は列から刈り込まれる。
#[test]
fn slot_with_talk_only_on_a_part_still_counts_glyphs() {
    let table = table_of(MOUTH_ON_PART);
    assert!(
        table.has_talk() && table.has_animated_parts() && table.animations(0).is_empty(),
        "前提: `talk` は部品にだけ在る"
    );
    let mut st = Stage::shell(MOUTH_ON_PART);
    st.tick(1000);
    st.show(0, 1000);
    st.hear(1000, clear_all());
    st.hear(1000, text(0.0, "0", NINE));

    st.tick(1120);
    assert_eq!(st.counted(Slot::Shell), Some((0, 3)));
    st.tick(1260);
    assert_eq!(st.counted(Slot::Shell), Some((0, 6)));
    assert!(st.rt.playback.is_empty(), "一番上では何も始まらない");
    let feed = &st.rt.feeds[&scope()];
    assert_eq!(feed.time_of(5), None, "数え終えた文字は刈り込む");
    assert!(feed.time_of(6).is_some());
}

/// 面 0 の一番上に `talk,3` が 2 本: animation 0 は `-1` で止まり、animation 1 は末尾のコマを残して
/// 終える。
const TWO_MOUTHS: &str = "surface0\n{\n\
    animation0.interval,talk,3\n\
    animation0.pattern0,overlay,601,0,0,0\n\
    animation0.pattern1,overlay,-1,40,0,0\n\
    animation1.interval,talk,3\n\
    animation1.pattern0,overlay,611,0,0,0\n\
    animation1.pattern1,overlay,612,40,0,0\n}\n\
    surface601\n{\n}\nsurface611\n{\n}\nsurface612\n{\n}\n";

/// `talk` の開始・停止・末尾の記録は `debug!` で、区切りごとに 1 件ずつ。文字ごと・刻みごとの記録は
/// 無い（30 文字・約 100 刻みで、区切り 10 回分の記録だけ・要件 7.3）。
#[test]
fn talk_records_are_debug_and_bounded_by_boundaries() {
    let mut st = Stage::shell(TWO_MOUTHS);
    let thirty = NINE.repeat(3) + "こさし";
    assert_eq!(thirty.chars().count(), 30);
    let logs = capture_logs(|| {
        st.tick(1000);
        st.show(0, 1000);
        st.hear(1000, clear_all());
        st.hear(1000, text(0.0, "0", &thirty));
        let mut now = 1016;
        while now <= 2600 {
            st.tick(now);
            now += 16;
        }
    });

    let ours: Vec<&String> = logs
        .iter()
        .filter(|l| l.contains("seriko: trigger") || l.contains("seriko: loop"))
        .collect();
    assert_eq!(count(&logs, "seriko: trigger 面に入った"), 1, "{ours:?}");
    assert_eq!(
        count(&logs, "seriko: trigger talk を鳴らした"),
        20,
        "{ours:?}"
    );
    assert_eq!(count(&logs, "seriko: loop 停止"), 10, "{ours:?}");
    assert_eq!(count(&logs, "seriko: loop 末尾残留"), 10, "{ours:?}");
    assert_eq!(ours.len(), 41, "ほかの記録は無い: {ours:?}");
    assert!(
        ours.iter().all(|l| l.contains("level=DEBUG")),
        "どれも debug!: {ours:?}"
    );
}

/// 面 0 の一番上に `talk,3` の口で、長さが区切りの間隔（3 文字＝150ms）ちょうど。
const EXACT_MOUTH: &str = "surface0\n{\n\
    animation0.interval,talk,3\n\
    animation0.pattern0,overlay,601,0,0,0\n\
    animation0.pattern1,overlay,602,75,0,0\n\
    animation0.pattern2,overlay,-1,75,0,0\n}\n\
    surface601\n{\n}\nsurface602\n{\n}\n";

/// 長さが区切りの間隔ちょうどの口は、区切りごとに毎回動く（前の再生は区切りの時刻には終えている・
/// 要件 4.1・4.7）。
#[test]
fn mouth_as_long_as_the_boundary_gap_moves_at_every_boundary() {
    let mut st = Stage::shell(EXACT_MOUTH);
    st.tick(1000);
    st.show(0, 1000);
    st.hear(1000, clear_all());
    st.hear(1000, text(0.0, "0", &(NINE.to_string() + "こさし")));

    let mut starts = Vec::new();
    let mut now = 1016;
    while now <= 1700 {
        st.tick(now);
        if let Some(at) = st.started() {
            if starts.last() != Some(&at) {
                starts.push(at);
            }
        }
        now += 16;
    }
    assert_eq!(starts, [1100, 1250, 1400, 1550]);
}

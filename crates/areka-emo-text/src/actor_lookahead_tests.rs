//! # 本番と同じ経路を歩く検査（文節の折り返しで、表示済みの字が動かない・要件 1・2.5・5）
//!
//! 台本の文字列 → `areka_parsers::sakura::parse` → `areka_sakura::compile` → 刻印
//! （`with_absolute_start_time`）→ `CuePlayer::from_sheet` → `register_sink`（本物の
//! [`EmoTextSink`](crate::sink::EmoTextSink)）→ `tick(時刻)` → UI の待ち行列 →
//! [`spawn_emo_text`] の取り出し → [`TextLayerRuntime`] → `arrange_for_test`
//! （＝本番の提示と同じ `arrange_lines`）に [`FixedMetrics`] を渡して行の列を取る。
//! 再生機は検査のスレッドで回し、待ち行列は [`pump_until_idle`] で汲み出す
//! （`areka_sakura::drive` の初回 `Tick` の腕と同じ 刻印 → `from_sheet` → `register_sink` → `tick`）。
//! GPU の資源は使わない。
//!
//! 判定の関数 [`reveal_violations`] は、各段階の行の列を「行ごとの字の並び」に直し、
//! (a) 前の段階で見えていた字が同じ行の同じ番目にあること、
//! (b) 出終わったときの行の列の、その字までの部分と同じであること、を判定する。

use std::cell::RefCell;
use std::rc::Rc;

use areka_parsers::balloon::{
    BalloonModel, Font, FontColor, Origin, ValidRect, WindowPosition, WordWrapPoint,
};
use areka_sakura::contract::{ActorKey, CueCommand};
use areka_sakura::sysvar::SystemVarSnapshot;
use bevy_ecs::prelude::World;
use dola::cue::{CuePayload, CuePlayer, CueSink, TalkCue};
use log_capture_kit::{CapturedEvent, capture};

use super::test_support::pump_until_idle;
use super::{ResolvedBalloonText, TextLayerRuntime, TextSlotBinding, spawn_emo_text};
use crate::layout::{FixedMetrics, PositionedLine};
use crate::place::PlaceKey;
use crate::sink::EmoTextSink;
use crate::state::TextLayerConfig;

// ── 経路を歩く支え ──

/// 刻印する絶対の開始時刻。0 でない値にして、絶対の時刻と相対の時刻の取り違えを拾う。
pub(super) const ANCHOR: f64 = 100.0;
/// 字の丈（`FixedMetrics` では全角 1 字の送り幅＝字の丈）。
const FONT_HEIGHT: u32 = 20;
/// 欄の幅: 全角 9 字（180）が入り、10 字目（200）が入らない値。
/// 「イイジャン！‥‥」の 8 字は 1 行に入り、「‥‥ええと、」の 6 字は「イイジャン！」の後ろに入らない。
pub(super) const NINE_WIDE: (u32, u32) = (190, 200);

/// 文節の折り返し（`budoux_newline,1`）のバルーン。描画範囲・折り返しの基準は面の全域
/// （validrect・wordwrappoint を書かない＝右辺で折り返す）、原点は (0,0)。
pub(super) fn budoux_model() -> BalloonModel {
    balloon_model(None, Some("1"))
}

/// `budoux_newline` の無い（1 字ずつの折り返しの）バルーン。ほかは [`budoux_model`] と同じ。
pub(super) fn char_by_char_model() -> BalloonModel {
    balloon_model(None, None)
}

/// 書字方向（`writing_mode`）と `budoux_newline` を選べるバルーン。ほかは [`budoux_model`] と同じ。
pub(super) fn balloon_model(
    writing_mode: Option<&str>,
    budoux_newline: Option<&str>,
) -> BalloonModel {
    BalloonModel::new(
        WindowPosition::new(None, None),
        Origin::new(Some(0), Some(0)),
        WordWrapPoint::new(None, None),
        ValidRect::new(None, None, None, None),
        Font::new(None, Some(FONT_HEIGHT), FontColor::new(None, None, None)),
        writing_mode.map(str::to_string),
        budoux_newline.map(str::to_string),
    )
}

/// 経路を歩いた 1 段階: 判定の時刻（トークの頭からの秒）と、その時刻の行ごとの字の並び。
#[derive(Clone, Debug)]
pub(super) struct Stage {
    pub(super) time: f64,
    pub(super) rows: Vec<Vec<String>>,
}

/// 合図の届け方（検査ごとに経路の 1 か所だけを変える）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Delivery {
    /// 本番どおり: 先渡しを通し、判定の時刻ごとに `tick` する。
    Staged,
    /// 対照: `emit` だけを内側へ通す包み越し（先渡しが届かない＝修正前の届き方）。
    WithoutPreview,
    /// 最後の判定の時刻へ 1 度だけ `tick` して全部を届け、各時刻の行の列を取る。
    AllAtOnce,
}

/// 検査用の包み: `emit` だけを内側へ通し、`preview`（先渡し）は既定の「何もしない」に任せて落とす。
struct EmitOnly<S>(S);

impl<S: CueSink> CueSink for EmitOnly<S> {
    fn emit(&mut self, cue: TalkCue) {
        self.0.emit(cue);
    }
}

/// 経路の土台: 本物の実行時と受け口（[`spawn_emo_text`]）と、装着先を作るための `World`。
/// 装着の登録（[`attach`](Self::attach)）と再生機の登録（[`play`](Self::play)）の順は検査が選ぶ。
pub(super) struct Rig {
    pub(super) runtime: Rc<RefCell<TextLayerRuntime>>,
    pub(super) world: World,
    sink: EmoTextSink,
    _drain: wintf_winmsg_executor::JoinHandle<()>,
}

impl Rig {
    pub(super) fn new() -> Rig {
        let runtime = Rc::new(RefCell::new(TextLayerRuntime::new(
            TextLayerConfig::default(),
        )));
        let (sink, drain) =
            spawn_emo_text(Rc::clone(&runtime)).expect("spawn_emo_text on the pump thread");
        Rig {
            runtime,
            world: World::new(),
            sink,
            _drain: drain,
        }
    }

    /// 装着の登録（本番の `register_actor`＝バルーンの定義の見た目が状態へ入る）。
    pub(super) fn attach(&mut self, actor: &str, model: &BalloonModel, image: (u32, u32)) {
        let slot = self.world.spawn_empty().id();
        let window = self.world.spawn_empty().id();
        self.runtime.borrow_mut().register_actor(
            ActorKey::from(actor),
            TextSlotBinding::new(slot, window, 1.0, image, image),
            ResolvedBalloonText::resolve(model, image),
        );
    }

    /// 台本を `parse` → `compile` → 刻印（`anchor`）→ `from_sheet` し、受け口を登録して汲み出す
    /// （`Staged`・`AllAtOnce` はここで先渡しが届く）。`tick` はまだしない。
    pub(super) fn play(&self, script: &str, anchor: f64, delivery: Delivery) -> Talk {
        let talk = areka_sakura::compile(
            &areka_parsers::sakura::parse(script),
            &SystemVarSnapshot::default(),
        );
        // 合図が届く時刻（相対）と、その合図が出終わる時刻を判定の時刻にする。
        let mut times: Vec<f64> = talk
            .sheet
            .cues()
            .iter()
            .flat_map(|c| [c.start_time, c.start_time + c.duration])
            .collect();
        times.sort_by(f64::total_cmp);
        times.dedup();
        let mut player = CuePlayer::from_sheet(&talk.sheet.with_absolute_start_time(anchor));
        let sink = self.sink.clone();
        if delivery == Delivery::WithoutPreview {
            player.register_sink(Box::new(EmitOnly(sink)));
        } else {
            player.register_sink(Box::new(sink));
        }
        pump_until_idle();
        Talk {
            player,
            anchor,
            times,
        }
    }

    /// 台本に無い合図を、再生機が流すのと同じ UI の待ち行列へ直接 1 つ流して汲み出す。
    pub(super) fn emit(&self, cue: TalkCue) {
        self.sink.clone().emit(cue);
        pump_until_idle();
    }

    /// 場所の行の列（本番の提示と同じ `arrange_lines` に決まった字幅を渡す）。状態が無ければ空。
    pub(super) fn lines(&self, place: &PlaceKey, t: f64) -> Vec<PositionedLine> {
        self.runtime
            .borrow_mut()
            .arrange_for_test(place, &FixedMetrics, t)
            .unwrap_or_default()
    }

    /// 場所の行ごとの字の並び。
    pub(super) fn rows(&self, place: &PlaceKey, t: f64) -> Vec<Vec<String>> {
        self.lines(place, t)
            .iter()
            .map(|line| line.glyphs.iter().map(|g| g.text.to_string()).collect())
            .collect()
    }
}

/// 刻印済みの再生機 1 つと、その台本の判定の時刻（トークの頭からの秒・昇順）。
pub(super) struct Talk {
    player: CuePlayer,
    anchor: f64,
    pub(super) times: Vec<f64>,
}

impl Talk {
    /// トークの頭から `t` 秒の絶対時刻で `tick` し、UI の待ち行列を汲み出す。
    pub(super) fn tick(&mut self, t: f64) {
        self.player.tick(self.anchor + t);
        pump_until_idle();
    }
}

/// 台本を本番の経路で流し、合図が届く時刻と出終わる時刻のそれぞれで行の列を取る。
/// 最後の段階が「出終わったとき」になる。
pub(super) fn walk(
    script: &str,
    actor: &str,
    model: &BalloonModel,
    image: (u32, u32),
) -> Vec<Stage> {
    walk_with(script, actor, model, image, Delivery::Staged)
}

/// [`walk`] の届け方を選べる形。
pub(super) fn walk_with(
    script: &str,
    actor: &str,
    model: &BalloonModel,
    image: (u32, u32),
    delivery: Delivery,
) -> Vec<Stage> {
    walk_injecting(script, actor, model, image, delivery, None)
}

/// [`walk_with`] に、台本に無い合図を同じ場所へ直接 1 つ流す口を足した形。`stray = Some((i, 命令))`
/// なら、i 番目の判定の時刻の `tick` と汲み出しの後で、その時刻・長さ 0 の合図を本物の
/// [`EmoTextSink`](crate::sink::EmoTextSink) の複製へ `emit` して汲み出してから行の列を取る
/// （再生機が流すのと同じ UI の待ち行列を通る）。
pub(super) fn walk_injecting(
    script: &str,
    actor: &str,
    model: &BalloonModel,
    image: (u32, u32),
    delivery: Delivery,
    mut stray: Option<(usize, CueCommand)>,
) -> Vec<Stage> {
    let mut rig = Rig::new();
    rig.attach(actor, model, image);
    let mut talk = rig.play(script, ANCHOR, delivery);
    let actor = ActorKey::from(actor);
    let place = PlaceKey::balloon(&actor);
    if delivery == Delivery::AllAtOnce
        && let Some(&end) = talk.times.last()
    {
        talk.tick(end);
    }

    let mut stages = Vec::new();
    for (i, t) in talk.times.clone().into_iter().enumerate() {
        if delivery != Delivery::AllAtOnce {
            talk.tick(t);
        }
        if let Some((_, command)) = stray.take_if(|(at, _)| *at == i) {
            rig.emit(TalkCue {
                at: t,
                actor: actor.clone(),
                command,
                duration: 0.0,
            });
        }
        stages.push(Stage {
            time: t,
            rows: rig.rows(&place, t),
        });
    }
    stages
}

/// 行ごとの字の並びを (行, 番目, 字) の列へ平らにする。
pub(super) fn cells(rows: &[Vec<String>]) -> Vec<(usize, usize, &str)> {
    rows.iter()
        .enumerate()
        .flat_map(|(line, row)| {
            row.iter()
                .enumerate()
                .map(move |(col, text)| (line, col, text.as_str()))
        })
        .collect()
}

/// 行の列を 1 行ずつの文字列で見せる（失敗の出力用）。
pub(super) fn show(rows: &[Vec<String>]) -> Vec<String> {
    rows.iter().map(|row| row.concat()).collect()
}

/// 判定の関数: (a) 前の段階で見えていた字が同じ行の同じ番目にあるか、
/// (b) 出終わったとき（最後の段階）の行の列の、その字までの部分と同じか。
/// 違反を 1 件 1 行の文で返す（空なら合格）。
pub(super) fn reveal_violations(stages: &[Stage]) -> Vec<String> {
    let mut out = Vec::new();
    let Some(last) = stages.last() else {
        out.push("段階が 1 つも無い".to_string());
        return out;
    };
    let fin = cells(&last.rows);
    for pair in stages.windows(2) {
        let (prev, now) = (cells(&pair[0].rows), cells(&pair[1].rows));
        if now.len() < prev.len() || now[..prev.len()] != prev[..] {
            out.push(format!(
                "(a) t={:.3}→{:.3}: 見えていた字が動いた {:?} → {:?}",
                pair[0].time,
                pair[1].time,
                show(&pair[0].rows),
                show(&pair[1].rows)
            ));
        }
    }
    for stage in stages {
        let now = cells(&stage.rows);
        if now.len() > fin.len() || fin[..now.len()] != now[..] {
            out.push(format!(
                "(b) t={:.3}: 出終わったときと違う {:?}（出終わったとき {:?}）",
                stage.time,
                show(&stage.rows),
                show(&last.rows)
            ));
        }
    }
    out
}

// ── 検査 ──

/// 検査 1 の台本: emo2 の初回起動トークの写し（エモ側）。
pub(super) const EMO2_BOOT_TALK: &str = "\\1イイジャン！\\_w[450]‥\\_w[150]‥\\_w[150]ええと、";
/// 検査 1 の台本の出終わったときの区切り（欄の幅の選び方の前提）。
pub(super) const EMO2_BOOT_FINAL: [&str; 2] = ["イイジャン！", "‥‥ええと、"];

/// 検査 1: emo2 の初回起動トークの写し（エモ側）。「イイジャン！」「‥」「‥」「ええと、」が
/// 待ちのタグで別々の合図になって届いても、表示済みの字は動かず、「‥」は最初から
/// 出終わったときと同じ 2 行目の頭に出る（要件 1.1・1.2・1.4・2.5・5.1・5.2）。
#[test]
fn emo2_boot_talk_reveals_without_moving_glyphs() {
    let stages = walk(EMO2_BOOT_TALK, "1", &budoux_model(), NINE_WIDE);
    let last = stages.last().expect("段階がある");
    assert_eq!(
        show(&last.rows),
        EMO2_BOOT_FINAL,
        "出終わったときの区切り（欄の幅の選び方の前提）"
    );

    let mut violations = reveal_violations(&stages);
    // 「‥」が見え始めた最初の段階で、その「‥」が 2 行目の頭にあること。
    if let Some(stage) = stages
        .iter()
        .find(|s| cells(&s.rows).iter().any(|c| c.2 == "‥"))
    {
        let first = cells(&stage.rows).into_iter().find(|c| c.2 == "‥");
        if first.map(|c| (c.0, c.1)) != Some((1, 0)) {
            violations.push(format!(
                "t={:.3}: 最初の「‥」が 2 行目の頭に無い {:?}",
                stage.time,
                show(&stage.rows)
            ));
        }
    } else {
        violations.push("「‥」が一度も見えない".to_string());
    }
    assert!(
        violations.is_empty(),
        "表示済みの字が動いた:\n{}",
        violations.join("\n")
    );
}

/// 検査 2（対照）: 検査 1 の台本を、先渡しを落とす包み越しに流す（修正前の届き方）。判定の関数が
/// 「表示済みの字が動いた」と答えること（＝修正前の動きを赤と判定できること）、出終わった行の列が
/// 検査 1 と同じこと、字が 1 つも欠けていないこと、warn が場所・区間につき 1 件であることを見る
/// （要件 2.7・3.2・5.3・5.4）。
#[test]
fn dropping_the_preview_is_judged_as_moved_glyphs() {
    let (stages, events) = capture(|| {
        walk_with(
            EMO2_BOOT_TALK,
            "1",
            &budoux_model(),
            NINE_WIDE,
            Delivery::WithoutPreview,
        )
    });
    let violations = reveal_violations(&stages);
    assert!(
        violations.iter().any(|v| v.starts_with("(a)")),
        "先渡しが無ければ表示済みの字が動くはず（判定の関数が動きを拾えていない）: {violations:?}"
    );
    let last = stages.last().expect("段階がある");
    assert_eq!(
        show(&last.rows),
        EMO2_BOOT_FINAL,
        "出終わった行の列は検査 1 と同じ"
    );
    assert_eq!(
        show(&last.rows).concat(),
        "イイジャン！‥‥ええと、",
        "届いた字が 1 つも欠けていない"
    );

    // この台本は場所 1 つ・区間 1 つ（消去が無い）なので、warn はちょうど 1 件。
    let warns: Vec<&CapturedEvent> = events
        .iter()
        .filter(|e| e.level == tracing::Level::WARN)
        .collect();
    assert_eq!(
        warns.len(),
        1,
        "warn は場所・区間につき 1 件: {:?}",
        warns.iter().map(|e| e.message()).collect::<Vec<_>>()
    );
    assert_eq!(warns[0].field_str("reason"), Some("先渡しが無い"));
}

/// 検査 3: 同じ字を待ちのタグなしで書いた台本（1 つの合図で届く）の出終わった行の列が検査 1 と
/// 同じで、修正前の呼び方（先渡しを落とす＝届いた字で区切り → 配置）の結果とも同じ（要件 1.3・3.1）。
#[test]
fn one_cue_lays_out_like_the_staged_talk_and_the_old_call() {
    const ONE_CUE: &str = "\\1イイジャン！‥‥ええと、";
    let staged = walk(EMO2_BOOT_TALK, "1", &budoux_model(), NINE_WIDE);
    let one = walk(ONE_CUE, "1", &budoux_model(), NINE_WIDE);
    let old = walk_with(
        ONE_CUE,
        "1",
        &budoux_model(),
        NINE_WIDE,
        Delivery::WithoutPreview,
    );
    let rows = |stages: &[Stage]| show(&stages.last().expect("段階がある").rows);
    assert_eq!(
        rows(&one),
        rows(&staged),
        "1 つの合図でも検査 1 と同じ区切り"
    );
    assert_eq!(rows(&one), rows(&old), "1 つの合図なら修正前の呼び方と同じ");
}

/// 検査 4: 1 度の `tick` で全部を届けてから取った行の列が、同じ時刻の検査 1 の行の列と同じ
/// （同じ描画の回にまとめて届いても結果が変わらない・要件 1.5）。
#[test]
fn delivering_everything_in_one_tick_matches_the_staged_talk() {
    let staged = walk(EMO2_BOOT_TALK, "1", &budoux_model(), NINE_WIDE);
    let at_once = walk_with(
        EMO2_BOOT_TALK,
        "1",
        &budoux_model(),
        NINE_WIDE,
        Delivery::AllAtOnce,
    );
    assert_eq!(staged.len(), at_once.len(), "判定の時刻の数が同じ");
    let differ: Vec<String> = staged
        .iter()
        .zip(&at_once)
        .filter(|(a, b)| a.rows != b.rows)
        .map(|(a, b)| {
            format!(
                "t={:.3}: 1 つずつ {:?} ／ まとめて {:?}",
                a.time,
                show(&a.rows),
                show(&b.rows)
            )
        })
        .collect();
    assert!(
        differ.is_empty(),
        "まとめて届くと行の列が違う:\n{}",
        differ.join("\n")
    );
}

/// 捕まえた記録のうち warn だけ。
pub(super) fn warns(events: &[CapturedEvent]) -> Vec<&CapturedEvent> {
    events
        .iter()
        .filter(|e| e.level == tracing::Level::WARN)
        .collect()
}

/// 段階の列を行の列（1 行ずつの文字列）の列にする（失敗の出力と比べ合わせ用）。
pub(super) fn rows_of(stages: &[Stage]) -> Vec<Vec<String>> {
    stages.iter().map(|s| show(&s.rows)).collect()
}

/// 検査 5 の消去の前に置く待ち。消去の時刻を、消去の前の字が出終わる時刻から離す。
const PAUSE_BEFORE_CLEAR: &str = "\\_w[300]";
/// 検査 5 の消去の後の字（検査 1 と同じ分かれ方で、字が違う）。
const AFTER_CLEAR: &str = "イイジャン？\\_w[450]‥\\_w[150]‥\\_w[150]ええと。";

/// 検査 5: 途中に `\c` のある台本。消去の前の段階は、消去の前の字だけの台本（検査 1 の台本）の
/// 段階と 1 対 1 で同じ時刻・同じ行の列。消去の時刻から後の段階は、消去の後の字だけの台本の段階と
/// 1 対 1 で同じ行の列（どちらの台本も各合図の発火時刻ちょうどで刻むので、番目で対応づける）。
/// 消去の前の字は消去の後の区切りに効かない（要件 4.1・5.4）。
#[test]
fn clear_splits_the_talk_into_independent_sections() {
    let script = format!("{EMO2_BOOT_TALK}{PAUSE_BEFORE_CLEAR}\\c{AFTER_CLEAR}");
    let clear_at = areka_sakura::compile(
        &areka_parsers::sakura::parse(&script),
        &SystemVarSnapshot::default(),
    )
    .sheet
    .cues()
    .iter()
    .find(|c| matches!(c.payload, CuePayload::Command(CueCommand::Clear)))
    .expect("消去の合図がある")
    .start_time;
    let both = walk(&script, "1", &budoux_model(), NINE_WIDE);
    let before = walk(EMO2_BOOT_TALK, "1", &budoux_model(), NINE_WIDE);
    let after = walk(
        &format!("\\1{AFTER_CLEAR}"),
        "1",
        &budoux_model(),
        NINE_WIDE,
    );

    // 前提: それぞれの字だけの台本は単独で安定している。
    for (name, stages) in [("消去の前の字だけ", &before), ("消去の後の字だけ", &after)]
    {
        let violations = reveal_violations(stages);
        assert!(
            violations.is_empty(),
            "{name}の台本で表示済みの字が動いた:\n{}",
            violations.join("\n")
        );
    }

    let (pre, post): (Vec<Stage>, Vec<Stage>) = both.into_iter().partition(|s| s.time < clear_at);
    let line = |s: &Stage| format!("t={:?} {:?}", s.time, show(&s.rows));
    let lines = |stages: &[Stage]| stages.iter().map(line).collect::<Vec<_>>();
    assert_eq!(
        pre.iter().map(|s| (s.time, &s.rows)).collect::<Vec<_>>(),
        before.iter().map(|s| (s.time, &s.rows)).collect::<Vec<_>>(),
        "消去の前の段階が字だけの台本と 1 対 1 で同じでない:\n通し {:#?}\n字だけ {:#?}",
        lines(&pre),
        lines(&before)
    );
    assert_eq!(
        rows_of(&post),
        rows_of(&after),
        "消去の後の段階が字だけの台本と 1 対 1 で同じでない:\n通し {:#?}\n字だけ {:#?}",
        lines(&post),
        lines(&after)
    );
}

/// 検査 6 で台本に無い字の合図を流す段階（検査 1 の台本の「イイジャン！」が出終わった t=0.3）。
const STRAY_AFTER_STAGE: usize = 1;
/// 検査 6 で流す台本に無い字。
const STRAY_TEXT: &str = "ね";

/// 検査 6: 先渡しの後、台本に無い字の合図を同じ場所へ直接 1 つ流す。届いた字が行の列に全部あり、
/// 修正前の呼び方（先渡しを落とした同じ流れ）の結果と全段階で同じで、warn が「食い違う」の 1 件
/// （要件 2.7・5.4）。
#[test]
fn a_stray_cue_falls_back_to_the_arrived_text_without_losing_glyphs() {
    let stray = || Some((STRAY_AFTER_STAGE, CueCommand::Text(STRAY_TEXT.to_string())));
    let (stages, events) = capture(|| {
        walk_injecting(
            EMO2_BOOT_TALK,
            "1",
            &budoux_model(),
            NINE_WIDE,
            Delivery::Staged,
            stray(),
        )
    });
    let old = walk_injecting(
        EMO2_BOOT_TALK,
        "1",
        &budoux_model(),
        NINE_WIDE,
        Delivery::WithoutPreview,
        stray(),
    );
    let last = stages.last().expect("段階がある");
    assert_eq!(
        show(&last.rows).concat(),
        format!("イイジャン！{STRAY_TEXT}‥‥ええと、"),
        "届いた字が 1 つも欠けていない"
    );
    assert_eq!(
        rows_of(&stages),
        rows_of(&old),
        "修正前の呼び方（先渡しなし）と同じ"
    );
    let warns = warns(&events);
    assert_eq!(
        warns.len(),
        1,
        "warn は食い違った場所・区間につき 1 件: {:?}",
        warns.iter().map(|e| e.message()).collect::<Vec<_>>()
    );
    assert_eq!(
        warns[0].field_str("reason"),
        Some("届いた字が先渡しと食い違う")
    );
}

/// 検査 7: `budoux_newline` の無いバルーンで検査 1 の台本。各段階で安定し、修正前の呼び方
/// （先渡しを落とす＝届いた字で 1 字ずつの折り返し）と全段階で同じ。先渡しを落としても warn が 0 件
/// （1 字ずつの折り返しは区間の全文に触れない・要件 3.3・3.4・5.4）。
#[test]
fn char_by_char_wrap_is_stable_and_never_consults_the_lookahead() {
    let (staged, staged_events) =
        capture(|| walk(EMO2_BOOT_TALK, "1", &char_by_char_model(), NINE_WIDE));
    let (old, old_events) = capture(|| {
        walk_with(
            EMO2_BOOT_TALK,
            "1",
            &char_by_char_model(),
            NINE_WIDE,
            Delivery::WithoutPreview,
        )
    });
    let violations = reveal_violations(&staged);
    assert!(
        violations.is_empty(),
        "1 字ずつの折り返しで表示済みの字が動いた:\n{}",
        violations.join("\n")
    );
    assert_eq!(
        rows_of(&staged),
        rows_of(&old),
        "修正前の呼び方（1 字ずつの折り返し）と同じ"
    );
    for (name, events) in [("先渡しあり", &staged_events), ("先渡しなし", &old_events)] {
        let warns = warns(events);
        assert!(
            warns.is_empty(),
            "{name}で warn が出た（1 字ずつの折り返しが区間の全文に触れた）: {:?}",
            warns.iter().map(|e| e.message()).collect::<Vec<_>>()
        );
    }
}

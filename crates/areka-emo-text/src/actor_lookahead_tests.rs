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
use areka_sakura::contract::ActorKey;
use areka_sakura::sysvar::SystemVarSnapshot;
use bevy_ecs::prelude::World;
use dola::cue::{CuePlayer, CueSink, TalkCue};
use log_capture_kit::{CapturedEvent, capture};

use super::test_support::pump_until_idle;
use super::{ResolvedBalloonText, TextLayerRuntime, TextSlotBinding, spawn_emo_text};
use crate::layout::FixedMetrics;
use crate::place::PlaceKey;
use crate::state::TextLayerConfig;

// ── 経路を歩く支え ──

/// 刻印する絶対の開始時刻。0 でない値にして、絶対の時刻と相対の時刻の取り違えを拾う。
const ANCHOR: f64 = 100.0;
/// 字の丈（`FixedMetrics` では全角 1 字の送り幅＝字の丈）。
const FONT_HEIGHT: u32 = 20;
/// 欄の幅: 全角 9 字（180）が入り、10 字目（200）が入らない値。
/// 「イイジャン！‥‥」の 8 字は 1 行に入り、「‥‥ええと、」の 6 字は「イイジャン！」の後ろに入らない。
const NINE_WIDE: (u32, u32) = (190, 200);

/// 文節の折り返し（`budoux_newline,1`）のバルーン。描画範囲・折り返しの基準は面の全域
/// （validrect・wordwrappoint を書かない＝右辺で折り返す）、原点は (0,0)。
pub(super) fn budoux_model() -> BalloonModel {
    BalloonModel::new(
        WindowPosition::new(None, None),
        Origin::new(Some(0), Some(0)),
        WordWrapPoint::new(None, None),
        ValidRect::new(None, None, None, None),
        Font::new(None, Some(FONT_HEIGHT), FontColor::new(None, None, None)),
        None,
        Some("1".to_string()),
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

    let sheet = talk.sheet.with_absolute_start_time(ANCHOR);
    let mut player = CuePlayer::from_sheet(&sheet);

    let actor = ActorKey::from(actor);
    let place = PlaceKey::balloon(&actor);
    let runtime = Rc::new(RefCell::new(TextLayerRuntime::new(
        TextLayerConfig::default(),
    )));
    let mut world = World::new();
    let slot = world.spawn_empty().id();
    let window = world.spawn_empty().id();
    runtime.borrow_mut().register_actor(
        actor.clone(),
        TextSlotBinding::new(slot, window, 1.0, image, image),
        ResolvedBalloonText::resolve(model, image),
    );
    let (sink, _handle) =
        spawn_emo_text(Rc::clone(&runtime)).expect("spawn_emo_text on the pump thread");
    if delivery == Delivery::WithoutPreview {
        player.register_sink(Box::new(EmitOnly(sink)));
    } else {
        player.register_sink(Box::new(sink));
    }
    pump_until_idle();
    if delivery == Delivery::AllAtOnce
        && let Some(&end) = times.last()
    {
        player.tick(ANCHOR + end);
        pump_until_idle();
    }

    let mut stages = Vec::new();
    for t in times {
        if delivery != Delivery::AllAtOnce {
            player.tick(ANCHOR + t);
            pump_until_idle();
        }
        let lines = runtime
            .borrow_mut()
            .arrange_for_test(&place, &FixedMetrics, t)
            .unwrap_or_default();
        let rows = lines
            .iter()
            .map(|line| line.glyphs.iter().map(|g| g.text.to_string()).collect())
            .collect();
        stages.push(Stage { time: t, rows });
    }
    stages
}

/// 行ごとの字の並びを (行, 番目, 字) の列へ平らにする。
fn cells(rows: &[Vec<String>]) -> Vec<(usize, usize, &str)> {
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
fn show(rows: &[Vec<String>]) -> Vec<String> {
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
const EMO2_BOOT_TALK: &str = "\\1イイジャン！\\_w[450]‥\\_w[150]‥\\_w[150]ええと、";
/// 検査 1 の台本の出終わったときの区切り（欄の幅の選び方の前提）。
const EMO2_BOOT_FINAL: [&str; 2] = ["イイジャン！", "‥‥ええと、"];

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

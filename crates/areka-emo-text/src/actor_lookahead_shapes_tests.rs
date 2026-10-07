//! # 本番と同じ経路を歩く検査の続き（台本の形・中断・後から届く `\f`・起動直後の順・要件 1・2・4）
//!
//! 経路の土台（[`Rig`]・[`Talk`]）と判定の関数（[`reveal_violations`]）は
//! `actor_lookahead_tests.rs` のものをそのまま使う（検査 1〜7 と同じ経路・同じ判定）。
//! ここでは検査 1 の形を、書字方向・改行・選択肢・2 つの場所に広げ（検査 8）、トークの中断と
//! 次のトーク（検査 9）、字の無い行の後の字と後から届く `\f[height,…]`（検査 10）、
//! 先渡しがバルーンの装着・箱の束より先に届く起動直後の順（検査 11）を見る。

use areka_sakura::contract::{ActorKey, FONT_TAG_CARRIER};
use dola::cue::{CueCommand, CuePayload};
use log_capture_kit::capture;

use super::box_sync_tests::{layout, resolver};
use super::lookahead_tests::{
    ANCHOR, Delivery, EMO2_BOOT_FINAL, EMO2_BOOT_TALK, NINE_WIDE, Rig, Stage, Talk, balloon_model,
    budoux_model, reveal_violations, rows_of, show, walk, warns,
};
use crate::choice::{annotate_lines, derive_hit_rows, line_bands};
use crate::layout::FixedMetrics;
use crate::place::PlaceKey;
use crate::state::SurfaceKeyOutcome;

/// 場所ごとに、台本の判定の時刻のそれぞれで `tick` して行の列を取る（返す列は `places` と同じ順）。
fn walk_places(rig: &Rig, talk: &mut Talk, places: &[PlaceKey]) -> Vec<Vec<Stage>> {
    let mut out = vec![Vec::new(); places.len()];
    for t in talk.times.clone() {
        talk.tick(t);
        for (stages, place) in out.iter_mut().zip(places) {
            stages.push(Stage {
                time: t,
                rows: rig.rows(place, t),
            });
        }
    }
    out
}

/// スコープ名の列を普通のバルーンの場所の列にする。
fn balloons(actors: &[&str]) -> Vec<PlaceKey> {
    actors
        .iter()
        .map(|a| PlaceKey::balloon(&ActorKey::from(*a)))
        .collect()
}

// ── 検査 8: 縦書き・改行・選択肢・2 つの場所 ──

/// 検査 8 の台本の形 1 つ。
struct Shape {
    name: &'static str,
    script: &'static str,
    /// バルーンの `writing_mode`（`None`＝横書き）。
    writing_mode: Option<&'static str>,
    image: (u32, u32),
    /// 字を出すスコープ（それぞれの普通のバルーンを判定する）。
    actors: &'static [&'static str],
    /// 出終わったときの行の列（どのスコープも同じ）。
    last: &'static [&'static str],
}

/// 縦書きの面: 行内の軸（縦）は全角 9 字（180）が入り 10 字目（200）が入らない。横の幅は 2 列が入るだけの
/// 狭さにして、縦書きの指定が効かずに横書きで配置されると区切りが変わって赤になるようにする。
const NINE_TALL: (u32, u32) = (70, NINE_WIDE.0);

/// 選択肢の前に並ぶ字（選択肢の字はこの後ろに出る）。
const BEFORE_CHOICE: &str = "イイジャン！‥‥";

const SHAPES: [Shape; 5] = [
    Shape {
        name: "縦書き",
        script: EMO2_BOOT_TALK,
        writing_mode: Some("vertical_rl"),
        image: NINE_TALL,
        actors: &["1"],
        last: &EMO2_BOOT_FINAL,
    },
    Shape {
        name: "改行を挟む",
        script: "\\1ねえ\\_w[300]\\nイイジャン！\\_w[450]‥\\_w[150]‥\\_w[150]ええと、",
        writing_mode: None,
        image: NINE_WIDE,
        actors: &["1"],
        last: &["ねえ", "イイジャン！", "‥‥ええと、"],
    },
    Shape {
        name: "割合付きの改行を挟む",
        script: "\\1ねえ\\_w[300]\\n[half]イイジャン！\\_w[450]‥\\_w[150]‥\\_w[150]ええと、",
        writing_mode: None,
        image: NINE_WIDE,
        actors: &["1"],
        last: &["ねえ", "イイジャン！", "‥‥ええと、"],
    },
    Shape {
        name: "選択肢",
        script: "\\1イイジャン！\\_w[450]‥\\_w[150]‥\\_w[150]\\q[ええと、,OnAnswer]",
        writing_mode: None,
        image: NINE_WIDE,
        actors: &["1"],
        last: &EMO2_BOOT_FINAL,
    },
    Shape {
        name: "本体側と相方側が交互",
        script: "\\0イイジャン！\\_w[450]\\1イイジャン！\\_w[450]\\0‥\\_w[150]\\1‥\\_w[150]\
                 \\0‥\\_w[150]\\1‥\\_w[150]\\0ええと、\\_w[150]\\1ええと、",
        writing_mode: None,
        image: NINE_WIDE,
        actors: &["0", "1"],
        last: &EMO2_BOOT_FINAL,
    },
];

/// 形 1 つを本番の経路で流し、判定するスコープごとの段階の列を返す。
fn walk_shape(shape: &Shape, delivery: Delivery) -> Vec<Vec<Stage>> {
    let model = balloon_model(shape.writing_mode, Some("1"));
    let mut rig = Rig::new();
    for actor in shape.actors {
        rig.attach(actor, &model, shape.image);
    }
    let mut talk = rig.play(shape.script, ANCHOR, delivery);
    walk_places(&rig, &mut talk, &balloons(shape.actors))
}

/// 検査 8: 検査 1 の形を、縦書きのバルーン・`\n`（割合付きを含む）を挟む台本・`\q` の選択肢を
/// 含む台本・本体側と相方側が交互に話す台本で流し、スコープごとに同じ判定の関数にかける。
/// どの形も、先渡しを落とした対照では「動いた」と判定される（＝形が折り返しの付け替えを踏んでいる）
/// （要件 1.1・1.2・1.6・1.7・4.4・4.5）。
#[test]
fn every_shape_reveals_without_moving_glyphs() {
    let mut failures = Vec::new();
    for shape in &SHAPES {
        let staged = walk_shape(shape, Delivery::Staged);
        for (actor, stages) in shape.actors.iter().zip(&staged) {
            let last = stages.last().expect("段階がある");
            if show(&last.rows) != shape.last {
                failures.push(format!(
                    "{}（\\{actor}）: 出終わったときの区切りが前提と違う {:?}",
                    shape.name,
                    show(&last.rows)
                ));
            }
            for v in reveal_violations(stages) {
                failures.push(format!("{}（\\{actor}）: {v}", shape.name));
            }
        }
        let control = walk_shape(shape, Delivery::WithoutPreview);
        if !control.iter().any(|stages| {
            reveal_violations(stages)
                .iter()
                .any(|v| v.starts_with("(a)"))
        }) {
            failures.push(format!(
                "{}: 先渡しを落とした対照で字が動かない（形が折り返しの付け替えを踏んでいない）",
                shape.name
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// 検査 8 の選択肢: 各段階で、選択肢の強調の範囲（`annotate_lines`）とクリックの範囲
/// （`derive_hit_rows`）を、判定にかけたのと同じ行の列から本番の提示と同じ関数で出し、強調の範囲が
/// 覆う字が行の列に見えている選択肢の字と同じで、クリックの範囲が強調と同じ行内の範囲であること
/// （要件 4.4）。
#[test]
fn choice_highlight_and_click_ranges_come_from_the_judged_rows() {
    let shape = &SHAPES[3];
    let place = PlaceKey::balloon(&ActorKey::from(shape.actors[0]));
    let mut rig = Rig::new();
    rig.attach(shape.actors[0], &budoux_model(), shape.image);
    let mut talk = rig.play(shape.script, ANCHOR, Delivery::Staged);
    let mut failures = Vec::new();
    let mut covered_at_end = String::new();
    for t in talk.times.clone() {
        talk.tick(t);
        let lines = rig.lines(&place, t);
        let rt = rig.runtime.borrow();
        let resolved = &rt.layout_input[&place];
        let spans = rt
            .state
            .place_state(&place)
            .map(|s| s.choices().to_vec())
            .unwrap_or_default();
        let segments = annotate_lines(&lines, &spans);
        let bands = line_bands(&lines, resolved.mode, &FixedMetrics);
        let hits = derive_hit_rows(&lines, &segments, resolved.mode, &resolved.region, &bands);

        let shown: String = lines
            .iter()
            .flat_map(|l| l.glyphs.iter().map(|g| g.text.to_string()))
            .collect();
        let expected = shown.strip_prefix(BEFORE_CHOICE).unwrap_or_default();
        let covered: String = segments
            .iter()
            .flat_map(|seg| {
                lines[seg.line_index]
                    .glyphs
                    .iter()
                    .filter(|g| (seg.inline_range.0..seg.inline_range.1).contains(&g.inline_pos))
                    .map(|g| g.text.to_string())
            })
            .collect();
        if covered != expected {
            failures.push(format!(
                "t={t:.3}: 強調の範囲が覆う字 {covered:?} が見えている選択肢の字 {expected:?} と違う"
            ));
        }
        let ranges: Vec<_> = segments
            .iter()
            .map(|s| (s.ordinal, s.inline_range.0, s.inline_range.1))
            .collect();
        let clicks: Vec<_> = hits
            .iter()
            .map(|h| (h.ordinal, h.rect.left, h.rect.right))
            .collect();
        if clicks != ranges {
            failures.push(format!(
                "t={t:.3}: クリックの範囲 {clicks:?} が強調の範囲 {ranges:?} と違う"
            ));
        }
        covered_at_end = covered;
    }
    assert_eq!(
        covered_at_end, "ええと、",
        "出終わったときは選択肢の字が全部強調の範囲にある"
    );
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

// ── 検査 9: 中断と次のトーク ──

/// 検査 9 の 2 本目の台本（検査 1 と同じ分かれ方で、字が違う）。
const NEXT_TALK: &str = "\\1イイジャン？\\_w[450]‥\\_w[150]‥\\_w[150]ええと。";
/// 2 本目のトークの刻印（1 本目の後）。
const NEXT_ANCHOR: f64 = ANCHOR + 50.0;
/// 中断の後に「時刻だけ進めた」とする時刻（1 本目の台本がとうに出終わっている時刻）。
const LATER: f64 = 10.0;

/// 検査 9: 検査 1 の台本を最初の「‥」が出終わったところで止める（以後 1 本目は `tick` しない）。
/// 止めるまでの行の列は通しと同じで、時刻だけ進めても変わらず、2 本目の再生機を登録だけして
/// （先渡しだけが届く）も変わらない。2 本目を `tick` した後は、2 本目の字だけの台本と各段階で
/// 同じ行の列（要件 2.6）。
#[test]
fn an_interrupted_talk_keeps_its_rows_and_the_next_talk_lays_out_on_its_own() {
    let place = PlaceKey::balloon(&ActorKey::from("1"));
    let whole = walk(EMO2_BOOT_TALK, "1", &budoux_model(), NINE_WIDE);
    let stop = whole
        .iter()
        .position(|s| show(&s.rows).concat() == "イイジャン！‥")
        .expect("最初の「‥」だけが出終わった段階がある");

    let mut rig = Rig::new();
    rig.attach("1", &budoux_model(), NINE_WIDE);
    let mut first = rig.play(EMO2_BOOT_TALK, ANCHOR, Delivery::Staged);
    let mut until_stop = Vec::new();
    for t in first.times[..=stop].to_vec() {
        first.tick(t);
        until_stop.push(Stage {
            time: t,
            rows: rig.rows(&place, t),
        });
    }
    drop(first);
    assert_eq!(
        rows_of(&until_stop),
        rows_of(&whole[..=stop]),
        "止めるまでは通しと同じ"
    );
    let stopped = show(&until_stop[stop].rows);
    assert_eq!(
        show(&rig.rows(&place, LATER)),
        stopped,
        "時刻だけ進めても変わらない"
    );

    let mut next = rig.play(NEXT_TALK, NEXT_ANCHOR, Delivery::Staged);
    assert_eq!(
        show(&rig.rows(&place, LATER)),
        stopped,
        "2 本目の先渡しだけが届いても変わらない"
    );
    let next_stages = walk_places(&rig, &mut next, std::slice::from_ref(&place)).remove(0);
    let alone = walk(NEXT_TALK, "1", &budoux_model(), NINE_WIDE);
    assert_eq!(
        rows_of(&next_stages),
        rows_of(&alone),
        "2 本目は 2 本目の字だけの台本と同じ"
    );
}

// ── 検査 10: 字の無い行の後の字と、後から届く `\f[height,…]` ──

/// 検査 10 の台本: 区間の頭が「改行 → 相対の `\_l` → 字 → 待ち → `\f[height,…]` → 字」。
const LATE_HEIGHT_TALK: &str =
    "\\1\\n\\_l[@10,@1em]イイジャン！\\_w[450]\\f[height,30]‥\\_w[150]ええと、";

/// 検査 10: 最初の字の行内の位置と行の上下の位置が、`\f` の合図が届く前と後で同じで、出終わったとき
/// とも同じ（要件 1.1・1.2）。
#[test]
fn a_late_font_height_does_not_move_the_first_glyph() {
    let place = PlaceKey::balloon(&ActorKey::from("1"));
    let font_at = areka_sakura::compile(
        &areka_parsers::sakura::parse(LATE_HEIGHT_TALK),
        &areka_sakura::sysvar::SystemVarSnapshot::default(),
    )
    .sheet
    .cues()
    .iter()
    .find(|c| {
        matches!(&c.payload, CuePayload::Command(CueCommand::Custom { command, .. }) if command == FONT_TAG_CARRIER)
    })
    .expect("\\f の合図がある")
    .start_time;

    let mut rig = Rig::new();
    rig.attach("1", &budoux_model(), NINE_WIDE);
    let mut talk = rig.play(LATE_HEIGHT_TALK, ANCHOR, Delivery::Staged);
    // 最初の字が見えている段階ごとの（時刻, 字, 行内の位置, 行の上辺, 行の下辺）。
    let mut seen = Vec::new();
    for t in talk.times.clone() {
        talk.tick(t);
        let lines = rig.lines(&place, t);
        if let Some((line, glyph)) = lines.iter().find_map(|l| l.glyphs.first().map(|g| (l, g))) {
            seen.push((
                t,
                glyph.text.to_string(),
                glyph.inline_pos,
                line.rect.top,
                line.rect.bottom,
            ));
        }
    }
    assert!(
        seen.iter().any(|s| s.0 < font_at) && seen.iter().any(|s| s.0 >= font_at),
        "\\f の前と後の両方で最初の字が見えている: {seen:?}"
    );
    let last = seen.last().expect("最初の字が見えている").clone();
    assert_eq!(last.1, "イ", "最初の字");
    let moved: Vec<_> = seen
        .iter()
        .filter(|s| (s.2, s.3, s.4) != (last.2, last.3, last.4))
        .collect();
    assert!(
        moved.is_empty(),
        "最初の字の位置が出終わったときと違う（出終わったとき {last:?}）: {moved:?}"
    );
}

// ── 検査 11: 起動直後の順 ──

/// 空回しの出発点が、先渡しの後に合図と無関係に変わる 3 か所。
#[derive(Clone, Copy, Debug)]
enum LateStart {
    /// 装着の登録（バルーンの定義の見た目が入る・actor_attach.rs）。
    Attach,
    /// 箱の束の受け取り（箱の表と `\s` の解決の閉包が入る・actor_box.rs）。
    BoxLayout,
    /// `\s` の解決の閉包だけの差し込み（actor.rs の検査専用の口）。
    Resolver,
}

/// `\f` を含む検査 1 の形の台本（相方側）。
const BOLD_BOOT_TALK: &str = "\\1イイジャン！\\_w[450]‥\\_w[150]\\f[bold,1]‥\\_w[150]ええと、";
/// 頭で `\s[0]` の箱 a へ字を出してから、箱の無い `\s[4]` で普通のバルーンへ戻る台本（本体側）。
/// 解決の閉包か箱の表が無い空回しでは、箱 a の字が普通のバルーンの全文に混ざる。
const BOXED_BOOT_TALK: &str =
    "\\0\\s[0]ね\\_w[150]\\s[4]イイジャン！\\_w[450]‥\\_w[150]\\f[bold,1]‥\\_w[150]ええと、";

/// 検査 11: 再生機の登録（先渡しが届く）→ 汲み出し → 空回しの出発点を変える口（装着の登録・箱の束・
/// 解決の閉包）→ `tick` の順で、`\f` を含む検査 1 の形の台本を流す。どの順でも各段階で判定の関数が
/// 通り、出終わった行の列が検査 1 と同じで、warn が 0 件（要件 1.1・2.2・設計の討議 1）。
#[test]
fn a_preview_before_attaching_still_lays_out_on_the_full_text() {
    let mut failures = Vec::new();
    for late in [LateStart::Attach, LateStart::BoxLayout, LateStart::Resolver] {
        let (stages, events) = capture(|| {
            let mut rig = Rig::new();
            let (script, actor) = match late {
                LateStart::Attach => (BOLD_BOOT_TALK, "1"),
                LateStart::BoxLayout | LateStart::Resolver => (BOXED_BOOT_TALK, "0"),
            };
            match late {
                LateStart::Attach => {}
                LateStart::BoxLayout => rig.attach(actor, &budoux_model(), NINE_WIDE),
                LateStart::Resolver => {
                    rig.attach(actor, &budoux_model(), NINE_WIDE);
                    let never = Box::new(|_: &str| SurfaceKeyOutcome::Unresolved);
                    let mut rt = rig.runtime.borrow_mut();
                    rt.set_box_layout(&mut rig.world, layout(), never, vec![]);
                }
            }
            let mut talk = rig.play(script, ANCHOR, Delivery::Staged);
            match late {
                LateStart::Attach => rig.attach(actor, &budoux_model(), NINE_WIDE),
                LateStart::BoxLayout => {
                    let mut rt = rig.runtime.borrow_mut();
                    rt.set_box_layout(&mut rig.world, layout(), resolver(), vec![]);
                }
                LateStart::Resolver => rig.runtime.borrow_mut().set_surface_resolver(resolver()),
            }
            walk_places(&rig, &mut talk, &balloons(&[actor])).remove(0)
        });
        let last = stages.last().expect("段階がある");
        if show(&last.rows) != EMO2_BOOT_FINAL {
            failures.push(format!(
                "{late:?}: 出終わったときが検査 1 と違う {:?}",
                show(&last.rows)
            ));
        }
        for v in reveal_violations(&stages) {
            failures.push(format!("{late:?}: {v}"));
        }
        for w in warns(&events) {
            failures.push(format!(
                "{late:?}: warn {} {:?}",
                w.message(),
                w.field_str("reason")
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

//! アンカーの範囲の記録（`\_a` の開き → 伸長 → 閉じ → 消去）。
//!
//! 合図（`AnchorBegin`／`AnchorEnd`）だけから範囲を組み、範囲の位置（グリフ序数）・範囲の文字
//! （Reference0 になる）・通し番号・開いている印の寿命・既定の下線を固定する。箱の構成と組み立ての支援は
//! `state_route_tests.rs` のものを使う（0: a・b（既定は a） 3: 箱なし）。

use areka_sakura::contract::{FONT_TAG_CARRIER, TalkCue};
use log_capture_kit::capture;

use super::route_tests::{boxed, key, select, show, state_with_boxes};
use super::test_support::cue;
use super::*;
use crate::look::{StyleId, TextLook};
use crate::lookahead::{Basis, TalkLookahead, advance_state};

fn open(actor: &str, id: &str, references: &[&str]) -> TalkCue {
    cue(
        actor,
        0.0,
        CueCommand::AnchorBegin {
            id: id.to_owned(),
            references: references.iter().map(|r| (*r).to_owned()).collect(),
        },
    )
}

fn close(actor: &str) -> TalkCue {
    cue(actor, 0.0, CueCommand::AnchorEnd)
}

fn text(actor: &str, s: &str) -> TalkCue {
    cue(actor, 0.0, CueCommand::Text(s.into()))
}

fn choice(actor: &str, id: &str, s: &str) -> TalkCue {
    cue(
        actor,
        0.0,
        CueCommand::Choice {
            id: id.to_owned(),
            text: s.to_owned(),
            references: Vec::new(),
        },
    )
}

fn applied(cues: &[TalkCue]) -> TextLayerState {
    let mut state = TextLayerState::default();
    for c in cues {
        state.apply_cue(c);
    }
    state
}

/// スコープの普通のバルーンの場所の範囲の列（場所が無ければ空）。
fn spans(state: &TextLayerState, actor: &str) -> Vec<ChoiceSpan> {
    state
        .actor_state(&key(actor))
        .map(|s| s.choices().to_vec())
        .unwrap_or_default()
}

/// 箱の場所の範囲の列（場所が無ければ空）。
fn box_spans(state: &TextLayerState, actor: &str, name: &str) -> Vec<ChoiceSpan> {
    state
        .place_state(&PlaceKey {
            actor: key(actor),
            place: boxed(name),
        })
        .map(|s| s.choices().to_vec())
        .unwrap_or_default()
}

/// 引数の無いアンカーの範囲の期待値。
fn anchor(ordinal: usize, id: &str, label: &str, range: core::ops::Range<usize>) -> ChoiceSpan {
    ChoiceSpan {
        kind: SpanKind::Anchor,
        ordinal,
        id: id.to_owned(),
        label: label.to_owned(),
        references: Vec::new(),
        glyph_range: range,
    }
}

// ---- 開き → 文字 → 閉じ（要件 1.7・2.1）

/// 開きから閉じまでに追記された文字が範囲になり、前後の文字は入らない。
/// ID と引数は合図のまま写る。
#[test]
fn open_text_close_records_the_range_and_its_text() {
    let state = applied(&[
        text("0", "ま"),
        open("0", "x", &["r2", "", "r4"]),
        text("0", "あい"),
        close("0"),
        text("0", "う"),
    ]);
    assert_eq!(
        spans(&state, "0"),
        vec![ChoiceSpan {
            references: vec!["r2".to_owned(), String::new(), "r4".to_owned()],
            ..anchor(0, "x", "あい", 1..3)
        }]
    );
}

/// 範囲の位置は書記素クラスタで数え、範囲の文字はクラスタを連ねたもの。
#[test]
fn range_counts_grapheme_clusters() {
    let family = "\u{1F468}\u{200D}\u{1F469}\u{200D}\u{1F467}";
    let state = applied(&[
        open("0", "x", &[]),
        text("0", &format!("{family}e\u{301}")),
        close("0"),
    ]);
    assert_eq!(
        spans(&state, "0"),
        vec![anchor(0, "x", &format!("{family}e\u{301}"), 0..2)]
    );
}

/// 文字を挟まない対は、空の範囲（文字も空）を 1 件残す。
#[test]
fn empty_pair_records_an_empty_range() {
    let state = applied(&[text("0", "あ"), open("0", "x", &[]), close("0")]);
    assert_eq!(spans(&state, "0"), vec![anchor(0, "x", "", 1..1)]);
}

/// 改行・`\f`・`\_l` をまたいでも範囲は続き、範囲の文字に入るのは文字だけ。
#[test]
fn newline_and_decoration_between_join_only_characters() {
    let state = applied(&[
        open("0", "x", &[]),
        text("0", "あ"),
        cue("0", 0.0, CueCommand::NewLine { ratio: 1.0 }),
        cue(
            "0",
            0.0,
            CueCommand::command_carrier(FONT_TAG_CARRIER, vec!["bold".into(), "1".into()]),
        ),
        cue(
            "0",
            0.0,
            CueCommand::Cursor {
                x: "10".into(),
                y: String::new(),
            },
        ),
        text("0", "い"),
        close("0"),
    ]);
    assert_eq!(spans(&state, "0"), vec![anchor(0, "x", "あい", 0..2)]);
}

/// アンカーが選択肢を包むと、選択肢の文字も範囲と範囲の文字に入る。
/// 選択肢は自分の範囲を別に持ち、通し番号は 2 つで共通（列の添字）。
#[test]
fn anchor_wrapping_a_choice_includes_the_choice_text() {
    let state = applied(&[
        open("0", "x", &[]),
        text("0", "あ"),
        choice("0", "OnYes", "はい"),
        text("0", "う"),
        close("0"),
    ]);
    assert_eq!(
        spans(&state, "0"),
        vec![
            anchor(0, "x", "あはいう", 0..4),
            ChoiceSpan {
                kind: SpanKind::Choice,
                ordinal: 1,
                id: "OnYes".to_owned(),
                label: "はい".to_owned(),
                references: Vec::new(),
                glyph_range: 1..3,
            },
        ]
    );
}

// ---- 複数のアンカー（要件 2.8）

/// 1 つの台詞の 2 つのアンカーは、別の通し番号・別の範囲になる。あいだの文字はどちらにも入らない。
#[test]
fn two_anchors_get_different_ordinals() {
    let state = applied(&[
        open("0", "a", &[]),
        text("0", "あ"),
        close("0"),
        text("0", "ー"),
        open("0", "b", &[]),
        text("0", "いう"),
        close("0"),
    ]);
    assert_eq!(
        spans(&state, "0"),
        vec![anchor(0, "a", "あ", 0..1), anchor(1, "b", "いう", 2..4)]
    );
}

// ---- 到達しない防御（compile が補うので本番の列では起きない）

/// 開いている間の新たな開きは、直前の範囲をそこで閉じてから開く。
#[test]
fn reopening_closes_the_previous_range_there() {
    let state = applied(&[
        open("0", "a", &[]),
        text("0", "あ"),
        open("0", "b", &[]),
        text("0", "い"),
        close("0"),
        text("0", "う"),
    ]);
    assert_eq!(
        spans(&state, "0"),
        vec![anchor(0, "a", "あ", 0..1), anchor(1, "b", "い", 1..2)]
    );
}

/// 別の場所で開いたままの範囲も、新たな開きが閉じる。
#[test]
fn reopening_closes_a_range_left_open_in_another_place() {
    let state = applied(&[
        open("0", "a", &[]),
        text("0", "あ"),
        open("1", "b", &[]),
        text("1", "い"),
        text("0", "う"),
        close("1"),
    ]);
    assert_eq!(spans(&state, "0"), vec![anchor(0, "a", "あ", 0..1)]);
    assert_eq!(spans(&state, "1"), vec![anchor(0, "b", "い", 0..1)]);
}

/// 開いていないのに届いた閉じは、状態を変えない（場所も作らない）。
#[test]
fn stray_close_changes_nothing() {
    assert_eq!(applied(&[close("0")]), TextLayerState::default());

    let pair = [open("0", "x", &[]), text("0", "あ"), close("0")];
    let mut twice = pair.to_vec();
    twice.push(close("0"));
    twice.push(close("1"));
    assert_eq!(applied(&twice), applied(&pair));
}

// ---- 開いたままスコープや行き先が替わる（閉じは開いている範囲を持つ場所を閉じる）

/// `\_a[x]あ\1い\_a\0う`: 閉じはスコープ 1 宛てに届くが、スコープ 0 で開いている範囲を閉じる。
/// 範囲はスコープ 0 の「あ」だけ——「い」は別の場所の文字、「う」は閉じた後の文字。
#[test]
fn close_addressed_to_another_scope_closes_the_open_range() {
    let state = applied(&[
        open("0", "x", &[]),
        text("0", "あ"),
        text("1", "い"),
        close("1"),
        text("0", "う"),
    ]);
    assert_eq!(spans(&state, "0"), vec![anchor(0, "x", "あ", 0..1)]);
    assert_eq!(spans(&state, "1"), vec![], "別のスコープに範囲は出来ない");
}

/// 同じスコープで文字の行き先（箱）が替わっても同じ: 閉じは開いた箱の範囲を閉じる。
#[test]
fn close_after_a_destination_switch_closes_the_open_range() {
    let mut state = state_with_boxes();
    show(&mut state, "0", 0);
    for c in [
        open("0", "x", &[]),
        text("0", "あ"),
        select("0", "b"),
        text("0", "い"),
        close("0"),
        select("0", "a"),
        text("0", "う"),
    ] {
        state.apply_cue(&c);
    }
    assert_eq!(
        box_spans(&state, "0", "a"),
        vec![anchor(0, "x", "あ", 0..1)]
    );
    assert_eq!(box_spans(&state, "0", "b"), vec![]);
}

/// 開いたまま別の場所へ寄り道して戻ると、戻った後の文字は範囲に続く（寄り道の文字は入らない）。
#[test]
fn text_back_in_the_open_place_continues_the_range() {
    let state = applied(&[
        open("0", "x", &[]),
        text("0", "あ"),
        text("1", "い"),
        text("0", "う"),
        close("0"),
    ]);
    assert_eq!(spans(&state, "0"), vec![anchor(0, "x", "あう", 0..2)]);
    assert_eq!(spans(&state, "1"), vec![]);
}

// ---- 本文の消去（要件 2.7）

/// `\c` はその場所の範囲の列と開いている印を一緒に消す。印が残っていれば、消した後に
/// 同じ通し番号で入る選択肢が後続の文字で伸びてしまう。
#[test]
fn clear_drops_the_ranges_and_the_open_mark() {
    let state = applied(&[
        open("0", "x", &[]),
        text("0", "あ"),
        cue("0", 0.0, CueCommand::Clear),
        choice("0", "OnYes", "はい"),
        text("0", "う"),
    ]);
    assert_eq!(
        spans(&state, "0"),
        vec![ChoiceSpan {
            kind: SpanKind::Choice,
            ..anchor(0, "OnYes", "はい", 0..2)
        }]
    );
}

/// 別のスコープの `\c` は、開いている範囲に触れない。
#[test]
fn clear_of_another_scope_keeps_the_open_range() {
    let state = applied(&[
        open("0", "x", &[]),
        text("0", "あ"),
        cue("1", 0.0, CueCommand::Clear),
        text("0", "い"),
    ]);
    assert_eq!(spans(&state, "0"), vec![anchor(0, "x", "あい", 0..2)]);
}

/// 全員分の消去は、どの場所の範囲の列も開いている印も消す。
#[test]
fn clear_all_drops_every_range_and_the_open_mark() {
    let state = applied(&[
        open("1", "y", &[]),
        text("1", "い"),
        close("1"),
        open("0", "x", &[]),
        text("0", "あ"),
        cue("0", 0.0, CueCommand::ClearAll),
        choice("0", "OnYes", "はい"),
        text("0", "う"),
    ]);
    assert_eq!(
        spans(&state, "0"),
        vec![ChoiceSpan {
            kind: SpanKind::Choice,
            ..anchor(0, "OnYes", "はい", 0..2)
        }]
    );
    assert_eq!(spans(&state, "1"), vec![]);
}

// ---- 下見の空回し（要件 2.10）

/// 空回しの写しは、アンカーの合図で記録を 1 件も出さない（本番の適用が出す）。
/// 写しの中の範囲の記録は本番と同じ。
#[test]
fn rehearsal_emits_no_records() {
    // 開き・重なりの開き・閉じ・迷子の閉じ（防御の 2 か所を含む）。
    let talk = [
        open("0", "a", &[]),
        open("0", "b", &[]),
        close("0"),
        close("0"),
    ];
    let run = |mut state: TextLayerState| {
        for c in &talk {
            state.apply_cue(c);
        }
        state
    };

    // 対照: 本番の状態は 4 つの合図のどれにも記録を出す（debug どまり・警告は compile の担当）。
    let (production, loud) = capture(|| run(TextLayerState::default()));
    assert!(loud.len() >= talk.len(), "対照: 本番は記録を出す {loud:?}");
    assert!(
        loud.iter().all(|e| e.level == tracing::Level::DEBUG),
        "文字の層は警告しない {loud:?}"
    );
    for needle in ["開いたまま", "開いているアンカーが無い"] {
        assert!(
            loud.iter().any(|e| e.message().contains(needle)),
            "対照: 防御の記録「{needle}」が出る {loud:?}"
        );
    }

    let (rehearsed, quiet) = capture(|| run(TextLayerState::default().rehearsal_copy()));
    assert_eq!(quiet.len(), 0, "空回しは記録を出さない {quiet:?}");
    assert_eq!(
        rehearsed.places().collect::<Vec<_>>(),
        production.places().collect::<Vec<_>>(),
        "写しの中の範囲は本番と同じ"
    );
}

// ---- 既定の下線（要件 5.1・5.3〜5.5）

fn font(actor: &str, tokens: &[&str]) -> TalkCue {
    cue(
        actor,
        0.0,
        CueCommand::command_carrier(
            FONT_TAG_CARRIER,
            tokens.iter().map(|t| (*t).to_owned()).collect(),
        ),
    )
}

/// スコープの普通のバルーンの場所の内容。
fn balloon<'a>(state: &'a TextLayerState, actor: &str) -> &'a ActorTextState {
    state.actor_state(&key(actor)).expect("場所に状態がある")
}

/// 字ごとの見た目——装飾番号を装飾の表で引き直したもの（配置の層と同じ引き方）。
fn looks(content: &ActorTextState) -> Vec<&TextLook> {
    content
        .glyph_styles()
        .iter()
        .map(|id| {
            content
                .styles()
                .resolve(*id, &content.look_layers().default)
        })
        .collect()
}

/// 字ごとの下線。
fn underlines(content: &ActorTextState) -> Vec<bool> {
    looks(content).iter().map(|look| look.underline).collect()
}

/// 下線の付いた番号は範囲の中の文字だけに付く。`\f` を 1 つも挟まなくても、開きの直後の文字から
/// 付き、閉じの直後の文字からは付かない。アンカーが包んだ選択肢の文字にも付く。
#[test]
fn underline_number_covers_only_the_characters_inside_the_range() {
    let state = applied(&[
        text("0", "ま"),
        open("0", "x", &[]),
        text("0", "あ"),
        choice("0", "OnYes", "はい"),
        close("0"),
        text("0", "う"),
        choice("0", "OnNo", "いえ"),
    ]);
    let content = balloon(&state, "0");
    assert_eq!(
        underlines(content),
        [false, true, true, true, false, false, false]
    );
    // 範囲の外は既定の番号のまま。表に載るのは「既定の見た目＋下線」の 1 つだけ。
    let inside = content.glyph_styles()[1];
    let outside = StyleId::DEFAULT;
    assert_ne!(inside, outside);
    assert_eq!(
        content.glyph_styles(),
        [outside, inside, inside, inside, outside, outside, outside]
    );
    assert_eq!(content.styles().len(), 1);
}

/// 開いたまま別の場所へ寄り道した文字は範囲の外なので、下線も付かない。
#[test]
fn text_in_another_place_while_open_is_not_underlined() {
    let state = applied(&[
        open("0", "x", &[]),
        text("0", "あ"),
        text("1", "い"),
        text("0", "う"),
        close("0"),
    ]);
    assert_eq!(underlines(balloon(&state, "0")), [true, true]);
    assert_eq!(underlines(balloon(&state, "1")), [false]);
}

/// シェルの中の箱でも、普通のバルーンと同じ装飾番号の並びになる（要件 5.5）。
#[test]
fn box_text_gets_the_same_underline_numbers() {
    let talk = [
        text("0", "ま"),
        open("0", "x", &[]),
        text("0", "あ"),
        close("0"),
        text("0", "う"),
    ];
    let mut state = state_with_boxes();
    show(&mut state, "0", 0);
    for c in &talk {
        state.apply_cue(c);
    }
    let in_box = state
        .place_state(&PlaceKey {
            actor: key("0"),
            place: boxed("a"),
        })
        .expect("箱の場所に状態がある");
    assert_eq!(underlines(in_box), [false, true, false]);
    assert_eq!(
        in_box.glyph_styles(),
        balloon(&applied(&talk), "0").glyph_styles()
    );
}

/// 範囲の中で作者が下線を切っても、アンカーの下線が勝つ。作者のほかの指定は範囲の中でも効く。
#[test]
fn anchor_underline_wins_over_author_underline_off() {
    let state = applied(&[
        open("0", "x", &[]),
        text("0", "あ"),
        font("0", &["underline", "false"]),
        text("0", "い"),
        font("0", &["bold", "1"]),
        text("0", "う"),
        close("0"),
        text("0", "え"),
    ]);
    let content = balloon(&state, "0");
    assert_eq!(underlines(content), [true, true, true, false]);
    let bold: Vec<bool> = looks(content).iter().map(|look| look.bold).collect();
    assert_eq!(bold, [false, false, true, true]);
}

/// 作者の装飾の状態（次の文字が受け取る見た目）は、開いている間も閉じた後も、アンカーが
/// 無いときと同じ。下線を足すのは番号にする写しだけ。
#[test]
fn author_decoration_state_is_untouched_by_the_anchor() {
    // 下線なしの作者: 開いている間も閉じた後も、今の見た目は既定のまま。
    let mut state = applied(&[open("0", "x", &[]), text("0", "あ")]);
    let content = balloon(&state, "0");
    assert_eq!(content.current_look(), &content.look_layers().default);
    state.apply_cue(&close("0"));
    let content = balloon(&state, "0");
    assert_eq!(content.current_look(), &content.look_layers().default);

    // 下線ありの作者: 範囲の前・中・後が同じ見た目＝同じ番号で、閉じた後も下線は立ったまま。
    let state = applied(&[
        font("0", &["underline", "1"]),
        text("0", "ま"),
        open("0", "x", &[]),
        text("0", "あ"),
        close("0"),
        text("0", "う"),
    ]);
    let content = balloon(&state, "0");
    let author = content.glyph_styles()[0];
    assert_ne!(author, StyleId::DEFAULT);
    assert_eq!(content.glyph_styles(), [author; 3]);
    assert_eq!(content.styles().len(), 1);
    assert!(content.current_look().underline);
}

/// 一括の戻し（`\f[default]`／`\f[disable]`）の戻し先は、アンカーがあっても無くても同じ。
/// 範囲の中で戻した文字は「戻し先＋下線」になり、閉じた後の文字は戻し先そのもの。
#[test]
fn bulk_reset_restores_the_same_look_with_or_without_the_anchor() {
    for reset in ["default", "disable"] {
        let with = applied(&[
            font("0", &["bold", "1"]),
            open("0", "x", &[]),
            text("0", "あ"),
            font("0", &[reset]),
            text("0", "い"),
            close("0"),
            text("0", "う"),
        ]);
        let without = applied(&[
            font("0", &["bold", "1"]),
            text("0", "あ"),
            font("0", &[reset]),
            text("0", "い"),
            text("0", "う"),
        ]);
        let (with, without) = (balloon(&with, "0"), balloon(&without, "0"));
        assert_eq!(with.current_look(), without.current_look(), "{reset}");
        assert_eq!(underlines(with), [true, true, false], "{reset}");
        // 下線を除けば、字ごとの見た目はアンカーの無い台本と同じ。
        let plain: Vec<TextLook> = looks(with)
            .into_iter()
            .map(|look| TextLook {
                underline: false,
                ..look.clone()
            })
            .collect();
        let expected: Vec<TextLook> = looks(without).into_iter().cloned().collect();
        assert_eq!(plain, expected, "{reset}");
        assert_eq!(looks(with)[2], with.current_look(), "{reset}");
    }
}

// ---- 下見の空回しと下線（設計「既存アーキテクチャの分析」の空回しの項）

/// 下見の空回しと本番で、装飾番号の並びと装飾の表が一致する。先渡しを受け取ってから合図を
/// 1 つずつ流すと、流すたびに届いた内容が空回しの全文の先頭と一致し（全文で配置される）、
/// 食い違いの warn は 0 件。
#[test]
fn rehearsal_and_production_agree_on_style_numbers() {
    let talk = [
        text("0", "ま"),
        open("0", "x", &[]),
        text("0", "あ"),
        font("0", &["underline", "false"]),
        text("0", "い"),
        close("0"),
        text("0", "う"),
    ];

    // 空回しの写しと本番の状態へ同じ列を流す。
    let production = applied(&talk);
    let mut rehearsed = TextLayerState::default().rehearsal_copy();
    for c in &talk {
        rehearsed.apply_cue(c);
    }
    let (production, rehearsed) = (balloon(&production, "0"), balloon(&rehearsed, "0"));
    assert_eq!(underlines(production), [false, true, true, false]);
    assert_eq!(rehearsed.glyph_styles(), production.glyph_styles());
    assert_eq!(rehearsed.styles(), production.styles());

    // 先渡しの経路: 流すたびに `basis` が全文を返す。
    let place = PlaceKey::balloon(&key("0"));
    let mut state = TextLayerState::default();
    let mut look = TalkLookahead::default();
    look.install(&state, None, &talk);
    let (answers, events) = capture(|| {
        let mut answers = Vec::new();
        for c in &talk {
            let dest = PlaceKey {
                actor: c.actor.clone(),
                place: state.destination(&c.actor),
            };
            look.note_cue(c, &dest);
            advance_state(&mut state, None, c);
            let arrived = state.place_state(&place).expect("場所に状態がある");
            answers.push(matches!(look.basis(&place, arrived), Basis::Full { .. }));
        }
        answers
    });
    assert_eq!(answers, vec![true; talk.len()], "最後まで全文で配置される");
    assert!(
        events.iter().all(|e| e.level != tracing::Level::WARN),
        "食い違いの warn は出ない {events:?}"
    );
}

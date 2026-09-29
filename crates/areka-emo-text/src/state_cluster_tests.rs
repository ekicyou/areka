//! 文字の単位がクラスタであることを cue の適用で確かめる（要件 2.2・2.4・2.5・2.7・6.2）。
//!
//! 複数の符号でできた 6 形が、それぞれ 1 アイテム・1 段で積まれ、途中の段に部品が
//! 見えないことを判定する。形はソース上で紛れないよう符号の値で書く。

use super::test_support::{cue_dur, items_of, reveal_times_of};
use super::*;

/// 家族（ZWJ で結んだ列）: 👨 ZWJ 👩 ZWJ 👧
const FAMILY: &str = "\u{1F468}\u{200D}\u{1F469}\u{200D}\u{1F467}";
/// 国旗（地域表示記号の対）: 🇯 🇵
const FLAG_JP: &str = "\u{1F1EF}\u{1F1F5}";
/// 肌色の修飾つき: 👍 🏻
const THUMBS_UP_LIGHT: &str = "\u{1F44D}\u{1F3FB}";
/// 異体字セレクタつき: ❤ VS16
const HEART_VS16: &str = "\u{2764}\u{FE0F}";
/// キーキャップ: 1 VS16 囲みキーキャップ
const KEYCAP_ONE: &str = "\u{0031}\u{FE0F}\u{20E3}";
/// 結合文字つき: か 半濁点（結合用）
const KA_HANDAKUTEN: &str = "\u{304B}\u{309A}";

/// 要件 2.2 の 6 形すべて。
const SIX_FORMS: [&str; 6] = [
    FAMILY,
    FLAG_JP,
    THUMBS_UP_LIGHT,
    HEART_VS16,
    KEYCAP_ONE,
    KA_HANDAKUTEN,
];

/// 「👨‍👩‍👧🇯🇵」（時間 1.0）は 2 アイテムになり、各アイテムの文字列は形の全体。
/// 出す間隔は「時間 ÷ クラスタの数」＝0.5 で、時刻 0.5 の手前では家族の 1 つだけが
/// まるごと見える（部品の数で割ると 7 つに割れ、途中の段に 👨 だけが見える）。
#[test]
fn text_cue_of_family_and_flag_yields_two_whole_items() {
    let mut state = TextLayerState::default();
    let text = format!("{FAMILY}{FLAG_JP}");
    state.apply_cue(&cue_dur("0", 0.0, 1.0, CueCommand::Text(text.into())));

    assert_eq!(
        items_of(&state, "0"),
        &[TextItem::glyph(FAMILY), TextItem::glyph(FLAG_JP)]
    );
    // 間隔は 1.0 ÷ 2 = 0.5（2 の冪なので丸めは入らない）。
    assert_eq!(reveal_times_of(&state, "0"), vec![0.0, 0.5]);

    let actor = ActorKey::from("0");
    assert_eq!(state.visible_glyphs(&actor, 0.0), 1);
    assert_eq!(state.visible_glyphs(&actor, 0.25), 1);
    assert_eq!(state.visible_glyphs(&actor, 0.49), 1);
    assert_eq!(state.visible_glyphs(&actor, 0.5), 2);
    assert_eq!(state.visible_glyphs(&actor, 1.0), 2);
}

/// 6 形のそれぞれが 1 アイテム・1 時刻で積まれる（途中の段に部品が見えない）。
#[test]
fn each_of_six_forms_is_one_item_revealed_at_one_time() {
    for form in SIX_FORMS {
        let mut state = TextLayerState::default();
        state.apply_cue(&cue_dur("0", 0.0, 1.0, CueCommand::Text(form.into())));

        assert_eq!(
            items_of(&state, "0"),
            &[TextItem::glyph(form)],
            "{form:?} は 1 アイテムでなければならない"
        );
        assert_eq!(
            reveal_times_of(&state, "0"),
            vec![0.0],
            "{form:?} は 1 つの段で出なければならない"
        );
    }
}

/// 6 形を 1 つの cue にまとめても 6 アイテム・6 段で、つなげると元の文字列に戻る。
#[test]
fn six_forms_in_one_cue_yield_six_items_in_order() {
    let mut state = TextLayerState::default();
    let text: String = SIX_FORMS.concat();
    state.apply_cue(&cue_dur("0", 0.0, 1.5, CueCommand::Text(text.into())));

    let expected: Vec<TextItem> = SIX_FORMS.iter().map(|f| TextItem::glyph(f)).collect();
    assert_eq!(items_of(&state, "0"), expected.as_slice());
    // 間隔は 1.5 ÷ 6 = 0.25。
    assert_eq!(
        reveal_times_of(&state, "0"),
        vec![0.0, 0.25, 0.5, 0.75, 1.0, 1.25]
    );
}

/// 選択肢「A👍🏻B」の範囲の長さはクラスタの数の 3（部品で数えると 4）。
#[test]
fn choice_cue_range_counts_clusters() {
    let mut state = TextLayerState::default();
    let text = format!("A{THUMBS_UP_LIGHT}B");
    state.apply_cue(&cue_dur(
        "0",
        0.0,
        0.75,
        CueCommand::Choice {
            id: "OnThumbsUp".into(),
            text: text.into(),
            references: vec![],
        },
    ));

    let spans = state
        .actor_state(&ActorKey::from("0"))
        .expect("actor state should exist")
        .choices();
    assert_eq!(spans.len(), 1);
    assert_eq!(spans[0].glyph_range, 0..3);
    assert_eq!(spans[0].glyph_range.len(), 3);
    assert_eq!(
        items_of(&state, "0"),
        &[
            TextItem::glyph("A"),
            TextItem::glyph(THUMBS_UP_LIGHT),
            TextItem::glyph("B"),
        ]
    );
}

//! 文字を場所ごとに持つ（design.md「文字の行き先の決まり方」の末尾の段落・要件 4.5・4.6・
//! 6.7・7.1〜7.3・10.5）。
//!
//! 箱の構成と組み立ての支援は `state_route_tests.rs` のものを使う（同じ文面の表）:
//! - 0: a・b（既定は a） 1: b・a（既定は b） 2: c だけ 3: 箱なし

use super::route_tests::{boxed, key, place_items, select, show, state_with_boxes};
use super::test_support::cue;
use super::*;

fn text(state: &mut TextLayerState, actor: &str, s: &str) {
    state.apply_cue(&cue(actor, 0.0, CueCommand::Text(s.into())));
}

fn glyphs(s: &[&str]) -> Vec<TextItem> {
    s.iter().map(|c| TextItem::glyph(c)).collect()
}

fn at(actor: &str, place: TextPlace) -> PlaceKey {
    PlaceKey {
        actor: key(actor),
        place,
    }
}

/// スコープ 0 に、普通のバルーン「ふ」・箱 a「あ」・箱 b「び」（行き先は b）、
/// スコープ 1 に箱 a「い」（行き先は a）を書いた状態。
fn written() -> TextLayerState {
    let mut state = state_with_boxes();
    show(&mut state, "0", 3);
    text(&mut state, "0", "ふ");
    show(&mut state, "0", 0);
    text(&mut state, "0", "あ");
    state.apply_cue(&select("0", "b"));
    text(&mut state, "0", "び");
    show(&mut state, "1", 1);
    state.apply_cue(&select("1", "a"));
    text(&mut state, "1", "い");
    state
}

// ---- 行き先を切り替えても前の場所の文字は残る（要件 4.5）

#[test]
fn switching_destination_keeps_text_of_previous_places() {
    let mut state = written();
    // `\s` で箱の無いサーフェスや別の箱へ移っても、前の箱の文字は消えない。
    show(&mut state, "0", 2);
    text(&mut state, "0", "し");
    show(&mut state, "0", 3);
    assert_eq!(place_items(&state, "0", boxed("a")), glyphs(&["あ"]));
    assert_eq!(place_items(&state, "0", boxed("b")), glyphs(&["び"]));
    assert_eq!(place_items(&state, "0", boxed("c")), glyphs(&["し"]));
    assert_eq!(
        place_items(&state, "0", TextPlace::Balloon),
        glyphs(&["ふ"])
    );
    // 戻ってきた箱へは続きから書かれる。
    show(&mut state, "0", 0);
    text(&mut state, "0", "う");
    assert_eq!(place_items(&state, "0", boxed("a")), glyphs(&["あ", "う"]));
}

// ---- スコープと名前の組が場所の単位（要件 4.6）

#[test]
fn same_box_name_in_different_scopes_holds_separate_text() {
    let state = written();
    assert_eq!(place_items(&state, "0", boxed("a")), glyphs(&["あ"]));
    assert_eq!(place_items(&state, "1", boxed("a")), glyphs(&["い"]));
}

// ---- `\c` は行き先の場所だけ（要件 7.1〜7.3）

#[test]
fn clear_empties_only_destination_box_and_returns_to_write_start() {
    let mut state = written();
    let before = state.clone();
    state.apply_cue(&cue("0", 0.0, CueCommand::Clear));

    assert!(state.place_state(&at("0", boxed("b"))).unwrap().is_empty());
    for (actor, place) in [
        ("0", boxed("a")),
        ("0", TextPlace::Balloon),
        ("1", boxed("a")),
    ] {
        assert_eq!(
            state.place_state(&at(actor, place.clone())),
            before.place_state(&at(actor, place.clone())),
            "{actor} の {place:?} は消さない"
        );
    }
    assert_eq!(
        state.clear_count(&key("0")),
        0,
        "普通のバルーンは消していない"
    );
    assert_eq!(state.clear_count(&key("1")), 0);

    // 書き出し位置へ戻る: 次の文字は空の箱の先頭から書かれる。
    text(&mut state, "0", "べ");
    assert_eq!(place_items(&state, "0", boxed("b")), glyphs(&["べ"]));
    assert_eq!(state.destination(&key("0")), boxed("b"), "行き先は変えない");
}

#[test]
fn clear_on_balloon_destination_behaves_as_before_and_leaves_boxes() {
    // 箱の無い構成: 本 spec の前と同じ。
    let mut plain = TextLayerState::default();
    text(&mut plain, "0", "ふ");
    plain.apply_cue(&cue("0", 0.0, CueCommand::Clear));
    assert!(plain.actor_state(&key("0")).unwrap().is_empty());
    assert_eq!(plain.clear_count(&key("0")), 1);
    text(&mut plain, "0", "へ");
    assert_eq!(
        place_items(&plain, "0", TextPlace::Balloon),
        glyphs(&["へ"])
    );

    // 箱を持つ構成で行き先が普通のバルーン: 普通のバルーンだけを消し、箱の文字は残す。
    let mut state = written();
    show(&mut state, "0", 3);
    state.apply_cue(&cue("0", 0.0, CueCommand::Clear));
    assert!(state.actor_state(&key("0")).unwrap().is_empty());
    assert_eq!(state.clear_count(&key("0")), 1);
    assert_eq!(place_items(&state, "0", boxed("a")), glyphs(&["あ"]));
    assert_eq!(place_items(&state, "0", boxed("b")), glyphs(&["び"]));
}

// ---- 台詞の頭は全部の場所（要件 6.7）

#[test]
fn clear_all_empties_every_place_of_every_scope() {
    let mut state = written();
    state.apply_cue(&cue("0", 0.0, CueCommand::ClearAll));
    let places: Vec<(PlaceKey, bool)> = state
        .places()
        .map(|(k, s)| (k.clone(), s.is_empty()))
        .collect();
    assert_eq!(
        places,
        vec![
            (at("0", TextPlace::Balloon), true),
            (at("0", boxed("a")), true),
            (at("0", boxed("b")), true),
            (at("1", boxed("a")), true),
        ],
        "場所は残し、中身だけを空にする"
    );
    assert_eq!(state.clear_count(&key("0")), 1);
}

// ---- 読み口: 全部の場所・1 つの場所・既存の普通のバルーンの形（要件 10.5）

#[test]
fn places_iterates_every_place_in_key_order() {
    let state = written();
    let places: Vec<(PlaceKey, Vec<TextItem>)> = state
        .places()
        .map(|(k, s)| (k.clone(), s.items().to_vec()))
        .collect();
    assert_eq!(
        places,
        vec![
            (at("0", TextPlace::Balloon), glyphs(&["ふ"])),
            (at("0", boxed("a")), glyphs(&["あ"])),
            (at("0", boxed("b")), glyphs(&["び"])),
            (at("1", boxed("a")), glyphs(&["い"])),
        ]
    );
    assert_eq!(
        state.place_state(&at("1", boxed("a"))).unwrap().items(),
        glyphs(&["い"])
    );
    assert!(state.place_state(&at("1", boxed("b"))).is_none());
}

#[test]
fn existing_readers_return_balloon_place_values() {
    let state = written();
    let actors: Vec<(&ActorKey, Vec<TextItem>)> = state
        .actors()
        .map(|(k, s)| (k, s.items().to_vec()))
        .collect();
    assert_eq!(
        actors,
        vec![(&key("0"), glyphs(&["ふ"]))],
        "箱だけを持つスコープ 1 は現れない"
    );
    assert_eq!(
        state.actor_state(&key("0")).unwrap().items(),
        glyphs(&["ふ"])
    );
    assert!(state.actor_state(&key("1")).is_none());
    assert_eq!(state.visible_glyphs(&key("0"), f64::MAX), 1);
    assert_eq!(state.visible_glyphs(&key("1"), f64::MAX), 0);
}

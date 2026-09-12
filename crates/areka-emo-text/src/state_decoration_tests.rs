//! スコープの装飾状態と番号の配管（[`super::Decoration`]・`state.rs` の腕）の決定論テスト
//! （タスク 4.1・要件 3.1／3.2／3.3／3.4／3.5／3.7／3.8／3.9・2.5・10.7・15.3）。
//!
//! ## 章立て
//!
//! | § | 内容 | 要件 |
//! |---|---|---|
//! | §1 | スコープごとに独立した装飾状態（本体側の指定が相方側に効かない） | 3.1 |
//! | §2 | 以降の文字にだけ効く（追記済みの文字は変わらない・リビール中も同じ） | 3.2, 3.4, 10.7 |
//! | §3 | 番号列は追記点 2 か所（文字・選択肢）で glyph 序数と 1 対 1・0 文字は表を汚さない | 3.3, 3.5 |
//! | §4 | 内容の消去・改行・カーソル移動では装飾が戻らない | 3.7 |
//! | §5 | 台本の先頭（全消去）で装飾が既定へ戻る | 3.8 |
//! | §6 | 所有外のキーは値を保持し表示を変えない・キャリア名の自己選別 | 2.5 |
//! | §7 | 較正——過去に壊れうる形を再現すると赤になる述語 | 15.3 |
//!
//! タスク 4.2（「戻す操作」の公開・装着時の 2 層の差し込み・記録の 1 度化）は
//! 子モジュール [`reset`]（`state_decoration_reset_tests.rs`）が担う——1 ファイル
//! 1,000 行の見張りに余裕を残すためのファイル分けで、本ファイルの補助を共有する。
//!
//! **零は明示的に書く**——「改行では戻らない」「カーソル移動では戻らない」「内容の消去では
//! 戻らない」は互いに独立した事実なので、1 本の試験にまとめず別々に断言する（§4）。
//!
//! 装飾は描画に触れずに観測する（要件 3.9）——番号列 [`super::ActorTextState::glyph_styles`] を
//! 装飾の表 [`super::ActorTextState::styles`] で引き直した [`crate::look::TextLook`] が観測点で、
//! DirectWrite も GPU も要らない。

use areka_sakura::contract::{ActorKey, CueCommand, FONT_TAG_CARRIER, TalkCue};

use super::super::test_support::cue;
use super::super::{ActorTextState, TextItem, TextLayerState};
use crate::look::{StyleId, TextLook};

/// `\f[トークン…]` を運ぶ汎用キャリアの cue（再生時間 0）。
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

fn text(actor: &str, body: &str) -> TalkCue {
    cue(actor, 0.0, CueCommand::Text(body.to_owned()))
}

fn st<'a>(state: &'a TextLayerState, actor: &str) -> &'a ActorTextState {
    state
        .actor_state(&ActorKey::from(actor))
        .expect("actor state should exist")
}

/// グリフ序数の見た目——番号列を装飾の表で引き直したもの（配置層と同じ引き方）。
fn look_of<'a>(state: &'a TextLayerState, actor: &str, ordinal: usize) -> &'a TextLook {
    let actor_state = st(state, actor);
    actor_state.styles().resolve(
        actor_state.glyph_styles()[ordinal],
        &actor_state.look_layers().default,
    )
}

/// 「太字を 1 度指定して 2 文字」の最小の台本（多くの試験の下敷き）。
fn state_with_bold_then_text() -> TextLayerState {
    let mut state = TextLayerState::default();
    state.apply_cue(&font("0", &["bold", "1"]));
    state.apply_cue(&text("0", "あい"));
    state
}

// ------------------------------------------------- §1 スコープごとに独立（R3.1）

/// 本体側（`\0`）の `\f[bold,1]` は相方側（`\1`）の装飾状態に効かない。
#[test]
fn decoration_state_is_independent_per_scope() {
    let mut state = TextLayerState::default();
    state.apply_cue(&font("0", &["bold", "1"]));
    state.apply_cue(&text("0", "あ"));
    state.apply_cue(&text("1", "い"));

    assert!(st(&state, "0").current_look().bold, "本体側は太字になる");
    assert!(
        !st(&state, "1").current_look().bold,
        "相方側は本体側の指定を受け取らない"
    );
    assert!(look_of(&state, "0", 0).bold, "本体側の文字は太字");
    assert!(!look_of(&state, "1", 0).bold, "相方側の文字は太字でない");
}

/// 相方側の指定も本体側へ漏れない（逆向きも独立・対称であることを別に断言する）。
#[test]
fn decoration_from_kero_does_not_leak_into_sakura() {
    let mut state = TextLayerState::default();
    state.apply_cue(&text("0", "あ"));
    state.apply_cue(&font("1", &["italic", "1"]));
    state.apply_cue(&text("1", "い"));

    assert!(!st(&state, "0").current_look().italic);
    assert!(st(&state, "1").current_look().italic);
}

/// 装飾状態は届いた `\f` の actor のスコープにだけ生まれる（未知スコープの lazily 生成）。
#[test]
fn font_tag_creates_scope_lazily() {
    let mut state = TextLayerState::default();
    state.apply_cue(&font("2", &["bold", "1"]));

    assert!(st(&state, "2").current_look().bold);
    assert!(
        state.actor_state(&ActorKey::from("0")).is_none(),
        "他のスコープは生まれない"
    );
}

// ------------------------------- §2 以降の文字にだけ効く（R3.2／R3.4／R10.7）

/// 指定より前に追記済みの文字は既定のまま、後の文字だけが太字になる。
#[test]
fn font_tag_applies_only_to_glyphs_appended_after_it() {
    let mut state = TextLayerState::default();
    state.apply_cue(&text("0", "あ"));
    state.apply_cue(&font("0", &["bold", "1"]));
    state.apply_cue(&text("0", "い"));

    assert_eq!(st(&state, "0").glyph_styles().len(), 2);
    assert_eq!(
        st(&state, "0").glyph_styles()[0],
        StyleId::DEFAULT,
        "指定前の文字は既定の番号 0 のまま"
    );
    assert!(
        !look_of(&state, "0", 0).bold,
        "指定前の文字は太字にならない"
    );
    assert!(look_of(&state, "0", 1).bold, "指定後の文字だけが太字");
}

/// 戻す指定（`\f[default]`）も同じく以降の文字にだけ効く（R10.7）。
#[test]
fn reset_applies_only_to_glyphs_appended_after_it() {
    let mut state = state_with_bold_then_text();
    state.apply_cue(&font("0", &["default"]));
    state.apply_cue(&text("0", "う"));

    assert!(look_of(&state, "0", 0).bold, "戻す前の文字は太字のまま");
    assert!(look_of(&state, "0", 1).bold);
    assert!(!look_of(&state, "0", 2).bold, "戻した後の文字は既定");
    assert_eq!(st(&state, "0").glyph_styles()[2], StyleId::DEFAULT);
}

/// リビール中（文字が 1 文字ずつ現れる途中）に届いた `\f` も、リビール時刻を変えない（R3.4）。
#[test]
fn font_tag_does_not_change_reveal_times() {
    let mut baseline = TextLayerState::default();
    baseline.apply_cue(&text("0", "あい"));
    baseline.apply_cue(&text("0", "うえ"));

    let mut styled = TextLayerState::default();
    styled.apply_cue(&text("0", "あい"));
    styled.apply_cue(&font("0", &["bold", "1"]));
    styled.apply_cue(&text("0", "うえ"));

    assert_eq!(
        st(&styled, "0").reveal().times(),
        st(&baseline, "0").reveal().times(),
        "`\\f` は再生時間 0——リビール時刻列を 1 つも動かさない"
    );
    assert!(!look_of(&styled, "0", 1).bold);
    assert!(look_of(&styled, "0", 2).bold);
}

/// 同じ見た目が続く限り番号は同じ（表は種類数だけ増え、文字数では増えない）。
#[test]
fn same_look_folds_into_one_style_number() {
    let mut state = state_with_bold_then_text();
    state.apply_cue(&font("0", &["bold", "1"]));
    state.apply_cue(&text("0", "うえ"));

    assert_eq!(st(&state, "0").styles().len(), 1, "太字は 1 種類だけ");
    let ids = st(&state, "0").glyph_styles();
    assert_eq!(ids, &[ids[0]; 4], "4 文字とも同じ番号");
    assert_ne!(ids[0], StyleId::DEFAULT);
}

// ------------------------------------- §3 番号列とグリフ序数の 1 対 1（R3.3／R3.5）

/// 番号列の長さは items のグリフ数に等しい（改行・カーソル移動は序数を消費しない）。
#[test]
fn glyph_style_count_matches_glyph_count() {
    let mut state = TextLayerState::default();
    state.apply_cue(&text("0", "あ"));
    state.apply_cue(&cue("0", 0.0, CueCommand::NewLine { ratio: 1.0 }));
    state.apply_cue(&cue(
        "0",
        0.0,
        CueCommand::Cursor {
            x: "5em".to_owned(),
            y: String::new(),
        },
    ));
    state.apply_cue(&text("0", "いう"));

    let glyphs = st(&state, "0")
        .items()
        .iter()
        .filter(|it| matches!(it, TextItem::Glyph { .. }))
        .count();
    assert_eq!(glyphs, 3);
    assert_eq!(st(&state, "0").glyph_styles().len(), glyphs);
}

/// 選択肢（`\q`）の表示文字列にも、そのときの装飾状態が与えられる（R3.5）。
#[test]
fn choice_glyphs_carry_current_decoration() {
    let mut state = TextLayerState::default();
    state.apply_cue(&text("0", "あ"));
    state.apply_cue(&font("0", &["italic", "1"]));
    state.apply_cue(&cue(
        "0",
        0.0,
        CueCommand::Choice {
            id: "OnPick".to_owned(),
            text: "はい".to_owned(),
            references: Vec::new(),
        },
    ));

    assert_eq!(st(&state, "0").glyph_styles().len(), 3);
    assert!(!look_of(&state, "0", 0).italic);
    assert!(look_of(&state, "0", 1).italic, "選択肢の文字も斜体");
    assert!(look_of(&state, "0", 2).italic);
}

/// 0 文字の追記（空の `Text`）は番号列も装飾の表も増やさない——**到達する**経路の述語。
///
/// `Text` の腕は 0 文字を上位で分岐せず [`super::ActorTextState::push_current_style`] を
/// 無条件に呼ぶので、ガードが無ければ「既定と異なる見た目をグリフ 0 個で表へ積む」が実際に
/// 起きる。空の `Text` は上流に実在する（`state_reveal_tests.rs` の空チャンクの試験が流して
/// いる）。ゆえにこの述語が `push_current_style` の 0 文字ガードを踏む唯一の試験である。
#[test]
fn empty_text_appends_no_style_number() {
    let mut state = TextLayerState::default();
    state.apply_cue(&font("0", &["bold", "1"]));
    state.apply_cue(&text("0", ""));

    assert!(
        st(&state, "0").glyph_styles().is_empty(),
        "0 文字の追記は番号を 1 つも並べない"
    );
    assert!(
        st(&state, "0").styles().is_empty(),
        "文字が 1 つも無いのに既定と異なる見た目を表へ積まない"
    );

    // ガードは追記そのものを止めていない——後続の実文字は通常どおり番号を得る。
    state.apply_cue(&text("0", "あ"));
    assert_eq!(st(&state, "0").glyph_styles().len(), 1);
    assert_eq!(
        st(&state, "0").styles().len(),
        1,
        "表に載るのは今の 1 種類だけ"
    );
    assert!(look_of(&state, "0", 0).bold);
}

/// 空の選択肢（グリフを 1 つも追記しない縮退）も番号列と装飾の表を増やさない。
///
/// こちらは `Choice` の腕が 0 文字を**上位で分岐**して [`super::ActorTextState::push_current_style`]
/// を呼ばないことによる——`push_current_style` の 0 文字ガードには到達しない。すなわち
/// 「表を汚さない」ことの二重の保証のうち、呼び手側の分岐を押さえる述語である
/// （ガード側は上の `empty_text_appends_no_style_number` が押さえる）。
#[test]
fn empty_choice_appends_no_style_number() {
    let mut state = TextLayerState::default();
    state.apply_cue(&font("0", &["bold", "1"]));
    state.apply_cue(&cue(
        "0",
        0.0,
        CueCommand::Choice {
            id: "OnPick".to_owned(),
            text: String::new(),
            references: Vec::new(),
        },
    ));

    assert!(st(&state, "0").glyph_styles().is_empty());
    assert!(
        st(&state, "0").styles().is_empty(),
        "文字が 1 つも無いのに表へ登録しない"
    );
}

// ----------------------------------------- §4 消去・改行・カーソルで戻らない（R3.7）

/// 内容の消去（`\c`）は内容だけを消し、装飾状態を保つ。
#[test]
fn content_clear_keeps_decoration() {
    let mut state = state_with_bold_then_text();
    state.apply_cue(&cue("0", 0.0, CueCommand::Clear));

    assert!(st(&state, "0").items().is_empty(), "内容は消える");
    assert!(
        st(&state, "0").glyph_styles().is_empty(),
        "番号列も内容と同じ寿命"
    );
    assert!(st(&state, "0").styles().is_empty(), "装飾の表も空になる");
    assert!(
        st(&state, "0").current_look().bold,
        "現在の見た目（装飾状態）は保たれる"
    );

    state.apply_cue(&text("0", "う"));
    assert!(look_of(&state, "0", 0).bold, "消去後に置いた文字も太字");
}

/// 改行（`\n`）では装飾が戻らない。
#[test]
fn newline_keeps_decoration() {
    let mut state = state_with_bold_then_text();
    state.apply_cue(&cue("0", 0.0, CueCommand::NewLine { ratio: 1.0 }));
    state.apply_cue(&text("0", "う"));

    assert!(st(&state, "0").current_look().bold);
    assert!(look_of(&state, "0", 2).bold, "改行の後の文字も太字");
}

/// カーソル移動（`\_l`）では装飾が戻らない。
#[test]
fn cursor_move_keeps_decoration() {
    let mut state = state_with_bold_then_text();
    state.apply_cue(&cue(
        "0",
        0.0,
        CueCommand::Cursor {
            x: "0".to_owned(),
            y: "1lh".to_owned(),
        },
    ));
    state.apply_cue(&text("0", "う"));

    assert!(st(&state, "0").current_look().bold);
    assert!(look_of(&state, "0", 2).bold, "カーソル移動の後の文字も太字");
}

/// 内容の消去は所有外キーの保持も消さない（消えるのは内容だけ）。
#[test]
fn content_clear_keeps_unowned_vocabulary() {
    let mut state = TextLayerState::default();
    state.apply_cue(&font("0", &["align", "center"]));
    state.apply_cue(&text("0", "あ"));
    state.apply_cue(&cue("0", 0.0, CueCommand::Clear));

    assert_eq!(
        st(&state, "0")
            .unowned_vocab()
            .get("align")
            .map(Vec::as_slice),
        Some(["center".to_owned()].as_slice())
    );
}

// --------------------------------------------- §5 台本の先頭で戻る（R3.8）

/// 台本の先頭の全消去（`ClearAll`）は内容を消したうえで装飾も既定へ戻す。
#[test]
fn clear_all_resets_decoration_to_default() {
    let mut state = state_with_bold_then_text();
    state.apply_cue(&font("0", &["align", "center"]));
    state.apply_cue(&cue("0", 0.0, CueCommand::ClearAll));

    let actor = st(&state, "0");
    assert!(actor.items().is_empty());
    assert!(actor.glyph_styles().is_empty());
    assert_eq!(
        actor.current_look(),
        &actor.look_layers().default,
        "現在の見た目が既定へ戻る"
    );
    assert!(
        actor.unowned_vocab().is_empty(),
        "所有外キーの保持も戻す操作に含まれる"
    );
}

/// 全消去は保持している**全**スコープの装飾を戻す（本体側だけではない）。
#[test]
fn clear_all_resets_every_scope() {
    let mut state = TextLayerState::default();
    state.apply_cue(&font("0", &["bold", "1"]));
    state.apply_cue(&text("0", "あ"));
    state.apply_cue(&font("1", &["italic", "1"]));
    state.apply_cue(&text("1", "い"));
    state.apply_cue(&cue("0", 0.0, CueCommand::ClearAll));

    assert!(!st(&state, "0").current_look().bold);
    assert!(!st(&state, "1").current_look().italic);
}

/// 一括の戻し（`\f[default]`）も装飾状態を既定へ戻す（内容は消さない）。
#[test]
fn font_default_resets_look_without_clearing_content() {
    let mut state = state_with_bold_then_text();
    state.apply_cue(&font("0", &["align", "center"]));
    state.apply_cue(&font("0", &["default"]));

    let actor = st(&state, "0");
    assert_eq!(actor.current_look(), &actor.look_layers().default);
    assert!(actor.unowned_vocab().is_empty());
    assert_eq!(actor.items().len(), 2, "内容は消えない");
}

// --------------------------------------- §6 所有外キーと自己選別（R2.5／R2.4）

/// 所有外のキー（寄せ・影・選択肢マーカー・アンカー）は値を保持し、表示を変えない。
#[test]
fn unowned_keys_are_retained_without_changing_the_look() {
    let mut state = TextLayerState::default();
    state.apply_cue(&text("0", "あ"));
    let before = st(&state, "0").current_look().clone();

    state.apply_cue(&font("0", &["align", "center"]));
    state.apply_cue(&font("0", &["shadowcolor", "255", "0", "0"]));
    state.apply_cue(&font("0", &["cursorbrush", "1"]));
    state.apply_cue(&font("0", &["anchor.font.color", "blue"]));

    let actor = st(&state, "0");
    assert_eq!(actor.current_look(), &before, "見た目は 1 項目も変わらない");
    assert_eq!(
        actor.unowned_vocab().get("align").map(Vec::as_slice),
        Some(["center".to_owned()].as_slice())
    );
    assert_eq!(
        actor.unowned_vocab().get("shadowcolor").map(Vec::as_slice),
        Some(["255".to_owned(), "0".to_owned(), "0".to_owned()].as_slice()),
        "引数列は記述順のまま保つ"
    );
    assert!(actor.unowned_vocab().contains_key("cursorbrush"));
    assert!(actor.unowned_vocab().contains_key("anchor.font.color"));
}

/// 同じ所有外キーが 2 度来たら最新の引数列を保つ。
#[test]
fn unowned_key_keeps_the_latest_arguments() {
    let mut state = TextLayerState::default();
    state.apply_cue(&font("0", &["valign", "top"]));
    state.apply_cue(&font("0", &["valign", "bottom"]));

    assert_eq!(
        st(&state, "0")
            .unowned_vocab()
            .get("valign")
            .map(Vec::as_slice),
        Some(["bottom".to_owned()].as_slice())
    );
}

/// 名前の違う運搬（`\!` の他コマンド）は従来どおり読み飛ばす——装飾状態を触らない。
#[test]
fn carrier_with_another_name_is_ignored() {
    let mut state = state_with_bold_then_text();
    state.apply_cue(&cue(
        "0",
        0.0,
        CueCommand::command_carrier("raise", vec!["OnTest".to_owned()]),
    ));

    assert!(st(&state, "0").current_look().bold, "装飾状態は不変");
    assert_eq!(st(&state, "0").items().len(), 2, "内容も不変");
}

/// 不正な指定（未知のキー・キー無し）は当該項目を変えず、解析も再生も中断しない。
#[test]
fn malformed_font_tag_leaves_the_look_unchanged() {
    let mut state = state_with_bold_then_text();
    let before = st(&state, "0").current_look().clone();

    state.apply_cue(&font("0", &["nosuchkey", "1"]));
    state.apply_cue(&font("0", &[]));
    state.apply_cue(&font("0", &["bold", "yes"]));

    assert_eq!(st(&state, "0").current_look(), &before);
}

// ---------------------------------------------------------------- §7 較正

/// 較正: 内容の消去で装飾まで消える誤り（旧 `Clear` の `= ActorTextState::default()`）。
#[test]
fn calibration_content_clear_must_not_reset_decoration() {
    let mut state = state_with_bold_then_text();
    state.apply_cue(&cue("0", 0.0, CueCommand::Clear));

    assert_ne!(
        st(&state, "0").current_look(),
        &st(&state, "0").look_layers().default,
        "内容の消去で装飾まで既定へ戻っている（`\\c` は装飾を戻さない）"
    );
}

/// 較正: 本体側の指定が相方側にも効く誤り（スコープが 1 つの装飾状態を共有している）。
#[test]
fn calibration_scopes_must_not_share_decoration() {
    let mut state = TextLayerState::default();
    state.apply_cue(&font("0", &["bold", "1"]));
    state.apply_cue(&text("1", "い"));

    assert_ne!(
        st(&state, "1").current_look(),
        st(&state, "0").current_look(),
        "本体側と相方側が同じ装飾状態を共有している"
    );
    assert_eq!(
        st(&state, "1").glyph_styles()[0],
        StyleId::DEFAULT,
        "相方側の文字が本体側の指定を受け取っている"
    );
}

/// 較正: 装飾の指定が既に追記済みの文字へ遡って効く誤り（番号列でなく 1 つの見た目で描く）。
#[test]
fn calibration_decoration_must_not_apply_retroactively() {
    let mut state = TextLayerState::default();
    state.apply_cue(&text("0", "あ"));
    state.apply_cue(&font("0", &["bold", "1"]));
    state.apply_cue(&text("0", "い"));

    let ids = st(&state, "0").glyph_styles();
    assert_ne!(
        ids[0], ids[1],
        "追記済みの文字と後の文字が同じ番号になっている"
    );
    assert!(
        !look_of(&state, "0", 0).bold,
        "指定より前の文字にまで太字が遡って効いている"
    );
}

/// 較正: 台本の先頭の全消去で装飾が戻らない誤り（内容だけ消して装飾を残す）。
#[test]
fn calibration_clear_all_must_reset_decoration() {
    let mut state = state_with_bold_then_text();
    state.apply_cue(&cue("0", 0.0, CueCommand::ClearAll));

    assert!(
        !st(&state, "0").current_look().bold,
        "台本の先頭で装飾が戻っていない（`\\c` と同じ扱いになっている）"
    );
}

#[path = "state_decoration_reset_tests.rs"]
mod reset;

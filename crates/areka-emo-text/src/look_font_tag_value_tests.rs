//! `\f` の値の状態機械（[`super::apply_font_tag`]）のうち、**値を持つ 3 キー**
//! （`height`・`color`・`name`）の決定論テスト（タスク 3.4・要件 7.1〜7.7・8.5〜8.8・
//! 9.5／9.6・15.3）。
//!
//! ## 章立て
//!
//! | § | 内容 | 要件 |
//! |---|---|---|
//! | §12 | 大きさ（絶対・相対の重ね掛け・百分率の基準・層への戻し・非正／非有限・読めない値・語彙） | 7.1〜7.7 |
//! | §13 | 色（各書式・戻し先の語彙の解決先・アンカーの縮退・読めない値） | 8.5〜8.8 |
//! | §14 | フォント名（候補列を記述順のまま保持・層への戻し・候補なし） | 9.5, 9.6 |
//! | §15 | 較正——過去に壊れうる形を再現すると赤になる述語 | 15.3 |
//!
//! §7〜§11（真偽 5 キー・上下付き・一括の戻し・所有外キー）は兄弟の
//! `look_font_tag_tests.rs`、§1〜§6（見た目の値・2 層・装飾の表）は `look_tests.rs` にある。
//! 本ファイルを分けたのは `look_font_tag_tests.rs` が 605 行で、本タスクの分を足すと
//! 1 ファイル 1,000 行の見張り（要件 1.3・`crates/log-capture-kit/tests/file_length_guard_test.rs`）
//! の射程に入るため。
//!
//! **狙った 1 項目だけが動くこと**を毎回見る——[`expect_only`] は「出発点に 1 項目だけ手を
//! 入れた見た目」と丸ごと比べるので、`\f[height,15]` が色まで動かせば赤になる。
//!
//! **母数を先に固定する**——書式の表・語彙の表・スタイルシートの語の表は、いずれも件数を
//! 断言してから回す（表が痩せても緑にならないようにする）。

use super::*;
use crate::color::{
    REASON_COMPONENT_COUNT, REASON_MIXED_NOTATION, REASON_NOT_A_NUMBER, REASON_OUT_OF_RANGE,
    REASON_UNKNOWN_TOKEN,
};

/// 大きさ・色・フォント名が既定層・無効表示層・選択肢文字色で**すべて違う** 2 層＋1 色。
///
/// [`LookLayers::from_balloon`] は無効表示層を既定層から複製する（色だけ混色）ので、
/// 「大きさとフォント名の `disable` が既定へ行ってしまう」取り違えを見られない。
/// ここでは 3 つの引き先がすべて別の値になるよう直に組む。
fn value_layers() -> LookLayers {
    LookLayers {
        default: TextLook {
            name: vec!["Meiryo".to_owned(), "Arial".to_owned()],
            height: 20.0,
            color: (10, 20, 30),
            ..TextLook::ukadoc_default()
        },
        disable: TextLook {
            name: vec!["Yu Gothic".to_owned()],
            height: 14.0,
            color: (140, 145, 150),
            ..TextLook::ukadoc_default()
        },
        cursor_text: (7, 8, 9),
    }
}

/// 出発点——3 つの値のいずれも 2 層とは違い、装飾も一部が立っている見た目。
///
/// 装飾を立てておくのは「値のキーの腕が見た目を丸ごと置き換えていないか」を見るため。
fn styled() -> TextLook {
    TextLook {
        name: vec!["Consolas".to_owned()],
        height: 33.0,
        color: (1, 2, 3),
        bold: true,
        italic: false,
        underline: true,
        strike: false,
        outline: true,
        script: Script::Sub,
    }
}

/// [`styled`] を出発点に 1 件適用し、（適用後の見た目, 結果）を返す。
fn apply(args: &[&str]) -> (TextLook, Result<Option<Note>, FontTagIssue>) {
    let mut look = styled();
    let result = apply_font_tag(&mut look, &value_layers(), args);
    (look, result)
}

/// 受理され、かつ `patch` が触れた 1 項目**だけ**が動いたことを見る（印を返す）。
#[track_caller]
fn expect_only(args: &[&str], patch: impl FnOnce(&mut TextLook)) -> Option<Note> {
    let (look, result) = apply(args);
    let note = result.unwrap_or_else(|issue| panic!("{args:?} が拒まれた: {issue:?}"));
    let mut expected = styled();
    patch(&mut expected);
    assert_eq!(look, expected, "{args:?} が狙った 1 項目以外も変えている");
    note
}

/// 拒まれ、見た目が 1 項目も動かず、記録の材料（キー・値・理由）が揃っていることを見る。
#[track_caller]
fn expect_refused(args: &[&str], reason: &str) {
    let (look, result) = apply(args);
    let issue = result.expect_err(&format!("{args:?} は拒まれるべき"));
    assert_eq!(look, styled(), "{args:?} は拒まれたのに見た目を変えている");
    assert_eq!(issue.reason, reason, "{args:?} の理由");
    assert_eq!(issue.key, args[0], "{args:?} の記録のキー");
    assert_eq!(issue.value, args[1..].join(","), "{args:?} の記録の値");
}

// ---------------------------------------------------------------- §12 大きさ（height）

/// 値を持つ 3 キーは**受理される**——「所有しているが未実装」の仮置き（タスク 3.3）が
/// 残っていれば、ここが最初に赤くなる（要件 7.1・8.1・9.1）。
#[test]
fn the_three_keys_that_carry_a_value_are_interpreted() {
    for args in [
        vec!["height", "15"],
        vec!["color", "red"],
        vec!["name", "Meiryo"],
    ] {
        let (_, result) = apply(&args);
        assert!(result.is_ok(), "{args:?} が拒まれている: {result:?}");
    }
}

/// スタイルシートの大きさキーワードは CSS の絶対 7 語＋相対 2 語——母数を先に固定する
/// （要件 7.7）。語が増減したらこのテストが先に赤くなる。
#[test]
fn the_stylesheet_size_keywords_are_the_nine_css_words() {
    assert_eq!(STYLESHEET_SIZE_KEYWORDS.len(), 9);
    assert_eq!(
        STYLESHEET_SIZE_KEYWORDS.to_vec(),
        vec![
            "xx-small", "x-small", "small", "medium", "large", "x-large", "xx-large", "larger",
            "smaller",
        ]
    );
}

/// `\f[height,N]` は em の大きさを `N` image px にする（要件 7.1）。
#[test]
fn an_absolute_height_sets_the_em_size_in_image_px() {
    let cases = [("15", 15.0_f32), ("8", 8.0), ("1", 1.0), ("12.5", 12.5)];
    assert_eq!(cases.len(), 4);
    for (value, want) in cases {
        let note = expect_only(&["height", value], |look| look.height = want);
        assert_eq!(note, None, "{value} は記録を要さない");
    }
}

/// `\f[height,+N]`／`-N` は**そのとき効いている大きさ**に加減し、重ねて効く（要件 7.2）。
///
/// 出発点 33 → `+3` → 36 → `+3` → 39 → `-9` → 30。道中を毎回断言する——最後だけを見ると
/// 「2 度目の `+3` を無視する」形まで緑で通る。
#[test]
fn a_signed_height_stacks_on_the_size_in_effect() {
    let layers = value_layers();
    let mut look = styled();
    let steps = [("+3", 36.0_f32), ("+3", 39.0), ("-9", 30.0)];
    assert_eq!(steps.len(), 3);
    for (value, want) in steps {
        apply_font_tag(&mut look, &layers, &["height", value])
            .unwrap_or_else(|issue| panic!("\\f[height,{value}] が拒まれた: {issue:?}"));
        assert_eq!(look.height, want, "\\f[height,{value}] の後の大きさ");
    }
}

/// `\f[height,N%]` は**既定の見た目の大きさ**（ここでは 20）を基準にする（要件 7.3）。
#[test]
fn a_percent_height_is_measured_against_the_default_size() {
    let cases = [("200%", 40.0_f32), ("50%", 10.0), ("100%", 20.0)];
    assert_eq!(cases.len(), 3);
    for (value, want) in cases {
        let note = expect_only(&["height", value], |look| look.height = want);
        assert_eq!(note, None, "{value} は記録を要さない");
    }
}

/// `\f[height,default]`／`disable` は大きさ**だけ**を当該層の値へ（要件 7.4・7.5）。
#[test]
fn height_default_and_disable_take_the_layer_size() {
    let layers = value_layers();
    assert_ne!(
        layers.default.height, layers.disable.height,
        "2 層の大きさが同じでは行き先の取り違えを見られない"
    );
    for (word, want) in [
        ("default", layers.default.height),
        ("disable", layers.disable.height),
    ] {
        let note = expect_only(&["height", word], |look| look.height = want);
        assert_eq!(note, None, "{word} は記録を要さない");
    }
}

/// 結果が 0 以下になる大きさは値を変えず理由を返す（要件 7.6）。
#[test]
fn a_zero_or_negative_height_leaves_the_size_alone() {
    for value in ["0", "0.0", "0%"] {
        expect_refused(&["height", value], REASON_HEIGHT_NOT_POSITIVE);
    }
}

/// 相対指定で 0 以下へ落ちる場合も値を変えず理由を返す（要件 7.6）。
/// 出発点は 33 なので、`-33` はちょうど 0、`-40` は負。
#[test]
fn a_relative_height_that_falls_to_zero_or_below_leaves_the_size_alone() {
    assert_eq!(styled().height, 33.0, "出発点が変わったら下の値も直すこと");
    for value in ["-33", "-40"] {
        expect_refused(&["height", value], REASON_HEIGHT_NOT_POSITIVE);
    }
}

/// 非有限（無限大・非数）の大きさも値を変えず理由を返す（要件 7.6）。
#[test]
fn a_non_finite_height_leaves_the_size_alone() {
    for value in ["inf", "-inf", "NaN", "1e40"] {
        expect_refused(&["height", value], REASON_HEIGHT_NOT_POSITIVE);
    }
}

/// 数値・相対・百分率・6 語・スタイルシートの語のいずれとしても読めない値は、
/// 値を変えず理由を返す（要件 7.6 の「非数」）。語は**小文字の完全一致のみ**なので
/// `Default`・`X-Large` は読めない値である。
#[test]
fn a_height_value_that_cannot_be_read_leaves_the_size_alone() {
    for value in ["abc", "15px", "1.2.3", "", "+", "%", "Default", "X-Large"] {
        expect_refused(&["height", value], REASON_BAD_HEIGHT);
    }
}

/// 大きさの値はちょうど 1 つ——値なしと 2 つ以上はどちらも大きさを変えない（要件 7.6）。
#[test]
fn a_height_needs_exactly_one_value() {
    expect_refused(&["height"], REASON_VALUE_COUNT);
    expect_refused(&["height", "15", "20"], REASON_VALUE_COUNT);
}

/// スタイルシートの大きさキーワードは語彙として受理し、**大きさは変えず**印を返す
/// （要件 7.7・design §A 項目 3）。
#[test]
fn a_stylesheet_size_keyword_is_accepted_as_vocabulary_only() {
    for value in STYLESHEET_SIZE_KEYWORDS {
        let note = expect_only(&["height", value], |_| {});
        assert_eq!(
            note,
            Some(Note::StylesheetKeyword),
            "\\f[height,{value}] は語彙のみの印を返す"
        );
    }
}

// ------------------------------------------------------------------- §13 色（color）

/// 書式の解析は `color::parse_color` の担当だが、`\f[color]` の腕がそれを通していることは
/// ここで見る（要件 8.1〜8.4 の各書式が実際の色になる）。
#[test]
fn a_color_resolves_each_written_format() {
    let cases: [(&[&str], (u8, u8, u8)); 5] = [
        (&["red"], (255, 0, 0)),
        (&["100", "150", "200"], (100, 150, 200)),
        (&["#f0a"], (255, 0, 170)),
        (&["#1e90ff"], (0x1e, 0x90, 0xff)),
        (&["0%", "50%", "100%"], (0, 128, 255)),
    ];
    assert_eq!(cases.len(), 5);
    for (values, want) in cases {
        let mut args = vec!["color"];
        args.extend_from_slice(values);
        let note = expect_only(&args, |look| look.color = want);
        assert_eq!(note, None, "{values:?} は記録を要さない");
    }
}

/// 戻し先の語彙は 2 層＋選択肢文字色のどれを引くかまで固定する（要件 8.5〜8.7）。
#[test]
fn a_color_vocabulary_resolves_to_the_right_layer() {
    let layers = value_layers();
    let cases = [
        ("default", layers.default.color),
        ("default.plain", layers.default.color),
        ("disable", layers.disable.color),
        ("default.cursor", layers.cursor_text),
        ("default.cursornotselect", layers.cursor_text),
    ];
    assert_eq!(cases.len(), 5);
    for (word, want) in cases {
        let note = expect_only(&["color", word], |look| look.color = want);
        assert_eq!(note, None, "{word} は記録を要さない");
    }
}

/// アンカー 3 語はアンカーの色定義がまだ無いので **`default` と同じ色**を適用し、印を返す
/// （要件 8.8・design §A 項目 11）。
#[test]
fn an_anchor_color_falls_back_to_the_default_layer_with_a_mark() {
    let layers = value_layers();
    let words = [
        "default.anchor",
        "default.anchornotselect",
        "default.anchorvisited",
    ];
    assert_eq!(words.len(), 3);
    for word in words {
        let note = expect_only(&["color", word], |look| look.color = layers.default.color);
        assert_eq!(
            note,
            Some(Note::AnchorColorAsDefault),
            "\\f[color,{word}] は既定として適用した印を返す"
        );
    }
}

/// 読めない色指定は色を変えず、`color::parse_color` の理由をそのまま記録に載せる
/// （要件 8.9）。理由を潰して 1 つにまとめない——記録から何が悪かったかが分かる。
#[test]
fn a_color_that_cannot_be_read_leaves_the_color_alone() {
    let cases: [(&[&str], &str); 6] = [
        (&["zzz"], REASON_UNKNOWN_TOKEN),
        // 色名は小文字の完全一致のみ（`Red` は色名ではない）。
        (&["Red"], REASON_UNKNOWN_TOKEN),
        (&["300", "0", "0"], REASON_OUT_OF_RANGE),
        (&["100", "50%", "200"], REASON_MIXED_NOTATION),
        (&["a", "b", "c"], REASON_NOT_A_NUMBER),
        (&["1", "2"], REASON_COMPONENT_COUNT),
    ];
    assert_eq!(cases.len(), 6);
    for (values, reason) in cases {
        let mut args = vec!["color"];
        args.extend_from_slice(values);
        expect_refused(&args, reason);
    }
}

/// 値の無い `\f[color]` も色を変えない（成分数 0・要件 8.9）。
#[test]
fn a_color_without_a_value_leaves_the_color_alone() {
    expect_refused(&["color"], REASON_COMPONENT_COUNT);
}

// ------------------------------------------------------------------ §14 フォント名（name）

/// 候補列は**記述順のまま**保つ（要件 9.1・9.2）。実在の判定と読み飛ばしは描画層の担当なので、
/// フォントファイル名の候補もここでは落とさない（要件 9.3 は COM 層の `FontCatalog` が持つ）。
#[test]
fn a_name_keeps_the_candidate_list_in_written_order() {
    let cases: [(&[&str], &[&str]); 3] = [
        (&["Meiryo"], &["Meiryo"]),
        (&["Meiryo", "Arial"], &["Meiryo", "Arial"]),
        (
            &["Meiryo", "meiryo.ttf", "Arial"],
            &["Meiryo", "meiryo.ttf", "Arial"],
        ),
    ];
    assert_eq!(cases.len(), 3);
    for (values, want) in cases {
        let mut args = vec!["name"];
        args.extend_from_slice(values);
        let note = expect_only(&args, |look| {
            look.name = want.iter().map(|name| (*name).to_owned()).collect();
        });
        assert_eq!(note, None, "{values:?} は記録を要さない");
    }
}

/// `\f[name,default]`／`disable` はフォント**だけ**を当該層の候補列へ（要件 9.5・9.6）。
#[test]
fn name_default_and_disable_take_the_layer_candidates() {
    let layers = value_layers();
    assert_ne!(
        layers.default.name, layers.disable.name,
        "2 層の候補列が同じでは行き先の取り違えを見られない"
    );
    for (word, want) in [
        ("default", layers.default.name.clone()),
        ("disable", layers.disable.name.clone()),
    ] {
        let note = expect_only(&["name", word], |look| look.name = want.clone());
        assert_eq!(note, None, "{word} は記録を要さない");
    }
}

/// 候補が 1 つも無ければフォントを変えず理由を返す（空文字列だけの候補列も候補が無い）。
#[test]
fn a_name_without_a_candidate_leaves_the_font_alone() {
    expect_refused(&["name"], REASON_NO_CANDIDATE);
    expect_refused(&["name", ""], REASON_NO_CANDIDATE);
    expect_refused(&["name", "", ""], REASON_NO_CANDIDATE);
}

// ------------------------------------------------------------------ §15 較正（3.4）

/// 較正: 「符号付きの大きさを**既定**に足す」誤り（要件 7.2・15.3・design の
/// Implementation Notes が名指しする形）。
///
/// 出発点 33・既定 20 なので、正しければ 36、既定に足す実装なら 23 になる。
#[test]
fn calibration_a_signed_height_must_not_be_added_to_the_default_size() {
    let layers = value_layers();
    assert_eq!(styled().height, 33.0);
    assert_eq!(layers.default.height, 20.0);
    let (look, _) = apply(&["height", "+3"]);
    assert_eq!(
        look.height, 36.0,
        "符号付きの大きさを既定（20）に足している（正しくは、そのとき効いている 33 に足す）"
    );
}

/// 較正: 「百分率を**そのとき効いている大きさ**に掛ける」誤り（要件 7.3・15.3）。
///
/// 出発点 33・既定 20 なので、正しければ 40、現在に掛ける実装なら 66 になる。
#[test]
fn calibration_a_percent_height_must_not_be_measured_against_the_current_size() {
    let (look, _) = apply(&["height", "200%"]);
    assert_eq!(
        look.height, 40.0,
        "百分率をそのとき効いている大きさ（33）に掛けている（正しくは既定 20 が基準）"
    );
}

/// 較正: 「アンカー色を既定へ落とさず失敗させる／落としても印を返さない」誤り
/// （要件 8.8・15.3）。
///
/// 失敗させる実装なら `expect` で、印を返さない実装なら後ろの断言で赤になる。
#[test]
fn calibration_an_anchor_color_must_be_applied_as_default_and_marked() {
    let layers = value_layers();
    let (look, result) = apply(&["color", "default.anchor"]);
    let note =
        result.expect("\\f[color,default.anchor] は既定として受理される（拒んではならない）");
    assert_eq!(
        look.color, layers.default.color,
        "アンカー色が既定層の色に落ちていない"
    );
    assert_eq!(
        note,
        Some(Note::AnchorColorAsDefault),
        "既定として適用した印が返っていない（呼び手が warn を残せない）"
    );
}

//! `\f` の値の状態機械（[`super::apply_font_tag`]）のうち、真偽値で切り替える 5 項目・
//! 上下付き・一括の戻し・所有外キーの決定論テスト（タスク 3.3・要件 5.1〜5.5／5.8／5.9・
//! 6.1／6.2／6.4・10.1／10.2／10.4・2.5・15.3）。
//!
//! ## 章立て
//!
//! | § | 内容 | 要件 |
//! |---|---|---|
//! | §7 | 真偽 5 キー × 6 語（当該項目だけが動く・6 語以外と値なし） | 5.1〜5.5, 5.8, 5.9 |
//! | §8 | 上付き・下付き（後勝ちの排他・語彙のみの印） | 6.1, 6.2 |
//! | §9 | 一括の戻し（丸ごと置き換え・項目を列挙しない） | 10.1, 10.2, 10.4 |
//! | §10 | 所有外キー・未知キー・キーなし | 2.5, 2.6 |
//! | §11 | 較正——過去に壊れうる形を再現すると赤になる述語 | 15.3 |
//!
//! §1〜§6（見た目の値・2 層・装飾の表）は兄弟の `look_tests.rs` にある。本ファイルを分けたのは
//! `look_tests.rs` が 433 行で、本タスクの分を足すと 1 ファイル 1,000 行の見張り（要件 1.3・
//! `crates/log-capture-kit/tests/file_length_guard_test.rs`）の射程に入るため。
//!
//! **6 語は小文字の完全一致のみ**（design §A 項目 12）。大文字の語を受理する実装は §11 の
//! 較正で赤になる。
//!
//! **母数を先に固定する**——真偽 5 キーの並び（§7 の先頭）・6 語以外として試す値・所有外の
//! キーの一覧は、いずれも件数を断言してから回す（表が痩せても緑にならないようにする）。

use super::*;

// ------------------------------------------------------------------ §7 真偽 5 キー

/// 真偽値で切り替える 5 項目——キーと、その項目を指す場所。
///
/// 実装側の分岐（`switch_slot`）を呼ばずにテスト側で並びを持ち直している。実装の表を
/// そのまま借りると「同じ取り違えを両方がする」形になり、キーと項目の対応がずれても緑になる。
fn switch_keys() -> [(&'static str, fn(&mut TextLook) -> &mut bool); 5] {
    [
        ("bold", |look| &mut look.bold),
        ("italic", |look| &mut look.italic),
        ("underline", |look| &mut look.underline),
        ("strike", |look| &mut look.strike),
        ("outline", |look| &mut look.outline),
    ]
}

/// 真偽 5 項目が「既定層＝すべて真／無効表示層＝すべて偽」で割れた 2 層。
///
/// [`LookLayers::from_balloon`] はまだ 5 キーしか受け取れず（残り 8 キーの読み取りは
/// `areka-P0-balloon-font-descript-keys` の所有）、装飾の項目が層ごとに違う 2 層を作れない。
/// `default` と `disable` の行き先が取り違えられていないことを見るには層ごとに反対の値が
/// 要るので、ここでは各フィールドを直に組む。
fn split_layers() -> LookLayers {
    let default = TextLook {
        name: vec!["Meiryo".to_owned()],
        height: 20.0,
        color: (10, 20, 30),
        bold: true,
        italic: true,
        underline: true,
        strike: true,
        outline: true,
        script: Script::Sup,
    };
    let disable = TextLook {
        name: vec!["Meiryo".to_owned()],
        height: 20.0,
        color: (140, 145, 150),
        bold: false,
        italic: false,
        underline: false,
        strike: false,
        outline: false,
        script: Script::Sub,
    };
    LookLayers {
        default,
        disable,
        cursor_text: (7, 8, 9),
    }
}

/// 既定層とも無効表示層とも全項目が異なる「盛った見た目」（戻しが本当に丸ごと置き換えたかを
/// 見るための出発点）。
fn styled_look() -> TextLook {
    TextLook {
        name: vec!["Consolas".to_owned()],
        height: 33.0,
        color: (1, 2, 3),
        bold: false,
        italic: false,
        underline: false,
        strike: false,
        outline: false,
        script: Script::None,
    }
}

/// 真偽 5 項目をすべて真にした見た目。
fn all_switches_on(base: &TextLook) -> TextLook {
    let mut look = base.clone();
    for (_, slot) in switch_keys() {
        *slot(&mut look) = true;
    }
    look
}

/// 適用が成功したときに返るべき印——白抜きだけが「状態は更新するが表示は変えない」（R5.9）。
fn expected_switch_note(key: &str) -> Option<Note> {
    (key == "outline").then_some(Note::VocabularyOnly { key: "outline" })
}

/// 真偽値で切り替えるのは太字・斜体・下線・打ち消し線・白抜きの 5 項目——母数を先に固定する
/// （R5.1・R5.9）。項目が増減したらこのテストが先に赤くなる。
#[test]
fn the_switch_keys_are_bold_italic_underline_strike_and_outline() {
    let keys: Vec<&str> = switch_keys().iter().map(|(key, _)| *key).collect();
    assert_eq!(
        keys,
        vec!["bold", "italic", "underline", "strike", "outline"]
    );
}

/// `true`／`1` は当該項目だけを付ける（R5.1・R5.9 の印つき）。
#[test]
fn true_and_one_turn_only_that_item_on() {
    let layers = split_layers();
    for (key, slot) in switch_keys() {
        for word in ["true", "1"] {
            let before = TextLook::ukadoc_default();
            let mut look = before.clone();
            let note = apply_font_tag(&mut look, &layers, &[key, word])
                .unwrap_or_else(|issue| panic!("\\f[{key},{word}] が拒まれた: {issue:?}"));
            let mut expected = before.clone();
            *slot(&mut expected) = true;
            assert_eq!(
                look, expected,
                "\\f[{key},{word}] が当該項目だけを真にしていない"
            );
            assert_eq!(note, expected_switch_note(key), "\\f[{key},{word}] の印");
        }
    }
}

/// `false`／`0` は当該項目だけを外す（R5.2）。
#[test]
fn false_and_zero_turn_only_that_item_off() {
    let layers = split_layers();
    for (key, slot) in switch_keys() {
        for word in ["false", "0"] {
            let before = all_switches_on(&TextLook::ukadoc_default());
            let mut look = before.clone();
            apply_font_tag(&mut look, &layers, &[key, word])
                .unwrap_or_else(|issue| panic!("\\f[{key},{word}] が拒まれた: {issue:?}"));
            let mut expected = before.clone();
            *slot(&mut expected) = false;
            assert_eq!(
                look, expected,
                "\\f[{key},{word}] が当該項目だけを偽にしていない"
            );
        }
    }
}

/// `default`／`disable` は**当該項目だけ**をそれぞれの層の値へ合わせる（R5.3・R5.4）。
///
/// 大きさ・色・フォント名は層の値（20・(10,20,30)・Meiryo）だが、出発点の値のまま残る
/// ことも同時に述べている（`expected` は出発点の複製で、当該項目だけを書き換えたもの）。
#[test]
fn default_and_disable_take_only_that_item_from_the_matching_layer() {
    let layers = split_layers();
    for (key, slot) in switch_keys() {
        for (word, want) in [("default", true), ("disable", false)] {
            let before = if want {
                TextLook::ukadoc_default()
            } else {
                all_switches_on(&TextLook::ukadoc_default())
            };
            let mut look = before.clone();
            apply_font_tag(&mut look, &layers, &[key, word])
                .unwrap_or_else(|issue| panic!("\\f[{key},{word}] が拒まれた: {issue:?}"));
            let mut expected = before.clone();
            *slot(&mut expected) = want;
            assert_eq!(
                look, expected,
                "\\f[{key},{word}] が当該項目だけを層の値へ合わせていない"
            );
        }
    }
}

/// 6 語以外の値は当該項目を変えず、理由を返す（R5.5）。大文字は 6 語に**含まれない**
/// （語は小文字の完全一致のみ・design §A 項目 12）。
#[test]
fn a_word_outside_the_six_leaves_the_item_unchanged() {
    let layers = split_layers();
    let outside = [
        "TRUE", "True", "FALSE", "Default", "DISABLE", "yes", "on", "2", "-1", "",
    ];
    assert_eq!(outside.len(), 10, "6 語の外として試す値の母数");
    for (key, _) in switch_keys() {
        for word in outside {
            let before = styled_look();
            let mut look = before.clone();
            let issue = apply_font_tag(&mut look, &layers, &[key, word]).expect_err(&format!(
                "\\f[{key},{word}] は 6 語の外なので受理してはならない"
            ));
            assert_eq!(look, before, "\\f[{key},{word}] が見た目を変えている");
            assert_eq!(
                issue,
                FontTagIssue {
                    key: key.to_owned(),
                    value: word.to_owned(),
                    reason: REASON_BAD_SWITCH,
                }
            );
        }
    }
}

/// 値が無い（`\f[bold]`）ときも当該項目を変えず理由を返す——「6 語以外」とは別の断言
/// （R5.5 は「6 値のいずれでもない、**または値が無い**とき」の 2 つを定める）。
#[test]
fn a_switch_key_without_a_value_leaves_the_item_unchanged() {
    let layers = split_layers();
    for (key, _) in switch_keys() {
        let before = styled_look();
        let mut look = before.clone();
        let issue = apply_font_tag(&mut look, &layers, &[key])
            .expect_err(&format!("\\f[{key}] は値が無いので受理してはならない"));
        assert_eq!(look, before, "\\f[{key}] が見た目を変えている");
        assert_eq!(
            issue,
            FontTagIssue {
                key: key.to_owned(),
                value: String::new(),
                reason: REASON_VALUE_COUNT,
            }
        );
    }
}

/// 値が 2 つ以上（`\f[bold,1,0]`）も当該項目を変えず理由を返す（値は記述順のまま理由へ載る）。
#[test]
fn a_switch_key_with_more_than_one_value_leaves_the_item_unchanged() {
    let layers = split_layers();
    let before = styled_look();
    let mut look = before.clone();
    let issue = apply_font_tag(&mut look, &layers, &["bold", "1", "0"])
        .expect_err("値が 2 つある真偽値は受理してはならない");
    assert_eq!(look, before);
    assert_eq!(
        issue,
        FontTagIssue {
            key: "bold".to_owned(),
            value: "1,0".to_owned(),
            reason: REASON_VALUE_COUNT,
        }
    );
}

/// 5 項目は同時に組み合わせられる（R5.8）。
#[test]
fn the_five_items_combine_at_once() {
    let layers = split_layers();
    let mut look = TextLook::ukadoc_default();
    for (key, _) in switch_keys() {
        apply_font_tag(&mut look, &layers, &[key, "1"])
            .unwrap_or_else(|issue| panic!("\\f[{key},1] が拒まれた: {issue:?}"));
    }
    assert!(look.bold, "太字");
    assert!(look.italic, "斜体");
    assert!(look.underline, "下線");
    assert!(look.strike, "打ち消し線");
    assert!(look.outline, "白抜き");
    assert_eq!(look, all_switches_on(&TextLook::ukadoc_default()));
}

// ------------------------------------------------------------------ §8 上付き・下付き

/// 上下付きは後から指定した方を採り、先の方を外す（R6.2）。
#[test]
fn sub_and_sup_keep_only_the_one_specified_last() {
    let layers = split_layers();
    let mut look = TextLook::ukadoc_default();
    for (key, want) in [
        ("sub", Script::Sub),
        ("sup", Script::Sup),
        ("sub", Script::Sub),
        ("sup", Script::Sup),
    ] {
        apply_font_tag(&mut look, &layers, &[key, "1"])
            .unwrap_or_else(|issue| panic!("\\f[{key},1] が拒まれた: {issue:?}"));
        assert_eq!(look.script, want, "\\f[{key},1] の後は {want:?} だけが残る");
    }
}

/// 効いていない方を外しても、効いている方は残る（`sub` を外しても上付きは消えない）。
#[test]
fn turning_off_the_inactive_script_leaves_the_active_one() {
    let layers = split_layers();
    let mut look = TextLook {
        script: Script::Sup,
        ..TextLook::ukadoc_default()
    };
    apply_font_tag(&mut look, &layers, &["sub", "0"]).expect("\\f[sub,0] は受理される");
    assert_eq!(look.script, Script::Sup, "上付きまで外れている");
    apply_font_tag(&mut look, &layers, &["sup", "0"]).expect("\\f[sup,0] は受理される");
    assert_eq!(look.script, Script::None);
}

/// 上下付きも 6 語で解釈する（R6.1）。`default`／`disable` は当該の層の上下付きの値を引く
/// ——ここでは既定層が上付き・無効表示層が下付き（[`split_layers`]）。
#[test]
fn sub_and_sup_read_the_same_six_words() {
    let layers = split_layers();
    let cases = [
        ("sub", "true", Script::Sub),
        ("sub", "1", Script::Sub),
        ("sub", "false", Script::None),
        ("sub", "0", Script::None),
        ("sub", "default", Script::None),
        ("sub", "disable", Script::Sub),
        ("sup", "true", Script::Sup),
        ("sup", "1", Script::Sup),
        ("sup", "false", Script::None),
        ("sup", "0", Script::None),
        ("sup", "default", Script::Sup),
        ("sup", "disable", Script::None),
    ];
    assert_eq!(cases.len(), 12, "2 キー × 6 語の母数");
    for (key, word, want) in cases {
        let mut look = TextLook::ukadoc_default();
        apply_font_tag(&mut look, &layers, &[key, word])
            .unwrap_or_else(|issue| panic!("\\f[{key},{word}] が拒まれた: {issue:?}"));
        assert_eq!(look.script, want, "\\f[{key},{word}]");
    }
}

/// 上下付きは 6 語以外・値なしを当該項目を変えずに拒む（R5.5 と同じ規則・R6.1）。
#[test]
fn sub_and_sup_refuse_words_outside_the_six() {
    let layers = split_layers();
    for key in ["sub", "sup"] {
        for (args, reason) in [
            (vec![key, "TRUE"], REASON_BAD_SWITCH),
            (vec![key], REASON_VALUE_COUNT),
        ] {
            let before = TextLook {
                script: Script::Sup,
                ..styled_look()
            };
            let mut look = before.clone();
            let issue = apply_font_tag(&mut look, &layers, &args)
                .expect_err(&format!("{args:?} は受理してはならない"));
            assert_eq!(look, before, "{args:?} が見た目を変えている");
            assert_eq!(issue.reason, reason, "{args:?} の理由");
        }
    }
}

/// 上下付きと白抜きは「状態は更新するが表示は変えない」印を返す（R5.9・R6.1）。
/// 表示に効く 4 項目は印を返さない（零の側も明示する）。
#[test]
fn the_vocabulary_only_items_report_that_the_display_does_not_change() {
    let layers = split_layers();
    for key in ["sub", "sup", "outline"] {
        let mut look = TextLook::ukadoc_default();
        let note = apply_font_tag(&mut look, &layers, &[key, "1"])
            .unwrap_or_else(|issue| panic!("\\f[{key},1] が拒まれた: {issue:?}"));
        assert_eq!(
            note,
            Some(Note::VocabularyOnly { key }),
            "\\f[{key},1] が語彙のみの印を返していない"
        );
    }
    for key in ["bold", "italic", "underline", "strike"] {
        let mut look = TextLook::ukadoc_default();
        let note = apply_font_tag(&mut look, &layers, &[key, "1"])
            .unwrap_or_else(|issue| panic!("\\f[{key},1] が拒まれた: {issue:?}"));
        assert_eq!(note, None, "表示に効く \\f[{key},1] が印を返している");
    }
}

// ------------------------------------------------------------------ §9 一括の戻し

/// `\f[default]` は見た目を既定層で丸ごと置き換える（R10.1）。
#[test]
fn the_bulk_default_replaces_the_whole_look_with_the_default_layer() {
    let layers = split_layers();
    let mut look = styled_look();
    assert_ne!(
        look, layers.default,
        "出発点が既定層と同じでは何も述べていない"
    );
    let note = apply_font_tag(&mut look, &layers, &["default"]).expect("\\f[default] は受理される");
    assert_eq!(look, layers.default);
    assert_eq!(note, None);
}

/// `\f[disable]` は見た目を無効表示層で丸ごと置き換える（R10.2）。
#[test]
fn the_bulk_disable_replaces_the_whole_look_with_the_disable_layer() {
    let layers = split_layers();
    let mut look = styled_look();
    assert_ne!(look, layers.disable);
    let note = apply_font_tag(&mut look, &layers, &["disable"]).expect("\\f[disable] は受理される");
    assert_eq!(look, layers.disable);
    assert_eq!(note, None);
}

/// 一括の戻しは**項目を列挙しない**（R10.4）。後続仕様が [`TextLook`] に項目を足したとき
/// 戻しから漏れないのは「丸ごと置き換え」だからで、値の比較では今日の 9 項目しか見張れない。
/// 置き換えの 1 行そのものを字面で固定する（`layout_cursor_overflow_tests.rs` と同じ作法）。
#[test]
fn the_bulk_reset_replaces_the_look_as_a_whole_without_listing_items() {
    const LOOK_SRC: &str = include_str!("look.rs");
    for line in [
        "*current = layers.default.clone()",
        "*current = layers.disable.clone()",
    ] {
        assert!(
            LOOK_SRC.contains(line),
            "一括の戻しが丸ごとの置き換え（{line}）でなくなっている——項目を列挙すると\
             後続仕様が足した項目が戻しから漏れる"
        );
    }
}

/// 一括の戻しの意味はキーだけで決まる——正典の綴りは値を取らないので、余った値
/// （`\f[default,1]` のような綴り誤り）は戻しを妨げない（R10.1／R10.2）。
#[test]
fn a_stray_value_does_not_stop_the_bulk_reset() {
    let layers = split_layers();
    for (key, want) in [("default", &layers.default), ("disable", &layers.disable)] {
        let mut look = styled_look();
        let note = apply_font_tag(&mut look, &layers, &[key, "1"])
            .unwrap_or_else(|issue| panic!("\\f[{key},1] が拒まれた: {issue:?}"));
        assert_eq!(&look, want, "\\f[{key},1] が戻していない");
        assert_eq!(note, None);
    }
}

// ------------------------------------------------------------------ §10 所有外・未知・キーなし

/// 本仕様が意味を与えないキーは見た目を変えず「所有外」の印を返す（R2.5）。
/// 値の保持は呼び手（`state_decoration`）の担当で、ここでは見た目が動かないことだけを述べる。
#[test]
fn the_keys_owned_by_later_specs_do_not_change_the_look() {
    let layers = split_layers();
    let unowned = [
        vec!["align", "center"],
        vec!["valign", "bottom"],
        vec!["shadowcolor", "255", "0", "0"],
        vec!["shadowstyle", "1"],
        vec!["cursorcolor", "red"],
        vec!["cursorstyle", "1"],
        vec!["cursorbrushcolor", "red"],
        vec!["cursornotselectfontcolor", "red"],
        vec!["anchorstyle", "1"],
        vec!["anchorfontcolor", "red"],
        vec!["anchor.font.color", "red"],
        vec!["anchorvisitedpencolor", "red"],
    ];
    assert_eq!(unowned.len(), 12, "所有外として試すキーの母数");
    for args in unowned {
        let before = styled_look();
        let mut look = before.clone();
        let note = apply_font_tag(&mut look, &layers, &args)
            .unwrap_or_else(|issue| panic!("{args:?} は所有外として受理される: {issue:?}"));
        assert_eq!(look, before, "{args:?} が見た目を変えている");
        assert_eq!(note, Some(Note::Unowned), "{args:?} の印");
    }
}

/// 43 形のいずれでもないキーは見た目を変えず理由を返す（R2.6）。キーも小文字の完全一致のみで、
/// `Bold` は「太字」ではなく未知のキーである。
#[test]
fn an_unknown_key_leaves_the_look_unchanged() {
    let layers = split_layers();
    for key in ["Bold", "BOLD", "foo", "fontname", "sub2"] {
        let before = styled_look();
        let mut look = before.clone();
        let issue = apply_font_tag(&mut look, &layers, &[key, "1"])
            .expect_err(&format!("\\f[{key},1] は未知のキーとして拒まれる"));
        assert_eq!(look, before, "\\f[{key},1] が見た目を変えている");
        assert_eq!(
            issue,
            FontTagIssue {
                key: key.to_owned(),
                value: "1".to_owned(),
                reason: REASON_UNKNOWN_KEY,
            }
        );
    }
}

/// キーが無い 3 形（`\f`＝`[]`・`\f[]`＝`[]`・`\f[""]`＝`[""]`）は見た目を変えず理由を返す
/// （R2.6・design の「`args.first()` が `None` または空文字列」）。
#[test]
fn a_missing_key_leaves_the_look_unchanged() {
    let layers = split_layers();
    for args in [vec![], vec![""], vec!["", "1"]] {
        let before = styled_look();
        let mut look = before.clone();
        let issue = apply_font_tag(&mut look, &layers, &args)
            .expect_err(&format!("{args:?} はキーが無いので拒まれる"));
        assert_eq!(look, before, "{args:?} が見た目を変えている");
        assert_eq!(issue.reason, REASON_NO_KEY, "{args:?} の理由");
        assert_eq!(issue.key, "", "キーが無いので記録のキーも空");
    }
}

// ------------------------------------------------------------------ §11 較正（3.3）

/// 較正: 「当該項目の既定戻しが他項目も戻す」誤り（`\f[bold,default]` の腕で見た目を
/// 丸ごと置き換えてしまう形・R5.3・R15.3）。
#[test]
fn calibration_item_default_must_not_restore_the_other_items() {
    let layers = split_layers();
    let mut look = styled_look();
    apply_font_tag(&mut look, &layers, &["bold", "default"])
        .expect("\\f[bold,default] は受理される");

    assert_eq!(look.bold, layers.default.bold, "当該項目は既定へ戻る");
    assert_eq!(
        look.height,
        styled_look().height,
        "大きさまで既定へ戻っている（当該項目の戻しが丸ごと置き換えになっている）"
    );
    assert_eq!(look.color, styled_look().color, "色まで既定へ戻っている");
    assert_eq!(
        look.name,
        styled_look().name,
        "フォント名まで既定へ戻っている"
    );
    assert_eq!(
        look.script,
        styled_look().script,
        "上下付きまで既定へ戻っている"
    );
}

/// 較正: 「大文字の語を畳み込んで受理する」誤り（値を小文字化してから 6 語と比べる形・
/// design §A 項目 12 の「小文字の完全一致のみ」・R15.3）。
#[test]
fn calibration_the_six_words_must_not_be_case_folded() {
    let layers = split_layers();
    for word in ["TRUE", "True", "FALSE", "Default", "Disable", "DISABLE"] {
        let before = styled_look();
        let mut look = before.clone();
        let result = apply_font_tag(&mut look, &layers, &["bold", word]);
        assert!(
            result.is_err(),
            "\\f[bold,{word}] を受理している（値を小文字へ畳み込んでいる）"
        );
        assert_eq!(look, before);
    }
}

/// 較正: 「上下付きが排他でなく両立してしまう」誤り（`sub`・`sup` を独立した真偽値で
/// 持つ形・R6.2・R15.3）。
///
/// 独立した 2 つの真偽値で持つ実装なら、下付き→上付き→上付きを外す、の順で下付きが
/// 残る。1 つの値で持つ実装なら何も残らない。
#[test]
fn calibration_sub_and_sup_must_not_be_active_at_the_same_time() {
    let layers = split_layers();
    let mut look = TextLook::ukadoc_default();
    // 途中の状態も断言する——何もしない実装でも最後が `None` になるので、道中を述べないと
    // 「上下付きが 1 つも効かない」形まで緑で通る。
    for (key, word, want) in [
        ("sub", "1", Script::Sub),
        ("sup", "1", Script::Sup),
        ("sup", "0", Script::None),
    ] {
        apply_font_tag(&mut look, &layers, &[key, word])
            .unwrap_or_else(|issue| panic!("\\f[{key},{word}] が拒まれた: {issue:?}"));
        assert_eq!(
            look.script, want,
            "\\f[{key},{word}] の後——上付きを外したのに下付きが残っている\
             （上下付きを独立した 2 値で持っている）"
        );
    }
}

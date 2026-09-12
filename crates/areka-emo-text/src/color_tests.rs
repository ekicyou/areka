//! 「色指定」の解析（[`super::parse_color`]）と無効表示の混色（[`super::mix_disabled`]）の
//! 決定論テスト（タスク 3.1・要件 8.1〜8.4／8.9／8.10・4.6・15.3）。
//!
//! ## 章立て
//!
//! | § | 内容 | 要件 |
//! |---|---|---|
//! | §1 | 10 進 3 成分（境界・範囲外・非数・符号・書式の混在・成分数） | 8.1, 8.2, 8.9 |
//! | §2 | 百分率 3 成分（0〜255 への写像・境界・範囲外） | 8.2, 8.9 |
//! | §3 | 16 進 3 桁と 6 桁（各桁 2 倍・両者の一致・桁数違い） | 8.3, 8.9 |
//! | §4 | 色名表（母数・整列・全行の引き当て・実値の抜き取り・大文字は失敗） | 8.4, 8.9 |
//! | §5 | 戻し先を表す語彙 8 語（既定／素の既定／無効表示／選択肢／アンカー 3 種） | 8.5〜8.8 |
//! | §6 | 無効表示の混色（黒文字 × 白背景＝中間の灰・成分ごとの独立・端点） | 4.6 |
//! | §7 | 較正——過去の壊れ方を再現すると赤になる述語 | 15.3 |
//!
//! **期待値は正典の式から書く**——実装が返した値を書き写さない。百分率は
//! `round(v * 255 / 100)`、16 進 3 桁は各桁を 2 倍（`#f0a` → `#ff00aa`）が正典の定め
//! （requirements.md 付録 A「色指定（※）」）。
//!
//! 色名表を回すテストは、先に [`CSS_COLOR_NAMES`] の母数を固定してから回す——表が空の
//! ままだとループが恒真で緑になるため（§4）。また表を引き当てるだけのループは表の値が
//! 総崩れでも自己整合で緑になるので、正典の既知値を抜き取りで別に固定する（§4）。

use super::*;

/// 解析が成功して RGB になることを述べる補助（失敗は理由込みで落とす）。
fn rgb(args: &[&str]) -> (u8, u8, u8) {
    match parse_color(args) {
        Ok(ColorSpec::Rgb(r, g, b)) => (r, g, b),
        other => panic!("{args:?} が RGB に解析されない: {other:?}"),
    }
}

/// 解析が失敗することを述べ、その理由を返す補助。
fn reason(args: &[&str]) -> &'static str {
    match parse_color(args) {
        Err(e) => e.reason,
        Ok(spec) => panic!("{args:?} は解析失敗のはずだが {spec:?} になった"),
    }
}

// ---------------------------------------------------------------- §1 10 進 3 成分

/// 10 進 3 成分は 0〜255 をそのまま色にする（R8.1・正典の記述例 `\f[color,100,150,200]`）。
#[test]
fn decimal_triple_is_taken_as_is() {
    assert_eq!(rgb(&["100", "150", "200"]), (100, 150, 200));
    assert_eq!(rgb(&["0", "0", "0"]), (0, 0, 0));
    assert_eq!(rgb(&["255", "255", "255"]), (255, 255, 255));
    // 成分ごとに独立（取り違え・使い回しを赤にする）。
    assert_eq!(rgb(&["1", "2", "3"]), (1, 2, 3));
}

/// 256 以上は範囲外として失敗し、色を決めない（R8.9）。
#[test]
fn decimal_out_of_range_is_rejected() {
    assert_eq!(reason(&["256", "0", "0"]), REASON_OUT_OF_RANGE);
    assert_eq!(reason(&["0", "256", "0"]), REASON_OUT_OF_RANGE);
    assert_eq!(reason(&["0", "0", "256"]), REASON_OUT_OF_RANGE);
    assert_eq!(reason(&["1000", "1000", "1000"]), REASON_OUT_OF_RANGE);
}

/// 非数（負値・小数・空・字）は失敗する（R8.9）。
#[test]
fn decimal_non_numeric_is_rejected() {
    assert_eq!(reason(&["-1", "0", "0"]), REASON_NOT_A_NUMBER);
    assert_eq!(reason(&["1.5", "0", "0"]), REASON_NOT_A_NUMBER);
    assert_eq!(reason(&["", "0", "0"]), REASON_NOT_A_NUMBER);
    assert_eq!(reason(&["0", "x", "0"]), REASON_NOT_A_NUMBER);
}

/// 符号付きの数は 10 進でも百分率でも受けない（R8.1／R8.2 は符号を許していない・R8.9）。
///
/// `-1` だけを見ると「負だから範囲外で落ちた」に見えるが、`+1` を隣に置くと符号そのものを
/// 受けないことが字面で分かる。本仕様は `\f[height,+3]`（R7.2）で先頭の `+` に
/// 「相対指定」という別の意味を与えているので、色指定が黙って吸収してはならない。
#[test]
fn decimal_leading_plus_is_rejected() {
    assert_eq!(reason(&["+1", "0", "0"]), REASON_NOT_A_NUMBER);
    assert_eq!(reason(&["0", "+1", "0"]), REASON_NOT_A_NUMBER);
    assert_eq!(reason(&["0", "0", "+255"]), REASON_NOT_A_NUMBER);
    assert_eq!(reason(&["+50%", "0%", "0%"]), REASON_NOT_A_NUMBER);
}

/// 先頭ゼロは 10 進数値として自然に読む（符号と違って別の意味を持たない）。
#[test]
fn decimal_leading_zero_is_accepted() {
    assert_eq!(rgb(&["0255", "007", "0"]), (255, 7, 0));
}

/// 3 成分は 10 進か百分率のどちらかに揃っていなければならない（R8.1／R8.2 のいずれでもない
/// 形は R8.9 の「上記のいずれの書式でもない」に落ちる）。どちら向きの混ざり方も受けない。
#[test]
fn mixed_decimal_and_percent_is_rejected() {
    assert_eq!(reason(&["100", "50%", "200"]), REASON_MIXED_NOTATION);
    assert_eq!(reason(&["50%", "100", "20%"]), REASON_MIXED_NOTATION);
    assert_eq!(reason(&["100", "150", "200%"]), REASON_MIXED_NOTATION);
    assert_eq!(reason(&["50%", "90%", "20"]), REASON_MIXED_NOTATION);
}

/// 成分数が 1 でも 3 でもないときは失敗する（R8.9・「成分が足りない」）。
#[test]
fn wrong_component_count_is_rejected() {
    assert_eq!(reason(&[]), REASON_COMPONENT_COUNT);
    assert_eq!(reason(&["0", "0"]), REASON_COMPONENT_COUNT);
    assert_eq!(reason(&["0", "0", "0", "0"]), REASON_COMPONENT_COUNT);
}

// ---------------------------------------------------------------- §2 百分率 3 成分

/// 百分率は 0〜100 を 0〜255 へ写す（R8.2・`round(v * 255 / 100)`）。
#[test]
fn percent_triple_maps_to_0_255() {
    assert_eq!(rgb(&["0%", "0%", "0%"]), (0, 0, 0));
    assert_eq!(rgb(&["100%", "100%", "100%"]), (255, 255, 255));
    // round(50 * 255 / 100) = round(127.5) = 128
    assert_eq!(rgb(&["50%", "50%", "50%"]), (128, 128, 128));
    // 正典の記述例（`\f[anchor.font.color,50%,90%,20%]`）と同じ形。
    // round(90 * 255 / 100) = round(229.5) = 230 / round(20 * 255 / 100) = 51
    assert_eq!(rgb(&["50%", "90%", "20%"]), (128, 230, 51));
}

/// 100% を超える百分率は範囲外として失敗する（R8.9）。
#[test]
fn percent_out_of_range_is_rejected() {
    assert_eq!(reason(&["101%", "0%", "0%"]), REASON_OUT_OF_RANGE);
    assert_eq!(reason(&["0%", "255%", "0%"]), REASON_OUT_OF_RANGE);
}

/// 百分率の記号だけ・非数の百分率は失敗する（R8.9）。
#[test]
fn percent_non_numeric_is_rejected() {
    assert_eq!(reason(&["%", "0%", "0%"]), REASON_NOT_A_NUMBER);
    assert_eq!(reason(&["1.5%", "0%", "0%"]), REASON_NOT_A_NUMBER);
}

// ---------------------------------------------------------------- §3 16 進

/// 6 桁の 16 進は 2 桁ずつ 1 成分（R8.3）。大文字小文字は問わない。
#[test]
fn hex6_is_two_digits_per_component() {
    assert_eq!(rgb(&["#ff00aa"]), (0xff, 0x00, 0xaa));
    assert_eq!(rgb(&["#000000"]), (0, 0, 0));
    assert_eq!(rgb(&["#FFFFFF"]), (255, 255, 255));
    assert_eq!(rgb(&["#010203"]), (1, 2, 3));
}

/// 3 桁の 16 進は各桁を 2 倍にする（R8.3）。6 桁の対応形と一致する。
#[test]
fn hex3_doubles_each_digit_and_matches_hex6() {
    assert_eq!(rgb(&["#f0a"]), (0xff, 0x00, 0xaa));
    assert_eq!(rgb(&["#f0a"]), rgb(&["#ff00aa"]));
    assert_eq!(rgb(&["#000"]), rgb(&["#000000"]));
    assert_eq!(rgb(&["#fff"]), rgb(&["#ffffff"]));
    assert_eq!(rgb(&["#123"]), (0x11, 0x22, 0x33));
}

/// 3 桁 6 桁以外の桁数・16 進でない字は失敗する（R8.9）。
#[test]
fn hex_with_other_length_or_bad_digit_is_rejected() {
    assert_eq!(reason(&["#"]), REASON_BAD_HEX);
    assert_eq!(reason(&["#ff"]), REASON_BAD_HEX);
    assert_eq!(reason(&["#ffff"]), REASON_BAD_HEX);
    assert_eq!(reason(&["#fffffff"]), REASON_BAD_HEX);
    assert_eq!(reason(&["#gggggg"]), REASON_BAD_HEX);
    assert_eq!(reason(&["#gg0"]), REASON_BAD_HEX);
}

/// 16 進は 1 成分の書式なので、3 成分に並べても受けない（R8.9）。
#[test]
fn hex_in_triple_form_is_rejected() {
    assert_eq!(
        reason(&["#ff0000", "#00ff00", "#0000ff"]),
        REASON_NOT_A_NUMBER
    );
}

// ---------------------------------------------------------------- §4 色名表

/// 表の母数を先に固定する（表が空だと §4 のループが恒真で緑になるため）。
/// 147 語＝HTML/CSS（SVG 1.1／CSS Color 3）の拡張色名キーワード全体。
#[test]
fn color_name_table_has_147_entries() {
    assert_eq!(CSS_COLOR_NAMES.len(), 147);
}

/// 表は名前で厳密昇順（＝整列済みかつ重複なし）。二分探索の前提を守る。
#[test]
fn color_name_table_is_strictly_sorted() {
    for pair in CSS_COLOR_NAMES.windows(2) {
        assert!(
            pair[0].0 < pair[1].0,
            "色名表が昇順でない、または重複がある: {:?} → {:?}",
            pair[0].0,
            pair[1].0
        );
    }
}

/// 表のすべての名前が引き当てられ、表の値と一致する（R8.4）。
#[test]
fn every_table_name_resolves_to_its_entry() {
    assert_eq!(CSS_COLOR_NAMES.len(), 147, "母数が動いたら §4 を見直すこと");
    for (name, (r, g, b)) in CSS_COLOR_NAMES {
        assert_eq!(
            rgb(&[name]),
            (*r, *g, *b),
            "色名 {name} の引き当てが表と不一致"
        );
    }
}

/// 表の値そのものを正典の既知値で抜き取り固定する
/// （前のテストは表と実装の自己整合しか見ないので、表が総崩れでも緑になりうる）。
#[test]
fn well_known_color_names_have_canonical_values() {
    // 正典の記述例に出る 3 語（`\f[color,red]`／「red、white、black というような色名」）。
    assert_eq!(rgb(&["red"]), (0xff, 0x00, 0x00));
    assert_eq!(rgb(&["white"]), (0xff, 0xff, 0xff));
    assert_eq!(rgb(&["black"]), (0x00, 0x00, 0x00));
    // 綴り違いの同義語が同値であること。
    assert_eq!(rgb(&["gray"]), (0x80, 0x80, 0x80));
    assert_eq!(rgb(&["grey"]), rgb(&["gray"]));
    assert_eq!(rgb(&["aqua"]), rgb(&["cyan"]));
    assert_eq!(rgb(&["fuchsia"]), rgb(&["magenta"]));
    // 名前の直感に反する既知値（表を機械生成せず手で書いたときの取り違えを捕まえる）。
    assert_eq!(rgb(&["darkgray"]), (0xa9, 0xa9, 0xa9));
    assert!(
        rgb(&["darkgray"]) > rgb(&["gray"]),
        "darkgray は gray より明るいのが CSS の既知値"
    );
    assert_eq!(rgb(&["green"]), (0x00, 0x80, 0x00));
    assert_eq!(rgb(&["lime"]), (0x00, 0xff, 0x00));
}

/// 色名は小文字の完全一致のみ——大文字混じりは失敗する
/// （R8.4「色名を表すキーワード(全て小文字)で指定可能」・R8.9）。
#[test]
fn uppercase_color_name_is_rejected() {
    assert_eq!(reason(&["Red"]), REASON_UNKNOWN_TOKEN);
    assert_eq!(reason(&["RED"]), REASON_UNKNOWN_TOKEN);
    assert_eq!(reason(&["White"]), REASON_UNKNOWN_TOKEN);
    assert_eq!(reason(&["DarkGray"]), REASON_UNKNOWN_TOKEN);
}

/// 表に無い語は失敗する（R8.9）。空の 1 成分も同じ。
#[test]
fn unknown_word_is_rejected() {
    assert_eq!(reason(&["notacolor"]), REASON_UNKNOWN_TOKEN);
    assert_eq!(reason(&[""]), REASON_UNKNOWN_TOKEN);
    // CSS Color 4 で足された語は本表に無い（母数 147 の内訳を明示的に述べる）。
    assert_eq!(reason(&["rebeccapurple"]), REASON_UNKNOWN_TOKEN);
}

// ---------------------------------------------------------------- §5 戻し先の語彙

/// 戻し先を表す 8 語は色でなく語彙として返る（R8.5〜8.8）。解決は上位層の担当。
#[test]
fn restore_target_words_are_vocabulary_not_colors() {
    assert_eq!(parse_color(&["default"]), Ok(ColorSpec::Default));
    assert_eq!(parse_color(&["default.plain"]), Ok(ColorSpec::DefaultPlain));
    assert_eq!(parse_color(&["disable"]), Ok(ColorSpec::Disable));
    assert_eq!(
        parse_color(&["default.cursor"]),
        Ok(ColorSpec::DefaultCursor)
    );
    assert_eq!(
        parse_color(&["default.cursornotselect"]),
        Ok(ColorSpec::DefaultCursorNotSelect)
    );
    assert_eq!(
        parse_color(&["default.anchor"]),
        Ok(ColorSpec::DefaultAnchor)
    );
    assert_eq!(
        parse_color(&["default.anchornotselect"]),
        Ok(ColorSpec::DefaultAnchorNotSelect)
    );
    assert_eq!(
        parse_color(&["default.anchorvisited"]),
        Ok(ColorSpec::DefaultAnchorVisited)
    );
}

/// 語彙も小文字の完全一致のみ（R8.9）。
#[test]
fn restore_target_words_are_lowercase_only() {
    assert_eq!(reason(&["Default"]), REASON_UNKNOWN_TOKEN);
    assert_eq!(reason(&["DISABLE"]), REASON_UNKNOWN_TOKEN);
    assert_eq!(reason(&["default.Cursor"]), REASON_UNKNOWN_TOKEN);
    // 語彙に無い `default.*` も失敗（黙って `default` に落とさない）。
    assert_eq!(reason(&["default.nosuch"]), REASON_UNKNOWN_TOKEN);
}

// ---------------------------------------------------------------- §6 無効表示の混色

/// 黒文字 × 白背景の混色は中間の灰になる（R4.6・`(255 + 0 * 2) / 3 = 85`）。
#[test]
fn mix_disabled_black_text_on_white_background_is_mid_gray() {
    assert_eq!(mix_disabled((0, 0, 0), (255, 255, 255)), (85, 85, 85));
}

/// 成分ごとに独立した 1 式（成分の取り違え・使い回しを赤にする）。
#[test]
fn mix_disabled_is_per_component() {
    // (0 + 30*2)/3 = 20 / (150 + 60*2)/3 = 90 / (255 + 90*2)/3 = 145
    assert_eq!(mix_disabled((30, 60, 90), (0, 150, 255)), (20, 90, 145));
}

/// 文字色と背景が同じなら色は変わらない（薄める先が無い端点）。
#[test]
fn mix_disabled_with_equal_colors_is_identity() {
    assert_eq!(mix_disabled((0, 0, 0), (0, 0, 0)), (0, 0, 0));
    assert_eq!(
        mix_disabled((255, 255, 255), (255, 255, 255)),
        (255, 255, 255)
    );
    assert_eq!(mix_disabled((12, 34, 56), (12, 34, 56)), (12, 34, 56));
}

/// 混色は文字色の側に 2 の重みが乗る（引数の順序を入れ替えた誤りを赤にする）。
#[test]
fn mix_disabled_weights_the_text_color_twice() {
    let text_heavy = mix_disabled((0, 0, 0), (255, 255, 255));
    let background_heavy = mix_disabled((255, 255, 255), (0, 0, 0));
    assert_eq!(text_heavy, (85, 85, 85));
    assert_eq!(background_heavy, (170, 170, 170));
    assert_ne!(text_heavy, background_heavy, "引数の順序が効いていない");
}

/// 整数除算で切り捨てる（浮動小数の丸めに変えた実装を赤にする）。
#[test]
fn mix_disabled_truncates_by_integer_division() {
    // (1 + 1*2)/3 = 1 ちょうど / (2 + 1*2)/3 = 4/3 → 1 / (0 + 1*2)/3 = 2/3 → 0
    assert_eq!(mix_disabled((1, 1, 1), (1, 2, 0)), (1, 1, 0));
}

// ---------------------------------------------------------------- §7 較正

/// 較正: 百分率を 0〜255 へ写さず素通しする誤り（R15.3・タスク 3.1 名指し）。
#[test]
fn calibration_percent_must_not_pass_through_raw() {
    assert_ne!(
        parse_color(&["50%", "0%", "100%"]),
        Ok(ColorSpec::Rgb(50, 0, 100)),
        "百分率を素通ししている（0〜255 への写像が無い）"
    );
    assert_eq!(
        parse_color(&["50%", "0%", "100%"]),
        Ok(ColorSpec::Rgb(128, 0, 255))
    );
}

/// 較正: 16 進 3 桁を 2 倍にしない誤り（各桁をそのまま下位 4 ビットとして読む）。
#[test]
fn calibration_hex3_must_not_be_read_as_low_nibbles() {
    assert_ne!(
        parse_color(&["#f0a"]),
        Ok(ColorSpec::Rgb(0x0f, 0x00, 0x0a)),
        "16 進 3 桁を 2 倍していない"
    );
    assert_eq!(parse_color(&["#f0a"]), Ok(ColorSpec::Rgb(0xff, 0x00, 0xaa)));
}

/// 較正: 大文字の色名を小文字化して受けてしまう誤り（正典は「全て小文字」）。
#[test]
fn calibration_uppercase_name_must_not_be_folded() {
    assert!(
        parse_color(&["Red"]).is_err(),
        "大文字の色名を小文字へ畳んで受けている"
    );
}

/// 較正: 3 成分を成分ごとに独立解釈し、10 進と百分率の混在を黙って受けてしまう誤り
/// （正典は「3 成分とも 10 進」か「3 成分とも百分率」しか定めていない・R8.1／R8.2／R8.9）。
///
/// 成分ごとに独立して読む実装では `["100","50%","200"]` が `Rgb(100,128,200)` として通る。
#[test]
fn calibration_components_must_not_be_read_independently() {
    assert_ne!(
        parse_color(&["100", "50%", "200"]),
        Ok(ColorSpec::Rgb(100, 128, 200)),
        "3 成分を成分ごとに独立解釈して 10 進と百分率の混在を受けている"
    );
    assert_ne!(
        parse_color(&["50%", "100", "20%"]),
        Ok(ColorSpec::Rgb(128, 100, 51)),
        "3 成分を成分ごとに独立解釈して 10 進と百分率の混在を受けている（逆順）"
    );
    assert!(parse_color(&["100", "50%", "200"]).is_err());
    assert!(parse_color(&["50%", "100", "20%"]).is_err());
}

/// 較正: 先頭の `+` を数の一部として黙って吸収してしまう誤り（`u32` の解析は `+` を受ける）。
#[test]
fn calibration_leading_plus_must_not_be_absorbed() {
    assert_ne!(
        parse_color(&["+5", "0", "0"]),
        Ok(ColorSpec::Rgb(5, 0, 0)),
        "先頭の `+` を黙って吸収している（`-1` は落ちるのに `+1` は通る非対称）"
    );
    assert!(parse_color(&["+5", "0", "0"]).is_err());
}

/// 較正: 混色を「背景 + 文字色 × 2」でなく単純平均にした誤り。
#[test]
fn calibration_mix_must_not_be_a_plain_average() {
    assert_ne!(
        mix_disabled((0, 0, 0), (255, 255, 255)),
        (127, 127, 127),
        "混色が単純平均になっている（文字色の重み 2 が無い）"
    );
}

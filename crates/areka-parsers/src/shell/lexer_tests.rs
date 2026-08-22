//! `lexer` の単体テスト（一般構文分割・タスク 3.1 範囲）。
//!
//! 検証対象は surfaces.txt の**構文トークン分割のみ**（意味割当は decode の領分）:
//! - ブロック境界（ヘッダ行＋別行 `{` … `}`）の `BlockStart`/`BlockEnd` 化。
//! - ブロック内本体行・ヘッダ行の CSV 分割（`[...]` 内カンマは保護）。
//! - `charset,VALUE` 行・トップレベル行の `TopLevel` 化。
//! - `//` コメント行・空行の無視（トークンを生じない）。
//! - 未閉じ `{`・未知先頭語の `Raw` 寛容吸収（走査を中断しない）。
//!
//! ドット付きキー分割・`animationN` 集約・範囲展開・alias 写像は decode の領分ゆえ
//! ここでは検証しない（ドット付きキーは CSV 1 フィールドとして丸ごと保持されること
//! のみ確認する）。

use super::lexer::{Token, lex};

/// `//` コメント行と空行はトークンを一切生じない（要件 9.1）。
#[test]
fn comments_and_blank_lines_are_ignored() {
    let input = "// これはコメント\n\n   \n//########\n";
    assert_eq!(lex(input), vec![]);
}

/// 代表 `surfaceNNN { ... }` 断片（ヘッダ＋別行 `{`＋本体＋`}`）が
/// `BlockStart`(fields)/`Line`(fields)/`BlockEnd` へ分割される（要件 4.1）。
/// 本体行 `element0,overlay,surface0.png,0,0` の CSV 分割（パスは無加工）を確認する。
#[test]
fn surface_block_header_body_end() {
    let input = "surface0\n{\nelement0,overlay,surface0.png,0,0\n}\n";
    assert_eq!(
        lex(input),
        vec![
            Token::BlockStart(vec!["surface0".to_string()]),
            Token::Line(vec![
                "element0".to_string(),
                "overlay".to_string(),
                "surface0.png".to_string(),
                "0".to_string(),
                "0".to_string(),
            ]),
            Token::BlockEnd,
        ],
    );
}

/// バックスラッシュ区切りを含むパスを無加工で保持する（要件 4.3）。
#[test]
fn body_line_preserves_backslash_path_verbatim() {
    let input = "surface10\n{\nelement0,overlay,CityPop\\surface0010.png,0,0\n}\n";
    assert_eq!(
        lex(input),
        vec![
            Token::BlockStart(vec!["surface10".to_string()]),
            Token::Line(vec![
                "element0".to_string(),
                "overlay".to_string(),
                "CityPop\\surface0010.png".to_string(),
                "0".to_string(),
                "0".to_string(),
            ]),
            Token::BlockEnd,
        ],
    );
}

/// ドット付きキーは分割せず CSV 第1フィールドとして丸ごと保持する
/// （`.` 分割・集約は decode の領分・要件 4.1）。
#[test]
fn dotted_key_kept_as_single_field() {
    let input = "surface1000\n{\nanimation1400.interval,bind+random,4\n}\n";
    assert_eq!(
        lex(input),
        vec![
            Token::BlockStart(vec!["surface1000".to_string()]),
            Token::Line(vec![
                "animation1400.interval".to_string(),
                "bind+random".to_string(),
                "4".to_string(),
            ]),
            Token::BlockEnd,
        ],
    );
}

/// 複数行 `descript { version,1 }` ブロックが `BlockStart`/`Line`/`BlockEnd`
/// へ区切られる（スキップは decode の責務・lexer は認識可能にするのみ・要件 3.2）。
#[test]
fn descript_block_is_delimited() {
    let input = "descript\n{\nversion,1\n}\n";
    assert_eq!(
        lex(input),
        vec![
            Token::BlockStart(vec!["descript".to_string()]),
            Token::Line(vec!["version".to_string(), "1".to_string()]),
            Token::BlockEnd,
        ],
    );
}

/// トップレベル `charset,UTF-8` 行はブロックに属さない `TopLevel` として
/// 認識される（スキップは decode の責務・要件 3.1）。
#[test]
fn charset_top_level_line_is_recognized() {
    let input = "charset,UTF-8\n";
    assert_eq!(
        lex(input),
        vec![Token::TopLevel(vec![
            "charset".to_string(),
            "UTF-8".to_string(),
        ])],
    );
}

/// alias 風の行 `6,[2106,2206]` は `["6", "[2106,2206]"]` へ分割され、
/// 配列内カンマは保護される（要件 8.1・角括弧内カンマ非分割）。
#[test]
fn bracket_array_comma_is_protected() {
    let input = "kero.surface.alias\n{\n6,[2106,2206]\n}\n";
    assert_eq!(
        lex(input),
        vec![
            Token::BlockStart(vec!["kero.surface.alias".to_string()]),
            Token::Line(vec!["6".to_string(), "[2106,2206]".to_string()]),
            Token::BlockEnd,
        ],
    );
}

/// 単一 ID の配列 `0,[2100]` も `["0", "[2100]"]` として保持される。
#[test]
fn bracket_array_single_id() {
    let input = "kero.surface.alias\n{\n0,[2100]\n}\n";
    assert_eq!(
        lex(input),
        vec![
            Token::BlockStart(vec!["kero.surface.alias".to_string()]),
            Token::Line(vec!["0".to_string(), "[2100]".to_string()]),
            Token::BlockEnd,
        ],
    );
}

/// 日本語キーの alias 行も配列カンマ保護のうえ 2 フィールドへ分割される（要件 8.2）。
#[test]
fn bracket_array_japanese_key() {
    let input = "kero.surface.alias\n{\n静観,[2106,2206]\n}\n";
    assert_eq!(
        lex(input),
        vec![
            Token::BlockStart(vec!["kero.surface.alias".to_string()]),
            Token::Line(vec!["静観".to_string(), "[2106,2206]".to_string()]),
            Token::BlockEnd,
        ],
    );
}

/// append ヘッダ `surface.append10,2100-2110,2200-2210`（次行 `{`）は
/// ヘッダ CSV を保持した `BlockStart` になる。範囲 `2100-2110` はテキストのまま
/// （範囲展開は decode/下流・lexer は行わない・要件 7.2）。
#[test]
fn append_header_keeps_csv_ranges_verbatim() {
    let input = "surface.append10,2100-2110,2200-2210\n{\ncollision0,52,38,156,80,Head\n}\n";
    assert_eq!(
        lex(input),
        vec![
            Token::BlockStart(vec![
                "surface.append10".to_string(),
                "2100-2110".to_string(),
                "2200-2210".to_string(),
            ]),
            Token::Line(vec![
                "collision0".to_string(),
                "52".to_string(),
                "38".to_string(),
                "156".to_string(),
                "80".to_string(),
                "Head".to_string(),
            ]),
            Token::BlockEnd,
        ],
    );
}

/// 負値を含む pattern 行のフィールド分割（負号はテキストのまま保持・要件 5.5）。
#[test]
fn negative_value_field_preserved() {
    let input = "surface.append2200\n{\nanimation0.pattern3,overlay,-1,80,0,0\n}\n";
    assert_eq!(
        lex(input),
        vec![
            Token::BlockStart(vec!["surface.append2200".to_string()]),
            Token::Line(vec![
                "animation0.pattern3".to_string(),
                "overlay".to_string(),
                "-1".to_string(),
                "80".to_string(),
                "0".to_string(),
                "0".to_string(),
            ]),
            Token::BlockEnd,
        ],
    );
}

/// 未閉じ `{`（`}` 無しで EOF）は `Raw` 吸収され、走査はパニックせず、
/// **その前**の正常ブロックは失われない（要件 9.2）。
#[test]
fn unclosed_block_absorbed_as_raw_preserving_prior_block() {
    let input =
        "surface0\n{\nelement0,overlay,a.png,0,0\n}\nsurface1\n{\nelement0,overlay,b.png,0,0\n";
    let tokens = lex(input);
    // 先頭の正常ブロックは完全にトークン化される。
    assert_eq!(tokens[0], Token::BlockStart(vec!["surface0".to_string()]));
    assert_eq!(
        tokens[1],
        Token::Line(vec![
            "element0".to_string(),
            "overlay".to_string(),
            "a.png".to_string(),
            "0".to_string(),
            "0".to_string(),
        ])
    );
    assert_eq!(tokens[2], Token::BlockEnd);
    // 未閉じの残余は `Raw` として吸収される（少なくとも 1 つの Raw が存在する）。
    assert!(
        tokens.iter().any(|t| matches!(t, Token::Raw(_))),
        "未閉じブロックは Raw 吸収されるべき: {tokens:?}",
    );
}

/// 未知先頭語のトップレベル行は吸収され、**後続**の認識可能ブロックを壊さない
/// （要件 9.2）。未知トップレベル行は `TopLevel` として素朴に保持され、
/// スキップ判断は decode に委ねる。
#[test]
fn unknown_top_level_word_does_not_break_following_block() {
    let input = "mysterious,junk,line\nsurface0\n{\nelement0,overlay,a.png,0,0\n}\n";
    let tokens = lex(input);
    // 未知トップレベル行はトークン化されるが、後続ブロックは無傷。
    let start_idx = tokens
        .iter()
        .position(|t| *t == Token::BlockStart(vec!["surface0".to_string()]))
        .expect("後続の surface0 ブロックが認識されるべき");
    assert_eq!(
        tokens[start_idx + 1],
        Token::Line(vec![
            "element0".to_string(),
            "overlay".to_string(),
            "a.png".to_string(),
            "0".to_string(),
            "0".to_string(),
        ])
    );
    assert_eq!(tokens[start_idx + 2], Token::BlockEnd);
}

/// 行末コメント・ブロック内コメント・空行が混在してもブロック本体のみが
/// トークン化される（要件 9.1）。
#[test]
fn comments_inside_block_are_ignored() {
    let input = "surface1000\n{\n// --- collision ---\ncollision0,93,62,271,130,Head\n\n}\n";
    assert_eq!(
        lex(input),
        vec![
            Token::BlockStart(vec!["surface1000".to_string()]),
            Token::Line(vec![
                "collision0".to_string(),
                "93".to_string(),
                "62".to_string(),
                "271".to_string(),
                "130".to_string(),
                "Head".to_string(),
            ]),
            Token::BlockEnd,
        ],
    );
}

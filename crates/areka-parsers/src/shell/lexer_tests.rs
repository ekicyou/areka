//! `lexer` の単体テスト（一般構文分割）。
//!
//! 検証対象は surfaces.txt の**構文トークン分割のみ**（意味割当は decode の領分）:
//! - 閉じたブレス（見出し行＋別行 `{` … `}`）の `Brace` 化（見出しと本体の CSV 分割・`[...]` 内カンマは保護）。
//! - `charset,VALUE` 行・トップレベル行の `TopLevel` 化。
//! - `//` コメント行・空行の無視（トークンを生じない）。
//! - 見出しの決め: 見出しと `{` の間の行はコメント・本体の中の `{` で前のブレスを閉じて `Reopened`。
//! - 終わりで閉じていないブレスは中身を落として `Unclosed`（前のブレスは失われない）。
//!
//! ドット付きキー分割・`animationN` 集約・範囲展開・alias 写像は decode の領分ゆえ
//! ここでは検証しない（ドット付きキーは CSV 1 フィールドとして丸ごと保持されること
//! のみ確認する）。

use super::lexer::{Token, lex};

/// 欄の列を `Vec<String>` で作る。
fn fields(items: &[&str]) -> Vec<String> {
    items.iter().map(|s| s.to_string()).collect()
}

/// 閉じたブレス 1 つ。
fn brace(header: &[&str], body: &[&[&str]]) -> Token {
    Token::Brace {
        header: fields(header),
        body: body.iter().map(|line| fields(line)).collect(),
    }
}

/// `//` コメント行と空行はトークンを一切生じない（要件 9.1）。
#[test]
fn comments_and_blank_lines_are_ignored() {
    let input = "// これはコメント\n\n   \n//########\n";
    assert_eq!(lex(input), vec![]);
}

/// 代表 `surfaceNNN { ... }` 断片（見出し＋別行 `{`＋本体＋`}`）が 1 つの `Brace` になる（要件 4.1）。
/// 本体行 `element0,overlay,surface0.png,0,0` の CSV 分割（パスは無加工）を確認する。
#[test]
fn surface_block_header_body_end() {
    let input = "surface0\n{\nelement0,overlay,surface0.png,0,0\n}\n";
    assert_eq!(
        lex(input),
        vec![brace(
            &["surface0"],
            &[&["element0", "overlay", "surface0.png", "0", "0"]],
        )],
    );
}

/// バックスラッシュ区切りを含むパスを無加工で保持する（要件 4.3）。
#[test]
fn body_line_preserves_backslash_path_verbatim() {
    let input = "surface10\n{\nelement0,overlay,CityPop\\surface0010.png,0,0\n}\n";
    assert_eq!(
        lex(input),
        vec![brace(
            &["surface10"],
            &[&["element0", "overlay", "CityPop\\surface0010.png", "0", "0"]],
        )],
    );
}

/// ドット付きキーは分割せず CSV 第1フィールドとして丸ごと保持する
/// （`.` 分割・集約は decode の領分・要件 4.1）。
#[test]
fn dotted_key_kept_as_single_field() {
    let input = "surface1000\n{\nanimation1400.interval,bind+random,4\n}\n";
    assert_eq!(
        lex(input),
        vec![brace(
            &["surface1000"],
            &[&["animation1400.interval", "bind+random", "4"]],
        )],
    );
}

/// 複数行 `descript { version,1 }` ブロックが `Brace` に区切られる
/// （スキップは decode の責務・lexer は認識可能にするのみ・要件 3.2）。
#[test]
fn descript_block_is_delimited() {
    let input = "descript\n{\nversion,1\n}\n";
    assert_eq!(lex(input), vec![brace(&["descript"], &[&["version", "1"]])]);
}

/// トップレベル `charset,UTF-8` 行はブロックに属さない `TopLevel` として
/// 認識される（スキップは decode の責務・要件 3.1）。
#[test]
fn charset_top_level_line_is_recognized() {
    let input = "charset,UTF-8\n";
    assert_eq!(
        lex(input),
        vec![Token::TopLevel(fields(&["charset", "UTF-8"]))]
    );
}

/// alias 風の行 `6,[2106,2206]` は `["6", "[2106,2206]"]` へ分割され、
/// 配列内カンマは保護される（要件 8.1・角括弧内カンマ非分割）。
#[test]
fn bracket_array_comma_is_protected() {
    let input = "kero.surface.alias\n{\n6,[2106,2206]\n}\n";
    assert_eq!(
        lex(input),
        vec![brace(&["kero.surface.alias"], &[&["6", "[2106,2206]"]])],
    );
}

/// 単一 ID の配列 `0,[2100]` も `["0", "[2100]"]` として保持される。
#[test]
fn bracket_array_single_id() {
    let input = "kero.surface.alias\n{\n0,[2100]\n}\n";
    assert_eq!(
        lex(input),
        vec![brace(&["kero.surface.alias"], &[&["0", "[2100]"]])],
    );
}

/// 日本語キーの alias 行も配列カンマ保護のうえ 2 フィールドへ分割される（要件 8.2）。
#[test]
fn bracket_array_japanese_key() {
    let input = "kero.surface.alias\n{\n静観,[2106,2206]\n}\n";
    assert_eq!(
        lex(input),
        vec![brace(&["kero.surface.alias"], &[&["静観", "[2106,2206]"]])],
    );
}

/// append ヘッダ `surface.append10,2100-2110,2200-2210`（次行 `{`）は
/// ヘッダ CSV を保持した `Brace` になる。範囲 `2100-2110` はテキストのまま
/// （範囲展開は decode/下流・lexer は行わない・要件 7.2）。
#[test]
fn append_header_keeps_csv_ranges_verbatim() {
    let input = "surface.append10,2100-2110,2200-2210\n{\ncollision0,52,38,156,80,Head\n}\n";
    assert_eq!(
        lex(input),
        vec![brace(
            &["surface.append10", "2100-2110", "2200-2210"],
            &[&["collision0", "52", "38", "156", "80", "Head"]],
        )],
    );
}

/// 負値を含む pattern 行のフィールド分割（負号はテキストのまま保持・要件 5.5）。
#[test]
fn negative_value_field_preserved() {
    let input = "surface.append2200\n{\nanimation0.pattern3,overlay,-1,80,0,0\n}\n";
    assert_eq!(
        lex(input),
        vec![brace(
            &["surface.append2200"],
            &[&["animation0.pattern3", "overlay", "-1", "80", "0", "0"]],
        )],
    );
}

/// 終わりで閉じていないブレス（`}` 無しで EOF）は中身を落として `Unclosed`（見出しと 1 始まりの
/// 行の番号）になり、走査はパニックせず、**その前**の閉じたブレスは失われない（指摘 34・要件 9.2）。
#[test]
fn unclosed_block_reported_preserving_prior_block() {
    let input =
        "surface0\n{\nelement0,overlay,a.png,0,0\n}\nsurface1\n{\nelement0,overlay,b.png,0,0\n";
    assert_eq!(
        lex(input),
        vec![
            brace(
                &["surface0"],
                &[&["element0", "overlay", "a.png", "0", "0"]]
            ),
            Token::Unclosed {
                heading: "surface1".to_string(),
                line: 5,
            },
        ],
    );
}

/// 未知先頭語のトップレベル行は `TopLevel` として素朴に保持され、**後続**の認識可能ブロックを
/// 壊さない（要件 9.2）。スキップ判断は decode に委ねる。
#[test]
fn unknown_top_level_word_does_not_break_following_block() {
    let input = "mysterious,junk,line\nsurface0\n{\nelement0,overlay,a.png,0,0\n}\n";
    assert_eq!(
        lex(input),
        vec![
            Token::TopLevel(fields(&["mysterious", "junk", "line"])),
            brace(
                &["surface0"],
                &[&["element0", "overlay", "a.png", "0", "0"]]
            ),
        ],
    );
}

/// 行末コメント・ブロック内コメント・空行が混在してもブロック本体のみが
/// トークン化される（要件 9.1）。
#[test]
fn comments_inside_block_are_ignored() {
    let input = "surface1000\n{\n// --- collision ---\ncollision0,93,62,271,130,Head\n\n}\n";
    assert_eq!(
        lex(input),
        vec![brace(
            &["surface1000"],
            &[&["collision0", "93", "62", "271", "130", "Head"]],
        )],
    );
}

/// 本体の中に `{` だけの行が来たら、`}` が抜けたとみなし、本体の行から areka の読むブレスの名の行を
/// 見出しに選んで新しいブレスを始める。前のブレスは中身を残して閉じ、`Reopened`（前のブレスの見出しと
/// 行の番号）を出す（指摘 33）。
#[test]
fn brace_line_inside_body_starts_new_brace() {
    let input = "\
surface0
{
element0,overlay,a.png,0,0
surface1
{
element0,overlay,b.png,0,0
}
";
    assert_eq!(
        lex(input),
        vec![
            brace(
                &["surface0"],
                &[&["element0", "overlay", "a.png", "0", "0"]]
            ),
            Token::Reopened {
                heading: "surface0".to_string(),
                line: 1,
            },
            brace(
                &["surface1"],
                &[&["element0", "overlay", "b.png", "0", "0"]]
            ),
        ],
    );
}

/// 見出しと `{` の間の行（ブレス名でも `{}` でもない行）はコメントとして捨て、見出しは areka の読む
/// ブレスの名の行にする（指摘 71・ukadoc「認識されない行はすべてコメント行」）。名の行が無ければ
/// 今どおり `{` の直前の有意な行が見出し。
#[test]
fn non_brace_line_between_header_and_brace_is_comment() {
    let input = "\
surface0
メモ
{
element0,overlay,a.png,0,0
}
foo
{
x,1
}
";
    assert_eq!(
        lex(input),
        vec![
            brace(
                &["surface0"],
                &[&["element0", "overlay", "a.png", "0", "0"]]
            ),
            brace(&["foo"], &[&["x", "1"]]),
        ],
    );
}

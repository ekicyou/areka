//! 表情の表の転記（`parse_surfacetable`）の単体テスト（要件 2.1〜2.7・3.8・5.2〜5.8）。
//!
//! 設計の「行の読み分け」の表の各行を 1 つずつ固定する。検体ファイルは読まず、
//! テストの中に surfacetable.txt の文面を持つ。

use super::{SurfaceTable, SurfaceTableRow, parse_surfacetable};

fn row(id: u32, name: &str, group: &str, scope: u32) -> SurfaceTableRow {
    SurfaceTableRow {
        id,
        name: name.to_string(),
        group: group.to_string(),
        scope,
    }
}

fn table(rows: Vec<SurfaceTableRow>, unreadable: Vec<usize>) -> SurfaceTable {
    SurfaceTable { rows, unreadable }
}

/// 2.1・2.2: `group`＋`scope`＋行 → グループ名と scope が行に写る。
#[test]
fn group_and_scope_are_copied_to_rows() {
    let t = parse_surfacetable("group,本体\n{\nscope,1\n0,素\n1,照れ\n}\n");
    assert_eq!(
        t,
        table(
            vec![row(0, "素", "本体", 1), row(1, "照れ", "本体", 1)],
            vec![]
        )
    );
}

/// 2.3: `scope` の行が無い `group` の行は scope 0。
#[test]
fn group_without_scope_is_zero() {
    let t = parse_surfacetable("group,本体\n{\n0,素\n}\n");
    assert_eq!(t, table(vec![row(0, "素", "本体", 0)], vec![]));
}

/// 2.4: グループ名が空の `group`。
#[test]
fn empty_group_name_is_empty() {
    let t = parse_surfacetable("group,\n{\nscope,1\n10,素\n}\n");
    assert_eq!(t, table(vec![row(10, "素", "", 1)], vec![]));
}

/// 2.5: `group` の外の行（平たいファイル）はグループ名が空・scope 0。
#[test]
fn row_outside_group_is_empty_and_zero() {
    let t = parse_surfacetable("0,素\n1,照れ\n");
    assert_eq!(
        t,
        table(vec![row(0, "素", "", 0), row(1, "照れ", "", 0)], vec![])
    );
}

/// 2.6: 名前を省略した行（`0,`）も空の名前で転記する。
#[test]
fn omitted_name_is_empty() {
    let t = parse_surfacetable("0,\n");
    assert_eq!(t, table(vec![row(0, "", "", 0)], vec![]));
}

/// 2.7: 同じ scope の `group` が 2 つ → 各行は自分のグループ名を持つ。
#[test]
fn two_groups_with_same_scope_keep_own_names() {
    let t = parse_surfacetable("group,甲\n{\nscope,0\n5,笑\n}\ngroup,乙\n{\nscope,0\n3,泣\n}\n");
    assert_eq!(
        t,
        table(vec![row(5, "笑", "甲", 0), row(3, "泣", "乙", 0)], vec![])
    );
}

/// `__disabled` の group の行と `__parts` の行も、載せる・載せないを決めずに転記する。
#[test]
fn disabled_and_parts_are_transcribed() {
    let t = parse_surfacetable("group,__disabled\n{\n10,__parts\n}\n11,__parts\n");
    assert_eq!(
        t,
        table(
            vec![
                row(10, "__parts", "__disabled", 0),
                row(11, "__parts", "", 0)
            ],
            vec![]
        )
    );
}

/// `scope` が行の後に書かれても、同じ group の既に転記した行へ当てる。
#[test]
fn scope_after_rows_applies_to_whole_group() {
    let before = parse_surfacetable("group,g\n{\nscope,2\n0,a\n1,b\n}\n");
    let after = parse_surfacetable("group,g\n{\n0,a\nscope,2\n1,b\n}\n");
    assert_eq!(before, after);
    assert_eq!(
        after,
        table(vec![row(0, "a", "g", 2), row(1, "b", "g", 2)], vec![])
    );
}

/// `scope` の行の後置は、前の group の行には当たらない。
#[test]
fn scope_does_not_reach_previous_group() {
    let t = parse_surfacetable("group,g\n{\n0,a\n}\ngroup,h\n{\n1,b\nscope,1\n}\n");
    assert_eq!(
        t,
        table(vec![row(0, "a", "g", 0), row(1, "b", "h", 1)], vec![])
    );
}

/// `scope` が 2 つあれば後が勝つ（前に転記した行にも当てる）。
#[test]
fn second_scope_wins() {
    let t = parse_surfacetable("group,g\n{\nscope,1\n0,a\nscope,3\n1,b\n}\n");
    assert_eq!(
        t,
        table(vec![row(0, "a", "g", 3), row(1, "b", "g", 3)], vec![])
    );
}

/// 5.3・3.8: `version`・`option`・`//`・空行は行にならず `unreadable` にもならない。
#[test]
fn version_option_comment_and_blank_are_skipped() {
    let t = parse_surfacetable(
        "version,1\noption,DisableNoDefineSurfaces\n// 注釈,です\n\n   \n0,素\n",
    );
    assert_eq!(t, table(vec![row(0, "素", "", 0)], vec![]));
}

/// 5.4: タブと空白の字下げを落として読む。
#[test]
fn tab_and_space_indent_is_ignored() {
    let t = parse_surfacetable("group,g\n\t{\n\t  scope , 1 \n \t0 ,素 \n\t}\n");
    assert_eq!(t, table(vec![row(0, "素", "g", 1)], vec![]));
}

/// 最初の `,` の前だけを整え、後ろ（名前・グループ名）は空白やタブも書かれたとおり写す。
#[test]
fn text_after_first_comma_is_kept_as_written() {
    let t = parse_surfacetable("group,\t本体\n{\n0, 素\n1,照\tれ\n}\n");
    assert_eq!(
        t,
        table(
            vec![row(0, " 素", "\t本体", 0), row(1, "照\tれ", "\t本体", 0)],
            vec![]
        )
    );
}

/// 全角の空白は字として残る（落とすのは ASCII の空白とタブだけ）。
#[test]
fn ideographic_space_is_kept() {
    let t = parse_surfacetable("0,\u{3000}素\n\u{3000}1,照れ\n");
    assert_eq!(t, table(vec![row(0, "\u{3000}素", "", 0)], vec![2]));
}

/// 5.2: 見出し語は大小を区別しない（`Charset`・`GROUP`・`Scope`・`VERSION`・`Option`）。
#[test]
fn keywords_are_case_insensitive() {
    let t =
        parse_surfacetable("Charset,UTF-8\nVERSION,1\nOption,x\nGROUP,g\n{\nScope,1\n0,素\n}\n");
    assert_eq!(t, table(vec![row(0, "素", "g", 1)], vec![]));
}

/// 知っている `charset` の名前は読み飛ばす（どの行にあっても）。
#[test]
fn known_charset_is_skipped() {
    let t = parse_surfacetable("charset,Shift_JIS\n0,素\ncharset,UTF-8\n");
    assert_eq!(t, table(vec![row(0, "素", "", 0)], vec![]));
}

/// 5.8: 知らない `charset` の名前は `unreadable`。読める行は残る。
#[test]
fn unknown_charset_is_unreadable() {
    let t = parse_surfacetable("charset,no-such-encoding\n0,素\n");
    assert_eq!(t, table(vec![row(0, "素", "", 0)], vec![1]));
}

/// 5.5: 閉じていない `{` のままファイルが終わっても、読めた分を返す。
#[test]
fn unclosed_brace_returns_rows_read() {
    let t = parse_surfacetable("group,本体\n{\n0,\n1,照れ");
    assert_eq!(
        t,
        table(
            vec![row(0, "", "本体", 0), row(1, "照れ", "本体", 0)],
            vec![]
        )
    );
}

/// 5.6: 行末の `}` は名前の一部で、次の行も同じ group に属する。
#[test]
fn trailing_brace_is_part_of_name() {
    let t = parse_surfacetable("group,ほつれ銀糸\n{\n100,黒塗り}\n101,白\n}\n");
    assert_eq!(
        t,
        table(
            vec![
                row(100, "黒塗り}", "ほつれ銀糸", 0),
                row(101, "白", "ほつれ銀糸", 0)
            ],
            vec![]
        )
    );
}

/// 名前は最初の `,` より後ろの全部（`,` を含む）。
#[test]
fn name_keeps_later_commas() {
    let t = parse_surfacetable("0,a,b,c\n");
    assert_eq!(t, table(vec![row(0, "a,b,c", "", 0)], vec![]));
}

/// `}` だけの行の後の行は group の外。開いた group が無い `}` は読み飛ばす。
#[test]
fn closing_brace_ends_group() {
    let t = parse_surfacetable("}\ngroup,g\n{\nscope,1\n0,a\n}\n1,b\n}\n");
    assert_eq!(
        t,
        table(vec![row(0, "a", "g", 1), row(1, "b", "", 0)], vec![])
    );
}

/// `{` が無くても `group,` の行で group が始まり、次の `group,` で前の group は閉じる。
#[test]
fn group_without_brace_and_next_group_closes_previous() {
    let t = parse_surfacetable("group,g\nscope,1\n0,a\ngroup,h\n1,b\n");
    assert_eq!(
        t,
        table(vec![row(0, "a", "g", 1), row(1, "b", "h", 0)], vec![])
    );
}

/// 数値は ASCII の数字だけ（先頭の 0 は許す）。
#[test]
fn leading_zero_id_is_number() {
    let t = parse_surfacetable("007,a\n");
    assert_eq!(t, table(vec![row(7, "a", "", 0)], vec![]));
}

/// 5.7: 読めない行の各種は行番号だけを書かれた順に返し、読める行は残る。
#[test]
fn unreadable_lines_are_numbered_in_order() {
    let text = "\
abc,名前
0,素
カンマなし
+5,a
-1,b
,c
99999999999,d
１,全角数字
{ 1,x
}x
group,g
scope,x
scope,
2,照れ
}
scope,1
3,驚き
";
    let t = parse_surfacetable(text);
    assert_eq!(
        t,
        table(
            vec![
                row(0, "素", "", 0),
                row(2, "照れ", "g", 0),
                row(3, "驚き", "", 0)
            ],
            vec![1, 3, 4, 5, 6, 7, 8, 9, 10, 12, 13, 16]
        )
    );
}

/// `\r\n` の改行でも同じに読む。
#[test]
fn crlf_lines_are_read() {
    let t = parse_surfacetable("group,g\r\n{\r\nscope,1\r\n0,素\r\n}\r\n");
    assert_eq!(t, table(vec![row(0, "素", "g", 1)], vec![]));
}

/// 空の文面は空の転記。
#[test]
fn empty_text_is_empty_table() {
    assert_eq!(parse_surfacetable(""), SurfaceTable::default());
}

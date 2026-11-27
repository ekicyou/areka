//! method 忠実転記の完全マトリクス。

use super::{decode, lex};

/// 未知メソッドトークン（`frobnicate`）は妥当性を判定されず method に原文のまま運ばれ、
/// 行は落とされない（parser は語彙の可否を判定しない）。
#[test]
fn unknown_method_token_is_transcribed_verbatim_not_dropped() {
    let input =
        "surface0\n{\nanimation0.interval,bind\nanimation0.pattern0,frobnicate,100,0,0,0\n}\n";
    let shell = decode(lex(input));
    assert_eq!(shell.surfaces.len(), 1);
    let patterns = &shell.surfaces[0].animations[0].patterns;
    assert_eq!(patterns.len(), 1);
    assert_eq!(patterns[0].method.as_str(), "frobnicate");
    assert_eq!(patterns[0].surface_id, 100);
}

/// メソッド欄欠落（`animation0.pattern0` 単独＝field[1] 以降が無い極端に短い行）→ 行は
/// 落とされず、method は空文字 `""`（下流 `Unknown` が吸収）へ倒れ、パニックしない。
#[test]
fn missing_method_field_yields_empty_method_string() {
    let input = "surface0\n{\nanimation0.interval,bind\nanimation0.pattern0\n}\n";
    let shell = decode(lex(input));
    assert_eq!(shell.surfaces.len(), 1);
    let patterns = &shell.surfaces[0].animations[0].patterns;
    assert_eq!(patterns.len(), 1);
    assert_eq!(patterns[0].index, 0);
    assert_eq!(patterns[0].method.as_str(), "");
    assert_eq!(patterns[0].surface_id, 0);
}

/// 非 overlay の実メソッド `base` も落とされず method に原文のまま運ばれる。
#[test]
fn base_method_pattern_row_is_transcribed_not_dropped() {
    let input = "surface0\n{\nanimation0.interval,bind\nanimation0.pattern0,base,100,0,0,0\n}\n";
    let shell = decode(lex(input));
    assert_eq!(shell.surfaces.len(), 1);
    let patterns = &shell.surfaces[0].animations[0].patterns;
    assert_eq!(patterns.len(), 1);
    assert_eq!(patterns[0].method.as_str(), "base");
}

/// overlay / replace / 未知名(frobnicate) / 欠落 の 4 行が **全て** 出現順に保持され、
/// 各 `method.as_str()` が原文どおり（欠落は空文字）であることを 1 本の animation で確かめる。
#[test]
fn full_method_matrix_all_rows_preserved_in_order() {
    let input = "\
surface0
{
animation0.interval,bind
animation0.pattern0,overlay,100,0,0,0
animation0.pattern1,replace,101,0,0,0
animation0.pattern2,frobnicate,102,0,0,0
animation0.pattern3
}
";
    let shell = decode(lex(input));
    assert_eq!(shell.surfaces.len(), 1);
    let patterns = &shell.surfaces[0].animations[0].patterns;
    assert_eq!(patterns.len(), 4);
    let methods: Vec<&str> = patterns.iter().map(|p| p.method.as_str()).collect();
    assert_eq!(methods, vec!["overlay", "replace", "frobnicate", ""]);
    let indices: Vec<u32> = patterns.iter().map(|p| p.index).collect();
    assert_eq!(indices, vec![0, 1, 2, 3]);
}

// --- element定義の描画メソッドの語の集合 ---
//
// `base` の行は、どの番号でも、どのブレスでも、`overlay` と書いた同じ行と同じ値になる。

/// element の件数（`surface*`ブレスと `surface.append*`ブレスの合計）。
fn element_count(input: &str) -> usize {
    let shell = decode(lex(input));
    shell
        .surfaces
        .iter()
        .map(|s| s.elements.len())
        .sum::<usize>()
        + shell
            .appends
            .iter()
            .map(|a| a.elements.len())
            .sum::<usize>()
}

/// 指定した語を第 2 欄に持つ element の行を除いた文面。
fn without_element_lines_of(input: &str, word: &str) -> String {
    input
        .lines()
        .filter(|line| {
            let mut fields = line.split(',');
            let key = fields.next().unwrap_or("");
            !(key.starts_with("element") && fields.next() == Some(word))
        })
        .map(|line| format!("{line}\n"))
        .collect()
}

/// ⑴ `base` の行は `overlay` と書いた同じ行と同じ値になる:
/// `element0,base`・`element1` 以降の `base`・`surface.append*`ブレスの `base`・
/// ファイル名の欄が数字だけの `base`・複数の番号を並べた見出しを含む文面。
#[test]
fn base_element_lines_decode_like_overlay_lines() {
    let input = "\
surface26
{
element0,base,surface0.png,0,0
element1,overlay,face26.png,120,80
}
surface0,1
{
element0,base,body.png,3,4
element2,base,hair.png,5,6
}
surface.append26
{
element3,base,ribbon.png,7,8
}
surface40
{
element0,base,5,0,0
element1,overlay,mouth.png,1,2
}
";
    let as_overlay = input.replace(",base,", ",overlay,");
    assert_ne!(input, as_overlay, "書き替えが空振りしている");
    assert_eq!(decode(lex(input)), decode(lex(&as_overlay)));

    // 較正: base の行を除いた文面より、element がちょうど base の行の数（5）だけ多い。
    let base_lines = input.matches(",base,").count();
    assert_eq!(base_lines, 5);
    assert_eq!(
        element_count(input),
        element_count(&without_element_lines_of(input, "base")) + base_lines
    );
}

/// ⑵ 描けない語の行は値にならず、隣の行は残る: `auto`・空の欄・
/// ukadoc に無い語・`Overlay`（大文字）・`add`・`bind` の行は、その行を除いた文面と同じ。
#[test]
fn undrawable_element_lines_are_absorbed_and_neighbors_survive() {
    let undrawable = [
        "element1,auto,r.png,1,1",
        "element2,,empty.png,2,2",
        "element3,frobnicate,f.png,3,3",
        "element4,Overlay,upper.png,4,4",
        "element5,add,a.png,5,5",
        "element6,bind,b.png,6,6",
    ];
    for line in undrawable {
        let with = format!(
            "surface10\n{{\nelement0,overlay,body.png,0,0\n{line}\nelement9,base,top.png,9,9\n}}\nsurface.append10\n{{\n{line}\nelement7,overlay,app.png,7,7\n}}\n"
        );
        let without = with.replace(&format!("{line}\n"), "");
        assert_ne!(with, without, "除く行が見つからない: {line}");
        assert_eq!(decode(lex(&with)), decode(lex(&without)), "{line}");
        // 較正: 隣の行（overlay 2・base 1）は残っている。
        assert_eq!(element_count(&with), 3, "{line}");
    }
}

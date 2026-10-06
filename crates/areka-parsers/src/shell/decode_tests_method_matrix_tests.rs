//! method 忠実転記の完全マトリクス（元 decode_tests.rs タスク 1.3 区画）。
//!
//! 本ファイルは `decode_tests.rs` のテーマ分割（areka-P0-file-slimming タスク 8.5・要件 1.7）で
//! 切り出したものであり、移したテストの本文は分割前と同一である（末尾の
//! areka-P0-element-base-method タスク 1.1 の区画は後から足した）。

use super::{decode, lex};

// --- タスク 1.3: method 忠実転記の完全マトリクス（overlay/replace/未知名/欠落）＋
//     Interval::Other 転記の檻を decode 層で確定させる ---
//
// 検証範囲（要件 4.6/8.2/8.4）: タスク 1.2 で decode.rs が導入した 2 分岐
// （overlay フィルタ撤去＝非 overlay も落とさない・field[1] を method へ verbatim／
//  fallback-Bind 撤去＝未認識 interval を `Interval::Other` へ転記）を、
// タスク 1.2 の 3 テスト（overlay/replace/sometimes）に加えて **未知名・欠落・base**
// と **行数保存マトリクス** で網羅する。各テストは overlay フィルタ／fallback-Bind が
// 復活すると FAIL する（＝転記分岐を直接ピンする有意テスト）。

/// 未知メソッドトークン（`frobnicate`）は妥当性を判定されず method に verbatim 転記され、
/// 行は落とされない（要件 4.6/8.4・parser は原文を運ぶだけで語彙の可否を判定しない）。
/// overlay フィルタが復活すると `frobnicate != "overlay"` ゆえ行が落ち、len==1 が FAIL する。
#[test]
fn unknown_method_token_is_transcribed_verbatim_not_dropped() {
    let input =
        "surface0\n{\nanimation0.interval,bind\nanimation0.pattern0,frobnicate,100,0,0,0\n}\n";
    let shell = decode(lex(input));
    assert_eq!(shell.surfaces.len(), 1);
    let patterns = &shell.surfaces[0].animations[0].patterns;
    // 未知名でも落ちずに 1 個残り、method は原文どおり。
    assert_eq!(patterns.len(), 1);
    assert_eq!(patterns[0].method.as_str(), "frobnicate");
    assert_eq!(patterns[0].surface_id, 100);
}

/// メソッド欄欠落（`animation0.pattern0` 単独＝field[1] 以降が無い極端に短い行）→ 行は
/// 落とされず、method は空文字 `""`（下流 `Unknown` が吸収）へ倒れ、パニックしない
/// （要件 4.6/8.4・3.3）。overlay フィルタが復活すると field[1]==None ≠ "overlay" で
/// 行が落ち、len==1 が FAIL する。
#[test]
fn missing_method_field_yields_empty_method_string() {
    let input = "surface0\n{\nanimation0.interval,bind\nanimation0.pattern0\n}\n";
    let shell = decode(lex(input));
    assert_eq!(shell.surfaces.len(), 1);
    let patterns = &shell.surfaces[0].animations[0].patterns;
    // 欠落行も落ちずに index を保持したまま 1 個残り、method は空文字。
    assert_eq!(patterns.len(), 1);
    assert_eq!(patterns[0].index, 0);
    assert_eq!(patterns[0].method.as_str(), "");
    // 後続フィールドも欠落ゆえ既定 0（パニックしない）。
    assert_eq!(patterns[0].surface_id, 0);
}

/// 非 overlay の実メソッド `base` も落とされず method に verbatim 転記される（要件 4.6/8.4）。
/// `replace` に続く 2 例目の実メソッドで overlay フィルタ撤去のマトリクスを厚くする。
#[test]
fn base_method_pattern_row_is_transcribed_not_dropped() {
    let input = "surface0\n{\nanimation0.interval,bind\nanimation0.pattern0,base,100,0,0,0\n}\n";
    let shell = decode(lex(input));
    assert_eq!(shell.surfaces.len(), 1);
    let patterns = &shell.surfaces[0].animations[0].patterns;
    assert_eq!(patterns.len(), 1);
    assert_eq!(patterns[0].method.as_str(), "base");
}

/// method 忠実転記の完全マトリクスを 1 本の animation で確定させる:
/// overlay / replace / 未知名(frobnicate) / 欠落 の 4 行が **全て** 出現順に保持され、
/// 各 `method.as_str()` が原文どおり（欠落は空文字）であることをアサートする（要件 4.6/8.4）。
/// これは overlay フィルタ撤去の **行数保存の檻**でもある: フィルタが復活すると overlay 行
/// 1 個だけが残り len==4 が FAIL する。
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
    // 4 行すべてが落ちずに出現順で残る（overlay フィルタ復活なら 1 個に激減する）。
    assert_eq!(patterns.len(), 4);
    let methods: Vec<&str> = patterns.iter().map(|p| p.method.as_str()).collect();
    assert_eq!(methods, vec!["overlay", "replace", "frobnicate", ""]);
    // index も疎を合成せず素直に保持（0..3）。
    let indices: Vec<u32> = patterns.iter().map(|p| p.index).collect();
    assert_eq!(indices, vec![0, 1, 2, 3]);
}

// --- areka-P0-element-base-method タスク 1.1: element定義の描画メソッドの語の集合 ---
//
// 画像の element定義として値にする語は `overlay` と `base` の 2 語だけ（完全一致）。
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

/// ⑴ `base` の行は `overlay` と書いた同じ行と同じ値になる（要件 1.1・1.4・1.5・1.6・3.5）:
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

/// ⑵ 描けない語の行は値にならず、隣の行は残る（要件 2.4・2.5）: `replace`・空の欄・
/// ukadoc に無い語・`Overlay`（大文字）・`add`・`bind` の行は、その行を除いた文面と同じ。
#[test]
fn undrawable_element_lines_are_absorbed_and_neighbors_survive() {
    let undrawable = [
        "element1,replace,r.png,1,1",
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

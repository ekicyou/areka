//! method 忠実転記の完全マトリクス（元 decode_tests.rs タスク 1.3 区画）。
//!
//! 本ファイルは `decode_tests.rs` のテーマ分割（areka-P0-file-slimming タスク 8.5・要件 1.7）で
//! 切り出したものであり、テスト本文は分割前と同一である。

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

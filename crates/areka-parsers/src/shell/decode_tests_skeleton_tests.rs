//! decode スケルトンの骨格挙動とブロックディスパッチ優先順位（元 decode_tests.rs タスク 4.1 区画）。
//!
//! 本ファイルは `decode_tests.rs` のテーマ分割（areka-P0-file-slimming タスク 8.5・要件 1.7）で
//! 切り出したものであり、テスト本文は分割前と同一である。

use super::{AppendTarget, Element, ElementPath, Shell, Surface, decode, lex};

/// 空入力 → 空 `Shell`・非パニック（要件 3.3）。
#[test]
fn empty_input_yields_empty_shell() {
    let shell = decode(lex(""));
    assert_eq!(
        shell,
        Shell {
            surfaces: vec![],
            appends: vec![],
            aliases: vec![],
            animation_sort: None,
            collision_sort: None,
            definitions: vec![],
        }
    );
}

/// ヘッダのみ（charset 行 ＋ descript ブロック）→ 何も保持しない空 `Shell`（要件 3.1/3.2）。
#[test]
fn header_only_is_not_retained() {
    let input = "charset,UTF-8\ndescript\n{\nversion,1\n}\n";
    let shell = decode(lex(input));
    assert_eq!(
        shell,
        Shell {
            surfaces: vec![],
            appends: vec![],
            aliases: vec![],
            animation_sort: None,
            collision_sort: None,
            definitions: vec![],
        }
    );
}

/// `surface0` ブロック → surface ID 0 を持つ枠が 1 個。element 本体はタスク 4.2 で充填。
/// （animation はタスク 4.3 のシームゆえ空・append/alias は本タスクで生成しない。）
#[test]
fn surface_zero_block_extracts_id_zero() {
    let input = "surface0\n{\nelement0,overlay,surface0.png,0,0\n}\n";
    let shell = decode(lex(input));
    assert_eq!(shell.surfaces.len(), 1);
    assert_eq!(
        shell.surfaces[0],
        Surface {
            id: 0,
            targets: vec![AppendTarget::Single(0)],
            elements: vec![Element {
                layer: 0,
                path: ElementPath::new("surface0.png".to_string()),
                x: 0,
                y: 0,
            }],
            collisions: vec![],
            animations: vec![],
        }
    );
    // append/alias は本タスクでは生成しない。
    assert!(shell.appends.is_empty());
    assert!(shell.aliases.is_empty());
}

/// `surface1000` → id 1000 を正しく取り出す（数値抽出）。
#[test]
fn surface_large_id_is_extracted() {
    let input = "surface1000\n{\ncollision0,93,62,271,130,Head\n}\n";
    let shell = decode(lex(input));
    assert_eq!(shell.surfaces.len(), 1);
    assert_eq!(shell.surfaces[0].id, 1000);
}

/// 非数値 ID（`surface` の後ろに数字がない崩れヘッダ）→ パニックせず既定 0 に倒す（要件 3.3）。
#[test]
fn malformed_surface_id_defaults_to_zero() {
    let input = "surface\n{\nelement0,overlay,x.png,0,0\n}\n";
    let shell = decode(lex(input));
    assert_eq!(shell.surfaces.len(), 1);
    assert_eq!(shell.surfaces[0].id, 0);
}

/// ヘッダ欠落（surface 定義がいきなり本体行のみ・崩れ入力）→ 失敗せず継続（要件 3.3）。
#[test]
fn missing_header_does_not_fail() {
    // ブロック開始のない孤立行群。lexer は TopLevel/Raw に落とし、decode は吸収する。
    let input = "element0,overlay,x.png,0,0\ncollision0,1,2,3,4,Head\n";
    let shell = decode(lex(input));
    assert_eq!(
        shell,
        Shell {
            surfaces: vec![],
            appends: vec![],
            aliases: vec![],
            animation_sort: None,
            collision_sort: None,
            definitions: vec![],
        }
    );
}

/// 同一入力を 2 度 decode → 同一結果（決定性・要件 2.4 系）。
#[test]
fn decode_is_deterministic() {
    let input =
        "charset,UTF-8\nsurface0\n{\nelement0,overlay,surface0.png,0,0\n}\nsurface1000\n{\n}\n";
    let a = decode(lex(input));
    let b = decode(lex(input));
    assert_eq!(a, b);
}

/// ディスパッチ優先順位: `kero.surface.alias` / `surface.append*` を `surfaceNNN` と誤分類しない。
/// 本タスクでは alias/append の値化は行わない（4.4/4.5）ので surfaces のみを検査する。
/// 実 surfaceNNN（`surface2100`）だけが surfaces に入り、alias/append ヘッダ由来の
/// 偽 surface が混入しないことを保証する。
#[test]
fn dispatch_precedence_does_not_misclassify_alias_or_append() {
    let input = "\
surface2100
{
element0,overlay,CityPop.png,0,0
}
surface.append10,2100-2110,2200-2210
{
collision0,52,38,156,80,Head
}
kero.surface.alias
{
6,[2106,2206]
}
";
    let shell = decode(lex(input));
    // surfaces に入るのは実 surface2100 のみ（alias/append を surface と誤読しない）。
    assert_eq!(shell.surfaces.len(), 1);
    assert_eq!(shell.surfaces[0].id, 2100);
}

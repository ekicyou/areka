//! decode の単体テスト（タスク 4.1: surface ブロックの枠組みとヘッダの寛容スキップ）。
//!
//! 本タスクが検証するのは decode スケルトンの骨格挙動のみ:
//! - 空入力・ヘッダのみ（charset/descript）・ヘッダ欠落が失敗せず既定状態で継続する（要件 3.1/3.2/3.3）。
//! - `surfaceNNN` ブロックから surface ID と（当面空の）構成要素枠を取り出す（要件 4.1）。
//! - ディスパッチ優先順位により `kero.surface.alias` / `surface.append*` が
//!   `surfaceNNN` と誤分類されない（設計 System Flows のブロックディスパッチ）。
//!
//! element/collision/animation の値化・append/alias の値化はタスク 4.2〜4.6 の領分ゆえ
//! ここでは検証しない（本タスクは枠組みのみ）。期待値はリテラル直書き（sakura 規律）。

use super::decode::decode;
use super::lexer::lex;
use super::model::{Shell, Surface};

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
        }
    );
}

/// `surface0` ブロック → surface ID 0 を持つ枠が 1 個。本体枠は当面空（4.2/4.3 で充填）。
#[test]
fn surface_zero_block_extracts_id_zero() {
    let input = "surface0\n{\nelement0,overlay,surface0.png,0,0\n}\n";
    let shell = decode(lex(input));
    assert_eq!(shell.surfaces.len(), 1);
    assert_eq!(
        shell.surfaces[0],
        Surface {
            id: 0,
            elements: vec![],
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
        }
    );
}

/// 同一入力を 2 度 decode → 同一結果（決定性・要件 2.4 系）。
#[test]
fn decode_is_deterministic() {
    let input = "charset,UTF-8\nsurface0\n{\nelement0,overlay,surface0.png,0,0\n}\nsurface1000\n{\n}\n";
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

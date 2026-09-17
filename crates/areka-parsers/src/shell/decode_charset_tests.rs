//! surfaces.txt の固定物 4 種が同一の解析結果を産むことを固定する決定論テスト
//! （タスク 4.2・要件 9.5／構成上 6.1・6.2）。
//!
//! 窓も I/O も 32bit 成果物も実 SHIORI も実機も要さない——`charset::decode` と
//! `shell::parse` という純関数 2 つの入出力だけで成立する（要件 9.8。`tests/` ではなく
//! `src/` の兄弟ファイルに置く理由がこれ）。
//!
//! ## 何を主張し、何を主張しないか
//! 兄弟の読取点テスト（タスク 4.1）は「本番の読取点が復号を経由すること」までしか
//! 主張せず、復号後の文字列そのものは見ない。ゆえに Shift_JIS を別の 8bit 文字コードと
//! して復号する退化はあちらを素通りする。**その穴を塞ぐのが本ファイル**であり、
//! ここでは文字コードごとの解析結果の等値だけを見る（本番の配線は見ない）。
//!
//! ## 期待値の出どころ
//! 4 種のバイト列は**符号化器から導かず逐語の定数として**持つ（タスク 2.3 と同じ規律）。
//! 実行時に片方を他方へ変換して組むこともしない——両辺が同じだけずれて緑のままになる
//! 恒真を避けるためである。「あ」の 3 系統（UTF-8 `E3 81 82`／Shift_JIS `82 A0`／
//! EUC-JP `A4 A2`）は design.md §Supporting References の登記であり、他のひらがなは
//! JIS X 0208 第 4 区の並び（`ぁ` が 1 点目・以降 Unicode U+3041 からの並びと一致）から
//! 導ける: Shift_JIS は `82 9E+点`、EUC-JP は `A4 A0+点`、UTF-8 は `U+3040+点` の 3 バイト形。
//!
//! | 語 | 用途 | UTF-8 | Shift_JIS | EUC-JP |
//! |---|---|---|---|---|
//! | たちえ | element の画像パス | `E3 81 9F E3 81 A1 E3 81 88` | `82 BD 82 BF 82 A6` | `A4 BF A4 C1 A4 A8` |
//! | あたま | collision の領域名 | `E3 81 82 E3 81 9F E3 81 BE` | `82 A0 82 BD 82 DC` | `A4 A2 A4 BF A4 DE` |
//! | せいかん | alias キー | `E3 81 9B E3 81 84 E3 81 8B E3 82 93` | `82 B9 82 A2 82 A9 82 F1` | `A4 BB A4 A4 A4 AB A4 F3` |
//!
//! いずれも要素名・領域名・キーという**解析結果に文字列として現れる位置**に置いてある。
//! 文字コードの取り違えは必ず `Shell` の中身の差として出る（空の `Shell` 同士が偶然
//! 等しくなる形の等値にはならない・`equal_fixtures_are_not_vacuously_empty` で固定）。

use crate::charset::{DefaultEncoding, decode};
use crate::shell::{
    AliasKey, AppendTarget, Collision, CollisionName, DefRef, Element, ElementPath, Shell, Surface,
    SurfaceAlias, parse,
};

// ---- 固定物 4 種（符号化器から導かない逐語のバイト列）-------------------------
//
// 4 種はいずれも同内容:
//   surface0 { element0,overlay,たちえ.png,10,20 / collision0,1,2,3,4,あたま }
//   kero.surface.alias { せいかん,[10,20] }
// 異なるのは冒頭の charset 宣言の有無・綴りと、日本語部分のバイト表現だけである。

/// ⒜ UTF-8（`charset,UTF-8` 宣言つき）。emo2 の固定物と同じ形（BOM 無し・宣言あり）。
const FIXTURE_UTF_8_DECLARED: &[u8] = b"charset,UTF-8\n\
surface0\n\
{\n\
element0,overlay,\xE3\x81\x9F\xE3\x81\xA1\xE3\x81\x88.png,10,20\n\
collision0,1,2,3,4,\xE3\x81\x82\xE3\x81\x9F\xE3\x81\xBE\n\
}\n\
kero.surface.alias\n\
{\n\
\xE3\x81\x9B\xE3\x81\x84\xE3\x81\x8B\xE3\x82\x93,[10,20]\n\
}\n";

/// ⒝ Shift_JIS（`charset,Shift_JIS` 宣言つき）。宣言どおりに読めることを見る系統。
const FIXTURE_SHIFT_JIS_DECLARED: &[u8] = b"charset,Shift_JIS\n\
surface0\n\
{\n\
element0,overlay,\x82\xBD\x82\xBF\x82\xA6.png,10,20\n\
collision0,1,2,3,4,\x82\xA0\x82\xBD\x82\xDC\n\
}\n\
kero.surface.alias\n\
{\n\
\x82\xB9\x82\xA2\x82\xA9\x82\xF1,[10,20]\n\
}\n";

/// ⒞ Shift_JIS（宣言なし）。既定（`DefaultEncoding::Ansi` ＝ Shift_JIS）で読める系統。
const FIXTURE_SHIFT_JIS_UNDECLARED: &[u8] = b"surface0\n\
{\n\
element0,overlay,\x82\xBD\x82\xBF\x82\xA6.png,10,20\n\
collision0,1,2,3,4,\x82\xA0\x82\xBD\x82\xDC\n\
}\n\
kero.surface.alias\n\
{\n\
\x82\xB9\x82\xA2\x82\xA9\x82\xF1,[10,20]\n\
}\n";

/// ⒟ EUC-JP（`charset,EUC-JP` 宣言つき）。**既定の文字コードだけを特別扱いする実装では
/// 通らない系統**（宣言を無視して既定で読むと ⒞ と同じバイト列を別の意味に読むため）。
const FIXTURE_EUC_JP_DECLARED: &[u8] = b"charset,EUC-JP\n\
surface0\n\
{\n\
element0,overlay,\xA4\xBF\xA4\xC1\xA4\xA8.png,10,20\n\
collision0,1,2,3,4,\xA4\xA2\xA4\xBF\xA4\xDE\n\
}\n\
kero.surface.alias\n\
{\n\
\xA4\xBB\xA4\xA4\xA4\xAB\xA4\xF3,[10,20]\n\
}\n";

/// 対照用: ⒟ と同じ EUC-JP のバイト列から charset 宣言だけを落としたもの。
///
/// 等値の主張が空虚でないこと（＝比較対象が復号の違いに実際に反応すること）を示すための
/// 陽性対照であり、4 種の等値には参加しない。既定は Shift_JIS ゆえ、これを読むと
/// EUC-JP のバイト列が半角カナとして解釈され、解析結果は ⒜ と一致しない。
const CONTROL_EUC_JP_UNDECLARED: &[u8] = b"surface0\n\
{\n\
element0,overlay,\xA4\xBF\xA4\xC1\xA4\xA8.png,10,20\n\
collision0,1,2,3,4,\xA4\xA2\xA4\xBF\xA4\xDE\n\
}\n\
kero.surface.alias\n\
{\n\
\xA4\xBB\xA4\xA4\xA4\xAB\xA4\xF3,[10,20]\n\
}\n";

/// 等値に参加する 4 種（表示名つき。どの系統が崩れたかが失敗メッセージに出る）。
const FIXTURES: &[(&str, &[u8])] = &[
    ("⒜ UTF-8（宣言あり）", FIXTURE_UTF_8_DECLARED),
    ("⒝ Shift_JIS（宣言あり）", FIXTURE_SHIFT_JIS_DECLARED),
    ("⒞ Shift_JIS（宣言なし）", FIXTURE_SHIFT_JIS_UNDECLARED),
    ("⒟ EUC-JP（宣言あり）", FIXTURE_EUC_JP_DECLARED),
];

// ---- 補助 -------------------------------------------------------------------

/// 本番の 2 経路（起動時・配置採寸時）と同じ合わせ方で 1 枚を読む。
///
/// 既定は `DefaultEncoding::Ansi`（＝ Shift_JIS）。宣言があればそちらが優先される。
fn parse_fixture(bytes: &[u8]) -> Shell {
    parse(&decode(bytes, DefaultEncoding::Ansi))
}

/// 4 種が産むべき解析結果を、日本語をソース上のリテラルとして書き下したもの。
///
/// バイト列側の定数とは独立に書いてある（片方から他方を導いていない）。
fn expected_shell() -> Shell {
    Shell {
        surfaces: vec![Surface {
            id: 0,
            targets: vec![AppendTarget::Single(0)],
            elements: vec![Element {
                layer: 0,
                path: ElementPath::new("たちえ.png".to_string()),
                x: 10,
                y: 20,
            }],
            collisions: vec![Collision {
                index: 0,
                left: 1,
                top: 2,
                right: 3,
                bottom: 4,
                name: CollisionName::new("あたま".to_string()),
            }],
            animations: Vec::new(),
        }],
        appends: Vec::new(),
        aliases: vec![SurfaceAlias {
            key: AliasKey::new("せいかん".to_string()),
            ids: vec![10, 20],
        }],
        animation_sort: None,
        collision_sort: None,
        definitions: vec![DefRef::Surface(0), DefRef::Alias(0)],
    }
}

// ---- 等値（要件 9.5）--------------------------------------------------------

/// 4 種すべてが、日本語を含む同一の解析結果を産む（要件 9.5・6.1・6.2）。
///
/// 各系統をリテラルの期待値と突き合わせるので、等値は推移で従うと同時に
/// 「何と等しいのか」まで固定される（全員が一様に壊れて等しいままになる形を排す）。
///
/// **崩れた系統を先頭で打ち切らず全部数え上げる**——最初の 1 件で止めると、
/// どの文字コードが通らなくなったのかが失敗メッセージから読み取れない
/// （既定の文字コードだけを特別扱いする退化では ⒜ と ⒟ が同時に崩れる）。
#[test]
fn four_charset_fixtures_produce_the_same_parse_result() {
    let expected = expected_shell();
    let broken: Vec<&str> = FIXTURES
        .iter()
        .filter(|(_, bytes)| parse_fixture(bytes) != expected)
        .map(|(label, _)| *label)
        .collect();
    assert!(
        broken.is_empty(),
        "期待した解析結果と異なる系統: {}",
        broken.join(" / ")
    );
}

/// 上の等値を「4 種どうしの一致」としても明示する（要件 9.5 の文言どおりの形）。
#[test]
fn four_charset_fixtures_are_mutually_equal() {
    let (base_label, base_bytes) = FIXTURES[0];
    let base = parse_fixture(base_bytes);
    for (label, bytes) in &FIXTURES[1..] {
        assert_eq!(
            parse_fixture(bytes),
            base,
            "{label} が {base_label} と一致しない"
        );
    }
}

/// 等値が空虚でないこと——日本語が実際に解析結果まで届いている（要件 9.5）。
///
/// 空の `Shell` 同士や、日本語を落とした結果同士でも等値は成り立ってしまう。
/// 比較している値の中に復号済みの日本語が入っていることをここで固定する。
#[test]
fn equal_fixtures_are_not_vacuously_empty() {
    for (label, bytes) in FIXTURES {
        let shell = parse_fixture(bytes);
        assert_eq!(shell.surfaces.len(), 1, "{label}: surface が 1 枚でない");
        assert_eq!(
            shell.surfaces[0].elements[0].path.as_str(),
            "たちえ.png",
            "{label}: 要素名の日本語が解析結果に現れていない"
        );
        assert_eq!(
            shell.surfaces[0].collisions[0].name.as_str(),
            "あたま",
            "{label}: 領域名の日本語が解析結果に現れていない"
        );
        assert_eq!(
            shell.aliases[0].key.as_str(),
            "せいかん",
            "{label}: alias キーの日本語が解析結果に現れていない"
        );
    }
}

/// 陽性対照: 宣言を落とした EUC-JP を既定（Shift_JIS）で読むと解析結果は一致しない。
///
/// 上の等値が「何をしても緑になる比較」でないことの証拠である。
/// 同時に、既定の文字コードだけを特別扱いする実装（宣言を見ない実装）が
/// EUC-JP の系統で何を産むかを示している。
#[test]
fn euc_jp_read_as_the_default_charset_does_not_match() {
    let control = parse_fixture(CONTROL_EUC_JP_UNDECLARED);
    assert_ne!(
        control,
        expected_shell(),
        "EUC-JP のバイト列を既定（Shift_JIS）で読んでも一致してしまった：\
         等値の比較が復号の違いに反応していない"
    );
    // 取り違えは文字列の差として現れる（半角カナ化）。
    assert_ne!(
        control.surfaces[0].collisions[0].name.as_str(),
        "あたま",
        "領域名が取り違えの影響を受けていない"
    );
}

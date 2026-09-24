//! 採寸と表示が同じ結果になることの檻（spec: areka-P0-shell-implicit-surface task 5.3・
//! 要件 3.6／7.6／7.12）。
//!
//! 本ファイルが留めるのは 2 つである。
//!
//! 1. **外形の一致**: 里々の標準テンプレート `R_POST_and_KOMAINU` と YAYA の標準テンプレート
//!    `konnoyayame` について、採寸経路（[`build_shell_assets`] → [`compose_size`]）と
//!    表示経路（`load_shell_target` → `ShellTarget::build_world` → `Composer::compose`）の
//!    外形が、面 0・面 10 のどちらでも同じ値になること。期待値は要件 3.1 が挙げる実測
//!    （`R_POST_and_KOMAINU` は 236×462／140×160・`konnoyayame` は 260×390／200×200）。
//! 2. **複製が戻っていないこと**: `measure.rs` と `emo2_boot/assets.rs` の本文が、シェルの
//!    読み込みを自前でやり直していないこと。両者が `load_shell_target` へ寄ったのは task 5.1
//!    であり、どちらか一方だけが自前の読み込みへ戻ると (1) の一致はその場では緑のまま、
//!    実機でだけ「採寸した窓の大きさと表示された絵の外形が食い違う」形で現れる（要件 3.6）。
//!
//! `measure_tests.rs` は 981 行で追記の余地が無いため、本ファイルを兄弟として新設した
//! （要件 7.12）。検体は共有の受け口（`placement_shared_test_support.rs`）経由でのみ受け、
//! `vendors/sample_ghost/` の直パスは書かない（要件 7.11）。

use std::path::PathBuf;

use areka_emo_atlas::WicDecoderArm;
use areka_emo_compose::{BindSet, Composer, EmoWorld, PatternState};
use areka_emo_present::shell_target::load_shell_target;

use super::{build_shell_assets, compose_size};
use crate::placement::shared_test_support::{
    konnoyayame_shell_root, r_post_and_komainu_shell_root, with_com_initialized,
};

// ---------------------------------------------------------------------------
// (1) 採寸経路と表示経路の外形の一致
// ---------------------------------------------------------------------------

/// 相方側の初期面（ukadoc 正典の既定サーフェス＝10・`measure.rs` の `KERO_INITIAL_SURFACE_ID`
/// と同値）。
const KERO_SURFACE_ID: u32 = 10;

/// 検体 1 体ぶんの入力と期待値。
struct Template {
    /// 失敗時の表示に使う検体名。
    name: &'static str,
    /// シェルのフォルダ（共有の受け口が返す絶対パス）。
    shell_dir: PathBuf,
    /// 本体側の初期面（面 0）の外形（物理 px）。
    surface0: (i32, i32),
    /// 相方側の初期面（面 10）の外形（物理 px）。
    surface10: (i32, i32),
}

/// 表示経路の外形を得る（`ShellTarget::build_world` が組んだ面の表を素の `Composer` で合成する）。
///
/// [`compose_size`] と同じ入力（bind なし・空 pattern）で合成し、外形だけを取り出す。
/// 採寸側の関数を流用せず `Composer::compose` を直に呼ぶのは、一致の検査が「同じ関数を 2 度
/// 呼んだだけ」に退化しないようにするためである。
fn present_extent(
    composer: &mut Composer,
    world: &EmoWorld,
    atlas: &areka_emo_atlas::AtlasTable,
    surface_id: u32,
    template: &str,
) -> (i32, i32) {
    let composed = composer
        .compose(
            world,
            atlas,
            surface_id,
            &BindSet::default(),
            &PatternState::default(),
        )
        .unwrap_or_else(|e| panic!("{template}: 表示経路の面 {surface_id} の合成に失敗: {e}"));
    (composed.width() as i32, composed.height() as i32)
}

/// 検体 1 体について、採寸経路と表示経路の外形が期待値どおりに一致することを判定する。
fn assert_both_paths_agree(template: &Template) {
    with_com_initialized(|| {
        let decoder = WicDecoderArm::new().expect("WicDecoderArm（COM 初期化済みスレッド）");
        let mut composer = Composer::new();

        // A: 採寸経路（`measure.rs` が本番で通る 2 段そのもの）。
        let (measure_world, measure_atlas) = build_shell_assets(&template.shell_dir, &decoder)
            .unwrap_or_else(|e| panic!("{}: 採寸経路の資産構築に失敗: {e}", template.name));
        let measured = |id: u32, composer: &mut Composer| -> (i32, i32) {
            let size =
                compose_size(composer, &measure_world, &measure_atlas, id).unwrap_or_else(|e| {
                    panic!("{}: 採寸経路の面 {id} の採寸に失敗: {e}", template.name)
                });
            (size.w, size.h)
        };
        let measured0 = measured(0, &mut composer);
        let measured10 = measured(KERO_SURFACE_ID, &mut composer);

        // B: 表示経路（起動時の資産組立 `build_boot_assets` が通る 2 段と同じ形）。
        let target = load_shell_target(&template.shell_dir, &decoder)
            .unwrap_or_else(|e| panic!("{}: 表示経路のシェル読み込みに失敗: {e}", template.name));
        let present_world = target.build_world();
        let presented0 = present_extent(
            &mut composer,
            &present_world,
            target.atlas(),
            0,
            template.name,
        );
        let presented10 = present_extent(
            &mut composer,
            &present_world,
            target.atlas(),
            KERO_SURFACE_ID,
            template.name,
        );

        // 要件 3.1 の実測値そのもの（採寸側・表示側の双方で判定する＝2 経路が揃って
        // ずれた場合も赤になる）。
        assert_eq!(
            measured0, template.surface0,
            "{}: 採寸経路の面 0 の外形が要件 3.1 の実測と違う",
            template.name
        );
        assert_eq!(
            measured10, template.surface10,
            "{}: 採寸経路の面 10 の外形が要件 3.1 の実測と違う",
            template.name
        );
        // 要件 3.6: 採寸した窓の大きさと表示された絵の外形が食い違わない。
        assert_eq!(
            presented0, measured0,
            "{}: 面 0 の外形が採寸経路と表示経路で食い違う",
            template.name
        );
        assert_eq!(
            presented10, measured10,
            "{}: 面 10 の外形が採寸経路と表示経路で食い違う",
            template.name
        );
    });
}

/// 要件 3.6／7.6: 里々の標準テンプレートで、採寸と表示が同じ外形になる。
///
/// 面 10 はこの検体では `surfaces.txt` に宣言が無く、ファイル名の慣習だけで存在する面である
/// （要件 3.2）。宣言のある面 0 と並べて判定することで、「宣言のある面だけ 2 経路が揃う」形の
/// 退行も赤にできる。
#[test]
fn measure_and_present_agree_on_r_post_and_komainu() {
    assert_both_paths_agree(&Template {
        name: "R_POST_and_KOMAINU",
        shell_dir: r_post_and_komainu_shell_root(),
        surface0: (236, 462),
        surface10: (140, 160),
    });
}

/// 要件 3.6／7.6: YAYA の標準テンプレートで、採寸と表示が同じ外形になる。
#[test]
fn measure_and_present_agree_on_konnoyayame() {
    assert_both_paths_agree(&Template {
        name: "konnoyayame",
        shell_dir: konnoyayame_shell_root(),
        surface0: (260, 390),
        surface10: (200, 200),
    });
}

// ---------------------------------------------------------------------------
// (2) 本文走査（読み込みの複製が戻ったら赤になる）
// ---------------------------------------------------------------------------

/// 見張る綴り。
///
/// 末尾の `(` を落としてあるのは、`use areka_parsers::shell::parse;` で名前だけ持ち込んで
/// `parse(` と呼ぶ形も同じ 1 本で捕まえるため（`use` 行にもこの綴りが現れる）。
const SHELL_PARSE: &str = "shell::parse";

/// 行コメント・ブロックコメント・文字列リテラルを落とした本文。
///
/// 説明文（`//!`・`///`・`//`）の中の綴りを拾って偽陽性にせず、文字列リテラルへ逃がした綴りも
/// 数えないための下拵え。本関数が扱えるのはコメントと通常の文字列リテラルだけで、
/// 二重引用符の文字リテラルと生文字列（`r"…"`・`r#"…"#`）は扱わない。これらが持ち込まれると
/// その `"` を文字列の始まりと読み違え、以降の本文を丸ごと文字列の中身として飲み込む——
/// つまり失敗の向きは「赤になる」ではなく**静かに見逃す**である。だから散文で戒めるのではなく、
/// 走査の前に [`assert_no_blind_spot_literals`] でこの 2 形式が 0 件であることを機械で主張する。
/// 綴りを拾えること・説明文の中の綴りを拾わないことは、この下の 2 本の較正が実物で確かめる。
fn code_only(source: &str) -> String {
    let chars: Vec<char> = source.chars().collect();
    let mut out = String::with_capacity(source.len());
    let mut i = 0;
    // ブロックコメントの入れ子の深さ（Rust のブロックコメントは入れ子になる）。
    let mut block_depth = 0usize;
    while i < chars.len() {
        let c = chars[i];
        let next = chars.get(i + 1).copied();
        if block_depth > 0 {
            if c == '/' && next == Some('*') {
                block_depth += 1;
                i += 2;
            } else if c == '*' && next == Some('/') {
                block_depth -= 1;
                i += 2;
            } else {
                if c == '\n' {
                    out.push('\n');
                }
                i += 1;
            }
            continue;
        }
        if c == '/' && next == Some('/') {
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }
            continue;
        }
        if c == '/' && next == Some('*') {
            block_depth = 1;
            i += 2;
            continue;
        }
        if c == '"' {
            i += 1;
            while i < chars.len() {
                match chars[i] {
                    '\\' => i += 2,
                    '"' => {
                        i += 1;
                        break;
                    }
                    '\n' => {
                        out.push('\n');
                        i += 1;
                    }
                    _ => i += 1,
                }
            }
            continue;
        }
        out.push(c);
        i += 1;
    }
    out
}

/// 二重引用符の文字リテラルの綴り（この綴り自体を走査対象へ書かないよう、文字列で持つ）。
const CHAR_QUOTE_LITERAL: &str = "'\"'";

/// [`code_only`] の前提条件を生のソースに対して主張する。
///
/// 走査器はコメントと通常の文字列リテラルしか落とせない。二重引用符の文字リテラルや生文字列が
/// 混ざると本物の呼び出しを静かに見逃すので、見逃しが起こり得る形式が 0 件であることを
/// 本文の判定の**前**に確かめる。走査対象のファイルは今日すべて、どちらの形式も 0 件である。
fn assert_no_blind_spot_literals(raw: &str, file: &str) {
    assert!(
        !raw.contains(CHAR_QUOTE_LITERAL),
        "{file}: 二重引用符の文字リテラルが持ち込まれた。走査器はこの形式を扱えない\
         （本物の呼び出しを静かに見逃す）。こういうリテラルを持ち込む前に走査器を強くすること"
    );
    let bytes = raw.as_bytes();
    for i in 0..bytes.len() {
        // 生文字列の始まり＝識別子の途中でない `r` に `"` か `#` が続く形。
        if bytes[i] != b'r' {
            continue;
        }
        if i > 0 && (bytes[i - 1].is_ascii_alphanumeric() || bytes[i - 1] == b'_') {
            continue;
        }
        if matches!(bytes.get(i + 1), Some(b'"') | Some(b'#')) {
            panic!(
                "{file}: 生文字列が持ち込まれた。走査器はこの形式を扱えない\
                 （本物の呼び出しを静かに見逃す）。こういうリテラルを持ち込む前に走査器を強くすること"
            );
        }
    }
}

/// 較正（当たり）: 本文の中の実際の呼び出しを [`code_only`] が落とさない。
///
/// 当たりは合成せず、`git HEAD` の実物から採る——`frame_test_support.rs` の `empty_world` は
/// `areka_parsers::shell::parse("")` を本文で呼んでおり、この綴りが説明文の中ではなく
/// 関数の本体に在ることが対照の要点である。
#[test]
fn the_source_scan_still_sees_a_real_call() {
    let code = code_only(include_str!("../emo2_boot/frame_test_support.rs"));
    assert!(
        code.contains(SHELL_PARSE),
        "較正が死んでいる: 本文の中の実際の呼び出しを走査が拾えない"
    );
}

/// 較正（外れ）: 説明文の中にしか無い綴りを [`code_only`] が落とす。
///
/// これも実物から採る——`assets_tests.rs` の `boot_read_honours_the_files_own_charset_declaration`
/// の doc コメントだけがこの綴りを持ち、本文には 1 件も無い。
#[test]
fn the_source_scan_drops_a_spelling_that_only_lives_in_a_comment() {
    let raw = include_str!("../emo2_boot/assets_tests.rs");
    assert!(
        raw.contains(SHELL_PARSE),
        "較正の当たりが失われた: 説明文の中の綴りが実物から消えている（別の対照へ差し替えること）"
    );
    assert!(
        !code_only(raw).contains(SHELL_PARSE),
        "較正が死んでいる: 説明文の中の綴りを走査が拾ってしまう"
    );
}

/// 要件 3.6／7.6: 採寸側がシェルの読み込みを自前でやり直していない。
///
/// `build_shell_assets` は `load_shell_target` を呼ぶだけで、一覧・`surfaces.txt` の読取と解析・
/// 「番号 → 面の画像」の決定・焼きはすべて権威が持つ（task 5.1）。ここへ自前の解析が戻ると、
/// 列挙の規則が 2 実装へ分かれて「採寸した窓寸と実際に合成される枠がずれる」——上の外形の一致は
/// その場では緑のまま通り抜けるので、本文の側でも見張る。
#[test]
fn the_measure_side_does_not_parse_the_shell_itself() {
    let raw = include_str!("measure.rs");
    assert_no_blind_spot_literals(raw, "measure.rs");
    let code = code_only(raw);
    assert!(
        !code.contains(SHELL_PARSE),
        "measure.rs の本文にシェルの解析が戻っている（読み込みの権威は load_shell_target 1 本）"
    );
}

/// 要件 3.6: 起動側（`emo2_boot::assets`）も同じく自前でやり直していない。
///
/// 採寸側と起動側は「同じ絵を見る」ことが要件であり、片側だけが自前の読み込みへ戻る形が
/// いちばん危うい（実機でしか現れない）。2 か所を同じ 1 本の綴りで見張る。
#[test]
fn the_boot_side_does_not_parse_the_shell_itself() {
    let raw = include_str!("../emo2_boot/assets.rs");
    assert_no_blind_spot_literals(raw, "assets.rs");
    let code = code_only(raw);
    assert!(
        !code.contains(SHELL_PARSE),
        "assets.rs の本文にシェルの解析が戻っている（読み込みの権威は load_shell_target 1 本）"
    );
}

/// 要件 3.2: `load_shell_target` を呼ぶ examples 3 本も自前でやり直していない。
///
/// examples は製品と同じ入口で絵を読むことが前提の手元確認の道具であり、ここで自前の解析へ
/// 戻ると、目視で見た絵と製品の絵が静かに別物になる。名前を持ち込むだけの
/// `emo-present.rs`・`collision-probe.rs` は読み込みをしないので対象に含めない。
#[test]
fn the_examples_do_not_parse_the_shell_themselves() {
    let files = [
        (
            "examples/emo-present/setup.rs",
            include_str!("../../examples/emo-present/setup.rs"),
        ),
        (
            "examples/collision-probe/setup.rs",
            include_str!("../../examples/collision-probe/setup.rs"),
        ),
        (
            "examples/window-placement.rs",
            include_str!("../../examples/window-placement.rs"),
        ),
    ];
    for (file, raw) in files {
        assert_no_blind_spot_literals(raw, file);
        let code = code_only(raw);
        assert!(
            !code.contains(SHELL_PARSE),
            "{file} の本文にシェルの解析が戻っている（読み込みの権威は load_shell_target 1 本）"
        );
    }
}

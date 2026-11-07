//! 採寸と表示が同じ結果になることの檻の、本体の片割れ（spec: areka-P0-shell-implicit-surface
//! task 5.3・要件 3.6／7.6、spec: placement-crate-split 要件 5.2・5.3）。
//!
//! 配置の crate の片割れ（`crates/areka-placement/src/measure_template_tests.rs`）が、採寸と表示の
//! 外形の一致と、配置のファイル（`measure.rs`・`measure_first_frame.rs`・
//! `examples/window-placement.rs`）の走査を持つ。ここは本体のファイル（`emo2_boot/assets.rs`・
//! 本体の examples）がシェルの読み込みを自前でやり直していないことを見張る。採寸側と起動側の
//! どちらか一方だけが自前の読み込みへ戻ると、外形の一致はその場では緑のまま、実機でだけ
//! 「採寸した窓の大きさと表示された絵の外形が食い違う」形で現れる（要件 3.6）。
//!
//! 走査器は 2 つの片割れが共に使う `areka_placement::test_support::measure_template` に 1 つだけ
//! あり、ここでは本体の実物で較正する。

use areka_placement::test_support::measure_template::{
    SHELL_PARSE, assert_no_blind_spot_literals, code_only,
};

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

/// 要件 3.6: 起動側（`emo2_boot::assets`）がシェルの読み込みを自前でやり直していない。
///
/// 採寸側と起動側は「同じ絵を見る」ことが要件であり、片側だけが自前の読み込みへ戻る形が
/// いちばん危うい（実機でしか現れない）。採寸側（配置の片割れ）と同じ 1 本の綴りで見張る。
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

/// 要件 3.2: `load_shell_target` を呼ぶ examples 3 本も自前でやり直していない（本体の 2 本。
/// 配置の crate の 1 本は配置の片割れが見る）。
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

//! 翻訳の口（`TranslateSeams`）の兄弟テスト（要件 7.1・7.2・7.4）。

use super::*;

/// 素通しは展開も MAKOTO の口も台詞を 1 文字も変えずに返す（要件 7.2）。
/// 環境変数の綴り・さくらスクリプトのタグ・全角・空文字を混ぜて、どれも手を付けないことを見る。
#[test]
fn passthrough_returns_the_script_unchanged() {
    let seams = TranslateSeams::passthrough();
    for script in [
        "",
        "\\0\\s[0]こんにちは、%username さん。\\e",
        "%(selfname)\\_q%month月\\n[half]%%\\![raise,OnTest]",
        "  前後の空白と\r\n改行\t",
    ] {
        assert_eq!(
            (seams.expand)(script),
            script,
            "展開の素通しは台詞を変えない"
        );
        assert_eq!(
            (seams.makoto)(script, "OnBoot"),
            script,
            "MAKOTO の素通しは台詞を変えない"
        );
    }
}

/// 口の引数は文字列 2 つだけで、別スレッドの運行へ渡せる（要件 7.1・7.4）。
#[test]
fn seams_are_plain_string_closures_movable_to_another_thread() {
    let seams = TranslateSeams {
        expand: Box::new(|s: &str| s.replace("%username", "まず")),
        makoto: Box::new(|s: &str, id: &str| format!("{s}[{id}]")),
    };
    let out = std::thread::spawn(move || {
        let expanded = (seams.expand)("%username");
        (seams.makoto)(&expanded, "OnSecondChange")
    })
    .join()
    .expect("口は Send で別スレッドから呼べる");
    assert_eq!(out, "まず[OnSecondChange]");
}

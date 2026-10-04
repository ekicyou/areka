//! 環境変数の置き換え（`substitute_system_vars`）を字句層で固定するテスト。
//!
//! 置き換える位置は `lex` が `%名前` と読む位置と同じでなければならない（翻訳の前の展開が
//! 再生時の展開と 1 文字も違わないため）。固定するのは、貪欲な名前・タグの角括弧の中の `%`・
//! エスケープ `\%`／`\\`・未閉じの `[`・値の中の `\` と `%` のエスケープ・値の無い名前。
//! すべて決定論（`#[test]`・時計／GPU／実機に依存しない）。

use super::substitute_system_vars;

/// `username` だけに値「太郎」を持つ解決関数で置き換える。
fn sub_taro(input: &str) -> String {
    substitute_system_vars(input, &mut |name| {
        (name == "username").then(|| "太郎".to_string())
    })
}

/// 名前を `%usernameさん` の `username` で止め、値に置き換える。
#[test]
fn replaces_name_followed_by_non_word_text() {
    assert_eq!(sub_taro("%usernameさん"), "太郎さん");
    assert_eq!(sub_taro("こんにちは、%username。"), "こんにちは、太郎。");
}

/// 名前は英数字と `_` を貪欲に読む。`%usernameabc` の名前は `usernameabc` で、
/// `username` の値があっても置き換えない（最長一致の走査とは違う）。
#[test]
fn greedy_name_is_not_cut_at_known_name() {
    assert_eq!(sub_taro("%usernameabc"), "%usernameabc");
    assert_eq!(sub_taro("%username_1さん"), "%username_1さん");
}

/// タグの角括弧の中の `%` は引数の一部で、置き換えない。
#[test]
fn percent_inside_tag_brackets_is_not_replaced() {
    assert_eq!(
        sub_taro("\\![raise,OnX,%username]%username"),
        "\\![raise,OnX,%username]太郎"
    );
    assert_eq!(sub_taro("\\_a[%username]x\\_a"), "\\_a[%username]x\\_a");
}

/// `\%` は文字の `%` で、後ろの名前は置き換えない。
#[test]
fn escaped_percent_is_not_replaced() {
    assert_eq!(sub_taro("\\%username"), "\\%username");
}

/// `\\` は文字の `\` で、直後の `%名前` は置き換える（エスケープの綴りはそのまま）。
#[test]
fn escaped_backslash_then_name_is_replaced() {
    assert_eq!(sub_taro("\\\\%username"), "\\\\太郎");
    assert_eq!(sub_taro("\\\\\\%username"), "\\\\\\%username");
}

/// 未閉じの `[` は `\` から末尾までを 1 かたまりに読むので、その中は置き換えない。
#[test]
fn unclosed_bracket_absorbs_rest() {
    assert_eq!(
        sub_taro("%username\\![raise,%username"),
        "太郎\\![raise,%username"
    );
    assert_eq!(
        sub_taro("%username\\q[\"a,%username]"),
        "太郎\\q[\"a,%username]"
    );
}

/// 値の `\` は `\\`、`%` は `\%` にして埋める（台本の文字として読まれる形）。
#[test]
fn value_backslash_and_percent_are_escaped() {
    let out = substitute_system_vars("[%username]", &mut |name| {
        (name == "username").then(|| "a\\n%b\\".to_string())
    });
    assert_eq!(out, "[a\\\\n\\%b\\\\]");
}

/// `resolve` が `None` を返した名前は `%名前` のまま 1 バイトも変えない。
#[test]
fn unresolved_name_is_left_byte_for_byte() {
    assert_eq!(
        sub_taro("%selfname%username%keroname"),
        "%selfname太郎%keroname"
    );
}

/// 環境変数が無い入力・すべて `None` の入力は、入力と同じ文字列を返す。
#[test]
fn no_change_when_nothing_resolves() {
    let inputs = [
        "",
        "ただの文字列",
        "\\0\\s[10]こんにちは\\w9\\_a[OnX]リンク\\_a\\e",
        "%username%selfname\\![raise,%x]100%%",
        "\\_q未閉じ\\![open,\"a",
        "末尾の裸の\\",
    ];
    for input in inputs {
        let out = substitute_system_vars(input, &mut |_| None);
        assert_eq!(out, input, "入力: {input:?}");
    }
}

/// `resolve` に渡る名前は `lex` が読む名前と同じ（`%` を除いた名前・単独の `%` は空の名前）。
#[test]
fn resolve_receives_lexed_names_in_order() {
    let mut seen = Vec::new();
    let out = substitute_system_vars("%a1_b+%username\\%x[%y]%", &mut |name| {
        seen.push(name.to_string());
        None
    });
    assert_eq!(out, "%a1_b+%username\\%x[%y]%");
    assert_eq!(seen, ["a1_b", "username", "y", ""]);
}

//! `expand_system_vars` の檻（翻訳の前の展開・要件 2.2〜2.5）。
//!
//! 値の決め方は再生時の展開（`resolve_system_var`）と同じであること、読みが変わる並びでは
//! 元の文字列のまま返して警告を 1 件残すこと、展開の前後で再生の文字の並びが同じであることを
//! 確かめる。

use super::*;
use crate::compile::compile;
use dola::cue::{CueCommand, CuePayload};

/// 名前→値の写しを作る。
fn snapshot(pairs: &[(&str, &str)]) -> SystemVarSnapshot {
    let mut vars = SystemVarSnapshot::default();
    for (name, value) in pairs {
        vars.insert(*name, *value);
    }
    vars
}

/// 展開して、その間に出た `sysvar_expand_fallback` の警告の数と一緒に返す。
fn expand_counting(script: &str, vars: &SystemVarSnapshot) -> (String, usize) {
    let (out, events) = log_capture_kit::capture(|| expand_system_vars(script, vars));
    let count = events
        .iter()
        .filter(|e| {
            e.level == tracing::Level::WARN
                && e.field_str("event") == Some("sysvar_expand_fallback")
        })
        .count();
    (out, count)
}

/// 台本を今日の再生の経路（`parse` → `compile`）に通し、利用者に見える並びにする。
/// 文字はかたまりの切れ目を無視してつなぎ、文字以外の命令は種類ごとに 1 要素で挟む。
fn played(script: &str, vars: &SystemVarSnapshot) -> Vec<String> {
    let compiled = compile(&areka_parsers::sakura::parse(script), vars);
    let mut out: Vec<String> = Vec::new();
    let mut in_text = false;
    for cue in compiled.sheet.cues() {
        match &cue.payload {
            CuePayload::Command(CueCommand::Text(s)) => {
                if in_text {
                    out.last_mut().expect("文字のかたまりの続き").push_str(s);
                } else {
                    out.push(format!("text:{s}"));
                    in_text = true;
                }
            }
            other => {
                out.push(format!("{other:?}"));
                in_text = false;
            }
        }
    }
    out
}

/// 写しに値がある `username` はその値になる（要件 2.2・2.3）。
#[test]
fn username_with_value_is_expanded() {
    let vars = snapshot(&[("username", "太郎")]);
    let (out, warned) = expand_counting("%usernameさん", &vars);
    assert_eq!(out, "太郎さん");
    assert_eq!(warned, 0, "読みが変わらないので警告は出ない");
}

/// `username` に値が無ければ既定値「ユーザーさん」になる（要件 2.3）。
#[test]
fn username_without_value_uses_default() {
    let out = expand_system_vars("%usernameさん", &SystemVarSnapshot::default());
    assert_eq!(out, format!("{DEFAULT_USERNAME}さん"));
}

/// `username` 以外でも写しに値がある名前はその値になる（要件 2.3）。
#[test]
fn other_names_with_value_are_expanded() {
    let vars = snapshot(&[
        ("selfname", "さくら"),
        ("selfname2", "春野さくら"),
        ("keroname", "うにゅう"),
    ]);
    let out = expand_system_vars("%selfname・%selfname2・%keroname", &vars);
    assert_eq!(out, "さくら・春野さくら・うにゅう");
}

/// 値が無く既定値も無い名前は綴りのまま残る（要件 2.4）。
#[test]
fn names_without_value_stay_as_spelled() {
    let vars = snapshot(&[("username", "太郎")]);
    let script = "%selfnameと%keronameと%month";
    let out = expand_system_vars(script, &vars);
    assert_eq!(out, script);
}

/// 置き換える名前と綴りのまま残す名前が混ざっても、照合は通って展開される（要件 2.3・2.4）。
#[test]
fn mixed_expanded_and_pass_through_names_expand_without_fallback() {
    let vars = snapshot(&[("username", "太郎")]);
    let (out, warned) = expand_counting("%usernameさん、%month月", &vars);
    assert_eq!(out, "太郎さん、%month月");
    assert_eq!(warned, 0, "読みが変わらないので警告は出ない");
}

/// 値が空文字の名前は空に展開され、照合で誤って元へ戻らない（要件 2.3・2.5）。
#[test]
fn empty_value_expands_to_nothing_without_fallback() {
    let vars = snapshot(&[("username", "")]);
    let (out, warned) = expand_counting(r"\n%username\n", &vars);
    assert_eq!(out, r"\n\n");
    assert_eq!(warned, 0, "空の値で読みは変わらないので警告は出ない");
}

/// 読みが変わる 3 つの並びでは展開せずに元の文字列を返し、警告を 1 件ずつ残す
/// （要件 2.2・2.5）。
#[test]
fn reading_changing_sequences_fall_back_to_original() {
    let cases = [
        // `\w` の直後に数字が来ると待ちの短縮形として数字を飲み込む。
        (r"\w%usernameさん", "3太郎"),
        // `\n` の直後に `[` が来ると改行の引数として読まれる。
        (r"\n%usernameさん", "[50]太郎"),
        // `\_` の直後に文字が来ると `\_a` のタグとして読まれる。
        (r"\_%usernameさん", "a太郎"),
    ];
    for (script, value) in cases {
        let vars = snapshot(&[("username", value)]);
        let (out, warned) = expand_counting(script, &vars);
        assert_eq!(out, script, "{script} は元の文字列のまま返る");
        assert_eq!(warned, 1, "{script} で警告は 1 件");
    }
}

/// 展開の前後で、再生したときの文字の並びが同じ（要件 2.5）。値の中の `\`・`%`、
/// タグの引数の中の `%`、エスケープされた `%` を含む台詞でも同じ。
#[test]
fn playback_is_identical_before_and_after_expansion() {
    let vars = snapshot(&[("username", r"太\郎%x"), ("selfname", "さくら")]);
    let scripts = [
        "%usernameさん",
        r"\h\s[0]%selfnameです。\n%usernameさん、こんにちは。\e",
        r"100\%です。%username",
        r"\![raise,%username]%username",
        r"\q[%username,OnX]%keroname",
        r"%usernameabc%username",
        r"\_%username",
    ];
    for script in scripts {
        let expanded = expand_system_vars(script, &vars);
        assert_eq!(
            played(&expanded, &vars),
            played(script, &vars),
            "{script} → {expanded} で再生の並びが変わった"
        );
    }
    // 照合を素通りしていない（値の入る台詞は実際に展開されている）ことも押さえる。
    assert_eq!(expand_system_vars("%usernameさん", &vars), r"太\\郎\%xさん");
}

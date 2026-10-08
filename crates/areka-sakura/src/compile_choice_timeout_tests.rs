//! `\![set,choicetimeout,時間]` の時間の欄の読み取りの檻（要件 1.4/1.5/2.1〜2.3/6.1/8.1/8.2）。
//!
//! 固定するのは 4 点:
//!
//! 1. 設計の読み方の表の**全行**（欄なし・空欄→既定、整数→秒、前後の空白・`+`・余分な欄、
//!    正負の桁あふれ→飽和、それ以外→読めない）。結果の秒は常に有限。
//! 2. 整数 N のミリ秒を秒にして 1000 倍して丸めると N に戻る（往復で端数の誤差がない）。
//! 3. 台本の文字列から、選択待ちの区切りに入る値（要件 9.1 の各場合・位置と回数・終わりのタグ）。
//! 4. 指定の有無と値（読めない値を含む）で、区切りの値以外の cue の内容と時刻が変わらないこと
//!    （選択肢の無い台本では区切りも出ない・転記はそのまま）。

use super::test_support::{barrier_of, compile, cue_eq};
use super::{
    BarrierKind, ChoiceTimeoutDirective, Cue, CueCommand, CuePayload, parse_choice_timeout,
};

/// 先頭の `"choicetimeout"` に続く欄を並べた `raw_args` を作る。
fn args(fields: &[&str]) -> Vec<String> {
    std::iter::once("choicetimeout")
        .chain(fields.iter().copied())
        .map(str::to_string)
        .collect()
}

/// 読み取り結果を返し、`Secs` なら値が有限であることも確かめる。
fn parse(fields: &[&str]) -> ChoiceTimeoutDirective {
    let got = parse_choice_timeout(&args(fields));
    if let ChoiceTimeoutDirective::Secs(v) = got {
        assert!(v.is_finite(), "{fields:?} → 有限でない秒 {v}");
    }
    got
}

#[test]
fn reading_table_every_row() {
    use ChoiceTimeoutDirective::{Default, Secs, Unreadable};
    let rows: &[(&[&str], ChoiceTimeoutDirective)] = &[
        // 欄なし・空欄 → 既定（2.3）
        (&[], Default),
        (&[""], Default),
        (&[" "], Default),
        // 正の値 → 秒（1.1・1.4）
        (&["500"], Secs(500.0 / 1000.0)),
        // 0・負 → そのままの秒（無期限の判定は kanade・正規化しない・2.1）
        (&["0"], Secs(0.0)),
        (&["-1"], Secs(-1.0 / 1000.0)),
        (&["-5"], Secs(-5.0 / 1000.0)),
        // 前後の空白・`+`・余分な欄
        (&[" 500 "], Secs(500.0 / 1000.0)),
        (&["+500"], Secs(500.0 / 1000.0)),
        (&["500", "x"], Secs(500.0 / 1000.0)),
        // 桁あふれ → 飽和（1.5・2.2）
        (&["99999999999999999999"], Secs(i64::MAX as f64 / 1000.0)),
        (&["-99999999999999999999"], Secs(i64::MIN as f64 / 1000.0)),
        // 整数として読めない（6.1）
        (&["abc"], Unreadable),
        (&["1.5"], Unreadable),
        (&["500ms"], Unreadable),
        (&["５００"], Unreadable),
    ];
    for (fields, want) in rows {
        assert_eq!(parse(fields), *want, "raw_args = {fields:?}");
    }
}

#[test]
fn integer_ms_round_trips_through_secs() {
    for n in [1_i64, 999, 1000, 1001, 30000, 2147483647] {
        let field = n.to_string();
        let ChoiceTimeoutDirective::Secs(secs) = parse(&[&field]) else {
            panic!("{n} が秒として読めない");
        };
        assert_eq!((secs * 1000.0).round() as i64, n, "{n} ms の往復");
    }
}

// ── 台本の文字列 → 選択待ちの区切りの値（要件 9.1） ──

/// 選択肢 2 つ（区切りを出させるための共通の部品）。
const CHOICES: &str = r"\q[はい,a]\q[いいえ,b]";

/// 台本の文字列を parse → compile して cue 列を返す。
fn cues_of(script: &str) -> Vec<Cue> {
    compile(&areka_parsers::sakura::parse(script))
        .sheet
        .cues()
        .to_vec()
}

/// 区切りの cue の数。
fn barrier_count(cues: &[Cue]) -> usize {
    cues.iter()
        .filter(|c| matches!(c.payload, CuePayload::Barrier(_)))
        .count()
}

/// 選択肢を含む台本の、選択待ちの区切りに入る値。区切りは最後の cue にちょうど 1 個。
fn choice_timeout_of(script: &str) -> Option<f64> {
    let cues = cues_of(script);
    assert_eq!(barrier_count(&cues), 1, "{script}: 区切りはちょうど 1 個");
    match barrier_of(cues.last().expect("cue がある")) {
        BarrierKind::WaitForChoice { timeout } => *timeout,
        other => panic!("{script}: 最後の cue が選択待ちの区切りでない: {other:?}"),
    }
}

/// `set,choicetimeout` を転記した汎用キャリアか。
fn is_choice_timeout_carrier(cue: &Cue) -> bool {
    matches!(&cue.payload, CuePayload::Command(cmd)
    if cmd.as_command_carrier().is_some_and(|(name, tokens)| {
        name == "set" && tokens.first() == Some(&"choicetimeout")
    }))
}

/// `set,choicetimeout,{v}` をそのまま運ぶ汎用キャリアの中身。
fn carrier_payload(v: &str) -> CuePayload {
    CuePayload::Command(CueCommand::command_carrier(
        "set",
        vec!["choicetimeout".to_string(), v.to_string()],
    ))
}

/// 転記がちょうど 1 cue で、欄をそのまま運んでいることを確かめる（7.1）。
fn assert_single_verbatim_carrier(cues: &[Cue], v: &str, script: &str) {
    let carriers: Vec<&Cue> = cues
        .iter()
        .filter(|c| is_choice_timeout_carrier(c))
        .collect();
    assert_eq!(carriers.len(), 1, "{script}: 転記はちょうど 1 cue");
    assert_eq!(
        carriers[0].payload,
        carrier_payload(v),
        "{script}: 欄はそのまま転記される"
    );
}

/// 比べるための形: `set,choicetimeout` の転記を除き、区切りの値を `None` にそろえる。
fn without_directive(cues: &[Cue]) -> Vec<Cue> {
    cues.iter()
        .filter(|c| !is_choice_timeout_carrier(c))
        .cloned()
        .map(|mut c| {
            if matches!(
                c.payload,
                CuePayload::Barrier(BarrierKind::WaitForChoice { .. })
            ) {
                c.payload = CuePayload::Barrier(BarrierKind::WaitForChoice { timeout: None });
            }
            c
        })
        .collect()
}

/// cue 列が `cue_eq` で 1 つずつ同一であることを確かめる（内容と時刻）。
fn assert_cues_eq(a: &[Cue], b: &[Cue], what: &str) {
    assert_eq!(a.len(), b.len(), "{what}: cue の数");
    for (i, (x, y)) in a.iter().zip(b).enumerate() {
        assert!(cue_eq(x, y), "{what}: cue[{i}] が違う: {x:?} vs {y:?}");
    }
}

/// 台本の文字列 → 区切りの値の表（9.1・2.4・3.1〜3.3・6.1・6.2・1.5・2.2）。
#[test]
fn script_directive_sets_choice_barrier_timeout() {
    let set = |v: &str| format!(r"\![set,choicetimeout,{v}]");
    let omitted = r"\![set,choicetimeout]";
    let rows: Vec<(String, Option<f64>, &str)> = vec![
        // 指定なし → 未指定（2.4・9.6 の既存の檻と同じ値）
        (format!(r"{CHOICES}\e"), None, "指定なし"),
        // 正の値 → 秒（1.1・1.4）
        (
            format!(r"{}{CHOICES}\e", set("500")),
            Some(500.0 / 1000.0),
            "500",
        ),
        // 0・-1・他の負 → そのままの秒（無期限の判定は kanade・2.1・2.2）
        (format!(r"{}{CHOICES}\e", set("0")), Some(0.0), "0"),
        (
            format!(r"{}{CHOICES}\e", set("-1")),
            Some(-1.0 / 1000.0),
            "-1",
        ),
        (
            format!(r"{}{CHOICES}\e", set("-5")),
            Some(-5.0 / 1000.0),
            "-5",
        ),
        // 省略・空欄 → 既定（2.3）
        (format!(r"{omitted}{CHOICES}\e"), None, "省略"),
        (format!(r"{}{CHOICES}\e", set("")), None, "空欄"),
        // 選択肢の後ろ → 前に書いたときと同じ（3.1）
        (
            format!(r"{CHOICES}{}\e", set("700")),
            Some(700.0 / 1000.0),
            "選択肢の後ろ",
        ),
        // 複数回 → 最後が勝つ・最後が省略なら既定（3.2）
        (
            format!(r"{}{CHOICES}{}\e", set("500"), set("700")),
            Some(700.0 / 1000.0),
            "500→700",
        ),
        (
            format!(r"{}{CHOICES}{omitted}\e", set("500")),
            None,
            "500→省略",
        ),
        // 終わりのタグの後ろは数えない（3.3）
        (
            format!(r"{CHOICES}\e{}", set("500")),
            None,
            r"\e の後ろだけ",
        ),
        (
            format!(r"{CHOICES}\-{}", set("500")),
            None,
            r"\- の後ろだけ",
        ),
        (
            format!(r"{}{CHOICES}\e{}", set("500"), set("700")),
            Some(500.0 / 1000.0),
            r"\e の前 500・後ろ 700",
        ),
        // 読めない値 → 既定（6.1）
        (format!(r"{}{CHOICES}\e", set("abc")), None, "abc"),
        (format!(r"{}{CHOICES}\e", set("1.5")), None, "1.5"),
        (format!(r"{}{CHOICES}\e", set("500ms")), None, "500ms"),
        // 読めない → 読める（6.2）
        (
            format!(r"{}{CHOICES}{}\e", set("abc"), set("700")),
            Some(700.0 / 1000.0),
            "abc→700",
        ),
        // 正負の桁あふれ → 飽和（1.5・2.2）
        (
            format!(r"{}{CHOICES}\e", set("99999999999999999999")),
            Some(i64::MAX as f64 / 1000.0),
            "正の桁あふれ",
        ),
        (
            format!(r"{}{CHOICES}\e", set("-99999999999999999999")),
            Some(i64::MIN as f64 / 1000.0),
            "負の桁あふれ",
        ),
    ];
    for (script, want, what) in &rows {
        assert_eq!(choice_timeout_of(script), *want, "{what}: {script}");
    }
}

/// 選択肢の無い台本では、指定があっても区切りは出ず、転記の 1 cue を除けば全 cue が
/// 指定の無い台本と同一（3.4・6.3・7.1）。
#[test]
fn directive_without_choices_changes_nothing() {
    let plain = cues_of(r"\s[0]こんにちは\w9\s[1]またね\e");
    assert_eq!(barrier_count(&plain), 0, "選択肢が無ければ区切りは出ない");
    for v in ["500", "0", "-1", "", "abc"] {
        let script = format!(r"\s[0]こんにちは\![set,choicetimeout,{v}]\w9\s[1]またね\e");
        let cues = cues_of(&script);
        assert_eq!(barrier_count(&cues), 0, "{script}: 区切りは出ない");
        assert_single_verbatim_carrier(&cues, v, &script);
        assert_cues_eq(&without_directive(&cues), &plain, &script);
    }
}

/// 選択肢を含む台本で、指定なし・読める値・読めない値のどれでも、区切りの値と転記の
/// cue を除いた cue の内容と時刻が同一（6.3・7.1・7.2）。転記は欄をそのまま運ぶ。
#[test]
fn directive_value_changes_only_the_barrier_timeout() {
    let script = |v: Option<&str>| {
        let set = v.map_or(String::new(), |v| format!(r"\![set,choicetimeout,{v}]"));
        format!(r"\s[0]こんにちは{set}\w9\s[1]{CHOICES}\e")
    };
    let plain = cues_of(&script(None));
    for v in ["700", "abc", "0", "-1"] {
        let s = script(Some(v));
        let cues = cues_of(&s);
        assert_single_verbatim_carrier(&cues, v, &s);
        assert_cues_eq(&without_directive(&cues), &plain, &s);
    }
    // 読めない値の台本と読める値の台本も、比べる形では同一（6.3）。
    assert_cues_eq(
        &without_directive(&cues_of(&script(Some("abc")))),
        &without_directive(&cues_of(&script(Some("700")))),
        "abc と 700",
    );
}

/// 検査（`areka-P0-mcp-author-tools` の `check_script`）が再生と同じ読み取りを引けるよう、
/// 読み取りの関数と結果の型はクレートの根から届く（要件 3.4）。
#[test]
fn reader_is_reachable_from_the_crate_root() {
    use crate::{ChoiceTimeoutDirective as Root, parse_choice_timeout as root_parse};
    assert_eq!(root_parse(&args(&["abc"])), Root::Unreadable);
    assert_eq!(root_parse(&args(&[])), Root::Default);
    assert_eq!(root_parse(&args(&["500"])), Root::Secs(0.5));
}

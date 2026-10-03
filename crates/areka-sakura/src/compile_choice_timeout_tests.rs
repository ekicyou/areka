//! `\![set,choicetimeout,時間]` の時間の欄の読み取りの檻（要件 1.4/1.5/2.1〜2.3/6.1/8.1/8.2）。
//!
//! 固定するのは 2 点:
//!
//! 1. 設計の読み方の表の**全行**（欄なし・空欄→既定、整数→秒、前後の空白・`+`・余分な欄、
//!    正負の桁あふれ→飽和、それ以外→読めない）。結果の秒は常に有限。
//! 2. 整数 N のミリ秒を秒にして 1000 倍して丸めると N に戻る（往復で端数の誤差がない）。

use super::{ChoiceTimeoutDirective, parse_choice_timeout};

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

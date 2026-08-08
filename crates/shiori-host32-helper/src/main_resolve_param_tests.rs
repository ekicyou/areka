use super::*;

// R3.4: arg 優先。arg=Some, env=Some → arg（parent_hwnd/load_dir/shiori_name 3 適用の共通観点）。
#[test]
fn arg_takes_priority_over_env() {
    let got = resolve_param(Some("C:\\ghost\\master".to_string()), Some("C:\\env".to_string()));
    assert_eq!(got.as_deref(), Some("C:\\ghost\\master"));
}

// R3.4: env fallback。arg=None → env（load_dir/shiori_name が env のみ供給された経路）。
#[test]
fn env_used_when_arg_absent() {
    let got = resolve_param(None, Some("shiori.dll".to_string()));
    assert_eq!(got.as_deref(), Some("shiori.dll"));
}

// R3.4: arg が空文字/空白のみ → env へ（trim 後に空でない最初の値を採る・parent_hwnd と同型）。
#[test]
fn blank_arg_falls_through_to_env() {
    assert_eq!(
        resolve_param(Some("   ".to_string()), Some("fallback".to_string())).as_deref(),
        Some("fallback")
    );
    assert_eq!(
        resolve_param(Some(String::new()), Some("fallback".to_string())).as_deref(),
        Some("fallback")
    );
}

// R3.4: arg も env も空白のみ → None（両欠落と等価に扱う）。
#[test]
fn blank_arg_and_blank_env_yield_none() {
    assert_eq!(resolve_param(Some("  ".to_string()), Some("\t".to_string())), None);
}

// R3.5: arg=None, env=None → None（必須パラメーター欠落＝呼出側が exit(2) の判定に用いる）。
#[test]
fn both_absent_yield_none() {
    assert_eq!(resolve_param(None, None), None);
}

// 採用値は trim される（cwd 推測でなく arg/env 由来の値を正規化して返す）。
#[test]
fn adopted_value_is_trimmed() {
    assert_eq!(
        resolve_param(Some("  C:\\ghost\\master  ".to_string()), None).as_deref(),
        Some("C:\\ghost\\master")
    );
}

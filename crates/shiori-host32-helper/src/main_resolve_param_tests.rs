use super::*;

#[test]
fn arg_takes_priority_over_env() {
    let got = resolve_param(
        Some("C:\\ghost\\master".to_string()),
        Some("C:\\env".to_string()),
    );
    assert_eq!(got.as_deref(), Some("C:\\ghost\\master"));
}

#[test]
fn env_used_when_arg_absent() {
    let got = resolve_param(None, Some("shiori.dll".to_string()));
    assert_eq!(got.as_deref(), Some("shiori.dll"));
}

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

#[test]
fn blank_arg_and_blank_env_yield_none() {
    assert_eq!(
        resolve_param(Some("  ".to_string()), Some("\t".to_string())),
        None
    );
}

#[test]
fn both_absent_yield_none() {
    assert_eq!(resolve_param(None, None), None);
}

#[test]
fn adopted_value_is_trimmed() {
    assert_eq!(
        resolve_param(Some("  C:\\ghost\\master  ".to_string()), None).as_deref(),
        Some("C:\\ghost\\master")
    );
}

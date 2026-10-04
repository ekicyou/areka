//! `log_history` の決定論テスト（振り分け・`log_type` の語）。

use super::*;
use tracing::Level;

// ---------------------------------------------------------------------------
// 振り分け（要件 2.1〜2.5・2.8・2.9・2.11）
// ---------------------------------------------------------------------------

#[test]
fn warn_and_error_go_to_error_regardless_of_target() {
    // 更新の行でも warn 以上は error だけ（update には入らない）。
    assert_eq!(
        classify(Level::WARN, "areka::update::desk"),
        Some(Kind::Error)
    );
    assert_eq!(classify(Level::ERROR, "areka_update"), Some(Kind::Error));
    // 規則の表に無い外のライブラリも warn 以上は error。
    assert_eq!(
        classify(Level::WARN, "wgpu_core::device"),
        Some(Kind::Error)
    );
    assert_eq!(classify(Level::ERROR, "log"), Some(Kind::Error));
}

#[test]
fn convention_error_target_is_error_from_info_up() {
    assert_eq!(classify(Level::INFO, TARGET_ERROR), Some(Kind::Error));
    assert_eq!(classify(Level::WARN, TARGET_ERROR), Some(Kind::Error));
    assert_eq!(classify(Level::ERROR, TARGET_ERROR), Some(Kind::Error));
    // 自身か下ではなく完全一致だけ。
    assert_eq!(classify(Level::INFO, "areka::log::error::x"), None);
}

#[test]
fn convention_script_target_is_script_at_any_level_from_info_up() {
    assert_eq!(classify(Level::INFO, TARGET_SCRIPT), Some(Kind::Script));
    // warn・error で出しても error より先に当たる。
    assert_eq!(classify(Level::WARN, TARGET_SCRIPT), Some(Kind::Script));
    assert_eq!(classify(Level::ERROR, TARGET_SCRIPT), Some(Kind::Script));
    // 完全一致だけ。
    assert_eq!(classify(Level::INFO, "areka::log::scripts"), None);
    assert_eq!(classify(Level::INFO, "areka::log::script::x"), None);
}

#[test]
fn debug_and_trace_are_never_kept_even_for_convention_targets() {
    assert_eq!(classify(Level::DEBUG, TARGET_SCRIPT), None);
    assert_eq!(classify(Level::TRACE, TARGET_ERROR), None);
    assert_eq!(classify(Level::DEBUG, "areka::install::fetch_url"), None);
    assert_eq!(classify(Level::TRACE, "areka"), None);
    assert_eq!(classify(Level::DEBUG, "bevy_ecs::world"), None);
}

#[test]
fn network_rows_hit_before_update_rows() {
    assert_eq!(
        classify(Level::INFO, "areka::install::fetch_url"),
        Some(Kind::Network)
    );
    assert_eq!(
        classify(Level::INFO, "areka::install::fetch_url::inner"),
        Some(Kind::Network)
    );
    assert_eq!(
        classify(Level::INFO, "areka_update::winhttp"),
        Some(Kind::Network)
    );
    assert_eq!(
        classify(Level::INFO, "areka_update::fetch"),
        Some(Kind::Network)
    );
    // 名前の頭だけ同じものは当たらず、親の update の行へ落ちる。
    assert_eq!(
        classify(Level::INFO, "areka_update::fetcher"),
        Some(Kind::Update)
    );
}

#[test]
fn update_rows_match_self_or_children_only() {
    assert_eq!(classify(Level::INFO, "areka_update"), Some(Kind::Update));
    assert_eq!(classify(Level::INFO, "areka::update"), Some(Kind::Update));
    assert_eq!(
        classify(Level::INFO, "areka::update::desk"),
        Some(Kind::Update)
    );
    assert_eq!(classify(Level::INFO, "areka::install"), Some(Kind::Update));
    assert_eq!(
        classify(Level::INFO, "areka::install::nar"),
        Some(Kind::Update)
    );
    // `::` で区切られていない兄弟は当たらない。
    assert_eq!(classify(Level::INFO, "areka::updater"), None);
    assert_eq!(classify(Level::INFO, "areka_updater"), None);
    assert_eq!(classify(Level::INFO, "areka::installer"), None);
}

#[test]
fn status_rows_and_areka_is_exact_only() {
    for target in [
        "areka",
        "areka::boot_config",
        "areka::boot_resolve",
        "areka::ghost_session",
        "areka::emo2_boot::ghost_switch",
        "ghost-boot",
        "ghost-shutdown",
    ] {
        assert_eq!(
            classify(Level::INFO, target),
            Some(Kind::Status),
            "{target}"
        );
    }
    assert_eq!(
        classify(Level::INFO, "areka::emo2_boot::ghost_switch::x"),
        Some(Kind::Status)
    );
    // `areka` は完全一致だけ。下のモジュールの info は外れる。
    assert_eq!(classify(Level::INFO, "areka::emo2_boot"), None);
    assert_eq!(classify(Level::INFO, "areka::alert"), None);
    assert_eq!(classify(Level::INFO, "areka::menu"), None);
    assert_eq!(classify(Level::INFO, "ghost-boots"), None);
}

#[test]
fn unrelated_info_is_not_kept() {
    assert_eq!(classify(Level::INFO, "kanade"), None);
    assert_eq!(classify(Level::INFO, "areka_mcp"), None);
    assert_eq!(classify(Level::INFO, "log"), None);
    assert_eq!(classify(Level::INFO, ""), None);
}

#[test]
fn rules_table_is_the_designed_thirteen_rows_in_order() {
    let rows: Vec<(&str, Kind, bool)> = RULES.iter().map(|r| (r.target, r.kind, r.exact)).collect();
    assert_eq!(
        rows,
        vec![
            ("areka::install::fetch_url", Kind::Network, false),
            ("areka_update::winhttp", Kind::Network, false),
            ("areka_update::fetch", Kind::Network, false),
            ("areka_update", Kind::Update, false),
            ("areka::update", Kind::Update, false),
            ("areka::install", Kind::Update, false),
            ("areka", Kind::Status, true),
            ("areka::boot_config", Kind::Status, false),
            ("areka::boot_resolve", Kind::Status, false),
            ("areka::ghost_session", Kind::Status, false),
            ("areka::emo2_boot::ghost_switch", Kind::Status, false),
            ("ghost-boot", Kind::Status, false),
            ("ghost-shutdown", Kind::Status, false),
        ]
    );
}

#[test]
fn rule_matches_self_or_double_colon_child() {
    let child = Rule {
        target: "areka::update",
        kind: Kind::Update,
        exact: false,
    };
    assert!(child.matches("areka::update"));
    assert!(child.matches("areka::update::desk"));
    assert!(!child.matches("areka::updater"));
    assert!(!child.matches("areka::update:"));
    assert!(!child.matches("areka"));
    let exact = Rule {
        target: "areka",
        kind: Kind::Status,
        exact: true,
    };
    assert!(exact.matches("areka"));
    assert!(!exact.matches("areka::boot_config"));
}

// ---------------------------------------------------------------------------
// 種別の語と既定（要件 3.2・2.6・2.7 の表）
// ---------------------------------------------------------------------------

#[test]
fn log_type_matches_five_words_ignoring_ascii_case() {
    for kind in [
        Kind::Error,
        Kind::Script,
        Kind::Network,
        Kind::Update,
        Kind::Status,
    ] {
        assert_eq!(Kind::from_log_type(kind.word()), Some(kind));
    }
    assert_eq!(Kind::from_log_type("STATUS"), Some(Kind::Status));
    assert_eq!(Kind::from_log_type("Status"), Some(Kind::Status));
    assert_eq!(Kind::from_log_type("ERROR"), Some(Kind::Error));
}

#[test]
fn log_type_rejects_empty_padded_and_unknown_words() {
    assert_eq!(Kind::from_log_type(""), None);
    assert_eq!(Kind::from_log_type(" error "), None);
    assert_eq!(Kind::from_log_type("error "), None);
    assert_eq!(Kind::from_log_type("bogus"), None);
    assert_eq!(Kind::from_log_type("errors"), None);
}

#[test]
fn kind_words_and_defaults_follow_the_ssp_table() {
    let table: Vec<(&str, &str, &str)> = [
        Kind::Error,
        Kind::Script,
        Kind::Network,
        Kind::Update,
        Kind::Status,
    ]
    .into_iter()
    .map(|k| (k.word(), k.default_label(), k.default_name()))
    .collect();
    assert_eq!(
        table,
        vec![
            ("error", "Error", "[SYSTEM]"),
            ("script", "SSTP", "[SYSTEM]"),
            ("network", "Info", "[SYSTEM]"),
            ("update", "Info", "[SYSTEM]"),
            ("status", "STAT", "STAT"),
        ]
    );
}

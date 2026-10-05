//! `log_history` の決定論テスト（振り分け・`log_type` の語・下書き・入れ物）。

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

// ---------------------------------------------------------------------------
// 下書き（要件 1.5・2.6・2.7・2.10・4.3）
// ---------------------------------------------------------------------------

/// 文字列で渡された欄（生の値と `{:?}` の両方を持つ）。
fn s<'a>(name: &'a str, raw: &'a str, debug: &'a str) -> FieldText<'a> {
    FieldText {
        name,
        debug,
        raw: Some(raw),
    }
}

/// 文字列以外の欄（`{:?}` の形だけ）。
fn d<'a>(name: &'a str, debug: &'a str) -> FieldText<'a> {
    FieldText {
        name,
        debug,
        raw: None,
    }
}

fn msg(text: &str) -> FieldText<'_> {
    d("message", text)
}

/// 種別・語・名・本文の 4 つ組。
fn parts(draft: Draft) -> (Kind, String, String, String) {
    (draft.kind, draft.label, draft.name, draft.body)
}

#[test]
fn body_is_message_then_remaining_fields_in_written_order() {
    let got = draft(
        Level::INFO,
        "areka::update::desk",
        [
            d("count", "3"),
            msg("更新を確かめた"),
            s("event", "check", "\"check\""),
        ],
    )
    .unwrap();
    assert_eq!(got.body, "更新を確かめた count=3 event=\"check\"");
}

#[test]
fn body_is_message_only_or_fields_only() {
    let only_msg = draft(Level::WARN, "kanade", [msg("止まった")]).unwrap();
    assert_eq!(only_msg.body, "止まった");
    // メッセージが無ければ欄だけ（先頭の空白は付けない）。
    let only_fields = draft(
        Level::ERROR,
        "kanade",
        [d("code", "5"), s("path", "a b", "\"a b\"")],
    )
    .unwrap();
    assert_eq!(only_fields.body, "code=5 path=\"a b\"");
    let empty = draft(Level::ERROR, "kanade", []).unwrap();
    assert_eq!(empty.body, "");
}

#[test]
fn string_message_uses_the_raw_value_like_stdout() {
    // `warn!(message = "…")` は record_str で届き `{:?}` に引用符が付く。標準出力と同じく生の値を使う。
    let quoted = draft(
        Level::WARN,
        "kanade",
        [s("message", "止まった", "\"止まった\""), d("code", "5")],
    )
    .unwrap();
    assert_eq!(quoted.body, "止まった code=5");
}

#[test]
fn unclassified_event_has_no_draft() {
    assert_eq!(draft(Level::INFO, "kanade", [msg("x")]), None);
    assert_eq!(draft(Level::DEBUG, TARGET_SCRIPT, [msg("x")]), None);
}

#[test]
fn convention_rows_read_ghost_and_label_four_ways() {
    for target in [TARGET_SCRIPT, TARGET_ERROR] {
        let kind = classify(Level::INFO, target).unwrap();
        // 両方無し → 既定。
        assert_eq!(
            parts(draft(Level::INFO, target, [msg("m")]).unwrap()),
            (
                kind,
                kind.default_label().to_string(),
                kind.default_name().to_string(),
                "m".to_string()
            ),
            "{target}"
        );
        // ghost だけ。
        assert_eq!(
            parts(
                draft(
                    Level::INFO,
                    target,
                    [s("ghost", "emo", "\"emo\""), msg("m")]
                )
                .unwrap()
            ),
            (
                kind,
                kind.default_label().to_string(),
                "emo".to_string(),
                "m".to_string()
            ),
            "{target}"
        );
        // label だけ。
        assert_eq!(
            parts(
                draft(
                    Level::INFO,
                    target,
                    [s("label", "Ghost:OnBoot", "\"Ghost:OnBoot\""), msg("m")]
                )
                .unwrap()
            ),
            (
                kind,
                "Ghost:OnBoot".to_string(),
                kind.default_name().to_string(),
                "m".to_string()
            ),
            "{target}"
        );
        // 両方。欄は本文から除かれ、残りの欄は残る。
        assert_eq!(
            parts(
                draft(
                    Level::INFO,
                    target,
                    [
                        s("ghost", "emo", "\"emo\""),
                        s("label", "SSTP(Local,Auth)", "\"SSTP(Local,Auth)\""),
                        d("n", "1"),
                        msg("m"),
                    ]
                )
                .unwrap()
            ),
            (
                kind,
                "SSTP(Local,Auth)".to_string(),
                "emo".to_string(),
                "m n=1".to_string()
            ),
            "{target}"
        );
    }
}

#[test]
fn convention_field_without_raw_uses_debug_and_last_one_wins() {
    let got = draft(
        Level::INFO,
        TARGET_SCRIPT,
        [
            s("ghost", "first", "\"first\""),
            d("ghost", "Some(\"emo2\")"),
            msg("m"),
            s("label", "a", "\"a\""),
            s("label", "b", "\"b\""),
        ],
    )
    .unwrap();
    assert_eq!(got.name, "Some(\"emo2\")");
    assert_eq!(got.label, "b");
    // どれも本文から除く。
    assert_eq!(got.body, "m");
}

#[test]
fn defaults_follow_the_kind_for_all_five_kinds() {
    let cases = [
        (Level::WARN, "kanade", Kind::Error, "Error", "[SYSTEM]"),
        (Level::INFO, TARGET_SCRIPT, Kind::Script, "SSTP", "[SYSTEM]"),
        (
            Level::INFO,
            "areka::install::fetch_url",
            Kind::Network,
            "Info",
            "[SYSTEM]",
        ),
        (
            Level::INFO,
            "areka_update",
            Kind::Update,
            "Info",
            "[SYSTEM]",
        ),
        (Level::INFO, "areka", Kind::Status, "STAT", "STAT"),
    ];
    for (level, target, kind, label, name) in cases {
        let got = draft(level, target, [msg("m")]).unwrap();
        assert_eq!(
            (got.kind, got.label.as_str(), got.name.as_str()),
            (kind, label, name),
            "{target}"
        );
    }
}

#[test]
fn ghost_and_label_on_other_rows_stay_in_the_body() {
    let got = draft(
        Level::INFO,
        "areka::ghost_session",
        [msg("起動した"), s("ghost", "emo2", "\"emo2\"")],
    )
    .unwrap();
    assert_eq!(got.kind, Kind::Status);
    assert_eq!(got.name, "STAT");
    assert_eq!(got.label, "STAT");
    assert_eq!(got.body, "起動した ghost=\"emo2\"");
    // warn の外の行の label も普通の欄。
    let warn = draft(
        Level::WARN,
        "areka::menu",
        [s("label", "x", "\"x\""), msg("m")],
    )
    .unwrap();
    assert_eq!(
        (warn.label.as_str(), warn.body.as_str()),
        ("Error", "m label=\"x\"")
    );
}

#[test]
fn bridged_log_fields_are_dropped_from_the_body() {
    let got = draft(
        Level::WARN,
        "log",
        [
            msg("wgpu が警告した"),
            s("log.target", "wgpu_core", "\"wgpu_core\""),
            s("log.module_path", "wgpu_core::x", "\"wgpu_core::x\""),
            s("log.file", "x.rs", "\"x.rs\""),
            d("log.line", "Some(12)"),
            d("logical", "true"),
        ],
    )
    .unwrap();
    assert_eq!(got.kind, Kind::Error);
    assert_eq!(got.body, "wgpu が警告した logical=true");
}

#[test]
fn body_is_cut_after_4096_chars_counting_characters() {
    let exact = "あ".repeat(BODY_CAP_CHARS);
    let got = draft(Level::WARN, "kanade", [msg(&exact)]).unwrap();
    assert_eq!(got.body, exact);

    let over = format!("{exact}い");
    let got = draft(Level::WARN, "kanade", [msg(&over)]).unwrap();
    assert_eq!(got.body, format!("{exact}{TRUNCATED_SUFFIX}"));
    assert_eq!(got.body.chars().count(), 4096 + " ...(truncated)".len());
}

// ---------------------------------------------------------------------------
// 入れ物（要件 1.2・1.3・1.4・1.6・1.11）
// ---------------------------------------------------------------------------

const AT: Stamp = Stamp {
    year: 2026,
    month: 10,
    day: 4,
    hour: 9,
    minute: 30,
};

/// その種別の下書き（本文だけ変える）。
fn draft_of(kind: Kind, body: &str) -> Draft {
    Draft {
        kind,
        label: kind.default_label().to_string(),
        name: kind.default_name().to_string(),
        body: body.to_string(),
    }
}

fn ids(h: &History, kind: Kind) -> Vec<u64> {
    h.rows(kind).map(|r| r.id).collect()
}

#[test]
fn empty_history_has_last_id_zero_and_no_rows() {
    let h = History::new();
    assert_eq!(h.last_id(), 0);
    for kind in Kind::ALL {
        assert_eq!(h.rows(kind).count(), 0, "{kind:?}");
    }
}

#[test]
fn ids_run_from_one_across_all_kinds_in_push_order() {
    let mut h = History::new();
    let order = [
        Kind::Status,
        Kind::Error,
        Kind::Status,
        Kind::Script,
        Kind::Network,
        Kind::Update,
        Kind::Error,
    ];
    for (i, kind) in order.into_iter().enumerate() {
        assert_eq!(h.push(AT, draft_of(kind, "x")), i as u64 + 1);
        assert_eq!(h.last_id(), i as u64 + 1);
    }
    assert_eq!(ids(&h, Kind::Status), [1, 3]);
    assert_eq!(ids(&h, Kind::Error), [2, 7]);
    assert_eq!(ids(&h, Kind::Script), [4]);
    assert_eq!(ids(&h, Kind::Network), [5]);
    assert_eq!(ids(&h, Kind::Update), [6]);
}

#[test]
fn record_keeps_the_stamp_and_the_draft() {
    let mut h = History::new();
    let d = Draft {
        kind: Kind::Script,
        label: "SSTP".to_string(),
        name: "emo2".to_string(),
        body: r"\h\s[0]こんにちは\e".to_string(),
    };
    h.push(AT, d);
    let got: Vec<&Record> = h.rows(Kind::Script).collect();
    assert_eq!(
        got,
        [&Record {
            id: 1,
            at: AT,
            kind: Kind::Script,
            label: "SSTP".to_string(),
            name: "emo2".to_string(),
            body: r"\h\s[0]こんにちは\e".to_string(),
        }]
    );
}

#[test]
fn over_cap_drops_only_the_oldest_of_that_kind_and_ids_continue() {
    let mut h = History::new();
    h.push(AT, draft_of(Kind::Status, "s1"));
    h.push(AT, draft_of(Kind::Network, "n1"));
    for i in 0..PER_KIND_CAP + 1 {
        h.push(AT, draft_of(Kind::Error, &format!("e{i}")));
    }
    // 1・2 が他の種別、3〜1,003 が error。最古の 3 だけが消える。
    assert_eq!(h.last_id(), 1003);
    let err = ids(&h, Kind::Error);
    assert_eq!(err.len(), PER_KIND_CAP);
    assert_eq!(err.first(), Some(&4));
    assert_eq!(err.last(), Some(&1003));
    assert!(err.windows(2).all(|w| w[0] + 1 == w[1]));
    assert_eq!(h.rows(Kind::Error).next().unwrap().body, "e1");
    // 他の種別は減らない。
    assert_eq!(ids(&h, Kind::Status), [1]);
    assert_eq!(ids(&h, Kind::Network), [2]);

    // 捨てた後も番号は続きから（捨てた番号を使い回さない）。
    assert_eq!(h.push(AT, draft_of(Kind::Error, "next")), 1004);
    assert_eq!(h.push(AT, draft_of(Kind::Update, "u1")), 1005);
    let err = ids(&h, Kind::Error);
    assert_eq!(err.len(), PER_KIND_CAP);
    assert_eq!((err[0], err[PER_KIND_CAP - 1]), (5, 1004));
    assert_eq!(ids(&h, Kind::Update), [1005]);
}

#[test]
fn unclassified_event_does_not_advance_the_id() {
    let mut h = History::new();
    // 置き場へ積む側の形: 下書きが当たったときだけ積む。
    let mut feed = |level, target: &str| {
        if let Some(d) = draft(level, target, [msg("m")]) {
            h.push(AT, d);
        }
    };
    feed(Level::INFO, "areka");
    feed(Level::INFO, "areka::menu");
    feed(Level::DEBUG, TARGET_SCRIPT);
    feed(Level::TRACE, "areka");
    feed(Level::WARN, "kanade");
    assert_eq!(h.last_id(), 2);
    assert_eq!(ids(&h, Kind::Status), [1]);
    assert_eq!(ids(&h, Kind::Error), [2]);
}

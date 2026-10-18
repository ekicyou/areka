//! `--sstp-listen` の値 → 計画・試す番号の列・待受を立てる口（要件 8.1〜8.8・11.3）。

use super::*;
use log_capture_kit::capture;

fn rejected(item: &str, reason: &'static str) -> ListenNote {
    ListenNote::Rejected {
        item: item.to_owned(),
        reason,
    }
}

fn not_loopback(item: &str) -> ListenNote {
    ListenNote::NotLoopback {
        item: item.to_owned(),
    }
}

#[test]
fn plan_table() {
    use ListenPlan::*;
    let rows: Vec<(Option<&str>, ListenPlan, Vec<ListenNote>)> = vec![
        // 8.1・8.2: ループバックの 3 書式
        (Some("9900"), Ports(vec![9900]), vec![]),
        (Some("localhost:9900"), Ports(vec![9900]), vec![]),
        (Some("LocalHost:9900"), Ports(vec![9900]), vec![]),
        (Some("127.0.0.1:9900"), Ports(vec![9900]), vec![]),
        // 8.3: 書いた順
        (Some("9900,9901"), Ports(vec![9900, 9901]), vec![]),
        (Some("9901, 9900"), Ports(vec![9901, 9900]), vec![]),
        // 8.4: none（大文字小文字を問わない）・値なし
        (Some("none"), Off, vec![]),
        (Some("NONE"), Off, vec![]),
        (None, Off, vec![]),
        // 8.5: 書式の誤りはその項目だけ無視
        (Some("0"), AllRejected, vec![rejected("0", "port")]),
        (Some("70000"), AllRejected, vec![rejected("70000", "port")]),
        (Some("abc"), AllRejected, vec![rejected("abc", "port")]),
        (
            Some("none,9900"),
            Ports(vec![9900]),
            vec![rejected("none", "format")],
        ),
        (
            Some("9900,,9901"),
            Ports(vec![9900, 9901]),
            vec![rejected("", "format")],
        ),
        (
            Some("host:9900"),
            AllRejected,
            vec![rejected("host:9900", "format")],
        ),
        // 8.6: 127.0.0.1 以外
        (
            Some("0.0.0.0:9900"),
            AllRejected,
            vec![not_loopback("0.0.0.0:9900")],
        ),
        (
            Some("192.168.0.5:9900,9901"),
            Ports(vec![9901]),
            vec![not_loopback("192.168.0.5:9900")],
        ),
        (
            Some("[::1]:9900"),
            AllRejected,
            vec![not_loopback("[::1]:9900")],
        ),
        // 8.7: 全部はねる
        (
            Some("0,[::1]:9900,abc"),
            AllRejected,
            vec![
                rejected("0", "port"),
                not_loopback("[::1]:9900"),
                rejected("abc", "port"),
            ],
        ),
    ];
    for (value, want_plan, want_notes) in rows {
        let (got_plan, got_notes) = plan(value);
        assert_eq!(got_plan, want_plan, "value={value:?}");
        assert_eq!(got_notes, want_notes, "value={value:?}");
    }
}

#[test]
fn ports_to_try_four_branches() {
    // 8.8: 渡されていない＝今の候補
    assert_eq!(
        ports_to_try(&ListenPlan::NotGiven, || vec![9801, 9821]),
        Some(vec![9801, 9821])
    );
    // 8.1・8.3: 列ならその列だけ（今の候補は読まない）
    assert_eq!(
        ports_to_try(&ListenPlan::Ports(vec![9900, 9901]), || panic!(
            "今の候補を読んだ"
        )),
        Some(vec![9900, 9901])
    );
    // 8.4・8.7: 待ち受けない（今の候補へ戻らない）
    assert_eq!(
        ports_to_try(&ListenPlan::Off, || panic!("今の候補を読んだ")),
        None
    );
    assert_eq!(
        ports_to_try(&ListenPlan::AllRejected, || panic!("今の候補を読んだ")),
        None
    );
}

#[test]
fn start_mcp_off_raises_nothing_and_logs_info() {
    let (server, events) = capture(|| start_mcp(&ListenPlan::Off, Default::default(), None));
    assert!(server.is_none());
    assert_eq!(events.len(), 1, "{events:?}");
    assert_eq!(events[0].level, tracing::Level::INFO);
    assert_eq!(events[0].field_str("event"), Some("mcp_listen_off"));
}

#[test]
fn start_mcp_all_rejected_raises_nothing_and_logs_error() {
    let (server, events) =
        capture(|| start_mcp(&ListenPlan::AllRejected, Default::default(), None));
    assert!(server.is_none());
    assert_eq!(events.len(), 1, "{events:?}");
    assert_eq!(events[0].level, tracing::Level::ERROR);
    assert_eq!(
        events[0].field_str("event"),
        Some("mcp_listen_all_rejected")
    );
}

#[test]
fn notes_log_one_warn_each() {
    let ((), events) = capture(|| {
        rejected("0", "port").log();
        not_loopback("0.0.0.0:9900").log();
    });
    let kinds: Vec<_> = events
        .iter()
        .map(|e| (e.level, e.field_str("event")))
        .collect();
    assert_eq!(
        kinds,
        vec![
            (tracing::Level::WARN, Some("mcp_listen_rejected")),
            (tracing::Level::WARN, Some("mcp_listen_not_loopback")),
        ]
    );
}

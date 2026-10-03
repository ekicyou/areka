//! `gate` の決定論テスト（要件 4.5 の 8 値＋`Host` の表）。

use super::{Reject, check};

/// 正規の `Host`（Origin の表で Host 側を通しておくための値）。
const GOOD_HOST: Option<&str> = Some("127.0.0.1:1");

/// (要件 4.1, 4.2, 4.3, 4.5) 要件 4.5 の 8 値: `Origin` → 通す／拒む。
#[test]
fn gate_origin_table() {
    let table: [(Option<&str>, bool); 8] = [
        (None, true),
        (Some("http://localhost"), true),
        (Some("http://localhost:3000"), true),
        (Some("http://127.0.0.1:9821"), true),
        (Some("https://localhost"), true),
        (Some("http://evil.example"), false),
        (Some("null"), false),
        (Some("http://localhost.evil.example"), false),
    ];
    for (origin, pass) in table {
        let want = if pass {
            Ok(())
        } else {
            Err(Reject::Origin(origin.unwrap().to_owned()))
        };
        assert_eq!(check(origin, GOOD_HOST), want, "Origin {origin:?}");
    }
}

/// (要件 4.4) `Host`: ループバック 3 種は通す・それ以外と無しは拒む。
#[test]
fn gate_host_table() {
    let table: [(Option<&str>, bool); 5] = [
        (Some("127.0.0.1:9821"), true),
        (Some("localhost:9821"), true),
        (Some("[::1]:9821"), true),
        (Some("evil.example"), false),
        (None, false),
    ];
    for (host, pass) in table {
        let want = if pass {
            Ok(())
        } else {
            Err(Reject::Host(host.map(str::to_owned)))
        };
        assert_eq!(check(None, host), want, "Host {host:?}");
    }
}

/// (要件 4.2) 大文字小文字・ポートの有無と番号・`[::1]` を問わずループバックの `Origin` は通す。
#[test]
fn gate_origin_loopback_variants_pass() {
    for origin in [
        "HTTP://LOCALHOST",
        "Https://LocalHost:443",
        "http://127.0.0.1",
        "https://127.0.0.1:65535",
        "http://[::1]",
        "http://[::1]:3000",
        "https://[::1]",
    ] {
        assert_eq!(check(Some(origin), GOOD_HOST), Ok(()), "Origin {origin:?}");
    }
}

/// (要件 4.3) 他の scheme・host の無い値・紛らわしい綴り・読めない値は拒む。
#[test]
fn gate_origin_tricks_are_rejected() {
    for origin in [
        "",
        "localhost",
        "http://",
        "http://:3000",
        "ftp://localhost",
        "file://localhost",
        "ws://localhost",
        "chrome-extension://localhost",
        "http//localhost",
        "http://localhost@evil.example",
        "http://evil.example@localhost",
        "http://localhost.",
        "http://localhost:",
        "http://localhost:abc",
        "http://localhost:99999",
        "http://localhost/",
        "http://localhost/path",
        "http://localhost?q",
        "http://localhost#f",
        "http://127.0.0.2",
        "http://127.0.0.1.evil.example",
        "http://evil.localhost.example",
        "http://[::1",
        "http://[::1]x",
        "http://[::2]",
        "http://::1",
        "http://localhost\u{0}",
        " http://localhost",
        "http://localhost ",
        "null ",
        "NULL",
    ] {
        assert_eq!(
            check(Some(origin), GOOD_HOST),
            Err(Reject::Origin(origin.to_owned())),
            "Origin {origin:?}"
        );
    }
}

/// (要件 4.4) `Host` も大文字小文字・ポートの有無を問わず、紛らわしい綴りは拒む。
#[test]
fn gate_host_variants() {
    for host in [
        "LOCALHOST",
        "localhost",
        "127.0.0.1",
        "[::1]",
        "LocalHost:1",
    ] {
        assert_eq!(check(None, Some(host)), Ok(()), "Host {host:?}");
    }
    for host in [
        "",
        ":9821",
        "localhost.evil.example",
        "localhost.evil.example:9821",
        "localhost@evil.example",
        "evil.example:9821",
        "localhost.",
        "localhost:",
        "localhost:abc",
        "localhost:99999",
        "::1",
        "[::1",
        "[::1]9821",
        "127.0.0.1/",
        "http://localhost",
    ] {
        assert_eq!(
            check(None, Some(host)),
            Err(Reject::Host(Some(host.to_owned()))),
            "Host {host:?}"
        );
    }
}

/// (要件 4.3, 4.4) 両方悪いときは `Origin` を名指す（ブラウザからの悪用の方を先に記録する）。
#[test]
fn gate_bad_origin_wins_over_bad_host() {
    assert_eq!(
        check(Some("http://evil.example"), Some("evil.example")),
        Err(Reject::Origin("http://evil.example".to_owned()))
    );
    assert_eq!(
        check(Some("http://localhost"), Some("evil.example")),
        Err(Reject::Host(Some("evil.example".to_owned())))
    );
}

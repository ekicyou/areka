//! `Origin`／`Host` の検査（生の値を使わない純粋な判断・自身は記録しない）。
//!
//! 要求の型（hyper の `HeaderMap`）は受けず、ヘッダの値の文字列 2 つだけを受ける。
//! 記録（`warn!`）は呼び出し側の `dispatch` が 1 か所で残す。

// 使い手（dispatch）が繋がるのは task 3.3。それまでは dead_code を期待として置く。
#![expect(
    dead_code,
    reason = "dispatch からの呼び出し（task 3.3）までは使い手が居ない"
)]

/// 拒んだ理由: どちらのヘッダが悪かったかと、その値（`warn!` に載せる）。
#[derive(Debug, PartialEq, Eq)]
pub enum Reject {
    Origin(String),
    Host(Option<String>),
}

/// 通す host（ポートを除いた部分・大文字小文字を問わない）。
const LOOPBACK_HOSTS: [&str; 3] = ["localhost", "127.0.0.1", "[::1]"];

/// `origin`・`host` はヘッダの値そのもの（無ければ None）。
///
/// `Origin` は無しか、`http`／`https` の scheme でループバックの host を持つ値だけ通す
/// （要件 4.1〜4.3）。`Host` はループバックの host だけ通し、無しも拒む（要件 4.4）。
/// 両方悪いときは `Origin` を名指す。
pub fn check(origin: Option<&str>, host: Option<&str>) -> Result<(), Reject> {
    if let Some(o) = origin
        && !origin_is_loopback(o)
    {
        return Err(Reject::Origin(o.to_owned()));
    }
    match host {
        Some(h) if authority_is_loopback(h) => Ok(()),
        _ => Err(Reject::Host(host.map(str::to_owned))),
    }
}

/// `scheme://host[:port]` だけを読む（RFC 6454 の直列化形＝path・userinfo を持たない）。
/// `null`・他の scheme・余計な綴りの付いた値は読めないものとして拒む。
fn origin_is_loopback(origin: &str) -> bool {
    let Some((scheme, authority)) = origin.split_once("://") else {
        return false;
    };
    (scheme.eq_ignore_ascii_case("http") || scheme.eq_ignore_ascii_case("https"))
        && authority_is_loopback(authority)
}

/// `host[:port]`（`[::1]` は角括弧つき）の host がループバックか。
/// ポートは付いていれば 0〜65535 の数字だけ。`localhost.`・`localhost@…`・後ろに
/// path などが続く値は host が一致しないので拒む。
fn authority_is_loopback(authority: &str) -> bool {
    let end = if authority.starts_with('[') {
        authority.find(']').map(|i| i + 1)
    } else {
        Some(authority.find(':').unwrap_or(authority.len()))
    };
    let Some(end) = end else {
        return false;
    };
    let (host, port) = authority.split_at(end);
    let port_ok = port.is_empty()
        || port
            .strip_prefix(':')
            .is_some_and(|p| p.bytes().all(|b| b.is_ascii_digit()) && p.parse::<u16>().is_ok());
    port_ok && LOOPBACK_HOSTS.iter().any(|l| host.eq_ignore_ascii_case(l))
}

#[cfg(test)]
#[path = "gate_tests.rs"]
mod gate_tests;

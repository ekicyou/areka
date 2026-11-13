//! 出どころで実行を制限するタグの判定の表（areka-P0-script-security-level 要件 4.1・4.6・4.8）。
//!
//! ukadoc が出どころで実行を制限するタグを 1 つの表に持つ単一の権威。受け口はここを引き、
//! `permits` が偽ならそのタグの合図を送らない。未実装の行（`enforced: false`）は今の扱いを
//! 変えない（4.6）。`\![leave,nouserbreakmode]` は ukadoc が制限しないので載せない（4.5）。
//! 網羅台帳に `\![execute,…]` の 3 行がまだ無いため、出典は台帳でなく ukadoc の URL で書く。

use areka_sakura::contract::ScriptOrigin;

/// 制限されるタグの形。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RestrictedTag {
    /// `\![name,selector,…]`
    Command {
        name: &'static str,
        selector: &'static str,
    },
    /// `\m[umsg,wparam,lparam]`
    Umsg,
}

/// 制限の区分。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Restriction {
    /// 外部扱いの台本では実行しない。
    RefusedWhenExternal,
    /// SSTP 以外の出どころでは実行しない（SSTP 専用）。
    SstpOnly,
}

/// 判定の表の 1 行。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RestrictionRow {
    pub tag: RestrictedTag,
    /// 記録と台帳に出す綴り（要件 4.1 の綴り）。
    pub display: &'static str,
    pub restriction: Restriction,
    /// ukadoc の注記の要旨。
    pub note: &'static str,
    /// ukadoc の URL（項目の位置まで）。
    pub source: &'static str,
    /// 実装済みで止めがある。
    pub enforced: bool,
}

/// ukadoc（list_sakura_script）を全件たどって得た 5 行。増減したら ukadoc を引き直して直す。
pub(crate) const RESTRICTIONS: [RestrictionRow; 5] = [
    RestrictionRow {
        tag: RestrictedTag::Command {
            name: "execute",
            selector: "filewatch",
        },
        display: r"\![execute,filewatch,…]",
        restriction: Restriction::RefusedWhenExternal,
        note: "SSTP など外部から送られたスクリプトでは実行できない",
        source: "https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_!_execute%2Cfilewatch%2C%E3%83%91%E3%82%B9%2C%E3%82%AA%E3%83%97%E3%82%B7%E3%83%A7%E3%83%B3..._",
        enforced: false,
    },
    RestrictionRow {
        tag: RestrictedTag::Command {
            name: "execute",
            selector: "schedule-add",
        },
        display: r"\![execute,schedule-add,…]",
        restriction: Restriction::RefusedWhenExternal,
        note: "SSTP など外部から送られたスクリプトでは実行できない",
        source: "https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_!_execute%2Cschedule-add%2C%E3%82%AA%E3%83%97%E3%82%B7%E3%83%A7%E3%83%B3%2C%E3%82%AA%E3%83%97%E3%82%B7%E3%83%A7%E3%83%B3%2C%E3%82%AA%E3%83%97%E3%82%B7%E3%83%A7%E3%83%B3..._",
        enforced: false,
    },
    RestrictionRow {
        tag: RestrictedTag::Command {
            name: "execute",
            selector: "schedule-delete",
        },
        display: r"\![execute,schedule-delete,…]",
        restriction: Restriction::RefusedWhenExternal,
        note: "SSTP など外部から送られたスクリプトでは実行できない",
        source: "https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_!_execute%2Cschedule-delete%2CUID%2C%E3%82%AA%E3%83%97%E3%82%B7%E3%83%A7%E3%83%B3..._",
        enforced: false,
    },
    RestrictionRow {
        tag: RestrictedTag::Command {
            name: "enter",
            selector: "nouserbreakmode",
        },
        display: r"\![enter,nouserbreakmode]",
        restriction: Restriction::RefusedWhenExternal,
        note: "通常の SSTP では使用不可（Auth.SSTP（= Owned SSTP）は可）",
        source: "https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_!_enter%2Cnouserbreakmode_",
        enforced: true,
    },
    RestrictionRow {
        tag: RestrictedTag::Umsg,
        display: r"\m[umsg,wparam,lparam]",
        restriction: Restriction::SstpOnly,
        note: "SSTP 専用",
        source: "https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_m_umsg%2Cwparam%2Clparam_",
        enforced: false,
    },
];

/// `\!` の名前と選別子から行を引く（大小は区別しない）。
pub(crate) fn command_row(name: &str, selector: &str) -> Option<&'static RestrictionRow> {
    RESTRICTIONS.iter().find(|row| match row.tag {
        RestrictedTag::Command {
            name: n,
            selector: s,
        } => n.eq_ignore_ascii_case(name) && s.eq_ignore_ascii_case(selector),
        RestrictedTag::Umsg => false,
    })
}

/// その出どころの台本でこの行のタグを実行してよいか。
pub(crate) fn permits(row: &RestrictionRow, origin: &ScriptOrigin) -> bool {
    match row.restriction {
        Restriction::RefusedWhenExternal => !origin.is_external_treated(),
        Restriction::SstpOnly => origin.is_sstp(),
    }
}

#[cfg(test)]
#[path = "script_restriction_tests.rs"]
mod tests;

//! 台本とイベントの出どころの語彙（ukadoc の `SenderType`・Owned SSTP・`SecurityLevel`）。
//!
//! 依存ゼロ・`std` のみ。導く規則（kanade）・線の書き読み（host32）・止める判断（受け口）は
//! ここに持たない。下流は語の組を定数で足し、型の形は変えない（1.8）。
//!
//! # 対応する Requirements（areka-P0-script-security-level）
//! - 1.8: `SenderType` の 9 語・Owned かどうか・`local`／`external` で出どころを表す。
//! - 2.7: `SenderType` は先頭の語（`internal`／`external`）が必ず 1 つ在り、残りをカンマでつなぐ。
//! - 3.2: 実効のレベルは下げるだけで上げない。
//! - 3.3: レベルの値は英字の大文字小文字を区別せずに読む。

/// セキュリティレベル（ukadoc の `SecurityLevel` の 2 値）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecurityLevel {
    Local,
    External,
}

impl SecurityLevel {
    /// 線の綴り（`"local"`／`"external"`）。
    pub fn wire(self) -> &'static str {
        match self {
            SecurityLevel::Local => "local",
            SecurityLevel::External => "external",
        }
    }

    /// 前後の空白を落とし、英字の大文字小文字を区別せずに読む。どちらでもなければ `None`（3.3）。
    pub fn parse(raw: &str) -> Option<SecurityLevel> {
        let raw = raw.trim();
        if raw.eq_ignore_ascii_case("local") {
            Some(SecurityLevel::Local)
        } else if raw.eq_ignore_ascii_case("external") {
            Some(SecurityLevel::External)
        } else {
            None
        }
    }

    /// 下げるだけ: どちらかが `External` なら `External`（3.2）。
    pub fn lowered_by(self, other: SecurityLevel) -> SecurityLevel {
        if self == SecurityLevel::External || other == SecurityLevel::External {
            SecurityLevel::External
        } else {
            SecurityLevel::Local
        }
    }
}

/// `SenderType` の先頭の語（必ず 1 つ在る・2.7）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SenderHead {
    Internal,
    External,
}

impl SenderHead {
    fn wire(self) -> &'static str {
        match self {
            SenderHead::Internal => "internal",
            SenderHead::External => "external",
        }
    }
}

/// `SenderType` の先頭より後ろに付く 7 語。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SenderWord {
    SakuraApi,
    Embed,
    Raise,
    Property,
    Plugin,
    Sstp,
    Communicate,
}

impl SenderWord {
    fn wire(self) -> &'static str {
        match self {
            SenderWord::SakuraApi => "sakuraapi",
            SenderWord::Embed => "embed",
            SenderWord::Raise => "raise",
            SenderWord::Property => "property",
            SenderWord::Plugin => "plugin",
            SenderWord::Sstp => "sstp",
            SenderWord::Communicate => "communicate",
        }
    }
}

/// `SenderType` の語の組（先頭の語＋残りの語）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SenderType {
    pub head: SenderHead,
    pub rest: &'static [SenderWord],
}

impl SenderType {
    /// 先頭の語の後ろに残りをカンマでつなぐ（例 `"internal,raise"`・2.7）。
    pub fn render(&self) -> String {
        let mut s = String::from(self.head.wire());
        for w in self.rest {
            s.push(',');
            s.push_str(w.wire());
        }
        s
    }
}

/// SSTP 系の出どころか、Owned か（ukadoc の Owned SSTP）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SstpKind {
    NotSstp,
    Owned,
    NotOwned,
}

/// 台本・イベントの出どころ（語の組・Owned かどうか・実効のレベル）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScriptOrigin {
    pub sender_type: SenderType,
    pub sstp: SstpKind,
    pub level: SecurityLevel,
}

impl ScriptOrigin {
    /// ゴースト自身: `internal`・SSTP でない・`local`（1.1）。
    pub const GHOST: ScriptOrigin = ScriptOrigin {
        sender_type: SenderType {
            head: SenderHead::Internal,
            rest: &[],
        },
        sstp: SstpKind::NotSstp,
        level: SecurityLevel::Local,
    };

    /// MCP の SEND（Owned SSTP 相当）: `internal`・Owned・`local`（1.2・SSP の実測で
    /// `OnTranslate` の `SenderType` は `internal`）。
    pub const MCP_SEND: ScriptOrigin = ScriptOrigin {
        sender_type: SenderType {
            head: SenderHead::Internal,
            rest: &[],
        },
        sstp: SstpKind::Owned,
        level: SecurityLevel::Local,
    };

    /// MCP の NOTIFY（Owned SSTP 相当）: `external,sstp`・Owned・`local`（1.3）。
    pub const MCP_NOTIFY: ScriptOrigin = ScriptOrigin {
        sender_type: SenderType {
            head: SenderHead::External,
            rest: &[SenderWord::Sstp],
        },
        sstp: SstpKind::Owned,
        level: SecurityLevel::Local,
    };

    /// `\![get,property]` の返しのイベント: `internal,property`・SSTP でない・`local`
    /// （shell-scale-linkage 1.3・SSP の実測どおり）。
    pub const PROPERTY: ScriptOrigin = ScriptOrigin {
        sender_type: SenderType {
            head: SenderHead::Internal,
            rest: &[SenderWord::Property],
        },
        sstp: SstpKind::NotSstp,
        level: SecurityLevel::Local,
    };

    /// 実効のレベルを下げるだけ（`External` を `Local` に戻さない・3.2）。
    pub fn lowered_by(self, level: SecurityLevel) -> ScriptOrigin {
        ScriptOrigin {
            level: self.level.lowered_by(level),
            ..self
        }
    }

    /// 外部扱い: 実効のレベルが `External` か、Owned でない SSTP（要件の用語の定義）。
    pub fn is_external_treated(&self) -> bool {
        self.level == SecurityLevel::External || self.sstp == SstpKind::NotOwned
    }

    /// SSTP 系の出どころか（`\m[…]` の判定に使う）。
    pub fn is_sstp(&self) -> bool {
        self.sstp != SstpKind::NotSstp
    }
}

#[cfg(test)]
#[path = "origin_tests.rs"]
mod tests;

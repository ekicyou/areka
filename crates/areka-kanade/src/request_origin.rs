//! 要求の出どころ（送り手の名前と出どころの組）と、送るイベントの出所カテゴリから導く規則。
//!
//! 4 つのヘッダ（`SecurityLevel`・`SecurityOrigin`・`Sender`・`SenderType`）の文字列への写しと、
//! 応答の `SecurityLevel` の生の値の解釈もここに置く。32bit の仲介の型は使わず文字列だけを返す
//! （入れ物へ詰めるのは呼び手）。

use crate::msg::EventId;
use areka_talk::{ScriptOrigin, SecurityLevel};

/// `Sender` の名前。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SenderName {
    /// areka 自身（`"areka"`）。
    Areka,
    /// MCP の入口（`"MCP"`）。
    Mcp,
}

impl SenderName {
    fn wire(self) -> &'static str {
        match self {
            SenderName::Areka => "areka",
            SenderName::Mcp => "MCP",
        }
    }
}

/// 要求の出どころ（送り手の名前と出どころの組）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RequestOrigin {
    pub sender: SenderName,
    pub origin: ScriptOrigin,
}

/// 4 つのヘッダの文字列。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginWire {
    pub security_level: &'static str,
    pub security_origin: &'static str,
    pub sender: &'static str,
    pub sender_type: String,
}

impl RequestOrigin {
    /// 外からの台本（MCP の `sakurascript`）の `OnTranslate` の出どころ（要件 2.4）。
    pub const MCP_SEND_TRANSLATE: RequestOrigin = RequestOrigin {
        sender: SenderName::Mcp,
        origin: ScriptOrigin::MCP_SEND,
    };

    /// 送るイベントの要求の出どころ（要件 2.2・2.3・2.4〜2.6）。
    pub fn of_event(id: &EventId) -> RequestOrigin {
        // 既定の腕を置かない（カテゴリが増えたらコンパイラに求めさせる）。
        match id {
            EventId::Static(_) | EventId::Choice(_) => RequestOrigin {
                sender: SenderName::Areka,
                origin: ScriptOrigin::GHOST,
            },
            EventId::Sstp(_) => RequestOrigin {
                sender: SenderName::Mcp,
                origin: ScriptOrigin::MCP_NOTIFY,
            },
            EventId::Translate(origin) => *origin,
            // `\![get,property]` の返しのイベント（shell-scale-linkage 要件 1.3）。
            EventId::Property(_) => RequestOrigin {
                sender: SenderName::Areka,
                origin: ScriptOrigin::PROPERTY,
            },
        }
    }

    /// そのイベントへの応答の台本の基本の出どころ（要件 1.1・1.3・1.7）。
    pub fn base_of(id: &EventId) -> ScriptOrigin {
        Self::of_event(id).origin
    }

    /// SHIORI の応答の台本の `OnTranslate` の出どころ: 送り手は areka、出どころは元のイベントの
    /// 応答の台本の基本の出どころ（要件 2.5・2.6）。
    pub fn for_translated(source: &EventId) -> RequestOrigin {
        RequestOrigin {
            sender: SenderName::Areka,
            origin: Self::base_of(source),
        }
    }

    /// 4 つのヘッダの文字列（`SecurityOrigin` は SSTP の受信が来るまで常に `"null"`）。
    pub fn wire(&self) -> OriginWire {
        OriginWire {
            security_level: self.origin.level.wire(),
            security_origin: "null",
            sender: self.sender.wire(),
            sender_type: self.origin.sender_type.render(),
        }
    }
}

/// 応答の `SecurityLevel` の生の値を読む（要件 3.3・3.4）。
///
/// 無い → `Local`（下げない）／`local`・`external`（大小無視）→ その値／それ以外 → イベントの ID と
/// 値を warn に残して `Local`。
pub fn read_response_level(raw: Option<&str>, id: &str) -> SecurityLevel {
    let Some(raw) = raw else {
        return SecurityLevel::Local;
    };
    SecurityLevel::parse(raw).unwrap_or_else(|| {
        tracing::warn!(
            target: "kanade",
            event = "response_security_level_unreadable",
            id = id,
            value = raw,
            "応答の SecurityLevel が読めない——無いものとして local で扱う"
        );
        SecurityLevel::Local
    })
}

#[cfg(test)]
#[path = "request_origin_tests.rs"]
mod tests;

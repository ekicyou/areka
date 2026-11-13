//! strict の記録の areka の判定（spec: mcp-strict-errors 要件 2.7）。
//!
//! 判定は自分で持たず、`check_script` と同じ物差し（`\!` の受け取り手の台帳
//! [`ConsumerLedger::consumer_of`]・文字の層の `\f` の分類 [`classify_font_tag`]）の答えを写すだけにする。
//! 本番の 3 つの構築（窓の無いゴースト・窓の在るゴーストの実行系・spine）が [`production`] を入れる。

use std::sync::Arc;

use areka_emo_text::look::{FontClass, classify_font_tag};
use areka_sakura::strict::{DetectionPoint, StrictJudges};

use super::consumer_ledger::ConsumerLedger;

/// `\!` の台帳と `\f` の分類を引く判定。台帳は起動の時に 1 度だけ組む。
pub(crate) struct ArekaStrictJudges {
    ledger: ConsumerLedger,
}

impl ArekaStrictJudges {
    /// 正準の台帳（`check_script` と同じ表）で判定を作る。
    pub(crate) fn canonical() -> Self {
        Self {
            ledger: ConsumerLedger::canonical(),
        }
    }
}

impl StrictJudges for ArekaStrictJudges {
    fn command_handled(&self, name: &str, first: Option<&str>) -> bool {
        self.ledger.consumer_of(name, first).is_some()
    }

    fn font(&self, args: &[String]) -> Option<DetectionPoint> {
        Some(match classify_font_tag(args)? {
            FontClass::Ignored => DetectionPoint::FontIgnored,
            FontClass::UnknownKey => DetectionPoint::FontUnknownKey,
        })
    }
}

/// 本番の構築が `GhostBootOptions.strict_judges` へ入れる判定（無いと strict の記録が無言で 0 件になる）。
pub(crate) fn production() -> Option<Arc<dyn StrictJudges>> {
    Some(Arc::new(ArekaStrictJudges::canonical()))
}

#[cfg(test)]
#[path = "strict_judges_tests.rs"]
mod strict_judges_tests;

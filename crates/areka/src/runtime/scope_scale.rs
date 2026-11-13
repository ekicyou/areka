//! スコープの倍率の表と倍率の文字（spec shell-scale-linkage 要件 3・4）。
//!
//! 毎フレームの相が書き、読み手（`ghost_session_scope_scale`）と倍率の通知（`frame::scale_notice`）が引く。
//! `ghost_session_scope_scale.rs` から中身を変えずに下ろした（runtime-module-untangle 3.4）。

use std::collections::BTreeMap;
use std::sync::{Arc, RwLock};

use areka_emo_present::{AppliedScale, ScaleRatio};

use crate::runtime::target_map::TargetWindow;

/// 書き手（毎フレームの相）と読み手（どのスレッドからでも）が共有する表。
pub(crate) type SharedScopeScaleTable = Arc<RwLock<ScopeScaleTable>>;

/// スコープ 1 つの窓ごとの倍率（行が無い＝表示がまだ一度も成立していない）。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct ScopeScales {
    pub character: Option<AppliedScale>,
    pub balloon: Option<AppliedScale>,
}

/// スコープ → 窓ごとの倍率の表。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct ScopeScaleTable(BTreeMap<u32, ScopeScales>);

impl ScopeScaleTable {
    /// スコープ `scope` の窓 `window` の倍率を書く。
    pub(crate) fn set(&mut self, scope: u32, window: TargetWindow, value: AppliedScale) {
        let row = self.0.entry(scope).or_default();
        match window {
            TargetWindow::Shell => row.character = Some(value),
            TargetWindow::Balloon => row.balloon = Some(value),
        }
    }

    /// スコープ `scope` の窓 `window` の倍率。行が無いか、その窓がまだ無ければ `None`。
    pub(crate) fn get(&self, scope: u32, window: TargetWindow) -> Option<AppliedScale> {
        let row = self.0.get(&scope)?;
        match window {
            TargetWindow::Shell => row.character,
            TargetWindow::Balloon => row.balloon,
        }
    }
}

/// 倍率の百分率の文字（D6・整数だけで計算する）。
///
/// 割り切れれば整数、割り切れなければ小数第 2 位まで（第 3 位で四捨五入）で、
/// 末尾の `0` は落とすが小数点の後に 1 桁は残す。符号・空白・前ゼロは付けない（要件 3.6・3.7）。
pub(crate) fn percent_text(ratio: ScaleRatio) -> String {
    let (num, den) = ratio.parts();
    let (num, den) = (u128::from(num), u128::from(den));
    if (100 * num).is_multiple_of(den) {
        return (100 * num / den).to_string();
    }
    // 百分率の 100 分の 1 の単位で、半分を足して切り捨てる＝第 3 位で四捨五入。
    let h = (2 * 10000 * num + den) / (2 * den);
    let text = format!("{}.{:02}", h / 100, h % 100);
    match text.strip_suffix('0') {
        Some(trimmed) if !trimmed.ends_with('.') => trimmed.to_string(),
        _ => text,
    }
}

/// `scaling` の値＝横と縦の百分率（areka の倍率は縦横同じ）を `,` でつなぐ。
pub(crate) fn scaling_text(ratio: ScaleRatio) -> String {
    let p = percent_text(ratio);
    format!("{p},{p}")
}

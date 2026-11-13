//! スコープの倍率と DPI の読み手（spec shell-scale-linkage 要件 3・4）。
//!
//! 表示層が表示成立点で決めた（倍率・その倍率を出した DPI）を、スコープごとの表に持ち、
//! `currentghost.scope(ID).scaling` などの 4 つの名前に答える。値はメモリの表だけで、
//! 写しにも保存にも載せない（要件 4.6）。

use std::sync::Arc;
#[cfg(test)]
use std::sync::RwLock;

#[cfg(test)]
use areka_emo_present::{AppliedScale, ScaleRatio};
use areka_sylphya::{PropPath, PullShape, PullSource, SylphyaReader};

use crate::ghost_session::ghost_session_scope_rect::{ScopeSelector, scope_selector};
#[cfg(test)]
pub(crate) use crate::runtime::scope_scale::{ScopeScaleTable, percent_text};
pub(crate) use crate::runtime::scope_scale::{SharedScopeScaleTable, scaling_text};
use crate::runtime::target_map::TargetWindow;

/// 読み手が答える量。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ScaleQuantity {
    /// 百分率の「横,縦」。
    Scaling,
    /// その倍率を出した DPI（10 進の整数）。
    Dpi,
}

/// 倍率と DPI の読み手（4 つの形で 1 つずつ登録する）。
pub(crate) struct ScopeScaleSource {
    window: TargetWindow,
    quantity: ScaleQuantity,
    table: SharedScopeScaleTable,
}

impl ScopeScaleSource {
    pub(crate) fn new(
        window: TargetWindow,
        quantity: ScaleQuantity,
        table: SharedScopeScaleTable,
    ) -> Self {
        Self {
            window,
            quantity,
            table,
        }
    }
}

impl PullSource for ScopeScaleSource {
    fn read(&self, path: &PropPath) -> Option<String> {
        // 負の番号・書式の誤り・窓の無い番号は値なし（要件 3.9）。
        let ScopeSelector::Index(scope) = scope_selector(path)? else {
            return None;
        };
        let scope = u32::try_from(scope).ok()?;
        // 鍵は値を写す間だけ持つ。poison でも中身で続ける（要件 4.3）。
        let applied = self
            .table
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .get(scope, self.window)?;
        match self.quantity {
            ScaleQuantity::Scaling => Some(scaling_text(applied.ratio)),
            ScaleQuantity::Dpi => applied.dpi_x.map(|dpi| dpi.to_string()),
        }
    }
}

/// 4 つの形を実行系の読み口へ登録する（`rewire_runtime_consumers` から・同期）。
/// `None`（毎フレームの配線が無い起動）は空の表で登録する＝どの名前も値なし。
pub(crate) fn wire(reader: &SylphyaReader, table: Option<SharedScopeScaleTable>) {
    let table = table.unwrap_or_else(|| {
        tracing::debug!(
            event = "scope_scale_no_wiring",
            "毎フレームの配線が無いので空の倍率の表で登録した"
        );
        SharedScopeScaleTable::default()
    });
    for (window, quantity, pattern) in [
        // ukadoc: https://ssp.shillest.net/ukadoc/manual/list_propertysystem.html#currentghost.scope_28ID_29.scaling:1
        (
            TargetWindow::Shell,
            ScaleQuantity::Scaling,
            "currentghost.scope(*).scaling",
        ),
        // ukadoc: https://ssp.shillest.net/ukadoc/manual/list_propertysystem.html#currentghost.balloon.scope_28ID_29.scaling:1
        (
            TargetWindow::Balloon,
            ScaleQuantity::Scaling,
            "currentghost.balloon.scope(*).scaling",
        ),
        // ukadoc: https://ssp.shillest.net/ukadoc/manual/list_propertysystem.html#currentghost.scope_28ID_29.currentmonitor.dpi:1
        (
            TargetWindow::Shell,
            ScaleQuantity::Dpi,
            "currentghost.scope(*).currentmonitor.dpi",
        ),
        // areka 独自（ukadoc に無い名前）: doc/COMPAT_ARCHITECTURE.md 8 節を見よ。
        (
            TargetWindow::Balloon,
            ScaleQuantity::Dpi,
            "currentghost.balloon.scope(*).currentmonitor.dpi",
        ),
    ] {
        let shape = PullShape::parse(pattern).expect("固定の形は読める");
        let source = ScopeScaleSource::new(window, quantity, table.clone());
        reader.register_pull(shape, Arc::new(source));
    }
}

#[cfg(test)]
#[path = "ghost_session_scope_scale_tests.rs"]
mod tests;

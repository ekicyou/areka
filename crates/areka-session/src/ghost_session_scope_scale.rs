//! スコープの倍率と DPI の読み手。
//!
//! 表示層が表示成立点で決めた（倍率・その倍率を出した DPI）を持つ取っ手（ゴーストの層の速い住み場所）を、
//! `currentghost.scope(ID).scaling` などの 4 つの名前に答えさせる。値は取っ手だけが持ち、保存に載せない。

use std::sync::Arc;

use areka_sylphya::fast::FastHome;
use areka_sylphya::{PersistScope, PropPath, PullShape, SylphyaReader};

use crate::ghost_session::ghost_session_scope_rect::{ScopeSelector, scope_selector};
#[cfg(test)]
pub(crate) use areka_runtime::scope_scale::percent_text;
pub(crate) use areka_runtime::scope_scale::{ScopeScaleCells, scaling_text};
use areka_runtime::target_map::TargetWindow;

/// 読み手が答える量。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ScaleQuantity {
    /// 百分率の「横,縦」。
    Scaling,
    /// その倍率を出した DPI（10 進の整数）。
    Dpi,
}

/// 倍率と DPI の速い住み場所（4 つの形で 1 つずつ登録する）。
pub(crate) struct ScopeScaleSource {
    window: TargetWindow,
    quantity: ScaleQuantity,
    table: ScopeScaleCells,
}

impl ScopeScaleSource {
    pub(crate) fn new(
        window: TargetWindow,
        quantity: ScaleQuantity,
        table: ScopeScaleCells,
    ) -> Self {
        Self {
            window,
            quantity,
            table,
        }
    }
}

impl FastHome for ScopeScaleSource {
    fn read_text(&self, path: &PropPath) -> Option<String> {
        // 負の番号・書式の誤り・窓の無い番号は値なし。
        let ScopeSelector::Index(scope) = scope_selector(path)? else {
            return None;
        };
        let scope = u32::try_from(scope).ok()?;
        match self.quantity {
            ScaleQuantity::Scaling => Some(scaling_text(self.table.ratio(scope, self.window)?)),
            ScaleQuantity::Dpi => self
                .table
                .dpi(scope, self.window)
                .map(|dpi| dpi.to_string()),
        }
    }

    // 鍵の表が名前での書きを先に断るので届かない。
    fn write_text(&self, _path: &PropPath, _value: &str) -> Result<(), String> {
        Err("scale and DPI are not writable by name".to_owned())
    }

    // 4 つの形は同じ層に揃って登録され、層の付け替えで揃って捨てられるので、どの形からでも全部を捨てる。
    fn clear(&self) {
        self.table.clear();
    }
}

/// 4 つの形を実行系の読み口へ登録する（`rewire_runtime_consumers` から・同期）。
/// `None`（毎フレームの配線が無い起動）は空の取っ手で登録する＝どの名前も値なし。
pub(crate) fn wire(reader: &SylphyaReader, table: Option<ScopeScaleCells>) {
    let table = table.unwrap_or_else(|| {
        tracing::debug!(
            event = "scope_scale_no_wiring",
            "毎フレームの配線が無いので空の倍率の表で登録した"
        );
        ScopeScaleCells::default()
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
        reader.register_fast(shape, PersistScope::Ghost, Arc::new(source));
    }
}

#[cfg(any(test, feature = "test-support"))]
#[path = "ghost_session_scope_scale_test_support.rs"]
pub mod test_support;

#[cfg(test)]
#[path = "ghost_session_scope_scale_tests.rs"]
mod tests;

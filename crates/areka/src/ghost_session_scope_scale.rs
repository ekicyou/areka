//! スコープの倍率と DPI の読み手（spec shell-scale-linkage 要件 3・4）。
//!
//! 表示層が表示成立点で決めた（倍率・その倍率を出した DPI）を、スコープごとの表に持ち、
//! `currentghost.scope(ID).scaling` などの 4 つの名前に答える。値はメモリの表だけで、
//! 写しにも保存にも載せない（要件 4.6）。

use std::collections::BTreeMap;
use std::sync::{Arc, RwLock};

use areka_emo_present::{AppliedScale, ScaleRatio};
use areka_sylphya::{PropPath, PullShape, PullSource, SylphyaReader};

use crate::emo2_boot::target_map::TargetWindow;
use crate::ghost_session::ghost_session_scope_rect::{ScopeSelector, scope_selector};

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

#[cfg(test)]
#[path = "ghost_session_scope_scale_tests.rs"]
mod tests;

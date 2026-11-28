//! スコープの倍率の表と倍率の文字。
//!
//! 毎フレームの相が書き、読み手（`ghost_session_scope_scale`）と倍率の通知（`frame::scale_notice`）が引く。

use areka_emo_present::{AppliedScale, ScaleRatio};
use areka_sylphya::fast::{FastTable, FastValue};

use crate::target_map::TargetWindow;

/// 倍率を 64 ビットへ写す（上位 32 ビットが分子・下位が分母。分子は 1 以上なので 0 にならない）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RatioBits(pub ScaleRatio);

impl FastValue for RatioBits {
    fn to_bits(self) -> u64 {
        let (num, den) = self.0.parts();
        (u64::from(num) << 32) | u64::from(den)
    }

    fn from_bits(bits: u64) -> Self {
        let (num, den) = ((bits >> 32) as u32, bits as u32);
        Self(ScaleRatio::new(num, den).expect("表には to_bits の値だけが入る"))
    }
}

/// DPI を 64 ビットへ写す（1 を足して 0 を避ける）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DpiBits(pub u16);

impl FastValue for DpiBits {
    fn to_bits(self) -> u64 {
        u64::from(self.0) + 1
    }

    fn from_bits(bits: u64) -> Self {
        Self(u16::try_from(bits - 1).expect("表には to_bits の値だけが入る"))
    }
}

/// スコープの倍率と DPI の取っ手（キャラとバルーンの倍率と DPI の 4 つの速い表）。複製は同じ表を指す。
///
/// 書き手は毎フレームの相、読み手はどのスレッドからでも（ロックなし）。行が無い＝表示がまだ一度も成立していない。
#[derive(Clone, Default)]
pub struct ScopeScaleCells {
    character_ratio: FastTable<RatioBits>,
    character_dpi: FastTable<DpiBits>,
    balloon_ratio: FastTable<RatioBits>,
    balloon_dpi: FastTable<DpiBits>,
}

impl ScopeScaleCells {
    fn tables(&self, window: TargetWindow) -> (&FastTable<RatioBits>, &FastTable<DpiBits>) {
        match window {
            TargetWindow::Shell => (&self.character_ratio, &self.character_dpi),
            TargetWindow::Balloon => (&self.balloon_ratio, &self.balloon_dpi),
        }
    }

    /// スコープ `scope` の窓 `window` の倍率と DPI を書く。
    pub fn set(&self, scope: u32, window: TargetWindow, value: AppliedScale) {
        let (ratio, dpi) = self.tables(window);
        ratio.set(scope, Some(RatioBits(value.ratio)));
        dpi.set(scope, value.dpi_x.map(DpiBits));
    }

    /// スコープ `scope` の窓 `window` の倍率と DPI。その窓がまだ無ければ `None`。
    pub fn get(&self, scope: u32, window: TargetWindow) -> Option<AppliedScale> {
        Some(AppliedScale {
            ratio: self.ratio(scope, window)?,
            dpi_x: self.dpi(scope, window),
        })
    }

    /// スコープ `scope` の窓 `window` の倍率だけ（1 つの表だけを読む）。
    pub fn ratio(&self, scope: u32, window: TargetWindow) -> Option<ScaleRatio> {
        self.tables(window).0.get(scope).map(|b| b.0)
    }

    /// スコープ `scope` の窓 `window` の DPI だけ（1 つの表だけを読む）。
    pub fn dpi(&self, scope: u32, window: TargetWindow) -> Option<u16> {
        self.tables(window).1.get(scope).map(|b| b.0)
    }

    /// 4 つの表の値を全部捨てる。
    pub fn clear(&self) {
        self.character_ratio.clear();
        self.character_dpi.clear();
        self.balloon_ratio.clear();
        self.balloon_dpi.clear();
    }
}

/// 倍率の百分率の文字（整数だけで計算する）。
///
/// 割り切れれば整数、割り切れなければ小数第 2 位まで（第 3 位で四捨五入）で、
/// 末尾の `0` は落とすが小数点の後に 1 桁は残す。符号・空白・前ゼロは付けない。
pub fn percent_text(ratio: ScaleRatio) -> String {
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
pub fn scaling_text(ratio: ScaleRatio) -> String {
    let p = percent_text(ratio);
    format!("{p},{p}")
}

#[cfg(test)]
#[path = "scope_scale_tests.rs"]
mod tests;

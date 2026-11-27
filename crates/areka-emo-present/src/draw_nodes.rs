//! 計画を描画の木の節へ写す部品と、重ね方から D2D の合成モードへの写し。

use areka_emo_compose::Mix;
use windows::Win32::Graphics::Direct2D::Common::{
    D2D1_COMPOSITE_MODE, D2D1_COMPOSITE_MODE_DESTINATION_IN, D2D1_COMPOSITE_MODE_DESTINATION_OVER,
    D2D1_COMPOSITE_MODE_SOURCE_ATOP, D2D1_COMPOSITE_MODE_SOURCE_COPY,
    D2D1_COMPOSITE_MODE_SOURCE_OVER,
};

/// 重ね方 → D2D の合成モード（写しはこの 1 か所）。
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "節へ写す口から呼ばれるまで本番の呼び手が無い")
)]
pub(crate) fn mix_mode_of(mix: Mix) -> D2D1_COMPOSITE_MODE {
    match mix {
        Mix::SourceOver => D2D1_COMPOSITE_MODE_SOURCE_OVER,
        Mix::SourceAtop => D2D1_COMPOSITE_MODE_SOURCE_ATOP,
        Mix::DestinationOver => D2D1_COMPOSITE_MODE_DESTINATION_OVER,
        Mix::Copy => D2D1_COMPOSITE_MODE_SOURCE_COPY,
        Mix::DestinationIn => D2D1_COMPOSITE_MODE_DESTINATION_IN,
    }
}

#[cfg(test)]
#[path = "draw_nodes_tests.rs"]
mod tests;

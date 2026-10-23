//! 面 1 つを時点 T まで進めて組み、シェル全体の下地に置いて PNG のファイルにする（要件 4.1・4.3〜4.9・6.1〜6.8）。

use areka_emo_atlas::AtlasTable;
use areka_emo_compose::{BindSet, Composer, EmoWorld, ShellBounds};
use areka_seriko::{AnimationTable, settle_surface};

use crate::png_image;

/// シェル全体で 1 つの下地（[`ShellBounds`] から作る）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Canvas {
    /// 土台の原点から見た下地の左端（0 以下）。
    pub(crate) left: i32,
    /// 土台の原点から見た下地の上端（0 以下）。
    pub(crate) top: i32,
    pub(crate) width: u32,
    pub(crate) height: u32,
}

impl Canvas {
    /// 外形の大きさ `(right - left) × (bottom - top)` の下地（要件 4.3）。
    #[allow(dead_code)] // 呼び手はオフラインの書き出しの走り（タスク 7.8）
    pub(crate) fn from_bounds(bounds: ShellBounds) -> Self {
        let span =
            |lo: i32, hi: i32| (i64::from(hi) - i64::from(lo)).clamp(0, u32::MAX as i64) as u32;
        Canvas {
            left: bounds.left,
            top: bounds.top,
            width: span(bounds.left, bounds.right),
            height: span(bounds.top, bounds.bottom),
        }
    }
}

/// 面 1 つを描けなかったわけ。
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum RenderError {
    /// 合成が `Err` を返した（記録は合成が出している）。
    Compose,
    /// 組んだ絵が下地からはみ出した（作りの誤り）。
    CanvasOverflow,
}

/// 面 1 つを時点 T まで画面と同じ道で進め、画面と同じ入力の形で合成し、下地の `(-left, -top)` に
/// 置いた乗算済み BGRA を返す（テストが画素を見る口・要件 4.1・4.3〜4.5・4.7・4.9）。
#[allow(clippy::too_many_arguments)] // design の口の形（走りが持つ部品をそのまま渡す）
#[allow(dead_code)] // 呼び手はオフラインの書き出しの走り（タスク 7.8）
pub(crate) fn render_surface(
    composer: &mut Composer,
    world: &EmoWorld,
    atlas: &AtlasTable,
    table: &AnimationTable,
    binds: &BindSet,
    scope: u32,
    surface_id: u32,
    canvas: &Canvas,
) -> Result<Vec<u8>, RenderError> {
    let pattern = settle_surface(table, scope, binds, surface_id);
    // 画面の面の表示と同じ入力（world・atlas・面・着せ替え・面の状態）で組む。
    let pic = match composer.compose(world, atlas, surface_id, binds, &pattern) {
        Ok(pic) => pic,
        Err(_) => {
            // 合成が error を出しているので、ここは info だけ（1 つの誤りは 1 行・要件 4.8）。
            tracing::info!(
                event = "offline_dump_surface_skipped",
                surface = surface_id,
                "[offline_dump] 面を組めなかったので、この面を飛ばします"
            );
            return Err(RenderError::Compose);
        }
    };
    let (x0, y0) = (
        (-i64::from(canvas.left)) as usize,
        (-i64::from(canvas.top)) as usize,
    );
    let (w, h) = (pic.width() as usize, pic.height() as usize);
    let (cw, ch) = (canvas.width as usize, canvas.height as usize);
    if x0 + w > cw || y0 + h > ch {
        tracing::error!(
            event = "offline_dump_canvas_overflow",
            surface = surface_id,
            width = pic.width(),
            height = pic.height(),
            canvas_width = canvas.width,
            canvas_height = canvas.height,
            "[offline_dump] 組んだ絵がシェル全体の下地からはみ出したので、この面を飛ばします"
        );
        return Err(RenderError::CanvasOverflow);
    }
    let mut out = vec![0u8; cw * ch * 4];
    let stride = pic.stride() as usize;
    for (row, line) in pic.bytes().chunks_exact(stride).enumerate() {
        let at = ((y0 + row) * cw + x0) * 4;
        out[at..at + w * 4].copy_from_slice(&line[..w * 4]);
    }
    Ok(out)
}

/// [`render_surface`] の絵を MCP と同じ `png_image::encode_png` で PNG のバイト列にする（要件 4.6）。
#[allow(clippy::too_many_arguments)] // render_surface と同じ口
#[allow(dead_code)] // 呼び手は面の PNG のファイルの書き出し（タスク 7.6）
pub(crate) fn render_png(
    composer: &mut Composer,
    world: &EmoWorld,
    atlas: &AtlasTable,
    table: &AnimationTable,
    binds: &BindSet,
    scope: u32,
    surface_id: u32,
    canvas: &Canvas,
) -> Result<Vec<u8>, RenderError> {
    let bgra = render_surface(
        composer, world, atlas, table, binds, scope, surface_id, canvas,
    )?;
    Ok(png_image::encode_png(&bgra, canvas.width, canvas.height))
}

#[cfg(test)]
#[path = "render_tests.rs"]
mod render_tests;

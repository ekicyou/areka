//! 面 1 つを時点 T まで進めて組み、シェル全体の下地に置いて PNG のファイルにする（要件 4.1・4.3〜4.9・6.1〜6.8）。

use std::path::{Path, PathBuf};

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

/// 頭の文字列の既定（`--dump-output-prefix` が無いとき・要件 6.1）。
#[allow(dead_code)] // 呼び手はオフラインの書き出しの走り（タスク 7.8）
pub(crate) const DEFAULT_PREFIX: &str = "surface";

/// 面 1 つを描けなかったわけ。
#[derive(Debug)]
pub(crate) enum RenderError {
    /// 合成が `Err` を返した（記録は合成が出している）。
    Compose,
    /// 組んだ絵が下地からはみ出した（作りの誤り）。
    CanvasOverflow,
    /// PNG のファイルを書けなかった（記録は [`write_surface`] が出している）。
    Write(#[allow(dead_code)] /* 読み手は走り（タスク 7.8） */ std::io::Error),
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
#[allow(dead_code)] // 呼び手は write_surface 経由でオフラインの書き出しの走り（タスク 7.8）
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

/// 面の PNG のファイルの道 `<出力先>\<頭の文字列><番号>.png`（スコープは入れない・要件 6.1・6.2・6.7）。
/// 頭の文字列の `\`・`/` は出力先からの相対の道になる（6.8）。
#[allow(dead_code)] // 呼び手はオフラインの書き出しの走り（タスク 7.8）
pub(crate) fn png_path(output_dir: &Path, prefix: &str, surface_id: u32) -> PathBuf {
    output_dir.join(format!("{prefix}{surface_id}.png"))
}

/// 面 1 つを描いて [`png_path`] へ書く。書く直前に出力先だけを作り、頭の文字列の下のフォルダは
/// 作らない。同じ名前は置き換える。書けなければ error 1 行（要件 6.3〜6.6・6.8）。
#[allow(clippy::too_many_arguments)] // render_surface の口に出力先と頭の文字列を足した形
#[allow(dead_code)] // 呼び手はオフラインの書き出しの走り（タスク 7.8）
pub(crate) fn write_surface(
    composer: &mut Composer,
    world: &EmoWorld,
    atlas: &AtlasTable,
    table: &AnimationTable,
    binds: &BindSet,
    scope: u32,
    surface_id: u32,
    canvas: &Canvas,
    output_dir: &Path,
    prefix: &str,
) -> Result<PathBuf, RenderError> {
    let png = render_png(
        composer, world, atlas, table, binds, scope, surface_id, canvas,
    )?;
    let path = png_path(output_dir, prefix, surface_id);
    let written = std::fs::create_dir_all(output_dir).and_then(|()| std::fs::write(&path, png));
    if let Err(err) = written {
        tracing::error!(
            event = "offline_dump_png_write_failed",
            surface = surface_id,
            path = %path.display(),
            error = %err,
            "[offline_dump] 面の PNG を書けなかったので、この面を飛ばします"
        );
        return Err(RenderError::Write(err));
    }
    Ok(path)
}

#[cfg(test)]
#[path = "render_tests.rs"]
mod render_tests;

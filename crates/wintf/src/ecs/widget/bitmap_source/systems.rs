//! BitmapSource システム
//!
//! 非同期画像読み込み、パス解決、D2D描画システム。

use super::resource::{BitmapSourceGraphics, BitmapSourceResource};
use super::task_pool::WintfTaskPool;
use crate::com::d2d::D2D1DeviceContextExt;
use crate::ecs::graphics::{GraphicsCommandList, GraphicsCore};
use crate::ecs::layout::Arrangement;
use crate::ecs::widget::record::{record_command_list, store_command_list};
use bevy_ecs::name::Name;
use bevy_ecs::prelude::*;
use std::path::{Path, PathBuf};
use tracing::{debug, trace, warn};
use windows::Win32::Graphics::Direct2D::Common::D2D1_PIXEL_FORMAT;
use windows::Win32::Graphics::Direct2D::{
    D2D1_BITMAP_OPTIONS_NONE, D2D1_BITMAP_PROPERTIES1, D2D1_INTERPOLATION_MODE_HIGH_QUALITY_CUBIC,
    ID2D1DeviceContext,
};
use windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM;
use windows::Win32::Graphics::Imaging::D2D::IWICImagingFactory2;
use windows::Win32::Graphics::Imaging::{IWICBitmapSource, WICBitmapCacheOnLoad};
use windows::core::{Interface, Result};

// ============================================================
// パス解決
// ============================================================

/// パス解決: 実行ファイル基準
///
/// wintfシステム全体の思想として、相対パスは実行ファイルの
/// ディレクトリを基準とする。カレントディレクトリは実行時に
/// 変動する可能性があるため、使用しない。
pub fn resolve_path(path: &str) -> std::io::Result<PathBuf> {
    let path = Path::new(path);

    if path.is_absolute() {
        Ok(path.to_path_buf())
    } else {
        // 実行ファイルのディレクトリを基準に解決
        let exe_path = std::env::current_exe()?;
        let exe_dir = exe_path.parent().ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::NotFound, "exe directory not found")
        })?;
        Ok(exe_dir.join(path))
    }
}

// ============================================================
// WIC画像読み込み
// ============================================================

// `load_bitmap_source` は ECS 非依存の COM 層（`crate::com::wic`）へ移設した
// （design Decision D2 / Critical Issue 1）。パス互換のため re-export する。
// 戻り値は `(IWICBitmapSource, has_alpha)` の tuple に拡張されている。
pub use crate::com::wic::load_bitmap_source;

/// 画像を読み、裏の作業の中で 1 回だけ解読してメモリの画像にする。
///
/// `load_bitmap_source` が返す形式の変換器は読むたびに解読するので、
/// `WICBitmapCacheOnLoad` でメモリへ写す。UI スレッドの `copy_pixels`（αのマスク）と
/// `CreateBitmapFromWicBitmap`（描画）はこの写しを読むだけになる。
pub(crate) fn load_decoded_bitmap(
    factory: &IWICImagingFactory2,
    path: &Path,
) -> Result<IWICBitmapSource> {
    let (converter, _has_alpha) = load_bitmap_source(factory, path)?;
    let bitmap = unsafe { factory.CreateBitmapFromSource(&converter, WICBitmapCacheOnLoad)? };
    bitmap.cast()
}

// ============================================================
// 描画システム
// ============================================================

/// BitmapSourceResourceからD2D Bitmapを生成し、GraphicsCommandListに描画コマンドを出力
///
/// - BitmapSourceGraphics.is_valid() == false か画像の資源が変わった場合、D2D Bitmapを生成
/// - GraphicsCommandListに描画コマンドを出力（OFFSET(0,0)から描画）
pub fn draw_bitmap_sources(
    mut commands: Commands,
    mut query: Query<
        (
            Entity,
            Ref<BitmapSourceResource>,
            &mut BitmapSourceGraphics,
            &Arrangement,
            Option<&GraphicsCommandList>,
            Option<&Name>,
        ),
        // 命令の列が無い（立ち直りで外された）ときも描き直す
        Or<(
            Changed<BitmapSourceResource>,
            Changed<Arrangement>,
            Without<GraphicsCommandList>,
        )>,
    >,
    graphics_core: Option<Res<GraphicsCore>>,
) {
    if query.is_empty() {
        return;
    }
    let Some(graphics_core) = graphics_core else {
        warn!("GraphicsCore not available, skipping draw_bitmap_sources");
        return;
    };
    // グローバル共有DeviceContextを取得（無ければ部品ごとでなく 1 回だけ記録）
    let Some(dc) = graphics_core.device_context() else {
        warn!("DeviceContext not available, skipping draw_bitmap_sources");
        return;
    };

    for (entity, resource, mut graphics, arrangement, cmd_list_opt, name) in query.iter_mut() {
        trace!(
            ?entity,
            width = arrangement.size.width,
            height = arrangement.size.height,
            "Drawing bitmap source"
        );

        // D2D Bitmap生成（未生成・無効化・画像の入れ替えの場合）。作れない失敗も記録と同じ道を通す
        let bitmap = if graphics.is_valid() && !resource.is_changed() {
            Ok(())
        } else {
            create_d2d_bitmap(dc, resource.source()).map(|bitmap| graphics.set_bitmap(bitmap))
        };
        // Arrangement.size(DIP単位)の矩形に合わせてスケール描画
        // draw_image(ネイティブサイズ描画)ではなくDrawBitmapで宛先矩形を指定することで
        // ビットマップのネイティブ解像度やDPIに関係なく正確な表示サイズになる
        let dest_rect = crate::ecs::Rect {
            left: 0.0,
            top: 0.0,
            right: arrangement.size.width,
            bottom: arrangement.size.height,
        }
        .into();
        let recorded = bitmap.and_then(|()| {
            record_command_list(dc, |dc| {
                if let Some(bitmap) = graphics.bitmap() {
                    dc.draw_bitmap(
                        bitmap,
                        Some(&dest_rect),
                        1.0,
                        D2D1_INTERPOLATION_MODE_HIGH_QUALITY_CUBIC,
                        None,
                        None,
                    );
                }
                Ok(())
            })
        });
        store_command_list(
            &mut commands,
            entity,
            name,
            cmd_list_opt.is_some(),
            recorded,
            "bitmap source",
        );
    }
}

/// WIC BitmapSourceからD2D Bitmapを作成
fn create_d2d_bitmap(
    dc: &ID2D1DeviceContext,
    source: &IWICBitmapSource,
) -> Result<windows::Win32::Graphics::Direct2D::ID2D1Bitmap1> {
    use std::mem::ManuallyDrop;
    use windows::Win32::Graphics::Direct2D::ID2D1ColorContext;

    // DCの実際のDPIを取得してビットマップに設定する。
    // 96.0固定にするとDCのDPIスケール(例:125%=120dpi)とズレが生じ、
    // DrawBitmapで宛先矩形を指定しても内部スケール計算が狂うため、
    // 常にDCのDPIに合わせる。
    let (dpi_x, dpi_y) = unsafe {
        let mut x = 0.0f32;
        let mut y = 0.0f32;
        dc.GetDpi(&mut x, &mut y);
        (x, y)
    };

    let props = D2D1_BITMAP_PROPERTIES1 {
        pixelFormat: D2D1_PIXEL_FORMAT {
            format: DXGI_FORMAT_B8G8R8A8_UNORM,
            alphaMode: windows::Win32::Graphics::Direct2D::Common::D2D1_ALPHA_MODE_PREMULTIPLIED,
        },
        dpiX: dpi_x,
        dpiY: dpi_y,
        bitmapOptions: D2D1_BITMAP_OPTIONS_NONE,
        colorContext: ManuallyDrop::new(None::<ID2D1ColorContext>),
    };

    unsafe { dc.CreateBitmapFromWicBitmap(source, Some(&props)) }
}

// ============================================================
// drain_task_pool_commands システム
// ============================================================

/// WintfTaskPoolからコマンドをドレインしてWorldに適用
///
/// Inputスケジュールで実行される。
///
/// # 重要な設計
/// コマンド実行中も`WintfTaskPool`がWorld内に存在する必要がある。
/// なぜなら、コマンド内で`BitmapSource`をspawnした場合、
/// `on_bitmap_source_add`フックが`WintfTaskPool.spawn()`を呼び出すため。
/// リソースが取り除かれていると、画像読み込みタスクが起動されない。
pub fn drain_task_pool_commands(world: &mut World) {
    // まずコマンドをVecに収集（WintfTaskPoolをWorld内に保持したまま）
    let commands: Vec<_> = if let Some(task_pool) = world.get_resource::<WintfTaskPool>() {
        task_pool.drain_commands()
    } else {
        Vec::new()
    };

    // コマンドを実行（WintfTaskPoolはWorld内に存在する）
    for cmd in commands {
        cmd(world);
    }
}

// ============================================================
// αマスク生成システム
// ============================================================

use super::alpha_mask::AlphaMask;
use crate::com::wic::WICBitmapSourceExt;
use crate::ecs::layout::{HitTest, HitTestMode};
use tracing::error;

/// PBGRA32（1 画素 4 バイト）の行の幅と全体のバイト数。全体が u32 に収まらなければ `None`
/// （`IWICBitmapSource::CopyPixels` は全体のバイト数を u32 へ変換し、溢れると panic する）。
pub(crate) fn pbgra32_layout(width: u32, height: u32) -> Option<(u32, usize)> {
    let stride = width.checked_mul(4)?;
    let size = stride.checked_mul(height)?;
    Some((stride, size as usize))
}

/// αマスク生成完了時にBitmapSourceResourceにαマスクを設定するCommand
pub(super) struct SetAlphaMaskCommand {
    pub(super) entity: Entity,
    /// マスクを作った画像の COM の番地
    pub(super) source: usize,
    pub(super) mask: AlphaMask,
}

impl Command for SetAlphaMaskCommand {
    type Out = ();

    fn apply(self, world: &mut World) {
        // 今の資源の画像が、マスクを作った画像と同じときだけ入れる
        let resource = world
            .get_entity_mut(self.entity)
            .ok()
            .and_then(|e| e.into_mut::<BitmapSourceResource>())
            .filter(|r| r.source().as_raw() as usize == self.source);
        match resource {
            // 変化の印は立てない（描く係が D2D の画像をもう一度作らないため）
            Some(mut resource) => resource.bypass_change_detection().set_alpha_mask(self.mask),
            None => debug!(entity = ?self.entity, "Image changed or gone, dropping alpha mask"),
        }
    }
}

/// αマスク生成システム
///
/// BitmapSourceResourceが追加・入れ替えされた時か HitTest が変わった時、HitTestMode::AlphaMaskの場合のみ
/// 非同期でαマスクを生成する。
///
/// # Trigger
/// - `Changed<BitmapSourceResource>` か `Changed<HitTest>`（マスクがまだ無いものだけ）
/// - `HitTestMode::AlphaMask` の場合のみ実行
///
/// # Flow
/// 1. 解読済みのメモリの画像からピクセルデータを写す（UI スレッド・解読はしない）
/// 2. 非同期タスクでAlphaMask::from_pbgra32() でマスク生成
/// 3. Command 経由で BitmapSourceResource.alpha_mask に設定
pub fn generate_alpha_mask_system(
    // 読み込んだ後に当たりを AlphaMask へ替えたときも作る
    query: Query<
        (Entity, &BitmapSourceResource, &HitTest),
        Or<(Changed<BitmapSourceResource>, Changed<HitTest>)>,
    >,
    task_pool: Option<Res<WintfTaskPool>>,
) {
    let Some(task_pool) = task_pool else {
        return;
    };

    for (entity, resource, hit_test) in query.iter() {
        // HitTestMode::AlphaMask 以外はスキップ
        if hit_test.mode != HitTestMode::AlphaMask {
            continue;
        }

        // 既にαマスクが生成済みの場合はスキップ
        if resource.alpha_mask().is_some() {
            continue;
        }

        // UI スレッドで写す。資源は解読済みのメモリの画像なので、ここでは読むだけ
        let source = resource.source();

        let source_addr = source.as_raw() as usize;

        // 画像サイズを取得
        let (width, height) = match source.get_size() {
            Ok(size) => size,
            Err(e) => {
                error!(entity = ?entity, error = ?e, "Failed to get bitmap size for alpha mask");
                continue;
            }
        };

        // ピクセルデータを取得
        let Some((stride, buffer_size)) = pbgra32_layout(width, height) else {
            error!(entity = ?entity, width, height, "Bitmap is too large for alpha mask");
            continue;
        };
        let mut buffer = vec![0u8; buffer_size];

        if let Err(e) = source.copy_pixels(None, stride, &mut buffer) {
            error!(entity = ?entity, error = ?e, "Failed to copy pixels for alpha mask");
            continue;
        }

        // 非同期でαマスクを生成（ピクセルデータはSend可能）
        task_pool.spawn(move |tx| async move {
            // αマスクを生成
            let mask = AlphaMask::from_pbgra32(&buffer, width, height, stride);

            // Commandを送信してBitmapSourceResourceにαマスクを設定
            let cmd: super::task_pool::BoxedCommand = Box::new(move |world: &mut World| {
                SetAlphaMaskCommand {
                    entity,
                    source: source_addr,
                    mask,
                }
                .apply(world);
            });
            let _ = tx.send(cmd);
        });
    }
}

//! 読み戻しの公開の口（要件 5.1〜5.9・6.4・8.3）。使い手が描画面の持ち主の実体と座標で、最後に
//! 合成し終えた入れ物の画素を読む。係を持たず（呼ばれたときだけ読む・5.7）、閾値を持たない（5.8）。

use super::composite::SurfaceComposite;
use super::systems::format_entity_name;
use super::{GraphicsCore, SurfaceGraphics};
use bevy_ecs::name::Name;
use bevy_ecs::prelude::*;
use bevy_ecs::system::SystemParam;
use tracing::error;
use windows::Win32::Graphics::Direct2D::ID2D1Device;

/// 1 画素の答え。透明の画素 `Pixel([0, 0, 0, 0])` と「画素なし」「読めない」を区別できる。
///
/// 閾値を持たない（当たりかどうかは使い手が値で決める・5.8）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PixelRead {
    /// α 掛け済みの 8 ビット BGRA。
    Pixel([u8; 4]),
    /// 座標が描画面の外（GPU から読まない・5.3）。
    Outside,
    /// 描画面が無い・まだ合成されていない・装置が無いか失われた・GPU の呼び出しが失敗した（5.5・5.6）。
    /// 前の値や透明に化かさない。
    Unreadable,
}

/// 全体の答え（5.9）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SurfaceImage {
    /// 1 行＝幅×4 バイトに詰めた α 掛け済みの BGRA（左上から行の順）。
    Pixels {
        width: u32,
        height: u32,
        bytes: Vec<u8>,
    },
    /// `PixelRead::Unreadable` と同じ場合。
    Unreadable,
}

/// 描画面の持ち主（`SurfaceGraphics` を持つ実体）と座標で、最後に合成し終えた画素を読む ECS の口。
///
/// 読み戻しの決め:
/// - 読むのは最後に合成し終えた入れ物で、前のフレームの値を使い回さない（5.2）。呼び出しの中で
///   同期して読み、GPU が合成を終えるまで待つ。
/// - 座標は描画面の物理ピクセル（左上が (0, 0)）で、WUC の atlas の offset を含まない。
///   描画面の大きさの外は `Outside`（GPU から読まない・5.3）。
/// - 同じ座標で合成し直していなければ GPU から読まない（5.4）。
/// - 装置が無いか無効・描画面が無い・まだ合成されていない（大きさが変わって作り直す前も）は
///   `Unreadable`（5.5）。判定の順は 装置 → 描画面 → 座標 → 入れ物。
/// - GPU の失敗は、装置の失いなら記録せず（立ち直りの道が 1 度だけ記録する）、そうでなければ
///   `error!`（実体・HRESULT）を残して `Unreadable`（5.6）。
/// - 読むのは入れ物と読み戻しの写し先だけで、共有の DC に触らない。
#[derive(SystemParam)]
pub struct SurfaceReadback<'w, 's> {
    surfaces: Query<'w, 's, (&'static SurfaceGraphics, Option<&'static Name>)>,
    core: Option<Res<'w, GraphicsCore>>,
}

impl SurfaceReadback<'_, '_> {
    /// 1 画素を読む（座標は描画面の物理ピクセル）。
    pub fn read_pixel(&self, surface_owner: Entity, x: u32, y: u32) -> PixelRead {
        let Some((core, surface, name)) = self.target(surface_owner) else {
            return PixelRead::Unreadable;
        };
        pixel_in(surface.size, surface.composite(), x, y).unwrap_or_else(|err| {
            report(core, surface_owner, name, &err);
            PixelRead::Unreadable
        })
    }

    /// 全体を読む（`read_pixel` と同じ順で、座標の確かめと控えが無い）。
    pub fn read_all(&self, surface_owner: Entity) -> SurfaceImage {
        let Some((core, surface, name)) = self.target(surface_owner) else {
            return SurfaceImage::Unreadable;
        };
        let Some(device) = core.d2d_device() else {
            return SurfaceImage::Unreadable;
        };
        image_in(surface.size, surface.composite(), device).unwrap_or_else(|err| {
            report(core, surface_owner, name, &err);
            SurfaceImage::Unreadable
        })
    }

    /// 有効な装置と、描画面を持つ部品。どちらかが無ければ `None`。
    fn target(&self, owner: Entity) -> Option<(&GraphicsCore, &SurfaceGraphics, Option<&Name>)> {
        let core = self.core.as_deref().filter(|c| c.is_valid())?;
        let (surface, name) = self.surfaces.get(owner).ok()?;
        surface.is_valid().then_some((core, surface, name))
    }
}

/// 描画面の大きさで合成し終えた入れ物（世代 1 以上）。
fn composed(size: (u32, u32), composite: Option<&SurfaceComposite>) -> Option<&SurfaceComposite> {
    composite.filter(|c| c.generation() > 0 && c.size() == size)
}

/// 座標 → 入れ物の順の判定（装置と描画面は呼び手が確かめ済み）。GPU の失敗は `Err`。
pub(crate) fn pixel_in(
    size: (u32, u32),
    composite: Option<&SurfaceComposite>,
    x: u32,
    y: u32,
) -> windows::core::Result<PixelRead> {
    if x >= size.0 || y >= size.1 {
        return Ok(PixelRead::Outside);
    }
    match composed(size, composite) {
        Some(c) => c.read_pixel(x, y).map(PixelRead::Pixel),
        None => Ok(PixelRead::Unreadable),
    }
}

/// 入れ物の判定と全体の読み（写し先は `device` の新しい DC で作る）。GPU の失敗は `Err`。
pub(crate) fn image_in(
    size: (u32, u32),
    composite: Option<&SurfaceComposite>,
    device: &ID2D1Device,
) -> windows::core::Result<SurfaceImage> {
    let Some(c) = composed(size, composite) else {
        return Ok(SurfaceImage::Unreadable);
    };
    Ok(SurfaceImage::Pixels {
        width: size.0,
        height: size.1,
        bytes: c.read_all(device)?,
    })
}

/// GPU の失敗の記録（装置の失いは記録しない・5.6）。
fn report(core: &GraphicsCore, entity: Entity, name: Option<&Name>, err: &windows::core::Error) {
    if core.is_lost_failure(err) {
        return;
    }
    error!(
        entity = %format_entity_name(entity, name),
        error = ?err,
        hresult = format_args!("0x{:08X}", err.code().0),
        "[readback] read failed"
    );
}

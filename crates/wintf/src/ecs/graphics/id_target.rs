//! 画面に出ない ID の描き先。形の列を不透明の ID の色で上書きして塗る。

use super::GraphicsCore;
use super::composite::create_bitmap;
use std::num::NonZeroU32;
use tracing::error;
use windows::Win32::Foundation::{D2DERR_RECREATE_TARGET, E_INVALIDARG};
use windows::Win32::Graphics::Direct2D::Common::*;
use windows::Win32::Graphics::Direct2D::*;
use windows::core::{Error, Result};
use windows_numerics::{Matrix3x2, Vector2};

/// ID の上限（24 ビット）。0 は「ID なし」。
pub const MAX_ID: u32 = 0x00FF_FFFF;

const MAX_SIDE: u32 = 16_384;

/// 形。座標は描き先の画素の単位（左上が (0, 0)）。
#[derive(Clone, Debug, PartialEq)]
pub enum IdForm {
    Rect {
        left: f32,
        top: f32,
        right: f32,
        bottom: f32,
    },
    Ellipse {
        center: (f32, f32),
        radius: (f32, f32),
    },
    /// 書いた順の頂点。最後の頂点から最初の頂点へ閉じ、巻き数が 0 でない所を塗る。
    Polygon(Vec<(f32, f32)>),
}

/// ID（1〜`MAX_ID`）と形の組。
#[derive(Clone, Debug, PartialEq)]
pub struct IdShape {
    pub id: NonZeroU32,
    pub form: IdForm,
}

/// 1 点の答え。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IdRead {
    Id(NonZeroU32),
    /// 塗られていない画素（ID なし）。
    Empty,
    /// 座標が描き先の外（画素なし）。
    Outside,
    /// まだ塗っていない・装置が無いか失われた・塗りか読みが失敗した。
    Unreadable,
}

/// 画面に出ない ID の描き先（`B8G8R8A8_UNORM`・α 掛け済み・96 DPI）。描画面には結び付かない。
///
/// ID は不透明の色（B＝下位 8 ビット・G＝中位・R＝上位・A＝255）で塗り、塗っていない画素は透明のまま。
/// 装置が作り直されたら、使い手が新しい描き先を作って塗り直す。
pub struct IdTarget {
    bitmap: ID2D1Bitmap1,
    size: (u32, u32),
    generation: u64,
}

// SAFETY: 描き先は D2D1_FACTORY_TYPE_MULTI_THREADED の工場の系列（GraphicsCore の装置）の資源で、
// 同じ工場の系列への COM 呼び出しは D2D が内部で直列化する。世代は `&mut self` でだけ変わる。
unsafe impl Send for IdTarget {}
unsafe impl Sync for IdTarget {}

impl std::fmt::Debug for IdTarget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IdTarget")
            .field("size", &self.size)
            .field("generation", &self.generation)
            .finish()
    }
}

impl IdTarget {
    /// 一辺 1〜16,384 の描き先を `core` の装置で作る。装置が無ければ `D2DERR_RECREATE_TARGET`。
    pub fn new(core: &GraphicsCore, size: (u32, u32)) -> Result<Self> {
        let dc = core.device_context().ok_or_else(no_device)?;
        Self::with_dc(dc, size).inspect_err(|err| report(core, "create", err))
    }

    pub(crate) fn with_dc(dc: &ID2D1DeviceContext, size: (u32, u32)) -> Result<Self> {
        let side = 1..=MAX_SIDE;
        if !side.contains(&size.0) || !side.contains(&size.1) {
            return Err(Error::new(E_INVALIDARG, format!("id target size {size:?}")));
        }
        Ok(Self {
            bitmap: create_bitmap(dc, size, D2D1_BITMAP_OPTIONS_TARGET)?,
            size,
            generation: 0,
        })
    }

    pub fn size(&self) -> (u32, u32) {
        self.size
    }

    /// 塗り終えた回数（0＝まだか、最後の塗りが失敗した）。
    pub fn generation(&self) -> u64 {
        self.generation
    }

    /// 透明に消してから列の順に、縁のぼかしなしで上書きして塗る（UI スレッドから呼ぶ）。
    /// `MAX_ID` を越える ID を含む列は塗らずに失敗にする。失敗は記録し（装置の失いは記録しない）、
    /// 世代を 0 に戻す。
    pub fn paint(&mut self, core: &GraphicsCore, shapes: &[IdShape]) -> Result<()> {
        let Some(dc) = core.device_context() else {
            self.generation = 0;
            return Err(no_device());
        };
        self.paint_with(dc, shapes)
            .inspect_err(|err| report(core, "paint", err))
    }

    /// 共有の DC を借りて塗る。戻るとき `dc` は的なし・変換は恒等・ぼかしの設定は元どおり。
    pub(crate) fn paint_with(&mut self, dc: &ID2D1DeviceContext, shapes: &[IdShape]) -> Result<()> {
        let result = self.draw(dc, shapes);
        self.generation = match result {
            Ok(()) => self.generation + 1,
            Err(_) => 0,
        };
        result
    }

    fn draw(&self, dc: &ID2D1DeviceContext, shapes: &[IdShape]) -> Result<()> {
        if let Some(over) = shapes.iter().find(|s| s.id.get() > MAX_ID) {
            return Err(Error::new(
                E_INVALIDARG,
                format!("id {:#X} over MAX_ID", over.id),
            ));
        }
        // SAFETY: Windows API 境界。`dc` は UI スレッドだけが使う共有の DC（呼び手の約束）。
        unsafe {
            let factory = dc.GetFactory()?;
            let antialias = dc.GetAntialiasMode();
            dc.SetTarget(&self.bitmap);
            dc.SetDpi(96.0, 96.0);
            dc.BeginDraw();
            dc.SetTransform(&Matrix3x2::identity());
            dc.SetAntialiasMode(D2D1_ANTIALIAS_MODE_ALIASED);
            dc.SetPrimitiveBlend(D2D1_PRIMITIVE_BLEND_SOURCE_OVER);
            dc.Clear(Some(&D2D1_COLOR_F::default()));
            let filled = fill_all(dc, &factory, shapes);
            let ended = dc.EndDraw(None, None);
            dc.SetAntialiasMode(antialias);
            dc.SetTransform(&Matrix3x2::identity());
            dc.SetTarget(None);
            filled.and(ended)
        }
    }

    #[cfg(test)]
    pub(crate) fn bitmap(&self) -> &ID2D1Bitmap1 {
        &self.bitmap
    }
}

/// `BeginDraw` の中で呼ぶ。
fn fill_all(dc: &ID2D1DeviceContext, factory: &ID2D1Factory, shapes: &[IdShape]) -> Result<()> {
    // SAFETY: Windows API 境界。`dc` は呼び手が描きを始めた共有の DC。
    unsafe {
        let brush = dc.CreateSolidColorBrush(&D2D1_COLOR_F::default(), None)?;
        for shape in shapes {
            brush.SetColor(&id_color(shape.id.get()));
            match &shape.form {
                &IdForm::Rect {
                    left,
                    top,
                    right,
                    bottom,
                } => dc.FillRectangle(
                    &D2D_RECT_F {
                        left,
                        top,
                        right,
                        bottom,
                    },
                    &brush,
                ),
                &IdForm::Ellipse {
                    center: (x, y),
                    radius: (rx, ry),
                } => dc.FillEllipse(
                    &D2D1_ELLIPSE {
                        point: Vector2 { X: x, Y: y },
                        radiusX: rx,
                        radiusY: ry,
                    },
                    &brush,
                ),
                IdForm::Polygon(points) => {
                    if let Some(path) = polygon(factory, points)? {
                        dc.FillGeometry(&path, &brush, None);
                    }
                }
            }
        }
        Ok(())
    }
}

/// 巻き数で塗る閉じた path geometry（頂点が無ければ `None`）。
fn polygon(factory: &ID2D1Factory, points: &[(f32, f32)]) -> Result<Option<ID2D1PathGeometry>> {
    let mut points = points.iter().map(|&(x, y)| Vector2 { X: x, Y: y });
    let Some(first) = points.next() else {
        return Ok(None);
    };
    let rest: Vec<Vector2> = points.collect();
    // SAFETY: Windows API 境界。
    unsafe {
        let path = factory.CreatePathGeometry()?;
        let sink = path.Open()?;
        sink.SetFillMode(D2D1_FILL_MODE_WINDING);
        sink.BeginFigure(first, D2D1_FIGURE_BEGIN_FILLED);
        sink.AddLines(&rest);
        sink.EndFigure(D2D1_FIGURE_END_CLOSED);
        sink.Close()?;
        Ok(Some(path))
    }
}

fn id_color(id: u32) -> D2D1_COLOR_F {
    let channel = |shift: u32| ((id >> shift) & 0xFF) as f32 / 255.0;
    D2D1_COLOR_F {
        r: channel(16),
        g: channel(8),
        b: channel(0),
        a: 1.0,
    }
}

fn no_device() -> Error {
    Error::from(D2DERR_RECREATE_TARGET)
}

fn report(core: &GraphicsCore, what: &str, err: &Error) {
    if core.is_lost_failure(err) {
        return;
    }
    error!(
        error = ?err,
        hresult = format_args!("0x{:08X}", err.code().0),
        "[id_target] {what} failed"
    );
}

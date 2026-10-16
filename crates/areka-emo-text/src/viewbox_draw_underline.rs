//! 下線の受け手（COM 層・areka-P0-balloon-text-bugs 件 3・要件 3.1／3.2／3.5／3.10）。
//!
//! 行の TextLayout の下線の区間について、DirectWrite が教える下線の幾何（`DWRITE_UNDERLINE`）と
//! その区間の描画効果を集める。塗る矩形の決め方と面の内側への寄せは純粋層
//! [`crate::underline`] が担い、本モジュールは値を写すだけ（太さ・幅・縦書きの位置・色は
//! DirectWrite と D2D の決め方のまま＝要件 3.2）。
//!
//! - 受け手は `#[implement(IDWriteTextRenderer1)]`（縦書きの行は `IDWriteTextRenderer1` を持たない
//!   受け手を `DWRITE_E_TEXTRENDERERINCOMPATIBLE` で断るため）。`DrawUnderline` だけを記録し、
//!   字・打ち消し線・埋め込み物は何もしない。画素への寄せの問いには D2D の描画と同じ条件（拡大率 k の変換・
//!   1 DIP＝1 画像画素・寄せは有効＝描画は `D2D1_DRAW_TEXT_OPTIONS_NO_SNAP` を使わない）を答える。
//! - [`collect_underlines`] は区間に下線を立てて `IDWriteTextLayout::Draw` で集め、集めるのが
//!   失敗しても区間の下線を立てる前の状態へ必ず戻す。失敗は `error!`＋`Err`（当該フレームを見送る）。
//! - 描画効果は `ID2D1Brush` へ写して持つ。写せない効果は既定の文字色で塗る印（`None`・`debug!`）。

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use windows::Win32::Foundation::E_INVALIDARG;
use windows::Win32::Graphics::Direct2D::ID2D1Brush;
use windows::Win32::Graphics::DirectWrite::{
    DWRITE_GLYPH_ORIENTATION_ANGLE, DWRITE_GLYPH_ORIENTATION_ANGLE_0_DEGREES,
    DWRITE_GLYPH_ORIENTATION_ANGLE_90_DEGREES, DWRITE_GLYPH_RUN, DWRITE_GLYPH_RUN_DESCRIPTION,
    DWRITE_MATRIX, DWRITE_MEASURING_MODE, DWRITE_READING_DIRECTION_LEFT_TO_RIGHT,
    DWRITE_READING_DIRECTION_RIGHT_TO_LEFT, DWRITE_READING_DIRECTION_TOP_TO_BOTTOM,
    DWRITE_STRIKETHROUGH, DWRITE_TEXT_RANGE, DWRITE_UNDERLINE, IDWriteInlineObject,
    IDWritePixelSnapping_Impl, IDWriteTextLayout, IDWriteTextRenderer_Impl, IDWriteTextRenderer1,
    IDWriteTextRenderer1_Impl,
};
use windows_core::{BOOL, HRESULT, IUnknown, Interface, Ref, implement};

use super::device_err;
use crate::TextLayerError;
use crate::underline::{ReadingAxis, UnderlineRaw};

/// 1 本の線の材料（幾何と、その区間の描画効果のブラシ。`None` は既定の文字色で塗る）。
pub(crate) struct RecordedUnderline {
    /// DirectWrite が教えた下線の値の写し（行の原点基準・画像の画素）。
    pub raw: UnderlineRaw,
    /// 区間の描画効果のブラシ（無い・写せないときは `None`＝既定の文字色）。
    pub brush: Option<ID2D1Brush>,
}

/// 区間に下線を立て、`IDWriteTextLayout::Draw` を記録の受け手へ渡し、区間の下線を戻す。
///
/// 戻す先は立てる前の状態（区間の中の下線の有無の切れ目ごと）。戻しは集めるのが失敗しても
/// 行う。失敗は `error!` を残して `Err`（当該フレームを見送る）。`scale` は描画の拡大率 k
/// （画素への寄せの問いに答える変換）。`origin` は `Draw` に渡す原点＝D2D が字を描く原点
/// （DirectWrite はベースラインを原点込みで装置の画素へ寄せるので、原点×k が整数画素でない行で
/// 原点 (0,0) で集めると字のベースラインと 1 装置画素ずれうる）。線の値はこの原点込みで返る。
/// 区間が空なら何もしない。
pub(crate) fn collect_underlines(
    layout: &IDWriteTextLayout,
    ranges: &[DWRITE_TEXT_RANGE],
    scale: f32,
    origin: (f32, f32),
) -> Result<Vec<RecordedUnderline>, TextLayerError> {
    if ranges.is_empty() {
        return Ok(Vec::new());
    }
    // 立てる前の状態を先に写す（ここで失敗すれば何も変えていない）。
    let mut saved = Vec::new();
    for &range in ranges {
        underline_runs(layout, range, &mut saved)?;
    }
    let collected = set_and_draw(layout, ranges, scale, origin);
    let restored = restore(layout, &saved);
    let recorded = collected?;
    restored?;
    Ok(recorded)
}

/// 区間の中の下線の有無を、切れ目ごとの `(有無, 区間)` として `out` へ足す。
fn underline_runs(
    layout: &IDWriteTextLayout,
    range: DWRITE_TEXT_RANGE,
    out: &mut Vec<(bool, DWRITE_TEXT_RANGE)>,
) -> Result<(), TextLayerError> {
    let end = range.startPosition.saturating_add(range.length);
    let mut pos = range.startPosition;
    while pos < end {
        let mut has = BOOL(0);
        let mut run = DWRITE_TEXT_RANGE::default();
        unsafe { layout.GetUnderline(pos, &mut has, Some(&mut run)) }
            .map_err(device_err("GetUnderline(下線を集める前の写し)"))?;
        // 文字列の終わりの先は長さが果てまで続く区間で返るので、区間の終わりで切る。
        let run_end = run
            .startPosition
            .saturating_add(run.length)
            .clamp(pos + 1, end);
        out.push((
            has.as_bool(),
            DWRITE_TEXT_RANGE {
                startPosition: pos,
                length: run_end - pos,
            },
        ));
        pos = run_end;
    }
    Ok(())
}

/// 区間に下線を立てて記録の受け手で `Draw` する。
fn set_and_draw(
    layout: &IDWriteTextLayout,
    ranges: &[DWRITE_TEXT_RANGE],
    scale: f32,
    origin: (f32, f32),
) -> Result<Vec<RecordedUnderline>, TextLayerError> {
    for &range in ranges {
        unsafe { layout.SetUnderline(true, range) }
            .map_err(device_err("SetUnderline(下線を集める区間)"))?;
    }
    let recorded = Rc::new(RefCell::new(Vec::new()));
    let failure = Rc::new(Cell::new(None));
    let renderer: IDWriteTextRenderer1 = UnderlineRecorder {
        scale,
        recorded: Rc::clone(&recorded),
        failure: Rc::clone(&failure),
    }
    .into();
    unsafe { layout.Draw(None, &renderer, origin.0, origin.1) }
        .map_err(device_err("IDWriteTextLayout::Draw(下線を集める)"))?;
    // DirectWrite は `DrawUnderline` の失敗を `Draw` の戻り値へ伝えないので、受け手の側で見張る。
    if let Some(hresult) = failure.get() {
        return Err(device_err("DrawUnderline(下線を記録する)")(
            hresult.into(),
        ));
    }
    Ok(recorded.take())
}

/// 写した状態へ戻す。1 つ失敗しても残りを戻し、最初の失敗を返す。
fn restore(
    layout: &IDWriteTextLayout,
    saved: &[(bool, DWRITE_TEXT_RANGE)],
) -> Result<(), TextLayerError> {
    let mut first = Ok(());
    for &(has, range) in saved {
        let result = unsafe { layout.SetUnderline(has, range) }
            .map_err(device_err("SetUnderline(下線を戻す)"));
        if first.is_ok() {
            first = result;
        }
    }
    first
}

/// 下線だけを記録する受け手（`IDWriteTextLayout::Draw` の 1 回の間だけ生きる）。
#[implement(IDWriteTextRenderer1)]
struct UnderlineRecorder {
    /// 描画の拡大率 k（画素への寄せの問いの答え）。
    scale: f32,
    /// 集めた線（呼び手と共有）。
    recorded: Rc<RefCell<Vec<RecordedUnderline>>>,
    /// 記録の失敗（呼び手と共有・`Draw` が伝えないため）。
    failure: Rc<Cell<Option<HRESULT>>>,
}

impl UnderlineRecorder {
    /// `DrawUnderline` の値を線の材料へ写す。
    fn record(
        &self,
        x: f32,
        y: f32,
        angle: DWRITE_GLYPH_ORIENTATION_ANGLE,
        underline: *const DWRITE_UNDERLINE,
        effect: Ref<IUnknown>,
    ) -> Result<RecordedUnderline, HRESULT> {
        #[cfg(test)]
        if FAIL_RECORDING.get() {
            return Err(windows::Win32::Foundation::E_FAIL);
        }
        // SAFETY: DirectWrite が `Draw` の間だけ渡す下線の値（null は弾く）。
        let Some(u) = (unsafe { underline.as_ref() }) else {
            return Err(E_INVALIDARG);
        };
        let reading = match u.readingDirection {
            DWRITE_READING_DIRECTION_LEFT_TO_RIGHT => ReadingAxis::LeftToRight,
            DWRITE_READING_DIRECTION_RIGHT_TO_LEFT => ReadingAxis::RightToLeft,
            DWRITE_READING_DIRECTION_TOP_TO_BOTTOM => ReadingAxis::TopToBottom,
            other => {
                // areka の書き方（DirectionRecipe）は下から上を使わない。
                tracing::error!(
                    reading_direction = other.0,
                    "下線の読みの向きが areka の書き方に無い"
                );
                return Err(E_INVALIDARG);
            }
        };
        // `stroke_rect` の写し方は、横書きは回さず・縦書きは時計回りに 90 度回す前提
        // （D2D の既定の描き方と同じ）。DirectWrite の向きの角度がその前提と違えば記録しない。
        let expected = match reading {
            ReadingAxis::TopToBottom => DWRITE_GLYPH_ORIENTATION_ANGLE_90_DEGREES,
            ReadingAxis::LeftToRight | ReadingAxis::RightToLeft => {
                DWRITE_GLYPH_ORIENTATION_ANGLE_0_DEGREES
            }
        };
        if angle != expected {
            tracing::error!(
                orientation_angle = angle.0,
                ?reading,
                "下線の向きの角度が写し方の前提と違う"
            );
            return Err(E_INVALIDARG);
        }
        let brush = effect.as_ref().and_then(|e| match e.cast::<ID2D1Brush>() {
            Ok(brush) => Some(brush),
            Err(_) => {
                tracing::debug!("下線の描画効果をブラシへ写せない——既定の文字色で塗る");
                None
            }
        });
        Ok(RecordedUnderline {
            raw: UnderlineRaw {
                baseline_origin: (x, y),
                width: u.width,
                thickness: u.thickness,
                offset: u.offset,
                reading,
            },
            brush,
        })
    }
}

impl IDWritePixelSnapping_Impl for UnderlineRecorder_Impl {
    fn IsPixelSnappingDisabled(
        &self,
        _context: *const core::ffi::c_void,
    ) -> windows_core::Result<BOOL> {
        // D2D の描画は NO_SNAP を使わない＝寄せは有効。
        Ok(BOOL(0))
    }

    fn GetCurrentTransform(
        &self,
        _context: *const core::ffi::c_void,
        transform: *mut DWRITE_MATRIX,
    ) -> windows_core::Result<()> {
        if transform.is_null() {
            return Err(E_INVALIDARG.into());
        }
        // SAFETY: DirectWrite が渡す書き込み先（null は上で弾いた）。
        unsafe {
            transform.write(DWRITE_MATRIX {
                m11: self.scale,
                m12: 0.0,
                m21: 0.0,
                m22: self.scale,
                dx: 0.0,
                dy: 0.0,
            })
        };
        Ok(())
    }

    fn GetPixelsPerDip(&self, _context: *const core::ffi::c_void) -> windows_core::Result<f32> {
        // 1 DIP＝1 画像画素（拡大率は変換の側で答える）。
        Ok(1.0)
    }
}

impl IDWriteTextRenderer_Impl for UnderlineRecorder_Impl {
    fn DrawGlyphRun(
        &self,
        _context: *const core::ffi::c_void,
        _x: f32,
        _y: f32,
        _mode: DWRITE_MEASURING_MODE,
        _run: *const DWRITE_GLYPH_RUN,
        _description: *const DWRITE_GLYPH_RUN_DESCRIPTION,
        _effect: Ref<IUnknown>,
    ) -> windows_core::Result<()> {
        Ok(())
    }

    fn DrawUnderline(
        &self,
        context: *const core::ffi::c_void,
        x: f32,
        y: f32,
        underline: *const DWRITE_UNDERLINE,
        effect: Ref<IUnknown>,
    ) -> windows_core::Result<()> {
        // `IDWriteTextRenderer1` を持つ受け手には DirectWrite は角度つきの口を呼ぶ。
        IDWriteTextRenderer1_Impl::DrawUnderline(
            self,
            context,
            x,
            y,
            DWRITE_GLYPH_ORIENTATION_ANGLE_0_DEGREES,
            underline,
            effect,
        )
    }

    fn DrawStrikethrough(
        &self,
        _context: *const core::ffi::c_void,
        _x: f32,
        _y: f32,
        _strikethrough: *const DWRITE_STRIKETHROUGH,
        _effect: Ref<IUnknown>,
    ) -> windows_core::Result<()> {
        Ok(())
    }

    fn DrawInlineObject(
        &self,
        _context: *const core::ffi::c_void,
        _x: f32,
        _y: f32,
        _object: Ref<IDWriteInlineObject>,
        _sideways: BOOL,
        _right_to_left: BOOL,
        _effect: Ref<IUnknown>,
    ) -> windows_core::Result<()> {
        Ok(())
    }
}

impl IDWriteTextRenderer1_Impl for UnderlineRecorder_Impl {
    fn DrawGlyphRun(
        &self,
        _context: *const core::ffi::c_void,
        _x: f32,
        _y: f32,
        _angle: DWRITE_GLYPH_ORIENTATION_ANGLE,
        _mode: DWRITE_MEASURING_MODE,
        _run: *const DWRITE_GLYPH_RUN,
        _description: *const DWRITE_GLYPH_RUN_DESCRIPTION,
        _effect: Ref<IUnknown>,
    ) -> windows_core::Result<()> {
        Ok(())
    }

    fn DrawUnderline(
        &self,
        _context: *const core::ffi::c_void,
        x: f32,
        y: f32,
        angle: DWRITE_GLYPH_ORIENTATION_ANGLE,
        underline: *const DWRITE_UNDERLINE,
        effect: Ref<IUnknown>,
    ) -> windows_core::Result<()> {
        match self.record(x, y, angle, underline, effect) {
            Ok(line) => {
                self.recorded.borrow_mut().push(line);
                Ok(())
            }
            Err(hresult) => {
                self.failure.set(Some(hresult));
                Err(hresult.into())
            }
        }
    }

    fn DrawStrikethrough(
        &self,
        _context: *const core::ffi::c_void,
        _x: f32,
        _y: f32,
        _angle: DWRITE_GLYPH_ORIENTATION_ANGLE,
        _strikethrough: *const DWRITE_STRIKETHROUGH,
        _effect: Ref<IUnknown>,
    ) -> windows_core::Result<()> {
        Ok(())
    }

    fn DrawInlineObject(
        &self,
        _context: *const core::ffi::c_void,
        _x: f32,
        _y: f32,
        _angle: DWRITE_GLYPH_ORIENTATION_ANGLE,
        _object: Ref<IDWriteInlineObject>,
        _sideways: BOOL,
        _right_to_left: BOOL,
        _effect: Ref<IUnknown>,
    ) -> windows_core::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
thread_local! {
    /// 真のとき受け手の `DrawUnderline` が失敗する（失敗しても戻すことの檻だけが使う）。
    static FAIL_RECORDING: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

#[cfg(test)]
#[path = "viewbox_draw_underline_tests.rs"]
mod tests;

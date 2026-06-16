use std::cell::RefCell;
use windows::Win32::Graphics::Direct2D::Common::*;
use windows::Win32::Graphics::Direct2D::*;
use windows::Win32::Graphics::DirectWrite::*;
use windows::core::*;
use windows_numerics::*;

use super::command_types::*;

#[implement(ID2D1CommandSink5)]
#[derive(Clone, Debug)]
pub struct RecCommandSink {
    commands: RefCell<Vec<DrawCommand>>,
}

impl RecCommandSink {
    pub fn new() -> Self {
        Self {
            commands: RefCell::new(Vec::new()),
        }
    }

    pub fn clear(&mut self) {
        self.commands.borrow_mut().clear();
    }

    pub fn push(&self, command: DrawCommand) {
        self.commands.borrow_mut().push(command);
    }

    pub fn commands(&self) -> &Vec<DrawCommand> {
        // This is tricky because we can't return a reference to the inside of a RefCell.
        // For now, let's assume the caller will handle this. A better API might be needed.
        // A quick fix is to make this method unsafe or change its signature.
        // Let's try to make it work by returning a Ref<Vec<...>>
        // But the original signature returns &Vec.
        // Let's comment it out for now and see if it's used.
        // &self.commands
        todo!()
    }
}

impl ID2D1CommandSink_Impl for RecCommandSink_Impl {
    fn BeginDraw(&self) -> Result<()> {
        self.push(DrawCommand::BeginDraw);
        Ok(())
    }

    fn EndDraw(&self) -> Result<()> {
        self.push(DrawCommand::EndDraw);
        Ok(())
    }

    fn SetAntialiasMode(&self, antialiasmode: D2D1_ANTIALIAS_MODE) -> Result<()> {
        self.push(DrawCommand::SetAntialiasMode(SetAntialiasMode::new(
            antialiasmode,
        )));
        Ok(())
    }
    fn SetTags(&self, tag1: u64, tag2: u64) -> Result<()> {
        self.push(DrawCommand::SetTags(SetTags::new(tag1, tag2)));
        Ok(())
    }
    fn SetTextAntialiasMode(&self, textantialiasmode: D2D1_TEXT_ANTIALIAS_MODE) -> Result<()> {
        self.push(DrawCommand::SetTextAntialiasMode(
            SetTextAntialiasMode::new(textantialiasmode),
        ));
        Ok(())
    }

    fn SetTextRenderingParams(
        &self,
        textrenderingparams: Ref<IDWriteRenderingParams>,
    ) -> Result<()> {
        self.push(DrawCommand::SetTextRenderingParams(
            SetTextRenderingParams::new(textrenderingparams.as_ref()),
        ));
        Ok(())
    }

    fn SetTransform(&self, transform: *const Matrix3x2) -> Result<()> {
        // SAFETY 根拠: ID2D1CommandSink の各コールバックは D2D1 ランタイムの
        // コマンドリスト再生（ID2D1CommandList::Stream）からのみ呼ばれ、
        // 値構造体への *const 引数（transform/rect/cliprect/layerparameters1）は
        // D2D1 が記録済みコマンドから渡す非 null ポインタである（null 許容の
        // 引数は windows-rs 側で Option/null チェック付き as_ref に写像済み）。
        // 無条件 deref の前提を debug ビルドで検査する。
        debug_assert!(!transform.is_null());
        let transform = unsafe { *transform };
        self.push(DrawCommand::SetTransform(SetTransform::new(transform)));
        Ok(())
    }

    fn SetPrimitiveBlend(&self, primitiveblend: D2D1_PRIMITIVE_BLEND) -> Result<()> {
        self.push(DrawCommand::SetPrimitiveBlend(SetPrimitiveBlend::new(
            primitiveblend,
        )));
        Ok(())
    }

    fn SetUnitMode(&self, unitmode: D2D1_UNIT_MODE) -> Result<()> {
        self.push(DrawCommand::SetUnitMode(SetUnitMode::new(unitmode)));
        Ok(())
    }

    fn Clear(&self, color: *const D2D1_COLOR_F) -> Result<()> {
        let color = unsafe { color.as_ref() }.cloned();
        self.push(DrawCommand::Clear(Clear::new(color)));
        Ok(())
    }

    fn DrawGlyphRun(
        &self,
        baselineorigin: &Vector2,
        glyphrun: *const DWRITE_GLYPH_RUN,
        glyphrundescription: *const DWRITE_GLYPH_RUN_DESCRIPTION,
        foregroundbrush: Ref<ID2D1Brush>,
        measuringmode: DWRITE_MEASURING_MODE,
    ) -> Result<()> {
        unsafe {
            // SAFETY 根拠: DWRITE_GLYPH_RUN 契約上 glyphIndices は必須フィールド
            // （glyphAdvances/glyphOffsets と異なり null 不許可）であり、D2D1 は
            // 記録時に検証済みのグリフランのみを再生する。from_raw_parts は
            // len == 0 でも非 null・整列済みポインタを要求するため、
            // 前提を debug ビルドで検査する（glyphCount as usize は
            // u32 → usize の拡大変換のみで切り捨てなし）。
            debug_assert!(!glyphrun.is_null());
            let glyph_run = &*glyphrun;
            debug_assert!(!glyph_run.glyphIndices.is_null());
            let glyph_indices =
                std::slice::from_raw_parts(glyph_run.glyphIndices, glyph_run.glyphCount as usize)
                    .to_vec();
            let glyph_advances = if glyph_run.glyphAdvances.is_null() {
                Vec::new()
            } else {
                std::slice::from_raw_parts(glyph_run.glyphAdvances, glyph_run.glyphCount as usize)
                    .to_vec()
            };
            let glyph_offsets = if glyph_run.glyphOffsets.is_null() {
                Vec::new()
            } else {
                std::slice::from_raw_parts(glyph_run.glyphOffsets, glyph_run.glyphCount as usize)
                    .to_vec()
            };

            self.push(DrawCommand::DrawGlyphRun(DrawGlyphRun::new(
                *baselineorigin,
                glyph_run.fontFace.as_ref().unwrap(),
                glyph_run.fontEmSize,
                glyph_indices,
                glyph_advances,
                glyph_offsets,
                glyph_run.isSideways.as_bool(),
                glyph_run.bidiLevel,
                glyphrundescription.as_ref().cloned(),
                foregroundbrush.as_ref().unwrap(),
                measuringmode,
            )));
        }
        Ok(())
    }

    fn DrawLine(
        &self,
        point0: &Vector2,
        point1: &Vector2,
        brush: Ref<ID2D1Brush>,
        strokewidth: f32,
        strokestyle: Ref<ID2D1StrokeStyle>,
    ) -> Result<()> {
        self.push(DrawCommand::DrawLine(DrawLine::new(
            *point0,
            *point1,
            brush.as_ref().unwrap(),
            strokewidth,
            strokestyle.as_ref(),
        )));
        Ok(())
    }

    fn DrawGeometry(
        &self,
        geometry: Ref<ID2D1Geometry>,
        brush: Ref<ID2D1Brush>,
        strokewidth: f32,
        strokestyle: Ref<ID2D1StrokeStyle>,
    ) -> Result<()> {
        self.push(DrawCommand::DrawGeometry(DrawGeometry::new(
            geometry.as_ref().unwrap(),
            brush.as_ref().unwrap(),
            strokewidth,
            strokestyle.as_ref(),
        )));
        Ok(())
    }

    fn DrawRectangle(
        &self,
        rect: *const D2D_RECT_F,
        brush: Ref<ID2D1Brush>,
        strokewidth: f32,
        strokestyle: Ref<ID2D1StrokeStyle>,
    ) -> Result<()> {
        // SAFETY 根拠: SetTransform の注記参照（D2D1 再生契約により非 null）。
        debug_assert!(!rect.is_null());
        let rect = unsafe { *rect };
        self.push(DrawCommand::DrawRectangle(DrawRectangle::new(
            rect,
            brush.as_ref().unwrap(),
            strokewidth,
            strokestyle.as_ref(),
        )));
        Ok(())
    }

    fn DrawBitmap(
        &self,
        bitmap: Ref<ID2D1Bitmap>,
        destinationrectangle: *const D2D_RECT_F,
        opacity: f32,
        interpolationmode: D2D1_INTERPOLATION_MODE,
        sourcerectangle: *const D2D_RECT_F,
        perspectivetransform: *const Matrix4x4,
    ) -> Result<()> {
        self.push(DrawCommand::DrawBitmap(DrawBitmap::new(
            bitmap.as_ref().unwrap(),
            unsafe { destinationrectangle.as_ref() }.cloned(),
            opacity,
            interpolationmode,
            unsafe { sourcerectangle.as_ref() }.cloned(),
            unsafe { perspectivetransform.as_ref() }.cloned(),
        )));
        Ok(())
    }

    fn DrawImage(
        &self,
        image: Ref<ID2D1Image>,
        targetoffset: *const Vector2,
        imagerectangle: *const D2D_RECT_F,
        interpolationmode: D2D1_INTERPOLATION_MODE,
        compositemode: D2D1_COMPOSITE_MODE,
    ) -> Result<()> {
        self.push(DrawCommand::DrawImage(DrawImage::new(
            image.as_ref().unwrap(),
            unsafe { targetoffset.as_ref() }.cloned(),
            unsafe { imagerectangle.as_ref() }.cloned(),
            interpolationmode,
            compositemode,
        )));
        Ok(())
    }

    fn DrawGdiMetafile(
        &self,
        gdimetafile: Ref<ID2D1GdiMetafile>,
        targetoffset: *const Vector2,
    ) -> Result<()> {
        self.push(DrawCommand::DrawGdiMetafile2(DrawGdiMetafile2::new(
            gdimetafile.as_ref().unwrap(),
            unsafe { targetoffset.as_ref() }.cloned(),
        )));
        Ok(())
    }

    fn FillMesh(&self, mesh: Ref<ID2D1Mesh>, brush: Ref<ID2D1Brush>) -> Result<()> {
        self.push(DrawCommand::FillMesh(FillMesh::new(
            mesh.as_ref().unwrap(),
            brush.as_ref().unwrap(),
        )));
        Ok(())
    }

    fn FillOpacityMask(
        &self,
        opacitymask: Ref<ID2D1Bitmap>,
        brush: Ref<ID2D1Brush>,
        destinationrectangle: *const D2D_RECT_F,
        sourcerectangle: *const D2D_RECT_F,
    ) -> Result<()> {
        self.push(DrawCommand::FillOpacityMask(FillOpacityMask::new(
            opacitymask.as_ref().unwrap(),
            brush.as_ref().unwrap(),
            unsafe { destinationrectangle.as_ref() }.cloned(),
            unsafe { sourcerectangle.as_ref() }.cloned(),
        )));
        Ok(())
    }

    fn FillGeometry(
        &self,
        geometry: Ref<ID2D1Geometry>,
        brush: Ref<ID2D1Brush>,
        opacitybrush: Ref<ID2D1Brush>,
    ) -> Result<()> {
        self.push(DrawCommand::FillGeometry(FillGeometry::new(
            geometry.as_ref().unwrap(),
            brush.as_ref().unwrap(),
            opacitybrush.as_ref(),
        )));
        Ok(())
    }

    fn FillRectangle(&self, rect: *const D2D_RECT_F, brush: Ref<ID2D1Brush>) -> Result<()> {
        // SAFETY 根拠: SetTransform の注記参照（D2D1 再生契約により非 null）。
        debug_assert!(!rect.is_null());
        let rect = unsafe { *rect };
        self.push(DrawCommand::FillRectangle(FillRectangle::new(
            rect,
            brush.as_ref().unwrap(),
        )));
        Ok(())
    }

    fn PushAxisAlignedClip(
        &self,
        cliprect: *const D2D_RECT_F,
        antialiasmode: D2D1_ANTIALIAS_MODE,
    ) -> Result<()> {
        // SAFETY 根拠: SetTransform の注記参照（D2D1 再生契約により非 null）。
        debug_assert!(!cliprect.is_null());
        let cliprect = unsafe { *cliprect };
        self.push(DrawCommand::PushAxisAlignedClip(PushAxisAlignedClip::new(
            cliprect,
            antialiasmode,
        )));
        Ok(())
    }

    fn PushLayer(
        &self,
        layerparameters1: *const D2D1_LAYER_PARAMETERS1,
        layer: Ref<ID2D1Layer>,
    ) -> Result<()> {
        // SAFETY 根拠: SetTransform の注記参照（D2D1 再生契約により非 null）。
        debug_assert!(!layerparameters1.is_null());
        let params = unsafe { &*layerparameters1 };
        self.push(DrawCommand::PushLayer(PushLayer::new(
            params.contentBounds,
            params.geometricMask.as_ref(),
            params.maskAntialiasMode,
            params.maskTransform,
            params.opacity,
            params.opacityBrush.as_ref(),
            params.layerOptions,
            layer.as_ref(),
        )));
        Ok(())
    }

    fn PopAxisAlignedClip(&self) -> Result<()> {
        self.push(DrawCommand::PopAxisAlignedClip);
        Ok(())
    }

    fn PopLayer(&self) -> Result<()> {
        self.push(DrawCommand::PopLayer);
        Ok(())
    }
}

impl ID2D1CommandSink1_Impl for RecCommandSink_Impl {
    fn SetPrimitiveBlend1(&self, primitiveblend: D2D1_PRIMITIVE_BLEND) -> Result<()> {
        self.push(DrawCommand::SetPrimitiveBlend1(SetPrimitiveBlend1::new(
            primitiveblend,
        )));
        Ok(())
    }
}

impl ID2D1CommandSink2_Impl for RecCommandSink_Impl {
    fn DrawInk(
        &self,
        ink: Ref<ID2D1Ink>,
        brush: Ref<ID2D1Brush>,
        inkstyle: Ref<ID2D1InkStyle>,
    ) -> Result<()> {
        self.push(DrawCommand::DrawInk(DrawInk::new(
            ink.as_ref().unwrap(),
            brush.as_ref().unwrap(),
            inkstyle.as_ref(),
        )));
        Ok(())
    }
    fn DrawGradientMesh(&self, gradientmesh: Ref<ID2D1GradientMesh>) -> Result<()> {
        self.push(DrawCommand::DrawGradientMesh(DrawGradientMesh::new(
            gradientmesh.as_ref().unwrap(),
        )));
        Ok(())
    }
    fn DrawGdiMetafile(
        &self,
        gdimetafile: Ref<ID2D1GdiMetafile>,
        destinationrectangle: *const D2D_RECT_F,
        sourcerectangle: *const D2D_RECT_F,
    ) -> Result<()> {
        self.push(DrawCommand::DrawGdiMetafile(DrawGdiMetafile::new(
            gdimetafile.as_ref().unwrap(),
            unsafe { destinationrectangle.as_ref() }.cloned(),
            unsafe { sourcerectangle.as_ref() }.cloned(),
        )));
        Ok(())
    }
}

impl ID2D1CommandSink3_Impl for RecCommandSink_Impl {
    fn DrawSpriteBatch(
        &self,
        spritebatch: Ref<ID2D1SpriteBatch>,
        startindex: u32,
        spritecount: u32,
        bitmap: Ref<ID2D1Bitmap>,
        interpolationmode: D2D1_BITMAP_INTERPOLATION_MODE,
        spriteoptions: D2D1_SPRITE_OPTIONS,
    ) -> Result<()> {
        self.push(DrawCommand::DrawSpriteBatch(DrawSpriteBatch::new(
            spritebatch.as_ref().unwrap(),
            startindex,
            spritecount,
            bitmap.as_ref().unwrap(),
            interpolationmode,
            spriteoptions,
        )));
        Ok(())
    }
}

impl ID2D1CommandSink4_Impl for RecCommandSink_Impl {
    fn SetPrimitiveBlend2(&self, primitiveblend: D2D1_PRIMITIVE_BLEND) -> Result<()> {
        self.push(DrawCommand::SetPrimitiveBlend2(SetPrimitiveBlend2::new(
            primitiveblend,
        )));
        Ok(())
    }
}

impl ID2D1CommandSink5_Impl for RecCommandSink_Impl {
    fn BlendImage(
        &self,
        image: Ref<ID2D1Image>,
        blendmode: D2D1_BLEND_MODE,
        targetoffset: *const Vector2,
        imagerectangle: *const D2D_RECT_F,
        interpolationmode: D2D1_INTERPOLATION_MODE,
    ) -> Result<()> {
        self.push(DrawCommand::BlendImage(BlendImage::new(
            image.as_ref().unwrap(),
            blendmode,
            unsafe { targetoffset.as_ref() }.cloned(),
            unsafe { imagerectangle.as_ref() }.cloned(),
            interpolationmode,
        )));
        Ok(())
    }
}

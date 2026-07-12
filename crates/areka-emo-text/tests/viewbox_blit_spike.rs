//! # viewbox_blit_spike — スパイク・GO ゲート・使い捨て検証（areka-P0-emo-text-viewbox task 1）
//!
//! **これは使い捨てのスパイクである**。本設計（viewbox 方式＝ダーティ矩形スクロール）の
//! byte 等価（R6.1）と再描画レス（R3.1）の両立は、design.md「実装順序制約（spike 先頭）」が
//! 明記するとおり **「整数平行移動の blit 結果 ≡ 新位置で `DrawTextLayout` し直した結果」という
//! ClearType/AA 位相不変仮定ただ一点** に載っている。仮定が破れれば ScrollPlanner/
//! ViewboxExecutor の本実装へ進めない（性能劣化ではなく R3 の受け入れ基準そのものが崩れる
//! ＝設計前提崩壊＝NO-GO）。本ファイルはその仮定を実測して GO/NO-GO を下すためだけに存在し、
//! GO 確認後の本実装が入れば役目を終える。
//!
//! ## 検証する仮説（design.md Testing Strategy「実装順序制約」）
//!
//! k=1.0・premultiplied 透明背景の D2D ターゲットにおいて、同一 format／同一 TextLayout で
//! 「位置 A に描いてから whole-pixel 整数の面内 blit で位置 B へ写した結果」の保持ピクセルが、
//! 「最初から位置 B に描いた結果」と **byte 一致** すること（横書き＝縦 blit・縦書き＝横 blit・
//! 数行パターン）。併せて `DIRTY_GUARD_IMG_PX` の実効値（AA こぼれ幅）を実測する。
//!
//! ## 構成（新規 production 型は作らない）
//!
//! 既存 COM 経路の規約をそのまま写す（新規コンポーネント禁止）:
//! - テクスチャ生成＝DEFAULT/B8G8R8A8/RENDER_TARGET・全 0（premultiplied 透明）初期化
//!   （`surface.rs::create_transparent_source_tex` の写し）。
//! - D2D ターゲット化＝`CreateBitmapFromDxgiSurface`＋TARGET|CANNOT_DRAW・96 DPI 名目
//!   （`draw.rs::create_target_bitmap` の写し）。
//! - 描画列＝透明 clear→`SetTransform(scale(k=1.0)=identity)`→`DrawTextLayout`
//!   （`draw.rs::DrawExecutor::render` の写し）。
//! - readback＝staging 経由 `CopyResource`→`Map(READ)`・RowPitch≥stride のパディング処理
//!   （`surface.rs::read_back` の写し）。
//! - 面内 blit＝別テクスチャへ `CopySubresourceRegion`（src と dst が別テクスチャ＝重なり安全）。
//! - format／TextLayout は本番描画と同一の `create_text_format` 経路（design RN5 の byte 等価前提）。
//!
//! GPU は headless 実資源（`GraphicsCore::new()`）。撮影不可ゆえオフスクリーン readback で
//! 決定論化する（記憶 gpu-draw-verification-offscreen-d2d-target）。

use areka_emo_text::draw::{PROBE_MAX_EXTENT, ResolvedFont, create_text_format};
use areka_emo_text::writing::WritingMode;
use areka_parsers::balloon::{
    BalloonModel, Font, FontColor, Origin, ValidRect, WindowPosition, WordWrapPoint,
};

use windows::Win32::Graphics::Direct2D::Common::{
    D2D1_ALPHA_MODE_PREMULTIPLIED, D2D1_COLOR_F, D2D1_PIXEL_FORMAT,
};
use windows::Win32::Graphics::Direct2D::{
    D2D1_BITMAP_OPTIONS_CANNOT_DRAW, D2D1_BITMAP_OPTIONS_TARGET, D2D1_BITMAP_PROPERTIES1,
    D2D1_DEVICE_CONTEXT_OPTIONS_NONE, D2D1_DRAW_TEXT_OPTIONS_NONE, ID2D1Bitmap1, ID2D1DeviceContext,
    ID2D1Image,
};
use windows::Win32::Graphics::Direct3D11::{
    D3D11_BIND_RENDER_TARGET, D3D11_BOX, D3D11_CPU_ACCESS_READ, D3D11_MAP_READ,
    D3D11_MAPPED_SUBRESOURCE, D3D11_SUBRESOURCE_DATA, D3D11_TEXTURE2D_DESC, D3D11_USAGE_DEFAULT,
    D3D11_USAGE_STAGING, ID3D11Device, ID3D11DeviceContext, ID3D11Resource, ID3D11Texture2D,
};
use windows::Win32::Graphics::DirectWrite::IDWriteFactory2;
use windows::Win32::Graphics::Dxgi::Common::{DXGI_FORMAT_B8G8R8A8_UNORM, DXGI_SAMPLE_DESC};
use windows::Win32::Graphics::Dxgi::IDXGISurface;
use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx};
use windows::core::{HSTRING, Interface};
use windows_numerics::{Matrix3x2, Vector2};

use wintf::com::d2d::{D2D1DeviceContextExt, D2D1DeviceExt};
use wintf::com::dwrite::{DWriteFactoryExt, DWriteTextLayoutExt};
use wintf::ecs::GraphicsCore;
use windows::Win32::Graphics::DirectWrite::IDWriteTextLayout;

// ── 幾何定数（k=1.0・image px ＝物理 px。line_pitch(12)=15＝ceil(12×1.25) を 1 行スクロール量 N とする） ──

/// 供給面（描画面）の物理寸。content が余裕を持って収まり、blit の露出帯／保持域が
/// 明確に分かれる寸を採る。
const W: u32 = 200;
const H: u32 = 120;

/// 1 行ぶんのスクロール blit 量（whole-pixel 整数）。`DWriteMetrics::line_pitch(12)==15`
/// （既定 ＭＳ ゴシック 12px・`ceil(12×1.25)`）と一致させ、行を跨ぐ整数平行移動を作る。
const N: u32 = 15;

/// GO 機械判定の境界許容（保持域の中心核が一致することの主檻）。保持域を全辺 CORE_SHRINK px
/// 縮めた核が byte 完全一致することを要求する。位相不変が破れれば核の内部にも不一致が広がり
/// （equivalence_guard ≫ CORE_SHRINK）本檻が赤くなる＝NO-GO。境界 AA 起因の 1px 差は許容し
/// 実効ガードとして別途報告する（design 想定＝1 image px）。
const CORE_SHRINK: u32 = 4;

/// 数行パターン（全角＋半角混在の数文字）。同一 TextLayout を A/B 双方の描画で共有する。
const LINES: [&str; 4] = ["あA1む", "いB2ね", "うC3を", "えD4か"];

/// 各行の行送り軸オフセット（block 位置・image px）。行送りピッチ N と一致させ、
/// N の整数平行移動がちょうど 1 行ぶんのスクロールに対応するようにする。
const BLOCK_POS: [f32; 4] = [10.0, 25.0, 40.0, 55.0];

/// 行内軸の開始位置（image px）。左右端（横書き）・上下端（縦書き）から離し、
/// 露出帯以外での端 AA を排除する。
const INLINE_START: f32 = 10.0;

// ══════════════════════════════════════════════════════════════════════════════════════
// COM 資源（既存 surface.rs / draw.rs の規約を写した使い捨てヘルパ群）
// ══════════════════════════════════════════════════════════════════════════════════════

/// spike 用 GPU 資源束（headless・UI スレッド専有）。
struct Spike {
    // GraphicsCore は D3D/D2D/DWrite の所有者——drop 順のため最後まで保持する。
    _core: GraphicsCore,
    d3d: ID3D11Device,
    ctx: ID3D11DeviceContext,
    dc: ID2D1DeviceContext,
    dwrite: IDWriteFactory2,
}

impl Spike {
    fn new() -> Spike {
        // COM の MTA 初期化（S_FALSE/RPC_E_CHANGED_MODE は無視——テストスレッド毎・
        // attach_wiring_test.rs と同方針）。GraphicsCore 自体は CoInitialize 非依存だが慣行に倣う。
        unsafe {
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        }
        let core = GraphicsCore::new().expect("GraphicsCore::new（headless 実資源）");
        let d3d = core.d3d().expect("GraphicsCore::d3d").clone();
        let d2d = core.d2d_device().expect("GraphicsCore::d2d_device");
        let dc = d2d
            .create_device_context(D2D1_DEVICE_CONTEXT_OPTIONS_NONE)
            .expect("CreateDeviceContext（spike 専用 DC）");
        let dwrite = core.dwrite_factory().expect("GraphicsCore::dwrite_factory").clone();
        let ctx = unsafe { d3d.GetImmediateContext() }.expect("GetImmediateContext");
        Spike { _core: core, d3d, ctx, dc, dwrite }
    }

    /// DEFAULT/B8G8R8A8/RENDER_TARGET テクスチャを全 0（premultiplied 透明）初期化で作る
    /// （surface.rs::create_transparent_source_tex の写し）。
    fn make_texture(&self) -> ID3D11Texture2D {
        let desc = D3D11_TEXTURE2D_DESC {
            Width: W,
            Height: H,
            MipLevels: 1,
            ArraySize: 1,
            Format: DXGI_FORMAT_B8G8R8A8_UNORM,
            SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 },
            Usage: D3D11_USAGE_DEFAULT,
            BindFlags: D3D11_BIND_RENDER_TARGET.0 as u32,
            CPUAccessFlags: 0,
            MiscFlags: 0,
        };
        let zeros = vec![0u8; (W * H * 4) as usize];
        let init = D3D11_SUBRESOURCE_DATA {
            pSysMem: zeros.as_ptr() as *const _,
            SysMemPitch: W * 4,
            SysMemSlicePitch: 0,
        };
        let mut tex: Option<ID3D11Texture2D> = None;
        unsafe { self.d3d.CreateTexture2D(&desc, Some(&init), Some(&mut tex)) }
            .expect("CreateTexture2D（透明初期化）");
        tex.expect("CreateTexture2D returned None")
    }

    /// CPU_READ の staging（readback 用・surface.rs::create_staging の写し）。
    fn make_staging(&self) -> ID3D11Texture2D {
        let desc = D3D11_TEXTURE2D_DESC {
            Width: W,
            Height: H,
            MipLevels: 1,
            ArraySize: 1,
            Format: DXGI_FORMAT_B8G8R8A8_UNORM,
            SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 },
            Usage: D3D11_USAGE_STAGING,
            BindFlags: 0,
            CPUAccessFlags: D3D11_CPU_ACCESS_READ.0 as u32,
            MiscFlags: 0,
        };
        let mut tex: Option<ID3D11Texture2D> = None;
        unsafe { self.d3d.CreateTexture2D(&desc, None, Some(&mut tex)) }
            .expect("CreateTexture2D（staging）");
        tex.expect("CreateTexture2D(staging) returned None")
    }

    /// テクスチャを D2D ターゲット bitmap として巻く（draw.rs::create_target_bitmap の写し・
    /// premultiplied・96 DPI 名目・SHADER_RESOURCE 非 bind ゆえ CANNOT_DRAW 併記）。
    fn target_bitmap(&self, tex: &ID3D11Texture2D) -> ID2D1Bitmap1 {
        let dxgi_surface: IDXGISurface = tex.cast().expect("tex->IDXGISurface cast");
        let props = D2D1_BITMAP_PROPERTIES1 {
            pixelFormat: D2D1_PIXEL_FORMAT {
                format: DXGI_FORMAT_B8G8R8A8_UNORM,
                alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
            },
            dpiX: 96.0,
            dpiY: 96.0,
            bitmapOptions: D2D1_BITMAP_OPTIONS_TARGET | D2D1_BITMAP_OPTIONS_CANNOT_DRAW,
            colorContext: core::mem::ManuallyDrop::new(None),
        };
        unsafe { self.dc.CreateBitmapFromDxgiSurface(&dxgi_surface, Some(&props as *const _)) }
            .expect("CreateBitmapFromDxgiSurface")
    }

    /// 本番描画と同一経路（`create_text_format`）で行 TextLayout を組む
    /// （box 寸は draw.rs::line_layout と同一: 行内軸＝PROBE_MAX_EXTENT・行送り軸＝font_height）。
    fn line_layout(&self, text: &str, font: &ResolvedFont, mode: WritingMode) -> IDWriteTextLayout {
        let format = create_text_format(&self.dwrite, font, mode).expect("create_text_format");
        let (max_w, max_h) = match mode {
            WritingMode::HorizontalTb => (PROBE_MAX_EXTENT, font.height),
            WritingMode::VerticalRl | WritingMode::VerticalLr => (font.height, PROBE_MAX_EXTENT),
        };
        self.dwrite
            .create_text_layout(&HSTRING::from(text), &format, max_w, max_h)
            .expect("create_text_layout")
    }

    /// 行を各 origin へ描く（draw.rs::render の Phase 2 写し・透明 clear→identity 変換→DrawTextLayout）。
    /// k=1.0 ゆえ SetTransform は identity（scale(1.0)）一点。色は既定黒・α=1。
    fn draw(&self, tex: &ID3D11Texture2D, draws: &[(Vector2, IDWriteTextLayout)]) {
        let brush = self
            .dc
            .create_solid_color_brush(
                &D2D1_COLOR_F { r: 0.0, g: 0.0, b: 0.0, a: 1.0 },
                None,
            )
            .expect("CreateSolidColorBrush");
        let target = self.target_bitmap(tex);
        unsafe { self.dc.SetTarget(&target) };
        unsafe { self.dc.BeginDraw() };
        self.dc.clear(None); // premultiplied 全 0（透明）
        self.dc.set_transform(&Matrix3x2 {
            M11: 1.0,
            M12: 0.0,
            M21: 0.0,
            M22: 1.0,
            M31: 0.0,
            M32: 0.0,
        });
        for (origin, layout) in draws {
            self.dc
                .draw_text_layout(*origin, layout, &brush, D2D1_DRAW_TEXT_OPTIONS_NONE);
        }
        let end = unsafe { self.dc.EndDraw(None, None) };
        unsafe { self.dc.SetTarget(None::<&ID2D1Image>) };
        end.expect("EndDraw");
    }

    /// 面内 whole-pixel blit（src の矩形域を dst の (dstx,dsty) へ `CopySubresourceRegion`）。
    /// src と dst は別テクスチャ＝重なり安全。dst の非コピー域（露出帯）は透明初期化のまま残る。
    fn blit(&self, src: &ID3D11Texture2D, dst: &ID3D11Texture2D, srcbox: D3D11_BOX, dstx: u32, dsty: u32) {
        let src_res: ID3D11Resource = src.cast().expect("src->Resource cast");
        let dst_res: ID3D11Resource = dst.cast().expect("dst->Resource cast");
        unsafe {
            self.ctx.CopySubresourceRegion(
                &dst_res,
                0,
                dstx,
                dsty,
                0,
                &src_res,
                0,
                Some(&srcbox as *const _),
            );
        }
    }

    /// staging 経由 readback（surface.rs::read_back の写し・stride=W*4 の密 BGRA・
    /// RowPitch≥stride のパディングを行ごとに詰める）。
    fn readback(&self, tex: &ID3D11Texture2D, staging: &ID3D11Texture2D) -> Vec<u8> {
        let tex_res: ID3D11Resource = tex.cast().expect("tex->Resource cast");
        let staging_res: ID3D11Resource = staging.cast().expect("staging->Resource cast");
        unsafe { self.ctx.CopyResource(&staging_res, &tex_res) };

        let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
        unsafe { self.ctx.Map(&staging_res, 0, D3D11_MAP_READ, 0, Some(&mut mapped)) }
            .expect("Map(READ)");
        let stride = (W * 4) as usize;
        let row_pitch = mapped.RowPitch as usize;
        assert!(row_pitch >= stride, "RowPitch < stride（想定外・デバイス異常）");
        let mut dense = vec![0u8; stride * H as usize];
        unsafe {
            let base = mapped.pData as *const u8;
            for y in 0..H as usize {
                let src_row = base.add(y * row_pitch);
                let dst_row = dense.as_mut_ptr().add(y * stride);
                std::ptr::copy_nonoverlapping(src_row, dst_row, stride);
            }
            self.ctx.Unmap(&staging_res, 0);
        }
        dense
    }
}

// ══════════════════════════════════════════════════════════════════════════════════════
// 純粋な比較・計測（byte スライス操作）
// ══════════════════════════════════════════════════════════════════════════════════════

/// 既定 ＭＳ ゴシック 12px の ResolvedFont（font 定義欠落＝ukadoc 既定）。
fn default_font() -> ResolvedFont {
    let model = BalloonModel::new(
        WindowPosition::new(None, None),
        Origin::new(None, None),
        WordWrapPoint::new(None, None),
        ValidRect::new(None, None, None, None),
        Font::new(None, None, FontColor::new(None, None, None)),
        None,
    );
    ResolvedFont::resolve(&model)
}

/// 行内軸の送り総量（cluster 幅合計・image px）。横書き＝x 送り・縦書き＝y 送り。
fn inline_advance(layout: &IDWriteTextLayout) -> f32 {
    layout
        .get_cluster_metrics()
        .expect("GetClusterMetrics")
        .iter()
        .map(|c| c.width)
        .sum()
}

/// α≠0 ピクセルの外接矩形 (min_x,min_y,max_x,max_y)（両端含む・インクなしは None）。
fn ink_bbox(bytes: &[u8]) -> Option<(u32, u32, u32, u32)> {
    let mut bb: Option<(u32, u32, u32, u32)> = None;
    for (i, px) in bytes.chunks_exact(4).enumerate() {
        if px[3] != 0 {
            let (x, y) = (i as u32 % W, i as u32 / W);
            bb = Some(match bb {
                None => (x, y, x, y),
                Some((x0, y0, x1, y1)) => (x0.min(x), y0.min(y), x1.max(x), y1.max(y)),
            });
        }
    }
    bb
}

/// 矩形域 (rx0,ry0..rx1,ry1)（上端 exclusive）内の α≠0 ピクセル数。
fn opaque_count_region(bytes: &[u8], rx0: u32, ry0: u32, rx1: u32, ry1: u32) -> usize {
    let mut n = 0;
    for y in ry0..ry1 {
        for x in rx0..rx1 {
            let idx = ((y * W + x) * 4) as usize;
            if bytes[idx + 3] != 0 {
                n += 1;
            }
        }
    }
    n
}

/// 保持域（矩形 rx0,ry0..rx1,ry1・上端 exclusive）での 2 バッファ byte 比較。
///
/// 返り値 `(mismatch_count, equivalence_guard)`:
/// - `mismatch_count`: 4 バイト（BGRA）いずれかが異なるピクセル数。
/// - `equivalence_guard`: 全不一致を保持域から取り除くのに要する最小の全辺縮小量。
///   不一致なし＝0。不一致があれば `max(不一致の境界最短距離)+1`——中心核まで不一致が
///   及べば大きくなる（＝位相不変の破れ＝NO-GO の指標）。
fn compare_region(
    a: &[u8],
    b: &[u8],
    rx0: u32,
    ry0: u32,
    rx1: u32,
    ry1: u32,
) -> (usize, u32) {
    let mut mismatch = 0usize;
    let mut max_boundary_dist: Option<u32> = None;
    for y in ry0..ry1 {
        for x in rx0..rx1 {
            let idx = ((y * W + x) * 4) as usize;
            if a[idx..idx + 4] != b[idx..idx + 4] {
                mismatch += 1;
                // この不一致が保持域の外周からどれだけ内側にあるか（最短距離）。
                let d = (x - rx0)
                    .min(y - ry0)
                    .min(rx1 - 1 - x)
                    .min(ry1 - 1 - y);
                max_boundary_dist = Some(max_boundary_dist.map_or(d, |m| m.max(d)));
            }
        }
    }
    let guard = match max_boundary_dist {
        None => 0,
        Some(d) => d + 1,
    };
    (mismatch, guard)
}

// ══════════════════════════════════════════════════════════════════════════════════════
// スパイク本体（mode ごと）
// ══════════════════════════════════════════════════════════════════════════════════════

/// mode に応じて 1 行の origin を組む（block 軸へ `block_pos`・行内軸へ `INLINE_START`）。
fn origin_for(mode: WritingMode, block_pos: f32) -> Vector2 {
    match mode {
        // 横書き: 行送り＝y・行内＝x（draw.rs::render の HorizontalTb 分岐と同じ軸割当）。
        WritingMode::HorizontalTb => Vector2 { X: INLINE_START, Y: block_pos },
        // 縦書き: 行送り＝x・行内＝y（同 VerticalRl/Lr 分岐）。
        WritingMode::VerticalRl | WritingMode::VerticalLr => {
            Vector2 { X: block_pos, Y: INLINE_START }
        }
    }
}

/// block 軸へ delta を足した origin（experiment A＝位置 B ＋ blit 量 N の位置）。
fn shift_block(mode: WritingMode, o: Vector2, delta: f32) -> Vector2 {
    match mode {
        WritingMode::HorizontalTb => Vector2 { X: o.X, Y: o.Y + delta },
        WritingMode::VerticalRl | WritingMode::VerticalLr => Vector2 { X: o.X + delta, Y: o.Y },
    }
}

/// mode ごとの blit 等価検証＋ガード実測を実行し、GO/NO-GO を機械判定する。
///
/// 実験 B（direct）: 各行を最終ブロック位置（位置 B）へ直接描画→readback＝`bytes_b`。
/// 実験 Blit: 各行を「位置 B ＋ blit 量 N」（位置 A）へ描画→面内平行移動 N を別テクスチャへ
/// `CopySubresourceRegion`→readback＝`bytes_blit`。
/// 保持域（露出帯を除いた重なり域）で `bytes_b` と `bytes_blit` を byte 比較する。
fn run_mode(mode: WritingMode, label: &str) {
    let spike = Spike::new();
    let font = default_font();

    // 同一 TextLayout を A/B 双方の描画で共有する（design RN5＝byte 等価の構造前提）。
    let layouts: Vec<IDWriteTextLayout> =
        LINES.iter().map(|t| spike.line_layout(t, &font, mode)).collect();

    // 位置 B（最終＝スクロール後）と位置 A（＝B ＋ N・スクロール前）の origin 列。
    // blit 方向は writing_mode 追随（横書き＝縦 blit・下端露出／縦書き＝横 blit）。
    let b_draws: Vec<(Vector2, IDWriteTextLayout)> = BLOCK_POS
        .iter()
        .zip(&layouts)
        .map(|(&bp, layout)| {
            // 位置 B: block 軸を N だけ上流（横書き＝上・縦書き＝左）へ寄せた「スクロール後」。
            let b = shift_block(mode, origin_for(mode, bp), -(N as f32));
            (b, layout.clone())
        })
        .collect();
    let a_draws: Vec<(Vector2, IDWriteTextLayout)> = BLOCK_POS
        .iter()
        .zip(&layouts)
        .map(|(&bp, layout)| (origin_for(mode, bp), layout.clone()))
        .collect();

    let staging = spike.make_staging();

    // 実験 B: 位置 B へ直接描画。
    let tex_b = spike.make_texture();
    spike.draw(&tex_b, &b_draws);
    let bytes_b = spike.readback(&tex_b, &staging);

    // 実験 Blit: 位置 A へ描画→面内平行移動 N を tex_c（透明）へコピー。
    let tex_a = spike.make_texture();
    spike.draw(&tex_a, &a_draws);
    let tex_c = spike.make_texture();
    // 横書き＝上へ N（rows [N,H) → [0,H-N)）／縦書き＝左へ N（cols [N,W) → [0,W-N)）。
    let (srcbox, retained) = match mode {
        WritingMode::HorizontalTb => (
            D3D11_BOX { left: 0, top: N, front: 0, right: W, bottom: H, back: 1 },
            (0u32, 0u32, W, H - N), // 保持域＝露出帯（下端 [H-N,H)）を除く
        ),
        WritingMode::VerticalRl | WritingMode::VerticalLr => (
            D3D11_BOX { left: N, top: 0, front: 0, right: W, bottom: H, back: 1 },
            (0u32, 0u32, W - N, H), // 保持域＝露出帯（右端 [W-N,W)）を除く
        ),
    };
    spike.blit(&tex_a, &tex_c, srcbox, 0, 0);
    let bytes_blit = spike.readback(&tex_c, &staging);

    let (rx0, ry0, rx1, ry1) = retained;

    // 保持域中心核（露出帯以外の端 AA を除いた領域）にインクが存在することを確認
    // ——全透明どうしの空比較で偽 GO にしない。
    let core = (rx0 + CORE_SHRINK, ry0 + CORE_SHRINK, rx1 - CORE_SHRINK, ry1 - CORE_SHRINK);
    let core_ink_b = opaque_count_region(&bytes_b, core.0, core.1, core.2, core.3);
    let core_ink_blit = opaque_count_region(&bytes_blit, core.0, core.1, core.2, core.3);

    // 保持域全域の byte 比較。
    let (mismatch, equivalence_guard) = compare_region(&bytes_b, &bytes_blit, rx0, ry0, rx1, ry1);

    // AA こぼれ実測（DIRTY_GUARD_IMG_PX 候補）。
    let aa_guard = measure_aa_guard(&spike, &font, mode);

    let go = mismatch == 0 || equivalence_guard <= CORE_SHRINK;

    println!("──────── viewbox blit spike [{label}] ────────");
    println!("  面寸(物理px)               : {W}x{H}  blit量 N = {N}");
    println!("  保持域(重なり域)           : x[{rx0},{rx1}) y[{ry0},{ry1})");
    println!("  保持域インク数(B / blit)   : {core_ink_b} / {core_ink_blit}  (中心核 shrink={CORE_SHRINK})");
    println!("  保持域 byte 不一致ピクセル : {mismatch}");
    println!(
        "  等価境界ガード(実効)       : {equivalence_guard} image px  (0=保持域全域が byte 一致)"
    );
    println!(
        "  AA こぼれ(DIRTY_GUARD候補) : left={} top={} right={} bottom={} => 最大 {} image px",
        aa_guard.left, aa_guard.top, aa_guard.right, aa_guard.bottom, aa_guard.max()
    );
    println!(
        "  判定                       : {}  (byte 一致={}, 等価ガード {} <= 核許容 {})",
        if go { "GO" } else { "NO-GO" },
        mismatch == 0,
        equivalence_guard,
        CORE_SHRINK
    );

    // ── GO 機械判定 ──
    assert!(
        core_ink_b > 0 && core_ink_blit > 0,
        "[{label}] 保持域中心核にインクが無い＝比較が空（テスト構成の誤り・偽 GO 防止）"
    );
    assert!(
        equivalence_guard <= CORE_SHRINK,
        "[{label}] 保持域中心核に byte 不一致（等価ガード {equivalence_guard} > 核許容 {CORE_SHRINK}）\
         ＝整数平行移動の blit 結果 ≠ 新位置での DrawTextLayout ＝ ClearType/AA 位相不変仮定の破れ。\
         これは実装バグでなく設計前提崩壊の所見（NO-GO）——本タスクの結果をもって設計再考へ差し戻す。\
         不一致数={mismatch}"
    );
}

/// AA こぼれ計測結果（公称セルの各辺から滲み出た image px）。
struct AaGuard {
    left: u32,
    top: u32,
    right: u32,
    bottom: u32,
}

impl AaGuard {
    fn max(&self) -> u32 {
        self.left.max(self.top).max(self.right).max(self.bottom)
    }
}

/// 1 行を描いて、その行の公称セル（行内 inline_advance × 行送り line_pitch=N）の外側へ
/// α≠0 がどれだけ滲むかを測る（`DIRTY_GUARD_IMG_PX` 候補）。origin は端から充分離す。
fn measure_aa_guard(spike: &Spike, font: &ResolvedFont, mode: WritingMode) -> AaGuard {
    let text = "あWがp"; // 全角＋幅広半角＋濁点＋下方向へ出る 'p'（AA の張り出しを引き出す）
    let layout = spike.line_layout(text, font, mode);
    let inline = inline_advance(&layout);

    let origin = origin_for(mode, 40.0); // block=40・inline=INLINE_START では端に近いので block も 40
    let origin = match mode {
        // 端から離すため inline 開始も 40 へ寄せる（測定専用の余裕確保）。
        WritingMode::HorizontalTb => Vector2 { X: 40.0, Y: origin.Y },
        WritingMode::VerticalRl | WritingMode::VerticalLr => Vector2 { X: origin.X, Y: 40.0 },
    };

    let tex = spike.make_texture();
    spike.draw(&tex, &[(origin, layout)]);
    let staging = spike.make_staging();
    let bytes = spike.readback(&tex, &staging);
    let bbox = ink_bbox(&bytes).expect("計測行にインクが無い");

    // 公称セル（image px・両端 exclusive 側を +? で扱う）: 行内＝[start, start+inline)・
    // 行送り＝[block, block+N)。mode により軸を割り当てる。
    let (cell_x0, cell_y0, cell_x1, cell_y1) = match mode {
        WritingMode::HorizontalTb => {
            let x0 = 40.0f32;
            let y0 = origin.Y;
            (x0, y0, x0 + inline, y0 + N as f32)
        }
        WritingMode::VerticalRl | WritingMode::VerticalLr => {
            let x0 = origin.X;
            let y0 = 40.0f32;
            (x0, y0, x0 + N as f32, y0 + inline)
        }
    };
    // bbox は両端含む int。セル外への滲み出しを各辺で算出（セル内は 0）。
    // 右／下は bbox 上端が含みゆえ +1 して exclusive 端に揃える。
    let (bx0, by0, bx1, by1) = bbox;
    let spill = |diff: i64| -> u32 { diff.max(0) as u32 };
    AaGuard {
        left: spill(cell_x0.round() as i64 - bx0 as i64),
        top: spill(cell_y0.round() as i64 - by0 as i64),
        right: spill((bx1 as i64 + 1) - cell_x1.round() as i64),
        bottom: spill((by1 as i64 + 1) - cell_y1.round() as i64),
    }
}

// ══════════════════════════════════════════════════════════════════════════════════════
// テスト（各 mode が単一 pass/fail・GO の機械判定を持つ）
// ══════════════════════════════════════════════════════════════════════════════════════

/// 横書き（horizontal_tb・縦 blit・下端露出）の blit≡再描画 byte 等価とガード実測。
#[test]
fn blit_equals_redraw_horizontal_tb() {
    run_mode(WritingMode::HorizontalTb, "horizontal_tb");
}

/// 縦書き（vertical_rl・横 blit・右端露出）の blit≡再描画 byte 等価とガード実測。
#[test]
fn blit_equals_redraw_vertical_rl() {
    run_mode(WritingMode::VerticalRl, "vertical_rl");
}

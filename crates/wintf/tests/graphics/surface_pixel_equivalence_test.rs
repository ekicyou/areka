//! タスク 4.1: サーフェス層ビット等価ハーネス（ランタイム二重描画）
//!
//! 要件 8.5 / 8.6・design.md「Testing Strategy › E2E / 描画等価性（R8）」
//! ＞「サーフェス層ビット等価」節（ランタイム二重描画方式）に準拠。
//!
//! # 方式（永続ゴールデンを repo に持たない）
//! 同一の D2D 描画列（代表シーン `draw_scene`）を、テスト実行時にその場で 2 経路へ描く:
//!
//! - **(a) 参照基準**: `GraphicsCore` の D2D デバイスから直接得た `ID2D1DeviceContext` へ
//!   描画する。D2D 描画コードは DComp→WUC 移行で不変ゆえ、この「D2D 直描き基準」は
//!   「移行前サーフェス出力」と論理等価である。
//! - **(b) WUC 経路**: `WucGraphicsResource` の `CompositionDrawingSurface` を
//!   `ICompositionDrawingSurfaceInterop::begin_draw(None)`（**必ず None**・Some は
//!   E_INVALIDARG）で得た `ID2D1DeviceContext3` へ同一列を描く。
//!
//! # 読み戻し（決定論的・WIC 相当のピクセル比較）
//! WUC の `CompositionDrawingSurface` は compositor の atlas テクスチャへ配置され、
//! begin_draw が返す DC のターゲットは atlas である。atlas から直接読み戻すことは
//! できないため、design.md の指針どおり「begin_draw が返す DC 上で、同一 D2D デバイスに
//! 自前の CPU 読み取り可能ステージング `ID2D1Bitmap1`（`CPU_READ|CANNOT_DRAW`）を用意し、
//! そこへ描画結果を `CopyFromBitmap` してから `Map(READ)` で読み戻す」最強手段を採る。
//! (a) も同一の読み戻し経路（自前 TARGET → ステージング → Map）を用い、両者の読み戻し
//! 経路を対称化する（WIC `CopyPixels` と同じく B8G8R8A8 プリマルチプライドの生バイトを比較）。
//!
//! # 何をビット等価検証したか（要件 10.2・ごまかし禁止）
//! - **検証している**: WUC が供給する `ID2D1DeviceContext3` へ描いた代表シーンのピクセル出力が、
//!   D2D デバイスへ直描きした同一シーンのピクセル出力と **全画素バイト等価**であること。
//!   これは「WUC 経路の D2D 描画が移行前と同一結果を出す」ことのビット単位回帰である。
//! - **検証していない**: compositor の atlas 上の最終合成結果そのもの（atlas は直接読み戻せない）。
//!   atlas offset を含む合成層の等価性は別ハーネス（要件 8.7・合成層キャプチャ）の領分。
//!   本テストは begin_draw(None)→有効な `ID2D1DeviceContext3` 取得→同一 D2D 描画列の実行→
//!   end_draw の往復が成立し、その DC が生む画素が参照基準とビット等価であることを担保する。

use windows::Foundation::Size;
use windows::Graphics::DirectX::{DirectXAlphaMode, DirectXPixelFormat};
use windows::Win32::Graphics::Direct2D::Common::{
    D2D_RECT_F, D2D_SIZE_U, D2D1_ALPHA_MODE_PREMULTIPLIED, D2D1_COLOR_F, D2D1_PIXEL_FORMAT,
};
use windows::Win32::Graphics::Direct2D::{
    D2D1_BITMAP_OPTIONS_CANNOT_DRAW, D2D1_BITMAP_OPTIONS_CPU_READ, D2D1_BITMAP_OPTIONS_TARGET,
    D2D1_BITMAP_PROPERTIES1, D2D1_DEVICE_CONTEXT_OPTIONS_NONE, D2D1_MAP_OPTIONS_READ, ID2D1Bitmap1,
    ID2D1DeviceContext, ID2D1DeviceContext3,
};
use windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM;
use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx};
use windows::Win32::System::WinRT::Composition::ICompositionDrawingSurfaceInterop;
use windows::core::{Interface, Result};
use wintf::com::d2d::{D2D1DeviceContextExt, D2D1DeviceExt};
use wintf::com::wuc::DrawingSurfaceInteropExt;
use wintf::ecs::{GraphicsCore, WucGraphicsResource};

const W: u32 = 128;
const H: u32 = 128;

/// COM を MTA 初期化する（`WucGraphicsResource::new` は `DQTAT_COM_NONE` を使うため
/// COM 初期化済みスレッドを要求する）。冪等で、S_FALSE / RPC_E_CHANGED_MODE は無視する。
fn init_com_mta() {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
    }
}

/// 代表シーン（単純だが非自明な D2D 描画）。
///
/// 両経路に同一関数で適用する。DC は BeginDraw 済みの状態で渡され、ターゲットは
/// 呼び出し側が設定済み。背景クリア → 複数色の矩形塗り → 半透明矩形の重ね → 線（細矩形）。
fn draw_scene(dc: &ID2D1DeviceContext) -> Result<()> {
    // 背景を不透明の濃紺でクリア（透明ではなく決定論的な既知色）。
    dc.clear(Some(&D2D1_COLOR_F {
        r: 0.05,
        g: 0.10,
        b: 0.20,
        a: 1.0,
    }));

    // 不透明の赤矩形（左上）。
    let red = dc.create_solid_color_brush(
        &D2D1_COLOR_F {
            r: 0.90,
            g: 0.10,
            b: 0.15,
            a: 1.0,
        },
        None,
    )?;
    dc.fill_rectangle(
        &D2D_RECT_F {
            left: 8.0,
            top: 8.0,
            right: 72.0,
            bottom: 72.0,
        },
        &red,
    );

    // 不透明の緑矩形（右下・赤と一部重ならない位置）。
    let green = dc.create_solid_color_brush(
        &D2D1_COLOR_F {
            r: 0.10,
            g: 0.80,
            b: 0.25,
            a: 1.0,
        },
        None,
    )?;
    dc.fill_rectangle(
        &D2D_RECT_F {
            left: 56.0,
            top: 56.0,
            right: 120.0,
            bottom: 120.0,
        },
        &green,
    );

    // 半透明の青矩形を中央に重ねる（source-over ブレンドの決定論を検証）。
    let blue_semi = dc.create_solid_color_brush(
        &D2D1_COLOR_F {
            r: 0.15,
            g: 0.35,
            b: 0.95,
            a: 0.5,
        },
        None,
    )?;
    dc.fill_rectangle(
        &D2D_RECT_F {
            left: 32.0,
            top: 32.0,
            right: 96.0,
            bottom: 96.0,
        },
        &blue_semi,
    );

    // 細い横棒（線相当）を白で描く。
    let white = dc.create_solid_color_brush(
        &D2D1_COLOR_F {
            r: 1.0,
            g: 1.0,
            b: 1.0,
            a: 1.0,
        },
        None,
    )?;
    dc.fill_rectangle(
        &D2D_RECT_F {
            left: 4.0,
            top: 100.0,
            right: 124.0,
            bottom: 104.0,
        },
        &white,
    );

    Ok(())
}

/// B8G8R8A8 プリマルチプライドの描画可能 TARGET ビットマップを生成する。
fn create_target_bitmap(dc: &ID2D1DeviceContext) -> Result<ID2D1Bitmap1> {
    let props = D2D1_BITMAP_PROPERTIES1 {
        pixelFormat: D2D1_PIXEL_FORMAT {
            format: DXGI_FORMAT_B8G8R8A8_UNORM,
            alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
        },
        dpiX: 96.0,
        dpiY: 96.0,
        bitmapOptions: D2D1_BITMAP_OPTIONS_TARGET,
        colorContext: core::mem::ManuallyDrop::new(None),
    };
    unsafe {
        dc.CreateBitmap(
            D2D_SIZE_U {
                width: W,
                height: H,
            },
            None,
            0,
            &props,
        )
    }
}

/// CPU 読み取り可能なステージングビットマップ（`CPU_READ|CANNOT_DRAW`）を生成する。
fn create_readback_bitmap(dc: &ID2D1DeviceContext) -> Result<ID2D1Bitmap1> {
    let props = D2D1_BITMAP_PROPERTIES1 {
        pixelFormat: D2D1_PIXEL_FORMAT {
            format: DXGI_FORMAT_B8G8R8A8_UNORM,
            alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
        },
        dpiX: 96.0,
        dpiY: 96.0,
        bitmapOptions: D2D1_BITMAP_OPTIONS_CPU_READ | D2D1_BITMAP_OPTIONS_CANNOT_DRAW,
        colorContext: core::mem::ManuallyDrop::new(None),
    };
    unsafe {
        dc.CreateBitmap(
            D2D_SIZE_U {
                width: W,
                height: H,
            },
            None,
            0,
            &props,
        )
    }
}

/// TARGET ビットマップの内容を CPU 読み取り可能ステージングへコピーし、行を詰めた
/// `W*H*4` バイトの BGRA 生バイト列として読み戻す（stride を除去して純ピクセル配列に整える）。
fn readback_pixels(dc: &ID2D1DeviceContext, target: &ID2D1Bitmap1) -> Result<Vec<u8>> {
    let staging = create_readback_bitmap(dc)?;
    // TARGET → ステージング（全面コピー）。
    unsafe { staging.CopyFromBitmap(None, target, None)? };

    let mapped = unsafe { staging.Map(D2D1_MAP_OPTIONS_READ)? };
    let stride = mapped.pitch as usize;
    let row_bytes = (W as usize) * 4;
    let mut out = vec![0u8; row_bytes * (H as usize)];
    // SAFETY: mapped.bits は少なくとも stride*H バイトの読み取り可能領域を指す
    // （D2D1_MAP_OPTIONS_READ・CPU_READ ビットマップ）。行ごとに詰めてコピーする。
    unsafe {
        for y in 0..(H as usize) {
            let src = mapped.bits.add(y * stride);
            let dst = out.as_mut_ptr().add(y * row_bytes);
            core::ptr::copy_nonoverlapping(src, dst, row_bytes);
        }
    }
    unsafe { staging.Unmap()? };
    Ok(out)
}

/// (a) 参照基準: D2D デバイスから直接得た DC へ代表シーンを描き、ピクセルを読み戻す。
fn render_reference(core: &GraphicsCore) -> Result<Vec<u8>> {
    let device = core.d2d_device().expect("d2d device");
    let dc = device.create_device_context(D2D1_DEVICE_CONTEXT_OPTIONS_NONE)?;
    let target = create_target_bitmap(&dc)?;
    unsafe {
        dc.SetTarget(&target);
        dc.BeginDraw();
    }
    draw_scene(&dc)?;
    unsafe { dc.EndDraw(None, None)? };
    readback_pixels(&dc, &target)
}

/// (b) WUC 経路: `CompositionDrawingSurface` の begin_draw(None) が返す
/// `ID2D1DeviceContext3` へ同一シーンを描き、同一読み戻し経路でピクセルを得る。
///
/// atlas は直接読み戻せないため、WUC 供給 DC 上に自前 TARGET を SetTarget して描き、
/// その TARGET を CPU 読み取りステージングへコピーして読み戻す（design.md 指針）。
fn render_wuc(resource: &WucGraphicsResource) -> Result<Vec<u8>> {
    let gd = resource.graphics_device().expect("graphics_device");
    // begin_draw(None) が有効な atlas を返すよう、シーンより大きめのサーフェスを確保。
    let surface = gd.CreateDrawingSurface(
        Size {
            Width: W as f32,
            Height: H as f32,
        },
        DirectXPixelFormat::B8G8R8A8UIntNormalized,
        DirectXAlphaMode::Premultiplied,
    )?;
    let surface_interop = surface.cast::<ICompositionDrawingSurfaceInterop>()?;

    // WUC が供給する DC（必ず None）。offset は atlas 内配置（本テストでは自前 TARGET を
    // 使うため offset は消費しないが、begin_draw/end_draw の往復と DC 有効性は担保する）。
    let (dc3, _offset): (ID2D1DeviceContext3, _) = surface_interop.begin_draw(None)?;
    // ID2D1DeviceContext3 は ID2D1DeviceContext を包含する階層。DC として扱う。
    let dc: ID2D1DeviceContext = dc3.cast()?;

    // begin_draw が返す DC は既に描画状態（atlas を BeginDraw 済み）にある。ここで
    // 再度 BeginDraw を呼ぶと D2DERR_WRONG_STATE になるため、WUC のブラケットを流用する。
    // 自前 TARGET へ SetTarget して同一シーンを描き、TARGET を CPU ステージングへコピーする。
    let target = create_target_bitmap(&dc)?;
    unsafe { dc.SetTarget(&target) };
    draw_scene(&dc)?;
    // Flush して描画命令を TARGET へ確定させてから読み戻す（EndDraw は WUC の end_draw が担う）。
    unsafe { dc.Flush(None, None)? };

    // DC がまだ有効なうちに読み戻す（TARGET → ステージング → Map）。
    let pixels = readback_pixels(&dc, &target)?;

    // atlas ターゲットへ戻してから surface の end_draw（WUC のブラケットを閉じる）。
    unsafe { dc.SetTarget(None) };
    surface_interop.end_draw()?;

    Ok(pixels)
}

/// 代表シーンのサーフェス層ビット等価検証（要件 8.6）。
///
/// (a) D2D 直描き基準と (b) WUC begin_draw 供給 DC の描画結果が **全画素バイト等価**
/// であることを assert する。差分があれば最初の不一致画素座標と両者の BGRA を報告する。
#[test]
fn surface_pixel_equivalence_reference_vs_wuc() {
    crate::common::on_gpu_owner_thread(move || {
        init_com_mta();

        let core = GraphicsCore::new().expect("GraphicsCore::new 失敗（HARDWARE デバイス生成）");
        let d2d = core.d2d_device().expect("d2d device");

        // teardown レース対策: WucGraphicsResource 構築後は warmup サーフェスを 1 枚保持して
        // DispatcherQueue を安定化する（tests/visual/common/mod.rs の WucVisualFactory と同一 pattern）。
        let resource = WucGraphicsResource::new(d2d).expect("WucGraphicsResource::new 失敗");
        let warmup_surface = {
            let gd = resource.graphics_device().expect("graphics_device");
            gd.CreateDrawingSurface(
                Size {
                    Width: 1.0,
                    Height: 1.0,
                },
                DirectXPixelFormat::B8G8R8A8UIntNormalized,
                DirectXAlphaMode::Premultiplied,
            )
            .expect("warmup surface")
        };

        let reference = render_reference(&core).expect("(a) 参照基準の描画/読み戻し失敗");
        let wuc = render_wuc(&resource).expect("(b) WUC 経路の描画/読み戻し失敗");

        assert_eq!(
            reference.len(),
            wuc.len(),
            "読み戻しバイト長が一致しない: ref={} wuc={}",
            reference.len(),
            wuc.len()
        );
        assert_eq!(
            reference.len(),
            (W as usize) * (H as usize) * 4,
            "読み戻しバイト長が W*H*4 と一致しない"
        );

        // 自己検証（ごまかしの空 assert でないことの担保）: 参照基準に既知色が実在すること。
        // 背景（濃紺）ピクセル (0,0) と白棒 (64,102) が期待色であることを確認する。
        let px = |buf: &[u8], x: u32, y: u32| -> [u8; 4] {
            let i = ((y * W + x) * 4) as usize;
            [buf[i], buf[i + 1], buf[i + 2], buf[i + 3]]
        };
        let bg = px(&reference, 0, 0);
        // BGRA バイト順: [0]=B, [1]=G, [2]=R, [3]=A。濃紺は B > R かつ不透明。
        assert!(
            bg[3] == 255 && bg[0] > bg[2],
            "参照基準の背景が期待の不透明濃紺でない: BGRA={:?}",
            bg
        );
        let bar = px(&reference, 64, 102);
        assert!(
            bar == [255, 255, 255, 255],
            "参照基準の白棒 (64,102) が白でない: BGRA={:?}",
            bar
        );

        // 全画素ビット等価判定（差分ゼロ）。
        if reference != wuc {
            // 最初の不一致画素を特定して報告（ごまかさない・要件 10.2）。
            let mut first_mismatch = None;
            let mut mismatch_count = 0usize;
            for i in (0..reference.len()).step_by(4) {
                if reference[i..i + 4] != wuc[i..i + 4] {
                    mismatch_count += 1;
                    if first_mismatch.is_none() {
                        let pixel = i / 4;
                        let x = (pixel as u32) % W;
                        let y = (pixel as u32) / W;
                        first_mismatch = Some((
                            x,
                            y,
                            [
                                reference[i],
                                reference[i + 1],
                                reference[i + 2],
                                reference[i + 3],
                            ],
                            [wuc[i], wuc[i + 1], wuc[i + 2], wuc[i + 3]],
                        ));
                    }
                }
            }
            panic!(
                "サーフェス層ビット等価に失敗: 不一致画素数={} / 総画素数={} 最初の不一致={:?} (x,y,ref_BGRA,wuc_BGRA)",
                mismatch_count,
                (W * H),
                first_mismatch
            );
        }

        // warmup surface は DispatcherQueue 安定化のためテスト末尾まで保持する。
        let _ = warmup_surface;
        eprintln!(
            "[surface_pixel_equivalence] PASS: {}x{} 全 {} 画素がビット等価",
            W,
            H,
            W * H
        );
    });
}

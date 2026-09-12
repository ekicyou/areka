//! 供給の成功経路の檻（T-G1・GPU）: 恒等変換のオフスクリーン往復 golden。
//!
//! [`record_display`] が返した**閉じた**コマンドリストを、恒等変換（k=1）でオフスクリーンの
//! D2D ターゲット（透明クリア）へ再生し、CPU 読み取り用 bitmap へ複写して読み戻したバイト列が
//! [`ComposedSurface::bytes`]（原寸・premultiplied BGRA）と**バイト単位で一致**することを固定する。
//!
//! この 1 本で成功経路の 5 点——`CreateBitmap` の寸・pitch・premultiplied・宛先矩形・`Close`——が
//! まとめて檻に入る。撤去する自前供給面の往復テスト（`chain.rs` の
//! `upload_read_back_roundtrip_and_resize`）の正当な後継である。
//!
//! バイト一致が成り立つ条件は k=1・整数矩形・LINEAR が texel 中心で恒等・透明地への
//! SOURCE_OVER が src そのもの、の 4 点。DC の既定 DPI だけは実行機に左右されるため、記録にも
//! 再生にも使う DC を 96 に固定する（宛先矩形は論理 px ゆえ、96 でのみ 1 論理 px = 1 画素になる）。
//!
//! 読み戻しの型は wintf `tests/graphics/surface_pixel_equivalence_test.rs` と同じ
//! （`CPU_READ|CANNOT_DRAW` の staging へ `CopyFromBitmap` → `Map(READ)` → pitch を踏まえて行複写）。

use super::*;

use std::path::Path;

use areka_emo_atlas::{
    AlphaParams, MemoryDecoder, PackConfig, SetId, SurfaceSet, UseSelfAlpha, bake,
};
use areka_emo_compose::{BindSet, Composer, EmoWorld, PatternState};
use areka_parsers::shell::{AppendTarget, DefRef, Element, ElementPath, Shell, Surface};

use windows::Win32::Graphics::Direct2D::Common::{D2D1_COLOR_F, D2D1_COMPOSITE_MODE_SOURCE_OVER};
use windows::Win32::Graphics::Direct2D::{
    D2D1_BITMAP_OPTIONS, D2D1_BITMAP_OPTIONS_CANNOT_DRAW, D2D1_BITMAP_OPTIONS_CPU_READ,
    D2D1_BITMAP_OPTIONS_TARGET, D2D1_MAP_OPTIONS_READ, ID2D1Bitmap1,
};
use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx};
use wintf::ecs::GraphicsCore;

/// 実 GPU（HARDWARE デバイス）を組み、DPI を 96 に固定した共有 DC を返す。
///
/// `GraphicsCore` を戻り値に含めるのは DC の寿命がそれに縛られるため。GPU が無ければ
/// `expect` で赤くなる（黙って緑にする skip は置かない）。
fn gpu_dc() -> (GraphicsCore, ID2D1DeviceContext) {
    // 各テストは専用スレッドで走る。MTA を初期化（S_FALSE/RPC_E_CHANGED_MODE は無視）。
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
    }
    let core = GraphicsCore::new().expect("GraphicsCore::new 失敗（HARDWARE デバイス生成）");
    let dc = core
        .device_context()
        .expect("GraphicsCore::device_context が None")
        .clone();
    // SAFETY: Windows API 境界。生成直後の専有 DC に対する設定変更。
    unsafe { dc.SetDpi(96.0, 96.0) };
    (core, dc)
}

/// α に 0・中間・255 を混ぜた premultiplied BGRA を、上流の公開 API（atlas bake →
/// `EmoWorld` → `Composer::compose`）で本物合成する。
///
/// `ComposedSurface::bytes_mut` は emo-compose の `pub(crate)` ゆえ画素を直接焼けない。
/// 各成分は straight 色に α を掛けて作るので premultiplied 不変（成分 ≤ α）を自明に満たす。
fn composed_with_alpha_edges(w: u32, h: u32) -> ComposedSurface {
    let base = Path::new("shell/master");
    let surfaces = vec![Surface {
        id: 1000,
        targets: vec![AppendTarget::Single(1000)],
        elements: vec![Element {
            layer: 0,
            path: ElementPath::new("p.png".to_string()),
            x: 0,
            y: 0,
        }],
        collisions: Vec::new(),
        animations: Vec::new(),
    }];

    let stride = w * 4;
    let mut img: Vec<u8> = Vec::with_capacity((stride * h) as usize);
    for y in 0..h {
        for x in 0..w {
            let a: u8 = match (x + y) % 4 {
                0 => 0,
                1 => 64,
                2 => 160,
                _ => 255,
            };
            let pm = |v: u8| ((v as u16 * a as u16) / 255) as u8;
            img.push(pm((x as u8).wrapping_mul(37).wrapping_add(11)));
            img.push(pm((y as u8).wrapping_mul(53).wrapping_add(29)));
            img.push(pm(((x + y) as u8).wrapping_mul(71).wrapping_add(7)));
            img.push(a);
        }
    }
    let mut dec = MemoryDecoder::new();
    dec.insert(base.join("p.png"), w, h, stride, img, true);

    let set = SurfaceSet {
        surfaces: &surfaces,
        base_dir: base,
        alpha_params: AlphaParams {
            use_self_alpha: UseSelfAlpha::On,
        },
    };
    let baked = bake(&[set], &dec, PackConfig::default());
    assert!(
        baked.errors.is_empty(),
        "atlas bake セットアップは失敗しない: {:?}",
        baked.errors
    );

    let definitions = (0..surfaces.len()).map(DefRef::Surface).collect();
    let shell = Shell {
        surfaces,
        appends: Vec::new(),
        aliases: Vec::new(),
        animation_sort: None,
        collision_sort: None,
        definitions,
    };
    let mut world = EmoWorld::build(&shell);
    world.bind_atlas(&baked.table, SetId(0));

    Composer::new()
        .compose(
            &world,
            &baked.table,
            1000,
            &BindSet::default(),
            &PatternState::default(),
        )
        .expect("静的 element 単体の合成は Ok")
}

/// `w×h`・premultiplied BGRA の bitmap を作る（`options` で TARGET／staging を切り替える）。
fn make_bitmap(
    dc: &ID2D1DeviceContext,
    w: u32,
    h: u32,
    options: D2D1_BITMAP_OPTIONS,
) -> ID2D1Bitmap1 {
    let props = D2D1_BITMAP_PROPERTIES1 {
        pixelFormat: D2D1_PIXEL_FORMAT {
            format: DXGI_FORMAT_B8G8R8A8_UNORM,
            alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
        },
        dpiX: 96.0,
        dpiY: 96.0,
        bitmapOptions: options,
        colorContext: ManuallyDrop::new(None::<ID2D1ColorContext>),
    };
    // SAFETY: Windows API 境界。初期バイト列を渡さない空 bitmap の生成。
    unsafe {
        dc.CreateBitmap(
            D2D_SIZE_U {
                width: w,
                height: h,
            },
            None,
            0,
            &props,
        )
    }
    .expect("オフスクリーン bitmap の生成に失敗")
}

/// 閉じたコマンドリストを恒等変換で `w×h` のオフスクリーンへ再生し、行を詰めて読み戻す。
///
/// 再生の引数は本番 `render_surface`（wintf `ecs/graphics/systems/render.rs`）と同一
/// （透明クリア → `DrawImage(list, None, None, LINEAR, SOURCE_OVER)`）。
fn replay_and_read_back(
    dc: &ID2D1DeviceContext,
    list: &GraphicsCommandList,
    w: u32,
    h: u32,
) -> Vec<u8> {
    let target = make_bitmap(dc, w, h, D2D1_BITMAP_OPTIONS_TARGET);
    // SAFETY: Windows API 境界。ターゲット差し替えと描画（本スレッド直列）。
    unsafe { dc.SetTarget(&target) };
    dc.set_transform(&Matrix3x2::identity());
    // SAFETY: Windows API 境界。`BeginDraw`／`EndDraw` は本関数内で対になる。
    unsafe { dc.BeginDraw() };
    dc.clear(Some(&D2D1_COLOR_F {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 0.0,
    }));
    // SAFETY: Windows API 境界。`render_surface` と同一引数のコマンドリスト再生。
    unsafe {
        dc.DrawImage(
            list.command_list().expect("閉じたリストを持つ"),
            None,
            None,
            D2D1_INTERPOLATION_MODE_LINEAR,
            D2D1_COMPOSITE_MODE_SOURCE_OVER,
        );
        dc.EndDraw(None, None).expect("EndDraw（再生）失敗");
    }

    let staging = make_bitmap(
        dc,
        w,
        h,
        D2D1_BITMAP_OPTIONS_CPU_READ | D2D1_BITMAP_OPTIONS_CANNOT_DRAW,
    );
    let row = (w as usize) * 4;
    let mut out = vec![0u8; row * (h as usize)];
    // SAFETY: Windows API 境界。`Map(READ)` が返す `bits` は少なくとも `pitch * h` バイトの
    // 読み取り可能領域を指す。行ごとに `pitch` を跨いで詰め直す。
    unsafe {
        staging
            .CopyFromBitmap(None, &target, None)
            .expect("CopyFromBitmap（TARGET → staging）失敗");
        let mapped = staging.Map(D2D1_MAP_OPTIONS_READ).expect("Map(READ) 失敗");
        for y in 0..(h as usize) {
            std::ptr::copy_nonoverlapping(
                mapped.bits.add(y * mapped.pitch as usize),
                out.as_mut_ptr().add(y * row),
                row,
            );
        }
        staging.Unmap().expect("Unmap 失敗");
        // 共有 DC に自前ターゲットを残さない。
        dc.SetTarget(None);
    }
    out
}

/// T-G1: 恒等変換の往復が合成結果の原寸バイトと**バイト単位で一致**する。
#[test]
fn identity_offscreen_roundtrip_equals_composed_native_bytes() {
    let (_core, dc) = gpu_dc();
    let surface = composed_with_alpha_edges(7, 5);
    let (w, h) = (surface.width(), surface.height());
    let expected = surface.bytes();

    // 檻の前提（fixture が退化していれば一致は無意味）: 完全透明・半透明・不透明が同居し、
    // stride は密（w*4）である。
    assert_eq!(
        surface.stride(),
        w * 4,
        "合成結果は密配列（stride = w*4）であるべき"
    );
    let alphas: Vec<u8> = expected.iter().skip(3).step_by(4).copied().collect();
    assert!(
        alphas.contains(&0) && alphas.iter().any(|&a| a != 0 && a != 255),
        "fixture 前提: α=0 と中間 α が同居しなければ premultiplied の檻にならない"
    );

    let list = record_display(&dc, &surface).expect("record_display は Ok");
    let read_back = replay_and_read_back(&dc, &list, w, h);

    assert_eq!(
        read_back.len(),
        expected.len(),
        "読み戻し長が原寸バイト長と一致しない: {w}x{h}"
    );
    let bad = (0..expected.len())
        .step_by(4)
        .find(|&i| read_back[i..i + 4] != expected[i..i + 4]);
    assert!(
        bad.is_none(),
        "恒等往復が原寸バイトと一致しない: 最初の不一致画素={:?} 期待 BGRA={:?} 実測 BGRA={:?}",
        bad.map(|i| ((i / 4) as u32 % w, (i / 4) as u32 / w)),
        bad.map(|i| &expected[i..i + 4]),
        bad.map(|i| &read_back[i..i + 4])
    );
}

/// 成功側の最低線: `record_display` は中身を持つ（空でない）コマンドリストを返す。
#[test]
fn record_display_returns_a_non_empty_command_list() {
    let (_core, dc) = gpu_dc();
    let surface = composed_with_alpha_edges(4, 3);

    let list = record_display(&dc, &surface).expect("record_display は Ok");

    assert!(
        list.command_list().is_some(),
        "返ったリストが空（command_list() が None）"
    );
    assert_ne!(
        list,
        GraphicsCommandList::empty(),
        "返ったリストが GraphicsCommandList::empty() と等しい"
    );
}

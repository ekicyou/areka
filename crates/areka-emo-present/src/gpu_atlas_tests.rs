use super::*;

use std::sync::Arc;

use areka_emo_atlas::{
    AtlasEntry, AtlasKey, AtlasPage, PictureKind, Placement, Point, Rect, SetId, Size,
};
use log_capture_kit::capture;
use windows::Win32::Foundation::HMODULE;
use windows::Win32::Graphics::Direct2D::{
    D2D1_FACTORY_TYPE_MULTI_THREADED, D2D1CreateFactory, ID2D1Factory1,
};
use windows::Win32::Graphics::Direct3D::D3D_DRIVER_TYPE_WARP;
use windows::Win32::Graphics::Direct3D11::{D3D11_CREATE_DEVICE_BGRA_SUPPORT, D3D11_SDK_VERSION};
use windows::Win32::Graphics::Dxgi::IDXGIDevice;
use windows::core::Interface;
use wintf::com::d3d11::d3d11_create_device;
use wintf::ecs::{GraphicsCommandList, warp_d2d_device};

use crate::display::gpu_tests::replay_and_read_back;

const PAGE_W: u32 = 6;
const PAGE_H: u32 = 4;

/// 頁の (x, y) の画素（乗算済みで不透明）。
fn page_pixel(x: u32, y: u32) -> [u8; 4] {
    [(x * 40) as u8, (y * 60) as u8, 200, 255]
}

/// 6×4 の頁 1 枚に 2 つの絵を載せた表。
/// 絵 0: 頁の (0,0) 2×2 を原寸 4×3 の (1,1) へ。絵 1: 頁の (3,1) 3×3 を原寸 5×5 の (2,0) へ。
pub(crate) fn two_picture_atlas() -> AtlasTable {
    let mut bytes = Vec::new();
    for y in 0..PAGE_H {
        for x in 0..PAGE_W {
            bytes.extend_from_slice(&page_pixel(x, y));
        }
    }
    let key = |p: &str| AtlasKey {
        set: SetId(0),
        rel_path: p.into(),
        kind: PictureKind::Normal,
    };
    let entry = |w, h, uv: Rect, x, y| AtlasEntry {
        original: Size { w, h },
        placement: Some(Placement {
            page: 0,
            uv_rect: uv,
            trim_offset: Point { x, y },
        }),
    };
    AtlasTable::new(
        vec![key("a.png"), key("b.png")],
        vec![
            entry(
                4,
                3,
                Rect {
                    x: 0,
                    y: 0,
                    w: 2,
                    h: 2,
                },
                1,
                1,
            ),
            entry(
                5,
                5,
                Rect {
                    x: 3,
                    y: 1,
                    w: 3,
                    h: 3,
                },
                2,
                0,
            ),
        ],
        vec![AtlasPage {
            width: PAGE_W,
            height: PAGE_H,
            stride: PAGE_W * 4,
            bytes: Arc::from(bytes),
        }],
    )
}

pub(crate) fn warp() -> ID2D1Device {
    warp_d2d_device().expect("WARP の装置")
}

/// プロセスで共有の WARP とは別の、もう 1 つの WARP の装置（装置の替わりを作る）。
pub(crate) fn another_warp() -> ID2D1Device {
    let d3d = d3d11_create_device(
        None,
        D3D_DRIVER_TYPE_WARP,
        HMODULE::default(),
        D3D11_CREATE_DEVICE_BGRA_SUPPORT,
        None,
        D3D11_SDK_VERSION,
        None,
        None,
    )
    .expect("WARP の D3D11");
    let dxgi: IDXGIDevice = d3d.cast().expect("IDXGIDevice");
    // SAFETY: Windows API 境界。工場を作り、その上に装置を作るだけ。
    unsafe {
        let factory: ID2D1Factory1 =
            D2D1CreateFactory(D2D1_FACTORY_TYPE_MULTI_THREADED, None).expect("工場");
        factory.CreateDevice(&dxgi).expect("D2D の装置")
    }
}

fn read_back(device: &ID2D1Device, list: &ID2D1CommandList, w: u32, h: u32) -> Vec<u8> {
    let dc = device
        .create_device_context(D2D1_DEVICE_CONTEXT_OPTIONS_NONE)
        .expect("DC");
    // SAFETY: Windows API 境界。作ったばかりの専有の DC の設定。
    unsafe { dc.SetDpi(96.0, 96.0) };
    replay_and_read_back(&dc, &GraphicsCommandList::new(list.clone()), w, h)
}

/// `w×h` の原寸のうち `(ox, oy)` から頁の `uv` を写した絵。外は透明。
fn expected_leaf(w: u32, h: u32, uv: Rect, ox: u32, oy: u32) -> Vec<u8> {
    let mut out = vec![0u8; (w * h * 4) as usize];
    for y in 0..uv.h {
        for x in 0..uv.w {
            let i = (((oy + y) * w + ox + x) * 4) as usize;
            out[i..i + 4].copy_from_slice(&page_pixel(uv.x + x, uv.y + y));
        }
    }
    out
}

#[test]
fn same_leaf_twice_uploads_and_records_once() {
    let mut gpu = GpuAtlas::new(&warp(), &two_picture_atlas()).expect("GpuAtlas");
    let first = gpu.leaf(ElementId(1)).expect("葉").as_raw();
    let second = gpu.leaf(ElementId(1)).expect("葉").as_raw();

    assert_eq!(first, second, "同じ絵の葉は控えたものを返す");
    assert_eq!(gpu.uploads(), 1, "頁は 1 枚なので上げは 1 度");
    assert_eq!(gpu.recordings(), 1, "同じ絵の葉の記録は 1 度");

    gpu.leaf(ElementId(0)).expect("葉");
    assert_eq!(gpu.uploads(), 1, "別の絵の葉でも頁は上げ直さない");
    assert_eq!(gpu.recordings(), 2, "別の絵の葉は別に記録する");
}

#[test]
fn leaf_draws_page_region_at_trim_offset_in_original_size() {
    let device = warp();
    let mut gpu = GpuAtlas::new(&device, &two_picture_atlas()).expect("GpuAtlas");

    let a = gpu.leaf(ElementId(0)).expect("葉").clone();
    let b = gpu.leaf(ElementId(1)).expect("葉").clone();

    assert_eq!(
        read_back(&device, &a, 4, 3),
        expected_leaf(
            4,
            3,
            Rect {
                x: 0,
                y: 0,
                w: 2,
                h: 2
            },
            1,
            1
        )
    );
    assert_eq!(
        read_back(&device, &b, 5, 5),
        expected_leaf(
            5,
            5,
            Rect {
                x: 3,
                y: 1,
                w: 3,
                h: 3
            },
            2,
            0
        )
    );
}

#[test]
fn another_device_rebuilds_pages_and_leaves() {
    let atlas = two_picture_atlas();
    let mut old = GpuAtlas::new(&warp(), &atlas).expect("GpuAtlas");
    let old_leaf = old.leaf(ElementId(1)).expect("葉").as_raw();

    let device = another_warp();
    let mut new = GpuAtlas::new(&device, &atlas).expect("GpuAtlas");
    assert_eq!(new.device().as_raw(), device.as_raw());
    assert_ne!(new.device().as_raw(), old.device().as_raw());
    assert_eq!(new.recordings(), 0, "新しい装置では葉はまだ無い");

    let leaf = new.leaf(ElementId(1)).expect("葉").clone();
    assert_ne!(leaf.as_raw(), old_leaf, "装置を替えると葉は作り直す");
    assert_eq!((new.uploads(), new.recordings()), (1, 1));
    assert_eq!(
        read_back(&device, &leaf, 5, 5),
        expected_leaf(
            5,
            5,
            Rect {
                x: 3,
                y: 1,
                w: 3,
                h: 3
            },
            2,
            0
        ),
        "作り直した葉も同じ絵を描く"
    );
}

#[test]
fn black_fill_paints_opaque_black_inside_rect_only() {
    let device = warp();
    let mut gpu = GpuAtlas::new(&device, &two_picture_atlas()).expect("GpuAtlas");
    let list = gpu
        .black_fill(IRect {
            x: 1,
            y: 2,
            w: 3,
            h: 1,
        })
        .expect("黒の塗り");

    let got = read_back(&device, &list, 5, 4);
    for y in 0..4u32 {
        for x in 0..5u32 {
            let i = ((y * 5 + x) * 4) as usize;
            let inside = (1..4).contains(&x) && y == 2;
            let want = if inside { [0, 0, 0, 255] } else { [0; 4] };
            assert_eq!(got[i..i + 4], want, "({x},{y})");
        }
    }
}

#[test]
fn page_upload_failure_logs_error_and_returns_device_error() {
    // 行の幅に足りないバイト列の頁は上げない。
    let atlas = AtlasTable::new(
        Vec::new(),
        Vec::new(),
        vec![AtlasPage {
            width: 4,
            height: 2,
            stride: 16,
            bytes: Arc::from(vec![0u8; 16]),
        }],
    );

    let (result, events) = capture(|| GpuAtlas::new(&warp(), &atlas));

    match result {
        Err(PresentError::Device { context, .. }) => {
            assert_eq!(context, "CreateBitmap(atlas page: short bytes)")
        }
        Err(other) => panic!("装置の失敗のはずが {other:?}"),
        Ok(_) => panic!("上げられない頁で Ok が返った"),
    }
    let errors = events
        .iter()
        .filter(|e| e.level == tracing::Level::ERROR)
        .count();
    assert_eq!(errors, 1, "失敗は error! を 1 行残す");
}

/// 頁の無い置き場・頁をはみ出す uv の絵は、葉を空で記録して `warn!`（絵と頁の番号つき）を残す（パニックしない）。
#[test]
fn leaf_of_a_picture_outside_its_page_is_empty_and_warns() {
    let entry = |page, uv| AtlasEntry {
        original: Size { w: 2, h: 2 },
        placement: Some(Placement {
            page,
            uv_rect: uv,
            trim_offset: Point { x: 0, y: 0 },
        }),
    };
    let key = |p: &str| AtlasKey {
        set: SetId(0),
        rel_path: p.into(),
        kind: PictureKind::Normal,
    };
    let page = AtlasPage {
        width: 2,
        height: 2,
        stride: 8,
        bytes: Arc::from(vec![255u8; 16]),
    };
    let atlas = AtlasTable::new(
        vec![key("a.png"), key("b.png")],
        vec![
            entry(
                5,
                Rect {
                    x: 0,
                    y: 0,
                    w: 2,
                    h: 2,
                },
            ),
            entry(
                0,
                Rect {
                    x: 1,
                    y: 0,
                    w: 2,
                    h: 2,
                },
            ),
        ],
        vec![page],
    );
    let device = warp();
    let mut gpu = GpuAtlas::new(&device, &atlas).expect("GpuAtlas");

    for (element, needle) in [(0, "頁が存在しない"), (1, "uv が頁に収まらない")] {
        let (list, events) = capture(|| gpu.leaf(ElementId(element)).cloned());
        let list = list.expect("壊れた置き場でも葉は記録する");
        assert!(
            read_back(&device, &list, 2, 2).iter().all(|&b| b == 0),
            "絵 {element}: 何も描かない"
        );
        let warns: Vec<_> = events
            .iter()
            .filter(|e| e.level == tracing::Level::WARN && e.message().contains(needle))
            .collect();
        assert_eq!(warns.len(), 1, "絵 {element}: {events:?}");
        assert_eq!(
            warns[0].field("element"),
            Some(element.to_string().as_str())
        );
    }
}

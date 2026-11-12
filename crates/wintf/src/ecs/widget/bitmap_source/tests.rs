//! BitmapSource モジュールのユニットテスト

use super::*;
use windows_core::Interface;

// ============================================================
// Task 1.1: WicCore Tests
// ============================================================

// COMを初期化するヘルパー（COINIT_MULTITHREADED for WIC free-threaded factory）
// 初期化に失敗したら落とす。別の型の apartment に入っているスレッド（`RPC_E_CHANGED_MODE`）は
// そのまま使い、入っていない apartment を `CoUninitialize` で壊さない。
fn with_com_initialized<F: FnOnce()>(f: F) {
    use windows::Win32::Foundation::RPC_E_CHANGED_MODE;
    use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx, CoUninitialize};
    let hr = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
    let entered = hr.is_ok();
    assert!(
        entered || hr == RPC_E_CHANGED_MODE,
        "CoInitializeEx が失敗した: {hr:?}"
    );
    f();
    if entered {
        unsafe { CoUninitialize() };
    }
}

#[test]
fn test_wic_core_creation() {
    // WicCoreが正常に作成できることを確認
    with_com_initialized(|| {
        let result = WicCore::new();
        assert!(result.is_ok(), "WicCore creation should succeed");
    });
}

// ============================================================
// W5b-T 追加: resolve_path のデバイス非依存パス解決ロジック
//
// resolve_path は WIC/D2D に一切依存しない純粋なパス変換
// （絶対パスはそのまま / 相対パスは実行ファイルディレクトリ基準で join）。
// 既存テストはゼロだったため特性化する。
// ============================================================

#[test]
fn test_resolve_path_absolute_is_returned_unchanged() {
    use super::systems::resolve_path;
    use std::path::Path;

    // Windows 絶対パスはそのまま返る（current_exe を参照しない）
    let abs = r"C:\some\absolute\image.png";
    let resolved = resolve_path(abs).expect("absolute path resolves");
    assert!(resolved.is_absolute(), "result should stay absolute");
    assert_eq!(resolved, Path::new(abs));
}

#[test]
fn test_resolve_path_relative_is_joined_under_exe_dir() {
    use super::systems::resolve_path;

    // 相対パスは実行ファイルのディレクトリ配下へ join される
    let resolved = resolve_path("assets/logo.png").expect("relative path resolves");

    // 期待値: current_exe().parent() に "assets/logo.png" を結合したもの
    let exe = std::env::current_exe().expect("current_exe available in tests");
    let exe_dir = exe.parent().expect("exe has a parent dir");
    let expected = exe_dir.join("assets/logo.png");

    assert_eq!(resolved, expected);
    // 解決結果は実行ファイルディレクトリ配下（絶対パス）になる
    assert!(resolved.is_absolute());
    assert!(resolved.starts_with(exe_dir));
    assert!(resolved.ends_with("logo.png"));
}

// ============================================================
// 11 番: 解読は裏の作業の 1 回だけ
// ============================================================

#[test]
fn load_decoded_bitmap_returns_in_memory_bitmap() {
    use super::systems::load_decoded_bitmap;
    use windows::Win32::Graphics::Imaging::IWICBitmap;

    with_com_initialized(|| {
        let wic_core = WicCore::new().expect("WicCore creation failed");
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/assets/test_8x8_rgba.png");
        let source = load_decoded_bitmap(wic_core.factory(), &path).expect("load_decoded_bitmap");
        // 変換器は IWICBitmap を持たない。メモリの画像だけが cast できる
        assert!(
            source.cast::<IWICBitmap>().is_ok(),
            "decoded source must be an in-memory IWICBitmap, not the format converter"
        );
    });
}

// ============================================================
// 7 番: 外す・付け直すと、新しい絵と当たりになる（実 GPU）
// ============================================================

use bevy_ecs::prelude::*;

fn asset(name: &str) -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/assets")
        .join(name)
}

/// 裏の作業の代わりに読み、着いた命令と同じ道で入れる
fn arrive(world: &mut World, entity: Entity, wic: &WicCore, name: &str) {
    let path = asset(name);
    let source = super::systems::load_decoded_bitmap(wic.factory(), &path).expect("load");
    super::bitmap_source::insert_loaded_resource(
        world,
        entity,
        path.to_str().unwrap(),
        BitmapSourceResource::new(source),
    );
}

fn d2d_bitmap(world: &World, entity: Entity) -> windows::Win32::Graphics::Direct2D::ID2D1Bitmap1 {
    world
        .get::<BitmapSourceGraphics>(entity)
        .and_then(|g| g.bitmap().cloned())
        .expect("D2D bitmap")
}

/// αのマスクの裏の作業が着くまで命令を流す（上限つき）
fn wait_mask(world: &mut World, entity: Entity) {
    for _ in 0..500 {
        super::systems::drain_task_pool_commands(world);
        if world
            .get::<BitmapSourceResource>(entity)
            .is_some_and(|r| r.alpha_mask().is_some())
        {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    panic!("alpha mask did not arrive");
}

#[test]
fn reattach_rebuilds_bitmap_and_remove_keeps_hit_test() {
    use crate::ecs::Size;
    use crate::ecs::graphics::{GraphicsCommandList, GraphicsCore};
    use crate::ecs::layout::{Arrangement, HitTest};

    with_com_initialized(|| {
        let wic = WicCore::new().expect("WicCore");
        let mut world = World::new();
        world.insert_resource(GraphicsCore::new().expect("GraphicsCore"));
        // WicCore は World に入れない（フックの読み込みは起こさず、着く順をテストが決める）
        world.insert_resource(WintfTaskPool::with_threads(1));
        let mut schedule = Schedule::default();
        schedule.add_systems((draw_bitmap_sources, generate_alpha_mask_system).chain());

        let path_a = asset("test_8x8_rgba.png");
        let arrangement = Arrangement {
            size: Size {
                width: 8.0,
                height: 8.0,
            },
            ..Default::default()
        };
        let e = world
            .spawn((BitmapSource::new(path_a.to_str().unwrap()), arrangement))
            .id();
        arrive(&mut world, e, &wic, "test_8x8_rgba.png");
        schedule.run(&mut world);
        let first = d2d_bitmap(&world, e);
        wait_mask(&mut world, e);

        // マスクを入れても変化の印は立たず、D2D の画像は作り直されない
        schedule.run(&mut world);
        assert_eq!(d2d_bitmap(&world, e).as_raw(), first.as_raw());

        // 外す: 資源が消え、命令の列が空になり、HitTest は残る
        world.entity_mut(e).remove::<BitmapSource>();
        world.flush();
        assert!(world.get::<BitmapSourceResource>(e).is_none());
        assert!(world.get::<HitTest>(e).is_some());
        assert!(!world.get::<BitmapSourceGraphics>(e).unwrap().is_valid());
        assert!(
            world
                .get::<GraphicsCommandList>(e)
                .unwrap()
                .command_list()
                .is_none()
        );

        // 外した部品へ前の読み込みが遅れて着いても入らない
        arrive(&mut world, e, &wic, "test_8x8_rgba.png");
        assert!(world.get::<BitmapSourceResource>(e).is_none());

        // 別の画像に付け直すと、前の読み込みは入らず、新しい読み込みだけが入る
        world.entity_mut(e).insert(BitmapSource::new(
            asset("test_16x16_rgba.png").to_str().unwrap(),
        ));
        world.flush();
        arrive(&mut world, e, &wic, "test_8x8_rgba.png");
        assert!(world.get::<BitmapSourceResource>(e).is_none());
        arrive(&mut world, e, &wic, "test_16x16_rgba.png");
        schedule.run(&mut world);
        let second = d2d_bitmap(&world, e);
        assert_ne!(second.as_raw(), first.as_raw());
        wait_mask(&mut world, e);

        // 同じ部品の資源が入れ替わる（Added でなく Changed）と、絵もマスクも作り直される
        arrive(&mut world, e, &wic, "test_16x16_rgba.png");
        schedule.run(&mut world);
        assert_ne!(d2d_bitmap(&world, e).as_raw(), second.as_raw());
        wait_mask(&mut world, e);
    });
}

/// マスクは、作った画像が今の資源の画像と同じときだけ入る
#[test]
fn alpha_mask_for_previous_image_is_dropped() {
    use super::systems::SetAlphaMaskCommand;

    with_com_initialized(|| {
        let wic = WicCore::new().expect("WicCore");
        let mut world = World::new();
        let path = asset("test_8x8_rgba.png");
        let e = world.spawn(BitmapSource::new(path.to_str().unwrap())).id();
        arrive(&mut world, e, &wic, "test_8x8_rgba.png");
        let addr = world
            .get::<BitmapSourceResource>(e)
            .unwrap()
            .source()
            .as_raw() as usize;
        let mask = || AlphaMask::from_pbgra32(&[0xff; 4], 1, 1, 4);

        SetAlphaMaskCommand {
            entity: e,
            source: addr + 1,
            mask: mask(),
        }
        .apply(&mut world);
        assert!(
            world
                .get::<BitmapSourceResource>(e)
                .unwrap()
                .alpha_mask()
                .is_none()
        );

        SetAlphaMaskCommand {
            entity: e,
            source: addr,
            mask: mask(),
        }
        .apply(&mut world);
        assert!(
            world
                .get::<BitmapSourceResource>(e)
                .unwrap()
                .alpha_mask()
                .is_some()
        );
    });
}

// ============================================================
// 10・21 番: D2D の画像が作れない失敗も骨組みの道を通り、毎回やり直さない（実 GPU）
// ============================================================

#[test]
fn oversize_image_gets_empty_list_and_is_not_retried() {
    use crate::ecs::Size;
    use crate::ecs::graphics::{GraphicsCommandList, GraphicsCore};
    use crate::ecs::layout::Arrangement;
    use windows::Win32::Graphics::Imaging::{GUID_WICPixelFormat32bppPBGRA, WICBitmapCacheOnLoad};

    with_com_initialized(|| {
        let wic = WicCore::new().expect("WicCore");
        let mut world = World::new();
        let core = GraphicsCore::new().expect("GraphicsCore");
        // 装置の上限を 1 越える幅の画像は、CreateBitmapFromWicBitmap が必ず断る
        let max = unsafe { core.device_context().expect("dc").GetMaximumBitmapSize() };
        world.insert_resource(core);
        let source = unsafe {
            wic.factory().CreateBitmap(
                max + 1,
                1,
                &GUID_WICPixelFormat32bppPBGRA,
                WICBitmapCacheOnLoad,
            )
        }
        .expect("CreateBitmap")
        .cast()
        .expect("cast");
        let e = world
            .spawn((
                BitmapSourceResource::new(source),
                BitmapSourceGraphics::new(),
                Arrangement {
                    size: Size {
                        width: 8.0,
                        height: 8.0,
                    },
                    ..Default::default()
                },
            ))
            .id();
        let mut schedule = Schedule::default();
        schedule.add_systems(draw_bitmap_sources);
        schedule.run(&mut world);

        // 空の列が入るので Without<GraphicsCommandList> から外れ、次の回は対象にならない
        let list = world
            .get::<GraphicsCommandList>(e)
            .expect("empty list inserted");
        assert!(list.command_list().is_none());
        assert!(!world.get::<BitmapSourceGraphics>(e).unwrap().is_valid());
    });
}

/// 前の画像を持つ部品へ D2D で作れない画像が着いたら、前の画像を捨てる（次に描くとき古い絵へ戻らない）
#[test]
fn failed_bitmap_for_new_image_drops_previous_bitmap() {
    use crate::ecs::Size;
    use crate::ecs::graphics::GraphicsCore;
    use crate::ecs::layout::Arrangement;
    use windows::Win32::Graphics::Imaging::{GUID_WICPixelFormat32bppPBGRA, WICBitmapCacheOnLoad};

    with_com_initialized(|| {
        let wic = WicCore::new().expect("WicCore");
        let mut world = World::new();
        let core = GraphicsCore::new().expect("GraphicsCore");
        let max = unsafe { core.device_context().expect("dc").GetMaximumBitmapSize() };
        world.insert_resource(core);
        world.insert_resource(WintfTaskPool::with_threads(1));
        let path = asset("test_8x8_rgba.png");
        let e = world
            .spawn((
                BitmapSource::new(path.to_str().unwrap()),
                Arrangement {
                    size: Size {
                        width: 8.0,
                        height: 8.0,
                    },
                    ..Default::default()
                },
            ))
            .id();
        let mut schedule = Schedule::default();
        schedule.add_systems(draw_bitmap_sources);
        arrive(&mut world, e, &wic, "test_8x8_rgba.png");
        schedule.run(&mut world);
        assert!(world.get::<BitmapSourceGraphics>(e).unwrap().is_valid());

        // 装置の上限を越える画像に入れ替わる
        let source = unsafe {
            wic.factory().CreateBitmap(
                max + 1,
                1,
                &GUID_WICPixelFormat32bppPBGRA,
                WICBitmapCacheOnLoad,
            )
        }
        .expect("CreateBitmap")
        .cast()
        .expect("cast");
        world
            .entity_mut(e)
            .insert(BitmapSourceResource::new(source));
        schedule.run(&mut world);
        assert!(!world.get::<BitmapSourceGraphics>(e).unwrap().is_valid());
    });
}

// ============================================================
// 7.1 二度目の見直し N5: 読み込んだ後に当たりを AlphaMask へ替えてもマスクを作る（実 GPU なし）
// ============================================================

#[test]
fn switching_hit_test_to_alpha_mask_after_load_builds_mask() {
    use crate::ecs::layout::HitTest;

    with_com_initialized(|| {
        let wic = WicCore::new().expect("WicCore");
        let mut world = World::new();
        world.insert_resource(WintfTaskPool::with_threads(1));
        let mut schedule = Schedule::default();
        schedule.add_systems(generate_alpha_mask_system);

        let path = asset("test_8x8_rgba.png");
        let e = world
            .spawn((BitmapSource::new(path.to_str().unwrap()), HitTest::bounds()))
            .id();
        arrive(&mut world, e, &wic, "test_8x8_rgba.png");
        schedule.run(&mut world);
        super::systems::drain_task_pool_commands(&mut world);
        assert!(
            world
                .get::<BitmapSourceResource>(e)
                .unwrap()
                .alpha_mask()
                .is_none(),
            "矩形の当たりではマスクを作らない"
        );

        world.entity_mut(e).insert(HitTest::alpha_mask());
        schedule.run(&mut world);
        wait_mask(&mut world, e);
    });
}

// ============================================================
// αマスクの読み込みの大きさ（W5）— CPU だけ
// ============================================================

/// 行の幅と全体の大きさは u32 に収まるときだけ返し、収まらなければ測れないと返す
/// （`CopyPixels` は全体のバイト数を u32 へ変換して溢れると panic するため）。
#[test]
fn pbgra32_layout_reports_overflow_instead_of_wrapping() {
    use super::systems::pbgra32_layout;
    assert_eq!(pbgra32_layout(2, 3), Some((8, 24)));
    // 32768×32768 は 4 GiB で u32 を超える（64 ビットの usize には収まっても受けない）
    assert_eq!(pbgra32_layout(32768, 32768), None);
    // 行の幅が u32 を超える
    assert_eq!(pbgra32_layout(u32::MAX, 1), None);
}

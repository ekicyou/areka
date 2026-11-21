//! `gpu_on` が COM の初期化の失敗を捨てずに止める檻。

use std::panic::{AssertUnwindSafe, catch_unwind};

use bevy_ecs::world::World;
use windows::Win32::System::Com::{COINIT_APARTMENTTHREADED, CoInitializeEx, CoUninitialize};
use wintf::ecs::GraphicsCore;

use super::gpu_on;

/// STA の糸で MTA を求めると `RPC_E_CHANGED_MODE` が返る。`gpu_on` はそこで止まり、
/// GPU の許可も装置も取らない。
#[test]
fn gpu_on_stops_when_com_cannot_enter_the_mta() {
    std::thread::spawn(|| {
        // SAFETY: このスレッドの COM を STA で初期化するだけ。下の `CoUninitialize` と対になる。
        let sta = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };
        assert_eq!(
            sta,
            windows::Win32::Foundation::S_OK,
            "前提: 新しい糸は STA に入れる"
        );
        let mut world = World::new();
        let outcome = catch_unwind(AssertUnwindSafe(|| gpu_on(&mut world)));
        // SAFETY: 上の STA の初期化と対になる後始末。
        unsafe { CoUninitialize() };
        let message = match &outcome {
            Ok(_) => String::new(),
            Err(payload) => payload
                .downcast_ref::<String>()
                .cloned()
                .unwrap_or_default(),
        };
        assert!(
            message.contains("CoInitializeEx"),
            "COM の初期化の失敗で止まる（実際: 止まった={}, 文={message:?}）",
            outcome.is_err()
        );
        assert!(
            !world.contains_resource::<GraphicsCore>(),
            "止まる前に装置を入れない"
        );
    })
    .join()
    .unwrap();
}

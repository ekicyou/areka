//! `clip_key_for` の純粋なテスト（装置なし・工場だけ）。

use super::*;
use crate::ecs::test_support::capture_under_filter;
use windows::Win32::Graphics::Direct2D::Common::D2D_RECT_F;
use windows::Win32::Graphics::Direct2D::*;

const WARN_TEXT: &str = "WUC では範囲外なので切り抜かない";

fn geometry() -> ClipShape {
    let factory: ID2D1Factory1 =
        unsafe { D2D1CreateFactory(D2D1_FACTORY_TYPE_MULTI_THREADED, None) }.unwrap();
    let rect = D2D_RECT_F {
        left: 0.0,
        top: 0.0,
        right: 10.0,
        bottom: 10.0,
    };
    let g = unsafe { factory.CreateRectangleGeometry(&rect) }.unwrap();
    ClipShape::Geometry(g.into())
}

fn key(clip: Option<&ClipShape>, prev: Option<&ClipKey>) -> ClipKey {
    clip_key_for(
        Entity::from_raw_u32(7).unwrap(),
        Some(&Name::new("clip-owner")),
        clip,
        100.0,
        50.0,
        2.0,
        2.0,
        prev,
    )
}

#[test]
fn geometry_becomes_no_clip_key_and_warns_once() {
    let shape = geometry();
    let mut keys = Vec::new();
    let log = capture_under_filter("warn", || {
        let first = key(Some(&shape), None);
        // 同じ図形のまま配置だけ変わった回: 鍵は同じで warn は出ない。
        let second = key(Some(&shape), Some(&first));
        keys.push(first);
        keys.push(second);
    });
    for k in &keys {
        assert_eq!(k.3, None, "図形は控えに持たない（切り抜きなし）");
    }
    assert_eq!(keys[0], keys[1]);
    assert_eq!(log.matches(WARN_TEXT).count(), 1, "{log}");
    assert!(log.contains("clip-owner"), "実体名が要る: {log}");
}

#[test]
fn geometry_after_no_clip_changes_key_and_warns() {
    let shape = geometry();
    let none = key(None, None);
    let mut got = None;
    let log = capture_under_filter("warn", || got = Some(key(Some(&shape), Some(&none))));
    assert_ne!(got.unwrap(), none, "図形へ変わった回は鍵が変わる");
    assert_eq!(log.matches(WARN_TEXT).count(), 1, "{log}");
}

#[test]
fn rect_key_keeps_shape_in_physical_pixels() {
    let k = key(Some(&ClipShape::Rectangle), None);
    assert_eq!(k, (200.0, 100.0, 2.0, Some(ClipShape::Rectangle)));
}

#[test]
fn clip_shape_is_send_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<ClipShape>();
    assert_send_sync::<crate::ecs::graphics::Visual>();
    assert_send_sync::<VisualGraphics>();
}

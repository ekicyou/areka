//! GraphicsCommandList コンポーネントのギャップテスト (W3b-T)
//!
//! GraphicsCommandList は他テストでフィクスチャとして使用されるのみで、
//! 直接のユニットテストが存在しなかったため追加する。
//! 対象: crates/wintf/src/ecs/graphics/command_list.rs

use wintf::com::d2d::D2D1DeviceExt;
use wintf::ecs::{GraphicsCommandList, GraphicsCore};

/// empty() は内部 CommandList を持たない
#[test]
fn empty_has_no_command_list() {
    let cl = GraphicsCommandList::empty();
    assert!(cl.command_list().is_none());
}

/// new() は実 ID2D1CommandList を保持し、アクセサで取得できる
#[test]
fn new_wraps_real_command_list() {
    let graphics = GraphicsCore::new().expect("GraphicsCore creation");
    let device = graphics.d2d_device().expect("d2d device");
    let raw = device.create_command_list().expect("command list");

    let component = GraphicsCommandList::new(raw.clone());
    assert!(component.command_list().is_some());
    // COM スマートポインタは同一インターフェイスを指す
    assert_eq!(component.command_list(), Some(&raw));
}

/// Clone は同一 COM ポインタを共有し PartialEq で等価、empty とは非等価
#[test]
fn clone_and_partial_eq_semantics() {
    let graphics = GraphicsCore::new().expect("GraphicsCore creation");
    let device = graphics.d2d_device().expect("d2d device");
    let raw = device.create_command_list().expect("command list");

    let a = GraphicsCommandList::new(raw);
    let b = a.clone();
    assert_eq!(a, b, "clone shares the same COM pointer");

    assert_eq!(GraphicsCommandList::empty(), GraphicsCommandList::empty());
    assert_ne!(a, GraphicsCommandList::empty());

    // 別個に作成した CommandList は別ポインタなので非等価
    let other = device.create_command_list().expect("second command list");
    assert_ne!(a, GraphicsCommandList::new(other));
}

/// Debug 出力に型名が含まれる
#[test]
fn debug_format_contains_type_name() {
    let cl = GraphicsCommandList::empty();
    let s = format!("{:?}", cl);
    assert!(s.contains("GraphicsCommandList"), "debug output: {s}");
}

/// Send + Sync 境界（unsafe impl の固定化）
#[test]
fn command_list_is_send_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<GraphicsCommandList>();
}

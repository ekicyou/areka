//! 問い 5: 既定の形と `.minimal()` の `HWND` を包んで bevy の Component に持たせる。

pub mod b_default {
    #![allow(non_camel_case_types, non_snake_case, dead_code, clippy::all)]
    include!(concat!(env!("OUT_DIR"), "/hwnd_default.rs"));
}
pub mod b_minimal {
    #![allow(non_camel_case_types, non_snake_case, dead_code, clippy::all)]
    include!(concat!(env!("OUT_DIR"), "/hwnd_minimal.rs"));
}

use bevy_ecs::component::Component;

#[derive(Component, Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct WindowHandleDefault(pub b_default::HWND);
#[derive(Component, Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct WindowHandleMinimal(pub b_minimal::HWND);

// 根拠は README 問い 5: HWND は OS の中でプロセスをまたいで通じる番号。値を読むだけなら別スレッドからも安全。
// 窓に触る呼び出しは窓を作ったスレッドへ送る決まりを包みの外で守る。
unsafe impl Send for WindowHandleDefault {}
unsafe impl Sync for WindowHandleDefault {}
unsafe impl Send for WindowHandleMinimal {}
unsafe impl Sync for WindowHandleMinimal {}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_ecs::prelude::*;

    fn assert_send_sync<T: Send + Sync>() {}

    #[test]
    fn wrappers_are_send_sync() {
        assert_send_sync::<WindowHandleDefault>();
        assert_send_sync::<WindowHandleMinimal>();
    }

    fn check_default(q: Query<&WindowHandleDefault>, mut ok: ResMut<Seen>) {
        for h in &q {
            ok.default_valid = unsafe { b_default::IsWindow(Some(h.0)) }.as_bool();
        }
    }
    fn check_minimal(q: Query<&WindowHandleMinimal>, mut ok: ResMut<Seen>) {
        for h in &q {
            ok.minimal_valid = unsafe { b_minimal::IsWindow(h.0) }.as_bool();
        }
    }
    #[derive(Resource, Default)]
    struct Seen {
        default_valid: bool,
        minimal_valid: bool,
    }

    // 形ごとの口の表（README 問い 5）の根拠: HWND はどちらも `*mut HWND__` の別名（newtype なら型が合わず組めない）。
    #[test]
    fn hwnd_is_raw_pointer_alias() {
        fn same<T>(_: T, _: T) {}
        let d: *mut b_default::HWND__ = unsafe { b_default::GetDesktopWindow() };
        let m: *mut b_minimal::HWND__ = unsafe { b_minimal::GetDesktopWindow() };
        same::<b_default::HWND>(d, d);
        same::<b_minimal::HWND>(m, m);
        assert!(!d.is_null());
        assert!(!m.is_null());
    }

    #[test]
    fn desktop_handle_via_world() {
        let mut w = World::new();
        w.insert_resource(Seen::default());
        w.spawn(WindowHandleDefault(unsafe {
            b_default::GetDesktopWindow()
        }));
        w.spawn(WindowHandleMinimal(unsafe {
            b_minimal::GetDesktopWindow()
        }));
        let a = w.register_system(check_default);
        w.run_system(a).unwrap();
        let b = w.register_system(check_minimal);
        w.run_system(b).unwrap();
        let s = w.resource::<Seen>();
        assert!(s.default_valid && s.minimal_valid);
    }
}

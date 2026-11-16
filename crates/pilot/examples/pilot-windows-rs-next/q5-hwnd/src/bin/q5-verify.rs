// デスクトップの窓の取っ手を両方の包みで世界に入れ、別のスレッドの中のシステムから読んで IsWindow を確かめる。
use bevy_ecs::prelude::*;
use q5_hwnd::*;

#[derive(Resource, Default)]
struct Seen(bool, bool);

fn check(d: Query<&WindowHandleDefault>, m: Query<&WindowHandleMinimal>, mut s: ResMut<Seen>) {
    s.0 = d
        .iter()
        .all(|h| unsafe { b_default::IsWindow(Some(h.0)) }.as_bool())
        && d.iter().count() == 1;
    s.1 = m
        .iter()
        .all(|h| unsafe { b_minimal::IsWindow(h.0) }.as_bool())
        && m.iter().count() == 1;
}

fn main() {
    let mut w = World::new();
    w.insert_resource(Seen::default());
    w.spawn(WindowHandleDefault(unsafe {
        b_default::GetDesktopWindow()
    }));
    w.spawn(WindowHandleMinimal(unsafe {
        b_minimal::GetDesktopWindow()
    }));
    // World は Send なので別スレッドへ渡して読む（Component が Send + Sync である証拠）。
    let seen = std::thread::spawn(move || {
        let id = w.register_system(check);
        w.run_system(id).unwrap();
        let s = w.resource::<Seen>();
        (s.0, s.1)
    })
    .join()
    .unwrap();
    println!(
        "[q5] default IsWindow={} minimal IsWindow={}",
        seen.0, seen.1
    );
    std::process::exit(if seen == (true, true) { 0 } else { 1 });
}

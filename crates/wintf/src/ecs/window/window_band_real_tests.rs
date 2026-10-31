//! 実際の窓で、帯の系が並びを保って帯へ入れる・帯から出す・今いる帯の最上へ出すことを
//! 確かめる（`stay-on-top-directives` 要件 2.4・2.6・2.9・3.1・7.3）。
//!
//! 偽の OS（兄弟の `window_band_tests.rs` の `double`）は据えない——台本が無い間は本番の
//! Win32 が呼ばれる。窓は持ち主の関係を張った自分のトップレベルの窓 4 枚（キャラクター 2・
//! バルーン 2・鎖あり）で、手前から B0・C0・B1・C1 の 1 本の持ち主の列になる。
//!
//! トップレベルの窓の重なりは他のプロセスの窓に割り込まれうるので、兄弟の
//! `zorder_pair_maintain_always_on_top_tests.rs` と同じ実窓の門
//! （`AREKA_WINTF_REAL_WINDOW_ZORDER`・`#[ignore]`）の内側に置く。並びの主張は自分の 4 枚の
//! 相対の順だけを見るので、他の窓が間に入っても崩れない。

use bevy_ecs::prelude::*;
use windows::Win32::Foundation::{HINSTANCE, HWND, RECT};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DestroyWindow, GW_HWNDNEXT, GWL_EXSTYLE, GetTopWindow, GetWindow,
    GetWindowRect, HWND_TOP, HWND_TOPMOST, SW_SHOWNA, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE,
    SetWindowPos, ShowWindow, WINDOW_EX_STYLE, WS_EX_LAYERED, WS_EX_TOOLWINDOW, WS_EX_TOPMOST,
    WS_POPUP,
};
use windows::core::{PCWSTR, w};

use super::{BandOp, WindowBandQueue, apply_window_band};
use crate::api::{clear_window_owner, get_window_long_ptr, is_window_visible, set_window_owner};
use crate::ecs::window::WindowHandle;

const REAL_WINDOW_ZORDER_ENV: &str = "AREKA_WINTF_REAL_WINDOW_ZORDER";

/// 門が開いていなければ判定を 1 行も走らせずに落とす（素通りの緑を「実窓で測った」に化けさせない）。
fn require_explicit_real_window_run() {
    assert!(
        std::env::var_os(REAL_WINDOW_ZORDER_ENV).is_some(),
        "{REAL_WINDOW_ZORDER_ENV} が未設定である（判定は 1 行も走っていない）。走らせるには \
         環境変数を与えて `cargo test -p wintf --lib <テスト名> -- --ignored --exact` とすること"
    );
}

fn create_toplevel(title: PCWSTR, ex_style: u32, x: i32) -> HWND {
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    // SAFETY: Win32 境界。定義済 "Static" クラスで小さなテスト窓を作り、活性化せずに見せる。
    unsafe {
        let hinstance = GetModuleHandleW(None).expect("GetModuleHandleW");
        let hwnd = CreateWindowExW(
            WINDOW_EX_STYLE(ex_style),
            w!("Static"),
            title,
            WS_POPUP,
            x,
            40,
            16,
            12,
            None,
            None,
            Some(hinstance.into()),
            None,
        )
        .expect("CreateWindowExW should create a test window");
        let _ = ShowWindow(hwnd, SW_SHOWNA);
        hwnd
    }
}

/// 本番のゴーストの窓と同じ拡張スタイル（`WS_EX_LAYERED` なので画面には何も描かれない）。
fn create_ghost_shaped(title: PCWSTR, x: i32) -> HWND {
    create_toplevel(title, WS_EX_LAYERED.0 | WS_EX_TOOLWINDOW.0, x)
}

fn destroy(hwnd: HWND) {
    // SAFETY: Win32 境界。自分で作った窓を壊す。
    unsafe {
        let _ = DestroyWindow(hwnd);
    }
}

fn ex_style(hwnd: HWND) -> u32 {
    get_window_long_ptr(hwnd, GWL_EXSTYLE).expect("拡張スタイルを読めるはず") as u32
}

fn in_band(hwnd: HWND) -> bool {
    ex_style(hwnd) & WS_EX_TOPMOST.0 != 0
}

/// 位置と大きさ、`WS_EX_TOPMOST` を除いた拡張スタイル（どの指令でも変わってはならない）。
fn shape(hwnd: HWND) -> (RECT, u32) {
    let mut rect = RECT::default();
    // SAFETY: Win32 境界。自分で作った窓の矩形を読むだけ。
    unsafe { GetWindowRect(hwnd, &mut rect) }.expect("GetWindowRect");
    (rect, ex_style(hwnd) & !WS_EX_TOPMOST.0)
}

fn raise_to_top(hwnd: HWND, insert_after: HWND) {
    // SAFETY: Win32 境界。自分で作った窓の重なりだけを変える。
    unsafe {
        SetWindowPos(
            hwnd,
            Some(insert_after),
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
        )
    }
    .expect("SetWindowPos should reorder a self-owned test window");
}

/// 最上から辿った重なりの列（手前 → 奥）。
fn desktop_z() -> Vec<HWND> {
    let mut out = Vec::new();
    // SAFETY: Win32 境界。読み取りだけ。
    let mut current = unsafe { GetTopWindow(None) }.ok();
    while let Some(hwnd) = current {
        out.push(hwnd);
        // SAFETY: 同上。
        current = unsafe { GetWindow(hwnd, GW_HWNDNEXT) }.ok();
    }
    out
}

/// `windows` の窓だけを、今の重なりの手前 → 奥の順に並べる。
fn relative_order(windows: &[HWND]) -> Vec<HWND> {
    desktop_z()
        .into_iter()
        .filter(|hwnd| windows.contains(hwnd))
        .collect()
}

/// 自分の窓のうち一番手前の窓より手前に居る、可視の窓の列。
fn visible_in_front_of(windows: &[HWND]) -> Vec<HWND> {
    desktop_z()
        .into_iter()
        .take_while(|hwnd| !windows.contains(hwnd))
        .filter(|hwnd| is_window_visible(*hwnd))
        .collect()
}

fn run_op(world: &mut World, op: BandOp, entities: &[Entity]) {
    world
        .resource_mut::<WindowBandQueue>()
        .push(op, entities.to_vec());
    apply_window_band(world);
}

/// 持ち主の列（キャラクター 2・バルーン 2・鎖あり）が、入る・出る・手前へ出す（帯の外と中）の
/// どれでも並びを保ち、帯の印だけが変わる。
#[test]
#[ignore = "実窓の重なり順は他プロセスの可視窓に割り込まれて間欠的に赤くなる（AREKA_WINTF_REAL_WINDOW_ZORDER を与えて明示実行する）"]
fn band_ops_keep_the_owned_chain_order_and_shape_on_real_windows() {
    require_explicit_real_window_run();

    let c1 = create_ghost_shaped(w!("wintf-band-real-c1"), 40);
    let b1 = create_ghost_shaped(w!("wintf-band-real-b1"), 60);
    let c0 = create_ghost_shaped(w!("wintf-band-real-c0"), 80);
    let b0 = create_ghost_shaped(w!("wintf-band-real-b0"), 100);
    // 組（バルーン → キャラクター）と鎖（C0 → B1）。持たれる窓は持ち主より手前に保たれる。
    set_window_owner(b1, c1).expect("owner");
    set_window_owner(c0, b1).expect("owner");
    set_window_owner(b0, c0).expect("owner");
    raise_to_top(c1, HWND_TOP);
    let ours = [b0, c0, b1, c1];
    let original = relative_order(&ours);
    assert_eq!(original, ours, "前提: 手前から B0・C0・B1・C1 の持ち主の列");
    assert!(
        ours.iter().all(|h| !in_band(*h)),
        "前提: 全部帯の外で始まる"
    );
    let shapes: Vec<_> = ours.iter().map(|h| shape(*h)).collect();
    let assert_shape = |phase: &str| {
        let now: Vec<_> = ours.iter().map(|h| shape(*h)).collect();
        assert_eq!(
            now, shapes,
            "{phase}: 位置・大きさ・拡張スタイルが変わった（要件 2.6）"
        );
    };

    let mut world = World::new();
    world.init_resource::<WindowBandQueue>();
    let spawn = |world: &mut World, hwnd: HWND| {
        world
            .spawn(WindowHandle {
                hwnd,
                instance: HINSTANCE::default(),
            })
            .id()
    };
    let (eb0, ec0, eb1, ec1) = (
        spawn(&mut world, b0),
        spawn(&mut world, c0),
        spawn(&mut world, b1),
        spawn(&mut world, c1),
    );
    // 積む順は重なりの順と違えておく（帯の系が奥から順に並べ直すことも確かめる）。
    let entities = [eb0, ec1, ec0, eb1];

    // 入る: 全部帯・並びは元のまま（要件 2.4・3.1）。
    run_op(&mut world, BandOp::Enter, &entities);
    assert!(
        ours.iter().all(|h| in_band(*h)),
        "入るの後に全部帯に居るはず"
    );
    assert_eq!(relative_order(&ours), original, "入るの後も並びは元のまま");
    assert_shape("入る");

    // 出る: 全部帯の外・並びは元のまま（要件 3.1）。
    run_op(&mut world, BandOp::Leave, &entities);
    assert!(
        ours.iter().all(|h| !in_band(*h)),
        "出るの後に全部帯の外のはず"
    );
    assert_eq!(relative_order(&ours), original, "出るの後も並びは元のまま");
    assert_shape("出る");

    // 手前へ出す（帯の外）: 先に普通の窓の対照を最上へ置き、出した後は普通の窓の最上（要件 2.9）。
    let normal_probe = create_ghost_shaped(w!("wintf-band-real-normal-probe"), 120);
    raise_to_top(normal_probe, HWND_TOP);
    assert!(
        visible_in_front_of(&ours).contains(&normal_probe),
        "前提: 普通の窓の対照が自分の窓より手前に居る"
    );
    run_op(&mut world, BandOp::Raise, &entities);
    let normal_in_front: Vec<_> = visible_in_front_of(&ours)
        .into_iter()
        .filter(|h| !in_band(*h))
        .collect();
    assert!(
        normal_in_front.is_empty(),
        "帯の外の手前へ出すで普通の窓の最上に居るはず（手前に居た普通の窓: {normal_in_front:?}）"
    );
    assert!(
        ours.iter().all(|h| !in_band(*h)),
        "手前へ出すは帯を変えない"
    );
    assert_eq!(
        relative_order(&ours),
        original,
        "手前へ出すの後も並びは元のまま"
    );
    assert_shape("手前へ出す（帯の外）");

    // 手前へ出す（帯の中）: 帯の窓の対照を帯の最上へ置き、出した後はその対照より手前（帯の最上）。
    run_op(&mut world, BandOp::Enter, &entities);
    let band_probe = create_toplevel(
        w!("wintf-band-real-band-probe"),
        WS_EX_LAYERED.0 | WS_EX_TOOLWINDOW.0 | WS_EX_TOPMOST.0,
        140,
    );
    raise_to_top(band_probe, HWND_TOPMOST);
    assert!(
        visible_in_front_of(&ours).contains(&band_probe),
        "前提: 帯の窓の対照が自分の窓より手前に居る"
    );
    run_op(&mut world, BandOp::Raise, &entities);
    assert!(
        !visible_in_front_of(&ours).contains(&band_probe),
        "帯の中の手前へ出すで帯の最上（帯の対照より手前）に居るはず"
    );
    assert!(ours.iter().all(|h| in_band(*h)), "手前へ出すは帯を変えない");
    assert_eq!(
        relative_order(&ours),
        original,
        "手前へ出すの後も並びは元のまま"
    );
    assert_shape("手前へ出す（帯の中）");

    for hwnd in [b0, c0, b1] {
        let _ = clear_window_owner(hwnd);
    }
    for hwnd in [b0, c0, b1, c1, normal_probe, band_probe] {
        destroy(hwnd);
    }
}

/// 実機の 2 回目の走りの形（task 7.1）: 組が 2 つ（鎖なし・キャラクターに持ち主なし）で、
/// 普通の窓 1 枚の奥に居るところから入る。持ち主（キャラクター）も帯に入り、並びが元のまま、
/// 普通の窓より手前に居る。実機では持ち主だけが帯へ入らず元の位置に残った
/// （`[window_band] band mismatch after apply`）。
///
/// この試験は実機の赤を**再現しない**（直す前の `SWP_NOOWNERZORDER` なしでも同じプロセスの窓では緑
/// だった。他のプロセスの窓・DComp・IME の窓が効くと見られる）。直したことの証ではなく、形の回帰の
/// 見張りとして残す。直したことは 7.1 の実機の走りで確かめる。
#[test]
#[ignore = "実窓の重なり順は他プロセスの可視窓に割り込まれて間欠的に赤くなる（AREKA_WINTF_REAL_WINDOW_ZORDER を与えて明示実行する）"]
fn owners_of_balloons_enter_the_band_from_behind_a_normal_window_on_real_windows() {
    require_explicit_real_window_run();

    let c1 = create_ghost_shaped(w!("wintf-band-behind-c1"), 40);
    let b1 = create_ghost_shaped(w!("wintf-band-behind-b1"), 60);
    let c0 = create_ghost_shaped(w!("wintf-band-behind-c0"), 80);
    let b0 = create_ghost_shaped(w!("wintf-band-behind-b0"), 100);
    set_window_owner(b1, c1).expect("owner");
    set_window_owner(b0, c0).expect("owner");
    raise_to_top(c1, HWND_TOP);
    raise_to_top(c0, HWND_TOP);
    // 他のアプリの窓の代わり（描かれる普通の窓）を組より手前へ。
    let other = create_toplevel(w!("wintf-band-behind-other"), 0, 120);
    raise_to_top(other, HWND_TOP);
    let ours = [b0, c0, b1, c1];
    assert_eq!(
        relative_order(&[other, b0, c0, b1, c1]),
        [other, b0, c0, b1, c1],
        "前提: 手前から 普通の窓・B0・C0・B1・C1"
    );
    assert!(ours.iter().all(|h| !in_band(*h)), "前提: 全部帯の外");

    let mut world = World::new();
    world.init_resource::<WindowBandQueue>();
    let entities: Vec<Entity> = ours
        .iter()
        .map(|&hwnd| {
            world
                .spawn(WindowHandle {
                    hwnd,
                    instance: HINSTANCE::default(),
                })
                .id()
        })
        .collect();
    run_op(&mut world, BandOp::Enter, &entities);

    let outside: Vec<_> = ours.iter().filter(|h| !in_band(**h)).collect();
    let order = relative_order(&ours);
    let other_in_front = visible_in_front_of(&ours).contains(&other);
    for hwnd in [b0, b1] {
        let _ = clear_window_owner(hwnd);
    }
    for hwnd in [b0, c0, b1, c1, other] {
        destroy(hwnd);
    }
    assert!(
        outside.is_empty(),
        "入るの後に全部帯に居るはず（帯の外: {outside:?}・C0={c0:?} C1={c1:?}）"
    );
    assert_eq!(order, ours, "入るの後も並びは元のまま");
    assert!(!other_in_front, "入るの後は普通の窓より手前に居るはず");
}

//! 実際の窓で、確定段の並び（所有の結び → 帯の系 → 組の維持 → 鎖の適用）を 2 巡（是正を決めて
//! 流す 1 巡と検証の 1 巡）回し、オンの指令と同じ巡に組の是正が要る場面を確かめる（`stay-on-top-directives` 要件 2.1・2.4・
//! 3.1・6.3・7.3、task 5.2）。
//!
//! 組の維持の実測（`zorder_pair.rs` の `measure_*`）には偽の OS の継ぎ目が無いので、この 1 件は
//! 偽の OS ではなく実窓で測る（開発者の裁定）。トップレベルの窓の重なりは他のプロセスの窓に
//! 割り込まれうるので、兄弟の `window_band_real_tests.rs` と同じ実窓の門
//! （`AREKA_WINTF_REAL_WINDOW_ZORDER`・`#[ignore]`）の内側に置き、主張は自分の窓の相対の順と
//! 帯の印だけを見る。
//!
//! 並びは本番の `crates/areka/src/placement/spawn.rs` の `wire_zorder_pair` と同じである
//! （並びそのものの固定は areka 側の兄弟のテスト `spawn_window_band_wiring_tests.rs` が持つ）。

use bevy_ecs::prelude::*;
use bevy_ecs::schedule::{Schedule, SingleThreadedExecutor};
use windows::Win32::Foundation::{HINSTANCE, HWND};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DestroyWindow, GW_HWNDNEXT, GWL_EXSTYLE, GetTopWindow, GetWindow, HWND_TOP,
    SW_SHOWNA, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SetWindowPos, ShowWindow, WINDOW_EX_STYLE,
    WS_EX_LAYERED, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP,
};
use windows::core::{PCWSTR, w};

use super::{BandOp, WindowBandQueue, apply_window_band};
use crate::api::get_window_long_ptr;
use crate::ecs::test_support::capture_under_filter;
use crate::ecs::window::zorder_pair::measure_window_below;
use crate::ecs::window::zorder_pair_diag::hwnd_field;
use crate::ecs::window::{
    KeepDirectlyAbove, ReassertZOrder, SetWindowPosCommand, WindowHandle, ZOrderPairStrategy,
    apply_zorder_chain, apply_zorder_pair_maintenance, establish_owner_links,
};

const REAL_WINDOW_ZORDER_ENV: &str = "AREKA_WINTF_REAL_WINDOW_ZORDER";

const CAPTURE_DIRECTIVES: &str =
    "info,wintf::ecs::window::zorder_pair=debug,wintf::ecs::window::command=debug";
const FIX_TAG: &str = "[zorder-pair] fix";
const VERIFY_FAILED_TAG: &str = "[zorder-pair] verify-failed";

/// 門が開いていなければ判定を 1 行も走らせずに落とす（素通りの緑を「実窓で測った」に化けさせない）。
fn require_explicit_real_window_run() {
    assert!(
        std::env::var_os(REAL_WINDOW_ZORDER_ENV).is_some(),
        "{REAL_WINDOW_ZORDER_ENV} が未設定である（判定は 1 行も走っていない）。走らせるには \
         環境変数を与えて `cargo test -p wintf --lib <テスト名> -- --ignored --exact` とすること"
    );
}

/// 本番のゴーストの窓と同じ拡張スタイルの可視の窓（`WS_EX_LAYERED` なので画面には何も描かれない）。
fn create_ghost_shaped(title: PCWSTR, x: i32) -> HWND {
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    // SAFETY: Win32 境界。定義済 "Static" クラスで小さなテスト窓を作り、活性化せずに見せる。
    unsafe {
        let hinstance = GetModuleHandleW(None).expect("GetModuleHandleW");
        let hwnd = CreateWindowExW(
            WINDOW_EX_STYLE(WS_EX_LAYERED.0 | WS_EX_TOOLWINDOW.0),
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

fn destroy(hwnd: HWND) {
    // SAFETY: Win32 境界。自分で作った窓を壊す。
    unsafe {
        let _ = DestroyWindow(hwnd);
    }
}

fn in_band(hwnd: HWND) -> bool {
    let ex = get_window_long_ptr(hwnd, GWL_EXSTYLE).expect("拡張スタイルを読めるはず") as u32;
    ex & WS_EX_TOPMOST.0 != 0
}

fn insert_after(hwnd: HWND, after: HWND) {
    // SAFETY: Win32 境界。自分で作った窓の重なりだけを変える。
    unsafe {
        SetWindowPos(
            hwnd,
            Some(after),
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
        )
    }
    .expect("SetWindowPos should reorder a self-owned test window");
}

/// `windows` の窓だけを、今の重なりの手前 → 奥の順に並べる。
fn relative_order(windows: &[HWND]) -> Vec<HWND> {
    let mut out = Vec::new();
    // SAFETY: Win32 境界。読み取りだけ。
    let mut current = unsafe { GetTopWindow(None) }.ok();
    while let Some(hwnd) = current {
        if windows.contains(&hwnd) {
            out.push(hwnd);
        }
        // SAFETY: 同上。
        current = unsafe { GetWindow(hwnd, GW_HWNDNEXT) }.ok();
    }
    out
}

/// オンの指令と同じ巡に組の是正が要る場面: 帯を当てる前は組が帯の外で隣り合っておらず、
/// 是正は帯を当てた後の観測から決まり、流れた後もバルーンは帯の中でキャラクターのすぐ
/// 手前に居る。
///
/// 手前から「普通の窓 N／キャラクター C／バルーン B」で始める。帯の系は並びを保つので、帯を
/// 当てた後も B は C の奥に居て、是正が必ず要る。是正は C の手前の帯の窓のすぐ背後か、手前に
/// 可視の窓が無ければ `top-edge`（帯の最上）に決まる（どちらになるかは机の上の他の帯の窓
/// 次第）。帯を当てる前の観測で是正を決めると挿入位置は N（帯の外の窓）になり、巡の終わりに
/// 流れた是正がバルーンを帯の外へ落とす——並びが崩れたときに赤くなる形である。
///
/// 組は案 B（`ExplicitMaintenance`・owner を張らない）で組む。案 A の owner を張ると、帯へ入る
/// キャラクターに OS がバルーンを連れて行き、是正が要らなくなって場面そのものが作れない。
#[test]
#[ignore = "実窓の重なり順は他プロセスの可視窓に割り込まれて間欠的に赤くなる（AREKA_WINTF_REAL_WINDOW_ZORDER を与えて明示実行する）"]
fn an_on_directive_and_a_pair_fix_in_the_same_pass_leave_the_balloon_in_the_band_above_the_character()
 {
    require_explicit_real_window_run();

    let normal = create_ghost_shaped(w!("wintf-band-pipeline-normal"), 40);
    let character_hwnd = create_ghost_shaped(w!("wintf-band-pipeline-character"), 60);
    let balloon_hwnd = create_ghost_shaped(w!("wintf-band-pipeline-balloon"), 80);
    insert_after(normal, HWND_TOP);
    insert_after(character_hwnd, normal);
    insert_after(balloon_hwnd, character_hwnd);
    let ours = [normal, character_hwnd, balloon_hwnd];
    assert_eq!(
        relative_order(&ours),
        ours,
        "前提: 手前から N・C・B（バルーンがキャラクターの奥＝是正が要る）"
    );
    assert!(
        ours.iter().all(|h| !in_band(*h)),
        "前提: 帯を当てる前は全部帯の外"
    );

    let mut world = World::new();
    world.insert_resource(ZOrderPairStrategy::ExplicitMaintenance);
    world.init_resource::<WindowBandQueue>();
    let spawn = |world: &mut World, hwnd: HWND| {
        world
            .spawn(WindowHandle {
                hwnd,
                instance: HINSTANCE::default(),
            })
            .id()
    };
    let character = spawn(&mut world, character_hwnd);
    let balloon = spawn(&mut world, balloon_hwnd);
    world.entity_mut(balloon).insert((
        KeepDirectlyAbove { peer: character },
        ReassertZOrder::default(),
    ));
    // 取り出しの相が積むのと同じく、ゴーストの全部の窓へオンの指令を 1 件。
    world
        .resource_mut::<WindowBandQueue>()
        .push(BandOp::Enter, vec![character, balloon]);

    // 本番の `wire_zorder_pair`（areka の spawn.rs）と同じ並び・同じ同期点。
    let mut schedule = Schedule::default();
    schedule.set_executor(SingleThreadedExecutor::new());
    schedule.add_systems(
        (
            establish_owner_links,
            apply_window_band,
            apply_zorder_pair_maintenance,
            apply_zorder_chain,
        )
            .chain(),
    );
    let out = capture_under_filter(CAPTURE_DIRECTIVES, || {
        // 1 巡目: 結び・帯・是正の判断（指令は予約の列へ）。流して 2 巡目で検証する。
        schedule.run(&mut world);
        SetWindowPosCommand::flush();
        schedule.run(&mut world);
        SetWindowPosCommand::flush();
    });

    assert!(
        world.resource::<WindowBandQueue>().ops.is_empty(),
        "帯の系が列を空にしているはず"
    );
    assert!(in_band(character_hwnd), "キャラクターが帯に居るはず");
    assert!(
        in_band(balloon_hwnd),
        "流れた後もバルーンが帯に居るはず（帯の外の観測で決めた是正なら落ちる）: {out}"
    );
    assert!(!in_band(normal), "対照: 指令の外の窓は帯の外のまま");
    assert_eq!(
        measure_window_below(balloon_hwnd),
        Some(character_hwnd),
        "バルーンは帯の中でキャラクターのすぐ手前に居るはず: {out}"
    );
    let fix_lines: Vec<&str> = out.lines().filter(|l| l.contains(FIX_TAG)).collect();
    assert_eq!(fix_lines.len(), 1, "是正の記録は 1 本のはず: {out}");
    // 帯を当てた後の観測なら挿入位置は帯の最上か、C の手前に居る帯の窓。前の観測なら帯の外の N。
    let insert_after = fix_lines[0]
        .split_whitespace()
        .find_map(|field| field.strip_prefix("insert_after="))
        .expect("是正の行に insert_after がある");
    assert_ne!(
        insert_after,
        hwnd_field(Some(normal)),
        "是正が帯を当てる前の観測（挿入位置 N）で決まっている: {out}"
    );
    if insert_after != "top-edge" {
        let raw = usize::from_str_radix(insert_after.trim_start_matches("0x"), 16)
            .expect("挿入位置は top-edge か 16 進の HWND");
        assert!(
            in_band(HWND(raw as *mut core::ffi::c_void)),
            "是正の挿入位置は帯の窓のはず（帯を当てた後の観測）: {out}"
        );
    }
    assert_eq!(
        out.lines()
            .filter(|l| l.contains(VERIFY_FAILED_TAG))
            .count(),
        0,
        "検証は通るはず: {out}"
    );

    for hwnd in ours {
        destroy(hwnd);
    }
}

//! 重なり管理の結線（[`wire_zorder_pair`](super::wire_zorder_pair)・task 3.2）を固定する
//! ヘッドレステスト。
//!
//! 本ファイルが固定するのは 3 つである。
//!
//! - **実行時ストラテジが案 A・補助浮上なしで入ること**（要件 5.6）。既定値まかせではなく
//!   結線が値を明示して入れているかを見る。
//! - **確立系と維持系がどちらも確定段（`FrameFinalize`）に載ること**（要件 1.1）。
//! - **同じ 1 巡のうちに確立系 → 維持系の順で走ること**（要件 1.1）。確立系が挿した
//!   再断行の要求を、同じ巡の維持系が受け取れるかどうかがここの主題である。
//!
//! # なぜ本物の窓を作るのか
//!
//! 確立系は `SetWindowLongPtrW(GWLP_HWNDPARENT)` を実際に呼ぶ。偽のハンドルでは必ず
//! 失敗し、要求が挿されないため順序を確かめようがない。一方、**自プロセスが持つ 2 枚の
//! 窓の間の owner 関係は決定論的に成立する**（wintf 側の確立系テストと同じ判断）。
//! よって本ファイルは非表示のトップレベル窓を 2 枚作り、OS 側の事実
//! （`GetWindow(GW_OWNER)`）で確立を確かめる。
//!
//! 重なりの**隣接**は他プロセスの窓に割り込まれるため決定論的ではない。ゆえに本ファイルは
//! 隣接の値を一切検査しない——それは実機サインオフ（ゲート G6）の担当である。
//!
//! # 順序の主張に赤を用意する
//!
//! 「同じ巡で消費された」の主張は、消費の判定が甘ければ順序が逆でも通ってしまう。よって
//! 逆順に載せた対照を同じファイルに置き、そちらでは要求が未消費のまま残ることを固定する
//! （＝主張が実際に赤になりうることの較正）。

use bevy_ecs::prelude::*;
use bevy_ecs::schedule::{IntoScheduleConfigs, Schedules};
use tracing::Level;
use windows::Win32::Foundation::{HINSTANCE, HWND};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DestroyWindow, GW_OWNER, GetWindow, WINDOW_EX_STYLE, WINDOW_STYLE, WS_POPUP,
};
use windows::core::{PCWSTR, w};
use wintf::ecs::{
    FrameFinalize, KeepDirectlyAbove, ReassertZOrder, WindowHandle, ZOrderPairStrategy,
    apply_zorder_pair_maintenance, establish_owner_links,
};

use super::wire_zorder_pair;
use crate::placement::diag::zorder_pair_strategy_line;
use crate::placement::test_support::capture_logs;

// -------------------------------------------------------------------------
// 実窓・World のヘルパ
// -------------------------------------------------------------------------

/// 自プロセス所有の非表示トップレベル窓を作る（wintf 側の確立系テストと同一レシピ）。
///
/// owner 関係はトップレベル窓の間でのみ意味を持つため `WS_POPUP`（親なし）で作る。
fn create_test_window(title: PCWSTR) -> HWND {
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    // SAFETY: Win32 境界。定義済 "Static" クラスで非表示ウィンドウを生成する。
    unsafe {
        let hinstance = GetModuleHandleW(None).expect("GetModuleHandleW");
        CreateWindowExW(
            WINDOW_EX_STYLE(0),
            w!("Static"),
            title,
            WINDOW_STYLE(WS_POPUP.0),
            0,
            0,
            0,
            0,
            None,
            None,
            Some(hinstance.into()),
            None,
        )
        .expect("CreateWindowExW should create a hidden test window")
    }
}

/// 生成したテスト窓を破棄する（後始末）。
fn destroy_test_window(hwnd: HWND) {
    // SAFETY: Win32 境界。生成した所有ウィンドウを破棄する。
    unsafe {
        let _ = DestroyWindow(hwnd);
    }
}

/// OS 側が持つ owner を読み戻す（`GetWindow(GW_OWNER)`）。owner 無しは `None`。
fn owner_of(hwnd: HWND) -> Option<HWND> {
    // SAFETY: Win32 境界。読み取りのみ。
    unsafe { GetWindow(hwnd, GW_OWNER) }.ok()
}

/// ハンドル付きの窓 entity を作る。
fn spawn_window(world: &mut World, hwnd: HWND) -> Entity {
    world
        .spawn(WindowHandle {
            hwnd,
            instance: HINSTANCE::default(),
        })
        .id()
}

/// 実窓 2 枚とペア宣言済みの World を組む。返すのは（World, バルーン entity, 両ハンドル）。
///
/// `Schedules` は結線の前提（本番の `EcsWorld` 内 World には既在）ゆえここで入れておく。
fn world_with_declared_real_pair() -> (World, Entity, HWND, HWND) {
    let char_hwnd = create_test_window(w!("areka zorder pair wiring char"));
    let balloon_hwnd = create_test_window(w!("areka zorder pair wiring balloon"));

    let mut world = World::new();
    world.init_resource::<Schedules>();
    let char_window = spawn_window(&mut world, char_hwnd);
    let balloon_window = spawn_window(&mut world, balloon_hwnd);
    world
        .entity_mut(balloon_window)
        .insert(KeepDirectlyAbove { peer: char_window });

    (world, balloon_window, char_hwnd, balloon_hwnd)
}

/// 「再断行の要求が同じ巡で維持系に届いたか」を判定する。
///
/// 届いていれば、見送りなら要求そのものが外れ、是正なら検証待ちの段階へ進む。どちらでも
/// ない「挿さったままの未適用」だけが、届いていないことを表す。
fn request_was_consumed(world: &World, balloon_window: Entity) -> bool {
    match world.entity(balloon_window).get::<ReassertZOrder>() {
        None => true,
        Some(request) => request.pending_verify.is_some(),
    }
}

// -------------------------------------------------------------------------
// ストラテジ Resource
// -------------------------------------------------------------------------

/// 結線は実行時ストラテジを**案 A・補助浮上なし**で明示挿入する（要件 5.6）。
#[test]
fn wire_inserts_plan_a_strategy_resource() {
    let mut world = World::new();
    world.init_resource::<Schedules>();

    assert!(
        world.get_resource::<ZOrderPairStrategy>().is_none(),
        "前提: 結線の前にストラテジは入っていない"
    );

    wire_zorder_pair(&mut world);

    assert_eq!(
        *world.resource::<ZOrderPairStrategy>(),
        ZOrderPairStrategy::OwnerLink {
            raise_assist: false
        },
        "結線が入れる既定は案 A（owner 関係）・補助浮上なし（要件 5.6）"
    );
}

/// 結線は**挿入した当の値**を起動時ログへ 1 行残す（要件 5.6・task 5.1 の観測条項）。
///
/// 実機ゲート（`verification/plan-a-gate.md`）の結論は「案 A・補助浮上なし」であり、
/// 起動時のログを読んだ人がそれを確かめられることが本タスクの受け入れ条件である。
/// ゆえにここでは 2 つを同時に固定する——**字面がゲートの結論そのものであること**と、
/// **その字面が Resource へ入った値から組まれていること**。後者が無いと、片方だけ
/// 書き換えてもテストが緑のまま通り、記録が静かに嘘をつく。
///
/// 捕捉窓に他の記録は出ない（結線が出すのはこの 1 本だけ）ゆえ件数も固定する。
#[test]
fn wire_logs_the_strategy_it_actually_inserted() {
    let mut world = World::new();
    world.init_resource::<Schedules>();

    let (_, events) = capture_logs(|| wire_zorder_pair(&mut world));

    assert_eq!(events.len(), 1, "結線が出す記録はちょうど 1 本: {events:?}");
    assert_eq!(
        events[0].message(),
        zorder_pair_strategy_line(*world.resource::<ZOrderPairStrategy>()),
        "記録が Resource へ入った値から組まれていない（両者が乖離しうる）"
    );
    assert_eq!(
        events[0].message(),
        "[zorder-pair] strategy-selected plan=A mechanism=owner-link raise_assist=false",
        "起動時ログの字面がゲート判定表の結論（案 A・補助浮上なし）と一致しない"
    );
    assert_eq!(events[0].level, Level::DEBUG);
}

// -------------------------------------------------------------------------
// 確定段への登録
// -------------------------------------------------------------------------

/// 結線は確立系・ペア維持系・グループ維持系の 3 本を確定段（`FrameFinalize`）へ載せる
/// （要件 1.1。3 本目は areka-P0-scope-zorder-pinning task 6.1 の追加）。
#[test]
fn wire_registers_the_zorder_systems_into_frame_finalize() {
    let mut world = World::new();
    world.init_resource::<Schedules>();

    assert!(
        !world.resource::<Schedules>().contains(FrameFinalize),
        "前提: 結線の前に確定段は空（本テストの World は wintf の既定システムを持たない）"
    );

    wire_zorder_pair(&mut world);

    let schedules = world.resource::<Schedules>();
    let frame_finalize = schedules
        .get(FrameFinalize)
        .expect("結線後は確定段が存在する");
    assert_eq!(
        frame_finalize.systems_len(),
        3,
        "確定段へ載るのは確立系・ペア維持系・グループ維持系のちょうど 3 本（要件 1.1・task 6.1）"
    );
}

// -------------------------------------------------------------------------
// 同じ 1 巡での確立 → 維持
// -------------------------------------------------------------------------

/// 結線した確定段を 1 巡回すと、owner が張られ、その巡に挿さった再断行の要求が
/// 同じ巡のうちに維持系へ届く（要件 1.1・確立 → 維持の順序）。
#[test]
fn wired_frame_finalize_establishes_owner_and_consumes_the_request_in_one_pass() {
    let (mut world, balloon_window, char_hwnd, balloon_hwnd) = world_with_declared_real_pair();

    wire_zorder_pair(&mut world);
    world.run_schedule(FrameFinalize);

    assert_eq!(
        owner_of(balloon_hwnd),
        Some(char_hwnd),
        "確立系が走り、OS 側にバルーン窓 → キャラ窓の owner が張られている"
    );
    assert!(
        request_was_consumed(&world, balloon_window),
        "確立系が挿した再断行の要求は同じ巡の維持系が受け取る（未適用のまま残らない）"
    );

    destroy_test_window(balloon_hwnd);
    destroy_test_window(char_hwnd);
}

/// 対照（順序の主張の較正）: 同じ 2 本を**逆順**に載せると、要求は未適用のまま 1 巡持ち越される。
///
/// これが赤にならなければ、上のテストは順序を何も見ていないことになる。
#[test]
fn reversed_order_leaves_the_request_unconsumed_for_one_pass() {
    let (mut world, balloon_window, char_hwnd, balloon_hwnd) = world_with_declared_real_pair();

    world.insert_resource(ZOrderPairStrategy::OwnerLink {
        raise_assist: false,
    });
    world.resource_mut::<Schedules>().add_systems(
        FrameFinalize,
        (apply_zorder_pair_maintenance, establish_owner_links).chain(),
    );
    world.run_schedule(FrameFinalize);

    assert_eq!(
        owner_of(balloon_hwnd),
        Some(char_hwnd),
        "前提: 逆順でも確立そのものは同じ巡で済む（違うのは要求が届く巡だけ）"
    );
    assert!(
        !request_was_consumed(&world, balloon_window),
        "逆順では要求が未適用のまま残る＝上のテストの主張は順序を実際に見ている"
    );

    destroy_test_window(balloon_hwnd);
    destroy_test_window(char_hwnd);
}

//! 逆転・非隣接の鎖を**実窓 4 枚**へ本番の確立系と鎖の適用系で書き、前面走査の可視の
//! 並びが宣言どおりになることと、解除で鎖が無くなったとき各スコープが組へ戻ることを
//! 固定するテスト（`areka-P0-zorder-scope-inversion` 要件 1.2・2.2・4.5・8.3）。
//!
//! 兄弟の [`zorder_chain_apply_release_tests`](super::zorder_chain_apply_release_tests) は
//! 替え玉で「窓口を何の順に呼んだか」を固定する。本ファイルはその先——**呼んだことが
//! Windows 上で本当に宣言どおりの重なりになるか**——を受け持つ。足場（窓の作り・走査・
//! 後始末）は [`zorder_chain_order_tests`](super::zorder_chain_order_tests) から借りる。
//!
//! # 何を固定するか（2 本・同じ手順を計画だけ替えて回す）
//!
//! 1. 組の重なり（`b0` は `s0` の、`b1` は `s1` の直上）から計画 `s0,b0,b1,s1`（逆転）／
//!    `b1,s0,s1,b0`（非隣接）を公開して 1 巡回すと、所有関係が計画の連続対どおりの
//!    一直線になり、本番の前面走査（[`measure_windows_in_front`]・可視の窓だけを拾う）の
//!    並びが宣言どおりになる。**不可視の補助窓**（既定の IME 窓と同じ 0x0・不可視・
//!    鎖の窓に所有される窓）を鎖の間に挟んでも読み飛ばされる（設計の R2）。
//! 2. 続けて解除（望む鎖 `None`）を公開して 1 巡回すると、各スコープでバルーンが
//!    キャラクターのすぐ手前へ戻る（要件 4.5）。
//!
//! 始点は組の重なりなので、どちらの計画も**少なくとも 1 組が外れなければ**着かない
//! （組が張られたままでは `s` が自分の `b` より手前に出られない）。始点がそうなって
//! いることは各テストが自己検査する。

use bevy_ecs::prelude::*;
use bevy_ecs::schedule::{Schedule, SingleThreadedExecutor};
use windows::Win32::Foundation::{HINSTANCE, HWND};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SetWindowPos, WINDOW_EX_STYLE,
    WINDOW_STYLE, WS_EX_TOOLWINDOW, WS_POPUP,
};
use windows::core::{PCWSTR, w};

use super::apply_zorder_chain;
use super::zorder_chain_order_tests::{
    create_chain_window, is_visible, owner_shape, teardown, z_shape,
};
use crate::ecs::window::zorder_pair::measure_windows_in_front;
use crate::ecs::window::{
    ChainPlan, ChainSegment, CrossEdge, KeepDirectlyAbove, PairReleased, WindowHandle,
    ZOrderChainPlan, establish_owner_links,
};

/// 平坦な窓の列の添字（`[b0, s0, b1, s1]`）。
const B0: usize = 0;
const S0: usize = 1;
const B1: usize = 2;
const S1: usize = 3;

/// 本番と同じ順序で確立系 → 鎖の適用系を回す schedule（**1 つを使い回す**）。
///
/// 確立系は [`WindowHandle`] の付与を契機に動くので、巡ごとに作り直すと前の巡で見た付与を
/// 新しく見てしまう（兄弟 `zorder_chain_order_lifecycle_tests.rs` と同じ規律）。
fn schedule() -> Schedule {
    let mut schedule = Schedule::default();
    schedule.set_executor(SingleThreadedExecutor::new());
    schedule.add_systems((establish_owner_links, apply_zorder_chain).chain());
    schedule
}

/// スコープ 2 つ（バルーン＋キャラクター）を作る。返すのは（窓の列, entity の列）で、
/// どちらも `[b0, s0, b1, s1]` の並び。バルーンには本番と同じ組の宣言を付ける。
fn spawn_scopes(world: &mut World, title: PCWSTR) -> (Vec<HWND>, Vec<Entity>) {
    let mut set = Vec::new();
    let mut entities = Vec::new();
    for _ in 0..2 {
        let balloon_hwnd = create_chain_window(title);
        let character_hwnd = create_chain_window(title);
        let character = world
            .spawn(WindowHandle {
                hwnd: character_hwnd,
                instance: HINSTANCE::default(),
            })
            .id();
        let balloon = world
            .spawn((
                WindowHandle {
                    hwnd: balloon_hwnd,
                    instance: HINSTANCE::default(),
                },
                KeepDirectlyAbove { peer: character },
            ))
            .id();
        set.extend([balloon_hwnd, character_hwnd]);
        entities.extend([balloon, character]);
    }
    (set, entities)
}

/// 手前から奥へ `order`（平坦な列の添字）の鎖を公開する。繋ぎは連続対のうち
/// 「同じスコープの `[バルーン, キャラクター]`」を除く全部（合成の規則と同じ）。
fn publish_chain(world: &mut World, entities: &[Entity], order: &[usize]) {
    let cross_edges = order
        .windows(2)
        .filter(|p| !(p[0] % 2 == 0 && p[1] == p[0] + 1))
        .map(|p| CrossEdge {
            owned: entities[p[0]],
            owner: entities[p[1]],
            segment: ChainSegment::Group(0),
        })
        .collect();
    world.insert_resource(ZOrderChainPlan {
        chain: Some(ChainPlan {
            members: order.iter().map(|i| entities[*i]).collect(),
            cross_edges,
            absent: Vec::new(),
        }),
        dirty: true,
    });
}

/// 不可視の補助窓（既定の IME 窓と同じ 0x0・表示しない・`owner` に所有される）を作る。
fn create_hidden_helper(owner: HWND) -> HWND {
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    // SAFETY: Win32 境界。自プロセス所有の 0x0 窓を生成する（表示しない）。
    unsafe {
        let hinstance = GetModuleHandleW(None).expect("GetModuleHandleW");
        CreateWindowExW(
            WINDOW_EX_STYLE(WS_EX_TOOLWINDOW.0),
            w!("Static"),
            w!("zorder-chain-inversion/hidden-helper"),
            WINDOW_STYLE(WS_POPUP.0),
            10,
            10,
            0,
            0,
            Some(owner),
            None,
            Some(hinstance.into()),
            None,
        )
        .expect("CreateWindowExW should create a hidden helper window")
    }
}

/// `hwnd` を自分の窓 `after` の直後へ置く（Z のみ・活性化しない）。
fn place_after(hwnd: HWND, after: HWND) {
    // SAFETY: Win32 境界。自プロセスの窓の Z のみを動かす。
    unsafe {
        let _ = SetWindowPos(
            hwnd,
            Some(after),
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
        );
    }
}

/// 各スコープの「キャラクターの位置 − バルーンの位置」（`Some(1)` がすぐ手前）。
fn balloon_offsets(shape: &[usize]) -> Vec<Option<isize>> {
    [(B0, S0), (B1, S1)]
        .iter()
        .map(|(b, s)| {
            let b = shape.iter().position(|v| v == b)?;
            let s = shape.iter().position(|v| v == s)?;
            Some(s as isize - b as isize)
        })
        .collect()
}

/// 組の重なりから `order` の計画を当て、続けて解除するまでを実窓で測る。
fn inversion_then_reset(order: [usize; 4], title: PCWSTR) {
    let mut world = World::new();
    let (set, entities) = spawn_scopes(&mut world, title);
    let mut schedule = schedule();
    let all_visible = set.iter().all(|h| is_visible(*h));

    // 始点——確立系が組を張る（計画はまだ無い）。
    schedule.run(&mut world);
    let start_owners = owner_shape(&set);
    let start = z_shape(&set);

    // 刺激①——逆転・非隣接の計画。
    publish_chain(&mut world, &entities, &order);
    schedule.run(&mut world);
    let landed = z_shape(&set);
    let owners = owner_shape(&set);

    // 不可視の補助窓を鎖の先頭の直後（＝鎖の間）へ挟む。所有者は根なので所有の制約にも反しない。
    let helper = create_hidden_helper(set[order[3]]);
    place_after(helper, set[order[0]]);
    let mut with_helper = set.clone();
    with_helper.push(helper);
    let raw = z_shape(&with_helper);
    let helper_visible = is_visible(helper);
    // 本番の前面走査（根から手前へ・可視の窓だけ）。
    let front_scan: Vec<usize> = measure_windows_in_front(set[order[3]])
        .windows
        .iter()
        .filter_map(|found| with_helper.iter().position(|w| w == found))
        .collect();

    // 刺激②——解除（望む鎖そのものが無くなる）。
    world.insert_resource(ZOrderChainPlan {
        chain: None,
        dirty: true,
    });
    schedule.run(&mut world);
    let reset = z_shape(&set);
    let reset_owners = owner_shape(&set);
    let marks_left = world
        .query_filtered::<Entity, With<PairReleased>>()
        .iter(&world)
        .count();

    let mut all = set.clone();
    all.push(helper);
    teardown(&all);

    // 足場の自己検査。
    assert!(all_visible, "4 枚が表示状態になっていない");
    assert_eq!(
        start_owners,
        vec![Some(S0), None, Some(S1), None],
        "始点で組が張られていない（以下の比較の前提）"
    );
    assert_ne!(start, order.to_vec(), "始点が既に計画どおり（比較が空虚）");

    // ⑴ 所有関係が計画の連続対どおりの一直線になる（外した組の結び目は残らない）。
    let mut expected_owners = vec![None; 4];
    for p in order.windows(2) {
        expected_owners[p[0]] = Some(p[1]);
    }
    assert_eq!(
        owners, expected_owners,
        "所有関係が計画の一直線でない: {order:?}"
    );
    assert_eq!(
        landed,
        order.to_vec(),
        "実窓の並びが宣言どおりでない（要件 1.2・2.2）"
    );

    // ⑵ 不可視の補助窓は鎖の間に挟まっているが、前面走査は読み飛ばす（R2）。
    assert!(
        !helper_visible,
        "補助窓が可視になっている（檻の前提が崩れた）"
    );
    assert_eq!(
        raw,
        vec![order[0], 4, order[1], order[2], order[3]],
        "補助窓が鎖の間に挟まっていない（読み飛ばしの比較が空虚）"
    );
    assert_eq!(
        front_scan,
        vec![order[2], order[1], order[0]],
        "前面走査の可視の並びが宣言どおりでない、または不可視の補助窓を拾った"
    );

    // ⑶ 解除で各スコープが組へ戻る（要件 4.5）。
    assert_eq!(
        reset_owners,
        vec![Some(S0), None, Some(S1), None],
        "解除で組の結び目が張り直されていない"
    );
    assert_eq!(marks_left, 0, "解除の後も組を外した印が残る");
    assert_eq!(
        balloon_offsets(&reset),
        vec![Some(1), Some(1)],
        "解除の後、バルーンがキャラクターのすぐ手前に無い: {reset:?}"
    );
}

/// 逆転 `s0,b0,b1,s1`（`b0` の組だけが外れる）。
#[test]
fn an_inverted_plan_lands_on_real_windows_and_reset_restores_the_pairs() {
    inversion_then_reset([S0, B0, B1, S1], w!("zorder-chain-inversion/inverted"));
}

/// 非隣接 `b1,s0,s1,b0`（組は 2 つとも外れる）。
#[test]
fn a_split_plan_lands_on_real_windows_and_reset_restores_the_pairs() {
    inversion_then_reset([B1, S0, S1, B0], w!("zorder-chain-inversion/split"));
}

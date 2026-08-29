//! 所有の鎖を**実窓 4 枚**へ本番の適用系で書き、最終的な重なりが宣言どおりに着き、
//! そのあと**崩れない**ことを固定するテスト（要件 1.1／1.2／1.3／7.5／14.1／14.3）。
//!
//! 兄弟の [`zorder_chain_apply_tests`](super::zorder_chain_apply_tests) は偽ハンドルで
//! 「**何を・どの順で**実行環境へ頼んだか」までを固定する。本ファイルが受け持つのはその先
//! ——**頼んだことが Windows 上で本当に宣言どおりの重なりになるか**である。替え玉の檻は
//! 所有関係の書込と後押しの中身までしか見ておらず、「owner の向きを取り違えている」
//! 「後押しの形が再整列を起こさない」といった形は 1 本も赤にしない。
//!
//! # 初版がここで落ちた
//!
//! 改訂第 1 版（毎巡の観測と `SetWindowPos` による是正）は決定論の檻をすべて緑にしたまま
//! 実機で要件 1.1／1.2／1.3 を満たさなかった。原因の 1 つは、当時の実窓の檻が
//! **所有関係を 1 度も張らない窓**（`CreateWindowExW` の `hWndParent` に `None`・
//! 表示すらしない道具窓）の上に建っていて、本番が実際に置かれる配置を再現していなかった
//! ことである。本ファイルの窓は本番のゴースト窓と同じ `WS_POPUP`＋`WS_EX_TOOLWINDOW` で
//! 作り、`ShowWindow(SW_SHOWNOACTIVATE)` で**実際に表示状態**にし、所有関係は
//! **本番の適用系が書く**（テストは 1 本も張らない）。
//!
//! # 何を固定するか（3 本）
//!
//! 1. **成立**——宣言の逆順から始めて本番の適用系を 1 巡回すと、宣言順（手前から
//!    `0,1,2,3`）へ着く。始点が逆順なので 4 枚すべてが動かなければ着かない（要件 1.1／
//!    1.2／14.1）。
//! 2. **攪乱**——最も奥の窓を最前面へ持ち上げても順が保たれる。**これが初版で成立しな
//!    かった主張であり、本 spec の中心**である（要件 1.3／14.3）。是正の巡は 1 度も
//!    回さない——保つのは OS であって、こちらではない。
//! 3. **背面回り**——ゴーストの全窓が鎖の外の窓の背面へ回っても、鎖の中の相対順が
//!    保たれる（要件 7.5）。
//!
//! # 鎖の形（要件 1.2 の射程）
//!
//! 本ファイルの 4 枚は**キャラ窓 1 枚だけのスコープ 4 つ**とみなす。よって鎖の繋ぎ 3 本は
//! すべて横断 edge であり、本番の適用系が 3 本とも書く。スコープが 2 枚（キャラ窓＋バルーン）
//! を持つときの「バルーンがキャラ窓の直上」は**既存のペア機構**が張る繋ぎであって本 spec は
//! 触らないため、その不変は兄弟 task（4.2）の担当である。ここで固定するのは
//! 「**かたまりを宣言順に並べる**」側——連結された列がそのまま重なりになること——である。
//!
//! # 実窓を使うのに決定論である理由（要件 10.3）
//!
//! 測るのは**この数枚どうしの相対順だけ**である。
//!
//! - 走査（[`relative_z_order`]）は最前面から `GW_HWNDNEXT` で降りながら、自分が作った窓
//!   だけを拾う。**隣接ではなく順序**で見るので、既定の IME 窓のような不可視の隣
//!   （スレッドに 1 個・owner の直上に居座る）が何枚挟まっても添字の列は動かない。
//! - 窓は自プロセスが作った 0x0 の道具窓であり、他のテストも他のアプリもこれらを名指しで
//!   動かさない。指令はすべて `SWP_NOACTIVATE` なので活性化も奪わない。
//! - 助走（[`arrange_z`]）は**自分の窓どうしの相対指定だけ**で組む。`HWND_TOP`／
//!   `HWND_BOTTOM` の絶対帯指定は cargo 3 プロセス同時の regime で檻を不安定にした実績が
//!   あり（`api_owner_chain_probe_tests.rs` の module doc ⑴）、挿入位置に**他プロセスの
//!   窓**（`GW_HWNDPREV` で得たもの）を渡す形は、読み取りと書き込みの間にその窓が消えると
//!   黙って失敗する（同 ⑵）。どちらも使わない。絶対帯指定が残っているのは
//!   **それ自体が測定対象である攪乱の 1 行**だけである。
//! - 始点は生成順に頼らず明示的に組み、その成立をテスト本体が自己検査する——始点が
//!   揃っていなければ以降の比較は空虚だからである。
//!
//! # 主張しないこと
//!
//! **鎖と、鎖の外の窓との前後関係は主張しない。** 鎖は 1 つの塊として動き、後押しの際に
//! 鎖の外の窓を追い越すことがある（周囲の窓の状況に依る）。要件が縛るのは鎖の中の相対順
//! （要件 1.1／1.2／7.5）と**鎖の外どうし**の相対順（要件 6.1／6.2・兄弟 task 4.3）で
//! あって、その 2 群の間ではない。

use bevy_ecs::prelude::*;
use bevy_ecs::schedule::{Schedule, SingleThreadedExecutor};
use windows::Win32::Foundation::{HINSTANCE, HWND};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DestroyWindow, GW_HWNDNEXT, GW_OWNER, GetTopWindow, GetWindow, HWND_TOP,
    IsWindowVisible, SW_SHOWNOACTIVATE, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SetWindowPos,
    ShowWindow, WINDOW_EX_STYLE, WINDOW_STYLE, WS_EX_TOOLWINDOW, WS_POPUP,
};
use windows::core::{PCWSTR, w};

use super::apply_zorder_chain;
use crate::api::clear_window_owner;
use crate::ecs::window::zorder_pair::measure_windows_in_front;
use crate::ecs::window::{
    ChainPlan, ChainSegment, CrossEdge, CrossOwnerLink, WindowHandle, ZOrderChainPlan,
};

/// 鎖の窓数（繋ぎが 3 本＝先頭・中間・根がそれぞれ別の役を負う最小より 1 段深い）。
const CHAIN_SIZE: usize = 4;

/// 宣言どおりの最終形（手前から順の添字）。
const DECLARED: [usize; CHAIN_SIZE] = [0, 1, 2, 3];

/// 始点（宣言の逆順）——4 枚すべてが動かなければ [`DECLARED`] へ着かない配置。
const REVERSED: [usize; CHAIN_SIZE] = [3, 2, 1, 0];

// ---------------------------------------------------------------------------
// 実窓（0x0・可視・トップレベル・本番と同じ拡張スタイル）
// ---------------------------------------------------------------------------

/// 本番のゴースト窓と同じ作りの 0x0 窓を作り、活性化を奪わずに表示状態へ移す。
///
/// 表示状態にするのは、問いが「**既に表示されている窓**の重なりが宣言どおりになるか」
/// だからである。寸法は 0x0 なので画面には 1 ピクセルも出ないが、`WS_VISIBLE` は立ち、
/// OS から見れば可視のトップレベル窓である（本番の走査
/// [`measure_windows_in_front`] は不可視の窓を 1 枚も拾わないので、表示していない窓では
/// 適用系の実測欄が空虚になる＝要件 9.3 の経路が働かない）。
///
/// **表示状態は檻の主張の前提であって、成り行きに任せない。** 走査
/// [`relative_z_order`] は不可視の窓も辿ってしまうため、可視性を測らずにいると
/// `ShowWindow` が落ちても 3 本とも緑のままになる——それは初版が落ちた「表示すらしない窓の
/// 上に建った檻」への静かな逆戻りである。よって各テストは ⑴ 4 枚が実際に可視であること、
/// ⑵ 本番の走査がその 4 枚を拾うこと（1 本目）を自己検査する。
fn create_chain_window(title: PCWSTR) -> HWND {
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    // SAFETY: Win32 境界。自プロセス所有の 0x0 トップレベル窓を生成し、活性化を奪わずに
    // 表示状態へ移す。
    unsafe {
        let hinstance = GetModuleHandleW(None).expect("GetModuleHandleW");
        let hwnd = CreateWindowExW(
            WINDOW_EX_STYLE(WS_EX_TOOLWINDOW.0),
            w!("Static"),
            title,
            WINDOW_STYLE(WS_POPUP.0),
            10,
            10,
            0,
            0,
            None,
            None,
            Some(hinstance.into()),
            None,
        )
        .expect("CreateWindowExW should create a test window");
        let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
        hwnd
    }
}

/// Windows から見て可視か（絵の有無ではなく `WS_VISIBLE` の意味での可視）。
fn is_visible(hwnd: HWND) -> bool {
    // SAFETY: Win32 境界。読み取りのみ。
    unsafe { IsWindowVisible(hwnd) }.as_bool()
}

/// いまその窓を所有している窓（無ければ `None`）。
fn owner_of(hwnd: HWND) -> Option<HWND> {
    // SAFETY: Win32 境界。読み取りのみ。
    unsafe { GetWindow(hwnd, GW_OWNER) }.ok()
}

/// 所有関係を持つ窓だけを外す。
///
/// **owner を持たない窓へ `clear_window_owner` を当ててはならない**——元の owner が無いと
/// 偽の失敗を返す（`api_owner_chain_probe_tests.rs` の実測 5）。破棄の前に必ず外すのは、
/// 所有する窓を壊すと所有される窓も道連れになるためである。
fn unlink_all(windows: &[HWND]) {
    for hwnd in windows {
        if owner_of(*hwnd).is_none() {
            continue;
        }
        // **ここで倒れない**——後始末の途中で panic すると破棄まで進まず、可視で所有関係の
        // 付いた窓がプロセスに残る。3 プロセス同時 × 多数走行では、その残骸が後続の走査に
        // 混ざる。失敗は書き出すだけにして破棄まで必ず進む（檻の主張には使わない）。
        if let Err(err) = clear_window_owner(*hwnd) {
            eprintln!(
                "[zorder-chain-order] 後始末の切離しに失敗しました hwnd={hwnd:?} error={err}"
            );
        }
    }
}

/// 作った窓をすべて破棄する（作った枚数と壊す枚数を必ず揃える）。
fn destroy_all(windows: &[HWND]) {
    for hwnd in windows {
        // SAFETY: Win32 境界。自プロセスが生成した窓を破棄する。
        unsafe {
            let _ = DestroyWindow(*hwnd);
        }
    }
}

/// 所有関係を外してから破棄する（後始末の唯一の入口）。
fn teardown(windows: &[HWND]) {
    unlink_all(windows);
    destroy_all(windows);
}

// ---------------------------------------------------------------------------
// Z の読み取りと助走（順序で測る・絶対帯指定を使わない）
// ---------------------------------------------------------------------------

/// 与えた窓集合だけを Z の上から下へ並べて返す。
///
/// 最前面から `GW_HWNDNEXT` で降りながら、集合に属する窓だけを拾う。**生の 1 歩では
/// 測らない**——不可視の隣が間に挟まるので、隣接ではなく順序で見る。
fn relative_z_order(windows: &[HWND]) -> Vec<HWND> {
    let mut result = Vec::new();
    // SAFETY: Win32 境界。デスクトップ配下の最前面窓を得る読み取り専用 API。
    let mut cursor = unsafe { GetTopWindow(None) }.ok();
    let mut steps = 0usize;
    while let Some(hwnd) = cursor {
        if hwnd.is_invalid() {
            break;
        }
        if windows.contains(&hwnd) && !result.contains(&hwnd) {
            result.push(hwnd);
            if result.len() == windows.len() {
                break;
            }
        }
        steps += 1;
        // 走査が終わらない事態（別プロセスの窓が増え続ける等）で固まらない保険。
        if steps > 100_000 {
            break;
        }
        // SAFETY: Win32 境界。窓ハンドルに対する読み取り専用の走査。
        cursor = unsafe { GetWindow(hwnd, GW_HWNDNEXT) }.ok();
    }
    result
}

/// 組内の並びを**添字の列**（手前から奥）で表す。
fn z_shape(set: &[HWND]) -> Vec<usize> {
    relative_z_order(set)
        .iter()
        .filter_map(|hwnd| set.iter().position(|w| w == hwnd))
        .collect()
}

/// Z のみを動かす素の指令（助走・攪乱専用——**本番の経路ではない**）。
///
/// 挿入位置に渡すのは常に**このテストが作った窓**である（唯一の例外は、攪乱の測定対象
/// そのものである `HWND_TOP`）。他プロセスの窓を渡すと、読み取りと書き込みの間に消えた
/// ときに黙って失敗する。
fn place_after(hwnd: HWND, after: HWND) {
    // SAFETY: Win32 境界。自プロセスの窓の Z のみを動かす（活性化・移動・寸法変更なし）。
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

/// 組を `order`（手前から順の添字）の並びへ揃える助走。
///
/// 自分の窓どうしの相対指定だけで組む（絶対帯指定を使わない理由は module doc）。
fn arrange_z(set: &[HWND], order: &[usize]) {
    for pair in order.windows(2) {
        place_after(set[pair[1]], set[pair[0]]);
    }
}

// ---------------------------------------------------------------------------
// 本番の適用系を 1 巡回す道具立て
// ---------------------------------------------------------------------------

/// 適用系だけを載せた 1 巡分の schedule（**単一スレッドの実行器を明示**）。
///
/// 既定の多スレッド実行器では system が別スレッドで走りうる。実窓の操作は
/// UI スレッド固定（[`NonSendMarker`](bevy_ecs::system::NonSendMarker)）の前提であり、
/// 兄弟の替え玉の檻と同じ規律でここでも明示する。
fn chain_schedule() -> Schedule {
    let mut schedule = Schedule::default();
    schedule.set_executor(SingleThreadedExecutor::new());
    schedule.add_systems(apply_zorder_chain);
    schedule
}

/// 実窓のハンドルを持つ entity を作る。
fn spawn_window(world: &mut World, hwnd: HWND) -> Entity {
    world
        .spawn(WindowHandle {
            hwnd,
            instance: HINSTANCE::default(),
        })
        .id()
}

/// 手前から奥へ並べた entity 列から、連続対をすべて横断 edge にした鎖を公開する。
///
/// 本ファイルの窓は 1 枚だけのスコープ 4 つなので、隣り合わせはすべて本 spec が張る繋ぎ
/// である（同一スコープのペア対は 1 組も無い）。`dirty` を立てる＝内容が変わった巡。
fn publish_full_chain(world: &mut World, members: &[Entity]) {
    let cross_edges: Vec<CrossEdge> = members
        .windows(2)
        .map(|pair| CrossEdge {
            owned: pair[0],
            owner: pair[1],
            segment: ChainSegment::Group(0),
        })
        .collect();
    world.insert_resource(ZOrderChainPlan {
        chain: Some(ChainPlan {
            members: members.to_vec(),
            cross_edges,
            absent: Vec::new(),
        }),
        dirty: true,
    });
}

/// 実窓 4 枚と、それを宣言順の 1 本の鎖として公開した World を用意する。
///
/// 返すのは（窓の列, World, entity の列）。窓の列は宣言順（手前から奥）である。
fn chain_fixture(title: PCWSTR) -> (Vec<HWND>, World, Vec<Entity>) {
    let set: Vec<HWND> = (0..CHAIN_SIZE)
        .map(|_| create_chain_window(title))
        .collect();
    let mut world = World::new();
    let members: Vec<Entity> = set.iter().map(|h| spawn_window(&mut world, *h)).collect();
    publish_full_chain(&mut world, &members);
    (set, world, members)
}

/// 帳簿（本 spec が張った繋ぎ）を、被所有側の添字 → 所有側の添字の対で読み出す。
fn ledger_shape(world: &mut World, members: &[Entity]) -> Vec<(usize, usize)> {
    let mut found: Vec<(usize, usize)> = world
        .query::<(Entity, &CrossOwnerLink)>()
        .iter(world)
        .filter_map(|(owned, link)| {
            let owned_at = members.iter().position(|m| *m == owned)?;
            let owner_at = members.iter().position(|m| *m == link.owner)?;
            Some((owned_at, owner_at))
        })
        .collect();
    found.sort_unstable();
    found
}

/// 所有関係を添字の対で読み出す（`GetWindow(GW_OWNER)`・OS 側の現況）。
fn owner_shape(set: &[HWND]) -> Vec<Option<usize>> {
    set.iter()
        .map(|h| owner_of(*h).and_then(|o| set.iter().position(|w| *w == o)))
        .collect()
}

// ===========================================================================
// ⑴ 成立——宣言の逆順から 1 巡で宣言順へ（要件 1.1／1.2／14.1）
// ===========================================================================

/// 本番の適用系を 1 巡回すと、実窓 4 枚が宣言どおりの重なりへ着く。
///
/// 始点は宣言の**逆順**なので、4 枚すべてが動かなければ着かない。
#[test]
fn a_declared_chain_lands_in_the_declared_order_on_real_windows() {
    let (set, mut world, members) = chain_fixture(w!("zorder-chain-order/lands"));
    let all_visible = set.iter().all(|h| is_visible(*h));

    // 助走——宣言の逆順から始める。
    arrange_z(&set, &REVERSED);
    let start = z_shape(&set);

    // 本番の適用系を 1 巡（所有関係の書込・後押し 1 回・直後の実測はすべてこの中）。
    chain_schedule().run(&mut world);

    let landed = z_shape(&set);
    let owners = owner_shape(&set);
    let ledger = ledger_shape(&mut world, &members);
    // 本番の実測経路（要件 9.3）が実際にこの 4 枚を拾うこと。不可視の窓は 1 枚も
    // 入らない走査なので、表示状態が失われるとここが空になる。
    let production_scan: Vec<usize> = measure_windows_in_front(set[CHAIN_SIZE - 1])
        .windows
        .iter()
        .filter_map(|found| set.iter().position(|w| w == found))
        .collect();

    teardown(&set);

    // 窓の作りの自己検査——不可視の窓の上に建てた檻は、初版が落ちた配置そのものである。
    assert!(
        all_visible,
        "4 枚が表示状態になっていない（不可視の窓では要件 9.3 の実測経路が働かない）"
    );
    // 助走の自己検査——始点が揃っていなければ以下の比較は空虚。
    assert_eq!(
        start,
        REVERSED.to_vec(),
        "始点が宣言の逆順に揃っていない: {start:?}"
    );

    // 鎖が一直線であること（星形でも輪でもない・要件 14.4 の実窓側の裏づけ）。
    assert_eq!(
        owners,
        vec![Some(1), Some(2), Some(3), None],
        "本番の適用系が張った所有関係が一直線の鎖になっていない（末尾が根）"
    );
    assert_eq!(
        ledger,
        vec![(0, 1), (1, 2), (2, 3)],
        "本 spec が張った繋ぎの帳簿が鎖の連続対と一致しない"
    );

    // 最終形——宣言どおり。
    assert_eq!(
        landed,
        DECLARED.to_vec(),
        "本番の適用系を 1 巡回しても宣言順の重なりに着かない（要件 1.1／1.2／14.1）"
    );

    // 本番の走査が鎖の窓を拾うこと——根から手前へ辿るので、近い順に 2,1,0 が並ぶ。
    assert_eq!(
        production_scan,
        vec![2, 1, 0],
        "本番の実測経路が鎖の窓を宣言どおりに拾わない（要件 9.3・不可視の窓は列に入らない）"
    );
}

// ===========================================================================
// ⑵ 攪乱——最も奥の窓を最前面へ持ち上げても順が保たれる
//    （要件 1.3／14.3・**初版で成立しなかった主張＝本 spec の中心**）
// ===========================================================================

/// 収まった鎖は、最も奥の窓を最前面へ持ち上げても宣言順を保つ。
///
/// 持ち上げたあとに**適用系を 1 度も回さない**のが主張の要である。初版は「崩れたら次の巡で
/// 是正する」形だったので、回さなければ崩れたままだった。所有の鎖では OS が保つ。
///
/// 攪乱に絶対帯指定（`HWND_TOP`）を使うのは、これが**測定対象そのもの**だからである
/// ——利用者が最背面の窓を活性化して最前面へ持ち上げた状況の再現であり、助走ではない。
///
/// # 刺激が届いたことを自己検査する（3 本目と同じ規律）
///
/// 健全な状態では**攪乱の前後で鎖の並びが同一**なので、`after_disturb == DECLARED` だけでは
/// **攪乱を出しても出さなくても通る**——刺激の 1 行を消しても緑のままになる。それは
/// 「緑の檻が何も測っていなかった」という初版の失敗そのものの形である。よって鎖の外に
/// 検体窓を 1 枚置き、
///
/// 1. 攪乱**前**に検体窓が最前面（添字 0）に居ること
/// 2. 攪乱**後**に検体窓が最背面（添字 [`CHAIN_SIZE`]）へ移ったこと＝最も奥の窓が最前面へ
///    持ち上がり、**鎖が塊ごとその上へ出た**こと
/// 3. 指令そのものが成功を返したこと（`SetWindowPos` の戻り値を捨てない）
///
/// を採ってから、鎖の中の相対順が `DECLARED` のままであることを主張する。
///
/// **これは要件の主張ではない。** module doc「主張しないこと」のとおり、鎖と鎖の外の窓との
/// 前後関係は要件が縛っておらず、ここでもそれを要件として固定はしない——検体窓を見るのは
/// **刺激が本当に届いたことの自己検査**であり、3 本目の `outsider_before`／`outsider_after`
/// とまったく同じ位置づけである。だから崩れたときの文言も「要件が破れた」ではなく
/// 「刺激が届いていない＝この比較は空虚だ」と名乗る。
#[test]
fn a_settled_chain_holds_its_order_when_the_deepest_window_is_raised_to_the_front() {
    let (set, mut world, _members) = chain_fixture(w!("zorder-chain-order/disturb"));
    let all_visible = set.iter().all(|h| is_visible(*h));

    arrange_z(&set, &REVERSED);
    chain_schedule().run(&mut world);
    let settled = z_shape(&set);

    // 鎖の外の検体窓（刺激が届いたことの目印）。生成したてなので鎖より手前に居る。
    let control = create_chain_window(w!("zorder-chain-order/disturb-control"));
    let with_control: Vec<HWND> = set.iter().copied().chain([control]).collect();
    let control_at = |shape: &[usize]| shape.iter().position(|i| *i == CHAIN_SIZE);
    let before_shape = z_shape(&with_control);
    let control_before = control_at(&before_shape);

    // 攪乱——最も奥の窓（鎖の根）を最前面へ。以降、適用系は 1 度も回さない。
    // SAFETY: Win32 境界。自プロセスの窓の Z のみを動かす。
    let raise_root_ok = unsafe {
        SetWindowPos(
            set[CHAIN_SIZE - 1],
            Some(HWND_TOP),
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
        )
    }
    .is_ok();
    let after_disturb = z_shape(&set);
    let after_shape = z_shape(&with_control);
    let control_after = control_at(&after_shape);

    // もう一度、今度は先頭の窓を最前面へ（既に最前面にいる窓への指令でも崩れないこと）。
    // SAFETY: Win32 境界。自プロセスの窓の Z のみを動かす。
    let raise_head_ok = unsafe {
        SetWindowPos(
            set[0],
            Some(HWND_TOP),
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
        )
    }
    .is_ok();
    let after_second_disturb = z_shape(&set);

    teardown(&set);
    destroy_all(&[control]);

    assert!(
        all_visible,
        "4 枚が表示状態になっていない（不可視の窓では要件 9.3 の実測経路が働かない）"
    );
    assert_eq!(
        settled,
        DECLARED.to_vec(),
        "攪乱の前に鎖が宣言順へ収まっていない（この比較の前提）: {settled:?}"
    );

    // 刺激が届いたことの自己検査——要件の主張ではなく、以降の比較が空虚でないことの証拠。
    assert!(
        raise_root_ok,
        "最も奥の窓を最前面へ持ち上げる指令そのものが失敗した（攪乱が起きていない）"
    );
    assert!(
        raise_head_ok,
        "先頭の窓を最前面へ持ち上げる指令そのものが失敗した（2 度目の攪乱が起きていない）"
    );
    assert_eq!(
        control_before,
        Some(0),
        "攪乱の前に検体窓が鎖より手前に居ない（この位置から始めないと刺激の有無が見えない）: {before_shape:?}"
    );
    assert_eq!(
        control_after,
        Some(CHAIN_SIZE),
        "攪乱で鎖が検体窓より手前へ出ていない＝刺激が届いておらず、以下の比較は空虚である: {after_shape:?}"
    );

    assert_eq!(
        after_disturb,
        DECLARED.to_vec(),
        "最も奥の窓を最前面へ持ち上げると鎖の順が崩れた（要件 1.3／14.3・是正の巡は回していない）"
    );
    assert_eq!(
        after_second_disturb,
        DECLARED.to_vec(),
        "先頭の窓を最前面へ持ち上げると鎖の順が崩れた（要件 1.3／14.3）"
    );
}

// ===========================================================================
// ⑶ 背面回り——鎖の外の窓の背面へ全窓が回っても相対順は保たれる（要件 7.5）
// ===========================================================================

/// ゴーストの全窓が鎖の外の窓の背面へ回っても、鎖の中の相対順は保たれる。
///
/// 「他のアプリケーション」の役は、**鎖に属さない検体窓 1 枚**が務める。z 順から見れば
/// 他プロセスの窓と同じ「鎖の外の窓」であり、自プロセスの窓なので走査の途中で消えない
/// （挿入位置に渡しても黙って失敗しない）。
///
/// 背面へ回す操作も自分の窓どうしの相対指定で行う——鎖の先頭を検体窓の直後へ落とすと、
/// 鎖は 1 つの塊として下がる。**鎖と検体窓の間の前後関係は主張しない**（module doc）。
///
/// # 遷移が本当に起きたことを両側から挟む
///
/// 検体窓は生成したてだと最前面側に居るので、**そこから測ると初期状態を追認するだけ**に
/// なり、「背面に**回っても**」の遷移が 1 度も起きない。よって検体窓はまず鎖の根の直後
/// ——つまり全窓より奥——へ落としてから始め、
///
/// 1. 遷移**前**に検体窓が最背面（添字 [`CHAIN_SIZE`]）に居ること
/// 2. 遷移**後**に検体窓が最前面（添字 0）に居ること＝ゴーストの全窓が背面へ回ったこと
/// 3. そのとき鎖の中の相対順が宣言どおりのままであること（要件 7.5）
///
/// の 3 つで挟む。⑴⑵ が無いと、背面へ落とす 1 行を消しても緑のままになる。
#[test]
fn a_chain_holds_its_relative_order_when_every_window_falls_behind_an_outsider() {
    let (set, mut world, _members) = chain_fixture(w!("zorder-chain-order/behind"));
    let all_visible = set.iter().all(|h| is_visible(*h));

    arrange_z(&set, &REVERSED);
    chain_schedule().run(&mut world);
    let settled = z_shape(&set);

    // 鎖の外の窓（他アプリの窓の役）。生成したてでは最前面側に居るので、まず鎖の根の
    // 直後——全窓より奥——へ落とす。挿入位置は自分の窓なので助走の禁則に触れない。
    let outsider = create_chain_window(w!("zorder-chain-order/outsider"));
    place_after(outsider, set[CHAIN_SIZE - 1]);

    // 検体窓を含めた 5 枚での並び（添字 CHAIN_SIZE が検体窓）。
    let with_outsider: Vec<HWND> = set.iter().copied().chain([outsider]).collect();
    let outsider_at = |shape: &[usize]| shape.iter().position(|i| *i == CHAIN_SIZE);
    let before_shape = z_shape(&with_outsider);
    let outsider_before = outsider_at(&before_shape);

    // ゴーストの全窓を検体窓の背面へ。鎖の先頭を検体窓の直後へ落とすだけでよい
    // （鎖は塊として動く）。以降、適用系は 1 度も回さない。
    place_after(set[0], outsider);

    let after_behind = z_shape(&set);
    let after_shape = z_shape(&with_outsider);
    let outsider_after = outsider_at(&after_shape);

    teardown(&set);
    destroy_all(&[outsider]);

    assert!(
        all_visible,
        "4 枚が表示状態になっていない（不可視の窓では要件 9.3 の実測経路が働かない）"
    );
    assert_eq!(
        settled,
        DECLARED.to_vec(),
        "背面へ回す前に鎖が宣言順へ収まっていない（この比較の前提）: {settled:?}"
    );
    // 自己検査①——始点で検体窓が最背面に居ること。ここが崩れると「背面へ回る」遷移が
    // 起きようがなく、以下の比較は初期状態の追認に堕ちる。
    assert_eq!(
        outsider_before,
        Some(CHAIN_SIZE),
        "始点で検体窓が全窓より奥に居ない: {before_shape:?}"
    );
    // 自己検査②——遷移の結果として全窓が検体窓の背面へ回ったこと。
    assert_eq!(
        outsider_after,
        Some(0),
        "ゴーストの全窓が検体窓の背面へ回っていない（この状況で測らなければ要件 7.5 の検査にならない）: {after_shape:?}"
    );
    assert_eq!(
        after_behind,
        DECLARED.to_vec(),
        "全窓が鎖の外の窓の背面へ回ると鎖の中の相対順が崩れた（要件 7.5）"
    );
}

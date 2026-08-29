//! 未指定スコープの**後方参加**を実窓で固定するテスト（要件 15.1／15.2）。
//!
//! # なぜ areka 側に置くのか（実窓の檻の中で本ファイルだけが例外である理由）
//!
//! 実窓の檻 3 ファイルのうち 2 つ（`zorder_chain_order_tests.rs`／
//! `zorder_chain_order_lifecycle_tests.rs`／`zorder_chain_order_outsider_tests.rs`）は
//! wintf に在る。それらが測るのは「**渡された並びを実窓へ書けるか**」であり、
//! 並びそのものは檻が手で組んでよい。
//!
//! 本ファイルの主題は逆で、**並びの導出そのもの**である——「どのグループにも属さない
//! スコープを、全グループの後ろへ、スコープ ID の昇順で連ねる」という要件 15 の規則は
//! [`compose_chain`](super::compose_chain) が持っており、これは areka の関数である。
//! wintf は areka を import できない（既存規律）ので、wintf 側で同じ主題を書こうとすると
//! **檻が期待する並びを檻自身が組む**ことになり、測っているのは自分の入力だけになる。
//! それは本 spec が 4.1 で 2 度差し戻された欠陥（檻が自分の前提の破壊を生き延びる）と
//! 同じ形である。よって本ファイルだけは areka に置き、
//!
//! **タグの語彙 → 台帳 → 合成（areka）→ 適用系（wintf）→ 実窓の重なり**
//!
//! を 1 本に繋いで測る。
//!
//! # 入力は必ず**非昇順**で与える（task 2.1 のレビューの申し送り）
//!
//! 決定論の檻のうち「スコープ ID 昇順」を実際に主張できているのは 1 本だけで、他は
//! 入力が既に昇順だったため昇順性を 1 度も測っていなかった。同じ空虚を実窓で繰り返さない
//! ため、本ファイルは
//!
//! - 在庫のスコープを `[9, 4, 2, 1]` の**非昇順**で与える
//! - 実窓の助走を宣言の**逆順**から始める（＝スコープ 9 がスコープ 2 より**手前**にある
//!   状態から始める）
//!
//! ようにした。よって「昇順に並べ替える」処理が抜ければ、合成の結果も実窓の重なりも
//! 期待から外れて赤くなる。
//!
//! # 全体が ID 順に並んでいるだけ、では通らない配置にしてある
//!
//! グループの指定は `[4, 1]`（**ID の降順**）である。もし実装が全窓をスコープ ID で
//! 並べていれば結果は `1,2,4,9` になるが、要件が求める並びは
//! **`4,1`（登記どおりのグループ）→ `2,9`（未指定を昇順）** である。この配置は
//! 「グループは登記順・未指定だけが昇順」という 2 つの規則を同時に測る。
//!
//! # 刺激が届いたことを両側から挟む
//!
//! 助走で実際に逆順が組めていること・1 巡のあと重なりが**動いた**こと・所有の鎖が
//! OS 上に立ったことを先に採ってから、並びの主張を書く。これが無ければ
//! 「合成も適用も 1 度も走っていない」状態でも同じ緑になる。
//!
//! # 始点の選び方についての申し送り（後押しの空振り＝task 2.3 の担当）
//!
//! 始点を**ちょうど宣言の逆順**（`[7,6,…,0]`）に組み、かつ窓を添字の降順で生成すると、
//! 所有の鎖は正しく（一直線に）立つのに**後押しが重なりを 1 枚も動かさない**——決定論的に
//! 5 走行とも同じであった。機構は実測で判っている:
//!
//! 1. 後押しは `SetWindowPos(members[0], InsertAfter(members[1]))` である。
//!    `GetWindow(members[1], GW_HWNDNEXT) == members[0]`——すなわち**要求する挿入位置が
//!    現在位置と同一**——のとき、Windows は Z の変更が既に満たされていると見て何もせず、
//!    **所有関係の再強制も走らない**。完全な空振りである。
//! 2. 兄弟 `zorder_chain_order_outsider_tests.rs` の 1 本目が同じ形で収まるのは、
//!    そこでは `members[1]` と `members[0]` の間に**スレッド既定の不可視 IME 窓**
//!    （`class="IME"`・`visible=false`）が挟まっていて**隣接していない**からである
//!    （`GW_HWNDNEXT` 一致にならないので後押しは本物の Z 変更になる）。
//! 3. 本番の起動直後の配置がまさに⑴の引き金を引きうる——各スコープはバルーンを先に生成し
//!    （`spawn.rs:419,456`）、`SCOPE_BLOCK = [Balloon, Char]` なので `members[0]` は
//!    バルーン・`members[1]` はその相棒キャラ窓になる。
//!
//! **これは本番の欠陥であり、是正は追加 task 2.3「後押しの空振りを塞ぐ」の担当である**
//! （`tasks.md`）。射程は `nudge_command` と `apply_zorder_chain`——どちらも本 task の
//! 境界の外なので、ここでは 1 行も直していない。
//!
//! 本ファイルはその状態を避けて始点を選んでいる（[`SEED`]）。始点を選び直したのは
//! 「赤が出るまで入力をいじった」のではなく、**測りたい主題（要件 15 の並びの導出）へ
//! 別の欠陥を巻き込まないため**である。**前提の自己検査（`GW_HWNDNEXT != members[0]`）は
//! 敢えて置かない**——それを檻に書くと「偶然に乗って緑」という状態を固定してしまう。
//! 2.3 が引き金の配置そのもので収まることを示すほうが厳密に強い。
//!
//! # 主張しないこと
//!
//! 鎖と、鎖の外に居る窓（他アプリケーションの窓）との前後関係は主張しない
//! （`research.md` §12.5・DD-3b）。測るのは本ファイルが作った 8 枚どうしの相対順だけである。

use bevy_ecs::prelude::*;
use bevy_ecs::schedule::{Schedule, SingleThreadedExecutor};
use windows::Win32::Foundation::{HINSTANCE, HWND};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DestroyWindow, GW_HWNDNEXT, GW_OWNER, GetTopWindow, GetWindow,
    IsWindowVisible, SW_SHOWNOACTIVATE, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SetWindowPos,
    ShowWindow, WINDOW_EX_STYLE, WINDOW_STYLE, WS_EX_TOOLWINDOW, WS_POPUP,
};
use windows::core::{PCWSTR, w};
use wintf::ecs::window::{
    ChainSegment, ZOrderChainPlan, apply_zorder_chain, establish_owner_links,
};
use wintf::ecs::{KeepDirectlyAbove, WindowHandle};

use super::compose_chain;
use crate::placement::zorder_group_ledger::{
    GroupElement, GroupWindowKind, ZOrderGroupLedger, parse_zorder_tokens,
};

// ---------------------------------------------------------------------------
// 配置（期待する最終形をそのまま添字の意味にする）
// ---------------------------------------------------------------------------

/// グループが名前を挙げるスコープ（**指定の順＝手前から**・ID の降順にしてある）。
const GROUP_SCOPES: [u32; 2] = [4, 1];

/// どのグループにも属さないスコープ（**期待する後方参加の順＝スコープ ID の昇順**）。
const TAIL_SCOPES: [u32; 2] = [2, 9];

/// 在庫として合成へ渡すスコープ列。**非昇順**であり、グループの分も混ざっている。
///
/// 昇順に整えるのも、名前の挙がったスコープを除くのも、合成の仕事である
/// （module doc「入力は必ず非昇順で与える」）。
const INVENTORY_SCOPES: [u32; 4] = [9, 4, 2, 1];

/// 実窓の枚数（4 スコープ × バルーン窓とキャラ窓）。
const WINDOWS: usize = 8;

/// 添字 → スコープ（期待する最終形の並び。手前から `b4,s4,b1,s1,b2,s2,b9,s9`）。
const EXPECTED_SCOPE_AT: [u32; WINDOWS] = [4, 4, 1, 1, 2, 2, 9, 9];

/// 助走の始点（手前から順の添字）。**期待する最終形とあらゆる群で食い違わせてある。**
///
/// 手前から `b9 s9 | b2 s2 | b4 s4 | b1 s1`——すなわち
///
/// - 未指定スコープのかたまりが**全グループより手前**に居る（要件 15.1 の主張と正反対）
/// - 未指定どうしが**スコープ ID の降順**（`9` が `2` より手前・要件 15.2 の主張と正反対）
/// - グループどうしも指定の**逆順**（`1` が `4` より手前）
///
/// 生成したばかりの窓は最前面に来るので、素の並びは添字の降順 `[7..0]` である。この始点は
/// それとも期待する最終形とも違うため、**助走の 1 行を落とせば始点の自己検査が赤になる**
/// ——生成順がたまたま始点と同じだと、助走は 1 度も測られない飾りになってしまう。
const SEED: [usize; WINDOWS] = [6, 7, 4, 5, 0, 1, 2, 3];

/// 添字 → 窓種別（偶数がバルーン窓・奇数がキャラ窓＝「バルーンはキャラ窓の直上」）。
fn kind_at(index: usize) -> GroupWindowKind {
    if index % 2 == 0 {
        GroupWindowKind::Balloon
    } else {
        GroupWindowKind::Char
    }
}

/// 添字 → 要素。
fn element_at(index: usize) -> GroupElement {
    GroupElement {
        scope: EXPECTED_SCOPE_AT[index],
        kind: kind_at(index),
    }
}

// ---------------------------------------------------------------------------
// 実窓（0x0・可視・トップレベル・本番のゴースト窓と同じ拡張スタイル）
// ---------------------------------------------------------------------------

/// 本番のゴースト窓と同じ作りの 0x0 窓を作り、活性化を奪わずに表示状態へ移す。
///
/// 表示状態にするのは、問いが「**既に表示されている窓**の重なりが宣言どおりになるか」
/// だからである（`research.md` §12 の検体と同じ条件）。寸法は 0x0 なので画面には
/// 1 ピクセルも出ないが、`WS_VISIBLE` は立ち、OS から見れば可視のトップレベル窓である。
fn create_window(title: PCWSTR) -> HWND {
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

/// 作った窓をすべて破棄する（**手前側＝所有される側から順に**）。
///
/// 所有する窓を先に壊すと、所有される窓も道連れで破棄される（`research.md` §12.3）。
/// 本ファイルの窓の列は「添字 `i` が添字 `i+1` に所有される」向きに並んでいるので、
/// **添字の昇順で壊せば道連れは 1 度も起きない**——どの窓も、自分の所有者より先に消える。
/// 所有関係を外してから壊す（wintf 側の檻の作法）よりも、こちらのほうが破棄済みハンドルを
/// 1 度も触らずに済む。
fn teardown(windows: &[HWND]) {
    for hwnd in windows {
        // SAFETY: Win32 境界。自プロセスが生成した窓を破棄する。
        unsafe {
            let _ = DestroyWindow(*hwnd);
        }
    }
}

// ---------------------------------------------------------------------------
// Z の読み取りと助走（順序で測る・絶対帯指定を使わない）
// ---------------------------------------------------------------------------

/// 与えた窓集合だけを Z の上から下へ並べて返す。
///
/// 最前面から `GW_HWNDNEXT` で降りながら、集合に属する窓だけを拾う。**生の 1 歩では
/// 測らない**——不可視の隣（既定の IME 窓など）が間に挟まるので、隣接ではなく順序で見る。
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

/// Z のみを動かす素の指令（助走専用——**本番の経路ではない**）。
///
/// 挿入位置に渡すのは常に**このテストが作った窓**である。他プロセスの窓を渡すと、
/// 読み取りと書き込みの間にその窓が消えたときに黙って失敗する（`research.md` §12.9 の 2 件目）。
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
/// 自分の窓どうしの相対指定だけで組む（絶対帯指定は cargo 3 プロセス同時の regime で
/// 檻を不安定にした実績がある——`research.md` §12.9 の 1 件目）。
fn arrange_z(set: &[HWND], order: &[usize]) {
    for pair in order.windows(2) {
        place_after(set[pair[1]], set[pair[0]]);
    }
}

/// 所有関係を添字の対で読み出す（`GetWindow(GW_OWNER)`・OS 側の現況）。
fn owner_shape(set: &[HWND]) -> Vec<Option<usize>> {
    set.iter()
        .map(|h| owner_of(*h).and_then(|o| set.iter().position(|w| *w == o)))
        .collect()
}

// ---------------------------------------------------------------------------
// World と本番の 1 巡
// ---------------------------------------------------------------------------

/// 本番と同じ順序で 2 つの system を回す 1 巡（**確立系 → 鎖の適用系**）。
///
/// 同一スコープの「バルーンがキャラ窓の直上」は既存のペア機構
/// （[`establish_owner_links`]）が張る繋ぎであり、本 spec は 1 本も触らない。
/// スコープをまたぐ繋ぎだけを [`apply_zorder_chain`] が張る。
///
/// 実行器は**単一スレッドを明示**する（実窓の操作は UI スレッド固定の前提）。
fn scope_chain_schedule() -> Schedule {
    let mut schedule = Schedule::default();
    schedule.set_executor(SingleThreadedExecutor::new());
    schedule.add_systems((establish_owner_links, apply_zorder_chain).chain());
    schedule
}

// ===========================================================================
// 未指定スコープの後方参加（要件 15.1／15.2）
// ===========================================================================

/// 4 スコープのうち 2 つだけを指定すると、残りのスコープが**全グループの後ろへ・
/// スコープ ID の昇順で**実窓の重なりに現れる。
///
/// # 経路
///
/// タグの語彙 `\![set,zorder,4,1]` に相当するトークン列を
/// [`parse_zorder_tokens`] が解釈し、台帳が 1 グループとして受け、
/// [`compose_chain`](super::compose_chain) が在庫（**非昇順**）と突き合わせて鎖 1 本を組み、
/// 本番の適用系が実窓へ書く。檻が手で組む並びは**助走だけ**である。
///
/// # 期待する最終形（手前から）
///
/// ```text
///   b4 s4 | b1 s1 | b2 s2 | b9 s9
///   └─ グループ（指定の順）─┘└─ 未指定（ID 昇順）─┘
/// ```
///
/// 全窓をスコープ ID で並べたなら `1,2,4,9` になる。要件が求めるのはそれではない。
#[test]
fn unassigned_scopes_join_behind_every_group_in_ascending_scope_order() {
    // --- 実窓と World -------------------------------------------------------
    let set: Vec<HWND> = (0..WINDOWS)
        .map(|_| create_window(w!("zorder-chain-tail/join")))
        .collect();
    let all_visible = set.iter().all(|h| is_visible(*h));

    let mut world = World::new();
    // キャラ窓を先に立ててから、その相棒のバルーン窓へペア宣言を付ける
    // （[`KeepDirectlyAbove`] が既存ペア機構の入口である）。
    let mut entities: Vec<Option<Entity>> = vec![None; WINDOWS];
    for scope_start in (0..WINDOWS).step_by(2) {
        let character = world
            .spawn(WindowHandle {
                hwnd: set[scope_start + 1],
                instance: HINSTANCE::default(),
            })
            .id();
        let balloon = world
            .spawn((
                WindowHandle {
                    hwnd: set[scope_start],
                    instance: HINSTANCE::default(),
                },
                KeepDirectlyAbove { peer: character },
            ))
            .id();
        entities[scope_start] = Some(balloon);
        entities[scope_start + 1] = Some(character);
    }
    let entities: Vec<Entity> = entities.into_iter().map(|e| e.expect("entity")).collect();

    // --- 指定（タグの語彙 → 台帳） -----------------------------------------
    let tokens: Vec<String> = GROUP_SCOPES.iter().map(|s| s.to_string()).collect();
    let token_refs: Vec<&str> = tokens.iter().map(|s| s.as_str()).collect();
    let (members, _normalizations) =
        parse_zorder_tokens(&token_refs).expect("数値モードの 2 スコープ指定は受理されるはず");
    let mut ledger = ZOrderGroupLedger::default();
    let group_id = ledger
        .try_add_tag_group(members)
        .expect("最初の指定は必ず受理される");

    // --- 合成（在庫は非昇順で渡す） -----------------------------------------
    let resolve = |element: &GroupElement| -> Option<Entity> {
        (0..WINDOWS)
            .find(|i| element_at(*i) == *element)
            .map(|i| entities[i])
    };
    let plan = compose_chain(ledger.groups(), &INVENTORY_SCOPES, &resolve)
        .expect("グループが 1 つ在るので既定状態ではない");
    let composed: Vec<Entity> = plan.members.clone();
    let composed_segments: Vec<ChainSegment> =
        plan.cross_edges.iter().map(|edge| edge.segment).collect();

    // --- 助走（未指定が最前・グループは逆順・未指定どうしも降順） ------------
    arrange_z(&set, &SEED);
    let start = z_shape(&set);

    // --- 本番の 1 巡 --------------------------------------------------------
    world.insert_resource(ZOrderChainPlan {
        chain: Some(plan),
        dirty: true,
    });
    scope_chain_schedule().run(&mut world);

    let landed = z_shape(&set);
    let owners = owner_shape(&set);

    teardown(&set);

    // --- 前提の自己検査 -----------------------------------------------------
    assert!(
        all_visible,
        "8 枚が表示状態になっていない（不可視の窓では後押しの実測経路が働かない）"
    );
    assert_eq!(
        start,
        SEED.to_vec(),
        "助走が始点の配置に揃っていない（この配置から始めないと後方参加の並べ替えが見えない）: {start:?}"
    );

    // --- 合成そのもの（要件 15.1／15.2 の導出） -----------------------------
    let expected_members: Vec<Entity> = entities.clone();
    assert_eq!(
        composed, expected_members,
        "非昇順の在庫から組んだ鎖の並びが「グループは登記順・未指定は ID 昇順で後ろ」になっていない（要件 15.1／15.2）"
    );
    assert_eq!(
        composed_segments,
        vec![
            ChainSegment::Group(group_id),
            ChainSegment::Group(group_id),
            ChainSegment::Tail,
        ],
        "後方参加の区間が `Tail` として記録されていない（要件 9.1 の帰属）"
    );

    // --- 刺激が届いたことの自己検査 -----------------------------------------
    assert_eq!(
        owners,
        vec![
            Some(1),
            Some(2),
            Some(3),
            Some(4),
            Some(5),
            Some(6),
            Some(7),
            None
        ],
        "所有の鎖が一直線に立っていない＝合成も適用も届いておらず、以下の比較は空虚である"
    );
    assert_ne!(
        landed, start,
        "1 巡のあとも重なりが助走のままである＝後押しが届いていない"
    );

    // --- 実窓での最終形（要件 15.1／15.2） ----------------------------------
    let declared: Vec<usize> = (0..WINDOWS).collect();
    assert_eq!(
        landed, declared,
        "未指定スコープが全グループの後ろへスコープ ID の昇順で並んでいない（要件 15.1／15.2）: {landed:?}"
    );

    // 群ごとの主張——全体が ID 順に並んだだけでは通らないことを名指しで固定する。
    let position = |index: usize| {
        landed
            .iter()
            .position(|v| *v == index)
            .expect("8 枚すべてが列に居るはず")
    };
    let group_back = (0..4).map(position).max().expect("グループの窓が 4 枚");
    let tail_front = (4..WINDOWS).map(position).min().expect("未指定の窓が 4 枚");
    assert!(
        group_back < tail_front,
        "未指定スコープのかたまりがグループより手前へ入り込んでいる（要件 15.1）: {landed:?}"
    );
    assert!(
        position(4) < position(6),
        "未指定スコープどうしがスコープ ID の昇順になっていない（在庫は {INVENTORY_SCOPES:?} の非昇順で渡した・要件 15.2）: {landed:?}"
    );
    assert!(
        position(0) < position(2),
        "グループの並びが指定の順（{GROUP_SCOPES:?}）になっていない（要件 3.6・全体を ID 順に並べていないか）: {landed:?}"
    );

    // 同一スコープの「バルーンはキャラ窓の直上」（要件 6.3・既存ペア機構の担当）。
    for scope_start in (0..WINDOWS).step_by(2) {
        assert_eq!(
            position(scope_start + 1) as isize - position(scope_start) as isize,
            1,
            "スコープ {} でバルーン窓がキャラ窓の直上に居ない（要件 6.3）: {landed:?}",
            EXPECTED_SCOPE_AT[scope_start]
        );
    }
    // 未指定スコープの側も同じ形で参加していること（要件 15.1 の「かたまりとして」）。
    assert_eq!(
        TAIL_SCOPES,
        [EXPECTED_SCOPE_AT[4], EXPECTED_SCOPE_AT[6]],
        "期待表と後方参加スコープの定義が食い違っている（檻の内部矛盾）"
    );
}

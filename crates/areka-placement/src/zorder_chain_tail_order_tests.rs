//! 未指定スコープの**後方参加**を実窓で固定するテスト。
//!
//! # なぜ areka 側に置くのか（実窓のテストのうち本ファイルと兄弟の
//! `zorder_chain_inversion_order_tests.rs` が例外である理由）
//!
//! 実窓のテストのうち 4 ファイル（`zorder_chain_order_tests.rs`／
//! `zorder_chain_order_lifecycle_tests.rs`／`zorder_chain_order_outsider_tests.rs`／
//! `zorder_chain_order_inversion_tests.rs`）は
//! wintf に在る。それらが測るのは「**渡された並びを実窓へ書けるか**」であり、
//! 並びそのものはテストが手で組んでよい。
//!
//! 本ファイルの主題は逆で、**並びの導出そのもの**である——「どのグループにも属さない
//! スコープを、全グループの後ろへ、スコープ ID の昇順で連ねる」という規則は
//! [`compose_chain`](super::compose_chain) が持っており、これは areka の関数である。
//! wintf は areka を import できない（既存規律）ので、wintf 側で同じ主題を書こうとすると
//! **テストが期待する並びをテスト自身が組む**ことになり、測っているのは自分の入力だけになる。よって本ファイルは areka に置き（兄弟の
//! `zorder_chain_inversion_order_tests.rs` も同じ理由で、計画を本番の台帳と合成に組ませる）、
//!
//! **タグの語彙 → 台帳 → 合成（areka）→ 適用系（wintf）→ 実窓の重なり**
//!
//! を 1 本に繋いで測る。
//!
//! # 入力は必ず**非昇順**で与える
//!
//! 入力が既に昇順だと、昇順に並べ替える処理が抜けても緑のままになる。そこで本ファイルは
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
//! 並べていれば結果は `1,2,4,9` になるが、求める並びは
//! **`4,1`（登記どおりのグループ）→ `2,9`（未指定を昇順）** である。この配置は
//! 「グループは登記順・未指定だけが昇順」という 2 つの規則を同時に測る。
//!
//! # 刺激が届いたことを両側から挟む
//!
//! 助走で実際に逆順が組めていること・1 巡のあと重なりが**動いた**こと・所有の鎖が
//! OS 上に立ったことを先に採ってから、並びの主張を書く。これが無ければ
//! 「合成も適用も 1 度も走っていない」状態でも同じ緑になる。
//!
//! # 始点の選び方
//!
//! 始点が宣言の逆順（`[7,6,…,0]`）と同じだと、後押しの挿入位置が現在位置と同一になって
//! Windows が何もしない。wintf の `nudge_command` はこの形を直してある。本ファイルの始点
//! （[`SEED`]）は期待する最終形と食い違わせ、助走が飾りにならないようにしてある。
//!
//! # 主張しないこと
//!
//! 鎖と、鎖の外に居る窓（他アプリケーションの窓）との前後関係は主張しない。
//! 測るのは本ファイルが作った 8 枚どうしの相対順だけである。

use bevy_ecs::prelude::*;
use bevy_ecs::schedule::{Schedule, SingleThreadedExecutor};
use windows::Win32::Foundation::HWND;
use windows::core::w;
use wintf::ecs::window::{
    ChainSegment, ZOrderChainPlan, apply_zorder_chain, establish_owner_links,
};
use wintf::ecs::{KeepDirectlyAbove, WindowHandle};

use super::compose_chain;
use crate::zorder_group_ledger::{
    GroupElement, GroupWindowKind, ZOrderGroupLedger, parse_zorder_tokens,
};
use crate::zorder_test_support::{
    arrange_z, create_window, is_visible, owner_shape, teardown, z_shape,
};

// ---------------------------------------------------------------------------
// 配置（期待する最終形をそのまま添字の意味にする）
// ---------------------------------------------------------------------------

/// グループが名前を挙げるスコープ（**指定の順＝手前から**・ID の降順にしてある）。
const GROUP_SCOPES: [u32; 2] = [4, 1];

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
/// - 未指定スコープのかたまりが**全グループより手前**に居る（期待と正反対）
/// - 未指定どうしが**スコープ ID の降順**（`9` が `2` より手前・期待と正反対）
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

fn element_at(index: usize) -> GroupElement {
    GroupElement {
        scope: EXPECTED_SCOPE_AT[index],
        kind: kind_at(index),
    }
}

// ---------------------------------------------------------------------------
// World と本番の 1 巡
// ---------------------------------------------------------------------------

/// **確立系 → 鎖の適用系**の 2 つの system だけを順に回す 1 巡。
///
/// 本番の [`wire_zorder_pair`](crate::spawn::wire_zorder_pair) はこの 2 つのあいだに
/// 帯の適用系と組の維持系も挟むが、本ファイルはそれらを回さない。
///
/// 同一スコープの「バルーンがキャラ窓の直上」は既存のペア機構
/// （[`establish_owner_links`]）が張る繋ぎであり、鎖は 1 本も触らない。
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
// 未指定スコープの後方参加
// ===========================================================================

/// 4 スコープのうち 2 つだけを指定すると、残りのスコープが**全グループの後ろへ・
/// スコープ ID の昇順で**実窓の重なりに現れる。
///
/// # 経路
///
/// タグの語彙 `\![set,zorder,4,1]` に相当するトークン列を
/// [`parse_zorder_tokens`] が解釈し、台帳が 1 グループとして受け、
/// [`compose_chain`](super::compose_chain) が在庫（**非昇順**）と突き合わせて鎖 1 本を組み、
/// 本番の適用系が実窓へ書く。テストが手で組む並びは**助走だけ**である。
///
/// # 期待する最終形（手前から）
///
/// ```text
///   b4 s4 | b1 s1 | b2 s2 | b9 s9
///   └─ グループ（指定の順）─┘└─ 未指定（ID 昇順）─┘
/// ```
///
/// 全窓をスコープ ID で並べたなら `1,2,4,9` になるが、求めるのはそれではない。
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
    let mut entities: Vec<Entity> = Vec::with_capacity(WINDOWS);
    for scope_start in (0..WINDOWS).step_by(2) {
        let character = world
            .spawn(WindowHandle {
                hwnd: set[scope_start + 1],
            })
            .id();
        let balloon = world
            .spawn((
                WindowHandle {
                    hwnd: set[scope_start],
                },
                KeepDirectlyAbove { peer: character },
            ))
            .id();
        entities.push(balloon);
        entities.push(character);
    }

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

    // --- 合成そのもの -------------------------------------------------------
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

    // --- 実窓での最終形 -----------------------------------------------------
    let declared: Vec<usize> = (0..WINDOWS).collect();
    assert_eq!(
        landed, declared,
        "実窓の並びが手前から b4,s4,b1,s1,b2,s2,b9,s9 でない（グループは指定の順・未指定は全グループの後ろへ ID 昇順・各スコープはバルーンがキャラ窓の直上・要件 3.6／6.3／15.1／15.2）: {landed:?}"
    );
}

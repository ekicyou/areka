//! 所有の鎖の**出入り**を実窓で固定するテスト——解除・スプライス（差し込みと抜去）・
//! 破棄の非連動（要件 4.1／4.2／6.3／7.1／7.2／11.5／15.4）。
//!
//! 兄弟の [`zorder_chain_order_tests`](super::zorder_chain_order_tests) は
//! 「鎖が**成立して崩れない**」側を固定する。本ファイルが受け持つのはその反対側
//! ——**鎖から出ていくとき・鎖へ入ってくるとき**に、⑴ 束縛が正しく消え、
//! ⑵ 残る窓の順が保たれ、⑶ 巻き添えの破棄が起きないことである。
//! 実窓の足場（窓の作り・順序の走行器・助走・後始末）は兄弟から借りる。
//!
//! # 兄弟との配置の違い——ここは**スコープが 2 枚の窓を持つ**
//!
//! 兄弟の 4 枚は「キャラ窓 1 枚だけのスコープ 4 つ」であり、繋ぎ 3 本はすべて横断 edge
//! だった。本ファイルはそこを 1 段深くして、**バルーン窓＋キャラ窓の 2 枚を持つスコープ**
//! を並べる。鎖はこうなる（手前 → 奥）:
//!
//! ```text
//!   b0 ← s0 ← b1 ← s1        （b=バルーン窓・s=キャラ窓）
//!   └ペア┘ └横断┘ └ペア┘
//! ```
//!
//! - **ペア edge**（`b0 ← s0`）は既存のペア機構（[`establish_owner_links`]）が張る。
//!   本 spec は 1 本も触らない（design.md「Out of Boundary」）。
//! - **横断 edge**（`s0 ← b1`）だけが本 spec の担当であり、
//!   [`apply_zorder_chain`] が張る。
//!
//! したがって本ファイルの schedule は本番と同じ順序——**確立系 → 鎖の適用系**——で 2 つの
//! 本番 system を回す。ペア機構を替え玉にしないのは、要件 6.3
//! 「バルーンがキャラ窓の直上をグループ指定の有無にかかわらず保つ」が、
//! **本物のペア edge の上でしか検査できない**主張だからである。
//!
//! # 「直上」を隣接で測ってよい理由（測り方の但し書き）
//!
//! 走行器 [`z_shape`] は最前面から `GW_HWNDNEXT` で降りながら**このテストが作った窓だけ**を
//! 拾う。既定の IME 窓のような不可視の隣（スレッドに 1 個・owner の直上に居座る）は
//! 拾われないので、返る添字の列に不可視の窓は 1 枚も混じらない。よって
//! 「バルーンがキャラ窓の**直上**」は、生の 1 歩（`GW_HWNDPREV`）ではなく
//! **この列の中の位置の差が 1 であること**として言う（[`balloon_offsets`]）。
//! 生の 1 歩で測ると不可視の隣が偽の失敗を記録する。
//!
//! # 刺激には必ず「届いたこと」の自己検査を両側から挟む（兄弟 4.1 の教訓）
//!
//! 本ファイルの 3 本はいずれも**刺激**（解除する・窓が入って出る・窓を壊す）を中心に置く。
//! 健全な状態では刺激の前後で**重なりの列が同一**になることがあり、事後の主張だけでは
//! **刺激が起きても起きなくても通る**——それは「緑の檻が何も測っていない」形である。
//! よって各テストは、刺激そのものが世界に残した変化を先に採る:
//!
//! | テスト | 刺激 | 刺激が届いたことの証拠 |
//! |---|---|---|
//! | 解除 | 空の鎖を公開して 1 巡 | 横断 edge の所有関係が `Some(..)` → `None` へ落ち、帳簿が空になる。加えて**最奥の窓を持ち上げると今度は順が動く**（束縛が本当に消えた） |
//! | 出入り | 新しいスコープを鎖の途中へ公開／取り下げ | 所有関係の相手が張り替わる（`s0` の owner が `b1` → `b2` → `b1`） |
//! | 破棄 | 実体を落として 1 巡 → 窓を壊す | 去った窓の下流の所有関係が外れ、壊した 2 枚が窓でなくなる |
//!
//! # 主張しないこと
//!
//! **鎖と、鎖の外に居る窓との前後関係は主張しない。** 鎖は 1 つの塊として動き、鎖の外の窓を
//! 追い越すことがある（`research.md` §12.5）。鎖から抜けたスコープについて固定するのは
//! **所有関係**（ペア edge が無傷であること）だけで、重なりの位置は測らない。

use bevy_ecs::prelude::*;
use bevy_ecs::schedule::{Schedule, SingleThreadedExecutor};
use windows::Win32::Foundation::{HINSTANCE, HWND};
use windows::Win32::UI::WindowsAndMessaging::{
    DestroyWindow, HWND_TOP, IsWindow, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SetWindowPos,
};
use windows::core::{PCWSTR, w};

use super::apply_zorder_chain;
use super::zorder_chain_order_tests::{
    arrange_z, create_chain_window, is_visible, ledger_shape, owner_shape, teardown, z_shape,
};
use crate::ecs::window::{
    ChainPlan, ChainSegment, CrossEdge, KeepDirectlyAbove, WindowHandle, ZOrderChainPlan,
    establish_owner_links,
};

// ---------------------------------------------------------------------------
// スコープ（バルーン窓＋キャラ窓の 2 枚）
// ---------------------------------------------------------------------------

/// スコープ 1 つ分の 2 窓（entity）。
///
/// 平坦な窓の列では **添字 `2i` がバルーン窓・`2i+1` がキャラ窓**である。この対応は
/// [`balloon_offsets`] と各テストの期待値が共有する唯一の約束である。
#[derive(Clone, Copy)]
struct Scope {
    /// バルーン窓（手前側・キャラ窓に所有される）。
    balloon: Entity,
    /// キャラ窓（奥側・スコープの根）。
    character: Entity,
}

/// スコープを `count` 個ぶん作る。
///
/// 返すのは（窓の列, スコープの列）。窓の列は `[b0, s0, b1, s1, ...]` の平坦な並びであり、
/// **生成順は重なりの前提にしない**——各テストは [`arrange_z`] で始点を明示的に組み、
/// その成立を自己検査する。
///
/// バルーン窓の entity には本番と同じペア宣言（[`KeepDirectlyAbove`]）を付ける。これが
/// [`establish_owner_links`] の入口であり、ペア edge は**本番の手順で**張られる。
fn spawn_scopes(world: &mut World, count: usize, title: PCWSTR) -> (Vec<HWND>, Vec<Scope>) {
    let mut windows = Vec::with_capacity(count * 2);
    let mut scopes = Vec::with_capacity(count);
    for _ in 0..count {
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
        windows.push(balloon_hwnd);
        windows.push(character_hwnd);
        scopes.push(Scope { balloon, character });
    }
    (windows, scopes)
}

/// 望む鎖を公開する（手前から奥へ `order` の順にスコープを並べる）。
///
/// 横断 edge は**スコープの境目だけ**である——連続するスコープの
/// 「手前側のキャラ窓 ← 奥側のバルーン窓」1 本ずつ。同一スコープ内の
/// 「バルーン ← キャラ」は既存ペア機構の担当なので、計画には**構造上載らない**
/// （載せない努力ではなく、載せる経路がここに無い）。
///
/// 返すのは公開した `members`（帳簿の読み出しで添字を引くのに使う）。
fn publish_scope_chain(world: &mut World, order: &[Scope]) -> Vec<Entity> {
    let mut members = Vec::with_capacity(order.len() * 2);
    for scope in order {
        members.push(scope.balloon);
        members.push(scope.character);
    }
    let cross_edges: Vec<CrossEdge> = order
        .windows(2)
        .map(|pair| CrossEdge {
            owned: pair[0].character,
            owner: pair[1].balloon,
            segment: ChainSegment::Group(0),
        })
        .collect();
    world.insert_resource(ZOrderChainPlan {
        chain: Some(ChainPlan {
            members: members.clone(),
            cross_edges,
            absent: Vec::new(),
        }),
        dirty: true,
    });
    members
}

/// すべてのグループが解除された状態を公開する（要件 4.1／4.2／15.4）。
///
/// 望む鎖そのものが無くなる＝**既定状態への復帰**である。適用系は帳簿の全件を撤去し、
/// 鎖が空なので後押しも出さない（＝並べ替えを起こす経路が無い）。
fn publish_released(world: &mut World) {
    world.insert_resource(ZOrderChainPlan {
        chain: None,
        dirty: true,
    });
}

/// 本番と同じ順序で 2 つの system を回す 1 巡（**確立系 → 鎖の適用系**）。
///
/// 順序は design.md の結線（`establish_owner_links` → ペア維持 → `apply_zorder_chain`）に
/// 従う。ペア維持系を載せないのは、あれが引き金駆動の**是正**の腕であって、本ファイルが
/// 主張する「所有の鎖が構造で保つ」ことの前提ではないためである（維持を担うのは OS）。
///
/// 実行器は**単一スレッドを明示**する。実窓の操作は UI スレッド固定
/// （[`NonSendMarker`](bevy_ecs::system::NonSendMarker)）の前提であり、兄弟の檻と同じ規律。
///
/// **1 つの schedule を使い回すこと**——確立系は [`WindowHandle`] の付与を契機に動くので、
/// 巡ごとに新しい schedule を作ると「前の巡で既に見た付与」を毎回新しく見てしまう。
fn scope_chain_schedule() -> Schedule {
    let mut schedule = Schedule::default();
    schedule.set_executor(SingleThreadedExecutor::new());
    schedule.add_systems((establish_owner_links, apply_zorder_chain).chain());
    schedule
}

// ---------------------------------------------------------------------------
// 測り方（順序で測る・要件 6.3 の「直上」もこの列の中で言う）
// ---------------------------------------------------------------------------

/// 各スコープについて「キャラ窓の位置 − バルーン窓の位置」を返す。
///
/// `Some(1)` が**バルーンがキャラ窓の直上**である（要件 6.3）。どちらかが列に居なければ
/// `None`。列（[`z_shape`]）には不可視の窓が 1 枚も混じらないので、この差はそのまま
/// 「間に可視の窓が何枚あるか＋1」を意味する（module doc「測り方の但し書き」）。
fn balloon_offsets(shape: &[usize], scope_count: usize) -> Vec<Option<isize>> {
    (0..scope_count)
        .map(|scope| {
            let balloon = shape.iter().position(|v| *v == scope * 2);
            let character = shape.iter().position(|v| *v == scope * 2 + 1);
            match (balloon, character) {
                (Some(b), Some(c)) => Some(c as isize - b as isize),
                _ => None,
            }
        })
        .collect()
}

/// 平坦な窓の列の**逆順**（助走の始点。全枚数が動かなければ宣言順へ着かない配置）。
fn reversed(len: usize) -> Vec<usize> {
    (0..len).rev().collect()
}

/// 宣言どおりの並び（手前から `0,1,2,...`）。
fn declared(len: usize) -> Vec<usize> {
    (0..len).collect()
}

/// その窓ハンドルがまだ窓か（破棄されていないか）。
fn is_window(hwnd: HWND) -> bool {
    // SAFETY: Win32 境界。読み取りのみ。破棄済みのハンドルを渡しても安全に false を返す。
    unsafe { IsWindow(Some(hwnd)) }.as_bool()
}

/// Z のみを動かす素の指令で最前面へ持ち上げる（**攪乱専用・本番の経路ではない**）。
///
/// 絶対帯指定（`HWND_TOP`）を使うのは、これが**測定対象そのもの**——利用者が窓を活性化して
/// 最前面へ持ち上げた状況の再現——だからである。助走には使わない（兄弟の module doc）。
fn raise_to_front(hwnd: HWND) -> bool {
    // SAFETY: Win32 境界。自プロセスの窓の Z のみを動かす（活性化・移動・寸法変更なし）。
    unsafe {
        SetWindowPos(
            hwnd,
            Some(HWND_TOP),
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
        )
    }
    .is_ok()
}

// ===========================================================================
// ⑴ 解除——横断 edge の撤去だけで束縛が消え、並べ替えは起きない
//    （要件 4.1／4.2／6.3／15.4）
// ===========================================================================

/// 全グループを解除すると、本 spec が張った繋ぎだけが外れ、重なりは 1 枚も動かない。
///
/// 解除の実測（`research.md` §12.1 の最終行）は「鎖を外しても並べ替えは起きない
/// ——束縛が消えるだけ」である。適用系はこのとき後押しを 1 度も出さない（鎖が空なので
/// 出す相手が居ない）ので、並べ替えを起こす経路そのものが無い。
///
/// # 何を「既定状態の復元」と呼ぶか
///
/// ⑴ 本 spec の帳簿が空になり、⑵ 横断 edge の所有関係が OS 上でも消え、
/// ⑶ **ペア edge は 1 本も欠けない**（要件 6.3 はグループ指定の有無にかかわらず成立する）、
/// ⑷ そのうえで重なりは動かない——の 4 つである。
///
/// # 刺激が届いたことを両側から挟む
///
/// 解除の前後で**重なりの列は同一**なので、事後の列だけを見ると解除しなくても通る。
/// よって解除が世界に残す変化を 2 つ採る:
///
/// 1. **帳簿と所有関係**——`s0` の owner が `b1` から消え、帳簿が空になる（解除前は
///    どちらも `Some(..)`／1 件である）
/// 2. **束縛が本当に消えたこと**——解除の後で最奥のキャラ窓を最前面へ持ち上げると、
///    今度は**そのスコープが手前へ出る**。同じ攪乱を鎖が生きている間に当てても順は
///    動かない（兄弟テストの中心的主張）ので、この 1 行が「もう縛られていない」ことの
///    直接の証拠になる。**これは要件の主張ではない**——既定状態が非強制であること
///    （要件 6.1／6.2）の固定は兄弟 task の担当であり、ここでは刺激の自己検査として
///    のみ読む。
#[test]
fn releasing_every_group_drops_only_the_cross_links_and_reorders_nothing() {
    const SCOPES: usize = 2;
    let mut world = World::new();
    let (set, scopes) = spawn_scopes(&mut world, SCOPES, w!("zorder-chain-lifecycle/release"));
    let all_visible = set.iter().all(|h| is_visible(*h));
    let mut schedule = scope_chain_schedule();

    // 助走——宣言の逆順から始める（全枚数が動かなければ宣言順へ着かない）。
    arrange_z(&set, &reversed(set.len()));
    let start = z_shape(&set);

    let members = publish_scope_chain(&mut world, &scopes);
    schedule.run(&mut world);

    let landed = z_shape(&set);
    let owners_before = owner_shape(&set);
    let ledger_before = ledger_shape(&mut world, &members);
    let pairs_before = balloon_offsets(&landed, SCOPES);

    // 刺激——全グループの解除（descript 由来の基底も無い＝既定状態へ戻る・要件 4.2）。
    publish_released(&mut world);
    schedule.run(&mut world);

    let after_release = z_shape(&set);
    let owners_after = owner_shape(&set);
    let ledger_after = ledger_shape(&mut world, &members);
    let pairs_after = balloon_offsets(&after_release, SCOPES);

    // 束縛が本当に消えたことの自己検査——最奥のキャラ窓を最前面へ。
    let raised_ok = raise_to_front(set[SCOPES * 2 - 1]);
    let after_raise = z_shape(&set);
    let released_scope_moved = match (
        after_raise.iter().position(|v| *v == SCOPES * 2 - 1),
        after_raise.iter().position(|v| *v == 1),
    ) {
        (Some(deepest), Some(first)) => deepest < first,
        _ => false,
    };
    let pairs_after_raise = balloon_offsets(&after_raise, SCOPES);

    teardown(&set);

    assert!(
        all_visible,
        "4 枚が表示状態になっていない（不可視の窓では要件 9.3 の実測経路が働かない）"
    );
    assert_eq!(
        start,
        reversed(set.len()),
        "始点が宣言の逆順に揃っていない: {start:?}"
    );
    assert_eq!(
        landed,
        declared(set.len()),
        "解除の前に鎖が宣言順へ収まっていない（この比較の前提）: {landed:?}"
    );

    // 刺激が届いたことの自己検査①——解除の前は横断 edge も帳簿も在った。
    assert_eq!(
        owners_before,
        vec![Some(1), Some(2), Some(3), None],
        "解除の前に鎖が一直線に張られていない（この比較の前提）"
    );
    assert_eq!(
        ledger_before,
        vec![(1, 2)],
        "解除の前に本 spec の帳簿が横断 edge 1 本を持っていない（この比較の前提）"
    );

    // 解除の結果——横断 edge だけが消え、ペア edge は無傷（要件 4.1／6.3）。
    assert_eq!(
        owners_after,
        vec![Some(1), None, Some(3), None],
        "解除で消えたのが横断 edge 1 本ではない（ペア edge を外した／横断 edge が残った）"
    );
    assert!(
        ledger_after.is_empty(),
        "解除の後も本 spec の帳簿に繋ぎが残っている: {ledger_after:?}"
    );

    // 並べ替えは起きない（要件 4.1 の実測・§12.1 最終行）。
    assert_eq!(
        after_release, landed,
        "解除しただけで重なりが動いた（束縛が消えるだけのはずである）: {after_release:?}"
    );

    // バルーンがキャラ窓の直上——解除の前後で保たれる（要件 6.3）。
    assert_eq!(
        pairs_before,
        vec![Some(1); SCOPES],
        "解除の前にバルーンがキャラ窓の直上に居ない: {landed:?}"
    );
    assert_eq!(
        pairs_after,
        vec![Some(1); SCOPES],
        "解除でバルーンがキャラ窓の直上でなくなった（要件 6.3）: {after_release:?}"
    );

    // 刺激が届いたことの自己検査②——もう縛られていない。
    assert!(
        raised_ok,
        "最奥のキャラ窓を最前面へ持ち上げる指令そのものが失敗した（自己検査が働いていない）"
    );
    assert!(
        released_scope_moved,
        "解除の後なのに最奥のスコープが手前へ出ない＝束縛が消えていない（この比較は空虚である）: {after_raise:?}"
    );
    assert_eq!(
        pairs_after_raise,
        vec![Some(1); SCOPES],
        "既定状態で窓を活性化するとバルーンがキャラ窓の直上でなくなった（要件 6.3）: {after_raise:?}"
    );
}

// ===========================================================================
// ⑵ 出入り——後から現れたスコープが鎖の途中へ入り、抜けると元へ戻る（要件 7.1／6.3）
// ===========================================================================

/// 後から現れたスコープが鎖の**途中**へ差し込まれ、取り下げると元の並びへ戻る。
///
/// 手順は実測（`research.md` §12.4）どおり——**切る 1 本 → 張る 2 本 → 後押し 1 回**。
/// 適用系がこれを差分から自分で導く（撤去がすべて先・付与がすべて後）。
///
/// # 差し込む位置は末尾ではなく「途中」
///
/// 新しいスコープ 2 を、既存のスコープ 0 と 1 の**間**へ入れる。末尾へ足す形だと
/// 「切る 1 本」が現れず、スプライスの主張（途中状態が壊れない）が空洞になる。
///
/// # 刺激が届いたことを両側から挟む
///
/// 差し込みの前後で**所有関係の相手が張り替わる**——`s0` の owner が `b1` → `b2` →
/// （取り下げで）`b1` である。この 3 点を採るので、差し込みか取り下げのどちらかを
/// 落とすと必ず赤くなる。重なりの列も 3 点それぞれで測る。
///
/// # 鎖から抜けたスコープについて主張すること／しないこと
///
/// 抜けたスコープには**ペア edge が無傷で残る**ことだけを所有関係で主張する（要件 6.3）。
/// 鎖の外に居る窓と鎖との重なりの前後は要件が縛っていないので測らない
/// （module doc「主張しないこと」）。
#[test]
fn a_late_scope_splices_into_the_middle_of_the_chain_and_leaves_it_unchanged_on_withdrawal() {
    let mut world = World::new();
    let (mut set, scopes) = spawn_scopes(&mut world, 2, w!("zorder-chain-lifecycle/splice"));
    let all_visible = set.iter().all(|h| is_visible(*h));
    let mut schedule = scope_chain_schedule();

    arrange_z(&set, &reversed(set.len()));
    let start = z_shape(&set);

    publish_scope_chain(&mut world, &scopes);
    schedule.run(&mut world);
    let landed = z_shape(&set);
    let pairs_landed = balloon_offsets(&landed, 2);

    // 後から現れるスコープ（添字 4=バルーン・5=キャラ）。ここではまだ鎖に入っていない。
    let (late_set, late_scopes) = spawn_scopes(&mut world, 1, w!("zorder-chain-lifecycle/late"));
    set.extend_from_slice(&late_set);
    let owners_before_splice = owner_shape(&set);

    // 刺激①——スコープ 0 と 1 の**間**へ差し込む。
    publish_scope_chain(&mut world, &[scopes[0], late_scopes[0], scopes[1]]);
    schedule.run(&mut world);
    let spliced = z_shape(&set);
    let owners_spliced = owner_shape(&set);
    let pairs_spliced = balloon_offsets(&spliced, 3);

    // 刺激②——取り下げ（後から現れたスコープが鎖から抜ける）。
    let members_after = publish_scope_chain(&mut world, &scopes);
    schedule.run(&mut world);
    let chain_set: Vec<HWND> = set[..4].to_vec();
    let withdrawn = z_shape(&chain_set);
    let owners_withdrawn = owner_shape(&set);
    let ledger_withdrawn = ledger_shape(&mut world, &members_after);
    let pairs_withdrawn = balloon_offsets(&withdrawn, 2);

    teardown(&set);

    assert!(
        all_visible,
        "始めの 4 枚が表示状態になっていない（不可視の窓では要件 9.3 の実測経路が働かない）"
    );
    assert_eq!(
        start,
        vec![3, 2, 1, 0],
        "始点が宣言の逆順に揃っていない: {start:?}"
    );
    assert_eq!(
        landed,
        vec![0, 1, 2, 3],
        "差し込みの前に鎖が宣言順へ収まっていない（この比較の前提）: {landed:?}"
    );

    // 刺激①が届いたことの自己検査——差し込みの前は新しいスコープに所有関係が 1 本も無く、
    // `s0` は `b1` に所有されていた。
    assert_eq!(
        owners_before_splice,
        vec![Some(1), Some(2), Some(3), None, None, None],
        "差し込みの前の所有関係が前提どおりでない（新しいスコープはまだ鎖にも居ない）"
    );

    // 差し込みの結果——鎖の途中へ入る（要件 7.1）。
    assert_eq!(
        owners_spliced,
        vec![Some(1), Some(4), Some(3), None, Some(5), Some(2)],
        "差し込みで所有関係が `b0←s0←b2←s2←b1←s1` の一直線にならない"
    );
    assert_eq!(
        spliced,
        vec![0, 1, 4, 5, 2, 3],
        "後から現れたスコープが鎖の途中へ入っていない（要件 7.1）: {spliced:?}"
    );

    // 刺激②が届いたことの自己検査を兼ねた結果——`s0` の owner が `b2` から `b1` へ戻り、
    // 抜けたスコープのペア edge は無傷（要件 6.3）、その横断 edge だけが消える。
    assert_eq!(
        owners_withdrawn,
        vec![Some(1), Some(2), Some(3), None, Some(5), None],
        "取り下げで元の鎖へ戻っていない（`s0` の owner が戻らない／ペア edge を壊した）"
    );
    assert_eq!(
        withdrawn,
        vec![0, 1, 2, 3],
        "取り下げの後に残る鎖が宣言順へ戻っていない（要件 7.1）: {withdrawn:?}"
    );
    assert_eq!(
        ledger_withdrawn,
        vec![(1, 2)],
        "取り下げの後の帳簿が元の横断 edge 1 本だけになっていない: {ledger_withdrawn:?}"
    );

    // バルーンがキャラ窓の直上——3 つの局面すべてで保たれる（要件 6.3）。
    assert_eq!(
        pairs_landed,
        vec![Some(1); 2],
        "差し込みの前にバルーンがキャラ窓の直上に居ない: {landed:?}"
    );
    assert_eq!(
        pairs_spliced,
        vec![Some(1); 3],
        "差し込んだ状態でバルーンがキャラ窓の直上でなくなった（要件 6.3）: {spliced:?}"
    );
    assert_eq!(
        pairs_withdrawn,
        vec![Some(1); 2],
        "取り下げの後にバルーンがキャラ窓の直上でなくなった（要件 6.3）: {withdrawn:?}"
    );
}

// ===========================================================================
// ⑶ 破棄の非連動——鎖の窓を壊しても他スコープの窓が生き残る（要件 7.2／11.5）
// ===========================================================================

/// 鎖に居るスコープの窓を壊しても、他スコープの窓は 1 枚も道連れにならない。
///
/// OS の破棄カスケードは「**所有する窓を壊すと所有される窓も壊す**」向きに働く
/// （`research.md` §12.3）。鎖は所有関係の一直線なので、根を壊せば下流が全滅する
/// ——**先に外しておけば完全に封じられる**。本番でその「先に外す」を担うのが
/// [`apply_zorder_chain`] の手順 1（去る窓の切離し）であり、それは
/// **望む鎖の変化の門より前**に置かれている。窓が去ってから鎖が組み直されて公開される
/// までには少なくとも 1 巡の間があり、その間に破棄が走ると巻き込まれるからである。
///
/// 本テストはその門の位置ごと固定する——去る巡では鎖の内容を**一切公開し直さない**
/// （`dirty` は立たない）。それでも切離しが走ることが、要件 7.2 が成立する条件である。
///
/// # 刺激が届いたことを両側から挟む
///
/// 刺激は 2 つある。⑴ 実体が去って 1 巡回ること、⑵ 窓を実際に壊すこと。
///
/// 1. 去る前は最奥のスコープが鎖に繋がっている（`s1` の owner が `b2`）
/// 2. 去った巡の後は `s1` の owner が消えている＝**切離しが走った**
/// 3. 壊す前は 6 枚すべてが窓である
/// 4. 壊した後は 2 枚が窓でなくなり、残り 4 枚は窓のままである
///
/// ⑵ を落とすと 3・4 が、⑴ を落とすと 2 が——そして道連れが起きて 4 も——赤になる。
///
/// # 要件 11.5 との関係
///
/// 要件 11.5 は「グループの指定・解除が窓へ及ぼす利用者に見える変化を重なり順と
/// その維持だけに限る」と定める。**破棄の連動は「重なり順以外の変化」の代表**であり、
/// ここで封じられていることがその主張の実窓側の裏づけである。
#[test]
fn destroying_a_departing_scope_leaves_the_other_scopes_alive() {
    const SCOPES: usize = 3;
    let mut world = World::new();
    let (set, scopes) = spawn_scopes(&mut world, SCOPES, w!("zorder-chain-lifecycle/destroy"));
    let all_visible = set.iter().all(|h| is_visible(*h));
    let mut schedule = scope_chain_schedule();

    arrange_z(&set, &reversed(set.len()));
    let start = z_shape(&set);

    publish_scope_chain(&mut world, &scopes);
    schedule.run(&mut world);
    let landed = z_shape(&set);
    let owners_before = owner_shape(&set);
    let pairs_landed = balloon_offsets(&landed, SCOPES);

    // 刺激①——最奥のスコープ（鎖の根の側）が去る。鎖の内容は**公開し直さない**
    // ので `dirty` は立たない。それでも切離しは走らなければならない（要件 7.2）。
    world.despawn(scopes[SCOPES - 1].balloon);
    world.despawn(scopes[SCOPES - 1].character);
    schedule.run(&mut world);
    let owners_after_detach = owner_shape(&set);

    // 刺激②——去った窓を壊す。バルーン窓を先に壊すのは、キャラ窓を先に壊すと
    // ペア edge を通じてバルーン窓が道連れになり、2 度目の破棄が失敗するためである
    // （それ自体は正常だが、指令の成否を証拠に使えなくなる）。
    let departing: Vec<HWND> = set[(SCOPES - 1) * 2..].to_vec();
    let all_windows_before_destroy = set.iter().all(|h| is_window(*h));
    // SAFETY: Win32 境界。このテストが生成した窓を破棄する。
    let destroyed_ok = departing
        .iter()
        .all(|hwnd| unsafe { DestroyWindow(*hwnd) }.is_ok());
    let departing_gone = departing.iter().all(|h| !is_window(*h));

    let survivors: Vec<HWND> = set[..(SCOPES - 1) * 2].to_vec();
    let survivors_alive = survivors.iter().all(|h| is_window(*h));
    let survivors_visible = survivors.iter().all(|h| is_visible(*h));
    let survivor_shape = z_shape(&survivors);
    let survivor_owners = owner_shape(&survivors);
    let pairs_survivors = balloon_offsets(&survivor_shape, SCOPES - 1);

    teardown(&set);

    assert!(
        all_visible,
        "6 枚が表示状態になっていない（不可視の窓では要件 9.3 の実測経路が働かない）"
    );
    assert_eq!(
        start,
        reversed(set.len()),
        "始点が宣言の逆順に揃っていない: {start:?}"
    );
    assert_eq!(
        landed,
        declared(set.len()),
        "破棄の前に鎖が宣言順へ収まっていない（この比較の前提）: {landed:?}"
    );
    assert_eq!(
        owners_before,
        vec![Some(1), Some(2), Some(3), Some(4), Some(5), None],
        "破棄の前に 3 スコープが一直線の鎖になっていない（この比較の前提）"
    );
    assert_eq!(
        pairs_landed,
        vec![Some(1); SCOPES],
        "破棄の前にバルーンがキャラ窓の直上に居ない: {landed:?}"
    );

    // 刺激①が届いたことの自己検査——去った窓の下流の繋ぎが外れている。
    assert_eq!(
        owners_after_detach,
        vec![Some(1), Some(2), Some(3), None, Some(5), None],
        "去るスコープへの繋ぎが破棄より先に外れていない＝この後の破棄は道連れを起こす"
    );

    // 刺激②が届いたことの自己検査——実際に壊したこと。
    assert!(
        all_windows_before_destroy,
        "破棄の前に 6 枚すべてが窓ではない（この比較の前提）"
    );
    assert!(
        destroyed_ok,
        "去るスコープの窓を壊す指令そのものが失敗した（刺激が届いていない）"
    );
    assert!(
        departing_gone,
        "去るスコープの窓が破棄されていない（刺激が届いておらず、以下の比較は空虚である）"
    );

    // 破棄が道連れを起こさない（要件 7.2／11.5）。
    assert!(
        survivors_alive,
        "鎖の窓を壊したら他スコープの窓まで破棄された（要件 7.2）"
    );
    assert!(
        survivors_visible,
        "生き残った窓が表示状態を失った（要件 11.5・見える変化は重なり順に限る）"
    );
    assert_eq!(
        survivor_shape,
        declared(survivors.len()),
        "生き残った窓の相対順が指定どおりでない（要件 7.2）: {survivor_shape:?}"
    );
    assert_eq!(
        survivor_owners,
        vec![Some(1), Some(2), Some(3), None],
        "生き残った鎖の所有関係が一直線でない（切離しが余分な繋ぎまで外した）"
    );
    assert_eq!(
        pairs_survivors,
        vec![Some(1); SCOPES - 1],
        "破棄の後にバルーンがキャラ窓の直上でなくなった（要件 6.3）: {survivor_shape:?}"
    );
}

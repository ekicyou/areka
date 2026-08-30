//! 本 spec が**縛らないもの**を実窓で固定するテスト——鎖の外どうしの相対順の不変と、
//! 指定が一つも無い既定状態（要件 6.1／6.2）。
//!
//! 兄弟の [`zorder_chain_order_tests`](super::zorder_chain_order_tests) は「鎖が成立して
//! 崩れない」側を、[`zorder_chain_order_lifecycle_tests`](super::zorder_chain_order_lifecycle_tests)
//! は「鎖への出入り」を固定する。本ファイルが受け持つのはその外側——**鎖の権限が及ばない
//! 範囲で、本 spec が何も変えないこと**である。足場（窓の作り・順序の走行器・助走・
//! 後始末）は兄弟から借りる。
//!
//! # 何を固定するか（2 本）
//!
//! 1. **部外者どうしの相対順**——鎖に属さない検体窓 2 枚で鎖の前後を挟み、後押しの前後で
//!    **部外者どうしの前後関係**が変わらないこと（`research.md` §12.2／§12.5 の実測の
//!    恒久化）。逐語の実測値は 助走 `[3,2,1,0,4]` → 後押し 1 回で鎖が `0,1,2` へ収まり、
//!    **`3` が `4` より手前のまま**、である。
//! 2. **既定状態＝非強制**——グループ指定が一つも無い間は 1 命令も出ず、どの 2 つの窓の
//!    前後も固定の規則で決まらない（要件 6.1）。窓を活性化しても他の窓どうしの相対順が
//!    変わらない（要件 6.2）。
//!
//! # 要件 6.1／6.2 の充足はどちらの本かというと 2 本目である
//!
//! design.md「実窓テスト」6 が明記するとおり、**ゴーストの窓は指定が 1 つでもあれば
//! 全て鎖に入る**（要件 15・DD-11）。よってグループが有効な間に「鎖の外に居るゴースト窓」は
//! 存在せず、要件 6.1／6.2 が縛る「どのグループにも属さない窓どうしの相対順」は
//! **既定状態＝指令ゼロの檻**（2 本目）で満たされる。
//!
//! 1 本目の検体窓が代表するのは**他アプリケーションの窓**であり、その相対順が後押しで
//! 動かないことは要件の主張ではなく**実測の恒久化**である。だから 1 本目は
//! 「要件が破れた」ではなく「実測が変わった」と名乗る。
//!
//! # 主張しないこと
//!
//! **鎖と、鎖の外の窓との前後関係は主張しない。** 鎖は 1 つの塊として動き、後押しの際に
//! 鎖の外の窓を追い越すことがある（周囲の窓の状況に依り、並走走行で両方の結果が出る
//! ——`research.md` §12.5／§12.9 の 3 件目）。要件も正典もこの 2 群の間を規定していない
//! （要件 3.6／6.1・DD-3b）。ここで「追い越さない」と書けば、それは
//! **こちらが保証していないものを檻に書く**形——本 spec が 3 度潰した非決定の原因——に
//! 戻ることになる。
//!
//! # 刺激には必ず「届いたこと」の自己検査を両側から挟む（兄弟 4.1 の教訓）
//!
//! 本ファイルの 2 本はどちらも**否定形の主張**（「変わらない」「出さない」）であり、
//! 何もしない檻がそのまま通ってしまう形である。よって刺激が世界に残した変化を先に採る:
//!
//! | テスト | 刺激 | 刺激が届いたことの証拠 |
//! |---|---|---|
//! | 部外者の不変 | 鎖を公開して 1 巡（＝後押しが出る） | 鎖の中の並びが逆順 `[2,1,0]` から宣言順 `[0,1,2]` へ実際に変わる。繋ぎ 2 本が OS 上にも帳簿にも立つ |
//! | 既定状態 | ⑴最奥の窓を最前面へ ⑵**対照として**同じ World へ鎖を公開して 1 巡 | ⑴で並びが実際に動く。⑵で同じ窓・同じ schedule から繋ぎ 3 本と記録が**確かに出る**——「何も出なかった」が「そこまで届いていなかった」と区別できる |
//!
//! **対照が無ければ「1 命令も出ない」は空虚である。** 窓が作れていなくても、schedule が
//! 適用系を載せていなくても、同じ緑になるからである。

use bevy_ecs::prelude::*;
use bevy_ecs::schedule::{Schedule, SingleThreadedExecutor};
use windows::Win32::Foundation::{HINSTANCE, HWND};
use windows::Win32::UI::WindowsAndMessaging::{
    HWND_TOP, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SetWindowPos,
};
use windows::core::{PCWSTR, w};

use super::apply_zorder_chain;
use super::zorder_chain_order_tests::{
    arrange_z, create_chain_window, is_visible, ledger_shape, owner_shape, teardown, z_shape,
};
use crate::ecs::test_support::capture_under_filter;
use crate::ecs::window::{ChainPlan, ChainSegment, CrossEdge, WindowHandle, ZOrderChainPlan};

/// 実機サインオフと同じ絞り（鎖系の記録がすべて点く）。
const SIGNOFF_DIRECTIVES: &str = "info,wintf::ecs::window::zorder_chain=debug";

/// 重なり順系の記録の冠。作業ツリー名にハイフン付きの語が混ざるので**冠込みで**照合する。
const ZORDER_TAG: &str = "[zorder-";

// ---------------------------------------------------------------------------
// 道具立て（足場は兄弟から借り、ここに置くのは公開と schedule だけ）
// ---------------------------------------------------------------------------

/// 適用系だけを載せた 1 巡分の schedule（**単一スレッドの実行器を明示**）。
///
/// 実窓の操作は UI スレッド固定（[`NonSendMarker`](bevy_ecs::system::NonSendMarker)）の
/// 前提であり、既定の多スレッド実行器では記録の捕捉（スレッドローカルの差し替え）が
/// 1 行も拾えない。兄弟の檻と同じ規律でここでも明示する。
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

/// 手前から奥へ並べた entity 列を、連続対すべてを横断 edge にした鎖として公開する。
///
/// 本ファイルの窓は「キャラ窓 1 枚だけのスコープ」とみなす（同一スコープのペア対は
/// 1 組も無い）ので、隣り合わせはすべて本 spec が張る繋ぎである。
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

/// 実窓を `count` 枚作り、それぞれに entity を与える（**受け口は作らない**）。
///
/// 受け口（[`ZOrderChainPlan`]）を作らないのが既定状態の姿である——本番の指令消化の相は
/// グループが 1 つも無い間、受け口そのものを挿入しない（`zorder_drain.rs`
/// 「既定状態では受け口そのものを作らない」）。
fn bare_fixture(count: usize, title: PCWSTR) -> (Vec<HWND>, World, Vec<Entity>) {
    let set: Vec<HWND> = (0..count).map(|_| create_chain_window(title)).collect();
    let mut world = World::new();
    let members: Vec<Entity> = set.iter().map(|h| spawn_window(&mut world, *h)).collect();
    (set, world, members)
}

/// Z のみを動かす素の指令で最前面へ持ち上げる（**攪乱専用・本番の経路ではない**）。
///
/// 絶対帯指定（`HWND_TOP`）を使うのは、これが**測定対象そのもの**——利用者が窓を活性化して
/// 最前面へ持ち上げた状況の再現（要件 6.2 の「活性化されたとき」）——だからである。
/// 助走には使わない（兄弟の module doc）。
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

/// 捕捉した出力に含まれる重なり順系の記録行を返す（冠込みで照合する）。
fn zorder_lines(out: &str) -> Vec<&str> {
    out.lines().filter(|l| l.contains(ZORDER_TAG)).collect()
}

// ===========================================================================
// ⑴ 部外者どうしの相対順は後押しで動かない（`research.md` §12.2／§12.5 の恒久化）
// ===========================================================================

/// 鎖の窓数（後押しが「根を錨の直後へ差し直す」形を取れる最小＋1）。
///
/// ⚠ 初版はここを「先頭を 2 番目の直後へ差し直す」と説明していたが、その形は
/// `research.md` §13.2 で**撤回**された（枚数の下限は同じ 2 枚なので値は変わらない）。
const CHAIN: usize = 3;
/// 前の部外者の添字（5 枚の組の中での位置）。
const FRONT_OUTSIDER: usize = CHAIN;
/// 後ろの部外者の添字。
const BACK_OUTSIDER: usize = CHAIN + 1;

/// 鎖の前後を部外者 2 枚で挟み、後押しを出しても**部外者どうし**の前後が変わらない。
///
/// 助走は `research.md` §12.2 の逐語値と同じ `[3,2,1,0,4]`——前の部外者が最前面、
/// 鎖が宣言の逆順、後ろの部外者が最背面である。後押し 1 回のあと、鎖は宣言順 `0,1,2` へ
/// 収まり、前の部外者は後ろの部外者より手前のままである。
///
/// # 刺激が届いたことを両側から挟む
///
/// 健全な状態でも**部外者どうしの並びは前後で同一**なので、事後の比較だけでは
/// 「鎖を公開しなくても通る」——後押しが 1 度も出なくても緑になる。よって先に
///
/// 1. 助走で `[3,2,1,0,4]` が実際に組めていること
/// 2. 1 巡のあと鎖の中の並びが `[2,1,0]` から `[0,1,2]` へ**実際に変わった**こと
/// 3. 繋ぎ 2 本が OS 上（`GetWindow(GW_OWNER)`）にも帳簿にも立ったこと
///
/// を採る。⑵⑶ が無ければ「後押しが出た」ことの証拠が 1 つも無い。
///
/// # 主張しないこと
///
/// 鎖と部外者の間の前後関係は測らない（module doc）。鎖は塊として動き、後押しの際に
/// 部外者を追い越すことがある。
#[test]
fn outsiders_keep_their_relative_order_across_the_nudge() {
    let (set, mut world, members) = bare_fixture(CHAIN, w!("zorder-chain-outsider/nudge"));
    let front = create_chain_window(w!("zorder-chain-outsider/front"));
    let back = create_chain_window(w!("zorder-chain-outsider/back"));
    let all: Vec<HWND> = set.iter().copied().chain([front, back]).collect();
    let all_visible = all.iter().all(|h| is_visible(*h));

    // 助走——`research.md` §12.2 の逐語値と同じ `[3,2,1,0,4]`。
    // 自分の窓どうしの相対指定だけで組む（絶対帯指定を使わない）。
    let seed = [FRONT_OUTSIDER, 2, 1, 0, BACK_OUTSIDER];
    arrange_z(&all, &seed);
    let start = z_shape(&all);
    let chain_before = z_shape(&set);
    let outsiders_before = outsider_gap(&start);

    // 刺激——鎖を公開して 1 巡（所有関係の書込と後押し 1 回がこの中で起きる）。
    publish_full_chain(&mut world, &members);
    chain_schedule().run(&mut world);

    let after = z_shape(&all);
    let chain_after = z_shape(&set);
    let outsiders_after = outsider_gap(&after);
    let owners = owner_shape(&set);
    let ledger = ledger_shape(&mut world, &members);

    teardown(&all);

    assert!(
        all_visible,
        "5 枚が表示状態になっていない（不可視の窓では後押しの実測経路が働かない）"
    );
    assert_eq!(
        start,
        seed.to_vec(),
        "助走が §12.2 の逐語値 [3,2,1,0,4] に揃っていない: {start:?}"
    );

    // 刺激が届いたことの自己検査——ここが動かなければ以下の比較は空虚である。
    assert_eq!(
        chain_before,
        vec![2, 1, 0],
        "助走の時点で鎖が宣言の逆順に居ない: {chain_before:?}"
    );
    assert_eq!(
        chain_after,
        vec![0, 1, 2],
        "1 巡のあと鎖が宣言順へ収まっていない＝後押しが届いておらず、部外者の比較は空虚である: {chain_after:?}"
    );
    assert_eq!(
        owners,
        vec![Some(1), Some(2), None],
        "本番の適用系が張った所有関係が一直線の鎖になっていない（末尾が根）"
    );
    assert_eq!(
        ledger,
        vec![(0, 1), (1, 2)],
        "本 spec が張った繋ぎの帳簿が鎖の連続対と一致しない"
    );

    // 実測の恒久化——**部外者どうし**の前後だけを見る（鎖との前後は見ない）。
    assert_eq!(
        outsiders_before,
        Some(true),
        "助走の時点で前の部外者が後ろの部外者より手前に居ない: {start:?}"
    );
    assert_eq!(
        outsiders_after,
        Some(true),
        "後押しのあと部外者どうしの前後が入れ替わった（`research.md` §12.2／§12.5 の実測が変わった）: {after:?}"
    );
}

/// 5 枚の並びの中で「前の部外者が後ろの部外者より手前か」を返す（どちらか欠ければ `None`）。
fn outsider_gap(shape: &[usize]) -> Option<bool> {
    let front = shape.iter().position(|v| *v == FRONT_OUTSIDER)?;
    let back = shape.iter().position(|v| *v == BACK_OUTSIDER)?;
    Some(front < back)
}

// ===========================================================================
// ⑵ 既定状態——1 命令も出ず、どの 2 つの窓の前後も固定の規則で決まらない
//    （要件 6.1／6.2）
// ===========================================================================

/// 既定状態の窓数（活性化した 1 枚を除いても「他の窓どうし」が 3 枚残る）。
const DEFAULT_SIZE: usize = 4;

/// グループ指定が一つも無い間、本 spec は 1 命令も出さず、重なりは利用者の操作だけで決まる。
///
/// 本番の指令消化の相はグループが 1 つも無い間、受け口（[`ZOrderChainPlan`]）そのものを
/// 挿入しない。よって既定状態の再現は「受け口を作らない World で適用系を回す」ことである。
///
/// # 何を測るか
///
/// 1. **1 命令も出ないこと**——所有関係が 1 本も立たず（OS 側・帳簿の両方）、重なりが
///    1 枚も動かず、重なり順系の記録が 1 行も出ない（要件 6.1／6.4）
/// 2. **固定の規則で決まらないこと**——最奥の窓を最前面へ持ち上げると、その窓は最前面に
///    留まる。engine が押し戻す規則を持っていればここで戻る（要件 6.1）
/// 3. **他の窓どうしの相対順が変わらないこと**——活性化の前後で、活性化していない
///    3 枚の相対順が同一である（要件 6.2）
///
/// # 対照が無ければ「出ない」は空虚である
///
/// 「1 行も出ない」「1 本も立たない」は、窓が作れていなくても、schedule が適用系を
/// 載せていなくても、同じ緑になる。よって**同じ World・同じ窓・同じ schedule**へ鎖を
/// 公開して最後にもう 1 巡回し、
///
/// - 繋ぎ 3 本が OS 上にも帳簿にも立つこと
/// - 重なり順系の記録が**確かに出る**こと（同じ捕捉窓で）
/// - 重なりが宣言順へ動くこと
///
/// を採る。これが揃って初めて、先の「出なかった」が**機構が働いたうえでの不作為**だと
/// 言える。対照の巡は要件 6 の主張ではない——道具が生きていることの証拠である。
#[test]
fn the_default_state_issues_nothing_and_pins_no_pair_of_windows() {
    let (set, mut world, members) = bare_fixture(DEFAULT_SIZE, w!("zorder-chain-outsider/default"));
    let all_visible = set.iter().all(|h| is_visible(*h));
    let mut schedule = chain_schedule();

    // 助走——自分の窓どうしの相対指定だけで `[0,1,2,3]` へ揃える。
    let seed: Vec<usize> = (0..DEFAULT_SIZE).collect();
    arrange_z(&set, &seed);
    let start = z_shape(&set);

    // ⑴ 既定状態の 1 巡——受け口が無いので適用系は仕事を得られない。
    let quiet = capture_under_filter(SIGNOFF_DIRECTIVES, || {
        schedule.run(&mut world);
    });
    let after_idle = z_shape(&set);
    let owners_idle = owner_shape(&set);
    let ledger_idle = ledger_shape(&mut world, &members);

    // ⑵ 刺激——最奥の窓を活性化して最前面へ（要件 6.2 の「活性化されたとき」）。
    let raised_ok = raise_to_front(set[DEFAULT_SIZE - 1]);
    let after_raise = z_shape(&set);

    // ⑶ 活性化のあとも既定状態のまま 1 巡——押し戻す規則が無いこと。
    let quiet_after_raise = capture_under_filter(SIGNOFF_DIRECTIVES, || {
        schedule.run(&mut world);
    });
    let after_second_idle = z_shape(&set);
    let owners_after_raise = owner_shape(&set);

    // ⑷ 対照——同じ World・同じ窓・同じ schedule で、鎖を公開すれば確かに動く。
    publish_full_chain(&mut world, &members);
    let control = capture_under_filter(SIGNOFF_DIRECTIVES, || {
        schedule.run(&mut world);
    });
    let after_control = z_shape(&set);
    let owners_control = owner_shape(&set);
    let ledger_control = ledger_shape(&mut world, &members);

    teardown(&set);

    assert!(
        all_visible,
        "4 枚が表示状態になっていない（不可視の窓では後押しの実測経路が働かない）"
    );
    assert_eq!(start, seed, "助走が揃っていない: {start:?}");

    // --- 対照を先に確かめる（道具が生きていなければ以下の「出ない」は空虚） -------
    assert_eq!(
        owners_control,
        vec![Some(1), Some(2), Some(3), None],
        "対照の巡で所有の鎖が立たなかった＝この檻は機構へ届いていない（既定状態の主張は空虚）"
    );
    assert_eq!(
        ledger_control,
        vec![(0, 1), (1, 2), (2, 3)],
        "対照の巡で帳簿に繋ぎが載らなかった＝この檻は機構へ届いていない"
    );
    assert_eq!(
        after_control, seed,
        "対照の巡で重なりが宣言順へ動かなかった＝この檻は実窓へ届いていない: {after_control:?}"
    );
    assert!(
        !zorder_lines(&control).is_empty(),
        "対照の巡で重なり順系の記録が 1 行も出なかった＝捕捉窓そのものが働いていない"
    );

    // --- 要件 6.1／6.4——既定状態では 1 命令も出ない ------------------------------
    assert_eq!(
        owners_idle,
        vec![None; DEFAULT_SIZE],
        "既定状態で所有関係が立った（要件 6.1／6.4）"
    );
    assert!(
        ledger_idle.is_empty(),
        "既定状態で帳簿に繋ぎが載った（要件 6.1／6.4）: {ledger_idle:?}"
    );
    assert_eq!(
        after_idle, seed,
        "既定状態の 1 巡で重なりが動いた（要件 6.4）: {after_idle:?}"
    );
    assert!(
        zorder_lines(&quiet).is_empty(),
        "既定状態で重なり順系の記録が出た（要件 6.1／6.4）: {:?}",
        zorder_lines(&quiet)
    );

    // --- 要件 6.1——活性化した窓を押し戻す固定の規則を持たない --------------------
    assert!(
        raised_ok,
        "最奥の窓を最前面へ持ち上げる指令そのものが失敗した（刺激が起きていない）"
    );
    let expected_after_raise: Vec<usize> = std::iter::once(DEFAULT_SIZE - 1)
        .chain(0..DEFAULT_SIZE - 1)
        .collect();
    assert_eq!(
        after_raise, expected_after_raise,
        "活性化そのものが重なりを動かしていない＝以下の比較は空虚である: {after_raise:?}"
    );
    assert_eq!(
        after_second_idle, expected_after_raise,
        "既定状態なのに活性化した窓が押し戻された（固定の規則で前後を決めている・要件 6.1）: {after_second_idle:?}"
    );
    assert_eq!(
        owners_after_raise,
        vec![None; DEFAULT_SIZE],
        "既定状態の活性化に応じて所有関係が立った（要件 6.1／6.2）"
    );
    assert!(
        zorder_lines(&quiet_after_raise).is_empty(),
        "既定状態の活性化に応じて重なり順系の記録が出た（要件 6.2）: {:?}",
        zorder_lines(&quiet_after_raise)
    );

    // --- 要件 6.2——活性化していない窓どうしの相対順が変わらない ------------------
    let others_before: Vec<usize> = after_idle
        .iter()
        .copied()
        .filter(|v| *v != DEFAULT_SIZE - 1)
        .collect();
    let others_after: Vec<usize> = after_second_idle
        .iter()
        .copied()
        .filter(|v| *v != DEFAULT_SIZE - 1)
        .collect();
    assert_eq!(
        others_before, others_after,
        "1 枚の活性化で他の窓どうしの相対順が変わった（要件 6.2）: {others_before:?} → {others_after:?}"
    );
}

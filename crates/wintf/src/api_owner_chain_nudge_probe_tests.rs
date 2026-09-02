//! 後押しの形の再実測（`research.md` §13）——**どの窓を動かせば所有の鎖が並び直るのか**を
//! 全順列の掃き出しで確かめる調査檻。
//!
//! 兄弟の [`api_owner_chain_probe_tests`](super::api_owner_chain_probe_tests) は §12 の
//! 実測を担い、そこで採った後押し（⚠ **撤回済み**——鎖の先頭を 2 番目の直後へ差し直す形。
//! §13.2 を参照）を設計へ落とした。
//! ところが 4.3 の実窓検証で、その形が**完全に空振りする配置**が本番の起動直後に現れる
//! ことが判った。本ファイルはその原因を実測で突き止め、代わりに何を採るべきかを決める。
//!
//! # 判ったこと（§13）
//!
//! Windows が鎖全体を並べ直すのは、**所有する窓が動いたとき**である——所有される窓は
//! 所有者より手前という不変条件を保つため、所有側を動かすと被所有側が引き連れられ、それが
//! 鎖の奥から手前へ伝わる。鎖の中で他の窓を所有しているのは根（列の末尾）以下の各窓で
//! あり、**先頭だけは誰も所有していない**。よって先頭を動かしても鎖は並ばない。
//!
//! 初版の檻が緑だったのは、**スレッド既定の不可視 IME 窓**（`class="IME"`）が
//! 「そのスレッドで最初に作られた窓」に所有されており、それがたまたま鎖の先頭だったから
//! である＝先頭も「所有する窓」になっていた。本番にその保証は無い。
//!
//! # 受け皿（IME 窓を鎖から外す）
//!
//! よってこの檻は、鎖の窓より先に**受け皿の窓を 1 枚**作り、IME 窓をそちらへ付ける
//! （[`ensure_ime_anchor`]）。以降、鎖の窓どうしは本番と同じ裸の隣接で並ぶ。
//!
//! # 測り方
//!
//! 窓の作り・測り方・決定論の規律（絶対帯指定を助走に使わない／挿入位置に他プロセスの窓を
//! 渡さない）はすべて兄弟ファイルと同じであり、助手もそちらから借りる。重なりは
//! **順序**で読む（隣接では読まない）。唯一の例外は「その要求が現在位置と同じか」を
//! 確かめる箇所で、そこは `SetWindowPos` が見るのと同じ**生の**隣接で読む。
//!
//! # 主張しないこと
//!
//! 兄弟ファイルと同じく、**鎖と鎖の外の窓との前後関係は主張しない**（鎖は塊として動き、
//! 鎖の外の窓を追い越すことがある）。固定するのは ⑴ 鎖の中の相対順 ⑵ 鎖の外どうしの相対順。

use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::{GW_HWNDNEXT, GetWindow};
use windows::core::w;

use super::api_owner_chain_probe_tests::{
    arrange_z, create_probe_window, destroy_all, ensure_ime_anchor, link_chain, only, place_after,
    unlink_all, z_shape,
};

/// 鎖の窓数（本番の最小構成＝2 スコープ×2 枚と同じ 4 枚）。
const CHAIN_SIZE: usize = 4;

/// 宣言どおりの最終形（手前から順の添字）。
const DECLARED: [usize; CHAIN_SIZE] = [0, 1, 2, 3];

// ---------------------------------------------------------------------------
// 受け皿——IME 窓を鎖の外へ追い出す
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// 生の隣接（**「その要求は現在位置と同じか」を見るためだけに読む**）
// ---------------------------------------------------------------------------

/// その窓の生の 1 つ奥の窓（不可視の窓も含む・`GW_HWNDNEXT`）。
fn raw_next(hwnd: HWND) -> Option<HWND> {
    // SAFETY: Win32 境界。窓ハンドルに対する読み取り専用の走査。
    unsafe { GetWindow(hwnd, GW_HWNDNEXT) }.ok()
}

/// 先頭が 2 番目の**生の直後**に居るか（初版の後押しが空振りする引き金）。
fn head_is_raw_next_of_second(set: &[HWND]) -> bool {
    raw_next(set[1]) == Some(set[0])
}

/// 根が錨（1 つ手前の窓）の**生の直後**に居るか（採る形の素直な枝が空振りする引き金）。
fn root_is_raw_next_of_anchor(set: &[HWND]) -> bool {
    raw_next(set[CHAIN_SIZE - 2]) == Some(set[CHAIN_SIZE - 1])
}

// ===========================================================================
// 実測 10: 始点の全順列に対する後押しの形の掃き出し（§13.1）
// ===========================================================================

/// 後押しの形（測定用）。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Shape {
    /// ⒜ 初版——先頭を 2 番目の直後へ（先頭は何も所有していない）。
    HeadAfterSecond,
    /// ⒝ 2 番目を先頭の直後へ（2 番目は先頭を所有している）。
    SecondAfterHead,
    /// ⒞ 根を錨（1 つ手前の窓）の直後へ——**採る形の素直な枝**。
    RootAfterAnchor,
    /// ⒟ 根を先頭の直後へ——**採る形の切り替えた枝**。
    RootAfterHead,
    /// ⒠ 採る形そのもの（⒞ が現在位置と同じになる巡だけ ⒟ を採る）。
    Chosen,
    /// ⒡ 対照——後押しを出さない。
    NoNudge,
}

/// 1 回の掃き出しの結果。
struct SweepOnce {
    /// 着いた並び（手前から順の添字）。
    landed: Vec<usize>,
    /// 後押しの直前に「先頭が 2 番目の生の直後」だったか。
    head_adjacent: bool,
    /// 後押しの直前に「根が錨の生の直後」だったか。
    root_adjacent: bool,
}

/// 始点 `order` から鎖を張り、`shape` の後押しを 1 回出して、着いた並びを返す。
///
/// **窓は使い回す**——1 回ごとに 4 枚作って壊すと、24 通り × 6 形で 576 枚の生成・破棄に
/// なり、その churn が同じプロセスの他の実窓の檻を不安定にした（3 プロセス同時 × 25 周で
/// 75 走行中 3 本が赤・使い回しに改めて 0 本）。始点は [`arrange_z`] が明示的に組み直し、
/// 所有関係も毎回張り直すので、使い回しても各回は独立である。
fn sweep_once(set: &[HWND], order: &[usize], shape: Shape) -> SweepOnce {
    // 前回の鎖を落としてから始める（残っていると助走の Z 指令が鎖に引きずられる）。
    unlink_all(set);
    arrange_z(set, order);
    // 鎖は **0 が手前・3 が根**（`set[i]` は `set[i+1]` に所有される）。
    link_chain(set);

    let head_adjacent = head_is_raw_next_of_second(set);
    let root_adjacent = root_is_raw_next_of_anchor(set);
    let root = CHAIN_SIZE - 1;
    let anchor = CHAIN_SIZE - 2;
    match shape {
        Shape::HeadAfterSecond => place_after(set[0], set[1]),
        Shape::SecondAfterHead => place_after(set[1], set[0]),
        Shape::RootAfterAnchor => place_after(set[root], set[anchor]),
        Shape::RootAfterHead => place_after(set[root], set[0]),
        Shape::Chosen => {
            if root_adjacent {
                place_after(set[root], set[0]);
            } else {
                place_after(set[root], set[anchor]);
            }
        }
        Shape::NoNudge => {}
    }

    let landed = z_shape(set);
    unlink_all(set);
    SweepOnce {
        landed,
        head_adjacent,
        root_adjacent,
    }
}

/// 4 枚の並びの全順列（24 通り）。
fn all_orders() -> Vec<Vec<usize>> {
    let mut out = Vec::new();
    for a in 0..CHAIN_SIZE {
        for b in 0..CHAIN_SIZE {
            for c in 0..CHAIN_SIZE {
                for d in 0..CHAIN_SIZE {
                    let order = vec![a, b, c, d];
                    let mut seen = order.clone();
                    seen.sort_unstable();
                    seen.dedup();
                    if seen.len() == CHAIN_SIZE {
                        out.push(order);
                    }
                }
            }
        }
    }
    out
}

/// 1 つの形を全順列へ当て、（宣言順へ着いた回数, 先頭隣接の回数, 根隣接の回数）を返す。
fn sweep_shape(set: &[HWND], shape: Shape) -> (usize, usize, usize) {
    let mut landed_ok = 0;
    let mut head_adjacent = 0;
    let mut root_adjacent = 0;
    for order in all_orders() {
        let once = sweep_once(set, &order, shape);
        if once.landed == DECLARED.to_vec() {
            landed_ok += 1;
        }
        if once.head_adjacent {
            head_adjacent += 1;
        }
        if once.root_adjacent {
            root_adjacent += 1;
        }
    }
    (landed_ok, head_adjacent, root_adjacent)
}

/// **動かすのが所有側でなければ鎖は並ばない**——4 枚の始点 24 通りすべてで測る。
///
/// これが本 spec の後押しの形を決めた実測である。逐語値は `research.md` §13.1 の表。
#[test]
fn owner_chain_probe_only_moving_an_owning_window_rearranges_the_chain() {
    ensure_ime_anchor();
    let set: Vec<HWND> = (0..CHAIN_SIZE)
        .map(|_| create_probe_window(w!("owner-chain-nudge/sweep")))
        .collect();

    let head_after_second = sweep_shape(&set, Shape::HeadAfterSecond);
    let second_after_head = sweep_shape(&set, Shape::SecondAfterHead);
    let root_after_anchor = sweep_shape(&set, Shape::RootAfterAnchor);
    let root_after_head = sweep_shape(&set, Shape::RootAfterHead);
    let chosen = sweep_shape(&set, Shape::Chosen);
    let no_nudge = sweep_shape(&set, Shape::NoNudge);

    unlink_all(&set);
    destroy_all(&set);

    eprintln!("[probe-10] (landed_ok, head_adjacent, root_adjacent) / 24 通り");
    eprintln!("[probe-10] head->after second (初版) = {head_after_second:?}");
    eprintln!("[probe-10] second->after head       = {second_after_head:?}");
    eprintln!("[probe-10] root->after anchor       = {root_after_anchor:?}");
    eprintln!("[probe-10] root->after head         = {root_after_head:?}");
    eprintln!("[probe-10] chosen (2 択)            = {chosen:?}");
    eprintln!("[probe-10] no nudge (対照)          = {no_nudge:?}");

    // 対照——後押しが無ければ、既に宣言順だった 1 通りしか宣言順にならない
    // （所有関係を張るだけでは重なりは 1 ミリも動かない＝§12.1 実測 1 の再確認）。
    assert_eq!(
        no_nudge.0, 1,
        "【対照】後押し無しで並びが動いている（以下の比較が空虚になる）"
    );

    // 初版の形は**1 通りも収まらない**（先頭は何も所有していない）。
    assert_eq!(
        head_after_second.0, 0,
        "初版の後押し（先頭を 2 番目の直後へ）で鎖が収まっている＝受け皿が効かず易しい配置を測っている"
    );
    // 引き金が実際に立つ巡が在る（＝「収まらない」が測定漏れでないことの自己検査）。
    assert!(
        head_after_second.1 > 0,
        "先頭が 2 番目の生の直後に居る巡が 1 つも無い（掃き出しが引き金配置を作れていない）"
    );

    // 所有側を動かす 3 形はすべて全通りで収まる。
    assert_eq!(
        second_after_head.0, 24,
        "2 番目（先頭の所有者）を動かす形で収まらない巡がある"
    );
    assert_eq!(
        root_after_anchor.0, 24,
        "根を錨の直後へ差し直す形で収まらない巡がある"
    );
    assert_eq!(
        root_after_head.0, 24,
        "根を先頭の直後へ差し直す形で収まらない巡がある"
    );
    assert_eq!(chosen.0, 24, "本 spec が採る 2 択の形で収まらない巡がある");

    // 採る形の切り替え枝が実際に踏まれる巡が在る（枝が飾りでないことの自己検査）。
    assert!(
        chosen.2 > 0,
        "根が錨の生の直後に居る巡が 1 つも無い（切り替えの枝を 1 度も踏んでいない）"
    );
}

// ===========================================================================
// 実測 11: 採る形が鎖の外どうしの相対順を動かさないか（要件 6.1／6.2）
// ===========================================================================

/// 鎖を部外者で前後から挟み、採る形の後押しで**部外者どうし**の前後が変わらないこと。
#[test]
fn owner_chain_probe_the_chosen_nudge_keeps_outsiders_in_relative_order() {
    ensure_ime_anchor();
    let chain: Vec<HWND> = (0..3)
        .map(|_| create_probe_window(w!("owner-chain-nudge/out-chain")))
        .collect();
    let front = create_probe_window(w!("owner-chain-nudge/out-front"));
    let back = create_probe_window(w!("owner-chain-nudge/out-back"));
    let mut all = chain.clone();
    all.push(front);
    all.push(back);

    // 手前から: 部外者(3), 鎖の逆順 2,1,0, 部外者(4)。
    arrange_z(&all, &[3, 2, 1, 0, 4]);
    let start = z_shape(&all);

    link_chain(&chain);

    // 採る形——根（`chain[2]`）を錨（`chain[1]`）の直後へ。
    place_after(chain[2], chain[1]);
    let after = z_shape(&all);

    eprintln!("[probe-11] start = {start:?}");
    eprintln!("[probe-11] after = {after:?}");

    unlink_all(&chain);
    destroy_all(&all);

    assert_eq!(
        start,
        vec![3, 2, 1, 0, 4],
        "助走: 部外者で前後を挟み鎖は逆順"
    );
    assert_eq!(
        only(&after, &[0, 1, 2]),
        vec![0, 1, 2],
        "【実測】採る形の後押しで鎖は宣言順へ収まる"
    );
    assert_eq!(
        only(&after, &[3, 4]),
        vec![3, 4],
        "【実測・要件 6.1／6.2】鎖の外どうしの相対順は変わらない"
    );
}

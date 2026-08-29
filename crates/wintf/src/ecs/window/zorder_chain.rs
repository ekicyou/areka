//! 所有の鎖——**計画の受け口・帳簿・差分の純判断・後押しの選定・記録の唯一の出口**
//! （要件 4.1／7.1／7.2／8.2／10.1／10.4／11.1／14.4）。
//!
//! design.md「`zorder_chain`（新設・受け口と純関数）」が正本である。ここに在るのは
//! **判断だけ**であり、Win32 の呼び出しも World の走査も 1 つも無い（それは後続の適用系
//! `zorder_chain_apply` の担当である）。分けてあるのは、鎖の組み替えの導出を実機も
//! 実ディスプレイも無しに全分岐固定できる形にするためである（要件 10.1）。
//!
//! # 望む状態と現況の 2 つしか持たない
//!
//! 状態は [`ZOrderChainPlan`]（望む鎖・areka が公開する）と [`CrossOwnerLink`] の集合
//! （現況＝本 spec が実際に OS へ書いた繋ぎ）の 2 つだけである。**OS の z 順そのものは
//! 状態として持たない**——持てば周期的な観測が要り、要件 14.2 が禁じた「繰り返しの観測と
//! 是正」へ逆戻りする。維持は「所有される窓は所有者より手前」という OS の不変条件へ
//! 委ねる（要件 14.1）。
//!
//! # areka の型は 1 つも載せない
//!
//! `wintf → areka` の import は禁止（既存規律）なので、鎖の型は**こちらで定義し areka が
//! 組む**。よって欄に載せられるのは Entity と HWND と正準表記の文字列だけであり、スコープや
//! 窓種別の知識は areka 側に閉じる。既存の `KeepDirectlyAbove` と同じ分界である。
//!
//! # 記録の出口をここへ集約する理由（本 task の裁定）
//!
//! `tracing` の既定の出力先は**呼び出し元の module path** である。行を組む純関数
//! （[`zorder_chain_diag`](super::zorder_chain_diag)）の側からマクロを撃つと、出力先が
//! 行組立の層の名前になり、記録を出す場所が増えるたびに grep の対象が割れていく。
//! よって鎖系 7 語の**マクロ呼出はこの 1 ファイルに集約**し、出力先を
//! `wintf::ecs::window::zorder_chain` の 1 本に固定する（design.md の同節・初版の申し送り 2.1）。
//!
//! 例外は保全語彙 2 語（`[zorder-group] applied`／`rejected`）である。あちらは design.md
//! 「保全する既存語彙（要件 9.5）と、その新しい住処」が移設先ファイルを
//! `zorder_chain_diag.rs` と**名指しで**規定しており、記録そのものもそちらに置いてある。
//! 出力先は `…::zorder_chain_diag` になるが、実機サインオフの `RUST_LOG` 指定
//! `wintf::ecs::window::zorder_chain=debug` は**前方一致**でこれを点灯させ、
//! `signoff-scan.ps1` は出力先の文字列を 1 つも読まない（判定はタグの字面で行う）。
//! よって語彙の保全（要件 9.5）は割れない。2 つの住処が実際に分かれていることは
//! 兄弟テストが捕捉した行の出力先で固定している。
//!
//! # `dead_code` 許可は 1 つも残っていない
//!
//! 適用系（[`zorder_chain_apply`](super::zorder_chain_apply)）が着地したことで、差分の
//! 純判断・後押しの選定・記録の出口には本番の呼び手が付いた。段階的実装のために
//! モジュール全体を覆っていた許可は撤去してある。

use bevy_ecs::prelude::*;
use tracing::{debug, error};
use windows::Win32::Foundation::HWND;

use super::zorder_chain_diag::{
    ChainSegment, ChainSkipReason, DetachReason, absent_line, link_failed_line, linked_line,
    settled_line, skipped_line, unlink_failed_line, unlinked_line,
};
use super::{SetWindowPosCommand, WindowPos, ZOrder};

// ============================================================================
// 鎖の計画——areka が組み、wintf が適用する
// ============================================================================

/// 手前側の窓が奥側の窓に所有される 1 本の関係。
///
/// 向きを取り違えると鎖が逆さまになるため、欄の名前で向きを固定する——
/// `owned` が**手前**、`owner` が**奥**である（OS の不変条件「所有される窓は所有者より
/// 手前」がそのまま並びになる）。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct CrossEdge {
    /// 所有される窓（手前側）。
    pub owned: Entity,
    /// 所有する窓（奥側）。
    pub owner: Entity,
    /// この繋ぎが属する鎖の区間——台帳のグループ（`gN`・登記順）か、どのグループにも
    /// 属さないスコープの後方配置（`tail`・要件 15）か。
    ///
    /// **記録のためだけの欄である**（要件 9.1「どのグループの…」）。所有関係を書く
    /// 手順はこの値を 1 度も読まない。それでも計画に載せるのは、区間を知っているのは
    /// 台帳を持つ側（areka）だけであり、鎖を適用する側からは**構造上復元できない**
    /// ためである——列は「グループの連結＋後方配置」に畳まれており、境界が消える。
    pub segment: ChainSegment,
}

/// 全窓の鎖 1 本ぶんの計画（areka が構築し、wintf が適用する）。
///
/// `members` はグループの連結（登記順・先に登記されたほど手前）＋未指定スコープの
/// 後方配置（スコープ ID 昇順・要件 15）を、**実在する窓だけへ射影した**列である。
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct ChainPlan {
    /// 手前から奥へ並べた、実在する窓の Entity 列（射影後）。末尾が鎖の根。
    pub members: Vec<Entity>,
    /// 本 spec が張る横断 edge（`members` の連続対のうち、同一スコープのペア対でないもの）。
    pub cross_edges: Vec<CrossEdge>,
    /// 窓が存在しなかった宣言要素——**宣言したグループの ID と正準表記の対**
    /// （`(0, "b0")`／`(1, "s1")`。要件 1.4／8.4 の記録材料）。
    ///
    /// ID を伴うのは、記録行（`[zorder-chain] absent group_id= element=`）が
    /// 「どのグループの宣言が空振りしたか」を単独で読めなければならないからである。
    /// 後方配置のスコープは誰も宣言していないので、ここには 1 度も現れない。
    pub absent: Vec<(u32, String)>,
}

/// areka が公開する「望む鎖」。wintf 側の唯一の受け口。
#[derive(Resource, Default)]
pub struct ZOrderChainPlan {
    /// 望む鎖。`None` は既定状態（指定ゼロ＝1 命令も出さない・要件 6）。
    pub chain: Option<ChainPlan>,
    /// 内容が変わったことを示す。適用系が読んだら false へ戻す。
    pub dirty: bool,
}

/// 本 spec が張った横断 edge の帳簿（**被所有側**の Entity に付く）。
///
/// 被所有側に付けるのは、`clear_window_owner` が被所有側だけを引数に取るからである
/// （撤去の主語と帳簿の住処が同じになる）。`owner_hwnd` は**張った時点で実際に書き込んだ
/// 値**であり、撤去の前に `GetWindow(GW_OWNER)` の現況と突き合わせるために控える——
/// 食い違うときに撤去すると、ペア機構が張り替えた繋ぎを誤って外し、バルーン直上
/// （要件 6.3）を壊す。
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct CrossOwnerLink {
    /// 所有する窓（奥側）の Entity。
    pub owner: Entity,
    /// 所有される窓（手前側）の窓ハンドル。
    pub owned_hwnd: HWND,
    /// 張った時点で書き込んだ owner の窓ハンドル。撤去前の照合に使う。
    pub owner_hwnd: HWND,
    /// 張った時点でこの繋ぎが属していた区間（撤去の記録に載せる）。
    ///
    /// 撤去は「グループが解けた」「窓が去った」といった、区間そのものが既に消えている
    /// 局面で起きる。そのとき望む鎖から区間を引くことはできないので、**張ったときの値を
    /// 帳簿が控える**。望む鎖が同じ繋ぎを別の区間へ付け替えたときは、実行環境を呼ばずに
    /// 控えだけを差し替える（適用系の帰属の更新）。
    pub segment: ChainSegment,
}

// SAFETY: `CrossOwnerLink` は `HWND`（`*mut c_void` の newtype）を 2 つ保持するため、
// 自動では Send/Sync が導出されず、この手動 impl は `Component`（ECS は Send+Sync を
// 要求）にするために必須である。健全性: 保持するのは窓の不透明な識別子の**値の写し**で
// あり、所有権も破棄責務も持たない。この HWND を Win32 へ渡すのは鎖の適用系であり、
// UI スレッド固定された system である。既存の `OwnerLink`／`ZOrder::InsertAfter(HWND)`
// と同根の扱いである。
unsafe impl Send for CrossOwnerLink {}
unsafe impl Sync for CrossOwnerLink {}

// ============================================================================
// 差分の純判断——必ず「全ての撤去が先・全ての付与が後」
// ============================================================================

/// 鎖へ出すべき操作 1 つ。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum ChainOp {
    /// 先に外す。
    Detach {
        /// 所有される窓（手前側）＝帳簿の住処。
        owned: Entity,
        /// 外す理由（記録に載る）。
        reason: DetachReason,
    },
    /// 次に張る。
    Attach {
        /// 所有される窓（手前側）。
        owned: Entity,
        /// 所有する窓（奥側）。
        owner: Entity,
    },
}

/// 望む edge 列と現況の帳簿から、出すべき操作列を導く（純関数）。
///
/// 返り値は必ず **すべての [`ChainOp::Detach`] が先・すべての [`ChainOp::Attach`] が後**
/// である（§12.4 の実測手順「切る 1 本 → 張る 2 本」）。混ぜると、外す前に張った窓が
/// 一時的に 2 つの所有者を主張する形になり、途中状態で鎖が分岐する。
///
/// # ここで出る撤去の理由は 2 つだけ
///
/// - [`DetachReason::Teardown`]: 望む鎖にその窓の繋ぎがもう無い（解除・後方配置の撤去）
/// - [`DetachReason::Rechain`]: 同じ窓の相手が別の窓へ変わる（スプライス）
///
/// 残る 2 つは**この純関数では決まらない**——[`DetachReason::Departing`] は窓が去る事実
/// （帳簿より前の段で判る）、[`DetachReason::Diverged`] は撤去の直前に読んだ OS の現況と
/// 帳簿の食い違い（Win32 を読まないと判らない）であり、どちらも適用系が決める。
///
/// # 望む列に同じ窓が 2 度現れたら先頭に近い方だけを採る
///
/// 前提（`compose_chain` の不変条件）が満たされていればこの形は来ない。それでも
/// **構造で潰してある**のは、ここが「同じ窓へ 2 つの所有者を主張する」＝要件 14.4 が
/// 禁じた星形・分岐が生まれうる唯一の場所だからである。手前側を残すのは、鎖が手前から
/// 奥へ並ぶ列であり、先に現れた繋ぎの方が鎖の頭に近いためである。
pub(crate) fn plan_chain_ops(
    desired: &[CrossEdge],
    current: &[(Entity, CrossOwnerLink)],
) -> Vec<ChainOp> {
    // 同じ被所有側の重複を先頭優先で畳む（以降はこの列だけを望みとして扱う）。
    let mut wanted: Vec<CrossEdge> = Vec::with_capacity(desired.len());
    for edge in desired {
        if wanted.iter().any(|kept| kept.owned == edge.owned) {
            continue;
        }
        wanted.push(*edge);
    }

    let wanted_owner_of = |owned: Entity| {
        wanted
            .iter()
            .find(|edge| edge.owned == owned)
            .map(|edge| edge.owner)
    };
    let held_owner_of = |owned: Entity| {
        current
            .iter()
            .find(|(recorded, _)| *recorded == owned)
            .map(|(_, link)| link.owner)
    };

    // ⑴ 撤去——帳簿にあって、望みと食い違うもの。
    let mut ops: Vec<ChainOp> = current
        .iter()
        .filter_map(|(owned, link)| match wanted_owner_of(*owned) {
            Some(owner) if owner == link.owner => None,
            Some(_) => Some(ChainOp::Detach {
                owned: *owned,
                reason: DetachReason::Rechain,
            }),
            None => Some(ChainOp::Detach {
                owned: *owned,
                reason: DetachReason::Teardown,
            }),
        })
        .collect();

    // ⑵ 付与——望みにあって、帳簿がまだその相手を持っていないもの。
    ops.extend(wanted.iter().filter_map(|edge| {
        if held_owner_of(edge.owned) == Some(edge.owner) {
            return None;
        }
        Some(ChainOp::Attach {
            owned: edge.owned,
            owner: edge.owner,
        })
    }));

    ops
}

// ============================================================================
// 後押しの選定——自分の窓 2 枚しか参照しない 1 形
// ============================================================================

/// 後押しの指令を組む（純関数・§12.2 実測 9）。
///
/// 表示中の窓の owner を張り替えても、それだけでは重なりが動かない（§12.2 実測 6）。
/// 収めるには Z を伴う指令を 1 回だけ出す必要がある。その形を
/// **鎖の先頭（`members[0]`）を 2 番目（`members[1]`）の直後へ差し直す**の 1 つに固定する。
/// 参照するのはどちらも自分のゴースト窓であり、主張する関係は鎖が既に強制しているものと
/// 同じなので、位置・寸法は変わらず（要件 11.1）、鎖の外どうしの相対順も変わらない
/// （要件 6.1／6.2）。
///
/// # 他の形を選んではならない
///
/// - `SWP_NOZORDER`（触るだけ）は再整列を起こさない（実測 7）
/// - `GW_HWNDPREV` で得た「いま自分の 1 つ手前にいる窓」を挿入位置に渡す形は、その窓が
///   **他プロセスのもの**でありうる。読み取りと書き込みの間にそれが消えると
///   `SetWindowPos` が黙って失敗し、鎖が収まらない（`research.md` §12.9 の 2 件目で実測）
/// - `HWND_TOP`／`HWND_BOTTOM` の絶対帯指定は、グループの絶対位置を無用に動かす
///
/// いずれの禁じ手も「指令が名指しする窓が `members` の外から来る」形で現れる。兄弟テストは
/// 指令に現れる窓ハンドルを列挙して `members` の部分集合であることを主張しており、
/// 差し替えた瞬間に赤くなる。
///
/// # 位置・寸法を運ぶ欄がそもそも無い
///
/// 指令は位置も寸法も持たない [`WindowPos`] から組む。よって
/// `SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE` は「フラグを立て忘れない」ではなく
/// **組み立て方から従う**——これが要件 11.1 の構造的な根拠である（既存ペア機構の
/// `pair_fix_command` と同じ規律）。
///
/// # 積まずに返す
///
/// 返すのは値であって、[`SetWindowPosCommand::enqueue`] の遅延キューへは積まない。
/// 理由は 2 つ——⑴ 要件 9.2 が「組み替えの**直後に実測**した重なり」を求めており、
/// 遅延バッチでは同じ巡で測れない。⑵ 積む側は内部で起床の印を立てるので、通せば
/// 「書いたあとに次の巡を促す」形になり、要件 14.2 が退役させた反復是正が裏口から戻る。
///
/// `members.len() < 2` のときは後押しを出さない（張るべき繋ぎも 1 本も無い）。
pub(crate) fn nudge_command(members: &[HWND]) -> Option<SetWindowPosCommand> {
    let head = *members.first()?;
    let second = *members.get(1)?;

    let pos = WindowPos {
        zorder: ZOrder::InsertAfter(second),
        position: None,
        size: None,
        no_activate: true,
        ..WindowPos::new()
    };
    Some(SetWindowPosCommand::new(
        head,
        0,
        0,
        0,
        0,
        pos.build_flags_for_system(),
        pos.get_hwnd_insert_after(),
    ))
}

// ============================================================================
// 記録の出口——鎖系 7 語のマクロ呼出はここだけ
// ============================================================================

// 以下 7 本は行組立を 1 つも持たない——本文は
// [`zorder_chain_diag`](super::zorder_chain_diag) の純関数の戻り値をそのまま載せる。
// 組立を二重に持つと、片方だけが書式を変えたときに実機の手順が静かに嘘になる。

/// 繋いだ事実を記録する（debug・要件 9.1）。
pub(crate) fn log_chain_linked(
    segment: Option<ChainSegment>,
    owned: Entity,
    owner: Entity,
    owned_hwnd: Option<HWND>,
    owner_hwnd: Option<HWND>,
    pos: usize,
    total: usize,
) {
    debug!(
        "{}",
        linked_line(segment, owned, owner, owned_hwnd, owner_hwnd, pos, total)
    );
}

/// 外した事実を記録する（debug・要件 4.1／7.2／9.1）。
pub(crate) fn log_chain_unlinked(
    segment: Option<ChainSegment>,
    owned: Entity,
    owned_hwnd: Option<HWND>,
    owner_hwnd: Option<HWND>,
    reason: DetachReason,
) {
    debug!(
        "{}",
        unlinked_line(segment, owned, owned_hwnd, owner_hwnd, reason)
    );
}

/// 収まった事実を記録する（debug・要件 9.2）。
///
/// 鎖全体につき 1 行である。宣言（`declared=`）と後押しの直後の実測（`measured=`）を
/// 同じ 1 行に載せるのは、分けると「指令は出したが効かなかった」の判定が 2 行の突合に
/// なり、過去に同型の誤診を生んでいるためである。
pub(crate) fn log_chain_settled(
    nudged_hwnd: Option<HWND>,
    insert_after: Option<HWND>,
    declared: &[HWND],
    measured: &[HWND],
    nudge_ok: Option<bool>,
) {
    debug!(
        "{}",
        settled_line(nudged_hwnd, insert_after, declared, measured, nudge_ok)
    );
}

/// 宣言された要素の窓が不在だった事実を記録する（debug・要件 1.4／8.4）。
///
/// 鎖系 7 語のうち、**これだけは areka から呼ばれる**——不在は「宣言と在庫の食い違い」
/// であり、それを知っているのは台帳と在庫を持つ側だからである。よって保全語彙 2 語
/// （[`log_group_applied`](super::zorder_chain_diag::log_group_applied) 等）と同じく
/// crate の外へ開いてある。出力先は本モジュールのままなので、grep の対象は割れない。
/// 呼び手を立てるのは指令消化の相の出口を差し替える task である
/// （材料は [`ChainPlan::absent`] が `(group_id, element)` の対で運ぶ）。
pub fn log_chain_absent(group_id: u32, element: &str) {
    debug!("{}", absent_line(group_id, element));
}

/// 見送りを理由つきで記録する（debug・要件 8.3）。
///
/// 理由の無い見送りを作れないよう、引数は [`ChainSkipReason`] の 1 つに限ってある。
pub(crate) fn log_chain_skipped(reason: ChainSkipReason) {
    debug!("{}", skipped_line(reason));
}

/// 張り失敗を記録する（error・要件 8.2）。
///
/// 失敗した繋ぎ 1 本だけを飛ばして残りは張る、というのが裁定である。よってこの行は
/// 「どの区間のどの対が張れなかったか」を単独で読めなければならない。
pub(crate) fn log_chain_link_failed(
    segment: Option<ChainSegment>,
    owned_hwnd: Option<HWND>,
    owner_hwnd: Option<HWND>,
    error: &windows::core::Error,
) {
    error!(
        "{}",
        link_failed_line(segment, owned_hwnd, owner_hwnd, error)
    );
}

/// 外し失敗を記録する（error・要件 8.2）。
pub(crate) fn log_chain_unlink_failed(owned_hwnd: Option<HWND>, error: &windows::core::Error) {
    error!("{}", unlink_failed_line(owned_hwnd, error));
}

#[cfg(test)]
#[path = "zorder_chain_tests.rs"]
mod zorder_chain_tests;

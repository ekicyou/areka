//! 所有の鎖の**適用系**——差分を実行環境へ書き、後押しを 1 回出し、直後に実測して記録する
//! （要件 6.3／6.4／7.1／7.2／8.2／8.3／9.1／9.2／9.3／10.4／11.1／14.1／14.2／14.5）。
//!
//! design.md「`apply_zorder_chain`（新設・system）」が正本である。判断そのものは
//! [`zorder_chain`](super::zorder_chain) の純関数が持ち、ここに在るのは
//! **「決まった操作を実行環境へ書く」ことと「書いた事実を記録する」こと**だけである。
//!
//! # 1 巡の手順（design.md の手順 1〜7 そのまま）
//!
//! 1. **去る窓の切離し**——所有側の窓が消えた繋ぎを、破棄に先立って外す（要件 7.2）。
//!    ここだけは望む鎖の変化と無関係に走る（下の「なぜ差分の門より前に置くのか」）
//! 2. [`plan_chain_ops`] で差分を得る（**撤去がすべて先・付与がすべて後**）
//! 3. 各撤去: **外す前に現況の所有者を読み**、帳簿の控えと一致するときだけ外す。
//!    食い違えば実行環境を呼ばずに帳簿だけ落とす（要件 8.3・§12.6）
//! 4. 各付与: 所有関係を張る。失敗した繋ぎは**その 1 本だけ**飛ばし、残りは張る（要件 8.2）
//! 5. 実際に操作が走った巡だけ、鎖全体へ**後押しを 1 回**（[`nudge_command`]・要件 11.1）
//! 6. 後押しの**直後に**重なりを実測し、宣言と実測を同じ 1 行へ載せる（要件 9.2／9.3）
//! 7. 変化の印を落とす。**追加の起床は要求しない**（要件 14.5）
//!
//! # 促しの呼び出しを持たない（要件 14.2）
//!
//! 本モジュールは `tick_wake` にも [`SetWindowPosCommand::enqueue`] にも触れない。
//! 積む経路は内部で起床の印を立てるので、後押しをそこへ通せば「書いたあとに次の巡を促す」
//! 形になり、実機で NO-GO になった反復是正の機構が裏口から戻る。後押しは
//! `guarded_set_window_pos` を**直に**呼ぶ——⑴要件 9.2 が「組み替えの直後に実測した
//! 重なり」を求めており、遅延バッチでは同じ巡で測れない。⑵挿入位置は「いま自分の鎖の
//! 2 番目に居る窓」という生の相対位置であり、同じバッチの他の指令がその窓を動かすと
//! 意味が変わる。この不在は兄弟テストが**対照つきで**字面から固定している。
//!
//! # 後押しに観測札（`WriteTag`）は付けない（task 1.3 の申し送りへの裁定）
//!
//! **直に書く経路へ札を付ける前例は本番に在る**——`window_proc/window_pos.rs:519-537` は
//! `guarded_set_window_pos` の直呼びに対して `WriteRecord { stage: WriteStage::Sync, .. }`
//! を自分で組み、`WriteTag { origin: ORIGIN_DPI_SUGGESTED, .. }` を明示している。
//! よって「札はキュー経由でしか付かない」は事実ではない。それでも後押しには付けない:
//!
//! - `guarded_set_window_pos` の引数に札の載る欄が無い。付けるなら
//!   [`SetWindowPosCommand`] の欄ではなく、上の前例のように**この場で書込レコードを
//!   自分で組む**ことになる。それは DPI 遷移の観測系（`transition_diag`）へ本 spec の
//!   語彙を 1 つ足す工事であり、要件 11.1／境界（位置・寸法を扱う経路には触れない）から
//!   離れる
//! - 鎖側の観測は `[zorder-chain] settled` の 5 欄（差し直した窓・挿入位置・宣言・実測・
//!   後押しの成否）で閉じており、実機サインオフもこの 1 行だけを読む。加えて
//!   `guarded_set_window_pos` は札の有無によらず窓書込 1 行（`via="SetWindowPos"`）を
//!   必ず出すので、「いつ・どの窓へ書いたか」は札なしでも追える
//!
//! 札を活かすためだけに遅延キューへ通すのは、要件 14.2 が禁じた形そのものである。
//!
//! # なぜ去る窓の切離しを差分の門より前に置くのか
//!
//! 望む鎖に変化が無ければ 1 命令も出さない（要件 6.4）——これは**組み替え**の話である。
//! 一方で「破棄に先立って外す」（要件 7.2）は、窓が去るという**別の出来事**への応答で
//! ある。窓が去ってから望む鎖が組み直されて公開されるまでには少なくとも 1 巡の間が
//! あり、その間に `DestroyWindow` が走ると OS の破棄カスケードが鎖の下流を巻き込む。
//! よって切離しだけは印を待たない。**去る窓が 1 枚も無ければ、この段は実行環境を
//! 1 度も呼ばない**ので、「変化が無ければ無操作」は割れない。
//!
//! # 実行環境の窓口は 5 つに絞ってある
//!
//! [`os_read_owner`]／[`os_clear_owner`]／[`os_set_owner`]／[`os_nudge`]／
//! [`os_measure_front`] の 5 本だけが実行環境へ触る。決定論テストはこの 5 本を台本つきの
//! 替え玉へ差し替えて 1 巡を丸ごと踏む（`command.rs` の
//! `with_forced_batch_begin_failure` と同じ、`#[cfg(test)]` の札で倒す形）。本番のビルドに
//! 替え玉は 1 バイトも入らない。
//!
//! # 区間（`segment=`）は望む鎖が運んでくる
//!
//! 要件 9.1 は「**どのグループの**どの窓を、どの窓のすぐ手前に位置づけたか」を求める。
//! グループの境界は連結された列からは**構造上復元できない**（`members` は「グループの
//! 連結＋後方配置」に畳まれており、どこで切れていたかが残らない）ので、区間は
//! [`CrossEdge::segment`] として計画に載って届く。付与ではその値を、撤去では
//! **帳簿が張った時点で控えた値**（[`CrossOwnerLink::segment`]）を記録へ載せる
//! ——撤去が起きる局面では望む鎖から区間を引けないためである。
//! 不在要素も同じ理由でグループ ID を伴って届く（[`ChainPlan::absent`]）。

use bevy_ecs::prelude::*;
use bevy_ecs::system::NonSendMarker;
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::GWLP_HWNDPARENT;

use crate::api::{clear_window_owner, get_window_long_ptr, set_window_owner};

use super::WindowHandle;
use super::command::{SetWindowPosCommand, guarded_set_window_pos};
use super::zorder_chain::{
    ChainOp, CrossEdge, CrossOwnerLink, ZOrderChainPlan, log_chain_link_failed, log_chain_linked,
    log_chain_settled, log_chain_skipped, log_chain_unlink_failed, log_chain_unlinked,
    nudge_command, plan_chain_ops,
};
use super::zorder_chain_diag::{ChainSegment, ChainSkipReason, DetachReason};
use super::zorder_pair::measure_windows_in_front;

// ============================================================================
// クエリの形
// ============================================================================

/// 本 spec が張った繋ぎの帳簿をひと通り見るためのクエリ（帳簿は被所有側に付く）。
type CrossLinkQuery<'w, 's> = Query<'w, 's, (Entity, &'static CrossOwnerLink)>;

/// すべての entity に当たるクエリ（実体の生存と窓ハンドルの有無を分けて読むための形）。
///
/// `Err` は実体が消えた（despawn された）こと、`Ok(None)` は実体は在るが OS のハンドルが
/// まだ／もう無いことを意味する。既存ペア機構の `HandleQuery` と同じ形である。
type HandleQuery<'w, 's> = Query<'w, 's, Option<&'static WindowHandle>>;

// ============================================================================
// 適用系（system）
// ============================================================================

/// 鎖の適用系: 去る窓を切離し、差分を書き、後押しを 1 回出し、直後に実測して記録する。
///
/// # UI スレッド固定
///
/// [`NonSendMarker`] を取るのは Win32（所有関係の読み書き・後押し・前面走査）を呼ぶため
/// である。この印が付いた system をスケジュール実行器はメインスレッド以外で走らせない。
///
/// # 記録を残さないまま諦めない（要件 8.3）
///
/// 見送り・食い違い・失敗のいずれの経路にも記録がある——見送りは
/// `[zorder-chain] skipped reason=`、食い違いは `[zorder-chain] unlinked reason=Diverged`、
/// 失敗は `[zorder-chain] link-failed`／`unlink-failed` である。**唯一記録を出さないのは
/// 「印が立っていない巡」**であり、これは諦めではなく仕事が無いことである（毎巡出すと
/// 記録が氾濫し、実機のログ判定が使い物にならなくなる）。
pub fn apply_zorder_chain(
    _ui_thread: NonSendMarker,
    mut commands: Commands,
    plan: Option<ResMut<ZOrderChainPlan>>,
    links: CrossLinkQuery,
    handles: HandleQuery,
) {
    // ⑴ 去る窓の切離しは印を待たない（module doc「なぜ差分の門より前に置くのか」）。
    let departed = detach_cross_owner_links_for_departing(&mut commands, &links, &handles);

    // 受け口がまだ無い巡（結線前・areka が公開していない）は仕事そのものが無い。
    let Some(mut plan) = plan else {
        return;
    };
    // 望む鎖に変化が無ければ即座に何もしない（要件 6.4／14.2）。
    if !plan.dirty {
        return;
    }
    plan.dirty = false;

    let (members, desired) = match plan.chain.as_ref() {
        Some(chain) => (chain.members.clone(), chain.cross_edges.clone()),
        // 既定状態へ戻す公開（解除）——望む繋ぎはゼロであり、帳簿の全件が撤去される。
        None => (Vec::new(), Vec::new()),
    };

    // 現況の帳簿。⑴で外した分は差し引く——`Commands` の除去はこの system の終了後に
    // 適用されるため、クエリはまだ古い姿を返す。
    let current: Vec<(Entity, CrossOwnerLink)> = links
        .iter()
        .filter(|(owned, _)| !departed.contains(owned))
        .map(|(owned, link)| (owned, *link))
        .collect();

    let ops = plan_chain_ops(&desired, &current);
    // 相手が同じまま区間だけが変わった繋ぎは、実行環境を呼ばずに帳簿の控えだけ
    // 差し替える（撤去の記録に載る区間が古いままにならないようにする）。
    refresh_ledger_segments(&mut commands, &desired, &current);

    if ops.is_empty() {
        // 印は立っていたが出す操作が無い（同じ内容の再公開・ペアだけの鎖）。
        log_chain_skipped(ChainSkipReason::NoChange);
        return;
    }

    let acted = execute_ops(&mut commands, &ops, &desired, &current, &members, &handles);
    if !acted {
        // 1 度も実行環境を呼べなかった（ハンドル未取得・食い違いだけの巡）。理由は
        // 各操作の側で記録済みであり、後押しを出す根拠が無いので鎖には触れない。
        return;
    }

    nudge_and_measure(&members, &handles);
}

// ============================================================================
// ⑴ 去る窓の切離し（破棄に先立って外す・要件 7.2）
// ============================================================================

/// 所有側の窓が去った繋ぎを外し、外した被所有側の Entity を返す。
///
/// # なぜ所有側だけを見るのか
///
/// OS の破棄カスケードは**所有する窓を壊すと所有される窓も壊す**向きに働く。よって
/// 巻き込みを断つには、所有側が消える前に被所有側の所有関係を外せばよい。被所有側が
/// 先に消える場合は、帳簿（[`CrossOwnerLink`]）もその実体と一緒に消えるので、そもそも
/// ここへ現れない——そして被所有側の破棄は誰も巻き込まない。
///
/// # 外し方は通常の撤去と同じ規律
///
/// 帳簿にあり、かつ**外す前に読んだ現況が帳簿と一致する**ものだけを外す（§12.6）。
/// 食い違えば実行環境を呼ばずに帳簿だけ落とす——ペア機構が張り替えた繋ぎを誤って外すと
/// バルーンがキャラ窓の直上という不変条件（要件 6.3）を壊すためである。
fn detach_cross_owner_links_for_departing(
    commands: &mut Commands,
    links: &CrossLinkQuery,
    handles: &HandleQuery,
) -> Vec<Entity> {
    let mut departed = Vec::new();
    for (owned, link) in links.iter() {
        let owner_gone = match handles.get(link.owner) {
            // 所有側は健在——生きている窓の関係には触れない。
            Ok(Some(_)) => false,
            // 実体は在るがハンドルが外れた／実体ごと消えた。どちらも「窓はもう無い」。
            Ok(None) | Err(_) => true,
        };
        if !owner_gone {
            continue;
        }
        detach_one(commands, owned, link, DetachReason::Departing);
        departed.push(owned);
    }
    departed
}

/// 相手が同じまま区間だけが変わった繋ぎについて、帳簿の控えを差し替える。
///
/// 同じ 2 枚の隣り合わせが、あるときは後方配置の一部であり、あるときはグループの一部で
/// ある——という付け替えは実際に起こる（未指定だったスコープが、同じ並びのままグループに
/// 名指しされた場合）。所有関係そのものは変わらないので**実行環境は 1 度も呼ばない**が、
/// 控えを放っておくと、後で外したときの記録が古い区間を名乗る。
fn refresh_ledger_segments(
    commands: &mut Commands,
    desired: &[CrossEdge],
    current: &[(Entity, CrossOwnerLink)],
) {
    for edge in desired {
        let Some((_, link)) = current.iter().find(|(owned, _)| *owned == edge.owned) else {
            continue;
        };
        if link.owner != edge.owner || link.segment == edge.segment {
            continue;
        }
        commands.entity(edge.owned).insert(CrossOwnerLink {
            segment: edge.segment,
            ..*link
        });
    }
}

// ============================================================================
// ⑵〜⑷ 差分の実行
// ============================================================================

/// 操作列を順に実行する。実行環境を 1 度でも呼んだら `true` を返す。
///
/// 返り値が後押しの要否そのものである（要件: 実際に操作が走った巡だけ後押しを 1 回）。
/// 食い違いによる帳簿の取り下げやハンドル未取得の見送りは実行環境を呼ばないので、
/// それだけの巡では後押しも出ない——書いていないものを収める必要は無い。
fn execute_ops(
    commands: &mut Commands,
    ops: &[ChainOp],
    desired: &[CrossEdge],
    current: &[(Entity, CrossOwnerLink)],
    members: &[Entity],
    handles: &HandleQuery,
) -> bool {
    let total = members.len();
    let mut acted = false;

    for op in ops {
        match *op {
            ChainOp::Detach { owned, reason } => {
                let Some((_, link)) = current.iter().find(|(recorded, _)| *recorded == owned)
                else {
                    // 帳簿に無いものは外さない（純関数は帳簿からしか撤去を作らないので
                    // ここへは来ないが、来たら黙って通さず見送りとして記録する）。
                    log_chain_skipped(ChainSkipReason::HandleMissing);
                    continue;
                };
                acted |= detach_one(commands, owned, link, reason);
            }
            ChainOp::Attach { owned, owner } => {
                // 区間は望む鎖が運んでくる（構造上ここでは復元できない・要件 9.1）。
                let segment = desired
                    .iter()
                    .find(|edge| edge.owned == owned && edge.owner == owner)
                    .map(|edge| edge.segment);
                let (Some(owned_hwnd), Some(owner_hwnd)) =
                    (hwnd_of(handles, owned), hwnd_of(handles, owner))
                else {
                    // どちらかの窓ハンドルがまだ取れていない。**この 1 本だけ**を
                    // 理由つきで見送り、残りの繋ぎは張る（要件 8.2／8.3）。
                    log_chain_skipped(ChainSkipReason::HandleMissing);
                    continue;
                };
                acted = true;
                match os_set_owner(owned_hwnd, owner_hwnd) {
                    Ok(()) => {
                        commands.entity(owned).insert(CrossOwnerLink {
                            owner,
                            owned_hwnd,
                            owner_hwnd,
                            // **本番では常に実値である**——`plan_chain_ops` は `desired` に
                            // 在る繋ぎからしか付与を作らないので、上の検索は必ず当たる。
                            // ここが到達するのは差分の純関数が壊れたときだけであり、
                            // そのとき帳簿を空欄にはできないので後方配置へ倒す（到達不能な
                            // 防御であって、通常の経路の既定値ではない）。
                            segment: segment.unwrap_or(ChainSegment::Tail),
                        });
                        let pos = members
                            .iter()
                            .position(|m| *m == owned)
                            .map_or(0, |i| i + 1);
                        log_chain_linked(
                            segment,
                            owned,
                            owner,
                            Some(owned_hwnd),
                            Some(owner_hwnd),
                            pos,
                            total,
                        );
                    }
                    Err(err) => {
                        // 失敗した繋ぎ 1 本だけを飛ばす。同じ巡で再試行はしない
                        // （同じハンドルへの再試行は同じ失敗を繰り返すだけ）。
                        log_chain_link_failed(segment, Some(owned_hwnd), Some(owner_hwnd), &err);
                    }
                }
            }
        }
    }

    acted
}

/// 繋ぎ 1 本を外す（照合 → 撤去 → 帳簿の取り下げ）。実行環境を呼んだら `true`。
///
/// **照合は省略できない**——帳簿の控えと現況が食い違うのは、ペア機構が同じ窓の所有関係を
/// 張り替えた場合である。そこで外すと、バルーンがキャラ窓の直上という既存の不変条件
/// （要件 6.3）を壊す。食い違いでは実行環境を呼ばず、帳簿だけを落として理由を記録する。
fn detach_one(
    commands: &mut Commands,
    owned: Entity,
    link: &CrossOwnerLink,
    reason: DetachReason,
) -> bool {
    // 読むのも書くのも**帳簿が控えている窓**である。実体側のハンドルが差し替わって
    // いれば、それは自分が書いた窓ではない＝食い違いとして現況の照合で落ちる。
    let hwnd = link.owned_hwnd;
    let touched = match os_read_owner(hwnd) {
        Ok(actual) if actual == link.owner_hwnd => match os_clear_owner(hwnd) {
            Ok(()) => {
                log_chain_unlinked(
                    Some(link.segment),
                    owned,
                    Some(hwnd),
                    Some(link.owner_hwnd),
                    reason,
                );
                true
            }
            Err(err) => {
                log_chain_unlink_failed(Some(hwnd), &err);
                true
            }
        },
        Ok(_) => {
            // 現況が帳簿と違う。**実行環境は呼ばない**（要件 8.3・§12.6）。
            log_chain_unlinked(
                Some(link.segment),
                owned,
                Some(hwnd),
                Some(link.owner_hwnd),
                DetachReason::Diverged,
            );
            false
        }
        Err(err) => {
            // 現況が読めなければ「帳簿と一致する」と確かめられない。よって外さない。
            // 読めなかった事実そのものを失敗として残す（黙って諦めない・要件 8.3）。
            log_chain_unlink_failed(Some(hwnd), &err);
            false
        }
    };
    commands.entity(owned).remove::<CrossOwnerLink>();
    touched
}

// ============================================================================
// ⑸⑹ 後押し 1 回と、その直後の実測
// ============================================================================

/// 鎖全体へ後押しを 1 回出し、**その直後に**重なりを実測して 1 行へ載せる（要件 9.2）。
///
/// 後押しの形は [`nudge_command`] が 1 つに固定している（鎖の先頭を 2 番目の直後へ
/// 差し直す・参照するのは自分の窓 2 枚だけ）。窓が 2 枚未満なら後押しを出さない
/// ——張るべき繋ぎも無く、収めるものが無いためである。その場合も理由つきの見送りを
/// 記録する（黙って諦めない・要件 8.3）。
fn nudge_and_measure(members: &[Entity], handles: &HandleQuery) {
    let declared: Vec<HWND> = members
        .iter()
        .filter_map(|m| hwnd_of(handles, *m))
        .collect();

    let Some(cmd) = nudge_command(&declared) else {
        log_chain_skipped(ChainSkipReason::TooFewPresent);
        return;
    };

    let nudge_ok = os_nudge(&cmd).is_ok();
    // **後押しの直後**に測る。間に他の書込を挟むと、測った並びが何の結果なのかが
    // 事後に決められなくなる（要件 9.2 が同一行を求めるのはそのためである）。
    let measured = measure_chain_order(&declared);

    log_chain_settled(
        Some(cmd.hwnd),
        cmd.hwnd_insert_after,
        &declared,
        &measured,
        Some(nudge_ok),
    );
}

/// 鎖の窓が実際にどの順に並んでいるかを、手前から奥へ実測する（要件 9.3）。
///
/// 既存の前面走査（[`measure_windows_in_front`]）をそのまま流用する——**実行環境上で
/// 非表示の窓は列に入らない**ので、既定の IME 窓のような不可視の隣が挟まっても結果が
/// 動かない。走査は鎖の最も奥の窓を起点に手前へ辿り、出会った窓のうち鎖の要素だけを拾う。
///
/// 戻り値は手前から奥の順であり、最後の要素は起点そのものである。走査が失敗ないし
/// 打切りになった場合は拾えたところまでを返す（走査側が理由を記録する）。
fn measure_chain_order(declared: &[HWND]) -> Vec<HWND> {
    let Some(root) = declared.last().copied() else {
        return Vec::new();
    };
    let mut measured: Vec<HWND> = os_measure_front(root)
        .into_iter()
        .filter(|found| declared.contains(found))
        .collect();
    // 走査は起点から手前へ進むので、拾った順は奥から手前である。宣言と同じ向き
    // （手前から奥）へ揃えてから起点を末尾へ置く。
    measured.reverse();
    measured.push(root);
    measured
}

// ============================================================================
// 道具
// ============================================================================

/// Entity の窓ハンドルを引く（実体が無い／ハンドル未取得はどちらも `None`）。
fn hwnd_of(handles: &HandleQuery, entity: Entity) -> Option<HWND> {
    match handles.get(entity) {
        Ok(Some(handle)) => Some(handle.hwnd),
        Ok(None) | Err(_) => None,
    }
}

// ============================================================================
// 実行環境への窓口（5 本だけ・檻はここを替え玉へ差し替える）
// ============================================================================

/// いまその窓を所有している窓を読む。所有者が無ければ NULL の窓ハンドルが返る。
///
/// 読むのは `GWLP_HWNDPARENT` である——トップレベル窓に対するこの欄は親子関係ではなく
/// **owner** を指し、`set_window_owner`／`clear_window_owner` が書くのと同じ欄である
/// （`api.rs` の檻が `GetWindow(GW_OWNER)` と同じ値になることを実窓で固定している）。
/// 書いた欄をそのまま読み戻す形にしてあるので、照合が別の概念を突き合わせることがない。
fn os_read_owner(hwnd: HWND) -> windows::core::Result<HWND> {
    #[cfg(test)]
    if let Some(scripted) = double::read_owner(hwnd) {
        return scripted;
    }
    get_window_long_ptr(hwnd, GWLP_HWNDPARENT).map(|value| HWND(value as *mut _))
}

/// 所有関係を外す。
fn os_clear_owner(hwnd: HWND) -> windows::core::Result<()> {
    #[cfg(test)]
    if let Some(scripted) = double::clear_owner(hwnd) {
        return scripted;
    }
    clear_window_owner(hwnd)
}

/// 所有関係を張る（`owned` が `owner` に所有される）。
fn os_set_owner(owned: HWND, owner: HWND) -> windows::core::Result<()> {
    #[cfg(test)]
    if let Some(scripted) = double::set_owner(owned, owner) {
        return scripted;
    }
    set_window_owner(owned, owner)
}

/// 後押しを 1 回出す（遅延キューを通さず直に書く）。
fn os_nudge(cmd: &SetWindowPosCommand) -> windows::core::Result<()> {
    #[cfg(test)]
    if let Some(scripted) = double::nudge(cmd) {
        return scripted;
    }
    // SAFETY: Win32 境界。渡すのは呼び出し側が保持する `HWND` と整数だけであり、
    // 所有権も生存参照も持ち出さない。位置・寸法の欄は指令の組み立て方から
    // `SWP_NOMOVE | SWP_NOSIZE` が立つため読まれない（要件 11.1）。
    unsafe {
        guarded_set_window_pos(
            cmd.hwnd,
            cmd.hwnd_insert_after,
            cmd.x,
            cmd.y,
            cmd.width,
            cmd.height,
            cmd.flags,
        )
    }
}

/// 指定窓より手前に居る**可視の**窓を、手前へ向かって辿って集める（要件 9.3）。
fn os_measure_front(hwnd: HWND) -> Vec<HWND> {
    #[cfg(test)]
    if let Some(scripted) = double::measure_front(hwnd) {
        return scripted;
    }
    measure_windows_in_front(hwnd).windows
}

// ============================================================================
// 決定論テスト用の替え玉（`#[cfg(test)]`・本番のビルドには 1 バイトも入らない）
// ============================================================================

/// 実行環境の 5 つの窓口を台本つきで置き換える器。
///
/// 実窓を使わずに 1 巡を丸ごと踏むための道具である（実窓の檻は別 task の担当）。
/// `command.rs` の `with_forced_batch_begin_failure` と同じ、`#[cfg(test)]` の
/// スレッドローカルで倒す形をとる。
#[cfg(test)]
pub(crate) mod double {
    use std::cell::RefCell;
    use std::collections::{HashMap, HashSet};

    use windows::Win32::Foundation::HWND;
    use windows::core::HRESULT;

    use super::SetWindowPosCommand;

    /// 記録される実行環境の呼び出し 1 件（順序の主張に使う）。
    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    pub(crate) enum Call {
        /// 現在の所有者を読んだ。
        ReadOwner(usize),
        /// 所有関係を外した。
        ClearOwner(usize),
        /// 所有関係を張った（被所有側, 所有側）。
        SetOwner(usize, usize),
        /// 後押しを出した（差し直した窓, 挿入位置）。
        Nudge(usize, usize),
        /// 前面走査を行った。
        MeasureFront(usize),
    }

    /// 台本と、走行中に積まれた呼び出しの記録。
    #[derive(Default)]
    pub(crate) struct Script {
        /// 走行中に実際に呼ばれた窓口の列（**順序込み**）。
        pub calls: Vec<Call>,
        /// `owned` の現在の所有者（載っていない窓は所有者無し＝0）。
        pub owner_of: HashMap<usize, usize>,
        /// 所有者の読み取りを失敗させる窓。
        pub read_fails: HashSet<usize>,
        /// 撤去を失敗させる窓。
        pub clear_fails: HashSet<usize>,
        /// 付与を失敗させる窓（被所有側で指定する）。
        pub set_fails: HashSet<usize>,
        /// 後押しを失敗させる。
        pub nudge_fails: bool,
        /// 前面走査が返す列（起点 → 手前へ辿った順）。
        pub front_of: HashMap<usize, Vec<usize>>,
    }

    thread_local! {
        static SCRIPT: RefCell<Option<Script>> = const { RefCell::new(None) };
    }

    /// テスト用の失敗値。
    fn failure() -> windows::core::Error {
        windows::core::Error::from(HRESULT(0x8007_0578u32 as i32))
    }

    /// 台本を据えて `body` を走らせ、走行後の台本（呼び出しの記録込み）を返す。
    pub(crate) fn with_script<R>(script: Script, body: impl FnOnce() -> R) -> (R, Script) {
        struct Restore;
        impl Drop for Restore {
            fn drop(&mut self) {
                SCRIPT.with(|cell| *cell.borrow_mut() = None);
            }
        }
        SCRIPT.with(|cell| *cell.borrow_mut() = Some(script));
        let _restore = Restore;
        let out = body();
        let used = SCRIPT.with(|cell| cell.borrow_mut().take());
        (out, used.expect("台本は走行中に落ちない"))
    }

    /// 台本が据わっていれば `f` を適用し、結果を返す（据わっていなければ `None`）。
    fn with<R>(f: impl FnOnce(&mut Script) -> R) -> Option<R> {
        SCRIPT.with(|cell| cell.borrow_mut().as_mut().map(f))
    }

    pub(super) fn read_owner(hwnd: HWND) -> Option<windows::core::Result<HWND>> {
        with(|s| {
            let key = hwnd.0 as usize;
            s.calls.push(Call::ReadOwner(key));
            if s.read_fails.contains(&key) {
                return Err(failure());
            }
            let owner = s.owner_of.get(&key).copied().unwrap_or(0);
            Ok(HWND(owner as *mut _))
        })
    }

    pub(super) fn clear_owner(hwnd: HWND) -> Option<windows::core::Result<()>> {
        with(|s| {
            let key = hwnd.0 as usize;
            s.calls.push(Call::ClearOwner(key));
            if s.clear_fails.contains(&key) {
                return Err(failure());
            }
            s.owner_of.remove(&key);
            Ok(())
        })
    }

    pub(super) fn set_owner(owned: HWND, owner: HWND) -> Option<windows::core::Result<()>> {
        with(|s| {
            let key = owned.0 as usize;
            s.calls.push(Call::SetOwner(key, owner.0 as usize));
            if s.set_fails.contains(&key) {
                return Err(failure());
            }
            s.owner_of.insert(key, owner.0 as usize);
            Ok(())
        })
    }

    pub(super) fn nudge(cmd: &SetWindowPosCommand) -> Option<windows::core::Result<()>> {
        with(|s| {
            s.calls.push(Call::Nudge(
                cmd.hwnd.0 as usize,
                cmd.hwnd_insert_after.map_or(0, |h| h.0 as usize),
            ));
            if s.nudge_fails {
                return Err(failure());
            }
            Ok(())
        })
    }

    pub(super) fn measure_front(hwnd: HWND) -> Option<Vec<HWND>> {
        with(|s| {
            let key = hwnd.0 as usize;
            s.calls.push(Call::MeasureFront(key));
            s.front_of
                .get(&key)
                .map(|found| found.iter().map(|v| HWND(*v as *mut _)).collect())
                .unwrap_or_default()
        })
    }
}

#[cfg(test)]
#[path = "zorder_chain_apply_tests.rs"]
mod zorder_chain_apply_tests;

/// 実窓での最終形の檻（替え玉を据えないので、上の 5 つの窓口はそのまま Win32 を呼ぶ）。
#[cfg(test)]
#[path = "zorder_chain_order_tests.rs"]
mod zorder_chain_order_tests;

/// 実窓での出入りの檻（解除・スプライス・破棄の非連動）。
///
/// 兄弟の [`zorder_chain_order_tests`] と足場を共有するため別ファイルに分けてある
/// ——1 ファイル 1,000 行未満という本 spec の共通制約に従う。
#[cfg(test)]
#[path = "zorder_chain_order_lifecycle_tests.rs"]
mod zorder_chain_order_lifecycle_tests;

/// 実窓での**縛らないもの**の檻（鎖の外どうしの相対順・既定状態＝非強制）。
///
/// 兄弟 2 本と足場を共有するため別ファイルに分けてある——1 ファイル 1,000 行未満という
/// 本 spec の共通制約に従う。
#[cfg(test)]
#[path = "zorder_chain_order_outsider_tests.rs"]
mod zorder_chain_order_outsider_tests;

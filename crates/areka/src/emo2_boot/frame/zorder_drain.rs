//! 重なりの指令を台帳へ適用し、台帳を**鎖の計画**へ射影する相
//! （design「`zorder_drain`（既存・出口を差し替え）」・要件 1.4／4／5／7.1／8.4／
//! 14.5／15.3）。
//!
//! 兄弟の [`run_move_drain_phase`](super::drain_resnap::run_move_drain_phase) と同じ
//! 跨ぎの形である——台本のスレッドが送り出した指令を、画面を持つ側のスレッドで
//! 取り出して適用する。違うのは**適用した後にもう一仕事ある**ことで、こちらは台帳の
//! 内容を「いま実際に在る窓」の鎖へ写し、その写しを wintf の受け口へ置く。
//!
//! # 窓の正本が無い間は取り出さない
//!
//! `GhostWindows`（スコープ→窓 entity の唯一の正本）が World に居ない間は、指令を
//! 1 件も取り出さずに戻る。送信端と受信端をつなぐチャネルが**そのまま保留バッファを
//! 兼ねる**ので、取りこぼしは起きず、窓が生えた最初の相で到着順のまま一括で適用される
//! （move の相 `drain_resnap.rs:79-87` と同じ意図・要件 1.4）。取り出してから捨てる形に
//! すると、起動直後の `\![set,zorder,...]` が黙って消える。
//!
//! # 出口は「望む鎖 1 本」である
//!
//! 台帳はスコープ番号と窓種別のままで持ち、まだ現れていないスコープも取り除かない
//! （要件 1.4）。窓が実在するかを知っているのはこの相だけなので、
//! 「宣言 → 実在する窓の鎖」の写像はここが持つ。並びと繋ぎを決める判断そのものは
//! 純関数 [`compose_chain`] にあり、この相が足すのは 2 つだけである——在庫のスコープ
//! 一覧を引くことと、要素 1 つを実在する窓へ解決する規則を渡すことである。
//!
//! 「実在する」は 2 段である——`GhostWindows` にそのスコープが載っていること
//! （まだ生まれていない窓は載らない）と、指している entity が World にまだ居ること
//! （破棄済みは飛ばす・要件 7.2）。前者だけを見ると、対の後追い破棄の途中で
//! 既に消えた entity を受け口へ渡してしまう。
//!
//! # 公開は内容が前回と異なるときだけ（要件 14.5）
//!
//! 組んだ鎖が受け口の現在の内容と同じなら、書き込みも印立ても行わない。**窓の出現・
//! 破棄はこの 1 つの門で自然に検出される**——在庫が動けば合成の結果が動くからである。
//! どのグループにも属さないスコープの出入りも同じ門を通る（要件 7.1／15.3）。
//! 変化の検出に台帳の版（[`ZOrderGroupLedger::version`]）ではなく**合成の結果そのもの**を
//! 使うのは、版が進んでも結果が動かない場合があるからである（例: まだ 1 枚も窓が無い
//! スコープだけのグループが受理された巡）。結果の突き合わせは版の判定を包含する。
//!
//! グループが 1 つも無い状態では合成が計画そのものを作らず、受け口の Resource すら
//! 生えない——適用系は仕事を得られず、指令を 1 本も出さない。「既定状態では従来と
//! 同じ」（要件 6.1／6.4）はこの**不在**によって構造的に成り立つのであって、
//! 「出さないと判断する」ことによってではない。一度出来た受け口は解除の後も残す。
//! 消してしまうと、適用系が「張った繋ぎを外せ」という指示を受け取れなくなる
//! （要件 4.1／15.4）。
//!
//! # 記録はすべて wintf の唯一の入口を通す
//!
//! 受理・拒否・宣言要素の不在のいずれも、記録を出すのは wintf 側の
//! [`log_group_applied`]／[`log_group_rejected`]／[`log_chain_absent`] である。
//! `tracing` の出力先は呼び出し元の module path が既定なので、こちら側でマクロを
//! 呼ぶと実機サインオフの grep 対象が 2 本に割れる。本モジュールが組むのは
//! **本文の文字列**だけである。

use std::sync::mpsc::Receiver;

use bevy_ecs::entity::Entity;
use bevy_ecs::prelude::Resource;
use bevy_ecs::world::World;

use wintf::ecs::window::{
    ChainPlan, ZOrderChainPlan, log_chain_absent, log_group_applied, log_group_rejected,
};

use crate::emo2_boot::zorder_cue::ZOrderDirective;
use crate::placement::spawn::GhostWindows;
use crate::placement::zorder_chain_compose::{compose_chain, element_text};
use crate::placement::zorder_group_ledger::{
    GroupElement, GroupWindowKind, Normalization, ZOrderGroup, ZOrderGroupLedger, ZOrderReject,
    parse_zorder_tokens,
};

/// 値が無いことを表す番兵（既存のグループ系・ペア系の記録行と同じ字面）。
///
/// 欄ごと落とさないのは、「記録が出ていない」と「その経路にはその値が無い」の区別が
/// 事後に付かなくなるからである（退役した `zorder_group_diag.rs` の `UNKNOWN` と同じ規律。
/// あちらは wintf 内部の可視性ゆえ、同じ字面をこちらでも定義していた）。
const NO_VALUE: &str = "-";

// ---------------------------------------------------------------------------
// 相の本体
// ---------------------------------------------------------------------------

/// zorder drain 相（design「`zorder_drain`（既存・出口を差し替え）」・要件 1.4／4／5／
/// 6.1／7.1／7.2／8.4／14.5／15.3）。
///
/// 順に⑴窓の正本が無ければ何もしない ⑵届いている指令を到着順に台帳へ適用する
/// ⑶台帳と窓の在庫から望む鎖を組む ⑷窓が無かった宣言要素を報せる
/// ⑸内容が前回と異なれば受け口へ公開する。
///
/// # 引数が受け口と台帳を**直接**受け取る理由
///
/// 毎フレームの結線状態（`Emo2Wiring`）から読み出さず、受信端と台帳を引数で受け取る。
/// 結線状態へ欄を足すのは結線の task（6.2）の担当であり、この相の判断はそれを待たずに
/// 完成させられる——`Emo2Wiring` を 1 バイトも変えずに全分岐を檻へ入れられる形である。
///
/// # 失敗しても台本を殺さない
///
/// 解釈できない指定・不在の窓・閉じた受け口のいずれも、記録を残して次へ進むだけである
/// （log-first・非 panic）。1 件の縮退が後続の指令や他グループを巻き込まない
/// （要件 8.1／8.3）。
pub fn run_zorder_drain_phase(
    rx: &Receiver<ZOrderDirective>,
    ledger: &mut ZOrderGroupLedger,
    world: &mut World,
) {
    // ⑴ 窓の正本が無い間はチャネルが保留バッファを兼ねる（`try_iter` を呼ばない＝
    //    取りこぼさない）。窓が生えた最初の相で到着順のまま一括適用する（要件 1.4）。
    if world.get_resource::<GhostWindows>().is_none() {
        return;
    }

    // ⑵ 到着順（FIFO）に適用する。`try_iter` は現時点でキュー済みの指令を非ブロックで
    //    全件取り出し、空か全送信端 drop で尽きる（ブロックも panic もしない）。
    for directive in rx.try_iter() {
        apply_directive(ledger, &directive);
    }

    // ⑶ 台帳と窓の在庫から望む鎖を組む。
    let plan = compose_plan(ledger, world);
    // ⑷ 窓が無かった宣言要素の報告は**公開とは独立**に行う（下の関数の doc を参照）。
    let absent = plan
        .as_ref()
        .map(|chain| chain.absent.clone())
        .unwrap_or_default();
    report_absent_elements(world, &absent);
    // ⑸ 内容が前回と異なれば受け口へ置く。
    publish_chain_plan(world, plan);
}

// ---------------------------------------------------------------------------
// ⑵ 指令の適用（受理と拒否の記録つき）
// ---------------------------------------------------------------------------

/// 指令を 1 件だけ台帳へ適用し、受理か拒否かを記録する（要件 8.1／8.3／3.2／5.4）。
///
/// 拒否は 2 段のどちらでも起こり得る——トークンの解釈（モード混在・タグ内重複・
/// 要素 2 個未満・解釈不能）と、台帳への追加（既に他のグループが押さえているスコープ）。
/// どちらで落ちても**そのタグによる変更は一切行わない**（部分適用の禁止）。台帳が
/// 書き換わるのは検査を通り切った後だけなので、これは制御の流れで保証される。
///
/// 記録には**受け取ったトークン列**と**拒否理由**の両方を載せる（要件 8.1／8.3）。
/// どちらか一方だと、作者は何を書き間違えたのかを記録から復元できない。
fn apply_directive(ledger: &mut ZOrderGroupLedger, directive: &ZOrderDirective) {
    match directive {
        ZOrderDirective::Set { tokens } => {
            let borrowed: Vec<&str> = tokens.iter().map(String::as_str).collect();
            let (members, normalizations) = match parse_zorder_tokens(&borrowed) {
                Ok(parsed) => parsed,
                Err(reject) => {
                    log_group_rejected(&reject_reason_text(&reject), &tokens_text(tokens));
                    return;
                }
            };
            match ledger.try_add_tag_group(members) {
                Ok(id) => {
                    // 受理の記録は**台帳に載った後の内容**から組む（design の
                    // `applied`＝受理時の台帳内容）。正規化の記録も併せて載せる——
                    // 作者が書いた順をそのまま採らなかったことを報せる材料である
                    // （要件 2.4）。載せずに捨てると「黙って組み替えた」になる。
                    let detail = ledger
                        .groups()
                        .iter()
                        .find(|group| group.id == id)
                        .map(|group| set_applied_detail(group, &normalizations));
                    if let Some(detail) = detail {
                        log_group_applied(&detail);
                    }
                }
                Err(reject) => {
                    log_group_rejected(&reject_reason_text(&reject), &tokens_text(tokens));
                }
            }
        }
        ZOrderDirective::Reset => {
            ledger.reset_to_descript();
            log_group_applied(&reset_applied_detail(ledger));
        }
    }
}

// ---------------------------------------------------------------------------
// ⑶ 合成（台帳＋窓の在庫 → 望む鎖 1 本）
// ---------------------------------------------------------------------------

/// 台帳の全グループと窓の在庫から、望む鎖 1 本を組む
/// （要件 1.4／3.6／7.1／7.2／15.1／15.2）。
///
/// 並びと繋ぎを決める判断そのものは純関数 [`compose_chain`] が持つ。ここが足すのは
/// 在庫のスコープ一覧と、要素 1 つを実在する窓へ解決する規則の 2 つだけである。
///
/// `None` は既定状態（グループが 1 つも無い）を意味する。`GhostWindows` が無いときも
/// 同じく `None` を返す——呼び出し元が先に弾いているので、これは二重の防波堤である。
fn compose_plan(ledger: &ZOrderGroupLedger, world: &World) -> Option<ChainPlan> {
    let ghost_windows = world.get_resource::<GhostWindows>()?;
    // 未指定スコープの後方参加（要件 15.1）に使う在庫の一覧。合成側が昇順へ整えて
    // 重複も落とすので、ここでは順も重複も問わない。
    let all_scopes: Vec<u32> = ghost_windows
        .scopes()
        .filter_map(|scope| u32::try_from(scope).ok())
        .collect();
    compose_chain(ledger.groups(), &all_scopes, &|element| {
        resolve_member(ghost_windows, world, element)
    })
}

/// 要素 1 つを実在する窓の entity へ解決する（実在しなければ `None`）。
///
/// 実在の判定は 2 段である——`GhostWindows` に載っていること（まだ生まれていない窓は
/// 載らない）と、指す entity が World にまだ居ること（破棄済みは飛ばす・要件 7.2）。
/// 前者だけだと、対の後追い破棄の途中で既に消えた entity を受け口へ渡してしまう。
fn resolve_member(
    ghost_windows: &GhostWindows,
    world: &World,
    element: &GroupElement,
) -> Option<Entity> {
    let scope = usize::try_from(element.scope).ok()?;
    let entity = match element.kind {
        GroupWindowKind::Balloon => ghost_windows.balloon_window(scope)?,
        GroupWindowKind::Char => ghost_windows.char_window(scope)?,
    };
    world.get_entity(entity).ok().map(|_| entity)
}

// ---------------------------------------------------------------------------
// ⑷ 窓が無かった宣言要素の記録（公開とは独立）
// ---------------------------------------------------------------------------

/// 直近に報せた「窓が無かった宣言要素」の控え。
///
/// **正本ではない**。グループの内容（要素・順序・出所）は 1 バイトも持たず、
/// 「どの対を既に報せたか」だけを覚える。二重帳簿を作らないための線引きであり、
/// この Resource を失っても復元されるのは「もう一度同じ行が出る」ことだけで、
/// 重なりの判断には何の影響も無い。
#[derive(Resource, Default)]
struct ZOrderAbsentReports {
    /// 前回報せた `(グループ id, 要素の正準表記)` の並び（合成が返した順のまま）。
    reported: Vec<(u32, String)>,
}

/// 窓が無かった宣言要素を記録する（要件 8.3／8.4）。
///
/// # なぜ公開と切り離すのか
///
/// 公開には「受け口を作る理由が無い」「内容が動いていない」の 2 つの早期 return がある。
/// **要件 8.4 が名指しする「窓が一度も現れないまま推移する」形は鎖が空**なので、
/// 公開に紐付けると報告そのものが消える。既に安定した鎖が在るところへ全欠けのグループを
/// 足した場合も同様である。不在は**公開とは別の事実**であり、公開の有無に紐付けてはならない。
///
/// # 連呼はしない（毎巡走る相であることへの配慮）
///
/// 素直に毎回出すと、現れないスコープを 1 つ書いただけで同じ行が毎フレーム積もり、
/// 本物の変化を埋める。よって**前回報せた内容と違うときだけ**、その時点の不在を
/// 一式出す。揃えば控えは空になるので、再び欠ければまた報される。「一度きり」に
/// しないのは、欠けが増減した事実まで黙らせないためである。
///
/// 控えは報告の有無を決めるためだけに使い、判断には一切関与しない。
fn report_absent_elements(world: &mut World, absent: &[(u32, String)]) {
    // 報せるものが無く、控えもまだ無い＝既定状態。Resource を作らずに戻る
    // （グループが 1 つも無い間は何も生やさない・要件 6.1 と同じ姿勢）。
    if absent.is_empty() && world.get_resource::<ZOrderAbsentReports>().is_none() {
        return;
    }

    world.init_resource::<ZOrderAbsentReports>();
    let mut reports = world.resource_mut::<ZOrderAbsentReports>();
    if reports.reported == absent {
        return;
    }
    reports.reported = absent.to_vec();
    drop(reports);

    // 出す順は合成が返した順＝宣言の順（決定論・要件 10.3）。
    for (group_id, element) in absent {
        log_chain_absent(*group_id, element);
    }
}

// ---------------------------------------------------------------------------
// ⑸ 受け口への公開
// ---------------------------------------------------------------------------

/// 望む鎖が前回と違っていれば受け口へ置き、印を立てる（要件 4.1／6.1／7.1／14.5／15.3）。
///
/// # 同じ内容の巡では触れない
///
/// 受け口の現在の内容と一致するなら、書きもせず印も立てない。印は適用系への
/// 「書くべきものがある」の合図であり、何も動いていない巡に立てると適用系が毎巡
/// 空振りして、表示に変化の無い巡を省く門を実質無効にする。
///
/// # 既定状態では受け口そのものを作らない
///
/// グループが 1 つも無い（＝合成が `None`）とき、受け口をまだ持っていなければ挿入しない。
/// 適用系は仕事を得られず、指令を 1 本も出さない——要件 6.1／6.4 の「既定状態は従来と
/// 同じ」は、判断ではなくこの**不在**で成り立つ。
///
/// # 一度出来た受け口は解除でも消さない
///
/// 解除（要件 4.1／4.2／15.4）で公開するのは「鎖が無い」という**内容**であって、
/// 受け口の撤去ではない。Resource ごと消すと、適用系は自分が張った繋ぎを外す指示を
/// 受け取れないまま鎖が残る。
fn publish_chain_plan(world: &mut World, plan: Option<ChainPlan>) {
    if world.get_resource::<ZOrderChainPlan>().is_none() {
        if plan.is_none() {
            // 望む鎖が無く受け口も無い＝既定状態（要件 6.1 の構造的な根拠）。
            return;
        }
        world.init_resource::<ZOrderChainPlan>();
    }

    let mut receiver = world.resource_mut::<ZOrderChainPlan>();
    if receiver.chain == plan {
        // 何も動いていない＝公開も印立ても行わない（空振りの巡を作らない）。
        return;
    }
    receiver.chain = plan;
    receiver.dirty = true;
}

// ---------------------------------------------------------------------------
// 記録の本文（純関数・`tracing` を含まない）
// ---------------------------------------------------------------------------

/// 受理（`\![set,zorder,...]`）の本文——台帳に載った内容と、正規化で調整した内容。
///
/// `normalized` に載るのは、明示モードで名前の挙がったスコープごとの正規化の記録である。
/// `scope:true` は「作者が書いた順をそのままの形では採用しなかった」（同一スコープの
/// 2 窓を隣接ブロックへ寄せた・要件 2.4）を意味する。
///
/// 片方の窓しか書かれていなかったスコープには `+bN`／`+sN` が続き、**こちらが補った
/// 相棒窓**を名指しする（畳み込み・要件 2.6）。補いのときは並べ替えるべき 2 窓が
/// そもそも書かれていないので、字面は必ず `scope:false+bN` の形になる——`scope:false`
/// だけの欄は「2 窓とも書かれていて、その順をそのまま採った」を意味し続ける。
///
/// 数値モードでは調整も補いも起きないので常に番兵になる。
///
/// 起動の段（[`zorder_descript`](super::zorder_descript)）も shell 設定由来の基底を
/// 据えたときに**この関数**を呼ぶ。行の欄を二重に持つと、片方だけを直した日に記録の
/// 書式が静かに割れるからである。起動由来かタグ由来かは `source` 欄が弁別する。
pub(super) fn set_applied_detail(group: &ZOrderGroup, normalizations: &[Normalization]) -> String {
    format!(
        "action=set group_id={id} source={source:?} members={members} normalized={normalized}",
        id = group.id,
        source = group.source,
        members = members_text(&group.members),
        normalized = normalizations_text(normalizations),
    )
}

/// 受理（`\![reset,zorder]`）の本文——解除した後に台帳へ残った内容。
///
/// 解除は「何を落としたか」より「何が残ったか」で読む方が短い。基底が在れば残り 1 本、
/// 無ければ 0 本になる（要件 4.1／4.2）ので、`groups` と `base` の 2 欄で終状態が
/// 一意に決まる。
fn reset_applied_detail(ledger: &ZOrderGroupLedger) -> String {
    let base = ledger
        .groups()
        .iter()
        .find(|group| group.source == crate::placement::zorder_group_ledger::GroupSource::Descript);
    format!(
        "action=reset groups={count} base={base}",
        count = ledger.groups().len(),
        base = match base {
            Some(group) => members_text(&group.members),
            None => NO_VALUE.to_string(),
        },
    )
}

/// 拒否理由を 1 語へ畳む（空白を含めない——記録側が空白を `_` へ潰すため）。
///
/// 起動の段（[`zorder_descript`](super::zorder_descript)）も同じ関数を通す。理由の語彙が
/// 入口ごとに割れると、実機サインオフの grep が 2 通りの字面を追うことになる。
pub(super) fn reject_reason_text(reject: &ZOrderReject) -> String {
    match reject {
        ZOrderReject::ModeMixed => "ModeMixed".to_string(),
        ZOrderReject::DuplicateElement { element } => {
            format!("DuplicateElement({})", element_text(element))
        }
        ZOrderReject::TooFewElements { count } => format!("TooFewElements({count})"),
        ZOrderReject::UnparsableToken { token } => format!("UnparsableToken({token})"),
        ZOrderReject::CrossGroupRedesignation { scopes } => format!(
            "CrossGroupRedesignation({})",
            scopes
                .iter()
                .map(u32::to_string)
                .collect::<Vec<_>>()
                .join(",")
        ),
    }
}

/// 受け取ったトークン列を 1 欄へ（空列は番兵）。
///
/// **解釈前の字面をそのまま**並べる。作者が何を書いたのかが記録から復元できなければ、
/// 書き間違いを直せない（要件 8.1）。
fn tokens_text(tokens: &[String]) -> String {
    if tokens.is_empty() {
        return NO_VALUE.to_string();
    }
    tokens.join(",")
}

/// 要素列を手前から順に 1 欄へ（空列は番兵）。
fn members_text(members: &[GroupElement]) -> String {
    if members.is_empty() {
        return NO_VALUE.to_string();
    }
    members
        .iter()
        .map(element_text)
        .collect::<Vec<_>>()
        .join(",")
}

/// 正規化の記録を 1 欄へ（空列は番兵）。
///
/// 1 スコープぶんの字面は `scope:reordered`。相棒窓を補ったスコープ（畳み込み・
/// 要件 2.6）だけ、そこへ `+bN`／`+sN` を続けて**補った窓そのもの**を名指しする。
/// 要素列の欄（`members=`）と同じ省略記法を使うので、行の中で照らし合わせられる。
///
/// 2 窓そろって書かれたスコープの字面は畳み込みの導入前と 1 バイトも変わらない
/// ——既存の檻（`zorder_drain_tests.rs` の受理行・`zorder_descript_tests.rs` の
/// 起動行）が読み続けている語だからである（要件 9.5）。
fn normalizations_text(normalizations: &[Normalization]) -> String {
    if normalizations.is_empty() {
        return NO_VALUE.to_string();
    }
    normalizations
        .iter()
        .map(|n| {
            let mut field = format!("{}:{}", n.scope, n.reordered);
            if let Some(kind) = n.implied_partner {
                field.push('+');
                field.push_str(&element_text(&GroupElement {
                    scope: n.scope,
                    kind,
                }));
            }
            field
        })
        .collect::<Vec<_>>()
        .join(",")
}

#[cfg(test)]
#[path = "zorder_drain_test_support.rs"]
mod test_support;

#[cfg(test)]
#[path = "zorder_drain_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "zorder_drain_projection_tests.rs"]
mod projection_tests;

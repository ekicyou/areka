//! 重なりの指令を台帳へ適用し、台帳を**実在する窓の列**へ射影する相
//! （design「zorder drain 相（`emo2_boot/frame/zorder_drain.rs`）」・要件 1.4／6.1／
//! 7.1／7.2／8.4）。
//!
//! 兄弟の [`run_move_drain_phase`](super::drain_resnap::run_move_drain_phase) と同じ
//! 跨ぎの形である——台本のスレッドが送り出した指令を、画面を持つ側のスレッドで
//! 取り出して適用する。違うのは**適用した後にもう一仕事ある**ことで、こちらは台帳の
//! 内容を「いま実際に在る窓」の列へ写し、その写しを wintf の受け口へ置く。
//!
//! # 窓の正本が無い間は取り出さない
//!
//! `GhostWindows`（スコープ→窓 entity の唯一の正本）が World に居ない間は、指令を
//! 1 件も取り出さずに戻る。送信端と受信端をつなぐチャネルが**そのまま保留バッファを
//! 兼ねる**ので、取りこぼしは起きず、窓が生えた最初の相で到着順のまま一括で適用される
//! （move の相 `drain_resnap.rs:79-87` と同じ意図・要件 1.4）。取り出してから捨てる形に
//! すると、起動直後の `\![set,zorder,...]` が黙って消える。
//!
//! # 射影は「実在する窓だけ」を宣言順のまま抜き出す
//!
//! 台帳はスコープ番号と窓種別のままで持ち、まだ現れていないスコープも取り除かない
//! （要件 1.4）。窓が実在するかを知っているのはこの相だけなので、
//! 「宣言 → 実在する窓の列」の写像はここが持つ。写像の規則は 2 つだけである。
//!
//! - **実在しない要素は飛ばし、残る要素の相対順は宣言のまま保つ**（要件 1.4／7.2）。
//!   飛ばした要素があったグループは見送りの記録を残す（黙って落とさない・要件 8.4）。
//!   この記録は**受け口への書込とは独立**である——要件 8.4 が名指しする「窓が一度も
//!   現れないまま推移する」グループは射影が空になり書込が起きないので、書込に紐付けると
//!   肝心の場合が沈黙する（差し戻し 1 巡目の是正。詳細は [`report_missing_members`]）。
//! - **実在する窓が 2 枚未満のグループは射影から外す**（比べる相手が居らず維持のしようが
//!   無い）。ただし**台帳のエントリは残す**ので、窓が現れた後の相で射影へ戻ってくる
//!   （要件 7.1）。
//!
//! ここでの「実在する」は 2 段である——`GhostWindows` にそのスコープが載っていること
//! （まだ生まれていない窓は載らない）と、指している entity が World にまだ居ること
//! （破棄済みは飛ばす・要件 7.2）。前者だけを見ると、対の後追い破棄の途中で
//! 既に消えた entity を受け口へ渡してしまう。
//!
//! # 何も変わっていない巡では受け口に触れない（要件 6.1）
//!
//! 射影の結果が受け口の現在の内容と同じなら、書き込みも印立ても行わない。
//! グループが 1 つも無い状態では射影が空になり、受け口の Resource すら作らない
//! ——維持系は観測する対象を得られず、指令を 1 本も出さない。「既定状態では従来と
//! 同じ」（要件 6.1／6.4）はこの**不在**によって構造的に成り立つのであって、
//! 「出さないと判断する」ことによってではない。
//!
//! 変化の検出に台帳の版（[`ZOrderGroupLedger::version`]）ではなく**射影の結果そのもの**を
//! 使うのは、版が進んでも射影が動かない場合があるからである（例: まだ 1 枚も窓が無い
//! スコープだけのグループが受理された巡）。版で判定すると、その巡に印が立って維持系が
//! 空振りする。結果の突き合わせは版の判定を包含する（版が動かなければ結果も動かない）。
//!
//! # 記録はすべて wintf の唯一の入口を通す
//!
//! 受理・拒否・メンバー不在のいずれも、記録を出すのは wintf 側の
//! [`log_group_applied`]／[`log_group_rejected`]／[`log_group_member_missing`] である。
//! `tracing` の出力先は呼び出し元の module path が既定なので、こちら側でマクロを
//! 呼ぶと実機サインオフの grep 対象が 2 本に割れる（task 2.1 が入口を 1 つに閉じた
//! 理由そのもの）。本モジュールが組むのは**本文の文字列**だけである。

use std::collections::BTreeMap;
use std::sync::mpsc::Receiver;

use bevy_ecs::entity::Entity;
use bevy_ecs::prelude::Resource;
use bevy_ecs::world::World;

use wintf::ecs::window::{
    ZOrderGroupSpec, ZOrderGroups, log_group_applied, log_group_member_missing, log_group_rejected,
};

use crate::emo2_boot::zorder_cue::ZOrderDirective;
use crate::placement::spawn::GhostWindows;
use crate::placement::zorder_group_ledger::{
    GroupElement, GroupWindowKind, Normalization, ZOrderGroup, ZOrderGroupLedger, ZOrderReject,
    parse_zorder_tokens,
};

/// 値が無いことを表す番兵（既存のグループ系・ペア系の記録行と同じ字面）。
///
/// 欄ごと落とさないのは、「記録が出ていない」と「その経路にはその値が無い」の区別が
/// 事後に付かなくなるからである（`zorder_group_diag.rs` の `UNKNOWN` と同じ規律。
/// あちらは wintf 内部の可視性ゆえ、同じ字面をこちらでも定義する）。
const NO_VALUE: &str = "-";

// ---------------------------------------------------------------------------
// 相の本体
// ---------------------------------------------------------------------------

/// zorder drain 相（design「zorder drain 相」・要件 1.4／6.1／7.1／7.2／8.4）。
///
/// 順に⑴窓の正本が無ければ何もしない ⑵届いている指令を到着順に台帳へ適用する
/// ⑶台帳を実在する窓の列へ射影する ⑷射影が動いていれば受け口へ書いて印を立てる。
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

    // ⑶ 射影する。
    let projection = project_groups(ledger, world);
    // ⑷ 実在しない要素の報告は**受け口の書込とは独立**に行う（下の関数の doc を参照）。
    report_missing_members(world, &projection.incomplete);
    // ⑸ 射影が動いていれば受け口へ置く。
    publish_projection(world, projection.specs);
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
// ⑶ 射影（台帳 → 実在する窓の列）
// ---------------------------------------------------------------------------

/// 射影の結果——受け口へ置く列と、要素を飛ばしたグループの控え。
#[derive(Debug, Default, PartialEq, Eq)]
struct Projection {
    /// 維持の対象になるグループ（実在する窓が 2 枚以上・宣言順のまま）。
    specs: Vec<ZOrderGroupSpec>,
    /// 実在しない要素があったグループ（記録の対象・要件 8.4）。
    ///
    /// 射影から外れたグループも、射影に載ったが一部が欠けたグループも、どちらも
    /// ここへ入る。「窓が足りずに外した」ことと「窓が足りないまま残る要素で維持する」
    /// ことは、作者から見ればどちらも「書いたスコープの窓がまだ無い」1 つの事実である。
    incomplete: Vec<IncompleteGroup>,
}

/// 実在しない要素があったグループ 1 本の控え（記録に載せる実数を持つ）。
///
/// 欠けた数ではなく**宣言の数と実在の数**を持つ。`existing == 0`（一度も現れていない
/// ＝要件 8.4 が名指しする形）と `existing >= 2`（一部だけ現れて維持は続く形）は
/// 読み手にとって別の事実であり、引き算した 1 つの数からは復元できない。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct IncompleteGroup {
    /// 台帳が配ったグループの識別子。
    id: u32,
    /// 作者が書いた要素の数（台帳に載っている宣言の長さ）。
    declared: usize,
    /// そのうち実在する窓へ解決できた数。
    existing: usize,
}

/// 台帳の全グループを、実在する窓だけの列へ射影する（要件 1.4／7.1／7.2）。
///
/// `GhostWindows` が無ければ空の射影を返す（呼び出し元が先に弾いているので、
/// これは二重の防波堤である）。
fn project_groups(ledger: &ZOrderGroupLedger, world: &World) -> Projection {
    let Some(ghost_windows) = world.get_resource::<GhostWindows>() else {
        return Projection::default();
    };

    let mut projection = Projection::default();
    for group in ledger.groups() {
        let mut members: Vec<Entity> = Vec::with_capacity(group.members.len());
        for element in &group.members {
            // 飛ばすのは要素だけで、残る要素は**宣言の順のまま**積む（要件 1.4）。
            // 詰めるだけなので、存在する窓どうしの相対順は作者の指定と一致する。
            if let Some(entity) = resolve_member(ghost_windows, world, element) {
                members.push(entity);
            }
        }
        if members.len() < group.members.len() {
            projection.incomplete.push(IncompleteGroup {
                id: group.id,
                declared: group.members.len(),
                existing: members.len(),
            });
        }
        // 実在が 2 枚未満＝比べる相手が居ない。射影から外すが台帳のエントリは残るので、
        // 窓が現れた後の巡で戻ってくる（要件 7.1）。
        if members.len() >= 2 {
            projection.specs.push(ZOrderGroupSpec {
                id: group.id,
                members,
            });
        }
    }
    projection
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
// ⑷ 実在しない要素の報告（受け口の書込とは独立）
// ---------------------------------------------------------------------------

/// 直近に報告した「不完全なグループ」の控え（グループ id → 実在した窓の数）。
///
/// **正本ではない**。グループの内容（要素・順序・出所）は 1 バイトも持たず、
/// 「どの id について、実在何枚という事実を既に報せたか」だけを覚える。二重帳簿を
/// 作らないための線引きであり、この Resource を失っても復元されるのは
/// 「もう一度同じ行が出る」ことだけで、重なりの判断には何の影響も無い。
#[derive(Resource, Default)]
struct ZOrderMissingReports {
    /// グループ id → 前回報告時に実在した窓の数。
    seen: BTreeMap<u32, usize>,
}

/// 実在しない要素があったグループを記録する（要件 8.4／8.3）。
///
/// # なぜ受け口の書込と切り離すのか（差し戻し 1 巡目の是正）
///
/// 当初はこの報告を [`publish_projection`] の末尾に置いていたが、あちらには
/// 「受け口を作る理由が無い」「射影が動いていない」の 2 つの早期 return がある。
/// **要件 8.4 が名指しする「窓が一度も現れないまま推移する」グループは射影が空**
/// なので 1 つ目の return に落ち、報告そのものが消えていた。既に安定した射影が在る
/// ところへ全欠けのグループを足した場合も、射影が動かないので 2 つ目に落ちた。
/// 不在は**書込とは別の事実**であり、書込の有無に紐付けてはならない。
///
/// # 連呼はしない（毎巡走る相であることへの配慮）
///
/// 素直に毎回出すと、現れないスコープを 1 つ書いただけで同じ 1 行が毎フレーム積もり、
/// 本物の変化を埋める。よって**前回報告した内容と違うときだけ**出す——初めて不完全に
/// なった id と、実在の枚数が動いた id である。完全になった id は控えから落ちるので、
/// 再び欠ければまた報される。「一度きり」にしないのは、欠けが増減した事実まで
/// 黙らせないためである。
///
/// 控えは報告の有無を決めるためだけに使い、判断には一切関与しない。
fn report_missing_members(world: &mut World, incomplete: &[IncompleteGroup]) {
    // 報せるものが無く、控えもまだ無い＝既定状態。Resource を作らずに戻る
    // （グループが 1 つも無い間は何も生やさない・要件 6.1 と同じ姿勢）。
    if incomplete.is_empty() && world.get_resource::<ZOrderMissingReports>().is_none() {
        return;
    }

    world.init_resource::<ZOrderMissingReports>();
    let mut reports = world.resource_mut::<ZOrderMissingReports>();
    // 出す順は台帳の並び順（決定論・要件 10.3）。
    let fresh: Vec<IncompleteGroup> = incomplete
        .iter()
        .copied()
        .filter(|group| reports.seen.get(&group.id) != Some(&group.existing))
        .collect();
    reports.seen = incomplete
        .iter()
        .map(|group| (group.id, group.existing))
        .collect();
    drop(reports);

    for group in fresh {
        log_group_member_missing(group.id, group.declared, group.existing);
    }
}

// ---------------------------------------------------------------------------
// ⑸ 受け口への書込
// ---------------------------------------------------------------------------

/// 射影が動いていれば受け口へ書き、印を立てる（要件 6.1／7.1）。
///
/// # 何も変わっていない巡では触れない
///
/// 受け口の現在の内容と射影が一致するなら、書きもせず印も立てない。印は「是正が要る
/// かもしれない」の合図であり、何も動いていない巡に立てると維持系が毎巡空振りして
/// 表示に変化の無い巡を省く門を実質無効にする。
///
/// # 空の射影では受け口そのものを作らない
///
/// グループが 1 つも無い（＝既定状態）とき、受け口の Resource は挿入しない。維持系は
/// 観測する対象を得られず、指令を 1 本も出さない——要件 6.1／6.4 の「既定状態は従来と
/// 同じ」は、判断ではなくこの**不在**で成り立つ。
///
/// # 印は立てるだけで倒さない
///
/// 射影が空になった巡（全グループの窓が消えた等）でも印は倒さない。倒す条件
/// （維持対象が全て成立した）は維持系の持ち物であり、こちらが横から倒すと
/// 「他の追随トリガが立てた印」を消してしまう。空の射影に対して維持系は何も
/// 観測しないので、印が立ったままでも指令は 1 本も出ない。
fn publish_projection(world: &mut World, specs: Vec<ZOrderGroupSpec>) {
    if world.get_resource::<ZOrderGroups>().is_none() {
        if specs.is_empty() {
            // 並べるものが無い＝受け口を作る理由が無い（要件 6.1 の構造的な根拠）。
            return;
        }
        world.init_resource::<ZOrderGroups>();
    }

    let mut groups = world.resource_mut::<ZOrderGroups>();
    if groups.groups == specs {
        // 何も動いていない＝書込も印立ても行わない（空振りの巡を作らない）。
        return;
    }
    let projected = !specs.is_empty();
    groups.groups = specs;
    if projected {
        groups.pending = true;
    }
}

// ---------------------------------------------------------------------------
// 記録の本文（純関数・`tracing` を含まない）
// ---------------------------------------------------------------------------

/// 受理（`\![set,zorder,...]`）の本文——台帳に載った内容と、正規化で調整した内容。
///
/// `normalized` に載るのは同一スコープの 2 窓を隣接ブロックへ寄せた記録である
/// （要件 2.4）。`scope:true` は「作者が書いた順をそのままの形では採用しなかった」を
/// 意味する。数値モードでは調整そのものが起きないので常に番兵になる。
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

/// 要素 1 つを省略記法（`bN`／`sN`）の字面へ。
fn element_text(element: &GroupElement) -> String {
    let prefix = match element.kind {
        GroupWindowKind::Balloon => 'b',
        GroupWindowKind::Char => 's',
    };
    format!("{prefix}{}", element.scope)
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
fn normalizations_text(normalizations: &[Normalization]) -> String {
    if normalizations.is_empty() {
        return NO_VALUE.to_string();
    }
    normalizations
        .iter()
        .map(|n| format!("{}:{}", n.scope, n.reordered))
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

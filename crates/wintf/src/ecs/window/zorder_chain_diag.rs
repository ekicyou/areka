//! 鎖系の診断記録——**行を組む純関数とタグ定数の唯一の所在**。
//!
//! 実機サインオフの grep 判定語と出力書式は 1:1 で対応するため、書式が意図せず変われば
//! 手順が静かに嘘になる——組立を純関数へ切り出してテストで語彙を固定し、記録を出す側は
//! その戻り値をそのまま本文にする（組立を二重に持たない）。
//! 既存ペア機構の [`zorder_pair_diag`](super::zorder_pair_diag) と同じ分割である。
//!
//! # 保全語彙 2 語
//!
//! `[zorder-group] applied`／`[zorder-group] rejected` は、退役した `zorder_group` 系から
//! **字面を 1 字も変えずに**ここへ移した。呼び出し元（shell 設定の適用・指令消化の相）は
//! `wintf::ecs::window` の再輸出を通して同じ名前で呼ぶ。
//!
//! この 2 語だけは行の組立に加えて**記録そのもの**（`tracing` のマクロ呼出）もここに置く。
//! 出力先は module path 既定＝`wintf::ecs::window::zorder_chain_diag` となり、サインオフの
//! `RUST_LOG` 指定 `wintf::ecs::window::zorder_chain=debug` は**前方一致でこれを点灯させる**
//! （下の兄弟テストが実際に捕捉して固定している）。
//!
//! 鎖系 7 語と組の受け渡し 3 語の記録そのもの（マクロ呼出）は `zorder_chain` 側に集約する。
//! ここに在るのは行の組立だけである。
//!
//! # フィールドは落とさず番兵で埋める
//!
//! 値が取れなかったときに欄ごと落とすと、「記録が出ていない」のと「その経路にはその値が
//! 無い」の区別が事後に付かなくなる。よってすべての欄は必ず `field=value` の形で現れ、
//! 値が無いときは [`UNKNOWN`] になる（既存ペア機構と同じ字面）。
//!
//! # 組の受け渡しの 3 語
//!
//! `pair-released`／`pair-restored`／`link-refused` は、逆転・非隣接の組の結び目を鎖のために
//! 外す・結び直す・輪になる付与を見送る記録である。名簿は既存の 7 語の一覧と分けて持つ
//! （`pair_handover_tags`・試験用）。

use bevy_ecs::prelude::*;
use tracing::{debug, warn};
use windows::Win32::Foundation::HWND;

use super::zorder_pair_diag::{UNKNOWN, hwnd_field, tristate_field};

/// 繋いだ事実の記録タグ（debug 水準）。
const LINKED_TAG: &str = "[zorder-chain] linked";
/// 外した事実の記録タグ（debug 水準）。
const UNLINKED_TAG: &str = "[zorder-chain] unlinked";
/// 収まった事実の記録タグ（debug 水準・宣言と直後の実測を同一行に載せる）。
const SETTLED_TAG: &str = "[zorder-chain] settled";
/// 宣言された要素の窓が不在だった事実の記録タグ（debug 水準）。
const ABSENT_TAG: &str = "[zorder-chain] absent";
/// 見送りの記録タグ（理由必須・debug 水準）。
const SKIPPED_TAG: &str = "[zorder-chain] skipped";
/// 張り失敗の記録タグ（error 水準）。
const LINK_FAILED_TAG: &str = "[zorder-chain] link-failed";
/// 外し失敗の記録タグ（error 水準）。
const UNLINK_FAILED_TAG: &str = "[zorder-chain] unlink-failed";
/// 組の結び目を鎖のために外した事実の記録タグ（debug 水準）。
const PAIR_RELEASED_TAG: &str = "[zorder-chain] pair-released";
/// 外していた組の結び目を張り直した事実の記録タグ（debug 水準）。
const PAIR_RESTORED_TAG: &str = "[zorder-chain] pair-restored";
/// 外し損ねの巡に、持ち主の輪を作る付与を見送った事実の記録タグ（error 水準）。
const LINK_REFUSED_TAG: &str = "[zorder-chain] link-refused";

/// 受理の記録タグ（**退役した `zorder_group_diag` からの移設・字面は 1 字も変えない**）。
const APPLIED_TAG: &str = "[zorder-group] applied";
/// 拒否の記録タグ（**退役した `zorder_group_diag` からの移設・字面は 1 字も変えない**）。
const REJECTED_TAG: &str = "[zorder-group] rejected";

/// 鎖系の記録タグ 7 種（サインオフの grep 判定語の一覧）。
#[cfg(test)]
pub(crate) fn chain_record_tags() -> [&'static str; 7] {
    [
        LINKED_TAG,
        UNLINKED_TAG,
        SETTLED_TAG,
        ABSENT_TAG,
        SKIPPED_TAG,
        LINK_FAILED_TAG,
        UNLINK_FAILED_TAG,
    ]
}

/// 組の受け渡しの記録タグ 3 種（逆転・非隣接の組の外しと結び直し・輪の見送り）。
///
/// 既存の 7 語の一覧 [`chain_record_tags`] は、隣接の spec が字面を固定したテストが読む名簿
/// なので変えない（実機の手順は字面を自前で持つ）。新しい語はこの別の一覧に置く。
#[cfg(test)]
pub(crate) fn pair_handover_tags() -> [&'static str; 3] {
    [PAIR_RELEASED_TAG, PAIR_RESTORED_TAG, LINK_REFUSED_TAG]
}

/// 移設してきた保全語彙 2 種の一覧。
#[cfg(test)]
pub(crate) fn preserved_group_tags() -> [&'static str; 2] {
    [APPLIED_TAG, REJECTED_TAG]
}

/// その繋ぎが属する鎖の区間。
///
/// # なぜ crate の外へ開いているのか
///
/// 望む鎖（[`ChainPlan`](super::zorder_chain::ChainPlan)）は**areka が組む**——グループの
/// 登記順もスコープ ID も areka 側の知識だからである。よって区間の値も areka が詰められ
/// なければならず、`pub(crate)` では届かない。開いているのは**語彙**（グループの通し番号か
/// 後方配置か）だけであり、判断も記録もこちら側に閉じたままである。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ChainSegment {
    /// 台帳のグループ（登記順の通し番号）。
    Group(u32),
    /// どのグループにも属さないスコープの後方配置。
    Tail,
}

/// 繋ぎを外す理由。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum DetachReason {
    /// グループが解除された／このグループから外れた。
    Teardown,
    /// 同じ窓の owner が別の窓へ変わる（スプライス）。
    Rechain,
    /// 窓が去る（破棄より先に外す）。
    Departing,
    /// 帳簿と OS の現況が食い違う。撤去は行わず帳簿だけ落とす。
    Diverged,
}

/// 鎖の適用を見送った理由（理由の無い見送りを作れないようにする）。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum ChainSkipReason {
    /// 実在する窓が 2 枚未満で、張るべき繋ぎが 1 本も無い。
    TooFewPresent,
    /// 望む鎖が前回と同じで、出す操作が 1 つも無い。
    NoChange,
    /// 窓ハンドルがまだ取れていない。
    HandleMissing,
}

/// 付与を見送った理由（`link-refused` の `reason=` 欄）。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum LinkRefusedReason {
    /// 張ると持ち主の輪になる（辿りの上限を超えた場合も含む）。
    Loop,
    /// 持ち主を辿る途中で読めなかった。
    Unreadable,
}

impl std::fmt::Display for ChainSegment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChainSegment::Group(id) => write!(f, "g{id}"),
            ChainSegment::Tail => write!(f, "tail"),
        }
    }
}

/// 区間をログ用の 1 フィールドへ（不明は番兵）。
fn segment_field(segment: Option<ChainSegment>) -> String {
    match segment {
        Some(s) => s.to_string(),
        None => UNKNOWN.to_string(),
    }
}

/// 窓の列をログ用の 1 フィールドへ（空列は番兵）。
fn hwnd_list_field(hwnds: &[HWND]) -> String {
    if hwnds.is_empty() {
        return UNKNOWN.to_string();
    }
    hwnds
        .iter()
        .map(|h| hwnd_field(Some(*h)))
        .collect::<Vec<_>>()
        .join(",")
}

/// 呼び出し側から受け取った自由文を 1 フィールドへ畳む（空は番兵）。
fn text_field(value: &str) -> String {
    let folded = value.split_whitespace().collect::<Vec<_>>().join("_");
    if folded.is_empty() {
        return UNKNOWN.to_string();
    }
    folded
}

/// 繋いだ行（純関数・debug 水準で出す）。
///
/// 記録には「どのグループのどの窓を、どの窓のすぐ手前に位置づけたか」が要る。
/// その 3 つ——区間（`segment`）・被所有側（`owned`）・所有側（`owner`）——を同じ 1 行に
/// 載せ、あわせて鎖の何番目かを `pos=i/n` で示す。窓ハンドルを Entity と併記するのは、
/// 実機のログを Win32 の実測（`Spy++` 等）と突き合わせられるようにするためである。
///
/// `segment` はこの繋ぎが属する区間である——台帳のグループ（`gN`・登記順）か、
/// どのグループにも属さないスコープの後方配置（`tail`）か。これが無いと、
/// 全窓 1 本の鎖では「どのグループの繋ぎか」が記録から復元できない。
///
/// # `Option` は防御であって、本番は常に実値である
///
/// 区間は望む鎖が運んでくる——[`CrossEdge::segment`](super::zorder_chain::CrossEdge) に
/// 載って届く。付与は望む鎖に在る繋ぎからしか作られないので、適用系（`execute_ops`）が
/// 望む鎖から区間を引く検索は必ず当たる。
/// よって**本番の呼び出しは常に `Some(..)`** であり、`gN` か `tail` のどちらかが必ず入る。
///
/// 引数が `Option` なのは欄そのものを落とさないための防御で、取れないときは `-` を入れる。
pub(crate) fn linked_line(
    segment: Option<ChainSegment>,
    owned: Entity,
    owner: Entity,
    owned_hwnd: Option<HWND>,
    owner_hwnd: Option<HWND>,
    pos: usize,
    total: usize,
) -> String {
    format!(
        "{LINKED_TAG} segment={segment} owned={owned:?} owner={owner:?} \
         owned_hwnd={owned_h} owner_hwnd={owner_h} pos={pos}/{total}",
        segment = segment_field(segment),
        owned_h = hwnd_field(owned_hwnd),
        owner_h = hwnd_field(owner_hwnd),
    )
}

/// 外した行（純関数・debug 水準で出す）。
///
/// 撤去には所有側の Entity を載せない——撤去の時点で相手の Entity は既に消えている
/// ことがある（破棄より先に外す経路）。代わりに帳簿が控えている
/// **張った時点の owner の窓ハンドル**を載せる。これは撤去前の照合に使った値そのもの
/// であり、`Diverged`（帳簿と OS の現況が食い違う）の行を読むときに要る。
///
/// 区間は帳簿の控え（[`CrossOwnerLink::segment`](super::zorder_chain::CrossOwnerLink)）から
/// 出るので、本番の呼び出し（`detach_one`）は常に `Some(..)` である。`Option` は欄を落とさない
/// ための防御で、取れないときは番兵 `-` を入れる。
pub(crate) fn unlinked_line(
    segment: Option<ChainSegment>,
    owned: Entity,
    owned_hwnd: Option<HWND>,
    owner_hwnd: Option<HWND>,
    reason: DetachReason,
) -> String {
    format!(
        "{UNLINKED_TAG} segment={segment} owned={owned:?} \
         owned_hwnd={owned_h} owner_hwnd={owner_h} reason={reason:?}",
        segment = segment_field(segment),
        owned_h = hwnd_field(owned_hwnd),
        owner_h = hwnd_field(owner_hwnd),
    )
}

/// 収まった行（純関数・debug 水準で出す）。
///
/// **組み替えの宣言と、その直後に実測した重なりを同じ 1 行に載せる**。分けると
/// 「指令は出したが効かなかった」の判定が 2 行の突合になり、過去に同型の誤診を生んでいる
/// （既存ペア機構 [`zorder_pair_diag::fix_line`](super::zorder_pair_diag::fix_line) と同じ規律）。
///
/// - `nudged_hwnd`: 後押しで差し直した窓（鎖の根）
/// - `insert_after`: その挿入位置（錨＝根の 1 つ手前。錨の直後が既に根なら先頭）
///   ——参照するのは自分のゴースト窓 2 枚だけ
/// - `declared`: 宣言した鎖の並び（手前から奥）
/// - `measured`: 後押しの直後に前面走査が**実際に出会った**並び（不可視の窓は読み飛ばす）
/// - `nudge_ok`: 後押しそのものの成否（後押しの失敗は記録して続行する。この欄が無いと
///   失敗が黙って消える）
///
/// # 欄の並びは動かせない
///
/// 実機サインオフの切り出しは前 4 欄が**この順で隣り合っている**ことを前提にしている。
/// 欄を足すときは必ず 4 欄の**後ろ**へ足すこと（`nudge_ok` がそうしてある）。兄弟テストが
/// 隣接を字面のまま固定しているので、間へ割り込ませた瞬間に赤くなる。
pub(crate) fn settled_line(
    nudged_hwnd: Option<HWND>,
    insert_after: Option<HWND>,
    declared: &[HWND],
    measured: &[HWND],
    nudge_ok: Option<bool>,
) -> String {
    format!(
        "{SETTLED_TAG} nudged_hwnd={nudged} insert_after={after} \
         declared={declared} measured={measured} nudge_ok={ok}",
        nudged = hwnd_field(nudged_hwnd),
        after = hwnd_field(insert_after),
        declared = hwnd_list_field(declared),
        measured = hwnd_list_field(measured),
        ok = tristate_field(nudge_ok),
    )
}

/// 不在の行（純関数・debug 水準で出す）。
///
/// 宣言された要素のうち窓が実在しなかったものを記録する。射影は「実在する窓だけ」を
/// 抜き出すので、まだ生まれていない窓・破棄済みの窓はここで報せなければ、記録の上では
/// **最初から書かれていなかった**のと区別が付かない（黙って諦めることの一形態である）。
///
/// `element` の欄は要素の正準表記（種別の字＋スコープ＝`b0`／`s1`）である。
/// 字と数で受け取るので空白も空も起きず、文字列はこの行を組むときだけ作る。
pub(crate) fn absent_line(group_id: u32, kind: char, scope: u32) -> String {
    format!("{ABSENT_TAG} group_id={group_id} element={kind}{scope}")
}

/// 見送りの行（純関数・debug 水準で出す・理由必須）。
///
/// 理由語は [`ChainSkipReason`] の `Debug` 表現をそのまま用いる。3 種が互いに異なる語で
/// あることがそのまま「理由を伴う見送り」の実質になる——1 語へ潰れると記録があっても
/// 理由が読めない。語そのものは兄弟テストが逐語で固定している。
pub(crate) fn skipped_line(reason: ChainSkipReason) -> String {
    format!("{SKIPPED_TAG} reason={reason:?}")
}

/// 張り失敗の行（純関数・error 水準で出す）。
///
/// 失敗した繋ぎ **1 本だけ**を飛ばして残りは張る方針である。
/// よってこの行は「どの区間のどの対が張れなかったか」を単独で読めなければならない。
///
/// 区間は呼び手で違う。鎖の付与の失敗は望む鎖の区間（`Some(..)`）を載せ、組の結び目の
/// 張り直し（`restore_pair`）の失敗は区間を持たないので `None` を渡し、番兵 `-` が入る
/// （`segment=-`）。欄を落とさないのは [`linked_line`] と同じ理由である。
pub(crate) fn link_failed_line(
    segment: Option<ChainSegment>,
    owned_hwnd: Option<HWND>,
    owner_hwnd: Option<HWND>,
    error: &windows::core::Error,
) -> String {
    format!(
        "{LINK_FAILED_TAG} segment={segment} owned_hwnd={owned_h} owner_hwnd={owner_h} \
         error={code:?}",
        segment = segment_field(segment),
        owned_h = hwnd_field(owned_hwnd),
        owner_h = hwnd_field(owner_hwnd),
        code = error.code(),
    )
}

/// 外し失敗の行（純関数・error 水準で出す）。
///
/// 区間を載せないのは、この行を鎖の撤去（`detach_one`・区間は帳簿にある）と組の結び目の
/// 外し（`release_pair`・区間を持たない）の両方が出すためで、両方で同じ形にしてある。
/// 所有側の窓ハンドルも載せない——`clear_window_owner` は被所有側だけを引数に取るので、
/// 失敗の主語はそちらである。
pub(crate) fn unlink_failed_line(owned_hwnd: Option<HWND>, error: &windows::core::Error) -> String {
    format!(
        "{UNLINK_FAILED_TAG} owned_hwnd={owned_h} error={code:?}",
        owned_h = hwnd_field(owned_hwnd),
        code = error.code(),
    )
}

/// 組の結び目を外した行（純関数・debug 水準で出す）。
///
/// `entity` はバルーン（`KeepDirectlyAbove` を持つ窓）、`peer` はそのキャラクター。
/// `unlinked=false` は、結び目が張られていなかった・今の持ち主が帳簿と食い違った・
/// 読めなかったために OS の関係へは触れず、印だけを付けたことを表す。
pub(crate) fn pair_released_line(
    entity: Entity,
    peer: Entity,
    owned_hwnd: Option<HWND>,
    owner_hwnd: Option<HWND>,
    unlinked: bool,
) -> String {
    format!(
        "{PAIR_RELEASED_TAG} entity={entity:?} peer={peer:?} \
         owned_hwnd={owned_h} owner_hwnd={owner_h} unlinked={unlinked}",
        owned_h = hwnd_field(owned_hwnd),
        owner_h = hwnd_field(owner_hwnd),
    )
}

/// 外していた組の結び目を張り直した行（純関数・debug 水準で出す）。
pub(crate) fn pair_restored_line(
    entity: Entity,
    peer: Entity,
    owned_hwnd: Option<HWND>,
    owner_hwnd: Option<HWND>,
) -> String {
    format!(
        "{PAIR_RESTORED_TAG} entity={entity:?} peer={peer:?} \
         owned_hwnd={owned_h} owner_hwnd={owner_h}",
        owned_h = hwnd_field(owned_hwnd),
        owner_h = hwnd_field(owner_hwnd),
    )
}

/// 持ち主の輪を作る付与を見送った行（純関数・error 水準で出す）。
///
/// 見送った付与 1 本ごとに 1 行。残りの付与は張るので、この行は「どの対を張らなかったか」を
/// 単独で読めなければならない。
pub(crate) fn link_refused_line(
    owned: Entity,
    owner: Entity,
    owned_hwnd: Option<HWND>,
    owner_hwnd: Option<HWND>,
    reason: LinkRefusedReason,
) -> String {
    format!(
        "{LINK_REFUSED_TAG} owned={owned:?} owner={owner:?} \
         owned_hwnd={owned_h} owner_hwnd={owner_h} reason={reason:?}",
        owned_h = hwnd_field(owned_hwnd),
        owner_h = hwnd_field(owner_hwnd),
    )
}

/// 受理の記録行（純関数）——台帳が組んだ本文へ、こちらはタグだけを貼る。
///
/// **退役した `zorder_group_diag` からの移設であり、字面は 1 字も変えていない**。
///
/// 台帳の内容そのものは areka の型であり、`wintf → areka` の import は禁止ゆえここでは
/// 受け取れない。よって組み上がった本文を受け取り、タグと（呼び出し側の module path 既定
/// による）出力先だけをこちらが与える。
pub(crate) fn applied_line(detail: &str) -> String {
    format!("{APPLIED_TAG} {detail}")
}

/// 拒否の記録行（純関数・warn 水準で出す）。
///
/// **退役した `zorder_group_diag` からの移設であり、字面は 1 字も変えていない**。
///
/// 載るのは**拒否理由**と**受け取ったトークン列**の 2 欄である。トークン列を載せるのは、
/// 作者が何を書いたのかが記録から復元できなければ書き間違いを直せないからであり、
/// 理由を載せるのは「黙って無視された」を防ぐためである。
pub(crate) fn rejected_line(reason: &str, tokens: &str) -> String {
    format!(
        "{REJECTED_TAG} reason={reason} tokens={tokens}",
        reason = text_field(reason),
        tokens = text_field(tokens),
    )
}

/// 指定が受理された事実を記録する（台帳を持つ層＝areka から呼ぶ）。
///
/// 水準は **debug**——受理そのものは診断専用であり、既定運転では無音でよい。
pub fn log_group_applied(detail: &str) {
    debug!("{}", applied_line(detail));
}

/// 指定を拒否した事実を記録する（台帳を持つ層＝areka から呼ぶ）。
///
/// 水準は **warn**——`logging.md` の「無効なパラメーター」区分であり、作者の書き間違いは
/// 診断手順を有効化していない通常運転でも読めなければ「黙って無視された」に等しい。
pub fn log_group_rejected(reason: &str, tokens: &str) {
    warn!("{}", rejected_line(reason, tokens));
}

#[cfg(test)]
#[path = "zorder_chain_diag_tests.rs"]
mod zorder_chain_diag_tests;

//! 診断記録の**行を組む純関数**だけを集めた層（要件 6）。
//!
//! design.md「診断ログ語彙（要件 6）」のレコード表が正本であり、本モジュールはその表を
//! そのまま写した書式を 1 箇所に閉じ込める。サインオフの grep 判定語と出力書式は 1:1 で
//! 対応するため、書式が意図せず変われば手順が静かに嘘になる——組立を純関数へ切り出して
//! テストで語彙を固定し、記録を出す側はその戻り値をそのまま本文にする（組立を二重に持たない）。
//!
//! # ここに置いてよいもの・置いてはならないもの
//!
//! 置いてよいのは `tracing` のマクロを 1 つも含まない**純粋な文字列組立**だけである。
//! `log_*`／`record_*`（マクロ呼出を含む）は兄弟の [`zorder_pair`](super::zorder_pair) に
//! 残す——`tracing` の出力先は呼び出し元の module path が既定であり、こちらへ移すと
//! サインオフの grep 対象（`wintf::ecs::window::zorder_pair`）が分裂する。
//!
//! 本モジュールが独立しているのは兄弟が 1,000 行の上限に迫ったためであり、責務の境界は
//! 「マクロを含むか否か」の一線に置いている。

use bevy_ecs::prelude::*;
use windows::Win32::Foundation::HWND;

use super::zorder_pair::{ExpectedOrder, InsertSpec, PeerLoss, SkipReason};

/// 値が取得できなかったフィールドの番兵。
///
/// フィールドごと落とさないのは、落とすと「記録が出ていない」のと「その経路には
/// その値が無い」の区別が事後に付かなくなるためである。
const UNKNOWN: &str = "-";

/// owner 確立の記録タグ（grep 判定語）。
const OWNER_ESTABLISHED_TAG: &str = "[zorder-pair] owner-established";
/// 是正の記録タグ（指令と実測を同一行に載せる・要件 6.1）。
const FIX_TAG: &str = "[zorder-pair] fix";
/// 見送りの記録タグ（理由必須・要件 6.3）。
const SKIP_TAG: &str = "[zorder-pair] skip";
/// 検証不一致の記録タグ（error 水準・要件 6.2）。
const VERIFY_FAILED_TAG: &str = "[zorder-pair] verify-failed";
/// owner 確立失敗の記録タグ（error 水準・要件 6.2）。
const OWNER_ESTABLISH_FAILED_TAG: &str = "[zorder-pair] owner-establish-failed";
/// 沈降観測の記録タグ（要件 4.4／7.5）。
const SINK_OBSERVED_TAG: &str = "[zorder-pair] sink-observed";

/// HWND をログ用の 16 進表現へ（不在は番兵）。
///
/// `Debug` 表現をそのまま使わないのは、値の中に空白や記号が混じると
/// `field=value` の 1 行から機械的に切り出せなくなるためである。
pub(crate) fn hwnd_field(hwnd: Option<HWND>) -> String {
    match hwnd {
        Some(h) => format!("0x{:X}", h.0 as usize),
        None => UNKNOWN.to_string(),
    }
}

/// 真偽値をログ用表現へ（判定そのものが取れなかった場合は番兵）。
///
/// 「取れなかった」を `false` へ潰さない——潰すと「沈まなかった」という**異常**と
/// 「測れなかった」という**観測の欠落**が同じ字面になる。
fn tristate_field(value: Option<bool>) -> String {
    match value {
        Some(v) => v.to_string(),
        None => UNKNOWN.to_string(),
    }
}

impl SkipReason {
    /// 記録に載る理由語（サインオフの grep 判定語）。
    ///
    /// 5 種が互いに異なる語であることがそのまま要件 6.3 の「理由を伴う見送り」の
    /// 実質になる——1 語へ潰れると記録があっても理由が読めない。
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            SkipReason::AlreadyAdjacent => "AlreadyAdjacent",
            SkipReason::PeerMissing => "PeerMissing",
            SkipReason::HandleMissing => "HandleMissing",
            SkipReason::EchoOrIrrelevant => "EchoOrIrrelevant",
            SkipReason::StrategyDisabled => "StrategyDisabled",
        }
    }
}

impl InsertSpec {
    /// 挿入位置のログ用表現（縁は専用の語で、ハンドル表現と混ざらない）。
    fn as_field(&self) -> String {
        match self {
            InsertSpec::After(hwnd) => hwnd_field(Some(*hwnd)),
            InsertSpec::TopEdge => "top-edge".to_string(),
        }
    }
}

impl PeerLoss {
    /// 記録に載る語。
    fn as_str(&self) -> &'static str {
        match self {
            PeerLoss::Despawned => "despawned",
            PeerLoss::HandleRemoved => "handle-removed",
        }
    }
}

/// owner 確立の記録行（純関数）。
///
/// `measured_prev` は確立直後に採った「owner（キャラ窓）の最も近い可視の手前」の実測で
/// あり、ここに被 owner（バルーン窓）が現れることが案 A の中核保証（ゲート G6）の証跡になる。
pub(crate) fn owner_established_line(
    entity: Entity,
    peer: Entity,
    owned_hwnd: HWND,
    owner_hwnd: HWND,
    measured_prev: Option<HWND>,
) -> String {
    format!(
        "{OWNER_ESTABLISHED_TAG} entity={entity:?} peer={peer:?} owned_hwnd={owned} \
         owner_hwnd={owner} measured_prev={measured}",
        owned = hwnd_field(Some(owned_hwnd)),
        owner = hwnd_field(Some(owner_hwnd)),
        measured = hwnd_field(measured_prev),
    )
}

/// 是正の記録行（純関数）——**出した指令と、その後の実測を同じ 1 行に載せる**。
///
/// 行を「指令」と「実測」に分けないのは design.md「Implementation Notes > Validation」の
/// 裁定である。分けると「指令は出したが効かなかった」の判定が 2 行の突合になり、
/// 過去に同型の誤診を生んでいる。
pub(crate) fn fix_line(
    entity: Entity,
    peer: Entity,
    insert_after: InsertSpec,
    measured_next_after_fix: Option<HWND>,
) -> String {
    format!(
        "{FIX_TAG} entity={entity:?} peer={peer:?} insert_after={insert_after} \
         measured_next_after_fix={measured}",
        insert_after = insert_after.as_field(),
        measured = hwnd_field(measured_next_after_fix),
    )
}

/// 見送りの記録行（純関数・理由必須）。
pub(crate) fn skip_line(entity: Entity, peer: Entity, reason: SkipReason) -> String {
    format!(
        "{SKIP_TAG} entity={entity:?} peer={peer:?} reason={reason}",
        reason = reason.as_str(),
    )
}

/// 検証不一致の記録行（純関数）——期待した隣接と実測を同じ行へ。
pub(crate) fn verify_failed_line(
    entity: Entity,
    peer: Entity,
    expected: ExpectedOrder,
    measured: Option<HWND>,
) -> String {
    format!(
        "{VERIFY_FAILED_TAG} entity={entity:?} peer={peer:?} expected_above={above} \
         expected_below={below} measured={measured}",
        above = hwnd_field(Some(expected.above)),
        below = hwnd_field(Some(expected.below)),
        measured = hwnd_field(measured),
    )
}

/// owner 切離しの記録行（純関数）。
pub(crate) fn owner_detached_line(
    entity: Entity,
    peer: Entity,
    owned_hwnd: HWND,
    owner_hwnd: HWND,
    loss: PeerLoss,
) -> String {
    format!(
        "ペアの相手が消えたため owner を切り離しました entity={entity:?} peer={peer:?} \
         owned_hwnd={owned} owner_hwnd={owner} peer_state={state}",
        owned = hwnd_field(Some(owned_hwnd)),
        owner = hwnd_field(Some(owner_hwnd)),
        state = loss.as_str(),
    )
}

/// owner 切離し失敗の記録行（純関数）。
pub(crate) fn owner_detach_failed_line(
    entity: Entity,
    peer: Entity,
    owned_hwnd: HWND,
    owner_hwnd: HWND,
    error: &windows::core::Error,
) -> String {
    format!(
        "ペアの相手が消えましたが owner の切離しに失敗しました（処理は継続します） \
         entity={entity:?} peer={peer:?} owned_hwnd={owned} owner_hwnd={owner} \
         error={code:?} message={message}",
        owned = hwnd_field(Some(owned_hwnd)),
        owner = hwnd_field(Some(owner_hwnd)),
        code = error.code(),
        message = error.message(),
    )
}

/// owner 確立失敗の記録行（純関数）。
pub(crate) fn owner_establish_failed_line(entity: Entity, error: &windows::core::Error) -> String {
    format!(
        "{OWNER_ESTABLISH_FAILED_TAG} entity={entity:?} error={code:?} message={message}",
        code = error.code(),
        message = error.message(),
    )
}

/// 沈降観測の記録行（純関数）。
///
/// `behind_foreground` は「当該窓が前面窓より背面に居るか」の判定で、前面窓が取れない
/// ／比較できない場合は番兵になる（偽と混同させない）。
pub(crate) fn sink_observed_line(
    entity: Entity,
    adjacency_ok: bool,
    foreground: Option<HWND>,
    behind_foreground: Option<bool>,
) -> String {
    format!(
        "{SINK_OBSERVED_TAG} entity={entity:?} adjacency_ok={adjacency_ok} \
         foreground={foreground} behind_foreground={behind}",
        foreground = hwnd_field(foreground),
        behind = tristate_field(behind_foreground),
    )
}

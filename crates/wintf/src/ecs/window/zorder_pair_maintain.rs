//! ペアの重なりの維持系——再断行の要求を受け、実測して判断し、重なりだけを動かし、
//! 次の巡で実測と突き合わせる。
//!
//! design.md「System Flows > 維持系の判断と適用（共通・1 フレーム 1 巡）」の手順を
//! そのまま実装したものである。
//!
//! ```text
//! トリガ（ReassertZOrder）→ 観測の組立（ペア解決と GetWindow 実測）
//!   → decide_pair_fix（純関数）
//!       → 見送り: 理由つきの記録（要件 6.3）で終わり
//!       → 是正 : SetWindowPosCommand を積む（位置・寸法・活性化は伴わない）
//!                → tick 後の flush（既存経路）
//!                → 次巡で実測照合 → 一致なら是正の記録／不一致なら error（要件 6.1／6.2）
//! ```
//!
//! # 受動性——トリガ駆動のみ
//!
//! 本 system は毎フレーム走るが、[`ReassertZOrder`] を持つ窓が 1 つも無ければ判断そのものを
//! 行わない。タイマも定期巡回も持たず、自分から窓を前面へ上げることは決してない
//! （design.md「Responsibilities & Constraints」——これが要件 4.1／4.2 の受動性の実装である）。
//!
//! # 収束の二重の停止条件
//!
//! ① 適用は [`SetWindowPosCommand`] の flush 経由であり、誘発される `WM_WINDOWPOSCHANGED` は
//! [`is_self_initiated`](super::is_self_initiated) でエコーと判定される。
//! ② [`decide_pair_fix`] は実測が既に隣接なら必ず見送る（同値ガード）。
//! 加えて検証は一致・不一致のどちらでも要求を消し、**再試行の輪を作らない**
//! （design.md「Error Strategy」の検証不一致）。
//!
//! # 書き込まないもの
//!
//! 位置・寸法・窓スタイル・表示状態には一切書き込まない（要件 1.6／5.x）。指令は必ず
//! [`pair_fix_command`] が組み、そこで `SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE` に固定される。
//!
//! # 本モジュールが担わないもの（後続タスクの担当）
//!
//! - 破棄時の owner 切離し（`clear_window_owner`）——相手の不在は本モジュールが
//!   [`SkipReason::PeerMissing`] として見つけるが、切離しの実行は後続タスクが足す。
//! - 他アプリ活性化時の沈降観測（`sink-observed` の遅延実測）——観測の目印を付けるのは
//!   `WM_ACTIVATE` の非活性化枝であり、その目印を読む巡を本 system へ足すのも後続タスクである。
//! - 重なりが動いたことの検知（案 B／補助浮上のトリガ供給）。要求さえ届けば本 system は
//!   トリガ種別によらず同じ手順で処理する。
//!
//! 記録を出すマクロは隣の [`zorder_pair`](super::zorder_pair) に置く（`tracing` の出力先は
//! 呼び出し元の module path が既定であり、分けるとサインオフの grep 対象が分裂する）。
//! 本モジュールは記録用の関数を呼ぶだけである。

use bevy_ecs::prelude::*;
use bevy_ecs::system::NonSendMarker;
use windows::Win32::Foundation::HWND;

use super::zorder_pair::{
    InsertSpec, PairFixDecision, PairObservation, PairTrigger, SkipReason, decide_pair_fix,
    log_orphan_request_dropped, log_unbacked_verification_dropped, measure_window_above,
    measure_window_below, record_decision, record_verification,
};
use super::{
    ExpectedOrder, KeepDirectlyAbove, ReassertZOrder, SetWindowPosCommand, WindowHandle, WindowPos,
    ZOrder, ZOrderPairStrategy,
};

// ============================================================================
// IssuedPairFix - 出した指令の次巡への持ち越し
// ============================================================================

/// 適用済みの是正が**どの挿入位置を指令したか**の持ち越し（バルーン窓へ付与）。
///
/// 是正の記録（`fix`）は「出した指令」と「その後の実測」を同じ 1 行に載せる（要件 6.1）が、
/// その行が出るのは適用の**次の巡**である。よって適用時に
/// [`PairFixDecision::insert_spec`](super::zorder_pair::PairFixDecision) の射影をここへ預け、
/// 次巡の [`record_verification`] へ渡す。
///
/// [`ReassertZOrder`] に相乗りさせないのは、あちらが**他クレートから挿入される公開の
/// 契約点**（要件 2.6 のシーム）であり、その形を wintf 内部の都合で変えないためである。
/// 預けるのも外すのも本モジュールだけであり、要求が消えるときは必ず一緒に外れる。
///
/// 公開到達性を持つのは、維持系（`pub` な system）の引数に現れるためだけである
/// （[`OwnerEstablishFailed`](super::OwnerEstablishFailed) と同じ理由）。フィールドは
/// 公開しない——他クレートが組み立てる意味を持たない。
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct IssuedPairFix {
    /// 出した指令の挿入位置（記録の `insert_after` 欄になる値）
    pub(crate) insert_after: InsertSpec,
}

// SAFETY: `IssuedPairFix` は [`InsertSpec`] を保持し、その `After` バリアントが `HWND`
// （windows-rs では `*mut c_void` の newtype）を内包するため自動では Send/Sync が導出されず、
// `Component`（ECS は Send+Sync を要求）にするためこの手動 impl は必須である。
// 健全性: 保持するのは**出した指令の写し**にすぎず、窓の所有権も破棄責務も伴わない。
// この値を Win32 へ渡すのは本モジュールの維持系だけであり、当該 system は NonSend
// パラメータで UI スレッド固定されている。[`OwnerLink`](super::OwnerLink) と同根の
// crate 標準の HWND 取り扱い方針。
unsafe impl Send for IssuedPairFix {}
unsafe impl Sync for IssuedPairFix {}

// ============================================================================
// PairFixPlan - 是正の適用計画（純関数の出力）
// ============================================================================

/// 是正を実際に出すために要る 3 つの値。
///
/// 判断（どう直すか）と適用（何をどこへ）を分ける境界であり、World も Win32 も要らない
/// 純関数 [`plan_pair_fix`] が組む。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PairFixPlan {
    /// 動かす窓（宣言ペアの 2 枚のいずれか 1 枚・要件 3.4）
    pub target: HWND,
    /// 挿入位置（記録に載る「出した指令」）
    pub insert_after: InsertSpec,
    /// 次巡で照合する期待隣接
    pub expected: ExpectedOrder,
}

/// 判断と観測から適用計画を組む（Win32 も World も触らない）。
///
/// 動かす窓は腕が一意に決める——手前へ寄せる腕（要件 1.2）ならバルーン窓、背後へ寄せる腕
/// （要件 1.3）ならキャラ窓であり、**第三の窓を指す余地が型に無い**（要件 3.4）。
/// 期待隣接はどちらの腕でも同じ「バルーンがキャラのすぐ手前」に収束するため 1 形で足りる。
///
/// `None` を返すのは、見送りだった場合と、両窓のハンドルが揃っていない観測だった場合である。
/// 後者は [`decide_pair_fix`] が先に見送りを返すため実際には届かないが、
/// **`None` を返せる形にしておくことで適用側に panic 経路を作らせない**（要件 6.4）。
pub(crate) fn plan_pair_fix(obs: &PairObservation, fix: PairFixDecision) -> Option<PairFixPlan> {
    let above = obs.above_hwnd?;
    let below = obs.below_hwnd?;
    let target = match fix {
        PairFixDecision::Skip(_) => return None,
        PairFixDecision::PlaceAboveOverBelow { .. } => above,
        PairFixDecision::PlaceBelowUnderAbove { .. } => below,
    };
    Some(PairFixPlan {
        target,
        insert_after: fix.insert_spec()?,
        expected: ExpectedOrder { above, below },
    })
}

/// 是正の指令を組む（重なりだけを動かす形に固定する）。
///
/// 位置・寸法を持たない [`WindowPos`] から組むのは、フラグの意味づけ（`SWP_NOMOVE`／
/// `SWP_NOSIZE`／`SWP_NOZORDER` の自動判定）と `ZOrder` → `hWndInsertAfter` の写像を
/// **既存の 1 箇所に委ねる**ためである。ここで二重に持つと、片方だけが変わったときに
/// 静かに食い違う。結果として常に `SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE` になる
/// ——位置・寸法は不変（要件 1.6）で、活性化も伴わない。
///
/// [`InsertSpec::TopEdge`] は `ZOrder::Top`（最上位）であって `ZOrder::TopMost`
/// （常時最前面）ではない——本フィーチャーは常時最前面を決して指さない（要件 4.3／8.1）。
pub(crate) fn pair_fix_command(hwnd: HWND, insert_after: InsertSpec) -> SetWindowPosCommand {
    let pos = WindowPos {
        zorder: match insert_after {
            InsertSpec::After(after) => ZOrder::InsertAfter(after),
            InsertSpec::TopEdge => ZOrder::Top,
        },
        position: None,
        size: None,
        no_activate: true,
        ..WindowPos::new()
    };
    SetWindowPosCommand::new(
        hwnd,
        0,
        0,
        0,
        0,
        pos.build_flags_for_system(),
        pos.get_hwnd_insert_after(),
    )
}

// ============================================================================
// apply_zorder_pair_maintenance - 維持系（トリガ→判断→適用→検証）
// ============================================================================

/// 維持系: 再断行の要求を消費し、重なりを是正し、次の巡で実測と突き合わせる。
///
/// # 要求の一生
///
/// 1. **未適用**（`pending_verify` が `None`）: 実測を採って [`decide_pair_fix`] に諮る。
///    見送りなら理由つきの記録を残して要求ごと消える。是正なら指令を積み、
///    [`IssuedPairFix`] を預け、`pending_verify` へ期待隣接を入れて検証待ちへ進む。
/// 2. **検証待ち**（`pending_verify` が `Some`）: 実測を照合し、一致なら是正の記録、
///    不一致なら error（要件 6.2）。どちらでも要求はここで消える。
///
/// 見送りでも要求を消すのは、一回限りの要求だからである。残せば毎巡同じ判断を繰り返し、
/// 記録が氾濫して「トリガ駆動のみ」という性質も失われる。取りこぼした是正は次のトリガで
/// 自然に処理される（再試行の輪を作らない・design.md「Error Strategy」と同根）。
///
/// # 相手が居ない巡（要件 1.5）
///
/// 検証待ちであっても、相手の実体やハンドルが先に消えていれば実測照合へは進まない
/// ——正常な破棄が error を吐く経路を作らないためである。理由つきの見送りとして記録し、
/// 残る窓には一切書き込まない。
///
/// # 失敗しても止まらない（要件 6.4）
///
/// `SetWindowPos` の失敗は flush 側が warn で継続し（`command.rs`）、効かなかった事実は
/// 次巡の検証不一致として error に残る。本 system に panic 経路は無い。
///
/// # UI スレッド固定
///
/// [`NonSendMarker`] を取るのは Win32（`GetWindow` 走査）を呼ぶためである。この印が付いた
/// system をスケジュール実行器はメインスレッド以外で走らせない
/// （[`SetWindowPosCommand`] のキューもスレッドローカルで UI スレッドを前提とする）。
pub fn apply_zorder_pair_maintenance(
    _ui_thread: NonSendMarker,
    mut commands: Commands,
    strategy: Option<Res<ZOrderPairStrategy>>,
    mut requests: Query<(
        Entity,
        &KeepDirectlyAbove,
        &mut ReassertZOrder,
        Option<&IssuedPairFix>,
    )>,
    // ペア宣言を持たない窓に付いた要求。宣言を持つ側とは `Without` で交わらない。
    orphans: Query<Entity, (With<ReassertZOrder>, Without<KeepDirectlyAbove>)>,
    // すべての entity に当たるクエリ。`Err` は「その実体がもう無い」、`Ok(None)` は
    // 「実体はあるがハンドルがまだ無い」——見送りの理由を分けるためこの形にする
    // （確立系 `establish_owner_links` と同じ形）。
    handles: Query<Option<&WindowHandle>>,
) {
    // ストラテジ未挿入は結線前の状態。既定（案 A）で動く（design.md「State」の既定値）。
    let strategy = strategy.map(|s| *s).unwrap_or_default();

    for entity in orphans.iter() {
        log_orphan_request_dropped(entity);
        commands.entity(entity).remove::<ReassertZOrder>();
    }

    for (entity, declaration, mut request, issued) in requests.iter_mut() {
        let peer = declaration.peer;
        let above_hwnd = handles.get(entity).ok().flatten().map(|h| h.hwnd);
        let peer_lookup = handles.get(peer);
        let below_alive = peer_lookup.is_ok();
        let below_hwnd = peer_lookup.ok().flatten().map(|h| h.hwnd);

        if let Some(expected) = request.pending_verify {
            // 検証は 1 巡限り。結果によらずここで要求と持ち越しを外す。
            commands
                .entity(entity)
                .remove::<(ReassertZOrder, IssuedPairFix)>();

            if !below_alive {
                // 指令を出した後・検証の前に相手が消えた。これは要件 1.5 の正常系であり
                // 検証不一致の error にしてはならない。
                record_decision(entity, peer, PairFixDecision::Skip(SkipReason::PeerMissing));
                continue;
            }
            if above_hwnd.is_none() || below_hwnd.is_none() {
                record_decision(
                    entity,
                    peer,
                    PairFixDecision::Skip(SkipReason::HandleMissing),
                );
                continue;
            }
            let Some(issued) = issued else {
                // 自分で適用した要求なら必ず持ち越しが付いている。外から検証段階の値だけを
                // 作られた場合であり、出してもいない指令を記録せずに落とす。
                log_unbacked_verification_dropped(entity, expected);
                continue;
            };

            // 隣接の正準判定は「手前側の 1 つ背後が背後側か」（design.md Invariants）。
            // 実測が採れなかった場合も一致とは扱わない（`record_verification` の約束）。
            let _verified = record_verification(
                entity,
                peer,
                issued.insert_after,
                expected,
                measure_window_below(expected.above),
            );
            continue;
        }

        // 未適用の要求。実測は両窓のハンドルが揃っているときだけ採る——揃っていなければ
        // 判断は実測を見るまでもなく見送りへ倒れる（無効なハンドルで Win32 を叩かない）。
        let (measured_below_of_above, measured_above_of_below) = match (above_hwnd, below_hwnd) {
            (Some(above), Some(below)) if below_alive => {
                (measure_window_below(above), measure_window_above(below))
            }
            _ => (None, None),
        };
        let observation = PairObservation {
            trigger: PairTrigger::Reassert,
            strategy,
            // 走査対象そのものなので実体は在る。ハンドルの有無は `above_hwnd` が表す。
            above_alive: true,
            below_alive,
            above_hwnd,
            below_hwnd,
            measured_below_of_above,
            measured_above_of_below,
        };

        // 判断結果は必ず `record_decision` を通す——見送りはそこで理由つきの記録になり、
        // 返ってくるのは適用すべき是正だけである（要件 6.3 の規約）。
        let Some(fix) = record_decision(entity, peer, decide_pair_fix(&observation)) else {
            commands.entity(entity).remove::<ReassertZOrder>();
            continue;
        };
        let Some(plan) = plan_pair_fix(&observation, fix) else {
            // 是正の腕が返った以上ハンドルは揃っており、ここへは届かない。届いても
            // panic は作らず、指令を出さずに要求を落とす（要件 6.4）。
            commands.entity(entity).remove::<ReassertZOrder>();
            continue;
        };

        SetWindowPosCommand::enqueue(pair_fix_command(plan.target, plan.insert_after));
        // 実際の書込は tick 後の flush である。ゆえに実測照合は次の巡で行う——同じ巡で
        // 測っても指令はまだ反映されていない。
        request.pending_verify = Some(plan.expected);
        commands.entity(entity).insert(IssuedPairFix {
            insert_after: plan.insert_after,
        });
    }
}

#[cfg(test)]
#[path = "zorder_pair_maintain_tests.rs"]
mod tests;

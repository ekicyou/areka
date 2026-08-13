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
//! 唯一トリガを持たないのが破棄時の owner 切離し
//! （[`detach_owner_links_for_lost_peers`]）である。破棄には要求を挿してくれる相手が居らず、
//! 「相手が消えた」こと自体を見つけるほかないためであり、見るのは owner を張った窓の集合に
//! 限られる。ここでも窓の重なりは動かさない——外すのは owner の関係だけである。
//!
//! 沈降観測（[`observe_marked_pair_sinks`]）も本 system の中で回る。こちらもトリガ駆動で
//! あり、目印を置くのは `WM_ACTIVATE` の非活性化枝である。実測を非活性化の巡ではなく
//! **次の巡**で行う理由は [`zorder_pair_sink`](super::zorder_pair_sink) の doc を参照
//! （活性化の途中で測ると偽の失敗を記録するため）。読み取りだけを行い、窓の挙動は変えない。
//!
//! # 収束の二重の停止条件
//!
//! ① 適用は [`SetWindowPosCommand`] の flush 経由であり、誘発される `WM_WINDOWPOSCHANGED` は
//! [`is_self_initiated`](super::is_self_initiated) でエコーと判定される。
//! ② [`decide_pair_fix`] は実測が既に隣接なら必ず見送る（同値ガード）。
//! 加えて検証は一致・不一致のどちらでも要求を消し、**再試行の輪を作らない**
//! （design.md「Error Strategy」の検証不一致）。
//!
//! # 複数ペアが同じ巡で是正を求めたとき——動かすのは高々 1 ペア
//!
//! 上の 2 つは 1 つのペアだけを見た停止条件であり、ペアが複数あるときの**観測の陳腐化**は
//! 塞がない。実測は巡の中で採るが実際の書込はその巡の後の flush であるため、同じ巡で
//! 2 つの是正を積むと、後から適用される指令の挿入位置は先の適用で既に古い——指した窓の
//! 隣に別の窓が割り込み、要件 1.1 の「すぐ手前」が成立しない（実機の 2 スコープ起動で
//! 毎回 `verify-failed` になっていた形）。
//!
//! よって **1 巡で実際に指令を出すのは高々 1 ペア**とする。残るペアの要求は消さずに残し、
//! 次の巡の**新しい実測**から計算し直す。持ち越しは見送りではないので記録も出さない
//! ——要求は entity に残り次巡で必ず処理されるため、記録の無い握り潰しにはならない。
//!
//! 巡数はペア数だけ要るが、ペアはスコープ数（実運用で 2）を超えない。しかも
//! **適用した是正が、既に隣接している別のペアを壊すことはない**——是正は自分のキャラ窓の
//! すぐ手前へ 1 枚だけ挿し込む操作であり、他ペアのバルーンとキャラの間へ割り込めるのは
//! そのペアが元から隣接していない場合だけだからである。よって単調に収束する。
//! **前提は 2 つ**——ペアどうしがキャラ窓を共有しないこと（areka の spawn はスコープごとに
//! キャラ 1 枚・バルーン 1 枚を作るので成立する）、および**キャラ窓が可視であること**
//! （隣接を「最も近い可視の隣」で測る以上、この関係が可視窓どうしで対称に成り立つことが
//! 論証の土台になる）。design.md「維持系の判断と適用」の収束の項と同じ論証である。
//!
//! # 書き込まないもの
//!
//! 位置・寸法・窓スタイル・表示状態には一切書き込まない（要件 1.6／5.x）。指令は必ず
//! [`pair_fix_command`] が組み、そこで `SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE` に固定される。
//!
//! # 本モジュールが担わないもの（後続タスクの担当）
//!
//! - 切離しの適用時点を早める強化——相手の消滅を検知した巡ではなく、窓の登録が実際に
//!   取り崩される直前で外す形。実機ゲート G8 が「切離しが間に合わない順序」を FAIL と
//!   判定した場合にのみ後続タスクが足す（design.md「破棄経路」）。
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
    InsertSpec, PairFixDecision, PairObservation, PairTrigger, PeerLoss, SkipReason,
    decide_pair_fix, log_orphan_request_dropped, log_owner_detach_failed, log_owner_detached,
    log_unbacked_verification_dropped, measure_window_above, measure_window_below, record_decision,
    record_verification,
};
use super::zorder_pair_sink::{SinkMarkQuery, observe_marked_pair_sinks};
use super::{
    ExpectedOrder, KeepDirectlyAbove, OwnerLink, ReassertZOrder, SetWindowPosCommand, WindowHandle,
    WindowPos, ZOrder, ZOrderPairStrategy,
};
use crate::api::clear_window_owner;

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
// detach_owner_links_for_lost_peers - 破棄時の owner 切離し
// ============================================================================

/// owner を張った窓（バルーン窓）をひと通り見るためのクエリ。
type OwnerLinkQuery<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static KeepDirectlyAbove,
        &'static OwnerLink,
        Option<&'static WindowHandle>,
    ),
>;

/// すべての entity に当たるクエリ（相手の生存とハンドルの有無を分けて読むための形）。
///
/// 沈降観測（[`observe_marked_pair_sinks`]）も同じ形で読むため、型をここで 1 つに保つ。
pub(crate) type HandleQuery<'w, 's> = Query<'w, 's, Option<&'static WindowHandle>>;

/// 相手を失ったペアの owner を外す（要件 5.7／5.8／5.9・design.md「破棄経路」）。
///
/// # なぜ外すのか
///
/// owner を張ったままキャラ窓が `DestroyWindow` されると、OS はその窓が owner である
/// バルーン窓も一緒に破棄する。すると後から走るバルーン窓側の破棄が、もう無効になった
/// ハンドルへ `DestroyWindow` を撃つ。同一スコープの 2 窓が対で消えること自体は許容
/// されている（要件 5.8）——禁じられているのは、その重複した破棄で異常終了することだけ
/// である（要件 5.9）。相手が消えた時点で owner を外しておけば、両窓は本フィーチャー以前と
/// 同じ「独立して生成され独立して破棄される 2 枚」に戻り、連鎖そのものが起きない。
///
/// # いつ外すのか
///
/// 相手（`peer`）の実体が消えた巡、ないし相手の OS ハンドルが外れた巡である。どちらも
/// 「相手の窓はもう無い／今まさに無くなる」を意味し、owner を残す理由が消えている。
/// 実体の消滅（despawn）と実際の `DestroyWindow`（登録の取り崩し）の**間**に本処理が入る
/// のが標準の形であり、その間隔で塞ぎ切れない順序（同一巡で両者が消える・アプリ終了経路）は
/// 実機ゲート G8 の判定対象である。
///
/// # 他スコープへは届かない
///
/// 外す対象は「自分が張った owner」だけであり、その owner は宣言ペア
/// （[`KeepDirectlyAbove`]）の相手にしか張られていない。スコープをまたぐ owner は
/// そもそも存在しないため、他スコープの窓が切離しにも破棄にも巻き込まれない（要件 5.7）。
///
/// # 失敗しても止まらない
///
/// `clear_window_owner` の失敗は記録するだけで伝播させない（要件 6.4）。いずれの経路でも
/// [`OwnerLink`] は外す——同じハンドルでの再試行は同じ失敗を繰り返し、記録を二重化する
/// だけだからである（design.md「Error Strategy」の確立失敗と同根）。
fn detach_owner_links_for_lost_peers(
    commands: &mut Commands,
    links: &OwnerLinkQuery,
    handles: &HandleQuery,
) {
    for (entity, declaration, link, handle) in links.iter() {
        let peer = declaration.peer;
        let loss = match handles.get(peer) {
            // 相手は健在——owner を残す（生きている窓の関係には触れない）。
            Ok(Some(_)) => continue,
            Ok(None) => PeerLoss::HandleRemoved,
            Err(_) => PeerLoss::Despawned,
        };

        // 外した先を記録に残せる形でのみ Win32 を呼ぶ。自分のハンドルが既に無ければ
        // 外す対象の窓が特定できない——黙って読み飛ばさず理由つきの見送りにする（要件 6.3）。
        match handle {
            Some(handle) => match clear_window_owner(handle.hwnd) {
                Ok(()) => log_owner_detached(entity, peer, handle.hwnd, link.owner_hwnd, loss),
                Err(err) => {
                    log_owner_detach_failed(entity, peer, handle.hwnd, link.owner_hwnd, &err)
                }
            },
            None => {
                record_decision(
                    entity,
                    peer,
                    PairFixDecision::Skip(SkipReason::HandleMissing),
                );
            }
        }
        commands.entity(entity).remove::<OwnerLink>();
    }
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
/// ただし **1 巡で指令を出すのは高々 1 ペア**であり、同じ巡で是正を要する 2 つめ以降の
/// ペアは①のまま次巡へ持ち越される（module doc「複数ペアが同じ巡で…」）。持ち越しは
/// 見送りではない——要求は残り、次巡の新しい実測で必ずやり直される。
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
    // owner を張った窓。破棄時の切離しが見るのはこの集合だけである。
    links: OwnerLinkQuery,
    // 沈降観測の目印が付いた窓（`WM_ACTIVATE` の非活性化枝が前の巡で付けたもの）。
    sink_marks: SinkMarkQuery,
    // すべての entity に当たるクエリ。`Err` は「その実体がもう無い」、`Ok(None)` は
    // 「実体はあるがハンドルがまだ無い」——見送りの理由を分けるためこの形にする
    // （確立系 `establish_owner_links` と同じ形）。
    handles: HandleQuery,
) {
    // ストラテジ未挿入は結線前の状態。既定（案 A）で動く（design.md「State」の既定値）。
    let strategy = strategy.map(|s| *s).unwrap_or_default();

    // 破棄の後始末を先に済ませる——相手が消えた窓の owner は、この巡のうちに外れていないと
    // 登録の取り崩し（`DestroyWindow`）に間に合わない。
    detach_owner_links_for_lost_peers(&mut commands, &links, &handles);

    // 前の巡に非活性化された窓の沈降を、いま実測して記録する（読み取りのみ）。
    // 是正の処理より先に置くのは、記録に載る実測が**この巡の是正より前の重なり**である
    // ことを明らかにするためである（是正の書込はいずれにせよ tick 後の flush で起きる）。
    observe_marked_pair_sinks(&mut commands, &sink_marks, &handles);

    for entity in orphans.iter() {
        log_orphan_request_dropped(entity);
        commands.entity(entity).remove::<ReassertZOrder>();
    }

    // この巡で既に是正の指令を出したか。実際に窓を動かす指令は 1 巡につき 1 本までとし、
    // 残る要求は次巡の新しい実測で計算し直す（module doc「複数ペアが同じ巡で…」）。
    let mut fix_issued_this_pass = false;

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

            // 隣接の正準判定は「手前側の最も近い可視の背後が背後側か」（design.md Invariants）。
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
        if fix_issued_this_pass {
            // この巡では既に別のペアの窓を動かす指令を積んでいる。その書込は本巡の後の
            // flush で起きるため、いま手にしている実測（と、そこから決まる挿入位置）は
            // 適用の時点では既に古い。要求はそのまま残し、次巡でやり直す——ここは
            // 見送りではないので記録も出さないし、要求も消さない。
            continue;
        }
        let Some(plan) = plan_pair_fix(&observation, fix) else {
            // 是正の腕が返った以上ハンドルは揃っており、ここへは届かない。届いても
            // panic は作らず、指令を出さずに要求を落とす（要件 6.4）。
            commands.entity(entity).remove::<ReassertZOrder>();
            continue;
        };

        SetWindowPosCommand::enqueue(pair_fix_command(plan.target, plan.insert_after));
        fix_issued_this_pass = true;
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

#[cfg(test)]
#[path = "zorder_pair_multipair_tests.rs"]
mod multipair_tests;

#[cfg(test)]
#[path = "zorder_pair_detach_tests.rs"]
mod detach_tests;

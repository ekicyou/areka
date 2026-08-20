//! 可視性ガード（[`VisibilityVerdict`]・[`guard_visibility`]・[`apply_visibility_guard`] ほか）。

use bevy_ecs::prelude::*;
use tracing::warn;

use super::{
    MonitorSnapshot, PlacementRoute, PointPx, RectPx, SizePx, WorkAreaResolution,
    work_area_for_window_with_origin,
};

/// [`guard_visibility`] の判定（D6・S3/S3′）。
///
/// いずれの腕も**最終位置そのもの**を持つ（呼出側が「clamp されたか」を見て warn
/// 水準を分岐しつつ、位置は腕を問わず [`VisibilityVerdict::position`] で取れる）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VisibilityVerdict {
    /// 提案位置をそのまま採る（交差維持・またはユーザーの明示留置の尊重）。
    Keep(PointPx),
    /// 交差→非交差の遷移を検出し、X のみ `clamp_wa` の水平範囲へ引き戻した。
    ClampX(PointPx),
}

impl VisibilityVerdict {
    /// 判定によらず最終位置を取り出す。
    pub fn position(self) -> PointPx {
        match self {
            VisibilityVerdict::Keep(p) | VisibilityVerdict::ClampX(p) => p,
        }
    }
}

/// 可視性の**遷移**ガード（純関数・非ドラッグ経路専用・D5/D6・Req 3.1/3.2/3.4）。
///
/// S3／S3′ が登記する欠陥は「キャラ窓・バルーン窓の水平方向に可視性の不変条件が
/// 存在しない」ことである。本関数はその不変条件を**遷移**として定義する——静的な
/// 「常に可視領域内」ではない。ユーザーが自ら画面外へ運んだ窓を引き戻すのは
/// 明示操作の否定であり本 spec の Out of scope だからである。
///
/// # 判定規則（4 分岐・すべて交差の有無で表現＝絶対 px の閾値を持たない・Req 5.6）
///
/// | 提案矩形が work area 集合と交差 | 旧矩形 | 判定 |
/// | --- | --- | --- |
/// | する | 問わない | [`Keep`](VisibilityVerdict::Keep)（素通し） |
/// | しない | 交差していた | [`ClampX`](VisibilityVerdict::ClampX)（可視→不可視の遷移を阻止） |
/// | しない | 交差していなかった | [`Keep`](VisibilityVerdict::Keep)（ユーザーの明示留置を尊重） |
/// | しない | `None`（不明） | [`ClampX`](VisibilityVerdict::ClampX)（安全側） |
///
/// # 引数
///
/// - `old_rect`: 書込**前**の窓矩形（現 `WindowPos` の position＋size）。窓生成直後等で
///   読めない場合は `None`＝安全側 clamp。
/// - `proposed_pos`／`size`: 射影 T が出した提案位置と、その位置に置く窓の寸。
/// - `clamp_wa`: clamp 先の work area。**射影が Y に用いたのと同じ矩形**を呼出側が
///   貫通させる（[`evaluate_visibility_guard`] が [`work_area_for_window_with_origin`] の
///   戻り値を渡す）。
///   ガード内で引き直さないのは、Y と X が別モニタを基準にする不整合を作らないため。
/// - `snapshot`: 交差判定に用いる全 work area 集合。
///
/// # 事後条件・不変条件
///
/// - **Y は一切変更しない**（Y は射影 T の所有・D6）。`Keep`／`ClampX` のいずれでも
///   `verdict.position().y == proposed_pos.y`。
/// - `ClampX` の X は `clamp_wa.left ..= clamp_wa.right − size.w` の範囲へ入る
///   （`saturating` 演算・逆転区間でも panic しない `min`/`max` 流儀）。窓幅が
///   work area より広い場合は `left` が勝つ＝左端合わせで**必ず水平に重なる**。
/// - 正寸かつ `proposed_pos` の Y 範囲が `clamp_wa` と重なるとき（＝射影 T が Y を
///   決めた正常系）、`ClampX` 後の矩形は `clamp_wa` と交差する＝完全不可視が消える。
/// - World 非依存・副作用なし・panic しない。ログは出さない——`ClampX`／
///   `NearestFallback`／`OffscreenPull` の `warn!` は route（経路タグ）で
///   水準が変わる呼出側（[`evaluate_visibility_guard`]）の責務
///   （Req 3.3・ドラッグ経路 spam 回避の水準分岐は route を持つ層でしか書けない）。
///
/// # 縮退
///
/// 空 snapshot では何も交差しないため、`old_rect` が `Some`（＝同じく非交差）なら
/// `Keep`＝現状維持。架空の可視領域を発明しない（resolver／`work_area_for_window`
/// と同方針）。
pub fn guard_visibility(
    old_rect: Option<RectPx>,
    proposed_pos: PointPx,
    size: SizePx,
    clamp_wa: RectPx,
    snapshot: &MonitorSnapshot,
) -> VisibilityVerdict {
    // 1. 提案矩形がどれかの work area と交差していれば可視性は失われていない。
    if intersects_any_work_area(snapshot, rect_at(proposed_pos, size)) {
        return VisibilityVerdict::Keep(proposed_pos);
    }

    // 2. 旧矩形も非交差だった＝ユーザーが自ら画面外へ留置した窓（Out of scope）。
    //    旧矩形不明（`None`）はここに含めない＝安全側で clamp する。
    let was_already_off_screen = match old_rect {
        Some(old) => !intersects_any_work_area(snapshot, old),
        None => false,
    };
    if was_already_off_screen {
        return VisibilityVerdict::Keep(proposed_pos);
    }

    // 3. 交差→非交差の遷移（または旧矩形不明）＝X のみ引き戻す。Y は射影の所有。
    VisibilityVerdict::ClampX(PointPx {
        x: clamp_x_into(proposed_pos.x, size.w, clamp_wa),
        y: proposed_pos.y,
    })
}

/// 位置＋寸から窓矩形を作る（`right`/`bottom` は排他側・`saturating` で溢れない）。
pub(super) fn rect_at(pos: PointPx, size: SizePx) -> RectPx {
    RectPx {
        left: pos.x,
        top: pos.y,
        right: pos.x.saturating_add(size.w),
        bottom: pos.y.saturating_add(size.h),
    }
}

/// 2 矩形が**面積を持って**重なるか（半開区間・接触のみは交差としない）。
fn rects_intersect(a: RectPx, b: RectPx) -> bool {
    (a.left as i64) < (b.right as i64)
        && (b.left as i64) < (a.right as i64)
        && (a.top as i64) < (b.bottom as i64)
        && (b.top as i64) < (a.bottom as i64)
}

/// いずれかの work area と交差するか（空 snapshot は常に `false`）。
fn intersects_any_work_area(snapshot: &MonitorSnapshot, window: RectPx) -> bool {
    snapshot
        .work_areas
        .iter()
        .any(|wa| rects_intersect(window, *wa))
}

/// X を `wa.left ..= wa.right − w` へ引き戻す（`i32::clamp` は逆転区間で panic する
/// ため min/max 流儀・`work_area_for_window` の最近傍 clamp と同型の防波堤）。
fn clamp_x_into(x: i32, w: i32, wa: RectPx) -> i32 {
    x.min(wa.right.saturating_sub(w)).max(wa.left)
}

// =============================================================================
// 遷移ガードの配線（task 6.1・S3 是正・D5/D6/D13・Req 3.1/3.2/3.3）
// =============================================================================

/// 遷移ガードが X を引き戻したことを表す判定語（`diagnosis-procedure.md` §3.3）。
const VISIBILITY_CLAMP_TAG: &str = "[visibility-guard] ClampX";

/// work area 解決が最近傍フォールバックへ落ちたことを表す判定語（同上・Req 3.2）。
const VISIBILITY_NEAREST_FALLBACK_TAG: &str = "[visibility-guard] NearestFallback";
/// **どの work area とも交差しない位置に居た窓**を最近傍モニタへ引き寄せたことを表す
/// 判定語（areka-P0-dpi-transition-atomicity 要件 5.5・task 5.1 が新設）。
///
/// [`VISIBILITY_NEAREST_FALLBACK_TAG`] と観測する矩形が違う——あちらは射影が**決めた
/// 位置**、こちらは射影の**入力**（＝Y を決めるのに使った矩形）である。入力が画面外に
/// あった窓は、決めた位置がモニタ内へ収まれば あちらの腕に入らない（実測: 全 work area の
/// 上方に居る窓が最近傍モニタの下端へ引き寄せられても、観測は 1 行も出なかった）。
/// 副モニタを引き抜いたときにゴーストが主モニタへ引き寄せられるのは**正しい挙動**
/// （開発者の裁定 2026-08-20・位置は変えない）だが、**勝手に飛んだことは後から追えねば
/// ならない**——本語はそのための記録である。
///
/// # 交差の有無まで見る理由（帰属だけを見ると偽陽性になる）
///
/// 入力の帰属（`NearestFallback`）だけを条件にすると、**下端吸着の正常な resize** が
/// 引っ掛かる——旧位置に新しい（背の高い）寸を当てた矩形の中心は work area 下端より下へ
/// 出ることが珍しくなく、半開区間の帰属では非該当になるからである。これは
/// [`apply_visibility_guard`] の doc が「`raw` で判定すると射影が正しく接地させた窓まで
/// 報告する偽陽性になる」と予告していた事象で、実装中に既存の檻
/// （`frame_diag_route_tests` の破棄済み窓の檻）が実際に捕まえた（中心 `cy=1444` が
/// `wa.bottom=1444` にちょうど載る形）。ゆえに条件は「帰属しない **かつ** どの work area
/// とも**面積を持って交差しない**」＝真に画面の外に居た窓に限る。接地直前の窓は必ず
/// モニタと重なっているので、この腕には入らない。
///
/// 語が `[visibility-guard] NearestFallback` を**部分文字列として含まない**のは、
/// 既存の檻が件数（ちょうど 1 件）で判定しているためである。
const VISIBILITY_OFFSCREEN_PULL_TAG: &str = "[visibility-guard] OffscreenPull";

/// work area が解決できずガードを評価できなかったことを表す判定語（同上・Req 3.3）。
pub(super) const VISIBILITY_UNRESOLVED_TAG: &str = "[visibility-guard] WorkAreaUnresolved";

/// この `route` の書込が**非ドラッグの自動配置**か（＝遷移ガードの発火対象・D13 帰結⑴）。
///
/// # なぜ route が第一級の入力なのか
///
/// S3 が防ぐのは「**ユーザーが意図せず**窓を見失う」経路だけである（requirements.md
/// Boundary Context「ユーザーが自らドラッグして運んだ結果の不可視化」は Out of scope）。
/// 明示操作（ドラッグ・`\![move]`）とスクリプト／永続化が決めた位置を引き戻すのは
/// その否定であり、**同じ矩形・同じ幾何でも判定が反転する**。ゆえに発火条件は幾何では
/// 表現できず、書込の由来＝route を見るしかない。
///
/// # 網羅 `match` で書く理由
///
/// 既定腕（`_ => false` 等）を置くと、[`PlacementRoute`] へ語彙が増えたとき新経路が
/// 黙って片側へ倒れる。網羅 `match` ならコンパイラが判断を要求する（D14 帰結⑵と同じ流儀）。
///
/// # 適用外の内訳
///
/// - [`SpawnInitial`](PlacementRoute::SpawnInitial)／[`Restore`](PlacementRoute::Restore):
///   復元時の可視化保証は `areka-P0-position-persist` の所有（design Boundary）。
/// - [`MoveCue`](PlacementRoute::MoveCue): `\![move]` はスクリプトの明示操作（D13 帰結⑵）。
/// - [`BalloonLimitRelease`](PlacementRoute::BalloonLimitRelease): バルーン窓側の書込であり、
///   かつ `windowposition.limit` の関門が既に画面内を保証した後の位置である
///   （areka-P0-windowposition-limit DD7）。遷移ガードで重ねて引き戻す対象ではない。
/// - [`KeepPositionResize`](PlacementRoute::KeepPositionResize)／
///   [`BalloonFollow`](PlacementRoute::BalloonFollow): バルーン窓側の書込。
///   **本述語をそのままバルーン適用（task 6.2）の発火条件に流用しないこと**——
///   バルーンの適用可否は「随伴の**引き金**がドラッグだったか配置系だったか」で決まり、
///   `follow_balloon` の呼出元が持つ情報である（本述語の入力は書込自身の route）。
///   task 6.2 は [`BalloonFollowTrigger`] を新設して**引き金**を配管し、その
///   [`Placement`](BalloonFollowTrigger::Placement) 腕が**引き金の route** に対して
///   本述語を引く形にした（本述語へ `BalloonFollow` を渡す形にはしていない）。
pub(super) fn route_applies_visibility_guard(route: PlacementRoute) -> bool {
    match route {
        // 非ドラッグの自動配置（S3 の保護対象・D13 帰結⑴）
        PlacementRoute::AnchorChange
        | PlacementRoute::Resnap
        | PlacementRoute::DpiReproject
        | PlacementRoute::ReportedSizeReconcile
        // 作業領域の変化・遷移後の連鎖再解決も**システム由来の再アンカー**であり、
        // ユーザーの明示操作ではない（design D9 が既定位置の追跡対象として同じ 6 経路を
        // 挙げているのと同じ区分）。ゆえに S3 の保護対象＝ガードは発火する。
        | PlacementRoute::WorkAreaResnap
        | PlacementRoute::ChainRealign => true,
        // 明示操作・別 spec 所有・バルーン窓側（上記 doc の内訳）
        PlacementRoute::SpawnInitial
        | PlacementRoute::Restore
        | PlacementRoute::KeepPositionResize
        | PlacementRoute::BalloonFollow
        | PlacementRoute::MoveCue
        | PlacementRoute::BalloonLimitRelease => false,
    }
}

/// 射影 T の**下流・外側**で可視性の遷移ガードを適用する（D5: `project_anchor` の
/// 内部は変更しない）。
///
/// # 引数
///
/// - `route`: 発火条件（[`route_applies_visibility_guard`]）。適用外なら `proposed` を素通す。
/// - `snapshot`／`raw`: 射影 T が work area を選んだのと**同一の入力**。
/// - `old_rect`: 書込**前**の窓矩形。`None`＝不明で安全側 clamp（[`guard_visibility`]）。
///
/// # 2 つの解決を引き分ける（同じ純関数を 2 回引くのは意図的）
///
/// - **clamp 先**（`clamp_wa`）は射影 T が Y に用いたのと同じ矩形（`raw` × `size`）から
///   引く。ここを別の矩形で引き直すと Y と X が別モニタを基準にして
///   [`guard_visibility`] の事後条件（clamp 後に `clamp_wa` と交差する）が崩れる
///   （design Risks・[`guard_visibility`] doc の `clamp_wa` 項）。
/// - **食い違いの観測**（Req 3.2）は**射影 T が決めた位置**（`proposed` × `size`）の帰属で
///   判定する。要件が言う「窓位置を**決めた**とき」の位置がこれであり、射影の入力 `raw`
///   は下端吸着より前の一時状態にすぎない——`raw` で判定すると「射影が正しく接地させて
///   可視域へ収めた窓」まで食い違いとして報告する偽陽性になる（下端吸着では
///   `raw` の中心が work area 下端より下にあることは珍しくない）。
///
/// # ログ（Req 3.1/3.2/3.3・[[2.2 → 6.1 の申し送り]]）
///
/// [`guard_visibility`] は**意図的に無ログ**の純関数で、水準の分岐（非ドラッグ経路は
/// `warn!`／ドラッグ経路は従来 `debug!` のまま）は route を持つ本層でしか書けない。
/// ゆえに観測は本関数の責務である——ここで出さなければ Req 3.1/3.2 の観測が丸ごと欠落する。
///
/// # 縮退（Req 3.3）
///
/// `MonitorSnapshot` 不在／空 snapshot では work area が 1 つも無く、clamp 先を決められない。
/// このとき**位置には一切手を入れず** `warn!` を残す（架空の可視領域を発明しない＝
/// `work_area_for_window` と同方針）。この場合の `proposed` は射影 T 自身が同じ入力欠落で
/// identity へ縮退した値＝現在位置であり、「現状維持」がそのまま成立する。
pub(super) fn apply_visibility_guard(
    entity: Entity,
    route: PlacementRoute,
    snapshot: Option<&MonitorSnapshot>,
    old_rect: Option<RectPx>,
    raw: PointPx,
    proposed: PointPx,
    size: SizePx,
) -> PointPx {
    if !route_applies_visibility_guard(route) {
        return proposed;
    }
    evaluate_visibility_guard(entity, route, snapshot, old_rect, raw, proposed, size)
}

/// 発火可否の判定が**済んだ後**の本体（評価＋観測）。
///
/// キャラ窓（[`apply_visibility_guard`]＝書込自身の route で発火判定）とバルーン窓
/// （[`guard_balloon_position`]＝随伴の**引き金**で発火判定・task 6.2）が共有する。
/// 発火判定だけを外へ出したのは、両者で判定の**入力が違う**（書込の route ⇔ 引き金の
/// route）一方、評価規則・clamp 先の引き方・3 語の観測は**完全に同一**だからである
/// （design「バルーン適用（S3′ 是正）」＝新規機構ゼロ）。
///
/// `route` は**ログに載る経路名**であり、発火判定には用いない——バルーン随伴の書込は
/// [`PlacementRoute::BalloonFollow`] として記録される（[`enqueue_window_set_pos`] が出す
/// `[diag.window_move]` レコードと同じ語）ので、警告行とレコード行が同じ route 名で
/// 突合できる。
pub(super) fn evaluate_visibility_guard(
    entity: Entity,
    route: PlacementRoute,
    snapshot: Option<&MonitorSnapshot>,
    old_rect: Option<RectPx>,
    raw: PointPx,
    proposed: PointPx,
    size: SizePx,
) -> PointPx {
    let Some(snapshot) = snapshot else {
        warn!(
            entity = ?entity,
            ?route,
            "{VISIBILITY_UNRESOLVED_TAG} MonitorSnapshot 未挿入のため可視性を判定できない → 位置は現状維持"
        );
        return proposed;
    };
    // 射影が Y に用いたのと同じ矩形（raw × 新寸）から引き直す＝clamp 先の貫通。
    let Some((clamp_wa, input_resolution)) =
        work_area_for_window_with_origin(snapshot, rect_at(raw, size))
    else {
        warn!(
            entity = ?entity,
            ?route,
            "{VISIBILITY_UNRESOLVED_TAG} モニタ 0 台（空 snapshot）のため可視性を判定できない → 位置は現状維持"
        );
        return proposed;
    };

    // 射影の**入力**が画面の外（どの work area とも非交差）に居て、最近傍モニタへ引き寄せ
    // られた（atom 要件 5.5 の記録側）。ここを観測しないと、決めた位置がモニタ内へ収まる
    // 限り下の観測（`NearestFallback`）にも掛からず、**ゴーストが飛んだ事実が 1 行も残らない**。
    // 非ドラッグ経路でしか本関数へ来ないので、この `warn!` はドラッグの spam を生まない。
    //
    // **位置は変えない**（開発者の裁定 2026-08-20）。最近傍フォールバックは「解決できなかった」
    // ではなく「最近傍で解決した」であり、現行挙動が正である——副モニタを引き抜いたときに
    // 現状維持を選ぶと、ゴーストは画面外に取り残されて見えず触れなくなる。要件 5.5 の
    // 「位置を変更せずに現状を維持」が効くのはモニタ表が空のとき（上の腕）に限る。
    // 交差の有無まで条件に入れる理由は判定語の doc を参照（帰属だけだと下端吸着の正常系を
    // 偽陽性で叩く）。
    let input_rect = rect_at(raw, size);
    if input_resolution == WorkAreaResolution::NearestFallback
        && !intersects_any_work_area(snapshot, input_rect)
    {
        warn!(
            entity = ?entity,
            ?route,
            ?input_rect,
            ?clamp_wa,
            "{VISIBILITY_OFFSCREEN_PULL_TAG} どの work area とも交差しない位置に居た窓を最近傍モニタへ引き寄せた（位置はそのまま＝ゴーストを画面外に取り残さないための裁定挙動）"
        );
    }

    // 最近傍フォールバック＝**決めた位置**の窓中心がどのモニタにも属さない＝モニタ構成
    // 情報と実画面の食い違い、あるいは窓が既に可視領域外という異常の兆候（Req 3.2・
    // S3 後段「最近傍フォールバックが異常を無観測で吸収する」）。ドラッグ経路は毎イベント
    // 発火ゆえ従来 `debug!` のまま（本関数を通らない＝水準分岐が route で成立する）。
    let decided = work_area_for_window_with_origin(snapshot, rect_at(proposed, size));
    if matches!(decided, Some((_, WorkAreaResolution::NearestFallback))) {
        warn!(
            entity = ?entity,
            ?route,
            ?proposed,
            ?size,
            ?clamp_wa,
            "{VISIBILITY_NEAREST_FALLBACK_TAG} 決めた位置の窓中心がどの work area にも属さず最近傍で解決した（モニタ構成情報と実画面の食い違いの兆候）"
        );
    }

    // 判定は「腕を見て warn 水準を分岐する」ためだけに使い、位置は腕を問わず
    // [`VisibilityVerdict::position`] で取る（同 enum の doc が定める消費の形）。
    let verdict = guard_visibility(old_rect, proposed, size, clamp_wa, snapshot);
    if let VisibilityVerdict::ClampX(clamped) = verdict {
        warn!(
            entity = ?entity,
            ?route,
            ?old_rect,
            ?proposed,
            clamped = ?clamped,
            ?size,
            ?clamp_wa,
            "{VISIBILITY_CLAMP_TAG} 全 work area 非交差への遷移を検出し X を引き戻した（Y は射影の所有ゆえ不変）"
        );
    }
    verdict.position()
}

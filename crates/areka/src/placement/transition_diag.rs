//! 配置判断の**遷移観測レコード**（`kind=snapshot`／`hold`／`ground`／`chain`）——語彙・
//! レコード純関数・World からの転写口。
//!
//! design.md の Data Models「Logical Data Model（レコード語彙）」の areka 行が正本である。
//! 窓書込・サーフェス更新・モニタ表更新（wintf・areka-emo-present が出す）と**同一の
//! フレーム番号系列**へ、配置側の判断を 4 種のレコードとして並べる。
//!
//! ```text
//! [transition] frame=<u32> t_us=<u64> kind=snapshot monitors=<n> m0=<dpi>:<l,t,r,b> …
//! [transition] frame=<u32> t_us=<u64> kind=hold   entity=<e> scope=<s> win_kind=<k> window_dpi=… table_dpi=… since_frame=… decision=… site=…
//! [transition] frame=<u32> t_us=<u64> kind=ground scope=<s> ground_y=<y> wa_bottom=<b> diff=<d> route=<r>
//! [transition] frame=<u32> t_us=<u64> kind=chain  stage=<s> scopes=<n> moved=<n> reason=<r>
//! ```
//!
//! 接頭語（`frame`／`t_us`／`kind`）は [`base::record_prefix`] が組む。段階ごとに意味を持たない
//! フィールドは**落とさず**番兵 [`base::MISSING`]（`-`）で埋める——落とすと「記録が出ていない」と
//! 「その経路にはその値が無い」の区別が事後に付かない（wintf C1・emo-present C3 と同じ規律）。
//!
//! # 語彙の定義元
//!
//! 共有のフィールド名（`entity`／`scope`／`win_kind`／`stage`）は wintf が単一の定義元であり、
//! ここは**参照するだけ**である。areka が足すのは 4 つの種別語と、この 4 種にしか出ない
//! フィールド名・判定語だけ（wintf は上位 crate の語彙を持たない＝依存方向 wintf ← areka）。
//!
//! # 発行はまだ 1 種だけ
//!
//! 本ファイルは task 2.4（観測の増設）で建てた語彙であり、`snapshot`／`hold`／`chain` を出す
//! 状態機械は後続タスクが新設する（作業領域源の同期＝task 5.1、整合待ち＝task 5.4、
//! 連鎖再解決＝task 5.6）。今このリポジトリで実際に到達できる発行点は
//! [`log_char_ground`]（`resize_window_to` から）だけである。
//!
//! # 語彙は先に建てる（未消費の `#[allow(dead_code)]` の根拠）
//!
//! 判定語の一覧（`*_ALL`）と per-kind の必須フィールド列（`*_FIELDS`）、および `hold`／`chain`
//! にしか出ない語は、**判定器（task 3.1／3.2）と発行側（task 5.1／5.4／5.6）が同じ字面を見る**
//! ための単一の定義元である。どちらもまだ着地していないので、非 test ビルドでは未消費に見える
//! （areka は lib ターゲットを持たない bin crate ゆえ `pub` でも dead_code 免除されない）。
//! 個別に `#[allow(dead_code)]` を置くのはモジュール大の allow が**以後の真の dead code を隠す**
//! ためで、`placement::diag` が同じ理由でモジュール大の allow を撤去したのに倣う。
//!
//! # 既定 OFF・前置ガードは呼出側が持つ
//!
//! レコードを**組む**（＝`String` を確保する）前に [`is_enabled`] で分岐するのは発行点の責務で
//! ある。`recompose-budget` が成立させた定常状態のアロケーション 0（要件 10.4）は「既定運転で
//! 新たな確保が 1 バイトも起きない」ことを含むため、ガードが組立より外側になければならない。
//! 発行が `debug!` である以上、濾過テストではこの退行を検出できない——固定するのは
//! `follow_transition_diag_tests.rs` の**本文走査**である。

use bevy_ecs::prelude::*;
use wintf::ecs::FrameCount;
use wintf::ecs::window::monitor::Monitor;
use wintf::ecs::window::transition_diag::{
    self as base, FIELD_ENTITY, FIELD_SCOPE, FIELD_STAGE, FIELD_WIN_KIND, MISSING, Stamp,
    TickStart, WriteTag,
};

use super::diag::{PlacementRoute, WindowKind};
use super::follow::{MonitorSnapshot, work_area_for_window};
use super::resolver::{PointPx, RectPx, SizePx};
use super::spawn::{BalloonWindowMarker, CharWindowMarker};

/// 観測チャネルが点灯しているか（前置ガード）。
///
/// 定義元は wintf の 1 箇所だけである（判定を二重に持つと、片方だけが水準や target を
/// 変えたときに静かに食い違う）。ここに置くのは、配置側の発行点が `placement::transition_diag`
/// だけを見ればよいようにするための再輸出である。
pub use wintf::ecs::window::transition_diag::is_enabled;

// ---------------------------------------------------------------------------
// レコード種別（kind 語）
// ---------------------------------------------------------------------------

/// 作業領域源（モニタ表の写し）の同期。
pub const KIND_SNAPSHOT: &str = "snapshot";
/// 窓の拡大率とモニタ表の整合待ち。
pub const KIND_HOLD: &str = "hold";
/// 下端吸着キャラ窓の接地点と作業領域下端の差。
pub const KIND_GROUND: &str = "ground";
/// 連鎖（隣接ペア）の再解決。
pub const KIND_CHAIN: &str = "chain";

/// areka が発行する `kind` 語の全体（判定側の語彙照合・語の一意性テストが参照する）。
///
/// wintf の [`base::KIND_ALL`] とも emo-present の `KIND_SURFACE` とも交わらない
/// （交わると判定側のレコード振り分けと遷移の起点判定が壊れる）。
#[allow(dead_code)] // 語彙先着（module doc「語彙は先に建てる」）
pub const PLACEMENT_KIND_ALL: &[&str] = &[KIND_SNAPSHOT, KIND_HOLD, KIND_GROUND, KIND_CHAIN];

// ---------------------------------------------------------------------------
// 整合待ちの判定語（decision）と観測点語（site）
// ---------------------------------------------------------------------------

/// 表と一致（または表を引けない）ため、そのまま処理する。
#[allow(dead_code)] // 語彙先着（module doc「語彙は先に建てる」）
pub const HOLD_DECISION_PROCEED: &str = "proceed";
/// 不一致ゆえ当該窓の窓書込を見送る。
#[allow(dead_code)] // 語彙先着（module doc「語彙は先に建てる」）
pub const HOLD_DECISION_HOLD: &str = "hold";
/// 上限フレームを超えたので警告の上で処理する。
#[allow(dead_code)] // 語彙先着（module doc「語彙は先に建てる」）
pub const HOLD_DECISION_PROCEED_AFTER_TIMEOUT: &str = "proceed-after-timeout";

/// 整合待ちの判定語の全体。
#[allow(dead_code)] // 語彙先着（module doc「語彙は先に建てる」）
pub const HOLD_DECISION_ALL: &[&str] = &[
    HOLD_DECISION_PROCEED,
    HOLD_DECISION_HOLD,
    HOLD_DECISION_PROCEED_AFTER_TIMEOUT,
];

/// 判定を下した観測点: 拡大率の相。
#[allow(dead_code)] // 語彙先着（module doc「語彙は先に建てる」）
pub const HOLD_SITE_DPI: &str = "dpi";
/// 判定を下した観測点: 報告寸の突合。
#[allow(dead_code)] // 語彙先着（module doc「語彙は先に建てる」）
pub const HOLD_SITE_RECONCILE: &str = "reconcile";
/// 判定を下した観測点: 再スナップ。
#[allow(dead_code)] // 語彙先着（module doc「語彙は先に建てる」）
pub const HOLD_SITE_RESNAP: &str = "resnap";

/// 観測点語の全体（design C5 の「3 点すべて」＝待ち札の守備範囲そのもの）。
#[allow(dead_code)] // 語彙先着（module doc「語彙は先に建てる」）
pub const HOLD_SITE_ALL: &[&str] = &[HOLD_SITE_DPI, HOLD_SITE_RECONCILE, HOLD_SITE_RESNAP];

// ---------------------------------------------------------------------------
// 連鎖再解決の段階語（stage）
// ---------------------------------------------------------------------------

/// 遷移を検知して解き直しを武装した。
#[allow(dead_code)] // 語彙先着（module doc「語彙は先に建てる」）
pub const CHAIN_STAGE_ARMED: &str = "armed";
/// 解き直しを実行した。
#[allow(dead_code)] // 語彙先着（module doc「語彙は先に建てる」）
pub const CHAIN_STAGE_REALIGNED: &str = "realigned";
/// 条件未達で見送った。
#[allow(dead_code)] // 語彙先着（module doc「語彙は先に建てる」）
pub const CHAIN_STAGE_DEFERRED: &str = "deferred";

/// 連鎖レコードの段階語の全体。
#[allow(dead_code)] // 語彙先着（module doc「語彙は先に建てる」）
pub const CHAIN_STAGE_ALL: &[&str] = &[
    CHAIN_STAGE_ARMED,
    CHAIN_STAGE_REALIGNED,
    CHAIN_STAGE_DEFERRED,
];

// ---------------------------------------------------------------------------
// フィールド名（判定側が辞書引きする語）
// ---------------------------------------------------------------------------

/// 同期した作業領域源のモニタ台数。
pub const FIELD_MONITORS: &str = "monitors";
/// 窓が持つ拡大率。
pub const FIELD_WINDOW_DPI: &str = "window_dpi";
/// 帰属モニタの表が持つ拡大率（引けなければ番兵）。
pub const FIELD_TABLE_DPI: &str = "table_dpi";
/// 待ち始めたフレーム番号。
pub const FIELD_SINCE_FRAME: &str = "since_frame";
/// 整合待ちの判定（[`HOLD_DECISION_ALL`] のいずれか）。
pub const FIELD_DECISION: &str = "decision";
/// 判定を下した観測点（[`HOLD_SITE_ALL`] のいずれか）。
pub const FIELD_SITE: &str = "site";
/// 接地点（窓矩形の下端・物理 px）。
pub const FIELD_GROUND_Y: &str = "ground_y";
/// 作業領域の下端（実行時のモニタ表から引く・引けなければ番兵）。
pub const FIELD_WA_BOTTOM: &str = "wa_bottom";
/// 接地点と作業領域下端の差（`ground_y − wa_bottom`・引けなければ番兵）。
pub const FIELD_DIFF: &str = "diff";
/// 接地点を書いた経路語彙。
pub const FIELD_ROUTE: &str = "route";
/// 連鎖の対象スコープ数。
pub const FIELD_SCOPES: &str = "scopes";
/// 実際に動かしたスコープ数。
pub const FIELD_MOVED: &str = "moved";
/// 見送りの理由（見送り以外は番兵）。
pub const FIELD_REASON: &str = "reason";

/// `kind=snapshot` 行の必須フィールド（接頭語と可変長の `m<i>` を除く）。
#[allow(dead_code)] // 語彙先着（module doc「語彙は先に建てる」）
pub const SNAPSHOT_FIELDS: &[&str] = &[FIELD_MONITORS];

/// `kind=hold` 行の必須フィールド（接頭語を除く）。
#[allow(dead_code)] // 語彙先着（module doc「語彙は先に建てる」）
pub const HOLD_FIELDS: &[&str] = &[
    FIELD_ENTITY,
    FIELD_SCOPE,
    FIELD_WIN_KIND,
    FIELD_WINDOW_DPI,
    FIELD_TABLE_DPI,
    FIELD_SINCE_FRAME,
    FIELD_DECISION,
    FIELD_SITE,
];

/// `kind=ground` 行の必須フィールド（接頭語を除く）。
#[allow(dead_code)] // 語彙先着（module doc「語彙は先に建てる」）
pub const GROUND_FIELDS: &[&str] = &[
    FIELD_SCOPE,
    FIELD_GROUND_Y,
    FIELD_WA_BOTTOM,
    FIELD_DIFF,
    FIELD_ROUTE,
];

/// `kind=chain` 行の必須フィールド（接頭語を除く）。
#[allow(dead_code)] // 語彙先着（module doc「語彙は先に建てる」）
pub const CHAIN_FIELDS: &[&str] = &[FIELD_STAGE, FIELD_SCOPES, FIELD_MOVED, FIELD_REASON];

// ---------------------------------------------------------------------------
// レコード
// ---------------------------------------------------------------------------

/// 作業領域源に載るモニタ 1 台ぶん（拡大率＋作業領域）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
// 発行側（task 5.1 の作業領域源の同期）は未着地。areka は bin crate ゆえ `pub` でも
// dead_code 免除されない（`diag.rs` の同型 allow と同じ事情）。
#[allow(dead_code)]
pub struct MonitorEntry {
    /// モニタの拡大率。
    pub dpi: u32,
    /// 作業領域（物理 px）。
    pub work_area: RectPx,
}

/// 作業領域源を作り直した記録。
#[derive(Clone, Copy, Debug)]
#[allow(dead_code)] // 発行側は task 5.1（作業領域源の実行時同期）
pub struct SnapshotRecord<'a> {
    /// 刻印。
    pub stamp: Stamp,
    /// 同期後のモニタ列（列挙順）。**0 台は空スライス**＝`monitors=0` として観測できる。
    pub monitors: &'a [MonitorEntry],
}

/// 整合待ちの判定 1 件の記録。
#[derive(Clone, Copy, Debug)]
#[allow(dead_code)] // 発行側は task 5.4（拡大率と表の整合待ち）
pub struct HoldRecord {
    /// 刻印。
    pub stamp: Stamp,
    /// 対象窓の entity（wintf 側ログとの結合キー）。
    pub entity: Entity,
    /// スコープ番号（marker を持たない窓は `None`＝番兵）。
    pub scope: Option<u32>,
    /// 窓種別（[`WindowKind::as_str`] の値・判らなければ [`MISSING`]）。
    pub win_kind: &'static str,
    /// 窓が持つ拡大率。
    pub window_dpi: u32,
    /// 帰属モニタの表が持つ拡大率（帰属なし・表なしは `None`＝番兵）。
    pub table_dpi: Option<u32>,
    /// 待ち始めたフレーム番号。
    pub since_frame: u32,
    /// 判定（[`HOLD_DECISION_ALL`] のいずれか）。
    pub decision: &'static str,
    /// 判定を下した観測点（[`HOLD_SITE_ALL`] のいずれか）。
    pub site: &'static str,
}

/// 下端吸着キャラ窓の接地点の記録（要件 5.3）。
///
/// # 差は本レコードが計算する
///
/// `diff` を呼出側に計算させると、観測（行）と判定（`transition_judge` の `ground_diff_max`）が
/// 別々の引き算を持つことになり、片方だけが符号や基準を変えたときに静かに食い違う。
/// **`ground_y − wa_bottom`（負＝浮いている・正＝沈んでいる）**をここ 1 箇所で決める。
#[derive(Clone, Copy, Debug)]
pub struct GroundRecord {
    /// 刻印。
    pub stamp: Stamp,
    /// スコープ番号（キャラ窓 marker が無ければ `None`＝番兵）。
    pub scope: Option<usize>,
    /// 接地点＝書き込んだ窓矩形の下端（物理 px）。
    pub ground_y: i32,
    /// 作業領域の下端（物理 px）。解決できなければ `None`（下端も差も番兵）。
    ///
    /// `0` へ潰さないのは、「下端が 0 だった」と「解決できなかった」を同じ字面にしない
    /// ためである（[`base::FlushRecord::total_us`] と同じ規律）。
    pub wa_bottom: Option<i32>,
    /// 接地点を書いた経路。
    pub route: PlacementRoute,
}

/// 連鎖再解決の記録。
#[derive(Clone, Copy, Debug)]
#[allow(dead_code)] // 発行側は task 5.6（遷移後の連鎖再解決）
pub struct ChainRecord {
    /// 刻印。
    pub stamp: Stamp,
    /// 段階（[`CHAIN_STAGE_ALL`] のいずれか）。
    pub stage: &'static str,
    /// 対象スコープ数。
    pub scopes: usize,
    /// 実際に動かしたスコープ数。
    pub moved: usize,
    /// 見送りの理由（見送り以外は `None`＝番兵）。
    pub reason: Option<&'static str>,
}

// ---------------------------------------------------------------------------
// フィールドの表現（番兵はここだけが作る）
// ---------------------------------------------------------------------------

/// 省略可能な値を表現へ（不在は番兵）。
fn opt_field<T: std::fmt::Display>(value: Option<T>) -> String {
    match value {
        Some(v) => v.to_string(),
        None => MISSING.to_string(),
    }
}

// ---------------------------------------------------------------------------
// レコード純関数（時刻を読まない・ログに触れない）
// ---------------------------------------------------------------------------

/// 作業領域源の同期の記録行。
///
/// モニタは `m<i>=<dpi>:<l,t,r,b>` の形で 1 台 1 フィールドに畳む——台数が可変でも
/// フィールド名が衝突せず（`m0`／`m1`／…）、値に空白が入らないので `名前=値` の
/// 辞書化規則をそのまま通せる。
#[allow(dead_code)] // 発行側は task 5.1
pub fn snapshot_line(record: &SnapshotRecord<'_>) -> String {
    let mut line = format!(
        "{prefix} {FIELD_MONITORS}={count}",
        prefix = base::record_prefix(record.stamp, KIND_SNAPSHOT),
        count = record.monitors.len(),
    );
    for (index, monitor) in record.monitors.iter().enumerate() {
        let wa = monitor.work_area;
        line.push_str(&format!(
            " m{index}={dpi}:{left},{top},{right},{bottom}",
            dpi = monitor.dpi,
            left = wa.left,
            top = wa.top,
            right = wa.right,
            bottom = wa.bottom,
        ));
    }
    line
}

/// 整合待ちの記録行。
#[allow(dead_code)] // 発行側は task 5.4
pub fn hold_line(record: &HoldRecord) -> String {
    format!(
        "{prefix} {FIELD_ENTITY}={entity:?} {FIELD_SCOPE}={scope} {FIELD_WIN_KIND}={win_kind} \
         {FIELD_WINDOW_DPI}={window_dpi} {FIELD_TABLE_DPI}={table_dpi} \
         {FIELD_SINCE_FRAME}={since_frame} {FIELD_DECISION}={decision} {FIELD_SITE}={site}",
        prefix = base::record_prefix(record.stamp, KIND_HOLD),
        entity = record.entity,
        scope = opt_field(record.scope),
        win_kind = record.win_kind,
        window_dpi = record.window_dpi,
        table_dpi = opt_field(record.table_dpi),
        since_frame = record.since_frame,
        decision = record.decision,
        site = record.site,
    )
}

/// 接地点の記録行。差は `ground_y − wa_bottom`（下端が解決できなければ差も番兵）。
pub fn ground_line(record: &GroundRecord) -> String {
    let diff = record
        .wa_bottom
        .map(|bottom| record.ground_y.saturating_sub(bottom));
    format!(
        "{prefix} {FIELD_SCOPE}={scope} {FIELD_GROUND_Y}={ground_y} {FIELD_WA_BOTTOM}={wa_bottom} \
         {FIELD_DIFF}={diff} {FIELD_ROUTE}={route}",
        prefix = base::record_prefix(record.stamp, KIND_GROUND),
        scope = opt_field(record.scope),
        ground_y = record.ground_y,
        wa_bottom = opt_field(record.wa_bottom),
        diff = opt_field(diff),
        route = record.route.as_str(),
    )
}

/// 連鎖再解決の記録行。
#[allow(dead_code)] // 発行側は task 5.6
pub fn chain_line(record: &ChainRecord) -> String {
    format!(
        "{prefix} {FIELD_STAGE}={stage} {FIELD_SCOPES}={scopes} {FIELD_MOVED}={moved} \
         {FIELD_REASON}={reason}",
        prefix = base::record_prefix(record.stamp, KIND_CHAIN),
        stage = record.stage,
        scopes = record.scopes,
        moved = record.moved,
        reason = opt_field(record.reason),
    )
}

// ---------------------------------------------------------------------------
// World からの転写（刻印・書込タグ）
// ---------------------------------------------------------------------------

/// World 資源（`FrameCount`＋`TickStart`）から刻印を組む。
///
/// # スレッド局所ミラーを読まない（D1）
///
/// 配置側の発行点（`resize_window_to`／`enqueue_window_set_pos`／後続の各相）はいずれも
/// `&mut World` を受け取る**World を借りられる観測点**である。[`base::stamp`]（スレッド局所
/// ミラー）を読んでよいのは World を借りられない点（一括 flush・wndproc）だけであり、
/// 既定の多スレッド実行器ではワーカースレッドから UI スレッドの写しが見えない
/// （`frame=0` の行が出る）。資源から組めば、どのスレッドに載っても同一 tick の全レコードが
/// 同じ `frame` を持つ。
///
/// # 資源が無い World（tick 前・テスト World）
///
/// `EcsWorld::new` 以外で組まれた World には両資源が無い。その場合は `frame=0`・`t_us=0` を
/// 返す——[`base::current_frame`] が tick 前に `0` を返すのと同じ意味であり、値を捏造しない。
pub fn stamp_of(world: &World) -> Stamp {
    match (
        world.get_resource::<FrameCount>(),
        world.get_resource::<TickStart>(),
    ) {
        (Some(frame), Some(tick_start)) => base::stamp_from_world(frame, tick_start),
        _ => Stamp { frame: 0, t_us: 0 },
    }
}

/// 窓書込指令に載せる要求語彙タグを World から組む（要件 2.1）。
///
/// - `origin`: 書込を要求した経路（[`PlacementRoute::as_str`]）。経路語彙を持たない書込
///   （ドラッグ）は [`MISSING`]——実在する要求元を持たない書込と「タグを付け忘れた経路」を
///   同じ字面にすることになるが、ドラッグは wintf の `[drag]` target が所有する（`diag.rs`
///   の `route: None` の doc）ため、areka 側には名乗る語が無い。
/// - `scope`／`kind`: キャラ窓・バルーン窓の marker から読む。placement が生成した窓でなければ
///   種別を発明せず番兵にする（[`super::follow`] の窓移動レコードと同じ流儀）。
///
/// 前置ガードを掛けないのは、本関数が確保を 1 バイトも行わない（`&'static str` 2 つと
/// `Option<u32>`）ためである。窓書込そのものが遷移でしか起きない（要件 10.6＝定常フレームの
/// 窓書込ゼロ）ので、既定運転での費用は 0 に留まる。
pub fn write_tag(world: &World, window: Entity, route: Option<PlacementRoute>) -> WriteTag {
    let identity = world
        .get::<CharWindowMarker>(window)
        .map(|m| (WindowKind::Char, m.scope))
        .or_else(|| {
            world
                .get::<BalloonWindowMarker>(window)
                .map(|m| (WindowKind::Balloon, m.scope))
        });
    WriteTag {
        origin: route.map_or(MISSING, PlacementRoute::as_str),
        // スコープ番号は実運用で 1 桁だが、`u32` へ収まらない値を黙って丸めない
        // （収まらなければ番兵＝「読めなかった」として出す）。
        scope: identity.and_then(|(_, scope)| u32::try_from(scope).ok()),
        kind: identity.map_or(MISSING, |(kind, _)| kind.as_str()),
    }
}

// ---------------------------------------------------------------------------
// 接地点レコードの発行
// ---------------------------------------------------------------------------

/// **実行時のモニタ表**から、窓矩形の中心が属するモニタの作業領域下端を引く。
///
/// # なぜ作業領域源（[`MonitorSnapshot`] 資源）ではなく実行時の表を読むのか
///
/// 要件 5.3 が求めるのは「遷移後の接地点と**作業領域下端**の差」である。接地点は
/// [`MonitorSnapshot`] から導出されるので、同じ源から下端を引いたら差は定義上つねに 0 に
/// なり、**この観測は何も観測しない**。現在の欠陥はまさに「作業領域源が起動時のまま更新
/// されない」ことであり（`main.rs` の起動シーム・`follow/work_area.rs` の「セッション内固定」）、
/// 差を意味あるものにするには 2 つ目の源＝実行時のモニタ表と突き合わせるほかない。
///
/// # 位置の決め方は 1 bit も変えない
///
/// 本関数は**観測専用**である。接地点そのものは従来どおり [`MonitorSnapshot`] を読む
/// `project_anchor` が決める（design Allowed Dependencies の禁止項「`MonitorSnapshot` の
/// 消費者を wintf `Monitor` 直読へ変えること」は位置権威の話であり、ここには掛からない）。
/// task 5.1 が作業領域源を実行時同期すれば 2 つの源は一致し、本レコードの差は 0 になる
/// ——以後は「源が再び古びていないか」を常時見張る口として残る。
///
/// 帰属規則は [`work_area_for_window`] をそのまま使う（別規則を発明しない）。
/// モニタ 0 台・表そのものが無い World は `None`＝架空の矩形を発明しない。
pub fn live_work_area_bottom(world: &mut World, window: RectPx) -> Option<i32> {
    let mut monitors = world.query::<&Monitor>();
    let work_areas: Vec<RectPx> = monitors
        .iter(world)
        .map(|monitor| RectPx {
            left: monitor.work_area.left,
            top: monitor.work_area.top,
            right: monitor.work_area.right,
            bottom: monitor.work_area.bottom,
        })
        .collect();
    if work_areas.is_empty() {
        return None;
    }
    work_area_for_window(&MonitorSnapshot { work_areas }, window).map(|wa| wa.bottom)
}

/// 接地点レコードを 1 行出す（下端吸着のキャラ窓を書いた直後の観測点）。
///
/// `pos`／`size` は再射影が決めた位置と寸。**バルーンの表示位置補正（`windowposition-limit`
/// の関門）より前**の値だが、当該関門は `BalloonLimit(true)` の窓にしか作用せず、キャラ窓は
/// それを持たない（`follow_balloon_limit_tests.rs:246` が固定）ので、キャラ窓については
/// 書き込んだ値と一致する。接地点は `pos.y + size.h`
/// ＝窓矩形の下端であり、キャラ窓の原点（下端中央）の Y 成分そのものである（要件 10.1 の
/// 原点規約は読むだけで変えない）。
///
/// 呼出側は [`is_enabled`] で前置ガードすること（本関数は行を組む＝確保する）。
pub fn log_char_ground(
    world: &mut World,
    char_window: Entity,
    pos: PointPx,
    size: SizePx,
    route: PlacementRoute,
) {
    let ground_y = pos.y.saturating_add(size.h);
    let wa_bottom = live_work_area_bottom(
        world,
        RectPx {
            left: pos.x,
            top: pos.y,
            right: pos.x.saturating_add(size.w),
            bottom: ground_y,
        },
    );
    let record = GroundRecord {
        stamp: stamp_of(world),
        scope: world.get::<CharWindowMarker>(char_window).map(|m| m.scope),
        ground_y,
        wa_bottom,
        route,
    };
    base::emit_line(&ground_line(&record));
}

#[cfg(test)]
#[path = "transition_diag_tests.rs"]
mod transition_diag_tests;

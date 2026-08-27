//! 配置判断の**遷移観測レコード**（`kind=snapshot`／`hold`／`ground`／`chain`／`offset`）
//! ——語彙・レコード純関数・World からの転写口。
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
//! [transition] frame=<u32> t_us=<u64> kind=offset scope=<s> base_dpi=<d> new_dpi=<d> base_offset=<x,y> old_offset=<x,y> new_offset=<x,y> verdict=<v>
//! ```
//!
//! 接頭語（`frame`／`t_us`／`kind`）は [`base::record_prefix`] が組む。段階ごとに意味を持たない
//! フィールドは**落とさず**番兵 [`base::MISSING`]（`-`）で埋める——落とすと「記録が出ていない」と
//! 「その経路にはその値が無い」の区別が事後に付かない（wintf C1・emo-present C3 と同じ規律）。
//!
//! # 語彙の定義元
//!
//! 共有のフィールド名（`entity`／`scope`／`win_kind`／`stage`）は wintf が単一の定義元であり、
//! ここは**参照するだけ**である。areka が足すのは 5 つの種別語と、この 5 種にしか出ない
//! フィールド名・判定語だけ（wintf は上位 crate の語彙を持たない＝依存方向 wintf ← areka）。
//! `new_dpi` は wintf の `kind=monitor` と**同じ意味・同じ値の形**（表示 DPI の `dpi_x`）で
//! 使うため、欄名は [`base::FIELD_NEW_DPI`] を参照する（同じ語を二重に定義しない）。
//!
//! # 発行点
//!
//! 本ファイルは task 2.4（観測の増設）で建てた語彙であり、実際に到達できる発行点は 4 つ——
//! [`log_char_ground`]（`resize_window_to` から）・[`log_monitor_snapshot_sync`]（作業領域源の
//! 同期＝task 5.1 から）・[`log_hold`]（整合ゲートの 4 点＝task 5.4 の 3 点＋task 6.5 の 4 点目）・[`log_chain`]
//! （遷移後の連鎖再解決＝task 5.6 から）である。5 種目の [`log_offset_rescale`]
//! （拡大率遷移でのバルーン追従オフセットの追随＝`areka-P0-balloon-offset-dpi` の
//! 要件 3.7・design D10）は**語彙が先着**しており、発行する適用相は同 spec の後続タスクが
//! 建てる。
//!
//! # 語彙は先に建てる（未消費の `#[allow(dead_code)]` の根拠）
//!
//! 判定語の一覧（`*_ALL`）と per-kind の必須フィールド列（`*_FIELDS`）は、**判定器
//! （task 3.1／3.2）と発行側が同じ字面を見る**ための単一の定義元である。判定器は
//! `#[cfg(test)]` 限定の配置ゆえ、非 test ビルドでは未消費に見える
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
    self as base, FIELD_ENTITY, FIELD_NEW_DPI, FIELD_SCOPE, FIELD_STAGE, FIELD_WIN_KIND, MISSING,
    Stamp, TickStart, WriteTag,
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
/// 拡大率遷移での**バルーン追従オフセットの追随**（`areka-P0-balloon-offset-dpi` 要件 3.7）。
pub const KIND_OFFSET: &str = "offset";

/// areka が発行する `kind` 語の全体（判定側の語彙照合・語の一意性テストが参照する）。
///
/// wintf の [`base::KIND_ALL`] とも emo-present の `KIND_SURFACE` とも交わらない
/// （交わると判定側のレコード振り分けと遷移の起点判定が壊れる）。
#[allow(dead_code)] // 語彙先着（module doc「語彙は先に建てる」）
pub const PLACEMENT_KIND_ALL: &[&str] = &[
    KIND_SNAPSHOT,
    KIND_HOLD,
    KIND_GROUND,
    KIND_CHAIN,
    KIND_OFFSET,
];

// ---------------------------------------------------------------------------
// 整合待ちの判定語（decision）と観測点語（site）
// ---------------------------------------------------------------------------

/// 表と一致（または表を引けない）ため、そのまま処理する。
pub const HOLD_DECISION_PROCEED: &str = "proceed";
/// 不一致ゆえ当該窓の窓書込を見送る。
pub const HOLD_DECISION_HOLD: &str = "hold";
/// 上限フレームを超えたので警告の上で処理する。
pub const HOLD_DECISION_PROCEED_AFTER_TIMEOUT: &str = "proceed-after-timeout";

/// 整合待ちの判定語の全体。
#[allow(dead_code)] // 語彙先着（module doc「語彙は先に建てる」）
pub const HOLD_DECISION_ALL: &[&str] = &[
    HOLD_DECISION_PROCEED,
    HOLD_DECISION_HOLD,
    HOLD_DECISION_PROCEED_AFTER_TIMEOUT,
];

/// 判定を下した観測点: 拡大率の相。
pub const HOLD_SITE_DPI: &str = "dpi";
/// 判定を下した観測点: 報告寸の突合。
pub const HOLD_SITE_RECONCILE: &str = "reconcile";
/// 判定を下した観測点: **実表示寸**の再スナップ（`resnap_shell_targets`＝表示側の物理寸が
/// 窓寸と食い違う窓を書き直す点）。
pub const HOLD_SITE_RESNAP: &str = "resnap";
/// 判定を下した観測点: **作業領域変化を契機とする**再スナップ（`resnap_for_work_area_change`
/// ＝作業領域源が差し替わったフレームに現寸のまま接地点を引き直す点）。
///
/// [`HOLD_SITE_RESNAP`] と別語なのは、日本語の「再スナップ」が別々の 2 関数を指すからである
/// ——同じ語にすると、ログ上でどちらの点が見送ったのかが判らない（task 6.5）。
pub const HOLD_SITE_WORK_AREA_RESNAP: &str = "work-area-resnap";

/// 観測点語の全体（design C5 の「4 点すべて」＝待ち札の守備範囲そのもの）。
#[allow(dead_code)] // 語彙先着（module doc「語彙は先に建てる」）
pub const HOLD_SITE_ALL: &[&str] = &[
    HOLD_SITE_DPI,
    HOLD_SITE_RECONCILE,
    HOLD_SITE_RESNAP,
    HOLD_SITE_WORK_AREA_RESNAP,
];

// ---------------------------------------------------------------------------
// 連鎖再解決の段階語（stage）
// ---------------------------------------------------------------------------

/// 遷移を検知して解き直しを武装した。
pub const CHAIN_STAGE_ARMED: &str = "armed";
/// 解き直しを実行した。
pub const CHAIN_STAGE_REALIGNED: &str = "realigned";
/// 条件未達で見送った。
pub const CHAIN_STAGE_DEFERRED: &str = "deferred";

/// 連鎖レコードの段階語の全体。
#[allow(dead_code)] // 語彙先着（module doc「語彙は先に建てる」）
pub const CHAIN_STAGE_ALL: &[&str] = &[
    CHAIN_STAGE_ARMED,
    CHAIN_STAGE_REALIGNED,
    CHAIN_STAGE_DEFERRED,
];

// ---------------------------------------------------------------------------
// 追随の判定語（verdict）
// ---------------------------------------------------------------------------

/// 追随した——基準対から引き直した値を反映した（要件 3.1）。
pub const OFFSET_VERDICT_RESCALED: &str = "rescaled";
/// 未係留の基準を現在の表示 DPI へ係留した——**値は変えていない**（要件 5.2）。
pub const OFFSET_VERDICT_ANCHORED: &str = "anchored";
/// 基準 DPI と現在 DPI が同一——値も基準も変えていない（要件 3.3 の bit 同一）。
pub const OFFSET_VERDICT_UNCHANGED: &str = "unchanged";
/// キーワード由来の基本位置の素材が未消費のため追随を見送った（要件 4.3・design D7）。
///
/// **縮退ではない**ので警告を伴わない。それでも語を持つのは、開発者裁定（2026-08-27）が
/// 受容した残余——素材が未消費のまま寸据え置きの遷移を迎えると、揃えの更新が次の寸法変化まで
/// 取り残される——が**記録に残って沈黙しない**ことを、この語 1 つが担っているからである。
pub const OFFSET_VERDICT_KEYWORD_PENDING: &str = "keyword-pending";
/// 拡大率を解決できない——値も基準も変えていない（要件 3.6・警告を伴う）。
pub const OFFSET_VERDICT_UNRESOLVED: &str = "unresolved";
/// 追随したが `i32` 域を超えて飽和した（回り込ませていない・要件 2.5 と同型・警告を伴う）。
pub const OFFSET_VERDICT_SATURATED: &str = "saturated";

/// 追随の判定語の全体（6 値・判定側の語彙照合が参照する単一の定義元）。
///
/// 実機サインオフの機械判定（要件 8.3）は**この定数群を参照するだけ**で、字面のリテラルを
/// 自前で書かない——語を変えたときに片方だけが動く食い違いを構造で潰す。
#[allow(dead_code)] // 語彙先着（module doc「語彙は先に建てる」）
pub const OFFSET_VERDICT_ALL: &[&str] = &[
    OFFSET_VERDICT_RESCALED,
    OFFSET_VERDICT_ANCHORED,
    OFFSET_VERDICT_UNCHANGED,
    OFFSET_VERDICT_KEYWORD_PENDING,
    OFFSET_VERDICT_UNRESOLVED,
    OFFSET_VERDICT_SATURATED,
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
/// 基準対が属する表示 DPI（**未係留**＝永続値の腕は番兵）。
pub const FIELD_BASE_DPI: &str = "base_dpi";
/// 基準対の値（キャラ窓左上相対・物理 px・`x,y`）。
pub const FIELD_BASE_OFFSET: &str = "base_offset";
/// 追随を適用する**前**の追従オフセット（物理 px・`x,y`）。
pub const FIELD_OLD_OFFSET: &str = "old_offset";
/// 追随を適用した**後**の追従オフセット（物理 px・`x,y`）。
pub const FIELD_NEW_OFFSET: &str = "new_offset";
/// 追随の判定（[`OFFSET_VERDICT_ALL`] のいずれか）。
pub const FIELD_VERDICT: &str = "verdict";

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

/// `kind=offset` 行の必須フィールド（接頭語を除く）。
#[allow(dead_code)] // 語彙先着（module doc「語彙は先に建てる」）
pub const OFFSET_FIELDS: &[&str] = &[
    FIELD_SCOPE,
    FIELD_BASE_DPI,
    FIELD_NEW_DPI,
    FIELD_BASE_OFFSET,
    FIELD_OLD_OFFSET,
    FIELD_NEW_OFFSET,
    FIELD_VERDICT,
];

// ---------------------------------------------------------------------------
// レコード
// ---------------------------------------------------------------------------

/// 作業領域源に載るモニタ 1 台ぶん（拡大率＋作業領域）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MonitorEntry {
    /// モニタの拡大率。
    pub dpi: u32,
    /// 作業領域（物理 px）。
    pub work_area: RectPx,
}

/// 作業領域源を作り直した記録。
#[derive(Clone, Copy, Debug)]
pub struct SnapshotRecord<'a> {
    /// 刻印。
    pub stamp: Stamp,
    /// 同期後のモニタ列（列挙順）。**0 台は空スライス**＝`monitors=0` として観測できる。
    pub monitors: &'a [MonitorEntry],
}

/// 整合待ちの判定 1 件の記録。
#[derive(Clone, Copy, Debug)]
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

/// 拡大率遷移でのバルーン追従オフセットの追随 1 件の記録（要件 3.7）。
///
/// # 基準・前・後の 3 つを載せる
///
/// 追随は**基準対から毎回引き直す**（design D4）ため、`new_offset` を再現できるのは
/// `base_offset` と 2 つの DPI であって `old_offset` ではない。一方 `old_offset` を落とすと
/// 「実際に値が動いたか」が事後に判らない。3 つとも載せるのは、判定側が**引き直しの再現**と
/// **動いたかの判定**をどちらも行うためである。
#[allow(dead_code)] // 語彙先着（module doc「語彙は先に建てる」）
#[derive(Clone, Copy, Debug)]
pub struct OffsetRecord {
    /// 刻印。
    pub stamp: Stamp,
    /// スコープ番号（キャラ窓 marker が無ければ `None`＝番兵）。
    pub scope: Option<u32>,
    /// 基準対が属する表示 DPI（`dpi_x`）。**未係留**は `None`＝番兵——`0` へ潰すと
    /// 「未係留」と「0 を観測した」が同じ字面になる（[`GroundRecord::wa_bottom`] と同じ規律）。
    pub base_dpi: Option<u32>,
    /// 遷移後の表示 DPI（`dpi_x`）。`kind=monitor` の同名欄と同じ意味・同じ値の形。
    pub new_dpi: u32,
    /// 基準対の値（物理 px）。
    pub base_offset: PointPx,
    /// 追随前の追従オフセット（物理 px）。
    pub old_offset: PointPx,
    /// 追随後の追従オフセット（物理 px・値が動かない腕では [`Self::old_offset`] と同一）。
    pub new_offset: PointPx,
    /// 判定（[`OFFSET_VERDICT_ALL`] のいずれか）。
    pub verdict: &'static str,
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

/// 点を 1 フィールドへ畳む（`x,y`）。
///
/// 値に空白を入れないのは、判定側の `名前=値` の切り出し（空白区切り）をそのまま通すため
/// である（[`snapshot_line`] の `m<i>=<dpi>:<l,t,r,b>` と同じ流儀）。
#[allow(dead_code)] // 語彙先着（module doc「語彙は先に建てる」）
fn point_field(point: PointPx) -> String {
    format!("{x},{y}", x = point.x, y = point.y)
}

// ---------------------------------------------------------------------------
// レコード純関数（時刻を読まない・ログに触れない）
// ---------------------------------------------------------------------------

/// 作業領域源の同期の記録行。
///
/// モニタは `m<i>=<dpi>:<l,t,r,b>` の形で 1 台 1 フィールドに畳む——台数が可変でも
/// フィールド名が衝突せず（`m0`／`m1`／…）、値に空白が入らないので `名前=値` の
/// 辞書化規則をそのまま通せる。
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

/// 追随の記録行。未係留の基準 DPI と marker の無い窓は番兵で埋める（欄を落とさない）。
#[allow(dead_code)] // 語彙先着（module doc「語彙は先に建てる」）
pub fn offset_line(record: &OffsetRecord) -> String {
    format!(
        "{prefix} {FIELD_SCOPE}={scope} {FIELD_BASE_DPI}={base_dpi} {FIELD_NEW_DPI}={new_dpi} \
         {FIELD_BASE_OFFSET}={base_offset} {FIELD_OLD_OFFSET}={old_offset} \
         {FIELD_NEW_OFFSET}={new_offset} {FIELD_VERDICT}={verdict}",
        prefix = base::record_prefix(record.stamp, KIND_OFFSET),
        scope = opt_field(record.scope),
        base_dpi = opt_field(record.base_dpi),
        new_dpi = record.new_dpi,
        base_offset = point_field(record.base_offset),
        old_offset = point_field(record.old_offset),
        new_offset = point_field(record.new_offset),
        verdict = record.verdict,
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
    let (scope, kind) = window_identity(world, window);
    WriteTag {
        origin: route.map_or(MISSING, PlacementRoute::as_str),
        scope,
        kind,
    }
}

/// 窓の marker から `(scope, win_kind)` を読む（placement 生成の窓でなければ番兵）。
///
/// スコープ番号は実運用で 1 桁だが、`u32` へ収まらない値を黙って丸めない
/// （収まらなければ番兵＝「読めなかった」として出す）。
fn window_identity(world: &World, window: Entity) -> (Option<u32>, &'static str) {
    let identity = world
        .get::<CharWindowMarker>(window)
        .map(|m| (WindowKind::Char, m.scope))
        .or_else(|| {
            world
                .get::<BalloonWindowMarker>(window)
                .map(|m| (WindowKind::Balloon, m.scope))
        });
    (
        identity.and_then(|(_, scope)| u32::try_from(scope).ok()),
        identity.map_or(MISSING, |(kind, _)| kind.as_str()),
    )
}

// ---------------------------------------------------------------------------
// 整合待ちレコードの発行
// ---------------------------------------------------------------------------

/// 整合待ちの記録を 1 行出す（拡大率の相・報告寸の突合・実表示寸の再スナップ・作業領域変化を
/// 契機とする再スナップの 4 点が呼ぶ）。
///
/// 呼出側は [`is_enabled`] で前置ガードすること（本関数は行を組む＝確保する）。判定語
/// （`decision`）と観測点語（`site`）は本モジュールの定数が単一の定義元であり、呼出側は
/// enum からその定数を引いて渡す——字面をリテラルで持たせない。
pub fn log_hold(
    world: &World,
    window: Entity,
    window_dpi: u32,
    table_dpi: Option<u32>,
    since_frame: u32,
    decision: &'static str,
    site: &'static str,
) {
    let (scope, win_kind) = window_identity(world, window);
    let record = HoldRecord {
        stamp: stamp_of(world),
        entity: window,
        scope,
        win_kind,
        window_dpi,
        table_dpi,
        since_frame,
        decision,
        site,
    };
    base::emit_line(&hold_line(&record));
}

// ---------------------------------------------------------------------------
// 作業領域源レコードの発行
// ---------------------------------------------------------------------------

/// 作業領域源を作り直した記録を 1 行出す（同期段が実際に**差し替えた**フレームだけ）。
///
/// 呼出側は [`is_enabled`] で前置ガードすること（本関数は行を組む＝確保する）。
/// 同じ表で差し替えが起きなかったフレームでは呼ばれない——「毎フレーム出る行」にすると、
/// 判定側が遷移を切り出すときの雑音になるうえ、定常状態の確保ゼロという契約も壊れる。
pub fn log_monitor_snapshot_sync(world: &World, monitors: &[MonitorEntry]) {
    let record = SnapshotRecord {
        stamp: stamp_of(world),
        monitors,
    };
    base::emit_line(&snapshot_line(&record));
}

// ---------------------------------------------------------------------------
// 連鎖再解決レコードの発行
// ---------------------------------------------------------------------------

/// 連鎖再解決の記録を 1 行出す（武装・解き直し・見送りの 3 段階が呼ぶ）。
///
/// 呼出側は [`is_enabled`] で前置ガードすること（本関数は行を組む＝確保する）。段階語
/// （`stage`）と見送り理由（`reason`）はそれぞれ本モジュールの定数と
/// [`ChainDeferReason::as_str`](super::chain_finalize::ChainDeferReason::as_str) が単一の
/// 定義元であり、呼出側は字面をリテラルで持たない。
///
/// `reason` は見送り以外では `None`＝番兵で埋める（落とすと「記録が出ていない」と
/// 「その段階にはその値が無い」の区別が事後に付かない）。
pub fn log_chain(
    world: &World,
    stage: &'static str,
    scopes: usize,
    moved: usize,
    reason: Option<&'static str>,
) {
    let record = ChainRecord {
        stamp: stamp_of(world),
        stage,
        scopes,
        moved,
        reason,
    };
    base::emit_line(&chain_line(&record));
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
/// なり、**この観測は何も観測しない**。是正前の欠陥はまさに「作業領域源が起動時のまま更新
/// されない」ことであり（確定台帳 L3）、差を意味あるものにするには 2 つ目の源＝実行時の
/// モニタ表と突き合わせるほかない。
///
/// # 位置の決め方は 1 bit も変えない
///
/// 本関数は**観測専用**である。接地点そのものは従来どおり [`MonitorSnapshot`] を読む
/// `project_anchor` が決める（design Allowed Dependencies の禁止項「`MonitorSnapshot` の
/// 消費者を wintf `Monitor` 直読へ変えること」は位置権威の話であり、ここには掛からない）。
/// task 5.1（作業領域源の実行時同期）が着地したので 2 つの源は通常一致し、本レコードの差は
/// 0 になる——**この読み取りは撤去しない**。同期段が止まる・取りこぼす・順序が入れ替わる、
/// のいずれでも差が再び 0 でなくなる形で見える、源の陳腐化を常時見張る口だからである
/// （同じ源から引いたら差は定義上つねに 0 で、何も見張らなくなる）。
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

// ---------------------------------------------------------------------------
// 追随レコードの発行
// ---------------------------------------------------------------------------

/// 拡大率遷移でのバルーン追従オフセットの追随を 1 行出す（要件 3.7・design D10）。
///
/// # 1 遷移・1 スコープにつき高々 1 行
///
/// 本関数は 1 呼出につき 1 行だけを出す。判定語は腕ごとに 1 つ（[`OFFSET_VERDICT_ALL`]）で
/// あり、追随の適用相はキャラ窓 1 つにつき 1 度だけ判定を下してその結果を 1 語として渡す。
/// 腕ごとに別々の行を出す形にしないのは、判定側が「遷移 1 回ぶんの行」を数えて突合する
/// からである——複数行になると、どれが最終の判定かが事後に判らない。
///
/// # 前置ガードは**本関数が持つ**
///
/// 行の組立（`String` の確保）へ入る前に [`is_enabled`] で抜ける。呼出側に委ねないのは、
/// 引数がすべて Copy のスカラーで**確保を伴わない**からである——ガードを内側に置いても
/// 既定運転の費用は 0 のまま、書き忘れの余地だけが消える（`dpi_sync` の整合待ちの発行口と
/// 同じ形）。`debug!` は既定で濾過されるため、ガードを失っても**出力は変わらない**＝
/// 濾過テストでは検出できない退行であり、固定するのは `transition_diag_tests.rs` の
/// 本文走査である。
///
/// 判定語は本モジュールの `pub const` が単一の定義元であり、呼出側は字面をリテラルで
/// 持たない（語彙表に無い字面はその場で `debug_assert!` が落とす）。
#[allow(dead_code)] // 語彙先着（module doc「語彙は先に建てる」）
pub fn log_offset_rescale(
    world: &World,
    scope: Option<u32>,
    base_dpi: Option<u32>,
    new_dpi: u32,
    base_offset: PointPx,
    old_offset: PointPx,
    new_offset: PointPx,
    verdict: &'static str,
) {
    debug_assert!(
        OFFSET_VERDICT_ALL.contains(&verdict),
        "語彙表に無い判定語が渡された: {verdict}"
    );
    if !is_enabled() {
        return;
    }
    let record = OffsetRecord {
        stamp: stamp_of(world),
        scope,
        base_dpi,
        new_dpi,
        base_offset,
        old_offset,
        new_offset,
        verdict,
    };
    base::emit_line(&offset_line(&record));
}

#[cfg(test)]
#[path = "transition_diag_tests.rs"]
mod transition_diag_tests;

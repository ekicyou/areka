//! 配置観測（診断計装）の語彙・専用 target・レコード組立の単一の住処
//! （areka-P0-dpi-window-vanish design「placement::diag」・要件 1.1／1.2／1.7／2.4）。
//!
//! # 何のためにあるか
//!
//! 混在 DPI マルチモニタ実機でゴースト窓が「知らないうちに消える」事象は、消失の
//! **瞬間が未目撃**である。したがって運転後のログだけから「どのモニタのどこに・どの寸で
//! 居たか」「最後に位置を書いたのは誰か」を事後再構成できる証跡が要る（要件 1.1／1.2）。
//! 本モジュールはその証跡の語彙（[`PlacementRoute`]）・出力先（[`DIAG_TARGET`]）・
//! レコード組立（[`monitor_record_line`]／[`window_move_record_line`]）を 1 箇所に集める。
//!
//! # 恒久計装であって一時計装ではない（要件 1.7）
//!
//! 観測点は診断完了後も本体コードに残す。既定では**無音**であり、診断手順書が指定する
//! `RUST_LOG=…,areka::placement::diag=debug` でのみ点灯する。この「既定 OFF」は
//! **専用 target ＋ `debug!` 水準**の 2 段で担保され、in-source 檻
//! （`diag_records_are_silent_under_default_info_filter` 他）が実際の [`EnvFilter`] 濾過で
//! 機械的に固定する。
//!
//! [`EnvFilter`]: tracing_subscriber::EnvFilter
//!
//! # レコードは純関数が組み、ログはそれを出すだけ
//!
//! 診断手順書の grep 判定語と出力書式は 1:1 で対応する（要件 1.5）。書式が意図せず
//! 変われば手順書が静かに嘘になるため、**組立を純関数に切り出して in-source 檻で
//! リテラル固定**する（本 spec の共通規約「判定は比・不変条件で」の意図された例外＝
//! ここで固定したいのは物理量ではなく手順書との語彙一致そのもの）。
//! [`log_monitor_snapshot`]／[`log_window_move`] は純関数の戻り値をそのまま本文にする
//! （組立を二重実装しない＝檻が本番の出力を本当に固定する）。
//!
//! # 依存方向（design「Allowed Dependencies」の強制規約）
//!
//! 本モジュールは placement の**最下流**であり、`World`・wintf の型に依存しない。
//! 扱うのは素の数値・文字列・[`Entity`] のみで、wintf `Monitor` からの転写は
//! 呼出側（`mod.rs`／`follow.rs`）の仕事である。[`Entity`] だけを例外的に受けるのは、
//! これが **wintf 側ログ（`entity = ?e`）との結合キー**だからで、同じ `Debug` 表現で
//! 出すことが要件 1.9 の scope 別計数（手順書の 2 段 grep）の成立条件になる。

// 配線状況（task 1.4 完了時点）: 本モジュールの API は**全面配線済み**である。
// - モニタスナップショット側（`MonitorRecord`・`monitor_snapshot_header_line`／
//   `monitor_record_line`・`log_monitor_snapshot`）: `main.rs` の `MonitorSnapshot` 構築点
//   （要件 1.1 正典出力点）と `placement::prepare_ghost_windows` の列挙点が呼ぶ。
// - 窓移動レコード側（`WindowKind`・`WindowMoveRecord`・`window_move_record_line`・
//   `log_window_move`・`WINDOW_MOVE_RECORD_TAG`）: `follow.rs` の単一ライター
//   `enqueue_window_set_pos` が書込成功時に呼ぶ。
//
// ゆえに**モジュール大の `#![allow(dead_code)]` は撤去した**（残すと以後の真の dead code を
// 隠すため・tasks.md Implementation Notes「1.4 で全 API を配線したらこの属性を外すこと」）。
// 残る個別 `#[allow(dead_code)]` は 2 箇所だけで、いずれも実在の理由を各所へ明記してある
// （areka は bin crate ゆえ `pub` でも dead_code 免除されない）。

use std::fmt;

use bevy_ecs::entity::Entity;
use tracing::debug;

/// 恒久観測の専用ログ target（design D1: 既定 OFF・診断手順が `RUST_LOG` で点灯）。
///
/// 本モジュールの出力は**すべて**この target を通る。診断手順書の
/// `RUST_LOG=info,…,areka::placement::diag=debug` と 1:1 で対応し、
/// リテラルは in-source 檻が固定する（勝手に変えると手順書が無効化する）。
pub const DIAG_TARGET: &str = "areka::placement::diag";

/// モニタスナップショットの見出し行タグ（grep 判定語・台数がここに出る）。
pub const MONITOR_SNAPSHOT_TAG: &str = "[diag.monitor_snapshot]";

/// モニタ 1 台ぶんのレコード行タグ（grep 判定語）。
pub const MONITOR_RECORD_TAG: &str = "[diag.monitor]";

/// 窓移動レコード行タグ（grep 判定語・要件 1.2 の「経路」つき移動記録）。
pub const WINDOW_MOVE_RECORD_TAG: &str = "[diag.window_move]";

/// 破棄済み（despawn 済み）entity を消費側が**正常系として**打ち切ったことを表す判定語
/// （要件 6.2／6.3・design D8 消費側）。
///
/// # なぜ語彙を 1 箇所に置くのか
///
/// この打ち切りは「entity 不在＝破棄済み（正常終了系・`debug!`）」と「entity は実在するが
/// 接地点規約の component が欠落（真の異常・`warn!`）」を**区別**するために入れたものだが、
/// 区別は水準だけでなく**本文でも**引けなければ終了時ログの読み手に伝わらない。消費入口は
/// 追従層（`resize_window_to`／`resize_window_keep_position`）とフレーム層（再スナップ相・
/// 報告回収相）に分かれて 4 箇所あり、各所が独自の文言を持つと grep 語が静かに分裂する
/// （tasks.md Implementation Notes 1.4「同一文言の複製」の教訓）。ゆえに**共通の接頭辞だけを
/// 本定数が持ち**、後続の説明文は各消費点が自分の相を名乗る形にする。
///
/// # target について（本定数だけの例外）
///
/// 本モジュールの**出力**はすべて [`DIAG_TARGET`] を通るが、本定数を用いる `debug!` は
/// 各消費点の既定 target（`areka::placement::follow`／`areka::emo2_boot::frame`）へ出る——
/// 終了時ログの静穏性（要件 6.2）は診断手順の点灯有無に依らず成立すべきものだからである。
/// 本モジュールが持つのは語彙であって出力点ではない。
pub const DESPAWNED_SKIP_TAG: &str = "[despawn-skip]";

/// 値が取得できなかったフィールドの番兵。
///
/// フィールド自体は落とさない——落とすと経路ごとに grep の当たり方が変わり、
/// 「出ていない」のか「そのフィールドが無い経路だった」のかを事後に区別できなくなる。
const UNKNOWN: &str = "-";

/// 窓位置・窓寸を書き込んだ**経路**（要件 1.2 の「変化を引き起こした経路」＝
/// 要件 2.4 の「最終位置を書き込んだ主体」を名指しするための結論語彙）。
///
/// 消失の診断で最後に要るのは「どこに居たか」ではなく「**誰が最後に書いたか**」である。
/// 位置を書ける経路は 9 つあり、ログ上でそれらが見分けられなければ書き手を名指しできない。
/// 表示語（[`PlacementRoute::as_str`]）は診断手順書の判定語そのものゆえ檻で固定する。
///
/// # 語彙は「実在の書込トリガ」と 1:1 でなければならない（D13）
///
/// 当初の 7 語彙は 2 つの実在トリガを覆えなかった——`reconcile_window_size` の drain 後段からの
/// 呼出（[`ReportedSizeReconcile`](Self::ReportedSizeReconcile)）と `\![move]`
/// （[`MoveCue`](Self::MoveCue)）である。**1 語が 2 トリガを指すと route 名での切り分けが
/// 原理的に不能になる**（DPI 変化ゼロの起動が「DPI 由来」を名乗る偽陽性が毎回出る）ため、
/// 語彙側を完全化して 1:1 を保つ。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PlacementRoute {
    /// spawn 初期配置（`placement::spawn` の窓生成時）。
    // 単一ライターを通らない経路（spawn は entity 組立時に `WindowPos` を直接持たせる）ゆえ
    // 現状どこからも構築されない。design「Service Interface」が定める 9 語の一部であり、
    // D13 帰結⑷「語彙のみ保持・将来の配線先として予約」ゆえ削らない。
    #[allow(dead_code)]
    SpawnInitial,
    /// 位置永続化の復元マージ（可視化保証は position-persist の所有＝可視性ガード適用外）。
    // 同上（復元は spawn 前の `apply_restored_placements` が `ScopePlacement` をマージする）。
    #[allow(dead_code)]
    Restore,
    /// アンカー変化トリガ（`anchor_changed_system`）。
    AnchorChange,
    /// 毎フレーム再スナップ（`resnap_from_sizes`）。
    Resnap,
    /// DPI 変化に伴う位置の再射影（**`dpi_phase` 限定**）。
    ///
    /// drain の後段の報告回収は [`ReportedSizeReconcile`](Self::ReportedSizeReconcile) が担い、
    /// 本変種と混同しない（D13）——両者は `reconcile_window_size` という同一の共通末端を
    /// 通るが、**同一の末端を通ることは同一の経路であることを意味しない**。
    DpiReproject,
    /// バルーン窓の位置据置きリサイズ（`resize_window_keep_position`）。
    KeepPositionResize,
    /// キャラ窓確定後のバルーン随伴（`follow_balloon`）。
    BalloonFollow,
    /// drain の後段の報告回収（`reconcile_reported_sizes`・**初回表示の k₀ 補正を含む**）。
    ///
    /// `Changed<DPI>` **非依存**＝DPI 変化ゼロでも発火し得るため
    /// [`DpiReproject`](Self::DpiReproject) と別語にする（D13）。混同すると要件 1.9 の
    /// 受理回数突合（セッション②）に偽陽性が混入する。
    ReportedSizeReconcile,
    /// `\![move]` cue によるスクリプト明示移動（`move_window_to` の**対象窓**）。
    ///
    /// 明示操作の尊重ゆえ遷移ガード適用外＝ドラッグ・[`Restore`](Self::Restore) と同族（D13）。
    /// 随伴バルーン側の書込は [`BalloonFollow`](Self::BalloonFollow) を持つ。
    MoveCue,
}

impl PlacementRoute {
    /// 全経路（檻が語彙の網羅と一意性を固定するための単一の列）。
    ///
    /// バリアントを増やしたら[`as_str`](Self::as_str) の網羅 match がコンパイルを止め、
    /// ここへの追加漏れは檻（`placement_route_all_covers_nine_distinct_variants`）が捕まえる。
    // 消費者は in-source 檻（`#[cfg(test)]`）だけゆえ本番ビルドでは dead。檻専用の列である
    // ことが存在理由そのもの（語彙の網羅・一意性の単一の錨）ゆえ本属性を残す。
    #[allow(dead_code)]
    pub const ALL: [PlacementRoute; 9] = [
        PlacementRoute::SpawnInitial,
        PlacementRoute::Restore,
        PlacementRoute::AnchorChange,
        PlacementRoute::Resnap,
        PlacementRoute::DpiReproject,
        PlacementRoute::KeepPositionResize,
        PlacementRoute::BalloonFollow,
        PlacementRoute::ReportedSizeReconcile,
        PlacementRoute::MoveCue,
    ];

    /// ログ上の表示語（診断手順書の grep 判定語と 1:1）。
    pub const fn as_str(self) -> &'static str {
        match self {
            PlacementRoute::SpawnInitial => "SpawnInitial",
            PlacementRoute::Restore => "Restore",
            PlacementRoute::AnchorChange => "AnchorChange",
            PlacementRoute::Resnap => "Resnap",
            PlacementRoute::DpiReproject => "DpiReproject",
            PlacementRoute::KeepPositionResize => "KeepPositionResize",
            PlacementRoute::BalloonFollow => "BalloonFollow",
            PlacementRoute::ReportedSizeReconcile => "ReportedSizeReconcile",
            PlacementRoute::MoveCue => "MoveCue",
        }
    }
}

impl fmt::Display for PlacementRoute {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// ゴースト窓の種別（キャラ窓／バルーン窓）。
///
/// 手順書の 2 段 grep は「scope→キャラ窓 entity」の対応表をこの語で選別して作る
/// （バルーン窓の entity を混ぜると DPI 受理回数の計数が壊れる・要件 1.9）。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WindowKind {
    /// キャラ窓（`CharWindowMarker` を持つ窓）。
    Char,
    /// バルーン窓（`BalloonWindowMarker` を持つ窓）。
    Balloon,
}

impl WindowKind {
    /// ログ上の表示語（grep 判定語）。
    pub const fn as_str(self) -> &'static str {
        match self {
            WindowKind::Char => "char",
            WindowKind::Balloon => "balloon",
        }
    }
}

impl fmt::Display for WindowKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// モニタ 1 台ぶんの観測レコード（要件 1.1・すべて物理 px）。
///
/// wintf `Monitor` からの**忠実転写**であり、単位変換も正規化も行わない（U 契約:
/// 配置パイプラインの座標は物理 px 単一通貨）。転写は呼出側の仕事で、本型自身は
/// 表示基盤の型に一切依存しない純データ——これが `diag` を placement 最下流に
/// 置ける理由であり、決定論テストが実モニタ無しで合成値を流し込める理由でもある。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MonitorRecord {
    /// モニタ識別子（`HMONITOR` の生値）。
    pub handle: isize,
    /// 境界矩形 `(left, top, right, bottom)`（物理 px）。
    pub bounds: (i32, i32, i32, i32),
    /// work area 矩形 `(left, top, right, bottom)`（物理 px）。
    pub work_area: (i32, i32, i32, i32),
    /// モニタ DPI。
    pub dpi: u32,
    /// プライマリ標識。
    pub is_primary: bool,
}

/// 窓移動レコード（要件 1.2）——**単一ライターが位置を書き込んだ事実**の記録。
///
/// `enqueue_window_set_pos` の成功時に組む想定で、種別・scope は
/// `CharWindowMarker`／`BalloonWindowMarker`、`dpi` は `DPI` component から
/// 呼出側（`follow.rs`・タスク 1.4）が読んで詰める。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WindowMoveRecord {
    /// 位置を書いた経路（＝要件 2.4 の書き手の名指し）。
    pub route: PlacementRoute,
    /// 対象窓の entity。**wintf 側ログとの結合キー**（同じ `Debug` 表現で出す）。
    pub entity: Entity,
    /// 窓種別（キャラ／バルーン）。
    pub kind: WindowKind,
    /// スコープ番号（0=本体・1=相方・…）。
    pub scope: usize,
    /// 変化後の物理 X 座標。
    pub x: i32,
    /// 変化後の物理 Y 座標。
    pub y: i32,
    /// 変化後の物理寸 `(w, h)`。移動専用経路（`SWP_NOSIZE`）では `None`。
    pub size: Option<(i32, i32)>,
    /// 当該窓の DPI。`DPI` component 未付与なら `None`。
    pub dpi: Option<u32>,
}

/// モニタスナップショットの見出し行を組む（純関数）。
///
/// 台数を独立した行に出すのは、モニタ 0 台という**異常そのもの**をレコードの不在では
/// なく `count=0` として観測できるようにするため（不在は「出力が無効だった」と
/// 区別が付かない）。
pub fn monitor_snapshot_header_line(context: &str, count: usize) -> String {
    format!("{MONITOR_SNAPSHOT_TAG} context={context} count={count}")
}

/// モニタ 1 台ぶんのレコード行を組む（純関数・要件 1.1 の全フィールド）。
pub fn monitor_record_line(record: &MonitorRecord, index: usize) -> String {
    let (bl, bt, br, bb) = record.bounds;
    let (wl, wt, wr, wb) = record.work_area;
    format!(
        "{MONITOR_RECORD_TAG} index={index} handle={handle} \
         bounds={bl},{bt},{br},{bb} work_area={wl},{wt},{wr},{wb} \
         dpi={dpi} primary={primary}",
        handle = record.handle,
        dpi = record.dpi,
        primary = record.is_primary,
    )
}

/// 窓移動レコード行を組む（純関数・要件 1.2 の全フィールド）。
///
/// 寸・DPI が取れない経路でもフィールドは落とさず [`UNKNOWN`] を置く（語彙不変）。
pub fn window_move_record_line(record: &WindowMoveRecord) -> String {
    let (w, h) = match record.size {
        Some((w, h)) => (w.to_string(), h.to_string()),
        None => (UNKNOWN.to_string(), UNKNOWN.to_string()),
    };
    let dpi = match record.dpi {
        Some(dpi) => dpi.to_string(),
        None => UNKNOWN.to_string(),
    };
    format!(
        "{WINDOW_MOVE_RECORD_TAG} route={route} entity={entity:?} kind={kind} scope={scope} \
         x={x} y={y} w={w} h={h} dpi={dpi}",
        route = record.route.as_str(),
        entity = record.entity,
        kind = record.kind.as_str(),
        scope = record.scope,
        x = record.x,
        y = record.y,
    )
}

/// 検出した全モニタの観測レコードを出力する（要件 1.1・起動時 1 回きりの有界出力）。
///
/// `context` は呼出点タグ（`"prepare_ghost_windows"`・`"monitor_snapshot"` 等）で、
/// 同一運転内に複数の列挙点があってもログ上で出所を弁別できるようにする。
///
/// 出力は見出し 1 行＋モニタ 1 台 1 行。本文は純関数の組立結果そのもの
/// （組立を二重に持たない＝檻が本番の書式を固定する）。
pub fn log_monitor_snapshot(monitors: &[MonitorRecord], context: &str) {
    debug!(
        target: DIAG_TARGET,
        "{}",
        monitor_snapshot_header_line(context, monitors.len())
    );
    for (index, monitor) in monitors.iter().enumerate() {
        debug!(target: DIAG_TARGET, "{}", monitor_record_line(monitor, index));
    }
}

/// 窓移動レコードを 1 行出力する（要件 1.2・単一ライターの成功時）。
pub fn log_window_move(record: &WindowMoveRecord) {
    debug!(target: DIAG_TARGET, "{}", window_move_record_line(record));
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use bevy_ecs::entity::Entity;
    use tracing::Level;
    use tracing_subscriber::EnvFilter;

    use super::*;
    use crate::placement::test_support::{capture_logs, ensure_interest_probes};

    /// 合成モニタレコード（実 wintf `Monitor` 不要＝純データ）。
    fn rec(handle: isize, is_primary: bool) -> MonitorRecord {
        MonitorRecord {
            handle,
            bounds: (0, 0, 1920, 1080),
            work_area: (0, 0, 1920, 1040),
            dpi: 120,
            is_primary,
        }
    }

    fn ent(raw: u32) -> Entity {
        Entity::from_raw_u32(raw).expect("テスト用 entity index は有効")
    }

    // ------------------------------------------------------------------
    // 語彙の固定（Req 1.2/2.4・診断手順書の grep 判定語と 1:1）
    // ------------------------------------------------------------------

    /// 専用 target のリテラル固定（Req 1.7・手順書の `RUST_LOG` 記述と 1:1）。
    #[test]
    fn diag_target_literal_is_fixed() {
        assert_eq!(DIAG_TARGET, "areka::placement::diag");
    }

    /// レコード種別タグのリテラル固定（grep 判定語の錨）。
    #[test]
    fn record_tags_are_fixed() {
        assert_eq!(MONITOR_SNAPSHOT_TAG, "[diag.monitor_snapshot]");
        assert_eq!(MONITOR_RECORD_TAG, "[diag.monitor]");
        assert_eq!(WINDOW_MOVE_RECORD_TAG, "[diag.window_move]");
    }

    /// 経路語彙 9 種の表示語がリテラル固定されている（Req 2.4 の結論語彙・D13）。
    #[test]
    fn placement_route_vocabulary_is_fixed() {
        assert_eq!(PlacementRoute::SpawnInitial.as_str(), "SpawnInitial");
        assert_eq!(PlacementRoute::Restore.as_str(), "Restore");
        assert_eq!(PlacementRoute::AnchorChange.as_str(), "AnchorChange");
        assert_eq!(PlacementRoute::Resnap.as_str(), "Resnap");
        assert_eq!(PlacementRoute::DpiReproject.as_str(), "DpiReproject");
        assert_eq!(
            PlacementRoute::KeepPositionResize.as_str(),
            "KeepPositionResize"
        );
        assert_eq!(PlacementRoute::BalloonFollow.as_str(), "BalloonFollow");
        assert_eq!(
            PlacementRoute::ReportedSizeReconcile.as_str(),
            "ReportedSizeReconcile"
        );
        assert_eq!(PlacementRoute::MoveCue.as_str(), "MoveCue");
    }

    /// `ALL` は 9 バリアント全部・重複なし（語彙が 1 つでも欠けたら落ちる・D13）。
    #[test]
    fn placement_route_all_covers_nine_distinct_variants() {
        let all = PlacementRoute::ALL;
        assert_eq!(
            all.len(),
            9,
            "経路語彙は 9 種（design Service Interface・D13）"
        );
        for route in [
            PlacementRoute::SpawnInitial,
            PlacementRoute::Restore,
            PlacementRoute::AnchorChange,
            PlacementRoute::Resnap,
            PlacementRoute::DpiReproject,
            PlacementRoute::KeepPositionResize,
            PlacementRoute::BalloonFollow,
            PlacementRoute::ReportedSizeReconcile,
            PlacementRoute::MoveCue,
        ] {
            assert!(all.contains(&route), "ALL に {route} が無い");
        }
        let mut names: Vec<&str> = all.iter().map(|r| r.as_str()).collect();
        names.sort_unstable();
        let before = names.len();
        names.dedup();
        assert_eq!(before, names.len(), "表示語が重複している: {names:?}");
    }

    /// `Display` は `as_str` と同値（ログ組立の 2 経路が食い違わない）。
    #[test]
    fn placement_route_display_matches_as_str() {
        for route in PlacementRoute::ALL {
            assert_eq!(route.to_string(), route.as_str());
        }
    }

    /// 窓種別の語彙固定（手順書④の 2 段 grep が char/balloon で選別する）。
    #[test]
    fn window_kind_vocabulary_is_fixed() {
        assert_eq!(WindowKind::Char.as_str(), "char");
        assert_eq!(WindowKind::Balloon.as_str(), "balloon");
        assert_eq!(WindowKind::Char.to_string(), "char");
        assert_eq!(WindowKind::Balloon.to_string(), "balloon");
    }

    // ------------------------------------------------------------------
    // レコード組立（純関数・全フィールド固定）
    // ------------------------------------------------------------------

    /// モニタスナップショット見出し行の厳密固定。
    #[test]
    fn monitor_snapshot_header_line_is_fixed() {
        assert_eq!(
            monitor_snapshot_header_line("prepare_ghost_windows", 2),
            "[diag.monitor_snapshot] context=prepare_ghost_windows count=2"
        );
    }

    /// モニタレコード行が識別子・bounds・work_area・DPI・primary の**全フィールド**を持つ
    /// （1 つでも落ちたら落ちる檻・Req 1.1）。
    #[test]
    fn monitor_record_line_carries_every_field() {
        let line = monitor_record_line(&rec(65537, true), 0);
        assert_eq!(
            line,
            "[diag.monitor] index=0 handle=65537 bounds=0,0,1920,1080 \
             work_area=0,0,1920,1040 dpi=120 primary=true"
        );
        for key in [
            "index=",
            "handle=",
            "bounds=",
            "work_area=",
            "dpi=",
            "primary=",
        ] {
            assert!(line.contains(key), "フィールド `{key}` が欠落: {line}");
        }
    }

    /// 負座標・非プライマリ・3200 超座標（混在 DPI マルチモニタ実機相当）も忠実転写。
    #[test]
    fn monitor_record_line_transcribes_negative_and_wide_coordinates() {
        let record = MonitorRecord {
            handle: -3,
            bounds: (-1920, -40, 0, 1040),
            work_area: (-1920, -40, 0, 1000),
            dpi: 192,
            is_primary: false,
        };
        assert_eq!(
            monitor_record_line(&record, 3),
            "[diag.monitor] index=3 handle=-3 bounds=-1920,-40,0,1040 \
             work_area=-1920,-40,0,1000 dpi=192 primary=false"
        );
    }

    /// 窓移動レコードが route・entity・種別・scope・位置・寸・DPI の**全フィールド**を持つ
    /// （Req 1.2・design「窓移動レコード」）。
    #[test]
    fn window_move_record_line_carries_every_field() {
        let entity = ent(12);
        let record = WindowMoveRecord {
            route: PlacementRoute::DpiReproject,
            entity,
            kind: WindowKind::Char,
            scope: 1,
            x: -1500,
            y: 640,
            size: Some((336, 400)),
            dpi: Some(192),
        };
        let line = window_move_record_line(&record);
        assert_eq!(
            line,
            format!(
                "[diag.window_move] route=DpiReproject entity={entity:?} kind=char scope=1 \
                 x=-1500 y=640 w=336 h=400 dpi=192"
            )
        );
        for key in [
            "route=", "entity=", "kind=", "scope=", "x=", "y=", "w=", "h=", "dpi=",
        ] {
            assert!(line.contains(key), "フィールド `{key}` が欠落: {line}");
        }
    }

    /// entity は wintf 側ログ（`entity = ?e`＝`Debug` 表現）との**結合キー**ゆえ、
    /// 同一の `Debug` 表現で出す（手順書④の 2 段 grep が成立する条件）。
    #[test]
    fn window_move_record_entity_uses_debug_rendering_of_wintf_logs() {
        let entity = ent(4242);
        let record = WindowMoveRecord {
            route: PlacementRoute::Resnap,
            entity,
            kind: WindowKind::Balloon,
            scope: 0,
            x: 0,
            y: 0,
            size: None,
            dpi: None,
        };
        let line = window_move_record_line(&record);
        assert!(
            line.contains(&format!("entity={:?}", entity)),
            "wintf の `entity = ?e` と同じ Debug 表現ではない: {line}"
        );
    }

    /// 寸・DPI が取れない経路（移動専用・component 未付与）は欠落させず番兵で出す
    /// （フィールド語彙は全経路で不変＝grep が経路ごとに揺れない）。
    #[test]
    fn window_move_record_line_uses_sentinel_for_unknown_size_and_dpi() {
        let entity = ent(7);
        let record = WindowMoveRecord {
            route: PlacementRoute::BalloonFollow,
            entity,
            kind: WindowKind::Balloon,
            scope: 1,
            x: 100,
            y: 200,
            size: None,
            dpi: None,
        };
        assert_eq!(
            window_move_record_line(&record),
            format!(
                "[diag.window_move] route=BalloonFollow entity={entity:?} kind=balloon scope=1 \
                 x=100 y=200 w=- h=- dpi=-"
            )
        );
    }

    /// 全 9 経路が `route=<語>` として組み上がる（配管先が増えても語彙が固定される）。
    #[test]
    fn window_move_record_line_renders_every_route() {
        for route in PlacementRoute::ALL {
            let record = WindowMoveRecord {
                route,
                entity: ent(1),
                kind: WindowKind::Char,
                scope: 0,
                x: 0,
                y: 0,
                size: Some((1, 1)),
                dpi: Some(96),
            };
            assert!(
                window_move_record_line(&record).contains(&format!("route={}", route.as_str())),
                "route={route} がレコードに現れない"
            );
        }
    }

    // ------------------------------------------------------------------
    // 出力（debug! 水準・専用 target）
    // ------------------------------------------------------------------

    /// `log_monitor_snapshot` は見出し 1 行＋モニタ 1 台 1 行を `debug!` で出し、
    /// 本文は純関数の組立結果と**一致**する（ログと純関数の二重実装を許さない）。
    #[test]
    fn log_monitor_snapshot_emits_header_and_one_line_per_monitor_at_debug() {
        let monitors = [rec(1, true), rec(2, false)];
        let (_, events) = capture_logs(|| log_monitor_snapshot(&monitors, "prepare_ghost_windows"));

        let messages: Vec<&str> = events.iter().map(|e| e.message()).collect();
        assert_eq!(
            messages,
            vec![
                monitor_snapshot_header_line("prepare_ghost_windows", 2).as_str(),
                monitor_record_line(&monitors[0], 0).as_str(),
                monitor_record_line(&monitors[1], 1).as_str(),
            ]
        );
        for e in &events {
            assert_eq!(e.level, Level::DEBUG, "恒久観測は debug! 水準（Req 1.7）");
        }
    }

    /// モニタ 0 台でも見出し 1 行は出る（count=0 が観測できる・panic しない）。
    #[test]
    fn log_monitor_snapshot_with_no_monitors_still_emits_header() {
        let (_, events) = capture_logs(|| log_monitor_snapshot(&[], "monitor_snapshot"));
        let messages: Vec<&str> = events.iter().map(|e| e.message()).collect();
        assert_eq!(
            messages,
            vec!["[diag.monitor_snapshot] context=monitor_snapshot count=0"]
        );
    }

    /// `log_window_move` は純関数の組立結果を `debug!` 水準で 1 行出す。
    #[test]
    fn log_window_move_emits_the_assembled_record_at_debug() {
        let record = WindowMoveRecord {
            route: PlacementRoute::AnchorChange,
            entity: ent(9),
            kind: WindowKind::Char,
            scope: 0,
            x: 10,
            y: 20,
            size: Some((30, 40)),
            dpi: Some(120),
        };
        let (_, events) = capture_logs(|| log_window_move(&record));
        assert_eq!(events.len(), 1, "1 移動 1 レコード: {events:?}");
        assert_eq!(events[0].message(), window_move_record_line(&record));
        assert_eq!(events[0].level, Level::DEBUG);
    }

    // ------------------------------------------------------------------
    // Req 1.7: 既定では 1 行も出ない・手順書の RUST_LOG でのみ点灯
    // ------------------------------------------------------------------

    /// 与えた `RUST_LOG` 相当の directive で本モジュールの全出力を実際に濾して集める。
    fn emit_all_under_filter(directives: &str) -> String {
        ensure_interest_probes();

        #[derive(Clone)]
        struct VecWriter(Arc<Mutex<Vec<u8>>>);
        impl std::io::Write for VecWriter {
            fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
                self.0
                    .lock()
                    .expect("捕捉バッファの毒化なし")
                    .extend_from_slice(buf);
                Ok(buf.len())
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }

        let buf = Arc::new(Mutex::new(Vec::<u8>::new()));
        let sink = buf.clone();
        let subscriber = tracing_subscriber::fmt()
            .with_env_filter(EnvFilter::new(directives))
            .with_ansi(false)
            .with_writer(move || VecWriter(sink.clone()))
            .finish();

        tracing::subscriber::with_default(subscriber, || {
            tracing::callsite::rebuild_interest_cache();
            log_monitor_snapshot(&[rec(1, true)], "prepare_ghost_windows");
            log_window_move(&WindowMoveRecord {
                route: PlacementRoute::SpawnInitial,
                entity: ent(1),
                kind: WindowKind::Char,
                scope: 0,
                x: 0,
                y: 0,
                size: Some((1, 1)),
                dpi: Some(96),
            });
        });

        String::from_utf8(buf.lock().expect("捕捉バッファの毒化なし").clone()).expect("UTF-8")
    }

    /// 既定水準（`RUST_LOG=info`＝`main.rs` のフォールバック）では **1 行も出ない**（Req 1.7）。
    #[test]
    fn diag_records_are_silent_under_default_info_filter() {
        assert_eq!(
            emit_all_under_filter("info"),
            "",
            "既定 RUST_LOG=info で診断出力が漏れている（Req 1.7 違反）"
        );
    }

    /// 専用 target を `info` で開けても出ない＝観測点は `debug!` 水準に置かれている。
    #[test]
    fn diag_records_are_silent_when_target_opened_only_to_info() {
        assert_eq!(
            emit_all_under_filter("info,areka::placement::diag=info"),
            "",
            "debug! ではない水準の観測点が混ざっている（Req 1.7・水準割当表）"
        );
    }

    /// 手順書の directive（`areka::placement::diag=debug`）で点灯する
    /// ＝target のリテラルが手順書と 1:1 で結ばれていることの機械的証明（Req 1.5/1.7）。
    #[test]
    fn diag_records_light_up_under_the_procedure_directive() {
        let out = emit_all_under_filter("info,areka::placement::diag=debug");
        assert!(
            out.contains(MONITOR_SNAPSHOT_TAG)
                && out.contains(MONITOR_RECORD_TAG)
                && out.contains(WINDOW_MOVE_RECORD_TAG),
            "手順書の RUST_LOG で診断レコードが点灯しない: {out}"
        );
        assert!(
            out.contains(DIAG_TARGET),
            "出力に専用 target が現れない（target 指定漏れ）: {out}"
        );
    }
}

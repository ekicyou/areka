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
//! 本モジュールは placement の**最下流**であり、`World`・表示基盤の型に依存しない。
//! 扱うのは素の数値・文字列と、**下記 2 つの例外**だけである。wintf `Monitor` からの
//! 転写は呼出側（`mod.rs`／`follow.rs`）の仕事である。
//!
//! - [`Entity`] は **wintf 側ログ（`entity = ?e`）との結合キー**だから受ける。同じ `Debug`
//!   表現で出すことが要件 1.9 の scope 別計数（手順書の 2 段 grep）の成立条件になる。
//! - [`ZOrderPairStrategy`] は選ばれた方式そのものを記録する
//!   （areka-P0-ghost-window-zorder 要件 5.6）ため受ける。素の文字列で受ける形にすると
//!   「どの値がどの字面になるか」の写像が呼出側へ散り、**変種が増えたときに記録だけが
//!   古いまま**になりうる。列挙をそのまま受ければ網羅 match がコンパイラに守られる。
//!   いずれも `World` を要さない `Copy` 値であり、最下流という位置は動かない。

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
use wintf::ecs::ZOrderPairStrategy;

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

/// ゴースト窓ペア宣言レコード行タグ
/// （areka-P0-ghost-window-zorder 要件 6.1・design「診断ログ語彙（要件 6）」表の `declared` 行）。
///
/// # なぜ他 6 種と違ってこのレコードだけ areka 側に住むのか
///
/// ペア宣言の記録に載る必須フィールドは **scope とペア両窓**であり、scope を知る層は
/// areka だけである（wintf → areka の import は禁止ゆえ wintf 側に scope を渡す口も無い）。
/// 残り 6 種のレコード（`owner-established`／`fix`／`skip`／`verify-failed`／
/// `owner-establish-failed`／`sink-observed`）は wintf が自力で全フィールドを埋められる
/// ため wintf 側に住み、scope を持たない。実機サインオフはこの `declared` レコードと
/// wintf 側レコードを **entity の `Debug` 表現を結合キーとして 2 段 grep** で突き合わせる
/// （[`window_move_record_line`] で確立済みの先例と同じ手口）。
///
/// タグ文字列が wintf 側 6 種と同じ `[zorder-pair] ` 接頭辞を持つのは意図的で、
/// 接頭辞 1 語で当該機能の記録を全部拾えることが 2 段 grep の第 1 段になる。
pub const ZORDER_PAIR_DECLARED_TAG: &str = "[zorder-pair] declared";

/// 実行時ストラテジ選択レコード行タグ
/// （areka-P0-ghost-window-zorder 要件 5.6・design「診断ログ語彙（要件 6）」表の
/// `strategy-selected` 行）。
///
/// # なぜ記録するのか
///
/// 重なりを保証する手段は実機ゲート（design「Plan A 実機可否ゲートとフォールバック分岐」の
/// G1〜G8）の結果で決まる。決まった後は、**どの手段で動いているのかが運転中のログから
/// 読めなければならない**——実機で重なりの不具合を切り分ける人にとって、前提の方式が
/// 分からないログは診断の出発点を欠く。ゲート判定表は spec 配下の文書であり、
/// 目の前で走っているバイナリが本当にその結論どおりかは、走っているバイナリ自身が
/// 名乗る以外に確かめようがない。
///
/// # なぜ areka 側なのか
///
/// 方式を選ぶのは areka の結線（[`wire_zorder_pair`](super::spawn::wire_zorder_pair)）であり、
/// wintf は与えられた値に従うだけで「どの分岐が選ばれたか」を知る立場にない
/// （[`ZORDER_PAIR_DECLARED_TAG`] が areka 側に住むのと同じ理由の別の面）。
///
/// 接頭辞 `[zorder-pair] ` が wintf 側 6 種および `declared` と共通なのは意図的で、
/// 実機サインオフの第 1 段 grep が 1 語のまま当該機能の記録を全部拾えることを保つ。
pub const ZORDER_PAIR_STRATEGY_TAG: &str = "[zorder-pair] strategy-selected";

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
/// 位置を書ける経路は 10 あり、ログ上でそれらが見分けられなければ書き手を名指しできない。
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
    /// バルーン窓ドラッグ解放時の `windowposition.limit` 補正
    /// （`on_balloon_drag_end` の解放時補正・areka-P0-windowposition-limit DD7）。
    ///
    /// **ドラッグ中は無介入**（明示操作の尊重）で、解放の瞬間にだけ画面内へ引き戻す
    /// ——この書込はドラッグ随伴（[`BalloonFollow`](Self::BalloonFollow)）でも
    /// 位置据置きリサイズでもない**独立した新規書込**ゆえ 1:1 の原則で専用語を持つ
    /// （D13 の「1 語が 2 トリガを指すと切り分けが原理的に不能になる」と同じ理由）。
    ///
    /// 関門（`enqueue_window_set_pos` 内）が行う補正は本語を名乗らない——そちらは
    /// **元書込の route を保つ**。補正は同一書込の一部であって別書込ではないからである（DD7）。
    // 発行者は task 3.4（`on_balloon_drag_end` の解放時補正）で入る。それまでは語彙のみ
    // 先行して存在する（`ALL` から参照されるため dead_code にはならない）。
    BalloonLimitRelease,
}

impl PlacementRoute {
    /// 全経路（檻が語彙の網羅と一意性を固定するための単一の列）。
    ///
    /// バリアントを増やしたら[`as_str`](Self::as_str) の網羅 match がコンパイルを止め、
    /// ここへの追加漏れは檻（`placement_route_all_covers_ten_distinct_variants`）が捕まえる。
    // 消費者は in-source 檻（`#[cfg(test)]`）だけゆえ本番ビルドでは dead。檻専用の列である
    // ことが存在理由そのもの（語彙の網羅・一意性の単一の錨）ゆえ本属性を残す。
    #[allow(dead_code)]
    pub const ALL: [PlacementRoute; 10] = [
        PlacementRoute::SpawnInitial,
        PlacementRoute::Restore,
        PlacementRoute::AnchorChange,
        PlacementRoute::Resnap,
        PlacementRoute::DpiReproject,
        PlacementRoute::KeepPositionResize,
        PlacementRoute::BalloonFollow,
        PlacementRoute::ReportedSizeReconcile,
        PlacementRoute::MoveCue,
        PlacementRoute::BalloonLimitRelease,
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
            PlacementRoute::BalloonLimitRelease => "BalloonLimitRelease",
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

/// ゴースト窓ペア宣言レコード行を組む
/// （純関数・areka-P0-ghost-window-zorder 要件 6.1 の全フィールド）。
///
/// entity は wintf 側レコード（`entity=…`／`peer=…`）と**同じ `Debug` 表現**で出す
/// ——これが 2 段 grep の結合条件そのものである（[`window_move_record_line`] と同じ理由）。
/// キャラ窓とバルーン窓は別フィールド名で載せ、役割が字面から読めるようにする。
pub fn zorder_pair_declared_line(
    scope: usize,
    char_entity: Entity,
    balloon_entity: Entity,
) -> String {
    format!(
        "{ZORDER_PAIR_DECLARED_TAG} scope={scope} char_entity={char_entity:?} \
         balloon_entity={balloon_entity:?}"
    )
}

/// ゴースト窓ペア宣言レコードを 1 行出力する（要件 6.1・窓生成時の 1 スコープ 1 本）。
pub fn log_zorder_pair_declared(scope: usize, char_entity: Entity, balloon_entity: Entity) {
    debug!(
        target: DIAG_TARGET,
        "{}",
        zorder_pair_declared_line(scope, char_entity, balloon_entity)
    );
}

/// 実行時ストラテジ選択レコード行を組む（純関数・要件 5.6）。
///
/// 判定表の語彙（案 A／案 B）と実装の語彙（`OwnerLink`／`ExplicitMaintenance`）は別物ゆえ、
/// **両方**を載せる——`plan` はゲート判定表と突き合わせるための語、`mechanism` は何を
/// して重なりを保っているかを表す語である。どちらか片方だけだと、読み手はもう一方を
/// 頭の中で翻訳しなければならない。
///
/// `raise_assist` は案 A にしか無い概念だが、案 B でも**フィールドを落とさず**番兵
/// [`UNKNOWN`] を置く（本モジュールの既定方針＝経路によって grep の当たり方を変えない）。
pub fn zorder_pair_strategy_line(strategy: ZOrderPairStrategy) -> String {
    let (plan, mechanism, raise_assist) = match strategy {
        ZOrderPairStrategy::OwnerLink { raise_assist } => {
            ("A", "owner-link", raise_assist.to_string())
        }
        ZOrderPairStrategy::ExplicitMaintenance => {
            ("B", "explicit-maintenance", UNKNOWN.to_string())
        }
    };
    format!(
        "{ZORDER_PAIR_STRATEGY_TAG} plan={plan} mechanism={mechanism} raise_assist={raise_assist}"
    )
}

/// 実行時ストラテジ選択レコードを 1 行出力する（要件 5.6・起動時 1 運転 1 本）。
pub fn log_zorder_pair_strategy(strategy: ZOrderPairStrategy) {
    debug!(
        target: DIAG_TARGET,
        "{}",
        zorder_pair_strategy_line(strategy)
    );
}

#[cfg(test)]
#[path = "diag_tests.rs"]
mod tests;

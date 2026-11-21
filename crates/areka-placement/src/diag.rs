//! 配置観測（診断計装）の語彙・専用 target・レコード組立の単一の住処
//!
//! # 何のためにあるか
//!
//! 混在 DPI マルチモニタ実機でゴースト窓が「知らないうちに消える」事象は、消失の
//! **瞬間が未目撃**である。したがって運転後のログだけから「どのモニタのどこに・どの寸で
//! 居たか」「最後に位置を書いたのは誰か」を事後再構成できる証跡が要る。
//! 本モジュールはその証跡の語彙（[`PlacementRoute`]）・出力先（[`DIAG_TARGET`]）・
//! レコード組立（[`monitor_record_line`]／[`window_move_record_line`]）を 1 箇所に集める。
//!
//! # 恒久計装であって一時計装ではない
//!
//! 観測点は診断完了後も本体コードに残す。既定では**無音**であり、診断手順書が指定する
//! `RUST_LOG=…,areka_placement::diag=debug` でのみ点灯する。この「既定 OFF」は
//! **専用 target ＋ `debug!` 水準**の 2 段で担保され、in-source 檻
//! （`diag_records_are_silent_under_default_info_filter` 他）が実際の [`EnvFilter`] 濾過で
//! 機械的に固定する。
//!
//! [`EnvFilter`]: tracing_subscriber::EnvFilter
//!
//! # レコードは純関数が組み、ログはそれを出すだけ
//!
//! 診断手順書の grep 判定語と出力書式は 1:1 で対応する。書式が意図せず
//! 変われば手順書が静かに嘘になるため、**組立を純関数に切り出して in-source 檻で
//! リテラル固定**する（固定したいのは物理量ではなく手順書との語彙一致そのものゆえ、判定は比や
//! 不変条件でなくリテラルで行う）。
//! [`log_monitor_snapshot`]／[`log_window_move`] は純関数の戻り値をそのまま本文にする
//! （組立を二重実装しない＝檻が本番の出力を本当に固定する）。
//!
//! # 依存方向
//!
//! 本モジュールは placement の**最下流**であり、`World`・表示基盤の型に依存しない。
//! 扱うのは素の数値・文字列と、**下記 2 つの例外**だけである。wintf `Monitor` からの
//! 転写は呼出側（`lib.rs` の `monitor_records`）の仕事である。
//!
//! - [`Entity`] は **wintf 側ログ（`entity = ?e`）との結合キー**だから受ける。同じ `Debug`
//!   表現で出すことが scope 別計数（手順書の 2 段 grep）の成立条件になる。
//! - [`ZOrderPairStrategy`] は選ばれた方式そのものを記録する
//!   ため受ける。素の文字列で受ける形にすると
//!   「どの値がどの字面になるか」の写像が呼出側へ散り、**変種が増えたときに記録だけが
//!   古いまま**になりうる。列挙をそのまま受ければ網羅 match がコンパイラに守られる。
//!   いずれも `World` を要さない `Copy` 値であり、最下流という位置は動かない。

// 本番の呼び手:
// - モニタスナップショット側（`MonitorRecord`・`monitor_snapshot_header_line`／
//   `monitor_record_line`・`log_monitor_snapshot`）: `areka` の `main.rs`（`boot_monitor_snapshot`・
//   正典出力点）・`crates/areka-runtime/src/frame/work_area_sync.rs`（作業領域の同期）と
//   `prepare_ghost_windows_for_shell` の列挙点が呼ぶ。
// - 窓移動レコード側（`WindowKind`・`WindowMoveRecord`・`window_move_record_line`・
//   `log_window_move`・`WINDOW_MOVE_RECORD_TAG`）: `follow::window_move` の単一ライター
//   `enqueue_window_set_pos` が書込成功時に呼ぶ。

use std::fmt;

use bevy_ecs::entity::Entity;
use tracing::debug;
use wintf::ecs::ZOrderPairStrategy;
// 値が取得できなかったフィールドの番兵は wintf の記録と同じ綴りを使う。
// フィールド自体は落とさない（落とすと経路ごとに grep の当たり方が変わる）。
use wintf::ecs::window::transition_diag::MISSING;

/// 恒久観測の専用ログ target（既定 OFF・診断手順が `RUST_LOG` で点灯）。
///
/// 本モジュールの出力は**すべて**この target を通る。診断手順書の
/// `RUST_LOG=info,…,areka_placement::diag=debug` と 1:1 で対応し、
/// リテラルは in-source 檻が固定する（勝手に変えると手順書が無効化する）。
pub const DIAG_TARGET: &str = "areka_placement::diag";

/// モニタスナップショットの見出し行タグ（grep 判定語・台数がここに出る）。
pub const MONITOR_SNAPSHOT_TAG: &str = "[diag.monitor_snapshot]";

/// モニタ 1 台ぶんのレコード行タグ（grep 判定語）。
pub const MONITOR_RECORD_TAG: &str = "[diag.monitor]";

/// 窓移動レコード行タグ（grep 判定語・「経路」つき移動記録）。
pub const WINDOW_MOVE_RECORD_TAG: &str = "[diag.window_move]";

/// ゴースト窓ペア宣言レコード行タグ。
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

/// 実行時ストラテジ選択レコード行タグ。
///
/// # なぜ記録するのか
///
/// 重なりを保証する手段は実機ゲートの結果で決まる。決まった後は、**どの手段で動いているのかが
/// 運転中のログから読めなければならない**——実機で重なりの不具合を切り分ける人にとって、
/// 前提の方式が分からないログは診断の出発点を欠く。
/// 目の前で走っているバイナリが本当にゲートの結論どおりかは、走っているバイナリ自身が
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

/// 破棄済み（despawn 済み）entity を消費側が**正常系として**打ち切ったことを表す判定語。
///
/// # なぜ語彙を 1 箇所に置くのか
///
/// この打ち切りは「entity 不在＝破棄済み（正常終了系・`debug!`）」と「entity は実在するが
/// 接地点規約の component が欠落（真の異常・`warn!`）」を**区別**するために入れたものだが、
/// 区別は水準だけでなく**本文でも**引けなければ終了時ログの読み手に伝わらない。消費入口は
/// 追従層・フレーム層・終了処理に散らばり、各所が独自の文言を持つと grep 語が静かに分裂する。
/// ゆえに**共通の接頭辞だけを本定数が持ち**、後続の説明文は各消費点が自分の相を名乗る形にする。
///
/// # target について（本定数だけの例外）
///
/// 本モジュールの**出力**はすべて [`DIAG_TARGET`] を通るが、本定数を用いる `debug!` は
/// 各消費点のモジュールの既定 target へ出る——
/// 終了時ログの静穏性は診断手順の点灯有無に依らず成立すべきものだからである。
/// 本モジュールが持つのは語彙であって出力点ではない。
pub const DESPAWNED_SKIP_TAG: &str = "[despawn-skip]";

/// 窓位置・窓寸を書き込んだ**経路**（「変化を引き起こした経路」＝「最終位置を書き込んだ主体」を名指しするための結論語彙）。
///
/// 消失の診断で最後に要るのは「どこに居たか」ではなく「**誰が最後に書いたか**」である。
/// 位置を書ける経路は 13 あり、ログ上でそれらが見分けられなければ書き手を名指しできない。
/// 表示語（[`PlacementRoute::as_str`]）は診断手順書の判定語そのものゆえ檻で固定する。
///
/// # 語彙は「実在の書込トリガ」と 1:1 でなければならない
///
/// `reconcile_window_size` の drain 後段からの呼出（[`ReportedSizeReconcile`](Self::ReportedSizeReconcile)）と
/// `\![move]`（[`MoveCue`](Self::MoveCue)）は別の実在トリガである。**1 語が 2 トリガを指すと
/// route 名での切り分けが原理的に不能になる**（DPI 変化ゼロの起動が「DPI 由来」を名乗る偽陽性が
/// 毎回出る）ため、語彙は実在トリガと 1:1 に保つ。
///
/// ただし今は [`MoveCue`](Self::MoveCue)・[`Restore`](Self::Restore) の 2 語が複数の呼び手を
/// 持ち、語だけではどの呼び手かを見分けられない（呼び手は各変種の注に挙げる）。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PlacementRoute {
    /// spawn 初期配置（`spawn` の窓生成時）。
    // 単一ライターを通らない経路（spawn は entity 組立時に `WindowPos` を直接持たせる）ゆえ現状どこからも構築されない。語彙のみ保持し、将来の配線先として予約しているので削らない。
    SpawnInitial,
    /// 位置永続化の復元マージ（可視性ガード適用外）。
    // 起動時の復元は spawn 前の `persist::restore_and_judge` が `ScopePlacement` をマージするので
    // この語を作らない。作るのはバルーン窓を標準の部品で据え直す `reseed::reinstall_balloon` の
    // `follow_balloon` の引き金（記憶の復元と同じ明示の配置として扱う）。呼び手は 3 つある:
    // シェルの差し替え（`reseed::apply_shell_descript`）・面の windowposition の置き直し
    // （`face_windowposition::apply_face_windowposition`）・バルーン位置初期化
    // （`reset_balloon_pos::reset_balloon_positions`）。この語だけでは 3 つを見分けられない。
    Restore,
    /// アンカー変化トリガ。本番で書くのはシェルの差し替えで新しい揃え方へ今の窓寸のまま置き直す
    /// `reseed::reanchor_char_windows` だけ（`anchor_changed_system` は schedule に登録しない見本）。
    AnchorChange,
    /// 毎フレーム再スナップ（`resnap_from_sizes`）。
    Resnap,
    /// DPI 変化に伴う位置の再射影（**`dpi_phase` 限定**）。
    ///
    /// drain の後段の報告回収は [`ReportedSizeReconcile`](Self::ReportedSizeReconcile) が担い、
    /// 本変種と混同しない——両者は `reconcile_window_size` という同一の共通末端を
    /// 通るが、**同一の末端を通ることは同一の経路であることを意味しない**。
    DpiReproject,
    /// バルーン窓の位置据置きリサイズ（`resize_window_keep_position`）。
    KeepPositionResize,
    /// キャラ窓確定後のバルーン随伴（`follow_balloon`）。
    BalloonFollow,
    /// drain の後段の報告回収（`reconcile_reported_sizes`・**初回表示の k₀ 補正を含む**）。
    ///
    /// `Changed<DPI>` **非依存**＝DPI 変化ゼロでも発火し得るため
    /// [`DpiReproject`](Self::DpiReproject) と別語にする。混同すると受理回数の突合に
    /// 偽陽性が混入する。
    ReportedSizeReconcile,
    /// `\![move]` cue などの明示の操作による移動（`move_window_to` の**対象窓**）。
    ///
    /// 明示操作の尊重ゆえ遷移ガード適用外＝ドラッグ・[`Restore`](Self::Restore) と同族。
    /// 随伴バルーン側の書込は [`BalloonFollow`](Self::BalloonFollow) を持つ。
    // 名は `\![move]` から来たが、作る道は 2 つある。1 つは `follow::move_window_to` で、呼び手は
    // 台本の `\![move]`（本体の `runtime::move_cue_place`）とその連れ動き（`follow_sticky_move`）・
    // 起動の連鎖の確定（本体の `runtime::frame::drain_resnap` の `finalize_chain_once_with`）・
    // 位置の初期化（`reset_pos::reset_window_positions`＝台本・メニュー「シェル位置初期化」・近道）。
    // もう 1 つは今の大きさのまま当て直す `follow::resize_window_to` で、呼び手は台本の吸着の向きの
    // 変更（`script_alignment::apply_script_alignment`）とドラッグの終わりの当て直し
    // （`finish_char_drag`）。この語だけではこれらを見分けられない。
    MoveCue,
    /// バルーン窓ドラッグ解放時の `windowposition.limit` 補正
    /// （`on_balloon_drag_end` の解放時補正）。
    ///
    /// **ドラッグ中は無介入**（明示操作の尊重）で、解放の瞬間にだけ画面内へ引き戻す
    /// ——この書込はドラッグ随伴（[`BalloonFollow`](Self::BalloonFollow)）でも
    /// 位置据置きリサイズでもない**独立した新規書込**ゆえ 1:1 の原則で専用語を持つ。
    ///
    /// 関門（`enqueue_window_set_pos` 内）が行う補正は本語を名乗らない——そちらは
    /// **元書込の route を保つ**。補正は同一書込の一部であって別書込ではないからである。
    // 発行者は `follow::balloon_drag::apply_release_limit_correction` ただ 1 箇所。
    BalloonLimitRelease,
    /// 作業領域の変化を契機とする現寸での再スナップ（`work_area_sync::resnap_for_work_area_change`）。
    ///
    /// 拡大率は変わらず**作業領域だけ**が変わった窓を、現在の寸のまま新しい下端へ移す書込で
    /// ある。毎フレーム走る [`Resnap`](Self::Resnap) とも、拡大率変化に伴う
    /// [`DpiReproject`](Self::DpiReproject) とも**実在のトリガが違う**ため専用語を持つ。
    /// 混同すると「作業領域が動いたから移した」のか
    /// 「毎フレームの再スナップが動かした」のかがログ上で切り分けられない。
    // 発行者は `runtime::frame::work_area_sync::resnap_for_work_area_change`。
    // 語彙をここに置くのは、判定器と単一の窓書込口の札が同じ語を見るためである。
    WorkAreaResnap,
    /// DPI 遷移後の連鎖（隣接ペア）再解決（`chain_realign::realign_chain_once_with`）。
    ///
    /// 起動時一度きりの連鎖確定とは別機構で、遷移 1 回につき一度だけ
    /// 隣接を解き直す書込である。起動時確定は `SpawnInitial`／`Restore` の後段で位置を決めて
    /// [`MoveCue`](Self::MoveCue) で書くのに対し、こちらは遷移後の新しい幅で左端を連ね直す
    /// ——由来が違うので語も分ける。
    // 発行者は `chain_realign::realign_chain_once_with`。
    ChainRealign,
    /// 面の `move` のずれを窓の位置へ当てる書込（`follow::move_shift::apply_char_shift`）。
    ///
    /// 台本の明示の `\![move]`（[`MoveCue`](Self::MoveCue)）と違い、面の表示に従う**表示の側の
    /// 置き直し**である。ゆえに「利用者が動かした」と数えず（既定の位置もずれに従う）、見える範囲の
    /// 守りを当て、窓の書き込みの見送りが覆う経路に入れる。
    PatternMove,
}

impl PlacementRoute {
    /// 全経路（檻が語彙の網羅と一意性を固定するための単一の列）。
    ///
    /// バリアントを増やすと、[`as_str`](Self::as_str) と [`is_system_reanchor`](Self::is_system_reanchor)
    /// に加えて檻の表（`diag_tests.rs` の `expected`・網羅の `match`）も組めなくなる。表で番号を
    /// 振る人がここへも足し、檻（`placement_route_all_lists_every_route_once_in_order`）が
    /// 列の番号が 0〜N−1 をちょうど 1 度ずつ埋めることを確かめる。
    // 消費者は in-source 檻（`#[cfg(test)]`）だけである。檻専用の列であることが存在理由
    // そのもの（語彙の網羅・一意性の単一の錨）。
    pub const ALL: [PlacementRoute; 13] = [
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
        PlacementRoute::WorkAreaResnap,
        PlacementRoute::ChainRealign,
        PlacementRoute::PatternMove,
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
            PlacementRoute::WorkAreaResnap => "WorkAreaResnap",
            PlacementRoute::ChainRealign => "ChainRealign",
            PlacementRoute::PatternMove => "PatternMove",
        }
    }

    /// **システム由来の再アンカー**か。
    ///
    /// 表示環境の変化（拡大率・作業領域・報告寸・アンカー）を受けて**基盤が**窓を置き直した
    /// 書込が `true`、利用者やゴースト台本の**明示操作**による書込が `false` である。
    ///
    /// # なぜ 1 箇所で持つか
    ///
    /// この区分は 2 つの判断で読まれる——既定位置の追跡（単一の窓書込口
    /// `follow::window_move::enqueue_window_set_pos` が既定位置を書込先へ追随させるか）と、
    /// 可視性の遷移ガード（`follow::visibility::route_applies_visibility_guard` が
    /// 「可視 → 全 work area 非交差」を阻止するか）。両者は**同一の区分**（同じ集合・今は `PatternMove` を足した
    /// 7 経路）なので、列を 2 本持つと片方だけを直したときに静かに食い違う。ゆえに定義をここ
    /// 1 本にし、可視性側は本関数へ委譲する。
    ///
    /// 将来この 2 つの区分を**意図的に**割るなら、列を割ったうえで理由を残すこと
    /// （黙って片方へ腕を足さない）。
    ///
    /// # 網羅 match である理由
    ///
    /// 経路語彙が増えたとき、新しい語をどちら側へ置くかを**必ず**書かせるため。既定の腕
    /// （`_ => false`）を置くと、新設経路が黙って「明示操作」に倒れて既定位置が追随しなくなる。
    /// 各経路の答えは檻の表（`diag_tests.rs` の `expected`）と突き合わせる
    /// （`placement_route_answers_match_the_table`）。
    pub const fn is_system_reanchor(self) -> bool {
        match self {
            // 表示環境の変化を受けた基盤側の置き直し（下の `PatternMove` と合わせて 7 経路）。
            PlacementRoute::AnchorChange
            | PlacementRoute::Resnap
            | PlacementRoute::DpiReproject
            | PlacementRoute::ReportedSizeReconcile
            | PlacementRoute::WorkAreaResnap
            | PlacementRoute::ChainRealign
            // 面の `move` のずれ（表示の側の置き直し）。
            | PlacementRoute::PatternMove => true,
            // 明示操作（`MoveCue`・`Restore`・`BalloonLimitRelease`。ドラッグは route を
            // 持たない＝`None` ゆえ呼び手側で除かれる）と、既定位置を持たない窓側の経路
            // （`SpawnInitial` は既定位置そのものを作る点・`KeepPositionResize`／
            // `BalloonFollow` はバルーン窓の従属量）。
            PlacementRoute::SpawnInitial
            | PlacementRoute::Restore
            | PlacementRoute::KeepPositionResize
            | PlacementRoute::BalloonFollow
            | PlacementRoute::MoveCue
            | PlacementRoute::BalloonLimitRelease => false,
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
/// （バルーン窓の entity を混ぜると DPI 受理回数の計数が壊れる）。
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

/// モニタ 1 台ぶんの観測レコード（すべて物理 px）。
///
/// wintf `Monitor` からの**忠実転写**であり、単位変換も正規化も行わない（配置パイプラインの座標は物理 px 単一通貨）。転写は呼出側の仕事で、本型自身は
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

/// 窓移動レコード——**単一ライターが位置を書き込んだ事実**の記録。
///
/// `enqueue_window_set_pos` の成功時に組む想定で、種別・scope は
/// `CharWindowMarker`／`BalloonWindowMarker`、`dpi` は `DPI` component から
/// 呼出側（`follow::window_move`）が読んで詰める。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WindowMoveRecord {
    /// 位置を書いた経路（＝書き手の名指し）。
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

/// モニタ 1 台ぶんのレコード行を組む（純関数）。
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

/// 窓移動レコード行を組む（純関数）。
///
/// 寸・DPI が取れない経路でもフィールドは落とさず [`MISSING`] を置く（語彙不変）。
pub fn window_move_record_line(record: &WindowMoveRecord) -> String {
    let (w, h) = match record.size {
        Some((w, h)) => (w.to_string(), h.to_string()),
        None => (MISSING.to_string(), MISSING.to_string()),
    };
    let dpi = match record.dpi {
        Some(dpi) => dpi.to_string(),
        None => MISSING.to_string(),
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

/// 検出した全モニタの観測レコードを出力する（1 回の呼び出しで有界の行数）。
///
/// 呼ぶのは起動時の列挙・窓の準備・作業領域の同期で、作業領域が変わるたびにも出る。
/// `context` は呼出点タグ（`"monitor_snapshot"`・`"prepare_ghost_windows"`・`"work_area_sync"`）で、
/// 同一運転内に複数の列挙点があってもログ上で出所を弁別できるようにする。
///
/// 出力は見出し 1 行＋モニタ 1 台 1 行。
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

/// 窓移動レコードを 1 行出力する（単一ライターの成功時）。
pub fn log_window_move(record: &WindowMoveRecord) {
    debug!(target: DIAG_TARGET, "{}", window_move_record_line(record));
}

/// ゴースト窓ペア宣言レコード行を組む（純関数）。
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

/// ゴースト窓ペア宣言レコードを 1 行出力する（窓生成時の 1 スコープ 1 本）。
pub fn log_zorder_pair_declared(scope: usize, char_entity: Entity, balloon_entity: Entity) {
    debug!(
        target: DIAG_TARGET,
        "{}",
        zorder_pair_declared_line(scope, char_entity, balloon_entity)
    );
}

/// 実行時ストラテジ選択レコード行を組む（純関数）。
///
/// 判定表の語彙（案 A／案 B）と実装の語彙（`OwnerLink`／`ExplicitMaintenance`）は別物ゆえ、
/// **両方**を載せる——`plan` はゲート判定表と突き合わせるための語、`mechanism` は何を
/// して重なりを保っているかを表す語である。どちらか片方だけだと、読み手はもう一方を
/// 頭の中で翻訳しなければならない。
///
/// `raise_assist` は案 A にしか無い概念だが、案 B でも**フィールドを落とさず**番兵
/// [`MISSING`] を置く（本モジュールの既定方針＝経路によって grep の当たり方を変えない）。
pub fn zorder_pair_strategy_line(strategy: ZOrderPairStrategy) -> String {
    let (plan, mechanism, raise_assist) = match strategy {
        ZOrderPairStrategy::OwnerLink { raise_assist } => {
            ("A", "owner-link", raise_assist.to_string())
        }
        ZOrderPairStrategy::ExplicitMaintenance => {
            ("B", "explicit-maintenance", MISSING.to_string())
        }
    };
    format!(
        "{ZORDER_PAIR_STRATEGY_TAG} plan={plan} mechanism={mechanism} raise_assist={raise_assist}"
    )
}

/// 実行時ストラテジ選択レコードを 1 行出力する（起動時 1 運転 1 本）。
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

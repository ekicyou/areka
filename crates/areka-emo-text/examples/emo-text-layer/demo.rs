use super::{
    ActorKey, AtlasTable, BalloonModel, DWriteMetrics, EmoPresenter, EmoTextSink, EmoWorld, Entity,
    Observations, Rc, RefCell, ResolvedBalloonText, TextLayerRuntime,
};

// ---------------------------------------------------------------------------
// Demo（NonSend・EmoPresenter/TextLayerRuntime を内包する集約ルート）
// ---------------------------------------------------------------------------

/// 起動〜シナリオ駆動の全状態（NonSend リソース・UI スレッド所有）。
pub(super) struct Demo {
    pub(super) presenter: EmoPresenter,
    /// \0（sakura）バルーン窓。
    pub(super) win0: Entity,
    /// \1（kero）バルーン窓。
    pub(super) win1: Entity,
    /// attach_target で move 消費するアセット（装着後 None）。
    pub(super) assets0: Option<(EmoWorld, AtlasTable)>,
    pub(super) assets1: Option<(EmoWorld, AtlasTable)>,
    /// 2 層マージ済み balloon model（両 actor 共通・fixture 由来）。
    pub(super) model: BalloonModel,
    /// 装着＋結線完了フラグ。
    pub(super) attached: bool,
    /// UI スレッド所有の集約ルート（sink ドレインと present_frame が共有）。
    pub(super) runtime: Rc<RefCell<TextLayerRuntime>>,
    /// cue 受信口（UI ドレインへ配送）。
    pub(super) sink: Option<EmoTextSink>,
    /// drain の join ハンドル（生存保持のみ）。
    pub(super) _drain: Option<wintf_winmsg_executor::JoinHandle<()>>,
    /// \0/\1 共通の解決済み layout 入力（writing_mode/region/font）。
    pub(super) resolved: Option<ResolvedBalloonText>,
    /// チェックポイントの純粋 layout 再導出に使う実測 metrics（描画と同一 probe 経路）。
    pub(super) metrics: Option<DWriteMetrics>,
    /// 供給面（validrect）の物理寸。
    pub(super) dims: (u32, u32),
    /// talk 開始のフレーム時刻（秒）。
    pub(super) talk_start: f64,
    /// 現在のステージ（0..7）。
    pub(super) stage: usize,
    /// 現ステージの cue を sink へ流し終えたか。
    pub(super) fed: bool,
    pub(super) obs: Observations,
    pub(super) finished: bool,
    /// `--hold`: シナリオ完了後も窓を閉じず talk をループ再生し、balloon 上でのダブルクリックで終了する
    /// （自動クローズしない目視確認モード・PASS/FAIL 自動判定は行わない）。
    pub(super) hold: bool,
    /// ダブルクリック検出用: 直前フレームの左ボタン押下状態（rising edge 抽出）。
    pub(super) prev_lbutton: bool,
    /// ダブルクリック検出用: 直近クリック（rising edge）のフレーム時刻（連続 2 クリックの間隔判定）。
    pub(super) last_click_time: f64,
}

/// sakura/kero の ActorKey（結線側が所有する actor→target 対応の鍵・R9.5）。
pub(super) fn actor0() -> ActorKey {
    ActorKey::from("0")
}
pub(super) fn actor1() -> ActorKey {
    ActorKey::from("1")
}

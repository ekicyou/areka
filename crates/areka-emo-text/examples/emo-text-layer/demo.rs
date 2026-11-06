use super::{
    ActorKey, AtlasTable, BalloonModel, DWriteMetrics, EmoPresenter, EmoTextSink, EmoWorld, Entity,
    Observations, Rc, RefCell, ResolvedBalloonText, TextLayerConfig, TextLayerRuntime, World,
    balloon_assets, balloon_model, balloon_size, create_balloon_window, decoder, info,
};

/// \0（sakura）バルーン窓の初期位置（物理 px・スクリーン座標）。
const SAKURA_POS: (i32, i32) = (320, 160);
/// \1（kero）バルーン窓の縦間隔（sakura 窓の直下に置く・物理 px）。
const KERO_GAP_Y: i32 = 32;

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
    /// 提示フレームの失敗を `error!` で残し終えたか（最初の 1 回だけ `error!`・以後は `warn!`）。
    pub(super) present_err_logged: bool,
    /// `--hold` の watchdog 超過を `error!` で残し終えたか（hold は窓を閉じないので 1 回だけ）。
    pub(super) watchdog_logged: bool,
}

/// sakura/kero の ActorKey（結線側が所有する actor→target 対応の鍵・R9.5）。
pub(super) fn actor0() -> ActorKey {
    ActorKey::from("0")
}
pub(super) fn actor1() -> ActorKey {
    ActorKey::from("1")
}

/// アセット構築・窓生成・`Demo` 挿入を一括で行う（UI スレッド・emo-present example と同型）。
///
/// `--vertical` は balloon descript の parse 入力だけを変種へ差し替える。
/// 枠アセットは \0/\1 target の 2 つを共有 fixture から作る。
pub(super) fn build_and_spawn(world: &mut World, vertical: bool, hold: bool) {
    let decoder = decoder();
    let model = balloon_model(vertical);
    let assets0 = balloon_assets(&decoder);
    let assets1 = balloon_assets(&decoder);
    let (w, h) = balloon_size(&assets0);

    let win0 = create_balloon_window(world, "sakura", SAKURA_POS.0, SAKURA_POS.1, w, h);
    let win1 = create_balloon_window(
        world,
        "kero",
        SAKURA_POS.0,
        SAKURA_POS.1 + h as i32 + KERO_GAP_Y,
        w,
        h,
    );

    world.insert_non_send(Demo {
        presenter: EmoPresenter::new(),
        win0,
        win1,
        assets0: Some(assets0),
        assets1: Some(assets1),
        model,
        attached: false,
        runtime: Rc::new(RefCell::new(TextLayerRuntime::new(
            TextLayerConfig::default(),
        ))),
        sink: None,
        _drain: None,
        resolved: None,
        metrics: None,
        dims: (0, 0),
        talk_start: 0.0,
        stage: 0,
        fed: false,
        obs: Observations::default(),
        finished: false,
        hold,
        prev_lbutton: false,
        last_click_time: -10.0,
        present_err_logged: false,
        watchdog_logged: false,
    });
    info!(
        w,
        h, vertical, hold, "emo-text-layer: 窓生成とアセット構築を完了（GPU 資源到達で装着）"
    );
}

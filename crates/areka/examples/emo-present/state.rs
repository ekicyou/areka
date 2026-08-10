use super::{
    AtlasTable, BindSet, Component, EmoPresenter, EmoWorld, Entity, PatternState, PresentCommand,
    TargetId,
};

// ---------------------------------------------------------------------------
// Marker Components
// ---------------------------------------------------------------------------

/// シェル窓を識別するマーカー（クリック透過登録・終了 despawn の標的）。
#[derive(Debug, Clone, Copy, Component, PartialEq, Hash)]
pub struct ShellWindowMarker;

/// バルーン窓を識別するマーカー。
#[derive(Debug, Clone, Copy, Component, PartialEq, Hash)]
pub struct BalloonWindowMarker;

// ---------------------------------------------------------------------------
// Surface cycle（R6.4 の切替観測: surface0 ⇄ surface1000[binds] ⇄ Hide）
// ---------------------------------------------------------------------------

/// シェル target のまばたき巡回状態（[`CYCLE_INTERVAL_SECS`] 周期で開閉し、各遷移で `apply` を 1 回発行）。
///
/// さくらスクリプト `\s[1000]\![bind,腕,組み,1]\![bind,紅,差し,0]\![bind,口,‥‥,1]\![bind,眉,悲しみ,1]`
/// `\![bind,目,通常,1]\![bind,まばたき,通常,1]` 相当を seriko 代役でハンドコンパイルした表情を、
/// **まばたきアニメーションを指令切替で手動再現**する形で表示する（アニメ＝SERIKO ループは seriko 別 spec・未実装）。
/// 共通表情（腕組み=1101・口‥‥=1206・目通常=1302・眉悲しみ=1502・髪飾りリボン=1800／紅なし）は据え置き、
/// まばたき通常（1400）の有無だけをトグルする（1400 は静止合成で閉じまぶた(1412)を乗せる＝目を閉じる）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum CycleState {
    /// 目開き: まばたき bind 無し（目通常 1302 の開き目）。
    EyesOpen,
    /// 目閉じ: まばたき通常 1400 を加える（閉じまぶた 1412 が乗る）。
    EyesClosed,
}

impl CycleState {
    /// 次の巡回状態へ進める（開き⇔閉じのトグル）。
    pub(super) fn next(self) -> Self {
        match self {
            CycleState::EyesOpen => CycleState::EyesClosed,
            CycleState::EyesClosed => CycleState::EyesOpen,
        }
    }

    /// この状態でシェル target（`TargetId(0)`）へ発行する指令を組む。
    ///
    /// 切替は必ず `EmoPresenter::apply`（指令 API）経由で行うため、状態ごとの `PresentCommand` を
    /// ここで一元的に定義する（bypass しない）。共通表情に対しまばたき（1400）のみ差分する。
    pub(super) fn command(self) -> PresentCommand {
        // 共通表情: 腕組み・口‥‥・目通常・眉悲しみ・髪飾りリボン（紅なし）。
        let binds = match self {
            CycleState::EyesOpen => BindSet::from_ids([1101, 1206, 1302, 1502, 1800]),
            CycleState::EyesClosed => BindSet::from_ids([1101, 1206, 1302, 1400, 1502, 1800]),
        };
        PresentCommand::ShowSurface {
            target: TargetId(0),
            surface_id: 1000,
            binds,
            pattern: PatternState::default(),
            reply: None,
        }
    }
}

// ---------------------------------------------------------------------------
// Boot resource（NonSend・EmoPresenter を内包）
// ---------------------------------------------------------------------------

/// 起動時に構築した presenter・窓・アセットを束ね、GPU 資源が揃うまで attach/apply を保留する
/// NonSend リソース（`EmoPresenter` が `!Send` ゆえ本型も NonSend）。
///
/// `boot_present_system` が GPU 資源（`GraphicsCore`/`WucGraphicsResource`）到達フレームで
/// `attach_target`→`apply(ShowSurface)` を各 target 高々 1 回駆動し、`attached` を立てる。
pub(super) struct EmoBoot {
    pub(super) presenter: EmoPresenter,
    pub(super) shell_window: Entity,
    pub(super) balloon_window: Entity,
    /// シェル target のアセット（`attach_target` で move 消費・装着後は `None`）。
    pub(super) shell_assets: Option<(EmoWorld, AtlasTable)>,
    /// バルーン target のアセット（同上）。
    pub(super) balloon_assets: Option<(EmoWorld, AtlasTable)>,
    /// 装着＋初回表示を済ませたか（毎フレームの remove/insert churn を避けるゲート）。
    pub(super) attached: bool,
    /// シェル target が装着され巡回対象となったか（未装着シェルでは巡回しない）。
    pub(super) shell_cycling: bool,
    /// シェル target の現在のまばたき状態（装着直後は `EyesOpen`＝surface1000 初回表示に一致）。
    pub(super) cycle_state: CycleState,
    /// 次の切替を行う `FrameTime` 絶対時刻（秒）。装着完了時に `now + CYCLE_INTERVAL_SECS` で確定する。
    pub(super) next_switch_at: f64,
}

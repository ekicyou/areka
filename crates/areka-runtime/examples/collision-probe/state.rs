use super::{AtlasTable, EmoPresenter, EmoWorld, Entity};

// ---------------------------------------------------------------------------
// Boot resource（NonSend・EmoPresenter を内包・donor と同型）
// ---------------------------------------------------------------------------

/// probe の駆動フェーズ。GPU 資源到達で表示→本番 resize→（次フレーム以降で）物理寸整合 assert、の順に
/// 進む。DPI 追従フェーズは持たない（実適用 k は初回表示時点で実モニタ DPI から確定する＝1 実行 1 水準）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ProbePhase {
    /// GPU 資源到達待ち。到達フレームで attach→表示→期待 k ゲート→描画一致 anchor→本番 resize を駆動する。
    WaitingAttach,
    /// 本番 resize 済み。**次フレーム以降**（SetWindowPos flush 後）に `GetClientRect` で物理寸整合を
    /// assert する。
    WaitingVerify,
    /// 自動 assert 完了。以降は手動プロトコル（⑤ 解決一致）を live ログしながら常駐する。
    Done,
}

/// 起動時に構築した presenter・装着待ちアセット・駆動フェーズを束ねる NonSend リソース
/// （`EmoPresenter` が `!Send` ゆえ本型も NonSend・donor `EmoBoot`/`PlacementBoot` と同作法）。
pub(super) struct ProbeBoot {
    /// 提示段の統括ハブ（scope0 shell target を装着・表示する）。
    pub(super) presenter: EmoPresenter,
    /// scope0 のキャラ窓 entity（`spawn_ghost_windows` が生成・placeholder 100×100 で spawn 済み）。
    pub(super) char_window: Entity,
    /// scope0 shell アセット（`attach_target` で move 消費・装着後は `None`）。
    pub(super) assets: Option<(EmoWorld, AtlasTable)>,
    /// 駆動フェーズ。
    pub(super) phase: ProbePhase,
    /// 現表示サーフェスの **native 原寸**（作者定義 px・WaitingAttach で確定・ログ用）。
    pub(super) native_size: (u32, u32),
    /// 本番 resize で適用した **k 適用後の物理寸**（`target_physical_size` の値・③ assert の期待側）。
    pub(super) physical_size: (u32, u32),
}

//! シェル・バルーンの切替の型。
//!
//! 本体を引かない型と Reference の組み立ての規則（[`skin_ref_name`]・[`skin_ref_path`]）を置く。
//! `runtime` の中はここを直に引き、要求の入口（`shell_balloon_switch.rs`）は呼び手のために出し直す。

use std::path::Path;
use std::sync::mpsc::Receiver;
use std::time::Instant;

use areka_actor::ReplyReceiver;
use areka_emo_present::PresentOutcome;
use areka_emo_present::balloon_definitions::ScopeBalloonDefinitions;
use areka_kanade::{MarkedEnd, TalkGap};
use areka_placement::reseed::BalloonPlacementInputs;
use areka_placement::source::DescriptSource;
use areka_sylphya::PersistKey;

use super::shell_balloon_resolve::SkinCandidate;
use super::shell_box_assets::ShellBoxAssets;
use super::shell_cursor_assets::ShellCursorAssets;
use super::switch_assets::{SwapBuilt, SwitchBuildError};

/// 切替の種別。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkinKind {
    /// シェル（今のゴーストの `shell/` の下）。
    Shell,
    /// バルーン（ベースウェアの根の `balloon/` の下）。
    Balloon,
}

/// 台本の `\![change,shell,名(,--option=raise-event)]`・`\![change,balloon,名]` から届く切替要求
/// （受け口 `SwitchCueSink` が talk スレッドから送る）。名前は台本の字面のまま（無変形）。
///
/// `raise_event` はシェルでだけ真になりうる（バルーンに「切り替え前」のイベントは無い）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkinRequestRaw {
    pub kind: SkinKind,
    pub name: String,
    pub raise_event: bool,
}

/// 切替先の指し方。台本は名前（`descript.txt` の `name` → フォルダ名の順）、メニューはフォルダ名で指す。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SkinSpec {
    Name(String),
    Folder(String),
}

/// 要求の出どころ（`OnShellChanging` を送るかを決める＝台本の `raise-event` とメニューのシェル）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkinOrigin {
    Script { raise_event: bool },
    Menu,
}

/// 入口へ渡す切替要求（台本もメニューもこの 1 本を通る）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkinRequest {
    pub kind: SkinKind,
    pub target: SkinSpec,
    pub origin: SkinOrigin,
}

/// 入口の判定（受理以外はどれも `warn!` を 1 件残して何もしない）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkinVerdict {
    Accepted,
    /// シェル・バルーンの切替が進行中（重ねない）。
    Busy,
    /// ゴースト切替が進行中。
    GhostSwitching,
    /// ネットワーク更新の実行中。
    Updating,
    /// 切替先が決まらない。
    NotFound,
    /// 起動の文脈・置き場のゴースト・結線・seriko の送り手のどれかが無い（kanade へ送れないときも）。
    NoContext,
}

/// 切替の進行中の印（World の NonSend・高々 1 つ）。在れば次の切替要求は断る。
///
/// 置くのは受理のときだけ。消すのは差し替えの相（完了・中止・取りやめ・失敗）と終了の片付け
/// （`areka_session::shell_balloon_switch::discard_for_exit`）。
pub struct SkinSwitchInFlight {
    pub kind: SkinKind,
    pub target: SkinCandidate,
    pub stage: SkinSwitchStage,
}

/// 切替の段（差し替えの相 `frame/switch.rs` が進める）。
pub enum SkinSwitchStage {
    /// 台詞の切れ目の返事と、背景の資産づくりの結果を待っている（どちらが先に来てもよい）。
    Waiting {
        /// 切れ目の返事の受け手（資産より先に `Reached` が届いたら、資産がそろったフレームで
        /// 送り直した待ちの受け手に替わる）。
        gap: Option<ReplyReceiver<TalkGap>>,
        /// 資産より先に届いた 1 度目の `Reached`（印の台詞の終わり方を記録に使う）。
        gap_result: Option<TalkGap>,
        /// 背景の資産づくりの結果の受け手（受け取ったら `built` へ移す）。
        build: Receiver<Result<SwapBuilt, SwitchBuildError>>,
        built: Option<SwapBuilt>,
    },
    /// seriko へ差し替えを頼み、置き換えの返信を待っている（全部そろえば差し替えの相の完了の段が
    /// 後始末する）。
    Committed {
        /// 差し替えの世代（置き場の荷物と seriko の合図を結ぶ）。
        epoch: u64,
        /// scope ごとの置き換えの返信の受け手。
        replies: Vec<(u32, ReplyReceiver<PresentOutcome>)>,
        /// 後始末に使う、荷物と seriko の定義に入らなかった残り。
        finish: SwapFinish,
        /// 1 度目の切れ目の返事の印の台詞の終わり方（記録用）。
        marked: Option<MarkedEnd>,
        /// 頼んだ時刻（UI スレッドが相の中で費やした時間 `swap_ms` の起点）。
        committed_at: Instant,
    },
}

/// 資産のうち、荷物（scope ごとの `EmoWorld`・アトラス）と seriko の定義に入らず、完了の後始末で
/// 使う残り。
pub enum SwapFinish {
    /// シェル: 新しいシェルの配置の値・走っているバルーンの配置の値・位置の記憶・箱の束・カーソル。
    Shell {
        source: DescriptSource,
        balloon: BalloonPlacementInputs,
        restored: Vec<(PersistKey, String)>,
        boxes: ShellBoxAssets,
        cursors: ShellCursorAssets,
    },
    /// バルーン: scope ごとの面ごとの定義の束と背景色・新しいバルーンの作者の DPI（`dpi`）。
    Balloon {
        scopes: Vec<(u32, ScopeBalloonDefinitions, (u8, u8, u8))>,
        author_dpi: u16,
    },
}

/// Reference の名前（`descript.txt` の `name`、無ければフォルダ名）。
pub fn skin_ref_name(candidate: &SkinCandidate) -> String {
    candidate
        .name
        .clone()
        .unwrap_or_else(|| candidate.folder.clone())
}

/// Reference のパス（絶対パスの文字列・絶対化できなければそのまま）。
pub fn skin_ref_path(dir: &Path) -> String {
    std::path::absolute(dir)
        .unwrap_or_else(|_| dir.to_path_buf())
        .display()
        .to_string()
}

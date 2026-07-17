//! 出力契約（sink）— 演者非依存の**単一**出力契約トレイトと relevance 単一権威。
//!
//! cue 再生ランタイム（`CuePlayer`）から演者への受け渡し口は、**どの演者にも実装できる
//! 単一のトレイト** [`CueSink`] へ統合する（旧 sakura の `SurfaceSink`/`TextSink` の
//! 2 トレイト分割を廃す）。配送は全 cue を登録 sink 群へ **broadcast** し、どの action を
//! 演じるかの選別は**演者側の relevance 判定**で行うため、型で出力先を 2 分岐する必要が
//! ない（役割分割は broadcast＋演者側 relevance の下では不要・D4/D7）。
//!
//! relevance 判定の権威は [`cue_target_of`] **単一**とし、全演者がこの分類器を共有する
//! （中央 router による事前振り分けに依存しない・R2.4/R11.3）。

use super::command::{CueCommand, CueTarget, TalkCue};

/// 演者非依存の**単一**出力契約。全表現者（seriko／emo-text／将来の演者）がこの 1 本を
/// 各自 1 実装として持つ。
///
/// `CuePlayer` は登録された全 [`CueSink`] へ cue を **broadcast** する。演者は受け取った
/// 任意の搬送体について、自分が action するか否かを**演者側の relevance 判定**
/// （[`cue_target_of`] が単一権威）で決める——中央 router が事前に振り分けはしない
/// （R2.4）。担当外の cue でも [`TalkCue::duration`] は honor する（action を無視しても
/// duration は落とさない・R2.3）。
///
/// `emit` は infallible（`Result` 化しない）。本番アダプタは送出失敗を `tracing::error!` で
/// 観測する（ログ規律は各演者実装の領分）。
pub trait CueSink {
    /// 1 発火を演者へ届ける（[`TalkCue`] は `at`・`actor`・`duration` 込み）。
    ///
    /// broadcast ゆえ演者は自分の担当外 cue も受け取る。action の選別は演者が
    /// [`cue_target_of`] で行い、担当外でも duration は honor する。
    fn emit(&mut self, cue: TalkCue);
}

/// `CueCommand` → 配送先スロットの relevance 分類（**単一権威**・R11.3）。
///
/// broadcast された全 cue のうち、どれが**どの演者の担当か**を判定する唯一の場所。
/// 全演者（seriko＝`Shell`／emo-text＝`Balloon`）がこの分類器を共有し、各自の action
/// ゲートをこの分類に一致させる（中央 router に依存しない・演者側 relevance）。
///
/// - `Emote` / `EntityRef` / `BalloonSurface` → [`CueTarget::Shell`]、
/// - `Text` / `NewLine` / `Clear` / `ClearAll` / `Choice` → [`CueTarget::Balloon`]、
/// - 分類不能（`Custom`）と、**どの演者の担当でもない `Wait`**（action を持たず duration
///   のみ）は `None`。
///
/// バルーン面切替（`BalloonSurface`）はサーフェス消費系＝表示系（seriko 行き）ゆえ
/// [`CueTarget::Shell`] へ分類する。文字状態機械（`Balloon`＝emo-text）へ流すのは
/// 誤配線。全域写像で `None`（配送不能）へは落ちない。
///
/// 明示的な variant ごとの match（catch-all を置かない）により、将来 variant を追加した
/// 際にコンパイラが本関数の網羅性を強制的に再検討させる（分類漏れを型で塞ぐ）。
pub fn cue_target_of(command: &CueCommand) -> Option<CueTarget> {
    match command {
        CueCommand::Emote { .. } => Some(CueTarget::Shell),
        CueCommand::EntityRef(..) => Some(CueTarget::Shell),
        CueCommand::BalloonSurface { .. } => Some(CueTarget::Shell), // 表示系＝seriko
        CueCommand::Text(..) => Some(CueTarget::Balloon),
        CueCommand::NewLine { .. } => Some(CueTarget::Balloon),
        CueCommand::Clear => Some(CueTarget::Balloon),
        // 全スコープ消去はテキスト表現者（バルーン）の担当（Clear と同系）。
        CueCommand::ClearAll => Some(CueTarget::Balloon),
        CueCommand::Choice { .. } => Some(CueTarget::Balloon),
        CueCommand::Custom { .. } => None,
        // Wait は action を持たない純粋な待ち＝どの演者の担当でもない（全員が action を
        // 無視し duration のみ honor する）。分類不能（Custom）とは別理由で `None` だが、
        // 「action する演者がいない」点で帰結は同じ。
        CueCommand::Wait => None,
    }
}

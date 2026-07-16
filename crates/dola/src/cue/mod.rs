//! cue モジュール — 離散コマンド配信エンジン基盤
//!
//! dola クレートの 2 つめのエンジン。連続値アニメ宣言エンジン（`DolaRuntime`）に対して、
//! 離散コマンドの時刻ベース配信を担う。
//!
//! # 提供する型
//!
//! | 型 | 役割 |
//! |---|---|
//! | `TimedSchedule<T>` | 0 ベース相対オフセットの汎用配信エンジン |
//! | `CueCommand` | データ系 7 バリアント演出コマンド |
//! | `RoutingCommand` | 配送制御 3 バリアント |
//! | `CuePayload` | CueSheet 記述時の統一型 |
//! | `CueSheet` | 相対時刻コマンド列（演出台本） |
//! | `ActorKey` / `CueTarget` / `EntityKey` / `Cue` | 演出ドメイン型 |
//! | `TalkCue` | 配送エンベロープ（`Cue` の実行時投影・serde 非依存） |
//! | `CueSink` | 演者非依存の単一出力契約トレイト（全表現者が各自 1 実装） |
//! | `cue_target_of` | cue→演者担当の relevance 分類（全演者共有の単一権威） |
//! | `CuePlayer` | cue 再生の受動的注入時刻ランタイム（状態機械・バリア seam・選択肢先積み・完了 horizon） |
//! | `CuePlayerState` / `PendingChoice` | ランタイムの再生状態・先積み選択肢 |

mod command;
pub mod runtime;
pub mod schedule;
pub mod sheet;
mod sink;

pub use command::{
    ActorKey, BarrierKind, Cue, CueCommand, CuePayload, CueTarget, EntityKey, RoutingCommand,
    TalkCue,
};
pub use runtime::{CuePlayer, CuePlayerState, PendingChoice};
pub use schedule::{Entry, TimedSchedule};
pub use sheet::{CueSheet, to_talk_schedule};
pub use sink::{CueSink, cue_target_of};

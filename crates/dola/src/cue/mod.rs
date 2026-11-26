//! cue モジュール — 離散コマンド配信エンジン基盤
//!
//! dola クレートの 2 つめのエンジン。連続値アニメ宣言エンジン（`DolaRuntime`）に対して、
//! 離散コマンドの時刻ベース配信を担う。
//!
//! # 提供する型
//!
//! | 型 | 役割 |
//! |---|---|
//! | `TimedSchedule<T>` | 起点と相対の時刻から、届いた項目を 1 件ずつ取り出す時刻の表（起点を置き直せる） |
//! | `Due<T>` / `OriginNotice` | 時刻の表から取り出した 1 件・起点の知らせ |
//! | `CueCommand` | データ系 12 バリアント演出コマンド |
//! | `CuePayload` | CueSheet 記述時の統一型 |
//! | `CueSheet` | 相対時刻コマンド列（演出台本） |
//! | `ActorKey` / `CueTarget` / `Cue` | 演出ドメイン型 |
//! | `TalkCue` | 配送エンベロープ（`Cue` の実行時投影・serde 非依存） |
//! | `PendingCue` | 時刻の表の中の、まだ取り出していない合図の中身（発火時刻を持たない） |
//! | `CueSink` | 演者非依存の単一出力契約トレイト（全表現者が各自 1 実装） |
//! | `CueProbe` / `CueFinding` | 合図の受け手が誤りを知らせる口と、その知らせ 1 件（中身は不透明な文字列） |
//! | `cue_target_of` | cue→演者担当の型レベル relevance 分類（全演者共有の単一権威） |
//! | `CuePlayer` | cue 再生の受動的注入時刻ランタイム（状態機械・バリア seam・選択肢先積み・完了 horizon） |
//! | `CuePlayerState` / `PendingChoice` | ランタイムの再生状態・先積み選択肢 |
//! | `CueError` | cue の層の誤り（台本の起点が押されていない） |

mod command;
pub mod runtime;
pub mod schedule;
pub mod sheet;
mod sink;
#[cfg(any(test, feature = "test-support"))]
pub mod test_support;

pub use command::{
    ActorKey, BarrierKind, Cue, CueCommand, CuePayload, CueTarget, PendingCue, TalkCue,
};
pub use runtime::{CueError, CuePlayer, CuePlayerState, PendingChoice};
pub use schedule::{Due, Entry, OriginNotice, TimedSchedule};
pub use sheet::{CueSheet, to_talk_schedule};
pub use sink::{CueFinding, CueProbe, CueSink, cue_target_of};

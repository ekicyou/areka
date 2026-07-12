//! areka-P0-emo2-boot 統合結線モジュール（M-boot: 「emo2 が起動して喋る」最初の可視結果）。
//!
//! 完成済み 5 トラックのエンジン（seriko／sakura／emo-present／emo-text／actor＋dola）を
//! 束ねて「動くアプリ」にする最後の一結線を所有する。新規機構は作らず、シェルアニメーション
//! 側の表示指令を表示層の指令へ変換するアダプタ 1 個＋各エンジンの結線＋二段の観測に徹する
//! （design.md「変更境界」・R10.4）。
//!
//! 依存方向（レイヤ規律・design.md「依存方向（レイヤ規律）」）:
//! `target_map`（純粋・std のみ）→ `adapter`（seriko/emo-present 型）→ `talk_clock`（sakura 型＋clock）
//! → `assets`（parsers/atlas/compose/seriko/emo-present）→ `frame`（bevy_ecs World・emo-present/emo-text 駆動）
//! → `main.rs`（全結線）。左のモジュールは右を import しない。
//!
//! 本ファイル群は Foundation タスク（tasks.md task 1）の骨格であり、各サブモジュールの
//! 機能実装は後続タスク（2〜6）が担う。

pub mod target_map;
pub mod adapter;
pub mod talk_clock;
pub mod assets;
pub mod frame;

/// 統合結線の構築時（load-time）失敗を観測可能化する誤り型（log-first・R7.3）。
///
/// 各段（mount／shell 読取＋parse／bake／balloon 組立／UI アクター spawn）の失敗を
/// `#[from]` 変換で集約し、呼び手（`wire_emo2_boot`）が `MountError::StartPointMissing` 系は
/// `warn!`・他は `error!` に分類して `LogSink`×2 フォールバック boot へ倒す（design.md
/// 「Error Categories and Responses」）。
///
/// 骨格段階ではバリアントを持たない。design のバリアント計画
/// （Mount／ShellRead／Bake／Balloon／SpawnUi …・`#[from]` 変換付き）は後続の
/// assets／frame 実装タスクで充填する。
#[derive(Debug, thiserror::Error)]
pub enum BootWiringError {}

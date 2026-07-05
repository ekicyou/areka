//! # areka-sakura — さくらスクリプト再生エンジン ④ sakura
//!
//! ## 責務
//!
//! SHIORI が返す Value（さくらスクリプト）を時間軸上で再生する **per-talk transient**
//! エンジン。上流 [`areka_parsers::sakura::parse`] が生成する `Instruction` 列を
//! dola cue ドメインの `CueSheet` へ**純粋コンパイル**し、`TimedSchedule` を注入時刻で
//! 駆動して 2 系統——`CueTarget::Shell`→seriko（⑤）／`CueTarget::Balloon`→emo
//! text-layer（⑥）——へ発火を届け、終端で `TalkDone` を kanade（③）へ返す。
//! 「emo2 が喋る」の "喋る" を成立させる装置である。
//!
//! ## 三層構成（compile / drive / sink）
//!
//! - **compile**（純粋コンパイル）: `&[Instruction]` → `CompiledTalk{sheet, end}`。
//!   clock・sink・talk_id・アクターを知らない決定的な写像層。
//! - **drive**（per-talk transient アクター駆動）: `CueSheet` を `TimedSchedule` へ載せ、
//!   注入時刻（`Tick(f64)`）で駆動し `CueTarget` で 2 sink へ振り分け、`TalkDone` を返す。
//! - **sink**（出力 trait＋mock）: `SurfaceSink`/`TextSink` の 2 trait と、決定的観測用の
//!   mock sink。
//!
//! 依存方向（左が上流・右向き import のみ許可）:
//! `contract` → `compile` → `sink` → `drive`。
//!
//! ## 写像の正本
//!
//! 本クレートは `Instruction` → cue ドメインの**写像（マッピング）の正本**である
//! （scope→`ActorKey`・`Surface`→`Emote`・テキスト系→`Text`/`NewLine`/`Clear`・`at`＝f64 秒）。
//! **cue の型自体の正本は dola** が所有し、本クレートは消費する。
//!
//! wintf には依存しない（headless）。`std::time::Instant` は本クレートに一切現れない。

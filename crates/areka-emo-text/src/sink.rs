//! # sink — sakura の cue 受信口（結線層）
//!
//! sakura の `TextSink` を実装する `EmoTextSink` と搬送 envelope `TextMsg` を担う。
//! `emit` は sakura drive の worker スレッド上で呼ばれ、cue を UI ドレインへ非ブロック
//! 送出する（`UiSender` が配送口そのもの）。
//!
//! **層規律**: 結線層。送信失敗（UI アクター停止後）は `tracing::error!` のみ・panic しない
//! （`emit` は infallible 契約・log-first）。

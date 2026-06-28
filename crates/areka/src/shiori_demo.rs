//! 実走デモドライバ（要件 6）。
//!
//! `reference_brain::shiori_create` で脳を取得し、`ShioriSession` で activate →
//! 即時／遅延の数往復 request → `poll_completions` 待ち合わせ → Raise 観測 → unload
//! までを駆動し、各経路を `tracing::info!` で観測する。フラグ／環境変数ゲートで
//! 起動を制御する（要件 6.8）。
//!
//! 本タスク（1.1）は宣言のみのスタブ。`run_demo()` 本体は後続タスク（3.x）で配線する。

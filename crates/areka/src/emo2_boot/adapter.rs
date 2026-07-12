//! 表示指令の変換と配送（薄い変換＋配送のみ・状態を持たない・R3.6）。
//!
//! `map_display_command`（`DisplayCommand`→`PresentCommand` の純変換・DD-5・`target_map` を利用）と
//! `PresentBridge`（seriko の `SurfaceOutput` 本番実装・`mpsc::Sender` へ非ブロック送出）を
//! 所有する。`ShowBalloon`／`HideBalloon` はバルーン表示対象へ `BindSet::default()` 付きで
//! 配送する（R5.1／R5.2）。送出失敗（受信端 drop）は `debug!`・非数値 scope の drop は
//! `warn!` で log-first 観測する（R3.7・design.md「Error Categories and Responses」）。
//!
//! 骨格のみ。実装は tasks.md task 2.4（`map_display_command`）／task 2.5（`PresentBridge`）が担う。

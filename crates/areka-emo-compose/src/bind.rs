//! `BindSet`: 呼び手が渡す有効 bind 集合（`Send` 所有・整列済み）。
//!
//! bindgroup 切替・blink 発火・着せ替えなどの動的状態管理は行わず、渡された静的集合のみを
//! 合成対象とする。集合内の順序は `animation-sort` → animation ID の2段規則に沿って整列される。

//! I/O 層: surface 原寸採寸（task 4 で実装）。
//!
//! emo-atlas/compose で surface 原寸（物理 px）を合成採寸する
//! （scope0=id0・scope1=id10・balloon=balloon_root の surface0）。
//! 失敗は `PlacementError::Measure`（log-first）。

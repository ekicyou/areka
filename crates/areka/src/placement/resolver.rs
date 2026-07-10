//! 純粋 resolver（task 2 で実装）。
//!
//! 物理 px 値型（`RectPx`/`PointPx`/`SizePx`）で閉じたカスケード適用・
//! scope 相対配置・クランプ・balloon 暫定 offset・`virtual_desktop_union`。
//! std のみに依存し wintf 型を import しない（DPI パラメタ化単体テストの前提）。

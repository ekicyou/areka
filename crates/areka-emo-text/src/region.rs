//! # region — バルーン座標の画像空間解決と DPI/スケール契約（純粋層）
//!
//! origin／wordwrappoint／validrect の「負値=反対辺基準」解決・origin クランプ正準・
//! `TextRegion`／`ScaleContract`（画像座標空間と物理座標空間の 2 空間のみ・論理 px 不在）を担う。
//!
//! **層規律**: 純粋層——`windows` 系 crate への依存を一切持たない（決定論檻）。

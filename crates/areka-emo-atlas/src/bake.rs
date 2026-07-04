//! Baker: 頁バッファ確保・stride 決定・トリム矩形 blit（`blit_trimmed`）。
//!
//! 要件 **R4.3 / R6.3**。
//!
//! packing で算出した配置座標に従い、premultiplied BGRA の頁バッファ
//! （[`crate::table::AtlasPage`]）を確保し stride を決定して、トリム済み矩形を
//! 各頁へ blit する。空エントリ（全透明）は転写をスキップする。
//!
//! （本タスクは雛形。実装は後続タスクで追加する。）

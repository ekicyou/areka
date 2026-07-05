//! 正規化 Surface 定義の公開形: `SurfaceMaster` / `NormalizedElement` / `Transform`。
//!
//! 下流（seriko・collision-geometry）が再パース・再展開なしに消費できる正規化定義を提供する。
//! element（レイヤ・画像パス・座標）に加え collisions・animations・`animation-sort`／
//! `collision-sort` の順序キーを保持する。element 配置は変換行列（`Transform`）で表現し、
//! X,Y のみの平行移動を単位行列の特例として扱う（回転・拡縮は行列表現に予約）。

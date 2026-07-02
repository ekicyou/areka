//! balloon モデル型（I/O 契約の正本）。
//!
//! 後続タスク 2.1 で `BalloonModel` および幾何・フォント sub-struct を定義する。
//! 座標成分・色成分は個別に `Option<T>` を直持ちし「未指定（0 と区別）」を型で表す。
//! 全公開 struct に `#[non_exhaustive]`、read-only accessor を付す。

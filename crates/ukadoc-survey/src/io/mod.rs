//! 入出力層の束ね。
//!
//! ワークスペース根の解決・ファイルの読み書き・ソースの走査・スナップショットの
//! 読み込みだけを担い、判断は一切持たない（設計 Architecture）。
//! 一時ディレクトリは使わない（設計 File Structure Plan）。

pub mod files;
pub mod paths;
pub mod snapshot;
pub mod sources;

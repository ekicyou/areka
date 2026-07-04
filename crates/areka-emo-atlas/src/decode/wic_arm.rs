//! 既定デコード腕＝WIC（COM 隔離・`windows` の WIC 型を直接利用）。
//!
//! 設計決定 **D4**（要件 **R2.1, R2.2**）。
//!
//! `ElementDecoder` の既定実装。`windows` クレートの WIC 経路
//! （`CreateDecoderFromFilename → GetFrame(0) → IWICFormatConverter で
//! GUID_WICPixelFormat32bppPBGRA へ変換 → CopyPixels`）で PBGRA raw バッファを
//! 抽出する（wintf の `load_bitmap_source` 相当・本層は wintf 本体へ依存せず
//! `windows` を直接利用）。COM 依存は本モジュールに隔離し、MTA/COM 規律に従う。
//! 変換前フレームのピクセルフォーマット由来の α 有無を `DecodedImage` へ確定する。
//!
//! （本タスクは雛形。実装は後続タスク 2.2 で追加する。）

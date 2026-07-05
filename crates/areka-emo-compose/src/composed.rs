//! `ComposedSurface`: 合成コアの出力契約（premultiplied BGRA・size・stride・`Send` 所有）。
//!
//! 通信機構（channel・async）を介さず値・共有参照として直接返す出力型。surface id→合成結果の
//! キャッシュ・無効化は持たない（それは `emo-present` の責務）。全透明の退化結果も外形どおりの
//! `ComposedSurface` として正常表現する。

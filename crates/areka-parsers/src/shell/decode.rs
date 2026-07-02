//! decode — 意味層。
//!
//! 後続タスク 4.x で構文トークン → 値正規化済みモデルへの変換を実装する。
//! animationN 集約・append 範囲の記述子保持・alias 写像・subset 値正規化を担う。
//! subset のみ値化し、subset 外は passthrough シームへ委ねる寛容方針を採る。

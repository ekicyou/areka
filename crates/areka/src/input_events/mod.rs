//! UI→kanade のマウス入力配信配線（areka-P0-input-events）。
//!
//! キャラ窓のポインタイベントを捉え、当たり判定名を collision-geometry の resolver で解決し、
//! 送出間引き（[`throttle`]）を通して kanade へマウス入力メッセージとして配信する薄い配線層。
//!
//! 本モジュールは現状 [`throttle`]（送出間引きの純粋・決定的判定・task 2.4）のみを収める。
//! per-scope 間引き状態を `HashMap` で保持する `MouseWiring` とポインタハンドラ結線は
//! task 2.6／2.7 で本 mod へ増設される。

pub(crate) mod throttle;

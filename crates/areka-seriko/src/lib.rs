//! SERIKO アニメーションエンジン (⑤)。
//!
//! サーフェスの解決・状態・発行・構築・アクターの各責務をモジュールへ分割する土台。
//! 公開 re-export は後続の結合タスク（3.2）でまとめて追加するため、ここでは
//! 非公開モジュール宣言のみを置く。
#![allow(dead_code)]

mod resolve;
mod state;
mod output;
mod actor;
mod bind;

// 問い 2 の案 2 の emo-text 役: 自分の束縛を持たず、wintf 役が公開した束縛の型で描く。
use q2_plan2_wintf::*;
use windows_core::*;

#[path = "../../shared/draw_body.rs"]
mod draw_body;
pub use draw_body::{TEXT, count_ink, draw_line};

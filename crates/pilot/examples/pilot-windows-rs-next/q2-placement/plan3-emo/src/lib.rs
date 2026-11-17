// 問い 2 の案 3 の emo-text 役: 共有の束縛の型で、受け取った ID2D1DeviceContext に描く。
use q2_plan3_bind::*;
use windows_core::*;

#[path = "../../shared/draw_body.rs"]
mod draw_body;
pub use draw_body::{TEXT, count_ink, draw_line};

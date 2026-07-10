//! # surface — 自前 swapchain 供給面（COM 層）
//!
//! `TextSurface`（自前 swapchain・text_slot Visual への brush 装着・提示・決定論検証用
//! readback）を担う。装着後のグリフ更新は Present のみで完結し、バルーン surface 本体の
//! 再合成（emo-compose 再駆動）を強要しない。
//!
//! **層規律**: COM 層——UI スレッド専有。`windows`（DXGI/WUC）を触るのは
//! 本モジュールと draw のみ。失敗は log-first（`tracing::error!`＋`Err`）で扱い panic しない。

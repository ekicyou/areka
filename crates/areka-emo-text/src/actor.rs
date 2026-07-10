//! # actor — UI ドレインとフレーム提示ステップ（結線層）
//!
//! `spawn_emo_text`（`spawn_ui` 結線・UI ドレイン起動）・`TextLayerRuntime`
//! （UI スレッド所有の集約ルート）・`TextSlotBinding`・`present_frame`
//! （毎フレームの注入時刻駆動：リビール進行→レイアウト→描画→装着）を担う。
//!
//! **層規律**: 結線層。終了経路はちょうど 2 つ——`TextMsg::Close` 受領＝`Ok(Break)`、
//! 全 `UiSender` drop＝drain 正常終了（いずれも error ログなし）。個別メッセージの処理失敗は
//! `Err` 戻し→基盤が `error!`＋継続（log-first・ループを殺さない）。

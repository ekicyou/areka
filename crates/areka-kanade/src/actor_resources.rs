//! 殻で答える複数件のリソース照会（`KanadeMsg::ResourceQuery`・要件 3.2〜3.4／3.10）。
//!
//! UI（右クリックメニュー）が任意の時点で SHIORI リソースを複数件引くための入口である。
//! 運行状態機械（[`crate::schedule::step`]）を**経ない**——`actor.rs` の閉包が停止指示
//! （`KanadeMsg::Close`）と同じ並びで本モジュールの [`answer`] を呼び、その場で答えて次の
//! メッセージへ進む。状態機械を経ないので、`drive_translating` の「バッチ最後の応答だけを再投入する」
//! 規律に乗らず、複数件の応答が 1 件も落ちない。運行状態は読むだけで書き換えない。
//!
//! # 会話できる状態のときだけ往復する（要件 3.10）
//! [`queryable`] が真（`Phase::Steady`）のときだけ SHIORI へ往復する。起動中・終了中・停止後は
//! SHIORI へ 1 通も送らず、全件 [`ResourceOutcome::NoContent`] を返す（受け手は既定名で出す）。
//! 起動系列・終了系列の SHIORI 呼出順（固定の運行表）へ照会を割り込ませないためである。
//!
//! # 送出の前にリソース許可表で断る・最後の檻は送出点
//! 往復は [`crate::actor::round_trip_request`] を通す。その前に、リソース許可表（`ALLOWED_RESOURCE_IDS`）に
//! 無い id は SHIORI へ送らずに断る（`error!`）。送出の檻はイベント許可表との論理和で見るので、
//! イベントの id（`OnBoot` など）がリソースの GET として出ていかないよう、ここで先に止める。
//! 断った id だけが [`ResourceOutcome::Failed`] になり、残りの id は答える。

use std::sync::mpsc::Sender;

use areka_actor::ReplySender;

use crate::actor::round_trip_request;
use crate::msg::ShioriMsg;
use crate::schedule::resources::{
    ResourceOutcome, is_allowed_resource_id, outcome_of, resource_get,
};
use crate::schedule::{Phase, State};

/// いま SHIORI へリソースを問い合わせてよいか（純粋関数）。
///
/// 会話できる状態（`Steady`・会話の有無を問わない）だけが真。起動系（`Idle`〜`BootVersion`）・
/// 終了握手（`ClosePending`／`CloseTalkWait`）・切替の 4 相（`ChangePending`／`ChangeTalkWait`／
/// `ChangeClosePending`／`ChangeCloseTalkWait`）・`Unloading`・`Stopped` は偽（要件 3.10）。
fn queryable(phase: &Phase) -> bool {
    matches!(phase, Phase::Steady { .. })
}

/// 照会に答える。応答は入力の id と同じ順・同じ長さで **1 回だけ**返す。
///
/// Status ヘッダは現在の運行状態から導出する（会話中なら `talking`）。受信側が待ちを諦めて
/// 受信端を捨てていても落ちない（送出失敗は `debug!` を残して捨てる——異常ではない）。
pub(crate) fn answer(
    state: &State,
    shiori: &Sender<ShioriMsg>,
    ids: Vec<&'static str>,
    reply: ReplySender<Vec<(&'static str, ResourceOutcome)>>,
) {
    let can_query = queryable(&state.phase);
    if !can_query {
        tracing::debug!(
            target: "areka_kanade::resource",
            event = "resource_query_not_steady",
            count = ids.len(),
            "会話できる状態でない——SHIORI へ送らず全件 NoContent を返す"
        );
    }
    let snapshot = state.snapshot();
    let answers: Vec<(&'static str, ResourceOutcome)> = ids
        .into_iter()
        .map(|id| {
            let outcome = if !can_query {
                ResourceOutcome::NoContent
            } else if !is_allowed_resource_id(id) {
                tracing::error!(
                    target: "kanade",
                    event = "resource_id_not_allowed",
                    id = %id,
                    "リソース許可表に無い id——SHIORI へ送らずその id だけ失敗として返す"
                );
                ResourceOutcome::Failed(format!("resource_id_not_allowed: {id}"))
            } else {
                outcome_of(id, round_trip_request(shiori, resource_get(id, &snapshot)))
            };
            (id, outcome)
        })
        .collect();
    if reply.send(answers).is_err() {
        tracing::debug!(
            target: "areka_kanade::resource",
            event = "resource_query_reply_dropped",
            "照会の受信側は既に待ちを諦めている——応答を捨てる"
        );
    }
}

#[cfg(test)]
#[path = "actor_resources_tests.rs"]
mod tests;

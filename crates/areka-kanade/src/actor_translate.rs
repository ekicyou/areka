//! 殻が翻訳の依頼（`Action::Translate`）を実行する（要件 2.1・2.2・4.1・4.4・7.3）。
//!
//! 順序は「展開 → `OnTranslate` の組み立て → 生の往復 → 応答の読み → MAKOTO の口」。展開と
//! MAKOTO の口は外から渡された関数（[`TranslateSeams`]）で、kanade のスレッドで同期に呼ぶ。
//! 往復は送出の檻（[`round_trip_raw`]）を通すが、エラー応答の 204 への写しは通さない。エラー応答は
//! 応答の読み（[`read_reply`]）が出どころの表示つきで 1 件だけ記録し、元の台詞で進む。

use std::sync::mpsc::Sender;

use crate::actor::round_trip_raw;
use crate::msg::ShioriMsg;
use crate::schedule::events::{on_translate, on_translate_sstp};
use crate::schedule::translate::{
    ReplyReading, ScriptSource, TranslateRequest, TranslateResult, read_reply,
};
use crate::translate::TranslateSeams;

/// 展開 → `OnTranslate` の往復 → 応答の読み → MAKOTO の口、の順に実行する。
///
/// `OnTranslate` の組み立ては出どころで選ぶ: イベントへの応答は [`on_translate`]（今日のまま・
/// 要件 4.5・4.6）、外からの台本は [`on_translate_sstp`]（Reference1＝`sstp-send,owned`・要件 4.4）。
///
/// Status は依頼に載った値（運行表が捕まえた時点の状態）をそのまま送る。MAKOTO の口は応答の
/// 種類（200・204・エラー応答）に依らず（台詞, [`ScriptSource::makoto_id`]）で 1 回呼ぶ（要件 7.3）。
/// 輸送路の失敗だけは口を呼ばずに `Err` で返す（故障の判断と記録は運行表）。
pub(crate) fn run_translate(
    request: TranslateRequest,
    shiori: &Sender<ShioriMsg>,
    seams: &TranslateSeams,
) -> TranslateResult {
    let TranslateRequest {
        script,
        source,
        status,
    } = request;
    let expanded = (seams.expand)(&script);
    let call = match &source {
        ScriptSource::Event(event) => on_translate(&expanded, event, status),
        ScriptSource::SstpSend => on_translate_sstp(&expanded, status),
    };
    let outcome = round_trip_raw(shiori, call);
    match read_reply(&source, expanded, outcome) {
        ReplyReading::Proceed(text) => Ok((seams.makoto)(&text, source.makoto_id())),
        ReplyReading::Failed(failure) => Err(failure),
    }
}

#[cfg(test)]
#[path = "actor_translate_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "actor_translate_origin_tests.rs"]
mod origin_tests;

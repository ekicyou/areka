//! `\![get,property,イベント名,プロパティ名…]` の受け口（spec `shell-scale-linkage` 要件 1・2）。
//!
//! broadcast された全 cue のうち、名前 [`PROP_GET_CUE_NAME`]・第 1 引数 [`PROP_GET_SELECTOR`] の
//! キャリアだけを選び、各名前の値を MCP の `get_property` と同じ問い手・同じ関数
//! （[`SylphyaReader::resolve_dotted_str`]）で引いて、kanade へ返しのイベント
//! （[`KanadeMsg::PropertyEvent`]）を 1 回だけ頼む。送出は待たず、再生を止めない（要件 1.6）。
//! 段の分け方は [`crate::prop_sink::PropSetCueSink`] と同じ（自分宛の非正準は warn・担当外は debug）。

use std::sync::mpsc::Sender;

use areka_kanade::KanadeMsg;
use areka_kanade::sstp::validate_notify;
use areka_sakura::contract::{CueCommand, CueSink, TalkCue};
use areka_sylphya::{AskerContext, AskerId, DottedResolution, SylphyaReader};
use tracing::{debug, warn};

/// `\![get,property,…]` の運搬名。
pub const PROP_GET_CUE_NAME: &str = "get";
/// `\![get,property,…]` の選別子（第 1 引数）。
pub const PROP_GET_SELECTOR: &str = "property";

/// `\![get,property]` の台本のスレッド側の受け口（design「PropGetCueSink」）。
///
/// 受け口は許可の表も `On` 接頭も見ない（要件 2.1）。dispatcher が talk ごとに clone するので、
/// 中身はどれも clone できる送信端・読み口だけにする。
#[derive(Clone)]
pub struct PropGetCueSink {
    reader: SylphyaReader,
    asker: AskerContext,
    kanade: Sender<KanadeMsg>,
}

impl PropGetCueSink {
    /// その実行系の読み口・問い手（`ghost_asker_id`）・kanade の送出端から作る。
    pub fn new(reader: SylphyaReader, asker: AskerId, kanade: Sender<KanadeMsg>) -> Self {
        Self {
            reader,
            asker: AskerContext { asker },
            kanade,
        }
    }
}

impl CueSink for PropGetCueSink {
    fn emit(&mut self, cue: TalkCue) {
        // 1) キャリア抽出。宛名が自分（`get`）の非正準は warn、ほかは担当外として debug。
        let Some((name, tokens)) = cue.command.as_command_carrier() else {
            match &cue.command {
                CueCommand::Custom { command, .. } if command == PROP_GET_CUE_NAME => warn!(
                    command = ?cue.command,
                    "PropGetCueSink: 自分宛（{PROP_GET_CUE_NAME}）の非正準 Custom params を良性スキップ"
                ),
                _ => debug!(
                    command = ?cue.command,
                    "PropGetCueSink: 非キャリア／他人宛の cue を良性スキップ（担当外）"
                ),
            }
            return;
        };

        // 2) 名前と第 1 引数で選ぶ。
        // ukadoc: https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_5c_21_5bget_2cproperty_2c_30a4_30d9_30f3_30c8_540d_2c_30d7_30ed_30d1_30c6_30a3_540d_2c_30d7_30ed_30d1_30c6_30a3_540d_2c:1
        if name != PROP_GET_CUE_NAME || tokens.first() != Some(&PROP_GET_SELECTOR) {
            debug!(
                name,
                tokens = ?tokens,
                "PropGetCueSink: 担当外の命令を良性スキップ（名前と第 1 引数で選別）"
            );
            return;
        }

        // 3) イベント名が無い・空・空白だけは送らない（要件 2.2）。
        let event = match tokens.get(1) {
            Some(event) if !event.trim().is_empty() => *event,
            _ => {
                warn!(
                    event = "property_get_empty_event",
                    tokens = ?tokens,
                    "PropGetCueSink: イベント名が無い・空・空白だけのため送らない"
                );
                return;
            }
        };

        // 4) 名前が 1 つも無いときは値を引く前に打ち切る（要件 2.4）。
        let names = &tokens[2..];
        if names.is_empty() {
            debug!(
                event = "property_get_no_names",
                name = ?event,
                "PropGetCueSink: プロパティ名が無いため送らない"
            );
            return;
        }

        // 5) MCP の get_property と同じ問い手・同じ関数で引く。無い値は空にして番号を保つ（要件 1.1・1.4）。
        let references: Vec<String> = names
            .iter()
            .map(
                |key| match self.reader.resolve_dotted_str(&self.asker, key) {
                    DottedResolution::Value(value) => value,
                    DottedResolution::NotFound => String::new(),
                },
            )
            .collect();

        // 6) イベント名か値に CR・LF・NUL があれば送らない（要件 2.3・空は 3) で弾き済み）。
        if validate_notify(event, &references).is_err() {
            warn!(
                event = "property_get_control_char",
                name = ?event,
                references = ?references,
                "PropGetCueSink: イベント名か値に改行・NUL があるため送らない"
            );
            return;
        }

        // 7) kanade へ 1 回だけ頼み、待たない（要件 1.6・2.5）。
        let count = references.len();
        let msg = KanadeMsg::PropertyEvent {
            id: event.to_string(),
            references,
        };
        if self.kanade.send(msg).is_ok() {
            debug!(
                event = "property_get_sent",
                name = ?event,
                count,
                "PropGetCueSink: 返しのイベントを kanade へ頼んだ"
            );
        } else {
            warn!(
                event = "property_get_kanade_gone",
                name = ?event,
                "PropGetCueSink: kanade が止まっているため返しのイベントを送れない"
            );
        }
    }
}

#[cfg(test)]
#[path = "prop_get_sink_tests.rs"]
mod prop_get_sink_tests;

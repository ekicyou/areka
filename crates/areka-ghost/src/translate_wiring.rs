//! 翻訳（`OnTranslate`）のための写しの源の結線（spec `areka-P0-translate-pipeline`）。
//!
//! kanade は翻訳の前に台詞の環境変数を展開する（要件 2.1）。展開の値は再生と同じ写しの源
//! （[`SystemVarSource`]）から読む（要件 2.3）。ここでは、写しの源から kanade へ渡す展開の関数
//! （[`ScriptExpander`]）を組む関数と、注入された 1 つの源を翻訳用と再生用の 2 つに分ける関数を置く。

use std::sync::{Arc, Mutex};

use areka_kanade::ScriptExpander;
use areka_sakura::contract::SystemVarSnapshot;

use crate::runtime::SystemVarSource;

/// 写しの源から展開の関数を組む（呼ばれるたびに写しを 1 回読む）。
pub(crate) fn make_script_expander(source: SystemVarSource) -> ScriptExpander {
    Box::new(move |script| areka_sakura::expand_system_vars(script, &source()))
}

/// 注入された 1 つの源を、翻訳用と再生用の 2 つに分ける（同じ源を順に呼ぶ）。
///
/// 源は `Fn` だが `Sync` ではないので、2 つのスレッド（kanade と dispatcher）から呼べるよう
/// 排他に包んで共有する。
pub(crate) fn split_source(source: SystemVarSource) -> (SystemVarSource, SystemVarSource) {
    let translate = Arc::new(Mutex::new(source));
    let playback = Arc::clone(&translate);
    (
        Box::new(move || read_shared(&translate)),
        Box::new(move || read_shared(&playback)),
    )
}

/// 共有した源から写しを 1 回読む。
///
/// 源が前の読みの途中で panic して排他が壊れていたら、記録を残して空の写しで進む
/// （`username` は既定値になる・壊れた源の向こうは読まない）。
fn read_shared(shared: &Mutex<SystemVarSource>) -> SystemVarSnapshot {
    match shared.lock() {
        Ok(source) => source(),
        Err(_) => {
            tracing::error!(
                target: "areka_ghost",
                event = "translate_snapshot_poisoned",
                "system var source mutex poisoned; continuing with an empty snapshot"
            );
            SystemVarSnapshot::default()
        }
    }
}

#[cfg(test)]
#[path = "translate_wiring_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "translate_wiring_e2e_tests.rs"]
mod e2e_tests;

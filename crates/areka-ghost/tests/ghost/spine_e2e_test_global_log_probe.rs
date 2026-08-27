// ===================== 全スレッド横断ログ捕捉（S7 支援・task 8.6） =====================
//
// `boot_gate skip_first_boot`（要件 3.3・areka-kanade/src/schedule/boot.rs）は **kanade アクター
// スレッド**上で発火するため、spine の kanade 統合テストが使う呼出スレッド局所の捕捉窓
// （areka-ghost/src/test_log_capture.rs は lib 内 `#[cfg(test)]` ゆえ統合テストからは不可視・
// かつスレッド局所）では捕えられない。本モジュールは共有 crate `log-capture-kit` の全スレッド
// 窓口 [`log_capture_kit::install_global_capture_all`] で全スレッドのイベントを 1 本の共有
// バッファへ集約する（据え付けは**一度だけ**・取り消せない）。S7 が boot 前後の追加分を照合する。
//
// フィルタ無しの capture-all ゆえ全 callsite の Interest は always に保たれ（`Never` 焼き付き
// 不能）、他 e2e シナリオの意味論には一切干渉しない——バッファへ push するだけの観測系である。
// 自前の `set_global_default` はもう持たない（spec: areka-P0-test-cage-determinism・要件 1.5／
// 1.6／2.2）。他所が先に全体設定を置いていれば、窓口は黙って縮退せず明示的に panic する。

use std::sync::{Arc, Mutex};

use log_capture_kit::{CapturedEvent, install_global_capture_all};
use tracing::Level;

/// 捕捉した 1 行（照合対象は target／level／message 本文）。
///
/// 共有バッファが貯めるのは正準イベント [`CapturedEvent`] で、本型はそこからの**変換**である
/// （移行前に本モジュールが自前の Layer で組んでいた 3 項目と同じ内容）。
#[derive(Clone, Debug)]
pub struct CapturedLine {
    pub target: String,
    pub level: Level,
    pub message: String,
}

impl From<&CapturedEvent> for CapturedLine {
    /// `message` は `fmt::Arguments` ゆえ `record_debug` 経路で届き、その `{:?}` は整形済みの
    /// 本文そのもの（引用符なし）になる。欠落時に空文字になるのも移行前と同じ。
    fn from(ev: &CapturedEvent) -> Self {
        Self {
            target: ev.target.clone(),
            level: ev.level,
            message: ev.message().to_string(),
        }
    }
}

/// capture-all 捕捉先をプロセスへ一度だけ据え付け、共有バッファのクローンを返す。
///
/// 2 回目以降の呼出は同じバッファを返すのみ（全体設定はプロセスで高々 1 回）。別の全体設定が
/// 先に置かれていた場合は黙って縮退せず panic する（log-first）——ghost テストバイナリでは
/// 本 probe が唯一の全体設定である前提。
pub fn install() -> Arc<Mutex<Vec<CapturedEvent>>> {
    install_global_capture_all()
}

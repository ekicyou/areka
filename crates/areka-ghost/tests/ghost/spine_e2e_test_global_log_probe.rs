// ===================== 全スレッド横断ログ捕捉（S7 支援・task 8.6） =====================
//
// `boot_gate skip_first_boot`（要件 3.3・areka-kanade/src/schedule/boot.rs）は **kanade アクター
// スレッド**上で発火するため、spine の kanade 統合テストが使うスレッドローカル `with_default`
// 捕捉（areka-ghost/src/test_log_capture.rs は lib 内 `#[cfg(test)]` ゆえ統合テストからは不可視・
// かつ thread-local）では捕えられない。本モジュールは `set_global_default` で全スレッドのイベントを
// 1 本の共有バッファへ集約する **capture-all** subscriber を **一度だけ**常駐させ（`OnceLock`）、S7 が
// boot 前後の追加分を照合する。フィルタ無しの capture-all ゆえ全 callsite の Interest は always に
// 保たれ（`Never` 焼き付き不能・kanade/sylphya の interest-keeper と同じ帰結）、他 e2e シナリオの
// 意味論には一切干渉しない——バッファへ push するだけの観測系である。

use std::sync::{Arc, Mutex, OnceLock};

use tracing::field::{Field, Visit};
use tracing::{Event, Level, Subscriber};
use tracing_subscriber::layer::{Context, Layer};
use tracing_subscriber::prelude::*;
use tracing_subscriber::registry::LookupSpan;

/// 捕捉した 1 行（照合対象は target／level／message 本文）。
#[derive(Clone, Debug)]
pub struct CapturedLine {
    pub target: String,
    pub level: Level,
    pub message: String,
}

/// `message` フィールド（マクロ本文＝`fmt::Arguments`）を `Debug` 経路で取り出す訪問子。
struct MessageVisitor {
    message: String,
}

impl Visit for MessageVisitor {
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            self.message = format!("{value:?}");
        }
    }
}

/// 共有バッファへイベントを積む capture-all Layer（per-layer filter 無し）。
struct GlobalCaptureLayer {
    sink: Arc<Mutex<Vec<CapturedLine>>>,
}

impl<S> Layer<S> for GlobalCaptureLayer
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        let mut visitor = MessageVisitor {
            message: String::new(),
        };
        event.record(&mut visitor);
        let meta = event.metadata();
        self.sink
            .lock()
            .expect("global capture sink mutex は毒化しない")
            .push(CapturedLine {
                target: meta.target().to_string(),
                level: *meta.level(),
                message: visitor.message,
            });
    }
}

/// プロセスグローバル共有バッファ（一度だけ確立・以後常駐）。
static BUFFER: OnceLock<Arc<Mutex<Vec<CapturedLine>>>> = OnceLock::new();

/// capture-all subscriber をプロセスへ一度だけ常駐させ、共有バッファのクローンを返す。
///
/// 2 回目以降の呼出は `OnceLock` のキャッシュを返すのみ（`set_global_default` は高々 1 回）。
/// 別の global subscriber が先に設定されていた場合は `.expect()` で大声で panic する
/// （silent 縮退なし・log-first）——ghost テストバイナリでは本 probe が唯一の global である前提。
pub fn install() -> Arc<Mutex<Vec<CapturedLine>>> {
    BUFFER
        .get_or_init(|| {
            let sink: Arc<Mutex<Vec<CapturedLine>>> = Arc::new(Mutex::new(Vec::new()));
            let subscriber =
                tracing_subscriber::registry().with(GlobalCaptureLayer { sink: sink.clone() });
            tracing::subscriber::set_global_default(subscriber).expect(
                "ghost テストバイナリでは S7 の global log probe より先に別の global subscriber を \
                 設定してはならない: capture-all の常駐が boot_gate ログを全スレッドから捕える前提\
                 （他の統合テストは set_global_default / init を行わない）",
            );
            // 確立以前に焼き付いた callsite の再評価保険（コスト 0・意図の自己文書化）。
            tracing::callsite::rebuild_interest_cache();
            sink
        })
        .clone()
}

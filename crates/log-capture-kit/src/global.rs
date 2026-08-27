//! 全スレッド横断の一回限りの捕捉。`set_global_default` の唯一の呼出点。
//!
//! # 既定の捕捉窓との違い
//!
//! 既定の [`crate::capture`] は**呼出スレッドの**既定 dispatcher を窓の間だけ差し替える。
//! 窓を抜ければ元へ戻り、他スレッドで発火したイベントは入らない（要件 3.6）。テストの大半は
//! これで足りる。
//!
//! これに対し本 module の窓口は、プロセス全体の既定 dispatcher を**一度だけ**置く。用途は
//! ただ一つ、**別スレッドで発火するログを捕える**ことである（アクタースレッドで発火する
//! `warn!` を統合テストから照合する、など）。置いた設定はプロセスが終わるまで残り、取り消せ
//! ない。
//!
//! 名前が `capture`（窓を開いて閉じる）と `install_global_capture_all`（プロセスへ据え付ける）で
//! 対照的なのは意図的で、取り違えを名前の段階で止めるためである。戻り値の形も違う——前者は
//! 窓の中で起きたことを表す [`Vec`]、後者は**据え付けた蓄積先そのもの**（以後ずっと伸び続ける
//! 共有バッファ）である。
//!
//! # 両立条件
//!
//! 同じテストバイナリで全体設定を置いてよいのは本窓口だけであり、他所が
//! `set_global_default`／`init` を行っていると本窓口は**明示的に失敗する**（黙って縮退しない
//! ＝要件 1.6）。また据え付けた後は、そのバイナリ内の `tracing::enabled!` が**全スレッドで真**に
//! なる（フィルタを持たない捕捉先が既定になるため）。

use std::sync::{Arc, Mutex, OnceLock};

use crate::capture::CaptureSubscriber;
use crate::event::CapturedEvent;
use crate::probe::ensure_interest_probes;

/// 据え付けに失敗したときの宣告文。両立条件そのものを述べる。
///
/// `expect` の後ろに元のエラーが `Debug` で続く。
const INCOMPATIBLE_GLOBAL: &str = "全スレッド捕捉を据え付けられなかった。\
     同じテストバイナリで全体設定（グローバル subscriber）を置いてよいのは \
     log-capture-kit の install_global_capture_all だけであり、\
     他所の set_global_default／init と両立しない。\
     先に置かれている設定を取り除くか、当該テストを別バイナリへ分けること。\
     元のエラー";

/// 据え付けた蓄積先。プロセス寿命で 1 つ（設置成功は高々 1 回）。
static CAPTURE_ALL: OnceLock<Arc<Mutex<Vec<CapturedEvent>>>> = OnceLock::new();

/// 全スレッド横断の捕捉先をプロセスへ**一度だけ**据え付け、その蓄積先を返す。
///
/// 別スレッドで発火するログを捕える用途専用の窓口である。呼出スレッドで同期的に発火する
/// ログを見るだけなら [`crate::capture`] を使うこと（窓が閉じ、他テストへ影響が残らない）。
///
/// - TRACE を含む全レベル・全宛先が対象（濾過しない）。
/// - 2 度目以降の呼出は据え付けを行わず、**同じ蓄積先**を返す。
/// - 先に別の全体設定が置かれていた場合は panic する。黙って何もせず空の蓄積先を返す
///   （＝「ログが出なかった」という誤った結論を静かに作る）ことはしない。
///
/// # 両立条件
///
/// 呼んだ後は、そのテストバイナリ内の `tracing::enabled!` が**全スレッドで真**になる。
/// フィルタを持たない捕捉先がプロセスの既定になるためで、ログ有効判定の偽を前提にする
/// テストと同じバイナリには置けない。この窓口を使うファイルは番人テストの例外表に列挙する。
///
/// # Panics
///
/// 先に別の全体設定がある場合（両立条件の違反）。
pub fn install_global_capture_all() -> Arc<Mutex<Vec<CapturedEvent>>> {
    install_into(&CAPTURE_ALL, |subscriber| {
        tracing::subscriber::set_global_default(subscriber)
    })
}

/// 据え付けの本体。蓄積スロットと設置手続きを引数に取る。
///
/// 引数に分けてあるのは、**設置が失敗したときの分岐**（明示的な失敗）を自己テストから踏む
/// ためである。プロセスに 1 つしかない全体設定の枠を消費してしまうと、成功側と失敗側の
/// テストが互いの前提を壊し合い、実行順で色が変わる（本ワークスペースが禁じている形）。
fn install_into<E: std::fmt::Debug>(
    slot: &OnceLock<Arc<Mutex<Vec<CapturedEvent>>>>,
    set_global: impl FnOnce(CaptureSubscriber) -> Result<(), E>,
) -> Arc<Mutex<Vec<CapturedEvent>>> {
    slot.get_or_init(|| {
        // 据え付け前に焼かれた `never` を残さないための常駐 probe（冪等）。
        ensure_interest_probes();

        let subscriber = CaptureSubscriber::default();
        let sink = Arc::clone(&subscriber.0);
        set_global(subscriber).expect(INCOMPATIBLE_GLOBAL);
        // 据え付け以前に焼き付いた callsite の再評価（probe と同じ理由。ここでは保険）。
        tracing::callsite::rebuild_interest_cache();
        sink
    })
    .clone()
}

#[cfg(test)]
#[path = "global_tests.rs"]
mod tests;

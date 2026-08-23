//! 全スレッド横断の一回限りの捕捉窓口の自己テスト。
//!
//! この窓口はプロセスで**一度しか成功できない**設置（`set_global_default`）を内側に持つ。
//! そのため「2 度呼んでも同じ蓄積先」と「先に別の全体設定があると明示的に失敗する」の
//! 2 つを、素朴に書くと互いの前提を壊し合う（前者は設置を成功させ、後者は設置が失敗する
//! 状況を要求する）。実行順で色が変わるテストを作らないため、次のように分けてある。
//!
//! - 成功側は**実物の窓口**をそのまま 2 度呼ぶ。このテストバイナリで全体設定を置くのは
//!   本 crate だけなので、いつ走っても 1 度目は成功する（順序非依存）。別スレッドで発火した
//!   イベントが同じ蓄積先へ入ることまで確かめ、実物の設置経路が働いていることを示す。
//! - 失敗側は**設置手続きだけを差し替えた**同じ本体（[`install_into`]）へ、失敗を返す
//!   設置手続きと専用の蓄積スロットを渡す。プロセスに 1 つしかない全体設定の枠を消費せず、
//!   「設置が失敗したときに黙って縮退せず明示的に失敗する」という分岐そのものを踏む。
//!   実物の `set_global_default` が失敗を返す状況（別の全体設定が先にある）を、その戻り値で
//!   模している——分岐の判断材料は戻り値だけなので、模しても踏む経路は同一である。

use std::sync::{Arc, Mutex, OnceLock};

use super::*;
use crate::event::CapturedEvent;

/// 本テスト専用の宛先（他のテストと発行点を共有しない）。
const OTHER_THREAD_TARGET: &str = "log_capture_kit::tests::global_other_thread";

/// 「先に別の全体設定がある」ことを設置手続きの戻り値で表す模擬エラー。
#[derive(Debug)]
struct ForeignGlobalAlreadyInstalled;

/// 要件 1.6: 2 度呼んでも同じ蓄積先を返す（設置は高々 1 回）。
///
/// あわせて、実物の設置経路が本当に働いていること——**別スレッド**で発火したイベントが
/// 同じ蓄積先へ入ること——を同じテストの内側で示す。これが無いと「同じ Arc が返る」だけの
/// 空虚な緑になり得る。
#[test]
fn installs_once_and_the_second_call_returns_the_same_buffer() {
    let first = install_global_capture_all();
    let second = install_global_capture_all();

    assert!(
        Arc::ptr_eq(&first, &second),
        "2 度目の呼出が別の蓄積先を返した。設置が 2 度行われたか、\
         蓄積先が呼出ごとに作り直されている"
    );

    // 別スレッドで発火 → 呼出スレッドの捕捉窓では捕えられない経路（＝この窓口の存在理由）。
    std::thread::spawn(|| {
        tracing::info!(target: OTHER_THREAD_TARGET, marker = "global-capture-all");
    })
    .join()
    .expect("発火スレッドは panic しない");

    let hits = first
        .lock()
        .expect("蓄積先は毒化していない")
        .iter()
        .filter(|e| e.target == OTHER_THREAD_TARGET)
        .count();
    assert_eq!(
        hits, 1,
        "別スレッドで発火したイベントが蓄積先に入っていない。\
         全体設定が働いていないか、蓄積先が別物になっている"
    );
}

/// 要件 1.6: 先に別の全体設定がある場合は黙って縮退せず明示的に失敗する。
///
/// 失敗メッセージには両立条件（このテストバイナリで全体設定を置いてよいのは本窓口だけ）が
/// 載る。`expected` にその一部を書いてあるので、別の理由の panic では緑にならない。
#[test]
#[should_panic(expected = "全体設定（グローバル subscriber）を置いてよいのは")]
fn fails_explicitly_when_a_different_global_is_already_installed() {
    static SLOT: OnceLock<Arc<Mutex<Vec<CapturedEvent>>>> = OnceLock::new();

    let _ = install_into(&SLOT, |_subscriber| Err(ForeignGlobalAlreadyInstalled));
}

//! 切替点の檻。
//!
//! ここで縛れるのは**同一プロセス内で決まる判定**だけである。常駐の有無はプロセス寿命で
//! 1 度きりなので、1 回の走行で両側を見ることはできない。無効側（`off`）の実挙動を縛る
//! テストはこの crate に無い。無効側で走らせると、`capture_tests.rs` の (a) のうち窓の
//! 内側で他スレッドが先に登録する 1 本
//! （`captures_event_whose_callsite_another_thread_registers_inside_the_window`）は
//! 実行順しだいで赤になり得る（常駐が無いと同じプロセスの dispatcher の数で色が決まる）。
//! 既定の走行は変数を立てないので常駐側だけを通り、
//! `tests/capture_calibration_test.rs` の子プロセス較正が比べるのも硬化の無い素の捕捉と
//! [`crate::capture`] で、この切替の無効側ではない。無効側を通るのは、測定のために変数を
//! 手で立てて走らせたときだけである。
//!
//! 本ファイルの檻は**どちらの側で走らせても緑**である。それが狙いで、A/B の両側で
//! 「指定が確かにこのプロセスまで届いた」ことを同じ形で示す役に立てる。

use super::{
    PROBES_ENV, PROBES_OFF, PROBES_ON, ensure_interest_probes, interest_probes_enabled,
    probes_choice, probes_env_read_count,
};

/// 値は逐語で比べる。前後に空白のある値は `on`／`off` と見なさず断る。
#[test]
#[should_panic(expected = "AREKA_LOG_CAPTURE_PROBES の値が不正")]
fn a_value_with_surrounding_spaces_is_rejected() {
    probes_choice(" on");
}

/// 綴りがちょうど合う値は、それぞれの側に決まる。
#[test]
fn the_exact_values_choose_their_side() {
    assert!(probes_choice(PROBES_ON));
    assert!(!probes_choice(PROBES_OFF));
}

/// 指定と判定が食い違わない。**「立てたつもりで立っていない」を炙り出すための対照**で、
/// 未設定なら既定（常駐する）でなければならない。
#[test]
fn the_decision_agrees_with_what_the_environment_actually_says() {
    let requested = std::env::var(PROBES_ENV).ok();
    match requested.as_deref() {
        None => assert!(
            interest_probes_enabled(),
            "{PROBES_ENV} が未設定なら既定＝常駐する（導入前と 1 ビットも変えない）"
        ),
        Some(PROBES_ON) => assert!(
            interest_probes_enabled(),
            "{PROBES_ENV}={PROBES_ON} を指定したのに常駐していない"
        ),
        Some(PROBES_OFF) => assert!(
            !interest_probes_enabled(),
            "{PROBES_ENV}={PROBES_OFF} を指定したのに常駐している。\
             この状態で採った所要時間は既定側を測っている（測定として無意味）"
        ),
        Some(other) => panic!("{PROBES_ENV} の値が不正: {other:?}"),
    }
}

/// 環境変数を読むのは**プロセス寿命で 1 度きり**である（測定対象へ上乗せしないため）。
///
/// 判定の一致では縛れない性質なので、実際に**回数を数える**。同一の実行体の中で他のテストが
/// 先に捕捉窓を開いていても、確立は [`std::sync::OnceLock`] に畳まれるので回数は 1 のままである
/// ——だから誰が先に走っても答えは決定論的に 1 で、並列実行に依存しない。
#[test]
fn the_environment_is_read_exactly_once_for_the_whole_process() {
    ensure_interest_probes();
    ensure_interest_probes();
    ensure_interest_probes();
    assert_eq!(
        probes_env_read_count(),
        1,
        "環境変数の読み取りがプロセス寿命で 1 度きりになっていない。\
         測定対象の実行時間へ上乗せが乗る（要件 13.1）"
    );
}

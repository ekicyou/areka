//! 切替点の檻（要件 13.1・設計 C9）。
//!
//! ここで縛れるのは**同一プロセス内で決まる判定**だけである。常駐の有無はプロセス寿命で
//! 1 度きりなので、1 回の走行で両側を見ることはできない。無効側の実挙動——取りこぼした
//! 捕捉窓が黙って空を返さず失敗を宣告すること（要件 13.2）——は
//! `tests/capture_calibration_test.rs` の子プロセス較正が縛る。
//!
//! 本ファイルの檻は**どちらの側で走らせても緑**である。それが狙いで、A/B の両側で
//! 「指定が確かにこのプロセスまで届いた」ことを同じ形で示す役に立てる。

use super::{
    PROBES_ENV, PROBES_OFF, PROBES_ON, ensure_interest_probes, interest_probes_enabled,
    probes_env_read_count,
};

/// 変数名はワークスペースの規約（`AREKA_` 名前空間）に従う。
#[test]
fn the_switch_variable_lives_in_the_workspace_namespace() {
    assert!(
        PROBES_ENV.starts_with("AREKA_"),
        "実行時の環境変数は AREKA_ 名前空間（既存の較正用変数と同じ流儀）: {PROBES_ENV}"
    );
}

/// 指定と判定が食い違わない。**「立てたつもりで立っていない」を炙り出すための対照**で、
/// 未設定なら既定（常駐する）でなければならない。
#[test]
fn the_decision_agrees_with_what_the_environment_actually_says() {
    let requested = std::env::var(PROBES_ENV).ok();
    match requested.as_deref().map(str::trim) {
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

/// 確立は冪等で、何度呼んでも判定は動かない。
///
/// **この檻が縛るのは「判定が動かない」ことだけ**である。読み取り回数は縛れない——毎回読む形へ
/// 変えても同一プロセス内では答えが同じなので、判定の一致は恒真になる（タスク 11.1 のレビューが
/// 変異で実測した）。回数のほうは次の
/// [`the_environment_is_read_exactly_once_for_the_whole_process`] が縛る。
#[test]
fn establishing_the_probes_is_idempotent() {
    ensure_interest_probes();
    let first = interest_probes_enabled();
    ensure_interest_probes();
    ensure_interest_probes();
    assert_eq!(
        first,
        interest_probes_enabled(),
        "常駐の確立を繰り返すと判定が動いた（`OnceLock` に畳まれていない）"
    );
}

/// 環境変数を読むのは**プロセス寿命で 1 度きり**である（要件 13.1・測定対象への上乗せが無いこと）。
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

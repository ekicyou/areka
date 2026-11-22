// =============================================================================
// タグ入口の結線と、shell 設定由来の基底の起動時結線の決定論テスト（本体の側）
//
// 測るのは結線の字面そのもの（zwi05＝受け渡し口・入口の登録・受け渡し構造、
// zwi08＝設定の値を起動の段まで運ぶ経路）だけ。配線の欠けは挙動に現れにくく、
// 設定の値を運ぶ経路は途中の 1 行を落としても「設定を書いていないゴースト」と同じ挙動になる。
// 据えた値が実際に効くことは `areka-runtime` の `frame/zorder_descript_tests.rs` が挙動で受け持つ。
//
// 字面の走査は説明文を落とした本文へ当てる。素の全文には本ファイルと相手ファイルの
// 説明の語も入り、検査が恒真になる（zwi08 の末尾に両方向の対照がある）。
// zwi05 が読む `areka-runtime` の `src/lib.rs` の対照は、`areka-runtime` の `zorder_wiring_tests.rs` の `t_zwi06` の末尾にある。
//
// 到達性（zwi01／02）・相順と相の呼出（zwi06／07）・基底を据える順（zwi09）は、`areka-runtime` の
// 同じ名前のファイルにある。
// =============================================================================

use super::source_text_test_support::{code_only, squeeze};
use areka_session::crate_source;

// ---------------------------------------------------------------------------
// ⑴ 結線の字面——削っても挙動に現れない所を名指しで塞ぐ
// ---------------------------------------------------------------------------

/// 受け渡し口・入口の登録・受け渡し構造への引き渡しが、本番の結線に在る。
///
/// 受け口と入口の登録は実行系の側の起動（`runtime_boot.rs`）にあるので、そこは
/// `runtime_boot.rs` の字面で押さえる（窓の側の送出端は束で渡る）。
#[test]
fn t_zwi05_the_boot_wires_the_channel_the_sink_and_the_handoff() {
    let raw = crate_source!("areka-runtime", "src/lib.rs");
    let code = code_only(raw);
    let squeezed = squeeze(&code);
    let runtime_squeezed = squeeze(&code_only(crate_source!(
        "areka-session",
        "src/runtime_boot.rs"
    )));

    assert!(
        squeezed.contains(
            "let (zorder_tx, zorder_rx) = std::sync::mpsc::channel::<ZOrderDirective>();"
        ),
        "指令の受け渡し口（チャネル 1 組）が本文に無い: {squeezed}"
    );
    assert!(
        squeezed.contains("zorder_tx, window_state_tx, };"),
        "送出端が受け口の送出端の束（CueSinkSources）へ渡されていない"
    );
    assert!(
        runtime_squeezed.contains(
            "vec![ Box::new(surface), Box::new(ClockedTextSink::new(text, clock.clone())), Box::new(MoveCueSink::new(move_tx)), Box::new(BalloonLifecycleSink::new(lifecycle_tx, clock)), Box::new(ZOrderCueSink::new(zorder_tx)), Box::new(ReadmeCueSink::new(readme_tx)), Box::new(NoUserBreakCueSink::new(no_user_break_tx)), Box::new(ChangeCueSink::new(change_tx)), install_sink, update_sink, Box::new(SwitchCueSink::new(switch_tx)), Box::new(ReloadCueSink::new(reload_tx)), Box::new(WindowStateCueSink::new(window_state_tx)), ];"
        ),
        "入口の登録（配送の sinks）が既存 4 本＋重なりの受け口＋説明書の受け口＋中断を禁じる旗の受け口＋切替の受け口＋インストールの受け口＋更新の受け口＋シェル・バルーンの切替の受け口＋読み直しの受け口＋常に手前の受け口の形になっていない"
    );
    assert!(
        squeeze(&code_only(include_str!("../main.rs"))).contains(
            "[ Box::new(InstallCueSink::new(install::desk::raw_sender(world))), Box::new(UpdateCueSink::new(update::desk::raw_sender(world))), ]"
        ),
        "本体の受け口の 2 つ（列の 9・10 番目）がインストールの受け口＋更新の受け口の順になっていない"
    );
    assert!(
        squeezed.contains("lifecycle_rx, zorder_rx,"),
        "受信端が受け渡し構造（RuntimeWiring::new）へ渡されていない＝取り出しの相が読む先が無い"
    );
}

// ---------------------------------------------------------------------------
// ⑵ 起動の段の結線——shell 設定の値が台帳まで運ばれる
// ---------------------------------------------------------------------------

/// 起動窓の準備が読んだ `seriko.zorder` の生の値が、`main` の 1 本道で結線へ渡る。
///
/// 落としても挙動が変わらないので、運ぶ側の字面をここが唯一押さえる。
#[test]
fn t_zwi08_the_entry_point_carries_the_shell_setting_into_the_wiring() {
    // 窓を作る側（重なりの生の値の写し）と結線へ渡す行は `ghost_session.rs`、その戻りを受けて
    // ゴーストごとの結線へ運ぶ行は `fn main`（`main.rs`）に在る。
    let session_raw = crate_source!("areka-session", "src/ghost_session.rs");
    let session_code = code_only(session_raw);
    let session = squeeze(&session_code);
    let main_raw = include_str!("../main.rs");
    let squeezed = squeeze(&code_only(main_raw));

    assert!(
        session.contains("let zorder_raw = prepared.zorder_raw.clone();"),
        "準備の結果から重なりの生の値を取り出す行が本文に無い"
    );
    assert!(
        squeezed.contains(
            "let opened = ghost_session::open_ghost_windows(app.world().borrow_mut().world_mut(), &cfg); let descript = match opened {"
        ),
        "起動窓の戻り値から重なりの生の値を受け取る行が本文に無い"
    );
    assert!(
        squeezed.contains(
            "&descript, &ghost_decision, &balloon_decision, ) .map_err(write_failure::stop_boot)?;"
        ),
        "起動窓の戻り値がゴーストごとの結線（boot_ghost）へ渡されていない: {squeezed}"
    );
    assert!(
        session.contains(
            "descript.author_dpi, descript.zorder_raw.as_deref(), kanade_stop.clone(), shell,"
        ),
        "重なりの生の値が結線（wire_runtime）へ渡されていない＝設定が台帳へ届かない: {session}"
    );

    // 対照——落とし過ぎ／落とし漏れが無いこと。
    assert!(
        session_code.contains("pub(crate) fn open_ghost_windows("),
        "説明文を落とす処理が本文まで落としている"
    );
    assert!(
        !session_code.contains("起動窓の準備が descript から**1 度だけ**読み取った値"),
        "説明文が落ちていない（走査が恒真になっている）"
    );
    assert!(
        session_raw.contains("起動窓の準備が descript から**1 度だけ**読み取った値"),
        "対照の前提が崩れている（素の全文に説明文が無い）"
    );
}

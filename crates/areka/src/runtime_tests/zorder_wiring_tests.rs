// =============================================================================
// タグ入口の結線（task 6.2・要件 11.2）と、shell 設定由来の基底の起動時結線
// （task 6.3・要件 5.1／5.2）の決定論テスト
//
// この task が足すのは判断ではなく**配線**である。受け口も台帳も相の本体も既に在って
// 兄弟のテストが判断を全部押さえているので、ここで測るのは 3 点だけになる。
//
// 1. タグを含む台本が受け口へ届くと、指令が**台帳適用の相まで**通ること（到達性）。
// 2. その相が**鎖の適用系より前**に走ること（相順）。
// 3. 結線の字面そのもの（受け渡し口・入口の登録・受け渡し構造・相の呼出）。
//
// 3 を字面で押さえるのは、配線の欠けが挙動に現れにくいからである。呼出を丸ごと削っても
// 判断のテストは 1 本も赤くならず（task 4.2／5.1／5.2 で三度実証済み）、相順に至っては
// 「是正が 1 心拍ぶん遅れる」だけなので、実窓を持たない檻には**原理的に**映らない。
//
// 相順は 2 で挙動としても測る——ただし測れるのは「確定段へ `.before` で載せた仕事が
// 同じ巡のうちに適用系から見えるか」という**機構**であって、本番の `frame_system`
// がその形で載っていることは 3 の字面が受け持つ（本番の相は `RuntimeWiring` が挿さっていない
// World では丸ごと無操作なので、headless の檻からは観測できない）。
//
// 字面の走査は必ず**説明文を落とした本文**へ当てる。素の全文には本ファイルの説明の語も
// 相手ファイルの doc の語も入るので、当てる先を間違えると検査が恒真になる
// （task 3.1 の教訓。各檻の末尾に両方向の対照を置いてある）。
//
// task 6.3 が足す 2 本（⑷）も同じ性質を持つ——設定の値を起動の段まで運ぶ経路は、
// 途中の 1 行を落としても「設定を書いていないゴースト」と全く同じ挙動になるので、
// 挙動を見る檻には**原理的に**映らない。運ぶ側（`main.rs`）と据える側（`mod.rs`）の
// 字面をそれぞれ名指しで押さえる。据えた値が実際に効くことは兄弟の
// `frame/zorder_descript_tests.rs` が挙動で受け持つ。
// =============================================================================

use super::source_text_test_support::{code_only, squeeze};
use crate::crate_source_test_support::crate_source;

// ---------------------------------------------------------------------------
// ⑶ 結線の字面——削っても挙動に現れない 4 点を名指しで塞ぐ
// ---------------------------------------------------------------------------

/// 受け渡し口・入口の登録・受け渡し構造への引き渡しが、本番の結線に在る（task 6.2 の 3 点）。
///
/// 受け口と入口の登録は実行系の側の起動（`runtime_boot.rs`・areka-P0-mcp-reload 要件 4.2）へ
/// 移ったので、そこは `runtime_boot.rs` の字面で押さえる（窓の側の送出端は束で渡る）。
#[test]
fn t_zwi05_the_boot_wires_the_channel_the_sink_and_the_handoff() {
    let raw = crate_source!("areka-runtime", "src/lib.rs");
    let code = code_only(raw);
    let squeezed = squeeze(&code);
    let runtime_squeezed = squeeze(&code_only(include_str!("../session/runtime_boot.rs")));

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
            "vec![ Box::new(surface), Box::new(ClockedTextSink::new(text, clock.clone())), Box::new(MoveCueSink::new(move_tx)), Box::new(BalloonLifecycleSink::new(lifecycle_tx, clock)), Box::new(ZOrderCueSink::new(zorder_tx)), Box::new(ReadmeCueSink::new(readme_tx)), Box::new(NoUserBreakCueSink::new(no_user_break_tx)), Box::new(ChangeCueSink::new(change_tx)), Box::new(InstallCueSink::new(crate::install::desk::raw_sender(world))), Box::new(UpdateCueSink::new(crate::update::desk::raw_sender(world))), Box::new(SwitchCueSink::new(switch_tx)), Box::new(ReloadCueSink::new(reload_tx)), Box::new(WindowStateCueSink::new(window_state_tx)), ];"
        ),
        "入口の登録（配送の sinks）が既存 4 本＋重なりの受け口＋説明書の受け口＋中断を禁じる旗の受け口＋切替の受け口＋インストールの受け口＋更新の受け口＋シェル・バルーンの切替の受け口＋読み直しの受け口＋常に手前の受け口の形になっていない"
    );
    assert!(
        squeezed.contains("lifecycle_rx, zorder_rx,"),
        "受信端が受け渡し構造（RuntimeWiring::new）へ渡されていない＝取り出しの相が読む先が無い"
    );

    // 対照——落とし過ぎ／落とし漏れが無いこと。
    assert!(
        code.contains("pub fn wire_runtime("),
        "説明文を落とす処理が本文まで落としている"
    );
    assert!(
        !code.contains("完成済み 5 トラックのエンジン"),
        "説明文が落ちていない（走査が恒真になっている）"
    );
    assert!(
        raw.contains("完成済み 5 トラックのエンジン"),
        "対照の前提が崩れている（素の全文に説明文が無い）"
    );
}

// ---------------------------------------------------------------------------
// ⑷ 起動の段の結線——shell 設定の値が台帳まで運ばれる（task 6.3・要件 5.1／5.2）
// ---------------------------------------------------------------------------

/// 起動窓の準備が読んだ `seriko.zorder` の生の値が、`main` の 1 本道で結線へ渡る。
///
/// 落としても「設定を書いていないゴースト」と挙動が 1 ミリも変わらないので、挙動の檻には
/// 映らない。運ぶ側の字面をここが唯一押さえる。
#[test]
fn t_zwi08_the_entry_point_carries_the_shell_setting_into_the_wiring() {
    // 窓を作る側（重なりの生の値の写し）と結線へ渡す行は `ghost_session.rs`、その戻りを受けて
    // ゴーストごとの結線へ運ぶ行は `fn main`（`main.rs`）に在る（areka-P0-ghost-restart-unit
    // task 4.2・4.3）。
    let session_raw = include_str!("../ghost_session.rs");
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

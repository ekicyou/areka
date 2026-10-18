//! 骨格 boot→loop→exit の統合 smoke テスト（task 4.2・R4.1/R2.4、baseware-root-layout task 6.1 で
//! 3 方向へ更新）。
//!
//! env ゲート（`AREKA_APP_SMOKE_EXIT_MS`・task 2.3）を立てた areka バイナリの子プロセスを
//! 起動し、起動窓を開いて `app.run()` ループを回した後に自動終了（`quit_app`＝全窓 despawn
//! → 終了の指示）→ `run()` 復帰 → 終了コードで終わる経路を実プロセスで踏破・証明する。
//!
//! 3 方向（areka-P0-baseware-root-layout 要件 7.5・1.6・design「smoke 3 方向」）:
//! - **① 本物方向**（emo2 検体のゴースト／バルーンを argv の絶対パスで供給）: 本物のゴースト窓
//!   構成（2 スコープ×キャラ窓＋バルーン窓）で完走し **exit 0**（要件 7.1＝argv 起動の見え方）。
//! - **② 根方向**（argv なし・`AREKA_ROOT` に検体の根）: 根の下の 1 体を `route=Only` で、
//!   同梱バルーンを `route=Companion` で決め、本物のゴースト窓で完走し **exit 0**（要件 1.6）。
//! - **③ 0 体方向**（argv なし・空の一時の根）: 「ゴーストが見つかりません」の告知（`error!`）を
//!   残し、本物の窓を開かずに **非 0** で終わる（窓 0 で居座らない）。
//! - **④ 失敗方向**（SHIORI を `loadu` が偽を返す検証用 DLL にした emo2 の複製）: 告知の記録を
//!   1 件残して **非 0** で終わる（areka-P0-shiori-fault-notice 要件 4.3〜4.5）。
//! - **⑤ 書けない方向**（アプリの記憶の置き場が普通のファイル）: 窓を作る前の書けるかの確かめで
//!   止まり、「areka を起動できません」の告知を残して **非 0** で終わる
//!   （areka-P0-write-failure-notice 要件 6.1・7.6・7.7）。
//!
//! 子の起こし方・終わりの待ち・環境の整え方は実機の試験の窓口 `real-run-kit` に任せる
//! （areka-P0-real-run-mcp-kit 要件 7.1〜7.4）。窓口は全方向で `AREKA_NO_ALERT=1`（告知のモーダルで
//! 締切まで止まらない）・`NO_COLOR=1`（`route=Only` を ANSI に分断されずに照合する）を渡し、開発者の
//! 根・記憶の置き場を持ち込まず、記憶の置き場は回の置き場（`target\` の下）に取る。i686 の
//! `shiori-host32-helper.exe` も窓口が `areka.exe` の隣へ揃える。そのうえで ①② は
//! 「SHIORI が動かなくなりました」の行が 0 件＝SHIORI の失敗で終わっていないことも見る
//! （areka-P0-shiori-fault-notice 要件 4.1・4.2）。
//!
//! モニタ 0 台（headless）では ①② の起動窓の準備が `PlacementError::Monitor` で失敗し、
//! 「起動窓を開けません」の告知＋非 0 終了が契約どおりの挙動になる。その場合だけ非 0 を受理する
//! （旧「ダミー窓で完走を受理」の置き換え。ダミー窓は退役済み）。
//!
//! 実装規律:
//! - **合否は終了コード＋経路マーカー（tracing のメッセージ本文と `route=` 欄）で判定する**。
//!   良性の teardown warn（`WARN ... Could not despawn entity ... generation 1`・smoke timer と
//!   window-registry close の二重 despawn 競合）は warn レベルで exit 0 に影響しないため
//!   assert 対象にしない（tasks.md Implementation Notes・実測済み）。
//! - 検体は窓口が**走りごとに**使い捨ての複製を取る。① と ② は並走しうるので同じ複製を共有させない
//!   （② がゴーストの記憶 `last.balloon` を複製へ書くため、共有すると同じ複製での後続の起動が
//!   `route=Memory` へ変わりうる）。複製は取得のたびに新品なので、何度重ねても ② は `route=Companion`。

use real_run_kit::{Ended, EndedBy, Launch, RealRun};
use std::process::ExitStatus;
use std::time::Duration;
use temp_path_kit::TempPath;

/// 子へ渡す保険の自動終了。起動窓を開き run ループを一巡させるだけの十分小さな値。
const SMOKE_EXIT: Duration = Duration::from_millis(500);

/// wire 成立マーカー（`emo2_boot` の `info!`・実 sink 結線が end-to-end で踏まれた証跡・task 7.1）。
const WIRED: &str = "emo2-boot: 実 sink 結線が成立しました（wire 成立）";

/// 本物のゴースト窓を開いたマーカー（`ghost_session::prepare_ghost_windows` が組む窓を作るクロージャ）。
const REAL_WINDOWS: &str = "本物のゴースト窓を開きました";

/// SHIORI の失敗の告知の題名（`alert.rs` の `SHIORI_FAULT_TITLE` と同じ綴り・bin crate の定数は
/// テストから参照できないので写す）。これを含む行の数で「SHIORI の失敗で終わったか」を見る。
const SHIORI_FAULT_TITLE: &str = "SHIORI が動かなくなりました";

/// SHIORI の失敗で終わっていないこと＝告知の題名を含む行が 0 件（①② 共通・要件 4.2）。
/// モニタ 0 台の受理でも成り立つので、受理の判定より先に無条件で確かめる。
fn assert_no_shiori_fault(all: &str) {
    let n = all
        .lines()
        .filter(|l| l.contains(SHIORI_FAULT_TITLE))
        .count();
    assert_eq!(
        n, 0,
        "SHIORI の失敗で終わっています（「{SHIORI_FAULT_TITLE}」の行が {n} 件）。\
         \n--- child output ---\n{all}"
    );
}

/// `message` を本文に持つ行に `field` が載っていることを確かめる（経路の目印・要件 4.10・5.11）。
/// パスは綴らない（検体の置き場に依存しない）。
fn assert_line_has(all: &str, message: &str, field: &str) {
    let line = all
        .lines()
        .find(|l| l.contains(message))
        .unwrap_or_else(|| panic!("「{message}」の行がありません。\n--- child output ---\n{all}"));
    assert!(
        line.contains(field),
        "「{message}」の行に `{field}` がありません: {line}\n--- child output ---\n{all}"
    );
}

/// モニタ 0 台（headless）なら「起動窓を開けません」の告知と非 0 終了を受理して `true` を返す
/// （①② 共通）。モニタがある環境では `false`（呼び手が本物の窓と exit 0 を判定する）。
fn accepted_as_no_monitor(status: ExitStatus, all: &str) -> bool {
    if !(all.contains("起動窓を開けません") && all.contains("モニタ列挙に失敗")) {
        return false;
    }
    eprintln!(
        "note: モニタ 0 台環境のため「起動窓を開けません」＋非 0 終了で受理（Monitor エラー）"
    );
    assert!(
        !status.success(),
        "起動窓を開けないときは非 0 で終わるべきですが status={status:?} でした。\
         \n--- child output ---\n{all}"
    );
    assert!(
        !all.contains(REAL_WINDOWS),
        "起動窓を開けないのに本物のゴースト窓が開いたのは契約外。\n--- child output ---\n{all}"
    );
    true
}

/// モニタがある環境の完走判定（①② 共通）: exit 0・wire 成立・本物のゴースト窓。
fn assert_real_windows_and_exit_zero(status: ExitStatus, all: &str) {
    assert!(
        status.success(),
        "smoke プロセスは exit 0 で終了すべきですが status={status:?} でした。\
         \n--- child output ---\n{all}"
    );
    // wire 成立マーカーの存在 assert（task 7.1・R7.3 観測境界）: `main` は起動窓の後で
    // `emo2_boot::wire_emo2_boot` を呼び、実 fixture では `wired=true` で本マーカーを `info!` する。
    // `emo2_frame_system` の schedule 登録が end-to-end で少なくとも 1 回踏まれたことを固定する。
    assert!(
        all.contains(WIRED),
        "実 fixture 経路は emo2-boot の実 sink 結線（wire 成立）マーカーを出すべき。\
         \n--- child output ---\n{all}"
    );
    assert!(
        all.contains(REAL_WINDOWS),
        "検体あり環境では本物のゴースト窓を開くべき。\n--- child output ---\n{all}"
    );
}

/// 保険の自動終了を既定（500 ms）にした起動の指定。
fn smoke() -> RealRun {
    RealRun::new(env!("CARGO_BIN_EXE_areka")).safety_exit(SMOKE_EXIT)
}

/// 起こして、子が自分で終わるのを待ち（窓口の締切 60 秒）、終わり方と出力の全文を返す（要件 7.4）。
fn run_to_exit(spec: RealRun) -> (Ended, String) {
    let mut run = spec.spawn();
    let ended = run.wait_exit();
    (ended, run.output())
}

fn emo2_sample() -> Launch {
    Launch::Sample {
        name: "emo2".into(),
        balloon: Some("emo2-kakukaku".into()),
    }
}

fn emo2_root() -> Launch {
    Launch::SampleRoot {
        name: "emo2".into(),
    }
}

/// ① 本物方向（要件 7.1）: emo2 検体のゴースト／バルーンを argv の絶対パスで供給し、
/// 本物のゴースト窓構成（2 スコープ）で自動終了 → **exit 0** で完走する。
#[test]
fn argv_direction_boots_real_ghost_windows_and_exits_zero() {
    let (ended, all) = run_to_exit(smoke().launch(emo2_sample()));
    let status = ended.status;

    // argv の経路で決まる（列挙も記憶も見ない・要件 4.1・5.1）。
    assert_line_has(&all, "起動するゴーストを決めました", "route=Argv");
    assert_line_has(&all, "バルーンを決めました", "route=Argv");
    assert_no_shiori_fault(&all);

    if accepted_as_no_monitor(status, &all) {
        return;
    }
    assert_real_windows_and_exit_zero(status, &all);
    // 保険の自動終了で終わった（窓口の綴りの本物の檻・D-19）。
    assert_eq!(
        ended.by,
        EndedBy::SafetyExit,
        "保険の自動終了（origin=Smoke）で終わるはず。\
         \n--- child output ---\n{all}"
    );
}

/// ② 根方向（要件 1.6）: argv なし・`AREKA_ROOT` に検体の根をそのまま渡す。根の下の唯一の
/// ゴースト（`route=Only`）と同梱バルーン（`install.txt` の `balloon.directory`＝`route=Companion`）で
/// 本物のゴースト窓を開き、自動終了 → **exit 0** で完走する。
#[test]
fn root_direction_resolves_only_ghost_and_companion_balloon() {
    let (ended, all) = run_to_exit(smoke().launch(emo2_root()));
    let status = ended.status;

    assert_line_has(&all, "起動するゴーストを決めました", "route=Only");
    assert_line_has(&all, "バルーンを決めました", "route=Companion");
    assert_no_shiori_fault(&all);

    if accepted_as_no_monitor(status, &all) {
        return;
    }
    assert_real_windows_and_exit_zero(status, &all);
}

/// ③ 0 体方向（要件 7.5）: argv なし・空の一時の根。「ゴーストが見つかりません」の告知を残し、
/// 本物の窓を開かずに **非 0** で終わる（モニタの有無によらない＝窓を開く前に止まる）。
#[test]
fn empty_root_direction_alerts_ghost_missing_and_exits_nonzero() {
    let (ended, all) = run_to_exit(smoke().launch(Launch::Empty));
    let status = ended.status;

    assert!(
        !status.success(),
        "ゴースト 0 体の根では非 0 で終わるべきですが status={status:?} でした。\
         \n--- child output ---\n{all}"
    );
    assert!(
        all.contains("ゴーストが見つかりません"),
        "ゴースト 0 体の根では「ゴーストが見つかりません」を告げるべき。\n--- child output ---\n{all}"
    );
    assert!(
        !all.contains(REAL_WINDOWS),
        "ゴースト 0 体で本物のゴースト窓が開くのは契約外。\n--- child output ---\n{all}"
    );
}

/// ④ 失敗方向（要件 4.3〜4.5）: emo2 の複製の SHIORI を検証用 DLL（`shiori_loadu.dll`）へ差し替え、
/// `HOST32_TESTDLL_LOADU_FAIL=1` で `loadu` を偽にして起動する。自動終了（20 秒）より先に
/// 「接続できなかった」が起き、告知は抑止のうえ記録だけ残して **非 0** で終わる。
#[test]
fn fault_direction_shiori_connect_failure_exits_nonzero_with_one_alert() {
    let spec = smoke()
        .launch(emo2_sample())
        .before_spawn(|staged| {
            let master = staged
                .ghost_folder()
                .expect("Sample はゴーストのフォルダを持つ")
                .join("ghost/master");
            // SHIORI を検証用 DLL にした最小の descript（本番コードに口を足さない・要件 4.4）。
            std::fs::write(
                master.join("descript.txt"),
                "charset,UTF-8\r\nname,emo2-fault\r\nshiori,shiori_loadu.dll\r\n\
                 seriko.defaultsurfacedirectoryname,master\r\n",
            )
            .expect("descript.txt の差し替え");
            staged.ensure_i686(
                "shiori_loadu.dll",
                "HOST32_TESTDLL_LOADU_DLL",
                "shiori-host32-testdll-loadu",
                &master,
            );
        })
        .env("HOST32_TESTDLL_LOADU_FAIL", "1")
        // 失敗（数秒）を自動終了より先に起こす。窓口の締切 60 秒の内側（要件 4.5）。
        .safety_exit(Duration::from_secs(20));
    let (ended, all) = run_to_exit(spec);
    let status = ended.status;

    if accepted_as_no_monitor(status, &all) {
        return;
    }
    assert!(
        !status.success(),
        "SHIORI の失敗で終わるときは非 0 で終わるべきですが status={status:?} でした。\
         \n--- child output ---\n{all}"
    );
    // 告知の記録（`event="alert"` の行）のうち題名を持つものだけを数える（要件 3.5）。
    // ④ の失敗は `loadu` の偽なので、種類は「接続できなかった」でなければならない（要件 4.4）。
    let n = all
        .lines()
        .filter(|l| {
            l.contains("event=\"alert\"")
                && l.contains(SHIORI_FAULT_TITLE)
                && l.contains("SHIORI に接続できなかった")
        })
        .count();
    assert_eq!(
        n, 1,
        "「{SHIORI_FAULT_TITLE}」の告知の記録はちょうど 1 件のはずが {n} 件。\
         \n--- child output ---\n{all}"
    );
    assert!(
        all.contains(REAL_WINDOWS),
        "窓は出てから閉じるはず（本物のゴースト窓の目印なし）。\n--- child output ---\n{all}"
    );
}

/// ⑤ 書けない方向（areka-P0-write-failure-notice 要件 6.1・7.6・7.7）: `AREKA_ROOT` に検体の根、
/// アプリの記憶の置き場 `AREKA_PROFILE_DIR` に `target\` の下の**普通のファイル**を渡す。起動の前の
/// 書けるかの確かめで止まり、`memory_write_failed` と「areka を起動できません」の告知を残して
/// **非 0** で終わる。窓を作る前に止まるので、本物のゴースト窓の目印も印を書いた記録も 0
/// （モニタの有無によらない）。
#[test]
fn unwritable_direction_stops_boot_with_memory_write_alert() {
    let dir = TempPath::under_target("smoke-unwritable-profile");
    let profile = dir.child("profile-is-a-file");
    std::fs::write(&profile, b"not a folder").expect("置き場の代わりの普通のファイル");

    let (ended, all) = run_to_exit(smoke().launch(emo2_root()).profile_dir(&profile));
    let status = ended.status;

    assert!(
        !status.success(),
        "記憶を書けないときは非 0 で終わるべきですが status={status:?} でした。\
         \n--- child output ---\n{all}"
    );
    assert_line_has(&all, "event=\"memory_write_failed\"", "moment=\"boot\"");
    let alerts = all
        .lines()
        .filter(|l| {
            l.contains("event=\"alert\"")
                && l.contains("areka を起動できません")
                && l.contains("記憶を書けないので")
        })
        .count();
    assert_eq!(
        alerts, 1,
        "「areka を起動できません」（記憶を書けないので）の告知の記録はちょうど 1 件のはずが {alerts} 件。\
         \n--- child output ---\n{all}"
    );
    for mark in [REAL_WINDOWS, "session_mark_written"] {
        let n = all.lines().filter(|l| l.contains(mark)).count();
        assert_eq!(
            n, 0,
            "「{mark}」の行が {n} 件（0 のはず）。\n--- child output ---\n{all}"
        );
    }
}

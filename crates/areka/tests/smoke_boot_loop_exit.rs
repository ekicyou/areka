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
//!
//! 全方向で `AREKA_NO_ALERT=1`（告知のモーダルで番犬の締切まで止まらない・要件 9.2）。
//! ②③ の `AREKA_PROFILE_DIR` は一時フォルダ（開発者のアプリの記憶を読まず・書かない）。
//!
//! ①② は i686 の `shiori-host32-helper.exe` が `areka.exe` の隣にあることを前提にし、テストが
//! 自分で揃える（`ensure_helper_beside_areka`・x64 の helper は置き換える）。そのうえで
//! 「SHIORI が動かなくなりました」の行が 0 件＝SHIORI の失敗で終わっていないことも見る
//! （areka-P0-shiori-fault-notice 要件 4.1・4.2）。
//!
//! モニタ 0 台（headless）では ①② の起動窓の準備が `PlacementError::Monitor` で失敗し、
//! 「起動窓を開けません」の告知＋非 0 終了が契約どおりの挙動になる。その場合だけ非 0 を受理する
//! （旧「ダミー窓で完走を受理」の置き換え。ダミー窓は退役済み）。
//!
//! 実装規律:
//! - 子プロセスは `cargo run` ではなく Cargo が用意する `CARGO_BIN_EXE_areka`（統合テスト用に
//!   Cargo が bin を先にビルドして渡すパス）を直接起動する。再コンパイルによるノイズを避ける。
//! - タイムアウト番犬は純 std（`std::process` + `std::thread`/`std::time`・新規依存なし・R6.1）。
//!   `try_wait()` を短周期でポーリングし、寛大な締切内に終了しなければ `kill()` してテスト失敗。
//! - **合否は終了コード＋経路マーカー（tracing のメッセージ本文と `route=` 欄）で判定する**。
//!   子へ `NO_COLOR=1` を渡し、欄の名前と値の間に ANSI の着色が挟まらないようにする
//!   （`route=Only` をそのまま照合できる）。
//!   良性の teardown warn（`WARN ... Could not despawn entity ... generation 1`・smoke timer と
//!   window-registry close の二重 despawn 競合）は warn レベルで exit 0 に影響しないため
//!   assert 対象にしない（tasks.md Implementation Notes・実測済み）。
//! - 検体は**テストごとに** `SampleRoot::acquire` する（使い捨ての複製）。① と ② は並走しうるので
//!   同じ複製を共有させない（② がゴーストの記憶 `last.balloon` を複製へ書くため、共有すると同じ
//!   複製での後続の起動が `route=Memory` へ変わりうる）。複製は取得のたびに新品なので、実行を
//!   何度重ねても ② は `route=Companion` のまま。

use sample_ghost_kit::SampleRoot;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};
use std::sync::OnceLock;
use std::time::{Duration, Instant};
use temp_path_kit::TempPath;

/// 子プロセスへ渡す自動 close 遅延（ms）。起動窓を開き run ループを一巡させるだけの
/// 十分小さな値。0 でも受理されるが（即時発火）、窓生成と run 立ち上げの前後関係を
/// 現実的にするため小さな正値を与える。
const SMOKE_EXIT_MS: &str = "500";

/// タイムアウト番犬の締切。初回起動は遅い（窓生成＋COM/DPI 初期化）ため寛大に取る。
/// これを超えて終了しなければハング＝テスト失敗（TIMEOUT）と判定する。
const WATCHDOG_DEADLINE: Duration = Duration::from_secs(60);

/// `try_wait()` のポーリング周期。
const POLL_INTERVAL: Duration = Duration::from_millis(50);

/// wire 成立マーカー（`emo2_boot` の `info!`・実 sink 結線が end-to-end で踏まれた証跡・task 7.1）。
const WIRED: &str = "emo2-boot: 実 sink 結線が成立しました（wire 成立）";

/// 本物のゴースト窓を開いたマーカー（`main` の `open_startup_window` 成功アーム）。
const REAL_WINDOWS: &str = "本物のゴースト窓を開きました";

/// env ゲートを立てた areka バイナリを与えた引数と env で起動し、番犬締切内の終了を待って
/// `(status, stdout, stderr)` を返す共通ドライバ（3 方向で共有）。
fn run_smoke(args: &[&str], envs: &[(&str, &str)]) -> (ExitStatus, String, String) {
    let bin = env!("CARGO_BIN_EXE_areka");

    // 子プロセス起動: env ゲートを立て、診断用に stdout/stderr を捕捉する。
    // stdout/stderr を piped にすることで、失敗時に子の tracing 出力を assert メッセージへ回せる。
    let mut child = Command::new(bin)
        .args(args)
        .env("AREKA_APP_SMOKE_EXIT_MS", SMOKE_EXIT_MS)
        // 告知のメッセージボックスを抑える（モーダルで番犬の締切まで止まらず、非 0 で即座に終わる）。
        .env("AREKA_NO_ALERT", "1")
        // 欄の着色を切る（`route=Only` を ANSI に分断されずに照合する）。
        .env("NO_COLOR", "1")
        // 開発者のシェルの根・記憶の場所を持ち込まない（方向ごとに envs で与える）。
        .env_remove("AREKA_ROOT")
        .env_remove("AREKA_PROFILE_DIR")
        .envs(envs.iter().copied())
        // RUST_LOG は明示しない（骨格既定 info でよい）。診断は捕捉出力から得る。
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("areka smoke バイナリの起動に失敗しました（{bin}）: {e}"));

    // タイムアウト番犬: try_wait() を短周期でポーリングし、締切内の終了を待つ。
    let start = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => {
                if start.elapsed() >= WATCHDOG_DEADLINE {
                    // ハング: 子を kill してから明示的に失敗する（マスクしない）。
                    // kill 後に wait_with_output() で子を回収しつつ捕捉出力を診断へ回す。
                    let _ = child.kill();
                    let (out, err) = child
                        .wait_with_output()
                        .map(|o| {
                            (
                                String::from_utf8_lossy(&o.stdout).into_owned(),
                                String::from_utf8_lossy(&o.stderr).into_owned(),
                            )
                        })
                        .unwrap_or_default();
                    panic!(
                        "smoke プロセスが番犬締切（{:?}）内に終了しませんでした（ハング）。\
                         boot→loop→exit の欠陥の疑い。\n--- child stdout ---\n{out}\n--- child stderr ---\n{err}",
                        WATCHDOG_DEADLINE
                    );
                }
                std::thread::sleep(POLL_INTERVAL);
            }
            Err(e) => panic!("子プロセスの状態取得（try_wait）に失敗しました: {e}"),
        }
    };

    // 終了済み: 診断・経路マーカー assert のため出力を回収する。
    let output = child.wait_with_output();
    let (out, err) = output
        .map(|o| {
            (
                String::from_utf8_lossy(&o.stdout).into_owned(),
                String::from_utf8_lossy(&o.stderr).into_owned(),
            )
        })
        .unwrap_or_default();

    (status, out, err)
}

/// PE の機械種別が i386（0x014c）かを見る。`cargo build --workspace` は x64 の helper を
/// `areka.exe` の隣へ置くので、「隣にある」だけでは i686 の前提を満たさない（x64 の helper は
/// 32bit の SHIORI DLL を読めず接続に失敗する）。
fn is_i686_pe(path: &Path) -> bool {
    let Ok(b) = std::fs::read(path) else {
        return false;
    };
    let at = |i: usize, n: usize| b.get(i..i + n);
    let Some(pe) = at(0x3c, 4).map(|s| u32::from_le_bytes(s.try_into().unwrap()) as usize) else {
        return false;
    };
    at(pe + 4, 2) == Some(&[0x4c, 0x01][..])
}

/// i686 成果物 `name` を `dest_dir` に揃えてそのパスを返す（要件 4.1）。`dest_dir` に i686 の
/// `name` があればそのまま。無ければ env `env_var` → `<target>/i686-pc-windows-msvc/{debug,release}`
/// の順で探し、一時名へ書いてから改名で置く（並走するテストの子が書きかけを起動しない）。
/// どこにも無ければ建て方を案内して失敗する。
fn ensure_i686_artifact(name: &str, env_var: &str, dest_dir: &Path) -> PathBuf {
    let dest = dest_dir.join(name);
    if is_i686_pe(&dest) {
        return dest;
    }
    // CARGO_BIN_EXE_areka＝<target>/<profile>/areka.exe → <target>
    let target = Path::new(env!("CARGO_BIN_EXE_areka"))
        .ancestors()
        .nth(2)
        .expect("areka.exe の 2 つ上は target")
        .join("i686-pc-windows-msvc");
    // 環境変数で指したのに無い／i686 でないなら、黙って target へ移らず止める（既存の探索と同じ）。
    let explicit = std::env::var_os(env_var).map(PathBuf::from).inspect(|p| {
        assert!(
            is_i686_pe(p),
            "{env_var}={} が指すファイルが無いか i686 ではありません（i686 の {name} を先にビルド）",
            p.display()
        )
    });
    let src = explicit
        .into_iter()
        .chain(["debug", "release"].map(|p| target.join(p).join(name)))
        .find(|p| p.is_file())
        .unwrap_or_else(|| {
            panic!(
                "i686 の {name} が見つかりません（env {env_var} → {}\\{{debug,release}}）。\
                 PowerShell で先に建ててください: \
                 cargo build -p shiori-host32-helper --target i686-pc-windows-msvc ; \
                 cargo build -p shiori-host32-testdll-loadu --target i686-pc-windows-msvc",
                target.display()
            )
        });
    let tmp = dest_dir.join(format!("{name}.{}.tmp", std::process::id()));
    std::fs::copy(&src, &tmp)
        .unwrap_or_else(|e| panic!("{} → {} の複製に失敗: {e}", src.display(), tmp.display()));
    std::fs::rename(&tmp, &dest)
        .unwrap_or_else(|e| panic!("{} → {} の改名に失敗: {e}", tmp.display(), dest.display()));
    dest
}

/// `areka.exe` の隣に i686 の `shiori-host32-helper.exe` を揃える（①② の前提・要件 4.1）。
/// 置き先は同じバイナリの並走するテストで共有されるので、同じプロセス内で 1 度だけ行う。
fn ensure_helper_beside_areka() {
    static DONE: OnceLock<PathBuf> = OnceLock::new();
    DONE.get_or_init(|| {
        let dir = Path::new(env!("CARGO_BIN_EXE_areka"))
            .parent()
            .expect("areka.exe の親");
        ensure_i686_artifact("shiori-host32-helper.exe", "HOST32_HELPER_EXE", dir)
    });
}

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

/// ① 本物方向（要件 7.1）: emo2 検体のゴースト／バルーンを argv の絶対パスで供給し、
/// 本物のゴースト窓構成（2 スコープ）で自動終了 → **exit 0** で完走する。
#[test]
fn argv_direction_boots_real_ghost_windows_and_exits_zero() {
    ensure_helper_beside_areka();
    let emo2 = SampleRoot::acquire("emo2").expect("emo2 は登記済みの検体");
    let ghost_root = emo2.folder();
    let balloon_root = emo2.balloon("emo2-kakukaku").expect("emo2 の同梱バルーン");
    assert!(
        ghost_root.is_absolute() && ghost_root.join("ghost/master/descript.txt").exists(),
        "emo2 検体が見つかりません（絶対パス前提）: {}",
        ghost_root.display()
    );

    let (status, out, err) = run_smoke(
        &[
            ghost_root.to_str().expect("検体パスは UTF-8"),
            balloon_root.to_str().expect("検体パスは UTF-8"),
        ],
        &[],
    );
    let all = format!("{out}\n{err}");

    // argv の経路で決まる（列挙も記憶も見ない・要件 4.1・5.1）。
    assert_line_has(&all, "起動するゴーストを決めました", "route=Argv");
    assert_line_has(&all, "バルーンを決めました", "route=Argv");
    assert_no_shiori_fault(&all);

    if accepted_as_no_monitor(status, &all) {
        return;
    }
    assert_real_windows_and_exit_zero(status, &all);
}

/// ② 根方向（要件 1.6）: argv なし・`AREKA_ROOT` に検体の根をそのまま渡す。根の下の唯一の
/// ゴースト（`route=Only`）と同梱バルーン（`install.txt` の `balloon.directory`＝`route=Companion`）で
/// 本物のゴースト窓を開き、自動終了 → **exit 0** で完走する。
#[test]
fn root_direction_resolves_only_ghost_and_companion_balloon() {
    ensure_helper_beside_areka();
    let emo2 = SampleRoot::acquire("emo2").expect("emo2 は登記済みの検体");
    let root = emo2.root();
    assert!(root.is_absolute(), "検体の根は絶対パス: {}", root.display());
    let profile = TempPath::new("smoke-root-profile");

    let (status, out, err) = run_smoke(
        &[],
        &[
            ("AREKA_ROOT", root.to_str().expect("検体パスは UTF-8")),
            (
                "AREKA_PROFILE_DIR",
                profile.path().to_str().expect("一時パスは UTF-8"),
            ),
        ],
    );
    let all = format!("{out}\n{err}");

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
    let root = TempPath::new("smoke-empty-root");
    let profile = TempPath::new("smoke-empty-profile");

    let (status, out, err) = run_smoke(
        &[],
        &[
            (
                "AREKA_ROOT",
                root.path().to_str().expect("一時パスは UTF-8"),
            ),
            (
                "AREKA_PROFILE_DIR",
                profile.path().to_str().expect("一時パスは UTF-8"),
            ),
        ],
    );
    let all = format!("{out}\n{err}");

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

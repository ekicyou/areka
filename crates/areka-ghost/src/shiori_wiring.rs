//! 本番の SHIORI 接続手続き（`real_connect`）。
//!
//! 実行ファイル起動から応答待ち受け・読み込み確認までの一連の接続手順を、
//! 起動時に一度だけ実行される形で提供する。接続に必要なパスはマウント解決
//! 結果（`MountModel.shiori`）から取り出し、欠落時は接続失敗として扱う
//! （design.md「ghost::shiori_wiring」）。
//!
//! `crates/areka-kanade/tests/kanade/real_helper_test.rs` の `connect_real_helper`
//! 実証手順（`ParentMessageWindow::create()` → `spawn` → `pump_until_hello_or` →
//! `send_request(MsgTag::Load, ..)` ack 確認）を本番結線へ昇格したものであり、
//! host-32 の語彙・IPC・プロトコルは変更せず消費のみ行う（要件 3.5）。

use std::path::PathBuf;
use std::time::Duration;

use areka_kanade::{ShioriBackend, ShioriConnection};
use areka_parsers::charset::DefaultEncoding;
use areka_parsers::package::ShioriMount;
use shiori_host32_host::process_host::LOAD_ACK_TIMEOUT;
use shiori_host32_host::{Charset, CharsetNegotiator, HelperLifecycle, ParentMessageWindow, spawn};
use shiori_host32_ipc::MsgTag;

/// ハンドシェイク（HELLO 受領）の上限時間（既存 E2E の `HANDSHAKE_TIMEOUT` と同値）。
const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(5);

/// 起動時の決定・後退を記録する宛先（`sylphya_wiring` と同じ `ghost-boot`）。
///
/// `RUST_LOG` は **target 名**で指定する（`ghost-boot=debug`。モジュールパス名では点かない）。
const LOG_TARGET: &str = "ghost-boot";

/// 既定の文字コードを、ファイル層（`DefaultEncoding::to_encoding`）と同じ固定写像で決める
/// （要件 1.3）。
///
/// OS のロケール設定は読まない（読む箇所 0）。ファイル層から橋渡しせず ghost 側に写像を
/// 置くのは、host-32 側の `Charset` へ渡す際に「UTF-16 でない」ことを検査する失敗腕が要り、
/// その腕が固定写像では到達不能＝記録のない死んだ失敗経路になるため（design
/// §shiori_wiring の裁定）。ここには失敗経路が無い。
///
/// `DefaultEncoding` は `#[non_exhaustive]` ゆえ網羅の腕が要るが、本番の既定は `Ansi`
/// ＝Shift_JIS であり、写像は値を返すだけで失敗しない。
fn default_charset(default: DefaultEncoding) -> Charset {
    match default {
        DefaultEncoding::Ansi => Charset::SHIFT_JIS,
        DefaultEncoding::Utf8 => Charset::UTF_8,
        // `#[non_exhaustive]` が要求する腕。覆うのは将来の新しい変種だけで、
        // 現在の 2 つはいずれも上で名指ししている。
        _ => Charset::SHIFT_JIS,
    }
}

/// descript の 2 キーから初期の文字コードを決め、交渉状態を組み立てる（要件 2.1・2.4・2.6）。
///
/// 優先順は `shiori.forceencoding` ＞ `shiori.encoding` ＞ 既定（[`default_charset`]）。
/// 解決できない宣言は警告ログ 1 行（キー名・ラベル・理由・採用した後退先）を出して次の
/// 優先順へ後退し、`shiori.forceencoding` が退けられたときは**強制の効力も失う**
/// （以後は応答の `Charset` ヘッダに従う・要件 2.4）。記録なしに後退する経路は無い（要件 7.1）。
///
/// 警告の `fallback` には最終的に採用した正規名を書くため、決定を先に済ませてから
/// 警告・情報の順に記録する（design「後退先を先に決めてから記録する」）。
pub(crate) fn initial_charset(shiori: &ShioriMount, default: DefaultEncoding) -> CharsetNegotiator {
    // 退けた宣言（キー名・生ラベル・理由）。後退先が決まるまで記録を保留する。
    let mut rejected: Vec<(&'static str, &str, &'static str)> = Vec::new();
    let mut decided: Option<(Charset, bool, &'static str)> = None;

    if let Some(label) = shiori.force_encoding.as_deref() {
        match Charset::for_label(label) {
            Ok(charset) => decided = Some((charset, true, "forceencoding")),
            Err(error) => rejected.push(("shiori.forceencoding", label, error.reason())),
        }
    }
    if decided.is_none()
        && let Some(label) = shiori.encoding.as_deref()
    {
        match Charset::for_label(label) {
            Ok(charset) => decided = Some((charset, false, "encoding")),
            Err(error) => rejected.push(("shiori.encoding", label, error.reason())),
        }
    }
    let (charset, forced, source) = match decided {
        Some(decision) => decision,
        None => (default_charset(default), false, "default"),
    };

    for (key, label, reason) in rejected {
        tracing::warn!(
            target: LOG_TARGET,
            event = "charset_label_unresolved",
            key,
            label,
            reason,
            fallback = charset.name(),
            "descript の文字コード宣言を解決できない——無視して次の優先順へ後退する"
        );
    }
    tracing::info!(
        target: LOG_TARGET,
        event = "charset_initial",
        charset = charset.name(),
        source,
        "SHIORI 通信の初期の文字コードを決定した"
    );

    CharsetNegotiator::new(charset, forced)
}

/// 本番 connect クロージャを構成する（実行は shiori アクタースレッド上・一度だけ）。
///
/// 接続手順（`real_helper_test.rs::connect_real_helper` を昇格したもの）:
/// 1. `shiori.file` が `None` なら DLL ファイル名が未解決＝即座に接続失敗（`Err`）。
///    ウィンドウ生成・プロセス起動のいずれも試みない（推測しない・design.md）。
/// 2. `ParentMessageWindow::create()` で親 message-only 窓を生成する（`!Send`・
///    呼び出しスレッド＝shiori アクタースレッド上で実行される前提）。
/// 3. `spawn(helper_exe, load_dir, shiori_name, parent_hwnd)` で i686 helper を起動する。
/// 4. `pump_until_hello_or(HANDSHAKE_TIMEOUT)` で HELLO 受領（ハンドシェイク完了）を観測する。
/// 5. `send_request(MsgTag::Load, &[], LOAD_ACK_TIMEOUT)` で LOAD を発行し ack `[1]` を確認する。
///
/// いずれかの段で失敗すれば `Err(String)`（呼び出し側で `ShioriDown` へ写る）。成功時は
/// `Box<dyn ShioriBackend>`（実体は `ShioriConnection { window, helper, negotiator }`）を返す。
///
/// # 初期の文字コード（要件 2.7・5.2）
/// 初期値の決定（[`initial_charset`]）は**クロージャを返す前に**、つまり `real_connect` を
/// 呼んだスレッド上で同期に行う。理由は 2 つ（design §shiori_wiring）: 起動ログが呼出
/// スレッドの同期発火しか捕捉できないこと、そして接続に失敗しても決定と根拠が記録に
/// 残ること（要件 2.6）。決めた交渉状態はクロージャへ move され、接続成立時に
/// `ShioriConnection` の持ち物になる。
///
/// 交渉状態はこの呼び出しごとに新しく作られるので、ゴーストを起動し直す（SHIORI を
/// load し直す）たびに descript 由来の初期値へ戻り、前回セッションの採用結果は引き継が
/// れない（要件 5.2）。
pub fn real_connect(
    helper_exe: PathBuf,
    shiori: ShioriMount,
    default_encoding: DefaultEncoding,
) -> impl FnOnce() -> Result<Box<dyn ShioriBackend>, String> + Send + 'static {
    // クロージャを返す前に同期で決める（接続の成否に依らず記録が残る・ログが捕捉できる）。
    let negotiator = initial_charset(&shiori, default_encoding);

    move || {
        // 1. `file` が None なら推測せず即座に失敗（design.md「推測しない」）。
        let Some(shiori_name) = shiori.file else {
            return Err(format!(
                "SHIORI DLL のファイル名が未解決のため接続できません（dir={}）",
                shiori.dir.display()
            ));
        };
        let load_dir = shiori.dir;

        // 2. 親 message-only 窓（!Send・アクタースレッド上で生成される）。
        let window = ParentMessageWindow::create()
            .map_err(|e| format!("親 message-only 窓生成に失敗: {e}"))?;
        let parent_hwnd = window.hwnd_u32();

        // 3. i686 helper を spawn（cwd=load_dir）。
        let helper = spawn(&helper_exe, &load_dir, &shiori_name, parent_hwnd)
            .map_err(|e| format!("i686 helper の spawn に失敗: {e}"))?;

        // 4. HELLO 受領＝ハンドシェイク完了を観測（上限時間内に来なければ失敗）。
        let helper_hwnd = window.pump_until_hello_or(HANDSHAKE_TIMEOUT);
        if helper_hwnd.is_none() {
            return Err(
                "上限時間内に helper から HELLO を受領できなかった（ハンドシェイク未完）"
                    .to_string(),
            );
        }

        // 5. LOAD 先行（helper 内に proxy を確立＝REQUEST の構造的前提）→ ack[1] を確認。
        let load_ack = window
            .send_request(MsgTag::Load, &[], LOAD_ACK_TIMEOUT)
            .map_err(|e| format!("LOAD の send_request が失敗（ack 未達）: {e}"))?;
        if load_ack != vec![1u8] {
            return Err(format!(
                "SHIORI の LOAD が成功 ack [1] を返さなかった（proxy 未確立）: {load_ack:?}"
            ));
        }

        Ok(Box::new(ShioriConnection {
            window,
            helper: HelperLifecycle::new(helper),
            // 起動時に決めた初期値（この `real_connect` 呼び出しに 1 つ・要件 5.2）。
            //
            // 宣言を持たない UTF-8 のゴースト（emo2 の pasta）では初期値が既定の Shift_JIS に
            // なる。そこから採用が起きるまでに `Charset: Shift_JIS` を名乗る要求は **2 本**
            // ある——片道イベントの応答は採用の根拠に用いない（要件 5.3）ためで、並びは
            // `areka-kanade` の boot 状態機械（`schedule/boot.rs`）の発行点 3 つで決まる:
            //
            // 1. `boot_start`（Idle+Boot の遷移関数）が発行する `OnInitialize`＝**片道の
            //    NOTIFY**。その応答は `client.rs` の `notify` が破棄するだけで、応答ヘッダを
            //    採用へ回す手続きを呼ばない（同関数の本文にその経路が無いことは通信層の
            //    構造検査が数えている）。よってこの応答では採用が起きない。
            // 2. `on_reply` の `BootInit` + `Notified` 腕が発行する username リソース照会
            //    ＝**応答待ちの GET**。これも Shift_JIS で出る。相手が名乗る
            //    `Charset: UTF-8` を採用するのはこの応答。
            // 3. prefetch 応答を写して発行する `OnFirstBoot`＝GET（初回起動の場合。起動記録が
            //    あれば同じ腕が `OnFirstBoot` を飛ばして `OnBoot` を発行するが、3 本目から
            //    UTF-8 になる点は変わらない）。ここから UTF-8 になり、以後は変わらない（要件 2.7）。
            //
            // 2 本とも本文は ASCII のみ（References を持たず `Sender`／`Status` の値も ASCII の
            // 語彙）なので、適用前と違うのは `Charset` ヘッダの値だけ。採用の規則は実行層では
            // なく交渉状態の持ち物であり、ここは決めた値を渡すだけ。
            negotiator,
        }) as Box<dyn ShioriBackend>)
    }
}

#[cfg(test)]
mod tests {
    use areka_parsers::charset::DefaultEncoding;
    use areka_parsers::package;
    use temp_path_kit::TempPath;

    use super::*;

    /// `ShioriMount` は `#[non_exhaustive]`（`areka-parsers` 外からの struct リテラル構築は
    /// 不可）ゆえ、本番と同じ経路（`package::resolve`）で実フィクスチャから取得する。
    ///
    /// `ghost/master/descript.txt` に `shiori,<name>` 行が有れば `file: Some(name)`、
    /// 無ければ `file: None` になる（`resolve` の実装どおり・推測しない）。
    ///
    /// フィクスチャの置き場は共通窓口 `temp-path-kit` から受け取る。名前にプロセス識別子と
    /// 連番が入るので**プロセス間でも一意**であり、破棄で中身ごと消える（従来の手組みの名前は
    /// スレッド識別子までしか含まず、しかも後始末が無いため一時ディレクトリが残り続けていた）。
    /// 返り値の [`TempPath`] を呼び出し側がテストの間だけ生かしておくこと。
    fn build_shiori_mount(shiori_line: Option<&str>) -> (TempPath, ShioriMount) {
        let temp = TempPath::new("ghost-shiori-wiring");
        let root = temp.path().to_path_buf();
        let master_dir = root.join("ghost").join("master");
        std::fs::create_dir_all(&master_dir).expect("fixture master dir 作成に失敗");
        std::fs::create_dir_all(root.join("shell").join("master"))
            .expect("fixture shell dir 作成に失敗");

        let mut descript = String::new();
        if let Some(name) = shiori_line {
            descript.push_str(&format!("shiori,{name}\n"));
        }
        std::fs::write(master_dir.join("descript.txt"), descript)
            .expect("fixture descript.txt 書き込みに失敗");

        let mount = package::resolve(&root, DefaultEncoding::Ansi)
            .expect("fixture ghost_root の resolve に失敗");
        (temp, mount.shiori)
    }

    /// `shiori.file` が `None`（DLL ファイル名未解決）なら、ウィンドウ生成やプロセス起動を
    /// 一切試みずに即座に接続失敗となる（design.md「`file` が `None` の場合は接続失敗」）。
    ///
    /// `helper_exe` も意図的に存在しないパスにしておき、もし実装が誤って spawn まで
    /// 進んでしまった場合でも「別の」エラー（spawn 失敗）に化けてこのテストが誤って
    /// 通ってしまわないよう、エラーメッセージの内容がファイル名未解決由来であることまで確認する。
    #[test]
    fn missing_shiori_file_fails_immediately_without_spawning() {
        let helper_exe = PathBuf::from("Z:\\definitely\\does\\not\\exist\\helper.exe");
        let (_temp, shiori) = build_shiori_mount(None);
        assert_eq!(
            shiori.file, None,
            "fixture は shiori 行なし＝file:None のはず"
        );

        let connect = real_connect(helper_exe, shiori, DefaultEncoding::Ansi);
        let result = connect();

        match result {
            Err(err) => assert!(
                err.contains("ファイル名"),
                "エラーはファイル名未解決が理由であることを示すはず: {err}"
            ),
            Ok(_) => panic!("file: None は接続失敗になるはず"),
        }
    }

    /// `helper_exe` が実在しないパスの場合、実際の接続手続き（窓生成→spawn）を通しつつ、
    /// spawn 段で自然に失敗する（i686 helper／pasta DLL は一切不要）。
    ///
    /// `ParentMessageWindow::create()` 自体は Win32 の窓生成のみで helper の実在に依存しない
    /// ため成功し得るが、後続の `spawn(..)` は存在しないファイルの起動を試みて即座に
    /// I/O エラーを返す（`Command::spawn` は同期・timeout を待たない）ため、本テストは
    /// 数秒以内に bounded に完了する。
    #[test]
    fn nonexistent_helper_exe_fails_via_spawn_step() {
        let helper_exe = PathBuf::from("Z:\\definitely\\does\\not\\exist\\helper.exe");
        let (_temp, shiori) = build_shiori_mount(Some("whatever.dll"));
        assert_eq!(
            shiori.file,
            Some("whatever.dll".to_string()),
            "fixture は shiori 行あり＝file:Some のはず"
        );

        let connect = real_connect(helper_exe, shiori, DefaultEncoding::Ansi);
        let result = connect();

        match result {
            Err(_) => {}
            Ok(_) => panic!("存在しない helper_exe への spawn は失敗するはず"),
        }
    }
}

#[cfg(test)]
#[path = "shiori_wiring_charset_tests.rs"]
mod shiori_wiring_charset_tests;

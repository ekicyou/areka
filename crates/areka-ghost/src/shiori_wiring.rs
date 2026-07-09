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
use areka_parsers::package::ShioriMount;
use shiori_host32_host::process_host::LOAD_ACK_TIMEOUT;
use shiori_host32_host::{spawn, HelperLifecycle, ParentMessageWindow};
use shiori_host32_ipc::MsgTag;

/// ハンドシェイク（HELLO 受領）の上限時間（既存 E2E の `HANDSHAKE_TIMEOUT` と同値）。
const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(5);

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
/// `Box<dyn ShioriBackend>`（実体は `ShioriConnection { window, helper: HelperLifecycle::new(..) }`）
/// を返す。
pub fn real_connect(
    helper_exe: PathBuf,
    shiori: ShioriMount,
) -> impl FnOnce() -> Result<Box<dyn ShioriBackend>, String> + Send + 'static {
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
        }) as Box<dyn ShioriBackend>)
    }
}

#[cfg(test)]
mod tests {
    use areka_parsers::charset::DefaultEncoding;
    use areka_parsers::package;

    use super::*;

    /// `ShioriMount` は `#[non_exhaustive]`（`areka-parsers` 外からの struct リテラル構築は
    /// 不可）ゆえ、本番と同じ経路（`package::resolve`）で実フィクスチャから取得する。
    ///
    /// `ghost/master/descript.txt` に `shiori,<name>` 行が有れば `file: Some(name)`、
    /// 無ければ `file: None` になる（`resolve` の実装どおり・推測しない）。
    fn build_shiori_mount(shiori_line: Option<&str>) -> ShioriMount {
        let unique = format!(
            "areka-ghost-shiori-wiring-test-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        );
        let root = std::env::temp_dir().join(unique);
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
        mount.shiori
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
        let shiori = build_shiori_mount(None);
        assert_eq!(
            shiori.file, None,
            "fixture は shiori 行なし＝file:None のはず"
        );

        let connect = real_connect(helper_exe, shiori);
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
        let shiori = build_shiori_mount(Some("whatever.dll"));
        assert_eq!(
            shiori.file,
            Some("whatever.dll".to_string()),
            "fixture は shiori 行あり＝file:Some のはず"
        );

        let connect = real_connect(helper_exe, shiori);
        let result = connect();

        match result {
            Err(_) => {}
            Ok(_) => panic!("存在しない helper_exe への spawn は失敗するはず"),
        }
    }
}

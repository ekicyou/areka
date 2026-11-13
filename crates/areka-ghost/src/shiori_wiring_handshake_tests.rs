//! 握手（HELLO 待ち）の時間切れの誤りに helper の終わり方を載せる（audit-shiori-host32 指摘 35）。
//!
//! 引数を受け付けずにすぐ終わる OS の実行ファイル（`whoami.exe`）を helper として渡す。握手の
//! 5 秒を 1 回待ち、誤りの文に終わり方が載ることを見る。窓は message-only の 1 つだけ。
use std::path::PathBuf;

use areka_parsers::charset::DefaultEncoding;

use super::real_connect;

#[test]
fn handshake_timeout_names_the_helper_exit() {
    let system_root = std::env::var_os("SystemRoot").expect("SystemRoot が在る");
    let helper_exe = PathBuf::from(system_root).join(r"System32\whoami.exe");
    assert!(
        helper_exe.is_file(),
        "whoami.exe が無い: {}",
        helper_exe.display()
    );
    let (_temp, shiori) =
        crate::test_support::shiori_mount("ghost-shiori-handshake", "shiori,whatever.dll\n");

    let result = real_connect(helper_exe, shiori, DefaultEncoding::Ansi)();

    match result {
        Err(e) => {
            assert!(e.contains("HELLO"), "握手の時間切れで落ちるはず: {e}");
            assert!(
                e.contains("helper は終わり方 Abnormal("),
                "誤りに helper の終わり方が無い: {e}"
            );
        }
        Ok(_) => panic!("whoami.exe は HELLO を送らないので接続は失敗するはず"),
    }
}

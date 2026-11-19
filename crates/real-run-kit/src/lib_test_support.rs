//! 兄弟のテストの共有の手助け（生かしておく子・落ちても止める取っ手・panic の文の取り出し）。

use std::process::{Child, Command, Stdio};

/// 落ちても止める子（試すコードが panic しても、巻き戻しの間に止めて回収する）。
pub(crate) struct KillOnDrop(pub(crate) Child);

impl Drop for KillOnDrop {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// 締切まで生き続ける子（終わりの分岐へ入らない相手）。取っ手を捨てると止まる。
pub(crate) fn living_child() -> KillOnDrop {
    KillOnDrop(
        Command::new("ping")
            .args(["-n", "60", "127.0.0.1"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap(),
    )
}

/// 閉包が panic した文を返す（panic しなければ失敗）。
pub(crate) fn panic_text(f: impl FnOnce() + std::panic::UnwindSafe) -> String {
    let err = std::panic::catch_unwind(f).expect_err("panic するはず");
    err.downcast_ref::<String>()
        .cloned()
        .or_else(|| err.downcast_ref::<&str>().map(|s| s.to_string()))
        .unwrap_or_default()
}

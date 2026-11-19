//! 結合テストの共有の手助け（偽の子の指定・生きているかの確かめ・panic の文の取り出し）。
//!
//! 各テストは別のクレートとして組まれ、使う手助けがテストごとに違うので、使わない手助けの警告を許す。
#![allow(dead_code)]

use std::any::Any;
use std::process::Command;

use real_run_kit::RealRun;

/// 偽の子を手順つきで立てる指定（32bit の仲介は揃えない）。
pub fn fake(steps: &str) -> RealRun {
    RealRun::new(env!("CARGO_BIN_EXE_real-run-fake-areka"))
        .without_host32_helper()
        .env("REAL_RUN_FAKE_STEPS", steps)
}

/// その番号のプロセスが偽の子として生きているか（`tasklist` の行に実行ファイルの名前が在るか）。
///
/// shortcut: 番号と名前だけで引くので、止まった子の番号を並走する別の偽の子が使い回すと「生きている」と読んで偽の赤になる、揺れたら起動時刻でも照らす形に替える。
pub fn alive(pid: u32) -> bool {
    let out = Command::new("tasklist")
        .args(["/FI", &format!("PID eq {pid}"), "/NH"])
        .output()
        .expect("tasklist を起こせない");
    String::from_utf8_lossy(&out.stdout).contains("real-run-fake-areka")
}

/// 捕まえた panic の文（文字でなければ空）。
pub fn panic_message(err: &(dyn Any + Send)) -> String {
    err.downcast_ref::<String>()
        .cloned()
        .or_else(|| err.downcast_ref::<&str>().map(|s| s.to_string()))
        .unwrap_or_default()
}

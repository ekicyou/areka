//! 32bit の仲介の判定と揃え方（要件 2.1〜2.6）。
//!
//! `cargo build --workspace` は x64 の仲介を `areka.exe` の隣へ置くので、「隣にある」だけでは
//! 32bit の前提を満たさない（x64 の仲介は 32bit の SHIORI の DLL を読めず接続に落ちる）。
//! そこで PE の頭の機械の種類で判定し、32bit の物だけを置き先へ揃える。

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, PoisonError};

/// 揃える 32bit の成果物の定め（ファイル名・元を指す環境変数・建て方のパッケージ）。
pub(crate) struct I686Spec {
    pub file_name: &'static str,
    pub env_var: &'static str,
    pub package: &'static str,
}

/// 32bit の仲介の定め。`areka.exe` の隣へ揃える（要件 2.1）。
// launch が使うまでは試験の外で使われない。
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) const HELPER: I686Spec = I686Spec {
    file_name: "shiori-host32-helper.exe",
    env_var: "HOST32_HELPER_EXE",
    package: "shiori-host32-helper",
};

/// PE の頭の機械の種類が i386（`0x014c`）か。読めない・短いは偽。
pub(crate) fn is_i686_pe(path: &Path) -> bool {
    let Ok(b) = std::fs::read(path) else {
        return false;
    };
    let at = |i: usize, n: usize| b.get(i..i.checked_add(n)?);
    let Some(pe) = at(0x3c, 4).map(|s| u32::from_le_bytes(s.try_into().unwrap()) as usize) else {
        return false;
    };
    pe.checked_add(4).and_then(|i| at(i, 2)) == Some(&[0x4c, 0x01][..])
}

/// `spec` の成果物を `dest_dir` へ 32bit で揃えて、そのパスを返す（要件 2.1〜2.6）。
///
/// `env_value` は `spec.env_var` の値、`i686_target` は `<target>\i686-pc-windows-msvc`。
/// 置き先が 32bit ならそのまま。そうでなければ元を 環境変数 → debug → release の順に
/// 32bit であるものから選んで写す。環境変数が指す物が無いか 32bit でなければ、黙って
/// ほかへ移らずに panic する。どこにも無ければ探した場所と建て方を添えて panic する。
// launch が使うまでは試験の外で使われない。
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn ensure(
    spec: &I686Spec,
    env_value: Option<OsString>,
    i686_target: &Path,
    dest_dir: &Path,
) -> PathBuf {
    // プロセスの中では 1 本ずつ揃える（要件 2.6）。試験の panic で毒になっても鍵は使い続ける。
    static LOCK: Mutex<()> = Mutex::new(());
    let _guard = LOCK.lock().unwrap_or_else(PoisonError::into_inner);

    let name = spec.file_name;
    let env_var = spec.env_var;
    let dest = dest_dir.join(name);
    if is_i686_pe(&dest) {
        return dest;
    }
    let src = match env_value.map(PathBuf::from) {
        Some(p) => {
            assert!(
                is_i686_pe(&p),
                "{env_var}={} が指すファイルが無いか 32bit（i686）ではありません。\
                 ほかの場所へは移りません（32bit の {name} を指してください）",
                p.display()
            );
            p
        }
        None => {
            let places = ["debug", "release"].map(|p| i686_target.join(p).join(name));
            places
                .iter()
                .find(|p| is_i686_pe(p))
                .cloned()
                .unwrap_or_else(|| {
                    panic!(
                        "32bit（i686）の {name} が見つかりません。探した場所: 環境変数 {env_var}（未設定）・{}・{}。\
                         PowerShell で先に建ててください: cargo build -p {} --target i686-pc-windows-msvc",
                        places[0].display(),
                        places[1].display(),
                        spec.package
                    )
                })
        }
    };
    place(&src, &dest);
    dest
}

/// `src` を `<dest>.<pid>.<連番>.tmp` へ写してから `dest` へ改名する（書きかけを起動させない・
/// 要件 2.6）。改名に失敗したら置き先を見直し、32bit になっていれば通す（別のプロセスが先に
/// 置いた・使用中で替えられない）。32bit でなければ理由を添えて panic する。
fn place(src: &Path, dest: &Path) {
    static SERIAL: AtomicU64 = AtomicU64::new(0);
    let serial = SERIAL.fetch_add(1, Ordering::Relaxed);
    let mut tmp = dest.as_os_str().to_owned();
    tmp.push(format!(".{}.{serial}.tmp", std::process::id()));
    let tmp = PathBuf::from(tmp);
    if let Err(e) = std::fs::copy(src, &tmp) {
        let _ = std::fs::remove_file(&tmp);
        panic!("{} → {} の複製に失敗: {e}", src.display(), tmp.display());
    }
    if let Err(e) = std::fs::rename(&tmp, dest) {
        let _ = std::fs::remove_file(&tmp);
        assert!(
            is_i686_pe(dest),
            "{} → {} の改名に失敗し、置き先も 32bit（i686）ではありません: {e}",
            tmp.display(),
            dest.display()
        );
    }
}

#[cfg(test)]
#[path = "i686_tests.rs"]
mod tests;

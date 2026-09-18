//! テストが「空の根」として借りる使い捨ての作業フォルダ（spec: `areka-P0-nar-install`
//! 要件 7.1・7.10）。
//!
//! # なぜ OS の一時フォルダを使わないか
//!
//! 一時パスの窓口（`temp-path-kit`）は OS の一時フォルダの下に置き場を取るが、本仕様の
//! 根は `cargo clean` で消えること・ビルド成果物と取り違えが起きないことの両方を要求
//! されている（要件 7.1）。そこでビルド成果物の置き場の下に**本仕様専用の名前空間**を
//! 掘り、そこだけを使う（要件 7.10 の「OS の一時フォルダを使わない」）。
//!
//! # 置き場の見つけ方
//!
//! `CARGO_TARGET_DIR` が設定されていればそれ、無ければ**実行ファイルの祖先で名前が
//! `target` の最初のフォルダ**。どちらでも決まらなければ、探索の起点を付けて
//! [`SampleError::TargetDirNotFound`] を返す（黙って別の場所へ落ちない）。
//!
//! # 配るもの
//!
//! ```text
//! <置き場>/nar-samples/
//! └── work/
//!     ├── <プロセス識別子>-<連番>/       # 配る空の根
//!     └── <プロセス識別子>-<連番>.lock   # 生存の札（開いたまま持つ）
//! ```
//!
//! 札を**先に**作ってからフォルダを作る。札は削除を共有しない形で開いたままなので、
//! 生きている作業フォルダは他のプロセスの掃除から守られる（後続の掃除は、札を消せたか
//! どうかで持ち主の生死を見分ける）。
//!
//! # 使い方
//!
//! 値を束縛している間だけ根が存在する。`let _ = WorkDir::new()` はその場で破棄される。
//!
//! 下の例は**組むところまで**を走らせる（`no_run`）。doctest の実行ファイルだけは
//! ビルド成果物の置き場の外（OS の一時フォルダ）に置かれるので、この窓口は設計どおり
//! [`SampleError::TargetDirNotFound`] を返す。振る舞いの判定は兄弟テストが持ち、この例は
//! **窓口が crate の外から届くこと**（doctest は別 crate として組まれる）を固定する。
//!
//! ```rust,no_run
//! use sample_ghost_kit::WorkDir;
//!
//! let work = WorkDir::new().expect("ビルド成果物の置き場は見つかるはず");
//! assert!(work.path().is_dir());
//! assert!(work.path().is_absolute());
//!
//! let kept = work.path().to_path_buf();
//! drop(work);
//! assert!(!kept.exists());
//! ```

use std::ffi::OsStr;
use std::fs::{File, OpenOptions};
use std::os::windows::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use crate::SampleError;

/// ビルド成果物の置き場の下に掘る、本仕様専用の名前空間（要件 7.1）。
const NAMESPACE: &str = "nar-samples";

/// 使い捨ての根を並べる棚。
const WORK: &str = "work";

/// 読むことだけを共有する（＝削除は拒む）共有モード `FILE_SHARE_READ`。
const FILE_SHARE_READ: u32 = 1;

/// 同一プロセス内で単調増加する連番。プロセス間の一意性はプロセス識別子が担う。
static NEXT_SERIAL: AtomicU32 = AtomicU32::new(0);

/// ビルド成果物の置き場を決める唯一の式。
///
/// 環境変数の値と実行ファイルの場所を**引数で受ける**ので、環境を書き換えずに
/// （＝並走するテストに影響を与えずに）両方の経路と失敗を判定できる。
fn find_target_dir(configured: Option<&str>, exe: &Path) -> Result<PathBuf, SampleError> {
    if let Some(dir) = configured.map(str::trim).filter(|dir| !dir.is_empty()) {
        return std::path::absolute(dir).map_err(|source| SampleError::Io {
            what: "ビルド成果物の置き場の絶対化",
            path: PathBuf::from(dir),
            source,
        });
    }
    exe.ancestors()
        .find(|ancestor| ancestor.file_name() == Some(OsStr::new("target")))
        .map(Path::to_path_buf)
        .ok_or_else(|| SampleError::TargetDirNotFound {
            started_from: exe.to_path_buf(),
        })
}

/// 本仕様専用の名前空間の絶対パス。
fn namespace_dir() -> Result<PathBuf, SampleError> {
    let exe = std::env::current_exe().map_err(|source| SampleError::Io {
        what: "実行ファイルの場所の取得",
        path: PathBuf::new(),
        source,
    })?;
    let configured = std::env::var("CARGO_TARGET_DIR").ok();
    Ok(find_target_dir(configured.as_deref(), &exe)?.join(NAMESPACE))
}

/// 破棄で木ごと消える、札付きの使い捨ての作業フォルダ。
///
/// `areka-nar` のテストはこれを「空の根」として借りる（OS の一時フォルダを使わずに
/// 済ませるための唯一の窓）。
#[derive(Debug)]
pub struct WorkDir {
    path: PathBuf,
    lock: PathBuf,
    /// 生存の札。[`Drop`] で**先に**閉じる（開いたままでは自分でも消せない）。
    lease: Option<File>,
}

impl WorkDir {
    /// 空の作業フォルダを 1 つ取る。
    ///
    /// 札を `create_new` で作ってからフォルダを作るので、フォルダが見えている間は必ず
    /// 持ち主の札が開いている。
    ///
    /// # Errors
    ///
    /// ビルド成果物の置き場が決まらないとき [`SampleError::TargetDirNotFound`]、
    /// 札やフォルダを作れないとき [`SampleError::Io`]。
    pub fn new() -> Result<WorkDir, SampleError> {
        let work = namespace_dir()?.join(WORK);
        std::fs::create_dir_all(&work).map_err(|source| SampleError::Io {
            what: "作業フォルダの棚の作成",
            path: work.clone(),
            source,
        })?;

        let stem = format!(
            "{}-{}",
            std::process::id(),
            NEXT_SERIAL.fetch_add(1, Ordering::Relaxed)
        );
        let lock = work.join(format!("{stem}.lock"));
        let lease = OpenOptions::new()
            .write(true)
            .create_new(true)
            .share_mode(FILE_SHARE_READ)
            .open(&lock)
            .map_err(|source| SampleError::Io {
                what: "生存の札の作成",
                path: lock.clone(),
                source,
            })?;

        let path = work.join(&stem);
        if let Err(source) = std::fs::create_dir(&path) {
            // 札だけが残らないよう、作れなかったときはその場で畳む。
            drop(lease);
            report_cleanup(std::fs::remove_file(&lock), &lock);
            return Err(SampleError::Io {
                what: "作業フォルダの作成",
                path,
                source,
            });
        }

        Ok(WorkDir {
            path,
            lock,
            lease: Some(lease),
        })
    }

    /// 配られた作業フォルダそのもの（絶対パス）。
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// 生存の札のパス。掃除の担当（後続タスク）と、札が守られていることを確かめる
    /// テストが読む。
    ///
    /// 借り手が使うのは [`WorkDir::path`] だけなので `pub(crate)` に絞りたいところだが、
    /// 今日の呼び手はテストだけなので絞ると本体のビルドで「使われていない」の警告が出る。
    /// 掃除（後続タスク）が最初の非テストの呼び手になるので、絞るのはそのときでよい。
    pub fn lock_path(&self) -> &Path {
        &self.lock
    }
}

impl Drop for WorkDir {
    fn drop(&mut self) {
        // 札を開いたままでは削除できない（削除を共有していないため）。先に閉じる。
        drop(self.lease.take());
        report_cleanup(std::fs::remove_dir_all(&self.path), &self.path);
        report_cleanup(std::fs::remove_file(&self.lock), &self.lock);
    }
}

/// 後始末の失敗を必ず人の読める形で残す。
///
/// [`Drop`] は失敗を返せないので、呼び手へ伝える道がここしかない。本 crate はテスト
/// 専用で記録層に依存しない（依存を足さない）ため、宛先は標準エラー出力にする。
/// 既に無いのは後始末の目的が達成された状態なので黙って通す。残った木と札は次の走行の
/// 掃除が回収する。
fn report_cleanup(result: std::io::Result<()>, path: &Path) {
    if let Err(err) = result {
        if err.kind() == std::io::ErrorKind::NotFound {
            return;
        }
        eprintln!(
            "sample-ghost-kit: 作業フォルダの後始末に失敗した（次の走行の掃除で回収する）: {} ({err})",
            path.display()
        );
    }
}

#[cfg(test)]
#[path = "devroot_tests.rs"]
mod tests;

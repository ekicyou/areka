//! 回の置き場と検体の写し・落ちた回に証拠を残す・成功で消す（要件 1.2・1.4・6.1〜6.4・9.1・10.6）。
//!
//! 回の置き場は `TempPath::under_target("rr")`（`target\test-roots\` の下）。落ちたかは破棄の時の
//! `std::thread::panicking()` で決め、落ちた回だけ検体の写しを `<回>\sample\` へ複写して残す（D-6・D-7）。

use sample_ghost_kit::SampleRoot;
use std::path::{Path, PathBuf};
use temp_path_kit::TempPath;

/// 中身のあるフォルダを配られたときに取り直す上限（D-7）。
const CREATE_TRIES: usize = 16;

/// 走り 1 回の置き場と検体の写し。破棄で、落ちた回なら残し、成功なら消す。
// launch・run が使うまでは試験の外で使われない。
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) struct RunDirs {
    /// 回の置き場。落ちた回は `forget` して残すので `Option` で持つ。
    dir: Option<TempPath>,
    run: PathBuf,
    root: PathBuf,
    profile: PathBuf,
    sample: Option<SampleRoot>,
}

#[cfg_attr(not(test), allow(dead_code))]
impl RunDirs {
    /// 回の置き場を新しく作り、検体の名前があれば新しい写しを取る（1.2・1.4）。
    ///
    /// # Panics
    ///
    /// 中身の無いフォルダが 16 回で取れないとき・検体の写しを取れないとき（理由つき）。
    pub(crate) fn create(sample: Option<&str>) -> RunDirs {
        RunDirs::create_with(sample, || TempPath::under_target("rr"))
    }

    /// `create` の核。配るフォルダを引数で受ける（兄弟のテストが中身のあるフォルダを配る）。
    fn create_with(sample: Option<&str>, mut fresh: impl FnMut() -> TempPath) -> RunDirs {
        let dir = (0..CREATE_TRIES)
            .find_map(|_| {
                let dir = fresh();
                let empty = std::fs::read_dir(dir.path()).is_ok_and(|mut d| d.next().is_none());
                if empty {
                    Some(dir)
                } else {
                    // 前に残した落ちた回と同じ名前を引いた（番号の使い回し）。消さずに取り直す。
                    std::mem::forget(dir);
                    None
                }
            })
            .unwrap_or_else(|| {
                panic!("回の置き場: 中身の無いフォルダが {CREATE_TRIES} 回で取れなかった")
            });
        let run = dir.path().to_path_buf();
        let root = run.join("root");
        let profile = run.join("profile");
        for path in [&root, &profile] {
            std::fs::create_dir_all(path)
                .unwrap_or_else(|err| panic!("回の置き場: {} ({err})", path.display()));
        }
        let sample = sample.map(|name| {
            SampleRoot::acquire(name)
                .unwrap_or_else(|err| panic!("検体 {name:?} の写しを取れなかった: {err}"))
        });
        RunDirs {
            dir: Some(dir),
            run,
            root,
            profile,
            sample,
        }
    }

    /// 回の置き場（`<target>\test-roots\areka-rr-<pid>-<連番>`）。
    pub(crate) fn run(&self) -> &Path {
        &self.run
    }

    /// 空の根（`<回>\root`）。
    pub(crate) fn root(&self) -> &Path {
        &self.root
    }

    /// 記憶の置き場（`<回>\profile`）。
    pub(crate) fn profile(&self) -> &Path {
        &self.profile
    }

    /// 子の標準出力のファイルの場所（`<回>\stdout.log`）。
    pub(crate) fn stdout_log(&self) -> PathBuf {
        self.run.join("stdout.log")
    }

    /// 子の標準エラーのファイルの場所（`<回>\stderr.log`）。
    pub(crate) fn stderr_log(&self) -> PathBuf {
        self.run.join("stderr.log")
    }

    /// 検体の写し（検体の名前を渡したときだけ）。
    pub(crate) fn sample(&self) -> Option<&SampleRoot> {
        self.sample.as_ref()
    }

    /// 残すか消すかを決める。panic しない（破棄の中で二重の失敗を起こさない）。
    ///
    /// 落ちた回（6.1・6.2・6.4）は写しを `<回>\sample\` へ複写し、置き場を残して場所を
    /// 標準エラーへ 1 行書く。成功（6.3）は何もしない＝置き場と写しの破棄が消す。
    fn finish(&mut self, failed: bool) {
        if !failed {
            return;
        }
        let Some(dir) = self.dir.take() else {
            return; // 既に残した
        };
        if let Some(sample) = &self.sample {
            let to = self.run.join("sample");
            if let Err(err) = areka_user_data::copy_tree_atomic(sample.root(), &to, &self.run) {
                eprintln!(
                    "落ちた回の検体の写しを複写できませんでした: {} -> {} ({err})",
                    sample.root().display(),
                    to.display()
                );
            }
        }
        std::mem::forget(dir);
        eprintln!("落ちた回の証拠を残しました: {}", self.run.display());
    }
}

impl Drop for RunDirs {
    fn drop(&mut self) {
        self.finish(std::thread::panicking());
    }
}

#[cfg(test)]
#[path = "evidence_tests.rs"]
mod tests;

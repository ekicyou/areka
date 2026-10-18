//! `--help` の説明書を開く口（要件 9.1・9.3）。偽の OS の口で確かめる。

use super::*;
use std::ffi::OsString;
use temp_path_kit::TempPath;

/// 呼ばれた呼び出しを積み、決めた結果を返す偽の OS の口。
struct FakeOs {
    calls: Vec<OsCall>,
    result: Result<(), u32>,
}

impl FakeOs {
    fn new(result: Result<(), u32>) -> Self {
        FakeOs {
            calls: Vec::new(),
            result,
        }
    }
}

impl OsPort for FakeOs {
    fn shell_execute(&mut self, call: &OsCall) -> Result<(), u32> {
        self.calls.push(call.clone());
        self.result
    }

    fn env_var(&self, _name: &str) -> Option<String> {
        None
    }
}

/// `target\` の下に README.txt の在る・無いフォルダを作る。
fn exe_dir(label: &str, with_readme: bool) -> TempPath {
    let tmp = TempPath::under_target(label);
    if with_readme {
        std::fs::write(tmp.path().join("README.txt"), "readme").unwrap();
    }
    tmp
}

#[test]
fn opens_existing_readme_once() {
    let tmp = exe_dir("cli-help-exists", true);
    let dir = tmp.path();
    let mut os = FakeOs::new(Ok(()));
    let path = dir.join("README.txt");
    assert_eq!(open_readme(Some(dir), &mut os), Ok(path.clone()));
    assert_eq!(
        os.calls,
        vec![OsCall {
            verb: Verb::Open,
            file: OsString::from(path),
            params: None,
            dir: None,
        }]
    );
}

#[test]
fn missing_readme_is_not_opened() {
    let tmp = exe_dir("cli-help-missing", false);
    let dir = tmp.path();
    let mut os = FakeOs::new(Ok(()));
    assert_eq!(
        open_readme(Some(dir), &mut os),
        Err(HelpError::Missing(dir.join("README.txt")))
    );
    assert!(os.calls.is_empty());
}

#[test]
fn os_refusal_is_reported() {
    let tmp = exe_dir("cli-help-refused", true);
    let dir = tmp.path();
    let mut os = FakeOs::new(Err(1155));
    assert_eq!(
        open_readme(Some(dir), &mut os),
        Err(HelpError::OpenFailed {
            path: dir.join("README.txt"),
            code: 1155,
        })
    );
    assert_eq!(os.calls.len(), 1);
}

#[test]
fn unknown_exe_dir_is_reported() {
    let mut os = FakeOs::new(Ok(()));
    assert_eq!(open_readme(None, &mut os), Err(HelpError::NoExeDir));
    assert!(os.calls.is_empty());
}

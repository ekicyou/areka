//! バルーンの層の置き場と、シェル・バルーンを替えたときの層の付け替えのテスト。
use super::*;

use areka_sylphya::{PersistScope, WriteOutcome};
use temp_path_kit::TempPath;

use super::origin_tests::write_returning_ghost;
use crate::test_support::{fake_options, shutdown_bounded};

const VOLATILE: &str = "myplugin.state";
const KEPT: &str = "areka.boot.count";

fn write_shell(root: &Path, folder: &str) -> PathBuf {
    let dir = root.join("shell").join(folder);
    std::fs::create_dir_all(&dir).expect("create shell dir");
    std::fs::write(dir.join("descript.txt"), b"charset,UTF-8\nname,Other\n")
        .expect("write shell descript.txt");
    dir
}

#[test]
fn switching_shell_or_balloon_re_roots_its_layer() {
    let temp = TempPath::under_target("ghost-runtime-layer-root");
    let root = temp.path().join("ghost");
    write_returning_ghost(&root);
    let shell_a = root.join("shell").join("master");
    let shell_b = write_shell(&root, "B");
    let balloon_a = temp.path().join("balloon").join("A");
    let balloon_b = temp.path().join("balloon").join("B");
    for dir in [&balloon_a, &balloon_b] {
        std::fs::create_dir_all(dir).expect("create balloon dir");
    }

    let mut runtime = boot_with_balloon(
        fake_options(root.clone()),
        None,
        BootOrigin::Plain,
        None,
        Some(&balloon_a),
        None,
    )
    .expect("boot should succeed");
    let kvs = runtime
        .sylphya_reader()
        .kvs(crate::sylphya_wiring::ghost_asker_id(
            &runtime.mount().shiori.dir,
        ));

    for layer in [PersistScope::Shell, PersistScope::Balloon] {
        assert_eq!(kvs.set(layer, VOLATILE, "v"), WriteOutcome::Written);
        assert_eq!(kvs.put(layer, &[(KEPT, "1")]), WriteOutcome::Written);
    }
    assert!(
        balloon_a
            .join("profile")
            .join("areka")
            .join("sylphya.sqlite")
            .is_file()
    );

    runtime.set_shell_dir(shell_b);
    runtime.set_balloon_dir(&balloon_b);
    for layer in [PersistScope::Shell, PersistScope::Balloon] {
        assert_eq!(kvs.get_in(layer, VOLATILE), None, "{layer:?}");
        assert_eq!(kvs.get_in(layer, KEPT), None, "{layer:?}");
    }

    runtime.set_shell_dir(shell_a);
    runtime.set_balloon_dir(&balloon_a);
    for layer in [PersistScope::Shell, PersistScope::Balloon] {
        assert_eq!(kvs.get_in(layer, KEPT).as_deref(), Some("1"), "{layer:?}");
        assert_eq!(kvs.get_in(layer, VOLATILE), None, "{layer:?}");
    }

    drop(kvs);
    shutdown_bounded(runtime);
    drop(temp);
}

#[test]
fn setting_the_same_shell_dir_keeps_its_layer() {
    let temp = TempPath::under_target("ghost-runtime-layer-root-same");
    let root = temp.path().join("ghost");
    write_returning_ghost(&root);

    let mut runtime = boot_with_balloon(
        fake_options(root.clone()),
        None,
        BootOrigin::Plain,
        None,
        None,
        None,
    )
    .expect("boot should succeed");
    let kvs = runtime
        .sylphya_reader()
        .kvs(crate::sylphya_wiring::ghost_asker_id(
            &runtime.mount().shiori.dir,
        ));
    assert_eq!(
        kvs.set(PersistScope::Shell, VOLATILE, "v"),
        WriteOutcome::Written
    );

    let same = runtime.mount().shell.dir.clone();
    runtime.set_shell_dir(same);
    assert_eq!(
        kvs.get_in(PersistScope::Shell, VOLATILE).as_deref(),
        Some("v")
    );

    drop(kvs);
    shutdown_bounded(runtime);
    drop(temp);
}

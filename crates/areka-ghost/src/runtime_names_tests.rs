//! 動作中のゴーストの名前の差し替え（[`GhostRuntime::set_names`]）のテスト（mcp-reload 要件 7.3）。
//!
//! 検体のゴーストは由来つき起動のテストのもの（`name,TestGhost`・`sakura.name` なし）を使い、
//! 差し替えの後にマウントの名前と sylphya の静的プロパティが新しい名前で読めることを見る。
use super::*;
use areka_sylphya::FlatResolution;
use temp_path_kit::TempPath;

use super::origin_tests::{RecordingShiori, write_returning_ghost};
use crate::test_support::{Requests, fake_options, shutdown_bounded};

#[test]
fn set_names_replaces_mount_names_and_republishes_statics() {
    let temp = TempPath::new("ghost-runtime-names");
    let root = temp.path().to_path_buf();
    write_returning_ghost(&root);

    let requests: Requests = Default::default();
    let options = GhostBootOptions {
        shiori: ShioriWiring::Custom(Box::new(move || {
            Ok(Box::new(RecordingShiori(requests)) as Box<dyn ShioriBackend>)
        })),
        ..fake_options(root.clone())
    };
    let mut runtime = boot_with_origin(options, None, BootOrigin::Plain, None, None)
        .expect("boot should succeed");

    let mut names = GhostNames::default();
    names.name = Some("NewGhost".to_string());
    names.sakura_name = Some("あたらしい".to_string());
    names.sakura_name2 = Some("べつめい".to_string());
    names.kero_name = Some("うにゅう".to_string());
    runtime.set_names(names.clone());
    assert_eq!(runtime.mount().names, names);

    runtime
        .sylphya_publisher()
        .barrier()
        .expect("sylphya barrier");
    let asker = AskerContext {
        asker: crate::sylphya_wiring::ghost_asker_id(&runtime.mount().shiori.dir),
    };
    let reader = runtime.sylphya_reader();
    for (key, want) in [
        ("selfname", "あたらしい"),
        ("selfname2", "べつめい"),
        ("keroname", "うにゅう"),
    ] {
        assert_eq!(
            reader.resolve_flat(&asker, key),
            FlatResolution::Value(want.to_string()),
            "{key}"
        );
    }
    assert!(matches!(
        reader.resolve_dotted_str(&asker, "baseware.version"),
        DottedResolution::Value(_)
    ));

    shutdown_bounded(runtime);
    drop(temp);
}

/// 読み直しで新しい名前に無い静的な名前は鏡像から消える（9.15 ⑵）。`sakura.name2` を消した
/// descript の読み直しは、その descript で起動したのと同じ＝`%selfname2` に値が無い（素通し縮退）。
/// `kero.name` も消えると `%keroname` は起動時と同じく `sakura.name` へ戻る。SHIORI 照会層の
/// `username` は静的な名前でないので残る。
#[test]
fn set_names_without_name2_removes_selfname2_from_the_mirror() {
    let temp = TempPath::new("ghost-runtime-names-removed");
    let root = temp.path().to_path_buf();
    write_returning_ghost(&root);

    let requests: Requests = Default::default();
    let options = GhostBootOptions {
        shiori: ShioriWiring::Custom(Box::new(move || {
            Ok(Box::new(RecordingShiori(requests)) as Box<dyn ShioriBackend>)
        })),
        ..fake_options(root.clone())
    };
    let mut runtime = boot_with_origin(options, None, BootOrigin::Plain, None, None)
        .expect("boot should succeed");
    let asker = AskerContext {
        asker: crate::sylphya_wiring::ghost_asker_id(&runtime.mount().shiori.dir),
    };

    let mut names = GhostNames::default();
    names.sakura_name = Some("あたらしい".to_string());
    names.sakura_name2 = Some("べつめい".to_string());
    names.kero_name = Some("うにゅう".to_string());
    runtime.set_names(names);
    runtime.sylphya_publisher().publish_shiori(
        asker.asker.clone(),
        "username".to_string(),
        Some("ユーザー".to_string()),
    );

    let mut names = GhostNames::default();
    names.sakura_name = Some("つぎ".to_string());
    runtime.set_names(names);
    runtime
        .sylphya_publisher()
        .barrier()
        .expect("sylphya barrier");

    let reader = runtime.sylphya_reader();
    assert!(
        matches!(
            reader.resolve_flat(&asker, "selfname2"),
            FlatResolution::Degraded(_)
        ),
        "selfname2 must be gone after the reload"
    );
    for (key, want) in [
        ("selfname", "つぎ"),
        ("keroname", "つぎ"),
        ("username", "ユーザー"),
    ] {
        assert_eq!(
            reader.resolve_flat(&asker, key),
            FlatResolution::Value(want.to_string()),
            "{key}"
        );
    }

    shutdown_bounded(runtime);
    drop(temp);
}

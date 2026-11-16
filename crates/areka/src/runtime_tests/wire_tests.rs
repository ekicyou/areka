use super::*;

/// 観測可能な完了条件（tasks.md task 5.1）: 存在しない ghost_root に対し
/// `wire_runtime` は `wired=false`（帰結は呼び手が決める・R7.3）を返し、
/// ghost／seriko ハンドルは `None` となる（実 sink 結線を成立させない決定分岐の檻）。
///
/// 素の `World::new()` で足りる。`build_boot_assets` が `resolve` 起点不在で早期 `Err` を返すため、
/// World には一切触れず、spawn_emo_text／spawn_seriko／boot へも到達しない（＝アクタースレッドを
/// 起こさない）ので決定論的に判定できる。
#[test]
fn wire_runtime_falls_back_to_unwired_on_missing_ghost_root() {
    let mut world = World::new();
    let inputs = RuntimeInputs {
        ghost_root: PathBuf::from("C:/areka-nonexistent-ghost-root-runtime-5v1"),
        balloon_root: PathBuf::from("C:/areka-nonexistent-balloon-root-runtime-5v1"),
        shiori: ShioriWiring::Helper {
            helper_exe: PathBuf::from("shiori-host32-helper.exe"),
        },
        ticker: TickerMode::Disabled,
        app_profile_dir: None,
        boot_origin: BootOrigin::Plain,
    };

    let outcome = wire_runtime(
        &mut world,
        inputs,
        AuthorDpi::DEFAULT,
        None,
        std::sync::mpsc::channel().0,
        None,
        &DefaultSurfaces::builtin(),
        crate::session::runtime_boot::boot_runtime,
    );

    assert!(
        !outcome.wired,
        "存在しない ghost_root では実 sink 結線は成立せず wired=false を返す（R7.3）"
    );
    assert!(
        outcome.ghost.is_none(),
        "不成立時は ghost ランタイムを起動しない（None）"
    );
    assert!(
        outcome.seriko.is_none(),
        "不成立時は seriko アクターを起こさない（None）"
    );
}

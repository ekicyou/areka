//! 起動から終わりまでを読み取りだけで通し、根の木の前後を比べる（readonly-mode 要件 2.2・2.3・2.4・
//! 3.2・4.1・4.3・4.4・7.1・7.3・7.6・8.1・8.2・8.4・8.5・10.2・10.5）。
//!
//! 1 つの根（`SwitchRig` の検体の根＝`target\` の下の exe の隣の形）の中に、まだ無い利用者ごとの
//! 置き場の場所を置き、前の木を取る。読み取りだけの道（置き場を決める → 記憶の置き場を選ぶ →
//! 書けるかを確かめる → ゴーストの写しを決める）を通し、根の取っ手（[`WithholdGuard::under`]）と
//! World の印（[`ReadOnlyRun`]）の下で、A を起こす → 台本の切替命令で B へ切り替える → 窓の位置を
//! 覚えさせる → プロパティを変える → 置き場の並びに足す → インストールと更新を頼む → 降ろして
//! 後始末する、を順に踏む。判定するのは次の 3 つ。
//! - 後の木が前の木と同じ（利用者ごとの置き場も作られない）
//! - A・B の SHIORI が受けた要求に `OnInstall*` が 0 件、`OnUpdate*` は `OnUpdateResultEx` → `OnUpdateResult` だけ
//! - 走りの途中では、変えた値が保存の口から読み返せる
//!
//! プロセスに 1 つの旗は立てない（取っ手と印だけで立てる・構造の試験が固める）。

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

use areka_ghost::prop_sink::PropSetCueSink;
use areka_ghost::sylphya_wiring::profile_areka_root;
use areka_ghost::{BasewareRoots, StoreKind};
use areka_kanade::KanadeStopCause;
use areka_sakura::contract::{ActorKey, CueCommand, CueSink, TalkCue};
use areka_sylphya::persist::FsPersistIo;
use areka_sylphya::persist::withhold::WithholdGuard;
use areka_sylphya::{PersistKey, PersistScope, ScopeRoots, SylphyaPublisher, load_scope};
use areka_user_data::{Signals, Writability, establish_readonly_from, readonly_app_profile_dir};
use log_capture_kit::capture;
use wintf::ecs::widget::bitmap_source::WintfTaskPool;

use crate::app_exit::{ExitOrigin, FirstExit, quit_app};
use crate::boot_config::BootContext;
use crate::emo2_boot::ghost_switch::SwitchInFlight;
use crate::emo2_boot::ghost_switch_test_support::{FakeShiori, SwitchRig, standard_script};
use crate::emo2_boot::spine::RecordedCall;
use crate::ghost_session::GhostSlot;
use crate::install::{InstallOrder, InstallOrigin};
use crate::placement::persist::{char_pos_entries, persist_entries};
use crate::placement::resolver::PointPx;
use crate::readonly::ReadOnlyRun;
use crate::roots::user_root::copy_unless_readonly;
use crate::update::{RawUpdateRequest, TargetKind, UpdateReason};
use crate::write_failure::check_boot;
use crate::{MarkVerdict, Teardown, after_run, insert_persist_wiring, settle_session_mark};

/// A の `OnBoot` の台本: 台詞の途中で B への切替を命じる。
const A_TO_B: &str = "\\0A\\![change,ghost,B,--option=raise-event]\\e";

/// 根の下の木（相対パス → ファイルの中身。フォルダは `None`）。
fn tree(root: &Path) -> BTreeMap<PathBuf, Option<Vec<u8>>> {
    fn walk(root: &Path, dir: &Path, out: &mut BTreeMap<PathBuf, Option<Vec<u8>>>) {
        for entry in std::fs::read_dir(dir).expect("木を歩く") {
            let path = entry.expect("ディレクトリ項目").path();
            let rel = path.strip_prefix(root).expect("根の下").to_path_buf();
            if path.is_dir() {
                out.insert(rel, None);
                walk(root, &path, out);
            } else {
                out.insert(rel, Some(std::fs::read(&path).expect("ファイルを読む")));
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(root, root, &mut out);
    assert!(!out.is_empty(), "走査が空振りした: {}", root.display());
    out
}

/// 保存の口から 1 つの key を読み返す（控えを先に見る＝書けたときと同じ値）。
fn read_back(app: &Path, ghost: &Path, scope: PersistScope, key: PersistKey) -> Option<String> {
    let roots = ScopeRoots {
        app: Some(app.to_path_buf()),
        ghost: Some(ghost.to_path_buf()),
        ..ScopeRoots::default()
    };
    load_scope(scope, &roots, &FsPersistIo)
        .into_iter()
        .find_map(|(k, v)| (k == key).then_some(v))
}

fn publisher_of(rig: &SwitchRig) -> SylphyaPublisher {
    rig.world
        .non_send::<GhostSlot>()
        .0
        .as_ref()
        .and_then(|s| s.runtime())
        .expect("ゴーストの実行系が起きている")
        .sylphya_publisher()
        .clone()
}

fn prop_set_cue(key: PersistKey, value: &str) -> TalkCue {
    TalkCue {
        at: 0.0,
        actor: ActorKey::from("sakura"),
        command: CueCommand::command_carrier(
            areka_ghost::prop_sink::PROP_SET_CUE_NAME,
            vec![key.to_canonical_key(), value.to_owned()],
        ),
        duration: 0.0,
    }
}

/// 呼出列の id（GET と NOTIFY）。
fn ids(calls: &[Vec<RecordedCall>]) -> Vec<String> {
    calls
        .iter()
        .flatten()
        .filter_map(|c| match c {
            RecordedCall::Get { id, .. } | RecordedCall::Notify { id, .. } => Some(id.clone()),
            _ => None,
        })
        .collect()
}

#[test]
fn readonly_run_leaves_the_tree_as_it_was() {
    let a = FakeShiori::Scripted(Box::new(|| {
        standard_script(A_TO_B)
            .get(
                "OnGhostChanging",
                Ok(Some("\\0Bへ交代します\\e".to_owned())),
            )
            .get("OnGhostChanged", Ok(None))
    }));
    let b = FakeShiori::Scripted(Box::new(|| {
        // 断った更新の総括（返事なし）に 204 で答える。A には台本が無い＝A へ届けば偽の SHIORI が落ちる。
        standard_script("\\0B\\e")
            .get("OnGhostChanged", Ok(None))
            .get("OnUpdateResultEx", Ok(None))
            .get("OnUpdateResult", Ok(None))
    }));
    let mut rig = SwitchRig::new(vec![("A", a), ("B", b)]);
    // 切替先の窓の準備が閉包を投函する先（取り出しの系は回さないので走らない）。
    rig.world.insert_resource(WintfTaskPool::with_threads(1));
    rig.plant_boot_record("A");
    rig.plant_boot_record("B");
    rig.wire_app_memory();
    rig.world.insert_resource(ReadOnlyRun);

    let root = rig.root.dir().to_path_buf();
    let app = rig.app_dir();
    let ghost_b = profile_areka_root(&rig.root.ghost_dir("B").join("ghost").join("master"));
    // まだ無い利用者ごとの置き場（根の中に置くので、作られれば木の差に出る）。
    let user_root = root.join("user-root");
    let picked = root.join("picked");
    std::fs::create_dir_all(&picked).unwrap();
    let before = tree(&root);
    assert!(!user_root.exists());

    let guard = WithholdGuard::under(&root);
    let mut failed: Vec<String> = Vec::new();
    let mut check = |ok: bool, what: &str| {
        if !ok {
            failed.push(what.to_owned());
        }
    };

    let ((), events) = capture(|| {
        // --- 読み取りだけの道（起動の前） ---
        let signals = Signals {
            user_root_env: Some(OsString::from(&user_root)),
            local_app_data: None,
            winget_marker: None,
            writability: Writability::Writable,
        };
        let layout = establish_readonly_from(&signals, Some(&root), None);
        check(layout.user_root.is_none(), "まだ無い書ける根を使う形にした");
        // 初回の起動の形（隣の形・書ける根はまだ無い）: 記憶の置き場は exe の隣の `profile\areka` で、
        // 書ける根の下ではない（要件 4.3）。
        let chosen = readonly_app_profile_dir(None, &layout, Some(&root));
        let beside = root.join("profile").join("areka");
        check(
            chosen.as_deref() == Some(beside.as_path())
                && !chosen.as_ref().is_some_and(|d| d.starts_with(&user_root)),
            &format!("記憶の置き場が exe の隣でない: {chosen:?}"),
        );
        // 走りの App の記憶は土台の置き場（`root\profile`）を使う（本番の配置と違うのは土台の都合・
        // 根の木を丸ごと比べるので、どちらへの書き込みも差に出る）。
        check(
            check_boot(true, &rig.cfg("A"), &app).is_ok(),
            "書けるかの確かめが失敗を返した",
        );
        let roots = BasewareRoots::single(root.clone());
        let ghost_a_dir = rig.root.ghost_dir("A");
        check(
            copy_unless_readonly(true, &roots, Some(&root), StoreKind::Ghost, &ghost_a_dir)
                == ghost_a_dir,
            "ゴーストの写しが元のフォルダでない",
        );

        // --- 取っ手と印の下の走り ---
        rig.boot("A");
        check(rig.wait_steady(), "A が定常に届かない");
        let welcomed = rig.pump_talking_until(|rig| {
            rig.exit_requested()
                || (!rig.calls("B").is_empty()
                    && rig.world.get_non_send::<SwitchInFlight>().is_none())
        });
        check(welcomed && !rig.exit_requested(), "B への切替が済まない");

        let publisher = publisher_of(&rig);
        insert_persist_wiring(&mut rig.world, publisher.clone());
        persist_entries(&rig.world, char_pos_entries(0, PointPx { x: 10, y: 20 }));
        PropSetCueSink::new(publisher.clone()).emit(prop_set_cue(PersistKey::BootCount, "7"));
        rig.world.resource_mut::<BootContext>().roots_origin = crate::roots::RootsOrigin::Default;
        crate::roots::add_root(&mut rig.world, picked.clone());
        publisher.barrier().expect("記憶の書き手は生きている");

        // 走りの途中: 変えた値が読み返せる。
        let roots_memory = crate::roots::encode_memory(
            &rig.world.resource::<BootContext>().roots,
            crate::roots::user_root::exe_dir().as_deref(),
        );
        let window_x = PersistKey::WindowPos {
            scope: 0,
            axis: areka_sylphya::Axis::X,
        };
        for (scope, key, want) in [
            (PersistScope::App, PersistKey::LastGhost, "B".to_owned()),
            (PersistScope::Ghost, window_x, "10".to_owned()),
            (PersistScope::Ghost, PersistKey::BootCount, "7".to_owned()),
            (PersistScope::App, PersistKey::BasewareRoots, roots_memory),
        ] {
            let got = read_back(&app, &ghost_b, scope, key);
            check(
                got.as_deref() == Some(want.as_str()),
                &format!("{key:?}: 読み返し {got:?} ≠ 期待 {want:?}"),
            );
        }

        // インストールと更新を頼む（どちらも断られ、更新は総括の 2 件だけを B へ送る）。
        let install = crate::install::submit(
            &mut rig.world,
            InstallOrder {
                archives: vec![root.join("drop.nar")],
                origin: InstallOrigin::WindowDrop,
            },
        );
        check(
            install == crate::install::SubmitVerdict::ReadOnly,
            "インストールを断らない",
        );
        let update = crate::update::submit(
            &mut rig.world,
            RawUpdateRequest::Current(vec![TargetKind::Ghost]),
            UpdateReason::Manual,
        );
        check(
            update == crate::update::SubmitVerdict::ReadOnly,
            "更新を断らない",
        );
        check(
            rig.wait_for(|| ids(&rig.calls("B")).iter().any(|id| id == "OnUpdateResult")),
            "B に更新の総括が届かない",
        );

        // 降ろして後始末する（`fn main` と同じ順）。
        quit_app(
            &mut rig.world,
            ExitOrigin::KanadeStopped(KanadeStopCause::Quit),
        );
        let mut after = after_run(&mut rig.world);
        rig.world.insert_non_send(GhostSlot(after.session.take()));
        let down_ok = rig.shutdown();
        check(down_ok, "降ろせない");
        let verdict = after.mark.as_ref().map(|mark| {
            settle_session_mark(
                mark,
                Teardown {
                    run_ok: true,
                    down_ok,
                    shiori_cut: false,
                },
            )
        });
        check(
            matches!(verdict, Some((MarkVerdict::Clear, None))),
            &format!("印の後始末が消す・失敗なしでない: {verdict:?}"),
        );
        check(
            rig.world
                .get_resource::<FirstExit>()
                .is_some_and(|f| crate::app_exit::write_failure_of(&f.0).is_none()),
            "書けない失敗が致命の机に届いた",
        );
    });
    drop(guard);

    // SHIORI が受けた要求: OnInstall* は 0 件・OnUpdate* は Ex → Result だけ。
    let all: Vec<String> = ["A", "B"]
        .into_iter()
        .flat_map(|folder| ids(&rig.calls(folder)))
        .collect();
    check(
        all.iter().all(|id| !id.starts_with("OnInstall")),
        "OnInstall* を送った",
    );
    let updates: Vec<&str> = all
        .iter()
        .filter(|id| id.starts_with("OnUpdate"))
        .map(String::as_str)
        .collect();
    check(
        updates == ["OnUpdateResultEx", "OnUpdateResult"],
        &format!("OnUpdate* が総括の 2 件でない: {updates:?}"),
    );

    let count = |name: &str| {
        events
            .iter()
            .filter(|e| e.field_str("event") == Some(name))
            .count()
    };
    for (name, want) in [
        ("user_root_withheld", 1),
        ("readonly_boot_check_skipped", 1),
        ("readonly_refused", 2),
        ("user_data_copied", 0),
        ("user_data_copy_withheld", 0),
    ] {
        check(
            count(name) == want,
            &format!("記録 {name} が {} 件（期待 {want}）", count(name)),
        );
    }

    check(!user_root.exists(), "利用者ごとの置き場が作られた");
    check(tree(&root) == before, "根の木が変わった");
    assert!(
        failed.is_empty(),
        "読み取りだけの走りで守れなかったこと: {failed:#?}"
    );
}

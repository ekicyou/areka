//! ghost の読み直しでシェルの着せ替えを持ち越す送り方の決定論テスト。
//!
//! - 読み直しでなければ控えない（印が無い World は `None`）
//! - 送り方: 置き場のゴーストの seriko へ `\![bind,…]`・`\s[面]` と同じ合図が着せ替えから先に届き、
//!    seriko の最初の `Show` がもう着せ替えた集合を載せる
//!
//! 面の決め方・控えの組み方・着せ替えの決め方（`decide_binds`）は、`areka-runtime` の同じ名前のファイルにある。

use areka_parsers::package::{BindGroupDefaults, BindGroupName, ScopeBinds};
use areka_seriko::{
    BindResolver, DisplayCommand, MockSurfaceOutput, SerikoLoopConfig, SurfaceResolver,
    spawn_seriko,
};
use std::collections::BTreeMap;
use std::path::PathBuf;

use super::test_support::set;
use super::*;
use areka_session::ghost_session::{GhostSession, GhostSlot};
use areka_session::reload::carries_shell_state;

/// 読み直しの印が無ければ控えない（ふつうの切替・起動は今までどおり）。控えが無ければ渡しもしない。
#[test]
fn nothing_is_carried_without_a_ghost_reload() {
    let mut world = World::new();
    let carry = capture_for_reload(&world, carries_shell_state(&world));
    assert_eq!(carry, None);
    hand_over(&mut world, carry);
    assert!(!world.contains_non_send::<RuntimeWiring>());
}

/// 着せ替えごとに `\![bind,…]`、決めた面ごとに `\s[面]` と同じ合図が着せ替えから先に届き、seriko の
/// scope 0 の**最初の** `Show` がもう着せ替えた集合（101）を載せる（既定の姿が 1 度も出ない）。出さない
/// 面は届かない。
///
/// # 非空虚性
/// `emit_shows` が送らなければ面の `Show` が無く赤。`Skipped` を送れば scope 2 の `Show` が出て赤。
/// 面を着せ替えより先に送ると、scope 0 の最初の `Show` が既定の 100 を載せ、続けて 101 の `Show` が出て赤。
#[test]
fn replay_shows_the_decided_surfaces_and_binds_through_seriko() {
    let out = MockSurfaceOutput::new();
    let records = out.records();
    let (sink, seriko) = spawn_seriko(
        SurfaceResolver::new(BTreeMap::new()),
        areka_seriko::DefaultBinds::uniform(set(&[100])),
        BindResolver::from_groups(&BindGroupDefaults::from([(
            0,
            ScopeBinds {
                names: vec![
                    BindGroupName {
                        id: 100,
                        category: "服".into(),
                        part: "A".into(),
                        thumbnail: None,
                        line: 1,
                    },
                    BindGroupName {
                        id: 101,
                        category: "服".into(),
                        part: "B".into(),
                        thumbnail: None,
                        line: 1,
                    },
                ],
                ..Default::default()
            },
        )])),
        SerikoLoopConfig::disabled(),
        out,
    );
    let mut world = World::new();
    areka_session::hooks::install(&mut world);
    world.insert_non_send(GhostSlot(Some(
        GhostSession::for_test(None, PathBuf::from("ghost/A")).with_seriko_sink(sink),
    )));

    replay(
        &world,
        &[
            (0, CarriedShow::Carried(1000)),
            (
                1,
                CarriedShow::FellBack {
                    missing: 2011,
                    to: 10,
                    passed_over: None,
                },
            ),
            (
                2,
                CarriedShow::Skipped {
                    missing: 3000,
                    descript: 10,
                },
            ),
        ],
        &[(
            0,
            CarriedBinds {
                replays: vec![BindReplay {
                    category: "服".into(),
                    part: "B".into(),
                    on: true,
                }],
                dropped: vec![999],
            },
        )],
    );
    // 置き場のゴーストが持つ送り手を落とすとアクターの受信端が切れて終わる。
    drop(world);
    areka_runtime::spine::join_bounded("shell carry seriko join", seriko)
        .expect("seriko worker should end once the ghost's sink is dropped");

    let shown: Vec<(String, u32, Vec<u32>)> = records
        .lock()
        .unwrap()
        .iter()
        .filter_map(|cmd| match cmd {
            DisplayCommand::Show {
                scope,
                surface_id,
                binds,
                ..
            } => Some((scope.as_str().to_owned(), *surface_id, binds.ids().to_vec())),
            _ => None,
        })
        .collect();
    assert_eq!(
        shown,
        vec![
            ("0".to_owned(), 1000, vec![101]),
            ("1".to_owned(), 10, vec![100]),
        ],
        "{shown:?}"
    );
}

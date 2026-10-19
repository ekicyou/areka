//! 装着の相が ghost の読み直しで控えた面を出し直す檻（areka-P0-mcp-reload 要件 3.8・12.1）。
//!
//! 実 emo2 fixture＋実 GPU で本番 [`run_attach_phase`] を駆動し、置き場のゴーストの seriko（出力は捕まえる
//! だけ）へ届いた `Show` を見る。
//!
//! ⑴ 控えあり: 在る面はそのまま、無くなった面は既定の面（scope 1 は 10）で出す
//! ⑵ 控えなし（ふつうの起動・切替）: 装着の相はシェルの面を 1 つも出さない（defect #5 のまま）

use std::collections::BTreeMap;
use std::path::PathBuf;

use areka_seriko::{
    DisplayCommand, MockSurfaceOutput, SerikoLoopConfig, SurfaceResolver, spawn_seriko,
};

use super::*;
use crate::emo2_boot::shell_carry::{ShellCarry, snapshot};
use crate::ghost_session::{GhostSession, GhostSlot};

/// 面の番号がシェルに無いと分かっている値。
const GONE: u32 = 987_654;

/// seriko が出した `Show` の `(scope, 面, 着せ替えの ID)`。
type Shown = Vec<(String, u32, Vec<u32>)>;

/// 控え `carry` を持たせて装着の相を 1 回回し、seriko（本物の名前表と既定の着せ替え）が出した `Show` を返す。
fn shows_after_attach(carry: impl FnOnce(&BootAssets) -> Option<ShellCarry>) -> Shown {
    let (_gpu, mut world, _gw) = gpu_attach_world();
    let assets = build_boot_assets(&emo2_root(), &emo2_balloon_root(), &[0, 1], 96, 96)
        .expect("emo2 fixture の BootAssets 組立は成功する");
    let carry = carry(&assets);
    let out = MockSurfaceOutput::new();
    let records = out.records();
    let (sink, seriko) = spawn_seriko(
        SurfaceResolver::new(BTreeMap::new()),
        assets.static_binds.clone(),
        assets.bind_resolver.clone(),
        SerikoLoopConfig::disabled(),
        out,
    );
    world.insert_non_send(GhostSlot(Some(
        GhostSession::for_test(None, PathBuf::from("ghost/emo2")).with_seriko_sink(sink),
    )));
    let mut wiring = Emo2Wiring::new(
        EmoPresenter::new(),
        mpsc::channel::<PresentCommand>().1,
        mpsc::channel::<PositionCue>().1,
        mpsc::channel::<TalkLifecycleSignal>().1,
        mpsc::channel::<crate::emo2_boot::zorder_cue::ZOrderDirective>().1,
        Rc::new(RefCell::new(TextLayerRuntime::new(
            TextLayerConfig::default(),
        ))),
        zero_clock(),
        assets,
    );
    wiring.shell_carry = carry;

    run_attach_phase(&mut wiring, &mut world);
    assert!(wiring.attached, "前提: ゲート成立で attach が走る");
    assert!(wiring.shell_carry.is_none(), "控えは 1 度で使い切る");
    // 置き場のゴーストが持つ送り手を落とすとアクターの受信端が切れて終わる。
    drop(world);
    seriko.join().unwrap();
    records
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
        .collect()
}

/// ⑴ 控えた面のうち在るものはそのまま、無くなったものは既定の面で出す。
///
/// # 非空虚性
/// 装着の相が控えを使わなければ `Show` が 0 件で赤。無くなった面を確かめずに送れば scope 1 が `GONE` で赤。
#[test]
fn attach_replays_the_carried_surfaces_and_falls_back_for_a_missing_one() {
    let mut kept = None;
    let shown = shows_after_attach(|assets| {
        let ids: Vec<u32> = assets.shells[0].emo_world.surface_ids().collect();
        assert!(ids.contains(&10), "前提: emo2 のシェルに既定の面 10 が在る");
        assert!(!ids.contains(&GONE), "前提: {GONE} は無い");
        // 既定（0）でない在る面を選ぶ（既定へ戻したのと区別できるように）。
        let id = *ids
            .iter()
            .find(|&&id| id != 0 && id != 10)
            .expect("前提: 面が 3 つ以上");
        kept = Some(id);
        Some(snapshot(
            [0, 1],
            |scope| Some(if scope == 0 { id } else { GONE }),
            |_| None,
            assets.static_binds.clone(),
        ))
    });
    let kept = kept.unwrap();
    let faces: Vec<(String, u32)> = shown.iter().map(|(s, id, _)| (s.clone(), *id)).collect();
    assert_eq!(
        faces,
        vec![("0".to_owned(), kept), ("1".to_owned(), 10)],
        "{shown:?}"
    );
}

/// ⑶ 控えた着せ替えのうち、読み直したシェルに名前の在るもの（腕＝組み 1101）は着せ直し、名前の無い
/// もの（`GONE`）は落とす。腕は複数可でないので、既定の腕＝伸び（1100）は排他の置き換えで外れる。
///
/// # 非空虚性
/// 装着の相が着せ替えを出し直さなければ、scope 0 の `Show` が既定（1100 を含む）で赤。面を着せ替えより
/// 先に送ると、最初の `Show` が既定の姿で赤。
#[test]
fn attach_replays_the_carried_binds_and_drops_an_undefined_one() {
    let mut expected = Vec::new();
    let shown = shows_after_attach(|assets| {
        let defaults = assets.static_binds.ids().to_vec();
        assert!(
            defaults.contains(&1100) && !defaults.contains(&1101),
            "前提: emo2 の既定は 腕＝伸び（1100）: {defaults:?}"
        );
        let worn: Vec<u32> = defaults
            .iter()
            .copied()
            .filter(|id| *id != 1100)
            .chain([1101])
            .collect();
        expected = areka_emo_compose::BindSet::from_ids(worn.iter().copied())
            .ids()
            .to_vec();
        let carried = areka_emo_compose::BindSet::from_ids(worn.into_iter().chain([GONE]));
        // 読み直しの前の既定も同じ（descript を書き換えていない）。名前の無い `GONE` は台本で着た分として
        // 控えに載り、読み直したシェルで落ちる。
        let old_static = assets.static_binds.clone();
        Some(snapshot(
            [0],
            |_| Some(0),
            move |_| Some(carried.clone()),
            old_static,
        ))
    });
    // 着せ替えを面より先に送るので、scope 0 の最初の `Show` からもう着せ替えた集合（既定の姿は出ない）。
    let first = shown.iter().find(|(s, ..)| s == "0").cloned();
    assert_eq!(first, Some(("0".to_owned(), 0, expected)), "{shown:?}");
}

/// ⑵ 控えが無ければ装着の相はシェルの面を出さない（最初の `\s` まで非表示のまま・defect #5）。
#[test]
fn attach_without_a_carry_shows_no_shell_surface() {
    let shown = shows_after_attach(|_| None);
    assert!(shown.is_empty(), "{shown:?}");
}

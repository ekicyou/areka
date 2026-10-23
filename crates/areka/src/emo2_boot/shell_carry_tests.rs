//! ghost の読み直しでシェルの面と着せ替えを持ち越す決め方と送り方の決定論テスト
//! （areka-P0-mcp-reload 要件 3.8・12.1）。
//!
//! ⑴ 面の決め方: 控えた面が在れば出す／無ければ既定の面へ戻す／どちらも無ければ出さない
//! ⑵ 控えの組み方: 表示中のスコープだけ面を載せ、表示したことのあるスコープは着せ替えも載せる
//! ⑶ 読み直しでなければ控えない（印が無い World は `None`）
//! ⑷ 着せ替えの決め方（読み直しの前の既定との差だけを持ち越す）: 着ていたものは着る／名前の無くなった
//!    ものは落とす／違いが無ければ何も送らない／脱いでいた既定は脱ぐ（同じカテゴリを着る排他の置き換え
//!    があれば送らない）／scope 2 以降は自分（`char{N}`）の名前表で引く／descript の既定の書き換え
//!    （1→0・0→1）は持ち越さない
//! ⑸ 送り方: 置き場のゴーストの seriko へ `\![bind,…]`・`\s[面]` と同じ合図が着せ替えから先に届き、
//!    seriko の最初の `Show` がもう着せ替えた集合を載せる

use std::collections::BTreeMap;
use std::path::PathBuf;

use areka_emo_compose::BindSet;
use areka_seriko::{
    BindOptionDecls, BindResolver, DisplayCommand, MockSurfaceOutput, SerikoLoopConfig,
    SurfaceResolver, spawn_seriko,
};

use super::*;
use crate::ghost_session::GhostSession;

/// 在る面の集合（読み直したシェルの代わり）。
fn shell_with(ids: &'static [u32]) -> impl Fn(u32) -> bool {
    move |id| ids.contains(&id)
}

/// 本体側の名前表: 服（A=100・B=101）・帽子（C=200）。`multiple` の宣言は `multiple` に挙げたカテゴリ。
fn names(multiple: &[&str]) -> BindResolver {
    let sakura = BTreeMap::from([
        (("服".to_owned(), "A".to_owned()), 100),
        (("服".to_owned(), "B".to_owned()), 101),
        (("帽子".to_owned(), "C".to_owned()), 200),
    ]);
    BindResolver::new(
        sakura,
        BTreeMap::new(),
        BindOptionDecls {
            sakura_multiple: multiple.iter().map(|c| (*c).to_owned()).collect(),
            ..BindOptionDecls::default()
        },
    )
}

fn set(ids: &[u32]) -> BindSet {
    BindSet::from_ids(ids.iter().copied())
}

fn wear(category: &str, part: &str, on: bool) -> BindReplay {
    BindReplay {
        category: category.to_owned(),
        part: part.to_owned(),
        on,
    }
}

/// ⑴ 控えた面が読み直したシェルに在ればその面、無ければ既定の面、どちらも無ければ出さない。
#[test]
fn decide_keeps_the_carried_surface_or_falls_back_to_the_default() {
    assert_eq!(
        decide(1000, 0, shell_with(&[0, 1000])),
        CarriedShow::Carried(1000)
    );
    assert_eq!(
        decide(1000, 0, shell_with(&[0, 5])),
        CarriedShow::FellBack {
            missing: 1000,
            to: 0
        }
    );
    assert_eq!(
        decide(1000, 10, shell_with(&[0])),
        CarriedShow::Skipped { missing: 1000 }
    );
    assert_eq!(CarriedShow::Carried(7).surface(), Some(7));
    assert_eq!(
        CarriedShow::FellBack { missing: 7, to: 10 }.surface(),
        Some(10)
    );
    assert_eq!(CarriedShow::Skipped { missing: 7 }.surface(), None);
}

/// ⑵ 面は表示中のスコープだけ（scope 1 は非表示＝`None`）、着せ替えは表示したことのあるスコープだけ載る。
#[test]
fn snapshot_keeps_only_the_shown_scopes() {
    let carry = snapshot(
        [0, 1, 2],
        |scope| match scope {
            0 => Some(1000),
            2 => Some(2010),
            _ => None,
        },
        |scope| (scope != 2).then(|| set(&[scope + 100])),
        areka_seriko::DefaultBinds::uniform(set(&[100])),
    );
    assert_eq!(carry.surface(0), Some(1000));
    assert_eq!(carry.surface(1), None);
    assert_eq!(carry.surface(2), Some(2010));
    assert_eq!(carry.surface(3), None);
    assert_eq!(carry.binds(0), Some(&set(&[100])));
    assert_eq!(carry.binds(1), Some(&set(&[101])));
    assert_eq!(carry.binds(2), None);
    assert_eq!(
        carry.old_static(),
        &areka_seriko::DefaultBinds::uniform(set(&[100]))
    );
}

/// ⑶ 読み直しの印が無ければ控えない（ふつうの切替・起動は今までどおり）。控えが無ければ渡しもしない。
#[test]
fn nothing_is_carried_without_a_ghost_reload() {
    let mut world = World::new();
    let carry = capture_for_reload(&world);
    assert_eq!(carry, None);
    hand_over(&mut world, carry);
    assert!(!world.contains_non_send::<Emo2Wiring>());
}

/// 控えを渡す先の結線が無ければ `warn!` を 1 件残して捨てる（panic しない）。
#[test]
fn hand_over_without_wiring_warns_and_drops_the_carry() {
    let mut world = World::new();
    let ((), events) = log_capture_kit::capture(|| {
        hand_over(&mut world, Some(ShellCarry::default()));
    });
    let warned = events
        .iter()
        .filter(|e| e.field_str("event") == Some("reload_shell_carry_unavailable"))
        .count();
    assert_eq!(warned, 1, "{events:?}");
    assert!(!world.contains_non_send::<Emo2Wiring>());
}

/// ⑷ 着ていた着せ替え（既定に無い 101）は名前で着る。名前の無くなった着せ替え（999）は落とす。
#[test]
fn decide_binds_keeps_a_worn_bind_and_drops_an_undefined_one() {
    let decided = decide_binds(0, &set(&[101, 999]), &set(&[]), &set(&[]), &names(&[]));
    assert_eq!(decided.replays, vec![wear("服", "B", true)]);
    assert_eq!(decided.dropped, vec![999]);
}

/// ⑷ 控えと読み直したシェルの既定が同じ（着せ替えを変えていない）なら何も送らない。
#[test]
fn decide_binds_sends_nothing_without_a_difference() {
    let decided = decide_binds(0, &set(&[100]), &set(&[100]), &set(&[100]), &names(&[]));
    assert_eq!(decided, CarriedBinds::default());
}

/// ⑷ 脱いでいた既定（200）は脱ぐ。同じカテゴリを着る合図（服 B）があり複数可でなければ、既定の服 A は
/// 脱ぐ合図を送らない（排他の置き換えが外す）。複数可のカテゴリなら脱ぐ合図も送る。
#[test]
fn decide_binds_sheds_a_removed_default_unless_an_exclusive_wear_covers_it() {
    let decided = decide_binds(
        0,
        &set(&[101]),
        &set(&[100, 200]),
        &set(&[100, 200]),
        &names(&[]),
    );
    assert_eq!(
        decided.replays,
        vec![wear("服", "B", true), wear("帽子", "C", false)]
    );
    assert!(decided.dropped.is_empty());

    let decided = decide_binds(0, &set(&[101]), &set(&[100]), &set(&[100]), &names(&["服"]));
    assert_eq!(
        decided.replays,
        vec![wear("服", "B", true), wear("服", "A", false)]
    );
}

/// `char2.*` だけに服 B（101）の名前を持つシェルの着せ替えの名前表（作業フォルダ＝`target\\` の下に
/// ゴーストを組み、シェルの読みから作る）。服 A（100）の名前は持たない。
fn char2_names() -> BindResolver {
    let work = sample_ghost_kit::WorkDir::new().expect("作業フォルダを取れる");
    let master = work.path().join("ghost").join("master");
    std::fs::create_dir_all(&master).expect("ghost/master を作る");
    std::fs::write(master.join("descript.txt"), "charset,UTF-8\r\nname,t\r\n")
        .expect("ゴーストの descript.txt を書く");
    let shell = work.path().join("shell").join("master");
    std::fs::create_dir_all(&shell).expect("shell/master を作る");
    std::fs::write(
        shell.join("descript.txt"),
        "charset,UTF-8\r\nchar2.bindgroup101.name,服,B\r\n",
    )
    .expect("シェルの descript.txt を書く");
    let model = areka_parsers::package::resolve_with_shell(
        work.path(),
        areka_parsers::charset::DefaultEncoding::Utf8,
        None,
    )
    .expect("検体のシェルは解決できる");
    BindResolver::from_groups(&model.bindgroups)
}

/// ⑷ スコープ 2 は自分（`char2`）の名前表で引く: 名前の在る着た服 B は着る、`char2` に名前の無い
/// 脱いだ服 A は落とす（本体側の名前表は引かない・要件 14.2）。
#[test]
fn decide_binds_looks_up_a_char_scope_in_its_own_names() {
    let decided = decide_binds(2, &set(&[101]), &set(&[100]), &set(&[100]), &char2_names());
    assert_eq!(decided.replays, vec![wear("服", "B", true)]);
    assert_eq!(decided.dropped, vec![100]);
    // 本体側の名前表だけでは、スコープ 2 の名前は引けない（名前空間が分かれている）。
    let sakura_only = decide_binds(2, &set(&[101]), &set(&[100]), &set(&[100]), &names(&[]));
    assert!(sakura_only.replays.is_empty());
    assert_eq!(sakura_only.dropped, vec![101, 100]);
}

/// ⑷ 台本で触っていない既定を descript で書き換えた（帽子 C の既定 1→0・服 B の既定 0→1）なら、
/// 読み直したシェルの既定がそのまま効き、合図は 1 つも送らない（読み直しで書き換えが見える）。
///
/// # 非空虚性
/// 新しい既定との差を取ると、1→0 で帽子 C を着る・0→1 で服 B を脱ぐ合図が出て赤。
#[test]
fn decide_binds_lets_an_edited_default_take_effect() {
    // 1→0: 前の既定 {100, 200}・控えも {100, 200}（触っていない）・新しい既定 {100}。
    let off = decide_binds(
        0,
        &set(&[100, 200]),
        &set(&[100, 200]),
        &set(&[100]),
        &names(&[]),
    );
    assert_eq!(off, CarriedBinds::default());
    // 0→1: 前の既定 {100}・控えも {100}・新しい既定 {100, 101}（複数可の服）。
    let on = decide_binds(
        0,
        &set(&[100]),
        &set(&[100]),
        &set(&[100, 101]),
        &names(&["服"]),
    );
    assert_eq!(on, CarriedBinds::default());
}

/// ⑷ 台本で着た・脱いだ分は、既定が書き換わっても持ち越す: 前の既定 {100, 200} から帽子 C を脱ぎ服 B を
/// 着た控え {101}（服は排他なので A も外れた）を、既定 {100, 200} のままのシェルにも既定 {200} に
/// 書き換えたシェルにも出し直す。
#[test]
fn decide_binds_carries_a_dynamic_wear_and_shed_over_edited_defaults() {
    let old = set(&[100, 200]);
    let same = decide_binds(0, &set(&[101]), &old, &set(&[100, 200]), &names(&[]));
    assert_eq!(
        same.replays,
        vec![wear("服", "B", true), wear("帽子", "C", false)]
    );
    // 新しい既定 {200}: 服 A はもう既定でないので脱ぐ合図は要らない・服 B を着て帽子 C を脱ぐ。
    let edited = decide_binds(0, &set(&[101]), &old, &set(&[200]), &names(&[]));
    assert_eq!(
        edited.replays,
        vec![wear("服", "B", true), wear("帽子", "C", false)]
    );
    assert!(edited.dropped.is_empty());
}

/// ⑸ 着せ替えごとに `\![bind,…]`、決めた面ごとに `\s[面]` と同じ合図が着せ替えから先に届き、seriko の
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
        names(&[]),
        SerikoLoopConfig::disabled(),
        out,
    );
    let mut world = World::new();
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
                },
            ),
            (2, CarriedShow::Skipped { missing: 3000 }),
        ],
        &[(
            0,
            CarriedBinds {
                replays: vec![wear("服", "B", true)],
                dropped: vec![999],
            },
        )],
    );
    // 置き場のゴーストが持つ送り手を落とすとアクターの受信端が切れて終わる。
    drop(world);
    seriko.join().unwrap();

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

/// 置き場にゴーストが居なくても panic せず、何も送らない（`warn!` で残す）。
#[test]
fn replay_without_a_ghost_does_not_panic() {
    let world = World::new();
    replay(&world, &[(0, CarriedShow::Carried(0))], &[]);
    replay(&world, &[], &[]);
}

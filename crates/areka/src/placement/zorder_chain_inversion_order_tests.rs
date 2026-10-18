//! ぱすたと同じ指定 `\![reset,zorder]\![set,zorder,s0,b0,s1]` から、**台帳 → 合成（areka）→
//! 受け口 → 確立系・維持系・鎖の適用系（wintf・確定段の並び）→ 実窓の重なり**を 1 本に
//! 繋いで測るテスト（`areka-P0-zorder-scope-inversion` 要件 1.2・3.3・4.5・6.1・6.2・6.4・8.4）。
//!
//! 兄弟の wintf 側の実窓の檻（`zorder_chain_order_inversion_tests.rs`）は計画を手で組む。
//! 本ファイルは計画を**本番の台帳と合成**に組ませる——補い（`s0,b0,s1` → `s0,b0,b1,s1`）と
//! 繋ぎの導出が抜ければ、実窓の並びが崩れて赤くなる。
//!
//! 公開は指令消化の相（`zorder_drain.rs` の `publish_chain_plan`）と同じ規則で行う——
//! 合成の結果が受け口の内容と違うときだけ置き、印を立てる。相そのものは `frame` の
//! 非公開のモジュールなので、窓の在庫は本ファイルの「在る窓」の表で代わりに引く。
//!
//! 窓の添字は `[b0, s0, b1, s1]`。期待する最終形は手前から `s0, b0, b1, s1`、所有の鎖は
//! `s0 ← b0 ← b1 ← s1`（`b0` の組の結び目は外れ、`b1` の組の結び目が残る）。

use bevy_ecs::prelude::*;
use bevy_ecs::schedule::{Schedule, SingleThreadedExecutor};
use windows::Win32::Foundation::{HINSTANCE, HWND};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DestroyWindow, GW_HWNDNEXT, GW_OWNER, GetTopWindow, GetWindow, IsWindow,
    SW_SHOWNOACTIVATE, ShowWindow, WINDOW_EX_STYLE, WINDOW_STYLE, WS_EX_TOOLWINDOW, WS_POPUP,
};
use windows::core::w;
use wintf::ecs::window::{PairReleased, ZOrderChainPlan, apply_zorder_chain};
use wintf::ecs::{
    KeepDirectlyAbove, WindowHandle, apply_zorder_pair_maintenance, establish_owner_links,
    flush_window_pos_commands,
};

use super::compose_chain;
use crate::placement::zorder_group_ledger::{
    GroupElement, GroupWindowKind, ZOrderGroupLedger, parse_zorder_tokens,
};

const B0: usize = 0;
const S0: usize = 1;
const B1: usize = 2;
const S1: usize = 3;

/// 期待する最終形（手前から）。
const LANDED: [usize; 4] = [S0, B0, B1, S1];
/// 期待する所有関係（添字ごとの持ち主）: `s0 ← b0 ← b1 ← s1`。
const LANDED_OWNERS: [Option<usize>; 4] = [Some(B1), Some(B0), Some(S1), None];

fn element_at(index: usize) -> GroupElement {
    GroupElement {
        scope: (index / 2) as u32,
        kind: if index % 2 == 0 {
            GroupWindowKind::Balloon
        } else {
            GroupWindowKind::Char
        },
    }
}

// ---------------------------------------------------------------------------
// 実窓（0x0・可視・本番のゴースト窓と同じ拡張スタイル）と走査
// ---------------------------------------------------------------------------

fn create_raw_window() -> HWND {
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    // SAFETY: Win32 境界。自プロセス所有の 0x0 トップレベル窓を作り、活性化せずに表示する。
    unsafe {
        let hinstance = GetModuleHandleW(None).expect("GetModuleHandleW");
        let hwnd = CreateWindowExW(
            WINDOW_EX_STYLE(WS_EX_TOOLWINDOW.0),
            w!("Static"),
            w!("zorder-chain-inversion-areka"),
            WINDOW_STYLE(WS_POPUP.0),
            10,
            10,
            0,
            0,
            None,
            None,
            Some(hinstance.into()),
            None,
        )
        .expect("CreateWindowExW should create a test window");
        let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
        hwnd
    }
}

thread_local! {
    static IME_ANCHOR_MADE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// 鎖の窓を作る。スレッド既定の不可視 IME 窓が鎖の窓に付かないよう、受け皿を先に 1 枚作る
/// （兄弟 `zorder_chain_tail_order_tests.rs` の `ensure_ime_anchor` と同じ理由）。
fn create_window() -> HWND {
    IME_ANCHOR_MADE.with(|made| {
        if !made.replace(true) {
            let _ = create_raw_window();
        }
    });
    create_raw_window()
}

fn is_window(hwnd: HWND) -> bool {
    // SAFETY: Win32 境界。読み取りのみ。
    unsafe { IsWindow(Some(hwnd)) }.as_bool()
}

/// 前面から `GW_HWNDNEXT` で降り、`set` の生きている窓の添字だけを手前から並べる。
/// 他プロセスの窓が走査の途中で消えると切れるので、全部拾えるまで有界回数やり直す。
fn z_shape(set: &[Option<HWND>]) -> Vec<usize> {
    let wanted = set.iter().flatten().count();
    let mut best = Vec::new();
    for _ in 0..8 {
        let mut seen = Vec::new();
        // SAFETY: Win32 境界。読み取り専用の走査。
        let mut cursor = unsafe { GetTopWindow(None) }.ok();
        while let Some(hwnd) = cursor {
            if let Some(i) = set.iter().position(|w| *w == Some(hwnd)) {
                seen.push(i);
                if seen.len() == wanted {
                    return seen;
                }
            }
            // SAFETY: Win32 境界。読み取り専用の走査。
            cursor = unsafe { GetWindow(hwnd, GW_HWNDNEXT) }.ok();
        }
        if seen.len() > best.len() {
            best = seen;
        }
    }
    best
}

/// 添字ごとの持ち主（`GetWindow(GW_OWNER)`・`set` の外の持ち主と窓の無い枠は `None`）。
fn owner_shape(set: &[Option<HWND>]) -> Vec<Option<usize>> {
    set.iter()
        .map(|h| {
            // SAFETY: Win32 境界。読み取りのみ。
            let owner = h.and_then(|h| unsafe { GetWindow(h, GW_OWNER) }.ok())?;
            set.iter().position(|w| *w == Some(owner))
        })
        .collect()
}

// ---------------------------------------------------------------------------
// 足場: 窓 4 枚・台帳・本番の確定段の 3 本
// ---------------------------------------------------------------------------

struct Rig {
    world: World,
    schedule: Schedule,
    ledger: ZOrderGroupLedger,
    hwnds: [Option<HWND>; 4],
    entities: [Option<Entity>; 4],
    /// 壊した窓も含め、作った窓のすべて（後始末用）。
    made: Vec<HWND>,
}

impl Rig {
    /// `[b0, s0, b1, s1]` の実体を作る。`without_window` の添字は実体だけで窓（ハンドル）が無い。
    fn new(without_window: &[usize]) -> Self {
        let mut schedule = Schedule::default();
        schedule.set_executor(SingleThreadedExecutor::new());
        // 確定段と同じ並び（`spawn.rs` の `wire_zorder_pair`）。
        schedule.add_systems(
            (
                establish_owner_links,
                apply_zorder_pair_maintenance,
                apply_zorder_chain,
            )
                .chain(),
        );
        let mut rig = Self {
            world: World::new(),
            schedule,
            ledger: ZOrderGroupLedger::default(),
            hwnds: [None; 4],
            entities: [None; 4],
            made: Vec::new(),
        };
        for character in [S0, S1] {
            rig.spawn(character, !without_window.contains(&character));
            rig.spawn(character - 1, !without_window.contains(&(character - 1)));
        }
        rig
    }

    /// 添字 `index` の窓の実体を作る（バルーンなら相棒のキャラクターへの組の宣言を付ける）。
    fn spawn(&mut self, index: usize, with_window: bool) {
        let mut entity = self.world.spawn_empty();
        if index % 2 == 0 {
            let peer = self.entities[index + 1].expect("キャラクターを先に作る");
            entity.insert(KeepDirectlyAbove { peer });
        }
        self.entities[index] = Some(entity.id());
        if with_window {
            self.attach_window(index);
        }
    }

    /// 添字 `index` の実体へ窓を作って付ける（窓が後から現れる）。
    fn attach_window(&mut self, index: usize) {
        let hwnd = create_window();
        self.made.push(hwnd);
        self.hwnds[index] = Some(hwnd);
        let entity = self.entities[index].expect("実体が在る");
        self.world.entity_mut(entity).insert(WindowHandle {
            hwnd,
            instance: HINSTANCE::default(),
        });
    }

    /// 実体を落とす（窓はまだ壊さない——去る窓の切離しの巡を挟んでから壊す）。
    fn despawn(&mut self, index: usize) -> HWND {
        let entity = self.entities[index].take().expect("実体が在る");
        self.world.despawn(entity);
        self.hwnds[index].take().expect("窓が在る")
    }

    /// 台本の指令を台帳へ当てる（`\![reset,zorder]\![set,zorder,s0,b0,s1]`）。
    fn apply_pasta_directive(&mut self) {
        self.ledger.reset_to_descript();
        let (members, _) =
            parse_zorder_tokens(&["s0", "b0", "s1"]).expect("明示の記法は受理される");
        self.ledger
            .try_add_tag_group(members)
            .expect("最初の指定は受理される");
    }

    /// 合成して、内容が違うときだけ受け口へ置く（`zorder_drain.rs` の公開と同じ規則）。
    fn publish(&mut self) {
        let world = &self.world;
        let entities = self.entities;
        let plan = compose_chain(self.ledger.groups(), &[0, 1], &|element| {
            let index = (0..4).find(|i| element_at(*i) == *element)?;
            entities[index].filter(|e| world.get_entity(*e).is_ok())
        });
        if self.world.get_resource::<ZOrderChainPlan>().is_none() {
            if plan.is_none() {
                return;
            }
            self.world.init_resource::<ZOrderChainPlan>();
        }
        let mut receiver = self.world.resource_mut::<ZOrderChainPlan>();
        if receiver.chain != plan {
            receiver.chain = plan;
            receiver.dirty = true;
        }
    }

    /// 確定段の 1 巡と、そのあとの窓の指令の書き出し（本番は tick の後に流す）。
    fn run(&mut self) {
        self.schedule.run(&mut self.world);
        flush_window_pos_commands();
    }

    fn released_marks(&mut self) -> usize {
        self.world
            .query_filtered::<Entity, With<PairReleased>>()
            .iter(&self.world)
            .count()
    }

    /// 組の重なりから指令を当てて 1 巡回し、前 3 枚の重なりが着いたことを確かめる。
    fn land_pasta_directive(&mut self) {
        self.run();
        let start = z_shape(&self.hwnds);
        let start_owners = owner_shape(&self.hwnds);
        self.apply_pasta_directive();
        self.publish();
        self.run();
        assert_eq!(
            start_owners,
            vec![Some(S0), None, Some(S1), None],
            "始点で組が張られていない（以下の比較の前提）"
        );
        assert_ne!(start, LANDED.to_vec(), "始点が既に期待の並び（比較が空虚）");
        self.assert_landed("指令の後");
    }

    fn assert_landed(&self, when: &str) {
        assert_eq!(
            owner_shape(&self.hwnds),
            LANDED_OWNERS.to_vec(),
            "{when}: 所有の鎖が s0 ← b0 ← b1 ← s1 の一直線でない"
        );
        assert_eq!(
            z_shape(&self.hwnds),
            LANDED.to_vec(),
            "{when}: 実窓の並びが手前から s0, b0, b1, s1 でない（要件 1.2・3.3）"
        );
    }
}

impl Drop for Rig {
    fn drop(&mut self) {
        // 手前（所有される側）から壊せば道連れは起きない。壊れていればそのまま飛ばす。
        let order = z_shape(&self.hwnds);
        for hwnd in order
            .iter()
            .filter_map(|i| self.hwnds[*i])
            .chain(self.made.iter().copied())
        {
            if is_window(hwnd) {
                // SAFETY: Win32 境界。このテストが作った窓を壊す。
                unsafe {
                    let _ = DestroyWindow(hwnd);
                }
            }
        }
    }
}

// ===========================================================================
// テスト
// ===========================================================================

/// ぱすたと同じ指定から、台帳が `b1` を補い、合成と確定段の 3 本が前 3 枚を成り立たせる
/// （要件 1.2・3.3・8.4 の前段）。続く解除で各スコープが組へ戻る（要件 4.5）。
#[test]
fn the_pasta_directive_lands_s0_b0_b1_s1_on_real_windows() {
    let mut rig = Rig::new(&[]);
    rig.land_pasta_directive();
    assert_eq!(
        rig.ledger.groups()[0].members,
        LANDED.iter().map(|i| element_at(*i)).collect::<Vec<_>>(),
        "台帳が s0,b0,s1 を s0,b0,b1,s1 へ補っていない（要件 3.3）"
    );
    assert_eq!(rig.released_marks(), 1, "外した組は b0 の 1 つだけのはず");

    rig.ledger.reset_to_descript();
    rig.publish();
    rig.run();
    let reset = z_shape(&rig.hwnds);
    assert_eq!(
        owner_shape(&rig.hwnds),
        vec![Some(S0), None, Some(S1), None],
        "解除で組の結び目が張り直されていない（要件 4.5）"
    );
    for (b, s) in [(B0, S0), (B1, S1)] {
        let at = |i| reset.iter().position(|v| *v == i).expect("4 枚が列に居る");
        assert_eq!(
            at(s) as isize - at(b) as isize,
            1,
            "解除の後、バルーンがキャラクターのすぐ手前に無い: {reset:?}"
        );
    }
    assert_eq!(rig.released_marks(), 0, "解除の後も組を外した印が残る");
}

/// 指定の後に初めて `b0` の窓が現れても、組を張らずに書かれた位置へ入る（要件 6.1）。
#[test]
fn a_balloon_that_appears_after_the_directive_joins_in_written_order() {
    let mut rig = Rig::new(&[B0]);
    rig.run();
    rig.apply_pasta_directive();
    rig.publish();
    rig.run();
    // 刺激の前: b0 に窓が無いので b0 を挟む繋ぎはまだ書けない（持ち越し）。
    assert_eq!(
        owner_shape(&rig.hwnds),
        vec![None, None, Some(S1), None],
        "b0 の窓が無いのに b0 を挟む繋ぎが書かれた（比較の前提）"
    );

    rig.attach_window(B0);
    rig.run();
    rig.assert_landed("b0 の窓が現れた後");
    assert_eq!(rig.released_marks(), 1, "b0 の組は外したままのはず");
}

/// `b0` の窓が破棄され、後に再び現れても書かれた位置へ戻る（要件 6.2）。破棄の間も
/// 残る窓は書かれた順を保ち、壊した窓が他の窓を道連れにしない。
#[test]
fn a_balloon_destroyed_and_recreated_returns_to_its_written_place() {
    let mut rig = Rig::new(&[]);
    rig.land_pasta_directive();

    let gone = rig.despawn(B0);
    rig.publish();
    rig.run();
    // SAFETY: Win32 境界。このテストが作った窓を壊す。
    let destroyed = unsafe { DestroyWindow(gone) }.is_ok();
    assert!(
        destroyed && !is_window(gone),
        "b0 の窓が壊れていない（刺激が届いていない）"
    );
    assert!(
        [S0, B1, S1]
            .iter()
            .all(|i| is_window(rig.hwnds[*i].unwrap())),
        "b0 を壊したら他の窓まで道連れになった"
    );
    assert_eq!(
        z_shape(&rig.hwnds),
        vec![S0, B1, S1],
        "b0 が居ない間の並びが書かれた順でない"
    );

    rig.spawn(B0, true);
    rig.publish();
    rig.run();
    rig.assert_landed("b0 の窓が再び現れた後");
}

/// キャラクター `s0` の窓が破棄されても、残りは書かれた順のまま繋ぎ直され、他のスコープの
/// 窓は道連れにならない。相手の居ない組は印だけを外す（要件 6.4）。
#[test]
fn a_destroyed_character_leaves_the_rest_in_written_order() {
    let mut rig = Rig::new(&[]);
    rig.land_pasta_directive();

    let gone = rig.despawn(S0);
    rig.publish();
    rig.run();
    // SAFETY: Win32 境界。このテストが作った窓を壊す。
    let destroyed = unsafe { DestroyWindow(gone) }.is_ok();
    assert!(
        destroyed && !is_window(gone),
        "s0 の窓が壊れていない（刺激が届いていない）"
    );
    assert!(
        [B0, B1, S1]
            .iter()
            .all(|i| is_window(rig.hwnds[*i].unwrap())),
        "s0 を壊したら他の窓まで道連れになった"
    );
    assert_eq!(
        owner_shape(&rig.hwnds),
        vec![Some(B1), None, Some(S1), None],
        "s0 が去った後の所有の鎖が b0 ← b1 ← s1 でない"
    );
    assert_eq!(
        z_shape(&rig.hwnds),
        vec![B0, B1, S1],
        "s0 が去った後の並びが書かれた順でない"
    );
    assert_eq!(rig.released_marks(), 0, "相手の居ない組の印が残る");
}

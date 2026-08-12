//! 複数のペアが**同じ巡で**是正を要求したときに、維持系
//! [`apply_zorder_pair_maintenance`](super::apply_zorder_pair_maintenance) が
//! 両方とも隣接させられることを固定するヘッドレステスト（実機・実ディスプレイ不要）。
//!
//! # 何を守るためのファイルか
//!
//! 維持系は実測（`GetWindow`）を採ってから挿入位置を決めるが、実際の書込はその巡の
//! **後**に走る flush である。よって 1 巡のうちに 2 つのペアが是正の指令を積むと、
//! 後から適用される指令の挿入位置は、先に適用された指令によって既に陳腐化している
//! ——指した窓の隣に別の窓が割り込み、「バルーンはキャラのすぐ手前」（要件 1.1）が
//! 成立しない。実機の 2 スコープ起動で毎回 `verify-failed` が出た欠陥がこれである。
//!
//! 本ファイルはその形を実窓で再現し、
//!
//! - **どのペアも最終的に隣接すること**（有界な巡数で収束すること）
//! - **1 巡で実際に動かす窓は高々 1 ペア分であること**（陳腐化した実測を使わない構造）
//!
//! の 2 つを固定する。前者だけでは「巡ごとに全ペアを動かし続けてたまたま揃った」形も
//! 緑になりうるため、後者を別のテストで併せて押さえる。
//!
//! # なぜ親窓の下の子窓 4 枚で測るのか
//!
//! 兄弟列をテスト専用の親窓の中に閉じるためである（理由は
//! [`zorder_pair_maintain_tests`](super::tests) の module doc と同じ——トップレベル窓の
//! 重なり隣接は同一テストバイナリ内で決定論にならない）。**子窓の初期配置は生成順に
//! 依存させず、必ず `insert_after` で明示的に作る**。
//!
//! # 「出ないこと」を主張するときの作法
//!
//! 記録が出ないことを見る主張には、同じ捕捉窓の中に確実に拾える対照
//! （別の親窓に置いた既に隣接済みのペア＝見送りが 1 本出る側）を併置し、
//! 「その種の行がちょうど N 本」と**その行が指す entity** まで検査する。
//! シングルスレッド実行器を明示する理由は他の維持系テストと同じである。

use bevy_ecs::prelude::*;
use bevy_ecs::schedule::{ExecutorKind, Schedule};
use windows::Win32::Foundation::{HINSTANCE, HWND};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DestroyWindow, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SetWindowPos,
    WINDOW_EX_STYLE, WINDOW_STYLE, WS_CHILD, WS_POPUP,
};
use windows::core::{PCWSTR, w};

use super::{IssuedPairFix, apply_zorder_pair_maintenance};
use crate::api::{get_window_above, get_window_below};
use crate::ecs::test_support::capture_under_filter;
use crate::ecs::window::{
    KeepDirectlyAbove, ReassertZOrder, SetWindowPosCommand, WindowHandle, ZOrderPairStrategy,
};

/// 実機サインオフの `RUST_LOG` に、指令の実施記録を出す `command` モジュールを足したもの
/// （[`zorder_pair_maintain_tests`](super::tests) と同じ理由・同じ値）。
const CAPTURE_DIRECTIVES: &str =
    "info,wintf::ecs::window::zorder_pair=debug,wintf::ecs::window::command=debug";

const FIX_TAG: &str = "[zorder-pair] fix";
const SKIP_TAG: &str = "[zorder-pair] skip";
const VERIFY_FAILED_TAG: &str = "[zorder-pair] verify-failed";
/// `guarded_set_window_pos` の実施記録（＝実際に発行された指令 1 本）。
const SET_WINDOW_POS_TAG: &str = "[guarded_set_window_pos] Calling SetWindowPos";

// -------------------------------------------------------------------------
// 実窓・World・巡回のヘルパ
// -------------------------------------------------------------------------

fn create_window(title: PCWSTR, style: WINDOW_STYLE, parent: Option<HWND>) -> HWND {
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    // SAFETY: Win32 境界。定義済 "Static" クラスで非表示ウィンドウを生成する。
    unsafe {
        let hinstance = GetModuleHandleW(None).expect("GetModuleHandleW");
        CreateWindowExW(
            WINDOW_EX_STYLE(0),
            w!("Static"),
            title,
            style,
            0,
            0,
            0,
            0,
            parent,
            None,
            Some(hinstance.into()),
            None,
        )
        .expect("CreateWindowExW should create a hidden test window")
    }
}

/// テスト専用の親窓（兄弟列を閉じるための入れ物）。
fn create_test_parent(title: PCWSTR) -> HWND {
    create_window(title, WINDOW_STYLE(WS_POPUP.0), None)
}

/// 親窓の子として非表示の実窓を生成する（初期配置は [`insert_after`] で明示的に作ること）。
fn create_test_child(parent: HWND, title: PCWSTR) -> HWND {
    create_window(title, WINDOW_STYLE(WS_CHILD.0), Some(parent))
}

/// `hwnd` を `after` のすぐ背後（1 つ背面側）へ移す。
fn insert_after(hwnd: HWND, after: HWND) {
    // SAFETY: Win32 境界。自プロセス所有の実 HWND に対する重なりのみの変更。
    unsafe {
        SetWindowPos(
            hwnd,
            Some(after),
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
        )
        .expect("SetWindowPos should reorder self-owned test windows");
    }
}

fn destroy_test_window(hwnd: HWND) {
    // SAFETY: Win32 境界。生成した所有ウィンドウを破棄する（子窓は親の破棄で連鎖する）。
    unsafe {
        let _ = DestroyWindow(hwnd);
    }
}

fn spawn_window(world: &mut World, hwnd: HWND) -> Entity {
    world
        .spawn(WindowHandle {
            hwnd,
            instance: HINSTANCE::default(),
        })
        .id()
}

fn declare_pair(world: &mut World, balloon: Entity, character: Entity) {
    world
        .entity_mut(balloon)
        .insert(KeepDirectlyAbove { peer: character });
}

/// 一回限りの再断行要求を外から挿入する（要件 2.6 のシームと同じ形＝未適用の既定値）。
fn request_reassert(world: &mut World, balloon: Entity) {
    world.entity_mut(balloon).insert(ReassertZOrder::default());
}

/// 維持系だけを載せた 1 巡分の schedule。
fn maintain_schedule() -> Schedule {
    let mut schedule = Schedule::default();
    schedule.set_executor_kind(ExecutorKind::SingleThreaded);
    schedule.add_systems(apply_zorder_pair_maintenance);
    schedule
}

fn lines_with<'a>(out: &'a str, tag: &str) -> Vec<&'a str> {
    out.lines().filter(|l| l.contains(tag)).collect()
}

fn assert_line_count(out: &str, tag: &str, expected: usize, what: &str) {
    let found = lines_with(out, tag);
    assert_eq!(
        found.len(),
        expected,
        "{what}: `{tag}` の行は {expected} 本のはずが {} 本だった\n---捕捉全文---\n{out}",
        found.len()
    );
}

/// 指定タグの行が、指定 entity をちょうど 1 本ずつ指していることを確かめる。
///
/// 複数ペアを同時に扱うため、本数だけでは「同じペアの記録が 2 本出て、もう一方は 0 本」
/// を見逃す。行の `entity=` まで突き合わせる。
fn assert_lines_cover_entities(out: &str, tag: &str, entities: &[Entity], what: &str) {
    assert_line_count(out, tag, entities.len(), what);
    for entity in entities {
        let needle = format!("entity={entity:?}");
        let found = lines_with(out, tag)
            .into_iter()
            .filter(|l| l.contains(&needle))
            .count();
        assert_eq!(
            found, 1,
            "{what}: `{tag}` で {needle} を指す行は 1 本のはずが {found} 本だった\
             \n---捕捉全文---\n{out}"
        );
    }
}

/// 2 つのペア（4 枚の子窓）を、**どちらのバルーンも自分のキャラの隣に居ない**初期配置で
/// 用意する。
///
/// 手前 → 背後の順で `balloon0 / character1 / character0 / balloon1`。これは実機の
/// 2 スコープ起動直後（バルーンが両方ともキャラから離れている）を写したものであり、
/// 両ペアが同じ巡で是正を要求する。しかも**片方の是正の挿入位置がもう片方の是正で
/// 陳腐化する**——scope 0 は「character1 の背後へ」、scope 1 は「balloon0 の背後へ」を
/// 指すため、同じ巡でそのまま両方適用すると割り込みが起きる。
struct TwoPairs {
    parent: HWND,
    character0: HWND,
    balloon0: HWND,
    character1: HWND,
    balloon1: HWND,
}

impl TwoPairs {
    fn create(tag: PCWSTR) -> Self {
        let parent = create_test_parent(tag);
        let character0 = create_test_child(parent, w!("character0"));
        let balloon0 = create_test_child(parent, w!("balloon0"));
        let character1 = create_test_child(parent, w!("character1"));
        let balloon1 = create_test_child(parent, w!("balloon1"));

        // 生成順に依存させず、手前 → 背後を明示的に作る。
        insert_after(character1, balloon0);
        insert_after(character0, character1);
        insert_after(balloon1, character0);

        let pairs = Self {
            parent,
            character0,
            balloon0,
            character1,
            balloon1,
        };
        pairs.assert_initial_order();
        pairs
    }

    fn assert_initial_order(&self) {
        let below = |hwnd| get_window_below(hwnd).expect("GetWindow は成功するはず");
        assert_eq!(
            get_window_above(self.balloon0).expect("GetWindow は成功するはず"),
            None,
            "前提: 最も手前は balloon0"
        );
        assert_eq!(below(self.balloon0), Some(self.character1), "前提の並び");
        assert_eq!(below(self.character1), Some(self.character0), "前提の並び");
        assert_eq!(below(self.character0), Some(self.balloon1), "前提の並び");
        assert_eq!(below(self.balloon1), None, "前提: 最も背後は balloon1");
    }

    /// 両ペアが「バルーンはキャラのすぐ手前」になっていることを OS 側の実測で確かめる。
    fn assert_both_adjacent(&self) {
        assert_eq!(
            get_window_below(self.balloon0).expect("GetWindow は成功するはず"),
            Some(self.character0),
            "scope 0: バルーンの 1 つ背後は自分のキャラ窓であるはず"
        );
        assert_eq!(
            get_window_below(self.balloon1).expect("GetWindow は成功するはず"),
            Some(self.character1),
            "scope 1: バルーンの 1 つ背後は自分のキャラ窓であるはず"
        );
    }

    fn destroy(self) {
        destroy_test_window(self.parent);
    }
}

/// 既に隣接しているペアを別の親窓の下に 1 組作る（記録側の対照）。
///
/// 別の親窓に置くのは、対照の 2 枚が検査対象の兄弟列へ割り込まないようにするためである。
struct AdjacentControl {
    parent: HWND,
    character: HWND,
    balloon: HWND,
}

impl AdjacentControl {
    fn create(tag: PCWSTR) -> Self {
        let parent = create_test_parent(tag);
        let character = create_test_child(parent, w!("control-character"));
        let balloon = create_test_child(parent, w!("control-balloon"));
        insert_after(character, balloon);
        assert_eq!(
            get_window_below(balloon).expect("GetWindow は成功するはず"),
            Some(character),
            "前提: 対照のペアは既に隣接している（＝理由つきの見送りが 1 本出る）"
        );
        Self {
            parent,
            character,
            balloon,
        }
    }

    fn destroy(self) {
        destroy_test_window(self.parent);
    }
}

// -------------------------------------------------------------------------
// 収束（複数ペアが同じ巡で要求しても、どちらも隣接する）
// -------------------------------------------------------------------------

/// 2 つのペアが同じ巡で再断行を要求しても、有界な巡数でどちらも隣接する。
///
/// これが欠陥の再現テストである。1 巡のうちに両方の指令を積むと、後から適用される指令の
/// 挿入位置が先の適用で陳腐化し、`balloon0` と `character0` の間へ `balloon1` が割り込む
/// ——次巡の検証は `verify-failed`（error）になり、要件 1.1 が 2 スコープで成立しない。
#[test]
fn two_pairs_requesting_in_the_same_pass_both_become_adjacent() {
    let pairs = TwoPairs::create(w!("wintf-zorder-multipair-converge"));

    let mut world = World::new();
    world.insert_resource(ZOrderPairStrategy::default());
    let character0 = spawn_window(&mut world, pairs.character0);
    let balloon0 = spawn_window(&mut world, pairs.balloon0);
    let character1 = spawn_window(&mut world, pairs.character1);
    let balloon1 = spawn_window(&mut world, pairs.balloon1);
    declare_pair(&mut world, balloon0, character0);
    declare_pair(&mut world, balloon1, character1);
    request_reassert(&mut world, balloon0);
    request_reassert(&mut world, balloon1);

    let mut schedule = maintain_schedule();
    let out = capture_under_filter(CAPTURE_DIRECTIVES, || {
        // 本番と同じ形（1 巡ごとに tick 後の flush）を有界回数だけ回す。
        for _ in 0..4 {
            schedule.run(&mut world);
            SetWindowPosCommand::flush();
        }
    });

    pairs.assert_both_adjacent();

    assert_line_count(
        &out,
        VERIFY_FAILED_TAG,
        0,
        "どのペアの検証も期待どおりに終わるはず（同じ巡の別ペアに挿入位置を陳腐化されない）",
    );
    assert_lines_cover_entities(
        &out,
        FIX_TAG,
        &[balloon0, balloon1],
        "是正の記録は各ペア 1 本ずつ",
    );
    assert_line_count(&out, SKIP_TAG, 0, "見送る理由は無い");
    assert_line_count(
        &out,
        SET_WINDOW_POS_TAG,
        2,
        "発行された指令は 1 ペアにつき 1 本ずつ（同じペアを二度動かさない）",
    );

    assert!(
        !world.entity(balloon0).contains::<ReassertZOrder>()
            && !world.entity(balloon1).contains::<ReassertZOrder>(),
        "収束後はどちらの要求も消えているはず"
    );
    assert!(
        !world.entity(balloon0).contains::<IssuedPairFix>()
            && !world.entity(balloon1).contains::<IssuedPairFix>(),
        "指令の持ち越しも要求とともに消えるはず"
    );

    pairs.destroy();
}

// -------------------------------------------------------------------------
// 1 巡で動かすのは高々 1 ペア（陳腐化した実測を使わない構造）
// -------------------------------------------------------------------------

/// 是正を要するペアが 2 つあっても、1 巡で実際に発行する指令は 1 本だけであり、
/// 残るペアの要求は**消えずに次巡へ持ち越される**。
///
/// 同じ捕捉窓に、既に隣接している対照のペア（別の親窓）を置く——見送りの記録が
/// ちょうど 1 本、対照の entity を指して出ることで、捕捉が生きていることと、
/// 持ち越しが「黙って握り潰された見送り」ではないことが同時に示される。
#[test]
fn only_one_pair_is_corrected_per_pass_and_the_other_request_survives() {
    let pairs = TwoPairs::create(w!("wintf-zorder-multipair-one-per-pass"));
    let control = AdjacentControl::create(w!("wintf-zorder-multipair-one-per-pass-control"));

    let mut world = World::new();
    world.insert_resource(ZOrderPairStrategy::default());
    let character0 = spawn_window(&mut world, pairs.character0);
    let balloon0 = spawn_window(&mut world, pairs.balloon0);
    let character1 = spawn_window(&mut world, pairs.character1);
    let balloon1 = spawn_window(&mut world, pairs.balloon1);
    declare_pair(&mut world, balloon0, character0);
    declare_pair(&mut world, balloon1, character1);
    request_reassert(&mut world, balloon0);
    request_reassert(&mut world, balloon1);

    // 対照（確かに記録が出る側・既に隣接しているので理由つきの見送りになる）
    let control_character = spawn_window(&mut world, control.character);
    let control_balloon = spawn_window(&mut world, control.balloon);
    declare_pair(&mut world, control_balloon, control_character);
    request_reassert(&mut world, control_balloon);

    let mut schedule = maintain_schedule();
    let out = capture_under_filter(CAPTURE_DIRECTIVES, || {
        schedule.run(&mut world);
        SetWindowPosCommand::flush();
    });

    assert_line_count(
        &out,
        SET_WINDOW_POS_TAG,
        1,
        "1 巡で実際に動かす窓は高々 1 ペア分（対照は既に隣接しているので指令を出さない）",
    );
    assert_lines_cover_entities(
        &out,
        SKIP_TAG,
        &[control_balloon],
        "見送りの記録は既に隣接している対照のペアだけ（持ち越しは見送りではない）",
    );
    assert!(
        lines_with(&out, SKIP_TAG)[0].contains("reason=AlreadyAdjacent"),
        "対照の見送りの理由は「既に隣接している」であるはず: {out}"
    );
    assert_line_count(&out, FIX_TAG, 0, "検証はまだ次巡（是正の記録は出ない）");
    assert_line_count(&out, VERIFY_FAILED_TAG, 0, "検証段階へ進んでいない");

    // 片方は適用されて検証待ちへ、もう片方は未適用のまま残る。
    let applied = [balloon0, balloon1]
        .into_iter()
        .filter(|e| {
            world
                .entity(*e)
                .get::<ReassertZOrder>()
                .is_some_and(|r| r.pending_verify.is_some())
        })
        .count();
    let deferred: Vec<Entity> = [balloon0, balloon1]
        .into_iter()
        .filter(|e| {
            world
                .entity(*e)
                .get::<ReassertZOrder>()
                .is_some_and(|r| r.pending_verify.is_none())
        })
        .collect();
    assert_eq!(applied, 1, "適用されたのはちょうど 1 ペア");
    assert_eq!(deferred.len(), 1, "もう 1 ペアの要求は消えずに残るはず");
    assert!(
        !world.entity(deferred[0]).contains::<IssuedPairFix>(),
        "持ち越されたペアは指令を出していないので、出した指令の写しも持たない"
    );
    assert!(
        !world.entity(control_balloon).contains::<ReassertZOrder>(),
        "対照の要求は見送りとともに消えるはず"
    );

    control.destroy();
    pairs.destroy();
}

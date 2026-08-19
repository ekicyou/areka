//! 案 A の owner 確立系 [`establish_owner_links`](super::establish_owner_links) に対する
//! ヘッドレステスト（実機・実ディスプレイ不要）。
//!
//! 本ファイルが固定するのは 3 つである。
//!
//! - **確立が 1 回だけ走ること**——両窓に OS のハンドルが揃った巡で owner が張られ、
//!   以後は何度巡っても二度目が走らないこと（要件 1.1・design.md「案 A の確立フロー」2）。
//! - **揃わない巡では走らず、見送りが理由つきで記録されること**——片方のハンドルが
//!   まだ無い／相手の実体が既に無い／当該ストラテジが owner を張らない構成である、の
//!   3 通り（要件 6.3）。
//! - **失敗しても同じハンドルで繰り返さないこと**——失敗は error 水準で 1 度だけ記録され、
//!   同じハンドルの組では再試行しない（要件 6.2・design.md「Error Strategy」の確立失敗）。
//!
//! # なぜ本物の窓を作るのか（採った道とその理由）
//!
//! 確立系は `set_window_owner`（`SetWindowLongPtrW(GWLP_HWNDPARENT)`）を実際に呼ぶ。
//! ここを偽の実装へ差し替えると「owner が本当に張られたか」は永久に検査されず、
//! 検査できるのは自前の帳簿だけになる。一方、**owner 関係そのものは自プロセスが所有する
//! 2 枚の窓の間で決定論的に成立する**——タスク 1.1 で非決定になったのは
//! 「トップレベル窓どうしの**重なり隣接**」であって、owner の設定と読み戻し
//! （`GetWindow(GW_OWNER)`）は他プロセスの窓に割り込まれない。よって本ファイルは
//! 自前の非表示トップレベル窓を実際に 2 枚作り、`GetWindow(GW_OWNER)` で
//! **OS 側の事実**を確かめる。
//!
//! 隣接順（`measured_prev`）は非決定ゆえ**値を検査しない**——記録に載ることだけが要件で
//! あり、その値の正しさは実機サインオフ（ゲート G6）の担当である。
//!
//! 失敗経路は無効ハンドル（null HWND）で決定論的に作れる（`api.rs` のラッパテストと同型）。
//!
//! # 記録捕捉で「出ないこと」を主張するときの作法
//!
//! 「この巡では確立が走らない」の主張は、捕捉そのものが死んでいても成立してしまう。
//! よって出ないことを見る各テストは、**同じ捕捉窓の中で確かに出る記録を併置**し、
//! さらに「その種の行がちょうど N 本」を検査する——拾えない側（0 本）でも
//! 余計に出る側（N+1 本）でも赤になる形にしてある。

use bevy_ecs::prelude::*;
use bevy_ecs::schedule::{Schedule, SingleThreadedExecutor};
use windows::Win32::Foundation::{HINSTANCE, HWND};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DestroyWindow, GW_OWNER, GetWindow, WINDOW_EX_STYLE, WINDOW_STYLE, WS_POPUP,
};
use windows::core::{PCWSTR, w};

use super::{OwnerEstablishFailed, establish_owner_links, measure_window_above};
use crate::ecs::test_support::capture_under_filter;
use crate::ecs::window::ZOrderPairStrategy;
use crate::ecs::window::{KeepDirectlyAbove, OwnerLink, ReassertZOrder, WindowHandle};

/// 実機サインオフが用いる `RUST_LOG` 相当（design.md「Real-Machine Signoff」の wintf 側）。
const SIGNOFF_DIRECTIVES: &str = "info,wintf::ecs::window::zorder_pair=debug";

const OWNER_ESTABLISHED_TAG: &str = "[zorder-pair] owner-established";
const OWNER_ESTABLISH_FAILED_TAG: &str = "[zorder-pair] owner-establish-failed";
const SKIP_TAG: &str = "[zorder-pair] skip";

// -------------------------------------------------------------------------
// 実窓・World・巡回のヘルパ
// -------------------------------------------------------------------------

/// 自プロセス所有の非表示トップレベル窓を作る（`api.rs` のテスト生成レシピと同一）。
///
/// owner 関係はトップレベル窓の間でのみ意味を持つため `WS_POPUP`（親なし）で作る。
fn create_test_window(title: PCWSTR) -> HWND {
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    // SAFETY: Win32 境界。定義済 "Static" クラスで非表示ウィンドウを生成する。
    unsafe {
        let hinstance = GetModuleHandleW(None).expect("GetModuleHandleW");
        CreateWindowExW(
            WINDOW_EX_STYLE(0),
            w!("Static"),
            title,
            WINDOW_STYLE(WS_POPUP.0),
            0,
            0,
            0,
            0,
            None,
            None,
            Some(hinstance.into()),
            None,
        )
        .expect("CreateWindowExW should create a hidden test window")
    }
}

/// 生成したテスト窓を破棄する（後始末）。
fn destroy_test_window(hwnd: HWND) {
    // SAFETY: Win32 境界。生成した所有ウィンドウを破棄する。
    unsafe {
        let _ = DestroyWindow(hwnd);
    }
}

/// OS 側が持つ owner を読み戻す（`GetWindow(GW_OWNER)`）。owner 無しは `None`。
fn owner_of(hwnd: HWND) -> Option<HWND> {
    // SAFETY: Win32 境界。読み取りのみ。
    unsafe { GetWindow(hwnd, GW_OWNER) }.ok()
}

/// Win32 へは渡さない偽ハンドル（値としてのみ扱う）。
fn fake_hwnd(v: usize) -> HWND {
    HWND(v as *mut _)
}

/// [`WindowHandle`] 付きの窓 entity を作る。
fn spawn_window(world: &mut World, hwnd: HWND) -> Entity {
    world
        .spawn(WindowHandle {
            hwnd,
            instance: HINSTANCE::default(),
        })
        .id()
}

/// ハンドル未付与の窓 entity を作る（実体はあるが OS のハンドルがまだ無い状態）。
fn spawn_handleless_window(world: &mut World) -> Entity {
    world.spawn_empty().id()
}

/// バルーン窓へペア宣言を付ける（peer はキャラ窓）。
fn declare_pair(world: &mut World, balloon: Entity, character: Entity) {
    world
        .entity_mut(balloon)
        .insert(KeepDirectlyAbove { peer: character });
}

/// 既にあるハンドルを付け直して、同じハンドルでの付与巡をもう一度作る。
///
/// `Added<WindowHandle>` は「その巡で付いた」ことしか見ないため、除去→再付与で
/// 確立系のトリガをもう一度立てられる。再試行の抑止が効いているのか、そもそも
/// トリガが立っていないだけなのかを分けるために使う。
fn retrigger_handle(world: &mut World, entity: Entity) {
    let handle = *world
        .entity(entity)
        .get::<WindowHandle>()
        .expect("再付与の対象はハンドルを持っているはず");
    world.entity_mut(entity).remove::<WindowHandle>();
    world.entity_mut(entity).insert(handle);
}

/// 確立系だけを載せた 1 巡分の schedule。
///
/// **シングルスレッド実行器を明示する**——既定の多スレッド実行器ではシステムが別スレッドで
/// 走りうるため、[`capture_under_filter`]（スレッドローカルの dispatcher 差し替え）が
/// 1 行も拾えず、記録の検査が空虚に緑になる。
fn establish_schedule() -> Schedule {
    let mut schedule = Schedule::default();
    schedule.set_executor(SingleThreadedExecutor::new());
    schedule.add_systems(establish_owner_links);
    schedule
}

/// 捕捉した出力から、指定タグを含む行をすべて取り出す。
fn lines_with<'a>(out: &'a str, tag: &str) -> Vec<&'a str> {
    out.lines().filter(|l| l.contains(tag)).collect()
}

/// 指定タグの行がちょうど `expected` 本であることを確かめる。
///
/// 「0 本でないこと」だけを見ると余計に出る側の欠陥（同じ確立が 2 度走る等）を
/// 見逃すため、常に本数で固定する。
fn assert_line_count(out: &str, tag: &str, expected: usize, what: &str) {
    let found = lines_with(out, tag);
    assert_eq!(
        found.len(),
        expected,
        "{what}: `{tag}` の行は {expected} 本のはずが {} 本だった\n---捕捉全文---\n{out}",
        found.len()
    );
}

// -------------------------------------------------------------------------
// 確立が 1 回だけ走る
// -------------------------------------------------------------------------

/// 両窓のハンドルが揃っていれば owner が張られ、以後の巡では二度と張らない。
///
/// 検査するのは自前の帳簿（`OwnerLink`）だけではなく、**OS 側の事実**
/// （`GetWindow(GW_OWNER)`）も含む。加えて初期隣接を確定させる再断行の要求が
/// 1 度入ること（design.md「案 A の確立フロー」2）も見る。
#[test]
fn establishes_owner_once_and_never_again() {
    let owner_hwnd = create_test_window(w!("wintf-zorder-establish-once-char"));
    let owned_hwnd = create_test_window(w!("wintf-zorder-establish-once-balloon"));
    assert_eq!(
        owner_of(owned_hwnd),
        None,
        "生成直後のバルーン窓に owner は無いはず（ここが最初から有りなら以降の検査は無意味）"
    );

    let mut world = World::new();
    world.insert_resource(ZOrderPairStrategy::default());
    let character = spawn_window(&mut world, owner_hwnd);
    let balloon = spawn_window(&mut world, owned_hwnd);
    declare_pair(&mut world, balloon, character);

    let mut schedule = establish_schedule();
    let out = capture_under_filter(SIGNOFF_DIRECTIVES, || {
        // 1 巡目: 両ハンドルが揃った巡（ここで張られる）
        schedule.run(&mut world);
        // 2 巡目: 新しいハンドル付与が無い巡（何も起きない）
        schedule.run(&mut world);
        // 3 巡目: ハンドル付与をもう一度立てても、確立済みなら二度目は走らない
        retrigger_handle(&mut world, character);
        schedule.run(&mut world);
    });

    assert_line_count(&out, OWNER_ESTABLISHED_TAG, 1, "確立は 1 回だけ");
    assert_line_count(&out, OWNER_ESTABLISH_FAILED_TAG, 0, "失敗はしていない");

    assert_eq!(
        owner_of(owned_hwnd),
        Some(owner_hwnd),
        "OS 側でバルーン窓の owner がキャラ窓になっているはず"
    );
    assert_eq!(
        world.entity(balloon).get::<OwnerLink>().copied(),
        Some(OwnerLink { owner_hwnd }),
        "確立の記録はバルーン窓側へ付き、owner のハンドルを持つ"
    );
    assert!(
        world.entity(balloon).contains::<ReassertZOrder>(),
        "初期隣接を確定させる再断行の要求が入っているはず"
    );
    assert!(
        world.entity(character).get::<OwnerLink>().is_none(),
        "確立の記録はキャラ窓側には付かない（切離しの対象はバルーン窓）"
    );

    destroy_test_window(owned_hwnd);
    destroy_test_window(owner_hwnd);
}

/// ハンドルが別々の巡で付く場合、**揃った巡**で 1 回だけ張る。
/// 揃っていない巡は理由つきの見送りとして記録される。
#[test]
fn establishes_on_the_cycle_the_second_handle_arrives() {
    let owner_hwnd = create_test_window(w!("wintf-zorder-establish-late-char"));
    let owned_hwnd = create_test_window(w!("wintf-zorder-establish-late-balloon"));

    let mut world = World::new();
    world.insert_resource(ZOrderPairStrategy::default());
    let character = spawn_handleless_window(&mut world);
    let balloon = spawn_window(&mut world, owned_hwnd);
    declare_pair(&mut world, balloon, character);

    let mut schedule = establish_schedule();
    let out = capture_under_filter(SIGNOFF_DIRECTIVES, || {
        // 1 巡目: バルーン窓だけにハンドルが付いた巡——張らずに見送る
        schedule.run(&mut world);
        assert!(
            world.entity(balloon).get::<OwnerLink>().is_none(),
            "片方のハンドルが無い巡では確立しない"
        );
        // 2 巡目: キャラ窓にハンドルが付いた巡——ここで揃う
        world.entity_mut(character).insert(WindowHandle {
            hwnd: owner_hwnd,
            instance: HINSTANCE::default(),
        });
        schedule.run(&mut world);
        // 3 巡目: もう新しい付与は無い
        schedule.run(&mut world);
    });

    assert_line_count(&out, OWNER_ESTABLISHED_TAG, 1, "揃った巡で 1 回だけ張る");
    assert_line_count(
        &out,
        SKIP_TAG,
        1,
        "揃っていない巡は理由つきで 1 回だけ見送られる",
    );
    let skip = lines_with(&out, SKIP_TAG)[0];
    assert!(
        skip.contains("reason=HandleMissing"),
        "見送りの理由は「まだハンドルが付いていない」であるはず: {skip}"
    );
    assert_eq!(owner_of(owned_hwnd), Some(owner_hwnd));

    destroy_test_window(owned_hwnd);
    destroy_test_window(owner_hwnd);
}

// -------------------------------------------------------------------------
// 見送り（理由つき・同じ捕捉窓に必ず出る記録を併置する）
// -------------------------------------------------------------------------

/// 片方のハンドルが欠けたペアは見送られ、同じ巡の**揃ったペアは確立される**。
///
/// 2 つのペアを同じ World・同じ捕捉窓に置くのは、「確立が出ないこと」の主張が
/// 捕捉の死によって空虚に成立しないようにするためである。
///
/// 欠けたまま何巡しても見送りの記録は増えない——確立系はハンドルが付いた巡だけ動き、
/// 毎フレーム巡回では動かない（design.md「Responsibilities & Constraints」の受動性）。
/// 3 巡回すのはこの性質を本数で固定するためである。
#[test]
fn skips_handle_missing_pair_while_the_complete_pair_establishes() {
    let owner_hwnd = create_test_window(w!("wintf-zorder-establish-mixed-char"));
    let owned_hwnd = create_test_window(w!("wintf-zorder-establish-mixed-balloon"));

    let mut world = World::new();
    world.insert_resource(ZOrderPairStrategy::default());

    // 揃ったペア（対照・確かに記録が出る側）
    let character = spawn_window(&mut world, owner_hwnd);
    let balloon = spawn_window(&mut world, owned_hwnd);
    declare_pair(&mut world, balloon, character);

    // 片方欠けのペア（Win32 へは渡らないので偽ハンドルでよい）
    let half_character = spawn_handleless_window(&mut world);
    let half_balloon = spawn_window(&mut world, fake_hwnd(0x00B1));
    declare_pair(&mut world, half_balloon, half_character);

    let mut schedule = establish_schedule();
    let out = capture_under_filter(SIGNOFF_DIRECTIVES, || {
        schedule.run(&mut world);
        // ハンドル付与の無い巡では何も起きない（見送りの記録も増えない）
        schedule.run(&mut world);
        schedule.run(&mut world);
    });

    assert_line_count(&out, OWNER_ESTABLISHED_TAG, 1, "確立するのは揃ったペアだけ");
    assert_line_count(
        &out,
        SKIP_TAG,
        1,
        "見送りはハンドルが付いた巡に 1 回だけ（毎巡は記録しない）",
    );
    let skip = lines_with(&out, SKIP_TAG)[0];
    assert!(
        skip.contains(&format!("entity={half_balloon:?}")),
        "見送りの記録は欠けたペアのバルーン窓を指すはず: {skip}"
    );
    assert!(skip.contains("reason=HandleMissing"), "{skip}");

    assert!(
        world.entity(half_balloon).get::<OwnerLink>().is_none(),
        "欠けたペアには確立の記録が付かない"
    );
    assert!(
        !world.entity(half_balloon).contains::<ReassertZOrder>(),
        "欠けたペアには再断行の要求も入らない"
    );
    assert!(world.entity(balloon).get::<OwnerLink>().is_some());

    destroy_test_window(owned_hwnd);
    destroy_test_window(owner_hwnd);
}

/// 宣言の相手（キャラ窓の実体）が既に無いペアは、理由「相手が居ない」で見送る。
#[test]
fn skips_with_peer_missing_when_the_declared_peer_is_gone() {
    let owner_hwnd = create_test_window(w!("wintf-zorder-establish-gone-char"));
    let owned_hwnd = create_test_window(w!("wintf-zorder-establish-gone-balloon"));

    let mut world = World::new();
    world.insert_resource(ZOrderPairStrategy::default());

    // 対照（確かに確立される側）
    let character = spawn_window(&mut world, owner_hwnd);
    let balloon = spawn_window(&mut world, owned_hwnd);
    declare_pair(&mut world, balloon, character);

    // 相手が既に消えているペア
    let gone_character = spawn_handleless_window(&mut world);
    let orphan_balloon = spawn_window(&mut world, fake_hwnd(0x00B2));
    declare_pair(&mut world, orphan_balloon, gone_character);
    world.entity_mut(gone_character).despawn();

    let mut schedule = establish_schedule();
    let out = capture_under_filter(SIGNOFF_DIRECTIVES, || {
        schedule.run(&mut world);
        // ハンドル付与の無い巡では見送りの記録も増えない
        schedule.run(&mut world);
        schedule.run(&mut world);
    });

    assert_line_count(&out, OWNER_ESTABLISHED_TAG, 1, "確立するのは生きたペアだけ");
    assert_line_count(
        &out,
        SKIP_TAG,
        1,
        "見送りはハンドルが付いた巡に 1 回だけ（毎巡は記録しない）",
    );
    let skip = lines_with(&out, SKIP_TAG)[0];
    assert!(skip.contains("reason=PeerMissing"), "{skip}");
    assert!(
        world.entity(orphan_balloon).get::<OwnerLink>().is_none(),
        "相手が居ないペアには何も書き込まない（要件 1.5）"
    );
    assert!(
        !world.entity(orphan_balloon).contains::<ReassertZOrder>(),
        "相手が居ないペアには再断行の要求も入らない（要件 1.5）"
    );

    destroy_test_window(owned_hwnd);
    destroy_test_window(owner_hwnd);
}

/// 案 B（明示維持）のときは owner を張らず、理由「このストラテジでは扱わない」で見送る。
///
/// 「張られていない」は OS 側（`GetWindow(GW_OWNER)`）でも確かめる——帳簿だけを見ると
/// 「記録を付け忘れただけで owner は張っている」という最悪の形を見逃す。
/// 同じ捕捉窓に確かに出る記録（見送り 1 本）が併置されているため、
/// 「確立の記録が 0 本」の主張は捕捉の死では成立しない。
#[test]
fn skips_with_strategy_disabled_under_explicit_maintenance() {
    let owner_hwnd = create_test_window(w!("wintf-zorder-establish-planb-char"));
    let owned_hwnd = create_test_window(w!("wintf-zorder-establish-planb-balloon"));

    let mut world = World::new();
    world.insert_resource(ZOrderPairStrategy::ExplicitMaintenance);
    let character = spawn_window(&mut world, owner_hwnd);
    let balloon = spawn_window(&mut world, owned_hwnd);
    declare_pair(&mut world, balloon, character);

    let mut schedule = establish_schedule();
    let out = capture_under_filter(SIGNOFF_DIRECTIVES, || {
        schedule.run(&mut world);
        // ハンドル付与の無い巡では見送りの記録も増えない
        schedule.run(&mut world);
        schedule.run(&mut world);
    });

    assert_line_count(
        &out,
        SKIP_TAG,
        1,
        "見送りはハンドルが付いた巡に 1 回だけ（毎巡は記録しない）",
    );
    let skip = lines_with(&out, SKIP_TAG)[0];
    assert!(skip.contains("reason=StrategyDisabled"), "{skip}");
    assert_line_count(&out, OWNER_ESTABLISHED_TAG, 0, "案 B では owner を張らない");
    assert_eq!(
        owner_of(owned_hwnd),
        None,
        "案 B では OS 側にも owner が付かない"
    );
    assert!(world.entity(balloon).get::<OwnerLink>().is_none());
    assert!(!world.entity(balloon).contains::<ReassertZOrder>());

    destroy_test_window(owned_hwnd);
    destroy_test_window(owner_hwnd);
}

// -------------------------------------------------------------------------
// 失敗（error 水準・同じハンドルでは繰り返さない）
// -------------------------------------------------------------------------

/// 確立に失敗したら error 水準で 1 度だけ記録し、同じハンドルでは再試行しない。
///
/// 「再試行しない」の主張が「そもそもトリガが立っていなかった」で空虚に成立しないよう、
/// 最後に**失敗の印だけを外して同じトリガをもう一度立て**、そこで 2 本目が出ることまで
/// 確かめる。これで抑止しているのが印であることが両方向から決まる。
#[test]
fn records_failure_once_and_does_not_retry_on_the_same_handles() {
    let mut world = World::new();
    world.insert_resource(ZOrderPairStrategy::default());
    // 無効ハンドル（null）は決定論的に失敗する（`api.rs` のラッパテストと同型）。
    let character = spawn_window(&mut world, HWND::default());
    let balloon = spawn_window(&mut world, HWND::default());
    declare_pair(&mut world, balloon, character);

    let mut schedule = establish_schedule();
    let out = capture_under_filter(SIGNOFF_DIRECTIVES, || {
        // 1 巡目: 揃っている——試みて失敗する
        schedule.run(&mut world);
        // 2 巡目: 同じハンドルで付与巡をもう一度立てても再試行しない
        retrigger_handle(&mut world, character);
        schedule.run(&mut world);
    });

    assert_line_count(
        &out,
        OWNER_ESTABLISH_FAILED_TAG,
        1,
        "失敗の記録は 1 本だけ（同じハンドルでは再試行しない）",
    );
    assert_line_count(&out, OWNER_ESTABLISHED_TAG, 0, "失敗したので確立はしない");
    assert!(
        world.entity(balloon).get::<OwnerLink>().is_none(),
        "失敗したペアに確立の記録は付かない"
    );
    assert!(
        !world.entity(balloon).contains::<ReassertZOrder>(),
        "失敗したペアに再断行の要求は入らない"
    );
    assert!(
        world.entity(balloon).contains::<OwnerEstablishFailed>(),
        "再試行を抑える印が付いているはず"
    );

    // 抑止しているのが印であることの確認: 印を外して同じトリガを立てると 2 本目が出る。
    let out_again = capture_under_filter(SIGNOFF_DIRECTIVES, || {
        world.entity_mut(balloon).remove::<OwnerEstablishFailed>();
        retrigger_handle(&mut world, character);
        schedule.run(&mut world);
    });
    assert_line_count(
        &out_again,
        OWNER_ESTABLISH_FAILED_TAG,
        1,
        "印を外せば同じトリガで再び試み、再び失敗が記録される",
    );
}

// -------------------------------------------------------------------------
// 確立直後の実測（記録の付随フィールド）
// -------------------------------------------------------------------------

/// 実測の失敗は「手前に誰も居ない」と同じ `None` へ潰れるが、黙って消えはしない。
///
/// 無効ハンドルでの走査は必ず失敗する。値としては不在（記録では番兵）になり、
/// 失敗そのものは記録に残る（[areka-log-first-no-silent-failure]）。
#[test]
fn measure_window_above_maps_failure_to_absence_and_records_it() {
    let out = capture_under_filter(SIGNOFF_DIRECTIVES, || {
        assert_eq!(measure_window_above(HWND::default()), None);
    });
    assert!(
        !out.trim().is_empty(),
        "走査の失敗は記録に残るはず（黙って None へ潰さない）"
    );
}

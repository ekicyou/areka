//! 破棄時の owner 切離しに対するヘッドレステスト（実機・実ディスプレイ不要）。
//!
//! 本ファイルが固定するのは 4 つである。
//!
//! - **切離しが実際に走ること**——ペアの相手（キャラ窓）の実体が消えた巡で、残るバルーン窓の
//!   owner が OS 側で外れること（要件 5.9・design.md「破棄経路」の標準機構）。
//! - **残る窓と他スコープの窓が消えないこと**——切離し後にキャラ窓を破棄しても、バルーン窓も
//!   他スコープの 2 窓も生きたまま残ること（要件 5.7）。
//! - **他スコープへ owner を張らないこと**——確立後の 4 枚の owner を全て読み戻し、
//!   スコープをまたぐ owner 関係が 1 つも無いこと（要件 5.7 の構造的根拠）。
//! - **失敗しても処理が続くこと**——切離しに失敗しても記録を残して継続し、同じ失敗を毎巡
//!   繰り返さないこと（要件 6.4）。
//!
//! # なぜ本物の窓を作り、本当に破棄するのか
//!
//! 要件 5.9 が禁じているのは「OS の破棄カスケードに巻き込まれた窓へ、後発の破棄がもう一度
//! `DestroyWindow` を撃つ」形である。カスケードは Win32 が owner 関係に対して行う挙動であり、
//! 偽の実装へ差し替えると「切り離せたから巻き込まれなかった」という主張そのものが検査不能に
//! なる。よって本ファイルは自前の非表示トップレベル窓を実際に作り、本当に破棄し、
//! `IsWindow` で生死を確かめる（owner 関係は自プロセスの 2 枚の間で決定論的に成立する——
//! 非決定になるのはトップレベル窓どうしの**重なり隣接**であって owner ではない）。
//!
//! そのカスケードがこの環境で本当に起きるのかは、[`owner_cascade_destroys_the_owned_window_unless_it_is_detached`]
//! が最初に確かめる。これが無いと「バルーン窓が生きている」という判定は、カスケードが
//! そもそも起きない環境でも緑になってしまう（＝主張が空虚になる）。
//!
//! # 記録捕捉で「出ないこと」を主張するときの作法
//!
//! 「この破棄順では切離しが走らない」の主張は、捕捉そのものが死んでいても成立してしまう。
//! よって出ないことを見るテストは、**同じ捕捉窓の中で確かに出る記録を併置**し、
//! 「その種の行がちょうど N 本」と**その行が指す entity**まで検査する（複数ペアを同時に
//! 扱うため、他ペアの記録に紛れる形を塞ぐ）。シングルスレッド実行器を明示する理由は
//! 確立系・維持系のテストと同じである（既定の多スレッド実行器では
//! [`capture_under_filter`] が 1 行も拾えず、記録の検査が空虚に緑になる）。

use bevy_ecs::prelude::*;
use bevy_ecs::schedule::{ExecutorKind, Schedule};
use windows::Win32::Foundation::{HINSTANCE, HWND};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DestroyWindow, GW_OWNER, GetWindow, IsWindow, WINDOW_EX_STYLE, WINDOW_STYLE,
    WS_POPUP,
};
use windows::core::{PCWSTR, w};

use super::apply_zorder_pair_maintenance;
use crate::api::{clear_window_owner, set_window_owner};
use crate::ecs::test_support::capture_under_filter;
use crate::ecs::window::{
    KeepDirectlyAbove, OwnerLink, ReassertZOrder, WindowHandle, ZOrderPairStrategy,
    establish_owner_links,
};

/// 実機サインオフが用いる `RUST_LOG` 相当（design.md「Real-Machine Signoff」の wintf 側）。
const SIGNOFF_DIRECTIVES: &str = "info,wintf::ecs::window::zorder_pair=debug";

/// 切離しが走った記録の判定語。
const DETACHED_MARK: &str = "owner を切り離しました";
/// 切離しに失敗した記録の判定語。
const DETACH_FAILED_MARK: &str = "owner の切離しに失敗しました";
/// 見送りの記録タグ（理由必須）。
const SKIP_TAG: &str = "[zorder-pair] skip";

// -------------------------------------------------------------------------
// 実窓・World・巡回のヘルパ
// -------------------------------------------------------------------------

/// 自プロセス所有の非表示トップレベル窓を作る（確立系のテストと同一のレシピ）。
///
/// owner 関係はトップレベル窓の間でのみ意味を持つため `WS_POPUP`（親なし）で作る
/// ——子窓に対する `GWLP_HWNDPARENT` の書込は owner ではなく親の付け替えになる。
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

/// 生成したテスト窓を破棄する（本番の registry drop 相当）。
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

/// 窓がまだ実在するか（破棄に巻き込まれていないか）。
fn is_alive(hwnd: HWND) -> bool {
    // SAFETY: Win32 境界。読み取りのみ。
    unsafe { IsWindow(Some(hwnd)) }.as_bool()
}

fn hwnd_hex(hwnd: HWND) -> String {
    format!("0x{:X}", hwnd.0 as usize)
}

fn spawn_window(world: &mut World, hwnd: HWND) -> Entity {
    world
        .spawn(WindowHandle {
            hwnd,
            instance: HINSTANCE::default(),
        })
        .id()
}

/// 確立系と維持系を載せた 1 巡分の schedule。
///
/// 本番と同じ順（確立 → 維持）で並べる。切離しは維持系の中で走る。
fn pair_schedule() -> Schedule {
    let mut schedule = Schedule::default();
    schedule.set_executor_kind(ExecutorKind::SingleThreaded);
    schedule.add_systems((establish_owner_links, apply_zorder_pair_maintenance).chain());
    schedule
}

/// 実窓 2 枚のペアを World へ載せ、owner を実際に張るところまで進める。
///
/// 返すのは（バルーン窓 entity, キャラ窓 entity）。確立系が初期隣接の確定のために入れる
/// 再断行の要求はここで外す——**重なりの是正は維持系のテストの担当**であり、本ファイルが
/// 見るのは切離しだけである（残すと実窓の重なりを動かす指令が飛び、破棄の観測に無関係な
/// 記録が混じる）。
fn establish_pair(
    world: &mut World,
    schedule: &mut Schedule,
    character_hwnd: HWND,
    balloon_hwnd: HWND,
) -> (Entity, Entity) {
    let character = spawn_window(world, character_hwnd);
    let balloon = spawn_window(world, balloon_hwnd);
    world
        .entity_mut(balloon)
        .insert(KeepDirectlyAbove { peer: character });
    schedule.run(world);
    assert_eq!(
        owner_of(balloon_hwnd),
        Some(character_hwnd),
        "前提: 確立系がバルーン窓の owner をキャラ窓にしているはず"
    );
    assert!(
        world.entity(balloon).contains::<OwnerLink>(),
        "前提: 張れた事実の記録が付いているはず"
    );
    world.entity_mut(balloon).remove::<ReassertZOrder>();
    (balloon, character)
}

fn lines_with<'a>(out: &'a str, mark: &str) -> Vec<&'a str> {
    out.lines().filter(|l| l.contains(mark)).collect()
}

/// 指定の判定語を含む行がちょうど `expected` 本であることを確かめる。
///
/// 「0 本でないこと」だけを見ると余計に出る側の欠陥（毎巡同じ切離しを繰り返す等）を
/// 見逃すため、常に本数で固定する。
fn assert_line_count(out: &str, mark: &str, expected: usize, what: &str) {
    let found = lines_with(out, mark);
    assert_eq!(
        found.len(),
        expected,
        "{what}: `{mark}` の行は {expected} 本のはずが {} 本だった\n---捕捉全文---\n{out}",
        found.len()
    );
}

// -------------------------------------------------------------------------
// 前提の較正——owner の破棄カスケードはこの環境で本当に起きるのか
// -------------------------------------------------------------------------

/// owner を張ったまま owner 窓を破棄すると被 owner 窓も消える。切り離してあれば消えない。
///
/// これが要件 5.9 の出所であり、以降のテストで「バルーン窓が生きている」という判定が
/// 空虚でないことの根拠でもある（カスケードが起きない環境なら本テストが赤になる）。
#[test]
fn owner_cascade_destroys_the_owned_window_unless_it_is_detached() {
    // ① 張ったまま破棄する——被 owner 窓は道連れになる。
    let owner = create_test_window(w!("wintf-zorder-detach-cascade-owner"));
    let owned = create_test_window(w!("wintf-zorder-detach-cascade-owned"));
    set_window_owner(owned, owner).expect("set_window_owner must succeed");
    assert!(is_alive(owned), "前提: 破棄前は両方とも実在する");
    destroy_test_window(owner);
    assert!(
        !is_alive(owned),
        "owner 窓の破棄は被 owner 窓を巻き込む（これが要件 5.9 の出所）"
    );

    // ② 先に切り離してから破棄する——巻き込まれない。
    let owner = create_test_window(w!("wintf-zorder-detach-cascade-owner-2"));
    let owned = create_test_window(w!("wintf-zorder-detach-cascade-owned-2"));
    set_window_owner(owned, owner).expect("set_window_owner must succeed");
    clear_window_owner(owned).expect("clear_window_owner must succeed");
    destroy_test_window(owner);
    assert!(
        is_alive(owned),
        "切り離してあれば owner 窓の破棄に巻き込まれない"
    );

    destroy_test_window(owned);
}

// -------------------------------------------------------------------------
// キャラ先行の破棄（切離しが走る側）
// -------------------------------------------------------------------------

/// キャラ窓の実体が先に消えた巡で owner が外れ、後続のキャラ窓破棄がバルーン窓を
/// 巻き込まない。
///
/// 破棄の順は本番と同じ——実体の消滅（despawn）が先で、OS の破棄（`DestroyWindow`）は
/// 登録の取り崩しで後から走る。切離しはその**間**に入る。
#[test]
fn character_first_despawn_detaches_the_owner_and_spares_the_balloon() {
    let character_hwnd = create_test_window(w!("wintf-zorder-detach-char-first-character"));
    let balloon_hwnd = create_test_window(w!("wintf-zorder-detach-char-first-balloon"));

    let mut world = World::new();
    world.insert_resource(ZOrderPairStrategy::default());
    let mut schedule = pair_schedule();
    let (balloon, character) =
        establish_pair(&mut world, &mut schedule, character_hwnd, balloon_hwnd);

    // キャラ窓の実体が消える（窓そのものはまだ実在する）。
    world.entity_mut(character).despawn();

    let out = capture_under_filter(SIGNOFF_DIRECTIVES, || {
        schedule.run(&mut world);
    });

    assert_line_count(&out, DETACHED_MARK, 1, "切離しの記録は 1 本だけ");
    assert_line_count(&out, DETACH_FAILED_MARK, 0, "切離しは成功しているはず");
    let detached = lines_with(&out, DETACHED_MARK)[0];
    assert!(
        detached.contains(&format!("entity={balloon:?}")),
        "記録は owner を外した窓（バルーン窓）を指すはず: {detached}"
    );
    assert!(
        detached.contains(&format!("peer={character:?}")),
        "記録は消えた相手（キャラ窓）を指すはず: {detached}"
    );
    assert!(
        detached.contains(&format!("owned_hwnd={}", hwnd_hex(balloon_hwnd))),
        "記録には owner を外した窓のハンドルが載るはず: {detached}"
    );
    assert!(
        detached.contains(&format!("owner_hwnd={}", hwnd_hex(character_hwnd))),
        "記録には外した owner のハンドルが載るはず: {detached}"
    );
    assert!(
        detached.contains("peer_state=despawned"),
        "記録には相手が実体ごと消えたことが載るはず: {detached}"
    );

    // 帳簿ではなく OS 側の事実を見る。
    assert_eq!(
        owner_of(balloon_hwnd),
        None,
        "OS 側でも owner が外れているはず"
    );
    assert!(
        !world.entity(balloon).contains::<OwnerLink>(),
        "張れた事実の記録は切離しとともに消えるはず（毎巡の再実行を残さない）"
    );

    // ここで初めて OS の破棄が走る（本番の registry 取り崩し相当）。
    destroy_test_window(character_hwnd);
    assert!(!is_alive(character_hwnd), "キャラ窓は破棄されたはず");
    assert!(
        is_alive(balloon_hwnd),
        "切り離してあるのでバルーン窓は巻き込まれないはず（残る窓は消えない）"
    );

    // 後発の破棄が無効ハンドルを撃たない——バルーン窓は自分の破棄で初めて消える。
    destroy_test_window(balloon_hwnd);
    assert!(!is_alive(balloon_hwnd));
}

/// 相手の実体が残っていても、OS のハンドルが外れていれば切り離す。
///
/// ハンドルの除去は窓の破棄要求そのもの（`WindowHandle` の除去フックが `WM_CLOSE` を
/// 投函する）であり、その時点で相手の窓はもう無くなる。実体の消滅を待つと破棄に間に合わない
/// ため、こちらの形でも切離しへ入る。記録の `peer_state` で 2 つの形を見分けられる。
#[test]
fn peer_losing_its_handle_also_detaches_the_owner() {
    let character_hwnd = create_test_window(w!("wintf-zorder-detach-handle-removed-character"));
    let balloon_hwnd = create_test_window(w!("wintf-zorder-detach-handle-removed-balloon"));

    let mut world = World::new();
    world.insert_resource(ZOrderPairStrategy::default());
    let mut schedule = pair_schedule();
    let (balloon, character) =
        establish_pair(&mut world, &mut schedule, character_hwnd, balloon_hwnd);

    // 実体は残したまま、OS のハンドルだけが外れる。
    world.entity_mut(character).remove::<WindowHandle>();

    let out = capture_under_filter(SIGNOFF_DIRECTIVES, || {
        schedule.run(&mut world);
        schedule.run(&mut world);
    });

    assert_line_count(&out, DETACHED_MARK, 1, "切離しの記録は 1 本だけ");
    let detached = lines_with(&out, DETACHED_MARK)[0];
    assert!(
        detached.contains(&format!("entity={balloon:?}")),
        "記録は owner を外した窓を指すはず: {detached}"
    );
    assert!(
        detached.contains("peer_state=handle-removed"),
        "記録には相手のハンドルが外れた形として載るはず（実体の消滅とは区別する）: {detached}"
    );
    assert_eq!(
        owner_of(balloon_hwnd),
        None,
        "OS 側でも owner が外れているはず"
    );

    destroy_test_window(character_hwnd);
    assert!(is_alive(balloon_hwnd), "残る窓は巻き込まれない");
    destroy_test_window(balloon_hwnd);
}

// -------------------------------------------------------------------------
// バルーン先行の破棄（切離しの対象が居ない側）
// -------------------------------------------------------------------------

/// バルーン窓が先に消えた場合、キャラ窓は巻き込まれず、処理も止まらない。
///
/// 被 owner 側（バルーン窓）の破棄は owner 側（キャラ窓）を巻き込まないため、この順では
/// 切離しの対象そのものが存在しない。「切離しが走らない」ことを空虚でなく主張するため、
/// 同じ捕捉窓に**キャラ先行で消える別スコープ**を併置し、切離しの記録がちょうど 1 本
/// （＝そちらのペアの分だけ）であることと、その行が指す entity まで検査する。
#[test]
fn balloon_first_destruction_leaves_the_character_and_processing_continues() {
    let character_hwnd = create_test_window(w!("wintf-zorder-detach-balloon-first-character"));
    let balloon_hwnd = create_test_window(w!("wintf-zorder-detach-balloon-first-balloon"));
    let other_character_hwnd = create_test_window(w!("wintf-zorder-detach-balloon-first-other-c"));
    let other_balloon_hwnd = create_test_window(w!("wintf-zorder-detach-balloon-first-other-b"));

    let mut world = World::new();
    world.insert_resource(ZOrderPairStrategy::default());
    let mut schedule = pair_schedule();
    let (balloon, _character) =
        establish_pair(&mut world, &mut schedule, character_hwnd, balloon_hwnd);
    let (other_balloon, other_character) = establish_pair(
        &mut world,
        &mut schedule,
        other_character_hwnd,
        other_balloon_hwnd,
    );

    // バルーン先行——被 owner 側が実体ごと消え、続いて窓が破棄される。
    world.entity_mut(balloon).despawn();
    destroy_test_window(balloon_hwnd);
    // 対照（キャラ先行で消える別スコープ）。
    world.entity_mut(other_character).despawn();

    let out = capture_under_filter(SIGNOFF_DIRECTIVES, || {
        schedule.run(&mut world);
        schedule.run(&mut world);
    });

    assert_line_count(
        &out,
        DETACHED_MARK,
        1,
        "切離しが走るのは相手を失った側だけ（バルーン先行では対象が居ない）",
    );
    assert!(
        lines_with(&out, DETACHED_MARK)[0].contains(&format!("entity={other_balloon:?}")),
        "その 1 本は対照のスコープを指すはず"
    );
    assert_line_count(&out, DETACH_FAILED_MARK, 0, "失敗は起きていない");

    assert!(!is_alive(balloon_hwnd), "バルーン窓は破棄されたはず");
    assert!(
        is_alive(character_hwnd),
        "被 owner 窓の破棄は owner 窓を巻き込まない（残る窓は消えない）"
    );

    // 残ったキャラ窓は自分の破棄で正常に消える（処理は継続している）。
    destroy_test_window(character_hwnd);
    assert!(!is_alive(character_hwnd));
    destroy_test_window(other_character_hwnd);
    assert!(
        is_alive(other_balloon_hwnd),
        "対照のスコープも切離し済みゆえ巻き込まれないはず"
    );
    destroy_test_window(other_balloon_hwnd);
}

// -------------------------------------------------------------------------
// 他スコープを巻き込まない（要件 5.7）
// -------------------------------------------------------------------------

/// owner はスコープ内のペアにしか張られず、切離しも破棄も他スコープへ届かない。
///
/// 4 枚すべての owner を読み戻してスコープをまたぐ関係が 1 つも無いことを確かめ、
/// その上で片方のスコープを破棄する。残る 3 枚（同スコープのバルーン窓・他スコープの 2 枚）が
/// 生きていることまで見る。最後にもう一方のスコープも消し、切離しが 2 本目として走ること
/// ——すなわち 1 度目の切離しが繰り返されもせず、処理も止まっていないこと——を固定する。
#[test]
fn detachment_touches_only_the_pair_whose_peer_disappeared() {
    let a_character_hwnd = create_test_window(w!("wintf-zorder-detach-scope-a-character"));
    let a_balloon_hwnd = create_test_window(w!("wintf-zorder-detach-scope-a-balloon"));
    let b_character_hwnd = create_test_window(w!("wintf-zorder-detach-scope-b-character"));
    let b_balloon_hwnd = create_test_window(w!("wintf-zorder-detach-scope-b-balloon"));

    let mut world = World::new();
    world.insert_resource(ZOrderPairStrategy::default());
    let mut schedule = pair_schedule();
    let (a_balloon, a_character) =
        establish_pair(&mut world, &mut schedule, a_character_hwnd, a_balloon_hwnd);
    let (b_balloon, b_character) =
        establish_pair(&mut world, &mut schedule, b_character_hwnd, b_balloon_hwnd);

    // 構造の確認: owner はスコープ内にしか張られていない（要件 5.7 の根拠）。
    assert_eq!(owner_of(a_balloon_hwnd), Some(a_character_hwnd));
    assert_eq!(owner_of(b_balloon_hwnd), Some(b_character_hwnd));
    assert_eq!(
        owner_of(a_character_hwnd),
        None,
        "キャラ窓は誰の被 owner でもない"
    );
    assert_eq!(
        owner_of(b_character_hwnd),
        None,
        "キャラ窓は誰の被 owner でもない"
    );

    let out = capture_under_filter(SIGNOFF_DIRECTIVES, || {
        // スコープ A のキャラ窓が消える。
        world.entity_mut(a_character).despawn();
        schedule.run(&mut world);
        schedule.run(&mut world);

        assert_eq!(
            owner_of(a_balloon_hwnd),
            None,
            "スコープ A のバルーン窓は切り離されたはず"
        );
        assert_eq!(
            owner_of(b_balloon_hwnd),
            Some(b_character_hwnd),
            "スコープ B の owner には一切触れていないはず"
        );

        // スコープ A の窓を実際に破棄する。他スコープは無傷でなければならない。
        destroy_test_window(a_character_hwnd);
        assert!(is_alive(a_balloon_hwnd), "同スコープの残る窓は消えない");
        assert!(is_alive(b_character_hwnd), "他スコープの窓は消えない");
        assert!(is_alive(b_balloon_hwnd), "他スコープの窓は消えない");
        destroy_test_window(a_balloon_hwnd);

        // 続けてスコープ B も消える——処理は止まっていない。
        world.entity_mut(b_character).despawn();
        schedule.run(&mut world);
    });

    assert_line_count(
        &out,
        DETACHED_MARK,
        2,
        "切離しはスコープごとに 1 度ずつ（繰り返しも取りこぼしも無い）",
    );
    let detached = lines_with(&out, DETACHED_MARK);
    assert!(
        detached[0].contains(&format!("entity={a_balloon:?}")),
        "1 本目はスコープ A: {}",
        detached[0]
    );
    assert!(
        detached[1].contains(&format!("entity={b_balloon:?}")),
        "2 本目はスコープ B: {}",
        detached[1]
    );
    assert_line_count(&out, DETACH_FAILED_MARK, 0, "失敗は起きていない");

    assert_eq!(
        owner_of(b_balloon_hwnd),
        None,
        "スコープ B も自分の番で切り離される"
    );
    destroy_test_window(b_character_hwnd);
    assert!(
        is_alive(b_balloon_hwnd),
        "スコープ B のバルーン窓も巻き込まれない"
    );
    destroy_test_window(b_balloon_hwnd);
}

// -------------------------------------------------------------------------
// 失敗しても止まらない（要件 6.4）
// -------------------------------------------------------------------------

/// 切離しに失敗しても、記録を残して処理は続き、同じ失敗を毎巡繰り返さない。
///
/// 失敗は無効ハンドル（null HWND）で決定論的に作る。同じ捕捉窓に**切離しが成功する実窓の
/// ペア**を併置し、失敗の 1 本と成功の 1 本がそれぞれちょうど 1 本であることを見る
/// ——捕捉が死んでいれば成功側も 0 本になって赤くなり、毎巡繰り返せば本数で赤くなる。
#[test]
fn failed_detachment_is_recorded_once_and_processing_continues() {
    let character_hwnd = create_test_window(w!("wintf-zorder-detach-failure-character"));
    let balloon_hwnd = create_test_window(w!("wintf-zorder-detach-failure-balloon"));

    let mut world = World::new();
    world.insert_resource(ZOrderPairStrategy::default());
    let mut schedule = pair_schedule();
    // 対照（確かに切離しが成功する側）
    let (_balloon, character) =
        establish_pair(&mut world, &mut schedule, character_hwnd, balloon_hwnd);

    // 失敗する側——無効なハンドルを持つ窓に、張れた事実の記録だけが付いている。
    let broken_character = world.spawn_empty().id();
    let broken_balloon = spawn_window(&mut world, HWND::default());
    world.entity_mut(broken_balloon).insert((
        KeepDirectlyAbove {
            peer: broken_character,
        },
        OwnerLink {
            owner_hwnd: HWND::default(),
        },
    ));

    world.entity_mut(character).despawn();
    world.entity_mut(broken_character).despawn();

    let out = capture_under_filter(SIGNOFF_DIRECTIVES, || {
        schedule.run(&mut world);
        schedule.run(&mut world);
        schedule.run(&mut world);
    });

    assert_line_count(
        &out,
        DETACH_FAILED_MARK,
        1,
        "失敗は 1 度だけ記録される（毎巡繰り返さない）",
    );
    assert_line_count(
        &out,
        DETACHED_MARK,
        1,
        "対照のペアは同じ捕捉窓で確かに切り離される",
    );
    let failed = lines_with(&out, DETACH_FAILED_MARK)[0];
    assert!(
        failed.contains(&format!("entity={broken_balloon:?}")),
        "失敗の記録は当該の窓を指すはず: {failed}"
    );
    assert!(
        failed.contains("error="),
        "失敗の記録には理由が載るはず: {failed}"
    );
    assert!(
        !world.entity(broken_balloon).contains::<OwnerLink>(),
        "失敗しても記録は外す（同じ失敗を毎巡繰り返さない）"
    );

    destroy_test_window(character_hwnd);
    assert!(
        is_alive(balloon_hwnd),
        "対照のペアは切り離されているので巻き込まれない（処理は続いている）"
    );
    destroy_test_window(balloon_hwnd);
}

/// owner を外す先の窓が既に無い場合は、理由つきの見送りとして記録する。
///
/// 相手が消えた時点で自分のハンドルも無ければ、外す対象の窓が特定できない。黙って
/// 読み飛ばすと「記録の無い見送り」になるため、既存の見送り語彙で理由を残す（要件 6.3）。
/// 対照として、同じ捕捉窓で確かに切離しが走るペアを併置する。
#[test]
fn owner_link_without_a_handle_is_skipped_with_a_reason() {
    let character_hwnd = create_test_window(w!("wintf-zorder-detach-handleless-character"));
    let balloon_hwnd = create_test_window(w!("wintf-zorder-detach-handleless-balloon"));

    let mut world = World::new();
    world.insert_resource(ZOrderPairStrategy::default());
    let mut schedule = pair_schedule();
    // 対照（確かに切離しが走る側）
    let (_balloon, character) =
        establish_pair(&mut world, &mut schedule, character_hwnd, balloon_hwnd);

    // ハンドルを持たない側（張れた事実の記録だけが残っている）。
    let handleless_character = world.spawn_empty().id();
    let handleless_balloon = world
        .spawn((
            KeepDirectlyAbove {
                peer: handleless_character,
            },
            OwnerLink {
                owner_hwnd: HWND::default(),
            },
        ))
        .id();

    world.entity_mut(character).despawn();
    world.entity_mut(handleless_character).despawn();

    let out = capture_under_filter(SIGNOFF_DIRECTIVES, || {
        schedule.run(&mut world);
        schedule.run(&mut world);
    });

    assert_line_count(&out, SKIP_TAG, 1, "見送りは理由つきで 1 本だけ");
    let skip = lines_with(&out, SKIP_TAG)[0];
    assert!(
        skip.contains("reason=HandleMissing"),
        "見送りの理由は「ハンドルが無い」であるはず: {skip}"
    );
    assert!(
        skip.contains(&format!("entity={handleless_balloon:?}")),
        "見送りの記録は当該の窓を指すはず: {skip}"
    );
    assert_line_count(&out, DETACHED_MARK, 1, "対照のペアは確かに切り離される");
    assert_line_count(&out, DETACH_FAILED_MARK, 0, "Win32 は呼んでいない");
    assert!(
        !world.entity(handleless_balloon).contains::<OwnerLink>(),
        "外す先の無い記録は残さない（毎巡の再評価を作らない）"
    );

    destroy_test_window(character_hwnd);
    destroy_test_window(balloon_hwnd);
}

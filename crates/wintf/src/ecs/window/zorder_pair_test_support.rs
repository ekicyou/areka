//! 組の実窓テスト（維持・沈降・測り・確立・鎖の適用）が共有する下ごしらえの正本。
//!
//! 実窓の作り方・World への載せ方・1 巡分の schedule・記録の数え方・判定語を 1 か所に置く。
//! 各テストのファイルに写しを置くと、作法を直したときに 1 本だけ古いまま残る
//! （research.md 32 番）。テストの時だけ `window/mod.rs` から宣言する。
//!
//! # 窓の作り方
//!
//! トップレベル窓の重なり隣接は同一テストバイナリ内で決定論にならない——兄弟列が
//! デスクトップ全体で共有され、他テストや他プロセスの窓が割り込みうる。そこで重なりを測る
//! テストは、**テスト専用の親窓を 1 枚立て、その子窓**で窓を作る（[`create_test_parent`]・
//! [`create_test_child`]）。owner 関係はトップレベル窓の間でのみ意味を持つため、owner を
//! 見るテストは親の無い非表示の窓（[`create_test_window`]）で作る。
//!
//! # 記録の捕捉
//!
//! schedule はどれもシングルスレッド実行器を明示する——既定の多スレッド実行器では
//! システムが別スレッドで走りうるため、`capture_under_filter`（スレッドローカルの
//! dispatcher 差し替え）が 1 行も拾えず、記録の検査が空虚に緑になる。

use bevy_ecs::prelude::*;
use bevy_ecs::schedule::{Schedule, SingleThreadedExecutor};
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::{
    DestroyWindow, GW_OWNER, GetWindow, IsWindow, SW_SHOWNA, SWP_NOACTIVATE, SWP_NOMOVE,
    SWP_NOSIZE, SWP_NOZORDER, SetWindowPos, ShowWindow, WINDOW_EX_STYLE, WINDOW_STYLE, WS_CHILD,
    WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP, WS_VISIBLE,
};
use windows::core::{PCWSTR, w};

use crate::api::{get_window_below, is_window_visible};
use crate::ecs::window::{
    KeepDirectlyAbove, ReassertZOrder, SetWindowPosCommand, WindowHandle,
    apply_zorder_pair_maintenance, establish_owner_links, flush_window_pos_commands,
};

// -------------------------------------------------------------------------
// 捕捉の絞り込みと判定語
// -------------------------------------------------------------------------

/// 実機サインオフが用いる `RUST_LOG` 相当（design.md「Real-Machine Signoff」の wintf 側）。
pub(super) const SIGNOFF_DIRECTIVES: &str = "info,wintf::ecs::window::zorder_pair=debug";

/// [`SIGNOFF_DIRECTIVES`] に、指令の実施記録を出す `command` モジュールを足したもの。
///
/// 足しているのは「指令が何本出たか」を数えるためであり、サインオフの判定語を
/// 増やしているわけではない。
pub(super) const CAPTURE_DIRECTIVES: &str =
    "info,wintf::ecs::window::zorder_pair=debug,wintf::ecs::window::command=debug";

/// 是正の記録（指令と実測を同一行に載せる）。
pub(super) const FIX_TAG: &str = "[zorder-pair] fix";
/// 見送りの記録タグ（理由必須）。
pub(super) const SKIP_TAG: &str = "[zorder-pair] skip";
/// 検証不一致の記録（error 水準）。
pub(super) const VERIFY_FAILED_TAG: &str = "[zorder-pair] verify-failed";
/// 沈み込みを観測した記録。
pub(super) const SINK_OBSERVED_TAG: &str = "[zorder-pair] sink-observed";
/// owner を張れた記録。
pub(super) const OWNER_ESTABLISHED_TAG: &str = "[zorder-pair] owner-established";
/// 鎖が外させる組を見送った理由。
pub(super) const RELEASED_REASON: &str = "reason=ReleasedByChain";
/// 切離しが走った記録の判定語。
pub(super) const DETACHED_MARK: &str = "owner を切り離しました";
/// 窓の書き込みの実施記録（＝実際に発行された指令 1 本）。書き込みのファイルの文言そのもの。
pub(super) const SET_WINDOW_POS_TAG: &str = super::command::WINDOW_WRITE_LOG_MESSAGE;

// -------------------------------------------------------------------------
// 明示実行の門（実窓の重なり順を直に測るテスト: 常に手前・帯・帯の流れ）
// -------------------------------------------------------------------------

/// 実窓の重なり順を直に測るテストを明示実行へ限る門の環境変数。
///
/// 本ワークスペースの実行時環境変数は `AREKA_` 名前空間である（同じ流儀の先例:
/// `crates/log-capture-kit/tests/capture_calibration_test.rs` の `MODE_ENV`）。
const REAL_WINDOW_ZORDER_ENV: &str = "AREKA_WINTF_REAL_WINDOW_ZORDER";

/// 門が開いていなければ、判定を 1 行も実行せずに落とす。各テストは本体の 1 行目で呼ぶ。
///
/// # なぜ門を付けたか
///
/// 門の内側のテストは実窓の位置関係を直に測るため、他プロセスの可視窓が自分の窓の間へ
/// 割り込むと崩れる。既定の `cargo test` に置いたままだと間欠的な赤が出て、「全通過」
/// という記録が全通過を意味しなくなる。よって既定の走行からは外し（`#[ignore]`）、
/// 狙って走らせるときだけ走る形にした。根治（判定の作り直し）の引受先は
/// `.kiro/specs/zorder-chain-residue/brief.md` の A-1 で、そちらが「実窓の重なり順を
/// 割り込みに強い形で測る」を持つ。
///
/// # 門が閉じたまま呼ばれたら緑にしない
///
/// `--ignored` で無差別に呼ばれたときに素通りで緑を返すと、「実窓で測った」という
/// 記録に化ける。既存の手本（`crates/areka/src/placement_tests/transition_signoff_tests.rs`
/// の実機ログ判定）と同じく、条件が揃っていない呼び出しは失敗として落とす。
pub(super) fn require_explicit_real_window_run() {
    assert!(
        std::env::var_os(REAL_WINDOW_ZORDER_ENV).is_some(),
        "{REAL_WINDOW_ZORDER_ENV} が未設定である。実窓の重なり順は他プロセスの可視窓に\
         割り込まれて間欠的に赤くなるため明示実行へ隔離してある（判定は 1 行も走っていない）。\
         走らせるには環境変数を与えて `cargo test -p wintf --lib <テスト名> -- --ignored --exact` \
         とすること"
    );
}

// -------------------------------------------------------------------------
// 実窓
// -------------------------------------------------------------------------

/// 定義済 "Static" クラスで 0x0 のテスト窓を作る（生成の正本は [`crate::api::create_test_window`]）。
pub(super) fn create_window(
    title: PCWSTR,
    ex_style: WINDOW_EX_STYLE,
    style: WINDOW_STYLE,
    parent: Option<HWND>,
) -> HWND {
    crate::api::create_test_window(ex_style, style, parent, title)
}

/// 自プロセス所有の非表示トップレベル窓を作る（`api.rs` のテスト生成レシピと同一）。
///
/// owner 関係はトップレベル窓の間でのみ意味を持つため `WS_POPUP`（親なし）で作る
/// ——子窓に対する `GWLP_HWNDPARENT` の書込は owner ではなく親の付け替えになる。
pub(super) fn create_test_window(title: PCWSTR) -> HWND {
    create_window(title, WINDOW_EX_STYLE(0), WINDOW_STYLE(WS_POPUP.0), None)
}

/// テスト専用の親窓（兄弟列を閉じるための入れ物）。
///
/// **可視で作る**——実測層は可視の窓だけを隣と数えるため（`zorder_pair` の実測層 doc）、
/// 隠れた親の下の子窓では隣接そのものが測れない（`IsWindowVisible` は当該窓と祖先すべての
/// `WS_VISIBLE` を見るので、親が隠れていれば子はどう作っても不可視になる）。0x0 の道具窓
/// （`WS_EX_TOOLWINDOW`）を活性化を伴わない `SW_SHOWNA` で見せるため、画面にもタスクバーにも
/// 現れず、他のテストが握っている前面窓も奪わない。
pub(super) fn create_test_parent(title: PCWSTR) -> HWND {
    create_banded_test_parent(title, false)
}

/// [`create_test_parent`] の帯を選べる形。`always_on_top` が真なら一式を常時最前面の帯へ入れる。
pub(super) fn create_banded_test_parent(title: PCWSTR, always_on_top: bool) -> HWND {
    let ex_style = if always_on_top {
        WINDOW_EX_STYLE(WS_EX_TOOLWINDOW.0 | WS_EX_TOPMOST.0)
    } else {
        WINDOW_EX_STYLE(WS_EX_TOOLWINDOW.0)
    };
    let parent = create_window(title, ex_style, WINDOW_STYLE(WS_POPUP.0), None);
    // SAFETY: Win32 境界。自プロセス所有の窓を活性化せずに表示する。
    unsafe {
        let _ = ShowWindow(parent, SW_SHOWNA);
    }
    assert!(
        is_window_visible(parent),
        "前提: 親窓が可視でなければ子窓は決して可視にならない"
    );
    parent
}

/// 親窓の子として**可視の**実窓を生成する（本番のゴースト窓と同じ可視の窓を模す）。
///
/// 生成直後の兄弟間の重なりは前提にしない——初期配置は必ず [`insert_after`] で明示的に作る。
/// 実測すると子窓は**先に作ったものほど手前**（新しい子窓は兄弟列の背面側へ入る）であり、
/// 生成順から前後関係を読むと逆になる。
pub(super) fn create_test_child(parent: HWND, title: PCWSTR) -> HWND {
    create_window(
        title,
        WINDOW_EX_STYLE(0),
        WINDOW_STYLE(WS_CHILD.0 | WS_VISIBLE.0),
        Some(parent),
    )
}

/// 親窓の子として**不可視の**実窓を生成する（`WS_VISIBLE` 無し＝OS が owner の直上に置く
/// 既定の IME 窓に当たる役）。
pub(super) fn create_hidden_test_child(parent: HWND, title: PCWSTR) -> HWND {
    create_window(
        title,
        WINDOW_EX_STYLE(0),
        WINDOW_STYLE(WS_CHILD.0),
        Some(parent),
    )
}

/// `hwnd` を `after` のすぐ背後（1 つ背面側）へ移す（`api.rs` のテストもこれを使う）。
///
/// `SetWindowPos` の `hWndInsertAfter` は「位置決め対象の 1 つ手前に来る窓」であり、
/// 対象はその**背面側**へ入る。
pub(crate) fn insert_after(hwnd: HWND, after: HWND) {
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

/// キャラ窓・割り込み窓・バルーン窓の 3 枚を「バルーン／割り込み／キャラ」の順
/// （手前 → 背後）で可視の親の下に用意する。バルーンはキャラのすぐ手前に**居ない**
/// （是正を要する並び）。親ごと壊すのは `Drop`（テストが panic しても窓を残さない）。
pub(super) struct Trio {
    pub(super) parent: HWND,
    pub(super) character: HWND,
    pub(super) intruder: HWND,
    pub(super) balloon: HWND,
}

impl Trio {
    pub(super) fn create(tag: PCWSTR) -> Self {
        let parent = create_test_parent(tag);
        let character = create_test_child(parent, w!("character"));
        let intruder = create_test_child(parent, w!("intruder"));
        let balloon = create_test_child(parent, w!("balloon"));
        // 「バルーン／割り込み／キャラ」の順（手前 → 背後）を明示的に作る。
        insert_after(intruder, balloon);
        insert_after(character, intruder);
        let trio = Self {
            parent,
            character,
            intruder,
            balloon,
        };
        assert_eq!(
            get_window_below(trio.balloon).expect("GetWindow は成功するはず"),
            Some(trio.intruder),
            "前提: バルーンの 1 つ背後は割り込み窓（＝キャラの隣に居ない）"
        );
        trio
    }
}

impl Drop for Trio {
    fn drop(&mut self) {
        destroy_test_window(self.parent);
    }
}

/// 生成したテスト窓を破棄する（子窓は親の破棄で連鎖する）。
pub(super) fn destroy_test_window(hwnd: HWND) {
    // SAFETY: Win32 境界。生成した所有ウィンドウを破棄する。
    unsafe {
        let _ = DestroyWindow(hwnd);
    }
}

/// OS 側が持つ owner を読み戻す（`GetWindow(GW_OWNER)`）。owner 無しは `None`。
pub(super) fn owner_of(hwnd: HWND) -> Option<HWND> {
    // SAFETY: Win32 境界。読み取りのみ。
    unsafe { GetWindow(hwnd, GW_OWNER) }.ok()
}

/// 窓がまだ実在するか（破棄に巻き込まれていないか）。
pub(super) fn is_alive(hwnd: HWND) -> bool {
    // SAFETY: Win32 境界。読み取りのみ。
    unsafe { IsWindow(Some(hwnd)) }.as_bool()
}

/// 記録に載る形のハンドルの綴り。
pub(super) fn hwnd_hex(hwnd: HWND) -> String {
    format!("0x{:X}", hwnd.0 as usize)
}

/// Win32 へは渡さない偽ハンドル（値としてのみ扱う）。
pub(super) fn fake_hwnd(v: usize) -> HWND {
    HWND(v as *mut _)
}

// -------------------------------------------------------------------------
// World と巡回
// -------------------------------------------------------------------------

/// [`WindowHandle`] 付きの窓 entity を作る。
pub(super) fn spawn_window(world: &mut World, hwnd: HWND) -> Entity {
    world.spawn(WindowHandle { hwnd }).id()
}

/// バルーン窓へペア宣言を付ける（peer はキャラ窓）。
pub(super) fn declare_pair(world: &mut World, balloon: Entity, character: Entity) {
    world
        .entity_mut(balloon)
        .insert(KeepDirectlyAbove { peer: character });
}

/// 一回限りの再断行要求を外から挿入する（要件 2.6 のシームと同じ形＝未適用の既定値）。
pub(super) fn request_reassert(world: &mut World, balloon: Entity) {
    world.entity_mut(balloon).insert(ReassertZOrder::default());
}

/// 維持系だけを載せた 1 巡分の schedule。
pub(super) fn maintain_schedule() -> Schedule {
    let mut schedule = Schedule::default();
    schedule.set_executor(SingleThreadedExecutor::new());
    schedule.add_systems(apply_zorder_pair_maintenance);
    schedule
}

/// 確立系だけを載せた 1 巡分の schedule。
pub(super) fn establish_schedule() -> Schedule {
    let mut schedule = Schedule::default();
    schedule.set_executor(SingleThreadedExecutor::new());
    schedule.add_systems(establish_owner_links);
    schedule
}

/// 確立系と維持系を載せた 1 巡分の schedule。
///
/// 本番と同じ順（確立 → 維持）で並べる。切離しは維持系の中で走る。
pub(super) fn pair_schedule() -> Schedule {
    let mut schedule = Schedule::default();
    schedule.set_executor(SingleThreadedExecutor::new());
    schedule.add_systems((establish_owner_links, apply_zorder_pair_maintenance).chain());
    schedule
}

/// 重なりも位置も動かさない対照の指令を 1 本だけ発行する。
///
/// 「是正の指令が出ていない」ことを主張する捕捉窓に必ず併置する——捕捉が死んでいれば
/// この 1 本も現れず、テストは赤になる。
pub(super) fn issue_control_command(hwnd: HWND) {
    SetWindowPosCommand::enqueue(SetWindowPosCommand::new(
        hwnd,
        0,
        0,
        0,
        0,
        SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_NOZORDER,
        None,
    ));
    flush_window_pos_commands();
}

// -------------------------------------------------------------------------
// 記録の数え方
// -------------------------------------------------------------------------

/// 捕捉した出力から、指定タグを含む行をすべて取り出す。
pub(super) fn lines_with<'a>(out: &'a str, tag: &str) -> Vec<&'a str> {
    out.lines().filter(|l| l.contains(tag)).collect()
}

/// `entity` の組を、望む鎖が外させるので見送った記録（`ReleasedByChain`）の本数。
pub(super) fn released_skips(out: &str, entity: Entity) -> usize {
    let needle = format!("entity={entity:?} ");
    out.lines()
        .filter(|l| l.contains(SKIP_TAG) && l.contains(RELEASED_REASON) && l.contains(&needle))
        .count()
}

/// 指定タグの行がちょうど `expected` 本であることを確かめる。
///
/// 「0 本でないこと」だけを見ると余計に出る側の欠陥（同じ是正が 2 度走る・再試行の輪が
/// できる等）を見逃すため、常に本数で固定する。
pub(super) fn assert_line_count(out: &str, tag: &str, expected: usize, what: &str) {
    let found = lines_with(out, tag);
    assert_eq!(
        found.len(),
        expected,
        "{what}: `{tag}` の行は {expected} 本のはずが {} 本だった\n---捕捉全文---\n{out}",
        found.len()
    );
}

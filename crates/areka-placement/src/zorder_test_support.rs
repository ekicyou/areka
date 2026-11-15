//! Z 順のテスト（台帳・鎖の合成・鎖の実窓）が共に使う道具。
//!
//! 要素の `b`／`s`、要素から決定論的に組む `Entity`、繋ぎ 1 本、本番の解釈器を通したタグの
//! グループと、鎖の実窓のテスト（`zorder_chain_tail_order_tests.rs`・
//! `zorder_chain_inversion_order_tests.rs`）が共に使う Win32 の道具（窓を作る・重なりと
//! 所有を読む・助走）を置く。テストのときだけ組み込む。

use bevy_ecs::entity::Entity;
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DestroyWindow, GW_HWNDNEXT, GW_OWNER, GetTopWindow, GetWindow, IsWindow,
    IsWindowVisible, SW_SHOWNOACTIVATE, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SetWindowPos,
    ShowWindow, WINDOW_EX_STYLE, WINDOW_STYLE, WS_EX_TOOLWINDOW, WS_POPUP,
};
use windows::core::{PCWSTR, w};
use wintf::ecs::window::{ChainSegment, CrossEdge};

use crate::zorder_group_ledger::{
    GroupElement, GroupSource, GroupWindowKind, ZOrderGroup, parse_zorder_tokens,
};

/// バルーン窓の要素（`balloonN`／`bN`）。
pub(crate) fn b(scope: u32) -> GroupElement {
    GroupElement {
        scope,
        kind: GroupWindowKind::Balloon,
    }
}

/// キャラ窓の要素（`surfaceN`／`sN`）。
pub(crate) fn s(scope: u32) -> GroupElement {
    GroupElement {
        scope,
        kind: GroupWindowKind::Char,
    }
}

/// 要素 1 つに割り当てる決定論的な `Entity`（スコープ×種別で一意）。
pub(crate) fn ent(element: GroupElement) -> Entity {
    let kind = match element.kind {
        GroupWindowKind::Balloon => 0,
        GroupWindowKind::Char => 1,
    };
    Entity::from_raw_u32(element.scope * 2 + kind + 1).expect("テスト用 entity 索引は有効")
}

/// 要素列を `Entity` 列へ。
pub(crate) fn ents(elements: &[GroupElement]) -> Vec<Entity> {
    elements.iter().copied().map(ent).collect()
}

/// 手前側 `front` が奥側 `back` に所有される繋ぎ 1 本（`front ← back`・区間つき）。
///
/// 区間は**手前側の枠が属していたもの**である（合成の規則そのもの）。ここで書き下すのは
/// 「どのグループの繋ぎか」が記録から復元できることの主張であり、値を取り違えると赤くなる。
pub(crate) fn edge(front: GroupElement, back: GroupElement, segment: ChainSegment) -> CrossEdge {
    CrossEdge {
        owned: ent(front),
        owner: ent(back),
        segment,
    }
}

/// 連続する 2 枚を 1 つも除かずに繋いだ形＝「同一スコープ対の除外」を経路から外した出力。
pub(crate) fn all_consecutive_edges(members: &[Entity], segment: ChainSegment) -> Vec<CrossEdge> {
    members
        .windows(2)
        .map(|w| CrossEdge {
            owned: w[0],
            owner: w[1],
            segment,
        })
        .collect()
}

/// タグ由来のグループ 1 本を、本番と同じ解釈器（補い込み）を通して組む。
pub(crate) fn tag_group(id: u32, tokens: &[&str]) -> ZOrderGroup {
    let (members, _normalizations) =
        parse_zorder_tokens(tokens).expect("テストの指定は受理されるもの");
    ZOrderGroup {
        id,
        members,
        source: GroupSource::Tag,
    }
}

// ---------------------------------------------------------------------------
// 鎖の実窓の道具（Win32）
// ---------------------------------------------------------------------------

thread_local! {
    /// このスレッドで既定の IME 窓の受け皿をもう作ったか。
    static IME_ANCHOR_MADE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// 本番のゴースト窓と同じ作りの 0x0 窓を作り、活性化を奪わずに表示状態へ移す
/// （受け皿の面倒を見ない生の版）。
fn create_raw_window(title: PCWSTR) -> HWND {
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    // SAFETY: Win32 境界。自プロセス所有の 0x0 トップレベル窓を生成し、活性化を奪わずに
    // 表示状態へ移す。
    unsafe {
        let hinstance = GetModuleHandleW(None).expect("GetModuleHandleW");
        let hwnd = CreateWindowExW(
            WINDOW_EX_STYLE(WS_EX_TOOLWINDOW.0),
            w!("Static"),
            title,
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

/// 鎖の窓を作る。スレッド既定の不可視 IME 窓は「そのスレッドで最初に作られた窓」を所有するので、
/// 鎖の先頭になりうる窓が所有者になって本番に無い性質が入らないよう、受け皿を先に 1 枚作る
/// （受け皿は壊さない——壊すと所有が次の窓へ移りかねない。スレッドの終了で消える）。
pub(crate) fn create_window(title: PCWSTR) -> HWND {
    IME_ANCHOR_MADE.with(|made| {
        if !made.replace(true) {
            let _ = create_raw_window(w!("zorder-chain/ime-anchor"));
        }
    });
    create_raw_window(title)
}

/// 窓がまだ生きているか。
pub(crate) fn is_window(hwnd: HWND) -> bool {
    // SAFETY: Win32 境界。読み取りのみ。
    unsafe { IsWindow(Some(hwnd)) }.as_bool()
}

/// Windows から見て可視か（`WS_VISIBLE` の意味での可視）。
pub(crate) fn is_visible(hwnd: HWND) -> bool {
    // SAFETY: Win32 境界。読み取りのみ。
    unsafe { IsWindowVisible(hwnd) }.as_bool()
}

/// 作った窓を破棄する（**手前側＝所有される側から順に**渡すこと。所有する窓を先に壊すと
/// 所有される窓も道連れになる）。
pub(crate) fn teardown(windows: &[HWND]) {
    for hwnd in windows {
        // SAFETY: Win32 境界。自プロセスが生成した窓を破棄する。
        unsafe {
            let _ = DestroyWindow(*hwnd);
        }
    }
}

/// 走査をやり直す上限。観測できないまま無限に回らないためだけに置く。
const Z_SCAN_ATTEMPTS: usize = 8;

/// 1 回の走査で辿る窓の数の上限（別プロセスの窓が増え続けても固まらない保険）。
const Z_SCAN_MAX_STEPS: usize = 100_000;

/// `set` の窓の数（窓の無い枠は数えない）。
fn live_count<H: Copy + Into<Option<HWND>>>(set: &[H]) -> usize {
    set.iter().filter(|h| (**h).into().is_some()).count()
}

/// 走査の 1 回分（打ち切られたら短い列を返す）。
fn scan_z_once<H: Copy + Into<Option<HWND>>>(set: &[H]) -> Vec<usize> {
    let wanted = live_count(set);
    let mut seen = Vec::new();
    // SAFETY: Win32 境界。デスクトップ配下の最前面窓を得る読み取り専用 API。
    let mut cursor = unsafe { GetTopWindow(None) }.ok();
    let mut steps = 0usize;
    while let Some(hwnd) = cursor {
        if let Some(i) = set.iter().position(|h| (*h).into() == Some(hwnd))
            && !seen.contains(&i)
        {
            seen.push(i);
            if seen.len() == wanted {
                break;
            }
        }
        steps += 1;
        if steps > Z_SCAN_MAX_STEPS {
            break;
        }
        // SAFETY: Win32 境界。窓ハンドルに対する読み取り専用の走査。
        cursor = unsafe { GetWindow(hwnd, GW_HWNDNEXT) }.ok();
    }
    seen
}

/// `set` の生きている窓の添字だけを、Z の手前から奥へ並べる（窓の無い枠は数えない）。
///
/// 最前面から `GW_HWNDNEXT` で降りる。走査の最中に他プロセスの窓が消えると列が途中で切れる
/// （2026-08-30 の実測・3 プロセス同時走行で空の結果を得た）ので、全部拾えるまで有界回数
/// やり直し、使い切ったときはいちばん多く拾えた回を返す。生の 1 歩では測らない——不可視の
/// 隣（既定の IME 窓など）が間に挟まるので、隣接ではなく順序で見る。
pub(crate) fn z_shape<H: Copy + Into<Option<HWND>>>(set: &[H]) -> Vec<usize> {
    let wanted = live_count(set);
    let mut best = Vec::new();
    for _ in 0..Z_SCAN_ATTEMPTS {
        let seen = scan_z_once(set);
        if seen.len() == wanted {
            return seen;
        }
        if seen.len() > best.len() {
            best = seen;
        }
    }
    best
}

/// 添字ごとの持ち主（`GetWindow(GW_OWNER)`・`set` の外の持ち主と窓の無い枠は `None`）。
pub(crate) fn owner_shape<H: Copy + Into<Option<HWND>>>(set: &[H]) -> Vec<Option<usize>> {
    set.iter()
        .map(|h| {
            // SAFETY: Win32 境界。読み取りのみ。
            let owner = (*h)
                .into()
                .and_then(|h| unsafe { GetWindow(h, GW_OWNER) }.ok())?;
            set.iter().position(|w| (*w).into() == Some(owner))
        })
        .collect()
}

/// Z のみを動かす素の指令（助走専用——本番の経路ではない）。挿入位置に渡すのは常にこの
/// テストが作った窓である（他プロセスの窓だと、読み書きの間に消えて黙って失敗する）。
fn place_after(hwnd: HWND, after: HWND) {
    // SAFETY: Win32 境界。自プロセスの窓の Z のみを動かす（活性化・移動・寸法変更なし）。
    unsafe {
        let _ = SetWindowPos(
            hwnd,
            Some(after),
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
        );
    }
}

/// 組を `order`（手前から順の添字）の並びへ揃える助走（自分の窓どうしの相対指定だけで組む）。
pub(crate) fn arrange_z(set: &[HWND], order: &[usize]) {
    for pair in order.windows(2) {
        place_after(set[pair[1]], set[pair[0]]);
    }
}

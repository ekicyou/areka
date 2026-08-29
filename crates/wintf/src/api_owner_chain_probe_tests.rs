//! 設計前実測（owner の鎖）——`research.md` §11.6 の 2・3 を実測で閉じるための調査檻。
//!
//! spec `areka-P0-scope-zorder-pinning` の改訂第 2 版（要件 14）は、前後関係の維持を
//! 「所有の鎖」——分岐の無い一直線の owner 連結——で行うことを要件で固定した。設計を
//! 書く前に、次の 2 点を**推論ではなく実測で**閉じる。
//!
//! 1. **表示中の窓の owner 張り替えが z 順へ即時反映されるか**（§11.6-2）。反映が
//!    後押し待ちなら、張り替え直後の 1 回だけの後押しで足りるか。
//! 2. **横断 edge の副作用**（§11.6-3）——最小化・非表示・破棄が鎖を伝うか。
//!    要件 11.5（重なり順以外の見える性質を変えない）と要件 7.2（破棄を巻き込まない）が
//!    構造的に満たせるかの分かれ目。
//!
//! # 窓の作り
//!
//! 本番のゴースト窓と同じ `WS_POPUP` ＋ `WS_EX_TOOLWINDOW`（`placement/spawn.rs:572-576`）
//! で作り、`ShowWindow(SW_SHOWNOACTIVATE)` で**実際に表示状態**にする——「既に表示中の窓の
//! 張り替え」が問いだからである。寸法は 0x0 なので画面には 1 ピクセルも出ないが、
//! `WS_VISIBLE` は立ち、OS から見れば可視のトップレベル窓である。
//! `WS_EX_TOOLWINDOW` は本番と同じであり、この一点によって owner の付け外しが
//! **タスクバーへの出方を変えようがない**（道具窓は元からタスクバーに出ない）。
//!
//! # 測り方
//!
//! 重なりは `GetTopWindow` から `GW_HWNDNEXT` で降りる走査で読み、**自分が作った窓だけ**を
//! 拾って添字の列にする（`zorder_group_order_tests.rs` の助手と同形）。隣接ではなく順序で
//! 見るので、不可視の隣（既定 IME 窓など）や他プロセスの窓が何枚挟まっても結果は動かない。
//!
//! # 決定論のための規律（要件 10.3・実測で確定）
//!
//! 実測で潰した非決定の元は 2 つある。
//!
//! ⑴ **助走に絶対帯指定（`HWND_TOP`／`HWND_BOTTOM`）を使わない**。cargo 3 プロセス同時の
//!    regime で、絶対帯指定を助走に用いた檻だけが 18 走行中 3 回落ちた。助走は
//!    [`arrange_z`]（自分の窓どうしの相対指定）で組む。絶対帯指定を残してあるのは、
//!    **それ自体が測定対象である** [`owner_chain_probe_minimal_nudge_variants`] だけ。
//! ⑵ **挿入位置に他プロセスの窓を渡さない**。`GW_HWNDPREV`（いま 1 つ手前にいる窓）は
//!    自分の窓とは限らず、読み取りと書き込みの間に消えると `SetWindowPos` が黙って失敗する。
//!    full-suite の並走走行で実際に再現した。後押しは [`nudge_chain`]——**自分の窓 2 枚だけ**を
//!    参照する形——に統一してある。
//!
//! ⑵ は檻の都合ではなく**本番の設計判断**でもある（design.md の DD-3）。
//!
//! # 主張しないこと
//!
//! **鎖と、鎖の外の窓との前後関係は主張しない。** 鎖は 1 つの塊として動き、後押しの際に
//! 鎖の外の窓を追い越すことがある（周囲の窓の状況に依る——並走走行で両方の結果が出た）。
//! 要件が縛るのは ⑴ 鎖の中の相対順（要件 1.1／1.2）と ⑵ **鎖の外どうし**の相対順
//! （要件 6.1／6.2）であって、その 2 群の間ではない。檻もそこまでしか固定しない。

use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DestroyWindow, GW_HWNDNEXT, GW_OWNER, GetTopWindow, GetWindow, HWND_BOTTOM,
    HWND_TOP, IsIconic, IsWindow, IsWindowVisible, SW_HIDE, SW_MINIMIZE, SW_RESTORE,
    SW_SHOWNOACTIVATE, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SetWindowPos, ShowWindow,
    WINDOW_EX_STYLE, WINDOW_STYLE, WS_EX_TOOLWINDOW, WS_POPUP,
};
use windows::core::{PCWSTR, w};

use super::{clear_window_owner, set_window_owner};

// ---------------------------------------------------------------------------
// 実窓（0x0・可視・トップレベル・本番と同じ拡張スタイル）
// ---------------------------------------------------------------------------

fn create_probe_window(title: PCWSTR) -> HWND {
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
        .expect("CreateWindowExW should create a probe window");
        let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
        hwnd
    }
}

fn destroy_all(windows: &[HWND]) {
    for hwnd in windows {
        // SAFETY: Win32 境界。自プロセスが生成した窓を破棄する。
        unsafe {
            let _ = DestroyWindow(*hwnd);
        }
    }
}

fn is_visible(hwnd: HWND) -> bool {
    // SAFETY: Win32 境界。読み取りのみ。
    unsafe { IsWindowVisible(hwnd) }.as_bool()
}

fn is_alive(hwnd: HWND) -> bool {
    // SAFETY: Win32 境界。読み取りのみ。
    unsafe { IsWindow(Some(hwnd)) }.as_bool()
}

fn is_minimized(hwnd: HWND) -> bool {
    // SAFETY: Win32 境界。読み取りのみ。
    unsafe { IsIconic(hwnd) }.as_bool()
}

fn owner_of(hwnd: HWND) -> Option<HWND> {
    // SAFETY: Win32 境界。読み取りのみ。
    unsafe { GetWindow(hwnd, GW_OWNER) }.ok()
}

// ---------------------------------------------------------------------------
// Z の読み取り
// ---------------------------------------------------------------------------

fn relative_z_order(windows: &[HWND]) -> Vec<HWND> {
    let mut result = Vec::new();
    // SAFETY: Win32 境界。デスクトップ配下の最前面窓を得る読み取り専用 API。
    let mut cursor = unsafe { GetTopWindow(None) }.ok();
    let mut steps = 0usize;
    while let Some(hwnd) = cursor {
        if hwnd.is_invalid() {
            break;
        }
        if windows.contains(&hwnd) && !result.contains(&hwnd) {
            result.push(hwnd);
            if result.len() == windows.len() {
                break;
            }
        }
        steps += 1;
        if steps > 100_000 {
            break;
        }
        // SAFETY: Win32 境界。窓ハンドルに対する読み取り専用の走査。
        cursor = unsafe { GetWindow(hwnd, GW_HWNDNEXT) }.ok();
    }
    result
}

/// 添字の列から、指定した添字だけを順序を保って抜き出す。
///
/// **鎖の外の窓の絶対位置は主張しない**ための助手（module doc「主張しないこと」を参照）。
fn only(shape: &[usize], keep: &[usize]) -> Vec<usize> {
    shape.iter().copied().filter(|i| keep.contains(i)).collect()
}

/// 組内の並びを添字の列（手前から順）で返す。
fn z_shape(set: &[HWND]) -> Vec<usize> {
    relative_z_order(set)
        .iter()
        .filter_map(|hwnd| set.iter().position(|w| w == hwnd))
        .collect()
}

/// 素の Z 指令（助走・後押し専用）。
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

fn place_at(hwnd: HWND, insert_after: HWND) {
    place_after(hwnd, insert_after);
}

/// 本設計が採る後押し——**鎖の先頭を 2 番目の直後へ差し直す**。
///
/// 参照するのはどちらも自分の窓であり、主張する関係は鎖が既に強制しているものと同じなので、
/// 鎖の外の窓は 1 つも動かない。`GW_HWNDPREV`（＝「いま自分の 1 つ手前にいる窓」）を挿入位置に
/// 使う形も同じ効果を持つが、**その窓が他プロセスのものでありうる**——読み取りと書き込みの
/// 間に消えると `SetWindowPos` が黙って失敗し、鎖が収まらない（full-suite の並走走行で実際に
/// 再現した）。よって自分の窓だけを参照するこちらを採る。
fn nudge_chain(chain: &[HWND]) {
    if chain.len() >= 2 {
        place_after(chain[0], chain[1]);
    }
}

/// 組を `order`（手前から順の添字）へ揃える助走。
fn arrange_z(set: &[HWND], order: &[usize]) {
    for pair in order.windows(2) {
        place_after(set[pair[1]], set[pair[0]]);
    }
}

/// 鎖を張る（`chain` は**手前から奥**の順。`chain[i]` は `chain[i+1]` に所有される）。
fn link_chain(chain: &[HWND]) {
    for pair in chain.windows(2) {
        set_window_owner(pair[0], pair[1]).expect("set_window_owner should succeed");
    }
}

/// owner を持つ窓だけを外す。
///
/// **owner を持たない窓へ `clear_window_owner` を当ててはならない**——実測 5 が示すとおり
/// `SetWindowLongPtrW(GWLP_HWNDPARENT, 0)` は元の owner が無いとき偽の失敗を返す。
fn unlink_all(windows: &[HWND]) {
    for hwnd in windows {
        if owner_of(*hwnd).is_some() {
            clear_window_owner(*hwnd).expect("clear_window_owner should succeed");
        }
    }
}

// ===========================================================================
// 実測 1（§11.6-2）: 表示中の窓の owner 張り替えは z 順へ即時反映されるか
// ===========================================================================

#[test]
fn owner_chain_probe_relink_on_visible_windows() {
    let set: Vec<HWND> = (0..4)
        .map(|_| create_probe_window(w!("owner-chain-probe/relink")))
        .collect();

    // 助走——宣言の逆順（手前から 3,2,1,0）。鎖が効けば 0,1,2,3 へ動くはず。
    arrange_z(&set, &[3, 2, 1, 0]);
    let before = z_shape(&set);

    // 鎖を張る（手前から 0,1,2,3 ＝ 0 が 1 に所有され…3 が根）。
    link_chain(&set);
    let owners: Vec<Option<usize>> = set
        .iter()
        .map(|h| owner_of(*h).and_then(|o| set.iter().position(|w| *w == o)))
        .collect();
    let after_link = z_shape(&set);

    // 後押し① 根を最背面へ落とす（1 回だけ）。
    place_at(set[3], HWND_BOTTOM);
    let after_nudge_root_bottom = z_shape(&set);

    // 後押し② 先頭を最前面へ持ち上げる（1 回だけ）。
    place_at(set[0], HWND_TOP);
    let after_nudge_head_top = z_shape(&set);

    // 攪乱——利用者の活性化に相当する形で最背面の窓を最前面へ持ち上げる。
    place_at(set[3], HWND_TOP);
    let after_disturb = z_shape(&set);

    eprintln!("[probe-1] before(reversed)        = {before:?}");
    eprintln!("[probe-1] owners(index)           = {owners:?}");
    eprintln!("[probe-1] after_link(no nudge)    = {after_link:?}");
    eprintln!("[probe-1] after_nudge_root_bottom = {after_nudge_root_bottom:?}");
    eprintln!("[probe-1] after_nudge_head_top    = {after_nudge_head_top:?}");
    eprintln!("[probe-1] after_disturb_root_top  = {after_disturb:?}");

    unlink_all(&set);
    let after_unlink = z_shape(&set);
    eprintln!("[probe-1] after_unlink            = {after_unlink:?}");

    destroy_all(&set);

    assert_eq!(before, vec![3, 2, 1, 0], "助走が宣言の逆順に揃っていること");
    assert_eq!(
        owners,
        vec![Some(1), Some(2), Some(3), None],
        "鎖は一直線（星形でも輪でもない）"
    );
    assert_eq!(
        after_link, before,
        "【実測】表示中の窓への owner 張り替えは、それだけでは重なりを 1 ミリも動かさない"
    );
    assert_eq!(
        after_nudge_root_bottom,
        vec![0, 1, 2, 3],
        "【実測】Z を伴う後押し 1 回で鎖全体が宣言順へ収まる"
    );
    assert_eq!(
        after_nudge_head_top,
        vec![0, 1, 2, 3],
        "収まった後は追加の後押しでも崩れない"
    );
    assert_eq!(
        after_disturb,
        vec![0, 1, 2, 3],
        "【実測・要件 14.3】最も奥の窓を最前面へ持ち上げても、OS が鎖の順を保つ"
    );
    assert_eq!(
        after_unlink,
        vec![0, 1, 2, 3],
        "【実測・要件 6】鎖を外しても並べ替えは起きない——束縛が消えるだけ"
    );
}

// ===========================================================================
// 実測 1b: 既に鎖が張られている状態から**別の順へ張り替える**
// ===========================================================================

#[test]
fn owner_chain_probe_rechain_to_a_different_order() {
    let set: Vec<HWND> = (0..4)
        .map(|_| create_probe_window(w!("owner-chain-probe/rechain")))
        .collect();

    link_chain(&set); // 手前から 0,1,2,3
    place_at(set[3], HWND_BOTTOM);
    let first = z_shape(&set);

    // 逆順の鎖へ張り替える（先に全部外してから張り直す＝途中で星形も輪も作らない）。
    unlink_all(&set);
    let after_unlink = z_shape(&set);
    let reversed: Vec<HWND> = set.iter().rev().copied().collect();
    link_chain(&reversed); // 手前から 3,2,1,0
    let after_relink_no_nudge = z_shape(&set);
    place_at(set[0], HWND_BOTTOM); // 新しい根（添字 0）を最背面へ
    let after_relink_nudge = z_shape(&set);

    eprintln!("[probe-1b] first_chain            = {first:?}");
    eprintln!("[probe-1b] after_unlink           = {after_unlink:?}");
    eprintln!("[probe-1b] rechain_no_nudge       = {after_relink_no_nudge:?}");
    eprintln!("[probe-1b] rechain_after_nudge    = {after_relink_nudge:?}");

    unlink_all(&set);
    destroy_all(&set);

    assert_eq!(first, vec![0, 1, 2, 3], "最初の鎖が成立していること");
    assert_eq!(after_unlink, first, "外した瞬間には並びは動かない");
    assert_eq!(
        after_relink_no_nudge, first,
        "張り替えだけでは動かない（実測 1 と同じ）"
    );
    assert_eq!(
        after_relink_nudge,
        vec![3, 2, 1, 0],
        "【実測】全部外してから逆順で張り直し、後押し 1 回で新しい順へ収まる"
    );
}

// ===========================================================================
// 実測 2（§11.6-3）: 副作用——最小化・非表示・破棄は鎖を伝うか
// ===========================================================================

#[test]
fn owner_chain_probe_minimize_and_hide_cascade() {
    // 鎖: front <- a <- b <- c :back （a は b に、b は c に所有される）
    let a = create_probe_window(w!("owner-chain-probe/casc-a"));
    let b = create_probe_window(w!("owner-chain-probe/casc-b"));
    let c = create_probe_window(w!("owner-chain-probe/casc-c"));
    let set = [a, b, c];
    link_chain(&set);

    let base = (is_visible(a), is_visible(b), is_visible(c));

    // ⑴ 根（最も奥・最上位の owner）を最小化する。
    // SAFETY: Win32 境界。自プロセスの窓の表示状態を変える。
    unsafe {
        let _ = ShowWindow(c, SW_MINIMIZE);
    }
    let after_min_root = (
        is_visible(a),
        is_visible(b),
        is_visible(c),
        is_minimized(c),
        is_minimized(a),
    );
    // SAFETY: Win32 境界。
    unsafe {
        let _ = ShowWindow(c, SW_RESTORE);
    }
    let after_restore_root = (is_visible(a), is_visible(b), is_visible(c));

    // ⑵ 中間の窓を最小化する。
    // SAFETY: Win32 境界。
    unsafe {
        let _ = ShowWindow(b, SW_MINIMIZE);
    }
    let after_min_mid = (is_visible(a), is_visible(b), is_visible(c));
    // SAFETY: Win32 境界。
    unsafe {
        let _ = ShowWindow(b, SW_RESTORE);
    }

    // ⑶ 根を非表示にする。
    // SAFETY: Win32 境界。
    unsafe {
        let _ = ShowWindow(c, SW_HIDE);
    }
    let after_hide_root = (is_visible(a), is_visible(b), is_visible(c));
    // SAFETY: Win32 境界。
    unsafe {
        let _ = ShowWindow(c, SW_SHOWNOACTIVATE);
    }

    eprintln!("[probe-2] base(a,b,c visible)          = {base:?}");
    eprintln!("[probe-2] after_min_root(a,b,c,ic,ia)  = {after_min_root:?}");
    eprintln!("[probe-2] after_restore_root(a,b,c)    = {after_restore_root:?}");
    eprintln!("[probe-2] after_min_mid(a,b,c)         = {after_min_mid:?}");
    eprintln!("[probe-2] after_hide_root(a,b,c)       = {after_hide_root:?}");

    unlink_all(&set);
    destroy_all(&set);

    assert_eq!(base, (true, true, true), "3 枚とも可視で始まること");
    assert_eq!(
        after_min_root,
        (false, false, true, true, false),
        "【実測・要件 11.5 の要注意点】所有者を最小化すると鎖の下流の窓がすべて不可視になる（隠れるのであって最小化されるのではない）"
    );
    assert_eq!(
        after_restore_root,
        (true, true, true),
        "【実測】復元で元に戻る＝連動は可逆"
    );
    assert_eq!(
        after_min_mid,
        (false, true, true),
        "【実測】連動は下流（手前側）へだけ伝う。上流の所有者は影響を受けない"
    );
    assert_eq!(
        after_hide_root,
        (true, true, false),
        "【実測】`SW_HIDE` は連動しない——伝うのは最小化だけ"
    );
}

#[test]
fn owner_chain_probe_destroy_cascade_and_neutralization() {
    // ⑴ 素のまま根を破棄する。
    let a = create_probe_window(w!("owner-chain-probe/dest-a"));
    let b = create_probe_window(w!("owner-chain-probe/dest-b"));
    let c = create_probe_window(w!("owner-chain-probe/dest-c"));
    link_chain(&[a, b, c]);
    // SAFETY: Win32 境界。
    unsafe {
        let _ = DestroyWindow(c);
    }
    let after_destroy_root = (is_alive(a), is_alive(b), is_alive(c));
    destroy_all(&[a, b]);

    // ⑵ 中間の窓を破棄する。
    let d = create_probe_window(w!("owner-chain-probe/dest-d"));
    let e = create_probe_window(w!("owner-chain-probe/dest-e"));
    let f = create_probe_window(w!("owner-chain-probe/dest-f"));
    link_chain(&[d, e, f]);
    // SAFETY: Win32 境界。
    unsafe {
        let _ = DestroyWindow(e);
    }
    let after_destroy_mid = (is_alive(d), is_alive(e), is_alive(f));
    let d_owner_after = owner_of(d).map(|o| o == f);
    destroy_all(&[d, f]);

    // ⑶ 先に鎖から外してから破棄する（ペア切離の雛形＝`zorder_pair_maintain.rs:286`）。
    let g = create_probe_window(w!("owner-chain-probe/dest-g"));
    let h = create_probe_window(w!("owner-chain-probe/dest-h"));
    let i = create_probe_window(w!("owner-chain-probe/dest-i"));
    link_chain(&[g, h, i]);
    clear_window_owner(g).expect("clear g");
    clear_window_owner(h).expect("clear h");
    // SAFETY: Win32 境界。
    unsafe {
        let _ = DestroyWindow(i);
    }
    let after_unlinked_destroy = (is_alive(g), is_alive(h), is_alive(i));
    destroy_all(&[g, h]);

    eprintln!("[probe-3] destroy_root(a,b,c alive)   = {after_destroy_root:?}");
    eprintln!("[probe-3] destroy_mid(d,e,f alive)    = {after_destroy_mid:?}");
    eprintln!("[probe-3] d_owner_is_f_after_mid_gone = {d_owner_after:?}");
    eprintln!("[probe-3] unlinked_destroy(g,h,i)     = {after_unlinked_destroy:?}");

    assert_eq!(
        after_destroy_root,
        (false, false, false),
        "【実測】鎖の根を破棄すると下流の窓がすべて道連れになる"
    );
    assert_eq!(
        after_destroy_mid,
        (false, false, true),
        "【実測】途中の窓の破棄は下流だけを巻き込み、上流の所有者は残る"
    );
    assert_eq!(
        after_unlinked_destroy,
        (true, true, false),
        "【実測・要件 7.2】先に鎖から外してから破棄すれば道連れは完全に消える"
    );
}

// ===========================================================================
// 実測 4（§11.6-4）: スプライス——鎖の途中へ窓を挿す／途中の窓を抜く
// ===========================================================================

#[test]
fn owner_chain_probe_splice_in_and_out() {
    // 既存の鎖: front <- a <- b <- c :back
    let a = create_probe_window(w!("owner-chain-probe/spl-a"));
    let b = create_probe_window(w!("owner-chain-probe/spl-b"));
    let c = create_probe_window(w!("owner-chain-probe/spl-c"));
    let x = create_probe_window(w!("owner-chain-probe/spl-x")); // 後から現れる窓
    let all = [a, b, c, x];

    // 助走は **自分の窓どうしの相対配置だけ**で組む（絶対帯指定はデスクトップ全体の状態に
    // 依存し、並走走行で非決定になる）。手前から a,b,c,x。
    arrange_z(&all, &[0, 1, 2, 3]);
    link_chain(&[a, b, c]);
    nudge_chain(&[a, b, c]);
    let established = z_shape(&all);

    // 挿入——b と c の間へ x を差す。既存 edge を 1 本だけ切り、2 本張る。
    clear_window_owner(b).expect("clear b before splice");
    let mid_state = z_shape(&all);
    set_window_owner(b, x).expect("b <- x");
    set_window_owner(x, c).expect("x <- c");
    let after_splice_no_nudge = z_shape(&all);
    nudge_chain(&[a, b, c]);
    let after_splice_nudge = z_shape(&all);

    // 抜去——x を鎖から外し、b を直接 c へ繋ぎ直す。
    clear_window_owner(b).expect("clear b");
    clear_window_owner(x).expect("clear x");
    set_window_owner(b, c).expect("b <- c");
    nudge_chain(&[a, b, c]);
    let after_unsplice = z_shape(&all);

    eprintln!("[probe-4] established(a,b,c,x)    = {established:?}");
    eprintln!("[probe-4] mid_state(edge 1 本切)  = {mid_state:?}");
    eprintln!("[probe-4] splice_no_nudge         = {after_splice_no_nudge:?}");
    eprintln!("[probe-4] splice_after_nudge      = {after_splice_nudge:?}");
    eprintln!("[probe-4] after_unsplice          = {after_unsplice:?}");
    // 抜けた窓 x（添字 3）が最後にどの絶対位置へ落ち着くかは、こちらが決めていない
    // ——本 spec は鎖の外の窓を動かさないからである（要件 6.1）。よって主張するのは
    // **残った鎖 a,b,c の相対順だけ**にする。x の絶対位置まで固定すると、こちらが
    // 保証していないものを檻に書くことになり、実際に並走走行で赤くなる。
    let chain_order_after_unsplice: Vec<usize> =
        after_unsplice.iter().copied().filter(|i| *i < 3).collect();

    unlink_all(&all);
    destroy_all(&all);

    // 鎖の外の窓 x（添字 3）の絶対位置は主張しない——鎖は塊として動き、鎖の外の窓を
    // 追い越すことがある。主張するのは鎖 a,b,c の相対順だけ。
    assert_eq!(
        only(&established, &[0, 1, 2]),
        vec![0, 1, 2],
        "助走: 鎖 a,b,c が手前から宣言順で並んでいること"
    );
    assert_eq!(
        only(&mid_state, &[0, 1, 2]),
        vec![0, 1, 2],
        "【実測・要件 8.2】edge を 1 本切った途中状態でも鎖の順は崩れない"
    );
    assert_eq!(
        only(&after_splice_no_nudge, &[0, 1, 2]),
        vec![0, 1, 2],
        "張り直しだけでは動かない"
    );
    assert_eq!(
        after_splice_nudge,
        vec![0, 1, 3, 2],
        "【実測・要件 7.1】後押し 1 回で a,b,x,c ＝ 差し込んだ鎖の順へ収まる"
    );
    assert_eq!(
        chain_order_after_unsplice,
        vec![0, 1, 2],
        "【実測・要件 7.2】x を抜いても残りの鎖 a,b,c は宣言順を保つ"
    );
}

// ===========================================================================
// 実測 5: `clear_window_owner` は owner を持たない窓に対して偽の失敗を返す
// ===========================================================================

#[test]
fn owner_chain_probe_clear_owner_on_unowned_window_reports_failure() {
    let lone = create_probe_window(w!("owner-chain-probe/lone"));
    let owner = create_probe_window(w!("owner-chain-probe/lone-owner"));

    // ⑴ owner を持たない窓を外そうとする。
    let unowned_result = clear_window_owner(lone).map_err(|e| e.code());

    // ⑵ 対照——owner を持つ窓を外すのは成功する。
    set_window_owner(lone, owner).expect("set_window_owner");
    let owned_result = clear_window_owner(lone).map_err(|e| e.code());

    // ⑶ 同じ窓を二度外そうとすると⑴と同じ失敗になる。
    let second_result = clear_window_owner(lone).map_err(|e| e.code());

    eprintln!("[probe-5] clear_on_unowned  = {unowned_result:?}");
    eprintln!("[probe-5] clear_on_owned    = {owned_result:?}");
    eprintln!("[probe-5] clear_twice       = {second_result:?}");

    destroy_all(&[lone, owner]);

    assert!(
        owned_result.is_ok(),
        "owner を持つ窓の切離しは成功しなければならない"
    );
    assert!(
        unowned_result.is_err(),
        "owner を持たない窓の切離しは失敗として返る（設計はこれを踏まない）"
    );
    assert_eq!(
        unowned_result, second_result,
        "二度目の切離しは一度目の未所有と同じ結果"
    );
}

// ===========================================================================
// 実測 6: 後押しの最小形——どの窓を、どこへ 1 回動かせば鎖が効くか
// ===========================================================================

#[test]
fn owner_chain_probe_minimal_nudge_variants() {
    // ⑴ 鎖の根を最背面へ。
    let s1: Vec<HWND> = (0..4)
        .map(|_| create_probe_window(w!("owner-chain-probe/nudge-root-bottom")))
        .collect();
    arrange_z(&s1, &[3, 2, 1, 0]);
    link_chain(&s1);
    place_at(s1[3], HWND_BOTTOM);
    let root_bottom = z_shape(&s1);

    // ⑵ 鎖の先頭を最前面へ。
    let s2: Vec<HWND> = (0..4)
        .map(|_| create_probe_window(w!("owner-chain-probe/nudge-head-top")))
        .collect();
    arrange_z(&s2, &[3, 2, 1, 0]);
    link_chain(&s2);
    place_at(s2[0], HWND_TOP);
    let head_top = z_shape(&s2);

    // ⑶ 中間の窓だけを動かす。
    let s3: Vec<HWND> = (0..4)
        .map(|_| create_probe_window(w!("owner-chain-probe/nudge-mid")))
        .collect();
    arrange_z(&s3, &[3, 2, 1, 0]);
    link_chain(&s3);
    place_at(s3[2], HWND_TOP);
    let mid_top = z_shape(&s3);

    // ⑷ 後押しをまったく出さない（対照）。
    let s4: Vec<HWND> = (0..4)
        .map(|_| create_probe_window(w!("owner-chain-probe/nudge-none")))
        .collect();
    arrange_z(&s4, &[3, 2, 1, 0]);
    link_chain(&s4);
    let no_nudge = z_shape(&s4);

    eprintln!("[probe-6] root->BOTTOM = {root_bottom:?}");
    eprintln!("[probe-6] head->TOP    = {head_top:?}");
    eprintln!("[probe-6] mid->TOP     = {mid_top:?}");
    eprintln!("[probe-6] no nudge     = {no_nudge:?}  (対照)");

    for s in [&s1, &s2, &s3, &s4] {
        unlink_all(s);
        destroy_all(s);
    }

    assert_eq!(root_bottom, vec![0, 1, 2, 3], "根への後押しで足りる");
    assert_eq!(head_top, vec![0, 1, 2, 3], "先頭への後押しでも足りる");
    assert_eq!(mid_top, vec![0, 1, 2, 3], "途中の窓への後押しでも足りる");
    assert_eq!(
        no_nudge,
        vec![3, 2, 1, 0],
        "【対照】後押しが無ければ何も起きない——上の 3 本は空虚ではない"
    );
}

// ===========================================================================
// 実測 7: 後押しが「グループ外の窓との関係」をどう動かすか（要件 6.2／11.1）
//
// 位置も寸法も Z も変えない後押し（`SWP_NOZORDER`）で足りるなら、グループ外の窓との
// 関係は 1 ミリも動かない。足りないなら、どの形が最も動かさないかを測る。
// ===========================================================================

/// 位置・寸法・Z のいずれも指定しない後押し（純粋な「触るだけ」）。
fn touch_only(hwnd: HWND) {
    use windows::Win32::UI::WindowsAndMessaging::SWP_NOZORDER;
    // SAFETY: Win32 境界。位置・寸法・Z のいずれも変更しない指令。
    unsafe {
        let _ = SetWindowPos(
            hwnd,
            None,
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_NOZORDER,
        );
    }
}

#[test]
fn owner_chain_probe_nudge_effect_on_outsiders() {
    // 組: 0,1,2 が鎖・3 が部外者。
    let chain: Vec<HWND> = (0..3)
        .map(|_| create_probe_window(w!("owner-chain-probe/out-chain")))
        .collect();
    let outsider = create_probe_window(w!("owner-chain-probe/out-outsider"));
    let mut all = chain.clone();
    all.push(outsider);

    // 助走——**自分の窓どうしの相対配置だけ**で組む。手前から 部外者(3), 2, 1, 0。
    arrange_z(&all, &[3, 2, 1, 0]);
    let start = z_shape(&all);

    link_chain(&chain);

    // ⑴ 触るだけの後押し（Z を指定しない）。
    touch_only(chain[0]);
    let after_touch_head = z_shape(&all);
    touch_only(chain[2]);
    let after_touch_root = z_shape(&all);

    // ⑵ Z を伴う後押し——根をその場（直前の窓の直後）へ差し直す。
    nudge_chain(&chain);
    let after_root_bottom = z_shape(&all);

    eprintln!("[probe-7] start(out=3 が最前面)   = {start:?}");
    eprintln!("[probe-7] after_touch_head        = {after_touch_head:?}");
    eprintln!("[probe-7] after_touch_root        = {after_touch_root:?}");
    eprintln!("[probe-7] after_root_bottom       = {after_root_bottom:?}");

    unlink_all(&chain);
    destroy_all(&all);

    assert_eq!(start, vec![3, 2, 1, 0], "助走: 部外者が最前面・鎖は逆順");
    assert_eq!(
        after_touch_head, start,
        "【実測】`SWP_NOZORDER` の後押しでは再整列が起きない——Z を伴う指令でなければならない"
    );
    assert_eq!(after_touch_root, start, "根を触っても同じ");
    assert_eq!(
        only(&after_root_bottom, &[0, 1, 2]),
        vec![0, 1, 2],
        "【実測】Z を伴う後押しなら鎖は宣言順へ収まる"
    );
}

// ===========================================================================
// 実測 9: **自分の窓だけ**を参照する後押し（実測 8 の弱点を潰す）
//
// 実測 8 の「その場への差し直し」は挿入位置に `GW_HWNDPREV`＝**他プロセスの窓**を渡しうる。
// その窓が読み取りと書き込みの間に消えると `SetWindowPos` が黙って失敗し、鎖が収まらない
// （full-suite の並走走行で実際に再現した）。
//
// 代案: 鎖の**先頭を 2 番目の直後へ**差し直す。参照するのはどちらも自分の窓であり、
// 主張する関係は鎖が既に強制しているものと同じなので、他の窓は 1 つも動かないはずである。
// ===========================================================================

#[test]
fn owner_chain_probe_nudge_referencing_only_our_own_windows() {
    let chain: Vec<HWND> = (0..3)
        .map(|_| create_probe_window(w!("owner-chain-probe/own-chain")))
        .collect();
    let front_outsider = create_probe_window(w!("owner-chain-probe/own-front"));
    let back_outsider = create_probe_window(w!("owner-chain-probe/own-back"));
    let mut all = chain.clone();
    all.push(front_outsider);
    all.push(back_outsider);

    // 手前から: front_outsider(3), 鎖の逆順 2,1,0, back_outsider(4)。
    arrange_z(&all, &[3, 2, 1, 0, 4]);
    let start = z_shape(&all);

    link_chain(&chain);

    // 後押し——先頭を 2 番目の直後へ。参照はどちらも自分の窓。
    place_after(chain[0], chain[1]);
    let after_own_nudge = z_shape(&all);

    eprintln!("[probe-9] start            = {start:?}");
    eprintln!("[probe-9] after_own_nudge  = {after_own_nudge:?}");

    unlink_all(&chain);
    destroy_all(&all);

    assert_eq!(
        start,
        vec![3, 2, 1, 0, 4],
        "助走: 部外者で前後を挟み鎖は逆順"
    );
    assert_eq!(
        only(&after_own_nudge, &[0, 1, 2]),
        vec![0, 1, 2],
        "【実測】自分の窓だけを参照する後押しでも鎖は宣言順へ収まる"
    );
    assert_eq!(
        only(&after_own_nudge, &[3, 4]),
        vec![3, 4],
        "【実測・要件 6.1／6.2】鎖の外どうしの相対順は変わらない"
    );
}

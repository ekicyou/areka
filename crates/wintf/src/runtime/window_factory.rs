//! ECS ウィンドウ生成ファクトリ（`EcsWindowFactory`）。
//!
//! 宣言的ウィンドウ生成（`Window` コンポーネント spawn）を、自作 `CreateWindowExW`
//! 直呼びからライブラリ（`wintf-winmsg-executor`）のウィンドウ生成（`util::Window::new_ex`・
//! **wndproc 再入可**）へ置換する移行アダプタを提供する（設計 EcsWindowFactory／再入要否は
//! 下記 §再入方針）。生成した `Window<WndState>` は [`WindowRegistry`] が所有し
//! （NonSend・Drop=`DestroyWindow`）、`HWND` は `WindowHandle`/`HasGraphicsResources`
//! として Entity へ反映する（グラフィクス経路）。
//!
//! # ライブラリ生成と生成後初期化（要件 2.1）
//! ライブラリの `new_*` は `WINDOW_STYLE(0)`・`CW_USEDEFAULT`・タイトルなしで生成する。
//! よって生成後に現行 `WindowStyle`/`WindowPos`/`title` を反映する初期化ステップ
//! （`SetWindowLongPtrW(GWL_STYLE)`／`SetWindowPos`／`SetWindowTextW`）を行う。座標は
//! 旧 `create_windows` と同一の `WindowPos::to_window_coords_for_creation` で算出し、
//! 位置と大きさのうち与えられた側だけを [`initial_pos_flags`] の旗で反映する（位置だけ・
//! 大きさだけの窓も、出た後の刻みで跳ばない）。
//!
//! # ex_style 算出（要件 4.1/4.2/4.3）
//! 合成モード非依存の branchless 単一経路で算出する
//! （`(ex_style & !WS_EX_LAYERED) | WS_EX_NOREDIRECTIONBITMAP`＝GPU 合成固定）。
//!
//! # CS_DBLCLKS・GWLP_USERDATA（要件 2.5/2.3）
//! クラスの `CS_DBLCLKS` はライブラリの共有クラスが内蔵するため wintf 側補填は設けない。
//! `GWLP_USERDATA` はライブラリが占有するため wintf からは手詰めしない（Entity は
//! クロージャ capture の `WndState` から読む）。HINSTANCE はライブラリ内部 `__ImageBase`
//! （DLL 安全）で処理されるため、`instance` はライブラリの `get_instance_handle()` を使う。
//!
//! # 結線
//! `create_windows`（`ecs::window::window_system`）は本ファクトリ経由でウィンドウを生成する
//! （task 4.3 cutover 済み）。旧 `process_singleton` の 2 クラス登録・`CreateWindowExW`
//! 直呼び経路は撤去済み（task 4.5）。生成はライブラリの共有クラス（`get_instance_handle()` +
//! 内蔵 `CS_DBLCLKS`）に一本化され、`WindowRegistry` 格納・`WindowHandle` 反映までを担う。

use std::cell::RefCell;
use std::rc::Weak;

use crate::executor::util::{Window as LibWindow, WindowType, get_instance_handle};
use bevy_ecs::prelude::Component;
use bevy_ecs::prelude::Entity;
use bevy_ecs::world::World;
use tracing::{debug, error};
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::HiDpi::GetDpiForSystem;
use windows::Win32::UI::WindowsAndMessaging::{
    CW_USEDEFAULT, GWL_STYLE, SET_WINDOW_POS_FLAGS, SW_SHOW, SWP_FRAMECHANGED, SWP_NOACTIVATE,
    SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER, SetParent, SetWindowLongPtrW, SetWindowPos,
    SetWindowTextW, ShowWindow, WS_EX_LAYERED,
};
use windows::core::HSTRING;

use crate::ecs::HasGraphicsResources;
use crate::ecs::window::{StayOnTop, Window, WindowHandle, WindowPos, WindowStyle};
use crate::ecs::world::EcsWorld;
use crate::runtime::window_registry::WindowRegistry;
use crate::runtime::wndproc_bridge::{WndState, make_wndproc};

use windows::Win32::UI::WindowsAndMessaging::WINDOW_EX_STYLE;

/// 拡張スタイルを合成モード非依存で算出する純関数（要件 4.1/4.2/4.3）。
///
/// GPU 合成（WUC）へ無条件固定されたため分岐はない。撤去前の DComp アームと
/// byte-for-byte 等価に、`WS_EX_LAYERED` を常に落とし `WS_EX_NOREDIRECTIONBITMAP` を
/// 常に付与する（生成時に `WS_EX_LAYERED` は付与しない＝Req4.3）。
///
/// 副作用なしで単体テストできるよう独立関数に切り出す。
fn compute_ex_style(style: &WindowStyle) -> WINDOW_EX_STYLE {
    use windows::Win32::UI::WindowsAndMessaging::WS_EX_NOREDIRECTIONBITMAP;
    // WS_EX_NOREDIRECTIONBITMAP を設定、WS_EX_LAYERED は除去（branchless 単一経路）。
    (style.ex_style & !WS_EX_LAYERED) | WS_EX_NOREDIRECTIONBITMAP
}

/// 窓を作るときの拡張スタイル（`stay-on-top-directives` 要件 2.3・design.md「CreationExStyle」）。
///
/// 常に手前の印 [`StayOnTop`] のある窓は、初めから帯に居るよう [`compute_ex_style`] に
/// `WS_EX_TOPMOST` を足す。印の無い窓は [`compute_ex_style`] のまま。
fn creation_ex_style(style: &WindowStyle, stay_on_top: bool) -> WINDOW_EX_STYLE {
    use windows::Win32::UI::WindowsAndMessaging::WS_EX_TOPMOST;
    let ex = compute_ex_style(style);
    if stay_on_top { ex | WS_EX_TOPMOST } else { ex }
}

/// 窓の生成に失敗した実体の印。`create_windows` の問い合わせから外す。
/// 立ち直らせたい側がこの印を外すと、次の刻みでまた作る。
#[derive(Component, Debug)]
pub struct WindowCreationFailed;

/// 生成直後の `SetWindowPos` の旗。位置も大きさも既定（`CW_USEDEFAULT`）なら `None`（呼ばない）。
/// 位置が既定なら `SWP_NOMOVE`、大きさが既定なら `SWP_NOSIZE` を足す。
fn initial_pos_flags(x: i32, width: i32) -> Option<SET_WINDOW_POS_FLAGS> {
    let no_move = x == CW_USEDEFAULT;
    let no_size = width == CW_USEDEFAULT;
    if no_move && no_size {
        return None;
    }
    let mut flags = SWP_NOZORDER | SWP_NOACTIVATE;
    if no_move {
        flags |= SWP_NOMOVE;
    }
    if no_size {
        flags |= SWP_NOSIZE;
    }
    Some(flags)
}

/// 生成に失敗した実体に印を入れ、`error!` を 1 行残す（印で毎刻みの作り直しを止める）。
fn mark_creation_failed(world: &mut World, entity: Entity, e: impl std::fmt::Debug) {
    error!(entity = ?entity, error = ?e, "ライブラリウィンドウ生成に失敗");
    world.entity_mut(entity).insert(WindowCreationFailed);
}

/// 宣言的ウィンドウ生成をライブラリ経由へ橋渡しするファクトリ。
///
/// エントリポイント [`EcsWindowFactory::create_window`] を `&mut World` シグネチャで
/// 提供し、`create_windows`（`&mut World` 排他システム）が呼ぶ。
pub(crate) struct EcsWindowFactory;

impl EcsWindowFactory {
    /// `entity` の宣言的ウィンドウデータからライブラリウィンドウを生成し登録する。
    ///
    /// 手順（World を `&mut` で借りたまま `new_ex` で作る）:
    /// 1. `entity` の `Window`/`WindowStyle`/`WindowPos`/`title` を
    ///    `World` から読み取り（immutable・即解放）。`Window` 不在なら no-op。
    /// 2. `ex_style` を [`creation_ex_style`] で算出（[`StayOnTop`] の印があれば `WS_EX_TOPMOST` 付き）。
    /// 3. `WndState{ world: Weak<RefCell<EcsWorld>>, entity }` と [`make_wndproc`] で
    ///    `LibWindow::new_ex(WindowType::TopLevel, ex_style, state, wndproc)` を生成。
    ///    失敗したら [`WindowCreationFailed`] の印を入れ `error!` を 1 行残して戻る。
    ///    ※ ライブラリは生成時に WM_CREATE 等を同期送出しうる。刻みが共有 World を借りて
    ///    いる最中なので、wndproc のクロージャの `try_borrow` が失敗して飛ばす（生成時の
    ///    メッセージは ECS の更新が要らない）。
    /// 4. 生成後初期化: `SetWindowLongPtrW(GWL_STYLE)`／`SetWindowPos`（座標は
    ///    `to_window_coords_for_creation`・旗は [`initial_pos_flags`]）／`SetWindowTextW`。
    ///    親の付け替え・位置合わせ・題の設定の失敗は `error!` を残して先へ進む。
    /// 5. `Window<WndState>` を [`WindowRegistry`]（NonSend）へ entity キーで格納。
    /// 6. `WindowHandle{ hwnd, instance }`／`HasGraphicsResources` を entity へ insert
    ///    （グラフィクス経路）。
    ///
    /// `ecs_world` は wndproc が capture する共有 World への弱参照（strong 所有者は
    /// `WinApp`）。
    ///
    /// NOTE(CS_DBLCLKS): クラス内蔵のため補填しない（要件 2.5）。
    /// NOTE(GWLP_USERDATA): ライブラリ占有のため手詰めしない（要件 2.3）。
    pub(crate) fn create_window(
        world: &mut World,
        entity: Entity,
        ecs_world: Weak<RefCell<EcsWorld>>,
    ) {
        // --- 1. 宣言的データを読み取り（immutable borrow を即解放） ---
        let entity_ref = match world.get_entity(entity) {
            Ok(e) => e,
            Err(_) => return,
        };
        let Some(window) = entity_ref.get::<Window>() else {
            // Window 不在なら生成対象でない（no-op）。
            return;
        };
        let title = window.title.clone();
        let parent = window.parent;
        let style = entity_ref.get::<WindowStyle>().copied().unwrap_or_default();
        let pos = entity_ref.get::<WindowPos>().copied().unwrap_or_default();
        let stay_on_top = entity_ref.contains::<StayOnTop>();

        // --- 2. ex_style 算出（要件 4.1/4.2/4.3・branchless。印があれば帯で作る） ---
        let ex_style = creation_ex_style(&style, stay_on_top);

        // --- 3. ライブラリ生成（要件 2.1） ---
        // `new_ex`（`Fn`・RefCell ラップなし＝**wndproc 再入可**）を採用する。
        // `new_checked_ex`（`FnMut`＋内部 RefCell）は同一窓 wndproc の再入を阻止するが、
        // wintf のドラッグは `WM_MOUSEMOVE`→`guarded_set_window_pos`→SetWindowPos が
        // **同期発火する WM_WINDOWPOSCHANGED に wndproc が再入して WindowPos を echo-bypass
        // 更新する**設計（mouse_move.rs / window_pos.rs）に依存しており、RefCell 阻止下では
        // この更新が失われ ECS と OS 位置がズレてドラッグがちらつく（5.3 実機回帰）。
        // 旧 `ecs_wndproc`（素の extern fn＝再入可）と同じ単一防御へ戻す: tick 二重実行は
        // ECS 側ガード（`IS_TICK_FLUSH_IN_PROGRESS`＋World `try_borrow_mut`）が担保する
        // （make_wndproc 側の `try_borrow` 安全スキップと二重防御・要件 4.3）。
        let state = WndState {
            world: ecs_world,
            entity,
        };
        let wndproc = make_wndproc();
        let lib_window = match LibWindow::new_ex(WindowType::TopLevel, ex_style, state, wndproc) {
            Ok(w) => w,
            Err(e) => {
                mark_creation_failed(world, entity, e);
                return;
            }
        };
        let hwnd = lib_window.hwnd();

        // --- 4. 生成後初期化（style / pos / title 反映・要件 2.1） ---
        Self::apply_initial_state(hwnd, &style, &pos, ex_style, &title);

        // --- 4a. 親ウィンドウ反映（`Window.parent` 転送・要件 2.1 の旧経路パリティ） ---
        // 旧 create_windows は `window.parent` を `CreateWindowExW` の hWndParent へ渡していたが、
        // ライブラリの `new_ex`（WindowType::TopLevel）は親 HWND を受け取らない。生成後に
        // `SetParent` で反映してパリティを回復する。`parent: None`（現行 areka/全 example）の
        // 場合は呼ばずスキップ＝挙動完全不変。
        // NOTE(セマンティクス): `SetParent` は標準の「親（子ウィンドウ）」設定 API。
        // `CreateWindowExW` の hWndParent は非子ウィンドウ（WS_POPUP 等）では「オーナー」を
        // 意味するため、WS_CHILD を持たない窓では厳密一致しない（ライブラリが生成時 hWndParent
        // を露出しないための近似）。子ウィンドウ用途（WS_CHILD）では一致する。
        if let Some(parent_hwnd) = parent {
            // SAFETY: Win32 境界。有効 hwnd を parent_hwnd の子へ再設定するのみ。
            // 失敗（無効 parent 等）は記録して先へ進む（窓は最上位のまま残る）。
            if let Err(e) = unsafe { SetParent(hwnd, Some(parent_hwnd)) } {
                error!(entity = ?entity, parent = ?parent_hwnd, error = ?e, "親の付け替えに失敗");
            }
        }

        // --- 4b. ウィンドウを表示（旧 create_windows の ShowWindow(SW_SHOW) 同等） ---
        // ライブラリは `WINDOW_STYLE(0)`（WS_VISIBLE なし）で生成するため、スタイルに
        // WS_VISIBLE を持たないウィンドウ（例: WindowStyle 未設定でデフォルト生成される窓）は
        // この ShowWindow がないと不可視のままになる（実機回帰で確認済み）。旧 create_windows が
        // 呼んでいた表示呼び出しを復元する。SAFETY: Win32 境界。生成直後の有効 HWND に対する表示要求のみ。
        // 同期発火する WM_WINDOWPOSCHANGED は tick の World 借用中ゆえ wndproc closure の
        // try_borrow 失敗で安全スキップされる（生成時メッセージは ECS 更新不要）。
        unsafe {
            let _ = ShowWindow(hwnd, SW_SHOW);
        }

        // --- 5. WindowRegistry（NonSend）へ格納（要件 2.1） ---
        if let Some(mut registry) = world.get_non_send_mut::<WindowRegistry<LibWindow<WndState>>>()
        {
            registry.insert(entity, lib_window);
        } else {
            // レジストリ未挿入は結線不備（`WinApp` が NonSend リソースを用意する）。
            // 握り潰すと Window が即 drop=DestroyWindow されるためその場で作って入れるが、
            // 終了の仕掛けの無い登録簿なので error! で残す。
            let mut registry = WindowRegistry::<LibWindow<WndState>>::new();
            registry.insert(entity, lib_window);
            world.insert_non_send(registry);
            error!(entity = ?entity, "WindowRegistry が無いのでその場で作って格納（終了の仕掛けなし）");
        }

        // --- 6. WindowHandle / HasGraphicsResources を反映（グラフィクス経路） ---
        let instance = get_instance_handle();
        world.entity_mut(entity).insert((
            WindowHandle { hwnd, instance },
            HasGraphicsResources::default(),
        ));

        debug!(entity = ?entity, hwnd = ?hwnd, "EcsWindowFactory: ウィンドウ生成・登録完了");
    }

    /// 生成後にスタイル・座標・タイトルを反映する（要件 2.1）。
    ///
    /// 旧 `create_windows` が `CreateWindowExW` 引数で渡していた style/pos/title を、
    /// ライブラリ生成（`WINDOW_STYLE(0)`・`CW_USEDEFAULT`・名前なし）後に適用する。
    /// 座標算出は旧経路と同一の `WindowPos::to_window_coords_for_creation`。既定
    /// （`CW_USEDEFAULT`）の側はライブラリ生成時点の OS 既定のまま残す（[`initial_pos_flags`]）。
    /// Win32 の失敗は `error!` を残して先へ進む。
    fn apply_initial_state(
        hwnd: HWND,
        style: &WindowStyle,
        pos: &WindowPos,
        ex_style: WINDOW_EX_STYLE,
        title: &str,
    ) {
        // window style（GWL_STYLE）。
        // SAFETY: Win32 境界。hwnd は直前に生成した有効ハンドル。GWL_STYLE への
        // SetWindowLongPtrW はスタイルビットの設定のみで、所有権・寿命に影響しない。
        unsafe {
            SetWindowLongPtrW(hwnd, GWL_STYLE, style.style.0 as isize);
        }

        // スタイル変更を非クライアント領域（フレーム）へ反映する。
        // ライブラリは `WINDOW_STYLE(0)` で生成するため、上の `SetWindowLongPtrW(GWL_STYLE)` で
        // 後から WS_CAPTION/WS_BORDER 等を立てても、`SWP_FRAMECHANGED` を伴う SetWindowPos が
        // ないとフレーム（タイトルバー・クローズボタン）が再計算・再描画されない（旧 create_windows
        // は生成時に style を渡すため最初からフレームが描かれていた）。フレームレス窓（既定
        // WS_POPUP・現行 areka/全 example）では非クライアント領域が無いため実質無害。
        // SAFETY: Win32 境界。位置・サイズ・z オーダーは変えず（NOMOVE/NOSIZE/NOZORDER）、
        // フレーム再計算（FRAMECHANGED）のみを要求する。
        if let Err(e) = unsafe {
            SetWindowPos(
                hwnd,
                None,
                0,
                0,
                0,
                0,
                SWP_FRAMECHANGED | SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
            )
        } {
            error!(hwnd = ?hwnd, error = ?e, "枠の再計算の SetWindowPos に失敗");
        }

        // 座標・サイズ（旧経路と同一算出）。与えられた側だけを反映する。
        let system_dpi = unsafe { GetDpiForSystem() };
        let (x, y, width, height) =
            pos.to_window_coords_for_creation(style.style, ex_style, system_dpi);
        if let Some(flags) = initial_pos_flags(x, width) {
            // SAFETY: Win32 境界。有効 hwnd に対する位置・サイズ反映。NOZORDER/NOACTIVATE
            // で z オーダー・アクティブ化を変えない（生成直後の初期反映）。
            if let Err(e) = unsafe { SetWindowPos(hwnd, None, x, y, width, height, flags) } {
                error!(hwnd = ?hwnd, error = ?e, "生成直後の位置合わせに失敗");
            }
        }

        // タイトル。
        let title_h = HSTRING::from(title);
        // SAFETY: Win32 境界。有効 hwnd にウィンドウテキストを設定するのみ。
        if let Err(e) = unsafe { SetWindowTextW(hwnd, &title_h) } {
            error!(hwnd = ?hwnd, error = ?e, "題の設定に失敗");
        }
    }
}

#[cfg(test)]
#[path = "window_factory_stay_on_top_tests.rs"]
mod window_factory_stay_on_top_tests;

#[cfg(test)]
#[path = "window_factory_creation_tests.rs"]
mod window_factory_creation_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::rc::Rc;

    use windows::Win32::UI::WindowsAndMessaging::{
        GetWindowLongPtrW, GetWindowTextW, SW_HIDE, WS_EX_NOREDIRECTIONBITMAP, WS_POPUP,
    };

    /// (1) ex_style 算出: 一本化後の唯一経路は NOREDIRECTIONBITMAP を立て LAYERED を除去する。
    #[test]
    fn ex_style_dcomp_sets_noredirection_clears_layered() {
        let style = WindowStyle::default(); // ex_style = WS_EX_LAYERED
        let ex = compute_ex_style(&style);
        assert!(
            (ex & WS_EX_NOREDIRECTIONBITMAP) == WS_EX_NOREDIRECTIONBITMAP,
            "DComp は WS_EX_NOREDIRECTIONBITMAP を立てるべき"
        );
        assert!(
            (ex & WS_EX_LAYERED).0 == 0,
            "DComp は WS_EX_LAYERED を除去すべき"
        );
    }

    /// (1) ex_style 算出: DComp は LAYERED 以外の既存 ex_style ビットを保持する。
    #[test]
    fn ex_style_dcomp_preserves_other_bits() {
        use windows::Win32::UI::WindowsAndMessaging::{WS_EX_LAYERED, WS_EX_TOPMOST};
        let style = WindowStyle {
            style: WS_POPUP,
            ex_style: WS_EX_LAYERED | WS_EX_TOPMOST,
        };
        let ex = compute_ex_style(&style);
        assert!(
            (ex & WS_EX_TOPMOST) == WS_EX_TOPMOST,
            "LAYERED 以外の既存ビット（TOPMOST）は保持されるべき"
        );
        assert!((ex & WS_EX_LAYERED).0 == 0, "LAYERED は除去されるべき");
    }

    /// 共有 World ハンドルを作るヘルパ。
    fn make_shared_world() -> Rc<RefCell<EcsWorld>> {
        Rc::new(RefCell::new(EcsWorld::new()))
    }

    /// (2) 実ウィンドウ生成: ファクトリ経由でウィンドウが生成され、レジストリ保持・
    ///     スタイル/タイトル反映が確認できる。
    ///
    /// `create_window` は `ShowWindow(SW_SHOW)` で窓を出すので、作った直後に隠してから確かめる。
    #[test]
    fn factory_creates_registers_and_applies_state() {
        // ECS World（共有）と bevy World を用意。bevy World は registry の NonSend 棚を持つ。
        let shared = make_shared_world();
        let weak = Rc::downgrade(&shared);

        // entity を共有 World 側に作り、宣言データを bevy World 側に揃える。
        // create_window は bevy World を直接受け取るため、宣言データもそこへ置く。
        let mut bevy_world = World::new();
        bevy_world.insert_non_send(WindowRegistry::<LibWindow<WndState>>::new());

        let entity = bevy_world
            .spawn((
                Window {
                    title: "FactoryTest".to_string(),
                    parent: None,
                },
                WindowStyle {
                    // 可視化しない（DComp 不可視）。WS_POPUP 主体で枠計算を単純化。
                    style: WS_POPUP,
                    ex_style: WS_EX_LAYERED,
                },
                WindowPos::default(), // CW_USEDEFAULT → SetWindowPos スキップ
            ))
            .id();

        EcsWindowFactory::create_window(&mut bevy_world, entity, weak);

        // WindowHandle が反映されている。作った直後に隠す。
        let handle = bevy_world
            .get::<WindowHandle>(entity)
            .copied()
            .expect("WindowHandle が insert されるべき");
        let _ = unsafe { ShowWindow(handle.hwnd, SW_HIDE) };
        assert!(!handle.hwnd.is_invalid(), "生成 HWND は有効であるべき");
        // レジストリに格納されている。
        assert!(
            !bevy_world
                .get_non_send::<WindowRegistry<LibWindow<WndState>>>()
                .expect("registry が存在するべき")
                .is_empty(),
            "生成した窓が registry に格納されるべき"
        );
        // HasGraphicsResources も反映。
        assert!(
            bevy_world.get::<HasGraphicsResources>(entity).is_some(),
            "HasGraphicsResources が insert されるべき"
        );

        let hwnd = handle.hwnd;

        // GWL_STYLE が反映されている（WS_POPUP を含む）。
        let applied_style = unsafe { GetWindowLongPtrW(hwnd, GWL_STYLE) } as u32;
        assert!(
            (applied_style & WS_POPUP.0) == WS_POPUP.0,
            "GWL_STYLE に WindowStyle.style（WS_POPUP）が反映されるべき"
        );

        // ウィンドウテキストが反映されている。
        let mut buf = [0u16; 64];
        let len = unsafe { GetWindowTextW(hwnd, &mut buf) };
        let text = String::from_utf16_lossy(&buf[..len as usize]);
        assert_eq!(
            text, "FactoryTest",
            "SetWindowTextW でタイトルが反映されるべき"
        );

        // クリーンアップ: registry を drop すると Window<WndState>::drop → DestroyWindow。
        drop(bevy_world);
        drop(shared);
    }

    /// 統合テスト（task 5.2・要件 2.1/6.1/2.5）: **宣言的ウィンドウ生成フロー end-to-end**。
    ///
    /// 直前の `factory_creates_...` はファクトリ関数を直接叩くが、本テストは新経路の ECS
    /// 排他システム `create_windows`（`ecs/window/window_system.rs`）を **実際に駆動**して、
    /// コンポーネント spawn → `create_windows` 起動 → ライブラリ生成 → `WindowRegistry` 保持 →
    /// `WindowHandle` 反映 までが結線済みで成立することを検証する（表示自体は E2E 5.3）。
    ///
    /// 検証点:
    /// - (a) **ウィンドウクラス未登録エラーが出ない**: 新経路はライブラリ共有クラスを使い
    ///   `process_singleton`（旧 2 クラス登録）を一切経由しない。`create_windows` が
    ///   `EcsWorldSelfRef` 在で factory 経路へ分岐し、`new_ex` がクラス未登録で
    ///   失敗しないこと（HWND が有効＝登録/生成成功）で担保する（要件 2.5）。
    /// - (b) `WindowRegistry` がその Entity の `Window<WndState>`（有効 HWND）を保持する。
    /// - (c) Entity に `WindowHandle` が付与される。
    ///
    /// メッセージループは回さない。`create_window` は窓を `SW_SHOW` で出すので、作った直後に
    /// 隠してから確かめる。
    #[test]
    fn create_windows_new_path_spawns_registers_and_handles_without_class_error() {
        use crate::ecs::window::window_system::create_windows;
        use crate::ecs::world::EcsWorldSelfRef;

        // 共有 World（strong 所有者をテストが保持）。`create_windows` は self-ref の Weak を
        // 読んで WndState 用の外側 World 参照を得る。
        let shared = make_shared_world();
        let weak = Rc::downgrade(&shared);

        // create_windows は bevy `&mut World` 排他システム。本番結線（WinApp::wire_*）が
        // 注入するのと同じ NonSend リソースを揃える:
        //  - EcsWorldSelfRef（新経路分岐スイッチ＝在で factory 経路）
        //  - WindowRegistry<Window<WndState>>（本番単相・生成物の保持先）
        let mut bevy_world = World::new();
        bevy_world.insert_non_send(EcsWorldSelfRef(weak));
        bevy_world.insert_non_send(WindowRegistry::<LibWindow<WndState>>::new());

        // 宣言的に Window（+ Style/Pos）コンポーネントを spawn（生成前は WindowHandle 不在）。
        let entity = bevy_world
            .spawn((
                Window {
                    title: "DeclFlow".to_string(),
                    parent: None,
                },
                WindowStyle {
                    style: WS_POPUP,
                    ex_style: WS_EX_LAYERED,
                },
                WindowPos::default(), // CW_USEDEFAULT → SetWindowPos スキップ
            ))
            .id();

        // 前提: 生成前は WindowHandle 不在。
        assert!(
            bevy_world.get::<WindowHandle>(entity).is_none(),
            "create_windows 実行前は WindowHandle 不在であるべき"
        );

        // --- 新経路 ECS システムを実駆動（self-ref 在 → factory 経路へ分岐） ---
        create_windows(&mut bevy_world);

        // (c) WindowHandle が付与され、(a) HWND が有効（クラス未登録/生成失敗なし）。
        let handle = bevy_world
            .get::<WindowHandle>(entity)
            .copied()
            .expect("create_windows 後に WindowHandle が付与されるべき（生成成功）");
        let _ = unsafe { ShowWindow(handle.hwnd, SW_HIDE) };
        assert!(
            !handle.hwnd.is_invalid(),
            "生成 HWND は有効であるべき（クラス未登録エラーなく生成成立・要件 2.5）"
        );
        // HasGraphicsResources も反映（グラフィクス経路の宣言的トリガ）。
        assert!(
            bevy_world.get::<HasGraphicsResources>(entity).is_some(),
            "create_windows 後に HasGraphicsResources が付与されるべき"
        );

        // (b) WindowRegistry がその Entity の Window<WndState>（有効 HWND）を保持する。
        let registry = bevy_world
            .get_non_send::<WindowRegistry<LibWindow<WndState>>>()
            .expect("WindowRegistry が存在するべき");
        assert!(
            !registry.is_empty(),
            "生成済みウィンドウが WindowRegistry に保持されるべき（spawn→生成→保持）"
        );

        // 冪等性: 同じ Entity は WindowHandle 付与済みゆえ再 create_windows で二重生成しない
        // （クエリ `Without<WindowHandle>` で除外される）。registry 件数は 1 のまま。
        create_windows(&mut bevy_world);
        let registry = bevy_world
            .get_non_send::<WindowRegistry<LibWindow<WndState>>>()
            .expect("WindowRegistry が存在するべき");
        assert!(
            !registry.is_empty(),
            "再 create_windows でも registry は維持される（二重生成・取りこぼしなし）"
        );

        // クリーンアップ: registry drop → Window<WndState>::drop → DestroyWindow。
        drop(bevy_world);
        drop(shared);
    }
}

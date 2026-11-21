//! ECS ウィンドウ生成ファクトリ（`EcsWindowFactory`）。
//!
//! 宣言的ウィンドウ生成（`Window` コンポーネント spawn）をライブラリ（`wintf-winmsg-executor`）の
//! `util::Window::new_ex`（wndproc 再入可）で行う。生成した `Window<WndState>` は
//! [`WindowRegistry`] が所有し（NonSend・Drop=`DestroyWindow`）、`HWND` は
//! `WindowHandle`/`HasGraphicsResources` として Entity へ反映する。
//!
//! # ライブラリ生成と生成後初期化
//! ライブラリの `new_*` は `WINDOW_STYLE(0)`・`CW_USEDEFAULT`・タイトルなしで生成するので、
//! 生成後に `WindowStyle`/`WindowPos`/`title` を反映する。位置と大きさのうち与えられた側だけを
//! [`initial_pos_flags`] の旗で反映する（位置だけ・大きさだけの窓も、出た後の刻みで跳ばない）。
//!
//! # CS_DBLCLKS・GWLP_USERDATA
//! `CS_DBLCLKS` はライブラリの共有クラスが内蔵するため wintf 側で補わない。
//! `GWLP_USERDATA` はライブラリが占有するため wintf からは書かない（Entity は
//! クロージャ capture の `WndState` から読む）。

use std::cell::RefCell;
use std::rc::Weak;

use crate::executor::util::{Window as LibWindow, WindowType};
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

/// 生成時の拡張スタイル。GPU 合成（WUC）固定なので、`WS_EX_LAYERED` を常に落とし
/// `WS_EX_NOREDIRECTIONBITMAP` を常に付ける。
fn compute_ex_style(style: &WindowStyle) -> WINDOW_EX_STYLE {
    use windows::Win32::UI::WindowsAndMessaging::WS_EX_NOREDIRECTIONBITMAP;
    (style.ex_style & !WS_EX_LAYERED) | WS_EX_NOREDIRECTIONBITMAP
}

/// 窓を作るときの拡張スタイル。常に手前の印 [`StayOnTop`] のある窓は、初めから帯に
/// 居るよう [`compute_ex_style`] に `WS_EX_TOPMOST` を足す。
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

/// 宣言的ウィンドウ生成をライブラリ経由へ橋渡しするファクトリ。`create_windows`
/// （`&mut World` 排他システム）が呼ぶ。
pub(crate) struct EcsWindowFactory;

impl EcsWindowFactory {
    /// `entity` の宣言的ウィンドウデータからライブラリウィンドウを生成し登録する。
    ///
    /// `Window` が無ければ何もしない。生成に失敗したら [`WindowCreationFailed`] の印を入れる。
    /// 生成後の親の付け替え・位置合わせ・題の設定の失敗は `error!` を残して先へ進む。
    ///
    /// ライブラリは生成時に WM_CREATE 等を同期送出しうるが、刻みが共有 World を借りている
    /// 最中なので wndproc のクロージャの `try_borrow` が失敗して飛ばす（生成時のメッセージは
    /// ECS の更新が要らない）。
    ///
    /// `ecs_world` は wndproc が capture する共有 World への弱参照（strong 所有者は
    /// `WinApp`）。
    pub(crate) fn create_window(
        world: &mut World,
        entity: Entity,
        ecs_world: Weak<RefCell<EcsWorld>>,
    ) {
        let entity_ref = match world.get_entity(entity) {
            Ok(e) => e,
            Err(_) => return,
        };
        let Some(window) = entity_ref.get::<Window>() else {
            return;
        };
        let title = window.title.clone();
        let parent = window.parent;
        let style = entity_ref.get::<WindowStyle>().copied().unwrap_or_default();
        let pos = entity_ref.get::<WindowPos>().copied().unwrap_or_default();
        let stay_on_top = entity_ref.contains::<StayOnTop>();

        let ex_style = creation_ex_style(&style, stay_on_top);

        // `new_checked_ex`（`FnMut`＋内部 RefCell）は同一窓の wndproc 再入を阻止するので使わない。
        // ドラッグは `SetWindowPos` が同期発火する WM_WINDOWPOSCHANGED に wndproc が再入して
        // WindowPos を echo-bypass 更新することに依存しており（mouse_move.rs / window_pos.rs）、
        // 阻止するとこの更新が失われ ECS と OS の位置がズレてドラッグがちらつく。刻みの二重実行は
        // ECS 側（`IS_TICK_FLUSH_IN_PROGRESS`＋World `try_borrow_mut`）が防ぐ。
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

        Self::apply_initial_state(hwnd, &style, &pos, ex_style, &title);

        // ライブラリの `new_ex`（WindowType::TopLevel）は親 HWND を受け取らないので、生成後に
        // `SetParent` で付ける。`CreateWindowExW` の hWndParent は WS_CHILD の無い窓では
        // 「オーナー」を意味するので、WS_CHILD を持たない窓では一致しない近似である。
        if let Some(parent_hwnd) = parent {
            // SAFETY: Win32 境界。有効 hwnd を parent_hwnd の子へ再設定するのみ。
            // 失敗（無効 parent 等）しても窓は最上位のまま残る。
            if let Err(e) = unsafe { SetParent(hwnd, Some(parent_hwnd)) } {
                error!(entity = ?entity, parent = ?parent_hwnd, error = ?e, "親の付け替えに失敗");
            }
        }

        // ライブラリは `WINDOW_STYLE(0)`（WS_VISIBLE なし）で生成するので、スタイルに
        // WS_VISIBLE を持たない窓はこれが無いと不可視のままになる。
        // SAFETY: Win32 境界。生成直後の有効 HWND に対する表示要求のみ。
        // 同期発火する WM_WINDOWPOSCHANGED は tick の World 借用中ゆえ wndproc closure の
        // try_borrow 失敗で安全スキップされる（生成時メッセージは ECS 更新不要）。
        unsafe {
            let _ = ShowWindow(hwnd, SW_SHOW);
        }

        if let Some(mut registry) = world.get_non_send_mut::<WindowRegistry<LibWindow<WndState>>>()
        {
            registry.insert(entity, lib_window);
        } else {
            // レジストリ未挿入は結線不備（`WinApp` が用意する）。握り潰すと Window が即
            // drop=DestroyWindow されるのでその場で作って入れるが、終了の仕掛けが無いので error! で残す。
            let mut registry = WindowRegistry::<LibWindow<WndState>>::new();
            registry.insert(entity, lib_window);
            world.insert_non_send(registry);
            error!(entity = ?entity, "WindowRegistry が無いのでその場で作って格納（終了の仕掛けなし）");
        }

        world
            .entity_mut(entity)
            .insert((WindowHandle { hwnd }, HasGraphicsResources::default()));

        debug!(entity = ?entity, hwnd = ?hwnd, "EcsWindowFactory: ウィンドウ生成・登録完了");
    }

    /// 生成後にスタイル・座標・タイトルを反映する。既定（`CW_USEDEFAULT`）の側は
    /// ライブラリ生成時点の OS 既定のまま残す。Win32 の失敗は `error!` を残して先へ進む。
    fn apply_initial_state(
        hwnd: HWND,
        style: &WindowStyle,
        pos: &WindowPos,
        ex_style: WINDOW_EX_STYLE,
        title: &str,
    ) {
        // SAFETY: Win32 境界。hwnd は直前に生成した有効ハンドル。GWL_STYLE への
        // SetWindowLongPtrW はスタイルビットの設定のみで、所有権・寿命に影響しない。
        unsafe {
            SetWindowLongPtrW(hwnd, GWL_STYLE, style.style.0 as isize);
        }

        // 生成後に GWL_STYLE で WS_CAPTION/WS_BORDER 等を立てても、`SWP_FRAMECHANGED` が
        // 無いとフレームが再計算・再描画されない。
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

    /// ex_style は NOREDIRECTIONBITMAP を立て LAYERED を除去する。
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

    /// LAYERED 以外の既存 ex_style ビットは残す。
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

    fn make_shared_world() -> Rc<RefCell<EcsWorld>> {
        Rc::new(RefCell::new(EcsWorld::new()))
    }

    /// ファクトリ経由の実ウィンドウ生成で、レジストリ保持・スタイル／タイトル反映が確かめられる。
    ///
    /// `create_window` は `ShowWindow(SW_SHOW)` で窓を出すので、作った直後に隠してから確かめる。
    #[test]
    fn factory_creates_registers_and_applies_state() {
        let shared = make_shared_world();
        let weak = Rc::downgrade(&shared);

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
                    // WS_POPUP で枠計算を単純にする。
                    style: WS_POPUP,
                    ex_style: WS_EX_LAYERED,
                },
                WindowPos::default(), // CW_USEDEFAULT → SetWindowPos スキップ
            ))
            .id();

        EcsWindowFactory::create_window(&mut bevy_world, entity, weak);

        let handle = bevy_world
            .get::<WindowHandle>(entity)
            .copied()
            .expect("WindowHandle が insert されるべき");
        let _ = unsafe { ShowWindow(handle.hwnd, SW_HIDE) };
        assert!(!handle.hwnd.is_invalid(), "生成 HWND は有効であるべき");
        assert!(
            !bevy_world
                .get_non_send::<WindowRegistry<LibWindow<WndState>>>()
                .expect("registry が存在するべき")
                .is_empty(),
            "生成した窓が registry に格納されるべき"
        );
        assert!(
            bevy_world.get::<HasGraphicsResources>(entity).is_some(),
            "HasGraphicsResources が insert されるべき"
        );

        let hwnd = handle.hwnd;

        let applied_style = unsafe { GetWindowLongPtrW(hwnd, GWL_STYLE) } as u32;
        assert!(
            (applied_style & WS_POPUP.0) == WS_POPUP.0,
            "GWL_STYLE に WindowStyle.style（WS_POPUP）が反映されるべき"
        );

        let mut buf = [0u16; 64];
        let len = unsafe { GetWindowTextW(hwnd, &mut buf) };
        let text = String::from_utf16_lossy(&buf[..len as usize]);
        assert_eq!(
            text, "FactoryTest",
            "SetWindowTextW でタイトルが反映されるべき"
        );

        // registry の drop で Window<WndState>::drop → DestroyWindow。
        drop(bevy_world);
        drop(shared);
    }

    /// 宣言的ウィンドウ生成の end-to-end。ECS 排他システム `create_windows` を実際に駆動し、
    /// spawn → ライブラリ生成 → `WindowRegistry` 保持 → `WindowHandle` 反映までを確かめる。
    ///
    /// HWND が有効であることで、ライブラリ共有クラスでの生成がクラス未登録で失敗しないことも見る。
    ///
    /// メッセージループは回さない。`create_window` は窓を `SW_SHOW` で出すので、作った直後に
    /// 隠してから確かめる。
    #[test]
    fn create_windows_new_path_spawns_registers_and_handles_without_class_error() {
        use crate::ecs::window::window_system::create_windows;
        use crate::ecs::world::EcsWorldSelfRef;
        use bevy_ecs::system::RunSystemOnce;

        // strong 所有者はテストが持つ。`create_windows` は self-ref の Weak から WndState 用の
        // World 参照を得る。
        let shared = make_shared_world();
        let weak = Rc::downgrade(&shared);

        // 本番結線（WinApp::wire_*）が注入するのと同じ NonSend リソースを揃える。
        let mut bevy_world = World::new();
        bevy_world.insert_non_send(EcsWorldSelfRef(weak));
        bevy_world.insert_non_send(WindowRegistry::<LibWindow<WndState>>::new());

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

        assert!(
            bevy_world.get::<WindowHandle>(entity).is_none(),
            "create_windows 実行前は WindowHandle 不在であるべき"
        );

        bevy_world.run_system_once(create_windows).unwrap();

        let handle = bevy_world
            .get::<WindowHandle>(entity)
            .copied()
            .expect("create_windows 後に WindowHandle が付与されるべき（生成成功）");
        let _ = unsafe { ShowWindow(handle.hwnd, SW_HIDE) };
        assert!(
            !handle.hwnd.is_invalid(),
            "生成 HWND は有効であるべき（クラス未登録エラーなく生成成立・要件 2.5）"
        );
        assert!(
            bevy_world.get::<HasGraphicsResources>(entity).is_some(),
            "create_windows 後に HasGraphicsResources が付与されるべき"
        );

        let registry = bevy_world
            .get_non_send::<WindowRegistry<LibWindow<WndState>>>()
            .expect("WindowRegistry が存在するべき");
        assert!(
            !registry.is_empty(),
            "生成済みウィンドウが WindowRegistry に保持されるべき（spawn→生成→保持）"
        );

        // registry は Entity を鍵に上書きするので二重生成は件数では見えない——HWND が
        // 替わらないことで見る。
        bevy_world.run_system_once(create_windows).unwrap();
        assert_eq!(
            bevy_world.get::<WindowHandle>(entity).map(|h| h.hwnd),
            Some(handle.hwnd),
            "再 create_windows で同じ Entity に窓を作り直さない（二重生成なし）"
        );

        // registry の drop で Window<WndState>::drop → DestroyWindow。
        drop(bevy_world);
        drop(shared);
    }
}

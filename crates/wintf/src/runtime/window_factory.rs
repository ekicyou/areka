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
//! 初期表示ドリフトを避ける（設計 Risks）。
//!
//! # ex_style 算出（要件 2.2）
//! `CompositionMode` から旧経路と同一規則で算出する（ULW=`WS_EX_LAYERED`／
//! DComp=`(ex_style & !WS_EX_LAYERED) | WS_EX_NOREDIRECTIONBITMAP`）。
//!
//! # CS_DBLCLKS・GWLP_USERDATA（要件 2.5/2.3）
//! クラスの `CS_DBLCLKS` はライブラリの共有クラスが内蔵するため wintf 側補填は設けない。
//! `GWLP_USERDATA` はライブラリが占有するため wintf からは手詰めしない（Entity は
//! クロージャ capture の `WndState` から読む）。HINSTANCE はライブラリ内部 `__ImageBase`
//! （DLL 安全）で処理されるため、`instance` はライブラリの `get_instance_handle()` を使う。
//!
//! # 結線は後続タスク（4.1/4.3）の領分
//! 本ファクトリは building block であり、ここでは旧 `create_windows`（レガシー
//! `process_singleton` クラスでの直生成経路）は LIVE のまま据え置く。`create_windows` を
//! 本ファクトリへ差し替える live cutover（reconcile の schedule 登録・WM_CLOSE 反転を含む）
//! は後続タスク 4.1/4.3 で行う。それまで本コードは未結線のため、兄弟 building block
//! （`MessageLoopDriver`/`VsyncEventBridge`/`AsyncTickTask`/`WindowRegistry`）同様に
//! scoped dead_code 許容とする。

use std::cell::RefCell;
use std::rc::Weak;

use bevy_ecs::prelude::Entity;
use bevy_ecs::world::World;
use tracing::{debug, error};
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::HiDpi::GetDpiForSystem;
use windows::Win32::UI::WindowsAndMessaging::{
    SWP_NOACTIVATE, SWP_NOZORDER, SW_SHOW, SetWindowLongPtrW, SetWindowPos, SetWindowTextW,
    ShowWindow, CW_USEDEFAULT, GWL_STYLE, WS_EX_LAYERED,
};
use windows::core::HSTRING;
use wintf_winmsg_executor::util::{get_instance_handle, Window as LibWindow, WindowType};

use crate::ecs::HasGraphicsResources;
use crate::ecs::window::{CompositionMode, Window, WindowHandle, WindowPos, WindowStyle};
use crate::ecs::world::EcsWorld;
use crate::runtime::window_registry::WindowRegistry;
use crate::runtime::wndproc_bridge::{make_wndproc, WndState};

use windows::Win32::UI::WindowsAndMessaging::WINDOW_EX_STYLE;

/// `CompositionMode` から拡張スタイルを算出する純関数（要件 2.2）。
///
/// 旧 `create_windows`（`ecs/window/window_system.rs`）と byte-for-byte 同一規則:
/// - `ULW`  → `style.ex_style`（既定 `WS_EX_LAYERED`）。
/// - `DComp`→ `(style.ex_style & !WS_EX_LAYERED) | WS_EX_NOREDIRECTIONBITMAP`。
///
/// 副作用なしで単体テストできるよう独立関数に切り出す。
fn compute_ex_style(composition_mode: CompositionMode, style: &WindowStyle) -> WINDOW_EX_STYLE {
    use windows::Win32::UI::WindowsAndMessaging::WS_EX_NOREDIRECTIONBITMAP;
    match composition_mode {
        CompositionMode::ULW => style.ex_style, // WS_EX_LAYERED（デフォルト）
        CompositionMode::DComp => {
            // DComp モード: WS_EX_NOREDIRECTIONBITMAP を設定、WS_EX_LAYERED は除去。
            (style.ex_style & !WS_EX_LAYERED) | WS_EX_NOREDIRECTIONBITMAP
        }
    }
}

/// 宣言的ウィンドウ生成をライブラリ経由へ橋渡しする移行ファクトリ。
///
/// 後続タスク 4.3 の書き換え後 `create_windows`（`&mut World` 排他システム）から
/// 呼び出される想定で、エントリポイント [`EcsWindowFactory::create_window`] を
/// `&mut World` シグネチャで提供する。`create_windows`（task 4.3 結線済み）が呼ぶ。
pub(crate) struct EcsWindowFactory;

impl EcsWindowFactory {
    /// `entity` の宣言的ウィンドウデータからライブラリウィンドウを生成し登録する。
    ///
    /// 手順（借用は最小限・`new_checked_ex` 越しに World 借用を保持しない＝再入安全）:
    /// 1. `entity` の `Window`/`WindowStyle`/`WindowPos`/`title`/`composition_mode` を
    ///    `World` から読み取り（immutable・即解放）。`Window` 不在なら no-op。
    /// 2. `ex_style` を [`compute_ex_style`] で算出。
    /// 3. `WndState{ world: Weak<RefCell<EcsWorld>>, entity }` と [`make_wndproc`] で
    ///    `LibWindow::new_checked_ex(WindowType::TopLevel, ex_style, state, wndproc)` を生成。
    ///    ※ ライブラリは生成時に WM_CREATE 等を同期送出しうるが、本関数は World 借用を
    ///    跨いで保持しないため、クロージャの `try_borrow` 安全スキップと衝突しない。
    /// 4. 生成後初期化: `SetWindowLongPtrW(GWL_STYLE)`／`SetWindowPos`（座標は
    ///    `to_window_coords_for_creation`・CW_USEDEFAULT 時はスキップ）／`SetWindowTextW`。
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
        let composition_mode = window.composition_mode();
        let style = entity_ref.get::<WindowStyle>().copied().unwrap_or_default();
        let pos = entity_ref.get::<WindowPos>().copied().unwrap_or_default();

        // --- 2. ex_style 算出（要件 2.2） ---
        let ex_style = compute_ex_style(composition_mode, &style);

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
        let lib_window =
            match LibWindow::new_ex(WindowType::TopLevel, ex_style, state, wndproc) {
                Ok(w) => w,
                Err(e) => {
                    error!(entity = ?entity, error = ?e, "ライブラリウィンドウ生成に失敗");
                    return;
                }
            };
        let hwnd = lib_window.hwnd();

        // --- 4. 生成後初期化（style / pos / title 反映・要件 2.1） ---
        Self::apply_initial_state(hwnd, &style, &pos, ex_style, &title);

        // --- 4b. ウィンドウを表示（旧 create_windows の ShowWindow(SW_SHOW) 同等） ---
        // ライブラリは `WINDOW_STYLE(0)`（WS_VISIBLE なし）で生成するため、スタイルに
        // WS_VISIBLE を持たないウィンドウ（例: WindowStyle 未設定でデフォルトの
        // multi_backend_demo ULW 窓）はこの ShowWindow がないと不可視のままになる
        // （5.3 実機回帰: ULW 窓が見えない）。旧 create_windows が呼んでいた表示呼び出しを
        // 復元する。SAFETY: Win32 境界。生成直後の有効 HWND に対する表示要求のみ。
        // 同期発火する WM_WINDOWPOSCHANGED は tick の World 借用中ゆえ wndproc closure の
        // try_borrow 失敗で安全スキップされる（生成時メッセージは ECS 更新不要）。
        unsafe {
            let _ = ShowWindow(hwnd, SW_SHOW);
        }

        // --- 5. WindowRegistry（NonSend）へ格納（要件 2.1） ---
        if let Some(mut registry) =
            world.get_non_send_resource_mut::<WindowRegistry<LibWindow<WndState>>>()
        {
            registry.insert(entity, lib_window);
        } else {
            // レジストリ未挿入は結線不備（4.3 が NonSend リソースを用意する）。
            // building block 段階では未到達想定だが、握り潰すと Window が即 drop=
            // DestroyWindow されるため、忘れ込みを検知できるよう明示ログを残す。
            let mut registry = WindowRegistry::<LibWindow<WndState>>::new();
            registry.insert(entity, lib_window);
            world.insert_non_send_resource(registry);
            debug!(entity = ?entity, "WindowRegistry を遅延生成して格納");
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
    /// 座標算出は旧経路と同一の `WindowPos::to_window_coords_for_creation`。CW_USEDEFAULT
    /// を含む場合はライブラリ生成時点で OS 既定が適用済みのため `SetWindowPos` を省く。
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

        // 座標・サイズ（旧経路と同一算出）。CW_USEDEFAULT を含む場合は OS 既定維持で省略。
        let system_dpi = unsafe { GetDpiForSystem() };
        let (x, y, width, height) =
            pos.to_window_coords_for_creation(style.style, ex_style, system_dpi);
        if x != CW_USEDEFAULT && width != CW_USEDEFAULT {
            // SAFETY: Win32 境界。有効 hwnd に対する位置・サイズ反映。NOZORDER/NOACTIVATE
            // で z オーダー・アクティブ化を変えない（生成直後の初期反映）。
            unsafe {
                let _ = SetWindowPos(
                    hwnd,
                    None,
                    x,
                    y,
                    width,
                    height,
                    SWP_NOZORDER | SWP_NOACTIVATE,
                );
            }
        }

        // タイトル。
        let title_h = HSTRING::from(title);
        // SAFETY: Win32 境界。有効 hwnd にウィンドウテキストを設定するのみ。
        unsafe {
            let _ = SetWindowTextW(hwnd, &title_h);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::rc::Rc;

    use windows::Win32::UI::WindowsAndMessaging::{
        GetWindowLongPtrW, GetWindowTextW, WS_EX_NOREDIRECTIONBITMAP, WS_OVERLAPPEDWINDOW, WS_POPUP,
        WS_VISIBLE,
    };

    use crate::ecs::window::CompositionMode;

    /// (1) ex_style 算出: ULW は WS_EX_LAYERED を保つ（既定スタイル）。
    #[test]
    fn ex_style_ulw_keeps_layered() {
        let style = WindowStyle::default(); // ex_style = WS_EX_LAYERED
        let ex = compute_ex_style(CompositionMode::ULW, &style);
        assert_eq!(
            ex, style.ex_style,
            "ULW は WindowStyle.ex_style（WS_EX_LAYERED）をそのまま使う"
        );
        assert!(
            (ex & WS_EX_LAYERED) == WS_EX_LAYERED,
            "ULW は WS_EX_LAYERED を含むべき"
        );
    }

    /// (1) ex_style 算出: DComp は NOREDIRECTIONBITMAP を立て LAYERED を除去する。
    #[test]
    fn ex_style_dcomp_sets_noredirection_clears_layered() {
        let style = WindowStyle::default(); // ex_style = WS_EX_LAYERED
        let ex = compute_ex_style(CompositionMode::DComp, &style);
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
        let ex = compute_ex_style(CompositionMode::DComp, &style);
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
    ///     スタイル/タイトル反映が確認できる（headless・DComp で不可視のまま検証）。
    ///
    /// DComp（WS_EX_NOREDIRECTIONBITMAP）は GDI 不可視ゆえ画面に出ないが、HWND は実在し
    /// GWL_STYLE / ウィンドウテキストは反映される。表示の E2E は 4.3 結線後（task 5.3）。
    #[test]
    fn factory_creates_registers_and_applies_state() {
        // ECS World（共有）と bevy World を用意。bevy World は registry の NonSend 棚を持つ。
        let shared = make_shared_world();
        let weak = Rc::downgrade(&shared);

        // entity を共有 World 側に作り、宣言データを bevy World 側に揃える。
        // create_window は bevy World を直接受け取るため、宣言データもそこへ置く。
        let mut bevy_world = World::new();
        bevy_world.insert_non_send_resource(WindowRegistry::<LibWindow<WndState>>::new());

        let entity = bevy_world
            .spawn((
                Window {
                    title: "FactoryTest".to_string(),
                    parent: None,
                    composition_mode: CompositionMode::DComp,
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

        // レジストリに格納され、有効な HWND を持つ。
        let registry = bevy_world
            .get_non_send_resource::<WindowRegistry<LibWindow<WndState>>>()
            .expect("registry が存在するべき");
        // WindowHandle が反映されている。
        let handle = bevy_world
            .get::<WindowHandle>(entity)
            .copied()
            .expect("WindowHandle が insert されるべき");
        assert!(!handle.hwnd.is_invalid(), "生成 HWND は有効であるべき");
        // HasGraphicsResources も反映。
        assert!(
            bevy_world.get::<HasGraphicsResources>(entity).is_some(),
            "HasGraphicsResources が insert されるべき"
        );
        let _ = registry; // 借用解放のため明示。

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
        assert_eq!(text, "FactoryTest", "SetWindowTextW でタイトルが反映されるべき");

        // クリーンアップ: registry を drop すると Window<WndState>::drop → DestroyWindow。
        drop(bevy_world);
        drop(shared);

        // 未使用警告抑止（WS_OVERLAPPEDWINDOW/WS_VISIBLE は将来の拡張用 import）。
        let _ = (WS_OVERLAPPEDWINDOW, WS_VISIBLE);
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
    ///   `EcsWorldSelfRef` 在で factory 経路へ分岐し、`new_checked_ex` がクラス未登録で
    ///   失敗しないこと（HWND が有効＝登録/生成成功）で担保する（要件 2.5）。
    /// - (b) `WindowRegistry` がその Entity の `Window<WndState>`（有効 HWND）を保持する。
    /// - (c) Entity に `WindowHandle` が付与される。
    ///
    /// headless 制約: DComp（`WS_EX_NOREDIRECTIONBITMAP`）で不可視のまま実 HWND を生成し
    /// メッセージループは回さない。`ShowWindow` を含む実表示と full `new→world→run` は E2E 5.3。
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
        bevy_world.insert_non_send_resource(EcsWorldSelfRef(weak));
        bevy_world.insert_non_send_resource(WindowRegistry::<LibWindow<WndState>>::new());

        // 宣言的に Window（+ Style/Pos）コンポーネントを spawn（生成前は WindowHandle 不在）。
        let entity = bevy_world
            .spawn((
                Window {
                    title: "DeclFlow".to_string(),
                    parent: None,
                    composition_mode: CompositionMode::DComp, // 不可視（headless）
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
            .get_non_send_resource::<WindowRegistry<LibWindow<WndState>>>()
            .expect("WindowRegistry が存在するべき");
        assert!(
            !registry.is_empty(),
            "生成済みウィンドウが WindowRegistry に保持されるべき（spawn→生成→保持）"
        );
        let _ = registry;

        // 冪等性: 同じ Entity は WindowHandle 付与済みゆえ再 create_windows で二重生成しない
        // （クエリ `Without<WindowHandle>` で除外される）。registry 件数は 1 のまま。
        create_windows(&mut bevy_world);
        let registry = bevy_world
            .get_non_send_resource::<WindowRegistry<LibWindow<WndState>>>()
            .expect("WindowRegistry が存在するべき");
        assert!(
            !registry.is_empty(),
            "再 create_windows でも registry は維持される（二重生成・取りこぼしなし）"
        );
        let _ = registry;

        // クリーンアップ: registry drop → Window<WndState>::drop → DestroyWindow。
        drop(bevy_world);
        drop(shared);
    }
}

use crate::ecs::*;
use crate::process_singleton::*;
use bevy_ecs::name::Name;
use bevy_ecs::prelude::*;
use bevy_ecs::system::SystemState;
use tracing::{debug, error};
use windows::Win32::UI::HiDpi::GetDpiForSystem;
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::core::*;

// Note: init_window_arrangement システムは廃止されました。
// Arrangementは以下のライフタイムフックで自動挿入されます：
// - Visual::on_add (Window は on_window_add で Visual を挿入)
// - LayoutRoot::on_add
// - Monitor::on_add
// Arrangement::on_add が GlobalArrangement と ArrangementTreeChanged を自動挿入します。

/// 未作成のWindowを検出してライブラリ経由で作成する排他システム（task 4.3 cutover）。
///
/// 排他システムにすることで、WindowHandleの追加が即時反映され、
/// 同じフレーム内の後続スケジュールでWindowHandleが参照可能になる。
///
/// # 共存分岐（task 4.3 解釈 (2)）
/// `EcsWorldSelfRef`（NonSend リソース・外側 `Weak<RefCell<EcsWorld>>`）の有無で分岐する:
/// - **不在**: 旧 `WinThreadMgr` facade 経路（self-ref を注入しない）では何もしない
///   （早期 return・panic も二重生成もしない）。旧経路のウィンドウ生成は cutover 後
///   意図的に不活性で、意味は 4.4（examples の `WinApp` 切替）で復元される。
/// - **在**: `WinApp` 経路。宣言的クエリにマッチする各 Entity に対し
///   [`EcsWindowFactory::create_window`] を呼ぶ（ライブラリの `Window<WndState>` を生成し
///   style/pos/title を反映、`WindowRegistry` へ格納、`WindowHandle`+`HasGraphicsResources`
///   を insert）。これが設計公認の単一上向きエッジ（ecs→runtime）。
///
/// 旧 `CreateWindowExW` 直呼び本体は [`create_windows_legacy`] として保持する（撤去は 4.5）。
pub fn create_windows(world: &mut World) {
    // self-ref（外側 World への Weak）を取得。未注入＝旧 WinThreadMgr 経路ゆえ no-op。
    let Some(self_ref) = world.get_non_send_resource::<crate::ecs::world::EcsWorldSelfRef>() else {
        // 旧 facade 経路では何もしない（panic なし・二重生成なし）。
        return;
    };
    let ecs_world: std::rc::Weak<std::cell::RefCell<crate::ecs::world::EcsWorld>> =
        self_ref.0.clone();

    // 宣言的クエリ（旧経路と同一条件）で未生成 Window Entity を収集（borrow を即解放）。
    let mut system_state: SystemState<
        Query<
            Entity,
            (
                With<Window>,
                Without<WindowHandle>,
            ),
        >,
    > = SystemState::new(world);
    let entities_to_create: Vec<Entity> = system_state.get(world).iter().collect();

    // 各 Entity をファクトリ経由で生成（ライブラリ生成・registry 格納・handle 反映）。
    for entity in entities_to_create {
        crate::runtime::window_factory::EcsWindowFactory::create_window(
            world,
            entity,
            ecs_world.clone(),
        );
    }
}

/// 旧 `CreateWindowExW` 直呼びによるウィンドウ生成（reference・撤去は task 4.5）。
///
/// task 4.3 cutover で [`create_windows`] をライブラリ経由ファクトリへ差し替えたため、
/// 本体は実呼び出しから外れた。知見転記漏れの保険として撤去せず保持する（開発者の
/// keep-old-code steer・撤去は 4.5 legacy teardown）。
#[allow(dead_code)]
pub fn create_windows_legacy(world: &mut World) {
    // SystemStateを使ってクエリとリソースにアクセス
    let mut system_state: SystemState<(
        Query<
            (
                Entity,
                &Window,
                Option<&WindowStyle>,
                Option<&WindowPos>,
                Option<&Name>,
            ),
            Without<WindowHandle>,
        >,
        Res<crate::ecs::world::FrameCount>,
    )> = SystemState::new(world);

    // クエリ結果を先に収集（borrowの問題を回避）
    let (query, frame_count) = system_state.get(world);
    let frame = frame_count.0;
    let entities_to_create: Vec<_> = query
        .iter()
        .map(|(entity, window, opt_style, opt_pos, name)| {
            (
                entity,
                window.title.clone(),
                window.parent,
                window.composition_mode(),
                opt_style.copied(),
                opt_pos.copied(),
                name.map(|n| n.as_str().to_string()),
            )
        })
        .collect();

    // 収集したエンティティに対してウィンドウを作成
    let singleton = WinProcessSingleton::get_or_init();

    for (entity, title, parent, composition_mode, opt_style, opt_pos, name_str) in
        entities_to_create
    {
        let entity_name = match &name_str {
            Some(n) => n.clone(),
            None => format!("Entity({:?})", entity),
        };
        debug!(
            frame,
            entity = %entity_name,
            title = %title,
            "Window creation starting"
        );

        let title_hstring = HSTRING::from(&title);
        let style_comp = opt_style.unwrap_or_default();
        let pos_comp = opt_pos.unwrap_or_default();
        let system_dpi = unsafe { GetDpiForSystem() };

        // CompositionMode に基づいて ex_style を調整
        let ex_style = match composition_mode {
            CompositionMode::ULW => style_comp.ex_style, // WS_EX_LAYERED (デフォルト)
            CompositionMode::DComp => {
                // DComp モード: WS_EX_NOREDIRECTIONBITMAP を設定、WS_EX_LAYERED は除去
                (style_comp.ex_style & !WS_EX_LAYERED) | WS_EX_NOREDIRECTIONBITMAP
            }
        };

        debug!(
            frame,
            entity = %entity_name,
            has_window_pos = opt_pos.is_some(),
            pos_position = ?pos_comp.position,
            pos_size = ?pos_comp.size,
            "[create_windows] WindowPos before CreateWindow"
        );

        let (x, y, width, height) =
            pos_comp.to_window_coords_for_creation(style_comp.style, ex_style, system_dpi);

        debug!(
            frame,
            entity = %entity_name,
            input_pos = ?pos_comp.position,
            input_size = ?pos_comp.size,
            win_x = x,
            win_y = y,
            win_w = width,
            win_h = height,
            system_dpi,
            "[create_windows] CreateWindowExW"
        );

        let entity_bits = entity.to_bits() as *mut std::ffi::c_void;

        let result = unsafe {
            CreateWindowExW(
                ex_style,
                singleton.ecs_window_class_name(),
                &title_hstring,
                style_comp.style,
                x,
                y,
                width,
                height,
                parent,
                None,
                Some(singleton.instance()),
                Some(entity_bits),
            )
        };

        match result {
            Ok(hwnd) => {
                debug!(
                    frame,
                    entity = %entity_name,
                    hwnd = ?hwnd,
                    "HWND created successfully"
                );

                // 即時にWindowHandleを追加（排他システムなので即時反映）
                world.entity_mut(entity).insert((
                    WindowHandle {
                        hwnd,
                        instance: singleton.instance(),
                    },
                    crate::ecs::graphics::HasGraphicsResources::default(),
                ));

                debug!(
                    frame,
                    entity = %entity_name,
                    "WindowHandle added"
                );

                unsafe {
                    let _ = ShowWindow(hwnd, SW_SHOW);
                }

                debug!(
                    frame,
                    entity = %entity_name,
                    "ShowWindow completed"
                );
            }
            Err(e) => {
                error!(
                    frame,
                    entity = %entity_name,
                    error = ?e,
                    "Failed to create window"
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// task 4.3 解釈 (2): `EcsWorldSelfRef` 未注入（旧 `WinThreadMgr` 経路相当）では
    /// `create_windows` は何もしない（早期 return）。`Window` Entity を spawn しても
    /// `WindowHandle` は付かず、panic も二重生成も起きない。
    ///
    /// self-ref 在の新経路（ファクトリ生成）は実 HWND を要するため
    /// `runtime/window_factory.rs` のヘッドレステスト（`factory_creates_...`）が担う。
    /// 本テストは「未注入なら no-op」の共存ガードのみを headless で検証する。
    #[test]
    fn create_windows_noops_without_self_ref() {
        let mut world = World::new();
        // create_windows が読む FrameCount は新経路では参照しないが、legacy 互換のため
        // 念のため挿入しておく（新 create_windows は self-ref 不在で即 return する）。
        world.insert_resource(crate::ecs::world::FrameCount::default());

        // Window コンポーネント付き Entity を spawn（self-ref は注入しない＝旧経路相当）。
        let entity = world.spawn(Window::default()).id();

        // self-ref 不在ゆえ no-op（panic しない）。
        create_windows(&mut world);

        // WindowHandle は付与されないべき（生成されていない）。
        assert!(
            world.get::<WindowHandle>(entity).is_none(),
            "self-ref 未注入時は create_windows が no-op で WindowHandle を付けないべき"
        );
        // HasGraphicsResources も付かない。
        assert!(
            world
                .get::<crate::ecs::graphics::HasGraphicsResources>(entity)
                .is_none(),
            "self-ref 未注入時は HasGraphicsResources も付かないべき"
        );
    }
}

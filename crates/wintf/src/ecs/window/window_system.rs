use crate::ecs::*;
use bevy_ecs::prelude::*;

// Note: init_window_arrangement システムは廃止されました。
// Arrangementは以下のライフタイムフックで自動挿入されます：
// - Visual::on_add (Window は on_window_add で Visual を挿入)
// - LayoutRoot::on_add
// - Monitor::on_add
// Arrangement::on_add が GlobalArrangement と ArrangementTreeChanged を自動挿入します。

/// 生成を待っている窓の 3 条件。生成に失敗した印の付いた実体は、印が外されるまで作り直さない。
type PendingWindow = (
    With<Window>,
    Without<WindowHandle>,
    Without<crate::runtime::window_factory::WindowCreationFailed>,
);

/// 未作成のWindowを検出してライブラリ経由で作成する排他システム。
///
/// 排他システムにすることで、WindowHandleの追加が即時反映され、
/// 同じフレーム内の後続スケジュールでWindowHandleが参照可能になる。
///
/// # 生成ガード
/// `EcsWorldSelfRef`（NonSend リソース・外側 `Weak<RefCell<EcsWorld>>`）の有無で分岐する:
/// - **不在**: self-ref を注入しない構成（headless テスト等）では何もしない
///   （早期 return・panic も二重生成もしない）。
/// - **在**: `WinApp` 経路。宣言的クエリにマッチする各 Entity に対し
///   [`EcsWindowFactory::create_window`] を呼ぶ（ライブラリの `Window<WndState>` を生成し
///   style/pos/title を反映、`WindowRegistry` へ格納、`WindowHandle`+`HasGraphicsResources`
///   を insert）。これが設計公認の単一上向きエッジ（ecs→runtime）。
pub fn create_windows(world: &mut World, pending: &mut QueryState<Entity, PendingWindow>) {
    // self-ref（外側 World への Weak）を取得。未注入なら no-op（headless テスト等）。
    let Some(self_ref) = world.get_non_send::<crate::ecs::world::EcsWorldSelfRef>() else {
        // self-ref 不在では何もしない（panic なし・二重生成なし）。
        return;
    };
    let ecs_world: std::rc::Weak<std::cell::RefCell<crate::ecs::world::EcsWorld>> =
        self_ref.0.clone();

    // 宣言的クエリで未生成 Window Entity を収集（borrow を即解放）。
    let entities_to_create: Vec<Entity> = pending.iter(world).collect();

    // 各 Entity をファクトリ経由で生成（ライブラリ生成・registry 格納・handle 反映）。
    for entity in entities_to_create {
        crate::runtime::window_factory::EcsWindowFactory::create_window(
            world,
            entity,
            ecs_world.clone(),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_ecs::system::RunSystemOnce;

    /// `EcsWorldSelfRef` 未注入時は `create_windows` は何もしない（早期 return）。
    /// `Window` Entity を spawn しても `WindowHandle` は付かず、panic も二重生成も起きない。
    ///
    /// self-ref 在の新経路（ファクトリ生成）は実 HWND を要するため
    /// `runtime/window_factory.rs` のヘッドレステスト（`factory_creates_...`）が担う。
    /// 本テストは「未注入なら no-op」の生成ガードのみを headless で検証する。
    #[test]
    fn create_windows_noops_without_self_ref() {
        let mut world = World::new();

        // Window コンポーネント付き Entity を spawn（self-ref は注入しない）。
        let entity = world.spawn(Window::default()).id();

        // self-ref 不在ゆえ no-op（panic しない）。
        world.run_system_once(create_windows).unwrap();

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

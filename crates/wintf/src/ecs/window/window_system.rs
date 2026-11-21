use crate::ecs::*;
use bevy_ecs::prelude::*;

/// 生成を待っている窓の 3 条件。生成に失敗した印の付いた実体は、印が外されるまで作り直さない。
type PendingWindow = (
    With<Window>,
    Without<WindowHandle>,
    Without<crate::runtime::window_factory::WindowCreationFailed>,
);

/// 未作成のWindowを検出してライブラリ経由で作成する排他システム。
///
/// 排他システムにすることで、`WindowHandle` の追加が即時反映され、同じフレーム内の
/// 後続スケジュールから参照できる。
///
/// # 生成ガード
/// `EcsWorldSelfRef`（NonSend リソース）が無い構成（headless テスト等）では何もしない。
/// 在るとき（`WinApp` 経路）は、宣言的クエリにマッチする各 Entity に
/// [`EcsWindowFactory::create_window`] を呼ぶ。
pub fn create_windows(world: &mut World, pending: &mut QueryState<Entity, PendingWindow>) {
    let Some(self_ref) = world.get_non_send::<crate::ecs::world::EcsWorldSelfRef>() else {
        return;
    };
    let ecs_world: std::rc::Weak<std::cell::RefCell<crate::ecs::world::EcsWorld>> =
        self_ref.0.clone();

    let entities_to_create: Vec<Entity> = pending.iter(world).collect();

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

        let entity = world.spawn(Window::default()).id();

        world.run_system_once(create_windows).unwrap();

        assert!(
            world.get::<WindowHandle>(entity).is_none(),
            "self-ref 未注入時は create_windows が no-op で WindowHandle を付けないべき"
        );
        assert!(
            world
                .get::<crate::ecs::graphics::HasGraphicsResources>(entity)
                .is_none(),
            "self-ref 未注入時は HasGraphicsResources も付かないべき"
        );
    }
}

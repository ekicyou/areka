//! イベントディスパッチモジュール
//!
//! WinUI3 スタイルのポインターイベント配信機構を提供する。
//! ヒットテストで特定されたエンティティから親エンティティへイベントを伝播し、
//! 登録されたハンドラを呼び出す。

use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::prelude::*;

use super::PointerState;

// ============================================================================
// Phase<T> enum (Task 2.1)
// ============================================================================

/// イベント伝播フェーズ（Rust らしい enum 表現）
///
/// フェーズとイベントデータを一体化し、パターンマッチで処理可能。
/// WinUI3 の PreviewXxx (Tunnel) / Xxx (Bubble) パターンに対応。
#[derive(Clone, Debug)]
pub enum Phase<T> {
    /// トンネルフェーズ（親→子、WinUI3 PreviewXxx 相当）
    Tunnel(T),
    /// バブルフェーズ（子→親、通常イベント）
    Bubble(T),
}

impl<T> Phase<T> {
    /// フェーズに関係なくイベントデータを取得
    #[inline]
    pub fn value(&self) -> &T {
        match self {
            Phase::Tunnel(v) | Phase::Bubble(v) => v,
        }
    }

    /// トンネルフェーズか判定
    #[inline]
    pub fn is_tunnel(&self) -> bool {
        matches!(self, Phase::Tunnel(_))
    }

    /// バブルフェーズか判定
    #[inline]
    pub fn is_bubble(&self) -> bool {
        matches!(self, Phase::Bubble(_))
    }
}

// ============================================================================
// EventHandler<T> 型エイリアス (Task 2.2)
// ============================================================================

/// 汎用イベントハンドラ型
///
/// # Type Parameters
/// - `T`: イベントデータ型（PointerState 等）
///
/// # Arguments
/// - `world`: 可変 World 参照（コンポーネント読み書き用）
/// - `sender`: 元のヒット対象エンティティ（不変、WinUI3 の OriginalSource 相当）
/// - `entity`: 現在処理中のエンティティ（バブリングで変化、WinUI3 の sender 相当）
/// - `ev`: フェーズ付きイベントデータ
///
/// # Returns
/// - `true`: イベント処理済み（伝播停止）
/// - `false`: 未処理（次のエンティティへ伝播続行）
pub type EventHandler<T> =
    fn(world: &mut World, sender: Entity, entity: Entity, ev: &Phase<T>) -> bool;

/// ポインターイベントハンドラ型エイリアス
pub type PointerEventHandler = EventHandler<PointerState>;

// ============================================================================
// ハンドラコンポーネント群 (Task 3)
// ============================================================================

/// ポインター押下ハンドラコンポーネント（WinUI3 OnPointerPressed 相当）
///
/// SparseSet ストレージで少数エンティティに最適化。
#[derive(Component, Clone, Copy)]
#[component(storage = "SparseSet")]
pub struct OnPointerPressed(pub PointerEventHandler);

/// ポインター解放ハンドラコンポーネント（WinUI3 OnPointerReleased 相当）
#[derive(Component, Clone, Copy)]
#[component(storage = "SparseSet")]
pub struct OnPointerReleased(pub PointerEventHandler);

/// ポインター進入ハンドラコンポーネント（WinUI3 OnPointerEntered 相当）
#[derive(Component, Clone, Copy)]
#[component(storage = "SparseSet")]
pub struct OnPointerEntered(pub PointerEventHandler);

/// ポインター退出ハンドラコンポーネント（WinUI3 OnPointerExited 相当）
#[derive(Component, Clone, Copy)]
#[component(storage = "SparseSet")]
pub struct OnPointerExited(pub PointerEventHandler);

/// ポインター移動ハンドラコンポーネント（WinUI3 OnPointerMoved 相当）
#[derive(Component, Clone, Copy)]
#[component(storage = "SparseSet")]
pub struct OnPointerMoved(pub PointerEventHandler);

// ============================================================================
// ディスパッチシステム (Task 4)
// ============================================================================

/// 親チェーン構築（sender → root の順で格納）
///
/// バブリング経路を構築する汎用ヘルパー関数。
/// 公開APIとして、他のイベントシステム（ドラッグ等）でも利用可能。
/// Window コンポーネントを持つエンティティに到達したら停止する（Window 自身は含む）。
pub fn build_bubble_path(world: &World, start: Entity) -> Vec<Entity> {
    use crate::ecs::window::Window;

    let mut path = vec![start];

    // start 自身が Window の場合はここで停止
    if world.get::<Window>(start).is_some() {
        return path;
    }

    let mut current = start;
    while let Some(child_of) = world.get::<ChildOf>(current) {
        let parent = child_of.parent();
        path.push(parent);

        // Window に到達したら停止（Window はパスに含める）
        if world.get::<Window>(parent).is_some() {
            break;
        }

        current = parent;
    }
    path
}

/// イベント種別を判定してハンドラを呼び出す内部関数（ジェネリック版）
///
/// Phase<T>でイベントを配信する汎用関数。
/// 任意のイベント型TとハンドラコンポーネントHに対して、Tunnel/Bubble伝播を実行する。
/// 公開APIとして、他のイベントシステム（ドラッグ等）でも利用可能。
///
/// # Type Parameters
/// - `T`: イベントデータ型（Clone可能である必要がある）
/// - `H`: ハンドラコンポーネント型（Copy可能である必要がある）
///
/// # Arguments
/// - `world`: 可変World参照
/// - `sender`: イベント発生元エンティティ
/// - `path`: バブリングパス（sender → root）
/// - `event`: イベントデータ
/// - `get_handler`: ハンドラコンポーネントからハンドラ関数を取得する関数
pub fn dispatch_event_for_handler<T: Clone, H: Component + Copy>(
    world: &mut World,
    sender: Entity,
    path: &[Entity],
    event: &T,
    get_handler: fn(&H) -> EventHandler<T>,
) {
    // Tunnel フェーズ: root → sender
    for &entity in path.iter().rev() {
        // エンティティ存在チェック
        if world.get_entity(entity).is_err() {
            return; // 静かに終了
        }

        // ハンドラ取得
        if let Some(handler_comp) = world.get::<H>(entity).copied() {
            let handler = get_handler(&handler_comp);
            if handler(world, sender, entity, &Phase::Tunnel(event.clone())) {
                return; // handled
            }
        }
    }

    // Bubble フェーズ: sender → root
    for &entity in path.iter() {
        // エンティティ存在チェック
        if world.get_entity(entity).is_err() {
            return; // 静かに終了
        }

        // ハンドラ取得
        if let Some(handler_comp) = world.get::<H>(entity).copied() {
            let handler = get_handler(&handler_comp);
            if handler(world, sender, entity, &Phase::Bubble(event.clone())) {
                return; // handled
            }
        }
    }
}

/// ポインターイベントディスパッチシステム
///
/// Input スケジュールで実行される排他システム。
/// PointerState を持つ全エンティティについて独立にバブリング経路を構築し、
/// 登録されたハンドラを順次呼び出す。
///
/// # Algorithm
/// 1. PointerState を持つ全エンティティを収集（Clone してデタッチ）
/// 2. 各 (sender, state) について独立に処理:
///    a. ChildOf.parent() を辿り親チェーン（パス）を構築
///    b. Tunnel: root → sender の順で OnPointer* ハンドラを収集・実行
///    c. Bubble: sender → root の順で OnPointer* ハンドラを収集・実行
///    d. 各呼び出し前にエンティティ存在チェック（削除済みなら静かに終了）
///    e. ハンドラが `true` を返したら伝播停止
pub fn dispatch_pointer_events(world: &mut World) {
    // Pass 1: 全ての PointerState を持つエンティティを収集
    let targets: Vec<(Entity, PointerState)> = {
        let mut query = world.query::<(Entity, &PointerState)>();
        query.iter(world).map(|(e, s)| (e, s.clone())).collect()
    };

    // 各ポインター状態について独立に dispatch
    for (sender, state) in &targets {
        // 親チェーン構築
        let path = build_bubble_path(world, *sender);

        // OnPointerMoved: 常に発火（移動イベント）
        dispatch_event_for_handler::<PointerState, OnPointerMoved>(
            world,
            *sender,
            &path,
            state,
            |h| h.0,
        );

        // OnPointerPressed: ボタンが押されている場合に発火（1フレームのみ）
        if state.left_down || state.right_down || state.middle_down {
            dispatch_event_for_handler::<PointerState, OnPointerPressed>(
                world,
                *sender,
                &path,
                state,
                |h| h.0,
            );
        }
    }

    // ボタン状態とダブルクリック情報をクリア（次フレームで再発火しないように）
    for (entity, _) in &targets {
        if let Some(mut pointer_state) = world.get_mut::<PointerState>(*entity) {
            pointer_state.left_down = false;
            pointer_state.right_down = false;
            pointer_state.middle_down = false;
            pointer_state.xbutton1_down = false;
            pointer_state.xbutton2_down = false;
            pointer_state.double_click = super::DoubleClick::None;
        }
    }
}

// ============================================================================
// テスト (Task 5)
// ============================================================================

#[cfg(test)]
mod tests;

//! フレームカウンタとスケジュールラベル定義

use bevy_ecs::prelude::*;
use bevy_ecs::schedule::*;

/// フレームカウンタリソース
#[derive(Resource, Default, Debug)]
pub struct FrameCount(pub u32);

// 各プライオリティ用のScheduleLabelマーカー構造体
// 実行順序（EcsWorld::try_tick_world の try_run_schedule 呼び出し順と一致する不変条件。
// この順序が登録順 EcsWorld::new と一致することは W7b-V で実コード照合済み）:
//   Input → Update → PreLayout → Layout → PostLayout → UISetup → GraphicsSetup
//   → Draw → PreRenderSurface → RenderSurface → Composition → CommitComposition → FrameFinalize

/// 入力処理スケジュール
///
/// キーボード・マウス・タッチ等の入力イベントを処理する。
/// マルチスレッド実行可能。
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Input;

/// 通常のロジック更新スケジュール
///
/// アプリケーションロジック、状態更新、アニメーション計算等を行う。
/// マルチスレッド実行可能。
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Update;

/// レイアウト計算前スケジュール
///
/// レイアウト計算の準備処理（サイズ制約の設定等）を行う。
/// マルチスレッド実行可能。
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
pub struct PreLayout;

/// レイアウト計算スケジュール
///
/// ウィンドウ/ウィジェットのサイズと位置を計算する。
/// マルチスレッド実行可能。
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Layout;

/// レイアウト計算後スケジュール
///
/// レイアウト結果を使った処理（グラフィックスリソース作成等）を行う。
/// マルチスレッド実行可能。
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
pub struct PostLayout;

/// UIセットアップスケジュール（メインスレッド固定）
///
/// Win32 APIを使用したウィンドウ作成・破棄等のUIスレッド専用処理を行う。
/// CreateWindowEx, DestroyWindow, PostQuitMessage等のメッセージループに影響する処理を含む。
/// SingleThreadedエグゼキュータで実行される。
///
/// **重要**: UIスレッド固定が必要な処理のみをここに配置すること。
/// DirectComposition/Direct2D等のグラフィックスAPI呼び出しは通常マルチスレッド対応なので、
/// PostLayoutやDrawで実行すべき。
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
pub struct UISetup;

/// グラフィックスセットアップスケジュール
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
pub struct GraphicsSetup;

/// 描画コマンド生成スケジュール
///
/// ID2D1CommandListを使用した描画コマンドリストを作成する。
/// 実際の描画は行わず、描画命令をバッファに記録するだけ。
/// マルチスレッド実行可能（各ウィンドウのCommandListを並列作成可能）。
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Draw;

/// サーフェス更新前スケジュール
///
/// IDCompositionSurfaceへの更新前に行う。
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
pub struct PreRenderSurface;

/// サーフェス更新スケジュール
///
/// IDCompositionSurfaceへの描画を実行する。
/// BeginDraw/EndDrawでSurfaceを取得し、ID2D1DeviceContextで描画を行う。
/// CommandListがある場合はDrawImage()で再生する。
/// マルチスレッド実行可能（各Surfaceは独立）。
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
pub struct RenderSurface;

/// 最終合成スケジュール
///
/// IDCompositionVisualの操作（Transform, Opacity, Effect設定等）を行う。
/// ビジュアルツリーの構築・更新、アニメーション設定等。
/// マルチスレッド実行可能。
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Composition;

/// DirectCompositionコミットスケジュール
///
/// IDCompositionDevice3::Commit()を呼び出し、すべてのビジュアル変更を確定する。
/// このスケジュールは常にワールドスケジュールの最後に実行される。
/// マルチスレッド実行可能（Commit()はスレッドセーフ）。
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
pub struct CommitComposition;

/// フレーム終了時クリーンアップスケジュール
///
/// CommitComposition の後に実行される。
/// - 一時的なマーカーコンポーネント（MouseLeave等）の除去
/// - 1フレームのみ有効な状態（DoubleClick, Wheel等）のリセット
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
pub struct FrameFinalize;

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    /// `FrameCount` の Default は 0（フレーム未経過の初期値）。
    #[test]
    fn test_frame_count_default_is_zero() {
        assert_eq!(FrameCount::default().0, 0);
    }

    /// `FrameCount` は内部 u32 を素通しで保持する（new 相当のタプル構築）。
    #[test]
    fn test_frame_count_stores_inner_value() {
        assert_eq!(FrameCount(42).0, 42);
        assert_eq!(FrameCount(u32::MAX).0, u32::MAX);
    }

    /// ハッシュ値を計算するヘルパ。
    fn hash_of<T: Hash>(value: &T) -> u64 {
        let mut hasher = DefaultHasher::new();
        value.hash(&mut hasher);
        hasher.finish()
    }

    /// 各スケジュールラベルは PartialEq/Eq で同一バリアントどうしが等価。
    /// `Schedules`（HashMap キー）での同定に依存する契約。
    #[test]
    fn test_schedule_labels_equal_themselves() {
        assert_eq!(Input, Input);
        assert_eq!(Update, Update);
        assert_eq!(PreLayout, PreLayout);
        assert_eq!(Layout, Layout);
        assert_eq!(PostLayout, PostLayout);
        assert_eq!(UISetup, UISetup);
        assert_eq!(GraphicsSetup, GraphicsSetup);
        assert_eq!(Draw, Draw);
        assert_eq!(PreRenderSurface, PreRenderSurface);
        assert_eq!(RenderSurface, RenderSurface);
        assert_eq!(Composition, Composition);
        assert_eq!(CommitComposition, CommitComposition);
        assert_eq!(FrameFinalize, FrameFinalize);
    }

    /// `Clone`（ScheduleLabel 経由の dyn ラベル複製でも使われる）が元と等価な値を返す。
    #[test]
    fn test_schedule_labels_clone_preserves_equality() {
        assert_eq!(Input.clone(), Input);
        assert_eq!(Layout.clone(), Layout);
        assert_eq!(FrameFinalize.clone(), FrameFinalize);
    }

    /// 同一バリアントの Hash は一致する（Eq と Hash の整合 = HashMap キー要件）。
    #[test]
    fn test_schedule_labels_hash_is_consistent_with_eq() {
        assert_eq!(hash_of(&Input), hash_of(&Input));
        assert_eq!(hash_of(&Update), hash_of(&Update));
        assert_eq!(hash_of(&FrameFinalize), hash_of(&FrameFinalize));
    }

    /// `Debug` 整形が各ラベルの型名を含む（ユニット構造体の標準整形）。
    #[test]
    fn test_schedule_labels_debug_contains_type_name() {
        assert_eq!(format!("{:?}", Input), "Input");
        assert_eq!(format!("{:?}", Update), "Update");
        assert_eq!(format!("{:?}", PreLayout), "PreLayout");
        assert_eq!(format!("{:?}", Layout), "Layout");
        assert_eq!(format!("{:?}", PostLayout), "PostLayout");
        assert_eq!(format!("{:?}", UISetup), "UISetup");
        assert_eq!(format!("{:?}", GraphicsSetup), "GraphicsSetup");
        assert_eq!(format!("{:?}", Draw), "Draw");
        assert_eq!(format!("{:?}", PreRenderSurface), "PreRenderSurface");
        assert_eq!(format!("{:?}", RenderSurface), "RenderSurface");
        assert_eq!(format!("{:?}", Composition), "Composition");
        assert_eq!(format!("{:?}", CommitComposition), "CommitComposition");
        assert_eq!(format!("{:?}", FrameFinalize), "FrameFinalize");
    }

    /// 異なるラベル型を `dyn ScheduleLabel` として比較すると非等価（型混同しない）。
    /// `Box<dyn ScheduleLabel>` の intern_label 経由で `Schedules` が型を区別する根拠。
    #[test]
    fn test_distinct_schedule_labels_are_not_equal_as_dyn() {
        let input: Box<dyn ScheduleLabel> = Box::new(Input);
        let update: Box<dyn ScheduleLabel> = Box::new(Update);
        let input2: Box<dyn ScheduleLabel> = Box::new(Input);

        // Box<dyn ScheduleLabel> は Copy でないため参照経由で比較する。
        assert!(input != update, "Input と Update は別ラベル");
        assert!(input == input2, "同一バリアントの dyn ラベルは等価");
    }
}

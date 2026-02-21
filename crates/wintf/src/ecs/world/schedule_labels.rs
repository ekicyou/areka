//! フレームカウンタとスケジュールラベル定義

use bevy_ecs::prelude::*;
use bevy_ecs::schedule::*;

/// フレームカウンタリソース
#[derive(Resource, Default, Debug)]
pub struct FrameCount(pub u32);

// 各プライオリティ用のScheduleLabelマーカー構造体
// 実行順序: Input → Update → PreLayout → Layout → PostLayout → UISetup → Draw → Render → RenderSurface → Composition → CommitComposition

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

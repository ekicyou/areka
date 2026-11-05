//! フレームカウンタとスケジュールラベル定義

use bevy_ecs::prelude::*;
use bevy_ecs::schedule::*;

/// フレームカウンタリソース
#[derive(Resource, Default, Debug)]
pub struct FrameCount(pub u32);

// 各段の ScheduleLabel。`EcsWorld::try_tick_world` は毎回この順で 13 本を回す:
//   Input → Update → PreLayout → Layout → PostLayout → UISetup → GraphicsSetup
//   → Draw → PreRenderSurface → RenderSurface → Composition → CommitComposition → FrameFinalize
// 単一スレッド（UI スレッド）で回すのは `world::single_threaded_schedules()` の 6 本
// （UISetup・GraphicsSetup・PreRenderSurface・RenderSurface・Composition・CommitComposition）。
// WUC（Windows.UI.Composition）はスレッド親和なので、WUC に触る係はこの 6 本にだけ置く。
// 残りの 7 本は多スレッドで回りうる。

/// 入力の段（多スレッド）。
///
/// 非同期の仕事の結果を取り込み、ポインタとドラッグの出来事を配る。
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Input;

/// 状態の更新の段（多スレッド）。
///
/// 表示構成の変化の検知・モニターの配置の更新・タイプライターの進行を行う。
/// 使い手のクレートも自分のロジックをここへ足す。
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Update;

/// レイアウトの前の段（多スレッド）。
///
/// wintf 自身の係は置いていない。使い手がレイアウトの前に済ませたい処理を足す口。
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
pub struct PreLayout;

/// レイアウトの段（多スレッド）。
///
/// taffy の型と木を作り、大きさと位置を計算して配置（Arrangement）へ写す。
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Layout;

/// レイアウトの後の段（多スレッド）。
///
/// 配置を木へ伝え、窓の位置（WindowPos）へ写す。WUC には触らない。
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
pub struct PostLayout;

/// 窓の段（単一スレッド・UI スレッド）。
///
/// 窓の作成（CreateWindowEx）と、窓の位置の反映（SetWindowPos）を行う。
/// Win32 の窓はそれを作ったスレッドに縛られるので、UI スレッドで回す。
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
pub struct UISetup;

/// 描画の装置と WUC の資源の段（単一スレッド・UI スレッド）。
///
/// 装置の失いの確かめ → 装置に頼る部品の無効化 → 描画の装置（GraphicsCore）の作成
/// → WUC の資源の作成 → 窓ごとの WUC の的の作成、の順に回す。作る係は作れる回だけ回る。
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
pub struct GraphicsSetup;

/// 描画の命令の段（多スレッド）。
///
/// 部品ごとに D2D の命令の列（ID2D1CommandList）を作るだけで、描画面にも WUC にも触らない。
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Draw;

/// 描画面の用意の段（単一スレッド・UI スレッド）。
///
/// 後から窓へつないだ部品に見た目の部品を入れ、WUC の見た目（Visual）と描画面（Surface）を
/// 作り（WUC が在る回だけ）、描き直す描画面に印を付け、窓の見た目をつなぐ。
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
pub struct PreRenderSurface;

/// 描画面への描画の段（単一スレッド・UI スレッド）。
///
/// 印の付いた WUC の描画面に BeginDraw/EndDraw で D2D の文脈を取り、命令の列を描く。
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
pub struct RenderSurface;

/// 合成の段（単一スレッド・UI スレッド）。
///
/// WUC の見た目の木の親子・性質（位置・不透明度など）・切り抜きを ECS に合わせる。
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Composition;

/// 合成の確定の段（単一スレッド・UI スレッド）。
///
/// wintf 自身の係は置いていない。WUC の変更は暗黙に反映されるので、明示の commit は要らない。
/// 13 本の順を変えないために段だけ残している。
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
pub struct CommitComposition;

/// 回の終わりの片付けの段（多スレッド）。
///
/// 1 回だけ有効なポインタの状態（MouseLeave・DoubleClick・Wheel など）を消し、
/// ドラッグ中は次の回を予約し、ドラッグの出来事の列（Messages）を進め、ツールチップを判定する。
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
pub struct FrameFinalize;

#[cfg(test)]
mod tests {
    use super::*;

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
}

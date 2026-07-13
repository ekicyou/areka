# Brief: areka-P0-surface-resize-resnap

> 由来: 2026-07-13 M-boot（`areka-P0-emo2-boot`）実機サインオフ（R9.3）で発見の実機欠陥#1。詳細な根本追跡は同マイルストーンの discovery 記録（roadmap「M-boot 実機サインオフ発見」節）参照。

## Problem
実機（実 emo2・実 DPI）で、むらさき（scope0）が挨拶用の焼き込み立ち絵 `surface0`（434×687）を表示した後、さくらスクリプトの `\s[1000]` 等で**本来の本体サーフェスへ切り替わる**と、切替後サーフェスの**サイズが初期サーフェスと異なる**ため、マスコットが**画面下端に吸着しなくなる**（宙に浮く／下端からずれる）。ゴーストは「常に画面下端に立つ」のが SSP 互換の既定挙動であり、この破綻は M-boot の目視品質を損なう。

## Current State
- 窓の位置・サイズは **spawn 時に一度だけ**確定する。`crates/areka/src/placement/measure.rs:62` が初期サーフェス（scope0=surface0→434×687／scope1=surface10→336×400）のみを採寸し、`crates/areka/src/placement/spawn.rs:149-233`（`spawn_ghost_windows`）がその固定サイズを `WindowPos` に焼き込む。
- 下端吸着ロジック `crates/areka/src/placement/follow.rs`（`BottomSnapPolicy`＝Y = work_area.bottom − h）は存在するが、**ドラッグイベント（`on_char_drag`/`on_char_drag_end`）にしか結線されていない**。
- 実行時のサーフェス切替（emo-present の `ShowSurface` で異サイズ内容へ）→ **窓リサイズ／再吸着を駆動するシームが無い**。ゆえに切替後も旧サイズ・旧 Y のまま下端から浮く。
- window-placement（完了済み）は**spawn 時配置＋ドラッグ再吸着のみ**を明示的にスコープとしており、実行時サイズ変化は範囲外。

## Desired Outcome
実行時にキャラクターの表示サーフェスのサイズが変わっても、窓が新サイズへ追随し、**下端吸着が維持される**（画面下端に立ち続ける）。ドラッグ中／後の再吸着（既存）と一貫した単一の位置ポリシーで駆動される。

## Approach
emo-present（⑥emo）が `ShowSurface` 適用でサーフェスの表示サイズが変化したことを検知し、**placement（⓪ghost）へサイズ変化通知シーム**を1本張る。placement 側は通知を受けて窓サイズを新サイズへ反映し、既存 `BottomSnapPolicy` を再適用して下端 Y を再計算する（ドラッグ再吸着と同じ単一ライター経路に合流＝振動回避）。新規機構は最小＝「サイズ変化通知」＋「resize＋re-snap の駆動口」に徹し、既存の採寸・spawn・drag ポリシーは再利用する。

## Scope
- **In**: 実行時サーフェスサイズ変化の検知（emo-present）→ placement へのサイズ変化通知 I/O 契約 → 窓 resize ＋ 下端 re-snap（既存 `BottomSnapPolicy` 再適用）。実機（実 DPI≠96）目視で「切替後も下端吸着維持」を受け入れ条件とする（window-placement の本番ゴースト先行原則に従う）。
- **Out**: バルーン窓の追従位置記憶（既存 follow が所有）／二人立ちの窓割当本格化（M-dual）／初期サーフェスを何にするか＝**表示するか否かの決定（欠陥#5＝emo2-boot 側で is 修正済み前提）**／サーフェス合成・文字層。

## Boundary Candidates
- 検知・通知（emo-present 側のサイズ変化イベント発火）
- 反映・再吸着（placement 側の resize＋re-snap 駆動＝単一位置ライターへ合流）
- I/O 契約（サイズ変化メッセージ型＝クロスエンジン契約・actor-foundation 規約に載せる）

## Out of Boundary
- 初期表示サーフェスの選択・非表示既定（-1）＝emo2-boot（#5 で対応済み前提）
- ドラッグ機構そのもの（完了済み event-drag-system／window-placement）
- 位置永続化（position-persist・M-life）

## Upstream / Downstream
- **Upstream**: `areka-P0-window-placement`（完了・採寸/spawn/follow/BottomSnapPolicy）・`areka-P0-emo-present`（完了・ShowSurface/表示サイズ）・`areka-P0-emo2-boot`（#5 初期非表示修正が前提＝「最初に見えるサーフェス」がサイズ基準）・`areka-P0-actor-foundation`（通信規約）。
- **Downstream**: M-dual（二人立ちの窓リサイズ／再配置が同シームを再利用）。

## Existing Spec Touchpoints
- **Extends**: なし（新規境界。window-placement は spawn 時のみ・本 spec が実行時サイズ変化を追加所有）。
- **Adjacent**: `areka-P0-window-placement`（follow/snap を消費・再定義しない）・`areka-P0-emo-present`（表示サイズの source）・`areka-P0-emo2-boot`（#5 と interlock）。

## Constraints
- Rust 2024・新規 crates.io 依存なし・tokio 不使用。
- **本番ゴースト先行＋実 DPI（≠96）実機受け入れ**が観測条件（window-placement リジェクトの教訓・dpi=96 自己整合は欠陥を隠す）。
- 位置ライターは単一（生ドラッグ座標→実窓位置の反映段階で正座標確定＝事後補正の振動を排除。window-placement DD15 v2 の `DragPositionPolicy` 単一ライター原則に合流）。

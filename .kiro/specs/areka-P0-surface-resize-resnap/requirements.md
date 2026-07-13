# Requirements Document

## Project Description (Input)

実機（実 emo2・実 DPI≠96）で、むらさき（scope0）が挨拶用の焼き込み立ち絵 `surface0`（434×687）を表示した後、さくらスクリプトの `\s[1000]` 等で本体サーフェスへ切り替わると、切替後サーフェスのサイズが初期サーフェスと異なるため、マスコットが画面下端に吸着しなくなる（宙に浮く／下端からずれる）。窓の位置・サイズは spawn 時に一度だけ確定し（`placement/measure.rs`＋`placement/spawn.rs`）、実行時のサーフェス切替（emo-present の `ShowSurface`）で窓リサイズ／再吸着を駆動するシームが無いのが原因。本 spec は、⑥emo-present が表示サーフェスの寸法変化を検知し、⓪ghost（placement）へサイズ変化を通知する 1 本のシームを張り、placement が窓を新寸へリサイズして既存 `BottomSnapPolicy` を再適用し下端吸着を維持する（ドラッグ再吸着と同じ単一位置ライター経路へ合流＝振動回避）。既存の採寸・spawn・drag ポリシーは再利用し、新規機構は「サイズ変化通知」＋「resize＋re-snap の駆動口」に徹する。由来: 2026-07-13 M-boot（`areka-P0-emo2-boot`）実機サインオフ（R9.3）で発見の実機欠陥#1。

## Introduction

本ユニットは、実行時にキャラクターの表示サーフェスのサイズが変わっても、窓が新サイズへ追随し**下端吸着が維持される**（画面下端に立ち続ける）ことを実現する。⑥emo（表示エンジン）が `ShowSurface` 適用で表示サーフェス寸法が変化したことを検知し、**⓪ghost（placement）へサイズ変化通知シームを 1 本**張る。placement は通知を受けて窓サイズを新寸へ反映し、既存 `BottomSnapPolicy`（Y = work area 下端 − 高さ）を再適用して下端 Y を再計算する。ドラッグ中／後の再吸着（`areka-P0-window-placement` 完了・既存）と一貫した**単一の位置ポリシー**で駆動し、事後補正の振動を避ける。

検証は `areka-P0-window-placement` の本番ゴースト先行原則を継承し、**本番ゴースト（実 emo2・emo-present 経由の実 surface）を実 DPI（≠96・例 125%）で表示した上で「切替後も下端吸着維持」を目視で確認**することを受け入れ条件とする（dpi=96 の自己整合が欠陥を隠す教訓を継承）。決定論的に検証可能な部分（新寸法反映・下端 Y 再計算・非正寸縮退・べき等）は DPI をパラメタ化した純粋関数テストで固定する。

## Boundary Context

- **In scope**: 実行時サーフェスサイズ変化の検知（⑥emo-present の `ShowSurface` 適用点）／サイズ変化通知の I/O 契約（対象スコープ＋新寸法をクロスエンジンで届ける単方向メッセージ・既存 UI 配送ブリッジへ載せる）／⓪ghost（placement）での窓リサイズ＋下端 re-snap（既存 `BottomSnapPolicy` 再適用）／`free` スコープの寸法のみ反映／随伴バルーン offset 維持／不在・非正寸のログ付き縮退／実 DPI（≠96）本番ゴースト目視受け入れ＋決定論部の純粋関数テスト。
- **Out of scope**: 初期表示サーフェスの選択・非表示既定（-1）＝`areka-P0-emo2-boot` の #5 で対応済み前提（「最初に見えるサーフェス」がサイズ基準）／サーフェス合成・文字層・αマスク生成の中身／バルーンの正式な配置規則・バルーン窓の位置記憶（既存 follow が所有）／二人立ちの窓割当・本格連動（M-dual）／位置永続化（`position-persist`・M-life）／ドラッグ機構そのもの（`event-drag-system`／`window-placement` 完了）。
- **Adjacent expectations**: `areka-P0-window-placement`（完了）の採寸・spawn・`follow`（`DragPositionPolicy`／`BottomSnapPolicy`／`move_window_to` 単一ライター・`BalloonFollow`）を**消費・再利用し再定義しない**。`areka-P0-emo-present`（完了）は表示寸法の source（`ShowSurface` 適用で表示サイズが変化）。`areka-P0-emo2-boot`（#5 初期非表示修正が前提・frame.rs の drain phase が `PresentCommand` 適用点＝配送候補）と interlock。通信規約は `areka-P0-actor-foundation`（UI 配送ブリッジ）を利用し新規フレームワークを導入しない。Downstream: M-dual が同シームを再利用。emo2 は最小適合 fixture であって書式の聖典ではない（正典は ukadoc）。

## Requirements

### Requirement 1: 実行時サーフェスサイズ変化の検知（⑥emo-present）

**Objective:** As ⑥emo（表示エンジン）, I want `ShowSurface` 適用で表示サーフェスの寸法が変わったことを検知できること, so that 窓側へサイズ追随を促すシームを発火できる

#### Acceptance Criteria

1. When 実行時に `ShowSurface` が適用され表示サーフェスの寸法（幅・高さ）が直前の表示寸法と異なるとき, the サーフェスサイズ変化検知 shall サイズ変化を検知し新しい表示寸法を確定する。
2. When 初回の `ShowSurface` で表示寸法が初めて確定するとき, the サーフェスサイズ変化検知 shall それを比較基準（ベースライン）として確定し、以後の実行時変化判定に用いる。
3. When `ShowSurface` が適用され新しい表示寸法が直前の表示寸法と同一のとき, the サーフェスサイズ変化検知 shall サイズ変化を発火しない（不要な通知・再配置を避ける）。
4. The サーフェスサイズ変化検知 shall 検知対象を「表示サーフェスの実寸法」に限定し、寸法が同一なら合成内容・文字層・αマスクの中身が変化しても発火しない。
5. The サーフェスサイズ変化検知 shall 検知した新寸法を、どのスコープ（キャラ窓）に属する変化かを識別可能な形で保持する。

### Requirement 2: サイズ変化通知の I/O 契約（クロスエンジン）

**Objective:** As ⓪ghost（窓所有者）, I want ⑥emo からサーフェスの新寸法を受け取るシームがあること, so that 窓を新寸へ追随させられる

#### Acceptance Criteria

1. When サーフェスサイズ変化が検知されたとき, the サイズ変化通知シーム shall 対象スコープ（どのキャラ窓か）と新しい表示寸法（幅・高さ）を含むメッセージを placement へ届ける。
2. The サイズ変化通知シーム shall 単方向（⑥emo→⓪ghost）とし、placement からの応答を要求しない。
3. The サイズ変化通知シーム shall 通知する寸法を実際に適用されたサーフェスの寸法と一致させ、古い（適用前の）寸法を通知しない。
4. The サイズ変化通知シーム shall 既存のプロジェクト内クロスエンジン通信（`areka-P0-actor-foundation` の UI 配送ブリッジ）へ載せ、新規の通信フレームワーク・新規 crates.io 依存を導入しない。

### Requirement 3: 窓リサイズと下端再吸着（⓪ghost placement）

**Objective:** As ⓪ghost（placement）, I want サイズ変化通知を受けて窓を新寸へリサイズし下端へ再吸着すること, so that サーフェス切替後もマスコットが画面下端に立ち続ける

#### Acceptance Criteria

1. When placement がサーフェスサイズ変化通知を受け取ったとき, the 窓リサイズ・再吸着機構 shall 対象キャラ窓の寸法を通知された新しい表示寸法へ反映する。
2. When 対象キャラ窓が bottom 吸着スコープ（`Bottom`／`Seam`）であるとき, the 窓リサイズ・再吸着機構 shall 新しい高さに基づき下端 Y（現在窓が属するモニタの work area 下端 − 新しい高さ）を再計算し窓 Y を更新して、窓下端を work area 下端へ保つ（既存 `BottomSnapPolicy` を再適用）。
3. When 対象キャラ窓が `free`（下端非吸着）スコープであるとき, the 窓リサイズ・再吸着機構 shall 窓寸法のみを新寸へ反映し、下端 Y の再計算は行わず現在 Y を保持する。
4. The 窓リサイズ・再吸着機構 shall 窓寸法と再吸着 Y の更新を単一の位置ライター経路（ドラッグ再吸着と同じ `DragPositionPolicy`／`BottomSnapPolicy` を用いる `move_window_to` 系の正規口）へ合流させ、`enqueue_window_move` を迂回した bypass 書込を新設しない。
5. The 窓リサイズ・再吸着機構 shall サーフェス切替のたびに窓が上下に振動する挙動を生じさせず、確定した寸法と座標を反映段階で一度だけ書く。
6. When 新しい表示寸法が現在の窓寸法と同一のとき, the 窓リサイズ・再吸着機構 shall 窓寸法・位置を変更しない（べき等）。
7. When サイズ変化に伴い対象キャラ窓を移動・リサイズするとき, the 窓リサイズ・再吸着機構 shall 随伴バルーン窓の追従 offset（既存 `BalloonFollow.offset`）を維持し、バルーンの正式な配置規則を新たに所有しない。
8. If 通知された対象キャラ窓が不在または窓生成前（`WindowHandle` 未付与）であるとき, then the 窓リサイズ・再吸着機構 shall 窓を破壊せずログ（`warn!` 以上）を残して no-op とする（silent failure を避ける・log-first 規律）。
9. If 通知された新しい表示寸法が非正（幅・高さ ≤ 0）であるとき, then the 窓リサイズ・再吸着機構 shall リサイズ・再吸着を行わず現状を保持しログを残す（`BottomSnapPolicy` の非正寸法縮退と整合）。

### Requirement 4: 実 DPI（≠96）本番ゴーストでの受け入れと決定論検証

**Objective:** As 受け入れ検証者, I want 本番ゴーストを実 DPI（≠96）で表示しサーフェス切替後も下端吸着が維持されることを確認でき、決定論部を純粋関数テストで固定できること, so that dpi=96 自己整合が隠す欠陥（window-placement リジェクトの教訓）の再発を防げる

#### Acceptance Criteria

1. While per-monitor v2 DPI が 96 以外（例 125%）で本番ゴースト（実 emo2・emo-present 経由の実 surface）を表示しているとき, the サーフェスサイズ追随機構 shall 初期挨拶サーフェス（`surface0`）から本体サーフェス（例 `\s[1000]`）への切替後もキャラ窓を画面下端へ吸着させ続ける（宙に浮かない・下端からずれない）。
2. If 受け入れ検証が dpi=96 のみで緑になっているとき, then the 受け入れ判定 shall 不合格とし、実 DPI（≠96）実機での目視証跡を必達とする。
3. The サーフェスサイズ追随機構 shall 単発デモ（ハードコード窓寸・架空 work area）でなく本番ゴースト表示に対して検証する（`areka-P0-window-placement` の本番ゴースト先行原則を継承）。
4. When サーフェス切替が起きたとき, the サーフェスサイズ追随機構 shall 決定論的に検証可能な部分（新寸法の窓反映・下端 Y 再計算・非正寸縮退・べき等・寸法差分判定）を DPI をパラメタ化（96／120／144／192）した純粋関数テストで固定でき、下端 Y 算出自体を headless で網羅する（実 DPI 目視は最終確認に留める）。

### Requirement 5: 境界と非所有

**Objective:** As spec 境界の維持者, I want 本 spec が所有しない隣接責務を明示すること, so that スコープ肥大と二重所有を避ける

#### Acceptance Criteria

1. The サーフェスサイズ追随機構 shall 初期表示サーフェスの選択・非表示既定（-1）を所有しない（`areka-P0-emo2-boot` #5 の領分。「最初に見えるサーフェス」がサイズ基準である前提を利用するのみ）。
2. The サーフェスサイズ追随機構 shall サーフェス合成・文字層・αマスク生成の中身を所有せず、表示寸法の変化のみを追随入力とする。
3. The サーフェスサイズ追随機構 shall 二人立ちの窓割当・本格的な相方連動（M-dual）を所有せず、同シームの再利用余地を残すに留める。
4. The サーフェスサイズ追随機構 shall バルーンの正式な配置規則・位置永続化（`position-persist`・M-life）を所有せず、既存 follow の offset を破壊しない範囲で追随する。
5. The サーフェスサイズ追随機構 shall ドラッグ機構そのもの（`event-drag-system`／`areka-P0-window-placement`）を再定義せず、既存の `DragPositionPolicy`／`BottomSnapPolicy`／`move_window_to` を再利用する。

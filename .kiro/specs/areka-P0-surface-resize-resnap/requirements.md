# Requirements Document

## Project Description (Input)

実機（実 emo2・実 DPI≠96）で、むらさき（scope0）が既定サーフェス `surface0`（434×687）を表示した後、さくらスクリプトの `\s[1000]` 等で本体サーフェスへ切り替わると、切替後サーフェスの寸法が初期サーフェスと異なるため、マスコットが画面下端に吸着しなくなる（宙に浮く／下端からずれる）。窓の位置・サイズは spawn 時に一度だけ確定し（`placement/measure.rs`＋`placement/spawn.rs`）、実行時のサーフェス切替（emo-present の `ShowSurface`）で窓を追随させるシームが無いのが原因。

本 spec の幹は **2 つの座標系とその間の変換 T の恒常維持**である。キャラ位置の真実は **シェル座標系（下端アンカー＝画面 work area 下端に接する基準）** で保持され、OS 窓の position/size は **ウィンドウ座標系（左上原点＋サーフェス寸法）** である。両者は変換 **T**（`window.size = surface 寸法` ／ `window.top_Y = work_area 下端 − surface 高さ`）で結ばれ、この T は既存 `BottomSnapPolicy`（Y = 下端 − h）が体現している。サーフェスが切り替わり寸法が変わるたびに T を再適用して窓座標を再導出し、下端アンカーを保つ。ドラッグ（アンカーを動かす）と resize（T の入力 surface 高さを変える）は同一の T・同一の単一位置ライター経路へ合流する。既存の採寸・spawn・drag ポリシーは再利用し、新規機構は「T の再適用トリガ（サーフェス寸法変化）」＋「T 再適用の反映口」に徹する。由来: 2026-07-13 M-boot（`areka-P0-emo2-boot`）実機サインオフ（R9.3）で発見の実機欠陥#1。

## Introduction

本ユニットは、実行時にキャラクターの表示サーフェスの寸法が変わっても、**シェル座標系の下端アンカーが保たれ続ける**（マスコットが画面下端に立ち続ける）ことを実現する。中核は **シェル座標系（下端基準）→ ウィンドウ座標系（サーフェス寸法基準）の変換 T の恒常維持**である。

キャラ位置の真実はシェル座標系の下端アンカーで保持され、OS 窓の position/size はサーフェス寸法に従属する投影結果にすぎない。サーフェスが切り替わり `surface.height` が変われば、`window.top_Y = work_area 下端 − surface.height` を再投影しなければ下端アンカーが保てない。ゆえに「サーフェス切替のたびに T をやり直す」ことが本 spec の本質である。この T は既存 `BottomSnapPolicy` が既に体現しており、「再吸着（re-snap）」とは新しい `surface.height` で T を再適用することにほかならない。

この framing の下で、ドラッグと resize は同一操作の別断面となる: ドラッグは**アンカー（シェル座標）を動かし**、resize は **T の入力である surface 高さを変える**。両者は同じ T・同じ**単一位置ライター**（`areka-P0-window-placement` 完了・既存の `DragPositionPolicy`／`BottomSnapPolicy`／`move_window_to`）へ合流し、事後補正の振動を避ける。

検知（⑥emo-present・表示寸法の source）と反映（⓪placement・窓所有者）は **同一 UI スレッド・同一 World** 上にあるため、両者を結ぶのは跨境界の「通信」ではなく同一 World 内のデータ依存である。したがって配送の実体（単方向メッセージ／同一 frame system 内の直接呼び／ECS 派生状態）は**要件段階では強制せず設計フェーズの判断に委ねる**（新規フレームワーク・新規依存は導入しない）。

検証は `areka-P0-window-placement` の本番ゴースト先行原則を継承し、**本番ゴースト（実 emo2・emo-present 経由の実 surface）を実 DPI（≠96・例 125%）で表示した上で「切替後も下端アンカー維持」を目視で確認**することを受け入れ条件とする（dpi=96 の自己整合が欠陥を隠す教訓を継承）。決定論的に検証可能な部分（T の再導出・下端 Y 再計算・非正寸縮退・べき等・寸法差分判定）は純粋関数テストで固定する。

## Boundary Context

- **In scope**: 実行時サーフェス寸法変化を T 再適用のトリガとして扱うこと（⑥emo-present の表示寸法を起点）／シェル座標系（下端アンカー）→ ウィンドウ座標系（サーフェス寸法）の変換 T の恒常維持／`BottomSnap` スコープでの T 再適用（下端 Y 再計算）と `free` スコープでの寸法のみ反映（Y 保持）／窓 size・位置更新の単一位置ライター経路への合流／随伴バルーン offset 維持／同寸のべき等・不在／非正寸のログ付き縮退／実 DPI（≠96）本番ゴースト目視受け入れ＋決定論部の純粋関数テスト。
- **Out of scope**: 初期表示サーフェスの選択・非表示既定（-1）＝`areka-P0-emo2-boot` の #5 で対応済み前提（「最初に見えるサーフェス」が T の入力寸法基準）／サーフェス合成・文字層・αマスク生成の中身／バルーンの正式な配置規則・バルーン窓の位置記憶（既存 follow が所有）／二人立ちの窓割当・本格連動（M-dual）／位置永続化（`position-persist`・M-life）／ドラッグ機構そのもの（`event-drag-system`／`window-placement` 完了）／サイズ変化を placement へ結ぶ**配送機構の実体選定**（同一スレッド同一 World ゆえ設計フェーズの DD-2 で確定）。
- **Adjacent expectations**: `areka-P0-window-placement`（完了）の採寸・spawn・`follow`（`DragPositionPolicy`／`BottomSnapPolicy`＝変換 T／`move_window_to` 単一ライター・`BalloonFollow`）を**消費・再利用し再定義しない**。`areka-P0-emo-present`（完了）は T の入力となる表示寸法の source（`ShowSurface` 適用で表示寸法が変化）。`areka-P0-emo2-boot`（#5 初期非表示修正が前提・frame.rs の drain phase が `PresentCommand` 適用点＝寸法変化の起点候補）と interlock。通信規約は `areka-P0-actor-foundation`（UI 配送ブリッジ）の既存資産の範囲で対応し新規フレームワークを導入しない。Downstream: M-dual が同 T・同ライターを再利用。emo2 は最小適合 fixture であって書式の聖典ではない（正典は ukadoc）。

## Requirements

### Requirement 1: シェル→ウィンドウ座標変換 T の恒常維持（中核）

**Objective:** As ⓪ghost（窓所有者）, I want キャラ位置をシェル座標系（下端アンカー）で保持し OS 窓の position/size をサーフェス寸法から変換 T で常に再導出すること, so that サーフェスが切り替わり寸法が変わっても下端アンカーが保たれ続ける

#### Acceptance Criteria

1. The サーフェスサイズ追随機構 shall キャラ窓の位置の真実をシェル座標系（下端アンカー＝現在窓が属するモニタの work area 下端に接する基準）で保持し、OS 窓の position/size を変換 T（`window.size = surface 寸法` ／ `window.top_Y = work_area 下端 − surface 高さ`）で導出する（既存 `BottomSnapPolicy` を T として再利用し再定義しない）。
2. When 実行時にサーフェスが切り替わり合成表示サーフェスの寸法（幅・高さ）が直前の表示寸法と異なるとき, the サーフェスサイズ追随機構 shall 新しいサーフェス寸法を入力に T を再適用し、対象キャラ窓の size と `top_Y` を再導出して下端アンカーを保つ。
3. The サーフェスサイズ追随機構 shall 窓 size と再吸着 Y の更新を、ドラッグ再吸着と同一の単一位置ライター経路（`DragPositionPolicy`／`BottomSnapPolicy` を用いる `move_window_to` 系の正規口）へ合流させ、`enqueue_window_move` を迂回した bypass 書込を新設しない。
4. The サーフェスサイズ追随機構 shall ドラッグ（アンカーを動かす）と resize（T の入力である surface 高さを変える）を同一の変換 T・同一の単一ライターで扱い、座標系変換の実装を二重化しない。
5. The サーフェスサイズ追随機構 shall 確定した size と座標を反映段階で一度だけ書き、サーフェス切替のたびに窓が上下に振動する挙動を生じさせない。

### Requirement 2: スコープ種別による T 適用の分岐と随伴バルーン

**Objective:** As ⓪ghost（placement）, I want スコープ種別に応じて T の適用を分岐すること, so that free 窓のユーザ位置を壊さず下端窓のみ再吸着し、随伴バルーンを破壊しない

#### Acceptance Criteria

1. When 対象キャラ窓が下端吸着スコープ（`Bottom`／`Seam`＝`BottomSnap` marker あり）であるとき, the サーフェスサイズ追随機構 shall T を適用し、新しい高さに基づく下端 Y（現在窓が属するモニタの work area 下端 − 新しい高さ）を再計算して窓 Y を更新し、窓下端を work area 下端へ保つ。
2. When 対象キャラ窓が `free`（`BottomSnap` marker なし）スコープであるとき, the サーフェスサイズ追随機構 shall アンカーがウィンドウ左上（自己座標）であることに従い窓寸法のみを新寸へ反映し、下端 Y の再計算は行わず現在 Y を保持する。
3. When サイズ変化に伴い対象キャラ窓を移動・リサイズするとき, the サーフェスサイズ追随機構 shall 随伴バルーン窓の追従 offset（既存 `BalloonFollow.offset`）を維持し、バルーンの正式な配置規則を新たに所有しない。

### Requirement 3: べき等と縮退（冗長回避・失敗経路）

**Objective:** As ⓪ghost（placement）, I want 同寸・不在・非正寸を安全に扱うこと, so that 不要な再配置・silent failure・振動を避ける

#### Acceptance Criteria

1. When 合成表示サーフェスの新しい寸法が現在の窓寸法と同一であるとき, the サーフェスサイズ追随機構 shall T の再適用・窓 size/位置の変更を行わない（べき等・冗長回避）。
2. The サーフェスサイズ追随機構 shall 追随入力を「表示サーフェスの実寸法」に限定し、寸法が同一なら合成内容・文字層・αマスクの中身が変化しても再配置を発火しない。
3. If 対象キャラ窓が不在または窓生成前（`WindowHandle` 未付与）であるとき, then the サーフェスサイズ追随機構 shall 窓を破壊せずログ（`warn!` 以上）を残して no-op とする（silent failure を避ける・log-first 規律）。
4. If 追随入力の新しいサーフェス寸法が非正（幅・高さ ≤ 0）であるとき, then the サーフェスサイズ追随機構 shall T の再適用を行わず現状を保持しログを残す（`BottomSnapPolicy` の非正寸法縮退と整合）。

### Requirement 4: T 再適用の起点と配送実体の非強制（制約）

**Objective:** As spec 実装者, I want サーフェス寸法変化を同一 UI スレッド・同一 World 内で placement 反映へ結ぶこと, so that 過剰な通信機構を導入せず新寸で T を再適用できる

#### Acceptance Criteria

1. The サーフェスサイズ追随機構 shall T 再適用の起点となる「現在の合成表示サーフェス寸法」を ⑥emo-present（表示寸法の source・`ShowSurface` 適用点）から取得し、placement へ渡す寸法を実際に適用されたサーフェスの寸法と一致させ、古い（適用前の）寸法で T を再適用しない。
2. Where 検知（⑥emo-present）と反映（⓪placement）が同一 UI スレッド・同一 World 上にあることを前提とするとき, the サーフェスサイズ追随機構 shall 寸法変化を placement 反映へ結ぶ配送実体（単方向メッセージ／同一 frame system 内の直接関数呼び／ECS 派生状態のいずれか）を設計フェーズの判断に委ね、要件段階では特定の配送機構を強制しない。
3. The サーフェスサイズ追随機構 shall 配送を既存プロジェクト内資産（`areka-P0-actor-foundation` の UI 配送ブリッジ等）の範囲で成立させ、新規の通信フレームワーク・新規 crates.io 依存を導入しない。
4. The サーフェスサイズ追随機構 shall 新しいサーフェス寸法が属するスコープ（どのキャラ窓か）を識別可能にし、追随の駆動入力を**シェル（キャラ本体）サーフェスの寸法**に限定して、バルーンサーフェスの寸法変化ではキャラ窓 resize を駆動しない。

### Requirement 5: 実 DPI（≠96）本番ゴーストでの受け入れと決定論検証

**Objective:** As 受け入れ検証者, I want 本番ゴーストを実 DPI（≠96）で表示しサーフェス切替後も下端アンカーが維持されることを確認でき、決定論部を純粋関数テストで固定できること, so that dpi=96 自己整合が隠す欠陥（window-placement リジェクトの教訓）の再発を防げる

#### Acceptance Criteria

1. While per-monitor v2 DPI が 96 以外（例 125%）で本番ゴースト（実 emo2・emo-present 経由の実 surface）を表示しているとき, the サーフェスサイズ追随機構 shall 既定サーフェス（`surface0`）から本体サーフェス（例 `\s[1000]`）への切替後もキャラ窓の下端アンカーを画面下端へ保ち続ける（宙に浮かない・下端からずれない）。ここで `surface0→\s[1000]` は「任意の寸法差を持つサーフェス切替」の一例であり、追随は特定サーフェス番号に依存しない。
2. If 受け入れ検証が dpi=96 のみで緑になっているとき, then the 受け入れ判定 shall 不合格とし、実 DPI（≠96）実機での目視証跡を必達とする。
3. The サーフェスサイズ追随機構 shall 単発デモ（ハードコード窓寸・架空 work area）でなく本番ゴースト表示に対して検証する（`areka-P0-window-placement` の本番ゴースト先行原則を継承）。
4. When サーフェス切替が起きたとき, the サーフェスサイズ追随機構 shall 決定論的に検証可能な部分（T による新寸の窓反映・下端 Y 再計算・非正寸縮退・べき等・寸法差分判定）を、多様な寸法・work area 値を網羅する純粋関数テストで固定し、下端 Y 算出を headless で網羅する（物理 px 単一通貨ゆえ下端 Y 算出は DPI 非依存であり、実 DPI 依存性の確認は R5.1–5.3 の実機目視へ委ねる）。

### Requirement 6: 境界と非所有

**Objective:** As spec 境界の維持者, I want 本 spec が所有しない隣接責務を明示すること, so that スコープ肥大と二重所有を避ける

#### Acceptance Criteria

1. The サーフェスサイズ追随機構 shall 初期表示サーフェスの選択・非表示既定（-1）を所有しない（`areka-P0-emo2-boot` #5 の領分。「最初に見えるサーフェス」が T の入力寸法基準である前提を利用するのみ）。
2. The サーフェスサイズ追随機構 shall サーフェス合成・文字層・αマスク生成の中身を所有せず、表示寸法の変化のみを T 再適用の入力とする。
3. The サーフェスサイズ追随機構 shall 二人立ちの窓割当・本格的な相方連動（M-dual）を所有せず、同 T・同ライターの再利用余地を残すに留める。
4. The サーフェスサイズ追随機構 shall バルーンの正式な配置規則・位置永続化（`position-persist`・M-life）を所有せず、既存 follow の offset を破壊しない範囲で追随する。
5. The サーフェスサイズ追随機構 shall ドラッグ機構そのもの（`event-drag-system`／`areka-P0-window-placement`）を再定義せず、既存の `DragPositionPolicy`／`BottomSnapPolicy`（＝変換 T）／`move_window_to` を再利用する。

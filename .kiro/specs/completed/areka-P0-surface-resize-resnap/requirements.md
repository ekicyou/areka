# Requirements Document

## Project Description (Input)

実機（実 emo2・実 DPI≠96）で、むらさき（scope0）が既定サーフェス `surface0`（434×687）を表示した後、さくらスクリプトの `\s[1000]` 等で本体サーフェスへ切り替わると、切替後サーフェスの寸法が初期サーフェスと異なるため、マスコットが画面下端に吸着しなくなる（宙に浮く／下端からずれる）。窓の位置・サイズは spawn 時に一度だけ確定し（`placement/measure.rs`＋`placement/spawn.rs`）、実行時のサーフェス切替（emo-present の `ShowSurface`）で窓を追随させるシームが無いのが原因。

本 spec の幹は **2 つの座標系とその間の変換 T の恒常維持**である。キャラ位置の真実は **シェル座標系（アンカー辺基準）** で保持され、OS 窓の position/size は **ウィンドウ座標系（左上原点＋サーフェス寸法）** である。アンカー辺はゴースト定義 `seriko.alignmenttodesktop`（ukadoc・既定 `bottom`・値は `top`／`bottom`／`left`／`right`／`free`）で決まり、変換 **T** は「アンカーされた辺を work area の対応辺へ固定したまま、新しいサーフェス寸法で窓の position/size を再導出する射影」である。既定 `bottom` は既存 `BottomSnapPolicy`（`top_Y = work_area 下端 − 高さ`）が体現しており、T はこれを 5 アンカーへ一般化したものである。サーフェスが切り替わり寸法が変わるたび、またはアンカーが実行時に変わる（`\![set,alignmenttodesktop,方向]`）たびに T を再適用してアンカー辺を保つ。ドラッグ（アンカーの自由軸を動かす）と resize（T の入力寸法を変える）は同一の T・同一の単一位置ライター経路へ合流する。既存の採寸・spawn・drag ポリシーは再利用し、新規機構は「T の再適用トリガ（サーフェス寸法変化・アンカー変化）」＋「T（5 アンカー射影）の反映口」に徹する。由来: 2026-07-13 M-boot（`areka-P0-emo2-boot`）実機サインオフ（R9.3）で発見の実機欠陥#1。

## Introduction

本ユニットは、実行時にキャラクターの表示サーフェスの寸法が変わっても、**シェル座標系のアンカー辺が保たれ続ける**（`seriko.alignmenttodesktop` の指定どおりデスクトップの指定辺へ吸着し続ける）ことを実現する。中核は **シェル座標系（アンカー辺基準）→ ウィンドウ座標系（サーフェス寸法基準）の変換 T の恒常維持**である。

キャラ位置の真実はシェル座標系のアンカー辺で保持され、OS 窓の position/size はサーフェス寸法に従属する投影結果にすぎない。サーフェスが切り替わり寸法が変われば、アンカー辺を work area の対応辺へ固定したまま T を再投影しなければアンカーが保てない。ゆえに「サーフェス切替・アンカー変更のたびに T をやり直す」ことが本 spec の本質である。

**アンカー辺は `seriko.alignmenttodesktop`（ukadoc・descript_ghost／descript_shell）が支配する**。既定は `bottom`、値は `top`／`bottom`／`left`／`right`／`free`。優先度チェーン（ゴースト全体 ＜ ゴースト側スコープ個別 ＜ シェル全体 ＜ シェル側スコープ個別）で解決され、さらに実行時に `\![set,alignmenttodesktop,方向]`（ゴースト終了まで有効・`default` で既定へ復帰）で動的に変わる。T は各アンカーで次の射影を行う（物理 px 単一通貨・`wa` = 現在窓が属するモニタの work area）:

| アンカー | 固定する辺 | 射影 | 寸法変化の駆動軸 | ドラッグ自由軸 |
|---|---|---|---|---|
| `bottom` | 下端 | `top_Y = wa.bottom − h`, X 保持 | 高さ h | 左右 |
| `top` | 上端 | `top_Y = wa.top`, X 保持 | （Y 不動・size のみ） | 左右 |
| `left` | 左端 | `left_X = wa.left`, Y 保持 | （X 不動・size のみ） | 上下 |
| `right` | 右端 | `left_X = wa.right − w`, Y 保持 | 幅 w | 上下 |
| `free` | なし | position 保持・size のみ | （なし） | 全方向 |

既定 `bottom` は既存 `BottomSnapPolicy` そのもの＝「再吸着（re-snap）」とは新しい寸法で T を再適用すること。この framing の下でドラッグと resize は同一操作の別断面となる: ドラッグはアンカーの自由軸を動かし、resize は T の入力寸法を変える。両者は同じ T・同じ**単一位置ライター**（`areka-P0-window-placement` 完了・既存の `DragPositionPolicy`／`BottomSnapPolicy`／`move_window_to`）へ合流し、事後補正の振動を避ける。

検知（⑥emo-present・表示寸法の source）と反映（⓪placement・窓所有者）は **同一 UI スレッド・同一 World** 上にあるため、両者を結ぶのは跨境界の「通信」ではなく同一 World 内のデータ依存である。配送の実体（単方向メッセージ／同一 frame system 内の直接呼び／ECS 派生状態）は**要件段階では強制せず設計フェーズの判断に委ねる**（新規フレームワーク・新規依存は導入しない）。アンカー値の解決（優先度チェーンの読取り）と `\![set,alignmenttodesktop]` の routing は上流（parsers／seriko／window-placement）が所有し、本 spec は解決済みアンカーと寸法を入力として T を維持する。

検証は `areka-P0-window-placement` の本番ゴースト先行原則を継承し、**本番ゴースト（実 emo2・emo-present 経由の実 surface）を実 DPI（≠96・例 125%）で表示した上で、M-boot 欠陥に対応する `bottom` アンカーで「切替後もアンカー維持」を目視で確認**することを受け入れ条件とする（dpi=96 の自己整合が欠陥を隠す教訓を継承）。決定論的に検証可能な部分（各アンカーの T 射影・アンカー辺 Y/X 再計算・非正寸縮退・べき等・寸法差分判定）は純粋関数テストで全アンカー網羅する。

## Boundary Context

- **In scope**: 実行時サーフェス寸法変化・アンカー変化を T 再適用のトリガとして扱うこと（⑥emo-present の表示寸法＋解決済みアンカーを起点）／シェル座標系（アンカー辺）→ ウィンドウ座標系（サーフェス寸法）の変換 T（5 アンカー射影＝`top`／`bottom`／`left`／`right`／`free`）の恒常維持／各アンカーでのアンカー辺 Y/X 再計算（`bottom` は既存 `BottomSnapPolicy` の再適用・他アンカーはその一般化）と `free` の寸法のみ反映／窓 size・位置更新の単一位置ライター経路への合流／随伴バルーン offset 維持／同寸のべき等・不在／非正寸のログ付き縮退／実 DPI（≠96）本番ゴースト目視受け入れ（`bottom`）＋全アンカーの決定論純粋関数テスト。
- **Out of scope**: 初期表示サーフェスの選択・非表示既定（-1）＝`areka-P0-emo2-boot` の #5 で対応済み前提／サーフェス合成・文字層・αマスク生成の中身／表示倍率変更 `\![set,scaling]` による実効寸法変化（隣接の別トリガ・本 spec は寸法値の変化のみを一般に受ける）／`seriko.alignmenttodesktop` **優先度チェーンの読取り・解決**と `\![set,alignmenttodesktop]` cue の **routing**（parsers／seriko／window-placement の領分・本 spec は解決済みアンカーを消費）／バルーンの正式な配置規則・バルーン窓の位置記憶（既存 follow が所有）／二人立ちの窓割当・本格連動（M-dual）／位置永続化（`position-persist`・M-life）／ドラッグ機構そのもの（`event-drag-system`／`window-placement` 完了）／サイズ変化を placement へ結ぶ**配送機構の実体選定**（同一スレッド同一 World ゆえ設計フェーズの DD-2）。
- **Adjacent expectations**: `areka-P0-window-placement`（完了）の採寸・spawn・`follow`（`DragPositionPolicy`／`BottomSnapPolicy`＝T の `bottom` 事例／`move_window_to` 単一ライター・`BalloonFollow`・`BottomSnap` marker）を**消費・再利用し再定義しない**（T は `BottomSnapPolicy` を 5 アンカーへ一般化）。`areka-P0-emo-present`（完了）は T の入力となる表示寸法の source（`ShowSurface` 適用で表示寸法が変化）。`areka-P0-emo2-boot`（#5 初期非表示修正が前提・frame.rs の drain phase が `PresentCommand` 適用点＝寸法変化の起点候補）と interlock。アンカー値は `seriko.alignmenttodesktop`（ukadoc・既定 `bottom`）由来で優先度チェーン解決＋実行時 `\![set,alignmenttodesktop]` で可変。通信規約は `areka-P0-actor-foundation`（UI 配送ブリッジ）の既存資産の範囲で対応し新規フレームワークを導入しない。Downstream: M-dual が同 T・同ライターを再利用。emo2 は最小適合 fixture であって書式の聖典ではない（正典は ukadoc）。

## Requirements

### Requirement 1: アンカー射影 T の恒常維持（中核）

**Objective:** As ⓪ghost（窓所有者）, I want キャラ位置をシェル座標系（アンカー辺）で保持し OS 窓の position/size をサーフェス寸法から変換 T（5 アンカー射影）で常に再導出すること, so that サーフェスが切り替わり寸法が変わっても `seriko.alignmenttodesktop` の指定どおりアンカー辺が保たれ続ける

#### Acceptance Criteria

1. The サーフェスサイズ追随機構 shall キャラ窓の位置の真実をシェル座標系（アンカー辺＝現在窓が属するモニタの work area の対応辺）で保持し、OS 窓の position/size を変換 T（アンカーされた辺を work area の対応辺へ固定したまま新しいサーフェス寸法で position/size を再導出する射影）で導出する。
2. The サーフェスサイズ追随機構 shall T の `bottom` アンカー射影に既存 `BottomSnapPolicy`（`top_Y = work_area 下端 − 高さ`）を再利用し再定義せず、`top`／`left`／`right`／`free` はその一般化として同一の射影規約（アンカー辺固定・非アンカー軸保持）で定義する。
3. When 実行時にサーフェスが切り替わり合成表示サーフェスの寸法（幅・高さ）が直前の表示寸法と異なるとき, the サーフェスサイズ追随機構 shall 対象キャラ窓の現在アンカーに対応する T を再適用し、window の size と position を再導出してアンカー辺を保つ。
4. When 実行時に対象キャラ窓のアンカー（`seriko.alignmenttodesktop`）が変わったとき, the サーフェスサイズ追随機構 shall 新しいアンカーに対応する T を現在の表示寸法で再適用し、新しいアンカー辺を work area の対応辺へ合わせる。
5. The サーフェスサイズ追随機構 shall window の size と位置の更新を、ドラッグ再吸着と同一の単一位置ライター経路（`DragPositionPolicy`／`BottomSnapPolicy` を用いる `move_window_to` 系の正規口）へ合流させ、`enqueue_window_move` を迂回した bypass 書込を新設しない。
6. The サーフェスサイズ追随機構 shall ドラッグ（アンカーの自由軸を動かす）と resize（T の入力寸法を変える）を同一の変換 T・同一の単一ライターで扱い、座標系変換の実装を二重化しない。
7. The サーフェスサイズ追随機構 shall 確定した size と座標を反映段階で一度だけ書き、サーフェス切替・アンカー変更のたびに窓が振動する挙動を生じさせない。

### Requirement 2: アンカー別の T 射影規則と随伴バルーン

**Objective:** As ⓪ghost（placement）, I want `seriko.alignmenttodesktop` の各値に応じて T の射影を分岐すること, so that 指定辺への吸着を保ちつつ非アンカー軸のユーザ位置を壊さず、随伴バルーンを破壊しない

#### Acceptance Criteria

1. When 対象キャラ窓のアンカーが `bottom` であるとき, the サーフェスサイズ追随機構 shall `top_Y = wa.bottom − 新しい高さ` を再計算して窓 Y を更新し、X（左右位置）は保持して、窓下端を work area 下端へ保つ。
2. When 対象キャラ窓のアンカーが `top` であるとき, the サーフェスサイズ追随機構 shall 窓上端を `wa.top` へ固定し、X を保持し、寸法のみを新寸へ反映する。
3. When 対象キャラ窓のアンカーが `left` であるとき, the サーフェスサイズ追随機構 shall 窓左端を `wa.left` へ固定し、Y を保持し、寸法のみを新寸へ反映する。
4. When 対象キャラ窓のアンカーが `right` であるとき, the サーフェスサイズ追随機構 shall `left_X = wa.right − 新しい幅` を再計算して窓 X を更新し、Y を保持して、窓右端を work area 右端へ保つ。
5. When 対象キャラ窓のアンカーが `free` であるとき, the サーフェスサイズ追随機構 shall アンカー辺を持たないことに従い窓寸法のみを新寸へ反映し、position（X・Y）の再計算は行わず現在位置を保持する。
6. When サイズ変化・アンカー変化に伴い対象キャラ窓を移動・リサイズするとき, the サーフェスサイズ追随機構 shall 随伴バルーン窓の追従 offset（既存 `BalloonFollow.offset`）を維持し、バルーンの正式な配置規則を新たに所有しない。

### Requirement 3: べき等と縮退（冗長回避・失敗経路）

**Objective:** As ⓪ghost（placement）, I want 同寸・不在・非正寸を安全に扱うこと, so that 不要な再配置・silent failure・振動を避ける

#### Acceptance Criteria

1. When 合成表示サーフェスの新しい寸法が現在の窓寸法と同一、かつアンカーも不変であるとき, the サーフェスサイズ追随機構 shall T の再適用・窓 size/位置の変更を行わない（べき等・冗長回避）。
2. The サーフェスサイズ追随機構 shall 追随入力を「表示サーフェスの実寸法」と「解決済みアンカー」に限定し、寸法・アンカーがともに同一なら合成内容・文字層・αマスクの中身が変化しても再配置を発火しない。
3. If 対象キャラ窓が不在または窓生成前（`WindowHandle` 未付与）であるとき, then the サーフェスサイズ追随機構 shall 窓を破壊せずログ（`warn!` 以上）を残して no-op とする（silent failure を避ける・log-first 規律）。
4. If 追随入力の新しいサーフェス寸法が非正（幅・高さ ≤ 0）であるとき, then the サーフェスサイズ追随機構 shall T の再適用を行わず現状を保持しログを残す（`BottomSnapPolicy` の非正寸法縮退と整合）。

### Requirement 4: T 入力の起点・アンカー provenance・配送実体の非強制（制約）

**Objective:** As spec 実装者, I want サーフェス寸法変化と解決済みアンカーを同一 UI スレッド・同一 World 内で placement 反映へ結ぶこと, so that 過剰な通信機構を導入せず新寸・新アンカーで T を再適用できる

#### Acceptance Criteria

1. The サーフェスサイズ追随機構 shall T 再適用の入力寸法となる「現在の合成表示サーフェス寸法」を ⑥emo-present（表示寸法の source・`ShowSurface` 適用点）から取得し、placement へ渡す寸法を実際に適用されたサーフェスの寸法と一致させ、古い（適用前の）寸法で T を再適用しない。
2. The サーフェスサイズ追随機構 shall 対象キャラ窓の現在アンカーを、`seriko.alignmenttodesktop` の優先度チェーン解決（parsers／window-placement）と実行時 `\![set,alignmenttodesktop]`（seriko）による**解決済みアンカー**として消費し、優先度チェーンの読取り・cue routing 自体は所有しない。
3. Where 検知（⑥emo-present）と反映（⓪placement）が同一 UI スレッド・同一 World 上にあることを前提とするとき, the サーフェスサイズ追随機構 shall 寸法・アンカー変化を placement 反映へ結ぶ配送実体（単方向メッセージ／同一 frame system 内の直接関数呼び／ECS 派生状態のいずれか）を設計フェーズの判断に委ね、要件段階では特定の配送機構を強制しない。
4. The サーフェスサイズ追随機構 shall 配送を既存プロジェクト内資産（`areka-P0-actor-foundation` の UI 配送ブリッジ等）の範囲で成立させ、新規の通信フレームワーク・新規 crates.io 依存を導入しない。
5. The サーフェスサイズ追随機構 shall 新しいサーフェス寸法が属するスコープ（どのキャラ窓か）を識別可能にし、追随の駆動入力を**シェル（キャラ本体）サーフェスの寸法**に限定して、バルーンサーフェスの寸法変化ではキャラ窓 resize を駆動しない。

### Requirement 5: 実 DPI（≠96）本番ゴーストでの受け入れと決定論検証

**Objective:** As 受け入れ検証者, I want 本番ゴーストを実 DPI（≠96）で表示しサーフェス切替後もアンカーが維持されることを確認でき、全アンカーの決定論部を純粋関数テストで固定できること, so that dpi=96 自己整合が隠す欠陥（window-placement リジェクトの教訓）の再発を防げる

#### Acceptance Criteria

1. While per-monitor v2 DPI が 96 以外（例 125%）で本番ゴースト（実 emo2・emo-present 経由の実 surface）を `bottom` アンカーで表示しているとき, the サーフェスサイズ追随機構 shall 既定サーフェス（`surface0`）から本体サーフェス（例 `\s[1000]`）への切替後もキャラ窓の下端アンカーを画面下端へ保ち続ける（宙に浮かない・下端からずれない）。ここで `surface0→\s[1000]` は「任意の寸法差を持つサーフェス切替」の一例であり、追随は特定サーフェス番号に依存しない。
2. If 受け入れ検証が dpi=96 のみで緑になっているとき, then the 受け入れ判定 shall 不合格とし、実 DPI（≠96）実機での目視証跡を必達とする。
3. The サーフェスサイズ追随機構 shall 単発デモ（ハードコード窓寸・架空 work area）でなく本番ゴースト表示に対して検証する（`areka-P0-window-placement` の本番ゴースト先行原則を継承）。
4. When サーフェス切替・アンカー変化が起きたとき, the サーフェスサイズ追随機構 shall 決定論的に検証可能な部分（各アンカー `top`／`bottom`／`left`／`right`／`free` の T 射影・アンカー辺 Y/X 再計算・非正寸縮退・べき等・寸法差分判定）を、多様な寸法・work area 値を網羅する純粋関数テストで全アンカー固定し、headless で網羅する（物理 px 単一通貨ゆえアンカー辺算出は DPI 非依存であり、実 DPI 依存性の確認は R5.1–5.3 の `bottom` 実機目視へ委ねる）。
5. When 実 DPI（≠96）本番ゴーストで `bottom` アンカー表示中にサーフェス切替（resize）が起きたとき, the サーフェスサイズ追随機構 shall 切替直後にキャラ窓が振動せず一度書きで確定位置へ収束すること、および resize 後もバルーン窓の透過ヒット（αマスクのクリックスルー）が維持されることを、アンカー維持（R5.1）とは独立した実 DPI 実機の目視観測項目として満たし、`WM_WINDOWPOSCHANGED` echo の二重反映による退行（窓振動・バルーンのクリック死）を受け入れ退行ゲートとする。

### Requirement 6: 境界と非所有

**Objective:** As spec 境界の維持者, I want 本 spec が所有しない隣接責務を明示すること, so that スコープ肥大と二重所有を避ける

#### Acceptance Criteria

1. The サーフェスサイズ追随機構 shall 初期表示サーフェスの選択・非表示既定（-1）を所有しない（`areka-P0-emo2-boot` #5 の領分。「最初に見えるサーフェス」が T の入力寸法基準である前提を利用するのみ）。
2. The サーフェスサイズ追随機構 shall サーフェス合成・文字層・αマスク生成の中身、および表示倍率変更 `\![set,scaling]` の機構を所有せず、表示寸法の変化のみを T 再適用の入力とする。
3. The サーフェスサイズ追随機構 shall `seriko.alignmenttodesktop` 優先度チェーンの読取り・解決と `\![set,alignmenttodesktop]` cue の routing を所有せず（parsers／seriko／window-placement の領分）、解決済みアンカーを消費するに留める。
4. The サーフェスサイズ追随機構 shall 二人立ちの窓割当・本格的な相方連動（M-dual）を所有せず、同 T・同ライターの再利用余地を残すに留める。
5. The サーフェスサイズ追随機構 shall バルーンの正式な配置規則・位置永続化（`position-persist`・M-life）を所有せず、既存 follow の offset を破壊しない範囲で追随する。
6. The サーフェスサイズ追随機構 shall ドラッグ機構そのもの（`event-drag-system`／`areka-P0-window-placement`）を再定義せず、既存の `DragPositionPolicy`／`BottomSnapPolicy`（＝T の `bottom` 事例）／`move_window_to` を再利用する。

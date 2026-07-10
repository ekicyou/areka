# Requirements Document

## Introduction

本ユニットは、⓪ ghost（ゴーストエンジン）が所有する窓ライフサイクル（lifecycle／窓配置／位置永続化）のうち「窓配置」を実装で埋める。ゴースト定義（descript）とスコープ数から**キャラ窓（scope0 主体＋scope1 相方）と各スコープ対応のバルーン窓**を生成し、ukadoc 準拠の既定位置（`seriko.alignmenttodesktop` カスケード）へ配置し、**全面ドラッグ（バルーン追従含む）**できる機構を提供する。窓数はハードコードでなく構成入力から決定する。

検証は**本番ゴースト（emo2 の実 surface 表示＝emo-present 経由）を実 DPI（per-monitor v2・dpi≠96、例 125%）で表示した上で、それに対して**行う。2026-07-05 のリジェクト（物理 px＝`Monitor.work_area`/`WindowPos` と論理 DIP＝`BoxStyle` の単位混在で既定位置解決が窓を沈め、ドラッグが二重スケールで画面外へ消失、しかも dpi=96 のテスト緑が欠陥を隠した）を教訓に、**実 DPI（≠96）実行を受け入れの必達条件**とする。単発デモ（ハードコード窓・架空 work area）への合わせ込みは禁止する。

## Boundary Context

- **In scope**: 窓生成の機構化（スコープ数対応・バルーン窓含む）／`seriko.alignmenttodesktop` 既定位置のカスケード解決（emo2 使用値の実挙動）／work area 計算／全面ドラッグ＋バルーン追従（暫定 offset）／既定 z-order（非 topmost）／実 DPI（≠96）での位置・移動の正しさ／生成した窓 entity の後続（emo2-boot）への受け渡し／窓移動の UI スレッド上公開関数。
- **Out of scope**: 位置永続化 `ghost.dat`（position-persist・M-life）／二人立ちの surface 連動・本格結線（M-dual）／`\![move]` キャラ移動（sakura-dialogue-tags 結合クラスタ）／バルーンの正式配置規則（balloon 表示系）／surface 描画・合成の中身（emo-surface）／メニュー・chrome（M2）／`EmoPresenter` の装着呼出し（emo2-boot の領分）／実 sink 差し替え（emo2-boot）。
- **Adjacent expectations**: 本ユニットは Window entity を生成して返すのみで、装着（`EmoPresenter::attach_target`）は emo2-boot が行う。ukadoc 値域の全量・SSP de-facto 挙動（scope 相対配置・z-order 実挙動・`defaultx`⇔`defaultleft` の同義/別義・X 原点・バルーン推奨 DPI の扱い）は design 段階で正典（ukadoc）と SSP 実挙動を確認して確定する。並走中の `areka-P0-emo-text-layer` とは改変ファイルが交差しない（本ユニットは `crates/areka` を、あちらは `crates/areka-emo-present`/`crates/areka-parsers` を所有）。

## Requirements

### Requirement 1: 構成駆動の窓生成

**Objective:** As a ゴーストエンジン（⓪ghost）, I want ゴースト定義とスコープ数からキャラ窓・バルーン窓を生成できること, so that ハードコードした窓数でなく構成に従って本物のゴースト窓を生やせる

#### Acceptance Criteria
1. When ゴースト定義（shell dir と shell/ghost descript の KV）が供給されたとき, the 窓配置機構 shall スコープ数に対応する数のキャラ窓を生成する。
2. When ゴースト定義が供給されたとき, the 窓配置機構 shall 各キャラ窓（スコープ）に対応するバルーン窓を 1 つずつ生成する（ukadoc 正典: `sakura.balloon.alignment`＝本体側の吹き出し／`kero.balloon.alignment`＝相方側の吹き出しに準拠し、バルーンはスコープごとに 1 枚存在する。正式な左右配置規則は balloon 表示系の後続へ委ね、本ユニットは暫定 offset 追従まで）。
3. The 窓配置機構 shall 生成する窓数をハードコードせず構成入力から決定する。
4. When 本番ゴースト（emo2 の実 surface）が供給されたとき, the 窓配置機構 shall 起動窓シーム（`open_startup_window`）でダミー窓を本物のゴースト窓へ置き換える。
5. When キャラ窓を生成するとき, the 窓配置機構 shall 初期位置・追従 offset をデモ由来の固定座標値（例 (400,200) や (335,0)）から持ち込まず、既定位置解決の結果を用いる。

### Requirement 2: 既定位置の解決（alignmenttodesktop カスケード）

**Objective:** As a ゴーストエンジン, I want descript の配置キーを 4 層カスケードで解決し work area 基準の既定位置を決めること, so that ゴーストが ukadoc 準拠の既定位置に出現する

#### Acceptance Criteria
1. When キャラ窓を配置するとき, the 窓配置機構 shall Windows の work area（タスクバー除外）を基準に既定位置を計算する。
2. When `seriko.alignmenttodesktop` の値が明示されていないとき, the 窓配置機構 shall 既定値 `bottom` を適用し、キャラ窓を work area の下端へ整列する。
3. The 窓配置機構 shall 配置キーの優先順位を「ghost 全体 ＜ ghost スコープ別（`sakura.seriko.*`／`kero.seriko.*`）＜ shell 全体 ＜ shell スコープ別（`char*.seriko.*`）」のカスケードで解決する。
4. When `alignmenttodesktop` が `bottom`（下端整列）のとき, the 窓配置機構 shall Y 座標を work area 下端に固定し、`defaulttop` を無視する。
5. When descript が `defaultx`（または `defaultleft`）を指定しているとき, the 窓配置機構 shall それを X 初期座標へ反映する（下端整列時も X 調整として有効）。
6. When `alignmenttodesktop` が `free` のとき, the 窓配置機構 shall `defaulttop`／`defaultleft` を有効な X/Y 初期座標として適用する。
7. The 窓配置機構 shall 配置キーの両表記（`defaultx`⇔`defaultleft`、`defaulty`⇔`defaulttop`）を寛容に受理する。
8. Where emo2 が使用しない配置値・キー（例 未使用の `alignmenttodesktop` 値域）が指定されたとき, the 窓配置機構 shall 実挙動を持たずシームとして受理する（最小実装＋拡張シーム）。

### Requirement 3: 座標単位契約と実 DPI 検証

**Objective:** As a 受け入れ検証者, I want 実 DPI（≠96）で窓が既定位置に正しく出現しドラッグが破綻しないことを確認できること, so that 物理 px と論理 DIP の単位混在（2026-07-05 リジェクトの直接原因）の再発を防げる

#### Acceptance Criteria
1. While per-monitor v2 DPI が 96 以外（例 125%）で動作しているとき, the 窓配置機構 shall キャラ窓を work area 基準の既定位置（既定 `bottom`）へ画面内に正しく出現させる。
2. The 窓配置機構 shall 物理ピクセルと論理 DIP を混在させた座標演算を行わない。
3. When 既定位置を計算するとき, the 窓配置機構 shall 入出力の座標単位を一貫して扱い、二重スケールを生じさせない。
4. The 既定位置解決器 shall DPI をパラメタ化（96／120／144／192）した単体テストで物理/論理変換を固定でき、純粋関数として検証可能である。
5. If 受け入れ検証が dpi=96 のみで緑になっているとき, then the 受け入れ判定 shall 不合格とし、実 DPI（≠96）実行の証跡を必達とする。

### Requirement 4: 全面ドラッグとバルーン追従

**Objective:** As a ユーザー, I want キャラ窓を surface 全面からドラッグで移動でき、バルーン窓が追従すること, so that マスコットを自由に配置できる

#### Acceptance Criteria
1. When ユーザーがキャラ窓の surface 上でドラッグを開始したとき, the 窓配置機構 shall キャラ窓を全面ドラッグで移動させる（修飾キー不要）。
2. While キャラ窓がドラッグされているとき, the 窓配置機構 shall バルーン窓をキャラ窓へ暫定 offset で追従させる。
3. When 実 DPI（≠96）でドラッグしているとき, the 窓配置機構 shall 窓を画面外へ消失させず、一貫した移動量で移動・追従させる。
4. The 窓配置機構 shall バルーンの正式な配置規則を所有せず、暫定 offset のみを提供する（正式規則は balloon 表示系の後続へ委ねる）。

### Requirement 5: 既定 z-order

**Objective:** As a ゴーストエンジン, I want 生成窓の既定 z-order を非 topmost とすること, so that SSP の de-facto 挙動と整合する

#### Acceptance Criteria
1. When キャラ窓・バルーン窓を生成するとき, the 窓配置機構 shall 既定 z-order を非 topmost とする。
2. Where `seriko.zorder`／`seriko.sticky-window` が指定されたとき, the 窓配置機構 shall emo2 が使用しない限りシームのみとし、実挙動を実装しない。

### Requirement 6: 後続への窓引き渡し

**Objective:** As a M-boot 統合（emo2-boot）, I want 生成された Window entity をスコープ別キャラ窓・バルーン窓の双方について識別可能な形で受け取れること, so that 各窓へ `EmoPresenter` を装着できる

#### Acceptance Criteria
1. When 窓を生成したとき, the 窓配置機構 shall スコープ別のキャラ窓とスコープ別のバルーン窓の Window entity を後続が取得可能な形で公開する。
2. The 窓配置機構 shall 各窓を「スコープ×種別（キャラ窓／バルーン窓）」で識別できるキーで公開し、スコープ番号のみでは各スコープのバルーン窓を取り出せない事態を避ける。
3. The 窓配置機構 shall `EmoPresenter` の装着呼出しを自ら行わず、装着は emo2-boot の領分とする。

### Requirement 7: 窓移動の公開 API（UI スレッド）

**Objective:** As a 将来の他アクター（sakura 発 `\![move]`・二人立ち連動等）, I want 窓移動を UI スレッド上で呼ばれる関数として利用できること, so that UI 配送ブリッジ経由で後から窓移動指令を届けられる

#### Acceptance Criteria
1. The 窓配置機構 shall 窓移動操作を、UI スレッド上で呼び出される関数として公開する。
2. While 窓の生成・移動・z-order 操作を行うとき, the 窓配置機構 shall それらを UI スレッド専有で実行する。
3. The 窓配置機構 shall 本ユニットでは actor 依存を持たず、UI 配送ブリッジ（`spawn_ui`／`UiSender`）との結線は後続に委ねる。

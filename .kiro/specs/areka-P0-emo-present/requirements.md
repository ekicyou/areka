# Requirements Document

## Project Description (Input)
⑥ emo トラック直列チェーン 3/3（emo-atlas → emo-compose → emo-present）。合成コア（emo-compose ✅ 完了）が生む 1枚物 premultiplied BGRA ビットマップを画面に出す結線を実装する。すなわち wintf への表示供給・クリックスルー用 AlphaMask の合成結果からの生成と同期更新・surface id 切替の指令 API・合成キャッシュ（＋無効化口）を備え、旧 emo-surface のゴール（surface0＋バルーン枠の表示）を専用 example で完走させる。合成は emo 自前・アトラス転写・1枚物が方針正本。窓あたり visual は最小限（入れ子 Visual 合成不使用・text-layer 用スロットのみ予約）。表示・AlphaMask の観測は実 DPI（dpi≠96）でも実施する。WUC 更新は UI スレッド固定。emo-compose は純粋関数のまま・キャッシュは本層が所有する。

## Introduction

areka の ⑥ emo トラック直列チェーンの最終段（3/3・emo-atlas → emo-compose → **emo-present**）である。上流の合成コア（emo-compose ✅ 完了）は surface id と bind 集合から 1枚物の premultiplied BGRA ビットマップ（`ComposedSurface`）を純粋関数として生成するが、それを**画面に出す結線が存在しない**。本ユニットは、合成済みバッファを wintf の窓へ表示し、クリックスルー用の当たり判定マスク（AlphaMask）を同じ合成結果から生成・同期し、surface id を切り替える指令 API と合成結果のキャッシュを備える。これにより旧 emo-surface のゴール——**emo2 fixture の surface0 とバルーン枠を表示し、キャラ不透明領域のみクリック捕捉・透明域は背後プロセスへ透過**——を専用 example で完走させる。

合成そのもの・アトラス構築・窓の既定位置決め・テキスト描画・SERIKO 再生は本ユニットの責務外である。指令 API の形は下流の `areka-P0-seriko-engine`（並走）が呼び手として消費する契約の正本であり、本ドキュメントがその形を定める。

## Boundary Context

- **In scope（本ユニットが観測可能に実現する振る舞い）**:
  - 合成済み 1枚物 BGRA バッファをメモリから wintf の窓へ表示する（ファイル読込ではなくメモリ供給）。
  - 表示中の合成結果からクリックスルー用の当たり判定マスクを生成し、キャラ不透明領域のみクリック捕捉・透明域は背後へ透過させる。
  - surface id（＋bind 集合）を運ぶ指令で表示中の surface を切り替える。非表示（`\s[-1]` 相当）状態への遷移を含む。
  - 切替時に表示バッファと当たり判定マスクを対で入れ替える（原子性）。
  - surface id → 合成結果のキャッシュと、その無効化の口。
  - balloons*.png 等のバルーン枠を fixture 直指定で同一機構により表示する。
  - 合成済みサーフェスを本レイヤが所有する読み戻し可能なオフスクリーン面（自前 swap chain 相当）としてコンポジターへ供給し、その面の CPU 読み戻し経路を確保する。
  - 上記を実 DPI（dpi≠96）でも観測可能にする専用 example。
- **Out of scope（本ユニットが所有しない）**:
  - surface 合成の実体（`areka-P0-emo-compose`）／アトラス構築（`areka-P0-emo-atlas`）。
  - 窓の既定位置・配置・ドラッグの機構化（`areka-P0-window-placement`。example 内の窓は観測用の仮設）。
  - バルーン内テキスト描画・テキスト有効描画領域の定義消費（`areka-P0-emo-text-layer`）。
  - SERIKO ループ／interval／blink・surface 状態の所有（`areka-P0-seriko-engine` ほか）。
  - 指令の channel/actor 契約の確定・呼び手の結線（kanade/seriko 結線時）。
  - arrow/marker/online などバルーン付随マーカーの表示（後続）。
  - オフスクリーン面を直読みしてヒットテスト当たり判定を実導出すること（本ユニットは R8 で読み戻し経路の確保に留め、直読みヒットテストの実導出は後続。M-boot の当たり判定は R2 の `ComposedSurface` CPU バイト経由）。
- **Adjacent expectations（隣接ユニットへの期待・依存）**:
  - `areka-P0-emo-compose` の実シンボル（`ComposedSurface`・`Composer::compose_into`/`compose`・`EmoWorld`・`BindSet`・`AtlasTable`）を再定義せず消費する。合成出力は premultiplied BGRA であることに依存する。
  - wintf の表示・当たり判定・クリックスルー基盤（メモリ供給可能な表示経路・α マスク当たり判定・`WS_EX_TRANSPARENT` 動的トグル）を消費する。WUC 系の更新は UI スレッド固定である前提に従う。
  - `areka-P0-seriko-engine` は本ユニットの指令 API の呼び手であり、非表示遷移（`\s[-1]`）を発行できる必要がある。本ユニットは非表示の意味論を指令 API に持つ。
  - `areka-P0-window-placement` が将来生成する窓へそのまま装着できるよう、表示装着 API は窓ハンドル（Window entity/handle）を受け取る形とする。

## Requirements

### Requirement 1: メモリ供給による合成済みサーフェスの表示

**Objective:** As a emo ランタイム, I want 合成済みの 1枚物 BGRA バッファを窓へ表示できること, so that emo-compose の出力を画面上のマスコットとして観測できる

#### Acceptance Criteria

1. When 合成済みの `ComposedSurface`（premultiplied BGRA・幅/高さ/ストライド/バイト列を持つ）が表示装着 API に渡される, the emo-present レイヤ shall そのバッファを指定された窓の表示内容として反映する。
2. The emo-present レイヤ shall 表示にあたり合成結果のピクセル形式（premultiplied BGRA）を変換せずそのまま供給する。
3. When 表示装着 API が窓ハンドル（Window entity/handle）を伴って呼び出される, the emo-present レイヤ shall その窓に対して表示内容を装着する。
4. The emo-present レイヤ shall 窓あたりの表示レイヤ構成を最小限（surface 本体レイヤ＋後続テキスト層のための予約口）に留め、入れ子の合成レイヤを表示のために増設しない。
5. When surface の原寸（幅・高さ）が前回表示と異なる, the emo-present レイヤ shall 表示領域を新しい原寸に追随させる。
6. While DPI が 96 以外である, the emo-present レイヤ shall surface を等倍（合成時の物理ピクセル基準）で表示する。

### Requirement 2: クリックスルー用当たり判定マスクの生成と同期

**Objective:** As a ユーザ, I want マスコットのキャラ不透明領域だけがクリックを受け、透明領域のクリックは背後の別プロセスへ透過すること, so that デスクトップマスコットとして自然に操作できる

#### Acceptance Criteria

1. When 合成済みサーフェスが表示される, the emo-present レイヤ shall 同じ合成結果（premultiplied BGRA）から当たり判定マスクを生成し、当たり判定へ供給する。
2. While マスコットが表示されている, when ユーザがキャラ不透明領域上をクリックする, the emo-present レイヤ shall そのクリックを当該窓が捕捉する状態を維持する。
3. While マスコットが表示されている, when ユーザが透明領域上をクリックする, the emo-present レイヤ shall そのクリックを背後の別プロセスへ透過させる状態を維持する。
4. When 表示中の surface が切り替わる, the emo-present レイヤ shall 表示バッファと当たり判定マスクを対で入れ替え、いずれか一方だけが古い状態にならないようにする。
5. While DPI が 96 以外である, the emo-present レイヤ shall 当たり判定マスクの座標判定を表示中の surface と同一座標系で一致させる。

### Requirement 3: surface 切替の指令 API

**Objective:** As a 表示指令の呼び手（seriko エンジン）, I want surface id と bind 集合を運ぶ単一の指令で表示を更新できること, so that スクリプトが surface を切り替えたとき画面が追随する

#### Acceptance Criteria

1. The emo-present レイヤ shall 表示スコープ・surface id・bind 集合（`BindSet` 相当）を運ぶ指令 API を提供する。
2. When 有効な surface id と bind 集合を伴う切替指令を受け取る, the emo-present レイヤ shall 当該 surface を合成（またはキャッシュから取得）して表示と当たり判定マスクを更新する。
3. When 非表示（`\s[-1]` 相当）を意味する指令を受け取る, the emo-present レイヤ shall 当該スコープの surface を非表示状態にする。
4. If 指令が解決不能な surface id を含む, then the emo-present レイヤ shall エラーを記録し、当該指令の適用を行わず（表示を破壊せず）にスキップする。
5. The 指令 API shall 借用を持たない `Send` 可能な所有データのみで構成し、将来メッセージ enum の 1 バリアントへ転写できる形とする。
6. Where 指令が応答を必要とする, the emo-present レイヤ shall 応答用チャネル（reply 用 Sender 同梱）を受け取れる形の指令を許容する。

### Requirement 4: 合成キャッシュと無効化

**Objective:** As a emo ランタイム, I want surface id 単位で合成結果をキャッシュし、必要時に無効化できること, so that 同一 surface の再表示で合成を繰り返さず、資産再読込時には確実に作り直せる

#### Acceptance Criteria

1. The emo-present レイヤ shall surface id をキーに合成結果を保持するキャッシュを備える。
2. When 既にキャッシュされている surface への切替指令を受け取る, the emo-present レイヤ shall 再合成せずキャッシュされた結果を用いて表示を更新する。
3. When キャッシュ無効化が要求される, the emo-present レイヤ shall 保持している合成結果を破棄し、以後の切替で再合成させる。
4. The emo-present レイヤ shall キャッシュを本レイヤで所有し、上流の合成関数（emo-compose）を純粋（状態を持たない）なまま扱う。

### Requirement 5: バルーン枠の表示

**Objective:** As a emo ランタイム, I want シェルサーフェスと同一機構でバルーン枠を表示できること, so that surface0 とバルーン枠が並んで表示され旧 emo-surface のゴールを満たす

#### Acceptance Criteria

1. When fixture 直指定のバルーン枠画像（balloons*.png）が供給される, the emo-present レイヤ shall シェルサーフェスと同一の表示・合成機構でバルーン枠を表示する（＝バルーン枠も `ComposedSurface` 化して同一経路を通す。枠専用の直 WIC バイパスは用いない）。
2. The emo-present レイヤ shall バルーン枠画像のアルファ（PNG 自身のアルファ）を尊重して枠の透明部を透過表示する。
3. The emo-present レイヤ shall バルーン枠の表示範囲を M-boot では枠そのもの（背景枠）に限定し、テキスト・arrow・marker・online マーカーの描画を含めない。
4. Where バルーンのアンカーオフセット（`sakura.balloon.offsetx`/`offsety` 相当）が与えられる, the emo-present レイヤ shall バルーン枠をシェルサーフェスに対して指定オフセットで配置する。

### Requirement 6: 観測用専用 example（実 DPI 含む）

**Objective:** As a 開発者, I want 単一 pass/fail で振る舞いを確認できる専用 example, so that 表示・クリック透過・切替が正しいことを実機で証明できる

#### Acceptance Criteria

1. The emo-present example shall emo2 fixture から surface0 とバルーン枠（balloons*.png）を表示する。
2. When example が起動する, the emo-present example shall メモリ供給された合成バッファの描画結果が emo-compose の golden と pixel 単位でバイト一致することを決定論的に検証する。
3. When ユーザがキャラ不透明領域をクリックする, the emo-present example shall そのクリックを捕捉し、透明域のクリックは背後へ透過することを観測可能にする。
4. When 指令 API で surface id を切り替える, the emo-present example shall 表示が新しい surface へ更新されることを観測可能にする。
5. The emo-present example shall 上記の表示と当たり判定を DPI が 96 以外の環境でも実施できる（実 DPI 観測）。
6. The emo-present example shall 保全済みの mock-shell を窓生成・クリックスルー登録の donor として用い、本番 `main.rs` を変更しない。
7. The emo-present example shall 上記 golden 検証（6.2）のため、描画のレンダリング先を GPU コンポジター surface ではなく通常の D2D オフスクリーン描画先へ向けて readback できる検証シームを備える。コンポジターによる提示そのものの pixel 検証は行わない（提示経路は wintf 既存資産の責務とみなす）。

### Requirement 7: 更新スレッド規律

**Objective:** As a emo ランタイム, I want 表示・当たり判定の更新をスレッド規律に従って行うこと, so that WUC のスレッド親和性を破らず表示破綻を避ける

#### Acceptance Criteria

1. The emo-present レイヤ shall 窓の表示内容更新・当たり判定マスク更新を UI スレッド上で実施する。
2. Where 合成（CPU 処理）を UI スレッド外のワーカーで行う, the emo-present レイヤ shall 完成した合成バッファをチャネル/キュー経由で UI スレッドへ引き渡してから表示更新を行う。

### Requirement 8: コンポジター供給面の自前所有と直読みヒットテスト経路の確保

**Objective:** As a emo ランタイム, I want 合成済みサーフェスを本レイヤが所有する読み戻し可能なオフスクリーン面としてコンポジターへ供給すること, so that 将来オフスクリーン面を直読みしてヒットテストを導出する経路を確保できる

#### Acceptance Criteria

1. The emo-present レイヤ shall 合成済みサーフェスをコンポジターへ供給する際、書き込み専用の合成面（WUC 内部アトラスへ書く `CompositionDrawingSurface` 等）ではなく、本レイヤが所有し CPU へ読み戻し可能なオフスクリーン面（自前 swap chain 相当のコンポジター供給面）を用いる。
2. When 表示中の合成結果が更新される, the emo-present レイヤ shall その自前所有面の内容を更新後の合成結果と一致させる。
3. The emo-present レイヤ shall 自前所有面の表示中画素を CPU へ読み戻せる経路（readback）を提供する。
4. The emo-present レイヤ shall 上記読み戻し経路を、将来オフスクリーン面を直読みしてヒットテスト用の当たり判定を導出する経路の基盤として利用可能な形で提供する（当たり判定の実導出そのものは後続ユニットの責務）。
5. Where 供給面の原寸が変化する, the emo-present レイヤ shall 自前所有面のバッファを新しい原寸へ追随させる（R1.5 と整合）。

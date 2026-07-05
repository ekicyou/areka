# Requirements Document

## Introduction

`areka-P0-emo-compose` は emo（⑥ render engine）三段直列チェーンの **2/3**（`emo-atlas` → **`emo-compose`** → `emo-present`）を担う**合成コア**である。型付き Shell モデル（`areka_parsers::shell::Shell`）と焼付済みアトラス（`areka-emo-atlas` の `AtlasTable`）はすでに存在するが、**surface を合成する頭脳——実サーフェスツリーの構築と、アトラス転写による1枚物ビットマップ合成——が存在しない**。DComp/WUC の visual ブレンドは SERIKO 合成メソッドをピクセル正確に写像できないため、この合成コアは emo 自前で持つ（開発者決定・記憶 areka-emo-own-compositor-atlas）。

本フィーチャは、Shell モデル＋アトラス＋「surface id ＋ 有効 bind 集合」を入力に、**合成済み1枚ビットマップ（premultiplied BGRA）を純粋に生成**できることを到達点とする。通信非依存の純粋層であり、wintf・WUC・窓・visual を一切知らない。観測は表示不要のオフスクリーン pixel 単体テスト（golden または要点サンプリング）で行う。

本要件は WHAT（利用者・下流から観測可能な振る舞い）のみを定義し、バックエンド選定（CPU ピクセル演算 vs D2D オフスクリーン）・合成式の de-facto 確定・データ構造の内部設計・クレート配置は design フェーズへ委ねる。

## Boundary Context

- **In scope（本フィーチャが所有する観測可能振る舞い）**:
  - Shell モデルから実サーフェスツリーを構築（疎 id の解決・`surface.append` の範囲展開と適用・`kero.surface.alias` 解決）し、**collisions/animations を保持した公開正規化 Surface 定義**を生成する。
  - 正規化 Surface 定義から合成プラン（レイヤ順・変換行列・合成メソッド・アトラス参照/入れ子参照）を導出する。
  - アトラス転写で1枚物 premultiplied BGRA ビットマップへ合成する（合成先バッファ再利用・途中アロケーションなし）。
  - 合成入力＝**surface id ＋ 有効 bind 集合**（`compose(surface_id, active_binds)` 形）。有効 bind の pattern0 overlay を **`animation-sort` → animation ID 順**の2段規則（`animation-sort` 未指定時は ukadoc 既定 `descend`・画家のアルゴリズム）で静的合成する。
  - 入れ子 surface 参照の再帰合成＋循環検出（非パニック打ち切り）。
  - 合成メソッド写像表を全量定義し、emo2 が実際に使うメソッド（実測 `overlay`）を実装する。
  - トリム契約（配置座標＋trim_offset での等価転写・全透明エントリのスキップ）の遵守。
  - **上流 `areka-parsers`（shell）の転記ギャップ解消**（本チェーン内・議題1決定）: 現行 parser が passthrough 吸収している ukadoc 正典キー **`animation-sort`（既定 descend）** と **`collision-sort`（既定 none）** を転記層で値化し、`Shell`／正規化 Surface 定義が保持するよう小さく拡張する（転記に徹し順序適用は消費側）。emo-compose は `animation-sort` を合成順序に消費し、`collision-sort` は下流 collision-geometry のため運搬する。
- **Out of scope（本フィーチャが所有しないもの）**:
  - アトラス焼付・画像デコード・正規化・αトリミング（**`areka-emo-atlas`** が所有）。
  - 表示・wintf/WUC 連携・`AlphaMask` 生成・合成キャッシュ・surface 指令 API（**`areka-emo-present`** が所有）。
  - SERIKO ループ再生・MAYUNA bind の**動的**状態管理・着せ替え UI・blink 発火（**`seriko`** が所有。本フィーチャは呼び手から渡された**静的**な有効 bind 集合を合成するのみ）。
  - バルーン文字・glyph 描画（**`emo-text-layer`** が所有）。
  - emo2 が使用しない合成メソッドの実装（型シームのみ）。
  - DPI 拡縮（表示側 wintf の責務。本フィーチャはピクセル等倍＝surface 原寸）。
- **Adjacent expectations（隣接フィーチャへの期待・提供）**:
  - **上流 `areka-emo-atlas`**: `AtlasTable`/`AtlasEntry`/`Placement`/`AtlasPage`/`AtlasKey`/`ElementId`/`SetId`（premultiplied BGRA 頁バッファ・trim_offset・placement None＝全透明）を**正本型として消費**し再定義しない。
  - **上流 `areka-parsers`（shell）**: `Shell`/`Surface`/`Element`/`Animation`/`Pattern`/`Collision`/`SurfaceAppend`/`AppendTarget`/`SurfaceAlias` 等の転記層モデルを消費する。範囲展開・alias 解決・実ツリー構築は本フィーチャの責務。加えて本チェーンでは転記層へ **`animation-sort`／`collision-sort`** の値化を追加する（従来 passthrough 吸収の ukadoc 正典キー・転記層の忠実性補完）。
  - **下流 `seriko`（shell-anim-engine）と `collision-geometry`**: 本フィーチャが公開する正規化 Surface 定義（collisions/animations 保持）を**同じ結果として消費**する（各自で再展開しないことで不一致バグを根絶）。
  - **下流 `emo-present`**: 合成結果 `ComposedSurface`（premultiplied BGRA・size・stride 明示）を無変換で WUC upload と `AlphaMask` 生成に使える形で受け取る。呼び手（emo-present/統合層）が有効 bind の静的集合を渡す。

## Requirements

### Requirement 1: 実サーフェスツリー構築（疎 id 解決・純粋データ変換）

**Objective:** As a 下流エンジン（seriko / collision-geometry / 本フィーチャの合成段）, I want Shell モデルの転記層表現を合成可能な正規化 Surface 定義へ変換してほしい, so that 疎 id・append・alias を各自で再展開せずに同一の正規化結果を消費できる。

#### Acceptance Criteria

1. When Shell モデルが与えられたとき, the emo-compose Surface Tree Builder shall 疎な surface id 集合を解決し、各 surface id に対応する正規化 Surface 定義を生成する。
2. When 正規化 Surface 定義を生成するとき, the emo-compose Surface Tree Builder shall element（レイヤ・画像パス・座標）に加えて collisions と animations を保持した完全な定義を生成する。
3. When 正規化結果を公開するとき, the emo-compose Surface Tree Builder shall 下流（seriko・collision-geometry）が再パース・再展開なしに消費できる公開形として提供する。
4. If 参照された surface id が Shell モデルに存在しないとき, then the emo-compose Surface Tree Builder shall パニックせず、その旨をログ（`warn` 以上）に記録したうえで欠落を観測可能な形で扱う。
5. The emo-compose Surface Tree Builder shall 同一入力に対して決定的（バイト等価）な正規化 Surface 定義を生成する。
6. When 正規化 Surface 定義を生成するとき, the emo-compose Surface Tree Builder shall 転記層が保持する `animation-sort`／`collision-sort`（描画順・判定順キー）を正規化結果へ引き継ぎ、下流（seriko・collision-geometry）が同一の順序規則を再展開なしに消費できるようにする。

### Requirement 2: `surface.append` 範囲展開と適用

**Objective:** As a 合成コア, I want `surface.append` のターゲット記述子（単一/範囲）を実 surface id へ展開して適用してほしい, so that append 由来の element/collision/animation が対象 surface に正しく取り込まれる。

#### Acceptance Criteria

1. When `SurfaceAppend` のターゲットが単一 id 指定であるとき, the emo-compose Surface Tree Builder shall その id の surface へ append 定義（collision・animation 等）を適用する。
2. When `SurfaceAppend` のターゲットが範囲指定（`a-b`）であるとき, the emo-compose Surface Tree Builder shall 両端を含む各 id へ append 定義を展開適用する。
3. When 複数の append が同一 surface に効くとき, the emo-compose Surface Tree Builder shall Shell モデルの出現順を保った決定的順序で適用する。
4. The emo-compose Surface Tree Builder shall append の適用範囲（element・collision・animation のどれに効くか）を正規化 Surface 定義へ反映する。

### Requirement 3: alias 解決

**Objective:** As a 合成コア, I want `kero.surface.alias` の alias キー→数値 id リスト写像を解決してほしい, so that alias 経由の surface 参照を実 id へ解決して合成できる。

#### Acceptance Criteria

1. When Shell モデルに `SurfaceAlias` 写像が含まれるとき, the emo-compose Surface Tree Builder shall alias キーを順序付き数値 id リストへ解決する。
2. When 同一 alias キーが重複定義されているとき, the emo-compose Surface Tree Builder shall Shell モデルが保持する重複・出現順を決定的に扱う。
3. If 解決対象の alias キーが写像に存在しないとき, then the emo-compose Surface Tree Builder shall パニックせず、その旨をログ（`warn` 以上）に記録して未解決として扱う。

### Requirement 4: 合成プラン導出（レイヤ順・変換行列・合成メソッド）

**Objective:** As a 合成コア, I want 正規化 Surface 定義から転写命令列を導出してほしい, so that 合成実行段がバックエンド非依存に転写を実施できる。

#### Acceptance Criteria

1. When 正規化 Surface 定義が与えられたとき, the emo-compose Plan Builder shall レイヤ順・変換行列・合成メソッド・アトラス参照（または入れ子 surface 参照）を含む転写命令列を導出する。
2. The emo-compose Plan Builder shall element 配置を変換行列として表現し、X,Y のみの平行移動を単位行列（回転・拡縮なし）の特例として扱う。
3. Where 命令がアトラス上の element を参照するとき, the emo-compose Plan Builder shall `AtlasTable` の解決結果（`ElementId`・`Placement`）を命令に含める。
4. Where 命令が入れ子 surface を参照するとき, the emo-compose Plan Builder shall 参照先 surface を再帰合成対象として命令に含める。
5. The emo-compose Plan Builder shall 同一の正規化 Surface 定義に対して決定的な命令列を導出する。

### Requirement 5: 有効 bind 集合の静的合成

**Objective:** As a 呼び手（emo-present / 統合層）, I want `compose(surface_id, active_binds)` へ有効 bind の静的集合を渡すと、bind パーツが正しく重ねられた合成結果を得たい, so that 全パーツが MAYUNA bind である surface（emo2 side0 本体 surface1000）が空白にならず表示できる。

#### Acceptance Criteria

1. The emo-compose Compositor shall 合成入力として surface id と有効 bind 集合（`compose(surface_id, active_binds)` 形）を受け取る。
2. When 有効 bind 集合が与えられたとき, the emo-compose Compositor shall 各有効 bind の pattern0 overlay を合成対象に含める。
3. When 複数の有効 bind を重ねるとき, the emo-compose Compositor shall **`animation-sort` → animation ID 順**の2段規則で合成する（`animation-sort` 未指定時は ukadoc 既定 `descend`・画家のアルゴリズム。descend/ascend が画素積層へ効く de-facto 方向は design で ukadoc 実測確定）。
4. Where surface が静的 element を持たず全パーツが bind であるとき, the emo-compose Compositor shall 有効 bind 集合のみから可視ビットマップを生成する（bind 集合が非空なら空白にしない）。
5. While bind の動的状態管理（bindgroup 切替・blink 発火・着せ替え）が発生するとき, the emo-compose Compositor shall それを自ら管理せず、呼び手が渡す静的集合のみを合成する。
6. Where 対象シェルが `animation-sort` を指定しないとき（emo2 実測＝未指定）, the emo-compose Compositor shall ukadoc 既定 `descend` を順序規則に適用する。

### Requirement 6: アトラス転写による1枚物ビットマップ合成

**Objective:** As a 下流 `emo-present`, I want 転写命令列とアトラスから合成済み1枚ビットマップを得たい, so that 無変換で WUC upload と AlphaMask 生成に使える完成品を受け取れる。

#### Acceptance Criteria

1. When 転写命令列とアトラス（`AtlasTable`）が与えられたとき, the emo-compose Compositor shall アトラス頁バッファから合成先バッファへ element を転写し、1枚の premultiplied BGRA ビットマップを生成する。
2. When element を転写するとき, the emo-compose Compositor shall 転写先座標を「element 配置座標＋trim_offset」として算出し、トリムが見た目を変えないことを保証する。
3. If アトラスエントリが空（`placement` が None＝全透明）であるとき, then the emo-compose Compositor shall そのエントリの転写をスキップする。
4. The emo-compose Compositor shall 合成演算を premultiplied 前提で行い（SourceOver: `dst = src + dst*(1-src_a)`）、straight α の式を混在させない。
5. The emo-compose Compositor shall 合成結果のサイズを base surface 原寸とし、ピクセル等倍（DPI 拡縮を持ち込まない）で生成する。

### Requirement 7: 入れ子 surface 参照の再帰合成と循環検出

**Objective:** As a 合成コア, I want 入れ子 surface 参照を再帰合成しつつ循環を検出して安全に打ち切りたい, so that 入れ子定義があっても無限再帰でクラッシュしない。

#### Acceptance Criteria

1. When 合成対象 surface が他 surface を参照するとき, the emo-compose Compositor shall 参照先を再帰的に合成して結果を取り込む。
2. If 入れ子参照が循環（自己参照または相互参照）を含むとき, then the emo-compose Compositor shall 訪問集合により循環を検出し、パニックせずに合成を打ち切る。
3. When 循環を検出して打ち切ったとき, the emo-compose Compositor shall その旨をログ（`warn` 以上）に記録する。

### Requirement 8: 合成メソッド写像表（全量定義・emo2 使用分のみ実装）

**Objective:** As a 保守者, I want 合成メソッド写像表を全量で持ちつつ emo2 使用分のみ実装したい, so that 将来メソッドを増やす際の拡張シームを保ちながら過剰実装を避けられる。

#### Acceptance Criteria

1. The emo-compose Method Registry shall ukadoc 由来の合成メソッド群（`overlay`・`overlayfast`・`replace`・`base`・`reduce`・`asis`・`interpolate`・`add`・`bind`・`blend-*` 群など）を写像表として全量列挙する。
2. Where 合成メソッドが emo2 で実際に使用されるとき（実測 `overlay`）, the emo-compose Method Registry shall そのメソッドの合成挙動を実装する。
3. Where 合成メソッドが emo2 で使用されないとき, the emo-compose Method Registry shall 型シーム（未実装であることが明示された口）として保持し、実装しない。
4. When 命令が未実装メソッドを参照したとき, the emo-compose Compositor shall パニックせず、未対応であることをログ（`warn` 以上）に記録して観測可能な形で扱う。

### Requirement 9: 出力契約 `ComposedSurface` と通信非依存性

**Objective:** As a 下流 `emo-present` と将来のアクター経路, I want 合成結果を `Send` な所有データ `ComposedSurface` として受け取りたい, so that worker→UI アクター間のメッセージ/共有バッファに借用を跨がせずに乗せられる。

#### Acceptance Criteria

1. When 合成が完了したとき, the emo-compose Compositor shall premultiplied BGRA・size（base surface 原寸）・stride を明示した `ComposedSurface` を返す。
2. The emo-compose Compositor shall `compose()` の入出力（有効 bind 集合・`ComposedSurface`）を `Send` な所有データとして提供する。
3. The emo-compose Compositor shall 合成結果を通信機構（channel・async）を介さず値・共有参照として直接返す。
4. The emo-compose Compositor shall surface id→合成結果のキャッシュ・無効化を持たない（それは `emo-present` の責務）。

### Requirement 10: 決定性・再合成予算・純粋層規律

**Objective:** As a テスト・毎フレーム再合成の駆動者（M-life seriko-loop）, I want 合成が決定的かつアロケーション予算内に収まってほしい, so that golden テストが安定し毎フレーム再合成に耐えられる。

#### Acceptance Criteria

1. The emo-compose Compositor shall 同一入力（Shell モデル・アトラス・surface id・有効 bind 集合）に対してバイト等価な合成結果を生成する。
2. Where 合成演算が浮動小数の丸め差を生じ得るとき, the emo-compose Compositor shall 整数または固定小数で演算して決定性を確保する。
3. When 合成を実行するとき, the emo-compose Compositor shall 合成先バッファを再利用し、アトラス転写を O(elements) で行い、途中アロケーションを発生させない。
4. The emo-compose Compositor shall wintf・WUC・窓・visual に依存しない純粋層として振る舞う（通信・スレッド・channel を持たない）。
5. If 合成経路で失敗が生じたとき, then the emo-compose Compositor shall 安易にパニックせず、`error` ログ＋戻り値で失敗を表現する（パニックは致命かつ直前ログ付きに限定する）。

### Requirement 11: emo2 fixture によるオフスクリーン pixel 観測

**Objective:** As a 開発者, I want emo2 fixture の合成結果を表示不要のオフスクリーン pixel テストで観測したい, so that 単一 pass/fail で合成コアの正しさを別ユニット非依存に検証できる。

#### Acceptance Criteria

1. When emo2 fixture の surface0（base＋element）を合成したとき, the emo-compose Compositor shall 期待ピクセル（golden または要点サンプリング）と一致する合成結果を生成する。
2. When emo2 fixture の surface1000 に有効 bind 集合を与えて合成したとき, the emo-compose Compositor shall bind パーツが重なった非空の合成結果を生成する。
3. When トリム済み element を転写したとき, the emo-compose Compositor shall 「配置座標＋trim_offset」による転写がトリム前と見た目等価であることを pixel テストで満たす。
4. The emo-compose Compositor shall オフスクリーン pixel 単体テストを実上流（実行時エンジン）非依存に成立させる（fixture/正規化モデル直入力で観測する）。

### Requirement 12: 制約遵守（依存・スコープ・既知ドリフト解消）

**Objective:** As a 保守者, I want 本フィーチャが最小実装・最小依存で組まれ既知ドリフトを解消してほしい, so that M1 の実装規律（実装ファースト・spec 工場禁止・最小実装＋薄いシーム）に沿う。

#### Acceptance Criteria

1. The emo-compose Compositor shall Rust 2024 で実装し、tokio を使用しない。
2. The emo-compose Compositor shall 新規外部依存を追加しない（合成コアは std のみを理想とし、アトラスのクレート依存は `areka-emo-atlas` 側に閉じる）。
3. The emo-compose Compositor shall emo2 が実際に使う機能のみを実装し、写像表・変換行列・入れ子・循環検出の**構造**は最初から保持しつつ未使用分は型シームに留める。
4. When 本チェーンに着手するとき, the emo-compose Compositor shall 既知ドリフト（`crates/areka-parsers/src/balloon/model.rs:6` doc コメントの旧名 `text-layer`/`surface-engine` 参照）を現行エンジン固有名へ追随修正する。
5. When 本チェーンに着手するとき, the emo-compose 実装 shall 上流 `areka-parsers::shell` の転記層へ `animation-sort`／`collision-sort`（従来 passthrough 吸収の ukadoc 正典キー）の値化を追加する（議題1決定）。追加は転記に徹し（順序適用は消費側の責務）、既存の転記契約・parser テストを壊さない。

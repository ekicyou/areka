# Requirements Document

## Project Description (Input)

emo2 のバルーン定義（`descript.txt` ＋ サーフェス別上書き `balloons0s.txt`/`balloonk0s.txt`）を**バルーンモデル**へ解析する parser を `areka-parsers` クレートの `balloon` モジュールとして追加する。統一グラフィック方針（バルーン＝シェル surface 上の文字層）ゆえ、下流の `text-layer`／`surface-engine` がバルーン枠・文字領域・座標を消費するモデルの生成源が要る。既存 `sakura` モジュールの確立パターン（`Result` 無しの寛容パース・NewType＋opaque＋accessor・`#[non_exhaustive]`・`tracing` のみ・in-source テスト）を踏襲し、emo2 が実際に使うフィールドのみを対象とする（過剰・予測実装は禁止）。base descript → サーフェス別 s0s/k0s 上書きをマージしたバルーンモデルを生成し、emo2 fixture（`emo2-kakukaku`）で pass することを、純粋関数・単体テストのみで観測可能にする。

本 spec は M1 `areka-P0-emo2-boot` の parser トラック（`shell-parse ∥ balloon-parse ∥ package-mount` の並行・単体テスト可・host 不要・依存無し＝即着手可）に属する。

## Introduction

本要件は、`areka-parsers::balloon` モジュールが emo2 バルーン定義ファイル群を型付きバルーンモデルへ寛容に解析し、共通既定（base descript）にサーフェス別差分（s0s/k0s）を順マージした結果を下流エンジンへ提供するための、利用者観測可能な振る舞いを定義する。対象は emo2 適合ゴースト（`emo2-kakukaku`）が実際に使用するフィールドに限定する。解析結果の妥当性は emo2 実物 fixture に対する単体テストで検証できることを要件とする。

## Boundary Context

- **In scope（本 spec が担う振る舞い）**:
  - `areka_parsers::balloon` モジュールと、バルーンモデル型（descript フィールド＋マージ済みサーフェス別状態）の定義。
  - `descript.txt`（base 共通既定）の解析。
  - `balloons0s.txt`（sakura 側）／`balloonk0s.txt`（kero 側）のサーフェス別差分解析と、base への順マージ（overlay 上書き）。
  - 座標フィールドの符号意味を保持した解析。`validrect` / `wordwrappoint` は**負値＝反対端基準**（ベース画像の右下からの相対座標）、`windowposition` は**基本位置からの調整量で符号は方向を示す**（y は下が＋・上が－）。いずれも符号を失わずモデルへ保持する。
  - font・anchor 文字色・origin・スクロール矢印（`arrow0`/`arrow1`）の解析。バルーン本体画像参照（descript に明示行は無く命名規約由来）のサーフェス別解決。
  - emo2 fixture に対する単体テスト。
- **Out of scope（本 spec が担わない振る舞い）**:
  - バルーン描画・文字レイアウト・折返し実行（下流 `areka-P0-text-layer` の領分）。
  - surface 合成（下流 `areka-P0-surface-engine` の領分）。
  - emo2-kakukaku が未使用のフィールド群: `communicatebox`／`onlinemarker`／`sstpmarker`／`sstpmessage`／`marker.png`／`number.*`／cursor スタイル。これらは本 spec で意味解釈しない。
  - 他 parser（shell / package）の領分。
- **Adjacent expectations（隣接系への期待・非所有）**:
  - sakura/kero の左右配置は shell descript の `*.balloon.alignment` が決めるものであり、バルーン単体では決まらない。本 spec はバルーン定義側のみを解析し、この配置決定を所有しない（下流エンジンが shell 側と突き合わせる）。
  - `balloons0s.txt`（sakura 側差分）と `balloonk0s.txt`（kero 側差分）はいずれも本リポジトリの fixture に実データが vendored 済みであり、対応する PNG（`balloons0.png`／`balloonk0.png`）も fixture に同梱される。実データ検証は s0s・k0s の両サーフェスを対象とし、両側それぞれの確定値（符号を含む）を単体テストで固定する。

## Requirements

### Requirement 1: balloon モジュールの公開面とパース入口

**Objective:** As a 下流エンジン開発者, I want `areka_parsers::balloon` が公開する純粋関数からバルーンモデルを得たい, so that host 環境やファイル I/O に依存せず型付きモデルを消費できる

#### Acceptance Criteria

1. The balloon parser shall `areka_parsers::balloon` モジュールとして公開面（バルーンモデル型と解析関数）を提供する。
2. When 呼び出し側が base descript 文字列とサーフェス別差分文字列（s0s／k0s）を入力として与えたとき, the balloon parser shall 外部状態やファイル I/O に依存せず単一のマージ済みバルーンモデルを返す。
3. The balloon parser shall `Result` を用いない寛容パースとして振る舞い、解析失敗によるエラー返却や panic を行わない。
4. Where 入力文字列が空または全行未知の場合, the balloon parser shall 既定値のみで構成された有効なバルーンモデルを返す。
5. The balloon parser shall 既存 `sakura` モジュールの規律（不透明 NewType ＋ read-only アクセサ、`#[non_exhaustive]`、派生は最小限）に整合したモデル型を公開する。

### Requirement 2: base descript フィールドの解析

**Objective:** As a 下流エンジン開発者, I want emo2 の base `descript.txt` が定める共通既定を型付きで得たい, so that サーフェス別差分が無いフィールドでも既定値を利用できる

#### Acceptance Criteria

1. When 入力 descript に `type,balloon` 行が含まれるとき, the balloon parser shall それをバルーン種別として解析する。
2. When 入力 descript に `use_self_alpha,1` 行が含まれるとき, the balloon parser shall PNG 自己アルファ有効としてモデルへ反映する。
3. When 入力 descript に `origin.x` / `origin.y` 行が含まれるとき, the balloon parser shall 原点座標をモデルへ反映する。
4. When 入力 descript に `font.name`（例: `Yu Gothic UI`）行が含まれるとき, the balloon parser shall フォント名をモデルへ反映する。
5. When 入力 descript に `font.height`（例: `28`）行が含まれるとき, the balloon parser shall フォント高をモデルへ反映する。
6. When 入力 descript に `font.color.r` / `font.color.g` / `font.color.b` 行が含まれるとき, the balloon parser shall 本文文字色（RGB）をモデルへ反映する。
7. When 入力 descript に `anchor.font.color.r` / `anchor.font.color.g` / `anchor.font.color.b` 行が含まれるとき, the balloon parser shall リンク（アンカー）文字色（RGB）をモデルへ反映する。
8. Where バルーン本体画像は descript 等に明示ファイル名が記述されず SSP 命名規約（`balloon{s|k}{ID}.png`・偶数=左向き／奇数=右向き。例: sakura 側 `balloons0.png`、kero 側 `balloonk0.png`）で導出される場合, the balloon parser shall サーフェス種別（sakura／kero）とサーフェス ID を保持し、下流がファイル I/O 無しに命名規約でバルーン本体画像を解決できる形でモデルへ反映する。
9. When 入力 descript に `arrow0.x` / `arrow0.y` / `arrow1.x` / `arrow1.y` 行が含まれるとき, the balloon parser shall スクロール矢印座標をモデルへ反映する。

### Requirement 3: 座標フィールドの負値＝反対端基準の保持

**Objective:** As a 下流エンジン開発者, I want 負値を反対端基準として区別できるバルーン座標を得たい, so that 描画時に基準端を取り違えず配置・折返しを計算できる

#### Acceptance Criteria

1. When 入力に `windowposition.x` / `windowposition.y`（例: sakura で x=266・y=-129、kero で x=-190・y=-75）が含まれるとき, the balloon parser shall 値の符号を保持したままウィンドウ位置をモデルへ反映する（符号は反対端基準ではなく基本位置からの調整方向を示す。y は下が＋・上が－）。
2. When 入力に `wordwrappoint.x` / `wordwrappoint.y`（例: `x,-34`）が含まれるとき, the balloon parser shall 折返し点座標を、負値＝右端基準と区別できる符号付き値としてモデルへ反映する。
3. When 入力に `validrect.top` / `validrect.bottom` / `validrect.left` / `validrect.right`（例: `bottom,-56`）が含まれるとき, the balloon parser shall 有効矩形の各辺を、負値＝反対端基準と区別できる符号付き値としてモデルへ反映する。
4. The balloon parser shall これらの座標値について、正値と負値を情報として失わずモデルに保持し、基準端の解釈を下流が判定できる形で提供する。

### Requirement 4: base → サーフェス別（s0s/k0s）マージ

**Objective:** As a 下流エンジン開発者, I want base descript にサーフェス別差分を順マージしたモデルを得たい, so that sakura／kero それぞれの最終確定値を再マージ不要で消費できる

#### Acceptance Criteria

1. The balloon parser shall base descript を共通既定とし、サーフェス別差分（s0s／k0s）を overlay で上書きするマージ規則を適用する。
2. When base に存在するフィールドがサーフェス別差分にも存在するとき, the balloon parser shall サーフェス別差分の値を優先してマージ済みモデルへ反映する。
3. When base に存在するフィールドがサーフェス別差分に存在しないとき, the balloon parser shall base の値をマージ済みモデルへ保持する。
4. When base に存在しないフィールドがサーフェス別差分にのみ存在するとき（例: `windowposition` は s0s／k0s のみに定義）, the balloon parser shall サーフェス別差分の値をマージ済みモデルへ反映する。
5. The balloon parser shall sakura 側（s0s）と kero 側（k0s）を区別してマージ済み結果を提供し、両サーフェスの確定値を取り違えない。

### Requirement 5: 未使用フィールドの寛容な取り扱い（過剰実装の禁止）

**Objective:** As a メンテナ, I want emo2 未使用フィールドを意味解釈せず寛容に扱うモデルを得たい, so that 過剰・予測実装を避けつつ未知行でも解析が破綻しない

#### Acceptance Criteria

1. Where 入力に emo2-kakukaku 未使用フィールド（`communicatebox` / `onlinemarker` / `sstpmarker` / `sstpmessage` / `marker` / `number.*` / cursor スタイル）が含まれる場合, the balloon parser shall それらを本 spec のモデル意味へ解釈せず、解析を破綻させずに扱う。
2. If 入力行が本 spec の対象フィールドとして認識できない未知行である場合, the balloon parser shall 当該行を寛容に取り込み（生保持等）、後続行の解析を継続する。
3. The balloon parser shall 対象外フィールドの存在を理由に panic・エラー返却・後続解析の中断を行わない。

### Requirement 6: emo2 fixture による検証可能性

**Objective:** As a メンテナ, I want emo2 実物 fixture に対する単体テストで解析結果を検証したい, so that 座標の負値基準やマージ規則の取り違えを実データで捕捉できる

#### Acceptance Criteria

1. The balloon parser shall emo2 fixture（`emo2-kakukaku` の `descript.txt` ＋ `balloons0s.txt`／`balloonk0s.txt`）を入力として解析でき、単体テストで結果を観測できる。
2. When emo2 fixture を base descript ＋ s0s でマージして解析したとき, the balloon parser shall sakura 側の確定値（例: `windowposition` x=266・y=-129、`wordwrappoint.x,-49`、`validrect.bottom,-56`）を、符号を保持したマージ済みモデルとして生成する。
3. When emo2 fixture を base descript ＋ k0s でマージして解析したとき, the balloon parser shall kero 側の確定値（例: `windowposition` x=-190・y=-75、`validrect.bottom,-70`）を、符号を保持したマージ済みモデルとして生成する。
4. The balloon parser shall 純粋関数・単体テストのみで結果が観測可能であり、描画・合成・host 実行を要さずに検証できる。

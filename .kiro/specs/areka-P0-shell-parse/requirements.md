# Requirements Document

## Project Description (Input)

emo2 の `surfaces.txt`（SERIKO/2.0）を**シェルサーフェスモデル**へ解析する parser が存在しない。下流の `shell-anim-engine`（SERIKO ループ）と `surface-engine`（統一 surface 合成）が消費するモデルの生成源が要る。本 spec は `areka-parsers` クレートへ `shell` モジュールを追加し、確立済みの `sakura` パターン（`Result` 無しの寛容パース・NewType＋opaque＋read-only accessor・`#[non_exhaustive]` enum・依存は `tracing` のみ・in-source `#[cfg(test)]` テスト）を踏襲する。実装範囲は emo2 が実際に使う機能のみ（過剰・予測実装は禁止）とし、拡張は型の `#[non_exhaustive]` シームのみ残す。emo2 実物 fixture（`crates/pilot/examples/shiori-host-32/fixtures/emo2/shell/master/surfaces.txt`）で pass することを唯一の適合基準とする。

## Introduction

本機能は emo2 ゴーストの `surfaces.txt`（SERIKO/2.0 記述）を、下流エンジンが再パース不要で消費できる**型付きシェルサーフェスモデル**へ変換する純粋関数パーサである。パーサはホスト環境・外部状態に非依存で、単体テストのみで観測可能（host 不要）。適合対象は emo2 が実際に使用する SERIKO/2.0 機能サブセット（`overlay` メソッド・`bind`/`random,N`/`bind+random,N` インターバル・矩形 collision・全 offset `0,0`・surface alias 透過）に限定する。emo2 が使用しない SERIKO 機能（他 method・他 interval・collisionex・element 座標オフセット）は実装しない。未知トークンは寛容に `Raw` 相当へ吸収し、パースは失敗を返さない。

## Boundary Context

- **In scope**:
  - `areka_parsers::shell` モジュールの新設と、シェルサーフェスモデル型（surface 定義・element overlay・SERIKO animation/interval・collision 矩形・surface alias）の定義。
  - `descript` ヘッダブロック（`charset` / `version`）の解釈。
  - surface 定義ブロック（`surfaceNNN { ... }`）のパース：ID とその element/collision/animation。
  - element overlay リスト（`elementN,overlay,path,x,y`）と、animation パターンの `overlay` メソッド（負 ID `overlay,-1` はレイヤクリアとして表現）。
  - SERIKO animation ブロック（`animationN.interval,...` と `animationN.patternM,overlay,...`）で、interval は `bind` / `random,N` / `bind+random,N` の 3 種のみ。
  - `surface.appendNNN` 追記ブロック（既存 surface へ collision/animation を追加）と、ターゲット指定の複数列挙・範囲指定（例 `surface.append10,2100-2110,2200-2210`）の解決。
  - collision 矩形リスト（`collisionN,left,top,right,bottom,name`・name は `Head`/`Bust` 等の不透明文字列）。
  - `kero.surface.alias` ブロック：alias キー（数値・日本語文字列いずれも）から surface ID リスト（`[id,...]`）への写像。alias キー・値は**不透明に保持**し、`\s[]` 中身の意味解釈は行わない。
  - emo2 fixture ベースの in-source `#[cfg(test)]` テスト。
- **Out of scope**:
  - レンダリング・surface 合成（`areka-P0-surface-engine` の領分）。
  - アニメ実行・SERIKO ループ・MAYUNA 実行時合成・z-order 実描画（`areka-P0-shell-anim-engine` の領分）。パーサは z-order を animation ID として保持するのみで、順序付けの実行はしない。
  - collision 矩形からリージョン／actor への写像（`areka-P0-collision-geometry` 増分の領分）。
  - emo2 未使用の SERIKO method（`overlayfast`/`base`/`replace`/`interpolate`/`asis`/`move`/`add`/`reduce`）・interval（`sometimes`/`rarely`/`periodic`/`always`/`runonce`/`never`/`talk,n` 等）・`collisionex`（円/楕円/多角形）・element 座標オフセット（全 `0,0` のため per-element 変換不要）。
  - 他 parser（`balloon-parse` / `package-mount`）の領分。
  - PNG 画像の読み込み・検証（パーサはパス文字列を保持するのみ）。
  - charset バイト列のデコード（`areka-parsers` の `charset` 共通基盤が担う。本 parser は UTF-8 デコード済み `&str` を入力に取る）。
- **Adjacent expectations**:
  - **Upstream**: `areka-parsers` クレート（`sakura` パターンの先行確立）・`areka-P0-parser-foundation`（charset デコード＋KV 共通基盤・先行依存）・emo2 fixture。本 parser はデコード済みテキストを受け取る前提で、charset 判定は担わない。
  - **Downstream**: `areka-P0-shell-anim-engine`（SERIKO ループ）・`areka-P0-surface-engine`（統一 surface 合成）・`areka-P0-collision-geometry`（collision 消費）。これらはシェルサーフェスモデル型を import して消費し、再パースは行わない。型の正本は本クレートが所有する。
  - **並走**: `balloon-parse` / `package-mount` と同クレート別モジュールで非衝突・並走安全。

## Requirements

### Requirement 1: シェルサーフェスモデル型の定義（下流共有 I/O 契約）

**Objective:** As a 下流エンジン（shell-anim-engine / surface-engine / collision-geometry）, I want `surfaces.txt` を型付きシェルサーフェスモデルとして受け取ること, so that 再パースやテキスト再解析なしに surface 合成・アニメ実行・collision 消費ができる。

#### Acceptance Criteria

1. The shell parser shall シェルサーフェスモデル型（surface 定義・element overlay・animation/interval・collision 矩形・surface alias を含む）を `areka_parsers::shell` モジュールの公開面で提供する。
2. The shell parser shall モデル型の値を正規化済み（ID は数値型・座標は数値型・alias 値は ID リスト）で提供し、下流が再パースを要しないようにする。
3. Where 意味の解釈が surface 層／下流に委譲される値（surface alias キー・`\s[]` 相当の不透明文字列）が存在する場合, the shell parser shall その値を不透明 NewType として保持し、read-only アクセサ経由でのみ中身を公開する。
4. The shell parser shall 公開 enum 型を `#[non_exhaustive]` として定義し、将来の element/interval/method 種別追加を後方互換に保つ。
5. The shell parser shall モデル型に `Clone` / `Debug` / `PartialEq` を派生し、`serde` 依存を持たない。

### Requirement 2: 寛容パース facade（失敗を返さない純粋関数）

**Objective:** As a パーサ利用者, I want `surfaces.txt` テキストを単一の純粋関数でモデルへ変換できること, so that host 非依存・単体テストのみで観測でき、不正入力でも処理が停止しない。

#### Acceptance Criteria

1. The shell parser shall デコード済み `&str` を入力に取り、シェルサーフェスモデルを返す公開関数を提供する（`Result` を返さない寛容パス）。
2. When 入力が空文字列である, the shell parser shall 空のシェルサーフェスモデルを返し、パニックしない。
3. If 構文的に区切れたが意味未対応または不正なトークンが存在する, then the shell parser shall そのトークンを寛容に保持（未知は破棄せず生保持相当で吸収）し、パース全体を失敗させない。
4. The shell parser shall 外部状態・ファイル I/O・ホスト環境に依存せず、同一入力に対して常に同一の出力を返す。

### Requirement 3: descript ヘッダブロックの解釈

**Objective:** As a パーサ利用者, I want `surfaces.txt` 先頭の `charset` 行と `descript { version }` ブロックを解釈できること, so that シェルの記述バージョンと文字コード宣言をモデルに反映できる。

#### Acceptance Criteria

1. When 入力に先頭 `charset,VALUE` 行が存在する, the shell parser shall その charset 宣言値を不透明文字列としてモデルに保持する。
2. When 入力に `descript { version,N }` ブロックが存在する, the shell parser shall その version 値をモデルに保持する。
3. If `charset` 行または `descript` ブロックが欠落する, then the shell parser shall 既定（未指定）として扱い、パースを失敗させない。

### Requirement 4: surface 定義ブロックのパース

**Objective:** As a 下流 surface-engine, I want 各 `surfaceNNN { ... }` ブロックを surface ID とその構成要素（element/collision/animation）へ解析できること, so that 個々のサーフェス定義を合成源として利用できる。

#### Acceptance Criteria

1. When 入力に `surfaceNNN { ... }` ブロックが存在する, the shell parser shall surface ID（数値 NNN）と、そのブロック内の element/collision/animation 群を 1 つの surface 定義としてモデルに収める。
2. The shell parser shall element overlay 行（`elementN,overlay,PATH,X,Y`）を、レイヤインデックス・overlay メソッド・画像パス文字列・座標を持つ element としてモデルに収める。
3. The shell parser shall element の画像パス文字列を無加工（区切り文字を含め）で保持し、画像の読み込み・検証を行わない。
4. When 同一 surface ブロック内に複数 element（例 `element0` / `element1`）が存在する, the shell parser shall それらをレイヤインデックス昇順の element リストとして保持する。
5. Where element メソッドが `overlay` 以外である場合, the shell parser shall 当該行を寛容に扱い（emo2 は overlay 以外を使用しないため未対応として吸収）、パースを失敗させない。

### Requirement 5: SERIKO animation と interval（3 種）のパース

**Objective:** As a 下流 shell-anim-engine, I want animation ブロックの interval とパターン列を解析できること, so that bind 着せ替え・random まばたきを SERIKO ループで駆動できる。

#### Acceptance Criteria

1. When 入力に `animationN.interval,bind` が存在する, the shell parser shall 当該 animation の interval を `bind` としてモデルに保持する。
2. When 入力に `animationN.interval,random,K` が存在する, the shell parser shall 当該 animation の interval を `random` と数値パラメータ K としてモデルに保持する。
3. When 入力に `animationN.interval,bind+random,K` が存在する, the shell parser shall 当該 animation の interval を `bind+random` と数値パラメータ K としてモデルに保持する。
4. The shell parser shall `animationN.patternM,overlay,SURFACE_ID,WAIT,X,Y` 行を、パターンインデックス M・overlay 参照先 surface ID・待ち時間・座標を持つパターンとしてモデルに収める。
5. When パターンの参照先 surface ID が負値（例 `overlay,-1`）である, the shell parser shall それをレイヤクリアを表す値としてモデルに保持する。
6. The shell parser shall 各 animation の ID（数値 N）をモデルに保持し、z-order の実順序付けは行わない（順序決定は下流に委譲）。
7. Where interval 種別が `bind` / `random` / `bind+random` 以外である場合, the shell parser shall 当該 animation を寛容に扱い（emo2 未使用の interval として吸収）、パースを失敗させない。

### Requirement 6: collision 矩形リストのパース

**Objective:** As a 下流 collision-geometry, I want surface 内の矩形 collision 定義を解析できること, so that 撫で判定用のあたり領域を消費できる。

#### Acceptance Criteria

1. When 入力に `collisionN,LEFT,TOP,RIGHT,BOTTOM,NAME` 行が存在する, the shell parser shall collision インデックス・矩形座標（left/top/right/bottom）・領域名を持つ collision としてモデルに収める。
2. The shell parser shall collision の領域名（例 `Head` / `Bust`）を不透明文字列として保持する。
3. Where collision 種別が矩形以外（`collisionex` の円/楕円/多角形）である場合, the shell parser shall 当該行を寛容に扱い（emo2 未使用として吸収）、パースを失敗させない。

### Requirement 7: surface.append 追記ブロックとターゲット範囲解決

**Objective:** As a 下流エンジン, I want `surface.appendNNN` 追記ブロックとその複数ターゲット・範囲指定を解析できること, so that 既存 surface へ追加される collision/animation（例 まばたき）を正しいサーフェス群へ結び付けられる。

#### Acceptance Criteria

1. When 入力に `surface.appendNNN { ... }` ブロックが存在する, the shell parser shall その追記対象 surface ID と、ブロック内の collision/animation を追記定義としてモデルに保持する。
2. When 追記ブロックのターゲット指定が複数列挙（例 `surface.append10,2100`）または範囲（例 `2100-2110`）を含む, the shell parser shall 指定されたすべての対象 surface ID を解決し、追記定義をそれらへ関連付ける。
3. The shell parser shall 追記ブロックの collision/animation を、通常 surface ブロックと同一のモデル表現で保持する。

### Requirement 8: surface alias 透過（不透明写像）

**Objective:** As a 下流 surface 層, I want `kero.surface.alias` の alias 名から surface ID リストへの写像を受け取ること, so that `\s[静観]` のような日本語エイリアス指定を解決できる。

#### Acceptance Criteria

1. When 入力に `kero.surface.alias { ... }` ブロックが存在する, the shell parser shall 各 alias エントリ（`KEY,[ID,...]`）を、alias キーと surface ID リストの写像としてモデルに保持する。
2. The shell parser shall alias キー（数値・日本語文字列いずれも）を不透明文字列として保持し、キーの意味解釈を行わない。
3. The shell parser shall alias 値の surface ID リスト（`[id,id,...]`）を数値 ID の順序付きリストとして保持する。
4. If 同一 alias キーが複数回出現する, then the shell parser shall パースを失敗させず、出現をモデルに保持する（衝突解決の意味論は下流に委譲）。

### Requirement 9: 寛容トークン処理とコメント・空行

**Objective:** As a パーサ利用者, I want `surfaces.txt` 中のコメント・空行・未知トークンを安全に扱えること, so that 実ファイルの装飾やバージョン差でパースが破綻しない。

#### Acceptance Criteria

1. When 入力行がコメント（`//` 始まり）または空行である, the shell parser shall その行を無視し、モデルへ影響を与えない。
2. If 認識できないブロックまたは行が存在する, then the shell parser shall それを寛容に吸収し、後続の認識可能なブロックのパースを継続する。
3. The shell parser shall 認識できない入力によってパニックせず、部分的に認識できたモデルを返す。

### Requirement 10: emo2 fixture 適合と過剰実装の禁止

**Objective:** As a 開発者, I want emo2 実物 fixture で本 parser を検証すること, so that 実需に基づく最小適合を担保し、予測実装を排除できる。

#### Acceptance Criteria

1. The shell parser shall emo2 fixture（`crates/pilot/examples/shiori-host-32/fixtures/emo2/shell/master/surfaces.txt`）を入力として、surface 定義・element overlay・animation/interval・collision 矩形・surface.append・alias を含むモデルを in-source `#[cfg(test)]` テストで検証できる。
2. The shell parser shall emo2 が使用しない SERIKO 機能（他 method・他 interval・collisionex・element 座標オフセット・レンダリング/合成/アニメ実行）を実装しない。
3. Where 2 例目の実物 fixture が新機能を要求するまで, the shell parser shall emo2 使用分を超える抽象・拡張を追加しない（拡張余地は `#[non_exhaustive]` シームのみで残す）。

### Requirement 11: クレート統合と依存規律

**Objective:** As a `areka-parsers` メンテナ, I want `shell` モジュールが既存クレート規律に従うこと, so that `sakura` パターンと一貫し、並走する他モジュールと非衝突で統合できる。

#### Acceptance Criteria

1. The shell parser shall `areka_parsers::shell` として `areka-parsers` クレートに追加され、公開 facade 関数と共有モデル型を `mod.rs` の公開面へ集約する。
2. The shell parser shall 追加依存を持たず、既存クレート依存（`tracing` およびロギングのみ）に限定する。
3. The shell parser shall `sakura` / `balloon` / `package` 兄弟モジュールと非衝突で、同クレート内に並存できる。
4. The shell parser shall Rust 2024・std 中心の構成に従う。

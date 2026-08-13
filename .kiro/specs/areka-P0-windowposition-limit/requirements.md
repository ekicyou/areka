# Requirements Document

## Introduction

バルーン `descript.txt` の `windowposition` 族に残る 2 つの未実装を解消する（⓪ghost placement 帰属・挙動バグ＋正典語彙の追跡）。

1. **`windowposition.limit`（正典既定 1）**: バルーンを強制的に画面内へ維持する既定挙動（1=維持・0=維持しない）が未実装のため、ゴーストが画面端に寄っている状態でバルーンが作業領域外へ素直にはみ出す（**実機で観測済み**・互換対応表 `doc/COMPAT_ARCHITECTURE.md` §8 :145 に追跡登記済み）。現行のバルーン初期配置は「クランプなし」＝ limit=0 相当の挙動であり、正典既定と逆。
2. **`windowposition.x` のキーワード値**: 数値のみ実装済み（kero-balloon R3.2/R7.6）。キーワード値（`center`・`top`・`bottom`＝シェル中央上／中央下への固定。ukadoc 実測で `windowposition.y` にキーワードは存在しない）は未実装で、現行パースは非数値を無警告で「未指定」と同一視する。

正典（ukadoc `descript_balloon.html`）の確定事項: `limit` の値域は 0/1・既定 1。`x` はピクセル数値またはキーワードで、`center`（SSP のみ `top` も同義）＝シェルの中央上へ固定・`bottom`（SSP のみ）＝シェルの中央下へ固定。`y` は数値のみ（下が正）で、その**基本位置**は `x` の指定により変わる（`center`/`top`＝バルーン下端とシェル画像上端が接する／`bottom`＝バルーン上端とシェル画像下端が接する／数値指定＝バルーンとシェル画像の上端が重なる）。

`limit` の**適用時点**（初期配置のみか・追従やドラッグ中もか・ユーザーが手で画面外へ置いた場合に強制的に戻すか）は正典が沈黙しており、参照実装 SSP の実測で確定してから実装する（測ってから式を書く・kero-balloon R7.6 の先例）。

また、先行 `areka-P0-scope-chain-gap` の着地により、キャラ窓の位置書き込み経路は初期解決の 1 本ではなくなった（実表示寸確定フレームでの一度きりの連鎖再解決がキャラ窓を直接動かし、バルーンが相対位置のまま随伴する）。limit=1 の保証はどの経路で最後に位置が書かれたかによらず成立しなければならない（保証をどの書き込み口へ実装するかは設計判断）。

## Boundary Context

- **In scope**:
  - `windowposition.limit` の語彙受理（scope 別・面別上書きマージ後）と正典既定 1 の画面内維持挙動。
  - `windowposition.x` のキーワード語彙（`center`・`top`・`bottom`）の受理と、キーワードに応じた基本位置（`y` の基本位置切替を含む）。
  - limit の適用時点・基準領域の SSP 実測確定と、その結果に従う実装。
  - 決定論テスト（limit 全分岐＋キーワード語彙）と実機サインオフ。
  - 互換対応表 §8 の追跡行（:145）の消化・更新。
- **Out of scope**:
  - `windowposition.x` の符号規約（kero-balloon R7.6 で SSP 実測確定済み・画面座標そのまま・左右非反転＝不変）。
  - キャラ窓の配置規則（初期配置・作業領域内クランプ・二体の連鎖規則・実表示寸確定の一度きりの再解決＝いずれも既存のまま）。キャラ窓側の「必ず作業領域内」という構造保証の再建は本仕様の所有ではない（本仕様が所有するのはバルーンの limit 保証のみ）。
  - バルーン相対位置の基準（キャラ窓左上・窓相対＝kero-balloon R3.8 確定）と保存・復元の仕組みそのもの。
  - バルーンドラッグの自由度の制限（ユーザー操作を制限するかは SSP 観測の結果に従う——観測前に決めない）。
  - 画面端でのバルーン左右反転等の美観配置政策（M2 予約）と、既存の可視性遷移ガード（完全不可視への遷移を防ぐ安全網・warn シーム）の変更。
  - ghost 側 `descript.txt` への `windowposition` 系記載の受理（SSP 拡張。語彙記録のみ・Requirement 7.4）。
  - 丸め規約の新設（既存丸め権威のみ）。
- **Adjacent expectations**:
  - バルーンの表示／非表示ライフサイクルは `balloon-visibility` の確定挙動を前提として消費するのみで変更しない。
  - limit の補正をどの位置書き込み経路（初期解決・確定時の随伴・追従）へ実装するかは設計フェーズの Boundary Commitments とする。SSP 観測で適用時点が「追従・ドラッグ中」へ確定した場合、後続 `dpi-transition-atomicity` と同系統のファイルへ触れる可能性があるため、確定後にウェーブ台帳の再判定を仰ぐ（roadmap 干渉台帳 atom⇄wpl）。
  - バルーン窓が属するモニタの決定規則（既存の帰属規則）は変更しない。

## Requirements

### Requirement 1: `windowposition.limit` の語彙受理

**Objective:** As a バルーン作者, I want `descript.txt` の `windowposition.limit` が正典どおり読まれること, so that バルーンを画面内へ維持するかどうかを宣言で制御できる

#### Acceptance Criteria

1. When scope 別のバルーン定義（既定設定＋面別上書きのマージ結果）に `windowposition.limit` が 0 または 1 で指定されているとき, the areka バルーン定義 shall その値を当該 scope の limit 値として保持する。
2. Where `windowposition.limit` が未指定のとき, the areka バルーン定義 shall 正典既定値 1 を当該 scope の limit 値とする。
3. If `windowposition.limit` に 0/1 以外の値が指定されたとき, then the areka バルーン定義 shall 警告を記録したうえで正典既定値 1 へ縮退する。
4. The areka バルーン定義 shall limit 値を `windowposition.x`/`y` と同じ scope 単位（各 scope が採用した面の面別上書きをマージした定義）で解決する。

### Requirement 2: limit=1 の画面内維持（既定挙動）

**Objective:** As a ユーザ, I want ゴーストが画面端に寄っていてもバルーンが読める位置に収まること, so that バルーンの文字が画面外へ切れて読めなくなることがない

#### Acceptance Criteria

1. While ある scope の limit 値が 1 であるとき, when Requirement 3 で確定した適用時点に該当するバルーン位置の決定が行われたとき, the areka 窓配置 shall バルーン窓矩形が対象モニタの制限領域（作業領域か画面全体かは Requirement 3.1(e) で確定）内へ完全に収まるよう位置を補正する。
2. The areka 窓配置 shall Requirement 2.1 の補正を上下左右の全 4 辺について行い、はみ出しの方向・辺の組み合わせによらず決定論的に同一規則で補正する。
3. If バルーン窓が制限領域より大きく両端を同時に収められないとき, then the areka 窓配置 shall 左辺・上辺を優先して収める（キャラ窓の既存クランプと同一の優先規則）。
4. While ある scope の limit 値が 0 であるとき, the areka 窓配置 shall 位置補正を行わず、バルーンが制限領域外へはみ出すことを許す（現行挙動の維持）。
5. When 起動時の一連の位置書き込み（実表示寸確定による一度きりの連鎖再解決でキャラ窓が動き、バルーンが随伴する場合を含む）が完了して定常状態に達したとき, the areka 窓配置 shall limit=1 の scope の可視バルーンが制限領域内にあることを、どの経路で最後に位置が書かれたかによらず保証する。
6. While ある scope のバルーンが非表示であるとき, the areka 窓配置 shall 非表示中の位置補正の実施時点を問わず、次に可視となった時点で Requirement 2.1 の保証を成立させる。
7. The areka 窓配置 shall limit の補正によってキャラ窓の位置を変更しない。
8. The areka 窓配置 shall limit の補正によってバルーンの表示／非表示状態を変更しない。
9. When 表示スケール係数 k が 1.0 以外のとき, the areka 窓配置 shall k 適用後の実表示寸（物理 px）で内包判定と補正を行い、丸めは既存丸め権威のみを用いる。

### Requirement 3: 適用時点と基準領域の SSP 実測確定

**Objective:** As a 互換検証者, I want limit の適用時点を推測ではなく参照実装 SSP の実挙動で確定したい, so that 補正の掛け場所と掛け方を実測に基づいて決められる（測ってから式を書く）

#### Acceptance Criteria

1. When limit の適用挙動を確定するとき, the 開発チーム shall 参照実装 SSP を受理オラクルとし、少なくとも次の各点を実測して確定する:
   - (a) 初期配置時に適用されるか。
   - (b) キャラ窓のドラッグ・移動へバルーンが追従する間も継続適用されるか。
   - (c) ユーザーがバルーン自体をドラッグして画面外へ置いた場合に強制的に戻すか。
   - (d) 保存されたバルーン位置の復元時に適用されるか。
   - (e) 制限領域が作業領域（タスクバー除外）か画面全体か。
   - (f) 画面端で補正が起きた後にキャラ窓を画面中央へ戻したとき、バルーンが作者指定の相対位置へ復帰するか（補正が相対位置へ焼き付くか）。
2. The areka 窓配置 shall Requirement 3.1 で確定した適用時点においてのみ limit の補正を適用し、SSP が適用しないと観測された時点では適用しない。
3. Where ukadoc の記述と SSP の実挙動が食い違うとき, the 開発チーム shall SSP 実挙動を正として実装し、乖離を互換対応表へ記録する（互換ベースウェア戦略・kero-balloon R7.6 の先例）。
4. Where SSP がユーザーのバルーンドラッグ操作を制限しない（画面外へ置ける）と観測されたとき, the areka 窓配置 shall ユーザー操作の自由度を新たに制限しない。

### Requirement 4: `windowposition.x` のキーワード語彙と基本位置

**Objective:** As a バルーン作者, I want `windowposition.x` のキーワード指定（`center`・`top`・`bottom`）が正典どおり効くこと, so that バルーンをシェルの中央上・中央下へ固定する既存資産がそのまま表示できる

#### Acceptance Criteria

1. When scope 別のバルーン定義の `windowposition.x` にキーワード `center`・`top`・`bottom` のいずれかが指定されているとき, the areka バルーン定義 shall それをキーワード指定として受理する（`top` は `center` と同義・`top`/`bottom` は SSP 拡張だがいずれも実装対象）。
2. When `windowposition.x` が `center` または `top` であるとき, the areka 窓配置 shall バルーンの初期既定位置をシェルの中央上（水平＝シェル画像の中央へバルーンの中央を揃え、垂直＝バルーン下端とシェル画像上端が接する基本位置）とする。
3. When `windowposition.x` が `bottom` であるとき, the areka 窓配置 shall バルーンの初期既定位置をシェルの中央下（水平＝シェル画像の中央へバルーンの中央を揃え、垂直＝バルーン上端とシェル画像下端が接する基本位置）とする。
4. When `windowposition.x` がキーワード指定であり `windowposition.y` に数値指定があるとき, the areka 窓配置 shall キーワードが定める基本位置から `y` の数値（下が正・画面座標系と同符号）を調整量として適用する。
5. The areka 窓配置 shall 数値指定時の基本位置（バルーンとシェル画像の上端が重なる位置を基準とする現行実装）と数値変換の挙動を変更しない。
6. If `windowposition.x` に数値でもキーワード語彙でもない値が指定されたとき, then the areka バルーン定義 shall 警告を記録したうえで未指定（正典既定値 0 の数値扱い）へ縮退する（現行の無警告な同一視を警告付きへ是正する）。
7. While 永続化されたバルーン相対位置が存在するとき, the areka 窓配置 shall 永続値を優先し、キーワード指定の適用を初期既定位置の供給にとどめる（保存値優先の順位を変更しない）。
8. When 表示スケール係数 k が 1.0 以外のとき, the areka 窓配置 shall キーワード由来の基本位置および `y` 調整量に対し既存の k 適用規約と丸め権威をそのまま適用し、新たな丸め規約を導入しない。
9. Where キーワード指定と limit=1 が同時に有効なとき, the areka 窓配置 shall キーワードで定めた初期既定位置に対しても Requirement 2 の補正を同一規則で適用する。

### Requirement 5: 既存挙動の維持（回帰境界）

**Objective:** As a 開発チーム, I want 本仕様が数値指定の確定済み挙動とキャラ窓配置を変えないこと, so that 先行仕様で SSP 実測により確定した挙動が退行しない

#### Acceptance Criteria

1. The areka 窓配置 shall 数値指定の `windowposition.x`/`y` の変換（画面座標そのまま・左右非反転の符号規約、k 適用、丸め権威）を本仕様の前後で同一に保つ。
2. While ある scope の limit 値が 0 であり `windowposition.x` にキーワード指定が無いとき, the areka 窓配置 shall バルーン初期配置の出力を本仕様適用前と同一に保つ。
3. The areka 窓配置 shall キャラ窓の配置規則（初期配置・キャラ窓の既存クランプ・二体の連鎖規則・実表示寸確定の一度きりの再解決）を本仕様で変更しない。
4. The areka 窓配置 shall バルーン相対位置の基準（キャラ窓左上・窓相対）および保存・復元の優先順位を本仕様で変更しない（Requirement 3.1(f) の観測結果により補正後の相対位置の扱いを確定する場合も、基準そのものは不変）。
5. The areka 窓配置 shall バルーン窓が属するモニタの決定規則を本仕様で変更しない。

### Requirement 6: 観測性

**Objective:** As a 開発チーム, I want limit の補正とキーワード解決が実機ログで追えること, so that 実機サインオフと将来の欠陥調査で挙動を突合できる

#### Acceptance Criteria

1. When limit の補正がバルーン位置を実際に動かしたとき, the areka shall 当該 scope・補正前後の位置（または変位）・契機をログへ記録する。
2. When scope 別の limit 値およびキーワード指定の解決が確定したとき, the areka shall その実値を既存の `windowposition` 観測ログと同水準で scope とともに記録する。
3. If バルーン定義の不正値を既定値へ縮退させたとき, then the areka shall その事実を警告として記録し、ログの無い縮退経路を作らない。

### Requirement 7: 検証と正典整合の記録

**Objective:** As a 開発チーム, I want 決定論テスト・実機サインオフ・互換対応表更新までを完了条件にすること, so that 追跡登記されていた未実装が証跡付きで消化される

#### Acceptance Criteria

1. The 検証 shall limit の判断分岐を決定論テストで全網羅する（少なくとも limit {0,1} × はみ出し方向 4 辺 × k=1 および k≠1 の行列、制限領域より大きいバルーンの優先規則、非はみ出し時の無補正を含む）。
2. The 検証 shall キーワード語彙の判断分岐を決定論テストで全網羅する（`center`/`top`/`bottom` の各基本位置 × `y` 調整量の有無 × k=1 および k≠1、不正値の警告付き縮退、未指定との区別を含む）。
3. When 既存テストが本仕様の挙動と矛盾するとき, the 開発チーム shall 当該テストとその前提を述べる doc コメントを本仕様の挙動へ更新する（「バルーンはクランプされない」ことを主張する既存テストの反転を含む・矛盾するテストを放置しない）。
4. When 本仕様の完了を判定するとき, the 開発チーム shall 互換対応表（`doc/COMPAT_ARCHITECTURE.md` §8）の追跡行（キーワード指定と `windowposition.limit` の未実装登記）を実装済みへ更新し、Requirement 3.1 の実測確定結果（適用時点・制限領域・焼き付きの有無）、正典沈黙箇所の裁定（巨大バルーン時の優先規則等）とその根拠区分（正典整合／SSP 実測／areka 裁量）、および本仕様が先送りする項目（ghost 側 `descript.txt` への `windowposition` 系記載の受理＝SSP 拡張）を記録する。
5. When 実機サインオフを行うとき, the 開発チーム shall ゴースト emo2 を絶対パスで起動してキャラ窓を画面端へ置き、limit=1 でバルーンが制限領域内へ収まることを k=1.0 以外の水準を含めて目視で確認し、Requirement 6 のログと突合する。
6. When 本仕様の完了を判定するとき, the 開発チーム shall ワークスペース全体のテストが緑であることを確認する。

# Requirements Document

## Project Description (Input)

エンジン群（shiori/parsers/kanade/sakura/seriko/emo）のユニットは揃いつつあるが、「アプリとしての areka」＝ `crates/areka` バイナリの `main.rs` を所有する仕様が存在しない。現 `main.rs` はモック UI（ハードコード窓・固定テキスト・初期位置 (400,200)・追従 (335,0) 決め打ち・DPI 処理ゼロ）が占有しており、①本番アプリの骨格（構成入力・初期化・終了）の置き場が無い、②モックデモはエンジン結線時に上書きで消える運命（動く資産の喪失）、③emo-present と window-placement が同じ `main.rs` を取り合う構造的衝突、の三重苦になっている。

本仕様（**アプリ組み上げ三段の第一段＝骨格**。後続は ghost-setup（エンジン結線）→ emo2-conformance-e2e（適合証明））は、`main.rs` を**本番アプリの骨格**へ作り替え、モックデモを**別名の example として挙動不変で保全**し、`crates/areka` の取り合いを構造ごと解消する。骨格は構成入力（ghost root path／balloon root path）を解決してログに出し、エンジン未結線のまま正常終了するところまでを担う。既存の SHIORI 契約チェーン（`shiori_host`/`shiori_session`/`reference_brain`/`shiori_demo`＋ e2e テスト群）は本物の資産として残置する。

## Introduction

本仕様は areka バイナリクレート（`crates/areka`）の `main.rs` を、モック UI 混成状態から**本番アプリの骨格**へ移行する。骨格の責務は「アプリ起動の器」に限定される――ロギング初期化・パニックハンドラ・UI ランタイム起動・**構成入力（ゴースト／バルーンのルートパス）の解決とログ出力**・後段（ghost-setup）が結線を差し込むための空の接続点・正常終了である。モック UI（シェル＋バルーン 2 窓・ドラッグ追従・ダブルクリック終了）は挙動不変のまま別名 example へ退避し、動く資産として保全する。

骨格自身は**窓を生成しない**（座標・配置ロジックを持たない）。本番の窓生成・配置は後続の window-placement が骨格の上で行い、エンジンの起動・結線・ライフサイクル統括は ghost-setup が接続点に実装する。本仕様はそれらの中身を実装しない。

## Boundary Context

- **In scope**（本仕様が観測可能な振る舞いとして所有する範囲）
  - モックデモ（現 `main.rs` のモック UI 相当）の別名 example への保全。**挙動不変が受け入れ基準**。
  - 骨格 `main`: ロギング初期化・パニックハンドラ・UI ランタイム起動・構成入力（ゴースト／バルーンのルートパス）解決とログ出力・エンジン未結線のままの正常終了。
  - SHIORI 契約チェーン（`shiori_host`/`shiori_session`/`reference_brain`/`shiori_demo`＋ e2e テスト群）の帰属維持（残置）と、`shiori_demo` 実走デモの起動時挙動（環境変数ゲート）の不変。
  - 後段（ghost-setup）が結線を差し込むための空の接続点。
  - example をビルド対象として利用可能にするための登録。

- **Out of scope**（本仕様が実装しない範囲。所有者を併記）
  - エンジンの起動・結線・ライフサイクル統括（**ghost-setup**）。
  - boot／close イベントの発火順序・運行（**kanade**）。
  - 本番の窓生成・配置・DPI 対応（**window-placement**）／サーフェス表示・描画（**emo チェーン／emo-present**）。
  - ゴースト位置・vanish count 等の状態永続化（**position-persist（M-life）**）。
  - SSTP・FMO・DirectSSTP・Plugin／HEADLINE／SAORI ホスティング・ネットワーク自動更新・ゴースト／バルーン選択 UI（**M2**）。

- **Adjacent expectations**（隣接仕様・資産への期待と非所有）
  - `completed/areka-mock-shell`：本仕様はそのデモ実体を挙動不変で保全する（新しい振る舞いを足さない）。
  - 完了済み SHIORI 系仕様：モジュール帰属は現状維持し、e2e テスト群が緑のまま通ることを本仕様の受け入れ基準に含める。
  - `areka-P0-emo-present`／`areka-P0-window-placement`：両者の `crates/areka` 衝突は本仕様の完了で構造的に解消される。emo-present は保全された example を観測土台の donor として使い、window-placement は骨格の上で窓機構を実装する（いずれも本仕様が窓・描画を実装するわけではない）。

## Requirements

### Requirement 1: モックデモの別名 example への挙動不変な保全

**Objective:** As a areka の開発者, I want 現 `main.rs` のモック UI を別名の example として挙動不変で保全してほしい, so that エンジン結線で `main.rs` が作り替えられても、動くデモ資産（シェル＋バルーン表示・ドラッグ追従・ダブルクリック終了）を失わずに手動検証と後続仕様の観測土台として使い続けられる

#### Acceptance Criteria

1. Where モックデモ example が提供されるとき, the areka クレート shall シェルウィンドウとバルーンウィンドウの 2 枚を表示する。
2. When 利用者がシェル画像を左クリックしてドラッグする, the モックデモ example shall シェルウィンドウを移動させ、バルーンウィンドウをシェルへ追従させる。
3. When 利用者がシェル画像をダブルクリックする, the モックデモ example shall 全ウィンドウを終了する。
4. The モックデモ example shall 移行前のモック UI と同一の観測挙動（表示・ドラッグ追従・ダブルクリック終了・縦書きテキスト表示）を提供する。
5. The モックデモ example shall モックデモ固有のアセット・座標定数・表示テキストを example 側に保持し、本番の骨格コードへ持ち込まない。
6. When 利用者がモックデモ example を名前で指定して起動する, the areka クレート shall そのモックデモ example をビルド対象として認識し、実行できる。

### Requirement 2: 本番アプリ骨格の起動と初期化

**Objective:** As a areka の開発者, I want `main.rs` を本番アプリの起動骨格（ロギング・パニックハンドラ・UI ランタイム起動）にしてほしい, so that 本番アプリの初期化・終了の一貫した置き場が生まれ、後続のエンジン結線・窓機構がその上に安全に積み上げられる

#### Acceptance Criteria

1. When 本番アプリが起動する, the areka アプリ shall 構造化ロギングを初期化し、環境変数によるログレベル指定に従う。
2. If ログレベル指定の環境変数が未設定・不正・非 UTF-8 のいずれかである, then the areka アプリ shall 既定のログレベルへフォールバックし、この経路で異常終了しない。
3. When 本番アプリが起動する, the areka アプリ shall パニックハンドラを設定する。
4. When 本番アプリが起動する, the areka アプリ shall UI ランタイムを起動する。
5. The areka アプリ骨格 shall 自身で窓を生成せず、座標・配置ロジックを保持しない。

### Requirement 3: 構成入力の解決とログ出力

**Objective:** As a areka の開発者, I want 骨格 `main` がゴースト／バルーンのルートパスという構成入力を解決してログに出してほしい, so that 後続のエンジン結線が「どのゴースト／バルーンを対象にするか」を骨格から受け取れ、選択 UI 無し（引数または既定規約）でも起動対象が観測できる

#### Acceptance Criteria

1. When 本番アプリが起動する, the areka アプリ shall ゴーストのルートパスとバルーンのルートパスを解決する。
2. When ゴーストのルートパスとバルーンのルートパスが解決された, the areka アプリ shall 解決結果をログに出力する。
3. Where ゴースト／バルーンのルートパスが起動時引数で与えられる, the areka アプリ shall その引数値を構成入力として採用する。
4. If ゴースト／バルーンのルートパスが起動時引数で与えられない, then the areka アプリ shall 既定の構成入力を採用する。
5. The areka アプリ shall ゴースト／バルーンの実行時選択 UI を提供しない。

### Requirement 4: エンジン未結線での正常終了と接続点

**Objective:** As a areka の開発者, I want 骨格が「エンジン未結線のまま起動して正常終了」でき、後段が結線を差し込む空の接続点を持ってほしい, so that ghost-setup が実装される前でも骨格単体で完結して検証でき、後段のエンジン結線が既存経路を壊さずに接続できる

#### Acceptance Criteria

1. While エンジンが未結線である, the areka アプリ shall 構成入力の解決とログ出力を経て正常に終了できる。
2. The areka アプリ shall 後段のエンジン結線を差し込むための接続点を提供し、その接続点は本仕様では中身を持たない。
3. The areka アプリ shall エンジンの起動・結線・ライフサイクル統括、boot／close イベントの発火順序、窓生成・描画、状態永続化を本仕様では実装しない。

### Requirement 5: SHIORI 契約チェーンの帰属維持

**Objective:** As a areka の開発者, I want 既存の SHIORI 契約チェーン（`shiori_host`／`shiori_session`／`reference_brain`／`shiori_demo` と e2e テスト群）を本物の資産として残置してほしい, so that 完了済み SHIORI 系仕様の成果とそれに依存する e2e テストが、`main.rs` の骨格化によって失われない

#### Acceptance Criteria

1. The areka アプリ shall SHIORI 契約チェーン（`shiori_host`／`shiori_session`／`reference_brain`／`shiori_demo`）を本番コード側に残置する。
2. When 既存の SHIORI e2e テスト群を実行する, the areka クレート shall それらのテストを緑（成功）のまま維持する。
3. Where SHIORI 実走デモの起動時ゲート（環境変数）が無効である, the areka アプリ shall SHIORI 実走デモを起動時に駆動しない。
4. When SHIORI 実走デモの起動時ゲート（環境変数）が有効である, the areka アプリ shall SHIORI 実走デモを駆動し、その成否にかかわらず本番アプリの通常起動を中断しない。

### Requirement 6: 制約と非機能要件

**Objective:** As a areka の開発者, I want 本仕様の変更が既定の技術制約（新規依存なし・既存テスト維持・デモ挙動不変）を守ってほしい, so that 骨格化のリファクタが動く資産と検証済みの契約を退行させない

#### Acceptance Criteria

1. The areka クレート shall 本仕様の変更において新規の外部依存を追加しない。
2. When 本仕様の変更後にクレートのテストを実行する, the areka クレート shall 既存の緑のテストを緑のまま維持する。
3. The モックデモ example shall 移行前のモック UI と観測上等価な挙動を保つ。

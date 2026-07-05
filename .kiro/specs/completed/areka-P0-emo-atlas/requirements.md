# Requirements Document

## Introduction

areka-P0-emo-atlas は render エンジン **emo**（⑥）の自前合成の**素材基盤層**である。emo の合成は wintf Visual 合成に依存せず 1 枚物ビットマップを自前生成する方針であり、その前段として **element 画像を透過正規化してアトラスに焼き付け、転写可能な形で保持する層**が必要になる。本層はこの基盤を提供する ⑥ emo トラック直列チェーンの 1/3（emo-atlas → emo-compose → emo-present）である。

本層は shell／balloon の element 画像群を、**透過正規化済み premultiplied BGRA として、α=0 領域を除外したタイトなトリム矩形**でアトラス（複数頁対応）へ焼付する。合成側（emo-compose）は「element path → (頁, UV 矩形, トリムオフセット, 原寸)」を索引でき、この索引表と頁バッファが本層の唯一の成果物である。

本層は**通信非依存の純粋層**（channel を持たない・オフスクリーン単体テストのみで pass/fail が確定する・表示を伴わない）であり、決定性（同一入力→同一 packing 結果）を持つ。正典は ukadoc（`use_self_alpha`／`.pna`／キーカラー規則）であり、emo2 fixture は最小適合サンプルとして扱う。

## Boundary Context

- **In scope**:
  - surface（shell surface および surface システムで表現される balloon）から「焼付対象 element 画像パス一覧」を導出するマニフェスト列挙（balloon は surface として shell と区別なく扱う）
  - element 画像のデコード（既存 WIC 経路の流用・新規デコード依存ゼロ）
  - 透過正規化（`use_self_alpha` 解釈・premultiplied BGRA へ統一）
  - α トリミング（有効矩形抽出・トリムオフセット／トリム寸／原寸の記録）
  - アトラスへの静的バッチ packing（複数頁・padding・同一パス重複排除）
  - アトラス索引表 API（element path → エントリ）と頁バッファの提供
- **Out of scope**:
  - 合成（element 配置・行列適用・レイヤー順の焼き込み）＝ emo-compose が所有
  - 表示・wintf 接続・AlphaMask 生成 ＝ emo-present が所有
  - SERIKO アニメーション再生・毎フレーム再合成のタイミング制御 ＝ seriko が所有
  - 動的アトラス（毎フレームの実行時挿入）
  - emo2 が実際に使用しない透過腕の実装（キーカラー腕・`.pna` 腕は口だけ残す）
- **Adjacent expectations**:
  - 上流の shell／balloon／package モデル（element パス・shell dir 解決・透過設定）は入力として注入される。本層はこれらの元ファイル（descript 等）を自ら読みに行かない。
  - balloon は描画上 surface システムで表現される（内部に element を持ち、element 合成で画像化する）。本層は balloon を shell surface と区別せず、同一の surface／element マニフェスト機構で扱う。balloon 画像を surface 表現（element を持つ surface）へ適合させる責務は上流（隣接ユニット）が担い、本層は完成した surface 表現を入力として受け取る。
  - `use_self_alpha` 等の透過パラメータは上流由来の設定として受け取る。
  - アトラス索引表と頁バッファの形（AtlasKey→AtlasEntry ＋ premultiplied BGRA バッファ）は emo-compose と共有する契約であり、本層が正本を定義する。
  - 成果物は将来 emo アクターのスレッドから共有参照される想定であり、スレッド間で安全に手渡せる所有形で提供する。

## Requirements

### Requirement 1: 焼付対象マニフェストの導出

**Objective:** As emo-compose（下流合成層）, I want shell／balloon モデルから焼付対象となる全 element 画像パスの一覧を得たい, so that 合成に必要な素材が漏れなくアトラスへ載る

#### Acceptance Criteria
1. When shell モデルと balloon モデルが入力として与えられる, the Atlas Manifest shall 全 surface が参照する element 画像パスを列挙する。
2. When surface が element 自己参照（base 画像を element として参照する流儀）を用いている, the Atlas Manifest shall base 画像を別枠扱いせず通常の element として列挙する。
3. When surface が bind アニメーション pattern を介して他 surface の element 画像を間接参照している, the Atlas Manifest shall その間接参照先の element 画像も列挙する。
4. When balloon が surface システムを介して（内部に element を持つ surface として）表現される, the Atlas Manifest shall balloon の element 画像を shell surface の element と区別なく同一機構（要件 1.1）で列挙する。
5. Where element パスがサブディレクトリを含む, the Atlas Manifest shall パスを改変せずそのまま列挙対象とする。
6. When 同一 element 画像パスが複数の参照元から現れる, the Atlas Manifest shall そのパスを重複なく 1 件として扱う。

### Requirement 2: element 画像のデコード

**Objective:** As 正規化処理, I want マニフェストの各 element 画像を画素バッファへ復号したい, so that 透過正規化とトリミングの入力が得られる

#### Acceptance Criteria
1. When マニフェストの element 画像パスが与えられる, the Atlas Decoder shall 当該画像を画素アクセス可能なバッファへデコードする。
2. If 指定パスの画像が存在しない、または復号できない, then the Atlas Decoder shall 当該エントリを診断可能なエラーとして報告し、他エントリの処理を継続する。
3. The Atlas Decoder shall デコード手段を差し替え可能な形で提供し、既定手段の詳細を上位の焼付処理に露出しない。

### Requirement 3: 透過正規化（premultiplied BGRA 統一）

**Objective:** As アトラス焼付, I want 各 element 画像を透過解釈済みの premultiplied BGRA へ統一したい, so that 全段（正規化→焼付→下流転写）で premultiplied 一貫性が保たれ、にじみ・暗縁を防げる

#### Acceptance Criteria
1. While `use_self_alpha` が有効（1／true）である, when 画像がアルファチャンネル付き（または対応する `.pna` を持つ）である, the Atlas Normalizer shall そのアルファチャンネルを透明度として採用する。
2. While `use_self_alpha` が有効（1／true）である, when 画像にアルファチャンネルも `.pna` も存在しない, the Atlas Normalizer shall 画像左上のピクセル色をキー色とする従来のキーカラー透過を適用する。
3. When 透過解釈の優先順位を適用する, the Atlas Normalizer shall アルファチャンネル ＞ `.pna` ＞ キーカラーの順に採用する。
4. The Atlas Normalizer shall 正規化結果を premultiplied BGRA として出力する。
5. Where emo2 fixture が実際に使用しない透過腕（キーカラー腕・`.pna` 腕）が存在する, the Atlas Normalizer shall 当該腕を実装せず拡張シーム（型の口）としてのみ提供する。
6. The Atlas Normalizer shall 透過パラメータ（`use_self_alpha` 等）を入力として受け取り、元の設定ファイルを自ら読み取らない。

### Requirement 4: α トリミングとオフセット記録

**Objective:** As 転写量の最小化, I want 正規化済み画像から α>0 のタイトな有効矩形だけをアトラスへ焼きたい, so that メモリと転写量を減らしつつ、合成側の見た目が変わらない

#### Acceptance Criteria
1. When 正規化済み画像が与えられる, the Atlas Trimmer shall α>0 の画素をすべて含む最小（タイト）な有効矩形を算出する。
2. When 有効矩形が算出される, the Atlas Trimmer shall トリムオフセット（原画像内の有効矩形左上座標）・トリム後寸法・原寸を記録する。
3. When アトラスへ焼き付ける, the Atlas Trimmer shall トリム後の矩形のみを焼付し、原画像全体を焼付しない。
4. If 画像が全透明（α>0 の画素を持たない）である, then the Atlas Trimmer shall 当該画像を空エントリとして記録し、アトラスへの焼付をスキップする。
5. The Atlas Trimmer shall トリムが合成側の element 配置座標を変えないこと（合成側が配置座標＋トリムオフセットで転写すれば見た目が等価であること）を保証する。

### Requirement 5: アトラスへの packing（複数頁・padding・重複排除）

**Objective:** As アトラス生成, I want トリム済み矩形群を頁へ静的バッチで配置したい, so that 決定的で重なりのない、サンプリング bleed のないアトラスが得られる

#### Acceptance Criteria
1. When トリム済み矩形群が与えられる, the Atlas Packer shall 各矩形を頁内で互いに重ならないよう配置する。
2. When 矩形を配置する, the Atlas Packer shall 各矩形の周囲に padding を確保し、隣接矩形との bleed を防ぐ。
3. While UV 矩形を記録する, the Atlas Packer shall padding を含まない実矩形を UV として記録する。
4. If 全矩形が単一頁に収まらない, then the Atlas Packer shall 複数頁へ自然に分割配置する。
5. When 同一入力（同一の矩形集合と設定）が与えられる, the Atlas Packer shall 常に同一の配置結果を返す（決定的）。
6. When 同一 element 画像パスが複数回参照される, the Atlas Packer shall 当該画像を 1 度だけ焼付し、単一のアトラスエントリへ索引する。

### Requirement 6: アトラス索引表と成果物 API

**Objective:** As emo-compose（唯一の消費者）, I want element path からアトラスエントリと頁バッファを引きたい, so that 転写だけで合成でき、素材解釈を再実装しなくてよい

#### Acceptance Criteria
1. When element path をキーに問い合わせる, the Atlas Table shall 対応するエントリ（頁番号・UV 矩形・トリムオフセット・原寸）を返す。
2. When 全透明などで焼付がスキップされた path を問い合わせる, the Atlas Table shall 空（転写スキップ）を示すエントリを返す。
3. The Atlas Table shall 各頁を premultiplied BGRA バッファとして stride を明示した形で提供する。
4. The Atlas Table shall アトラス索引表と頁バッファをスレッド間で安全に手渡せる所有形（共有参照可能な形）で提供する。
5. The Atlas Table shall 通信機構（channel 等）に依存せず、成果物を値／共有参照として直接提供する。

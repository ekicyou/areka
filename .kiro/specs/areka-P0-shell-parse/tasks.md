# Implementation Plan

- [x] 1. Foundation: shell モジュール雛形とクレート登録
- [x] 1.1 shell モジュールを areka-parsers に接ぎ木し、公開面の骨格を用意する
  - クレートに新モジュールを追加し、既存 3 兄弟（charset / kv / sakura）と非衝突に並存させる
  - 四層（model / lexer / decode / parse）のサブモジュール宣言と、公開面を一点集約する骨格を置く
  - 外部依存を一切増やさない（tracing のみ）ことを維持する
  - `cargo build -p areka-parsers` が新モジュールを含めて成功し、Rust 2024・std 中心構成を保つ
  - _Requirements: 11.1, 11.2, 11.3, 11.4_
  - _Boundary: mod_

- [x] 2. Core: シェルサーフェスモデル型（下流共有 I/O 契約）
- [x] 2.1 (P) 下流共有のシェルサーフェスモデル型一式を定義する
  - surface 定義・element overlay・animation/interval・pattern・collision 矩形・surface.append・alias を表す型を、値正規化済み（ID/座標は数値・alias 値は ID リスト）で定義する
  - 意味解釈を下流へ委ねる値（画像パス・alias キー・collision 名）を不透明 NewType＋read-only アクセサで保持する
  - 公開 enum（interval 種別・append ターゲット種別）を後方互換シーム付きで定義し、descript/charset 用フィールドは持たない
  - Clone/Debug/PartialEq を派生し serde を持たないこと、不透明アクセサの読み取り専用契約、シーム網羅を単体テストで確認する
  - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5, 3.4, 5.5, 7.2, 8.4, 10.5_
  - _Boundary: model_
  - _Depends: 1.1_

- [ ] 3. Core: surfaces.txt 構文層（lexer）
- [ ] 3.1 (P) surfaces.txt をブロック/行/CSV へ区切る構文スキャナを実装する
  - 行指向の線形スキャナで、ブロック（surfaceNNN / surface.appendNNN / descript / kero.surface.alias）境界と、ブロック内の CSV・ドット付きキー・[id,...] 配列値を字句分割する（意味割当はしない）
  - コメント行・空行を無視し、charset 行と descript ブロックをスキップ対象として認識する
  - 未閉じブロック・未知先頭語を寛容に吸収して走査を中断せず、内部トークンはモジュール外非公開にする
  - 代表ブロック断片・コメント/空行・未閉じ入力を対象にした単体テストが緑になる
  - _Requirements: 3.1, 3.2, 4.1, 9.1, 9.2_
  - _Boundary: lexer_
  - _Depends: 1.1_

- [ ] 4. Core: 意味層（decode）— 値正規化・集約・捕捉・写像・寛容吸収
- [ ] 4.1 surface ブロックの枠組みとヘッダの寛容スキップを実装する
  - charset 行・descript ブロックを retain せず読み飛ばし、欠落時も既定状態で継続する
  - surface 定義ブロックから surface ID とその構成要素群の枠組みを取り出す
  - 空入力・ヘッダのみ・ヘッダ欠落の各断片が失敗せず処理される単体テストが緑になる
  - _Requirements: 3.1, 3.2, 3.3, 4.1_
  - _Boundary: decode_
  - _Depends: 2.1, 3.1_

- [ ] 4.2 element overlay と collision 矩形の正規化を実装する
  - element overlay 行をレイヤインデックス昇順で、レイヤ/メソッド/画像パス/座標として正規化する
  - element の画像パスを無加工で保持し、画像の読み込み・検証はしない
  - collision 矩形行を始点/終点座標（ukadoc の始点X/始点Y/終点X/終点Y 順）＋不透明領域名として正規化する
  - 複数 element の昇順保持と collision 矩形＋opaque 名を確認する単体テストが緑になる
  - _Requirements: 4.2, 4.3, 4.4, 6.1, 6.2_
  - _Boundary: decode_

- [ ] 4.3 animationN 集約（interval 3 種・疎 pattern・負センチネル）を実装する
  - animationN.interval と複数の animationN.patternM を同一 animation ID へ束ね、interval を始点とする状態機械を実装する
  - interval を bind / random,K / bind+random,K の 3 種として正規化する
  - pattern を index 明示保持（連番前提なし・疎許容）し、負のサーフェス参照 ID をセンチネルとして失わず保持する（意味付けは下流）
  - 3 種 interval・疎 pattern（pattern0 欠番）・負 ID を含む断片が期待どおり正規化される単体テストが緑になる
  - _Requirements: 5.1, 5.2, 5.3, 5.4, 5.5, 5.6_
  - _Boundary: decode_

- [ ] 4.4 surface.append ターゲット指定の捕捉（展開しない転記）を実装する
  - ヘッダ数値を第1要素とし、後続の単一 ID・範囲 a-b を同格の順序付き記述子リストとして保持する（範囲は展開しない・ヘッダのカテゴリ番号的特別扱いはしない）
  - 追記ブロック内の collision/animation を通常 surface と同一のモデル表現で保持する（animation は 4.3 の集約を再利用する）
  - 範囲展開と実サーフェス定義ツリーへの転記は下流に委ね、パーサは指定を忠実に転記するのみとする
  - 混在ターゲット（ヘッダ＋列挙＋範囲）と列挙なしヘッダの双方が記述子で捕捉される単体テストが緑になる
  - _Requirements: 7.1, 7.2, 7.3_
  - _Boundary: decode_
  - _Depends: 4.3_

- [ ] 4.5 kero.surface.alias 写像（不透明キー・順序付き ID・重複保持）を実装する
  - 各エントリを、不透明 alias キーと順序付き数値 ID リストの写像として保持する
  - 数値キー・日本語キーいずれも意味解釈せず不透明に保持する
  - 同一キーの複数出現を潰さず全出現保持する（衝突解決は下流）
  - 数値/日本語/重複キーを含む alias 断片が期待どおり写像される単体テストが緑になる
  - _Requirements: 8.1, 8.2, 8.3, 8.4_
  - _Boundary: decode_

- [ ] 4.6 subset 外・不正入力の寛容吸収を実装する
  - overlay 以外の element/pattern メソッド、3 種以外の interval、collisionex を値化せず passthrough で吸収する
  - 非数トークン・欠損フィールドを既定値へ倒し、パニックせず後続の認識可能ブロックを継続する
  - subset 外を含む断片が隣接する認識可能ブロックのパースを壊さない単体テストが緑になる
  - _Requirements: 2.3, 4.5, 5.7, 6.3, 9.2, 10.4_
  - _Boundary: decode_

- [ ] 5. Integration: 公開 facade（parse）と公開面集約
- [ ] 5.1 純粋関数 parse を結線し、モデル型と公開関数を一点集約する
  - 構文層と意味層を結線した単一公開関数で、デコード済み文字列から部分認識を含むモデルを返す（Result を返さない）
  - 空入力で空のモデルを返し、同一入力で同一出力・副作用なし・パニックなしを保証する
  - モデル型と公開関数をモジュール公開面へ集約し、下流が import のみで消費できる状態にする
  - 空入力・決定性・facade 結線の単体テストが緑になり、`cargo build -p areka-parsers` が成功する
  - _Requirements: 2.1, 2.2, 2.4, 9.3, 11.1_
  - _Boundary: parse, mod_

- [ ] 6. Validation: ukadoc 準拠自前テスト（主軸）＋emo2 スモーク
- [ ] 6.1 ukadoc 準拠の自前 in-source 適合テスト（主軸）を作成する
  - surface 定義＋element＋collision＋animation（bind/random/bind+random）＋surface.append（範囲）＋alias（重複キー）＋負 ID＋コメント/空行を含む最小 surfaces.txt 断片を自作し、公開 parse 経由でモデル全構成を end-to-end 検証する
  - 仕様解釈は ukadoc を正典とし（emo2 の偶発的内容を正解根拠にしない）、期待値はリテラル直書きにする
  - 自前断片ベースの end-to-end テストが緑になる
  - _Requirements: 10.1, 10.2_
  - _Boundary: validation_tests_
  - _Depends: 5.1_

- [ ] 6.2 emo2 fixture スモークと subset 外吸収の検証を追加する
  - emo2 実物 fixture の代表抜粋をリテラルで parse し、パニックせず・スコープ内機能（surface/element/animation/collision/append/alias）を解釈し切ることを確認する（唯一の適合基準とはしない）
  - subset 外機能を含む断片が吸収され、隣接する認識可能ブロックを壊さないことを確認する
  - スモーク＋吸収テストが緑になる
  - _Requirements: 10.3, 4.5, 5.7, 6.3, 9.2_
  - _Boundary: validation_tests_
  - _Depends: 5.1_

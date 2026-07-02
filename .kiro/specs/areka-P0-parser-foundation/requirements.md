# Requirements Document

## Introduction

`areka_parsers` にこれから増える各パーサー（balloon／shell／package／sakura 辞書系）が共通で必要とする土台として、2 つの共通基盤モジュールを確立する。

1. **charset デコード層（`decode`）**: 伺かの設定・辞書ファイルは冒頭の `charset,文字コード` 行で自身のエンコードを宣言する。宣言に従ってバイト列全体をデコードしなければ、Shift_JIS ゴーストが文字化けする。この振る舞いは全対象ファイルで同一（ukadoc: `descript_balloon`／`descript_ghost`／`descript_headline`／`descript_install` で文言まで同一）。
2. **KV 読み込み層（`kv`）**: surface 読み込みパーサー（`surfaces.txt` のセクション構造）以外は `key,value` フラット行という全く同じ論理構造を持つ。

各後続 spec が個別実装すると多重実装（balloon／shell／package で三重）になる。本フィーチャはこの多重実装を排し、後続パーサー spec を「共通基盤が返した文字列／マップから自分のキーを引いて型付け・解決する薄い固有層のみ」に縮退させることを目的とする。

本フィーチャは M1 `areka-P0-emo2-boot` の parser トラック共通基盤であり、host 非依存・依存追加最小・単体テストで検証可能な純粋処理として成立する。正典は ukadoc（emo2 fixture は最小適合サンプルにすぎない）。

## Boundary Context

- **In scope（本フィーチャが所有する振る舞い）**:
  - バイト列から charset 宣言行を検出し、宣言エンコードで全体をデコードして文字列を返す振る舞い（`decode`）。
  - デコード済み文字列を素朴な `key,value` マップへ変換する振る舞い（`kv`）。
  - 上記を検証する単体テスト（emo2 fixture の UTF-8 入力＋非 UTF-8（Shift_JIS）合成入力）。
- **Out of scope（本フィーチャが所有しない振る舞い）**:
  - キーの意味解釈・既知/未知の分類・型付け・優先度解決（すべて各後続 spec の固有層の領分）。
  - `surfaces.txt` のセクション構造パース（KV 非対象。`decode` のみ利用する）。
  - sakura スクリプト構文の解析（`decode` は利用しうるが構文解析は本フィーチャ外）。
  - ファイルの読み込み I/O（バイト列は呼び出し側が渡す）。
- **Adjacent expectations（隣接する期待）**:
  - `decode` は全パーサー（sakura 辞書系を含む）が例外なく前段で利用する。`kv` は surface 以外の全パーサーが利用する。後続 spec（`areka-P0-balloon-parse`／`areka-P0-shell-parse`／`areka-P0-package-mount`）は本基盤に先行依存する。
  - 既存 `areka_parsers` の規律（`Result` を返さない寛容処理・panic しない・`tracing` のみ・in-source テスト・公開パス経由の契約固定）を踏襲する。

## Requirements

### Requirement 1: charset 宣言行の検出

**Objective:** パーサー開発者として、バイト列の冒頭を実エンコードに依存せず走査して charset 宣言を取り出したい。それによりファイル全体を正しいエンコードで読み直せるようにするため。

#### Acceptance Criteria

1. When 呼び出し側がバイト列を渡す, the Decode module shall 冒頭部を ASCII としてプリスキャンし `charset,<文字コード名>` の宣言を探索する。
2. Where charset 宣言行が冒頭部に存在する, the Decode module shall 宣言された文字コード名を抽出する。
3. When charset 名を抽出する, the Decode module shall 宣言行前後の空白・大文字小文字差・行末（CRLF／LF）を寛容に扱って抽出する。
4. If バイト列の冒頭部に charset 宣言行が見つからない, then the Decode module shall 宣言なしとして扱い、既定エンコードによるデコードへ進む（Requirement 2.3）。
5. The Decode module shall charset 名が ASCII で表現されるという前提のもと、実エンコードに関わらず宣言の走査を成立させる。

### Requirement 2: 宣言エンコードによる全体デコード

**Objective:** パーサー開発者として、宣言された文字コードでバイト列全体を 1 つの文字列へデコードしたい。それにより Shift_JIS ゴーストの文字化けを防ぐため。

#### Acceptance Criteria

1. When charset 宣言が抽出できた, the Decode module shall 宣言されたエンコードでバイト列全体をデコードして文字列を返す。
2. When 宣言が Shift_JIS などの非 UTF-8 エンコードである, the Decode module shall そのエンコードとして全体をデコードする（デコード後の文字列は当該ゴーストの意図した文字列と一致する）。
3. If charset 宣言が存在しない, then the Decode module shall 既定エンコードとして UTF-8 を用いてデコードする。
4. If 宣言された文字コード名が未対応または解釈不能である, then the Decode module shall 既定エンコード（UTF-8）へ寛容にフォールバックしてデコードを継続する。
5. If バイト列が宣言エンコードとして不正な並びを含む, then the Decode module shall デコードを中断せず、破綻しない（不正部を代替文字等で吸収して）文字列を返す。
6. The Decode module shall デコード結果として単一の文字列を返し、エラー型を返さず、panic しない。

### Requirement 3: charset デコードの純粋性

**Objective:** システム統合者として、charset デコードをファイル I/O から切り離した純粋処理として扱いたい。それにより呼び出し側が読み込み手段を自由に選べ、単体テストが外部状態に依存しないようにするため。

#### Acceptance Criteria

1. The Decode module shall 入力としてバイト列を受け取り、ファイルパスやファイルシステムへアクセスしない。
2. When 同一のバイト列を複数回デコードする, the Decode module shall 常に同一の文字列を返す（純粋関数として決定的である）。
3. The Decode module shall 外部の可変状態を保持せず、副作用（ログ出力を除く）を持たない。

### Requirement 4: KV マップ化

**Objective:** 後続パーサー spec 開発者として、デコード済み文字列を素朴な `key,value` マップとして受け取りたい。それにより各 spec が「自分のキーを引く薄い固有層」だけを書けばよいようにするため。

#### Acceptance Criteria

1. When 呼び出し側がデコード済み文字列を渡す, the KV module shall 各行を最初のカンマで `key` と `value` に分割してマップへ格納する。
2. The KV module shall キーの既知/未知を一切分類せず、専用スロットや未知行コレクションを設けず、すべてを同一のフラットなマップに格納する。
3. When 同一キーが複数行に現れる, the KV module shall 後に現れた値で先の値を上書きする（後勝ち）。
4. When 行の前後に空白が含まれる, the KV module shall キーおよび値の前後空白を寛容に除去する。
5. If 行が空である, then the KV module shall その行をスキップする。
6. If 行にカンマが含まれず key/value に分割できない, then the KV module shall その行をスキップする。
7. The KV module shall 値を文字列のまま保持し、数値化・符号解釈・その他の型付けを行わない。
8. The KV module shall キーの出現順序を保持しない（設定マップであり順序保持は不要）。

### Requirement 5: 行区切り・BOM の寛容な取り扱い

**Objective:** パーサー開発者として、CRLF／LF 混在や BOM 付きファイルでも同じ結果を得たい。それにより作成環境の差異でパースが破綻しないようにするため。

#### Acceptance Criteria

1. When 入力が CRLF 改行または LF 改行を含む, the KV module shall いずれの改行様式でも行を正しく分割する。
2. Where バイト列の先頭に BOM が存在する, the Decode module shall BOM を charset 宣言の探索およびデコード結果に悪影響を与えないよう寛容に扱う。
3. If 入力が空である, then the KV module shall 空のマップを返し、panic しない。

### Requirement 6: 寛容処理の規律

**Objective:** システム統合者として、共通基盤が既存 `areka_parsers` の規律に沿って破綻しないことを保証したい。それにより不完全・想定外の入力でも下流が安全に動作するようにするため。

#### Acceptance Criteria

1. The Decode module and KV module shall いかなる入力に対しても panic せず、エラー型（`Result` 等）を返さずに結果を返す。
2. If 入力が想定外または不完全である, then the Decode module and KV module shall 情報を捨てて破綻するのではなく、寛容に処理を継続して最善の結果を返す。
3. Where 想定外の入力を寛容に吸収した, the Decode module and KV module shall 診断ログとして記録してよい（それ以外の副作用は持たない）。

### Requirement 7: 単体テストによる検証

**Objective:** 品質担当者として、共通基盤の振る舞いを host 非依存の単体テストで検証したい。それにより後続 spec が本基盤を安心して前提にできるようにするため。

#### Acceptance Criteria

1. The parser foundation shall emo2 fixture（UTF-8 入力）を用いた単体テストで、charset デコードおよび KV マップ化の期待結果を検証する。
2. The parser foundation shall 非 UTF-8（Shift_JIS）合成入力を用いた単体テストで、宣言エンコードによる全体デコードが文字化けを起こさないことを検証する。
3. When fixture 由来の期待値をテストへ記述する, the parser foundation tests shall 期待値をリテラルで直書きし、採取元の正本ファイル名と行を明示する（クレート跨ぎの `include_str!` 依存を避ける）。
4. The parser foundation shall 公開 API パスを経由するテストで契約を固定する。

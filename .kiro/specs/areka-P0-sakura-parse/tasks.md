# 実装計画

- [ ] 1. Foundation: `areka-parsers` クレートを新設する
  - `crates/areka-parsers/` を作成し `Cargo.toml`（package = "areka-parsers"・edition 2024・依存は std のみ、任意で既存 workspace の `tracing`）を定義する
  - `src/lib.rs`（`pub mod sakura;`）と `src/sakura/mod.rs`（公開面集約のスケルトン）を置く。`areka` は変更しない（bin のまま）
  - workspace の `members = ["crates/*"]` により本クレートが自動的にメンバーへ含まれることを確認する
  - 観測: `cargo build -p areka-parsers` が成功し、空の `sakura` モジュールがビルドできる
  - _Requirements: 12.2_

- [ ] 2. (P) 命令モデル型（下流共有 I/O 契約）を定義する
  - フラットな `#[non_exhaustive] enum Instruction` の全 variant（Text / SpeakerScope / Surface / Wait / NewLine / Choice / Cursor / End / Clear / Quit / Move / SystemVar / GenericCommand / Raw）と付随する値型（SurfaceArg / NewLineRatio / Choice / MoveArgs）を定義する
  - 不透明 NewType に読み取り専用アクセサ（サーフェス文字列・改行比率）を付け、別クレートの下流が中身を読めるようにする
  - 派生は Clone / Debug / PartialEq のみ（serde・Eq・Hash は付さない）。木構造は持たないフラット表現とする
  - 観測: 別モジュールから各 variant を構築・比較でき、不透明値をアクセサで読み取れる単体テストが green
  - _Requirements: 1.1, 2.1, 2.2, 3.1, 3.4, 4.1, 5.1, 5.2, 6.1, 6.2, 6.3, 6.4, 7.1, 7.3, 8.1, 9.1, 10.1, 11.1_
  - _Boundary: model_
  - _Depends: 1_

- [ ] 3. 構文層（Lexer）を実装する
- [ ] 3.1 (P) さくらスクリプトの一般構文分割スキャナ
  - `char_indices` による手書き線形スキャナで、正準タグ（`\` ＋ワード＋ `[...]`）・bare タグ（`\e` `\c` `\-` `\n`）・`\wN` 短縮・`%keyword`・タグ間テキストを構文トークンへ分割する
  - 角括弧内をカンマ区切りで複数引数へ分割する。UTF-8 を `char_indices` で走査し charset 変換はしない
  - 観測: `\s[1000]` `\p[0]` `%username` `\w2` `\![a,b,c]` `こんにちは` が期待トークン列へ分割される単体テストが green
  - _Requirements: 8.1, 9.1, 9.2, 12.1, 13.1, 13.2, 13.3_
  - _Boundary: lexer_
  - _Depends: 1_
- [ ] 3.2 エスケープ・引数クォート・寛容な境界処理
  - エスケープ `\\`→`\`・`\%`→`%`・角括弧内 `\]`→`]` をリテラルとして取り込み、引数クォート `"..."`（内側 `,` を保護・`""`→`"`）を 1 引数として扱う
  - 未閉じ `[`/`"`・未知タグなど区切れない入力を Raw トークンへ吸収し、走査を中断しない
  - 観測: 各エスケープ／クォート／未閉じ・未知ケースの lex 結果が単体テストで固定され、前後の正常トークンが欠落しない
  - _Requirements: 10.3, 13.4, 13.5, 13.6, 13.7, 13.8_
  - _Boundary: lexer_

- [ ] 4. 意味層（Decode）を実装する
- [ ] 4.1 emo2 subset の値正規化デコード
  - 構文トークンを `Instruction` へ写像し、待ち時間（`\w[n]`/`\wN` = n×50ms・`\_w[ms]` = 絶対 ms）を単一 Wait へ、改行（素の `\n`=1.0・`\n[percent]`=percent/100・`\n[half]`=0.5・負値は戻り）を比率へ正規化する
  - `\q[タイトル,ID,...]` を disp/target ＋追加 references に分離、`\![move,...]` を Move へ、`\p[n]`/`\_l[x,y]`/`\e`/`\c`/`\-` を各命令へ、`\s[...]` を不透明保持、`%keyword` を非展開トークンへ写す。未デコード文字列断片を残さない
  - 観測: 各 subset タグの decode 結果（値正規化・境界値込み）が期待 `Instruction` と一致する単体テストが green
  - _Requirements: 1.2, 2.1, 2.2, 2.3, 3.1, 3.2, 3.3, 3.4, 4.1, 4.2, 5.1, 5.2, 6.1, 6.2, 6.3, 6.4, 7.1, 8.1, 8.2, 9.1_
  - _Boundary: decode_
  - _Depends: 2, 3.2_
- [ ] 4.2 寛容パススルー・汎用コマンド・`\q` 旧形の吸収
  - `move` 以外の `\!` を GenericCommand（種別＋生引数）へ、emo2 subset 外タグ・不正トークンを Raw へ吸収し、エラーを送出しない
  - `\q` 旧仕様 2 連ブラケット `\q[ID][タイトル]` / `\q*[ID][タイトル]` を Choice 化せず Raw で保持し、`\![*]` マーカーは Choice へ畳む
  - 観測: 未対応タグ・旧 `\q` を含む入力でも解析が中断せず、前後の正常命令が保持される単体テストが green
  - _Requirements: 5.3, 5.4, 7.2, 7.3, 10.1, 10.2, 11.2_
  - _Boundary: decode_

- [ ] 5. Integration: `parse` 公開関数で Lexer→Decode を結線する
  - `pub fn parse(input: &str) -> Vec<Instruction>` を公開し、字句解析→デコードを順に呼んで命令列を返す（`mod.rs` で parse / Instruction / 値型を公開面へ集約）
  - 空入力で空の命令列、混在入力で入力順を保持、同一入力で同一出力（純粋）、命令を実行せずエラーも送出しない
  - 観測: `areka_parsers::sakura::parse("")` が空 Vec を返し、複数タグ＋テキスト混在入力が順序保持の命令列を返す単体テストが green
  - _Requirements: 1.1, 1.3, 1.4, 1.5, 10.2, 10.3, 12.2_
  - _Boundary: parse_
  - _Depends: 4.2_

- [ ] 6. Validation: subset・構文網羅と代表 OnBoot の通し検証
  - 構文（エスケープ／クォート／角括弧／未知タグ）・値正規化（待ち時間・改行の境界値）・Choice（旧 2 連形が隣接命令を壊さない）・不透明 Surface・寛容パススルー・純粋関数契約（空入力・順序・同一入力等価・UTF-8 日本語）を網羅する単体テスト群を整備する
  - 作者提供の実 OnBoot 例をインライン代表フィクスチャとして、想定命令列への通し変換を固定する（`\![bind]`×6→GenericCommand、`\s[通常]` 不透明、`\_w`→Wait、`\n`/`\n[150]`、`\e`→End）
  - 特定ゴースト実体ファイルは同梱せず、`\q`/`\![move]`/`%username` など OnBoot 例に現れないタグは個別の手書きテストで網羅する
  - 観測: `cargo test -p areka-parsers` が全 green で、上記網羅ケースと OnBoot 通し例が含まれる
  - _Requirements: 1.5, 5.3, 12.3, 12.4_
  - _Depends: 5_

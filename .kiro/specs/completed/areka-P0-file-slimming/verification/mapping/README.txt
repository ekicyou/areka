areka-P0-file-slimming / 対応表フラグメントの置き場
==================================================

design §Data Models の対応表（旧完全修飾名 -> 新完全修飾名）を、クレート単位のコミット
（タスク 7.1 の切り分け単位）に合わせて分割して保持するためのディレクトリ。

命名規約（本 spec 内で固定・以後のタスクはこの規約に従うこと）
---------------------------------------------------------------
    .kiro/specs/areka-P0-file-slimming/verification/mapping/<crate>.csv

  - `<crate>` は Cargo のパッケージ名をそのまま使う（例: `areka.csv` / `areka-seriko.csv` /
    `wintf.csv` / `areka-ghost.csv`）。ディレクトリを掘らない。1 クレート 1 ファイル。
  - 列は結合先と同一の 3 列・同一順序: `old_fqn,new_fqn,reason`
  - `reason` は `theme_split` のみ（design §Data Models の enum）。
  - テーマ分割でモジュールパスが変わったテスト関数のみ 1 行を持つ。FQN が変わらない移設は行を持たない。
  - 文字コードは UTF-8（BOM 無し）、改行は CRLF/LF いずれでもよい（Import-Csv がどちらも読む）。
  - 行の並びは問わない（全単射検証は集合として行う）。

検証と結合
----------
  1. クレート単位（追記のつど）:
         pwsh -File .kiro/specs/areka-P0-file-slimming/verification/Test-MappingBijection.ps1 `
             -Path .kiro/specs/areka-P0-file-slimming/verification/mapping/<crate>.csv

  2. 最終照合（タスク 7.1）— 全フラグメントを結合して単一の対応表へ書き出す:
         pwsh -File .kiro/specs/areka-P0-file-slimming/verification/Test-MappingBijection.ps1 `
             -Path .kiro/specs/areka-P0-file-slimming/verification/mapping `
             -Out  .kiro/specs/areka-P0-file-slimming/verification/test_name_mapping.csv

     `-Out` は検証 PASS のときだけ書き出す。この `README.txt` は結合対象に含まれない
     （結合対象は当ディレクトリ直下の `*.csv` のみ）。

  3. 結合後のリスト照合:
         pwsh -File .kiro/specs/areka-P0-file-slimming/verification/Compare-TestLists.ps1 `
             -Before ...\before_default.txt -After ...\after_default.txt `
             -Mapping ...\test_name_mapping.csv

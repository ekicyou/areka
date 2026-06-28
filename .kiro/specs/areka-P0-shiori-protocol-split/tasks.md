# Implementation Plan

> 本仕様の実装は「使い捨ての一回限り移行・検証プログラム（Python 等・恒久資産化しない）」でフラグメント群を生成し、意味的同値ゲート合格後に旧単一ファイルを削除する**非破壊の物理／符号化形リファクタ**である。完了後に残す資産はフラグメント群・共有フラグメント・再構成マニフェスト・改訂ドキュメント・合否エビデンスのみ（スクリプト自体は残さない）。

- [ ] 1. Foundation: 移行・検証スクリプト基盤と入力 baseline 捕捉
- [ ] 1.1 移行/検証スクリプトの足場と旧 TOML 前提検証・baseline 捕捉
  - 使い捨ての移行/検証プログラム（TOML v1.0.0 パーサ）を用意し、旧 `shiori_protocol.toml` を parse する
  - 前提検証: 446 entry（event 287／resource 159）・802 field・9 silence_ruling・共有テーブル 5 の存在をカウント検証し、欠落時は中断する
  - 変換前 parse 結果を正規化した baseline として捕捉する（削除前に検証基準を確定）
  - observable: スクリプト実行で旧 TOML が parse され前提カウントが一致し、baseline が作業領域に保持される
  - _Requirements: 9.5_
  - _Boundary: 一回限り移行・検証スクリプト_

- [ ] 2. Core: 符号化変換とフラグメント・共有・マニフェスト生成
- [ ] 2.1 keyed/inline 符号化変換ロジック
  - 各 entry を id キー連想テーブルへ、各 field を意味名キー inline table（1 field = 1 行）へ、各 silence_ruling を id キー連想テーブルへ変換する
  - キーを常時 quote し dot/asterisk 混じり id（`OnUpdate.OnDownloadBegin`・`char*.defaultx` 等）を破綻なく表現する。`reference`/`reference_variadic` を保持（両保持 32 件・reference 無し 6 件）、応答意味・provenance・description・silence_ref を inline 値として保持する
  - `[mapping]` の `canonical_key`/`alias_key`/`alias_variadic_key`/`reference_backed_by` 等を新しいテーブルキー表現へ整合する（意味不変）
  - observable: 変換後の連想テーブル構造が生成され、entry id・field 意味名・silence_ruling id の重複はパーサがキー重複として機械検出する
  - _Requirements: 1.3, 4.1, 4.2, 4.3, 4.4, 4.5, 4.6_
  - _Boundary: フラグメント符号化スキーマ_

- [ ] 2.2 (P) 共有フラグメントの集約生成
  - `[meta]`/`[mapping]`/`[envelope]`/`[reserved_headers]` と全 9 silence_ruling を単一の共有フラグメントへ集約する
  - silence_ref の文字列参照が集約後も解決可能であることを保つ
  - observable: 共有フラグメント `_shared.toml` が書き出され、4 共有テーブル＋9 件の keyed silence_ruling を含み単独で parse できる
  - _Requirements: 5.1, 5.2, 5.3_
  - _Boundary: 共有フラグメント_
  - _Depends: 2.1_

- [ ] 2.3 (P) カテゴリ別フラグメント生成・サイズ規律・entry 境界サブ分割
  - event/resource をカテゴリ純度でフラグメントへ振り分け（kind 別ディレクトリへ整理）、各フラグメントを 600 行以下に収める
  - 600 行超カテゴリ（`shortcut_key` 等）を entry 境界で順序付きサブ分割し、単一 entry を複数フラグメントへまたがせない
  - observable: 全 446 entry がカテゴリ別フラグメントへ過不足なく配置され、各ファイルが ≤600 行かつ entry 無分割で parse できる
  - _Requirements: 1.1, 1.2, 2.1, 2.2, 2.3, 2.4_
  - _Boundary: フラグメント物理レイアウト_
  - _Depends: 2.1_

- [ ] 2.4 再構成マニフェスト生成
  - 全フラグメントの結合順を単一真実源として列挙し、サブ分割（`.01/.02`）順序と共有フラグメント取り込み位置を曖昧さなく固定する（`NN.` 接頭辞は捜索補助・従属であって権威でない）
  - observable: `_manifest.toml` が全フラグメントを決定的な結合順で列挙し、欠落・重複参照がない
  - _Requirements: 3.1_
  - _Boundary: 再構成マニフェスト_
  - _Depends: 2.2, 2.3_

- [ ] 3. Validation: 構造検証・決定的 merge・意味的同値ゲート
- [ ] 3.1 フラグメント構造検証
  - 結合した entry id 集合・各 entry の field 意味名集合・silence_ruling id 集合に重複がないこと、quote が破綻しないこと、各フラグメント ≤600 行かつ entry 無分割、全 description/provenance が非空、共有テーブル＋silence が共有フラグメントのみに存在し silence_ref が解決可能であることを検査する
  - observable: 構造検査が全項目 pass を出力し、違反時は該当フラグメント/キーを特定して fail する
  - _Requirements: 1.4, 2.1, 2.3, 2.4, 4.1, 4.2, 4.3, 4.4, 5.1, 5.2, 5.3, 6.1, 6.2_
  - _Boundary: 一回限り移行・検証スクリプト_
  - _Depends: 2.4_

- [ ] 3.2 決定的・冪等 merge
  - マニフェスト順にフラグメント群を結合して正準ビューを生成し、同一入力に対し同一ビューになることを 2 回 merge で確認する
  - observable: マニフェスト順 merge が正準ビューを生成し、2 回実行が同値（冪等）である
  - _Requirements: 3.2, 3.3_
  - _Boundary: 再構成マニフェスト, 一回限り移行・検証スクリプト_
  - _Depends: 3.1_

- [ ] 3.3 意味的同値ゲートと非破壊エビデンス
  - `parse(旧 TOML baseline)` と `parse(merge(fragments))` を 8 要素（entry 集合／field 集合／共有テーブル／silence_ruling／全 description／全 provenance／封筒マッピング／予約ヘッダ集合）で順序非依存に比較する
  - reference 正規化（両保持 32・reference 無し 6・任意キー欠如同値）・残差ゼロ（閉包条件）・`[mapping]` 意味保存例外を適用し、1 つでも差分または未被覆キーがあれば不合格として成果物を棄却する
  - 合格時は合否結果＋削除コミット参照の最小エビデンスを残す（正規化ダンプの恒久同梱はしない）
  - observable: ゲートが 8 要素＋残差ゼロで合否を判定し、合格時に最小エビデンス（合否＋削除コミット参照）が記録される
  - _Requirements: 1.4, 3.4, 6.1, 6.2, 9.1, 9.2, 9.3, 9.4, 9.5_
  - _Boundary: 意味的同値ゲート, 一回限り移行・検証スクリプト_
  - _Depends: 3.2_

- [ ] 4. Finalization: 旧ファイル削除・README 改訂・要件改訂継承
- [ ] 4.1 旧単一ファイル削除と README 改訂・典拠参照整合
  - 同値ゲート合格後に `shiori_protocol.toml` を tree から削除する（非権威の生成物としても残置しない）
  - `doc/shiori/README.md` を「SSOT＝fragments／`shiori_protocol.toml` は廃止（削除）／正準ビューはオンデマンド merge」へ改訂し、ukadoc ピン留めスナップショット参照（provenance・SOURCES.md・sha256）の整合を保ち、削除後の既存参照がフラグメント群を指すよう整合する
  - observable: `shiori_protocol.toml` が tree から消え、README が fragments を正本と宣言し、旧ファイルへの残存参照がゼロ
  - _Requirements: 6.3, 7.1, 7.2, 7.3, 7.4, 8.4_
  - _Boundary: 完了仕様 要件改訂・README 改訂_
  - _Depends: 3.3_

- [ ] 4.2 完了仕様 要件3・11 の改訂継承記述
  - 完了仕様 `areka-P0-shiori-protocol` の要件3・11（単一ファイル正本）を「論理 SSOT＝フラグメント群および決定的結合結果」へ改訂継承する記述を本仕様 docs／README に置き、`completed/` 配下は一切書き換えない
  - 二重定義禁止・全 description のデータ保持・provenance 維持・派生同値/冪等の精神維持を明記し、改訂理由（DP1 `array of entry` の符号化形刷新・Revalidation Trigger 該当）を系譜として残す
  - observable: 論理 SSOT への改訂継承が README＋本仕様に記述され、`completed/` 配下のファイルは無改変のまま系譜が追跡可能
  - _Requirements: 8.1, 8.2, 8.3_
  - _Boundary: 完了仕様 要件改訂・README 改訂_
  - _Depends: 4.1_

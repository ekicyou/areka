# Implementation Plan

- [x] 1. parsers 名前解決基盤
- [x] 1.1 (P) bindgroup 名前宣言のデータ型とアクセサを追加する
  - カテゴリ名・パーツ名・任意のサムネイル名を保持する型と、本体側/相方側を区別する列挙を追加する
  - (カテゴリ名, パーツ名) の問い合わせに着せ替えIDを返す機能と、カテゴリ単位の集合を返す機能を用意する
  - 未宣言の組み合わせを問い合わせた場合はIDを捏造せず「解決不能」と判別できる結果を返す
  - Observable: 空の名前表に対して問い合わせても解決不能を表す結果が得られ、パニックしないことを確認できる
  - _Requirements: 1.3, 1.4, 1.5_
  - _Boundary: parsers BindGroupNames_

- [x] 1.2 descript の bindgroup 名前宣言をマウントモデルへ取り込む
  - 本体側・相方側それぞれの bindgroup 名前宣言を、既定 on/off の取り込みと同じ走査で読み取り、カテゴリ名・パーツ名（・任意のサムネイル名）へ分解して転記する
  - パーツ名が欠落した宣言は転記対象から除外し、記録を残す
  - 同一の (カテゴリ名, パーツ名) が複数回宣言された場合は最後の宣言を優先する
  - Observable: emo2 相当の descript を読み込むと、宣言済みの (カテゴリ名, パーツ名) の組み合わせすべてが着せ替えIDへ解決できる
  - _Requirements: 1.1, 1.2, 1.6_
  - _Depends: 1.1_

- [x] 1.3 名前解決の網羅テストと既存挙動の非退行確認
  - 2フィールド宣言・3フィールド（サムネイル付き）宣言・パーツ欠落宣言・重複宣言・本体側と相方側の区別・未宣言の組み合わせをそれぞれ検証する
  - 既存の（既定 on/off のみを扱う）マウント結果が名前解決の追加によって変化しないことを確認する
  - Observable: 全ケースが実行テストとして緑になり、既存のマウント関連テストも変わらず緑のまま残る
  - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5, 1.6_
  - _Depends: 1.2_

- [x] 2. dola 名前語彙退役と消費者台帳基盤
- [x] 2.1 (P) dola のコマンド名写像機能を撤去し文書を自己選別モデルへ改訂する
  - コマンド名から担当を引く機能を削除し、汎用コマンドの型注釈を「消費側が自分で名前を選別する」という説明へ書き換える
  - Observable: dola のソースにコマンド名の具体的な文字列（例 "move"）が一切残っていないことを確認できる
  - _Requirements: 2.6_
  - _Boundary: dola cue sink_

- [x] 2.2 (P) dola 側の既存検証を自己選別モデルへ更新する
  - 撤去した機能を前提にしていた dola 側の既存検証を、削除後の型注釈・振る舞いに合わせて書き換える（または陳腐化した検証として退役させる）
  - Observable: dola 側の検証がすべて緑になり、撤去前の機能への参照がどこにも残っていない
  - _Requirements: 2.6_
  - _Boundary: dola cue sink tests_
  - _Depends: 2.1_

- [x] 2.3 (P) areka-sakura 側の既存検証を自己選別モデルへ更新する
  - 撤去した機能を前提にしていた areka-sakura 側の既存検証を、消費側の自己選別モデルの帰結（未登記名はどの消費者も反応しない）を確認する検証へ書き換える
  - Observable: areka-sakura 側の検証がすべて緑になり、撤去前の機能への参照がどこにも残っていない
  - _Requirements: 2.5_
  - _Boundary: areka-sakura drive_
  - _Depends: 2.1_

- [x] 2.4 (P) 既存コマンド消費者（move）の名前自己選別への簡素化とログ水準の整列
  - 既存の move コマンド消費者を、削除した写像機能を参照しない単純な名前一致判定へ書き換える
  - ログの水準を「自分宛の破損だけを警告として報告し、他人宛や未登記の名前は静かに読み流す」という統一規律へ揃える
  - Observable: move コマンドの解釈・配送先は変わらず、破損データを与えたときのログ水準が新しい規律どおりになることを確認できる
  - _Requirements: 2.4_
  - _Boundary: MoveCueSink_
  - _Depends: 2.1_

- [x] 2.5 (P) コマンド名の消費者台帳を結線層に新設する
  - コマンド名と担当消費者の対応を宣言する一覧と、同一名前に複数の担当者が登録されないことを確認する仕組みを結線層に用意する
  - 既存の move コマンドを台帳へ登録する
  - Observable: 台帳に move の登録が存在し、同一名前を重複登録しようとすると検出できることを確認できる
  - _Requirements: 2.2_
  - _Boundary: areka 消費者台帳_

- [x] 3. seriko 名前解決・引数解釈・積算の純関数群
- [x] 3.1 (P) bind 名前解決とスコープ写像の純関数を実装する
  - 本体側・相方側を区別した名前解決の素データを受け取り、(カテゴリ名, パーツ名) から着せ替えIDを引く機能を実装する（parsers への依存は持たない）
  - スコープ識別子から本体側/相方側のどちらの名前空間を参照すべきかを判定する機能を実装する（該当しないスコープは判定なしとする）
  - Observable: 宣言済みの組み合わせは着せ替えIDが得られ、未宣言の組み合わせ・該当しないスコープは判定なしが得られることをテストで確認できる
  - _Requirements: 3.2, 3.7, 4.3_
  - _Boundary: seriko bind resolver_

- [x] 3.2 bind コマンド引数の解釈を実装する
  - 名前キー＋明示 on/off の形・数値欄が空欄のトグル形・パーツ名が空欄のカテゴリ単位形・カテゴリ名欠落や不正な値による破損形をそれぞれ区別する純関数を実装する
  - Observable: 代表的な入力パターン（正常形・トグル形・カテゴリ単位形・破損形）それぞれについて期待どおりの区分が得られることをテストで確認できる
  - _Requirements: 4.1, 4.2_
  - _Boundary: seriko bind directive_

- [x] 3.3 着せ替え集合の on/off 積算を実装する
  - 現在の着せ替え集合と対象ID・on/off指定から、新しい着せ替え集合を求める純関数を実装する
  - 同じ操作の繰り返しや順序違いでも同じ結果集合になることを保証する
  - Observable: on→on の重複適用や順序を入れ替えた適用列が同一の結果集合になることをテストで確認できる
  - _Requirements: 3.3, 3.4_
  - _Boundary: seriko bind accumulate_

- [x] 4. (P) emo-present 再合成回帰（test-only）
  - 同一サーフェスに対し異なる着せ替え集合で表示を発行すると再合成が発生し、同一の着せ替え集合であれば再合成なしで復帰することを確認するテストを追加する
  - 本体のロジックには変更を加えない
  - Observable: 異なる着せ替え集合ではキャッシュがミスして再合成が走り、同一集合では再合成が走らないことをテストで確認できる
  - _Requirements: 6.1, 6.2, 6.3_
  - _Boundary: emo-present ComposeCache_

- [x] 5. seriko 動的bind状態と表示再発行の判定
- [x] 5.1 per-scope の動的な着せ替え状態を追加する
  - シーンごとの状態管理に、初期値を既存の既定 on/off とする動的な着せ替え集合を追加する
  - 既存の表示発行が、この動的な着せ替え集合を参照するように切り替える（bind操作が行われていない間は従来と同じ結果になる）
  - Observable: bind操作を一切行わない既存経路の表示発行結果が、変更前と同一であることを確認できる
  - _Requirements: 3.1, 3.8_
  - _Boundary: seriko ScopeStates_
  - _Depends: 3.3_

- [x] 5.2 bind 適用結果に応じた表示発行の判定を実装する
  - 着せ替えの有効化・無効化を適用した結果、表示中のシーンであれば新しい着せ替え集合を載せた表示発行が必要と判定し、非表示のシーンであれば状態だけを更新して発行しないと判定する
  - 適用結果が直前と同一の集合になる場合は再発行が不要と判定する
  - この処理がシェル面・バルーン面の既存の状態遷移に影響しないことを保証する
  - Observable: 表示中シーンでの着せ替え変化は新しい表示発行が得られ、非表示シーンでの変化は発行なしで状態のみ更新され、同一集合への変化は発行なしになることをテストで確認できる
  - _Requirements: 3.5, 3.6, 3.8_
  - _Boundary: seriko ScopeStates_
  - _Depends: 5.1_

- [x] 6. seriko bind コマンド消費分岐と起動シグネチャの拡張
- [x] 6.1 bind コマンドの宛名選別と引数解釈への振り分け
  - 汎用コマンド cue のうち宛名が bind であるものを名前一致で選別し、宛名が他のコマンドまたは未登記の場合は静かに読み流す（bind-noevent のような未実装コマンド名も同様に読み流される）
  - 宛名は bind だが中身が壊れている場合は警告として記録する
  - 選別できた入力を、正常形・トグル形・カテゴリ単位形・破損形へ分類し、正常形以外はそれぞれ規律どおりの水準で記録した上で処理を打ち切る
  - Observable: 宛名選別・分類の各パターン（他コマンド宛・未登記・宛名一致だが壊れている・正常形・トグル形・カテゴリ単位形・破損形）それぞれで期待どおりの記録水準と処理継続/打ち切りをテストで確認できる
  - _Requirements: 2.1, 2.5, 4.1, 4.2, 4.4_
  - _Boundary: seriko actor_
  - _Depends: 3.1, 3.2_

- [x] 6.2 起動構成へ名前解決情報を additive に注入できるようにする
  - seriko の起動構成に、bind の名前解決情報を渡せる引数を追加する（未指定時は空の解決情報として扱えるようにし、既存の呼び出し元がそのまま動くようにする）
  - Observable: 名前解決情報を渡さずに構築した既存経路が、変更前と同じ挙動のまま実行できることを確認できる
  - _Requirements: 8.1_
  - _Depends: 6.1_

- [x] 6.3 解決・状態適用・表示発行と実機確認用の記録
  - 正常形として分類された入力について、カテゴリ名・パーツ名を着せ替えIDへ解決し、解決できない場合は記録のうえ状態を変更せず処理を終える
  - 解決できた場合は状態への適用を行い、その結果に応じて表示発行の要否を判定する
  - 着せ替えが適用され表示発行が行われた場合、実機確認で検索できる記録を残す
  - Observable: 解決不能な入力では記録のみで状態・表示に変化がなく、解決できた入力では状態が更新され記録付きで表示発行の要否が判定されることをテストで確認できる
  - _Requirements: 3.2, 3.3, 3.7, 7.1_
  - _Boundary: seriko actor_
  - _Depends: 6.2, 5.2_

- [x] 6.4 既存の seriko 統合テストを新しい起動シグネチャへ追随させる
  - 既存の統合テスト群が、空の名前解決情報を渡して構築するように更新され、挙動が変わらないことを確認する
  - Observable: 既存の統合テストがすべて変更前と同じ結果で緑になることを確認できる
  - _Requirements: 8.1_
  - _Depends: 6.3_

- [x] 7. areka 起動配線と消費者台帳への登録
- [x] 7.1 起動時の資産構築へ bind 名前解決情報を組み込む
  - ゴーストマウント時に取り込んだ bindgroup 名前宣言から、bind 名前解決情報を構築し、起動時に保持する資産へ追加する
  - Observable: 名前宣言を持つゴーストを読み込んだとき、起動時資産から対応する着せ替えIDが解決できることを確認できる
  - _Requirements: 7.1_
  - _Boundary: areka BootAssets_
  - _Depends: 1.2_

- [x] 7.2 seriko 起動呼び出しへ名前解決情報を配線し、bind を消費者台帳へ登録する
  - 起動時資産に構築した名前解決情報を、seriko の起動呼び出しへ渡す
  - コマンド名 bind の担当消費者として seriko を消費者台帳へ登録する
  - Observable: 台帳に bind→seriko の登録が存在し、move の登録と共存すること、および同一名前の重複登録が検出されることを確認できる
  - _Requirements: 2.2, 7.1_
  - _Boundary: areka 起動配線_
  - _Depends: 2.5, 6.4, 7.1_

- [x] 8. (P) bind 決定論エンドツーエンド観測
  - 名前キー bind の on/off 列を含むスクリプトを直接投入し、着せ替え状態の積算と新しい着せ替え集合を載せた表示発行が、注入した時刻だけで（実時間待機なしに）観測できるようにする
  - 解決不能な bind を含むスクリプトを投入した場合は、表示発行が増えずログ事象として観測できることを確認する
  - bind が混在するスクリプトで、文字表示側の処理が汚染されないことも合わせて観測する
  - 本仕様の検証に必要な、宣言済み bindgroup 名を持つ最小構成の fixture を自前で用意する
  - Observable: on/off 列の投入で表示発行列に期待どおりの差分が現れ、解決不能な入力では表示発行が増えないことをテストで確認できる
  - _Requirements: 2.3, 5.1, 5.2, 5.3, 5.4, 5.5_
  - _Boundary: seriko bind e2e_
  - _Depends: 6.4_

- [x] 9. 全体回帰と実機サインオフ
- [x] 9.1 ワークスペース全体の回帰確認
  - 本増分適用後、既存のテストを含むワークスペース全体のテストがすべて成功することを確認する
  - 新規の外部依存を追加していないこと、既存の cue コマンドのワイヤ形式が変わっていないことを確認する
  - Observable: ワークスペース全体のテスト実行が成功で完了する
  - _Requirements: 8.1, 8.2, 8.3, 8.4_
  - _Depends: 7.2, 8_

- [x] 9.2 実機による表情変化の人間サインオフ（10.5 の pattern0 是正込み再実施で充足・2026-07-23 合格）
  - 実ゴースト・実描画環境・実際のDPI設定で起動し、bind スクリプトの発火によって表情パーツが実際に着脱されることを目視で確認する
  - 起動は絶対パスで行う
  - 単発デモへの合わせ込みではなく、本番ゴースト表示を先行させたうえで判定する
  - Observable: 実機での目視確認により、着せ替えパーツの着脱が表情変化として反映されていることが確認できる
  - _Requirements: 7.1, 7.2, 7.3_
  - _Depends: 9.1_

- [x] 10. mustselect 排他選択の実導出（2026-07-23 実機サインオフ改定・R4.5/D11）
- [x] 10.1 (P) descript の mustselect カテゴリ宣言を取り込む
  - `sakura/kero.bindoption*.group,カテゴリ名,mustselect` を走査し、mustselect と宣言されたカテゴリ名の集合をマウントモデルへ転記する（`multiple` 指定・非宣言は既定＝非排他として無視）
  - 本体側・相方側を区別して保持し、あるカテゴリが mustselect かを判別できるアクセサを用意する
  - Observable: emo2 相当の descript を読み込むと、腕・口・眉・目 が mustselect と判別でき、宣言のないカテゴリ（紅など）は非 mustselect と判別できる
  - _Requirements: 4.5_
  - _Boundary: parsers BindGroupNames_

- [x] 10.2 seriko に排他選択の解決・状態・分岐を追加する
  - bind 名前解決情報へ「mustselect カテゴリ集合」と「カテゴリ→着せ替えID索引」を additive に追加し、あるカテゴリが mustselect か・そのカテゴリに属するID集合を引ける純関数を提供する（parsers 非依存・素データ構築は不変）
  - 現在の着せ替え集合から、指定カテゴリの全IDを外したうえで対象IDを有効化する排他置換の状態適用を追加する（結果集合の Changed/StateOnly/Unchanged 峻別は既存と同型・冪等も維持）
  - bind 消費分岐で、着衣かつ mustselect カテゴリのときは排他置換を、それ以外（脱衣・非 mustselect）は既存の加算/除去を用いるよう振り分ける
  - 名前解決情報の起動時注入シグネチャ変更に伴い、既存の呼び出し元（seriko 側テスト・e2e・app 層の橋渡し）を空の排他情報で追随させ、挙動不変を保つ
  - Observable: mustselect カテゴリで新パーツを着衣すると同カテゴリの旧パーツが自動で外れ高々1パーツ有効になる／非 mustselect カテゴリは従来どおり加算されることをテストで確認できる
  - _Requirements: 4.5_
  - _Boundary: seriko bind resolver/state/actor + 呼出元追随_
  - _Depends: 10.1_

- [x] 10.3 起動配線で実 mustselect 情報を構築・注入する
  - ゴーストマウント時に取り込んだ mustselect カテゴリ集合とカテゴリ→ID索引から、起動時の名前解決情報を構築し、seriko の起動呼び出しへ渡す
  - Observable: mustselect 宣言を持つゴーストを読み込んだとき、起動時資産から対応するカテゴリが mustselect と判別でき、そのカテゴリのID集合が引けることを確認できる
  - _Requirements: 4.5, 7.1_
  - _Boundary: areka 起動配線_
  - _Depends: 10.1, 10.2_

- [x] 10.4 (P) mustselect 排他の決定論エンドツーエンド観測とワークスペース回帰
  - 同一 mustselect カテゴリで複数パーツを off なしに着衣する列（実 emo2 の目=笑顔→ジトー→静観 等を模す）を直接投入し、常に高々1パーツのみ有効な着せ替え集合を載せた表示発行になることを注入時刻だけで観測する
  - 非 mustselect カテゴリ（紅など）の明示 on/off が従来どおり成立することも合わせて観測する
  - 本増分適用後、ワークスペース全体のテストがすべて成功することを確認する
  - Observable: mustselect 列の投入で結果集合が「置換」（旧パーツが消え新パーツのみ）になり、非 mustselect は加算のままであることをテストで確認でき、ワークスペース全体が緑になる
  - _Requirements: 4.5, 8.1_
  - _Boundary: seriko bind e2e_
  - _Depends: 10.2_

- [x] 10.5 実機による mustselect 排他＋目開き（pattern0 是正）の人間サインオフ（9.2 の再実施）
  - 実 emo2・実 pasta.dll・実 DPI・絶対パス起動で、(a) mustselect カテゴリ（腕・口・眉・目）の表情パーツが重畳せず1枚ずつ正しく切り替わること、(b) 目が開いている（まばたきの閉じ目フレームがベース目を覆わない）ことを目視で確認する
  - 瞬きアニメーション自体は seriko-loop（M-life）未実装ゆえ「瞬きしないだけ」は許容とする
  - Observable: 実機での目視確認により、着せ替えパーツが積算重畳せず排他的に着脱され、かつ目が開いた状態でむらさきの表情が破綻なく変化することが確認できる
  - _Requirements: 4.5, 7.1, 7.2, 7.3, 9.1_
  - _Depends: 10.3, 10.4, 11.2, 11.3_

- [x] 11. emo-compose pattern0 静的合成の是正（2026-07-23 実機サインオフ第2欠陥・R9/D12）
- [x] 11.1 仕様改定（requirements R9・design D12・tasks 群 11 の焼き込み）
  - 実機サインオフ第2欠陥（常時閉じ目）の真因（emo-compose `flatten_surface` が pattern0 非保持アニメで最小 index の閉じ目フレームを静的合成）を仕様へ反映し、pattern0 厳密選択を本仕様スコープとして明文化する
  - Observable: requirements.md に Requirement 9、design.md に D12＋Out of Boundary/File Structure 改定、tasks.md に群 11 が存在し、10.5 の依存が 11.2/11.3 を含む
  - _Requirements: 9.1, 9.2, 9.3, 9.4, 9.5_

- [x] 11.2 emo-compose の静的合成を厳密 pattern0（index==0）選択へ是正する
  - `flatten_surface`（plan.rs:307）の pattern 選択を最小 index フォールバックから index==0 厳密へ変更し、pattern0 を持たない bind animation（まばたき等の再生専用フレーム）は静的合成へ寄与させず良性 skip（debug ログ）する
  - キャンバス外形算出（plan.rs:529）は無改変とし、bind オン/オフ・pattern0 有無でサイズ不変の契約を維持する
  - pattern0 非保持 skip の判断分岐と emo2 まばたき bind の静的不活性を決定論テストで網羅する（pattern0 無しは寄与なし・pattern0＋pattern1 共存で pattern1 は不採用・skip の debug ログ正カウント・emo2 surface1000 が まばたき bind 有無で同一描画命令列＝「常時閉じ目」再発檻・既存 golden 非退行）
  - Observable: pattern0 を持たない有効 bind が生成描画命令に現れず、pattern0 を持つ bind は従来どおり pattern0 のみ合成され、既存 emo-compose テスト（golden 含む）が緑のまま残る
  - _Requirements: 9.1, 9.2, 9.3, 9.5_
  - _Boundary: emo-compose plan.rs_
  - _Depends: 11.1_

- [x] 11.3 ワークスペース全体の回帰確認（pattern0 是正後）
  - 本増分適用後、既存のテストを含むワークスペース全体のテストがすべて成功することを確認する（emo-present ComposeCache 檻・seriko bind_e2e・mustselect 檻・emo-compose golden を含む）
  - Observable: ワークスペース全体のテスト実行が成功で完了する
  - _Requirements: 8.1, 9.4_
  - _Depends: 11.2_

## Implementation Notes

- (task 6.1-6.4) `spawn_seriko` へ `bind_resolver: BindResolver` を additive 追加（第3引数）＝seriko in-source/統合テスト・areka 本番 mod.rs:288/spine.rs:444 の全呼出元を跨ぐコンパイル結合ゆえ 6.1〜6.4 を1つの atomic 変更として実装。**areka 側は `BindResolver::empty()` の暫定コンパイル橋（`// TODO(task 7.2)` マーカー付き）で緑を維持**——task 7.2 が `BootAssets.bind_resolver`（実名前解決表）へ差し替える。bind 分岐は `handle_message` の `cue_target_of==None` 枝内・Wait 前（D1）・`name=="bind"` 自己選別。D8 severity 全縮退枝を `capture_logs_flow` でログ捕捉檻化（bind_* 9本・対比含む）。実機 grep マーカー `info!("seriko: bind 適用")` は Changed のみ発火。**推移的注記**: empty resolver は実 `\![bind]` で ERROR ログ（D8①解決不能）を出すが表示無変化＝task 7.2 完了で解消。
- (task 2.1-2.4) `command_target_of` は dola 本体・dola tests・areka-sakura・areka move_cue の**本番 use を跨ぐ共有シンボル**で、削除は独立コンパイル不能ゆえ 2.1〜2.4 を1つの atomic リファクタとして一括実装・一括コミット（レビュー APPROVED）。severity は D8④ 宛名規律（自分宛破損=warn／担当外=debug）へ整列し、`with_default` ログ捕捉檻で3アームを非空虚化（`move_severity_log_tests` 4本・対比檻含む）。既存の areka ログ捕捉定石は `adapter.rs:306-342` のインライン Capture Layer。「1名前=高々1消費者」は台帳（task 2.5・7.2）で保証する構造へ移設済み。
- (task 1.3) `areka-parsers/src/package/mod.rs` は `BindGroupDefaults` を `pub use` するが `BindGroupName`/`BindScope` は未 re-export。parsers 内部テストは module パスで到達済みだが、task 7.1（areka app 層が `MountModel.bindgroups` の名前転記から `BindResolver` を構築）で公開ファサード経由の到達が必要になれば、この 2 型を `mod.rs` で re-export すること。

# Implementation Plan — areka-P0-balloon-vertical-canon

> 生成: 2026-08-28（`-y` 自動承認）。設計 Migration 6 段に対応（群 1=段1・群 2=段1・群 3=段2・群 4=段3＋段4〔不可分・ただし「檻を先に・編集を後に」へ並べ直し済み——宣言削除はクランプ有無どちらでも挙動不変のため、各タスク末で常に全緑を保てる〕・群 5=段2/4・群 6=段5・群 7=段6）。
> **file:line は 2026-08-27〜28 実測**。着手時に必ず引き直すこと（本リポジトリで陳腐化を通算 8 度踏んでいる）。
> 実装上の正本: 型と契約は design.md の C1〜C9／DD1〜DD9・記録水準は Error Handling 表・§8 の行内容は Data Models 登記台帳。

- [x] 1. 転記層の受口（`vertical` の生値転記）
- [x] 1.1 `vertical` 生値の保持を balloon モデルへ追加する
  - 生値フィールド＋additive ビルダー（`with_cursor`／`with_windowposition_raw` と同じ流儀）＋アクセサ。`new()` の 7 引数署名は非改変
  - 未宣言（`None`）と宣言（空文字列含む）を潰さない。解釈・検証・警告は一切行わない（転記層の無警告契約）
  - モデルのテストに「additive 既定は未宣言」「未宣言と `"0"` 宣言の区別」を追加
  - **Observable**: 既存 30 呼出箇所を 1 つも変えずにワークスペースがコンパイルされ、新テストが緑
  - _Requirements: 1.4, 1.8_
- [x] 1.2 `vertical` の転記と 2 層マージを通す
  - `writing_mode` の転記の隣へ 1 行、末尾のビルダー鎖へ 1 行。**キー非依存のマージ関数は非改変**（追加コード 0 で後勝ちが成立することの証跡）
  - 解析のテストに 4 形を追加: 単層宣言／面別上書き層の後勝ち／未指定は `None`／語彙外値の素通し（既存 `writing_mode` テスト群と同型）
  - **Observable**: 4 テスト緑・転記層のログ発行 0 件のまま
  - _Requirements: 1.4, 1.5, 10.4_

- [x] 2. 書字方向の唯一の決定点
- [x] 2.1 書字方向の決定記録型を新設し既存の解決を委譲へ縮小する
  - 宣言分類 2 種（正典キー: 未宣言/横/縦/不正・拡張キー: 未宣言/宣言/未知）と採用出所の enum、決定記録型（`mode`／`source`／`conflicting`／`.vertical` 導出値）を design C2 の契約どおりに実装
  - 共存規則: 有効宣言なし→正典既定の横書き（記録なし）／単独→そのキー／一致併記→無記録／**矛盾併記→拡張キー採用＋DEBUG 記録（両キーの生値を構造化フィールドで・resolve の内側で）**／不正値・未知値は「指定なし」として合流（DD6・警告は発行済み）
  - 既存の解決 API は決定記録の `.mode()` を返す薄い委譲へ（**戻り値型不変＝既存 14 呼出箇所は無改変**）
  - 記録水準は design Error Handling 表が正本（不正値＝warn・矛盾併記＝debug・両方なし＝無記録）
  - **Observable**: 既存 writing.rs インライン `mod tests`（warn 件数の逐語固定含む）が**無改変で緑**（2.3 の証跡）・`actor.rs:153` 非改変
  - _Requirements: 1.1, 1.2, 1.3, 1.6, 1.7, 2.1, 2.2, 2.3, 2.4, 2.5, 2.6, 2.7, 9.1, 9.2_
- [x] 2.2 決定点の檻を新設する（兄弟テストファイル）
  - `writing_decision_tests.rs` 新設＋`lib.rs` の `PURE_SOURCES` へ列挙（列挙しないと被覆が黙って縮む）
  - `vertical` 分類 4 分岐（未宣言／`0`／`1`／不正値＝`2`・空文字列。不正値は warn ちょうど 1 件）
  - 共存規則 6 組合せ（単独 2 形＋3 形／一致併記＝記録 0 件／不一致併記＝拡張キー採用＋debug ちょうど 1 件／未知値＋`vertical` 宣言＝`vertical` 採用）
  - 記録水準は `count_levels` で warn／debug を**別々に**逐語固定。0 件主張は捕捉窓内の対照イベント込み（恒真檻の禁止）
  - `.vertical` 導出 3 値（横→`0`・`vertical_rl`→`1`・**`vertical_lr`→`1`**）
  - **Observable**: 新檻全緑＋`PURE_SOURCES` 構造檻緑
  - _Requirements: 1.6, 1.7, 2.2, 2.3, 2.4, 2.5, 2.7, 7.1, 10.3, 10.6_

- [x] 3. 一致の固定（コード変更 0・檻のみ）
- [x] 3.1 (P) 縦書き座標意味論の檻を新設する
  - `region_vertical_canon_tests.rs` 新設＋`PURE_SOURCES` へ列挙
  - `wordwrappoint.y` の既定＝`validrect.bottom`・負値＝下辺基準／**`wordwrappoint.x` だけを変えた 2 モデルが同一の `TextRegion` を与える**（型の保証を読める形へ）／`validrect` 4 辺が横書きと同一に解決される
  - SC5（列の上限＝`validrect.left`）は**既存挙動＋既存檻**（`layout_visible_window_tests.rs:60-79`）の確認のみ——新規の檻は作らない
  - **Observable**: 新檻緑・`region.rs` 本番コード非改変
  - _Requirements: 3.4, 3.5, 3.6, 3.8, 10.5, 10.6_
  - _Boundary: C4（region 檻のみ）_
- [x] 3.2 (P) フォント縦書き等価の構造檻を追加する
  - `draw_format_metrics_tests.rs`（兄弟テスト）へ追記——**`draw.rs`（974 行・上限まで 26 行）へは 1 行も足さない**
  - 3 モードの `reading`／`flow` 写像（縦書き 2 モード＝`TOP_TO_BOTTOM`＋`RIGHT_TO_LEFT`／`LEFT_TO_RIGHT`）
  - 本番ソースに `@` 前置のフォント名生成・標準ゴシックへの差し替えが**存在しない**こと（字面檻・「何を守っているか」を檻の doc に明記）
  - `DirectionRecipe::for_mode` の本番呼出が `create_text_format` の内側 1 箇所のみ（計測と描画が同じ工場を通る証跡）
  - **Observable**: 檻緑・`draw.rs` 974 行のまま・1,000 行番人緑
  - _Requirements: 6.1, 6.2, 6.3, 6.4, 6.6_
  - _Boundary: C5（draw 兄弟テストのみ）_

- [x] 4. origin クランプ正準の撤去と追随（段3＋段4＝不可分の論理単位）
- [x] 4.1 意味論の棚卸し（着手時・どの編集よりも先）
  - repo 全域（`crates/**`・`examples/**`・`tests/fixtures/**`）の `origin.x`／`origin.y` 宣言を列挙し、各々の **2 層マージ後の validrect と突合**して内外を判定する（語 grep は使わない——当該語を含まない定義ファイルを原理的に見つけられないため）
  - 範囲外集合が既知 4 件（`emo2-vertical`／`emo2-choice/descript-cursor`／`emo2-choice/descript-plain`／`emo2-kakukaku`）と一致することを確認。**5 件目が出た場合は DD5（意図別: 縮退なら宣言削除・宣言そのものなら期待値を字義位置へ）で分類してから先へ進む**
  - **Observable**: 棚卸し記録（対象ファイル一覧・内外判定・是正方針・方法の限界「語 grep では見つからない類が在る」）を **4.2 で新設する檻のモジュール doc へ恒久記録**する（セッション記録に残さない）
  - _Requirements: 3.10, 10.7_
- [x] 4.2 実ゴースト開始点の檻を新設する（クランプ現存下で先に固定）
  - `tests/shipped_fixture_region_test.rs` 新設——`emo2-kakukaku` の `descript.txt`＋`balloons0s.txt`／`balloonk0s.txt` を 2 層マージ→`TextRegion` 解決で **sakura (36,46)／kero (24,40) を逐語固定**（現在この観測点は 0 本＝全緑のまま壊れる唯一の穴）
  - **檻は宣言がまだ在る状態（クランプ経由）で書く**。以後 4.3 の宣言削除・4.4 のクランプ撤去を**またいで無改変のまま緑**であり続けることが、両編集の挙動不変の反証可能な証跡になる
  - モジュール doc に 4.1 の棚卸し記録（方法・対象・限界）を収める
  - **Observable**: 新檻緑（この時点の実挙動を固定）
  - _Requirements: 3.10, 10.7_
  - _Depends: 4.1_
- [x] 4.3 フィクスチャと実ゴースト定義を正典推奨形へ是正する（挙動不変）
  - `emo2-vertical/descript.txt:15-16`・`emo2-choice/descript-cursor.txt:18-19`（＋:17 のクランプ言及コメント是正）・`emo2-choice/descript-plain.txt:17-18`・`crates/pilot/.../emo2-kakukaku/descript.txt:13-14` の origin 宣言を削除（**pilot はデータファイルのみ・コードは非接触**）
  - **【4.1 棚卸しで追加】`crates/pilot/.../emo2-kakukaku-wplimit/descript.txt:13-14` の origin 宣言も削除する（5 件目）**。design C9 が「範囲 [0,0] 境界内で不変」としたのは基層のみを見た誤判定で、面別上書き層を重ねると原本と同一の範囲外。削除で開始点 sakura (36,46)／kero (24,40) が不変
  - **【4.1 棚卸しで追加＝第 3 類の追随】宣言削除で赤くなる既存テストを同一コミットで是正する**——`crates/areka-parsers/src/balloon/validation_tests.rs:60-61／:116-117／:158-159`（3 テスト・6 assert）と `crates/areka-emo-present/src/balloon_model_tests.rs:118-123`。いずれも「基層の値が面別上書き層に無くても継承される」ことの**証拠**として `origin().x() == Some(0)` を使っている。**期待値を `None` へ替えるだけにしないこと**——それでは継承の被覆が消えて要件 10.7 に反する。**継承の証拠を同条件で継承される別キー**（`wordwrappoint.y`／`font.height`／`font.color` 等）**へ移す**こと。あわせて `validation_tests.rs` の doc（:6・:8-10・:53 の「採取元: descript.txt L13–L14」）の陳腐化も是正する
  - 削除はクランプ現存下でも挙動不変（未宣言→書字開始角＝同値）
  - **Observable**: 4.2 の檻・`vertical_fixture_test.rs:117` の `(356,46)`・choice の `(5,5)` が**すべて無改変のまま緑**
  - _Requirements: 3.10, 10.9_
  - _Depends: 4.2_
- [x] 4.4 クランプを撤去し宣言 origin を字義どおりにする
  - `clamp_origin_component` → `resolve_origin_component` 改称。`Some` 腕は負値（反対端基準）解決後**そのまま返し**、validrect 外なら debug 1 件。`None` 腕と `start_corner` の match は非改変。**validrect 引数が返値に影響しない**ことを不変条件とする（クランプが残っていない証拠）
  - モジュール doc（`region.rs:3`／`:24-27`／`:177`／`:189`／`:211`／`:271`・`layout.rs:29` の 1 行）を本仕様の規約へ指し直す
  - `region.rs` インライン既存テスト 5 件を DD5（意図別）で是正——期待値だけを機械的に書き換えない
  - **【4.1 棚卸しで追加】in-code の範囲外 origin 宣言も DD5 で是正する**——`crates/areka-emo-text/tests/draw_readback_test.rs` の `validrect_model`（＋doc のクランプ言及）／`crates/areka-emo-text/tests/scale_invariance_test.rs` の 3 箇所（＋doc）／**`crates/areka-emo-text/src/actor_scale_refresh_tests.rs:116-124`（design のどの行にも無かった・撤去後も緑のままなので全緑では検出できない）**。いずれも意図は「書字開始角の縮退」なので `Origin::new(None, None)` へ
  - origin 4 分岐の檻を `region_vertical_canon_tests.rs` へ追加: validrect 内宣言（字義・記録 0）／外宣言（**字義**＋debug 1）／未宣言（開始角＋debug 1）／負値宣言
  - **Observable**: crate 全緑＋**4.2 の檻が無改変のまま緑**＋4 分岐檻緑
  - _Requirements: 3.1, 3.2, 3.3, 3.7, 3.8, 3.9, 3.10, 3.11, 10.5, 10.7_
- [x] 4.5 文言掃除（語 grep は文言網羅のみに使う）
  - 「クランプ」「clamp_origin」「書字開始角」の語 grep で doc／assert メッセージを是正: `scale_invariance_test.rs`・`draw_readback_test.rs`・`pipeline_test.rs`・`layout_wrap_tests.rs`・`draw_oracle_tests.rs`・`canvas.rs`・`viewbox_draw_test_support.rs:93-96`（挙動はいずれも不変——validrect が画像端一致 or 未指定経路）
  - **Observable**: `crates/**` で「クランプ正準」ヒット 0 件＋**陽性対照**＝同一 grep が `.kiro/specs/areka-P0-balloon-vertical-canon/` 配下で ≥1 ヒット（道具と pathspec の生存証明）＋対象ファイルの実在列挙
  - _Requirements: 3.10, 10.7_

- [x] 5. 正典キー版フィクスチャと同値檻
- [x] 5.1 `emo2-vertical-canon` を新設し拡張キー版との同値を檻にする
  - `descript.txt` は既存 `emo2-vertical` との差分を **`writing_mode,vertical_rl` → `vertical,1` の 1 行だけ**に保つ・`balloons0s.txt` は同内容・origin 宣言なし（正典推奨形）・枠画像は共有フィクスチャを借用（複製しない）
  - `vertical_fixture_test.rs` へ追加: 正典キー版が縦書きへ解決される（基層のみ／2 層マージ後の双方）・**両版の `WritingMode` と `TextRegion` 全成分の逐語一致**
  - 期待 `TextRegion`＝left 36／top 46／right 356／bottom 168／start (356,46)／wrap 164（design Data Models の表と一致）
  - **Observable**: 同値檻緑・既存 4 テスト無改変で緑
  - _Requirements: 10.1, 10.2, 10.9_
  - _Depends: 2.1, 4.3_

- [x] 6. 互換台帳への登記（文書のみ・コード非接触）
- [x] 6.1 双方向登記の着手時再検証（DD8 前半・6.2〜6.4 の前提）
  - design.md「追跡先の双方向登記」表 6 行の file:line を**引き直す**（追跡先 brief は同ウェーブ中に動きうる）。変動があれば表を追随
  - **Observable**: 6 行それぞれの再検証結果（一致／追随内容）が design.md の表に反映されている
  - _Requirements: 4.5, 5.5, 7.5, 8.4_
- [x] 6.2 COMPAT §8 行 1〜2 を登記する（`writing_mode` 優先順位・クランプ撤去の上書き行）
  - 行内容は design Data Models 登記台帳 #1〜#2 が正本。上書き行は §8 :153（scg が R2.9 を上書きした行）を雛形とし、出所（`completed/areka-P0-emo-text-layer/design.md:464`／`:716`）を名指し・§8 :170 の別種クランプ（`balloon_limit.rs`）と項目名で区別
  - **挿入点は編集時に §8 末尾を再導出する（`:175` を信用しない）**——zsp が同ウェーブで同じ表末尾へ追記予定＝後着側が rebase を負う
  - **Observable**: 行 1〜2 が表に存在し、内容が台帳 #1〜#2 と逐語対応・出典列に本仕様＋要件/SC 番号
  - _Requirements: 2.8, 3.10, 11.1, 11.2, 11.3, 11.4, 11.8_
  - _Depends: 6.1_
- [x] 6.3 COMPAT §8 行 3〜8 を登記する（未実装語彙＋追跡先）
  - 台帳 #3〜#8: フォント等価（`@` 非使用・差替非模倣・グリフ一致非保証）／切替なし（SC11）／列の上限（SC5＝既存挙動）／`\f` 写像（SC1 の採択側と理由・decoration へ継承）／矢印（SC10・residue 項目 1 第 3 軸）／`\_l` 正典と既知非互換（SC8/SC9/SC15・正典文の逐語引用・cursor-tag-canon）
  - 各行の根拠列に「**正典側は未規定のまま**」を明記（解決済みと偽らない）。追跡先名指しは 6.1 の再検証結果に基づく
  - **Observable**: 行 3〜8 が存在し台帳と逐語対応・コード非接触（記録は表示結果を変えない）
  - _Requirements: 4.1, 4.2, 4.3, 4.5, 4.6, 5.1, 5.2, 5.3, 5.4, 5.5, 5.6, 5.7, 6.5, 9.3, 9.4, 11.5, 11.6, 12.3, 12.4_
  - _Depends: 6.1_
- [x] 6.4 COMPAT §8 行 9〜13 を登記する（プロパティ族＋正典参照・不安定さ）
  - 台帳 #9〜#13: `.vertical` 導出規則と 2 つの穴（枝の不在・照会経路の不在）／同族 `validwidth`/`validheight`/`lines` の 2.8.83 意味論と 2.8.80 逆転（SC4/SC13）／`origin.y` 分岐なし（SC6）／正典参照の出所（**snapshot 陳腐化はプロパティ節限局・座標節は一致**）／正典側の不安定さ（SC14・追随点 2 関数）
  - **Observable**: 行 9〜13 が存在し台帳と逐語対応
  - _Requirements: 3.3, 7.2, 7.3, 8.1, 8.2, 8.3, 11.7, 12.1, 12.2, 12.5, 12.6_
  - _Depends: 6.1_
- [x] 6.5 (P) `emo2-conformance-scope.md` の陳腐化を是正する
  - `:85` の「縦書きを M2 へ後ろ倒し」を本仕様（M1・W6.95）へ追随・**`:61` の適合スコープ判断（痕跡なし・適合 14 項目に不要）は変更しない**・`:60` の `\f[]` に文字装飾系 3 spec の所有確定への参照を添える
  - **Observable**: 当該 3 箇所の差分のみで他行不変
  - _Requirements: 11.9_
  - _Boundary: doc/emo2-conformance-scope.md（6.2〜6.4 の COMPAT_ARCHITECTURE.md とファイル非重複）_

- [x] 7. 最終ゲート
- [x] 7.1 全体検証と双方向登記の完了時再検証
  - ワークスペース全テスト緑（**ただし「全緑」を十分性の証拠にしない**——4.1 の棚卸し記録と各檻の存在が正）
  - `layout_cursor_tests.rs`（670 行・13 本）が**無改変で緑**＝`\_l` を 1 ビットも変えていない証跡
  - 1,000 行番人緑（`draw.rs` 974 のまま非接触・例外表非改変）・`PURE_SOURCES` に新規 2 本が列挙済み
  - **非接触の確認**: `crates/areka-sylphya/**`・`emo2_boot/**`・`placement/**`・`presenter/**`・`crates/pilot/**` のコード・`.kiro/specs/completed/**` に差分 0（プロパティ照会は現行どおり値なしのまま＝7.4/8.5/8.6・面別上書き解決規則不変＝9.5 の証跡。**pathspec の実在を証明してから「差分なし」を記録する**）
  - §8 の 13 項目名（①writing_mode ②クランプ撤去 ③フォント等価 ④切替なし ⑤列の上限 ⑥\f 写像 ⑦矢印 ⑧\_l ⑨.vertical 導出 ⑩同族意味論 ⑪origin.y 分岐なし ⑫正典参照の出所 ⑬正典の不安定さ）を**逐語突合**——§8 末尾は再導出する（`:175` 非信用）
  - 双方向登記表 6 行の file:line を**もう一度**引き直す（DD8 後半＝完了時レグ・追跡先 brief の変動があれば追随）
  - **Observable**: 全ゲート緑＋非接触確認（pathspec 実在証明付き）と再検証結果が記録されている
  - _Requirements: 4.4, 4.5, 7.4, 7.5, 8.4, 8.5, 8.6, 9.5, 10.7, 10.8_

## Implementation Notes

> 実装中に判明した横断的な知見。以降のタスクの実装者・レビュアーはここを先に読むこと。

- **ワークツリーでは submodule `vendors/pasta` が未 populate**（1.1 着手時）。素の `cargo` が `pasta_core` を解決できず即死する。親が `git submodule update --init --recursive` で解消済み。以降のタスクで submodule コマンドを再実行する必要はない。
- **`cargo fmt --all -- --check` を検証コマンドに含めること**（1.1 で `model_tests.rs` に 1 件の違反が漏れ、1.2 のレビューで検出された）。`cargo test` は fmt 違反を検出しない。各タスクの完了前に workspace 全体で違反 0 件を確認する。
- **file:line の陳腐化は 1.1／1.2 の時点では発生していない**（`parse.rs` の `writing_mode`:110・`budoux_newline`:113・ビルダー鎖 :151-152 は design 実測どおりだった）。ただし群 4 以降は自分の編集で必ずずれるため、引き直しを省略しないこと。
- **RED は「実装前に落ちること」を実測で確かめる**。1.2 では 4 本のうち 1 本（未指定は `None`）が実装前から緑になってしまい、`vertical,0` 宣言との対比を同一テスト内へ入れて真に赤くした。恒真テストを書いていないかを毎回疑うこと。
- **grep の 0 件主張には陽性対照を添える**（無警告契約の確認など）。空出力は「無い」と「grep が空振りした」を区別しない。
- **群 2 の共存規則で design 未明文の 2 点を確定した（2.2 以降の檻はこれに従うこと）**: ⑴ `vertical,1` ＋ `writing_mode,vertical_lr` は**異なる方向**＝`conflicting()` true・debug 1 件（要件 2.2 が `vertical,1` を `VerticalRl` へ逐語固定・Flow 1 の一致腕が「**その方向**を採用」と単数で書かれているため）。⑵ 両キーが有効宣言なら `source()` は常に `ExtensionKey`（方向一致時も含む・`DirectionSource` に「両者一致」の枠が無いため）。いずれも最終 `WritingMode` は不変で、差は記録の有無と `source()` の値のみ。実装者・レビュアーの双方が独立に spec 整合と判定した。
- **`writing.rs` の doc に残る `R5.x` は完了 spec `areka-P0-emo-text-layer` の要件番号**であり、本仕様の Requirement 5（縦書きで意味が変わる正典語彙）とは別物。ファイル内一貫性のため 2.1 では現行番号体系を踏襲した。**群 4 以降で新たに要件番号を doc へ書くときは出典 spec 名を添えること**。
- **`wordwrappoint.y` の負値の基準は「ベース画像の下辺（image height）」であって `validrect.bottom` ではない**（`region.rs` の `resolve_or(model.wordwrappoint().y(), height, bottom, ...)`＝extent が height・fallback が bottom）。未宣言のときだけ `validrect.bottom` へ縮退する。実装者・レビュアーが独立に file:line で裏取りし、3.1 の檻が変異注入でこの区別を単独で捕まえることも実測済み。**群 6 の COMPAT 登記で「下辺基準」と書くときはこの区別を落とさないこと**。
- **3.1 の檻はクランプ非依存に作ってある**（origin は未宣言か validrect 内側の値のみ）。レビュアーが `clamp_origin_component` を無効化した状態で新檻 7 本が全緑であることを実測済み＝**4.4 のクランプ撤去で偽の赤を出さない**。4.4 で赤くなるのは `region.rs` の既存インライン `mod tests` 5 本だけのはず（DD5 で意図別に是正する対象）。
- **1,000 行番人の実体は `crates/log-capture-kit/tests/file_length_guard_test.rs`**（`LINE_LIMIT = 1000`・`OVER_LIMIT_ALLOWED` 例外表 11 件・件数定数 `OVER_LIMIT_ALLOWED_COUNT`）。検証コマンドに `cargo test -p log-capture-kit` を含めること。`cargo test -p areka-emo-text` では番人は走らない。
- **3.2 の字面檻は `draw.rs` の字面に強く依存する**（`pub fn create_text_format(` の書式・`pub const DEFAULT_FONT_NAME` の可視性・`for_mode` の全出現が 2 件であること・`seam @ (…)` 束縛が `@` の唯一の供給源であること）。レビュアーが 7 種の変異で空振りしないことを実証済みだが、**`draw.rs` を触る後続タスクは無い前提**で成立している（4.5 の対象一覧に `draw.rs` は無い・7.1 は「非接触」を確認するだけ）。将来 `draw.rs` をリファクタするときは檻の側も更新すること（檻の doc に自己説明済み）。
- **付録 A の「複製 fixture」の読み方に注意**: `emo2-kakukaku-wplimit` は `descript.txt` と 2 枚の PNG が原本とバイト同一だが、**面別上書き層 2 本はバイト同一ではない**（`windowposition.*` の行が異なる＝同 fixture の `readme.txt` が明記する意図的な差分）。`validrect`／`wordwrappoint` は同値なので `TextRegion` は一致する。4.3 で「複製だから上書き層も同じ」と読むと誤る。
- **4.2 の檻は群 4 の順序が意味を持つことの証跡そのもの**。レビュアーが独立に ⑴ クランプ無効化→**赤 3 本** ⑵ 両 fixture の origin 宣言削除（4.3 単独）→**緑 6/6** ⑶ ⑴＋⑵ の合成（群 4 完了後の姿）→**檻を 1 バイトも変えずに緑 6/6** ⑷ fixture のリネーム→**フルパス付き panic で赤**、を実測済み。**4.3・4.4 の実装者は、この檻が赤くなったら自分の編集が挙動を変えた証拠だと理解すること**（檻の期待値を書き換えて緑にするのは禁止）。
- **Git Bash の `sed -i` はファイル全体の CRLF を LF へ書き換える**（本 repo は `core.autocrlf=true`・`.gitattributes` 無しでワークツリー規約が CRLF）。コミット差分は正規化で綺麗に見えるのでレビューをすり抜ける。4.3 では期待バイト差 24 に対し 54/55/46/107/107 になったことで検出し復元した。**ファイル編集後は必ずバイト差か CR 数／LF 数の一致を確認すること。**
- **完了 spec `areka-P0-balloon-parse` の R5.1 が 4.3 で陳腐化した**（「`origin`(0,0) を含むモデルを生成する」と逐語で述べているが fixture から宣言が消えた）。アーカイブ本体は非改変が規律なので、乖離は `validation_tests.rs` のモジュール doc に経緯付きで可視化してある。**群 6 の COMPAT 登記でこの上書きも記録すること**（design DD4 の同型＝上書きした出所を名指しする）。
- **`emo2-choice` の 2 fixture は開始点 (5,5) を観測する消費者を持たないまま宣言削除された**（`choice_fixture_test.rs` は `TextRegion` を解決しない）。4.1 の棚卸しが着手前に記録済みの既知の穴で 4.3 が作ったものではないが、**7.1 の最終ゲートで意識すること**。
- **4.5 の文言是正の実対象は design C9 の一覧より狭い**。実装者・レビュアーが独立に実測したところ、「クランプ正準」の**生きた文言**として残るのは `crates/areka-emo-text/src/viewbox_draw_test_support.rs` の 1 件のみ（`vertical_fixture_test.rs`／`layout_wrap_tests.rs`／`pipeline_test.rs`／`draw_oracle_tests.rs`／`canvas.rs`／`scale_invariance_test.rs`／`draw_readback_test.rs` は 0 件か 4.4 で是正済み）。`region.rs` に残る 5 箇所は**すべて「撤去された」「かつて」を伴う歴史記述**で生きた主張ではない。**4.5 は着手時に自分で grep を引き直すこと**（design の一覧を鵜呑みにしない）。
- **4.4 で `region.rs` の `Some` 腕の `debug!` から構造化フィールド `corner` が落ちた**（結果に関与しなくなったため）。`None` 腕の文言・フィールドは逐語不変。記録水準表に `corner` の規定は無いので仕様違反ではない。
- **`region.rs` の末尾 4 行（3.1 が追加した接続宣言）だけが LF 単独**で残り 752 行は CRLF。コミット時に正規化されるので差分・挙動には出ないが、群 5 以降で `region.rs` を触るときは認識しておくこと。
- **bash の二重引用符の中にバッククォートを書かないこと**。`python -c "..."` の中に Markdown のコード表記を入れると bash がコマンド置換として実行してしまい、tasks.md の申し送り 3 行から識別子が丸ごと消えた（4.4 のコミットで実際に踏み、次のコミットで是正した）。この種の編集はスクリプトファイル経由か単一引用符で行うこと。
- **6.2 で行 2 の出典列へ疑義 SC15 を追加した**（台帳 #2 の要件欄は 3.10／3.11 のみだが、`requirements.md` の疑義↔要件対応表が SC15 を R3.10 と明示的に結び付けているため）。**SC15 の主登記先は台帳 #8**（`\_l` の縦書き座標系の行）なので、**6.3 で #8 を書く担当は SC15 が §8 に 2 度現れることを意識し、主登記が #8 側だと読み手に分かる書き方にすること**。
- ⚠**`writing.rs` の未知値 warn の文言が実挙動とわずかにずれている**（「未知の writing_mode 値のため horizontal_tb へフォールバックする」と述べるが、2.1 以降の実挙動は「指定なし扱い」で、`vertical` が宣言されていればそちらが採られる）。**design の Error Handling 表が「現行の文言・件数を維持」と明記しているため本仕様では直さない**（既存インラインテストが逐語固定してもいる）。要件 2.7 を実装／檻掛けする後続 spec への申し送り。
- **6.3 の行 4（会話中の切替なし）は追跡先 spec を 1 本も名指ししていない**。要件 9.4 は「語彙の記録と**追跡先を伴って**登記する」と命じているが、裁定 5 で「実装しない」と確定した以上、引受先が存在しない。行の中身に「areka はこの状態に入らないため引受先を置かない」旨を添えれば完全になる。**7.1 の最終ゲートで判断すること**（台帳 #4 自身が追跡先を要求せずに 9.4 を要件欄へ載せているので、設計側で確定済みの構図でもある）。
- **7.1 の §8 逐語突合で使う 13 項目名**（`doc/COMPAT_ARCHITECTURE.md` の連続 13 行・出典 spec 列が本仕様）: ⑴`writing_mode` の存在・語彙・`vertical` との優先順位 ⑵バルーン文字の描画開始点＝origin クランプ撤去 ⑶フォント縦書き異体の挙動等価 ⑷会話中の書字方向切替 ⑸縦書きで列が並ぶ範囲の上限 ⑹`[align]`／`[valign]`／下線の縦書き写像 ⑺`arrow0`／`arrow1` の縦書き再解釈 ⑻`\_l` の縦書き座標系の正典写像と areka の既知非互換 ⑼`currentghost.balloon.scope(ID).vertical` の導出規則と 2 つの穴 ⑽同族 `validwidth`／`validheight`／`lines`（＋各 `.initial`）の意味論 ⑾`origin.y` の既定に縦書きの分岐が無いこと ⑿本仕様が引いた正典参照の出所 ⒀正典側の不安定さ。**行 9 だけは「正典側は未規定のまま」の定型を持たない**（2 つの穴は areka 側の欠落であって正典の未規定ではないため・台帳 #9 も要求していない）。定型の有無を機械照合しないこと。
- **付録 A-1 の「範囲内だった in-code 宣言」の列挙に `canvas.rs`（6 箇所）が漏れている**（7.1 のレビュアーが実測で発見）。分類自体は正しい——当該モデルは validrect が全 `None`＝画像全域なので origin(0,0) は範囲内で是正不要。**網羅記載の漏れであって判定の誤りではない**。付録 A をやり直すときは補うこと。
- **4.5 の Observable「`crates/**` で「クランプ正準」ヒット 0 件」は literal には成立しない**——4.4 が DD4 の要求で意図的に歴史記述を書き込んだため。**親の裁定で「生きた主張 0 件＋残存は全件が撤去の目印（撤去された／かつて／旧／もう残っていない）を伴う歴史記述」へ読み替えた。**実装者・レビュアーが独立に全数調査し、生きた主張 0 件・目印を欠く歴史記述 0 件・「書字開始角」53 件は全件が未宣言の縮退か行内 alignment で正しい語、を実測。陽性対照は spec 配下 77 件。**この読み替えは 7.1 の最終ゲートでも同じ基準を使うこと**（件数は 8→9 に増えているので、数だけ見て後退と読まない）。

---

## 付録 A: 4.1 意味論の棚卸し記録（2026-08-28 実測）

> **これは 4.1 の Observable の正本である。** 4.2 でこの内容を新設の檻
> `tests/shipped_fixture_region_test.rs` のモジュール doc へ恒久記録する。移送後も
> 監査証跡としてここに残す。**数値はすべて着手時に自分で計算した実測**であり、
> design.md C9 の候補地表（2026-08-27〜28 実測）を写したものではない。

### A-0 前提となる解決規則（実物から引き直した）

- 2 層マージ: `crates/areka-parsers/src/balloon/parse.rs` の `parse` — descript 基層を複製し、面別上書き層の各エントリを後勝ち `insert` で重ねる（キー非依存）
- 座標解決: `crates/areka-emo-text/src/region.rs`
  - `resolve_coord` = `v >= 0 ? v : extent + v`（負値＝反対端基準）
  - `resolve_or` = 未宣言は fallback（validrect は画像端）へ縮退
  - `clamp_origin_component` = `Some` は解決後に `range.0 <= resolved && resolved <= range.1`（**両端含む**）なら素通し、外なら書字開始角へ寄せる。x/y は独立判定
  - 書字開始角: `HorizontalTb`／`VerticalLr` = (left, top)、`VerticalRl` = (right, top)
- 棚卸し時点でクランプは**現存**。下表の「現行 start」は実装の現在値である

### A-1 対象ファイル一覧

**(1-a) origin 宣言を持つバルーン定義データファイル＝5 本**（いずれも `origin.x,0`／`origin.y,0`）

| # | ファイル |
|---|---|
| D1 | `crates/areka-emo-text/examples/fixtures/emo2-vertical/descript.txt:15-16` |
| D2 | `crates/areka-emo-text/tests/fixtures/emo2-choice/descript-cursor.txt:18-19` |
| D3 | `crates/areka-emo-text/tests/fixtures/emo2-choice/descript-plain.txt:17-18` |
| D4 | `crates/pilot/examples/shiori-host-32/fixtures/emo2/emo2-kakukaku/descript.txt:13-14` |
| D5 | `crates/pilot/examples/shiori-host-32/fixtures/emo2-kakukaku-wplimit/descript.txt:13-14` |

**(1-b) origin 宣言を持たないバルーン定義ファイル（面別上書き層・突合の相手側）＝5 本**: `emo2-vertical/balloons0s.txt`／`emo2/emo2-kakukaku/balloons0s.txt`・`balloonk0s.txt`／`emo2-kakukaku-wplimit/balloons0s.txt`・`balloonk0s.txt`

**(1-c) in-code モデルのうち解決後 validrect の外にあるもの＝3 箇所＋`region.rs` の fixture 複製 2 箇所**

| # | 場所 |
|---|---|
| C1 | `crates/areka-emo-text/tests/draw_readback_test.rs` の `validrect_model` |
| C2 | `crates/areka-emo-text/tests/scale_invariance_test.rs` の 3 箇所 |
| C3 | `crates/areka-emo-text/src/actor_scale_refresh_tests.rs:116-124`（**design のどの行にも無い**） |
| C4 | `crates/areka-emo-text/src/region.rs` の `fixture_model()` |
| C5 | `crates/areka-emo-text/src/region.rs` の in-code KV（`origin.x,0`／`origin.y,0`） |

範囲内だった in-code 宣言（是正不要・記録のみ）: `layout_test_support.rs` 経由の全呼出／`viewbox_test_support.rs`／`viewbox_draw_test_support.rs`／`actor_test_support.rs`／`attach_wiring_test.rs`／`draw_oracle_tests.rs`／`viewbox_scroll_test.rs`／`actor_runtime_frame_tests.rs`（いずれも validrect 未指定＝画像全域、または left=0／top=0 ゆえ origin(0,0) は範囲内）。`region.rs` の origin(100,50)／`None`／(-100,-100)→(300,124) も範囲内。`areka-parsers` の `model_tests.rs`／`parse_tests.rs` は `TextRegion` を解決しない転記層テスト。`choice_tests.rs`／`choice_decorate_tests.rs`／`balloon_pure_core_tests.rs` は origin が `None`。`region_vertical_canon_tests.rs` はクランプ発火に依存しない設計（origin は `None` か validrect 内側の (200,60)）。

### A-2 各宣言の内外判定（PNG 原寸は IHDR から実測）

| # | マージ後 validrect | origin.x 判定 | origin.y 判定 | mode | 書字開始角 | 現行 start | 撤去のみ | 宣言削除 |
|---|---|---|---|---|---|---|---|---|
| D1 | left36／top46／right356／bottom168（画像 400×224） | 0 ∉ [36,356] ＝**外** | 0 ∉ [46,168] ＝**外** | VerticalRl | (356,46) | (356,46) | (0,0) | (356,46) |
| D2 | left5／top5／right W-5／bottom H-5 | **外**（0 < 5・画像寸に依らない） | **外** | HorizontalTb | (5,5) | (5,5) | (0,0) | (5,5) |
| D3 | D2 と同一幾何 | **外** | **外** | HorizontalTb | (5,5) | (5,5) | (0,0) | (5,5) |
| D4 sakura | left36／top46／right356／bottom168（`balloons0.png` 400×224） | **外** | **外** | HorizontalTb | (36,46) | (36,46) | (0,0) | (36,46) |
| D4 kero | left24／top40／right240／bottom133（`balloonk0.png` 288×203） | **外** | **外** | HorizontalTb | (24,40) | (24,40) | (0,0) | (24,40) |
| D5 sakura／kero | **D4 と完全に同一**（複製 fixture・面別上書き層の validrect も画像も原本と同値） | **外** | **外** | HorizontalTb | (36,46)／(24,40) | 同左 | (0,0) | 同左 |
| C1 | left36／top46／right156／bottom94（IMAGE 200×150） | **外** | **外** | 両モードで使用 | — | (36,46)／(156,46) | (0,0) | 同左 |
| C2 | left36／top46／right356／bottom168（IMAGE 400×224） | **外** | **外** | HorizontalTb | (36,46) | (36,46) | (0,0) | 同左 |
| C3 | left24／top16／right360／bottom200（NATIVE 400×224） | **外** | **外** | — | (24,16) | (24,16) | (0,0) | 同左 |
| C4／C5 | left36／top46／right356／bottom168 | **外** | **外** | 複数 | — | — | — | — |

D2／D3 は現状 `TextRegion` を解決する消費者が 1 本も無い（`TextRegion::resolve` の全呼出を列挙して確認）。design が言う「開始点 (5,5) 不変」は解決されれば真だが、それを観測している檻は存在しない。

### A-3 範囲外集合と既知 4 件の突合結果

- 既知 4 件（design C9 の表）: `emo2-vertical`／`emo2-choice/descript-cursor`／`emo2-choice/descript-plain`／`emo2-kakukaku`
- 実測の範囲外集合（データファイル）: 上記 4 件 ＋ **`emo2-kakukaku-wplimit`**（D5）＝**5 件。一致しない。**
- 既知 4 件のいずれかが範囲内だった、という逆向きの食い違いは無い（4 件とも x・y 双方で範囲外）
- in-code は design が一括で述べていたものが 3 箇所に具体化し、うち **C3 は design のどの行にも現れていない**

### A-4 各件の是正方針（DD5 の分類と根拠）

| # | 方針 | 根拠 |
|---|---|---|
| D1 | **宣言削除** | 意図は書字開始角の縮退。`vertical_fixture_test.rs` が `start() == (356,46)` を「`vertical_rl` の書字開始角は validrect 右上」として固定しており宣言値そのものを見ていない。要件 10.9 の明示対象 |
| D2 | **宣言削除＋直前コメントの是正** | コメントが「原点＝validrect-local 書字開始角へクランプ」と撤去される規約そのものを述べている。fixture の関心は `cursor.*` スタイル導出で origin は幾何の付随物 |
| D3 | **宣言削除** | 「`descript-cursor.txt` と同値」と述べる対の fixture。差を `cursor.*` の有無だけに絞る設計ゆえ対で同じ是正を当てる |
| D4 | **宣言削除** | 意図は書字開始角。正典が「通常は指定せず validrect の定義に任せる」と述べ、削除で sakura (36,46)／kero (24,40) が不変。要件 10.9・DD5 の一般化の直接対象 |
| D5 | **宣言削除**（D4 と同時に・同じ理由） | `readme.txt` が「`descript.txt` と全画像は原本と 1 バイトも違わない」複製であると宣言。**残置するとクランプ撤去後にこの fixture だけ文字が (0,0) から始まる**（`windowposition-limit` の実機サインオフ用バルーン） |
| C1 | `Origin::new(None, None)` へ＋doc からクランプ正準の語を除く | doc が意図を「書字開始角へ寄る」と明記。縦書き檻も「書字開始角＝validrect 右上」を見ている |
| C2 | `Origin::new(None, None)` へ＋doc 是正 | 開始 x=36 を前提に折返し位置の絶対値を固定しており意図は書字開始角。未宣言化で全期待値が不変 |
| C3 | `Origin::new(None, None)` へ | 関心は「validrect だけが変わったとき region が変わるか」で origin は付随物 |
| C4／C5 の消費者 | **意図ごとに個別判断**（design C3 の Risks） | 期待値が動く assert は 5 本。「書字開始角を見たい」檻は `origin=None` モデルへ／「2 層マージが非退化領域を作る」檻は in-code KV から origin を落とす／「範囲外成分だけが独立に寄る」檻は**撤去後は成分独立に字義位置が返る**ことを見る檻へ意味を差し替える |

### A-5 `emo2-kakukaku` の開始点の実測値（4.2 の檻に逐語で入れる値）

- **sakura（scope 0・`balloons0s.txt` が面別上書き層）**: 画像 `balloons0.png` = 400×224。`top,46`（非負素通し）／`bottom,-56` → 224-56=168／`left,36`（非負素通し）／`right,-44` → 400-44=356。`writing_mode`・`vertical` とも未宣言 → `HorizontalTb` → 書字開始角 (36,46)。origin(0,0) は両成分とも範囲外 → **start = (36, 46)**（参考: `wrap_threshold` = 400-49 = 351）
- **kero（scope 1・`balloonk0s.txt` が面別上書き層）**: 画像 `balloonk0.png` = 288×203。`top,40`／`bottom,-70` → 203-70=133／`left,24`／`right,-48` → 288-48=240。`HorizontalTb` → 書字開始角 (24,40)。両成分とも範囲外 → **start = (24, 40)**（参考: `balloonk0s.txt` に `wordwrappoint` 行が無く descript の -34 を継承 → `wrap_threshold` = 288-34 = 254）
- **design.md の (36,46)／(24,40) と一致した。** 宣言削除後は未宣言→書字開始角の経路で同値になるため、4.2 の檻はクランプ現存下でも削除後でも同じ期待値で緑になる（**両編集を跨いで不変であることが証跡になる**）

### A-6 宣言削除で赤くなる既存テスト（第 3 類・design の Modified Files に無い）

解決後の start ではなく**宣言された生値**を見ており、DD5 の 2 分類に素直に乗らない。「descript の値が面別上書き層に無くても継承される」ことの証拠として origin を使っている。

- `crates/areka-parsers/src/balloon/validation_tests.rs:60-61／:116-117／:158-159`（3 テスト・6 assert）＋ doc の陳腐化（:6・:8-10・:53「採取元: descript.txt L13–L14（origin）」）。**`emo2-kakukaku` の実物を読む**
- `crates/areka-emo-present/src/balloon_model_tests.rs:118-123`（2 scope 分のループ）。fixture は `balloon_test_support.rs` 経由で `emo2-kakukaku` の実物

**是正方針**: 期待値を `None` へ替えるだけにしない（継承の被覆が消えて要件 10.7 に反する）。**継承の証拠を同条件で継承される別キー**（`wordwrappoint.y`／`font.height`／`font.color`）**へ移す**。

宣言削除で**不変**であることを確認済みの消費者（是正不要）: `vertical_fixture_test.rs`（start (356,46)）／`pipeline_test.rs`（縦書き通し・開始角 (356,46)）／`emo2_fixture_e2e_test.rs`（region.left/top のみ使用）／`choice_fixture_test.rs`（`TextRegion` を解決しない）／`crates/areka/src/emo2_boot/assets_tests.rs`（validrect／windowposition のみ）。

### A-7 方法の限界と、やり直しの手順

- **語の grep では見つからない類が在る。** 「クランプ」「clamp_origin」「書字開始角」を検索しても、当該語を 1 度も書いていない定義ファイル（`emo2-kakukaku/descript.txt`・`emo2-kakukaku-wplimit/descript.txt`）は原理的にヒットしない。2026-08-27 の棚卸しが実ゴースト定義を取りこぼしたのはこの理由。**語 grep は文言是正の網羅にだけ使い、対象の発見には使わない**
- 本棚卸しは**意味論**で行った: (a) origin 宣言を repo 全域から漏れなく列挙し、(b) 各々の 2 層マージ後 validrect を実物の解決規則どおりに計算し、(c) 成分ごと独立に内外を判定した
- **基層だけを見て内外を判定してはならない。** `emo2-kakukaku` 系の基層は validrect 全 0（範囲 [0,0]・両端含む判定で origin 0 は「内」）だが、本番は必ず面別上書き層を重ねるため実範囲は [36,356]×[46,168] で「外」になる。2026-08-27〜28 の設計判断がこの取り違えで `wplimit` を「不変」と誤判定した
- **全緑は十分性の証拠にならない。** `actor_scale_refresh_tests.rs` の範囲外宣言は撤去後も緑のままであり、`emo2-kakukaku` の実ゴースト開始点は現在どのテストも固定していない（＝壊しても全緑）

**やり直しの手順（この順で）**

1. `grep -rn -E "^[[:space:]]*origin\.(x|y)[[:space:]]*," .`（`target`／`.git`／`vendors`／`.kiro` を除外。**ファイルシステム側**を見ること——インデックス側だけだと未追跡ファイルを落とす）
2. 網の妥当性を別マーカーで裏取り: `grep -rl -E "^[[:space:]]*validrect\." .` で「バルーン定義らしきファイル」を全列挙し、1 の結果を包含しているか見る
3. in-code は `grep -rn "Origin::" .` で全構築点を列挙し、`Some(..)` のものだけ残す
4. 各宣言について、対応する面別上書き層（`balloon(s|k)*s.txt`）を後勝ちで重ね、採用画像の PNG IHDR から原寸を読み、`region.rs` の `resolve_or`／`resolve_coord` の規則どおりに validrect 4 辺を計算する
5. `resolved` が `[range.0, range.1]`（**両端含む**）に入るかを x・y 独立に判定する
6. 消費側の追随を `grep -rn "\.origin()" .` と `grep -rn "TextRegion::resolve" .` の 2 本で洗う（前者が生値を見る檻・後者が解決結果を見る檻）

### A-8 探索方法の独立性（網羅の担保）

6 通りを独立に実施し、対象ファイル集合は完全に一致した——⑴ `git grep` によるインデックス側全文 ⑵ `find` による `descript*.txt`／`balloon*.txt` の名前ベース列挙 ⑶ ファイルシステム全域の行頭アンカー付き `grep` ⑷ `Origin::` 全出現の列挙（in-code 補完） ⑸ **別マーカー `^validrect\.` を含むファイルの全列挙**（origin 語を含まない定義ファイルを拾う網） ⑹ 緩い全文検索（`doc/`・`assets/`・`tools/` 込み）。⑴⑶⑹ はデータファイル 5 本で一致。⑸ は 10 本のバルーン定義ファイルを挙げ、うち origin 宣言を持つのはその 5 本。⑵ の名前ベース列挙も同じ 10 本。

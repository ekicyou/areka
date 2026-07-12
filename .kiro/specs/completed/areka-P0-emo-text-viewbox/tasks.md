# 実装計画

## Foundation

- [x] 1. blit≡再描画位相不変仮定のreadback byte比較spike（実装ゲート）
  - 同一の`create_text_format`経路で生成したTextLayoutを使い、(a)位置Aへ描画した後にwhole-pixel blitで位置Bへ移した結果と(b)最初から位置Bへ描画した結果を、横書き・縦書き（vertical_rl）それぞれ数行パターンでオフスクリーンreadbackしてbyte比較する使い捨て検証コードを書く（既存のTextSurface/D2D DeviceContextヘルパをそのまま流用し新規コンポーネントは作らない）
  - AAこぼれの実測からダーティ矩形のガード余白の候補値を記録する
  - 比較が不一致だった場合は本タスクの結果をもって設計の前提崩壊として報告し、以降のタスクへ進まない
  - 観測可能な完了状態: 横書き・縦書きそれぞれのbyte比較結果（一致/不一致とガード幅の実測値）が記録され、GO/NO-GO判断が下せる状態になる
  - _Requirements: 3.1, 3.4, 6.1_

## Core

- [x] 2. (P) 行TextLayoutキャッシュを共有ストアとして抽出する
  - 既存の行TextLayout取得・キャッシュ・生成カウントの処理を、複数の描画実行から同一経路で呼べる形へ抽出する（キー・再利用規則・破棄規律は変更しない）
  - 既存の描画実行部（全域再描画）が抽出後のストアを経由するよう置き換える
  - 観測可能な完了状態: 抽出前後で既存の行キャッシュ関連ユニットテストが無改変のままgreenになる
  - _Requirements: 6.1_
  - _Boundary: LineLayoutStore ＋ 共有 format 経路_

- [x] 3. Core: スクロール位置の純粋な計画（ScrollPlanner）
- [x] 3.1 (P) スクロール位置の内部表現と軸写像・量子化を実装する
  - 真位置（f32連続量・物理px）と確定位置（whole-pixel整数）を分離して保持する表現を作る
  - 可視窓の`block_offset`から真位置への写像を、writing_modeに応じた軸（横書き=縦方向・縦書き=横方向）で行い、符号は可視窓決定側の規約をそのまま素通しする（独自の軸規則を作らない）
  - 真位置から確定位置への量子化を、真位置からの直接丸めで行い、増分丸めによる累積ドリフトが生じない形にする
  - choice-renderが後で消費する座標契約点（canvas座標から描画面座標への写像・現在の量子化状態）を読み取れる形で公開する
  - 観測可能な完了状態: 横書き・縦書き・逆行書きそれぞれで、可視窓のオフセットから期待される軸・符号・量子化位置が算出できる
  - _Requirements: 1.2, 2.3, 3.6, 5.1, 5.2, 5.3, 5.4, 6.4, 8.2, 8.3, 9.3_
  - _Boundary: ScrollPlanner_

- [x] 3.2 ダーティ矩形の導出ロジックを実装する
  - スクロールで露出する帯（blitの逆側に生じる未保持領域）を算出する
  - 内容キャンバスの全行を対象に、前回確定時からの変化行（内容・位置・寸法の差分）を検出する仕組みを作る（typewriter進行中の現在行・複数行同時のcatch-up・新規行追加を一様に扱う）
  - 検出した露出帯と変化行を統合し、物理px整数格子へ拡張してガード余白を加え、描画面の寸法内へクランプする
  - Clear直後・初回・フォーマット再構築時はダーティ領域を全域として扱う
  - 観測可能な完了状態: 可視窓のみ移動する入力では露出帯のみがダーティとして返り、typewriterで1グリフだけ進む入力では現在行のみがダーティとして返る
  - _Requirements: 2.2, 3.2, 3.3, 4.2_
  - _Boundary: ScrollPlanner_
  - _Depends: 3.1_

- [x] 3.3 plan/commit二相を実装する
  - 計画（状態を変えず結果だけを返す）と確定（描画実行が成功した後にだけ状態を反映する）を分離した二相の呼び出し形にする
  - 変化なし・全域リセット・blit＋ダーティ描画の3種類の計画結果を返せるようにする
  - 観測可能な完了状態: 計画のみを繰り返し呼んでも状態が変化せず、確定を挟むと次回計画に反映される
  - _Requirements: 2.3, 4.3_
  - _Boundary: ScrollPlanner_
  - _Depends: 3.1, 3.2_

- [x] 3.4 ScrollPlanner純粋層のユニットテスト一式を揃える
  - 以下をユニットテストとして檻化する: (a)横書き/vertical_rl/vertical_lrの軸写像3方向が正しい (b)非整数スケールの長スクロール列で確定位置と真位置の差が常に0.5px以内に収まる（k=1.0では両者が一致する） (c)可視窓のみ移動／typewriter1グリフ進行／複数行catch-up／Clear／変化なしのそれぞれでダーティ領域が期待通りになる (d)任意の計画結果でblitの写域とダーティ領域の和が描画面全域を被覆する (e)確定しない計画の反復が同一の結果を返す
  - 観測可能な完了状態: 上記5種のユニットテストが全てgreenになる
  - _Requirements: 6.4_
  - _Boundary: ScrollPlanner_
  - _Depends: 3.3_

- [x] 4. Core: 描画面の固定ダブルバッファ化と固定層差し込み点
- [x] 4.1 (P) 固定寸2面バッファと面内whole-pixel blitを実装する
  - 現在の単一描画面を、同一の固定validrect物理寸を持つ2枚のバッファ（面の役割は交換可能）へ拡張する。面の寸法を変える手段は設けない
  - 保持済みピクセルを別の面へwhole-pixel単位でずらして複製する操作（ずらし量ゼロは全面複製）を実装する。複製元と複製先が別テクスチャであることを保証し、重なりによる未定義動作を構造的に排除する
  - 2面の役割交換（コピーを伴わない）を実装する。提示・読み戻しは常に最新確定面を対象にする
  - 本番の描画実行が使う書き込み先アクセサに加え、比較専用オラクル（後続タスクで隔離される既存の全域再描画方式）がそのまま読み出せるテスト限定の読み出し口も公開する
  - 観測可能な完了状態: 既知パターンを書き込んだ面に対しゼロ・正・負それぞれのずらし量で複製→役割交換→読み戻しを行う往復検証で、期待位置に元の内容がbyte一致で現れる
  - _Requirements: 1.1, 1.2, 1.3, 1.5, 4.1, 4.4, 5.4_
  - _Boundary: TextSurface（ダブルバッファ化）_

- [x] 4.2 固定層差し込み点を型シームとして予約する
  - スクロールの面内blitの影響を受けない別の合成層を差し込める型（実挙動は持たない）を追加する
  - 提示処理の中に、この固定層を合成すべき位置（保持面の複製直後・提示直前）を明示する
  - 観測可能な完了状態: 型シームを追加してもスクロール系の既存読み戻し検証結果が変化しない（固定層予約が読み戻し対象に含まれないことが確認できる）
  - _Requirements: 7.1, 7.2, 7.3, 7.4_
  - _Boundary: TextSurface（ダブルバッファ化）_
  - _Depends: 4.1_

- [x] 5. (P) 既存の全域再描画方式を比較専用の独立オラクルとして保全する
  - 現行の全域再描画実行部を、本番経路からは呼ばれないテスト限定の独立実装として隔離する（描画ロジック・origin計算式は一切変更しない）
  - 隔離した実装の描画先を、ダブルバッファ化された描画面がオラクル向けに公開したテスト限定の読み出し口へ向け直す（単一面前提の呼び出しを置き換える）
  - 隔離した実装が比較専用オラクルであり除去は本ユニットの範囲外であることをコード上に明記する
  - 観測可能な完了状態: 隔離・向け直し後も既存のdraw.rs内ユニットテスト・統合テストが無改変のままgreenになる
  - _Requirements: 6.1_
  - _Boundary: DrawExecutor（#[cfg(test)] オラクル）_
  - _Depends: 2, 4.1_

- [x] 6. (P) ViewboxExecutor: plan実行パイプラインを実装する
  - 1フレームの実行を「計画取得→保持ピクセルの面内blit→ダーティ矩形ごとの限定描画→面の役割交換→計画の確定」の順で行う実行部を作る
  - ダーティ矩形ごとの描画は、恒等変換下で物理整数矩形へ描画範囲を限定した上で、その範囲内だけを透明化し、合成スケールを適用してから該当行を描画し、範囲限定を解除する順序で行う。描画対象の座標計算は既存の全域再描画方式と同一の式を使う
  - 確定済み内容用に別途のビットマップキャッシュや描画コマンド列キャッシュを設けない（保持面＋blitのみを保持機構とする）
  - ダーティ限定はDirect2Dの矩形範囲限定機構を直接用い、wintf側のクリップ機構には依存しない
  - 観測可能な完了状態: 可視窓が変化しない入力ではblit・描画が発生せず、可視窓のみ移動する入力では保持ピクセルの複製と露出帯の描画だけが発生する
  - _Requirements: 1.1, 1.4, 2.2, 3.1, 3.2, 3.3, 3.4, 9.4_
  - _Boundary: ViewboxExecutor_
  - _Depends: 2, 3.4, 4.1_

- [x] 7. ViewboxExecutor: 決定論観測統計とエラー縮退規律を実装する
  - 行TextLayout生成回数・ダーティ描画実行回数・blit回数・全域リセット回数を常時計上する統計を持たせる
  - Clear適用時に計画状態・行キャッシュを初期化し、次フレームで全域リセットとして描画されるようにする
  - デバイス呼び出し失敗時は当該フレームを未確定のままスキップし次フレームで再計画する。行指紋と内容キャンバスの想定外の不整合を検知した場合はログを残した上で当該フレームを全域ダーティへ縮退させる
  - 観測可能な完了状態: 意図的に不整合を注入した入力で全域ダーティへ縮退した描画結果が得られ、通常入力では統計値が期待どおり増分する
  - _Requirements: 3.5, 4.3_
  - _Boundary: ViewboxExecutor_
  - _Depends: 6_

## Integration

- [x] 8. actor結線: 描画実行の差し替えとClear/present条件化を行う
  - フレーム提示の描画実行呼び出し先を新しい実行部へ差し替える。可視窓決定・内容キャンバス構築の呼び順は変更しない
  - Clear cue適用時の処理を新しい全域リセット要求へ写像する
  - フレーム提示は描画実行が変化ありと返した場合にのみ面の提示を行う
  - 外部から消費されるsink登録・actor view登録・フレーム提示関数のシグネチャは変更しない
  - 観測可能な完了状態: 既存の実pump統合テスト（登録→装着→typewriter進行→Clear）が新しい描画実行経路を通って従来どおりgreenになる
  - _Requirements: 2.1, 4.3, 9.2, 9.5_
  - _Boundary: actor 結線_
  - _Depends: 7_

- [x] 9. actor結線: 決定論観測の読み口を追加する
  - actor単位で描画統計を読み出せる口を、既存の面読み出し口と同型の形で追加する
  - 観測可能な完了状態: 登録済みactorに対してフレーム提示後の統計値を外部から読み出せる
  - _Requirements: 3.5, 10.3_
  - _Boundary: actor 結線_
  - _Depends: 8_

## Validation

- [x] 10. (P) pixel等価のlive-diff検証を実装する
  - 同一プロセス・同一の面型を用い、同一のcue列・同一の注入時刻列を、比較専用オラクル（全域再描画方式）と新しい実行部の両方で描画し、読み戻し結果を合成スケールk=1.0でbyte比較する検証を作る
  - 検証シナリオは横書き・縦書き（vertical_rl）のそれぞれについて、あふれ前・スクロール発火直後・連続スクロール・Clear直後・Clear後の再追記を含める
  - 観測可能な完了状態: 全シナリオでbyte比較が一致し、意図的に不一致を起こす改変を入れると検証が失敗する（検証自体の有効性を確認できる）
  - _Requirements: 4.5, 6.1, 6.2, 6.3, 6.5, 8.1_
  - _Boundary: ViewboxExecutor, DrawExecutor（#[cfg(test)] オラクル）_
  - _Depends: 5, 7_

- [x] 11. (P) 再描画レス・ダーティ限定描画の決定論カウント検証を実装する
  - 実pump（登録→装着→フレーム提示）を通し、可視窓のみが移動するフレームでの行描画実行回数の増分が露出帯と交差する行数以下であること、内容・可視窓とも不変のフレームでは全ての統計値の増分が0であることを検証する
  - 観測可能な完了状態: 上記2条件を検証するテストがgreenになり、意図的に全域再描画へ戻す改変を入れると失敗する
  - _Requirements: 3.1, 3.2, 3.5_
  - _Boundary: viewbox_scroll_test.rs（新規統合テスト）_
  - _Depends: 9_

- [x] 12. (P) 既存回帰資産の無改変green確認を行う
  - 純粋層（状態機械・writing_mode解決・領域解決・レイアウト・内容キャンバス）とsinkに対する既存テストを一切変更せずに実行し、全てgreenであることを確認する
  - 既存の読み戻し述語検証（単調増加・Clear全透明・面封じ込め・スクロールで先頭行消失・同一入力同一pixel・横書き/縦書き）・スケール不変検証・装着結線検証・パイプライン検証・縦書きfixture検証を無改変のまま実行し、全てgreenであることを確認する
  - 回帰が発生した場合は純粋層・テストファイルを変更せず描画実行側の実装を修正して解消する
  - 観測可能な完了状態: 変更前後で対象テストファイル群に差分がなく、かつ全て実行green
  - _Requirements: 2.4, 2.5, 7.4, 9.1, 9.2, 9.5, 10.6_
  - _Boundary: 既存回帰資産（読み取り専用実行・回帰時のみ描画実行側を修正）_
  - _Depends: 9_

- [x] 13. (P) 観測exampleのviewbox経路差し替えとチェックポイント追加を行う
  - 既存の観測exampleが通るスクロール経路が新しい実行部を通るようにする（cue列・注入時刻駆動のシナリオ自体は変更しない）
  - 既存のtypewriter／改行／あふれ→スクロール／Clear／複数actor独立（横書き・縦書き）の各チェックポイントが単一pass/failのまま維持されることを確認する
  - あふれ→スクロール区間の前後で描画統計を読み出し、「可視窓のみ移動するフレームでの行描画実行回数増分が露出帯交差行数以下」「内容・可視窓とも不変のフレームでの増分0」をチェックポイントとして追加する
  - 観測可能な完了状態: exampleの実行結果が全チェックポイントでPASSを返す
  - _Requirements: 10.1, 10.2, 10.3, 10.4, 10.5_
  - _Boundary: 観測 example（examples/emo-text-layer.rs）_
  - _Depends: 9_

## Implementation Notes

- **Task 1 (spike) = GO**（2026-07-12・`tests/viewbox_blit_spike.rs`）: k=1.0・premultiplied 透明背景で「位置 A に描画→whole-pixel 整数 blit（`CopySubresourceRegion`・別テクスチャ）で位置 B へ」と「最初から位置 B へ描画」の保持域が **byte 完全一致**（横書き＝縦 blit・下端露出／縦書き（vertical_rl）＝横 blit・右端露出の両方で不一致 0・3 回再現）。ClearType/AA 位相不変仮定は成立＝設計前提は実証済み。ViewboxExecutor/ScrollPlanner 本実装へ進んでよい。
- **`DIRTY_GUARD_IMG_PX` 実測 = 0**（既定 ＭＳ ゴシック 12px・公称セル `inline_advance × line_pitch(15)` 外への AA こぼれ 0）。ただし font/size 依存ゆえ design の保守既定 **1 image px** を採用するのが安全（live-diff 檻が破れを検出し、破れた場合はガード定数一点の増加で吸収）。
- spike 由来の再利用元: テクスチャ生成＝`surface.rs::create_transparent_source_tex`／D2D ターゲット化＝`draw.rs::create_target_bitmap`／描画列＝`draw.rs::DrawExecutor::render` Phase2／readback＝`surface.rs::read_back`（RowPitch≥stride パディング）。行 TextLayout は本番と同一 `create_text_format` 経路（byte 等価の構造前提 RN5）。

- **`draw_text_layout_calls` の count 上の綾（task 13 で判明・非欠陥）**: `ViewboxExecutor::render` はダーティ矩形ごとに `draw_lines` 全体を描く（クリップで pixels は正しく限定・live-diff task 10 で byte 等価実証済み）ため、`draw_text_layout_calls 増分 = dirty_len × draws.len()`。多ダーティ矩形フレーム（typewriter 進行＋スクロール＝露出帯∪変化行∪空行で dirty_len>1）では素朴な「増分 < 可視行数」が小 fixture（可視3行）で成立しない（例で draw_delta=3=vlc=3）。**これは全域再描画への退行ではなく count 上の綾**——確定 content は面内 blit で保持され再描画されない（blit=1・全域再描画なら blit=0）・生成増分は流入1行のみ（create≤1）・描画増分はスクロール深さに依らず一定（draw1==draw2＝確定行を蓄積再描画しない）。再描画レスは task 13 C8 で**深さ不変＋blit＋NoChange＝0** の頑健な不変で実証（fixture 脆弱な「draw<可視行数」に依存しない）。design の「可視窓のみ移動フレームで draw ≤ 露出帯交差行数」不変は純粋 window-only-move（dirty_len=1・自然には typewriter で発生しない）で成立。**将来の最適化余地**: ダーティ矩形ごとに交差住人のみ描画（design System Flows「dirty 交差住人」寄り）で冗長クリップ描画を削減し count を ≤露出帯交差行数へ締める（byte 等価は live-diff で担保・M1 では非必達）。

## 追加検証・強化（初期レビューの gap 対応・2026-07-12）

実装完了後の批判的自己点検で見つけた「自動化できたのに未実施」の穴を追加で塞いだ（G1〜G5）。スコープ外は owning spec へ申し送り（G6）。

- **G1（完了・a25d916b）**: vertical_lr スクロール実描画 byte 等価を live-diff へ追加。従来 live-diff は horizontal_tb+vertical_rl のみで vertical_lr は純粋層軸写像 unit 檻だけだった。**3 書字方向すべての実 GPU 描画 byte 等価カバー完成**。
- **G2（完了・73bdb5e0）**: k≠1.0（k=1.25）の実描画許容差檻。R6.4「k≠1.0 で ≤0.5px」が実 GPU 描画で一度も走っていなかった穴を塞ぐ。`LiveDiffRig::new_scaled`＋`checkpoint_block_tol`（ブロック軸インク範囲差 ≤tol 物理 px）で小数アキュムレータ→whole-pixel blit→実描画→readback を k=1.25・横/縦rl/縦lr で実走し ≤2px（≈ceil(0.5×1.25)+AA）を確認。**k≠1.0 述語のみだった検証に実描画経路を追加**（残る手動確認は実機 DPI 目視のみ）。
- **G3（完了・f530bcc9）**: 再描画レスカウント檻を fixture 非依存へ頑健化。脆弱な「draw < 可視行数」（可視8行 fixture 依存）を tight check へ降格し、blit==1（全域再描画なら0＝真の負のコントロール）＋depth-invariance（draw_delta==draw_delta2＝スクロール深さ不変）を主檻へ。
- **G4（完了・2531dcf2）**: AA ガード=1 の非 default フォント/大サイズ byte 等価。ＭＳ Ｐゴシック（プロポーショナル）・20px（大サイズ）でも横/縦rl byte 完全一致＝**DIRTY_GUARD_IMG_PX(1) が font/サイズ非依存で十分**を実描画で確認。
- **G5（完了・61d25134）**: 描画中デバイス失敗の再試行安全を実描画で檻化。`#[cfg(test)]` fault-injection（EndDraw 後 flip/commit 前に Err）で「失敗フレーム＝front 不変・planner 未 commit・再試行で正しく反映」を檻化（純粋 ScrollPlanner 3.3 の COM 側 runtime 版・#[cfg(test)] ゲートゆえ本番無影響）。

## 実機観測由来のバグ修正（D1・2026-07-12）

- **後方（un-reveal/un-scroll）スクロールの byte 非等価バグ（実機で行間文字欠けを観測→根本修正）**: 実機 example で行間に文字欠けを目視観測。診断（example 実 fixture＝font28/DWriteMetrics/実文字を oracle vs viewbox で byte 比較）で**前方進行は全フレーム byte 一致だが後方時刻ジャンプで diverge**（back_t で 47 行相違）と判明。**根本原因**: viewbox は前方スクロール前提で、後方（内容が減る un-reveal）ではスクロールアウトした確定行の再露出を面内 blit で保持できず、かつ指紋一致で「変化行でない」と判定され再描画されない→欠落。**トリガ**: task 13 で追加した example C8 検分の `present_at(earlier t_mid)`＝後方ジャンプが実機表示を汚した（私が入れた退行）。**修正**（`viewbox.rs` `ScrollPlanner::plan`）: 内容が前回確定より減った（`canvas.residents.len() < prev_lines.len()`）とき全域ダーティ Update（blit=0・面全域・全住人）へ縮退——既存の format 変更/不整合縮退と同型・正しさ優先。**これで viewbox が任意アクセスパターン（後方ジャンプ含む）に堅牢化**し byte 等価を維持（実 talk は前方のみゆえ通常不発の防御）。檻: `plan_shrunk_content_degrades_to_full_domain`（純粋層 unit）＋`diag_line_boundary_dropout_vs_oracle`（統合・前方全フレーム＋後方ジャンプで oracle と byte 一致）。**教訓**: live-diff（FixedMetrics＋あいうえお・前方のみ）が見逃した「実 fixture の実文字＋後方アクセス」を実機観測が捕捉——記憶 areka-placement-real-ghost-first の通り実画面はテスト緑に勝る。

## 実機観測由来のバグ修正（D2・2026-07-12）

- **横書きで各行の下端インクが描画されない（実機観測→根本修正）**: 実機 example（横書き）で「文字列の下の方が描画されていない」を目視観測。**出力画像の目視診断**（実 fixture＝font28/実フォント **Yu Gothic UI**/実 validrect を oracle vs viewbox で read_back byte 比較し PNG ダンプ）で、**欠け画素が各行 em ボックス下端の直下（行0[0,28]→y29,30,31／行1[35,63]→y64,65,66／行2[70,98]→y99,100）に集中**すると判明。**根本原因**: レイアウトの行矩形は em ボックス（高さ＝font_height）だが DirectWrite の実描画は行ボックス（ascent＋descent）で行い、**Yu Gothic UI は em 下端より 2〜3px 下（descent 側）へインクがはみ出す**。行ごとのダーティ矩形が em ボックス丈（font_height＋1px guard）しかないため、はみ出しインクがクリップで切り落とされ行の下端が欠ける（oracle は全域再描画でクリップしないため diverge）。**byte 等価檻の盲点**: 既存 live-diff／`diag_line_boundary_dropout_vs_oracle` は既定 **ＭＳ ゴシック**（em に収まる字）を使い、この字は descent はみ出しが無いため見逃した。**修正（実測 `GetOverhangMetrics` 方式）**: 経験則の推定値でなく **各行の実インク境界を DirectWrite で実測**して使う。(1) `wintf` `DWriteTextLayoutExt::get_overhang_metrics`（`IDWriteTextLayout::GetOverhangMetrics` の安全ラッパ）を新設。(2) `LineLayoutStore`（draw.rs）が行 TextLayout 生成時に **1 回だけ**はみ出しを実測し `LineOverhang`（各辺 `max(0, o.·)`・image px）をキャッシュ（確定行は再計測しない）——行ボックスのブロック軸寸が `font_height`（横＝`max_height`／縦＝`max_width`）ゆえその軸の overhang が em ボックスからのはみ出しを**直接**与える（行内軸は巨大 `PROBE_MAX_EXTENT` 箱ゆえ巨大負値→`max(0.0)` で 0 に丸まりブロック軸だけ効く＝`SetMaxWidth` 不要で描画不変）。(3) render（viewbox_draw.rs）が plan 前に全住人レイアウトを確保して `overhangs` を収集し、pure 層 `ScrollPlanner::plan_with_overhangs`／`derive_dirty_with_overhangs`（新設・従来 `plan`/`derive_dirty` は空スライス委譲の薄いラッパ＝pure 層は windows 非依存維持）へ手渡す。(4) `resident_rect`（viewbox.rs）が em ボックスを overhang の実測分だけブロック軸へ外側拡張（horizontal＝top/bottom で Y・vertical＝left/right で X）。**利点**: 経験則の当てずっぽう（フォント/サイズ差で過不足）を排し、イタリック/アクセント/合字/縦書き top-bottom まで正確に被覆。**「実インクはみ出し < 行 pitch のギャップ」がフォント設計上必ず成立**ゆえ隣接行 em/AA 領域へ届かず、確定行の余計な再描画（再描画レス檻の崩壊）を構造的に起こさない（描画数檻 viewbox_scroll_test/C8 不変）。**檻**: `yugothic_real_fixture_matches_oracle_byte_for_byte`（実 fixture＋実 Yu Gothic UI で全フレーム byte 一致・**overhang を 0 にすると t=0.06 で相違 y=29,30 に落ちる＝実測値こそが修正の本体**）／`overhang_extends_changed_line_dirty_beyond_em_box`（pure 層・overhang でダーティが em を超えて広がる機構＋縦書き軸読み替え）／`get_overhang_metrics_is_box_relative_and_finite`（wintf・ボックス基準の落とし穴）。既存 pure層 unit は overhang 無し＝em ボックス丈（従来 `derive_dirty` ラッパ）ゆえ期待値不変・描画数檻不変。両モード example PASS(27)。**教訓**: byte 等価テストが既定フォント（はみ出さない字）で緑でも、**実フォントの実描画を出力画像で目視**しなければ捕捉できない欠陥がある——記憶 areka-placement-real-ghost-first の通り実画面（AI vision による出力画像確認）はテスト緑に勝る。目視診断ツールは `diag_dump_horizontal_pngs`（#[ignore]・自前 PNG エンコード・`AREKA_DIAG_OUT` へ oracle/viewbox/DIFF を出力）として常設。

## スコープ外ギャップの申し送り（G6・owning spec）

本 spec のスコープ外——他 spec/上位層が所有。本 spec では実装しない根拠を記録:

- **実 talk 経路の main 結線**（`GhostBootOptions.text_sink` への注入・実アプリの実 talk 駆動）: design「Out of Boundary」明記のスコープ外。owning＝**emo2-boot**（sink/register/present_frame を消費）／**ghost-setup**（⓪ゴーストエンジンの boot 結線）。viewbox は実 talk では未駆動だが、**本番 present_frame 経路（＝ViewboxExecutor）は既存統合テスト＋live-diff で byte 等価検証済み**ゆえ、結線先の render 経路の正しさは担保済み。
- **device loss 回復**（`GraphicsCore` invalidate/generation 後の面/executor 再生成）: wintf グラフィクス基盤（`XxxGraphics` invalidate/generation 規約）＋ actor/ghost lifecycle（再 attach 経路）の責務。本 spec の ViewboxExecutor は device loss 後に stale（committed/prev_lines/専用 DC/ダブルバッファ）になるが、**回復＝上位の再 attach の領分**（本 spec は device *failure* の 1 フレーム skip＋再試行のみ所有＝G5 で檻化済み。device *loss* 復旧は別）。
- **長時間 talk のメモリ**: 固定寸ダブルバッファ 2 枚＋staging 1 枚・**寸変更 API 不在**（task 4.1「面寸 attach 時確定」）で構造的に固定上限＝実測不要（無限成長を構造排除・R4.4）。
- **`\_b` 固定層の実挙動**（画像読込・固定層描画）: R7.2 仕様通り型シーム（`FixedOverlaySeam`）＋合成点 doc のみ・実挙動なし。owning＝後続の `\_b` 対応増分。
- **滑らか補間・慣性の実挙動**: R8 仕様通り f32/committed 分離のシームのみ・M1 はステップスクロール。owning＝**M2** スクロール演出。

## 意図的な繰り延べ事項

- Requirement 6.4・10.6の非96 DPI（k≠1.0）における**実機**手動確認（実 DPI モニタでの文字の滲みなし・スクロール整合の目視）は、コーディング作業でないためタスク化しない。自動検証は 3.4（純粋層 drift 恒真）・12（スケール不変資産）に加え **G2（k=1.25 の実 GPU 描画 ≤0.5px 許容差檻）** で述語＋実描画レベルまでカバー済み。残るのは実機 DPI 目視のみで DoD 申し送り事項（記憶 areka-placement-real-ghost-first）。

# Implementation Plan

- [ ] 1. Foundation: `\_l` 語彙と Choice/Cursor cue の実消費（state.rs）
- [x] 1.1 `\_l` 座標語彙とパーサを実装する
  - `CursorCoord`/`CursorUnit` enum を追加（絶対 px/em/lh・相対 `@`・`%`・省略・パース不能の全形を表現）
  - `parse_cursor_coord` を純関数として実装（不透明文字列の全入力に対し値を返す・パニックしない・`Result` を返さない）
  - Observable: 裸数値・`Nem`・`Nlh`・`N%`・`@N`・空文字列・非数値の各入力に対する呼び出しが、対応する `CursorCoord` variant を返す
  - _Requirements: 2.1, 2.4, 6.5_
  - _Boundary: StateIncrement_

- [x] 1.2 Choice/Cursor の良性スキップシームを実消費へ置換する
  - _Depends: 1.1_
  - `ChoiceSpan{ordinal,id,label,references,glyph_range}` と `ActorTextState.choices` を追加（items と同一ライフサイクル＝`Clear`/`ClearAll` で同時初期化）
  - `TextItem::CursorMove{x: CursorCoord, y: CursorCoord}` を追加し、Cursor cue 消費時に items へ追記
  - `apply_cue` の Choice アームを置換: `text` のグリフを items へ追記＋`ChoiceSpan` 記録（reveal は Text cue と同一の時刻式）。空 `text` は `warn!` ログ＋空範囲スパン記録（グリフ追記なし）
  - `choice_warned`/`cursor_warned` の once-guard warn-and-skip ロジックを撤去する
  - Observable: Choice cue を注入しても once-guard の警告は発火せず、`text` が非空なら非空の `glyph_range` を持つ `ChoiceSpan` が記録される
  - _Requirements: 1.1, 1.2, 1.5, 2.1, 5.1, 5.3, 9.5_
  - _Boundary: StateIncrement_

- [x] 1.3 語彙と消費挙動の単体テストを書く
  - `CursorCoord` の全形（負値絶対・未知単位を含む）を網羅する
  - Choice アームの配送順スパン記録・空 text 縮退・`Clear`/`ClearAll` での items 同時初期化を網羅する
  - Cursor アームが `CursorMove` を生成し、グリフ/reveal 状態を変更しないことを検証する
  - Observable: 縮退表の各分岐が actor ごとに厳密に1回だけログされ、`ChoiceSpan.ordinal` が単調増加することを示すテストが緑になる
  - _Requirements: 1.5, 2.1, 2.4, 5.1, 5.3, 6.5, 7.5_
  - _Boundary: StateIncrement_

- [ ] 2. (P) Foundation: バルーン cursor.\* スタイルモデル（areka-parsers・独立クレート）
- [x] 2.1 (P) balloon descript モデルへ cursor.\* の additive サブ構造体を追加する
  - `Cursor{style, brush_color, pen_color, font_color, blendmethod}` を追加し、既存 KV 後勝ちマージへ相乗りする
  - 未モデル化サブキー（shadowcolor/shadowstyle 等）は寛容パース passthrough のまま保持する
  - Observable: `cursor.style`/`cursor.brush.color`/`cursor.font.color`/`cursor.blendmethod` を含む descript.txt をパースすると対応フィールドが値を持ち、cursor.\* 未記載の descript.txt では全フィールドが `None` になる
  - _Requirements: 4.2, 6.2_
  - _Boundary: BalloonCursorModel_

- [x] 2.2 (P) balloon cursor.\* モデルの単体テストを書く
  - _Depends: 2.1_
  - 各サブキーの有無・既存の「cursor キーを font へ巻き込まない」不変条件が緑のまま・shadow 系が passthrough のままであることを検証する
  - Observable: 既存 balloon パーサテストが緑のまま、かつ cursor.\* キーが font モデルフィールドから分離されていることを示す新規テストが通過する
  - _Requirements: 4.2, 6.2, 9.5_
  - _Boundary: BalloonCursorModel_

- [x] 3. (P) Foundation: 選択肢住人型と既存 match 箇所への通し配線（canvas.rs/viewbox.rs/viewbox_draw.rs/draw.rs）
  - `ResidentContent::Choice(ChoiceLineContent)` と `ChoiceLineContent{run, segments, hovered, highlight}`／`ChoiceRowSegment`／`HighlightPaint` を canvas.rs の純データ型として追加する
  - viewbox.rs（`line_fingerprint`/`resident_rect`）・viewbox_draw.rs（`render`）・draw.rs（oracle）へ、Choice 行を等価な GlyphRun とまったく同様に扱う Choice アームを追加する（ハイライトはまだ描かない・挙動変化なし）
  - Observable: 新しい `#[non_exhaustive]` variant を含めて workspace 全体がビルドに成功し、hover 無しの Choice 住人が等価な GlyphRun 住人とピクセル同一に描画される
  - _Requirements: 1.1, 1.4, 9.5_
  - _Boundary: CanvasChoice, ViewboxFingerprint, HighlightDraw, DrawOracle_

- [ ] 4. (P) Core: `\_l` カーソル換算とレイアウト注入（layout.rs）
- [x] 4.1 em/lh/px→image px 換算を実装する
  - _Depends: 1.1_
  - `cursor_to_image_px(coord, origin, font_height, line_pitch)` を実装（絶対 Px/Em/Lh の非負値は `Some(image px)`、それ以外は `None`）
  - Observable: 既知の `font_height`/`line_pitch` で各単位を呼び出すと正典式どおりの値を返し、非対応形は `None` を返す
  - _Requirements: 2.1, 2.2, 2.4_
  - _Boundary: LayoutCursor_

- [x] 4.2 pending-cursor 遅延実体化を実装する
  - _Depends: 1.2_
  - `CursorMove` 到着時は換算のうえ保留するのみ（現在行は閉じない）。次の可視グリフ配置直前にフラッシュ: 現在行が非空なら確定→保留改行 Σratio を適用→指定軸を上書き。後続可視グリフの無い末尾 `CursorMove` は蒸発。両軸 `None` は完全 no-op（行区切りもしない）
  - `cursor_to_image_px` が `None` を返す4つの縮退分岐（負値絶対・`%`・`@`・パース不能）について、actor ごとの warn-once ログを追加する
  - Observable: フラッシュ順序（行確定→保留改行適用→軸上書き）が仕様どおりに動作し、各縮退分岐が actor ごとに厳密1回だけログされる
  - _Requirements: 2.1, 2.3, 2.4, 2.5, 6.5_
  - _Boundary: LayoutCursor_

- [x] 4.3 換算とフラッシュ順序の単体テストを書く
  - 換算表の全縮退分岐・newline-defer との複合順序・末尾蒸発・両軸 `None` の no-op を網羅する
  - Observable: 保留改行比率と `CursorMove` が同一フラッシュに混在するケースを含め、フラッシュ順序（行確定→保留比率→軸上書き）を示すテストが通過する
  - _Requirements: 2.2, 2.4, 2.5, 7.5_
  - _Boundary: LayoutCursor_

- [ ] 5. (P) Core: ChoicePure モジュール（choice.rs）
- [x] 5.1 選択肢スパンからの行注釈を実装する
  - _Depends: 1.2_
  - `lib.rs` へ `pub mod choice;` を追加し、`pure_layer_modules_have_no_windows_imports` 構造檻のモジュール一覧へ登録する
  - `annotate_lines(lines, spans)` を実装（グリフ序数スパンを配置済み行へ写像・折返し跨ぎは行ごと分割・部分リビール時は可視グリフ数で範囲を打ち切り）
  - Observable: `choice` モジュールがコンパイルされ windows 非依存で構造檻に登録済みであることに加え、2 行に折り返された 3 スパン入力が期待どおりの行別分割の `LineChoiceSegment` を生成する
  - _Requirements: 1.2, 2.3, 3.3, 3.4_
  - _Boundary: ChoicePure_

- [x] 5.2 ヒット行導出と窓物理写像を実装する
  - _Depends: 5.1_
  - `derive_hit_rows(lines, segments, mode)`（canvas-local・文字幅のヒット矩形、行全幅ではない）と `to_window_physical(row, region, mode, committed, contract)`（`(origin + block) × k + committed` 式・writing_mode ごとの軸割当）を実装する
  - Observable: 固定入力に対し、描画とヒット行の双方が同一導出パスを使うことで矩形が完全一致する
  - _Requirements: 2.2, 3.1, 3.3, 3.4_
  - _Boundary: ChoicePure_

- [ ] 5.3 ハイライトスタイル解決を実装する
  - _Depends: 2.1_
  - `ResolvedChoiceStyle`（`SquareFill`/`Invert`/`NoMarker`・`#[non_exhaustive]`）と `resolve(cursor_model, default_font_color)` を実装（cursor.\* 指定→`SquareFill`、未指定→`Invert`（塗り=既定font色・文字=255−各成分）、`style=none`→`NoMarker`、underline系→warn-once `SquareFill` 縮退、ROP `blendmethod`→warn-once `none` 扱い）
  - `paint()` で描画実行用の `(fill, text)` 正規形を返す
  - Observable: fixture 指定・未指定・`none`・underline・ROP blendmethod の各組み合わせを解決すると、文書化された variant と `paint()` の色が返る
  - _Requirements: 4.2, 4.3, 6.1, 6.5_
  - _Boundary: ChoicePure_

- [ ] 5.4 canvas 装飾を実装する
  - _Depends: 5.1, 5.3, 3_
  - `decorate_canvas(canvas, segments, hover, style, default_font_color)` を実装（セグメントを含む GlyphRun 住人を Choice 住人へ置換し hover 印＋正規化済みハイライト塗りを焼き込む・セグメント空なら canvas を無変更で返す）
  - Observable: `hover=Some(ordinal)` で装飾すると該当行の `ChoiceLineContent.highlight` にのみ解決済み塗りが設定され他行は `None`、セグメント空リストでは入力と同一の canvas が返る
  - _Requirements: 1.1, 4.2, 4.3, 4.5_
  - _Boundary: ChoicePure_

- [ ] 5.5 ChoicePure モジュールの単体テストを書く
  - `annotate_lines`/`derive_hit_rows` の単一行1選択肢・同一行複数選択肢・折返し跨ぎ・部分リビール・空範囲除外・縦書き2方向を網羅する
  - `to_window_physical` を k≠1.0・committed≠0・writing_mode 3方向でパラメタライズして網羅する
  - `decorate_canvas`/`ResolvedChoiceStyle` の hover 無し/一致/stale ordinal・全スタイル分岐・`paint()` の反転式を網羅する
  - Observable: choice.rs に windows クレート依存の無い GPU 不要な単体テスト群が上記全分岐を通過する
  - _Requirements: 3.4, 4.2, 4.3, 4.5, 6.5, 7.5_
  - _Boundary: ChoicePure_

- [ ] 6. Core: hover 対応ダーティ導出（viewbox.rs）
- [ ] 6.1 行指紋へ hover 印フィールドを追加する
  - _Depends: 3, 5.4_
  - `CommittedLine` へ `choice_marker` フィールドを追加（非 Choice 行は0・Choice 行は `hovered.map_or(0, |o| o+1)`）し、既存 `line_fingerprint`/`derive_dirty` アルゴリズムは無改変のまま供給する
  - Observable: 2つの選択肢行間で hover を切り替えると、変化した2行の指紋のみが差分となり他行の指紋は不変
  - _Requirements: 4.4_
  - _Boundary: ViewboxFingerprint_

- [ ] 6.2 hover 指紋統合の単体テストを書く
  - hover の設定/切替/解除がそれぞれ影響行のみのダーティ導出に限定され、全域ダーティにならないことを検証する
  - Observable: `derive_dirty` の出力行数が `choice_marker` の変化した行数と厳密に一致することを示すテストが通過する
  - _Requirements: 4.4, 7.5_
  - _Boundary: ViewboxFingerprint_

- [ ] 7. Core: ハイライト描画（viewbox_draw.rs）
  - _Depends: 3, 5.3, 5.4, 6.1_
  - ダーティクリップ済み描画パス内で Choice 行のハイライト塗り＋文字色切替を実装する（hover セグメント矩形を塗り色で塗る→行全範囲へ `DrawingEffect` を `None` へリセット→hover セグメント範囲へ文字色効果を適用→`DrawTextLayout`。`highlight=None` の行は素描画）
  - `scroll_state()` の読み口アクセサを追加する
  - Observable: hover 中の Choice 行を描画すると、セグメント範囲内は塗り＋文字色切替済みの画素、範囲外は素の画素になる
  - _Requirements: 4.2, 4.3, 4.5, 4.6, 9.4_
  - _Boundary: HighlightDraw_

- [ ] 8. Integration: ランタイム契約 API と提示パイプライン（actor.rs）
- [ ] 8.1 選択肢契約 API を実装する
  - _Depends: 5.3_
  - `TextLayerRuntime::inject_choice_hover(actor, Option<ordinal>)`／`choice_hit_rows(actor) -> &[ChoiceHitRow]`／`choice_active(actor) -> bool` を実装し、`ResolvedBalloonText.choice_style` を balloon cursor モデルから一度解決する
  - 実ポインタ配線・クリック解決・`ChoiceSelection` の定義/発行は明示的に対象外とする（`areka-P0-choice-interact` の領分）
  - Observable: 現存しない ordinal で `inject_choice_hover` を呼んでもパニックせず、`choice_active` は選択肢スパンの実状態を反映し続ける
  - _Requirements: 1.3, 3.2, 3.5, 4.1, 6.4_
  - _Boundary: RuntimeContract_

- [ ] 8.2 提示パイプラインとヒット行スナップショットを配線する
  - _Depends: 8.1, 5.2, 7_
  - `present_actor` にて layout 後に `annotate_lines`→`decorate_canvas`→`executor.render` を実行し、render 成功時に同一 layout から `derive_hit_rows`＋`to_window_physical` でスナップショットを更新する（`NoChange` フレームは更新をスキップ）
  - 新規のスクロール可視判定ロジックは追加しない（既存 `visible_window` をそのまま再利用する）
  - Observable: 提示成功直後の `choice_hit_rows` 照会が直前フレームと整合する矩形を返し、`NoChange` フレームでは直前のスナップショットが不変のまま保たれる
  - _Requirements: 3.1, 3.2, 3.3, 5.2, 6.3_
  - _Boundary: RuntimeContract_

- [ ] 8.3 Clear/ClearAll の原子的無効化を配線する
  - _Depends: 8.2_
  - 既存の Clear/ClearAll `apply_cue` アーム（既に items を消去済み）を拡張し、当該 actor（ClearAll は全 actor）の hover を `None` へリセットする（スパン無効化は 1.2 の items 同時初期化に相乗り済み）
  - Observable: Clear 注入後の提示で `choice_hit_rows` が空になり `choice_active=false` となり、後続の新選択肢集合へ stale な hover が誤適用されない
  - _Requirements: 5.1, 5.2, 5.3, 5.4_
  - _Boundary: RuntimeContract_

- [ ] 9. Integration: 決定論的観測テスト群
- [ ] 9.1 描画＋字下げの readback テスト
  - _Depends: 8.3_
  - Choice cue×3＋NewLine＋Cursor cue を注入して提示し、readback で3行の選択肢が期待どおりの字下げ位置に描画されていることを検証する（注入 cue／Tick のみ・synthetic pointer・sleep 不使用）
  - Observable: readback pixel アサーションが繰り返し実行でも決定論的に通過する
  - _Requirements: 7.1, 7.4_
  - _Boundary: RuntimeContract_

- [ ] 9.2 hover pixel 檻＋ダーティ限定テスト
  - _Depends: 8.3_
  - hover on/off 対を注入し、塗り色＋文字色切替の画素の出現/消滅と、`DrawStats` で影響行のみが再描画されている（全域再描画でない）ことを検証する
  - Observable: on/off 画素対のアサーションと `DrawStats` ダーティ行数アサーションの両方が決定論的に通過する
  - _Requirements: 4.4, 7.2, 7.4_
  - _Boundary: RuntimeContract_

- [ ] 9.3 矩形反転縮退の pixel 檻テスト
  - _Depends: 8.3_
  - cursor.\* 未指定の test-local バルーンを用意し、hover 注入で既定文字色塗り＋反転文字色の画素を検証する
  - Observable: 反転縮退の画素アサーションが決定論的に通過する
  - _Requirements: 4.3, 6.1, 7.2_
  - _Boundary: RuntimeContract_

- [ ] 9.4 ライフサイクル無効化テスト
  - _Depends: 8.3_
  - Clear 注入、および新 talk（ClearAll＋新 Choice 集合）注入のそれぞれについて、選択肢画素の消滅・`choice_hit_rows` 空・`choice_active=false` が同一フレームで同時観測されることを検証する
  - Observable: Clear ケースと新 talk ケースの双方で同一フレーム原子性のアサーションが通過する
  - _Requirements: 5.1, 5.2, 5.3, 7.3, 7.4_
  - _Boundary: RuntimeContract_

- [ ] 9.5 既存回帰確認
  - _Depends: 8.3_
  - 既存の byte 等価 golden・typewriter・scroll・viewbox テスト群が、選択肢を含まないフィクスチャのまま緑を維持することを確認する
  - Observable: 既存 emo-text テストが無改変のまま全て通過し、additive 増分であることが構造的に確認できる
  - _Requirements: 9.5_
  - _Boundary: DrawOracle_

- [ ] 10. (P) Integration: 実機用 hover 注入デバッグ導線（emo2_boot/hover_inject.rs）
  - _Depends: 8.1_
  - env ゲート駆動の周期巡回導線を実装する（`AREKA_CHOICE_HOVER_INJECT=cycle[:ms]`・frame clock 時刻駆動・実 sleep 不使用で `choice_active` な actor の hit row ordinal を周期巡回し `inject_choice_hover` を呼ぶ・各ステップを info ログ）
  - `emo2_boot/mod.rs` へモジュール登録し `frame.rs` の text phase から駆動する。未設定/不正値は完全無効
  - Observable: env 変数固定周期で決定論的な ordinal 系列が再現され、env 変数未設定では `inject_choice_hover` が一度も呼ばれない
  - _Requirements: 8.2, 8.4, 8.6_
  - _Boundary: HoverInjectConduit_

- [ ] 11. Validation: 実フォント目視確認と fixture
- [ ] 11.1 test-local fixture 準備と実フォント目視確認
  - _Depends: 9.3_
  - 9.3 で用意済みの cursor.\* 指定 descript を再利用し、cursor.\* 未指定 descript（新規）＋短い2〜4項目メニュー台本を追加する。pixel 檻テストに加えて実フォント出力の目視確認を行う
  - Observable: test-local fixture がテストツリー配下に存在し、実フォント出力の目視確認記録が pixel 檻テストに伴っている
  - _Requirements: 7.6_
  - _Boundary: RuntimeContract_

- [ ] 11.2 headless emo2 fixture 統合テスト
  - _Depends: 11.1_
  - 実 emo2 fixture のメニュー cue 列（cue 配送→選択肢描画）を headless readback でエンドツーエンドに検証する（実窓は起動しない）
  - Observable: 実 fixture の descript／台本内容を用いた headless 統合テストが通過する
  - _Requirements: 6.1, 6.2, 6.3_
  - _Boundary: RuntimeContract_

- [ ] 12. Validation: 実機サインオフと最終回帰
- [ ] 12.1 実機サインオフ手順を実施する
  - _Depends: 10, 11.2_
  - pasta.dll を絶対パスで起動し、本番ゴースト表示を先行させたうえでダブルクリックしてメニューを表示、選択肢行が字下げどおり可視であることを目視確認する。`AREKA_CHOICE_HOVER_INJECT=cycle` と有界な `AREKA_APP_SMOKE_EXIT_MS` を設定し、ハイライトが巡回する様子を目視確認したうえで、`RUST_LOG` から注入 ordinal のログを grep で確認する（実ポインタ操作は判定に混ぜない）
  - Observable: 選択肢行の可視・ハイライト巡回の人間目視・注入 ordinal の `RUST_LOG` grep 一致の3点がそろったサインオフ記録が残る
  - _Requirements: 8.1, 8.2, 8.3, 8.4, 8.5_
  - _Boundary: HoverInjectConduit_

- [ ] 12.2 最終ワークスペース回帰を実施する
  - _Depends: 12.1_
  - workspace テストゲートが要求する i686 host-32 成果物を先にビルドし、`cargo test --workspace` を実行して exit 0 を確認する。新規 crates.io 依存が追加されていないこと、emo-present crate 本体が無改変であることも確認する
  - Observable: `cargo test --workspace` が exit 0 で完了し、依存差分に新規 crates.io エントリが無いことが確認できる
  - _Requirements: 9.1, 9.2, 9.3, 9.6_
  - _Boundary: RuntimeContract_

## Implementation Notes
- state.rs のテストモジュールに `WarnCounter`（tracing::Subscriber 実装, `tracing::subscriber::with_default` で使用）ログ檻あり — warn 回数アサーションに再利用可（4.2/viewbox の warn-once テスト等）。
- balloon cursor.* モデル: `BalloonModel::new` は7引数のまま不変・`with_cursor(self, BalloonCursor)` ビルダーでマージに相乗り。`CursorColor` は `FontColor` のミラー（+Default）。fields private + accessor（`cursor() -> &BalloonCursor`）。5.3 の `ResolvedChoiceStyle::resolve` は accessor 経由で読む。
- ResidentContent variant 集合 = {GlyphRun, Choice, Image, Surface}。task 3 で wildcard `seam =>` アームを明示 `Image(_) | Surface(_)` へ変換済（Choice の暗黙吸収防止）。line_fingerprint の Choice アームは task 6.1 で choice_marker を足す前提の GlyphRun ミラー。
- **Task 8.2 申し送り**: layout.rs は加法的に `layout_with_cursor_warn(…, actor, &mut CursorWarnGuard)` を新設。`layout` ラッパ経由は縮退 warn を抑止（挙動は同一・字下げは効く）。production の縮退 warn-once（6.5）を有効化するには present_actor が persistent `CursorWarnGuard`（`unresolved_warned` と同型）を保持して `layout_with_cursor_warn` を呼ぶ必要あり＝Task 8.2 の配線。
- choice.rs: `PositionedLine{rect: LineRect, glyphs: Vec<PositionedGlyph>}`・`PositionedGlyph{ch, inline_pos, advance}`（inline_pos=行内軸 image px 絶対）。annotate は配置済みグリフを0起点連番で数え items 全序数と一致させ純整数交差（lo/hi）で部分リビール自然打切り。lib.rs `PURE_SOURCES` へ choice.rs 登録済。
- **設計不整合の解決（Task 8 申し送り）**: design 型枠は `derive_hit_rows(lines, segments, mode)` だが §座標写像式（正本）＋型枠 doc は canvas-local 入力を要求。layout 出力は絶対 image px ゆえ validrect 原点差引きが必須で、実装は `derive_hit_rows(lines, segments, mode, region: &TextRegion)` へ region を additive 追加（from_layout の `rect.left-region.left()` と同一パターン）。**Task 8 の present_actor は `derive_hit_rows(.., region)` を呼ぶこと**（region は既に present_actor が保持）。`to_window_physical(row, region, mode, committed, contract)` は §座標写像式どおり。

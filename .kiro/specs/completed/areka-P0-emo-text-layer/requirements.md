# Requirements Document

## Project Description (Input)

⑥ emo トラックの M-boot 残ユニット（emo 直列3チェーン emo-atlas → emo-compose → emo-present ✅ 完了後の第4ユニット）。sakura ✅ が発火する Balloon 向け `TalkCue`（Text/NewLine/Clear）を受け、emo-present ✅ が予約したバルーン上のスロット（`emo-text-layer-slot`）に**文字を実際に描く層**を実装する。縦書き/横書き両対応・typewriter 逐次表示・改行・領域あふれ時のスクロールを、注入時刻駆動で決定論テスト可能に実現する。描画先は行列変換領域の内部表現（M1 実挙動は恒等/平行移動のみ・回転と文字装飾は M2 予約の型シーム）。縦書きは areka 拡張キー `writing_mode`（snake_case・CSS `writing-mode` 語彙・descript＋画像別2層マージ）で opt-in し、折返し軸は `wordwrappoint.y`・スクロール軸が回転する。描画面は「バルーン内容キャンバス」（テキストが最初の住人・`\_b` 画像は後続住人の型シーム）として設計する。方針正本は roadmap「emo の責務範囲」節・記憶 areka-emo-ui-layer-text-roadmap。

## Introduction

areka の ⑥ emo トラックのうち、バルーン枠までを表示する emo-present ✅ の次段として、**バルーンにセリフ文字を流す層**を実装するユニットである。上流の sakura ✅ は Balloon 向け cue（Text/NewLine/Clear）を発火し、emo-present ✅ はバルーン枠を表示して text-layer 用の空スロット（上位 z の Visual）を予約済みだが、**文字を描く層が存在しない**ため M-boot の「emo2 が喋る」の可視部分が未達である。ghost-setup の `text_sink` も現状はログのみの終端である。

本ユニットは、sakura の `TextSink` 契約を実装する受信アクターを設け、受け取った cue を UI スレッドへ配送して、予約スロット上に文字を typewriter 進行で描画する。横書き（既定）と縦書き（areka 拡張キー `writing_mode` による opt-in）の両方に対応し、改行・領域あふれ時のスクロールを行う。文字レイアウトは balloon descript ✅ 由来のフォント・テキスト領域定義を消費して DirectWrite で解決する。描画先は単純な矩形ではなく行列変換付き領域として保持し、M1 の実挙動は恒等/平行移動に限るが、回転と文字装飾（アウトライン/多色/シャドウ）は M2 予約の型シームとして構造に持つ。

sink の main 経路への結線（`GhostBootOptions.text_sink` への注入）は emo2-boot、viewbox 合成によるスクロール実現は emo-text-viewbox、選択肢表示は choice-render（M-dialogue）、バルーン枠の描画は emo-present（済み）の責務であり、いずれも本ユニットの責務外である。本ユニットは sakura/balloon-parse/emo-present の既存契約を再定義せず消費し、balloon model への `writing_mode` 転記フィールド増分と emo-present の text_slot 到達手段の公開増分のみを additive に加える。

本ユニットは emo トラック第4の独立 crate **`areka-emo-text`**（spec/feature 名は `areka-P0-emo-text-layer`・crate 名は atlas/compose/present の単一トークン命名に倣い `areka-emo-text`・両者のマッピングは本書で確定）として実装する。**描画（DirectWrite レイアウト・typewriter 進行・縦書き）は emo が所有**し、wintf は窓/surface 手渡し（ComposedSurface/swapchain）と縦書きレシピの donor（lift 元）に留める（wintf のテキスト widget を実行時依存にしない）。この方針は emo の自前合成哲学（surface 合成は wintf 非依存の emo 自前コンポーネント）と、シェルとバルーンを同一描画エンジンへ将来融合する M1 基盤原則（記憶 areka-unified-shell-balloon-graphics・areka-emo-own-compositor-atlas）に基づく（2026-07-09 要件ディスカッション #1 裁定）。

## Boundary Context

- **In scope（本ユニットが観測可能に実現する振る舞い）**:
  - sakura の `TextSink` を実装し、Balloon 向け cue（Text/NewLine/Clear）を受信して、描画を担う UI スレッドへ配送する（受信端はワーカー、描画は UI スレッド固定）。cue の `actor`（`ActorKey`・\0=sakura／\1=kero…）を鍵に、状態と装着をアクター別へ振り分ける（構造は最初から多アクター・M1 実挙動は fixture script 次第）。
  - 受け取った cue 列を「表示中テキストの行/グリフ状態」へ変換する純粋な状態遷移（追記・改行・全消去・スクロール発火判定）。
  - 文字を typewriter 進行（1文字ずつ）で表示する。進行は注入時刻駆動（実時間 sleep 不使用）で決定論的に検証できる。
  - balloon descript ✅ 由来のフォント（欠落は SSP 既定＝ＭＳ ゴシック）とテキスト領域（origin/wordwrappoint/validrect）を消費して文字レイアウトを解決する。
  - 縦書き/横書きの切替を areka 拡張キー `writing_mode`（snake_case・CSS `writing-mode` 語彙の `horizontal_tb`/`vertical_rl`/`vertical_lr`）で宣言し、descript.txt（既定）＜ 画像別 `balloons*s.txt`/`balloonk*s.txt`（後勝ち）の2層マージで解決する。マーカー無しは `horizontal_tb` 既定。
  - 縦書き時は折返し軸を `wordwrappoint.y` とし、スクロール軸を横方向へ回転させる。
  - 領域（validrect）あふれ時にスクロールする（横書き＝縦スクロール・縦書き＝横スクロール）。実現は全域キャンバス再描画とし、「可視窓の決定（純粋）」と「描画実行」を分離する移行シームを残す。
  - 描画先を行列変換付き領域として保持する（M1 実挙動は恒等/平行移動のみ）。
  - 描画面を **emo 共有描画基盤**（統一 resident/行列モデル）として設計する。住人（キャンバスに置かれる変換行列付き矩形コンテンツ）としてグリフ（文字）・画像（`\_b`）・将来の SERIKO サーフェスを同格に扱える抽象を持ち、M1 の実装住人はテキストのみとする。抽象は emo-compose の surface 合成（行列原則）と収束可能な統一形として設計するに留め、M1 では emo-compose を改変しない（共有 canvas の抽出・シェル/バルーン compositor 融合・背景 SERIKO の住人化は後続 roadmap ユニットへ予約）。
  - emo-present の予約スロット（`emo-text-layer-slot` Visual）へ描画内容を装着する公開経路を emo-present へ additive に増設する。装着時に surface 本体の再合成を強要しない。
  - 上記を単一 pass/fail で観測できる専用 example（emo2 fixture のバルーン枠上に fixture スクリプト由来の cue 列を注入時刻駆動で流す）。

- **Out of scope（本ユニットが所有しない）**:
  - 選択肢表示（`\q`・choice-render／M-dialogue）。本ユニットは行レイアウト/クリック範囲の再利用シームを用意するのみ。
  - `\f` 系文字装飾の実挙動・回転テキストの実挙動・ポップアート装飾（M2・型シームのみ）。
  - `text_orientation`（欧文の向き）・`text_combine_upright`（縦中横）の実挙動（M2・予約名の記録のみ）。
  - viewbox 合成（クリップ視窓＋内容オフセット）によるスクロール実現（`areka-P0-emo-text-viewbox`・本ユニットは可視窓/描画分離シームまで）。
  - sink の main 結線（`GhostBootOptions.text_sink` への注入・実 talk 経路の結線は emo2-boot）。
  - sakura の cue 時刻（pacing）の改変（必要と判明した場合の増分申し送りまで）。
  - トーク上書き/中断の可否判定とガード（さくらスクリプト `\t`・`\![enter,nouserbreakmode]` 等）の尊重（上流 kanade の中断ファンネルの責務・emo は届いた cue 列を後出し優先で即時適用するのみ）。
  - バルーン枠の描画・配置・キャッシュ（emo-present 済み）／バルーン窓の生成・配置（window-placement）／surface 合成（emo-compose）。

- **Adjacent expectations（隣接ユニットへの期待・依存）**:
  - sakura ✅ の `TextSink`/`TalkCue`（Text/NewLine/Clear）契約を再定義せず消費する。cue の `at`（talk 起点相対秒）は chunk 開始時刻であり、typewriter の per-glyph 進行は本層が所有する前提に従う。
  - balloon-parse ✅ の領域・フォントモデル（Origin/WordWrapPoint/ValidRect/Font/FontColor）と descript＋画像別の2層マージ機構を消費する。`writing_mode` は balloon model への転記フィールドを additive に増やすのみで、解釈は本層が行う（parser は転記に徹する）。
  - emo-present ✅ の予約スロット（`emo-text-layer-slot` Visual）へ装着する。装着 API の公開増分は本ユニットが emo-present へ additive に加える。
  - actor-foundation ✅ の UI スレッド配送（`spawn_ui`/`UiSender`）を消費する。描画は UI スレッド固定（WUC/D2D・MTA＋`DQTAT_COM_NONE`）。
  - `areka-P0-window-placement` と並走する。本ユニットは `crates/areka` の既存ファイル（main.rs・placement 系）を触らず、emo-present と areka-parsers への変更は additive 増分のみ・example 新規追加のみ行う（衝突面ゼロ）。
  - emo2-boot／emo-text-viewbox／choice-render／M2 text effects が本ユニットの成果（sink 型・可視窓/描画分離シーム・行レイアウト/クリック範囲シーム・行列領域/装飾シーム）を下流で消費する。
  - **シェル/バルーン compositor 融合（後続 roadmap ユニット）**が本ユニットの共有描画基盤（統一 resident/行列モデル）を消費し、emo-compose の surface 合成との収束・背景 SERIKO サーフェスの住人化を実現する。本ユニットは収束可能な統一形の設計に留め、emo-compose の改変・共有 canvas の抽出は行わない。

## Requirements

### Requirement 1: cue 受信アクターと UI スレッド配送・終了規律

**Objective:** As a emo テキスト層, I want sakura の Balloon 向け cue を受信して描画スレッドへ配送すること, so that スクリプトのセリフがバルーンへ流れる経路が成立する

#### Acceptance Criteria

1. The emo テキスト層 shall sakura ✅ の `TextSink` を実装し、Balloon 向け cue（Text／NewLine／Clear）を受信端で受け取る。
2. When Balloon 向け cue を受信端が受け取る, the emo テキスト層 shall その cue を UI スレッドへ配送し、UI スレッド上で描画状態を更新する。
3. The emo テキスト層 shall 受信端をワーカー側、描画を UI スレッド側とし、受信から描画までを UI スレッド固定の配送口（`spawn_ui`/`UiSender` 相当）経由で行う。
4. When 受信端の全送信元が切断される、または終了指示（Close 相当）を受け取る, the emo テキスト層 shall 受信ループをクリーンに終了する（error ログを伴わず正常終了する）。
5. If cue 配送・状態更新の途中で失敗が生じる, then the emo テキスト層 shall その失敗を error ログとして記録し、後続 cue の受理を破壊しない。
6. When Balloon 向け cue を受け取る, the emo テキスト層 shall その cue の `actor`（`ActorKey`・"0"=sakura／"1"=kero…）を鍵として対応するアクター別のテキスト状態へ振り分ける。描画状態は `ActorKey → テキスト状態` のマップとして保持し、実装住人（描画される actor）は fixture スクリプトが発話させる actor に従う（構造は最初から多アクター・M1 実挙動は script 次第）。

### Requirement 2: テキスト状態機械（純粋・決定論）

**Objective:** As a emo テキスト層, I want cue 列を表示テキストの行/グリフ状態へ純粋に遷移させること, so that DirectWrite metrics に依存せず描画前状態を決定論的に検証できる

#### Acceptance Criteria

1. When Text cue を受け取る, the emo テキスト層 shall 表示テキストへ当該文字列を追記する。
2. When NewLine cue を受け取る, the emo テキスト層 shall 表示テキストの行を改める。
3. When Clear cue を受け取る, the emo テキスト層 shall 表示テキストを全消去し、行/グリフ状態を初期状態へ戻す（typewriter 進行中の場合、未リビールの文字も含めて破棄する＝後出し優先）。
4. The emo テキスト層 shall cue 列から行/グリフ状態への遷移を、実描画（DirectWrite）を伴わずに実行できる純粋な形で提供する。
5. The emo テキスト層 shall 同一の cue 列と同一の入力条件に対し、行/グリフ状態の遷移結果が決定論的に一致するようにする。

### Requirement 3: typewriter 逐次表示（注入時刻駆動）

**Objective:** As a ユーザ, I want セリフが1文字ずつ順に現れること, so that 伺かのバルーン表示として自然な逐次表示になる

#### Acceptance Criteria

1. While テキストが表示途中である, the emo テキスト層 shall 文字を1文字ずつ順に可視化する（typewriter 進行）。
2. The emo テキスト層 shall per-glyph の進行間隔を本層で所有する（balloon descript の文字送り待ち相当を含む）。
3. The emo テキスト層 shall typewriter 進行を注入された時刻（talk 起点相対秒）に基づいて進め、実時間 sleep に依存しない。
4. When 受信した cue の `at`（chunk 開始時刻）に達する, the emo テキスト層 shall 当該 chunk を即時にテキスト状態へ適用し、その逐次表示（リビール）を開始する。`at` はリビール開始の下限（それより早く可視化しない）であり、直前 chunk が未リビールでもリビールカーソルは現バッファ末尾を本層ペースで追う（長文時はリビールが遅延しうる・無損失）。
5. The emo テキスト層 shall 同一の cue 列と同一の注入時刻列に対し、各時刻での可視文字数が決定論的に一致するようにする。
6. When typewriter 進行中に後続 cue が到着する, the emo テキスト層 shall 後出し優先で即時適用する（Text/NewLine は追記・Clear は未リビール分を含め全消去）。トーク上書きを抑止するガードは本層の責務でなく、中断可否は上流（kanade）で決着済みの前提とする。

### Requirement 4: フォント・テキスト領域の解決と文字レイアウト

**Objective:** As a emo テキスト層, I want balloon descript 由来のフォントと領域で文字をレイアウトすること, so that バルーンの定義どおりに文字が配置・折返しされる

#### Acceptance Criteria

1. The emo テキスト層 shall balloon-parse ✅ のフォント定義（name/height/color）を消費して文字を描画する。
2. If フォント定義が欠落する, then the emo テキスト層 shall SSP 既定フォント（ＭＳ ゴシック）へフォールバックする。
3. The emo テキスト層 shall balloon-parse ✅ のテキスト領域定義（origin＝描画原点・wordwrappoint＝折返し点・validrect＝有効矩形）を消費して文字の配置・折返し・有効範囲を決定する。
4. When テキスト領域定義の座標が反対辺基準（負値）で与えられる, the emo テキスト層 shall balloon-parse ✅ のモデル規約に従って反対辺基準として解釈する。
5. The emo テキスト層 shall 文字レイアウトのうち DirectWrite metrics に依存しない決定部（折返し位置・行送り・スクロール発火）を、metrics に依存しない構造テストで検証できる形で分離する。
6. The emo テキスト層 shall テキストレイアウト座標（font.height／origin／wordwrappoint／validrect）をバルーン surface の画像座標空間（`descript_balloon.dpi`＝作者基準・省略時 96 が定義する空間）で解決し、描画ターゲットには実際の合成スケール（バルーン surface と同一のスケール）を適用して、任意のモニタ DPI で文字がバルーン画像と整合（ずれない・validrect からあふれない）するようにする。**DPI/スケールは M1 の対象外にせず最初から正しく扱う**（論理/物理の混在を設計時に排する・記憶 areka-window-placement-dpi-coordinate-defect の教訓）。

### Requirement 5: writing_mode 宣言の解決（2層マージ）

**Objective:** As a ゴースト作者, I want バルーン定義で縦書きを宣言できること, so that 日本語縦書きのバルーンを既定の横書きと切り替えられる

#### Acceptance Criteria

1. The emo テキスト層 shall 縦書き/横書きを宣言する areka 拡張キー `writing_mode` を受理する。キー・値とも snake_case とし、値は CSS `writing-mode` 語彙の `horizontal_tb`／`vertical_rl`／`vertical_lr` とする。
2. Where `writing_mode` が宣言される, the emo テキスト層 shall descript.txt（バルーン全体既定）を下位、画像別 `balloons*s.txt`／`balloonk*s.txt`（画像別上書き）を上位とする2層マージ（後勝ち）で有効値を解決する。
3. When `writing_mode` マーカーが存在しない, the emo テキスト層 shall `horizontal_tb`（横書き・SSP 互換の既定）として扱う。
4. If `writing_mode` に未知の値が指定される, then the emo テキスト層 shall warn ログを記録し、`horizontal_tb` へフォールバックする。
5. The emo テキスト層 shall `writing_mode` の解決結果を、文字レイアウトの方向（横書き左→右／日本語縦書き右→左／縦書き左→右）へ 1:1 で写像する。
6. When balloon model へ `writing_mode` の転記フィールドを設ける, the emo テキスト層 shall balloon-parse ✅ のモデルへ additive な転記フィールドを増やすに留め、値の解釈は本層で行う（parser は転記に徹する）。
7. The emo テキスト層 shall M2 予約キー `text_orientation`／`text_combine_upright` を予約名として記録するに留め、その実挙動を実装しない。

### Requirement 6: 縦書き/横書きの軸解釈

**Objective:** As a emo テキスト層, I want writing_mode に応じて折返しとスクロールの軸を回転させること, so that 縦書きでも正しく折返し・スクロールできる

#### Acceptance Criteria

1. While `writing_mode` が横書き（`horizontal_tb`）である, the emo テキスト層 shall 行内を左→右・行送りを上→下とし、折返し軸を横（wordwrappoint の x 相当）・スクロールを縦方向とする。
2. While `writing_mode` が縦書き（`vertical_rl`）である, the emo テキスト層 shall 行内を上→下・行送りを右→左とし、折返し軸を `wordwrappoint.y`・スクロールを横方向（行が左へ流れる）とする。
3. The emo テキスト層 shall 縦書き時の origin／wordwrappoint／validrect の軸読み替え規則（横書きの top/bottom/left/right が縦書きでどう回るか）を、単一の明文化規則に従って一貫して解決する。
4. The emo テキスト層 shall 縦書きを完了条件に含める一方、実装順として横書きを先行させても、`writing_mode` 抽象（方向写像・折返し軸・スクロール軸の切替点）を最初から構造に持つ。

### Requirement 7: 領域あふれ時のスクロール

**Objective:** As a ユーザ, I want テキストが有効領域を超えたときスクロールして続きが読めること, so that 長いセリフでも全文が表示される

#### Acceptance Criteria

1. When 追記された文字が validrect の有効領域を超える, the emo テキスト層 shall テキストをスクロールさせて新しい内容を可視領域に収める。
2. While 横書きである, the emo テキスト層 shall 縦方向へスクロールさせる。While 縦書きである, the emo テキスト層 shall 横方向へスクロールさせる。
3. The emo テキスト層 shall スクロールを validrect サイズのキャンバスへ可視窓を描き直す全域再描画で実現する。
4. The emo テキスト層 shall スクロール描画を「可視窓の決定（スクロール位置→表示行の純粋な計算）」と「描画実行」に分離し、後続の viewbox 合成化が描画実行の差し替えだけで済む移行シームを残す。
5. The emo テキスト層 shall スクロール発火（あふれ判定）を DirectWrite metrics に依存しない構造テストで決定論的に検証できる形にする。

### Requirement 8: 行列変換領域と emo 共有描画基盤

**Objective:** As a emo テキスト層, I want 描画先を変換行列付きの emo 共有描画基盤（内容キャンバス）として持つこと, so that M2 の回転・文字装飾・画像同居、および将来のシェル/バルーン描画融合（背景 SERIKO 上のテキスト演出）を破壊的変更なしに解禁できる

#### Acceptance Criteria

1. The emo テキスト層 shall 描画先を単純な矩形ではなく変換行列付き領域として保持する。
2. While M1 である, the emo テキスト層 shall 変換行列の実挙動を恒等/平行移動のみに限る。
3. The emo テキスト層 shall 回転値および文字装飾（アウトライン/多色/シャドウ）を M2 予約の型シームとして保持するに留め、その実挙動を実装しない。
4. The emo テキスト層 shall 描画面を emo 共有描画基盤（統一 resident/行列モデル）として設計し、住人（キャンバスに置かれる変換行列付き矩形コンテンツ）としてグリフ（文字）・画像（`\_b`）・将来の SERIKO サーフェスを同格に扱える抽象を持つ。文字を M1 の唯一の実装住人とする。
5. The emo テキスト層 shall M1 の実挙動をテキスト描画に限り、`\_b` 画像等の実挙動を実装しない（fixture 実測で未使用であることを前提とし、使用が判明した場合はシームとして扱う）。
6. The emo テキスト層 shall 共有描画基盤の抽象を emo-compose の surface 合成（行列原則）と収束可能な統一形として設計するに留め、M1 では emo-compose を改変しない（共有 canvas の抽出・シェル/バルーン compositor 融合・背景 SERIKO の住人化は後続 roadmap ユニットへ予約する）。

### Requirement 9: 予約スロットへの描画装着

**Objective:** As a emo テキスト層, I want emo-present の予約スロットへ描画内容を装着すること, so that バルーン枠の上に文字層が独立して重なる

#### Acceptance Criteria

1. The emo テキスト層 shall emo-present ✅ の予約スロット（`emo-text-layer-slot` Visual）へ描画内容を装着する公開経路を用いる。
2. When 予約スロット到達手段が emo-present に非公開である, the emo テキスト層 shall emo-present へ additive な公開増分（text_slot への到達手段または装着 API）を加える。
3. When 文字（グリフ）更新のみが生じる, the emo テキスト層 shall surface 本体の再合成（emo-compose 再駆動）を強要せず、テキスト層を独立に更新する。
4. The emo テキスト層 shall 予約スロットの装着経路を、choice-render（M-dialogue）が行レイアウト・クリック可能範囲の返却に再利用できる構造シームとして提供する（クリック範囲の実導出は実装しない）。
5. When 複数の actor（\0／\1…）が発話する, the emo テキスト層 shall 各 actor のテキストを、その actor に対応する target の予約スロット（`emo-text-layer-slot`）へ振り分けて装着する（単一 actor のみの場合は当該 actor の target へ装着する）。

### Requirement 10: クロスユニット契約シーム

**Objective:** As a 下流ユニット, I want 本ユニットが後続を詰ませない契約シームを残すこと, so that emo2-boot・emo-text-viewbox・choice-render・sakura への接続が破壊的変更なしに進む

#### Acceptance Criteria

1. The emo テキスト層 shall sakura の `TextSink + Clone + Send + 'static` を満たす sink 型を提供し、`GhostBootOptions.text_sink` へ注入可能な形にする（注入・main 結線そのものは emo2-boot の責務）。
2. The emo テキスト層 shall テキスト層の per-glyph pacing が sakura の cue 時刻（`at`）に影響しない前提で動作し、厳密な SSP 互換 pacing が必要と判明した場合は sakura への増分申し送りとして扱う（本ユニットで sakura を改変しない）。
3. The emo テキスト層 shall `\f` 系文字装飾および `disable.font.*` 拡張を、emo2 fixture で未使用の範囲では型シームとして保持するに留め、実挙動を実装しない。
4. The emo テキスト層 shall バルーン推奨 DPI（`descript_balloon` の `dpi`・省略時 96）を**最初から正しく扱う**（R4.6 の座標/スケール契約に従う・96 素通しの先送りはしない）。window-placement と DPI/スケール契約を共有し、論理/物理の混在（記憶 areka-window-placement-dpi-coordinate-defect）を設計時に排する。
5. The emo テキスト層 shall トーク上書きを抑止するガード（さくらスクリプト `\t` タイムクリティカル／`\![enter,nouserbreakmode]` 等）を実装せず、中断可否の判定を上流（kanade の中断ファンネル）の責務とする。emo は届いた cue 列を後出し優先で忠実に適用する。

### Requirement 11: 観測用専用 example（注入時刻駆動 pass/fail）

**Objective:** As a 開発者, I want 単一 pass/fail で振る舞いを確認できる専用 example, so that 文字表示・改行・スクロール・縦横切替・全消去が正しいことを実機で証明できる

#### Acceptance Criteria

1. The emo テキスト層 example shall emo2 fixture のバルーン枠（emo-present ✅ の `build_balloon_target` 相当）上に、fixture スクリプト由来の cue 列（Text/NewLine/Clear）を注入時刻駆動で流す。
2. When example が cue 列を注入する, the emo テキスト層 example shall 文字が typewriter 進行で描画されることを観測可能にする。
3. When NewLine cue を注入する, the emo テキスト層 example shall 改行を観測可能にし、validrect あふれ時にスクロールすることを観測可能にする。
4. When `writing_mode` マーカー（descript／画像別 `balloons*s.txt`・`balloonk*s.txt`）で縦書き/横書きを切り替える, the emo テキスト層 example shall 縦書きでは折返しが `wordwrappoint.y`・スクロールが横方向へ切り替わることを観測可能にする。
5. When Clear cue を注入する, the emo テキスト層 example shall 表示が全消去されることを観測可能にする。
6. The emo テキスト層 example shall 上記のうちレイアウト決定論部分（折返し位置・行送り・スクロール発火・`writing_mode` 2層マージ解決）を、DirectWrite metrics に依存しない構造テストと既定フォントでの単体テストで決定論的に検証する。
7. The emo テキスト層 example shall `crates/areka` の既存ファイル（main.rs・placement 系）を変更せず、新規 example ファイルの追加のみで観測を成立させる。
8. When fixture スクリプトが複数の actor（\0／\1）を発話させる, the emo テキスト層 example shall 各 actor のテキストが対応するバルーンへ振り分けられることを観測可能にする（単一 actor のみの場合は当該 actor のバルーンで観測する）。
9. The emo テキスト層 example/テスト shall DPI/スケールの正しさを検証する——レイアウト決定部（折返し・行送り・スクロール発火・validrect 整合）はスケール非依存の構造テストで、実描画は非 96 DPI を含む実 DPI で観測可能にする（実 DPI 実行を経ない「テスト緑」を正しさの証明としない・記憶 areka-placement-real-ghost-first）。

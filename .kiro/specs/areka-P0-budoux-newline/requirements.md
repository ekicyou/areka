# Requirements Document

## Project Description (Input)

emo テキスト層（crate `areka-emo-text`）の自動折り返しは**文字単位**（`inline_pos + advance > 閾値` で行を閉じる）であり、語や文節の途中でぶつ切りになる。emo2 実機バルーンは 1 行が狭く（sakura 側 ≈ 全角 13 文字／kero 側 ≈ 全角 9 文字・実測 2026-07-18）、ぶつ切りが頻発して可読性を損なう。本 spec は、balloon descript に areka 拡張キー `budoux_newline` を書いた（opt-in した）バルーンに限り、自動折り返しの**分割点を budoux 分かち書き境界（文節っぽい塊の境界）に揃える**ワードラップモードを emo テキスト層内で完結して導入する。塊は行末で途中分割されず丸ごと次行へ送られる。既定は OFF（キー無し・`0`/`false`＝現行の文字単位折返しのまま）。塊が行頭からでも 1 行に収まらない場合のみ、その塊に限って従来の文字単位折返しへ縮退する（開発者裁定 2026-07-18・(a) 案）。明示改行（`\n` 系 LineBreak）の意味論・閾値の源（`wordwrappoint`／`validrect`）は不変で、分割点の選び方だけを変える。縦書き（`vertical_rl`／`vertical_lr`）でも軸読み替え正準表の上で同一規則とし、typewriter（reveal）中に配置済みグリフが後から行を移る「リフロー跳び」を起こさない。分かち書き境界は決定論的な純関数（budouy `vendored-models`・ネットワーク不要のモデル同梱形）で計算し、GPU 非依存で全網羅テストする。実機は fixture balloon descript へ `budoux_newline,1` を追記して有界 auto-exit＋出力画像 AI vision 目視で確認する。前提として `areka-P0-newline-defer` の完了後に直列で着手する（同一 `layout.rs` の折返し/改行分岐とテスト檻を直接編集するため）。

## Introduction

本ユニットは、完了済み `areka-P0-emo-text-layer`（crate `areka-emo-text`）が実装した**文字単位**の自動折り返しを拡張し、balloon descript の areka 拡張キー `budoux_newline` で opt-in した場合に限って**分かち書き境界（budoux セグメント境界）でのワードラップ**へ切り替える P0 機能追加ユニットである。

本 spec における折り返しの正準モデルは「**分かち書き境界で区切られた塊（セグメント）は、行末で途中分割せず、残り行幅に収まらなければ塊まるごと次行へ送る**」である。閾値そのもの（`TextRegion.wrap_threshold`＝ balloon descript `wordwrappoint`・無指定時は validrect 遠辺へ縮退）は不変で、変えるのは「どこで行を分けるか（分割点の選択）」だけである。塊の先頭グリフを置く時点で「塊全体が残り行幅に収まるか」を実グリフ advance の合計で先決し、収まらなければ塊の前で行を送る。この**先決**により、typewriter が塊の途中を reveal した後に配置済みグリフが行を移る「リフロー跳び」は起きない。

`budoux_newline` は SSP に存在しない**純 areka 拡張**であり、`writing_mode` と同格の snake_case・prefix 無しの拡張キー規約に従う。既定は OFF（キー無し・`0`/`false`）で、OFF 時は既存の文字単位折返しコードパスを一切変えない。未知値は `warn!` の上で OFF へフォールバックする（`writing_mode` と同じ縮退姿勢）。受理値は真偽フラグ（ON＝`1`／`true`・OFF＝`0`／`false`——伝統的な数字フラグと今風の `true`/`false` の双方を受理する）に留めるが、値解決は将来のワードラップ戦略名を第一級化しうる型シーム（`WrapMode` enum・設計で確保）に載せ、本 spec の実導出は bool 受理に閉じる（討議 #1 決定 2026-07-18・[defer-canon-with-full-vocabulary-and-tracking-spec] のシーム確保姿勢）。分かち書き境界の計算は決定論的な純関数であり、実描画（DirectWrite metrics）に依存せず全網羅的に検証できる。あふれ→スクロール機構それ自体・明示改行（`\n` 系）の意味論・`wordwrappoint`／`validrect` の解決規則は変更しない。

## Boundary Context

- **In scope（本ユニットが観測可能に実現する振る舞い）**:
  - balloon descript `budoux_newline` キーの転記（基層＋画像別上書き層の後勝ちマージに乗る）と、emo テキスト層での語彙解決（`1`/`true`→ON・`0`/`false`／欠落→OFF・未知値→warn+OFF）。
  - ON 時の分かち書き境界ワードラップ（塊を行末で途中分割せず、残り行幅に収まらなければ塊まるごと次行へ送る）。
  - 長大セグメントの文字単位縮退（行頭からでも 1 行に収まらない塊に限り、その塊だけ従来の文字単位折返しへ縮退・はみ出し／無限ループなし）。
  - OFF 時（既定）の既存文字単位折返し挙動の完全不変（非回帰）。
  - 明示改行（`\n` 系 LineBreak）の意味論と閾値の源（`wordwrappoint`／`validrect`）の不変。
  - 縦書き（`vertical_rl`／`vertical_lr`）でも軸読み替え正準表の上で同一のワードラップ・縮退規則。
  - typewriter（reveal）との整合＝塊先頭配置時の先決による「リフロー跳び」の不発生。
  - 分かち書き境界の決定論・純関数・全網羅検証（境界計算・折返し判定・縮退・OFF 不変）。
  - 実機（emo2 fixture・pasta SHIORI）での分かち書きワードラップの可視確認（fixture balloon descript への `budoux_newline,1` 追記・有界 auto-exit＋出力画像 AI vision 目視）。

- **Out of scope（本ユニットが所有・改変しない）**:
  - sakura compile／cue 語彙の改変（境界ヒント cue 案は層違反として棄却済み——折返し可否はバルーン幅依存＝emo の関心）。
  - pasta／fixture／サブモジュールの改変（fixture balloon descript への `budoux_newline,1` 追記は実機確認用の最小編集のみ可）。
  - 禁則処理（行頭句読点回避等）の独自実装（budoux 境界が実用上吸収する分のみ・専用禁則エンジンは作らない）。
  - `wordwrappoint`／`validrect` の解決規則そのものの変更（閾値の源は不変・分割点の選び方だけ変える）。
  - 明示改行（`\n` 系）・スクロール／あふれ機構それ自体の意味論変更（`areka-P0-newline-defer` が扱う改行遅延の意味論は非改変）。
  - `budouy` セグメンテーションのオンラインモデル取得（ネットワーク依存の形は採らない・`vendored-models` 同梱形のみ）。

- **Adjacent expectations（隣接ユニットへの期待・依存）**:
  - **Extends**: `completed/areka-P0-emo-text-layer` の折返し判定の**分割点選択**を拡張する（OFF 時は既存挙動不変・R6 の軸読み替え正準表を継承）。
  - **先行必須**: `completed/areka-P0-newline-defer`。両者とも同一 `layout.rs` の折返し分岐と改行（LineBreak／保留改行）分岐およびテスト檻を直接編集するため、`newline-defer` 完了後に直列で着手する（同時進行は衝突）。本 spec は改行遅延の意味論を前提として乗る（保留改行の実体化時に、本 spec のワードラップ判定が働く）。
  - **同 crate 併走の回避**: `areka-P0-choice-render`（W3・同 crate `state.rs` 宛先）。本 spec は `state.rs` 非改変を理想とし、分割点計算を `state.rs` の外に置けるかは design で確定する。
  - `budouy` 0.2.2（`features=["vendored-models"]`・Apache-2.0）を新規依存として `areka-emo-text` にのみ追加する（開発者指名により承認済み）。
  - **Downstream**: `areka-P0-emo2-conformance-e2e`（適合走行で本機能の実機効果を最終確認）。

## Requirements

### Requirement 1: `budoux_newline` 拡張キーの転記と語彙解決

**Objective:** As a ゴースト作者, I want balloon descript に `budoux_newline` を書くことで分かち書きワードラップを opt-in できること, so that 既存ゴーストへ無害なまま、望むバルーンだけワードラップを有効化できる

#### Acceptance Criteria

1. When balloon descript に `budoux_newline` キーが記述されている, the parsers 層 shall その値を検証せず生文字列としてバルーンモデルへ転記する（転記層の規律・基層と画像別上書き層の後勝ちマージに乗せる）。
2. When 有効値解決で `budoux_newline` の値が `1` または `true` である, the emo テキスト層 shall 当該バルーンの折返しモードを分かち書きワードラップ（ON）として解決する。
3. When `budoux_newline` キーが欠落している、または値が `0` もしくは `false` である, the emo テキスト層 shall 折返しモードを従来の文字単位折返し（OFF）として解決する（正常系につきログなし）。
4. If `budoux_newline` の値が受理語彙（`1`／`true`／`0`／`false`）のいずれでもない, then the emo テキスト層 shall その値を含む `warn!` を出力した上で OFF へフォールバックする（縮退継続）。
5. The emo テキスト層 shall `budoux_newline` を SSP に存在しない純 areka 拡張キー（snake_case・prefix 無し）として扱い、未知キーとして自然無視される既存ゴーストの挙動に影響を与えない。

### Requirement 2: 分かち書き境界でのワードラップ折返し

**Objective:** As a ユーザ, I want 狭いバルーンでも語や文節が途中で途切れず読めること, so that 会話テキストの可読性が向上する

#### Acceptance Criteria

1. While 折返しモードが ON である, the emo テキスト層 shall テキストを分かち書き境界（budoux セグメント境界）で区切られた塊の列として扱い、折返しの分割点を塊の境界にのみ置く。
2. When ON で塊の先頭グリフを配置しようとし、当該塊の全グリフ advance 合計が現在行の残り行幅（閾値までの残り）に収まらない, the emo テキスト層 shall 当該塊の先頭グリフを配置する前に行を送り、塊を次行の先頭から配置する。
3. When ON で塊が現在行の残り行幅に収まる, the emo テキスト層 shall 塊を途中で分割せず現在行へ続けて配置する。
4. The emo テキスト層 shall 折返し判定に用いる残り行幅の閾値の源（`TextRegion.wrap_threshold`＝`wordwrappoint`／validrect 縮退）を変更せず、分割点の選び方のみを変える。
5. When ON で分かち書き境界を計算する, the emo テキスト層 shall 明示改行（`\n` 系 LineBreak）や Clear で区切られたテキストラン単位で境界を求め、テキストランをまたいで塊を結合しない。

### Requirement 3: 長大セグメントの文字単位縮退

**Objective:** As a ユーザ, I want どれだけ長い塊でもバルーンからはみ出さず表示されること, so that ワードラップ有効時でも表示が破綻しない

#### Acceptance Criteria

1. If ON で塊が行頭（残り行幅が最大）からでも 1 行の閾値に収まらない, then the emo テキスト層 shall 当該塊に限って従来の文字単位折返し（`inline_pos + advance > 閾値` で行を閉じる）へ縮退する。
2. While 長大セグメントを文字単位縮退している, the emo テキスト層 shall 行頭 1 グリフは閾値超過でも配置し（無限折返し回避）、はみ出し・無限ループを発生させない。
3. When 長大セグメントの文字単位縮退が完了した後に次の塊が続く, the emo テキスト層 shall 後続の塊に対しては通常の分かち書きワードラップ判定を再開する（縮退は当該塊に閉じる）。

### Requirement 4: OFF 経路（既定）の不変保証

**Objective:** As a ゴースト作者, I want `budoux_newline` を書かない既存ゴーストの折返しが一切変わらないこと, so that 本機能の追加が既存表示へ回帰を起こさない

#### Acceptance Criteria

1. While 折返しモードが OFF（既定・キー無し／`0`／`false`／未知値フォールバック）である, the emo テキスト層 shall 既存の文字単位折返し（`inline_pos + advance > 閾値`）の挙動・行構成・行送りを一切変更しない。
2. The emo テキスト層 shall OFF 時に budoux 分かち書き境界の計算を折返し結果へ反映しない。
3. The emo テキスト層 shall 既存の文字単位折返しの検証（emo-text-layer の折返し檻）が OFF 時の非回帰檻として引き続き成立する形を保つ。

### Requirement 5: 明示改行・スクロール機構の意味論不変

**Objective:** As a emo テキスト層, I want 分割点選択だけを変え、改行やあふれの意味論は変えないこと, so that 隣接ユニット（newline-defer 等）の挙動と干渉しない

#### Acceptance Criteria

1. The emo テキスト層 shall 明示改行（`\n`／`\n[ratio]`＝NewLine cue／LineBreak）の行送り・保留・実体化の意味論を本 spec で変更しない。
2. The emo テキスト層 shall あふれ→スクロール機構それ自体（あふれ判定入力・発火時の可視窓決定・全域再描画）を本 spec で変更しない。
3. While `areka-P0-newline-defer` の改行遅延が有効な状態でワードラップが ON である, the emo テキスト層 shall 保留改行が次グリフ配置で実体化する時点で本 spec のワードラップ判定を適用し、両者の意味論を矛盾なく両立させる。

### Requirement 6: 縦書きでの同一規則

**Objective:** As a emo テキスト層, I want ワードラップ規則を縦書きでも横書きと同一に適用すること, so that writing_mode に関わらず分かち書き折返しが成立する

#### Acceptance Criteria

1. While `writing_mode` が縦書き（`vertical_rl`／`vertical_lr`）でワードラップが ON である, the emo テキスト層 shall 分かち書き境界での折返し・長大セグメント縮退の規則を横書きと同一に適用する。
2. When 縦書きでワードラップの行送りを行う, the emo テキスト層 shall 完了済み `areka-P0-emo-text-layer` の軸読み替え正準表（行内軸・行送り軸・行送り方向）に従って行送り軸へ写像する。

### Requirement 7: typewriter（reveal）との整合とリフロー跳びの不発生

**Objective:** As a ユーザ, I want タイプライタ表示中に配置済みの文字が後から別の行へ飛ばないこと, so that 逐次表示中も文字位置が安定して見える

#### Acceptance Criteria

1. When ON で塊の先頭グリフを配置する, the emo テキスト層 shall その時点で塊全体が残り行幅に収まるかを（可視 prefix に依らず）全文 lookahead で判定し、行送りの要否を先決する。
2. While typewriter リビールが進行中である, the emo テキスト層 shall 一度配置したグリフの所属行を、後続グリフのリビールによって移動させない（リフロー跳びを起こさない）。
3. The emo テキスト層 shall ワードラップの行送り先決を reveal 進行（可視 prefix 打切り）と両立させ、可視化されるグリフの行位置が最終レイアウトと一致するようにする。

### Requirement 8: 分かち書き境界計算の決定論と全網羅検証

**Objective:** As a 開発者, I want ワードラップの判断分岐を決定論的に全網羅検証すること, so that 回帰なく安全に導入できる

#### Acceptance Criteria

1. The emo テキスト層 shall 分かち書き境界の計算を、ネットワークに依存しないモデル同梱形で行い、同一入力に対し常に同一境界を返す（決定論・オフライン CI 整合）。
2. The emo テキスト層 shall 分かち書き境界の計算・ワードラップ折返し判定・長大セグメント縮退・OFF 不変の各判断分岐を、実描画（DirectWrite metrics）に依存しない純粋な形で提供し、注入メトリクス（`FixedMetrics`）で決定論的に全網羅検証できるようにする。
3. The emo テキスト層 shall 「塊が残り行幅に収まらないとき塊まるごと次行へ送る」「行頭からでも収まらない塊のみ文字単位縮退する」「OFF 時は文字単位折返しが不変である」ことを、metrics 非依存の構造テストで決定論的に検証できる形にする。

### Requirement 9: 実機での分かち書きワードラップ確認

**Objective:** As a ユーザ, I want 実機バルーンで語や文節が途中で切れずに折り返されること, so that 実際のゴースト表示で可読性向上を確認できる

#### Acceptance Criteria

1. When 実機（emo2 fixture・pasta SHIORI）で `budoux_newline,1` を追記したバルーンにテキストを表示する, the emo テキスト層 shall 分かち書き境界で折り返し、塊を行末で途中分割しない表示を生成する。
2. When 実機で長大セグメントを表示する, the emo テキスト層 shall バルーンからはみ出さず、当該塊のみ文字単位縮退した表示を生成する。
3. The emo テキスト層 shall 実機確認を有界 auto-exit（`AREKA_APP_SMOKE_EXIT_MS`）＋出力画像の AI vision 目視（emo-text byte 等価の盲点対策）で行える形にする。

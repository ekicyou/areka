# Requirements Document

## Introduction

本仕様は areka の ⑤ seriko トラックにおける **M-boot ユニット（`areka-P0-seriko-engine`）** の要件を定める。

sakura（④）が再生出力として `SurfaceSink` へ流す surface 指令（`\s[ID]` 相当の `TalkCue`）を受け取り、**「今どのスコープにどの surface を出すか」という per-scope の surface 状態**と、shell descript 由来の**静的 bind 集合**を所有し、それに基づいて emo（⑥）への**表示指令（scope・surface_id・bind 集合）を発行する** actor を提供する。

上流契約は両方とも正本確定済みである——sakura の再生出力契約（`TalkCue`／`SurfaceSink`）と emo-compose の合成入力契約（surface id ＋ `BindSet`／alias 解決表）。本ユニットは既存契約を再定義せず、sakura の sink trait を実装し、emo-compose の alias 解決表を消費する。

本ユニットは **「静的＋指令適用」のみ** を担う。SERIKO の interval ループ／blink 等の時間駆動アニメ（M-life `seriko-loop`）、および bind の動的切替（M-mayuna `mayuna-compose`）は本ユニットの範囲外であり、本ユニットはそれらが後から差し込めるシーム（状態→合成指令の単一発行点・bind 状態の置き場）だけを用意する。

観測は表示を伴わず決定論的に行える——fixture の `TalkCue` 列を直入力し、mock の emo 出力先が受け取った発行列（scope・surface_id・bind 集合・非表示遷移）が期待と一致することで pass/fail を判定する。

## Boundary Context

- **In scope**:
  - sakura の surface 系出力先抽象（`SurfaceSink`）を実装するアクターの提供（inbox で surface 系 `TalkCue` と停止指令 Close を受ける）。
  - alias／name 文字列および数値 id を surface id へ解決すること（正本は emo-compose の alias 解決表）。
  - per-scope（話者スコープごと）の現 surface 状態の保持（非表示状態を含む）。
  - shell descript の bindgroup default に基づく静的 bind 集合の起動時解決と保持。
  - 状態変化に応じた emo への表示指令（scope・surface_id・bind 集合）の発行。
  - 発行点の単一化（後続の時間駆動ループが同じ発行点を再利用できる形）。
  - 解決不能・未知入力に対するログ規律に沿った失敗処理（silent failure 禁止）。
- **Out of scope**:
  - SERIKO の interval ループ・blink・時間駆動アニメ再生（`\i[ID]` 相当を含む）——`seriko-loop`（M-life）の領分。
  - bind 集合の動的切替（着せ替え操作）——`mayuna-compose`（M-mayuna）の領分。本ユニットは bind 状態の置き場のみを持つ。
  - surface の実合成（合成結果の正しさ）——`emo-compose`（完了済み）の領分。
  - 表示の実体・AlphaMask 生成——`emo-present` の領分。
  - collision（さわり判定領域）——`collision-geometry` の領分。
  - さくらスクリプトの解析・talk の運行・中断調停——sakura（④）／kanade（③）の領分。本ユニットは既に転写された surface 系 `TalkCue` を受けるだけである。
- **Adjacent expectations**:
  - **sakura（上流・正本）**: 話者スコープは既に `ActorKey`（"0"/"1"…）へ転写され、surface 指令は不透明な surface 引数（alias／name／数値／非表示指定）として届く。本ユニットは alias／id／非表示の**解釈責務**を負う（sakura は不透明転写のみ）。
  - **emo-compose（上流・正本）**: alias／name→id の解決表と bind 集合表現は emo-compose が正本として提供する。本ユニットは同一の解決表を消費し、二重定義しない。
  - **emo-present（並走・対向）**: 表示指令 API 形の正本は emo-present 側にある。本ユニットは同 API に「非表示」の意味論を発行できることを期待する。emo-present 完了前でも、本ユニット定義の観測用 mock 出力先で単体観測を成立させる。
  - **ghost-setup（並走・結線）**: 本ユニットのアクターは sakura dispatcher の surface 系 sink 差し込み口（`SurfaceSink` 実装）へ挿さる。trait 実装であること自体が結線契約であり、追加の口は設けない。

## Requirements

### Requirement 1: surface 系出力の受領アクター

**Objective:** As a ghost 起動系（ghost-setup）, I want sakura の surface 系出力先へ挿さる独立アクターを持ちたい, so that script が発する surface 指令を専用スレッド上で受けて surface 状態へ反映できる。

#### Acceptance Criteria

1. The seriko アクター shall sakura が定義する surface 系出力先抽象（`SurfaceSink`）を実装する（契約を再定義しない）。
2. When surface 系の発火（`TalkCue`）が出力先へ届く, the seriko アクター shall その発火を per-scope surface 状態の更新入力として受理する。
3. The seriko アクター shall areka-actor のアクター規約に従い独立スレッド上で稼働し、inbox 経由で発火と停止指令（Close）を受ける。
4. When 停止指令（Close）を受ける, または全ての送信端が破棄される, the seriko アクター shall 稼働を正常終了する。
5. While 稼働中, the seriko アクター shall 入力を到着順に処理し、後続に共有するデータは他スレッドへ安全に受け渡せる形（Send な所有データ）で発行する。

### Requirement 2: alias／name／数値の surface id 解決

**Objective:** As a script 作者, I want surface を alias／name の文字列でも数値 id でも指定したい, so that surfaces.txt の別名や名前定義を使ってサーフェスを切り替えられる。

#### Acceptance Criteria

1. When 数値 id を表す surface 指令を受ける, the seriko アクター shall その数値をそのまま解決後の surface id として扱う。
2. When alias または name の文字列を表す surface 指令を受ける, the seriko アクター shall emo-compose が正本として提供する解決表を用いて surface id を解決する。
3. The seriko アクター shall surfaces.txt の `surface.alias` で定義された文字列と `name` で定義された文字列を同一の解決経路で扱う（両方を同じ解決表から引く）。
4. If 解決表に存在しない alias／name を受ける, then the seriko アクター shall エラーとしてログに記録し、その指令を適用せずに読み飛ばす（surface 状態を変更しない）。
5. Where 1 つの alias が複数の surface id に対応する, the seriko アクター shall 決定論的な単一の選択規則に従って 1 つの surface id を選ぶ（具体的な選択規則は設計で確定する）。

### Requirement 3: per-scope surface 状態と非表示

**Objective:** As a script 作者, I want スコープ（本体・パートナー等）ごとに独立したサーフェス状態を持ちたい, so that あるキャラのサーフェスを変えても他のキャラの表示状態が保たれる。

#### Acceptance Criteria

1. The seriko アクター shall 話者スコープ（`ActorKey`）ごとに現在の surface 状態を独立して保持する。
2. When あるスコープに対する surface 指令を適用する, the seriko アクター shall そのスコープの現 surface 状態のみを更新し、他スコープの状態は変更しない。
3. When 非表示を表す surface 指令（`\s[-1]` 相当）を受ける, the seriko アクター shall 該当スコープの状態を非表示へ遷移させる。
4. While あるスコープが非表示状態にある, the seriko アクター shall そのスコープを非表示として保持し、次に表示 surface が指定されるまで surface を発行しない。
5. When 非表示状態のスコープに表示 surface が指定される, the seriko アクター shall そのスコープを当該 surface の表示状態へ遷移させる。

### Requirement 4: 静的 bind 集合

**Objective:** As a ghost 起動系, I want shell の着せ替え初期状態（bindgroup default）を起動時に反映したい, so that 起動直後から既定の着せ替え要素が合成入力に含まれる。

#### Acceptance Criteria

1. When アクター構築時に shell 定義を受ける, the seriko アクター shall shell descript の bindgroup default に基づく bind 集合を一度だけ解決する。
2. The seriko アクター shall 解決した静的 bind 集合を emo-compose の bind 集合表現（`BindSet`）として保持する。
3. While 本ユニットの稼働中, the seriko アクター shall bind 集合を静的（不変）に保ち、bind の動的切替を行わない（動的切替は範囲外）。
4. The seriko アクター shall bind 状態を per-scope surface 状態と同居する置き場として保持し、後続の動的切替ユニットがその置き場のみを差し替えられる形を提供する。

### Requirement 5: emo への表示指令発行

**Objective:** As a emo（表示合成系）, I want surface 状態の確定結果を表示指令として受け取りたい, so that どの surface をどの bind 集合で合成すべきかを一貫した入力で得られる。

#### Acceptance Criteria

1. When あるスコープの surface 状態が表示 surface へ確定する, the seriko アクター shall そのスコープ・解決済み surface id・現在の bind 集合を含む表示指令を発行する。
2. When あるスコープの状態が非表示へ遷移する, the seriko アクター shall そのスコープに対する非表示遷移を表示指令として発行する。
3. The seriko アクター shall 表示指令の発行を単一の発行点（単一関数）に集約し、後続の時間駆動ループが同じ発行点を再利用できる形にする。
4. The seriko アクター shall 表示指令を、他スレッド／後続 spec へ安全に受け渡せる所有データ（Send）として発行する。
5. Where emo-present の表示指令 API が未完成である, the seriko アクター shall 本ユニット定義の観測用出力先抽象を通じて表示指令を発行できる（emo-present 完了を待たない）。

### Requirement 6: 失敗処理とログ規律

**Objective:** As a 開発者・運用者, I want 解決不能や未知入力が黙って失われないでほしい, so that ゴースト実行時の異常を追跡できる。

#### Acceptance Criteria

1. If 解決不能な alias／name を受ける, then the seriko アクター shall エラーとしてログに記録したうえで当該指令を読み飛ばす（panic せず、他の指令処理を継続する）。
2. If 分類・解釈できない未知の surface 系入力を受ける, then the seriko アクター shall 警告またはエラーとしてログに記録し、当該入力を読み飛ばす。
3. The seriko アクター shall 失敗経路を silent failure（ログ無しの黙殺）にしない。
4. The seriko アクター shall panic を致命的状況に限定し、通常の入力起因の失敗は状態変更を伴わない読み飛ばしとして扱う。

### Requirement 7: 決定論的観測

**Objective:** As a 開発者, I want 表示なしで surface 状態遷移を検証したい, so that 回帰を実行テストで檻に入れられる。

#### Acceptance Criteria

1. When fixture の `TalkCue` 列（数値 id・alias 文字列・非表示指定を含む）を直入力する, the seriko アクター shall mock 出力先への発行列（scope・surface_id・bind 集合・非表示遷移）を期待どおりに生成する。
2. The 観測 shall 表示を伴わず、待機（sleep）を用いず、決定論的に pass/fail を判定できる。
3. The 観測 shall alias／name→id の解決を、emo2 fixture の alias 実データを用いて追験できる。
4. The 検証 shall 指令適用・解決失敗ログ・非表示遷移・停止（Close）による正常終了を、いずれも実行テストで確認できる（構造担保のみで代替しない）。

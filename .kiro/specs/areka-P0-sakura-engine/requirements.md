# Requirements Document

## Project Description (Input)

④ sakura＝**さくらスクリプト再生エンジン**（talk timeline・per-talk transient）。SHIORI が返す Value（さくらスクリプト）を時間軸上で再生する装置——`\w` の待ち・テキストの逐次供給・`\s` の surface 指令・`\e`/`\-` の終端検出——を提供する。ここが埋まらないと「emo2 が喋る」の "喋る" が成立しない。

sakura インスタンスは script 文字列を受け、上流 `areka_parsers::sakura::parse` が生成する `Instruction` 列を時間軸再生し、下流 2 分岐——surface 指令→seriko／テキスト・改行・進行→emo(text-layer)——へ発火列を届け、終端（`End`/`Quit`）で `TalkDone` を返して消える per-talk transient である。上流契約（`sakura-parse` の `Instruction` モデル・`dola` タイミング層・`areka-actor` 規約・kanade の talk 起動契約）は完了済みまたは並走で先決済みであり、本エンジンはそれらを消費・再定義しない。

## Introduction

本仕様は areka M1 の再生系エンジン ④ sakura（さくらスクリプト再生エンジン）を定義する。sakura は kanade（③ conductor）から `StartTalk{script, talk_id}` を受けると、上流パーサ `areka_parsers::sakura::parse` で得た `Instruction` 列をタイムライン（時刻付き発火列）へ展開し、待ち命令（`Wait`）を反映した時間軸で駆動する。駆動の結果、surface 指令とテキスト系指令の 2 系統の発火列を下流の別々の消費者（seriko・emo text-layer）へ届け、終端命令（`End`/`Quit`）で `TalkDone{talk_id, quit}` を kanade へ返して自己を破棄する（per-talk transient）。

本仕様の中核的検証は表示や実 kanade を伴わず、fixture script（emo2 boot 級: text＋`\s`＋`\w`＋`\e`）を script 直入力し、2 本の mock sink（surface 指令用・テキスト系用）へ届く発火列・発火時刻・終端信号が期待どおりであることを、時刻注入により決定的に観測することで達成する。

本仕様は、時刻注入による決定的テスト、寛容・非パニックのパーサ流儀、ログ無し失敗経路の禁止という areka のランタイム規律に従う。

## Boundary Context

- **In scope**:
  - talk timeline 再生: `Instruction` 列を時刻付き発火列へ展開し、`Wait` を時間軸へ反映する。
  - 下流 2 分岐の出力契約（本仕様が正本）: surface 指令系（→seriko）とテキスト系指令（→emo text-layer）、および `TalkDone`（→kanade）。
  - 終端と中断: `End`→`TalkDone{quit:false}`／`Quit`→`TalkDone{quit:true}`、kanade からの Close による即時中断。
  - M-boot 外タグの寛容な無視（ログ）＋型シーム。
  - per-talk transient の生成・破棄。
  - mock sink 2 本による観測ハーネス。
- **Out of scope**:
  - script の字句解析・`Instruction` 化（**sakura-parse** が完了済み・本仕様は再パースしない）。
  - surface id・alias の解釈（`sakura.surface.alias` 含む。**seriko / emo** の責務。`SurfaceArg` は不透明のまま渡す）。
  - typewriter の字送り間隔・グリフ描画・テキストレイアウト（**emo text-layer** の責務）。
  - `Choice`/`Move`/`Cursor`/`SystemVar`/`GenericCommand`/`Raw` の**実挙動**（**sakura-dialogue-tags**・M-dialogue。本仕様では受けて無視＋シームのみ）。
  - talk の選定・スケジューリング・boot/close 運行表（**kanade**）。
- **Adjacent expectations**:
  - `StartTalk{script, talk_id}` の受領契約と `TalkDone{talk_id, quit}` の返信契約は **kanade brief が正本**である。本仕様はこの型を消費し、再定義しない。
  - 入力の `Instruction` モデル（フラット enum・値正規化済み）は **`areka_parsers::sakura`（sakura-parse）が正本**である。本仕様は再パースを行わない。
  - タイミング層は **`dola`**（時刻注入式 `tick(current_time)`）が正本方針である。時間軸展開を dola 経由とするか自前 sequencer とするかは design 判断であり、本仕様は user/operator 観測可能な時間軸挙動のみを規定する。
  - アクター通信規約（inbox・envelope・停止＝Close 即時停止／積み残し破棄・handler Err はログして継続）は **`areka-actor`** が正本である。
  - 下流の出力契約（`SurfaceCommand` 級／`TextCommand` 級／`TalkDone`）は seriko・emo text-layer・kanade が消費するため、本仕様がその意味論（scope・発火時刻 `at` を含む）の正本となる。

## Requirements

### Requirement 1: talk 起動と script 受領

**Objective:** conductor（kanade）として、script 文字列を渡して 1 回の talk 再生を起動したい。これにより SHIORI が返した Value を発話として時間軸上に流せる。

#### Acceptance Criteria

1. When sakura が `StartTalk{script, talk_id}` 相当の talk 起動要求を受領したとき、the sakura engine shall その `script` を上流 `Instruction` モデルへ変換して当該 talk の再生対象とする。
2. When 再生対象の `Instruction` 列を確定するとき、the sakura engine shall 上流パーサ `areka_parsers::sakura::parse` を用い、独自の字句解析や再パースを行わない。
3. The sakura engine shall 受領した `talk_id` を、当該 talk が発火・終端で送出する全出力（surface 指令・テキスト系指令・`TalkDone`）に対応付ける。
4. If `script` が空、または `Instruction` 列が空になったとき、the sakura engine shall 時間軸再生を行わずに当該 talk を正常終端として扱い、`TalkDone{quit:false}` 相当の終端信号を返す。

### Requirement 2: タイムライン展開（Instruction → 時刻付き発火列）

**Objective:** 開発者として、`Instruction` 列を待ち命令を反映した時刻付き発火列へ純粋に展開したい。これにより再生の時間軸挙動を実時間 sleep に依存せず単体で検証できる。

#### Acceptance Criteria

1. When `Instruction` 列を展開するとき、the sakura engine shall 各出力発火に talk 起点からの相対時刻を意味論込みで付与する（出力契約の `at`）。
2. When `Instruction::Wait(duration)` を処理するとき、the sakura engine shall それ以降の発火の相対時刻に当該 `duration` を累積オフセットとして加算する。
3. The sakura engine shall `Instruction::Wait` が保持する `Duration` を待ち時間の唯一の真実として用い、`\w[n]`（n×50ms）や `\_w[ms]` の実時間換算を再計算しない（換算は上流 sakura-parse で正規化済み）。
4. While 複数の `Wait` が列中に現れるとき、the sakura engine shall それらを出現順に累積し、後続発火の相対時刻へ単調非減少に反映する。
5. The sakura engine shall タイムライン展開を決定的（同一 `Instruction` 列に対し同一の時刻付き発火列）に行う。

### Requirement 3: surface 指令の下流分岐（→seriko）

**Objective:** 下流 seriko の消費者として、surface 切替指令を不透明な引数のまま所定の時刻で受け取りたい。これにより id 解決・alias 解釈を自分の層で行える。

#### Acceptance Criteria

1. When `Instruction::Surface(surface_arg)` を処理するとき、the sakura engine shall surface 指令系の出力へ、当該 `SurfaceArg` と現在の話者スコープと発火時刻 `at` を含む発火を届ける。
2. The sakura engine shall `SurfaceArg` の中身を解釈・変換・alias 解決せず、不透明のまま下流へ渡す。
3. The sakura engine shall surface 指令を、テキスト系指令とは別の出力系統（別 sink）へ届ける。

### Requirement 4: テキスト系指令の下流分岐（→emo text-layer）

**Objective:** 下流 emo text-layer の消費者として、テキスト・改行・クリアの各指令を所定の時刻で受け取りたい。これにより typewriter 字送りとグリフ描画を自分の層で行える。

#### Acceptance Criteria

1. When `Instruction::Text(text)` を処理するとき、the sakura engine shall テキスト系の出力へ、当該テキストと現在の話者スコープと発火時刻 `at` を含む発火を届ける。
2. When `Instruction::NewLine(ratio)` を処理するとき、the sakura engine shall テキスト系の出力へ改行指令（比率と発火時刻 `at` を含む）を届ける。
3. When `Instruction::Clear` を処理するとき、the sakura engine shall テキスト系の出力へクリア指令（発火時刻 `at` を含む）を届ける。
4. The sakura engine shall typewriter の字送り間隔・グリフ描画・テキストレイアウトを自身で行わず、テキストの供給開始（どのテキストをどの時刻に供給するか）までを担う。

### Requirement 5: 話者スコープの共通付与

**Objective:** 下流の両消費者として、各発火がどの話者スコープに属するかを知りたい。これによりサーフェスとテキストを正しいキャラクタへ結びつけられる。

#### Acceptance Criteria

1. When `Instruction::SpeakerScope{n}` を処理するとき、the sakura engine shall 以降の発火に適用する現在の話者スコープを `n` に更新する。
2. The sakura engine shall surface 指令系・テキスト系のいずれの発火にも、その発火時点で有効な話者スコープを付与する。
3. While 話者スコープが未指定のまま talk が開始したとき、the sakura engine shall 既定の話者スコープを有効なスコープとして各発火に付与する。

### Requirement 6: 終端の検出と TalkDone 返信

**Objective:** conductor（kanade）として、talk が終端に達したこと、および通常終了か quit かの区別を確実に受け取りたい。これにより close 握手や次の運行判断を行える。

#### Acceptance Criteria

1. When `Instruction::End` を処理するとき、the sakura engine shall 当該 talk の再生を終端し、`TalkDone{talk_id, quit:false}` 相当の終端信号を返す。
2. When `Instruction::Quit` を処理するとき、the sakura engine shall 当該 talk の再生を終端し、`TalkDone{talk_id, quit:true}` 相当の終端信号を返す。
3. When `Instruction` 列を終端命令なしに末尾まで再生し終えたとき、the sakura engine shall 当該 talk を通常終了として終端し、`TalkDone{talk_id, quit:false}` 相当の終端信号を返す。
4. The sakura engine shall 1 回の talk につき `TalkDone` を高々 1 回だけ返す。
5. When 終端命令（`End` または `Quit`）以降に後続 `Instruction` が存在するとき、the sakura engine shall 終端以降の命令を発火せず破棄する。

### Requirement 7: kanade からの中断（Close）

**Objective:** conductor（kanade）として、再生途中の talk を即座に打ち切りたい。これにより close 時やゴースト切替時に積み残しを残さず停止できる。

#### Acceptance Criteria

1. When sakura が中断（Close 相当）を受領したとき、the sakura engine shall 進行中の再生を即時停止する。
2. When 中断により再生を停止するとき、the sakura engine shall 未発火の残余 `Instruction` を drain せず破棄する。
3. The sakura engine shall 中断による停止を、areka-actor の停止規約（Close 即時停止・積み残し破棄）に整合させる。

### Requirement 8: M-boot 外タグの寛容な無視とシーム

**Objective:** 開発者として、M-boot で実挙動を持たないタグを安全に受け流したい。これにより未対応タグを含む script でも再生が破綻せず、後続 M-dialogue で実挙動を追加する拡張余地を残せる。

#### Acceptance Criteria

1. When `Instruction::Choice`／`Move`／`Cursor`／`SystemVar`／`GenericCommand`／`Raw` のいずれかを処理するとき、the sakura engine shall 当該命令に対する実挙動を伴わずに再生を継続する。
2. When M-boot 外タグを無視するとき、the sakura engine shall 無視した事実をログ（`tracing` 相当）に記録する。
3. The sakura engine shall M-boot 外タグを、後続で実挙動を追加できる型シーム（拡張点）として扱い、当該タグの存在によって panic しない。

### Requirement 9: 決定的テストと観測ハーネス

**Objective:** 開発者として、表示や実 kanade を伴わず、fixture script の再生を単一 pass/fail で決定的に観測したい。これにより再生の正しさを実時間 sleep なしに証明できる。

#### Acceptance Criteria

1. The sakura engine shall 時間軸の進行を注入された時刻に基づいて駆動し、実時間の sleep に依存しない。
2. Where 観測ハーネスが与えられるとき、the sakura engine shall surface 指令系とテキスト系の 2 本の mock sink へ、それぞれの発火列と発火時刻を届ける。
3. When fixture script（text＋`\s`＋`\w`＋`\e` を含む emo2 boot 級）を script 直入力し注入時刻を進めるとき、the sakura engine shall 期待どおりの発火列・発火時刻（`\w[n]` の待ちが時間軸に反映されたもの）を各 mock sink へ届け、終端で正しい `TalkDone{quit}` を返す。
4. The sakura engine shall 同一の fixture script と同一の注入時刻列に対し、同一の観測結果（発火列・発火時刻・終端信号）を返す。

### Requirement 10: per-talk transient のライフサイクル

**Objective:** conductor（kanade）として、talk ごとに sakura が生まれて終端で消えることを期待したい。これにより talk 間で状態が漏れず、都度クリーンな再生が保証される。

#### Acceptance Criteria

1. The sakura engine shall 1 回の talk 再生を 1 つの transient な再生単位として扱い、talk 起動時に生成し終端時に破棄する。
2. When ある talk が終端（`End`/`Quit`/末尾到達）または中断（Close）に達したとき、the sakura engine shall その talk に属する再生状態（累積時刻・話者スコープ等）を破棄する。
3. The sakura engine shall ある talk の再生状態を後続の別 talk へ持ち越さない。

### Requirement 11: 失敗経路のログ規律と非パニック

**Objective:** 運用者として、再生中の失敗が黙って握り潰されないことを保証したい。これにより不具合の観測可能性を確保しつつ、通常の入力異常で再生が落ちないようにできる。

#### Acceptance Criteria

1. If 再生中に回復可能な失敗（下流 sink への送出失敗・想定外だが継続可能な入力等）が発生したとき、the sakura engine shall 当該失敗を error ログに記録し、可能な範囲で再生を継続または当該 talk を観測可能な状態遷移で終端する。
2. The sakura engine shall 通常の入力異常（未対応タグ・不正な引数を含む `Instruction`）に対して panic せず、寛容に受け流す。
3. If panic を伴う致命的状態に至るとき、the sakura engine shall panic の直前に原因を error ログへ記録する。
4. The sakura engine shall 失敗を黙って握り潰すログ無しの失敗経路を持たない。

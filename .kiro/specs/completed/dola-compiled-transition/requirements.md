# Requirements Document

## Introduction

dolaクレートのStoryboard / TransitionDef / EasingFunctionなどの宣言的定義を、ランタイムが直接消費できる「コンパイル済みトランジション」データ構造にコンパイルする機能を定義する。ストーリーボードに開始時刻（f64秒）を与えることで、値（変数）ごとに時刻範囲・値範囲・イージング関数の連続セグメントを平坦化し、ランタイム側がキーフレームDAG解決やトランジション参照解決を行わずに済むデータを生成する。

ランタイムの再生エンジン自体はスコープ外。ランタイムに渡すコンパイル済み情報の **生成** が本仕様の目的である。ただし、ランタイムが必要とする「割り切り情報」（ループ展開方針、time_scale適用方法、Object型変数の扱い等）もコンパイル結果に含める。

### 参考

- プロジェクト説明: dora定義を値別にコンパイルしたCompiledTransition（名前は暫定）の定義
- 参考議論: https://claude.ai/share/fb7707ad-da54-45b9-be43-038eed693949

## Requirements

### Requirement 1: コンパイル済みトランジション構造体の定義

**Objective:** dolaクレートの開発者として、ストーリーボードの宣言的定義から変数ごとのランタイム消費用データ構造を生成したい。これにより、ランタイムはキーフレーム解決やトランジション参照解決を行わずにアニメーション再生に集中できる。

#### Acceptance Criteria

1. The Dola Compiler shall 変数名をキーとしたコンパイル済みトランジションのマップ（`BTreeMap<String, CompiledVariableTimeline>` 等）をコンパイル結果として生成する
2. The Dola Compiler shall 各変数のタイムラインを、セグメント（`CompiledSegment`）の配列として表現する。セグメント間に時間的ギャップがあってもよい（ランタイムはギャップ区間において直前セグメントの終了値を保持する）
3. The Dola Compiler shall 各セグメントに開始時刻（f64秒）、終了時刻（f64秒）、開始値、終了値、イージング関数を含める
4. The Dola Compiler shall Object型変数のセグメントについて、イージングなしの即時切り替え情報を含める（値域はDynamicValueを使用）
5. The Dola Compiler shall コンパイル結果のルート構造体に、元のストーリーボード名と生成元のメタ情報を含める

### Requirement 2: 開始時刻を起点とした時刻解決

**Objective:** dolaクレートの開発者として、ストーリーボードに開始時刻（f64秒）を与えることで全エントリの絶対時刻を解決したい。これにより、キーフレームDAGの複雑な時刻計算をコンパイル時に済ませられる。

#### Acceptance Criteria

1. When 開始時刻（f64秒）がコンパイル関数に渡された場合, the Dola Compiler shall すべてのセグメントの開始・終了時刻を開始時刻からの絶対時刻として計算する
2. When エントリにdelayが指定されている場合, the Dola Compiler shall delay時間を開始時刻に加算してセグメント開始時刻を算出する
3. When エントリにat（キーフレーム起点）が指定されている場合, the Dola Compiler shall 参照先キーフレームの時刻を解決し、offsetがあればそれを加算した時刻を起点とする
4. When エントリにbetween（キーフレーム間配置）が指定されている場合, the Dola Compiler shall from/toキーフレーム間の時間範囲にトランジションを配置する
5. When 前エントリ連結（at/betweenなし）の場合, the Dola Compiler shall 同一変数の直前セグメント終了時刻を開始時刻として使用する。ただし、その変数の最初のエントリである場合はストーリーボードの開始時刻（コンパイル関数に渡された start_time + delay）を使用する

### Requirement 3: トランジション定義の解決と平坦化

**Objective:** dolaクレートの開発者として、TransitionRef（Named/Inline）を解決し、各セグメントの値範囲とイージングを確定したい。これにより、ランタイムはテンプレート参照の解決ロジックを持つ必要がなくなる。

#### Acceptance Criteria

1. When TransitionRef::Named が指定されている場合, the Dola Compiler shall DolaDocumentのtransitionマップから定義を解決し、インライン定義と同等にコンパイルする
2. When TransitionDefにfromが省略されている場合, the Dola Compiler shall 同一変数の直前セグメント終了値（なければ変数の初期値）をfromとして採用する
3. When TransitionDefにrelative_toが指定されている場合, the Dola Compiler shall from値にrelative_toを加算した値をtoとして算出する
4. The Dola Compiler shall EasingFunction（Named/Parametric）をセグメントにそのまま保持する（ランタイムが評価関数として使用）
5. When durationが省略されている場合, the Dola Compiler shall 即時遷移（duration = 0）として、開始時刻と終了時刻が等しいセグメントを生成する

### Requirement 4: time_scaleとループ情報の伝達

**Objective:** dolaクレートの開発者として、ストーリーボードのtime_scaleやloop_countなどのメタ情報をコンパイル結果に含めたい。これにより、ランタイムは再生速度制御やループ処理に必要な情報を参照できる。

#### Acceptance Criteria

1. The Dola Compiler shall コンパイル結果にtime_scale値を含める（ランタイムがセグメント時刻に乗算して使用する想定）
2. The Dola Compiler shall コンパイル結果にloop_count情報を含める（None=ループなし、Some(0)=無限、Some(n)=n回）
3. The Dola Compiler shall コンパイル結果にinterruption_policyを含める（ランタイムの競合解決用）
4. The Dola Compiler shall time_scaleはセグメント時刻に事前適用 **しない** こと（ランタイム側でリアルタイム適用する設計のため）

### Requirement 5: 割り切り情報（ランタイムヒント）

**Objective:** dolaクレートの開発者として、ランタイムが効率的に再生を実行するための「割り切り情報」をコンパイル結果に含めたい。これにより、ランタイム実装者が判断に必要な情報を明示的に参照できる。

#### Acceptance Criteria

1. The Dola Compiler shall 各変数タイムラインに変数の型情報（f64 / i64 / object）をヒントとして含める
2. Where i64型変数の場合, the Dola Compiler shall 補間後のi64丸め処理が必要である旨をヒントとして含める
3. Where Object型変数の場合, the Dola Compiler shall 「補間なし・即時切り替えのみ」である旨をヒントとして含める
4. Where i64型変数にtypewriterが指定されている場合, the Dola Compiler shall typewriter文字列情報をヒントとして含める（ランタイムがインデックスから部分文字列を取得するため）
5. The Dola Compiler shall コンパイル済みタイムラインの合計再生時間（time_scale未適用のベース時間）を算出して含める
6. Where 変数にmin/maxが定義されている場合, the Dola Compiler shall 値域制約情報をヒントとして含める（ランタイムのクランプ処理用）

### Requirement 6: コンパイルエラーハンドリング

**Objective:** dolaクレートの開発者として、コンパイル時にエラーが検出された場合、既存のDolaErrorと一貫した形式でエラーを報告したい。これにより、バリデーションエラーとコンパイルエラーを統一的に扱える。

#### Acceptance Criteria

1. The Dola Compiler shall コンパイル関数の冒頭で既存の `Validate` トレイトによるバリデーション（`doc.validate()`）を内部呼び出しし、バリデーションエラーがあればそのまま返す（呼び出し側が validate を忘れるリスクをゼロにする設計。二重呼び出しのコストは無視できる）
2. If キーフレーム参照が循環依存している場合, the Dola Compiler shall コンパイル固有エラーを返す（既存バリデーションではカバーされない）
3. The Dola Compiler shall コンパイル固有エラーを既存の `DolaError` enum に新バリアントとして追加する

### Requirement 7: コンパイルAPIの設計

**Objective:** dolaクレートの利用者として、シンプルなAPIでストーリーボードをコンパイルしたい。これにより、ランタイム統合側が最小限のコードでコンパイル結果を取得できる。

#### Acceptance Criteria

1. The Dola Compiler shall `DolaDocument`と対象ストーリーボード名と開始時刻を引数に取るコンパイル関数を提供する
2. The Dola Compiler shall コンパイル関数の戻り値を`Result<CompiledStoryboard, Vec<DolaError>>`（またはそれに準ずる型）とする
3. The Dola Compiler shall コンパイル結果の型および関数をdolaクレートのpublic APIとしてエクスポートする
4. The Dola Compiler shall コンパイル結果の型にSerialize/Deserializeを導出する（キャッシュやデバッグ出力に対応するため）

## スコープ外事項

- ランタイム再生エンジン（時刻に基づく値の補間計算）
- ランタイムのイージング関数評価ロジック
- ランタイムのloop展開実行（loop_count情報は含めるが、展開自体はランタイムの責務）
- ランタイムのinterruption_policy競合解決ロジック
- Windows Animation Manager (WAM) との統合コード

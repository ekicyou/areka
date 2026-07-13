# Requirements Document

## Introduction

さくらスクリプトの**テキスト再生には時間がかかる**（1文字あたりの暗黙ノミナルウェイト＋明示 `\_w[ms]` 精密ウェイト）。ところが areka の cue タイムラインは**この再生時間を一切モデル化していない**（テキスト cue は「点」＝0 時間扱い）。結果、テキストを喋り終わる前に後続 cue（次テキスト・`\s` 表情切替・`\n` 改行）が発火し、`areka-P0-emo2-boot` R9.3 実機サインオフで以下の綻びとなって現れた:

- **#3 ウェイト不発**: `\_w[ms]` が typewriter に pause として効いて見えない。
- **#4 改行早発**: 1行表示直後に `\n` の直前 `\_w` が無視されて必ず改行される（#3 と同一根）。
- **#6 新 talk で前会話が消えない**: 新しい talk が始まっても前 talk のテキストがバルーンに残り累積する。
- **副次: `\s` 表情非同期**: 表情切替が喋りと無関係なタイミングで発火する。

根本病理は「文字数→文字ウェイト量」を計算するロジックが複数箇所（emo-text・wintf typewriter）で独自実装され、タイムライン（cue 発火時刻）と reveal（typewriter 表示）が協調しないこと。**単一の権威が「このテキストの再生には XXX 秒かかる」を保持していない。**

本 spec は開発者決定の**案A（duration 付き cue の三権分立）**で解決する。**dola が単一の権威台本（保持）**・**台本を書くのが sakura（計算）**・**台本に従うのが emo-text（服従）**。決定論（注入時刻 `talk_time` 駆動・実時間 sleep/`Instant` 不使用）は維持する。対象は実機7件のうち **#3・#4・#6 と `\s` 表情同期**。

## Boundary Context

- **In scope**:
  - cue タイムラインへの**テキスト再生 duration の第一級モデル化**（dola が「再生に D 秒かかる」を保持し後続 cue を整列）。
  - **単一の純関数**「テキスト→再生時間」（暗黙 per-char ノミナル＋明示 `\_w` 換算）と char_wait 定数の一元化（所在は sakura）。
  - sakura compile が各テキスト cue へ再生時間を付与し、後続 cue の発火時刻を整列。
  - sakura compile が talk 台本の冒頭へ `Clear` cue を前置（#6・新 talk＝バルーン自動クリア）。
  - emo-text reveal が**渡された再生時間に服従**（自前 char_wait 計算を撤去）。
  - 実機受け入れ: #3（`\_w` が pause として体感）・#4（`\n` が `\_w` 分だけ遅れる）・#6（新 talk で前会話が消える）・`\s` 表情同期。
- **Out of scope**:
  - bind/mayuna 合成による表情変化（#2＝`mayuna-compose`）。
  - 実行時サーフェスサイズ変化→窓リサイズ/再吸着（#1＝`surface-resize-resnap`）。
  - テキストの**レイアウト/描画**そのもの（縦書き・折返し・フォントメトリクス）＝emo-text の既存領分。本 spec は**時間の権威のみ**扱い描画に触れない。
  - 選択肢・対話タグ（M-dialogue）。
  - wintf `Typewriter` widget の**完全統合**（第3の独自 char_wait 実装だが areka バルーンは emo-text ゆえ実行経路外）。
  - ユーザーによる文字送り速度設定 UI（M2 送り・本 spec は単一の既定 char_wait 定数で足る）。
  - #7 冒頭 1.5行空行（pasta 生成癖＝上流 `ekicyou/pasta` へ起票済み）。
- **Adjacent expectations**:
  - `areka-P0-mayuna-compose`（#2 bind）は dola `CueCommand`／sakura `compile`／emo-text `apply_cue` を本 spec と共有する。**本 spec が既存テキスト cue へ再生時間を付与する cue モデル形を先に確定**し、mayuna の新 bind cue（瞬時＝再生時間 0）はその確定形へ additive に載る（`CueCommand` は enum を additive 拡張した実績あり）。
  - emo-text の **Clear 機構は既存**（Clear cue で当該スコープの表示状態を消去）。本 spec は新 talk 冒頭で Clear cue を発火する配線と、それを honor する reveal 側の協調を担う。
  - 実機起動は**絶対パス必須**（相対パスだと helper が pasta.dll を LoadLibrary できず MOD_NOT_FOUND）。

## Requirements

### Requirement 1: 絶対時刻台本の保持と同期配送（保持＝dola）
**Objective:** 演出タイミング基盤 dola として、各 cue の発火時刻を絶対時刻として保持し、テキスト cue の再生時間 D を配送データとして運びたい。そうすれば、同一の台本を複数の独立した最終表現者（プロセス境界を跨ぐ場合を含む）へ手渡しても、表現者同士が協調せずとも全員が同一の絶対時刻でイベントを発火する——これこそ dola が「単一の権威台本」である所以。整列そのものは台本の絶対時刻として上流（sakura）が焼き込み、dola はそれを忠実に保持・配送する。

#### Acceptance Criteria
1. When テキスト再生 cue が台本に載る, the dola cue タイムライン shall 当該 cue の再生時間 D を、配送される cue の第一級データとして保持する。
2. The dola cue タイムライン shall 各 cue の発火時刻を絶対時刻（`start_time`）として保持し、当該時刻を上流が確定した通りに変えず配送する。
3. When 同一の台本を複数の独立した最終表現者（プロセス境界を跨ぐ場合を含む）が受け取る, the dola cue タイムライン shall 各表現者へ当該 cue を同一の絶対時刻で配送し、表現者が互いの発火時刻を参照せずとも同期が成立することを保証する。
4. The dola cue タイムライン shall 再生時間 D を不透明な秒数データとして受け取り、SakuraScript 固有の 1文字あたりウェイト値（例: 50ms）をハードコードしない。
5. Where テキスト cue に再生時間が与えられない, the dola cue タイムライン shall 当該 cue を絶対時刻の点として即時配送する（再生時間 0 相当の後方互換）。

### Requirement 2: 文字再生時間を計算する単一の純関数（計算＝sakura）
**Objective:** さくらスクリプトを cue 台本へコンパイルする層 sakura として、「テキスト→再生時間」を計算する純関数を1つだけ持ちたい。そうすればタイムライン側も reveal 側も同じ真実源から再生時間を導出でき、独自実装の重複が絶滅する。

#### Acceptance Criteria
1. The sakura shall テキストの再生時間を「暗黙の per-char ノミナルウェイト＋明示ウェイト（`\_w` 由来）の換算」から算出する**単一の純関数**を提供する。
2. When 同一のテキスト入力が与えられる, the 純関数 shall 実時間・レイアウト・描画状態に依存せず、常に同一の再生時間を返す。
3. The sakura shall 1文字あたりのノミナルウェイト定数を**単一の箇所（sakura）**で定義し、他の層（emo-text 等）で重複定義させない。
4. Where テキストが明示ウェイトを含む, the 純関数 shall 暗黙 per-char ノミナルウェイトに明示ウェイトを加算した再生時間を返す。
5. The 純関数 shall GPU・ウィンドウ・COM に依存せず、入力依存の全分岐を単体テストで網羅できる。

### Requirement 3: compile が再生時間を付与し絶対時刻へ焼き込む（計算＝sakura）
**Objective:** sakura compile として、各テキスト cue へ再生時間 D を第一級データとして付与し、後続 cue の絶対発火時刻へ D を焼き込んで台本を構成したい。そうすれば後続 cue（次テキスト・`\s` 表情・`\n` 改行）がテキストを喋り終わってから発火し、dola はその絶対時刻を忠実に配送するだけで同期が成立する。

#### Acceptance Criteria
1. When さくらスクリプトを cue 台本へ compile する, the sakura compile shall 各テキスト cue に対して Requirement 2 の純関数で算出した再生時間 D を第一級データとして付与する。
2. When テキスト cue の後に別の cue（次テキスト・`\s` 表情切替・`\n` 改行）が続く, the sakura compile shall 後続 cue の絶対発火時刻（`start_time`）を、テキスト cue の発火時刻に再生時間 D を加算した時刻以降へ確定させ、後続 cue がテキスト再生完了後に発火する台本を構成する。
3. While テキストが明示ウェイトを含まない, the sakura compile shall 暗黙 per-char ノミナル再生時間のみをタイムラインへ加算する。
4. The sakura compile shall 明示ウェイト（`\_w`）由来のウェイトを、暗黙 per-char 再生時間に加えて累積し、従来の明示ウェイト累積を退行させない。

### Requirement 4: 台本冒頭 Clear cue の前置（#6・sakura）
**Objective:** sakura compile として、各 talk 台本の冒頭に Clear cue を前置したい。そうすれば新しい talk が空のバルーンから始まり、前 talk のテキストが累積しない（#6）。

#### Acceptance Criteria
1. When 新しい talk 台本を compile する, the sakura compile shall 台本の先頭に、当該 talk が書き込むスコープのバルーンをクリアする Clear cue を前置する。
2. When 新しい talk が再生を開始する, the areka talk 再生パイプライン shall 前 talk のバルーンテキストを表示から取り除き、新しい talk のテキストのみを表示する。
3. While talk が複数のスコープ（`\0`/`\1` 等）へテキストを書く, the sakura compile shall 当該 talk が書き込む各スコープが新 talk 開始時に前 talk の残存テキストを持たないよう Clear を発行する。

### Requirement 5: emo-text reveal が付与された再生時間に服従（服従＝emo-text）
**Objective:** バルーンテキストの reveal 層 emo-text として、自前の char_wait 計算を捨て台本が定めた再生時間に従って文字を出したい。そうすれば reveal がタイムラインの単一真実源と協調する。

#### Acceptance Criteria
1. When テキスト cue を受け取る, the emo-text reveal shall 台本が定めた再生時間に基づいて文字送りのタイミングを決定する。
2. The emo-text reveal shall 独自の per-char ウェイト定数を保持せず、再生時間の真実源から文字送りを導出する。
3. While テキスト cue の再生時間 D と文字数 N が与えられる, the emo-text reveal shall N 文字を概ね D 秒かけて表示する。
4. When Clear cue を受け取る, the emo-text reveal shall 当該スコープの表示テキスト（未表示分を含む）を消去する。

### Requirement 6: 実機受け入れ（#3・#4・#6・`\s` 同期）
**Objective:** 実機サインオフを行う開発者として、#3/#4/#6 と表情同期が実 emo2・実 pasta.dll・実 DPI で観測可能に解消されることを確認したい。そうすれば R9.3 実機欠陥が確定的に閉じる。

#### Acceptance Criteria
1. When 実 emo2 ゴーストを実 pasta.dll・実 DPI で起動し talk を再生する, the areka talk 再生パイプライン shall スクリプトの `\_w[ms]` を pause として体感できる形で反映する（#3）。
2. When 1行を表示した直後の `\n` 改行の直前に `\_w` が置かれている, the areka talk 再生パイプライン shall 改行を `\_w` の時間分だけ遅らせて発火する（#4・改行早発しない）。
3. When 新しい talk が開始する, the areka talk 再生パイプライン shall 前の会話のバルーンテキストを消去し、累積表示しない（#6）。
4. When テキスト再生中に `\s` 表情切替が指定されている, the areka talk 再生パイプライン shall 表情切替を喋りと同期させる（当該テキストの再生完了後に切替）。
5. Where 実機受け入れ検証を実施する, the 検証手順 shall 人間サインオフを要件とし、ゴースト起動を絶対パスで行う（相対パス起動は helper の pasta.dll ロード失敗を招くため）。

### Requirement 7: 決定論・汎用性・ワイヤ互換の維持（非機能）
**Objective:** areka talk 再生パイプラインとして、再生時間モデル化が決定論・dola の汎用性・既存シリアライズ互換を壊さないようにしたい。そうすればテスト可能性と基盤の中立性、既存資産の読み込みが保たれる。

#### Acceptance Criteria
1. The areka talk 再生パイプライン shall 再生時間の算出・整列を注入時刻（`talk_time`）駆動で行い、実時間 sleep・`Instant` を使用しない。
2. The dola cue タイムライン shall SakuraScript 固有の意味論（per-char ウェイト値）を内包せず、再生時間をデータとして受け取る汎用基盤に留まる。
3. When 再生時間モデルを cue モデルへ追加する, the dola cue モデル shall 既存 cue variant のワイヤ形を変えず additive に拡張する。
4. When 再生時間情報を持たない既存のシリアライズ済み cue データを読み込む, the dola cue モデル shall それを従来どおり解釈できる（後方互換）。

### Requirement 8: スコープ境界と着手前提条件
**Objective:** 本 spec の担当範囲を誤読なく限定したい。そうすれば隣接 spec との衝突を避け、既に充足済みの前提条件を確認だけで済ませられる。

#### Acceptance Criteria
1. The 本 spec shall areka バルーン再生経路（emo-text）の再生時間権威のみを担保し、wintf `Typewriter` widget（実行経路外の第3独自実装）を対象外とする（統合可否は設計段階で判断する）。
2. The 本 spec shall テキストのレイアウト・描画（縦書き・折返し・フォントメトリクス）を変更せず、時間の権威のみを扱う。
3. When 本 spec の実装に着手する, the 実装者 shall `punctuation_wait` ハックおよび drive.rs の生スクリプト診断ログの不在を再確認し、万一残存する場合は撤去する。
4. If `\s` 表情同期を超える表情変化（bind/mayuna 合成）や実行時サーフェスリサイズが要求される, then the 本 spec shall それを対象外とし、それぞれ `mayuna-compose`／`surface-resize-resnap` へ委譲する。

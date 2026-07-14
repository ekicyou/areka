# Requirements Document

## Introduction

さくらスクリプトの**テキスト再生には時間がかかる**（1文字あたりの暗黙ノミナルウェイト＋明示 `\_w[ms]` 精密ウェイト）。ところが areka の cue タイムラインは**この再生時間を一切モデル化していない**：テキスト cue は「点」＝0 時間扱い、明示 `\_w` は compile が offset へ吸収して cue を残さない。結果、テキストを喋り終わる前に後続 cue（次テキスト・`\s` 表情切替・`\n` 改行）が発火し、`areka-P0-emo2-boot` R9.3 実機サインオフで以下の綻びとなって現れた:

- **#3 ウェイト不発**: `\_w[ms]` が typewriter に pause として効いて見えない。
- **#4 改行早発**: 1行表示直後に `\n` の直前 `\_w` が無視されて必ず改行される（#3 と同一根）。
- **#6 新 talk で前会話が消えない**: 新しい talk が始まっても前 talk のテキストがバルーンに残り累積する。
- **副次: `\s` 表情非同期**: 表情切替が喋りと無関係なタイミングで発火する。

根本病理は二層ある。第一に「文字数→文字ウェイト量」を計算するロジックが複数箇所（emo-text・wintf typewriter）で独自実装され、タイムライン（cue 発火時刻）と reveal（typewriter 表示）が協調しないこと——**単一の権威が「このテキストの再生には XXX 秒かかる」を保持していない**。第二に、より根源的に、**cue の再生時間（duration）が第一級データとしてタイムラインに載っておらず、純粋な待ち（`\_w`）が compile で吸収されて台本から消え、各表現者がコマンド種別で振り分けられて同一台本を共有しないこと**——**時間同期の土台そのものが欠けている**。

本 spec は開発者決定の**案A（duration 付き cue の三権分立）**を、次の統合モデルで実現する:

- **保持＝dola**: 再生時間 duration を **cue envelope の一律フィールド**として全 presentation cue へ持たせ（瞬時は明示的 0・「フィールド欠落」概念は作らない）、絶対時刻で全表現者へ **broadcast** 配送する。
- **計算＝sakura**: テキスト→暗黙再生時間 D を算出する**単一の純関数**を持ち、compile が各テキスト cue へ D を焼き込みつつ、**純粋な待ち `\_w` を第一級 Wait cue として発行**して台本を自己完結した楽譜にする。
- **服従＝全表現者（emo-text 他）**: **同期契約**——受け取った任意 cue の duration を、その action を処理するか否かに関わらず必ず honor し、協調なしに時間同期を保つ。emo-text は reveal を配送 D に従わせ自前 char_wait を撤去する。

決定論（注入時刻 `talk_time` 駆動・実時間 sleep/`Instant` 不使用）は維持する。対象は実機7件のうち **#3・#4・#6 と `\s` 表情同期**。

## Boundary Context

- **In scope**:
  - cue タイムラインへの**テキスト再生 duration の第一級モデル化**（`Cue` envelope の一律フィールド・全 presentation cue が保持・瞬時は 0）。
  - **`CueSheet` の絶対開始時刻保持**（`absolute_start_time`・dispatch 刻印）による**自己完結した絶対時刻台本**——台本のみから各 cue の絶対発火時刻と talk 絶対終了時刻を復元可能にし、cue を区間 `[start_time, start_time+duration)` として扱い、完了を「配送し終えた」でなく「再生し終えた（絶対終了時刻到達）」で判定する。
  - **cue 再生エンジンの層統合**（討議 #2）: cue 再生の"制御"（`CueSheet→schedule` 変換・再生状態機械・完了・バリア・broadcast）を dola（アニメ統括層）へ一本化し、変換 1 本・`CuePlayer` 受動ランタイム・`CueSink` 1 本へ集約。旧世代 wintf `ecs/cue`（`compile_sheet` 含む）を撤去し、sakura は front-end＋talk glue に縮小する（同じロジックの似て非なる版の散在＝車輪の再発明を根絶）。
  - **単一の純関数**「テキスト→暗黙 per-char 再生時間」（所在は sakura）。明示 `\_w` は parser が分離済みゆえ compile のタイムライン累積が合成を担う。
  - sakura compile が各テキスト cue へ再生時間を付与し、後続 cue の絶対発火時刻を整列。
  - **純粋 Wait cue の第一級発行**（`\_w`→`CueCommand::Wait`・action 空・duration のみ）＝台本の自己完結（末尾・単独の待ちも cue として台本に残る）。
  - sakura compile が talk 台本の冒頭へ `Clear` cue を前置（#6・新 talk＝バルーン自動クリア）。
  - **全 cue の broadcast 配送＋全表現者の duration honor 契約**（コマンド種別で特定表現者から cue を隠さず、無視は表現者側の relevance 判定で行う）。
  - emo-text reveal が**渡された再生時間に服従**（自前 char_wait 計算を撤去）。
  - 実機受け入れ: #3（`\_w` が pause として体感）・#4（`\n` が `\_w` 分だけ遅れる）・#6（新 talk で前会話が消える）・`\s` 表情同期。
- **Out of scope**:
  - bind/mayuna 合成による表情変化（#2＝`mayuna-compose`）。
  - 実行時サーフェスサイズ変化→窓リサイズ/再吸着（#1＝`surface-resize-resnap`）。
  - テキストの**レイアウト/描画**そのもの（縦書き・折返し・フォントメトリクス）＝emo-text の既存領分。本 spec は**時間の権威のみ**扱い描画に触れない。
  - 選択肢・一時停止・対話タグ（M-dialogue）の**動的制御フロー**。**dola の表現範囲は「絶対開始時刻＋累積時間」の静的タイムラインに限られる**ため、一時停止（`\x`）・選択肢（`\q`）等の Barrier シームからの再開は、オーケストレーター（kanade/sakura）が新たな絶対開始時刻を調停し台本を再配信することで達成する（dola へ pause/resume 状態を持ち込まない）。本 spec の再生時間 D は単一台本内の単調累積で表現され、この境界の内側に収まる。
  - **Barrier / Routing ペイロードの duration 化**: これらは presentation cue でなく（`Barrier`＝動的停止点・静的タイムライン外／`Routing`＝制御プレーンで `ready()` に届かず表現者へ配送されない）、**duration が本質的に非該当**。envelope duration は presentation cue（`CueCommand`）の時間権威のみを対象とする。
  - wintf `Typewriter` widget の**完全統合**（第3の独自 char_wait 実装だが areka バルーンは emo-text ゆえ実行経路外）。
  - ユーザーによる文字送り速度設定 UI（M2 送り・本 spec は単一の既定 char_wait 定数で足る）。
  - #7 冒頭 1.5行空行（pasta 生成癖＝上流 `ekicyou/pasta` へ起票済み）。
  - `\__w`（基準からの累積ウェイト・文字表示時間を差し引く形）と `\C`（追記モード）の対応（現状 parser 未対応・M-dialogue 以降）。
- **Adjacent expectations**:
  - `areka-P0-mayuna-compose`（#2 bind）は dola `CueCommand`／sakura `compile`／emo-text `apply_cue` を本 spec と共有する。**本 spec が確定する「全 cue が envelope duration を持つ」形へ、mayuna の瞬時 bind cue（duration 0）が additive に載る**（`CueCommand` は enum を additive 拡張した実績あり）。
  - emo-text の **Clear 機構は既存**（Clear cue で当該スコープの表示状態を消去）。本 spec は新 talk 冒頭で Clear cue を発火する配線と、それを honor する reveal 側の協調を担う。
  - 実機起動は**絶対パス必須**（相対パスだと helper が pasta.dll を LoadLibrary できず MOD_NOT_FOUND）。

## Requirements

### Requirement 1: 全 cue の再生時間保持と絶対時刻同期配送（保持＝dola）
**Objective:** 演出タイミング基盤 dola として、各 cue の再生時間を envelope の一律データとして保持し、絶対時刻で全表現者へ配送したい。そうすれば、同一台本を複数の独立した最終表現者（プロセス境界を跨ぐ場合を含む）へ手渡しても、表現者同士が協調せずとも全員が同一の絶対時刻でイベントを発火し、かつ「知らないコマンドの時間」も含めて時刻同期を保てる——これこそ dola が「単一の権威台本」である所以。

#### Acceptance Criteria
1. When cue が台本に載る, the dola cue モデル shall 当該 cue の再生時間 duration を **`Cue` envelope の一律フィールド**として保持する（presentation cue はコマンド種別を問わず必ず本フィールドを持つ）。
2. Where cue が時間を占有しない（瞬時コマンド）, the dola cue モデル shall 当該 cue の duration を **明示的な 0 として保持**し、フィールドを欠落させない（「duration フィールドを持たない presentation cue command」という概念を導入しない）。
3. The dola cue モデル shall 各 cue の発火時刻を絶対時刻（`start_time`）として保持し、当該時刻を上流が確定した通りに変えず配送する。
4. When 同一の台本を複数の独立した最終表現者（プロセス境界を跨ぐ場合を含む）が受け取る, the dola cue タイムライン shall 各 cue を全表現者へ同一の絶対時刻で配送し、表現者が互いの発火時刻を参照せずとも同期が成立することを保証する。
5. The dola cue モデル shall 再生時間 duration を不透明な秒数データとして受け取り、SakuraScript 固有の 1文字あたりウェイト値（例: 50ms）をハードコードしない。
6. Where cue のペイロードが Barrier（動的停止点）または Routing（制御プレーン・表現者未配送）である, the dola shall それらを duration 概念の非該当として扱い、静的 duration タイムラインの外に置く（本 spec の duration モデルは presentation cue のみを対象とする）。
7. The dola cue モデル shall `CueSheet` に絶対開始時刻 `absolute_start_time` を保持し、各 cue の相対 `start_time` ＋ duration と併せて、台本のみから全 cue の絶対発火時刻（`absolute_start_time + start_time`）と talk 絶対終了時刻（`absolute_start_time + max(start_time + duration)`）を復元可能にする（自己完結した絶対時刻台本・アンカーは dispatch 時に刻印）。

### Requirement 2: 全表現者の duration honor 契約（配送＝dola／服従＝全表現者）
**Objective:** areka talk 再生パイプラインとして、全 cue を全表現者へ配り、各表現者が自分に無関係なコマンドの duration すら無視しないようにしたい。そうすれば、どの表現者が何を演じるかに関わらず、全員のタイムラインが協調なしに同期する。honor は 2 段で成立する——**葉の表現者**は自分の担当でない cue から新たなローカル遅延を生じさせず（否定的 no-op）、**talk ライフサイクル**は台本の絶対終了時刻まで talk を早期終了させない。

#### Acceptance Criteria
1. The areka talk 再生パイプライン shall 全 cue を全表現者へ broadcast 配送し、コマンド種別による中央振り分けで特定の表現者から cue を隠さない。
2. When 葉の表現者（emo-text/seriko 等の最終表現者）が自身の担当でない cue を受け取る, the 表現者 shall その duration から新たなローカルな遅延を生じさせない（タイミングは焼き込み絶対時刻が担い、二重待ちを生まない・葉としては否定的 no-op 制約）。
3. While 表現者が自身の担当でないコマンドを受け取る, the 表現者 shall その action を無視してよいが、duration は無視してはならない（相対 `start_time` ＋ duration が示す占有区間を自身の整合に用いる）。
4. Where 表現者が自身の担当 cue を選別する, the 表現者 shall 演者側の relevance 判定（自分宛てか否か）で action 対象を決定する（中央 router による事前振り分けに依存しない）。
5. The areka talk ライフサイクル（drive の完了判定）shall talk を、台本から導かれる絶対終了時刻（`CueSheet.absolute_start_time + max(cue.start_time + cue.duration)`）に達するまで完了扱いにせず、最終 cue（末尾 Wait・最終 Text 等）の duration を talk 終端で落とさない（早期終了しない・#3 の実機構）。

### Requirement 3: 文字再生時間を計算する単一の純関数（計算＝sakura）
**Objective:** さくらスクリプトを cue 台本へコンパイルする層 sakura として、「テキスト→暗黙 per-char 再生時間」を計算する純関数を1つだけ持ちたい。そうすればタイムライン側も reveal 側も同じ真実源から再生時間を導出でき、独自実装の重複が絶滅する。

#### Acceptance Criteria
1. The sakura shall テキストの**暗黙 per-char ノミナル再生時間**を算出する**単一の純関数**を提供する。
2. When 同一のテキスト入力が与えられる, the 純関数 shall 実時間・レイアウト・描画状態に依存せず、常に同一の再生時間を返す。
3. The sakura shall 1文字あたりのノミナルウェイト定数を**単一の箇所（sakura）**で定義し、他の層（emo-text 等）で重複定義させない。
4. The 純関数 shall 明示ウェイト（`\_w`）を**畳まない**——明示 `\_w` は parser が `Instruction::Wait` へ分離済みゆえ、暗黙 per-char と明示ウェイトの合成は compile のタイムライン累積（後述 Requirement 4）が担い、純関数への二重計上を避ける。
5. The 純関数 shall GPU・ウィンドウ・COM に依存せず、入力依存の全分岐を単体テストで網羅できる。

### Requirement 4: compile が再生時間を付与し絶対時刻へ焼き込む（計算＝sakura）
**Objective:** sakura compile として、各テキスト cue へ再生時間 D を envelope データとして付与し、後続 cue の絶対発火時刻へ D を焼き込んで台本を構成したい。そうすれば後続 cue（次テキスト・`\s` 表情・`\n` 改行）がテキストを喋り終わってから発火し、dola はその絶対時刻を忠実に配送するだけで同期が成立する。

#### Acceptance Criteria
1. When さくらスクリプトを cue 台本へ compile する, the sakura compile shall 各テキスト cue に対して Requirement 3 の純関数で算出した再生時間 D を envelope duration として付与する。
2. When テキスト cue の後に別の cue が続く, the sakura compile shall 後続 cue の絶対発火時刻（`start_time`）を、テキスト cue の発火時刻に再生時間 D を加算した時刻以降へ確定させ、後続 cue がテキスト再生完了後に発火する台本を構成する。
3. While テキストが明示ウェイトを含まない, the sakura compile shall 暗黙 per-char ノミナル再生時間のみをタイムラインへ加算する。
4. The sakura compile shall 明示ウェイト（`\_w`）由来のウェイトを、暗黙 per-char 再生時間に加えて累積し、従来の明示ウェイト累積を退行させない。

### Requirement 5: 純粋 Wait cue の第一級発行と自己完結台本（計算＝sakura／保持＝dola）
**Objective:** sakura compile として、明示ウェイト `\_w` を offset へ吸収して消すのでなく、**action を持たず duration だけを持つ第一級の Wait cue**として台本へ載せたい。そうすれば台本が自己完結した楽譜になり、末尾・単独の待ちも失われず、また「待ち時間中に演じたい表現者（将来のバルーンスクロール等）」が待ちを cue として観測できる。

#### Acceptance Criteria
1. When compile が明示ウェイト `\_w` を処理する, the sakura compile shall action を持たず duration のみを持つ**第一級 Wait cue**（`CueCommand::Wait`）を当該絶対時刻へ発行し、同時に後続整列のため offset を当該 duration 分だけ進める。
2. The dola cue モデル shall Wait コマンドを、既存 variant のワイヤ形を変えない additive な追加として保持する。
3. When talk 台本を配送する, the areka talk 再生パイプライン shall 純粋な待ち（末尾・単独の待ちを含む）を cue として台本に含め、**台本のみから talk の全時間範囲が復元可能**である（側チャンネルの `end` 情報に依存しない）。
4. When 表現者が Wait cue を受け取る, the 表現者 shall その duration を honor する（Requirement 2）——Wait は action を持たない。

### Requirement 6: 台本冒頭 Clear cue の前置（#6・sakura）
**Objective:** sakura compile として、各 talk 台本の冒頭に Clear cue を前置したい。そうすれば新しい talk が空のバルーンから始まり、前 talk のテキストが累積しない（#6）。

#### Acceptance Criteria
1. When 新しい talk 台本を compile する, the sakura compile shall 台本の先頭に、当該 talk が書き込むスコープのバルーンをクリアする Clear cue を前置する。
2. When 新しい talk が再生を開始する, the areka talk 再生パイプライン shall 前 talk のバルーンテキストを表示から取り除き、新しい talk のテキストのみを表示する。
3. While talk が複数のスコープ（`\0`/`\1` 等）へテキストを書く, the sakura compile shall 当該 talk が書き込む各スコープが新 talk 開始時に前 talk の残存テキストを持たないよう Clear を発行する。

### Requirement 7: emo-text reveal が付与された再生時間に服従（服従＝emo-text）
**Objective:** バルーンテキストの reveal 層 emo-text として、自前の char_wait 計算を捨て台本が定めた再生時間に従って文字を出したい。そうすれば reveal がタイムラインの単一真実源と協調する。

#### Acceptance Criteria
1. When テキスト cue を受け取る, the emo-text reveal shall 台本が定めた再生時間（配送された cue の duration）に基づいて文字送りのタイミングを決定する。
2. The emo-text reveal shall 独自の per-char ウェイト定数を保持せず、再生時間の真実源から文字送りを導出する。
3. While テキスト cue の再生時間 D と文字数 N が与えられる, the emo-text reveal shall N 文字を概ね D 秒かけて表示する。
4. When Clear cue を受け取る, the emo-text reveal shall 当該スコープの表示テキスト（未表示分を含む）を消去する。
5. When emo-text が自身の担当でない cue（Emote・Wait 等）を受け取る, the emo-text shall その action を無視しつつ cue の duration を honor する（Requirement 2 整合）。

### Requirement 8: 実機受け入れ（#3・#4・#6・`\s` 同期）
**Objective:** 実機サインオフを行う開発者として、#3/#4/#6 と表情同期が実 emo2・実 pasta.dll・実 DPI で観測可能に解消されることを確認したい。そうすれば R9.3 実機欠陥が確定的に閉じる。

#### Acceptance Criteria
1. When 実 emo2 ゴーストを実 pasta.dll・実 DPI で起動し talk を再生する, the areka talk 再生パイプライン shall スクリプトの `\_w[ms]` を pause として体感できる形で反映する（#3）。
2. When 1行を表示した直後の `\n` 改行の直前に `\_w` が置かれている, the areka talk 再生パイプライン shall 改行を `\_w` の時間分だけ遅らせて発火する（#4・改行早発しない）。
3. When 新しい talk が開始する, the areka talk 再生パイプライン shall 前の会話のバルーンテキストを消去し、累積表示しない（#6）。
4. When テキスト再生中に `\s` 表情切替が指定されている, the areka talk 再生パイプライン shall 表情切替を喋りと同期させる（当該テキストの再生完了後に切替）。
5. Where 実機受け入れ検証を実施する, the 検証手順 shall 人間サインオフを要件とし、ゴースト起動を絶対パスで行う（相対パス起動は helper の pasta.dll ロード失敗を招くため）。

### Requirement 9: 決定論・汎用性・ワイヤ互換の維持（非機能）
**Objective:** areka talk 再生パイプラインとして、再生時間モデル化が決定論・dola の汎用性・既存シリアライズ互換を壊さないようにしたい。そうすればテスト可能性と基盤の中立性、既存資産の読み込みが保たれる。

#### Acceptance Criteria
1. The areka talk 再生パイプライン shall 再生時間の算出・整列を注入時刻（`talk_time`）駆動で行い、実時間 sleep・`Instant` を使用しない。
2. The dola cue タイムライン shall SakuraScript 固有の意味論（per-char ウェイト値）を内包せず、再生時間をデータとして受け取る汎用基盤に留まる。
3. When 再生時間・Wait・CueSheet 絶対開始時刻を cue モデルへ追加する, the dola cue モデル shall 既存 cue variant のワイヤ形を変えず additive に拡張する（envelope duration および `CueSheet.absolute_start_time` は `#[serde(default)]`＝0 はワイヤ省略・型には常在／Wait は新規 variant の additive 追加）。
4. When 再生時間情報を持たない既存のシリアライズ済み cue データを読み込む, the dola cue モデル shall それを duration=0 として従来どおり解釈できる（後方互換）。
5. The dola cue モデル shall 「duration フィールドを持たない presentation cue command」という概念を導入せず、全 presentation cue が envelope duration を保持する（結果としての 0 と、フィールドの欠落とを峻別し、欠落を作らない）。

### Requirement 10: スコープ境界と着手前提条件
**Objective:** 本 spec の担当範囲を誤読なく限定したい。そうすれば隣接 spec との衝突を避け、既に充足済みの前提条件を確認だけで済ませられる。

#### Acceptance Criteria
1. The 本 spec shall areka バルーン再生経路（emo-text）の再生時間権威と全 cue の broadcast 配送・honor 契約を担保し、wintf `Typewriter` widget（実行経路外の第3独自実装）を対象外とする（統合可否は設計段階で判断する）。
2. The 本 spec shall テキストのレイアウト・描画（縦書き・折返し・フォントメトリクス）を変更せず、時間の権威のみを扱う。
3. When 本 spec の実装に着手する, the 実装者 shall `punctuation_wait` ハックおよび drive.rs の生スクリプト診断ログの不在を再確認し、万一残存する場合は撤去する。
4. If `\s` 表情同期を超える表情変化（bind/mayuna 合成）や実行時サーフェスリサイズが要求される, then the 本 spec shall それを対象外とし、それぞれ `mayuna-compose`／`surface-resize-resnap` へ委譲する。
5. Where 動的制御フロー（一時停止・選択肢・Barrier シームからの再開）が要求される, the 本 spec shall それを dola の外側（オーケストレーターによる絶対開始時刻の再調停・台本再配信）とし、本 spec の静的 duration タイムラインへ持ち込まない。

### Requirement 11: cue 再生エンジンの層統合（1 か所集約・アニメ制御は dola）
**Objective:** areka コードベースの保守者として、cue を時刻再生する"制御"ロジックが複数箇所に似て非なる版で散在する状態を根絶し、正しい層（dola＝アニメ統括層）へ一本化したい。そうすれば車輪の再発明が消え、duration/絶対時刻/完了/broadcast が単一の土台に載る。

#### Acceptance Criteria
1. The dola cue 層 shall `CueSheet → 時刻スケジュール` の正規化を**単一の変換**として提供し、min 正規化で先頭待ちを消す旧 `compile_sheet` と sakura 独自 `to_schedule` の二重実装を廃する（絶対アンカー＋相対 `start_time` を保存・同一 `at` FIFO・horizon 算出）。
2. The dola cue 層 shall cue 再生ランタイム `CuePlayer`（受動的注入時刻オブジェクト: 再生状態機械・完了 horizon・バリア seam・Choice 先積み・全 `CueSink` への broadcast fan-out）を所有し、旧 wintf `CueQueue` と sakura `drive` の配送制御を一本化する（dola はスレッド/channel を持たず、アクター化は上流の責務）。
3. The dola cue 層 shall 演者非依存の出力契約 `CueSink` を**単一トレイト**として提供し、旧 `SurfaceSink`/`TextSink` の 2 トレイト分割を統合する（broadcast＋演者側 relevance ゆえ役割分割不要）。relevance 分類器 `cue_target_of` も dola が単一権威として所有する。
4. The areka-sakura shall SakuraScript front-end（`compile`・`text_playback_duration`）と talk アクター glue（`CuePlayer` を包み注入時刻を送り、完了→`TalkDone` 中継・Close/中断 funnel・バリア再調停）に縮小し、配送・状態機械・完了判定を再実装しない。
5. The 本 spec shall 旧世代 wintf `ecs/cue`（`CueQueue`/`dispatch`/`tracker`/`compile_sheet` 一式）を撤去し、その制御能力を dola `CuePlayer` へ移す（将来 ECS 演者が要る場合は `dola::CueSink` 実装で足り、別 cue エンジンを新設しない）。
6. The seriko/emo-text 各演者 shall `dola::CueSink` を各 1 実装として持ち、演者側 relevance で action・非該当 cue は duration を honor する。

# Requirements Document

## Project Description (Input)

- **困っている人**: areka で動くゴーストの作者と利用者。ゴースト（ぱすたさん・emo2 を含む）は SHIORI リクエストの `Status` ヘッダ（ukadoc `Status [SSP拡張]`＝`ukadoc:spec_shiori3:Status_20_5bSSP_62e1_5f35_5d:1`）を見て、`online`（ネットワーク通信中）や `nouserbreak`（中断の無効化モード中）のあいだ自発の雑談を止める。
- **今の状況**: areka は `Status` の語彙 10 状態を構造として持つが、実際に値を出しているのは `talking` と `choosing` の 2 つだけで、残り 8 つは常に「載せない」に固定されている。そのうち `online`（ネットワーク更新・URL からのインストール）、`nouserbreak`（`\![enter,nouserbreakmode]`）、`balloon(ID群)`（バルーンの表示）の 3 つは、出どころが α までに実在するようになった。にもかかわらず載せていないので、ネットワーク更新の最中などに雑談が割り込みうる（潜在のバグ）。
- **変えたいこと**: 出どころが実在する 3 状態（`online`・`nouserbreak`・`balloon`）を、正典どおりの意味と書式で `Status` に載せる。既存の送り方の約束（カンマ連結・ヘッダの位置・空なら行を出さない）は変えない。出どころがまだ無い 5 状態（`minimizing`・`induction`・`passive`・`timecritical`・`opening`）は載せないまま語彙に残し、持ち主を記録して引き継ぐ。

## Introduction

本 spec は、ゴーストへ送る SHIORI リクエストの `Status` ヘッダに、ukadoc が定める実行状態のうち `online`・`nouserbreak`・`balloon(ID群)` の 3 つを実際の状態から算出して載せる。正典は ukadoc `Status [SSP拡張]` の次の定めである。

- `online`＝「ネットワーク通信中」
- `nouserbreak`＝「`\![enter,nouserbreak]`中」（ukadoc の `Status` 欄の綴り。さくらスクリプトの正しい綴りは `\![enter,nouserbreakmode]`＝`ukadoc:list_sakura_script:_5c_21_5benter_2cnouserbreakmode_5d:1`。その意味は「スクリプト実行中断の無効化モード」で、`\![leave,nouserbreakmode]` で解除する）
- `balloon(ID群)`＝「バルーンが表示状態。キャラクターID=バルーンID の形式で列挙される。複数開いている場合は/区切りで列挙される。例：`balloon(0=2/1=0)`」
- 複数の状態は「カンマでつなげたもの」

areka は SSP の実測ではなく ukadoc の意味を採る。実 SSP は `balloon(0=2,1=0)` のように内側をカンマで区切って送るが、本 spec は ukadoc の例どおり `/` を採る（内側にカンマを使うと、状態どうしを区切るカンマと見分けがつかなくなるため）。

## Boundary Context

- **In scope**:
  - `online`・`nouserbreak`・`balloon(ID群)` を実際の状態から算出して `Status` に載せること。
  - 載せる時機（状態が変わってから反映されるまで）と、ゴーストを切り替えたときに前のゴーストの状態を持ち越さないこと。
  - 出どころがまだ無い 5 状態（`minimizing`・`induction`・`passive`・`timecritical`・`opening`）を「載せない」のまま語彙に残し、持ち主を記録して引き継ぐこと。
  - 上記の決定論的なテストと、送り口を通した観測。今までの値を期待していた既存テストの期待値の更新。
- **Out of scope**:
  - `Status` の語彙の構造と送り方の約束そのもの（`areka-P0-idle-talk` が確立したもの）。本 spec は変えずに使う。
  - `talking`（`areka-P0-idle-talk`）と `choosing`（`areka-P0-choice-select-events`）がいつ載るかの規則。
  - 出どころとなる仕組みの新設: 窓の最小化、`\![enter,inductionmode]`、`\![enter,passivemode]`、`\t`（タイムクリティカルセクション）、入力ボックス等（communicate／input／teach／dialog）。
  - ネットワーク更新・URL からのインストール・中断の無効化モード・バルーン表示そのものの振る舞い（いつ始まりいつ終わるか）の変更。
  - MCP の `get_status` など、`Status` を読む他の消費者の新設。
- **Adjacent expectations**:
  - ネットワーク更新（完了 `areka-P0-network-update`）、URL からのインストール、中断の無効化モード（完了 `areka-P0-balloon-break`）、バルーンの表示状態（完了 `areka-P0-balloon-visibility`）は、いま動いている振る舞いをそのまま出どころとして使う。バルーンの表示については、画面の表示状態を唯一の情報源とし、`Status` のために別の記録を作らない。
  - 同じウェーブ C1 の約束として、`emo-text-file-split` が分ける 6 本（`crates/areka/src/emo2_boot/balloon_visibility.rs`・`crates/areka/src/input_events/balloon.rs`・`crates/areka-emo-text/src/{actor,layout,viewbox,viewbox_draw}.rs`）、モジュールの宣言（`emo2_boot/mod.rs`・`input_events/mod.rs`）、`restart-chain-finalize-stall` の `emo2_boot/frame/drain_resnap.rs` には触らない。これらに及ぶと分かった時点で止めて報告する。
  - ぱすた（emo2 の SHIORI）は `Status` を部分一致で調べ、`online` または `nouserbreak` を含む間は OnSecondChange で雑談を始めない。`balloon(…)` では止めない。本 spec の効果（更新中などに雑談が割り込まない）はこの振る舞いを通して利用者に見える。

## Requirements

### Requirement 1: 送り方の約束を変えない

**Objective:** As a ゴースト作者, I want 状態が増えても `Status` の書式と位置が今までと同じであること, so that 既存のゴーストが同じ読み方のまま新しい状態も読める

#### Acceptance Criteria

1. When 複数の状態が同時に成り立っている場合, the areka shall それらを正典の語彙順（talking・choosing・minimizing・induction・passive・timecritical・nouserbreak・online・opening・balloon）にカンマで連結して 1 つの `Status` の値とする
2. If 成り立っている状態が 1 つも無い場合, the areka shall `Status` の行そのものを出さない（空の値の `Status:` 行を出さない）
3. The areka shall 同じ状態を 1 つの `Status` の値に 2 度以上含めない
4. The areka shall `talking` と `choosing` が載る条件を本 spec の前と同じに保つ
5. The areka shall `Status` の行をリクエストの中の今までと同じ位置に出す

### Requirement 2: online（ネットワーク通信中）

**Objective:** As a ゴースト作者, I want areka がネットワーク通信をしている間 `Status` に `online` が載ること, so that ゴーストが通信中の雑談を控えられる

#### Acceptance Criteria

1. While ネットワーク更新を実行している場合（更新の開始から終了まで）, the areka shall 送るリクエストの `Status` に `online` を含める
2. While URL からのインストールのためにファイルをダウンロードしている場合, the areka shall 送るリクエストの `Status` に `online` を含める
3. When ネットワーク更新またはダウンロードが終わった場合（成功・失敗・中止のいずれでも）, the areka shall その後に送るリクエストの `Status` に、ほかの通信が続いていない限り `online` を含めない
4. While ネットワーク更新とダウンロードが同時に行われている場合, the areka shall `online` を 1 つだけ含める
5. While ネットワーク通信をしていない場合, the areka shall `Status` に `online` を含めない

### Requirement 3: nouserbreak（中断の無効化モード中）

**Objective:** As a ゴースト作者, I want 台本が中断の無効化モードに入っている間 `Status` に `nouserbreak` が載ること, so that ゴーストが areka の実際の受け付け状態と同じ状態を知れる

#### Acceptance Criteria

1. When 再生中のトークで `\![enter,nouserbreakmode]` が実行された場合, the areka shall その後そのトークの間に送るリクエストの `Status` に `nouserbreak` を含める
2. When `\![leave,nouserbreakmode]` が実行された場合, the areka shall その後に送るリクエストの `Status` に `nouserbreak` を含めない
3. When 中断の無効化モードに入ったトークが、`\![leave,nouserbreakmode]` を経ずに終わった、または中断された場合, the areka shall その後に送るリクエストの `Status` に `nouserbreak` を含めない
4. While トークの再生中である場合, the areka shall `Status` の `nouserbreak` の有無を、利用者の操作によるトークの中断を areka が実際に受け付けない区間と一致させる（載っているのに中断できる、または載っていないのに中断できない、が起きない）
5. When ゴーストを切り替えた、または再起動した場合, the areka shall 前のゴーストの中断の無効化モードを新しいゴーストへのリクエストの `Status` に持ち越さない

### Requirement 4: balloon(ID群)（バルーンの表示）

**Objective:** As a ゴースト作者, I want どのキャラクターのどのバルーンが画面に出ているかが `Status` に載ること, so that ゴーストがバルーンの表示状態を正典の書式で知れる

#### Acceptance Criteria

1. While あるキャラクター（`\0`・`\1`・…）のバルーンが画面に表示されている場合, the areka shall `Status` の `balloon(…)` にそのキャラクターの「キャラクターID=バルーンID」を含める
2. The areka shall キャラクターID をスコープの番号（`\0` なら 0、`\1` なら 1）とし、バルーンID を表示中のバルーンの番号（`\b[N]` で選ばれた N。選ばれていなければ 0）とする
3. When 複数のキャラクターのバルーンが同時に表示されている場合, the areka shall 各組を `/` で区切り、キャラクターID の小さい順に並べる（例 `balloon(0=2/1=0)`）
4. If どのキャラクターのバルーンも表示されていない場合, the areka shall `balloon` そのものを `Status` に含めない（`balloon()` のような空の括弧を出さない）
5. When あるキャラクターのバルーンが消えた場合（タイムアウト・`\b[-1]`・消去など理由を問わず）, the areka shall その後に送るリクエストの `Status` にそのキャラクターの組を含めない
6. When 表示中のバルーンの番号が変わった場合, the areka shall その後に送るリクエストの `Status` に新しい番号を載せる
7. The areka shall `Status` が示すバルーンの表示を、利用者の画面に見えているバルーンと食い違わせない（見えていないバルーンを載せない・見えているバルーンを落とさない）
8. When ゴーストを切り替えた、または再起動した場合, the areka shall 前のゴーストのバルーンの表示を新しいゴーストへのリクエストの `Status` に持ち越さない

### Requirement 5: 反映の時機と適用範囲

**Objective:** As a ゴースト作者, I want 状態の変化がすぐに、どのリクエストにも同じ規則で反映されること, so that イベントの種類によって状態の見え方が食い違わない

#### Acceptance Criteria

1. The areka shall `online`・`nouserbreak`・`balloon` を、OnSecondChange に限らず、`Status` を載せるすべてのリクエストに同じ規則で載せる
2. When `online`・`nouserbreak`・`balloon` のいずれかの状態が始まった、または終わった場合, the areka shall その変化を、変化から 1 秒（OnSecondChange の間隔）を超えて遅れずに、以後に送るリクエストの `Status` に反映する

### Requirement 6: 出どころがまだ無い 5 状態

**Objective:** As a 開発者, I want 出どころがまだ無い状態を嘘で載せず、取りこぼさずに引き継ぐこと, so that 正典の残りが宿題として失われない

#### Acceptance Criteria

1. The areka shall `minimizing`・`induction`・`passive`・`timecritical`・`opening(種類)` を `Status` に載せない（areka にそれらの状態がまだ存在しないため）
2. The areka shall この 5 状態の語彙と書式（`opening` の種類を `/` で区切る書式を含む）を、出どころができたときに載せられる形のまま保つ
3. When 本 spec を完了する場合, the 開発の記録 shall この 5 状態と、関連するさくらスクリプト（`\![enter,inductionmode]`・`\![enter,passivemode]`・`\t`）および窓の最小化・入力ボックス等について、本 spec の後の持ち主をロードマップと ukadoc 網羅の対応表に記録する

### Requirement 7: 失敗の記録

**Objective:** As a 開発者, I want 状態を `Status` へ届ける途中の失敗がログに残ること, so that 状態が載らない原因を後から追える

#### Acceptance Criteria

1. If 状態の変化を `Status` を組み立てる側へ届けられなかった場合, the areka shall その失敗をエラーとしてログに記録する
2. The areka shall 送った `Status` の値を、今までどおりリクエストごとにログで追える状態に保つ

### Requirement 8: 検証

**Objective:** As a 開発者, I want 新しい状態の組み合わせと実際の送り口が試験で確かめられていること, so that 書式の崩れや載せ忘れが黙って入り込まない

#### Acceptance Criteria

1. The areka shall `talking`・`choosing`・`nouserbreak`・`online`・`balloon`（表示なし・1 キャラクター・複数キャラクター）のすべての組み合わせについて、`Status` の値（または行を出さないこと）を決定論的なテストで確かめる
2. The areka shall 複合した値（例 `talking,nouserbreak`・`online`・`talking,balloon(0=0)`・`balloon(0=2/1=0)`）が、実際にリクエストを送る口を通ってゴーストへそのまま届くことを試験で確かめる
3. Where 既存の試験が今までの値（例: バルーンが表示されている場面で `talking` だけ）を期待している場合, the areka shall その期待値を正典どおりの値へ改める
4. When `Status` に `online` が載った状態で OnSecondChange が emo2（ぱすた）へ送られた場合, the areka shall emo2 が雑談を始めないことを試験または実機の観測で確かめる

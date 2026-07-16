# Requirements Document

## Introduction

伺かの基本体裁は「放っておくと喋り出す」ことである。emo2 の脳 pasta.dll ではこの自発会話（放置トーク）を毎秒発火の **OnSecondChange** が駆動する（`doc/emo2-conformance-scope.md` §1「最重要・心臓部」）。OnSecondChange が pasta 内部の OnTalk/OnHour/コールバックを内部生成するため、areka は OnSecondChange を毎秒送るだけでよく、`OnTalk`/`OnHour` を自分から送ってはならない（送ると二重発火になる）。

自発会話の背骨（毎秒の pump・応答→トーク起動・talk 中の非割り込み）は既に完成した kanade 運行状態機械に配線済みである。本 spec は新規経路を作らず、その OnSecondChange リクエストを **ukadoc 正典どおりの Reference（Ref0〜3）＋ `Status` 共通ヘッダ**へ充足させ、送信イベント集合が正典の許可集合に留まることを回帰檻で固定し、実機で emo2 を放置したときに自発会話が発火することを人間がサインオフする。

正典は ukadoc（`OnSecondChange`: Reference0=OS 連続起動時間(hour)／Reference1=見切れ／Reference2=重なり／Reference3=トーク再生可能／再生不能時は Reference3=0 で NOTIFY・返却スクリプト無視）。`Status` ヘッダも ukadoc 正典（`Status [SSP拡張]`）に準拠し、ゴーストの実行状態をカンマ連結の状態集合（talking／choosing／minimizing／induction／passive／timecritical／nouserbreak／online／opening(種類)／balloon(ID群)）で表す——M1 は源が既に配線済みの `talking` を実導出し、残状態は語彙に第一級で保持しつつ非アクティブへ縮退＋実測差替シームを持つ（Reference1/Reference2 と同型）。emo2 は最小適合 fixture であって書式の聖典ではない。

## Boundary Context

- **In scope**:
  - OnSecondChange リクエストの Reference 正典充足（Reference0＝実 OS 連続起動時間、Reference1/Reference2＝M1 固定 "0"＋将来の実測差し替えシーム、Reference3＝トーク再生可否）。
  - OnSecondChange への `Status` 共通ヘッダ注入＝ukadoc `Status [SSP拡張]` の実行状態語彙全体（talking／choosing／minimizing／induction／passive／timecritical／nouserbreak／online／opening(種類)／balloon(ID群)）をカンマ連結の第一級状態集合として保持。M1 は `talking` を実導出（源＝運行状態 Steady{talk}）、残状態は語彙保持のまま非アクティブへ縮退＋実測差替シーム（Reference1/Reference2 と同型）。アクティブ集合が空のときはヘッダ行を省略（実 SSP wire 準拠）。
  - 送出 SHIORI イベント ID のホワイトリスト固定と、`OnTalk`/`OnHour` の恒久不送出檻。
  - 応答調停の回帰檻（GET→Value をトーク起動・204 は無起動・NOTIFY 応答スクリプトの破棄・talk 中非割り込み）。
  - 実 emo2・実 pasta.dll での放置→自発会話の実機サインオフ。
- **Out of scope**:
  - 見切れ／重なりの実測値算出（Reference1/2 は M1 固定 "0"・実測は将来増分）。
  - `Status` 残実行状態の**実導出**（源サブシステムからの実値算出）: choosing＝選択肢UI（`areka-P0-choice-select-events`）／balloon・minimizing・induction・passive・timecritical・nouserbreak・online・opening＝各源サブシステム着地時（追跡台帳＝`areka-P0-status-execution-states`）。本 spec は語彙保持＋非アクティブ縮退＋差替シームのみを所有し、残状態の実導出はしない。
  - 入力イベント（OnMouseMove 等）の送出（`input-events` の領分）。
  - トーク再生タイミングの正しさ（`areka-P0-cue-playback-duration` の領分）。
  - `secondchangeinterval` 等の発火間隔設定（プラグイン領分・M1 外）。
  - `OnTalk`/`OnHour` の送出（**恒久禁止**）。
  - Reference4（SSP のみ・OS レベル放置時間・秒）の送出（M1 では扱わない）。
- **Adjacent expectations**:
  - 毎秒 Tick の供給（絶対グリッド整列）は `completed/areka-P0-ghost-setup` の ticker が担う。本 spec は受領した Tick の中身の充足のみを扱い、Tick 供給機構は所有しない。
  - トーク配送・再生（sakura／emo-text／dispatcher）は本 spec の外。実機サインオフは「自発トークが発火する」ことに限定し、再生品質は `areka-P0-cue-playback-duration` の受け入れに帰属する。
  - Reference0 の時刻源は差し替え可能（本番は OS 連続起動時間相当を注入）で供給されることを期待する。

## Requirements

### Requirement 1: OnSecondChange の Reference 正典充足（Reference0〜3）

**Objective:** As an emo2 互換ベースウェアの運用者, I want 毎秒送られる OnSecondChange リクエストが ukadoc 正典どおりの Reference を載せること, so that pasta が正しく自発会話・時報を内部駆動できる

#### Acceptance Criteria

1. While アクティブなトークが無い（定常運転中）, when 毎秒相当の Tick を受領した, the kanade shall OnSecondChange を GET メソッドで送出する。
2. The OnSecondChange リクエスト shall Reference0 に、注入された時刻源から導出した OS 連続起動時間を時単位（ゼロ方向へ切り捨てた 10 進整数の文字列）で載せる。
3. The OnSecondChange リクエスト shall Reference1（見切れ）に文字列 "0" を、Reference2（重なり）に文字列 "0" を載せる（M1 固定値）。
4. While アクティブなトークが無い（トーク再生可能）, the OnSecondChange リクエスト shall Reference3 に文字列 "1" を載せる。
5. While アクティブなトークが再生中（トーク再生不能）, the OnSecondChange リクエスト shall Reference3 に文字列 "0" を載せる。
6. Where 将来の増分で見切れ・重なりの実測値が供給される, the kanade shall OnSecondChange の送出契約（ヘッダ構成・Reference の連番）を変えずに Reference1/Reference2 の値を実測値へ差し替えられる。

### Requirement 2: OnSecondChange の `Status` 共通ヘッダ（ukadoc 実行状態語彙への準拠）

**Objective:** As an emo2 互換ベースウェアの運用者, I want OnSecondChange リクエストの `Status` ヘッダが ukadoc 正典 `Status [SSP拡張]` の実行状態語彙にカンマ連結形式で準拠すること, so that pasta が正典どおりの実行状態情報で発火（自発会話）を制御できる

> 正典典拠: ukadoc `Status [SSP拡張]`（`ukadoc:spec_shiori3:Status_20_5bSSP_62e1_5f35_5d:1`）＝ゴーストの実行状態。複数ある場合はカンマでつなげたもの。語彙 = talking／choosing／minimizing／induction／passive／timecritical／nouserbreak／online／opening(種類)／balloon(ID群)。

#### Acceptance Criteria

1. The `Status` ヘッダ表現 shall ukadoc 正典 `Status [SSP拡張]` の実行状態語彙全体（talking／choosing／minimizing／induction／passive／timecritical／nouserbreak／online／opening(種類)／balloon(ID群)）を第一級の状態集合として保持し、パラメータ付き状態（opening(種類)／balloon(ID群)）の下位書式（`/` 区切り列挙・`charID=balloonID` 等）を表現できる。
2. While 1つ以上の実行状態がアクティブ, the OnSecondChange リクエスト shall アクティブな全状態を正典の語彙名でカンマ（`,`）連結し `Status` ヘッダに載せる。
3. While アクティブな実行状態が1つも無い（アイドル）, the OnSecondChange リクエスト shall `Status` ヘッダ行を一切付与しない（空値でも非 talking 値でもなく、行そのものを省略する＝実 SSP wire 準拠）。
4. While アクティブなトークが再生中, the kanade shall 実行状態 `talking` を運行状態 `Steady{talk}` から実値で導出する。
5. The kanade shall `Status` の各実行状態をそれぞれの権威ある源から導出し、M1 で源サブシステムが未実装の状態（choosing／minimizing／induction／passive／timecritical／nouserbreak／online／opening(種類)／balloon(ID群)）を語彙から除外せず、常に非アクティブとして導出する（Reference1/Reference2 と同型の実測差替シームを各状態に備える）。
6. Where 将来の増分で状態の源サブシステムが供給される, the kanade shall `Status` の送出契約（カンマ連結書式・ヘッダ位置・空集合→行省略）を変えずに当該状態の導出を実値へ差し替えられる。
7. The kanade shall `talking` をトーク再生中に限って送出し、アイドル時に `talking` を送出しない（アイドル時の `Status: talking` は pasta の自発会話を恒久抑制するため・Requirement 6 実機サインオフの前提）。

### Requirement 3: 送出イベント集合のホワイトリスト檻（`OnTalk`/`OnHour` 恒久不送出）

**Objective:** As an emo2 互換ベースウェアの運用者, I want areka が `OnTalk`/`OnHour` を決して送らないこと, so that pasta が OnSecondChange 内部で生成するトーク・時報と二重発火しない

#### Acceptance Criteria

1. The kanade shall SHIORI へ送出し得るイベント ID の集合を確定したホワイトリストに限定する。
2. The kanade shall いかなる運行状態においても `OnTalk` および `OnHour` を送出しない（emo2 が OnSecondChange 内部で自発生成するため・二重発火防止・恒久制約）。
3. When 自発会話（OnSecondChange 応答由来のトーク）が発火する, the kanade shall `OnTalk`/`OnHour` を新たに送出せずに当該トークの再生を開始する。

### Requirement 4: pump 応答の調停（GET→トーク起動・NOTIFY 破棄）

**Objective:** As an emo2 互換ベースウェアの運用者, I want OnSecondChange の応答が正典どおりに調停されること, so that 放置時に喋り出し、トーク中は割り込まれない

#### Acceptance Criteria

1. While アクティブなトークが無い, when OnSecondChange GET の応答がスクリプト（Value）である, the kanade shall そのスクリプトのトーク再生を開始する。
2. While アクティブなトークが無い, when OnSecondChange GET の応答が 204 No Content である, the kanade shall トークを起動せず定常運転を維持する。
3. While アクティブなトークが再生中, the kanade shall OnSecondChange を NOTIFY で送出し、その応答スクリプトを破棄する（新規トークを起動せず、進行中のトークに割り込まない）。
4. When 再生中のトークが完了する, the kanade shall 次の Tick から OnSecondChange の pump（GET）を再開する。

### Requirement 5: 決定論的な検証性

**Objective:** As an areka の開発者, I want idle-talk の全リクエスト生成経路を決定論的に検証できること, so that 回帰を実 sleep や実 32bit helper 無しに檻へ入れられる

#### Acceptance Criteria

1. The idle-talk のリクエスト生成経路 shall 注入された Tick（時刻同梱）と差し替え可能な mock SHIORI のみで、実 sleep・実時計・実 32bit helper を用いずに全経路を検証できる。
2. The OnSecondChange Reference0 の時刻源 shall 注入可能であり、テストが任意の単調ミリ秒値を与えて Reference0 の期待値を決定論的に検証できる。
3. When mock SHIORI が受領したリクエスト列を検査する, the テスト shall 各 OnSecondChange の Method（GET/NOTIFY）・Reference0〜3 の値・`Status` ヘッダの有無と値・および送出されたイベント ID 集合を観測できる。

### Requirement 6: 実機の自発会話サインオフ

**Objective:** As an areka の開発者, I want 実機で放置トークが発火することを人間が確認できること, so that 「放っておくと喋り出す」という伺かの基本体裁が実物で証明される

#### Acceptance Criteria

1. While 実 emo2・実 pasta.dll を起動して数分放置している, the areka ゴースト shall 自発会話（時報系トーク等）を発火させる。
2. While アクティブなトークが再生中, when 次の毎秒 Tick が発生する, the areka ゴースト shall 進行中のトークに割り込まない。
3. The 実機サインオフの合否判定 shall 「自発トークが発火すること」に限定し、トーク再生タイミングの正しさ（`areka-P0-cue-playback-duration` の領分）を判定に含めない。

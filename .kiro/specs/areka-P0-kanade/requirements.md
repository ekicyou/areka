# Requirements Document

## Project Description (Input)

areka-P0-kanade: kanade（③conductor）＝実行時経路（運行表）の所有者。エンジン群（shiori 通信・parser・再生系）が揃っても「いつ・誰に・何を流すか」の運行表＝conductor が存在しない。boot イベントの発火順序・毎秒 pump・SHIORI 応答（Value）の sakura への配送・close 握手——ゴーストを「生かす」中枢を、kanade アクター（独立スレッド・areka-actor 規約）として実現する。boot 運行→毎秒 pump→Value 配送→close 握手を運行表として駆動し、mock shiori アクター＋mock sakura sink による単一 pass/fail の決定的観測（実時間 sleep 非依存・時刻注入）で完了を判定する。talk 起動契約（StartTalk/TalkDone）は本 spec が正本（sakura-engine が消費）。

## Introduction

本仕様は、areka 互換ベースウェアの runtime 制御階層の起点となる conductor エンジン **kanade** を定義する。kanade はゴーストの実行時経路（運行表）を所有し、boot イベント系列の正典順序発火・毎秒 pump（OnSecondChange）・SHIORI 応答スクリプト（Value）の talk 起動配送・close 握手という 4 つの運行を、外部から注入されたメッセージ／Tick に駆動されて決定的に進行させる。SHIORI 実体や script 再生系は境界の向こう側（差し替え可能な相手方）であり、kanade 自身は script を解釈しない。

## Boundary Context

- **In scope**: kanade アクター（inbox メッセージ駆動の運行状態機械）／boot・close 運行表（正典順序・NOTIFY/GET の別・Reference 構成）／OnSecondChange pump（Tick 注入式）／SHIORI 呼出のメッセージ境界化（mock 差し替え可能）／talk 起動契約（StartTalk/TalkDone・本 spec が正本）／mock 観測ハーネス（単一 pass/fail・決定的）。
- **Out of scope**: script の解釈・再生（sakura-engine）／入力イベント配信 OnMouse* 系（input-events・M-life）／自発会話の選定ロジック（idle-talk・M-life）／vanish count・窓位置の永続化（position-persist）／helper 常駐健全性の証明（host32-lifecycle）／SHIORI 自動再起動（M2）。
- **Adjacent expectations**:
  - 死活報告の語彙・型は host32-lifecycle が正本（kanade は消費のみ。lifecycle 完了までは mock 死活で開発）。
  - アクター駆動・停止規約・返信往復の流儀は actor 基盤（areka-actor）が正本（kanade は基盤の消費者であり独自流儀を発明しない）。
  - SHIORI 呼出契約（GET=200 で Value あり／204 で Value なし・NOTIFY・エラー区別語彙）は shiori 通信層（host32-request 成果）が正本。
  - talk 起動契約（StartTalk/TalkDone）は本 spec が正本であり、sakura-engine が消費する（再定義しない）。
  - kanade への結線・boot 指示・close 完了待ちは ghost-setup が担う（kanade は指示の受け手）。
- **design 送り事項**（要件では確定しない・design で ukadoc 正典参照の上確定）: boot 系列・OnSecondChange の Reference 実装値の全確認（Reference 表の作成）／close 再生完了待ちの上限値（de-facto タイムアウト）／talk 重複時（再生中の新規 Value 受領）の調停規則／Tick の供給方式。

## Requirements

### Requirement 1: boot 運行表の駆動

**Objective:** ゴースト運用者として、起動指示ひとつで boot イベント系列が正典順序（ukadoc）どおりに SHIORI へ発火されてほしい。ゴーストが仕様どおりの初期化通知を受け取り、既存伺かゴーストが互換動作できるように。

#### Acceptance Criteria

1. When boot 指示を受領した, the kanade engine shall boot 系列の最初のイベントとして `OnInitialize` を NOTIFY として SHIORI へ発行する。
2. When `OnInitialize` の発行が完了した, the kanade engine shall 起動種別イベント（`OnFirstBoot`〔Ref0=vanish count〕／`OnGhostChanged`／`OnGhostCalled`／`OnVanished` のいずれか該当するもの）を GET として発行する。
3. If 起動種別イベントの応答が 204（Value なし）である, the kanade engine shall 正典のフォールスルー順に従って `OnBoot`（Ref0=シェル名）の GET 発行へ進む。
4. When `OnBoot` の応答を受領した, the kanade engine shall `basewareversion` を NOTIFY として発行し boot 系列を完了する。
5. The kanade engine shall boot 系列の全イベントについて NOTIFY／GET の別と Reference 構成を正典（ukadoc・roadmap kanade 行の転記）どおりに構成する。
6. Where vanish count 等の永続値が利用できない（M1・永続化はスコープ外）, the kanade engine shall 毎回の起動を同一の運行として扱い、固定値により boot 系列を完走する。

### Requirement 2: Value 配送と talk 起動契約（本 spec が正本）

**Objective:** sakura-engine（消費者）の開発者として、talk 起動のメッセージ契約が kanade 側の正本として一意に定義され、SHIORI 応答スクリプトが解釈されないまま届いてほしい。再生系と運行系の責務が混ざらないように。

#### Acceptance Criteria

1. When GET 応答として Value（スクリプト文字列）を受領した, the kanade engine shall 一意な talk 識別子を付与した talk 起動要求（`StartTalk{script, talk_id}` 相当）を sakura 配送先へ送出する。
2. The kanade engine shall talk 起動契約のメッセージ型（`StartTalk{script, talk_id}`／`TalkDone{talk_id, quit}` 相当）の正本を所有し、script 文字列を解釈せず不透明なまま渡す。
3. When GET 応答が 204（Value なし）である, the kanade engine shall talk 起動要求を送出しない。
4. When `TalkDone{talk_id, quit}` 通知を受領した, the kanade engine shall talk_id により対応する talk と突合し、運行状態を更新する（`\-` 由来の quit 検出は sakura 側の責務であり kanade は通知の quit フラグを消費する）。
5. If 突合できない talk_id の TalkDone 通知を受領した, the kanade engine shall エラーログを記録した上で運行を継続する。

### Requirement 3: 定常運転（OnSecondChange pump・Tick 注入）

**Objective:** 開発者として、毎秒 pump が実時間に依存せず注入 Tick で駆動されてほしい。運行表全体を決定的にテストできるように。

#### Acceptance Criteria

1. While boot 系列完了後の定常運転状態, when 1 秒相当の Tick が到来した, the kanade engine shall `OnSecondChange` を GET として発行する。
2. The kanade engine shall Tick／時刻を外部から注入可能とし、実時間の sleep・実時計に依存せずに運行表の進行を決定的に再現できる。
3. When `OnSecondChange` の応答として Value を受領した, the kanade engine shall Requirement 2 と同一の talk 起動経路で配送する。
4. While boot 系列が完了していない、または close 握手が開始された後, the kanade engine shall Tick を受領しても `OnSecondChange` を発行しない。

### Requirement 4: close 握手

**Objective:** ゴースト運用者として、終了指示に対して OnClose の別れ口上が再生完了まで待たれてから終了してほしい。ゴーストが唐突に切断されないように。

#### Acceptance Criteria

1. When close 指示を受領した, the kanade engine shall `OnClose`（Ref0=理由）を GET として発行する。
2. When `OnClose` の応答として Value を受領した, the kanade engine shall talk 起動要求として配送し、対応する再生完了通知（TalkDone）を受領するまで終了系列を進めない。
3. If `OnClose` の応答が 204（Value なし）である, the kanade engine shall `OnCloseAll` を発行した上で終了系列へ進む。
4. If 再生完了通知が上限時間内に届かない, the kanade engine shall エラーログを記録した上で終了系列を継続する（上限値は design で確定・注入時刻で判定しテスト可能とする）。
5. When 終了系列が完了した, the kanade engine shall アクターとして停止し、停止の完了が呼び手（結線側）から観測可能である。
6. When 全ての指示送信元が切断された, the kanade engine shall 正常終了する（宙吊りで残らない）。

### Requirement 5: SHIORI 呼出境界（mock 差し替え可能な相手方）

**Objective:** 開発者として、SHIORI への GET/NOTIFY がメッセージ往復の境界越しに行われてほしい。実 32bit helper を繋がずに mock で運行全体を観測でき、実構成では既存 shiori 通信層の契約にそのまま載るように。

#### Acceptance Criteria

1. The kanade engine shall SHIORI への GET／NOTIFY をメッセージ往復（request/reply）境界越しに行い、SHIORI 実体を mock に差し替えても運行表の全経路を観測できる。
2. The kanade engine shall GET と NOTIFY の別を境界越しでも保持し、NOTIFY の応答から talk 起動要求を生成しない。
3. Where 実 SHIORI 経路が構成されている, the kanade engine shall 既存 shiori 通信層の呼出契約（200=Value あり／204=Value なし・エラー区別語彙）どおりに応答を解釈する。
4. When SHIORI 側の死活報告（helper 異常終了等・語彙は host32-lifecycle 正本）を受領した, the kanade engine shall エラーログを記録し、観測可能な停止状態へ遷移する（M1=ログ＋停止・自動再起動しない）。

### Requirement 6: 失敗経路の可観測性

**Objective:** 運用者・開発者として、あらゆる失敗が区別可能なログと観測可能な状態遷移として現れてほしい。沈黙の失敗経路が存在しないように。

#### Acceptance Criteria

1. If SHIORI 呼出がエラー（タイムアウト／SHIORI エラー／helper 死活／接続確立失敗の区別語彙）で失敗した, the kanade engine shall 区別語彙ごとにエラーログを記録し、観測可能な状態遷移として処理する。
2. When メッセージ処理が回復可能なエラーを返した, the kanade engine shall エラーを記録した上で後続メッセージの処理を継続する。
3. The kanade engine shall ログ無しの失敗経路を持たない（すべての失敗アームでエラーログを記録する）。
4. If 回復不能な致命状態に陥った, the kanade engine shall 直前に致命ログを記録した場合に限り panic を許容する（安易な panic の禁止）。

### Requirement 7: 決定的観測ハーネス

**Objective:** 開発者として、mock shiori＋mock sakura sink を結線した単一 pass/fail の決定的観測でユニット完了を判定したい。実 helper・実ゴースト資産なしで運行表の正しさを反復検証できるように。

#### Acceptance Criteria

1. The 観測ハーネス shall mock shiori（OnBoot→固定 Value・OnSecondChange→204 基調＋散発 Value の fixture 応答）と mock sakura sink を kanade に結線し、boot 指示から close 完了までの運行全体を駆動する。
2. When 運行全体を駆動した, the 観測ハーネス shall (a) boot 系列が正典順序で発火したこと（NOTIFY／GET の別・Reference 構成込み）、(b) Value 受領→talk 起動要求が sink に到達したこと、(c) close 指示→sink の再生完了通知を待って終了系列が完走したこと、を単一の pass/fail として検証する。
3. The 観測ハーネス shall 実時間 sleep に依存せず（時刻／Tick 注入）、反復実行で同一結果となる。
4. Where 実 helper 追験の環境変数ゲートが有効, the 観測ハーネス shall 実 32bit helper 越しの追験を実行できる（既定では skip され、mock 観測のみで pass/fail が完結する）。

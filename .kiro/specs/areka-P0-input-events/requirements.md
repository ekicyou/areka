# Requirements Document

## Project Description (Input)

areka-P0-input-events: ③ kanade 帰属の入力配信ユニット（M-life「撫でクラスタ」の片側＋M-dialogue の入口）。現状、マウス入力を SHIORI へ届ける経路が**ゼロ**であり、emo2 の撫で（`dic/touch.pasta`＝OnMouseMove）とダブルクリックメニュー（`dic/menu.pasta`＝OnMouseDoubleClick）という M1 ゴール（boot→talk→**touch**→**menu**→close）の中核が成立しない。kanade の `Input` にマウス系 variant が無く、現在のダブルクリックは stand-in の即終了（全ゴースト窓 despawn）に留まっている。本仕様は、マウス移動とダブルクリックを正典 Reference layout で SHIORI へ配信し、応答スクリプトを既存の talk 起動棚（kanade Steady の StartTalk 調停）へ載せ、stand-in 即終了を正規経路へ退役させる。当たり判定名の解決は `areka-P0-collision-geometry` の resolver 契約（`HitRegion { scope, region }`）を消費し、本仕様では再定義しない。決定論観測（mock shiori・注入入力・sleep 不使用）＋実 emo2 での撫で反応・メニュー talk 起動の人間サインオフで完了を判定する。OnChoiceSelectEx・選択肢 UI は M-dialogue `choice-render` 完了後の増分へ明示分離する。

## Introduction

本仕様は、マウス入力（キャラ窓上の移動・ダブルクリック）を SHIORI 運行系（③ kanade）へ届ける配信ユニットを定義する。UI 配線層がキャラ窓のマウスイベントを捉え、当たり判定名を collision-geometry の resolver で解決し、kanade へマウス入力メッセージとして配信する。kanade はこれを `OnMouseMove` / `OnMouseDoubleClick` の正典 Reference layout（ukadoc）で GET として発行し、応答スクリプト（Value）を**既存の talk 起動棚**（Steady の単一 slot 調停・StartTalk）へ載せる。加えて、現在の stand-in ダブルクリック即終了（全ゴースト窓 despawn）を正規経路へ退役させ、メニュー経由 `\-` 終了（M-dialogue 完成）が整うまでの暫定退避終了を明示的に残す。本仕様は「マウス入力→kanade→GET→StartTalk」の背骨と Reference 組立の型を確立するに留め、`\q` 選択肢の表示・選択 UI・OnChoiceSelectEx は M-dialogue へ分離する。

## Boundary Context

- **In scope**: マウス移動・ダブルクリックの取得と kanade への配信（UI 配線層・resolver 契約の消費）／`OnMouseMove` / `OnMouseDoubleClick` の正典 Reference 組立（GET・純粋関数）／マウス GET 応答（Value）の既存 StartTalk 棚への配送と調停規律／OnMouseMove の機械的間引き規則（決定論檻）／stand-in ダブルクリック即終了の退役と暫定退避終了の明示保持／決定論観測ハーネス（mock shiori・注入入力・単一 pass/fail）＋実 emo2 での撫で反応・メニュー talk 起動の人間サインオフ。
- **Out of scope**: `\q` 選択肢の表示・選択 UI・OnChoiceSelectEx（M-dialogue `choice-render`＋増分）／撫での意味論（連打・滞留の解釈＝SHIORI 側の領分）／当たり判定の幾何解決そのもの（`areka-P0-collision-geometry`）／OnMouseWheel・OnMouseClick 単発・The Hand（M1 外・M2）／owner-draw 右クリックメニュー（M2）／collisionex（円・多角形・M2）／再生タイミングの品質（`areka-P0-cue-playback-duration`）。
- **Adjacent expectations**:
  - 当たり判定 I/O 契約（`HitRegion { scope, region }`・resolver の提供形・当たり判定外の `None` 意味論）は `areka-P0-collision-geometry` が正本であり、本仕様は消費のみで再定義しない。resolver の入力は `(scope, 窓 client 物理 px 座標)` であり、「現在表示中の surface id」は emo 側が内部で引く（配線層は surface を知らない）。
  - talk 起動契約（`StartTalk` / `TalkDone`）と Steady の単一 slot 調停は `completed/areka-P0-kanade` が正本であり、本仕様は既存棚へ載せるのみで新しい調停を発明しない。
  - kanade への結線・channel／relay の流儀は actor 基盤（`areka-actor`）と `completed/areka-P0-ghost-setup` が正本であり、本仕様は独自のスレッド／通信流儀を発明しない。
  - 正典 Reference layout の典拠は ukadoc（`OnMouseMove` / `OnMouseDoubleClick`）であり、emo2 fixture は最小適合サンプルにすぎない。
  - `Status` ヘッダ（`idle-talk` が設計）へ将来 `choosing` を足すのは M-dialogue 側の申し送りであり、本仕様は触れない。
  - OnChoiceSelectEx（選択確定イベント）は本仕様の「マウス入力→kanade→GET→StartTalk」背骨と Ref 組立の型をそのまま再利用できる形（イベント種の拡張余地）に切ることを M-dialogue へ申し送る。
- **design 送り事項**（要件では確定しない・design で ukadoc 正典参照の上確定）:
  - talk 再生中のマウス GET の扱い（送出するか・抑止するか・NOTIFY 化するか）を SSP 挙動に基づき確定する。
  - 右ダブルクリックの SSP 既定動作（本体メニューかゴースト送出か）を確認し、M1 は owner-draw メニュー不在ゆえ右も SHIORI へ素直に送る案を既定に検証する。
  - OnMouseMove の間引き規則を 1 つ確定する（例: 当たり判定の変化時＋一定間隔）。
  - 暫定退避終了の具体手段を 1 つ確定する（例: 修飾つきダブルクリック、または既存 env-gate 系）。
  - 当たり判定が無い（`None`）場合の Ref4 値（空文字転写か省略か）・Ref6 入力デバイス種の具体値を確定する。
  - M1 送出マウスイベント集合表（OnMouseMove / OnMouseDoubleClick の 2 種）を確定し、`idle-talk` の送出ホワイトリスト檻と整合させる。

## Requirements

### Requirement 1: マウスイベントの取得と kanade への配信（配線層）

**Objective:** ゴースト運用者として、キャラ窓上のマウス移動とダブルクリックが SHIORI 運行系（kanade）へ届いてほしい。撫でとメニューがゴーストへ伝わるように。

#### Acceptance Criteria

1. When キャラ窓上でマウス移動イベントが発生した, the input-events 配線層 shall 当該スコープと窓 client 物理 px 座標から当たり判定名を解決し、OnMouseMove 相当のマウス入力を kanade へ配信する。
2. When キャラ窓上でダブルクリックイベントが発生した, the input-events 配線層 shall 当該スコープ・座標・左右ボタン別・解決した当たり判定名を含む OnMouseDoubleClick 相当のマウス入力を kanade へ配信する。
3. The input-events 配線層 shall 当たり判定名の解決を collision-geometry の resolver 契約（`HitRegion { scope, region }`）の消費のみで行い、当たり判定の幾何・現在サーフェス解決を自前で再定義しない。
4. The input-events 配線層 shall kanade への配信を actor 基盤（`areka-actor`／`ghost-setup` 由来の channel・relay 規約）の上で行い、独自のスレッド／通信流儀を発明しない。
5. Where collision resolver がまだ結線されていない（並走開発中）, the input-events 配線層 shall mock resolver により決定的に当たり判定名を供給でき、配信経路を単一 pass/fail として観測できる。

### Requirement 2: OnMouseMove の正典 Reference 組立と発行

**Objective:** 開発者として、OnMouseMove が正典 Reference layout で SHIORI へ届いてほしい。既存ゴースト（touch.pasta）が撫でに反応できるように。

#### Acceptance Criteria

1. When OnMouseMove 相当のマウス入力を受領した, the kanade engine shall `OnMouseMove` を GET として発行する。
2. The kanade engine shall OnMouseMove の Reference を正典 layout（Ref0=ローカル x 座標・Ref1=ローカル y 座標・Ref2=ホイール回転量・Ref3=対象スコープ〔本体 0／相方 1〕・Ref4=当たり判定の識別子・Ref6=入力デバイス種）どおりに構成する。
3. The kanade engine shall Ref4（当たり判定の識別子）を collision resolver 由来の領域名（不透明 String）として解釈せず転写し、当たり判定が無い（`None`）場合の Ref4 値は正典（ukadoc／SSP 挙動）に従う（空文字転写か省略かは design で確定）。
4. Where ホイールイベントを送出しない M1 構成, the kanade engine shall Ref2（ホイール回転量）を固定値 "0" で構成し、実ホイール量の載せ替えは increment シームとして残す。

### Requirement 3: OnMouseDoubleClick の正典 Reference 組立と発行

**Objective:** 開発者として、ダブルクリックが正典 Reference layout で SHIORI へ届いてほしい。既存ゴースト（menu.pasta）がメニューで応答できるように。

#### Acceptance Criteria

1. When OnMouseDoubleClick 相当のマウス入力を受領した, the kanade engine shall `OnMouseDoubleClick` を GET として発行する。
2. The kanade engine shall OnMouseDoubleClick の Reference を正典 layout（Ref0／Ref1=座標・Ref2=常に "0"・Ref3=対象スコープ・Ref4=当たり判定の識別子・Ref5=ボタン〔左 0／右 1〕・Ref6=入力デバイス種）どおりに構成する。
3. When 左ダブルクリックを受領した, the kanade engine shall Ref5 を "0" として構成し、右ダブルクリックでは "1" として構成する。
4. The kanade engine shall Ref4（当たり判定の識別子）を Requirement 2 と同一の不透明 String 転写規則で構成する。

### Requirement 4: マウス GET 応答の talk 起動調停（既存棚の再利用）

**Objective:** 開発者として、マウスイベントへの SHIORI 応答スクリプトが既存の talk 起動経路にそのまま乗ってほしい。撫で・メニューの応答会話が新しい調停を発明せずに再生されるように。

#### Acceptance Criteria

1. When マウス GET の応答として Value（スクリプト文字列）を受領し、かつ talk 非再生中である, the kanade engine shall 一意な talk_id を付与した talk 起動要求（`StartTalk` 相当）を既存の talk 起動経路で送出する。
2. When マウス GET の応答が 204（Value なし）である, the kanade engine shall talk 起動要求を送出しない。
3. While talk 再生中, the kanade engine shall マウス GET 応答の調停を既存の単一 slot 調停規律（active talk 中の置換規律）に従って行い、新しい調停規則を導入しない（再生中にマウス GET を送出するか・抑止するか・NOTIFY 化するかは SSP 挙動に基づき design で確定する）。
4. The kanade engine shall マウスイベントの追加を既存 kanade の入力・副作用指示（`Input`／`Action`）への additive な増分として行い、既存の決定的状態機械の資産を壊さない。

### Requirement 5: OnMouseMove の送出間引き

**Objective:** 運用者として、マウス移動のたびに SHIORI へ問い合わせが殺到しないでほしい。撫での過剰送出でゴーストや helper が溢れないように。

#### Acceptance Criteria

1. The input-events 配線層 shall 生のマウス移動イベントごとに OnMouseMove を送出せず、機械的な間引き規則（具体規則は design で 1 つ確定・例: 当たり判定の変化時＋一定間隔）に従って送出を絞る。
2. The input-events 配線層 shall 間引き規則を純粋・決定的な判定として実装し、注入入力で全経路を檻化できる。
3. The input-events 配線層 shall 撫での意味論（連打・滞留の解釈）を送出側で発明せず、その解釈を SHIORI 側の領分として委ねる。

### Requirement 6: stand-in ダブルクリック即終了の退役と暫定退避終了

**Objective:** ゴースト運用者として、ダブルクリックが即アプリ終了ではなく正規のメニュー応答経路になってほしい。同時に、メニュー完成前でもアプリを閉じる手段を絶やさないように。

#### Acceptance Criteria

1. When キャラ窓で（左）ダブルクリックが発生した, the areka アプリ shall 従来の全ゴースト窓 despawn（stand-in 即終了）を行わず、OnMouseDoubleClick を SHIORI へ送出する正規経路へ委ねる。
2. The areka アプリ shall メニューからの `\-` 終了（M-dialogue 完成）が整うまで、アプリを終了させる暫定退避手段を明示的に 1 つ残す（具体手段は design で確定し、暫定であることを記録する）。
3. The areka アプリ shall 暫定退避終了を既存の正規終了経路（kanade の close／force-quit 系列）に載せ、stand-in の直接 despawn を新設しない。

### Requirement 7: 送出マウスイベント集合の限定

**Objective:** 開発者として、M1 で送出するマウスイベントが明確に限定されてほしい。`idle-talk` のホワイトリスト檻と整合し、未対応イベントで運行が乱れないように。

#### Acceptance Criteria

1. The input-events 配線層 shall M1 で送出するマウスイベントを `OnMouseMove` と `OnMouseDoubleClick` の 2 種のみとする。
2. The input-events 配線層 shall OnMouseWheel イベントを送出しない（ホイールは Ref2 の口だけを残す）。
3. Where OnMouseClick（単発クリック）が未ハンドル（204）となる M1, the input-events 配線層 shall OnMouseClick 単発を送出しない。
4. The input-events 配線層 shall The Hand（つつき等）・collisionex（円・多角形）・owner-draw 右クリックメニューを送出・実装せず、M2 以降へ残す。

### Requirement 8: 決定的観測と実機受け入れ

**Objective:** 開発者として、mock shiori と注入入力による単一 pass/fail でユニット完了を判定し、実 emo2 で撫で・メニューの人間サインオフを得たい。実 helper・実ゴースト資産なしで背骨の正しさを反復検証できるように。

#### Acceptance Criteria

1. The 観測ハーネス shall mock shiori・注入マウス入力・sleep 非依存で、(a) OnMouseMove 入力→GET・Ref0〜6 が期待 layout（region 転写含む）、(b) 左ダブルクリック→GET・Ref5="0"、(c) 応答 Value→StartTalk（既存 slot 調停・active talk 中の置換規律）、(d) 204→無動作、(e) 送出間引き規則、を単一 pass/fail として検証する。
2. The 観測ハーネス shall 実時間 sleep に依存せず（時刻／入力注入）、反復実行で同一結果となる。
3. When 実 emo2・実 pasta.dll・実 DPI で起動した, the areka アプリ shall Head を撫でると touch.pasta が反応し、ダブルクリックで menu.pasta の応答 talk が起動することを人間が確認できる（応答 talk が再生されるところまで——`\q` 選択肢の見た目の完成度は M-dialogue `choice-render` の領分）。

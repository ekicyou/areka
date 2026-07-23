# Requirements Document

## Introduction

areka（ukadoc 準拠の互換ベースウェア）で emo2 ゴーストは起動してもまばたきしない。⑤ seriko には時間源（クロック／スケジューラ）が無く、受信面は cue 到着駆動のみで SERIKO の時限アニメ（`random,N` / `bind+random,N`）を回すランタイムが不在である。さらに、合成入力に pattern 進行状態を運ぶ通貨が無く、上流の mayuna-compose が静的側（pattern0 厳格選択）を閉じた結果、pattern0 を持たないまばたき系アニメ（pattern1 以降のみ）の動的フレーム再生が本 spec の宿題として残った。

本 spec は SERIKO ループ・ランタイムを seriko に追加し、emo2 の実まばたき2系統——kero（`interval,random,4`＝bind 非依存）と sakura（`interval,bind+random,4`＝着せ替えゲート）——を実機で駆動する。自律的な時間源を additive に増設し、pattern タイムライン評価（毎秒抽選・bind ゲート・wait[ms] 進行・`-1` 停止／末尾残留・再生中は再抽選対象外）を純関数化して、注入 tick＋注入乱数で決定論的に検証可能にする。合成入力を PatternState で第一級に拡張し、発行は既存の単一発行点＋冪等ガードを継承する。

正典は ukadoc（本 spec は 2026-07-23 調査済みの転記に基づく）、適合対象は emo2 最小 fixture である。

## Boundary Context

- **In scope**: 自律 tick 時間源の供給／`random,N` の毎秒抽選（bind 非依存）／`bind+random,N` の着せ替えゲート抽選／pattern タイムライン進行規則（wait[ms] 累積・現在コマ1枚・`-1` 停止・末尾残留・再生中は非再抽選）／コマ描画メソッド（method）の忠実転記・PatternState 搬送・合成適用（下記「parser スコープ拡大」を含む）／合成入力の PatternState 第一級拡張（合成署名＋合成キャッシュキー）／冪等発行／注入 tick＋注入乱数による決定論／実機まばたき2系統の人間サインオフ。
- **Out of scope**: 動的 bind 切替（上流 mayuna-compose 完了・read-only 参照のみ）／talk cue 再生（上流 cue-playback 完了）／他の interval 語彙（sometimes/rarely/periodic/always/runonce/never/yen-e/talk/bind/bind+always/bind+runonce）・`-2`・wait 範囲記法・exclusive option（いずれも emo2 未使用）／口パク（`interval,talk`）／`\i[N]` 明示再生タグ。
- **Adjacent expectations**: bindgroup の ON/OFF は上流の着せ替え状態を read-only で参照するのみで本ランタイムは変更しない。合成側（emo-compose）の animation ID 整列規則（昇順描画＝画家のアルゴリズム）と pattern0 厳格選択は不変で、transient コマは同キーで合流する。合成キャッシュ（emo-present）はキーを PatternState 分だけ拡張し容量1メモ化の思想は不変。自律 tick 源は既存の他系統 tick を改変せず additive に追加する。talk cue タイムラインと pattern タイムラインは別の時間系であり、pattern ループは talk 非従属の自律駆動である。
- **Surface 種別の非仕切り（areka シェル/バルーン統一グラフィック思想）**: pattern ループの駆動対象は surface 種別（シェル／バルーン）を区別せず、interval アニメを定義する任意の surface に一様適用される。能力を面種で仕切ることはしない（将来「バルーン surface 内にキャラを貼って喋らせる」「シェル領域にテキスト領域を置く」等の統合を要件が阻害しないため）。emo2 fixture が interval アニメを立ち絵 surface のみに定義するのは観測されたデータ事実であり、本 spec のスコープ制限ではない。pattern タイムライン評価器・毎秒抽選・PatternState はいずれも surface 非依存に保つ。
- **Parser スコープ拡大（描画メソッド転記）**: SERIKO コマ記法は正典（ukadoc `animation*.pattern*,描画メソッド,サーフェス番号,ウェイト,X座標,Y座標`）で描画メソッド（method）を先頭の位置引数に持ち（旧形式は第3位置）、コマの合成は「厳密には描画メソッドによる」。既存の pattern 転記モデルはこの method を落として emo2 の overlay を暗黙化していた＝転記層の欠落。本 spec はこれを転記の穴と見なし、pattern 定義に描画メソッドを忠実に転記する拡張を scope に含める（本 spec の作業は上流 pattern 転記モデルへ及ぶ）。`-1`/`-2` 時に method/X/Y が無視される正典挙動（R4.3）は method 欄の存在を前提とする。

## Requirements

### Requirement 1: 自律時間源（Tick 供給）

**Objective:** 開発者として、talk cue に従属しない自律クロックを SERIKO ループへ供給したい。そうすれば時限アニメが cue の有無に関係なく進行する。

#### Acceptance Criteria

1. The SERIKO ループランタイム shall talk cue の到着に依存せず、自律的な時間進行 tick を受理する。
2. When 時間源が周期 tick を供給する, the SERIKO ループランタイム shall 各 tick が運ぶ単調増加時刻に基づいて pattern タイムラインを進める。
3. While ゴーストが非表示（描画フレーム駆動が停止し得る状態）, the SERIKO ループランタイム shall 時間進行を停止させず、tick 供給を表示状態に従属させない。
4. The 自律 tick 時間源 shall 既存の他用途 tick 系統（cue ディスパッチャ向け・kanade 向け）を改変せず additive に追加される。

### Requirement 2: `random,N` の毎秒抽選（bind 非依存）

**Objective:** ゴースト作者として、`interval,random,N` のアニメ（kero のまばたき）を「毎秒 1/N の確率で再生」の正典どおりに自律再生させたい。

#### Acceptance Criteria

1. While 対象サーフェスが表示中 かつ 当該 `interval,random,N` アニメが非再生中, the SERIKO ループランタイム shall 毎秒 1/N の確率で当該アニメの再生を開始する。
2. When 毎秒抽選が発火する, the SERIKO ループランタイム shall 当該アニメを pattern タイムラインの先頭コマから再生する。
3. While 当該アニメが再生中, the SERIKO ループランタイム shall 当該アニメを毎秒抽選の対象から除外し、再生中に再抽選で restart しない。
4. The SERIKO ループランタイム shall 各 interval アニメの毎秒抽選をアニメごと独立に再抽選する。

### Requirement 3: `bind+random,N` の着せ替えゲート抽選

**Objective:** ゴースト作者として、`interval,bind+random,N` のアニメ（sakura のまばたき）を「当該着せ替えが ON のときのみ毎秒 1/N で発生」の正典どおりに駆動させたい。

#### Acceptance Criteria

1. While 当該 bindgroup が OFF, the SERIKO ループランタイム shall `bind+random,N` の毎秒抽選判定を実行しない（判定自体が走らない）。
2. While 当該 bindgroup が ON かつ 当該アニメが非再生中, the SERIKO ループランタイム shall 毎秒 1/N の確率で当該アニメの再生を開始する。
3. The SERIKO ループランタイム shall bindgroup の ON/OFF 状態を read-only で参照し、着せ替え状態を変更しない。
4. Where fixture 既定の着せ替え状態が与えられる, the SERIKO ループランタイム shall 当該 bindgroup を OFF とみなし（`\![bind,…]` 貫通で ON にされた場合にのみ抽選が走る）。

### Requirement 4: pattern タイムライン進行規則

**Objective:** 開発者として、コマ進行を ukadoc 正典どおり（wait[ms] 累積・現在コマ1枚・`-1` 停止／末尾残留）に評価する純粋な規則を確立したい。そうすれば全経路をテストで網羅できる。

#### Acceptance Criteria

1. When アニメ再生が開始する, the pattern タイムライン評価器 shall 各コマの wait（前コマからそのコマへ切り替わるまでの遅延）を累積した経過時刻に従ってコマを進める。
2. The pattern タイムライン評価器 shall 各時点で当該アニメの「現在コマ1枚」を表す（各コマは直前コマをリセットしてベースへ合成する＝合成方法は当該コマの描画メソッドに従い〔R4.6〕、既定では前コマ overlay を無制限に累積しない）。
3. When コマの surface が `-1`, the pattern タイムライン評価器 shall 当該アニメを停止してベース表示へリセットし、method/x/y を無視する。
4. When `-1` 終端が無いまま最終コマへ到達, the pattern タイムライン評価器 shall 最終コマを残留させたまま当該アニメを終了状態にする。
5. The pattern タイムライン評価器 shall wait の単位を 1ms（SERIKO 2.0）として解釈する。
6. The pattern タイムライン評価器 shall 各コマの描画メソッド（method）を SERIKO 描画メソッド語彙の忠実な転記値として保持し、`-1` 以外のコマではベースへの合成方法として当該メソッドを解釈する（emo2 が用いる overlay を駆動し、他メソッドの完全形保持は R8 に従う）。

### Requirement 5: 合成入力の PatternState 拡張

**Objective:** 開発者として、pattern 進行状態を合成入力の第一級要素にしたい。そうすれば動的コマが静的合成と同じ経路で正しく重なる。

#### Acceptance Criteria

1. The SERIKO ループランタイム shall 合成入力に、サーフェス識別子・有効な着せ替え集合（BindSet）に加えて pattern 進行状態（PatternState＝現在コマの surface_id・描画メソッド・x/y 等）を第一級要素として供給する。
2. When PatternState が変化する, the 合成キャッシュ shall pattern 進行状態をキャッシュキーの一部として扱い再合成する（容量1メモ化の思想は不変）。
3. The 合成 shall transient な pattern コマを既存の animation ID 整列規則（昇順描画＝画家のアルゴリズム）へ合流させ、整列規則そのものは変更しない。
4. Where PatternState が空, the 合成 shall 従来（pattern0 静的土台のみ）の合成結果と一致する。

### Requirement 6: 冪等発行

**Objective:** 開発者として、pattern が動かない tick では再発行しないようにしたい。そうすれば毎フレーム無駄な再合成を防げる。

#### Acceptance Criteria

1. When PatternState が変化した tick, the SERIKO ループランタイム shall 単一の発行点から表示コマンドを1回発行する。
2. While PatternState が不変の tick, the SERIKO ループランタイム shall 表示コマンドを再発行しない。
3. The SERIKO ループランタイム shall 既存の単一発行点および冪等ガードを継承する。

### Requirement 7: 決定論と検証可能性

**Objective:** 開発者として、時刻と乱数を注入して全経路を決定論的に検証したい。そうすれば非決定要素なしで tick・確率・タイムラインを網羅テストできる。

#### Acceptance Criteria

1. The SERIKO ループランタイム shall 時刻と乱数を注入シーム経由で受け取り、実時間源・実 entropy 源への直接依存を評価経路から排除する。
2. When テストが固定の注入 tick 列と注入乱数列を与える, the SERIKO ループランタイム shall 期待される PatternState 列および golden 合成結果に一致する。
3. The 決定論テスト shall 実時間の sleep を用いずに tick 駆動のみで完結する。
4. While 本番実行, the SERIKO ループランタイム shall tick を実時間源へ、乱数を entropy 源へ接続する。
5. If いずれかの経路が失敗する, then the SERIKO ループランタイム shall 無音で失敗せずログを伴って失敗を報告する。

### Requirement 8: スコープ規律（未使用語彙の完全形保持）

**Objective:** メンテナとして、本 spec が駆動するのは2つの interval のみであることを明確にしつつ、将来の SERIKO 拡張のために語彙を完全形で保持したい。

#### Acceptance Criteria

1. The 実装 shall 挙動として `random,N` と `bind+random,N` の2つのみを駆動する。
2. Where 他の interval 語彙（sometimes/rarely/periodic,N/always/runonce/never/yen-e/talk,N/bind/bind+always/bind+runonce）・`-2`・wait 範囲記法・exclusive option が定義に現れる, the システム shall それらの型／語彙を完全形で保持し、本 spec では駆動しない。
3. The システム shall 口パク（`interval,talk`）・`\i[N]` 明示再生タグ・動的 bind 切替・talk cue 再生を本 spec の駆動対象から除外する。
4. Where コマの描画メソッド語彙（overlay/base/move/scaling/start/stop/alternativestart/alternativestop/parallelstart/parallelstop/insert/auto 等）が定義に現れる, the システム shall 全メソッドを忠実に転記し完全形の型値として保持したうえで、本 spec では emo2 が用いる合成メソッド（overlay）を駆動し、他メソッド（制御系 start/stop/parallel/alternative・幾何系 move/scaling・着せ替え系 insert 等）は完全形保持のまま駆動しない。

### Requirement 9: 実機まばたき2系統サインオフ

**Objective:** 開発者として、実 emo2・実 DPI でまばたき2系統が動くことを人間の目視で確認したい。そうすれば決定論テストだけでは捉えられない実結線の欠陥を検出できる。

#### Acceptance Criteria

1. When 実 emo2・実 pasta.dll・実 DPI で起動する, the areka shall kero（`random`）のまばたきを表示する。
2. When sakura の該当 bindgroup を `\![bind,まばたき,通常,1]` で ON にした状態で起動する, the areka shall sakura（`bind+random`）のまばたきを表示する。
3. The 実機サインオフ shall 人間の目視確認を受け入れ条件とする。
4. The SERIKO ループランタイム shall デファクト推定2点（`-1` 無し末尾到達時の最終コマ残留・再生中のアニメは再抽選対象外）を確定した期待挙動として実装し、実機で挙動齟齬が観測された場合は SSP の実観察で裏取りする。

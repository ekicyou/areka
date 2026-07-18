# Brief: areka-P0-sakura-time-directives

> **種別**: 追跡 spec（正典先送りの 4 点セット＝完全語彙＋縮退シーム＋追跡 spec＋roadmap 明記）。④sakura（compile）＋dola（Barrier シーム）帰属。
> **源**: `areka-P0-sakura-dialogue-tags` 要件ディスカッション議題5（2026-07-18 `\!` 汎用キャリア裁定）の compile 側 allowlist 但し書き（同 R4.3 が正本）。敵対的検証（refute-opacity）が「解釈 100% 消費側」の有界反例として確定したクラス。
> **着手ゲート**: M1 外（emo2 実使用は move/bind のみ＝本 allowlist コマンドの使用ゼロ）。これらを使うゴーストの適合が必要になった時に解禁。

## Problem

`\!` 汎用キャリア裁定は「転写は不透明・解釈は消費側」を原則とするが、**compile 自身が第一消費者になるべき時間指令系**が有界（10 個未満）存在する。これらは絶対時刻焼込（`text_playback_duration`・配送時導出は禁忌＝desync・[[areka-dola-absolute-time-sync-broadcast]]）・barrier パラメータ・実行時未確定の待機に構造上干渉し、**消費側の発火時解釈では既焼込の後続 cue 絶対時刻を直せず手遅れ**になる。

M1 は全て汎用 cue として転写（語彙第一級保持）しつつ、compile 追加解釈なし＋消費者不在の良性スキップ（時間効果なし）で縮退する。

## 語彙（完全形・ukadoc 一次 HTML 接地・これが allowlist の全量）

| 群 | コマンド | 干渉の構造 |
|---|---|---|
| **A. テキスト時間指令** | `\![quicksection,true\|false\|数値]`（`\_q` ほぼ互換＝瞬間表示） / `\![set,balloonwait,倍率\|ms指定]`（文字ごとウエイト倍率・スクリプト終了でリセット） | per-char D 焼込の計算そのものを変える |
| **B. script 単位属性→barrier パラメータ** | `\![set,choicetimeout,時間]`（**位置非依存**＝「選択肢より後ろに書いても有効」→ `WaitForChoice{timeout}` へ焼込） / `\![set,balloontimeout,時間]` | cue の区間モデルでは「後方 cue が前方 barrier を書換える」を表現できない＝compile（全 sheet 事前走査）必須 |
| **C. Barrier 級（実行時未確定・自己書換）** | `\![embed,イベント名,r*]`（タグ全体が SHIORI Result で置換され続行＝台本分割＋再調停が必要） / `\![sound,wait]`（=`\_V`） / `\![wait,syncobject,名前,--timeout=]` | 待機長がコンパイル時不可知＝静的絶対時刻タイムラインに直接載らない |
| **D. ブロッキング持続時間引数** | 同期 `\![move]` の時間スロット／`--time`（envelope duration へ転写し offset を進める・moveasync は 0） / `\![set,scaling,--time/--wait]` / `\![set,alpha,--time/--wait]` | duration 実値化に引数解釈が要る（転写は不透明のまま・duration 焼込だけ compile が name を覗く） |

## Desired Outcome

allowlist 各コマンドが compile で追加解釈され、A＝D 焼込補正／B＝barrier パラメータ焼込／C＝台本分割＋Barrier シーム＋オーケストレーター（kanade/sakura）再調停／D＝envelope duration 実値化へ正しく lowering される。**allowlist 外の compile 解釈は引き続き禁止**（dialogue-tags R4.3 が恒久の正本・汎用キャリアの不透明原則を侵食させない）。

## Approach

compile の汎用キャリアアームへ allowlist 判定を追加（純関数・全網羅檻）。C 群は dola の Barrier シームへ写像（動的制御は dola 外側＝settled 裁定）。段階導入可: A/B/D は compile 局所・C は台本分割の設計が要る（C だけ後続波でも良い）。

## Scope

- **In**: allowlist 8 コマンド族の compile 追加解釈・lowering・決定論檻（script 直入力→期待 cue/barrier/duration 列）。
- **Out**: allowlist 外の compile 解釈（恒久禁止）／各コマンドの**消費側**実装（該当演者の領分）／SSTP 経由の文脈依存挙動。

## Upstream / Downstream

- **Upstream**: `completed/areka-P0-sakura-dialogue-tags`（汎用キャリア＋R4.3 allowlist 契約の正本）／completed `cue-playback-duration`（絶対時刻台本・Barrier シーム・envelope duration）。
- **Downstream**: これらのコマンドを使う実ゴーストの適合／`areka-P0-choice-select-events`（`choicetimeout` の**ランタイム消費側**＝タイムアウト起点・OnChoiceTimeout 発火は W5 の領分・本 spec は compile 焼込のみ）。

## Constraints

- 正典は ukadoc。決定論檻必達・二重待ち禁止（タイミングは焼込絶対 start_time が唯一の権威）。
- 汎用キャリアのワイヤ形・消費側名前選別の規律（dialogue-tags R4.5/R8.7）は不変。

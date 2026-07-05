# Brief: areka-P0-sakura-engine

> **種別**: 本坑（main）。④ sakura＝**さくらスクリプト再生エンジン**（talk timeline・per-talk transient）。
> **調査日**: 2026-07-05（sakura-parse✅／dola✅／areka-actor✅ の実 API 調査済み）。

## Problem

SHIORI が返す Value（さくらスクリプト）を**時間軸上で再生する装置が無い**——`\w` の待ち・テキストの逐次供給・`\s` の surface 指令・`\e`/`\-` の終端検出。ここが埋まらないと「emo2 が喋る」の"喋る"が成立しない。

## Current State

- **入力モデル ✅（実シンボル）**: `areka_parsers::sakura::parse(&str) -> Vec<Instruction>`——flat enum: `Text(String)`／`SpeakerScope{n}`（`\p[n]`・`\0\1` 系）／`Surface(SurfaceArg)`（**不透明**・解釈しない）／`Wait(Duration)`（`\w`/`\_w` 統一済み）／`NewLine(NewLineRatio)`／`Choice`／`Cursor`／`End`（`\e`）／`Clear`／`Quit`（`\-`）／`Move(MoveArgs)`／`SystemVar`／`GenericCommand`／`Raw`（寛容フォールスルー）。
- **タイミング層 ✅**: `DolaRuntime`（`new`/`load_document`/`start`/`tick(current_time)`/`last_result`…）＋`clock::now()`。**`tick` は時刻注入式＝決定的テストが構造的に可能**。
- **アクター基盤 ✅**: `areka-actor`（spawn_actor/reply_channel）。**per-talk transient**＝talk ごとに生まれ消える（構築モデル正本: constructor＝さくらスクリプト・runtime・都度）。
- **上流契約（消費・再定義しない）**: talk 起動契約 `StartTalk{script, talk_id}`／`TalkDone{talk_id, quit}` ＝ **kanade brief が正本**。

## Desired Outcome

script 文字列を受けた sakura インスタンスが Instruction 列を**時間軸再生**し、下流 2 分岐——**surface 指令→seriko**／**テキスト・改行・進行→emo(text-layer)**——へ発火列を届け、終端（`End`/`Quit`）で `TalkDone` を返して消える。

**✔ 観測（単一 pass/fail・観測の独立化）**: fixture script（emo2 boot 級: text＋`\s`＋`\w`＋`\e`）を **script 直入力**し、**mock sink 2本**（surface 指令/テキスト系）に届く**発火列と発火時刻**が期待どおり（`\w[n]` の待ちが時間軸に反映・**時刻注入で決定的**）＋終端で `TalkDone{quit}` が正しい。kanade 不要・表示不要。

## Approach

1. **配置**: `crates/areka-sakura`（命名慣行）。per-talk transient——`spawn_actor` で talk ごと起動 or 呼出側での逐次 sequencer は **design 判断**（判断材料: transient 生成コスト・kanade からの中断（close 時の talk 打ち切り）の要否・dola instance の持ち方）。
2. **時間軸展開**: Instruction 列→タイムライン。**dola 経由が既定**（dola＝タイミング層の正本方針）——`Wait` を累積オフセットに畳んで dola storyboard/schedule へ載せる vs 自前 seq（`recv_timeout` 刻み）は design で比較（per-talk の document 生成コストと決定性を材料に）。**typewriter の字送り間隔はテキスト側（emo-text-layer）の責務**＝sakura は「このテキストをこの時点で供給開始」までを所有（境界を design で固定）。
3. **下流 2 分岐（roadmap 正本どおり）**: `Surface(SurfaceArg)`→**surface 指令**（SurfaceArg は不透明のまま——id 解決・alias は seriko/emo 側）／`Text`/`NewLine`/`Clear`→**テキスト系指令**。`SpeakerScope` は両分岐に共通の scope 文脈として付与。
4. **終端と中断**: `End`→TalkDone{quit:false}・`Quit`→TalkDone{quit:true}（**close 握手で kanade が待つ信号**）。kanade からの中断（Close）で即時停止（積み残し破棄＝areka-actor の停止規約に整合）。
5. **M-boot 外タグの扱い**: `Choice`/`Move`/`Cursor`/`SystemVar`/`GenericCommand`/`Raw` は**受けて無視（ログ）＋型シーム**（実装は sakura-dialogue-tags・M-dialogue）。寛容・非パニック。

## クロスユニット契約（申し開き）

- **本 brief が正本**: **再生出力契約**——`SurfaceCommand{scope, surface: SurfaceArg, at}` 級／`TextCommand{scope, text/newline/clear, at}` 級／`TalkDone{talk_id, quit}` のメッセージ型（**seriko・emo-text-layer・kanade が消費**・再定義しない。at＝タイムライン時刻の意味論込み）。
- **消費する正本**: `StartTalk`/`TalkDone` の授受＝kanade／`Instruction` モデル＝sakura-parse（完了・変更しない）／envelope＝areka-actor。
- **seriko への申し送り**: SurfaceArg 不透明のまま渡す＝**alias 解決（`sakura.surface.alias`）と id 解釈は seriko→emo 側**（parser 転記層の原則と同型）。seriko-engine brief 化時にこの契約を消費すること。

## ukadoc 必読（design 着手時に ukadoc MCP で正典参照）

- `list_sakura_script`: **`\w`/`\_w`**（待ち単位の正確な意味論——`\w` は 50ms 単位等の実値を確認）・**`\e`/`\-`**（終端規律）・**`\0`/`\1`/`\p`**（scope 切替の正準）・**`\n`**（改行と `\n[half]` 等の変種）・**`\s`**（引数形——ただし解釈は下流）。
- **具体指示**: design 冒頭で「M-boot 再生対象タグ表（実挙動/無視ログ/シーム）」を `Instruction` 全 variant について作成し、`\w` の実時間換算値を ukadoc で確定して fixture 期待値に反映すること。

## Scope

- **In**: talk timeline 再生（時刻注入・決定的）／下流 2 分岐の出力契約（正本）／終端・中断／M-boot 外タグの寛容無視＋シーム／per-talk transient の生成・破棄／mock sink 観測ハーネス。
- **Out**: script の字句解析（**sakura-parse** 完了）／surface id・alias の解釈（**seriko/emo**）／typewriter 字送り・グリフ描画（**emo-text-layer**）／`\q`/`\![move]`/`\_l` の実挙動（**sakura-dialogue-tags**・M-dialogue）／talk の選定・スケジューリング（**kanade**）。

## Boundary Candidates

- タイムライン展開（Instruction→時刻付き発火列・**純粋＝単体テスト主戦場**）／再生駆動（clock/dola 結線）／出力結線（sink 2本）の三層。

## Upstream / Downstream

- **Upstream**: `sakura-parse` ✅・`dola` ✅・`areka-actor` ✅・talk 起動契約＝`areka-P0-kanade`（並走・契約先決済み）。
- **Downstream**: `seriko-engine`（SurfaceCommand の消費者・brief 未作成＝本契約を前提に後続）／`emo-text-layer`（TextCommand の消費者）／`kanade`（TalkDone）。

## Existing Spec Touchpoints

- **Adjacent**: `areka-P0-kanade`（**並走可**——授受契約は kanade 正本で先決済み・mock で独立観測）／`areka-P0-emo-compose`（無関係・非衝突）。

## Constraints

- Rust 2024・tokio 禁止・areka-actor 規約に載る。新設クレート＝非衝突。
- 決定的テスト（時刻注入・実時間 sleep 非依存）。寛容・非パニック（parsers の流儀）。ログ無し失敗経路の禁止。

# Brief: areka-P0-sakura-dialogue-tags

> **種別**: 本坑（main）増分。④ sakura 帰属（M-dialogue の起点＝script 語彙の cue 貫通）＋⓪末端（`\![move]` 消費のみ・薄い結線）。
> **調査日**: 2026-07-16（再入精査⑧・fixture 実物調査＋コード実態偵察）。
> **⛔ 時限ゲート**: `areka-P0-cue-playback-duration`（実装中）の**完了が前提**——本 spec の編集面（sakura `compile.rs`・dola `CueCommand`/配送）は同 spec が全面改修中（settled な duration 付き cue モデル＋`CueSink`/`CuePlayer` へ載せる）。着手時に同 spec の最終形（`dola/src/cue/sink.rs`/`runtime.rs`）を再突合すること。

## Problem

emo2 のメニュー・位置調整・撫で talk が使う4語彙が、**parse は成功するのに compile catch-all（`compile.rs:92-94`「M-boot 外タグを無視」・除外仕様を test `compile.rs:387-419` が固定）で全て無音落ち**している:

- **`\q[タイトル,イベント名]`**（`menu.pasta:15/33/62`＝9個）→ `Instruction::Choice` 止まり＝選択肢が cue にならずメニューが存在しない。
- **`\_l[5em,2lh]`**（menu 3箇所・選択肢区切りの位置指定）→ `Instruction::Cursor` 止まり。
- **`\![move,-353,,,0,base,base]`**（`boot.pasta:79`・`menu.pasta:65`）→ `Instruction::Move` 止まり。**初回起動（OnFirstBoot・boot.pasta:78 セクション）時にも発火する**＝現状、初回起動時のエモ（kero）位置調整が黙って失われている（通常起動4バンドには move 無し）。
- **`%username`**（`touch.pasta:78/:99`）→ `Instruction::SystemVar` 止まり＝撫で talk で生文字列 `%username` が露出する（環境変数展開はベースウェア義務・ukadoc OnTranslate 文書「ベースウェアによる環境変数の展開」）。

## Current State（2026-07-16 実装偵察）

- **②parsers は完了済み**（本 spec に parser 作業なし）: `Instruction` 15 variant（`sakura/model.rs:23-61`・BalloonSurface 込み）に `Choice{disp,target,references}`（`:97-104`）・`Cursor{x,y}`（文字列保持）・`Move(MoveArgs{args})`・`SystemVar(String)` が実在。`\q[disp,target,ref...]` は arg0=disp・arg1=target・残り=references（`decode.rs:191,209-219`）。emo2 実物は **2引数形のみ**（references 空）。
- **dola に受け皿が半分ある**: `CueCommand::Choice{id,text}`（`command.rs:122-145`）・`BarrierKind::WaitForChoice{timeout:Option<f64>}`（`:87-94`）は初期設計から実在（記憶 [[areka-dola-cue-is-sakura-engine]]）。**parser `Choice{disp,target,references}` との形状不一致**（references の載せ先が無い）は design で裁定（emo2 未使用＝型シーム or 拡張）。
- **cue-playback ブランチが土台を建設中**（`claude/areka-p0-cue-playback-aba824`・43 files +4761/-367 実測）: dola に `CueSink` trait（`sink.rs:25-30`）・`cue_target_of` の dola 移動＋**Choice/ClearAll を Balloon 系へ分類済み**（`sink.rs:40,60`）・**`CuePlayer`**（`runtime.rs:92`）が `PendingChoice` 型（`:53`）＋`pending_choices` バッファ（`:98`・読み口 `:355`）・`WaitingForChoice` 状態（`:65-71`）・**`resolve_choice(choice_id)->Option<String>`**（`:279`）まで実装。**ただし同ブランチでも sakura compile の `Instruction::Choice` は依然 catch-all drop**（branch `compile.rs:116` 実測）＝**`\q`→cue 発行は本 spec が最初**。
- **`\![move]` の消費 API は既に在って眠っている**: `placement::move_window_to(world, window, x, y) -> bool`（`placement/follow.rs:500`・pub・`#[allow(dead_code)]`＝実コメント「呼び出し側（UI 配送ブリッジ結線）は後続 spec の領分」・BalloonFollow 随伴移動込み）——本 spec が dead_code を解消する結線先。

## Desired Outcome

fixture script 直入力で、4語彙が**決定論的に正しい cue／barrier 列へコンパイル**され、`\![move]` は末端まで貫通して**実機の初回起動（OnFirstBoot 経路）でエモが横へ動く**（boot.pasta:79 が最初の実観測点・通常起動4バンドには move 無し＝観測は初回起動状態で行う）。choice cue 形は本 spec が**正本**として確定し、下流（choice-render／choice-select-events）が消費する。

**✔ 観測（単一 pass/fail）**: 決定論（script 直入力・sleep 不使用）＝(a) `menu.pasta` メインメニュー選択肢 script→`Choice` cue×3＋`WaitForChoice` barrier＋`\_l` cue の期待列（at/duration 整列は cue-playback の settled 規則に整合） (b) `boot.pasta:79` の `\1\![move,-353,,,0,base,base]`→scope1 の Move cue（引数保持） (c) `%username`→注入値で展開済み Text（未設定時は既定値） (d) catch-all 除外 test（`compile.rs:387-419`）の**仕様変更を明示的に更新**（Choice/Cursor/Move/SystemVar は檻から卒業）。＋実機＝実 emo2 の**初回起動（OnFirstBoot 経路・初回状態で起動）**でエモの初期位置調整（`\![move]`）が目視で効く。

## Approach

1. **dola cue 語彙の増分**（additive・balloon-face-cue が `BalloonSurface` を追加した前例＝enum 拡張は安全）: `Cursor`（`\_l`・名称は design 確定）と `Move` の variant 追加。`Choice` は既存 `{id,text}` を基本に、references の扱い（emo2 未使用→捨てずに型シームで保持 or 拡張）を design で裁定。ペイロードは不透明側（[[areka-surface-args-opaque-string-downstream-resolve]]＝em/lh 単位や move 引数の解釈は消費側）。
2. **compile アーム新設**（catch-all からの救出・mayuna の `Bind` と同型）: `Choice`→`Choice` cue 発行＋**talk 内の選択肢群の直後に `WaitForChoice` barrier を置く規則**（CuePlayer `:195,223` の preload/停止実装と整合させる・タイムアウト値は None＝既定は choice-select-events の領分）／`Cursor`→Cursor cue／`Move`→Move cue／`SystemVar`→**展開**（下記3）。
3. **`%username` 展開**: 純関数（`SystemVar` 名→値・未知名は素通し or 空の裁定を ukadoc で）＋**値源は構築時注入**（`GhostBootOptions` 系 config・M1 は設定ファイル/引数の固定値＋既定値。ukadoc 環境変数ページで SSP 既定を確認）。展開位置は compile（Text へ合流）＝emo-text は関知しない。
4. **配送分類**: settled 配送モデルへ載せる——cue-playback 設計は **broadcast＋演者側 relevance**（記憶 [[areka-cue-runtime-consolidated-in-dola]]）だが branch 実測では `cue_target_of` が dola に残存（移行中）。着手時に settled 形を確認し、Cursor=Balloon 系（emo-text 消費）・Move=ghost/placement 消費・Choice=Balloon 系（branch で分類済み）で整合。
5. **`\![move]` 末端結線（本 spec の唯一の UI 貫通）**: Move cue の消費者を ghost 側に新設（`CueSink` 実装 or emo2_boot adapter の追加アーム＝settled 配送モデルに従う）→ `move_window_to`（`follow.rs:500`）呼出（UI スレッド配送は `UiSender` 規約）。`\![move,x,y,time,加速度?,base-x,base-y]` の**引数意味論と座標系（base 基準・物理/論理 px）を ukadoc で確定**——time 付き移動アニメは emo2 未使用（`-353,,,0,base,base`＝即時）ゆえ**即時移動のみ実装＋time は型シーム**。

## クロスユニット契約（並走を詰ませない事前考慮・2026-07-16）

- **choice cue 形＝本 spec が正本**: `CueCommand::Choice` の最終ペイロード（id/text/references の載せ方）と「選択肢群＋WaitForChoice barrier」の並び規則は本 spec が確定し、**choice-render（表示）と choice-select-events（`resolve_choice` 駆動）は消費のみ**（再定義しない）。3 spec の並走は本契約＋ChoiceSelection 契約（choice-render 正本）の先決で担保（撫でクラスタ＝collision-geometry⟷input-events の正本連鎖パターン再演）。
- **cue-playback-duration が絶対上流**: duration 焼込・`ClearAll` 前置・first-class `Wait`（branch task 5.2 済）の settled compile 構造へ**別アームを足す**形。emit 署名・`Cue` 形が動いている間は着手しない（時限ゲート）。
- **mayuna-compose と compile.rs 近接**: 双方 catch-all から別 variant を救出する additive アーム（mayuna=`GenericCommand{bind}`・本 spec=Choice/Cursor/Move/SystemVar）＝**別アーム・マージ可能**（balloon-face-cue 実績）。同時着手時は rebase 近接に注意のみ。
- **`\_l` の消費は choice-render**（em/lh 換算・カーソル適用は emo-text 側の知識）——本 spec は cue 転写まで（単位文字列は不透明のまま運ぶ）。
- **`\![move]` と position-persist の座標整合**: move 後の位置はドラッグ同様「ユーザ位置」か（persist 対象か）を design で position-persist brief と突合（`move_window_to` は既に BalloonFollow 随伴を処理）。

## ukadoc 必読（design 着手時に ukadoc MCP `get_doc`/`search_docs` で正典参照・2026-07-16 裏取り）

- **`\q` は計6形**（裏取り済み・2026-07-16 検証で全形確認）: 主要2形＝`ukadoc:list_sakura_script:_5cq_5b_30bf_30a4_30c8_30eb_2cID_5d:1`（2引数＝OnChoiceSelect・Ref0=ID）／`..._2cID_2cr2_2cr3..._5d:1`（拡張＝OnChoiceSelectEx・Ref0=ラベル・Ref1=ID・Ref2+=r*）＋ ID 複数形／**`\q[タイトル,OnID,r0,r1...]`＝On 始まり ID は任意名イベント直接発火・r\* は Ref0 起点**／`script:` 形／旧2ブラケット形。**emo2 実物は2引数形のみ・ID は `On〜` イベント名＝OnID 形の正典規則に該当**（カスケード則の確定は choice-select-events の領分だが、cue へ**両引数＋references を欠落なく運ぶ**のは本 spec の義務）。
- **`\_l[x,y]`**: em/lh 単位・省略形の有無を `list_sakura_script` で確認（fixture は `[5em,2lh]` のみ）。
- **`\![move]`**: 全引数形（x,y,time,基準,base-x,base-y の正確な並び・省略時挙動・base の意味）を `list_sakura_script` の move 項で確定。fixture 形 `-353,,,0,base,base` の空引数の意味も。
- **環境変数**: `ukadoc:list_sakura_script:環境変数の記述例` 周辺で `%username` の定義と SSP 既定値・他の % 変数（emo2 は %username のみ実使用）を確認。
- **`\*`／choicetimeout**: fixture 未使用（grep 0）を確認済み——scope doc §3 の「`\![*]` 選択肢マーカー M1 必須」記述は**実物と不一致**＝design で M1 外へ格下げ確認（正典は ukadoc・fixture が実需）。

## Scope

- **In**: dola `CueCommand` の Cursor/Move variant 増分＋Choice ペイロード裁定／compile アーム4本（Choice+barrier 規則・Cursor・Move・SystemVar 展開）／`%username` 値源注入＋既定値／Move cue の末端消費（ghost 側 sink→`move_window_to`・即時移動のみ）／catch-all 除外 test の仕様更新／決定論檻（fixture script→期待 cue 列）／実機サインオフ（起動時エモ位置調整）。
- **Out**: 選択肢の**表示・UI**（choice-render）／選択確定→SHIORI カスケード・タイムアウト（choice-select-events）／`\![bind]`（mayuna-compose）／time 付き移動アニメ（emo2 未使用＝型シーム）／`\![raise]` 等その他 GenericCommand（M1 外・catch-all 残留）／NOTIFY 系 update イベント（M2）。

## Boundary Candidates

- cue 語彙増分（dola・機械的 additive）／compile 写像＋barrier 規則（純関数・全網羅可能）／%username 展開（純関数＋値源注入）／move 末端結線（薄い UI 配線・実機確認必須）。

## Out of Boundary

- 選択肢クラスタの表示/入力（契約は本 spec の choice cue 形＋choice-render の ChoiceSelection で先決済み）。

## Upstream / Downstream

- **Upstream**: **`areka-P0-cue-playback-duration`（時限ゲート・settled cue モデル/CueSink/CuePlayer/ClearAll/duration）**／`completed/areka-P0-sakura-parse`（4 variant 転記済み）／`completed/areka-P0-window-placement`（`move_window_to`）／`completed/areka-P0-ghost-setup`（config・channel 規約）。
- **Downstream**: `areka-P0-choice-render`（choice cue 消費）／`areka-P0-choice-select-events`（barrier 解除・カスケード）／`areka-P0-emo2-conformance-e2e`（メニュー一周・位置調整を適合項目に）。

## Existing Spec Touchpoints

- **Extends**: `completed/areka-P0-sakura-engine`（compile の additive アーム＝既存決定論資産を壊さない・除外 test は意図的更新）。
- **Adjacent**: `areka-P0-mayuna-compose`（compile.rs 近接・別アーム）／`areka-P0-cue-playback-duration`（**絶対上流**）／`areka-P0-position-persist`（move 後位置の帰属突合）。
- **Supersedes**: roadmap 増分の旧「`sakura-dialogue-tags`（`\q`/`\_l`/`\![move]`）」表記を **`%username` 込み・compile〜move 末端の実物スコープ**へ確定（fixture 実測 2026-07-16）。

## Constraints

- Rust 2024・新規 crates.io 依存なし・tokio 不使用・additive（既存 cue variant のワイヤ形不変・serde 互換）。
- **決定論**: script 直入力→cue 列の全網羅（[[deterministic-test-coverage-mandate]]）。展開・写像・barrier 規則は純関数（[[test-only-decision-branches-not-proven-wiring]]）。
- 面引数・単位・move 引数は**不透明 String 転写・解釈は消費側**（[[areka-surface-args-opaque-string-downstream-resolve]]）。
- **実機受け入れ**: 実 emo2・実 pasta.dll・実 DPI で起動時 `\![move]` を人間サインオフ（[[areka-placement-real-ghost-first]]）。起動は絶対パス必須（MOD_NOT_FOUND 運用注意）。
- 正典は ukadoc・emo2 は最小適合 fixture（[[ukadoc-mcp-preferred-source]]）。

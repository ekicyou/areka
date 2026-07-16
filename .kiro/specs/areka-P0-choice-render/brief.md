# Brief: areka-P0-choice-render

> **種別**: 本坑（main）増分。⑥ emo 帰属（M-dialogue の表示側＝バルーン内選択肢 UI）。**ChoiceSelection I/O 契約の正本**（choice-select-events が消費・撫でクラスタの `HitRegion` 正本パターン再演）。
> **調査日**: 2026-07-16（再入精査⑧・fixture 実物調査＋コード実態偵察）。
> **⛔ 時限ゲート**: `areka-P0-cue-playback-duration`（実装中）完了＋choice cue 形（`sakura-dialogue-tags` 正本）先決が前提——emo-text の cue 受信面（`CueSink` 化）と `CuePlayer` の pending choices 供給が settled してから着手。契約先決済みなら sakura-dialogue-tags と**並走可**。

## Problem

emo2 のメニュー（`menu.pasta`）は `\q[おしゃべり頻度,Onおしゃべり頻度メニュー]` 等の選択肢**表示**を要求するが、areka に選択肢 UI が存在しない:

- emo-text `apply_cue` は Choice cue を**「actor ごと一度 warn して無視」する明示シーム**として実装済み（`state.rs:186-191`・warn 実文言「Choice cue は M1 未対応のため無視する（choice-render シーム）」）——本 spec がこのシームの宛先。
- 内容キャンバスの住人は `ResidentContent`＝`GlyphRun | Image | Surface`（`canvas.rs:171-180`・`#[non_exhaustive]`・Image/Surface は型シームのみ）——**クリック可能な住人・選択肢 resident が無い**。hover/highlight 能力も無い（`region.rs` は幾何のみ）。
- emo-present `PresentCommand` は Show/Hide/Invalidate のみ（`command.rs:38-67`）＝**ハイライトを present 層で重ねる経路は無い**→ 反転描画は emo-text canvas 自前（1枚物合成の思想どおり・[[areka-emo-own-compositor-atlas]]）。
- メニューは `\_l[5em,2lh]`（em/lh 単位カーソル）で「閉じる/もどる」を字下げ配置する——**`\_l` の消費（単位換算＋カーソル適用）も無所属**＝本 spec が所有。

## Current State（2026-07-16 実装偵察）

- **供給側は cue-playback ブランチが建設中**: `CuePlayer` が `CueCommand::Choice{id,text}` を `pending_choices` バッファへ分離（要素型 `PendingChoice`＝`runtime.rs:53`・分離実体は tick 内 `:193-201`）し、`WaitForChoice` barrier で停止（`:65-71,231-235`）・`pending_choices()` 読み口（`:355`）・`resolve_choice()` 解除（`:279`）まで実装済み。**表示器が居ない**だけ。
- **描画基盤は完備**: emo-text の状態機械→レイアウト→D2D 描画→`TextSurface`（viewbox 済み）・`TextSlotView`（emo-present 予約スロット）・DirectWrite フォントメトリクス（em/lh 換算の材料）・`TextRegion` 幾何。
- **入力基盤は完備**: wintf event/hit-test ✅・バルーン窓は `GhostWindows.balloon_window(scope)` で実在（`spawn.rs:88-93,101-123`）。バルーン窓のクリック捕捉と AlphaMask の関係（バルーン枠の不透明域＝クリック可）は emo-present 実装済み領分。
- **fixture 実物**（メニューの実形・`menu.pasta:15/33/62`）: 選択肢は**テキスト行として縦に並ぶ**（`\n` 区切り・2〜4項目〔:62 は2項目〕）・**バルーン fixture は cursor.\* スタイルキーを明示指定**（`emo2-kakukaku/descript.txt:41-51`＝`cursor.style,square`・`cursor.brush.color` 105,25,25・`cursor.font.color` 白・`cursor.blendmethod,none`——ukadoc 上 cursor.\* は選択肢マーカーの**描画スタイルキー群**であり画像ではない）＝**M1 ハイライトの期待値は fixture 指定スタイル（指定色の矩形塗り＋文字色切替）が第一候補**（scope doc §4「矩形反転で代替可」は cursor.\* 未指定時の縮退として保持・design で ukadoc/SSP 実観察と突合し確定）。

## Desired Outcome

Choice cue（＋直後の WaitForChoice barrier）を受けた emo-text が**バルーン内容キャンバスに選択肢行を描画**し、hover で**選択肢ハイライト（fixture の cursor.\* スタイル準拠＝square 塗り＋文字色切替・未指定バルーンは矩形反転縮退）**、クリックで **`ChoiceSelection`（本 spec 正本の I/O 契約）を発行**する。`\_l[x,y]`（em/lh）のカーソル移動が選択肢配置に効く。

**✔ 観測（単一 pass/fail）**: 決定論（注入 cue・synthetic pointer・sleep 不使用）＝(a) Choice cue×3＋`\_l` cue 注入→canvas readback で選択肢3行＋字下げ配置の描画検証 (b) synthetic hover→ハイライト矩形（cursor.\* スタイル準拠）の pixel 檻（on/off 対） (c) synthetic click→`ChoiceSelection{scope, id, label, extras}` が mock 受け口へ1回だけ発行 (d) 選択肢外クリック→無発行 (e) Clear/新 talk→選択肢消滅＋ hit 領域無効化。＋実機＝実 emo2 でメニューが**見えて・光って・選べる**（選択後の遷移は choice-select-events の領分＝判定を混ぜない）。

## Approach

1. **選択肢 resident の導入**: `ResidentContent` へ Choice 系 variant を additive 追加（`#[non_exhaustive]` 済み）or「GlyphRun＋選択肢メタ（id・hit 矩形）」のレジストリ並置——**描画は既存 GlyphRun 経路を再利用**（選択肢もテキスト）し、**hit 幾何と id 対応を第一級で持つ**形を design で選定。`TextRegion` 幾何を流用。
2. **`\_l` 消費**: em/lh→物理 px 換算（DirectWrite メトリクス・emo-text の2空間契約〔image_size/スケール〕に整合）＋レイアウトカーソル移動。負値・省略の縮退は ukadoc 確認。
3. **hover ハイライト**: canvas 合成パス内で選択肢行のハイライト描画——**fixture が cursor.\* スタイルキーを明示指定済み**（`style,square`・`brush.color` 105,25,25・`font.color` 白・`blendmethod,none`＝指定色の矩形塗り＋文字色切替を第一候補）・未指定バルーン向けの矩形反転縮退も design で確定（scope doc §4）。hover 状態は UI スレッドの pointer move→emo-text actor への通知（`UiSender` 規約）——**再描画は差分（viewbox ダーティ矩形）に乗せる**（全域再描画へ退行しない）。
4. **クリック解決→ChoiceSelection 発行**: バルーン窓の pointer 入力→`TextSlotView` 経由で hit 幾何を照会→`ChoiceSelection` を channel で発行（消費者は choice-select-events が結線・本 spec は mock 受け口で観測完結）。**バルーン窓のクリックスルー**（選択肢行above は hit・枠外は透過）との整合を design で確認。
5. **ライフサイクル**: 選択肢は talk の一部＝`Clear`/`ClearAll`/新 talk で消滅・hit 領域も同時無効化（表示と hit の原子性＝emo-present AlphaMask 対の教訓と同型）。スクロールとの相互作用（選択肢行がスクロールで隠れる場合の hit 追従）は M1 最小＝emo2 メニューは短い（design で fixture 実測を檻に）。

## クロスユニット契約（並走を詰ませない事前考慮・2026-07-16）

- **`ChoiceSelection`＝本 spec が正本**: `{scope, id: String, label: String, extras: Vec<String>}` 級（`Send` 所有データ・借用なし・actor-foundation envelope に載る形）。**choice-select-events は消費のみ**（再定義しない）。片側未完でも mock で観測が完結する形（撫でクラスタの resolver 1個接続点と同型）。
- **choice cue 形は sakura-dialogue-tags が正本**＝本 spec は消費のみ。pending choices の受け取り口（`CuePlayer.pending_choices()` 直読み vs cue 配送で受ける）は cue-playback の settled 配送モデルに従い design で確定。
- **`Status: choosing` は choice-select-events の領分**（idle-talk が設計したヘッダ enum の口へ値を足す）——本 spec は「選択肢表示中」状態を照会可能にするだけ（choice-select-events が読む）。
- **cursor.\*／marker.\*／`\![*]`／`\__q` の実態整理（2026-07-16 検証で是正済み）**: **cursor.\* スタイルキーは fixture が明示指定＝M1 対象（本 spec のハイライト仕様の源）**／cursor 画像キー（`cursor,ファイル名`＝マウスカーソル用・別物）と marker.\* キーは未使用（marker.png ファイル自体は同梱・キー未宣言）／`\![*]`/`\__q` は dic に無し＝M1 外（型シームのみ）。design 冒頭で ukadoc と突合。
- **emo-present 本体は原則無改変**（balloon-face-cue/mayuna と同じ規律）: 描画は emo-text canvas 内で完結・`TextSlotView` の読み口増分が要る場合も additive。

## ukadoc 必読（design 着手時に ukadoc MCP `get_doc`/`search_docs` で正典参照・2026-07-16 裏取り）

- **`\q` 表示仕様**: `ukadoc:list_sakura_script:_5cq_5b_30bf_30a4_30c8_30eb_2cID_5d:1`（自動改行は `\__q` の領分と明記＝`\q` は改行しない・fixture も `\n` 手動区切り）。アンカー `\_a`（emo2 未使用・M1 外確認）。
- **`\_l[x,y]`**: em/lh 単位の正確な定義（フォント高基準か行高基準か）・負値/省略挙動。
- **`descript_balloon`**: `cursor.*` 全キー（**選択肢マーカーのスタイルキー群**＝style／brush.color／pen.color／font.color／blendmethod——fixture 実指定 `emo2-kakukaku/descript.txt:41-51` と突合。「`cursor,ファイル名`」はマウスカーソル画像で**別物**）・`validrect` 内での選択肢配置・「テキスト領域に効くキー」（emo-text-layer design の3分類表を再利用）。
- **具体指示**: design 冒頭で「選択肢の視覚仕様＝SSP de-facto（行単位反転・クリック領域は行全幅か文字幅か）」を ukadoc＋SSP 実観察で1つ確定し、pixel 檻の期待値として固定すること。

## Scope

- **In**: 選択肢 resident（描画＋hit 幾何＋id 対応）／`\_l` 消費（em/lh 換算＋カーソル）／hover 矩形反転（差分再描画）／クリック解決→`ChoiceSelection` 発行（契約正本）／Clear/新 talk での消滅＋hit 無効化の原子性／決定論檻（readback pixel＋synthetic pointer）／実機サインオフ（見える・光る・選べる）。
- **Out**: `\q`→cue コンパイル（sakura-dialogue-tags）／選択確定→SHIORI カスケード・タイムアウト・`Status: choosing`（choice-select-events）／cursor.* 画像ハイライト・`\_a` アンカー・`\__q`（M1 外・型シーム）／balloonc*（communicate UI・M2）／選択肢のスクロール完全対応（emo2 メニュー実測範囲で最小）。

## Boundary Candidates

- 選択肢 resident＋hit 幾何（純粋レイアウト＝全網羅可能）／hover/反転描画（pixel 檻）／ChoiceSelection 発行（結線・mock 観測）／`\_l` 換算（純関数）。

## Out of Boundary

- SHIORI への選択配送（choice-select-events・契約先決済み）／バルーン窓の生成・追従（placement 完了域）。

## Upstream / Downstream

- **Upstream**: **`areka-P0-cue-playback-duration`（時限ゲート・CuePlayer/pending_choices/配送 settled）**／**`areka-P0-sakura-dialogue-tags`（choice cue 形の正本・契約先決で並走可）**／`completed/areka-P0-emo-text-layer`＋`-viewbox`（canvas・差分再描画・TextSlotView）／`completed/areka-P0-emo-present`（バルーン target・スロット）／wintf event/hit-test ✅。
- **Downstream**: `areka-P0-choice-select-events`（ChoiceSelection 消費・カスケード）／`areka-P0-emo2-conformance-e2e`（メニュー一周の適合項目）。

## Existing Spec Touchpoints

- **Extends**: `completed/areka-P0-emo-text-layer`（`state.rs:186-191` の明示シームを実装で置換・既存 typewriter/scroll 決定論資産は不変）。
- **Adjacent**: `areka-P0-input-events`（バルーン窓 pointer 配線の流儀を共有——ゴースト窓＝input-events・バルーン窓＝本 spec で**窓が別＝衝突なし**・申し送り整合済み）／`areka-P0-emo-text-viewbox` ✅（差分再描画への相乗り）。
- **Consumes**: scope doc §4「cursor.* 省略可＝矩形反転代替」（`doc/emo2-conformance-scope.md`）。

## Constraints

- Rust 2024・新規 crates.io 依存なし・tokio 不使用・WUC/D2D 操作は UI スレッド固定（[[areka-wuc-runs-on-mta-thread]]）。
- **決定論**: 注入 cue＋synthetic pointer＋readback で全経路網羅（[[deterministic-test-coverage-mandate]]）。実フォントでの出力画像目視も併用（[[emo-text-byte-equiv-default-font-blindspot]]）。
- 表示と hit の原子性（片方だけ古い状態を作らない）。
- **実機受け入れ**: 実 emo2・実 pasta.dll・実 DPI（≠96）でメニュー表示＋hover＋選択の人間サインオフ（[[areka-placement-real-ghost-first]]）。
- 正典は ukadoc・emo2 は最小適合 fixture（[[ukadoc-mcp-preferred-source]]）。

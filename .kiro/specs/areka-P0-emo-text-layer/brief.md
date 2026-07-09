# Brief: areka-P0-emo-text-layer

> **種別**: 本坑（main）。⑥ emo トラック（M-boot 残・emo 直列3チェーン完了後の第4ユニット）。
> **調査日**: 2026-07-09（emo-present✅/sakura✅/balloon-parse✅/wintf text 資産の実シンボル偵察＋ukadoc `descript_balloon` 正典確認）。
> **前提依存（順序ゲート・充足済み）**:
> ```
> _Depends(confirmed): completed/areka-P0-emo-present（text_slot 予約・バルーン枠表示）／completed/areka-P0-sakura-engine（TextSink/TalkCue 出力契約）／completed/areka-P0-balloon-parse（テキスト領域モデル）
> ```
> **方針正本**: emo は UI 層全般＋テキスト進化路線（記憶 areka-emo-ui-layer-text-roadmap／roadmap「emo の責務範囲」節）——M1＝縦書き/横書き両対応＋行列変換領域の内部表現（実挙動は恒等/平行移動のみ）・M2 予約＝回転実挙動＋ポップアート文字装飾。

## Problem

sakura ✅ は Balloon 向け `TalkCue`（Text/NewLine/Clear）を発火し、emo-present ✅ はバルーン枠を表示して **text-layer 用スロット（`emo-text-layer-slot` Entity）を予約済み**だが、**文字を実際に描く層が存在しない**。M-boot の「emo2 が喋る」の可視部分＝バルーンにセリフが流れる、が未達。ghost-setup の `text_sink` も `LogSink`（ログのみ）が終端。

## Current State（2026-07-09 実シンボル偵察）

- **emo-present ✅ の予約スロット（本ユニットの差し込み先）**: `VisualMount::attach` が `world.spawn((Name::new("emo-text-layer-slot"), Visual::default(), ChildOf(window)))` で **surface entity の兄弟・上位 z の空 Visual** を予約済み（`mount.rs`）。ただし `VisualMount`・`text_slot()` は **`pub(crate)`**——**本ユニットが公開面増分（text_slot への到達手段）を所有**する（emo-present crate への additive 変更・保護規約参照）。
- **sakura ✅ の出力契約（正本消費・再定義しない）**: `pub trait TextSink { fn emit(&mut self, cue: TalkCue); }`（`SurfaceSink` と別 trait で 2 系統を型分離・infallible）。`TalkCue { at: f64（talk 起点相対秒）, actor: ActorKey, command: CueCommand }`・`cue_target_of` が Balloon へ分類するのは **Text／NewLine／Clear／Choice**（Choice は M-dialogue の choice-render 領分＝本ユニットはシームのみ）。
- **seriko ✅ の先行実例（sink→actor パターンの donor）**: `SerikoSink`（`SurfaceSink` 実装・`tx.send(SerikoMsg::Cue)` で自 inbox へ橋渡し）＋`spawn_seriko(resolver, static_binds, out)`——**TextSink 実装アクターの構造はこれの写し**でよい（描画は UI スレッド＝`spawn_ui`/`UiSender` ブリッジ経由が seriko との差分）。
- **balloon-parse ✅ のテキスト領域モデル（正本消費＋additive 増分1点）**: `areka_parsers::balloon` に `Origin`（文字描画原点 x,y）・`WordWrapPoint`（折返し点・負値＝反対辺基準）・`ValidRect`（テキスト有効矩形 top/bottom/left/right・負値＝反対辺基準）・`Font`（name/height/color・各成分独立 `Option`）・`FontColor`（r/g/b）が実装済み——**領域・フォント定義はこのモデルを消費**（emo-present brief の申し送り「テキスト領域キーは emo-text-layer が消費」の実行）。**`descript.txt`＋画像別 `balloons*s.txt`/`balloonk*s.txt` の後勝ち2層マージも実装済み**（emo2-kakukaku fixture に両ファイル実在・R5.2/5.3 で検証済み）——`areka.writing-mode` は**このマージに新キー1個を additive で追加**するだけ（model へ `Option` フィールド増分・parser は転記に徹し解釈は本層＝記憶 areka-parser-transcribes-tree-downstream）。
- **wintf テキスト資産（lift 候補・`wintf::ecs::widget::text`）**: `TextDirection` **4 方向**（Horizontal LTR/RTL・**VerticalRightToLeft**（日本語縦書き）・VerticalLeftToRight）＝DirectWrite `SetReadingDirection(TOP_TO_BOTTOM)`＋`SetFlowDirection(RIGHT_TO_LEFT)` で実現済み／`Typewriter`（font_family/font_size/direction/**default_char_wait**）・`TypewriterTalk`・`TypewriterLayoutCache`（`IDWriteTextLayout` 保持）・IR（`TypewriterTimeline`/`TypewriterToken`/`TypewriterEvent`）・描画 system 群（`draw_typewriters` 等）。**逐次表示（typewriter 進行）の機構は既に wintf にある**——lift（emo 側へ再配置）か参照消費かは design 判断。
- **ghost-setup ✅ の結線口**: `GhostBootOptions.text_sink: T where T: TextSink + Clone + Send + 'static`（構築時注入・setter なし）——本ユニットの sink をここへ挿すのは **emo2-boot（M-boot 統合）**の領分（本ユニットは sink 型を作るまで）。

## Desired Outcome

sakura の Balloon 向け cue を受けて、**バルーン surface の上（予約スロット）に文字を描画する層**。縦書き/横書き両対応・typewriter 逐次表示・改行・領域あふれ時のスクロール。描画先は**行列変換領域の内部表現**（M1 実挙動は恒等/平行移動のみ・M2 で回転/装飾を解禁する構造）。

**✔ 観測（単一 pass/fail）**: 専用 example が emo2 fixture のバルーン枠（`build_balloon_target` ✅）上に、fixture スクリプト由来の `TalkCue` 列（Text/NewLine/Clear）を**注入時刻駆動**で流し、(a) 文字が typewriter 進行で描画される（b) `NewLine` で改行・`ValidRect` あふれでスクロール（c) **縦書き/横書きが `areka.writing-mode` マーカー（descript／画像別 `balloons*s.txt`・`balloonk*s.txt`）の宣言で切り替わる**（マーカー無し＝`horizontal-tb` 既定・fixture 側へのマーカー付与は design で確定）（d) `Clear` で全消去。レイアウト決定論部分（折返し位置・行送り・スクロール発火・**マーカー解決の2層マージ**）は **DirectWrite metrics に依存しない構造テスト**＋既定フォントでの単体テストで檻に入れる。

## Approach

1. **TextSink 実装アクター（SerikoSink パターンの写し）**: `TextSink::emit` → 自 inbox → **UI スレッドへは `spawn_ui`/`UiSender` で配送**（WUC/D2D 描画は UI スレッド固定・並行モデル正本）。Close/全断線で clean 終了（areka-actor 5 規約準拠）。
2. **テキスト状態機械（純粋層・単体テスト可）**: cue 列→「表示中テキストの行/グリフ状態」への純粋遷移（append/newline/clear/scroll 判定）。DirectWrite 非依存の構造で先に檻を作り、実描画はその状態の写像に徹する。
3. **レイアウト＝DirectWrite**: `Font`（balloon descript 由来・欠落は SSP 既定＝ＭＳ ゴシック）→ `IDWriteTextLayout`。縦書きは wintf 実証済みの ReadingDirection/FlowDirection 組合せを lift。
4. **縦書き opt-in＝areka 拡張キー `areka.writing-mode`（2026-07-09 開発者要望）**: バルーン定義で縦書きを宣言するマーカーを導入する。**値は CSS `writing-mode` 語彙を借用**——`horizontal-tb`（既定・SSP 互換）／`vertical-rl`（日本語縦書き）／`vertical-lr`。wintf `TextDirection` と 1:1 写像（`horizontal-tb`→`HorizontalLeftToRight`・`vertical-rl`→`VerticalRightToLeft`・`vertical-lr`→`VerticalLeftToRight`）。**置き場は balloon-parse ✅ の実装済み2層マージに乗せる**: `descript.txt`（バルーン全体既定）＜ **`balloons*s.txt`/`balloonk*s.txt`（画像別上書き・後勝ち）**——emo2-kakukaku fixture に両ファイル実在・マージ機構は流用のみ。**`areka.` 名前空間必須**（ukadoc/SSP 非標準の areka 拡張キー＝将来の SSP キーとの衝突回避。SSP は未知キーを無視するためバルーンの SSP 互換は壊れない）。未知値は warn＋`horizontal-tb` フォールバック（寛容・log-first）。CSS の `text-orientation`（欧文の向き）・`text-combine-upright`（縦中横）は **M2 予約キーとして型シームのみ**（`areka.text-orientation`/`areka.text-combine-upright` を予約名として記録・実装しない）。
5. **行列変換領域の内部表現**: 描画先を「矩形」でなく**変換行列付き領域**として持つ（surface 合成の行列原則と同型）。M1 の実挙動は恒等/平行移動のみ・回転値と装飾（アウトライン/多色/シャドウ）は**型シームのみ**（M2 予約への接続点）。
6. **スロットへの描画供給**: 予約スロット（`emo-text-layer-slot` Visual）へ描画内容を装着する公開経路を emo-present に**additive で増設**（`text_slot` 到達手段の公開 or 装着 API——最小の公開面は design 判断）。surface 本体の再合成を**強要しない**（毎グリフ更新が emo-compose を再駆動しない＝独立レイヤの本旨）。
7. **typewriter 進行の時刻規律**: per-glyph 進行は本層が所有（wintf `default_char_wait` 相当）・**cue の `at` は chunk 開始時刻**。時刻は sakura と同じ**注入時刻駆動**（実時間 sleep 不使用）で決定論テスト可能に。

## クロスユニット契約（後続を詰ませない事前考慮・2026-07-09）

- **⚠️ per-glyph 進行と sakura cue 時刻の整合（design 冒頭で確定）**: SSP 実挙動ではテキストの逐次表示中も script 再生は逐次（文字送りが後続タグを遅らせる）だが、**sakura ✅ の cue `at` は `\w` 系 wait のみで累積**しており text 長を考慮していない可能性が高い——**実装を確認**の上、M-boot は「text-layer 側 pacing が cue 時刻に影響しない」前提で開始してよい（emo2 boot script で破綻しない範囲を fixture で確認）。厳密な SSP 互換 pacing（文字送りが下流 cue を押す）が必要と判明した場合は **sakura への増分 issue として申し送り**（本ユニットで sakura を改変しない）。
- **Choice 表示（M-dialogue）への継承**: `\q` 選択肢は choice-render の領分だが、**行レイアウト・領域定義・スロット装着の公開形は choice-render が再利用できる形**に切る（テキスト行の「クリック可能範囲」を返せる構造シームだけ用意・実装しない）。
- **文字装飾タグ（`\f` 系）**: emo2 使用分を design で fixture 実測し、未使用なら**型シームのみ**（`disable.font.*` 等の SSP 拡張も同様）。
- **バルーン推奨 DPI**: `descript_balloon` の **`dpi,推奨DPI`**（SSP 2.7.21+・省略時 96 固定）——文字サイズのスケール解釈に効き得る。M1 は 96 前提素通しで可かを design で1判断（window-placement brief にも同キー注記あり・整合させる）。
- **並走保護規約（window-placement と同時着手・07-09 拡張キー追加で更新）**: 本ユニットは **`crates/areka`（main.rs・placement 系）を触らない**。`crates/areka-emo-present` への変更は **text_slot 公開増分（additive）のみ**・`crates/areka-parsers` への変更は **balloon model の `areka.writing-mode` 転記フィールド増分（additive）のみ**。あちら（placement）は emo-present／areka-parsers のどちらも改変しない。衝突面ゼロで並走可。
- **ghost-setup への sink 結線は emo2-boot の領分**: 本ユニットは `TextSink + Clone + Send + 'static` を満たす sink 型を作るまで（`GhostBootOptions.text_sink` への注入・実 talk 経路の結線は M-boot 統合）。

## ukadoc 必読（design 着手時に ukadoc MCP `get_doc`/`search_docs` で正典参照・2026-07-09 確認）

- **必読**: `descript_balloon` の**テキスト描画系キー全量**——`font.name`（既定ＭＳ ゴシック・カンマ区切り複数指定は SSP 拡張）・`font.height`・`font.color.*`・`disable.font.*`（`\f[disable]` 用・SSP 2.5.51+）・`origin`/`wordwrappoint`/`validrect` 系（balloon-parse モデルの典拠）・`dpi`（推奨 DPI）。emo-present brief の申し送り「**枠描画/テキスト領域/M1 対象外の3分類表**」を本ユニットの design で完遂すること（emo-present は枠のみで完了した）。
- **必読**: `list_sakura_script` のテキスト系タグ——`\n`（改行・`\n[half]` 有無）・`\c`（クリア）・`\f[...]`（フォント指定）・`\_l`/`\_w`（座標/wait 系は sakura 済みか確認）。**emo2 boot script の実使用タグを fixture 実測**し、M1 実挙動の subset を確定（未使用はシーム）。
- **brief 未網羅→design で埋める項目**: ① 縦書き時の `origin`/`wordwrappoint`/`validrect` の軸解釈（縦書きで x/y がどう回るか——ukadoc に明記が無ければ SSP de-facto を確認）② スクロールの正確な挙動（行単位か・アニメ有無・SSP de-facto）③ バルーン切替（`\b`）時のテキスト状態の扱い（M-boot 対象外の見込み・確認のみ）。

## Scope

- **In**: TextSink 実装アクター＋UI 配送／テキスト状態機械（純粋層）／DirectWrite レイアウト（縦書き/横書き・折返し・スクロール）／**`areka.writing-mode` 拡張キー**（CSS 語彙値・descript＋画像別2層マージ・parser 転記フィールドの additive 増分含む）／typewriter 逐次表示（注入時刻駆動）／行列変換領域の内部表現（恒等/平行移動）／balloon descript 由来の Font/領域解決／text_slot への装着経路（emo-present additive 増分）／専用 example。
- **Out**: 選択肢表示（**choice-render**・M-dialogue）／`\f` 装飾の実挙動・回転テキスト・ポップアート（**M2**・型シームのみ）／`areka.text-orientation`・`areka.text-combine-upright`（縦中横）の実挙動（**M2**・予約名の記録のみ）／sink の main 結線（**emo2-boot**）／sakura の cue 時刻改変（増分申し送りまで）／バルーン枠の描画（**emo-present** 済み）／communicatebox 系（M2）。

## Boundary Candidates

- テキスト状態機械（純粋・cue→行/グリフ状態）／レイアウト（DirectWrite 写像）／装着（スロット供給・emo-present 増分）の三片。
- sink（受信端・actor）と描画（UI スレッド）の境界＝UiSender。

## Out of Boundary

- バルーン窓の生成・配置（window-placement）／surface 合成（emo-compose）／枠表示・キャッシュ（emo-present 本体）。

## Upstream / Downstream

- **Upstream**: `completed/areka-P0-emo-present` ✅（予約スロット・枠表示・`build_balloon_target`）／`completed/areka-P0-sakura-engine` ✅（TextSink/TalkCue 契約正本）／`completed/areka-P0-balloon-parse` ✅（領域・Font モデル正本）／`completed/areka-P0-actor-foundation` ✅（`spawn_ui`/`UiSender`）／wintf text 資産 ✅（Typewriter/TextDirection/DirectWrite 縦書き）。
- **Downstream**: **`areka-P0-emo2-boot`（M-boot 統合・本 sink を `GhostBootOptions.text_sink` へ注入）**／`choice-render`（M-dialogue・行レイアウト/クリック範囲シームの再利用）／M2 text effects（行列領域＋装飾シームへ接続）。

## Existing Spec Touchpoints

- **Extends**: `completed/areka-P0-emo-present`（text_slot の公開増分・additive のみ）／`completed/wintf-P0-typewriter`・wintf text 資産（lift or 参照）。
- **Adjacent**: `areka-P0-window-placement`（**並走中の想定**——保護規約: こちらは areka-emo-present 増分・あちらは crates/areka＝非交差）／`areka-P0-choice-render`（M-dialogue・シーム継承先）。

## Constraints

- Rust 2024・`windows` 0.62.2・tokio 禁止。**描画は UI スレッド固定**（WUC/D2D・MTA＋`DQTAT_COM_NONE`＝記憶 areka-wuc-runs-on-mta-thread）・sink 受信は worker（UiSender で配送）。
- **決定論テスト網羅**（記憶 deterministic-test-coverage-mandate）: 状態機械・スクロール発火・時刻進行は注入時刻駆動で実行テスト化（sleep 不使用）。DirectWrite metrics 依存部は構造テストで分離。
- 最小実装＋薄い拡張シーム（emo2 使用タグのみ実挙動・行列/装飾は型シーム）。正典は ukadoc・emo2 fixture は最小適合サンプル。
- 既知ドリフト追随: `crates/areka-parsers/src/balloon/model.rs:6` の doc コメントが旧名 `text-layer`/`surface-engine` を参照——本ユニット着手時に固有名（emo-text-layer/emo-compose）へ修正（roadmap 注記済みの宿題）。

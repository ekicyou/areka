# Brief: areka-P0-kero-balloon

> **Discovery 実施**: 2026-07-24・sylphya 実装セッション（worktree `claude/areka-p0-sylphya-fc9b81`）にて。
> sylphya Task 10.3 実機サインオフ中に開発者が実画面で発見（kero＝エモ側バルーンがさくらスクリプト指定と異なる）。**実施は別セッション**——本ブリーフが引き継ぎ正本・調査は本セッションでコード実読済み（下記は全て path:line 根拠付きの確定事実）。

## Problem

kero（相方・エモ）側のバルーンが、バルーン定義の指定どおりに表示されない。emo2-kakukaku バルーンは **kero 専用の `balloonk0.png`＋`balloonk0s.txt`**（windowposition -190,-75／validrect 40,-70,24,-48＝sakura 側と別物）を持つのに、実機では **kero も sakura 用 `balloons0.png`＋`balloons0s.txt` の見た目・配置で表示**される（sylphya 10.3 サインオフのスクリーンショットで両バルーンが同一形状なのが観測そのもの）。

**症状の切り分け（重要）**: 疑われた「`\b[ID]` バルーン切り替え未対応」は**シロ**——`\b` 面切替は `completed/areka-P0-balloon-face-cue` が parser→compile→cue→seriko→adapter→PresentCommand まで一気通貫で実装済み（spine S3 が実描画+readback で檻化）。**クロなのは「スコープ別バルーン資産の選択」**＝kero が `balloonk*` を使う経路が存在しないこと。

## Current State（コード実読・確定事実）

1. **画像列挙が sakura 側固定**: `crates/areka-emo-present/src/balloon.rs:39` `FRAME_PREFIX = "balloons"`・`:88-93` `frame_id` は `balloons{N}.png` のみ受理。`:264` のテストが `frame_id("balloonk0.png") == None` を**明示的に固定**（＝balloonk は設計で除外中）。
2. **全 scope が同一 balloon 資産**: `crates/areka/src/emo2_boot/assets.rs:240-243` は scope ループで**毎回同じ `balloon_root` から `build_balloon_target`** を呼ぶ（scope 分岐なし）→ scope1（kero）も `balloons0.png` を得る。
3. **BalloonModel も 1 個共有**: `assets.rs:79` `BALLOON_FACE0_TXT = "balloons0s.txt"` ハードコード・`build_balloon_model` は 1 回だけ呼ばれ（`assets.rs:244,262-266`）、`frame.rs:452-457` `connect_balloon_text` が**全 scope の register_actor_view に同じ model** を渡す。`balloonk0s.txt` は**実行時に一切読まれない**（emo2-boot design.md:447 に「全 scope 共有」と設計時から明記＝意図的な M-boot 縮退）。
4. **placement 採寸も同前提**: `crates/areka/src/placement/measure.rs:208-229,380` が `balloons0.png` 名前固定・「全スコープ共通」前提 → kero が別サイズ枠を持つ場合 **kero 窓の採寸も誤る**。
5. **パーサ層は対応済み**: `areka-parsers::balloon` は `balloonk0s.txt` のマージ解析を R5.3 で檻化済み（`validation_tests.rs:126-146`）——**資産はパース可能・配線が捨てているだけ**。
6. **`\b` 経路（シロの証拠）**: decode.rs:142,204 → compile.rs:87-92 `CueCommand::BalloonSurface{key}` → sink.rs:58 `CueTarget::Shell` → areka-seriko actor.rs:364-392（resolve→apply→ShowBalloon/HideBalloon・冪等ガード）→ adapter.rs:54-64 → frame.rs:516-528 drain。spine.rs:800-903（S3）が `\b[-1]`→Hide／`\b[0]`→ShowSurface を実 readback で檻化。spine.rs:907-910（S4）は「`\b` 不在時に balloon 宛指令が leak しない」檻であり切替否定ではない。

## Desired Outcome

1. **kero（scope1）のバルーンが正典どおり `balloonk0.png`＋`balloonk0s.txt` で表示・配置される**（emo2-kakukaku 実機で sakura と異なる枠・windowposition が目視確認できる）。
2. **ukadoc フォールバック準拠**: `balloonk*` が**無い**バルーンでは kero が本体側 `balloons*` の対応 ID へ縮退する（正典「省略時は本体側の対応する ID のものが使われる」——現行挙動はこのフォールバック時と同じ見た目なので後方互換）。
3. placement 採寸が scope 別バルーン実寸で行われ、kero 窓配置が `balloonk0s.txt` の windowposition/validrect を反映する。
4. `\b` 面切替の既存配線（balloon-face-cue の檻）を**一切壊さない**（S3/S4 緑維持）。
5. 決定論檻: scope→prefix 選択（sakura=`balloons`/kero=`balloonk`）・フォールバック分岐・per-scope BalloonModel マージ（balloonk0s の windowposition 実値）を x64 純粋テストで檻化＋実機目視サインオフ。

## Approach（候補・最終確定は requirements/design で）

- **A. scope→prefix パラメタ化＋per-scope BalloonModel（本命）**: `balloon.rs` の FRAME_PREFIX/`frame_id` を prefix パラメタ化（`balloons`/`balloonk`＋フォールバック列挙）、`assets.rs` で scope ごとに `build_balloon_target`/`build_balloon_model` を選択構築し `ScopeAssets`（or per-scope フィールド）へ、`connect_balloon_text`/placement を scope 別 model 消費に配線替え。Pros: 正典準拠の最小増分・既存 `\b` 配線無改変。Cons: BootAssets/frame の配線変更が数箇所に及ぶ。規模: 中（小寄り）。
- **B. 資産合成で偶奇 ID マップ（balloons0→ID0, balloonk0→ID1 等の単一 World 合成）**: 単一 balloon World に両画像を面 ID で同居させ、scope 側は初期面 ID だけ変える。Pros: World 構築 1 本のまま。Cons: 正典のファイル名規約（balloons/balloonk は**別系列**・各々 0..N を持つ）と ID 空間が衝突・`\b[N]` の N の意味が歪む＝正典乖離リスク。規模: 小-中。
- **C. conformance-e2e に吸収（spec を切らず検証先行）**: `areka-P0-emo2-conformance-e2e` #10 の検証で gap 顕在化→その場で直す。Pros: spec 数を増やさない。Cons: conformance は「検証で構造的 gap が出たら **JIT の個別 spec を切る**」と brief 自身が予告しており（brief.md:58）、本件はまさにその構造的 gap＝独立 spec が予告どおりの筋。規模: —。
- **推奨**: **A**。正典のファイル名規約に素直で、`\b` の面 ID 意味論を保ち、balloon-face-cue の完成領域に触れない資産選択レイヤの増分だから。

## Scope

- **In**: scope→バルーン prefix 選択（sakura=`balloons*`/kero=`balloonk*`）・balloonk 欠落時の balloons フォールバック（正典準拠）・per-scope `BalloonModel`（`balloonk0s.txt` 読取）・placement 採寸の scope 別化・決定論檻＋実機目視
- **Out**: `\b` 面切替経路の変更（完成済・無改変）・`\p[2]` 以降の `balloonp*` フォールバック連鎖（M1 は二人立ちまで）・偶数/奇数=左右向きセット意味論・バルーン可視性ライフサイクル（`areka-P0-balloon-visibility` の領分）・多面バルーン（emo2 は各系列 1 枚）

## Boundary Candidates

- `areka-emo-present/balloon.rs`（画像列挙の prefix パラメタ化＝最下流の純関数増分）
- `areka/emo2_boot/assets.rs`（per-scope 資産構築・`ScopeAssets`/`BootAssets` の形）
- `areka/emo2_boot/frame.rs`（`connect_balloon_text` の per-scope model 配線）
- `areka/placement/measure.rs`（scope 別採寸）

## Out of Boundary

- seriko/`\b` cue 経路（balloon-face-cue 完成領域）・balloon show/hide 頭脳（balloon-visibility）・sakura スクリプト解釈・wintf

## Upstream / Downstream

- **Upstream**: `completed/areka-P0-emo2-boot`（現・全 scope 共有設計の導入元）／`completed/areka-P0-balloon-face-cue`（`\b` 配線・無改変で保護）／`areka-parsers::balloon`（R5.3 で balloonk0s 解析済＝そのまま使える）
- **Downstream**: `areka-P0-emo2-conformance-e2e` **#10**（kero 一式検証——本 spec が前提を充足）／`areka-P0-balloon-visibility`（同じ balloon 表示面の隣接・編集面の重なりに注意）／W4 position-persist（バルーンオフセット永続と kero windowposition の相互作用）

## Existing Spec Touchpoints

- **Extends**: なし（新規境界。conformance-e2e brief.md:58 が予告する「構造的 gap への JIT 個別 spec」に該当）
- **Adjacent**: `areka-P0-balloon-visibility`（brief 済・未着手——**同時期に frame.rs/バルーン面を触る可能性**があり、着手順 or ウェーブ分離の裁定が要る〔記憶: 最大並列よりウェーブ直列〕）／`areka-P0-emo2-conformance-e2e`
- **Roadmap 補正候補**: roadmap の「kero 窓/バルーン/target 偶奇は M-boot 充足済み」記述は balloonk 表示に関して実態と不一致——本 spec 立ち上げ時に追記訂正すること。

## Constraints

- ukadoc 正典準拠（`balloonk*`=相方側・省略時は本体側 ID へフォールバック・`balloons*s.txt`/`balloonk*s.txt` は対応面の descript 上書き）。正典沈黙箇所は areka 裁量＋`doc/COMPAT_ARCHITECTURE.md` 対応表記録。
- `\b` 既存檻（spine S3/S4）green 維持が受け入れ条件。決定論テスト必達・実機確認は emo2-kakukaku（balloonk0 あり）で「sakura と kero の枠が異なる」目視。
- 実機起動は **ghost/balloon root を絶対パスで**（記憶 `areka-emo2-signoff-needs-absolute-paths`——相対パスだと pasta.dll LOAD が 0x8007007E）。

## セッション引き継ぎ（別セッション実施用の記憶）

- **観測の原点**: sylphya 10.3 サインオフ（2026-07-24）の実機スクリーンショットで、むらさき（sakura）とエモ（kero）のバルーンが同一形状＝balloonk 不使用を開発者が発見。撫で talk 再現は「エモの頭/胸を撫でて Head1通常/Bust1通常」（`dic/touch.pasta`）。
- **実機実走の定型**: helper を `target/debug/shiori-host32-helper.exe` へ配置（i686 ビルドは PowerShell）→ `$env:AREKA_APP_SMOKE_EXIT_MS="180000"; $env:RUST_LOG="info"; & target\debug\areka.exe <絶対ghost> <絶対balloon>`。fixture=`crates/pilot/examples/shiori-host-32/fixtures/emo2`（balloon は `…/emo2-kakukaku`）。
- **本ブリーフの所在**: sylphya worktree ブランチ `claude/areka-p0-sylphya-fc9b81` 上。新セッションが main 分岐なら sylphya PR マージ後に可視（メモリ `areka-p0-kero-balloon-discovery` からも辿れる）。
- **同ブランチ上の未マージ隣接物**: sylphya 全実装（30 タスク・サインオフ済）＋ `wintf-gpu-test-crash` brief。`cargo test --workspace` は wintf GPU クラッシュ（境界外・pre-existing）で赤——本 spec の DoD 判定時も同注記が要る（メモリ `wintf-gpu-test-crash-discovery`）。
- **調査の残根拠**: 本ブリーフ Current State の path:line は 2026-07-24 時点の実読値。着手時に `git log -- crates/areka-emo-present/src/balloon.rs crates/areka/src/emo2_boot/assets.rs` で陳腐化チェックを（記憶: 並走 brief は陳腐化する・design 前に実測再突合）。

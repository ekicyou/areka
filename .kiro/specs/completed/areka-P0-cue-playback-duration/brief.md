# Brief: areka-P0-cue-playback-duration

> 由来: 2026-07-13 M-boot（`areka-P0-emo2-boot`）R9.3 実機サインオフの実機欠陥 #3/#4（タイプライターのウェイト不発・改行早発）を**実 pasta スクリプト接地で深掘り**した結果、症状の裏に潜む**アーキテクチャ根本欠陥**として発見。開発者が `/kiro-discovery` で「あちこちのウェイト計算が破綻の元」と喝破し、**案A（duration 付き cue・三権分立）でじっくり spec 化**と決定（2026-07-13）。**登録済み増分 `areka-P0-sakura-glyph-pacing` を吸収・撤回**（同増分は「④sakura のみ」「句読点自動ポーズ」という誤った狭いフレーミングだった）。

## Problem

さくらスクリプトの**テキスト再生には時間がかかる**（1文字あたり既定 50ms の暗黙ウェイト＋明示 `\_w[ms]`）。ところが areka の cue タイムラインは**この再生時間を一切モデル化していない**。結果、テキストを喋り終わる前に後続 cue（次テキスト・`\s` 表情切替・`\n` 改行）が発火し、実機で以下の綻びとなって現れる（R9.3 実機観測）:

- **#3 ウェイト不発**: スクリプトの `\_w[450]` 等が typewriter に効いて見えない（「、」「。」で止まらないのは、たまたまそこに `\_w` が置かれているだけ＝句読点は無関係。**開発者が「句読点自動ポーズ」仮説を明確に棄却**——ウェイトの源は `\_w` のみ・文字送りは一律 50ms）。
- **#4 改行早発**: 1行表示直後に必ず改行される（`\n` 直前の `\_w` が無視される）。#3 と**同一根**。
- **#6 新 talk で前の会話が消えない**: 新しい会話が始まっても前 talk のテキストがバルーンに残り累積する。SSP は**新 talk でバルーン自動クリア**が既定だが、areka は **talk 境界で Clear cue を誰も発火していない配線欠落**（Clear 機構自体は emo-text に実在）。**開発者裁定（2026-07-13）: ④sakura が担当＝compile で台本冒頭へ Clear cue を前置**する（「台本を書くのが sakura」の一部＝台本は Clear で始まり durations を載せる）。
- **副次: `\s` 表情非同期**: 表情切替がテキスト再生と無関係なタイミングで発火（喋りと表情がズレる）。

> **本 spec が束ねる7件中の位置**: 実機指摘 7件のうち #5 は emo2-boot でこの場修正済み・#1 は `surface-resize-resnap`・#2 は `mayuna-compose`・**#7（冒頭 1.5行空行）は pasta 側の生成癖と判定＝areka スコープ外→上流 ekicyou/pasta へ起票**。残る **#3・#4・#6 が本 spec**（sakura→cue→emo-text の talk 再生パイプライン正しさ＝三権分立で解決）。

**より根本的な病理（開発者の指摘）**: 「文字数→文字単位ウェイト量」を計算する純関数が**あちこちで独自実装**されている。この duplication があるため、タイムライン（cue 発火時刻）と reveal（typewriter 表示）が**協調せず破綻**する。単一の権威が「このテキストの再生には XXX 秒かかる」を保持していない。

## Current State

**実 pasta スクリプト接地で仕分け済み（2026-07-13・drive.rs 一時診断ログで生スクリプト捕捉）——②parsers も compile も無罪、破綻は時間モデルの不在:**

- **dola（下位タイミング層）＝ cue は「点」**: `Cue { actor, start_time: f64, payload }`（`crates/dola/src/cue/command.rs:186-195`）。`CueCommand::Text(String)` は**テキストのみで duration の概念なし**。`TimedSchedule` は start_time の一点で発火し次へ進む＝**テキストが時間を占有する概念が存在しない**。
- **sakura compile ＝ テキストを 0 時間扱い**: `compile.rs` は `Instruction::Wait`（明示 `\_w`）**のみ** offset を進め、`Instruction::Text` では offset を進めない（`compile.rs:38-40, 46-48`）。ゆえに後続 cue の start_time が**テキスト再生時間を含まず前倒し**になる。parse は正常（`\_w[450]`→`Wait(450ms)`・`\n[150]`→`NewLine`）、compile も明示ウェイトは正しく累積（実測: cue at=0.0→0.45→0.90→1.85）。**欠けているのは暗黙の 1文字=50ms 分だけ**。
- **char_wait の独自実装が最低2箇所**:
  - `areka-emo-text` `TextLayerConfig.char_wait = 0.05`（`state.rs`）＋独自の `r_i = max(r_{i-1}+char_wait, at)`。
  - `wintf` `Typewriter.default_char_wait = 0.05 // 50ms`（`ecs/widget/text/typewriter/mod.rs:58`）＋独自の `current_time += default_char_wait`（`typewriter_layout.rs:209`）。
  - いずれも cue タイムラインの権威ではなく、互いに協調しない。
- **誤った対症療法（emo2-boot ブランチ由来・⚠️2026-07-13 実コード再確認で「既に不在」）**: M-boot 実装中に `punctuation_wait`（句読点で 0.3秒 自動ポーズ）を emo-text へ追加したが**開発者が明確に棄却**（句読点で変える筋合いはない・ウェイトは `\_w` のみ）→ emo2-boot 完了時に revert 済み（roadmap 追記㉒ の誤 #3/#4 修正 `84152a40` revert）。**現ブランチのソースには `punctuation_wait` も drive.rs 生スクリプト診断ログも存在しない**（`crates/**` grep でゼロ・.kiro 文書のみヒット＝実コード偵察 2026-07-13 確認）。→ **本 spec の「撤去必須」前提は既充足**（着手時に再 grep で確認するのみ・撤去作業は発生しない見込み）。

## Desired Outcome

**cue タイムラインが「テキスト再生に XXX 秒かかる」を第一級情報として保持する単一の権威台本となり、文字ウェイト計算の純関数が唯一化される。** その結果:

- テキスト再生時間が cue タイムラインに正しく載り、**後続 cue（次テキスト・`\s`・`\n`）が「喋り終わってから」発火**する（#3/#4/#D＋表情同期が**副産物として一挙解決**）。
- 「文字数→再生時間」の純関数が**1つだけ**存在し、タイムライン側も reveal 側も**同じ1つの真実源**から導出する（duplication 絶滅）。
- 決定論（注入時刻駆動）は維持（実時間 sleep/`Instant` 不使用）。

## Approach

**案A: duration 付き cue の三権分立（開発者決定 2026-07-13）。**

> **dola が単一の権威台本、台本を書くのが sakura、台本に従うのが emo-text。**

- **保持（hold）＝ dola**: cue タイムラインの唯一の真実源。テキスト cue が**「再生に D 秒かかる」を第一級プロパティとして持つ**（`Cue`/`CueCommand::Text` へ duration を付与、または schedule が duration を認識）。dola は汎用の演出基盤ゆえ **50ms をハードコードせず**、duration を**データとして**受け取り、後続 cue を D 秒後に整列させる。
- **計算（compute・台本を書く）＝ sakura**: 「1文字=50ms」は**さくらスクリプトの意味論**＝sakura の領分。純関数 `text_playback_duration(text) → Duration`（暗黙 per-char ノミナル＋明示 `\_w` 換算）を**一度だけ**実装し、char_wait 定数も sakura に一元化。compile が各テキスト cue へ duration を書き込み、タイムラインを整列させる。**加えて台本冒頭へ `Clear` cue を前置**（#6・新 talk＝バルーン自動クリア＝「完全な台本」の一部）。
- **服従（obey・台本に従う）＝ emo-text**: 自前 char_wait を捨て、**渡された D で再生**する（N文字を D秒に割る＝結果は 50ms/字だが源は1つ）。reveal は台本が定めた duration に従うだけ。

**なぜ案A（vs 案B レンダラ報告／案C 事前焼込み）**:
1. さくらスクリプトの文字ウェイトは**レイアウト非依存の固定ノミナル**（折返しや `\n` で変わらない）＝`char_count × 50ms + Σ\_w` で**上流が厳密に計算可能**。案B（レンダラしか真の時間を知らない前提のフィードバックループ）は SakuraScript では**不要な複雑さ**。
2. duration を cue の第一級プロパティにすれば **dola が正直になる**（開発者の要望「dola が根本情報を把握せねば破綻」を満たす）＋純関数を1つに集約＝duplication 絶滅。
3. 案C（sakura が start_time 間隔へ焼込むだけ・dola は点のまま）はバグは消えるが **dola が duration を知らぬまま**＝「dola が単一権威台本であるべき」の思想に反するため棄却。

## Scope

- **In**:
  - dola cue タイムラインへの**テキスト再生 duration の第一級モデル化**（`Cue`/`CueCommand::Text` の duration 表現・schedule の duration 認識・後続 cue の整列）。
  - **単一の純関数** `text_playback_duration`（暗黙 per-char ウェイト＋明示 `\_w` 換算）と char_wait 定数の**一元化**（所在は sakura）。
  - sakura compile が duration を cue へ書き込む（テキスト再生時間をタイムラインへ加算）。
  - **sakura compile が台本冒頭へ `Clear` cue を前置**（#6・新 talk でバルーン自動クリア）。
  - emo-text reveal が**渡された duration に服従**（自前 char_wait 計算を撤去）。
  - 実機受け入れ: #3（`\_w` が pause として体感できる）・#4（`\n` が `\_w` 分だけ遅れる）・#6（新 talk で前会話が消える）・`\s` 表情同期。
  - **emo2-boot ブランチの後始末**（前提条件・**現ブランチでは既充足**＝上記 Current State 参照＝着手時に再 grep 確認のみ）: `punctuation_wait` ハック・drive.rs 一時診断ログの不在確認。
- **Out**:
  - bind/mayuna 合成による表情変化（#2＝`mayuna-compose`）。
  - 実行時サーフェスサイズ変化→窓リサイズ/再吸着（#1＝`surface-resize-resnap`）。
  - テキストの**レイアウト/描画**そのもの（縦書き・折返し・フォントメトリクス）＝emo-text の既存領分（本 spec は**時間の権威**のみ扱い、描画には触れない）。
  - 選択肢・対話タグ（M-dialogue）。
  - wintf `Typewriter` widget の完全統合（areka バルーンは emo-text ゆえ実行経路外）＝**隣接 duplication として認識するが、統合 or 明示スコープ外を設計段階で判断**。

## Boundary Candidates

- **計算の権威**: `text_playback_duration` 純関数＋char_wait 定数（sakura が単独所有・`\_w` 換算と同居）。
- **保持の権威**: cue タイムラインの duration 表現（dola `Cue`/`CueCommand::Text` の duration フィールド or schedule の duration 認識）。
- **服従の consumer**: emo-text reveal が duration を受けて表示（自前 pacing 撤去）。
- **後続 cue 整列**: duration を跨いだ次 cue の発火時刻決定（sakura compile が offset へ加算 or dola schedule が duration 認識）。
- **台本冒頭 Clear（#6）**: sakura compile が talk 先頭へ `Clear` cue を前置（新 talk＝バルーン自動クリア・scope 別クリアの粒度は要件で確定）。
- **wintf typewriter 整理**: 第3の独自実装の統合 or 明示的スコープ外化。

## Out of Boundary

- bind 状態管理・着せ替え（#2 `mayuna-compose`）／実行時 resize・再吸着（#1 `surface-resize-resnap`）。
- テキストレイアウト・縦書き・折返し（emo-text 既存・本 spec は時間のみ）。
- さくらスクリプトの新規タグ拡張（対話・選択肢＝M-dialogue）。
- ユーザーによる文字送り速度設定 UI（M2 送り・本 spec は**単一の既定 char_wait 定数**で足る）。
- **#7 冒頭 1.5行空行（leading `\n[150]`）＝pasta_lua の生成癖**: 作者 `dic/boot.pasta` の OnFirstBoot は話者1行ずつで `\n` 未記述——**pasta_lua が話者交替の行区切り `\n[150]` を、テキスト無しのセットアップ専用ターン（エモの `\1\![move]`）後にも挿入**するため空行が生じる。ゴースト作者でも areka でもなく **pasta エンジン側の問題**と判定（2026-07-13 開発者裁定）→ **areka スコープ外・上流 `ekicyou/pasta`（`vendors/pasta` submodule）へ起票**。fixture の忠実コピー `boot.pasta` を編集して回避するのも不採用（正典は ukadoc・emo2 は最小適合 fixture）。

## Upstream / Downstream

- **Upstream**: `dola`（cue モデル・TimedSchedule）／`areka-sakura`（compile・contract）／`areka-emo-text`（reveal・RevealSchedule）／`areka-P0-emo2-boot`（#3/#4/#D 症状を surface＝実 pasta スクリプト接地の出所）。
- **Downstream**: `areka-P0-emo2-boot` の #3/#4/#D 解消は本 spec に依存（症状の正しい修正はここで所有）／`\s` 表情同期／将来の対話・アニメ timing／`mayuna-compose`（bind cue も同じ duration 整列に載る可能性）。

## Existing Spec Touchpoints

- **Supersedes（吸収・撤回）**: `areka-P0-sakura-glyph-pacing`（roadmap 増分・④sakura）。同増分は #3/#4 の狭い・誤ったフレーミング（「句読点自動ポーズ facet」「emo-text reveal item-level 化」）だった。**正しい実体は本 spec の duration 権威アーキ**＝三権分立で置換。roadmap の #3/#4 記述も本 spec 参照へ更新。
- **Interlocks**: `areka-P0-emo2-boot`（同ブランチの `punctuation_wait` ハック＋drive.rs 診断ログの撤去が本 spec 実装の前提。emo2-boot 完了の扱い〔#3/#4/#D を本 spec へ委譲〕は着手時に確定）。
- **Adjacent（相互調整・2026-07-13 実コード偵察で衝突面を精密化）**: **`areka-P0-mayuna-compose`（#2 bind）と dola `CueCommand`＋sakura `compile.rs`／`contract.rs cue_target_of`＋emo-text `state.rs apply_cue` の4ファイルを共有**。ただし第一次 locus は素（disjoint）で、本 spec は既存 `Text` アームの挙動（duration 付与）を変え、mayuna は新 `Bind` アームを足す＝**別アーム＝マージ可能な近接編集**。**契約先決事項**: 本 spec が `Cue`/`CueCommand::Text` へ duration を足す形（＝emit()/Cue 署名の変更）を**先に確定**し、mayuna の `CueCommand::Bind` は**その確定形へ additive に載る**（bind は瞬時＝duration 0）。**推奨: 本 spec 先行 → mayuna が settled cue モデルへ**（`CueCommand` は既に balloon-face-cue が `BalloonSurface` を additive 追加した実績あり＝enum 拡張は安全）。／`wintf` `Typewriter`（第3 duplication・areka バルーンは emo-text ゆえ実行経路外＝統合 or 明示スコープ外を design で判断）。

## Constraints

- Rust 2024・新規 crates.io 依存なし・tokio 不使用。
- **決定論維持**: 時刻は注入（`talk_time`）・実時間 sleep/`Instant` 不使用（[[deterministic-test-coverage-mandate]]）。純関数化した duration 計算は GPU 不要で全網羅テスト可能（[[test-only-decision-branches-not-proven-wiring]]）。
- **dola は汎用基盤**: SakuraScript 固有の 50ms をハードコードしない（duration をデータとして受ける）。cue の serde 互換（既存 variant のワイヤ形不変・additive）に注意。
- **実機受け入れ**: 実 emo2・実 pasta.dll・実 DPI で #3/#4/#D＋表情同期を人間サインオフ（[[areka-placement-real-ghost-first]] の本番ゴースト先行原則）。**起動は絶対パス必須**（相対パスだと helper が pasta.dll を LoadLibrary できず MOD_NOT_FOUND＝2026-07-13 判明の運用注意）。

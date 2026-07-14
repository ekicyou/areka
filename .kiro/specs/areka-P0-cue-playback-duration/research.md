# ギャップ分析（research.md）— areka-P0-cue-playback-duration

> 生成日: 2026-07-13 ／ 言語: ja（spec.json）／ フェーズ: requirements-generated（gap 分析）
> 入力: requirements.md（確定・不変）・brief.md・.kiro/steering/{product,tech,structure,roadmap}.md
> 目的: 確定要件と既存コードベースの差分を精査し、設計フェーズの判断材料（複数案・トレードオフ・要調査点）を提示する。**決定はしない**。

## 概要（サマリ）

- **三権分立の実体は既にほぼ揃っている**。dola（保持）・sakura compile（計算/執筆）・emo-text reveal（服従）の 3 層は物理的に分離済みで、cue モデル・純粋コンパイル・注入時刻駆動 typewriter がすべて実在する。本 spec は「テキスト再生 duration」という**1 個の新データ**を 3 層に貫通させる横断改修であり、新規クレートや新規アーキテクチャは不要。
- **欠落は「テキスト cue が時間を占有する」概念のただ 1 点**。現状は `Instruction::Wait`（明示 `\_w`/`\w`）のみが offset を進め、`Instruction::Text` は offset を 0 進める（`compile.rs:38-48`）。暗黙 per-char ノミナルがタイムラインに載っていない。char_wait 定数は emo-text（`state.rs`・0.05）と wintf typewriter（`mod.rs:58`・0.05）に**二重実装**され、cue タイムラインの権威になっていない。
- **最大の設計軸は 2 つ**: ①duration を cue モデルの**どこに**載せるか（dola `Cue` エンベロープの新フィールド vs 新 `CueCommand` variant vs 新 schedule 機構）— serde 後方互換制約（R7.3/7.4）が `CueCommand::Text(String)` の in-place 改変を実質禁止する。②後続 cue の整列を**誰が**担うか（sakura が offset+=D を焼き込み dola は点配送のまま=既存 Wait と対称 vs dola `TimedSchedule` が「D 秒占有」を能動的に強制=挙動変更）。要件文（R1.2 と R3.2）に緊張があり、討議で確定要。
- **前提条件は既充足**。`punctuation_wait` ハックと drive.rs 生スクリプト診断ログは `crates/**` grep でゼロ（実コード再確認済）。着手時に再 grep で確認するのみで撤去作業は発生しない見込み。
- **決定論・serde 互換は現行資産で担保可能**。純粋関数化した duration 計算は GPU 不要で全網羅テスト可能。dola `Cue`/`CueCommand` は serde 派生・externally tagged で、additive 拡張の実績（`BalloonSurface` 追加）がある。TalkCue は serde 非依存の実行時契約型ゆえ duration フィールド追加は serde 制約外。

---

## 1. 現状のコードベース調査（三権の実コード所在）

### 1.1 保持＝dola（`crates/dola/src/cue/`）

- **`command.rs`**:
  - `Cue { actor: ActorKey, start_time: f64, payload: CuePayload }`（`command.rs:186-195`）。**duration の概念なし**。派生 `Clone, Debug, Serialize, Deserialize`（PartialEq 非導出）。
  - `CueCommand`（8 variant・externally tagged serde）。`Text(String)`（`command.rs:124-125`）。ワイヤ形 `{"Text":"hello"}`。`BalloonSurface{key}` を additive 追加した実績（`cue_command_balloon_surface_serde_roundtrip` テストで `{"BalloonSurface":{"key":"2"}}` を固定）。
  - `BarrierKind::Timeout { duration: f64 }`（`command.rs:92-93`）が**既に存在**（後述の代替案 C で再利用候補）。
- **`schedule.rs`**: `TimedSchedule<T>`。`Entry<T> = Payload(f64,T) | Barrier(f64,BarrierKind) | Routing(f64,RoutingCommand)`（`schedule.rs:17-24`）。`tick(current_time)`→`ready()` の 2 フェーズ。**点時刻配送**（`entry_offset > offset` で break・`schedule.rs:171-210`）。duration/占有の概念なし。NaN 全量配信ハザードあり（下流 sakura がガード済）。
- **`sheet.rs`**: `CueSheet(Vec<Cue>)`（serde）。`CueSheet::new` は start_time 昇順安定ソート。`compile_sheet`（min 正規化）は sakura が**使わない**（先頭待ちが潰れるため）。

### 1.2 計算/執筆＝sakura（`crates/areka-sakura/src/`）

- **`compile.rs`（純粋・決定的・no I/O）**: `compile(&[Instruction]) -> CompiledTalk{sheet, end}`。
  - `Instruction::Wait(d)` → `offset += d.as_secs_f64()`（`compile.rs:38-40`）。**明示ウェイトのみ offset を進める**。
  - `Instruction::Text(t)` → `emit(scope, offset, CueCommand::Text(t))`（`compile.rs:46-48`）。**offset を進めない=テキスト 0 時間**。← **本 spec の中核欠落**。
  - `Surface`→`Emote`（不透明転写）・`NewLine`→`NewLine{ratio}`・`Clear`→`Clear`・`End/Quit`で切詰め。
  - `emit(scope, offset, cmd)` が `Cue{actor: ActorKey(scope), start_time: offset, payload}` を構築（`compile.rs:107-113`）。**Clear 前置は存在しない**（現状 talk 冒頭に Clear cue を積まない=#6 の配線欠落）。
- **`contract.rs`**: `TalkCue{ at: f64, actor: ActorKey, command: CueCommand }`（`contract.rs:45-53`・**serde 非依存**・Clone/Debug/PartialEq のみ）。`cue_target_of`（全 variant 明示 match・catch-all 無し=variant 追加時にコンパイラが再検討強制）。`Text/NewLine/Clear/Choice`→`Balloon`、`Emote/EntityRef/BalloonSurface`→`Shell`。
- **`drive.rs`**: per-talk transient アクター。`to_schedule(&CueSheet) -> TimedSchedule<TalkCue>`（`drive.rs:288-308`）が `Cue` から `TalkCue{at: cue.start_time, actor, command}` を構築し `Entry::Payload(start_time, talk_cue)` で挿入。`on_tick` が `ready()` を `cue_target_of` で 2 sink（SurfaceSink/TextSink）へ振り分け。診断ログ・punctuation_wait なし。

### 1.3 服従＝emo-text（`crates/areka-emo-text/src/`）

- **`state.rs`（純粋層・windows 非依存・決定論）**:
  - `TextLayerConfig{ char_wait: f64=0.05, line_pitch_factor: f32=1.25 }`（`state.rs:36-52`）。← **撤去/従属化対象の per-char 定数（重複その1）**。
  - `RevealSchedule::extend_chunk(glyph_count, chunk_start, char_wait)`（`state.rs:107-116`）: `r_i = max(r_{i-1}+char_wait, chunk_start)`・先頭 `r_0=chunk_start`。← **自前 char_wait pacing**。
  - `TextLayerState::apply_cue(cue, config)`（`state.rs:165-203`）: `Text`=追記＋`extend_chunk(glyph_count, cue.at, config.char_wait)`／`NewLine`=マーカー追記／`Clear`=actor 状態全消去（未リビール分含む）。`visible_glyphs(actor, t)`=`r_i <= t` のグリフ数（二分探索）。
- **`actor.rs`（結線層）**: `TextLayerRuntime{ state, config: TextLayerConfig, ... }`（`actor.rs:138-152`）。`apply_cue` が `self.config` を `state.apply_cue` に渡す（`actor.rs:212-219`）。`present_frame(runtime, world, talk_time)` が `visible_glyphs(actor, talk_time)` を引いてレイアウト→描画。`config`（char_wait 込み）は `TextLayerRuntime::new(config)` で外部注入。
- **`sink.rs`**: `EmoTextSink`（`TextSink` 実装）→`TextMsg::Cue(TalkCue)` を UI ドレインへ。`TalkCue` を無変形で運ぶ（duration を載せるなら TalkCue が搬送体）。

### 1.4 実行経路外の第3重複＝wintf typewriter

- `crates/wintf/src/ecs/widget/text/typewriter/mod.rs:58` `Typewriter.default_char_wait = 0.05 // 50ms`＋`typewriter_layout.rs` の独自 `current_time += default_char_wait`。areka バルーンは emo-text 経路ゆえ**実行されない**（R8.1 で対象外）。

### 1.5 パーサの `\w` 単位（別概念の 50ms に注意）

- `decode.rs:36` `WAIT_UNIT_MS: u64 = 50`。`\w[n]`/`\wN`＝n×50ms、`\_w[ms]`＝絶対 ms を `Instruction::Wait(Duration)` へ正規化（`decode.rs:234-248`）。**これは「明示ウェイトの単位」であり「暗黙 per-char ノミナル」とは別物**。R2.3 の「per-char ノミナル定数の一元化（sakura）」と混同しないこと（3 個目の 50ms を作らない・逆に既存 2 個を安易に統合しない）。

### 1.6 実機駆動の時刻源（emo2_boot）

- `emo2_boot/talk_clock.rs`: `TalkClock` が worker の `observe_cue` で epoch を確立し UI で `talk_time = frame_now − epoch`（新 talk で epoch リセット＝`talk_time ≈ 0` 頭出し）。`frame.rs` が `talk_time` が `Some` のとき `present_frame` を呼ぶ。duration はこの talk 相対時刻の上で reveal を pacing する。

---

## 2. 要件→資産マップ（ギャップ種別: 🔴Missing／🟡Unknown／🔵Constraint）

| 要件 | 既存資産 | ギャップ |
|---|---|---|
| **R1** テキスト再生時間の第一級モデル化（dola 保持） | `Cue`/`CueCommand`/`TimedSchedule`（点配送）／`BarrierKind::Timeout{duration}` 既存 | 🔴 cue へ duration を保持する場所が無い。🔵 R7.3/7.4 serde 後方互換。🟡 R1.2「dola が後続を整列」の実装主体（能動整列 vs sakura 焼込み） |
| **R2** 文字再生時間の単一純関数（sakura） | 純粋 `compile`・`decode.rs` の `WAIT_UNIT_MS`（別概念） | 🔴 `text_playback_duration(text)` が未実装。🟡 純関数の入力粒度（Text チャンク単位の char×nominal のみか、明示 `\_w` も畳むか） |
| **R3** compile が duration を書込み後続整列 | `compile.rs` の offset 累積（Wait のみ） | 🔴 Text で offset を進めない。Text cue へ duration を付与する経路が無い |
| **R4** 台本冒頭 Clear 前置（#6） | emo-text `Clear` 機構は実在（actor 単位消去）／compile の scope 転写 | 🔴 compile が talk 冒頭に Clear cue を積まない。🟡 マルチ scope（`\0`/`\1`）の Clear 粒度（先頭一括 vs scope 初出時） |
| **R5** emo-text reveal が duration に服従 | `RevealSchedule::extend_chunk`（自前 char_wait）・`TextLayerConfig.char_wait` | 🔴 自前 char_wait を duration 由来へ差替え。🟡 D=0/未付与時の縮退・reveal 式（`at+i·D/N` か既存 max 追従の維持か） |
| **R6** 実機受入（#3/#4/#6/`\s`同期） | emo2_boot 実機起動・`TalkClock`・実 pasta.dll | 🔵 人間サインオフ必須・**絶対パス起動必須**（相対だと pasta.dll LoadLibrary 失敗）。🟡 `\s`同期は Shell 系 Emote cue の整列で副産物解決の想定 |
| **R7** 決定論・汎用性・ワイヤ互換 | 注入時刻駆動（Instant 不使用）確立済・serde additive 実績 | 🔵 dola に 50ms を焼かない（データとして受ける）。🔵 既存 variant ワイヤ形不変・serde default で後方互換 |
| **R8** スコープ境界・前提条件 | wintf typewriter（実行経路外）・`punctuation_wait`/診断ログ不在 | 🟢 前提既充足（再 grep 確認のみ）。🟡 wintf typewriter の統合可否は設計判断 |

---

## 3. 実装アプローチの選択肢

本 spec の核は**独立した 2 つの設計軸**であり、それぞれに複数案がある。まず軸ごとに提示し、最後に統合推奨を示す。

### 軸1: duration を cue モデルのどこに載せるか（保持の場所）

#### 案1-A: dola `Cue` エンベロープに duration フィールドを追加（＋TalkCue へ伝播）
- `Cue { actor, start_time, payload, #[serde(default)] duration: f64 }`（または `Option<f64>`）。`CueCommand` は無改変。`to_schedule` が `TalkCue{ at, actor, command, duration }` へ複写し emo-text へ届ける。
- ✅ `CueCommand::Text(String)` のワイヤ形不変（R7.3 厳守）。`#[serde(default)]` で duration 無しの既存データも読める（R7.4）。テキスト以外の cue（duration 0）も同一形で自然に載る（R1.4 後方互換）。
- ✅ duration が「テキスト固有」でなく「任意 cue が時間を占有しうる」汎用データになり、dola の汎用性（R7.2）と整合。mayuna の瞬時 bind cue（duration 0）も同形に載る（隣接 additive）。
- ❌ `Cue` と `TalkCue` の両方にフィールド追加が要る（伝播経路の一点複写）。`Cue` は PartialEq 非導出のため既存テストは fields 比較（`cue_eq`）で吸収済み。
- 🔵 `duration` の意味（「この cue の後、次 cue までの占有秒」）を型 doc で明確化しないと、start_time との関係が曖昧になる。

#### 案1-B: 新 `CueCommand` variant（例 `TimedText { text: String, duration: f64 }`）
- `Text(String)` を温存し、duration 付きテキストを別 variant として additive 追加。
- ✅ 既存 `Text` ワイヤ形不変・後方互換（R7.3/7.4）。variant 追加は `BalloonSurface` の実績どおり安全。
- ❌ **テキスト cue が 2 系統に分裂**。`cue_target_of`・emo-text `apply_cue`・drive の分類を両 variant で二重管理。「duration は任意 cue が持ちうる」思想（R7.2 汎用性）に逆行し、将来 `\s` 等へ duration を広げる際に再度分裂。
- ❌ 「純関数を 1 つに集約し duplication 絶滅」という本 spec の主眼と、variant 分裂が相性悪い。

#### 案1-C: dola `TimedSchedule` に占有機構を新設（`BarrierKind::Timeout` 再利用 or 新 Entry）
- sakura が各テキスト cue の直後に `Entry::Barrier(offset, BarrierKind::Timeout{duration: D})` を挿入し、schedule が D 秒占有（既存の barrier タイムアウト自動解除を流用）。
- ✅ dola に既存の `Timeout` 機構があり、schedule が「占有」を能動的に持つ思想（R1.2）に最も忠実。
- ❌ **barrier は schedule 全体を停止**（actor 非依存の大域停止）。マルチ actor/scope talk で他 actor の cue まで止める副作用。emo-text へ duration が**届かない**（barrier は payload を運ばない）ため R5（reveal 服従）を別経路で満たす必要があり、結局 duration を cue にも載せることになる=二重保持。
- ❌ barrier は「外部解決待ち」の停止点であって「テキスト再生の進行」ではない。意味論のミスマッチ。

### 軸2: 後続 cue の整列を誰が担うか（整列の主体）

#### 案2-A: sakura が offset += D を焼込む（既存 Wait と対称・dola は点配送のまま）
- `compile.rs` で `Text` を emit した直後に `offset += text_playback_duration(text)`（`Wait` と同じ累積機構）。同時に当該 Text cue へ D を**データとして**刻む（軸1で選んだ場所へ）。dola の点配送が次 cue を自然に t+D 以降で発火。
- ✅ 既存の Wait 累積（`compile.rs:38-40`）と**完全対称**＝最小改修・低リスク。decode の `wait_accumulation_is_monotonic` 系テストと同型で網羅可能。
- ✅ dola は「duration を保持するが挙動は点配送のまま」＝R1.1（保持）を満たしつつ TimedSchedule 無改変（R7.3 の schedule 側非破壊）。emo-text へは刻んだ D が届き R5 を満たす。
- 🟡 R1.2「dola スケジュールが後続を整列」の文面は、厳密には sakura が offset を作りдola が点配送で実現する形。「dola が duration を保持し、その保持値により整列が成立する」と読めば整合するが、**R1.2 と R3.2 の主体表現の緊張**を討議で確認要。

#### 案2-B: dola `TimedSchedule` が「D 秒占有」を能動的に強制
- schedule が Payload に付随する duration を読み、`occupied_until = max(occupied_until, fire_time + D)` を保持して、次 Payload の発火を occupied_until までブロック。
- ✅ 「dola が単一権威として能動的に整列」思想に最も忠実（R1.2 の字面）。sakura は offset 累積不要（duration を刻むだけ）。
- ❌ TimedSchedule の中核ループへの**挙動追加**（占有状態・冪等性・NaN ガード・barrier との相互作用の再検証）。既存の schedule テスト群への影響大＝中〜高リスク。
- ❌ actor 別 talk における「占有」の粒度（全 schedule 占有 vs actor 別占有）を新たに定義する必要。現状 1 talk=1 schedule ゆえ実害は小だが、汎用基盤としての設計負債。

### 軸3: emo-text reveal 式（服従の具体）

- 現状 `r_i = max(r_{i-1}+char_wait, cue.at)`。新方式は「N 文字を概ね D 秒で表示」（R5.3）。候補:
  - **3-i**: `r_i = cue.at + i · (D / N)`（i=0..N・chunk 内均等割り）。D を N で割った実効 char_wait を chunk ごとに導出。
  - **3-ii**: 既存 `max` 追従式を維持しつつ `char_wait := D/N` を chunk ごとに供給（跨 chunk の tail 追従挙動を温存）。
- 🟡 D=0（空テキスト/後方互換 cue）や N=0 の縮退（0 除算回避・即時表示 or 既定 char_wait フォールバック）を要定義。emo-text `TextLayerConfig.char_wait` を**残置しフォールバック**にするか、**完全撤去**するか（R5.2「独自 per-char 定数を保持しない」との整合）を討議で確定。

### 統合推奨（設計フェーズの出発点・決定ではない）

- **軸1=案1-A（`Cue`/`TalkCue` エンベロープに duration）＋軸2=案2-A（sakura が offset+=D 焼込み・dola は保持）＋軸3=3-i（chunk 内均等割り）** を第一候補として推奨する。
  - 根拠: R7.3/7.4 の serde 制約を最も素直に満たし、既存 Wait 累積との対称で最小改修・低リスク。duration が汎用データとして dola に載り「dola が根本情報を把握」する brief の思想を満たしつつ、TimedSchedule の中核挙動を変えない。emo-text へ D が届き reveal 服従が成立する。
  - **ただし** R1.2 の「dola が能動整列」を字義どおり実装すべき（案2-B）と開発者が判断する余地があり、これは**要件討議の最重要論点**（下記 判断項目1・2）。「あるべき姿から検討」（memory: analyze-ideal-form）に従い、案2-B を理想形として明示比較したうえで案2-A を推す立場。

---

## 4. Effort / Risk

| 領域 | Effort | Risk | 一言根拠 |
|---|---|---|---|
| sakura `text_playback_duration` 純関数＋char_wait 定数一元化 | S | Low | 純粋・決定的・GPU 不要で全網羅テスト可能。既存 decode の単位換算と同型 |
| sakura compile の offset+=D 焼込み＋duration 付与（案2-A） | S | Low | 既存 Wait 累積と対称。テストパターン流用可 |
| dola `Cue`/`TalkCue` へ duration 追加＋serde 後方互換（案1-A） | S〜M | Low〜Med | additive・serde default。既存 roundtrip テストへ 1 ケース追加。TalkCue は serde 外で安全 |
| dola TimedSchedule 能動占有（案2-B を採る場合） | M | **High** | 中核ループ改変・既存 schedule/barrier テスト全再検証・NaN/冪等再考 |
| sakura compile の talk 冒頭 Clear 前置（#6・マルチ scope） | S〜M | Med | scope 別 Clear の粒度（先頭一括 vs 初出時）が未確定＝設計判断依存 |
| emo-text reveal の duration 服従＋縮退定義 | S〜M | Med | 純粋層・決定論檻あり。D=0/N=0 縮退と既存 reveal テスト群の更新 |
| 実機受入（#3/#4/#6/`\s`同期・人間サインオフ） | M | Med | 実 emo2・実 pasta.dll・実 DPI・絶対パス。決定論外の観測ゲート |
| **全体** | **M** | **Med**（案2-A 採用時）／High（案2-B 採用時） | 横断だが各改修は既存パターンの延長。整列主体の選択がリスクを二分 |

---

## 5. 設計フェーズへの申し送り

### 推奨アプローチ（叩き台）
- 軸1=案1-A・軸2=案2-A・軸3=3-i を出発点に、案2-B（dola 能動整列）を理想形として明示比較する design を書く。
- 純関数 `text_playback_duration` は sakura に単独所有させ、per-char ノミナル定数（例 `CHAR_NOMINAL_MS`）を sakura に一元化。parser の `WAIT_UNIT_MS`（別概念）と emo-text `char_wait`（撤去/従属）と混同しない三者分別を design に明記。
- Clear 前置は compile の 2 pass（scope 事前走査→冒頭に scope 別 Clear 群を前置）か、scope 初出時 Clear かを design で確定。

### 要調査（Research Needed・設計フェーズで深掘り）
1. **serde 後方互換の厳密検証**: `Cue` へ `#[serde(default)] duration` を足したとき、既存 JSON（duration 無し）が欠損なく読めること・既存 variant ワイヤ形が完全不変であることの roundtrip 檻を design 段階で設計（R7.3/7.4）。
2. **マルチ scope Clear の ukadoc 挙動**: SSP の「新 talk でバルーン自動クリア」が全 scope 対象か、書き込む scope 限定かを ukadoc MCP で確認（正典は ukadoc・emo2 fixture は最小サンプル）。R4.3 の粒度確定に直結。
3. **`\s` 表情同期の実現経路**: Emote cue（Shell 系）が Text cue の duration 整列に相乗りして「喋り完了後に切替」となるか、Emote 自体に整列指定が要るかを実機 fixture（emo2 boot.pasta）で確認。副産物解決の想定の裏取り。
4. **reveal 縮退の決定論檻**: D=0/N=0/D<0 の縮退（0 除算回避・即時表示 or フォールバック）を純粋層テストで全網羅する設計。

---

## 6. 設計判断項目（要件討議へ送る・番号付き）

> 以下は「情報とオプション」であり決定ではない。要件討議（kiro-requirements-discussion）で開発者が裁定する論点。

1. **【最重要】整列の主体**: 後続 cue の整列を **(2-A) sakura が offset+=D 焼込み・dola は duration を保持しつつ点配送**とするか、**(2-B) dola TimedSchedule が「D 秒占有」を能動的に強制**するか。R1.2（dola が整列）と R3.2（遅延は dola 担当だが sakura が台本構成）の主体表現に緊張がある。案2-A は最小改修・低リスクだが「dola 能動」の字義から後退。案2-B は思想に忠実だが中核挙動改変で高リスク。

2. **duration の保持場所**: **(1-A) dola `Cue`/`TalkCue` エンベロープの新フィールド**（`CueCommand` 無改変・serde default で後方互換）か、**(1-B) 新 `CueCommand::TimedText` variant**（テキスト cue 2 系統化）か、**(1-C) `TimedSchedule` の占有機構/barrier 再利用**（emo-text へ届かず二重保持リスク）か。R7.3（既存 variant ワイヤ形不変）は 1-A を強く支持。

3. **純関数の入力粒度**: `text_playback_duration` は **(a) Text チャンク単位の `char_count × per-char ノミナル`のみ**を返し、明示 `\_w` は既存 `Instruction::Wait` 累積のまま分離するか、**(b) テキスト＋隣接明示ウェイトを一括**で畳むか。Instruction モデルは既に `Text`/`Wait` を分離済みゆえ (a) が構造整合。R2.1/2.4 の「暗黙＋明示の換算」文面は (b) を示唆するようにも読めるため確認要。

4. **per-char ノミナル定数の所在と 3 個の 50ms の分別**: sakura に一元化する per-char ノミナル定数と、parser の `WAIT_UNIT_MS=50`（`\w` 単位・別概念）、emo-text `char_wait=0.05`（撤去/従属対象）の三者を design でどう分別・命名するか。安易な統合も分裂も避ける。

5. **emo-text `TextLayerConfig.char_wait` の扱い**: **(a) 完全撤去**（R5.2「独自 per-char 定数を保持しない」に忠実）か、**(b) D=0/N=0 縮退時のフォールバック定数として残置**か。残置する場合 R5.2 との整合説明が要る。

6. **reveal 式**: `r_i = at + i·(D/N)`（chunk 内均等割り・3-i）か、既存 `max` 追従式に `char_wait:=D/N` を供給し跨 chunk tail 追従を温存（3-ii）か。

7. **talk 冒頭 Clear の粒度（#6）**: compile が **(a) 冒頭に単一 scope（既定 "0"）の Clear** を前置するか、**(b) talk が書き込む全 scope の Clear** を前置（要 2 pass 走査）か、**(c) scope 初出時に Clear を挿入**するか。マルチ scope（`\0`/`\1`）talk での前 talk 残存を確実に消す粒度。emo-text `Clear` は actor（=scope）単位消去である事実（`state.rs:181-185`）が制約。

8. **wintf `Typewriter`（第3重複）**: 明示的スコープ外（現状維持・実行経路外）で確定するか、この機会に統合/撤去するか。R8.1 は設計判断に委ねている。areka バルーンは emo-text 経路ゆえ機能上の実害は無い。

9. **duration の serde 表現**: `duration: f64`（`#[serde(default)]`・0.0 が「未占有」）か、`Option<f64>`（`None` が「未付与」・R1.4 の「与えられない」と明示区別）か。後方互換データの解釈（0 と未指定の区別要否）に影響。

---

## 7. 要件討議での分類・確定・追加グラウンディング（2026-07-13）

> `/kiro-start` の要件ディスカッション（controller inline）で本 gap 分析を取り込み、§6 の判断項目を A/B/C 分類し、ukadoc で裏取りした追加事実を記録する。

### 7.1 イシュー分類の結果

- **カテゴリ A（自明修正）= 0 件**: requirements.md は EARS 準拠・誤字/矛盾/EARS 違反なし。修正コミットは発生しない。
- **カテゴリ B（設計判断・設計フェーズへ先送り）= §6 の項目 2〜9（8 件）＋ 7.3 の `\C` 境界**: いずれも how／アーキ選択ゆえ `/kiro-spec-design` で裁定。要件文の変更は不要（要件は outcome を規定済み）。
- **カテゴリ C（開発者裁定・要件討議 Topic 1）= §6 の項目 1（整列の主体）**: R1.2／R3.2 の主体表現が案 2-B（dola 能動整列）を字義的に要求しており、gap 分析推奨の案 2-A（sakura offset 焼込み・dola は D を第一級保持）と字面が衝突。要件文の強度を開発者が裁定する（結果を R1.2／R3.2 へ反映）。

### 7.2 ukadoc グラウンディング（項目 7・#6 Clear の裏取り）

正典 ukadoc（`mcp__ukadoc__search_docs`）で確認:

- **`\c`（小文字）** = 「現スコープ側のバルーン内の文字をクリア」＝**per-scope クリア**。areka parser は `decode.rs:159` で `'c' => Instruction::Clear`、compile は `CueCommand::Clear`（Balloon 標的）へ写像済み。
- **`\C`（大文字）** = 「先頭に記述すると直前に表示したバルーンのテキストに**追記**をする。スコープは 0 番に戻る」＝クリアしない**追記モード**。逆に言えば **`\C` を書かない既定挙動＝新 talk でバルーンをクリア**が正典で確認され、**#6 の前提（新 talk＝バルーン自動クリア）は妥当**。
- **`\_w[時間]`** ドキュメントに「文字表示にかかった時間(SSP ユーザー設定で可変)も加味される」と明記＝**テキスト再生時間を勘定するのは正典挙動**であり、本 spec の duration モデル化の前提が ukadoc 整合。
- **結論**: R4.3「当該 talk が書き込む各スコープをクリア」は SSP 準拠で正しい（written-scopes 単位）。**クリアの実装機構**（冒頭一括前置の 2 pass vs scope 初出時挿入）は要件不変のまま設計判断（§6 項目 7）。

### 7.3 `\C` 追記モードのスコープ境界（新規・設計申し送り）

- areka parser は現状 `\c`（小文字→`Instruction::Clear`）のみ解釈し、**`\C`（大文字・追記モード）は未パース**（Raw 扱い→compile が無視ログで破棄）。ゆえに areka には現状「追記モード」が存在しない。
- R4.1 の「talk 冒頭へ**無条件** Clear cue を前置」は、この未対応 `\C` を前提に現状成立する。ただし将来 `\C` 追記モードを支援する際は、**Clear 前置を `\C` 不在の条件付き**にする必要がある（無条件前置は `\C` の append 意味論を破る）。
- 本 spec では **`\C` は明示的スコープ外**（対話系＝M-dialogue 以降で扱う）。設計は「Clear 前置は現状無条件で可・ただし `\C` 対応時に条件化を要する」ことを申し送りとして記録する。

### 7.4 Topic 1 裁定（2026-07-13・開発者決裁）— dola の本質＝絶対時刻台本の同期配送

要件討議 Topic 1（整列の主体・§6 項目 1）を開発者が決裁。**gap 分析の統合推奨（案 2-A）も、討議 controller が一時対置した案 2-B（dola が配送時に occupancy を導出）も、いずれも枠組みが不正確**であった。正しい枠組みは以下：

- **dola の役割の定義**: dola は**同一内容の台本を複数の独立した最終表現者へ手渡す**存在であり、**表現者が独立動作しても（プロセス境界を跨いでも）同一の絶対時刻でイベントが発火することを保証**する。→ 本 spec の複数表現者の具体例＝同一 `CueSheet` を `cue_target_of` で分岐する **SurfaceSink（seriko・`\s`）と TextSink（emo-text・text）**。両者が同一絶対時刻で駆動されることが `\s` 表情同期（R6.4）の実体。
- **start_time は絶対時刻**（開発者明言）。dola は配送時に時刻を導出・変換しない（導出は表現者ごと独立計算＝プロセス跨ぎで desync ゆえ同期保証と両立不能）。ゆえに**案 2-B design-2（dola TimedSchedule が occupancy を能動導出）は棄却**。
- **整列は sakura が絶対 start_time へ焼き込む**（`offset += D`・既存 `Wait` 累積と対称）。dola は焼き込み済み絶対時刻を忠実配送。← 機構としては当初の案 2-A に一致するが、**採否根拠はリスクではなく「dola の同期配送責務」**である。
- **D は cue の第一級データとして各表現者へ運ばれる**（emo-text reveal 用）。ギャップからの導出は不能（`\_w`・actor 切替・末尾で ギャップ≠D）ゆえ D の cue 搭載は必須。

**「重複」懸念の解消**: 単一の真実源は**計算層**（`text_playback_duration` 1 本＝R2）にある。絶対 start_time と cue の D は**同一計算の 2 投影を不変台本へ凍結**したもので、実行時ドリフト不能（再タイミング＝再 compile で両者一斉再生成）。絶対 start_time は同期の**必須要件**であり冗長ではない。→ 前ラウンドで controller が提案した **R1.5（単一表現・事前計算間隔の禁止）は撤回**（絶対 baked start_time と矛盾する）。

**派生設計論点の帰結**:
- 「start_time 意味論」＝**絶対時刻で確定**（§6 の設計論点から除去）。
- 「占有粒度 global/per-actor」＝**dola に occupancy 機構は無い**（sakura の単一 offset 累積が絶対時刻を焼く）ため**論点消滅**。
- §6 項目 2（duration 保持場所）・項目 9（serde 表現）は依然設計判断だが、**cue が絶対 start_time に加えて D フィールドを持つ**方向は確定（保持場所は「cue／TalkCue エンベロープの duration」＝案 1-A 系が本裁定と整合。variant 分裂の案 1-B は非採用方向）。

**要件反映**: R1 を「絶対時刻台本の保持と同期配送」へ再構成（絶対時刻 AC・cross-process 同期 AC を明記・旧 R1.2「dola スケジュールが整列」を除去）。R3 の Objective と AC2 を「sakura が絶対 start_time へ D を焼き込む」へ訂正（旧「発火の遅延は dola が担う」を削除）。R2/R5/R6/R7 は不変。

### 7.5 dola 表現範囲の外壁（2026-07-13・開発者補足）— 静的絶対時刻タイムラインの境界

Topic 1 裁定の系として開発者が dola の表現範囲を明示補足:

- dola が表現できるのは「**絶対開始時刻＋累積時間**」の**静的タイムライン**に限る（原点固定・前方単調累積の絶対時刻列）。一時停止/選択肢/再開のような**動的制御フローは dola の内側でモデル化しない**。
- 一時停止（`\x` クリック待ち）・選択肢（`\q`）は、コンパイル時に再開時刻が未知＝「絶対開始時刻＋累積時間」で表現不能。よって **Barrier シーム＝現台本の静的タイムラインの終端（外部解決待ちの停止点）**。**再開は オーケストレーター（kanade/sakura）が新たな絶対開始時刻を調停し、続きの dola 台本を再配信**する。動的制御フローは dola の**外側**に置く。これが「プロセスを跨いで複数演者がいても正確に同期可能」という dola 設計思想の核。
- **§3 軸1 案1-C（Barrier 再利用でテキスト占有を表現）棄却の補強**: Barrier は**外部解決待ちの seam 専用**。テキスト再生 D は**既知の累積 duration** ゆえ Barrier で表すのは意味論ミスマッチ。D は cue の第一級データとして絶対 start への累積で台本内側に載る。
- **本 spec への含意**: 再生時間 D は単一台本内の単調累積（絶対 start＋累積 D）で表現され、この外壁の**内側に完全に収まる**。選択肢・一時停止（M-dialogue）はスコープ外ゆえ、本 spec は dola へ pause/resume 状態を追加しない。

---

## 8. 設計フェーズ discovery / synthesis / 決定（2026-07-13）

> `/kiro-spec-design`（Light discovery・Extension）で確定要件と既存コードを突き合わせ、§6 の設計判断項目（Category B）を裁定し design.md へ落とした。実コードのシグネチャを再確認済み。

### 8.1 Discovery（Light・Extension・実コード再確認）

- **保持＝dola** `crates/dola/src/cue/command.rs`: `Cue { actor, start_time, payload }`（`Clone/Debug/Serialize/Deserialize`・**PartialEq 非導出**）。`CueCommand` 8 variant・externally tagged（`Text` ワイヤ形 `{"Text":"hello"}`）・`BalloonSurface` additive 実績。`sheet.rs::CueSheet::new` は `start_time` 昇順**安定ソート**。`compile_sheet`（min 正規化）は sakura 不使用。
- **計算＝sakura** `compile.rs`: `Wait(d)→offset += d.as_secs_f64()`／`Text(t)→emit(scope, offset, Text)`（**offset 進めず**）。`emit` が `Cue` 構築。冒頭 Clear なし。`contract.rs::TalkCue { at, actor, command }`（serde 非依存・PartialEq）。`drive.rs::to_schedule` が `Cue→TalkCue` 複写・`on_tick` が `cue_target_of` で 2 sink 振り分け（NaN/単調/冪等ガード実在）。診断ログ・punctuation_wait は grep ゼロ（再確認）。
- **服従＝emo-text** `state.rs`: `TextLayerConfig.char_wait=0.05`（撤去対象）・`RevealSchedule::extend_chunk(glyph_count, chunk_start, char_wait)` が `r_i = max(r_{i-1}+char_wait, chunk_start)`。`apply_cue(cue, config)` が Text 追記＋reveal 確定・Clear 全消去。`actor.rs::TextLayerRuntime` が `config`（char_wait＋line_pitch）を保持し `apply_cue`／`DWriteMetrics::new` へ渡す。`sink.rs`/tests に `TalkCue { ... }` リテラル多数（duration 追加でコンパイル強制更新）。
- **ukadoc グラウンディング（本フェーズ追加確認）**: `\_w[時間]` = 精密ウェイト（[時間]ms・単純な追加待ち）。「文字表示にかかった時間も加味」の記述は `\__w`（**基準からの累積ウェイト・二重アンダースコア**）の挙動であり `\_w` ではない。`\__w` は areka parser 未対応（`\C` 同様・M-dialogue 領分）＝本 spec スコープ外。テキスト再生が時間を占有するのは正典挙動（`\__w` が文字表示時間を差し引く事実がその裏付け）で、本 spec の duration モデル化の前提は ukadoc 整合。

### 8.2 Synthesis（3 レンズ）

- **一般化**: duration は「テキスト固有」でなく「任意 cue が時間を占有しうる」汎用データ。`Cue` エンベロープに載せることで mayuna の瞬時 bind cue（D=0）が additive に同形へ載る（隣接 spec の interface を先取り一般化・実装は現要件どまり）。
- **Build vs Adopt**: 新機構を作らず既存 `Wait` 累積（`offset += d`）を**採用・対称拡張**（`offset += D`）。`TimedSchedule` の点配送・reveal の `max` 追従式・serde additive パターン（`BalloonSurface`）を再利用。新クレート・新依存ゼロ。
- **簡素化**: 案 2-B（dola 能動占有）・案 1-B（variant 分裂）・案 1-C（Barrier 再利用）を棄却し、単一の `duration: f64` フィールド＋純関数 1 本へ収斂。emo-text の reveal 式構造は不変（interval の供給源のみ config→cue へ差し替え）＝抽象追加なし。

### 8.3 §6 設計判断項目の裁定（Category B）

| 項目 | 裁定 | 根拠 |
|---|---|---|
| 2 保持場所 | **案 1-A**: `Cue`/`TalkCue` エンベロープに `duration` | 7.3（Text ワイヤ形不変）・Topic 1 裁定と整合・汎用データ化 |
| 3 純関数粒度 | **(a)**: `text_playback_duration(text)=char_count×nominal`（暗黙のみ） | Instruction は Text/Wait 分離済み・R3.3/R3.4 が明示ウェイト累積を compile 層に位置づけ（純関数が畳むと二重計上）。R2.4 は「暗黙半分＝純関数／明示半分＝compile 合成」で reconcile |
| 4 定数所在 | `CHAR_NOMINAL_MS: u64 = 50`（sakura・唯一） | parser `WAIT_UNIT_MS`（`\w` 単位・別概念）・emo-text `char_wait`（撤去）と三者分別 |
| 5 char_wait | **完全撤去** | 5.2「独自 per-char 定数を保持しない」に忠実。D=0 縮退が即時可視を自然に与えフォールバック定数不要 |
| 6 reveal 式 | **3-ii**（`r_i = max(r_{i-1}+D/N, at)`・tail 追従温存） | 既存式構造を保存し跨チャンク無損失挙動・回帰リスク最小。研究 §3 の暫定 3-i から、`D=N×50ms` 時に旧挙動と**機能等価**（全 tick で可視グリフ数一致・厳密ビット等価ではない＝設計ディスカッション #1 で FP 緩和）となる 3-ii へ精緻化 |
| 7 Clear 粒度 | **書込スコープ集合を先頭前置**（走査後 post-pass・`BTreeSet<u32>`） | R4.1「冒頭前置」＋R4.3「書込各スコープ」・ukadoc written-scopes 準拠。安定ソート＋同一 at FIFO で Clear→Text 順を保証 |
| 8 wintf Typewriter | 対象外（現状維持） | 8.1・実行経路外 |
| 9 serde 表現 | `duration: f64` ＋ `#[serde(default)]`（0.0=瞬時） | 1.5「未付与＝0 相当」・7.4 後方互換・0 と未指定を区別する意味論的必要なし・算術単純 |

### 8.4 Design Decision: 整列は sakura 焼込み・dola は不透明保持（案 2-A / 案 1-A）

- **Context**: 後続 cue をテキスト再生完了後に発火させるための整列主体（R1/R3・Topic 1 開発者決裁）。
- **Selected**: sakura compile が `offset += D`（`text_playback_duration` の戻り値）で絶対 `start_time` へ焼き込み、当該 Text cue へ同一 D を第一級データとして付与。dola は焼込み済み絶対時刻と不透明 D を保持・点配送。emo-text は配送 D から reveal（interval=D/N）。
- **Rationale**: dola の同期配送責務（同一台本を複数表現者へ同一絶対時刻で・プロセス跨ぎ）。時刻を配送時導出すると desync ゆえ絶対時刻は同期の必須要件。既存 `Wait` 累積と対称で `TimedSchedule` 中核不改変。単一真実源は計算層（純関数 1 本）で start_time と D は 2 投影を不変台本へ凍結（実行時ドリフト不能）。
- **Trade-offs**: `Cue`/`TalkCue` 双方への 1 フィールド追加と全 `TalkCue` リテラル更新（コンパイラ強制・低リスク）。得るのは serde 後方互換・低回帰・汎用性。
- **Follow-up（実装検証）**: ①`Cue` serde default 後方互換の roundtrip 檻。②Clear 前置の「先頭前置＋安定ソート＋同一 at FIFO」順序保証の統合檻。③`D=N×50ms` 時に reveal が旧挙動と**機能等価**（可視グリフ数一致・厳密ビット等価は主張しない・期待値は同一 `D/N` 算術で再計算）となることの確認。④`\C` 追記モード対応時は Clear 前置を**条件化**する必要（現状 `\C` 未対応ゆえ無条件前置で妥当・design Revalidation Triggers へ記録）。

### 8.5 Risks & Mitigations（設計フェーズ）

- **順序依存（Clear 前置）**: 「先頭前置＋安定ソート＋同一 at FIFO」に load-bearing 依存 → `same_at_cues_preserve_script_order_fifo` 相当の統合檻で固定。
- **テスト波及**: `TalkCue`/`Cue` へのフィールド追加で全リテラル更新 → コンパイラ強制検出（obsolete でなく「シグネチャ変更で要更新」＝更新方針）。reveal テストは D 駆動へ差し替え（検証意図は保存）。
- **R2.4 の字義**: 純関数が明示ウェイトを畳まない選択は R3.3/R3.4 と整合し二重計上を回避する意図的解釈。design.md に reconcile を明記済み（paper-over ではない）。
- **実機ゲート**: #3/#4/#6/`\s` は決定論外の人間サインオフ（6.5）・絶対パス起動必須（pasta.dll LoadLibrary 失敗回避）。

---

## 9. 設計ディスカッション #1（2026-07-13）— envelope 一律 duration・第一級 Wait・broadcast・honor 契約

> `/kiro-design` の設計ディスカッション（controller inline・chat window）で、design-validation の Critical Issue 2 件を triage し、開発者との対話で設計を「envelope 一律 duration ＋ 純粋 Wait cue の第一級化 ＋ 全 cue broadcast ＋ 全演者 duration honor 契約」へ収束させた。当初の design.md（envelope duration＋Wait 吸収＋型 routing）から**根本的に発展**したため全面改稿。開発者曰く「設計が迷走してただけで最初から要件はここまで含んでいる」——要件を、それが元々意味していた姿へ書き起こした。実コードは workflow（8 エージェント：ground 5＋analyze 3）で突合検証済み。

### 9.1 Category A（自明修正・即コミット）
- **reveal「ビット等価」→「機能等価」**（Critical Issue 2）: `Duration::from_millis(N*50).as_secs_f64()/N` は f64 リテラル `0.05` と一般に不一致（N=3 で ≈0.049999999999999996・約 1 ULP 差）。機能挙動（可視グリフ数）は不変だが、旧 0.05 由来の期待値へ `assert_eq!` すると flaky 化（deterministic-test-coverage-mandate と衝突）。design.md の「ビット等価」を「機能等価」へ緩め、テスト期待値を同一 `D/N` 算術で再計算する方針を明記。commit e91b12f6。

### 9.2 Category B / 開発者裁定の連鎖（要件を正しい姿へ）
討議は Critical Issue 1（R2.4 純関数 reconcile）を起点に、開発者の一連の裁定で設計の根本モデルを確定させた：

1. **純関数は暗黙 per-char のみ（案Y）**: R2.4 の「単一純関数が暗黙＋明示を加算」の字義は言い過ぎ。`\_w` は parser が `Instruction::Wait` へ分離済みゆえ、純関数へ畳むと compile 累積と二重計上。純関数＝暗黙のみ／`\_w`＝compile 合成、と要件文を精緻化（R3）。案Z（純関数に Instruction 列を食わせ一括畳み）は二重計上ゆえ棄却。

2. **duration は envelope（payload 埋め込み棄却）**: 開発者「Text にも再生時間フィールドを」→ 一般化「すべての時間を消費するコマンドに時間情報が必要」。これは payload 埋め込みでなく **envelope 一律フィールド**が素直な器（全 cue が一律に時間スロットを持つ）。決定打は後述の honor 契約。

3. **Wait は第一級 cue（吸収でなく発行）**: 開発者「Wait が無いと色々破綻する・時間だけを待機するコマンドは必要」。実コード突合（workflow）で判明した破綻点＝**吸収は末尾・単独の待ちを台本から消す**。dola は同一台本を複数プロセスの表現者へ配る＝**台本は自己完結した楽譜**であるべきで、吸収された末尾待ちは別プロセス表現者に見えず早期終了＝desync。`CueCommand::Wait`（action 空・unit・duration は envelope）を additive 追加し、compile が `\_w`→Wait cue 発行＋`offset += d`。`BarrierKind::Timeout` は全スケジュール停止の外部解決 seam ゆえ不採用（非ブロッキング既知累積待ちと意味論不一致）。

4. **配送は broadcast（型 routing 廃止）**: 開発者「キューコマンドが種類で配信されている？ たとえ自分に関係なくても全員に配信されるべき・演者側で無視すればよい」。実コード突合で、現状 `cue_target_of` は 1 cue→1 sink の中央振り分けで表現者は同一台本を共有していない一方、**seriko/emo-text は既に演者側 relevance フィルタを持つ**（中央振り分けと二重フィルタで冗長）。broadcast（全 cue を全 sink へ）＋演者側フィルタへ寄せる。`CueTarget::All` は不要（振り分け判断自体が消える・cue_target_of は演者側ヘルパへ降格）。

5. **honor 契約（duration を決して落とさない）＋ envelope 決着**: 開発者「自分が処理しないコマンドでも duration だけは絶対に無視してはダメ・これが時刻同期の肝」。これが **duration=envelope を強制**する決定打——表現者が**コマンドを解釈せずに duration を honor** するには、duration が payload に埋まっていてはならない（解釈できない表現者が抽出できない）。envelope なら command-agnostic に一律 honor 可能。Wait は「action 空・duration のみの基礎コマンド」＝この契約の純粋形。

6. **0 と「フィールド欠落」の峻別**: 開発者「duration が結果的に 0 なのは許容、だが duration フィールドそのものが無いコマンドという概念を作ると将来問題が出る」。全 presentation cue が envelope duration を持ち、瞬時は明示的 0（`#[serde(default)]` で 0 はワイヤ省略・型には常在）。「duration を持たない cue command」概念は作らない＝honor 契約に例外（分岐）を作らない。

### 9.3 「duration が本質的に非該当なコマンド」調査（開発者要請）
`command.rs` 実定義を突合し、duration 保持の要否を列挙：
- **`CueCommand`（8＋Wait）は全て presentation timeline イベント**＝duration は必ず問える（瞬時は 0）。Text=reveal D／Wait=待ち／Custom=command 依存／Clear・Emote・BalloonSurface・NewLine・Choice・EntityRef=0（将来アニメで >0 化しうる）。**②「フィールド欠落」該当は一つも無い**（全て ①「0 でフィールド在り」）。
- **duration が本質的に非該当なのは `CuePayload` の別アームのみ**: `Barrier(BarrierKind)`＝動的停止点・静的タイムライン外（時間は timeout であって duration でない・research §7.5）／`Routing(RoutingCommand)`＝制御プレーンで `ready()` に届かず表現者未配送。これらは `CueCommand` でなく最初から別物ゆえ、presentation cue に欠落は生じない。
- 正直な発見: 現行語彙では**瞬時（duration=0）が多数派**で「無 duration が特殊」は as-is では成り立たない。開発者の直感は**アニメが載る理想形**の視点（Emote 仕草・BalloonSurface フェード等が duration を持つ将来）で、envelope 一律 duration がその future-proof な器。

### 9.4 実コード突合サマリ（workflow ground truth）
- `Cue { actor, start_time, payload }`（duration なし・PartialEq 非導出）。`CueCommand` 8 variant（時間コマンド無し）。時間占有は `BarrierKind::Timeout{duration}` のみ（別アーム）。
- `TimedSchedule` は純点配送＝Payload は duration を持たず**時間を占有しない**（Barrier のみ全停止）。→ Wait cue は「時間を待たせる」のでなく「台本を完結させ・duration を honor 対象として運ぶ」もの。整列は焼き込み絶対 start_time が担う（二重待ち禁止）。
- `cue_target_of` は exhaustive・catch-all 無し（variant 追加でコンパイラ強制）。`on_tick` は 1 sink emit。seriko は実装済みで自前 `cue_target_of` 判定・emo-text は網羅 match で非担当を明示無視＝**両者とも演者側フィルタ済み**（broadcast 化で本来の役目に）。
- `CueSheet::new` 安定ソート＋`on_tick` 同一 at FIFO で Clear→Text 順を担保。`TimedSchedule` の非負/NaN ガードは debug-only（release で NaN が partition_point を壊す・D3-V/P25）→ duration の有限・非負は計算層 `text_playback_duration` で保証。

### 9.5 要件反映
- **R1** を「全 cue の再生時間保持と絶対時刻同期配送」へ拡張（envelope 一律・0 とフィールド欠落峻別・Barrier/Routing 非該当を明記）。
- **R2** 新設「全表現者の duration honor 契約」（broadcast・action 可否に関わらず duration honor・演者側 relevance）。
- **R3** 純関数を暗黙 per-char のみへ精緻化（`\_w` は compile 合成）。
- **R5** 新設「純粋 Wait cue の第一級発行と自己完結台本」。
- **R7** emo-text に honor 契約 AC 追加。**R9** に Wait additive・envelope 一律・欠落概念不採用を追加。**R10** に動的制御は dola 外を追加。
- 旧 R2〜R8 は R3〜R10 へ繰り下げ（設計ディスカッションでの正当な要件更新）。**本 research の §1〜§8 は旧番号を参照**（対応: 旧 R1→新 R1 拡張／旧 R2→R3／旧 R3→R4／旧 R4→R6／旧 R5→R7／旧 R6→R8（例 旧 R6.4「\s 同期」→R8.4・旧 6.5「人間サインオフ」→R8.5）／旧 R7→R9（旧 R7.3/7.4「serde」→R9.3/9.4）／旧 R8→R10（旧 R8.1「wintf」→R10.1）。R2「honor 契約」は新設）。§9 以降は新番号。

## 10. 設計最終検証（討議 #2・敵対的再検証）

### 10.1 経緯
`/kiro-design` 再実行（開発者「最終検証としてデザインプロセスを回せ・議題があるなら新たに引っ張り出せ」）。finalized（GO・討議 #1 解消済）の design.md に対し、6 視点 × 実コード接地の敵対的 workflow を実施（trailing-wait-drive-lifecycle／broadcast-collateral／clear-talk-boundary／requirements-coverage-ears／serde-additivity-partialeq／reveal-degenerate-math ＋ synthesis）。16 raw findings → 新論点 5・obvious fix 5・非問題 5。判定 **GO-with-new-topics**（アーキ破壊 blocker なし）。

### 10.2 Topic 1（blocker×2・コード実証）: drive 層の早期終了
design.md は R2.2 honor を「talk ライフサイクル（`CompiledTalk.end` → drive）: 全時間範囲 max(start_time+duration)＝CompiledTalk.end… 早期終了しない」に帰していたが、実機構が不在:
- (a) `CompiledTalk.end`（compile.rs:117-122）は `TalkEndReason` enum（Ended/Quit）で**時間量でない**（名前衝突・時刻のように読める）。
- (b) 自然終了 drive.rs:226-238 は `is_completed()`＝entry 枯渇（＝最終 cue の**配送時刻**）で `TalkDone`+Break を発火、**duration 盲目**（Entry::Payload は offset のみ）。
- (c) File Structure（design.md:145-147）は broadcast+duration 複写のみで end_time も終了条件変更も無い。
- (d) 計画テスト（design.md:388）は compile-level 性質（sheet の max が末尾 Wait を含む）のみ確認＝ランタイム早期終了を構造的に捕捉不能。
- 帰結: 末尾 `\_w[800]`（Text@0 dur D／Wait@D dur 0.8）で `TalkDone` が t=D＝0.8s 早期・裸末尾 Text（at=0 dur D）で t≈0 に完了＝reveal 中。`TalkDone` は kanade 単一 slot へ流れ早期に slot 解放＝#3/重なり再発。R5.3（台本から extent 復元）は満たすが R2.2（drive が早期終了しない）は未実装＝両者を混同。

### 10.3 開発者決裁（あるべき姿）
開発者「今のコードは関係ない・**あるべき姿**を実装せよ・今のコードが間違っているからこの spec が立った」（[[analyze-ideal-form-not-minimal]]）。開発者の直感「**duration 期間が終わるまでコマンドは終了扱いにしない**」＝正しい理想。さらに「`start_time` とは別に **CueSheet レベルで絶対開始時刻**を持たせられる」を提起。→ **解: `CueSheet` が `absolute_start_time` を保持する自己完結絶対時刻台本**。cue は点でなく区間 `[start_time, start_time+duration)`。**talk 絶対終了時刻 = `absolute_start_time + max(start_time+duration)`**（台本のみから導出可能）。配送は各 cue の絶対時刻で点配送のまま（二重待ちなし）、**完了のみ duration-aware**（entry 枯渇でなく現在 ≥ 絶対終了時刻・`TimedSchedule` が horizon 保持）。2 段の焼き込み: compile が相対 start_time を焼き、dispatch が絶対アンカーを刻印。

### 10.4 決定の含意
- (1) **同期**: 絶対アンカーが台本に載る＝broadcast された台本を受けた任意表現者（プロセス跨ぎ）が協調なしに同一絶対発火/完了時刻を独立算出（dola の本質の物理的な形）。
- (2) **自己完結**: R5.3 を最強形へ（extent でなく絶対的配置が台本で閉じる）。
- (3) **早期終了解決**: 完了が「台本から導ける絶対終了時刻」＝どの層で判定しても同一値。ゆえに horizon の設置層は些末事化し、決めるべきは**データ配置**（台本に `absolute_start_time`・各 cue に `duration`）だった。
- 棄却（案乙）: horizon を sakura `CompiledTalk` のみに持たせ dola schedule を点配送の素の道具に留める案は、duration 普遍（D5）下で「cue を零幅点と見なす timeline」＝「duration 欠落 command」のタイムライン版の穴を残すため不採用。

### 10.5 要件・設計反映（Topic 1）
- requirements.md: **R1.7 新設**（CueSheet 絶対開始時刻・自己完結台本）。**R2 を 2 段 honor へ**（R2.2 葉＝否定的 no-op／**R2.5 新設**ライフサイクル＝絶対終了時刻まで早期終了せず）。R9.3 に `CueSheet.absolute_start_time` additive を追加。In scope に自己完結絶対時刻台本を明記。
- design.md: honor 契約 2 段書き換え・`CompiledTalk.end`（`TalkEndReason`）と終了「時刻」の峻別・File Structure（sheet.rs/schedule.rs/drive.rs 完了判定）・**D6 新設**・Testing に drive-level 注入 tick 檻（compile-level では捕捉不能）・Data Models（CueSheet 構造・区間・絶対終了時刻）・Boundary/Revalidation Trigger 追随。
- 記憶 [[areka-dola-absolute-time-sync-broadcast]] を「CueSheet 絶対開始時刻＝自己完結絶対時刻台本／完了は占有 horizon」で更新。

### 10.6 残 Topic（未討議・順次噛み砕いて進行）
- **Topic 2**: `areka-ghost`（live-path crate）がスコープ欠落。broadcast+Wait で sink.rs 網羅 match・dispatcher partition assert・spine_e2e・LogSink 二重 trait（二重ログ）が破損。File Structure/Revalidation Trigger へ要追加。
- **Topic 3**: wintf ECS `CueQueue` が envelope duration を構造的に落とす（Entry/pop_ready に duration 枠なし）。ただし areka live 経路は wintf pipeline 未使用ゆえ desync でなく doc/契約矛盾（compile.rs:116・design.md:58）。
- **Topic 4**: `RevealSchedule.visible()` の partition_point が負/NaN duration 未ガード。跨プロセス自己完結台本モデルで untrusted duration が到達しうる第一級経路。design.md:367 の「monotonic 保持」主張は跨 chunk で偽。
- **Topic 5**: R6.2（新 talk はテキストのみ表示）vs R6.3/design（書込スコープのみ Clear 前置）。新 talk が書かないスコープの前 talk 文字が永久残存（emo-text に talk 境界リセットなし）。emo2 単キャラで潜伏・M2 多キャラで顕在。
- **obvious fix（5・順次適用）**: ①File Structure スコープ過少（wintf/dola sheet_test.rs ~30・全 Cue/TalkCue 構築点）②areka-ghost sink.rs Wait arm ③test-helper duration 穴（Cue に PartialEq 無く cue_eq/field assert が duration 比較を強制されない）④Revalidation Trigger に broadcast で talk_clock 観測集合変化 ⑤design.md:367 文言修正（monotonic は第 1 chunk のみ）。

### 10.7 Topic 2 解決（areka-ghost スコープ組込・推奨案 A＝甲）
実コード突合で 3 波及を確認: (1) [sink.rs:56-67](../../../crates/areka-ghost/src/sink.rs) `command_kind` が 8 variant を catch-all 無しで網羅＝Wait 追加でコンパイルエラー (2) [sink.rs:39-49](../../../crates/areka-ghost/src/sink.rs) `LogSink` が SurfaceSink/TextSink 両実装＋[dispatcher.rs:101-105](../../../crates/areka-ghost/src/dispatcher.rs) が両スロットへ同一 sink clone＝broadcast で二重ログ (3) [dispatcher.rs:630-638](../../../crates/areka-ghost/src/dispatcher.rs) が `surface.len()==2`／`surface[0/1]==Emote{9/8}`（`\s[9]…\e`/`\s[8]again\e`）を檻＝broadcast で surface も Text 受信し破損。**決定（開発者「推奨案で」）**: areka-ghost を File Structure・Revalidation Triggers・Testing Strategy へ計上。`command_kind` に Wait 腕（catch-all 無し流儀）。`dispatcher`/`spine_e2e` の per-sink partition assertion を broadcast-aware（surface も全 cue 受信・relevance で action 選択）へ改訂。**診断既定 sink を単一 broadcast 配線へ寄せ cue ごと 1 回ログ**（二重ログ解消・D4「挙動保存の唯一の破れ」明記）。scaffold 限定ゆえ実 sink（seriko/emo-text 別オブジェクト）差込では元々無関係。obvious fix ①（横断機械変更に wintf/dola tests/全 Cue・TalkCue 構築点明記）②（sink.rs Wait 腕）④（Revalidation に broadcast で talk_clock 観測集合変化）を同時に畳込み。

### 10.8 Topic 3 解決 → cue 再生エンジンの dola 層統合（討議 #2・スコープ拡張 (A)）
Topic 3（wintf CueQueue が duration を落とす doc 矛盾）を深掘りした結果、より根源的な構造問題が判明:
- `compile_sheet`（dola・min 正規化＝先頭待ちを食う）と `to_schedule`（sakura・絶対保存）の **2 変換**が並存。[drive.rs:281](../../../crates/areka-sakura/src/drive.rs:281)「compile_sheet は使わない（先頭待ちを消すため禁止）」＝似て非なる 2 版。
- wintf `ecs/cue`（`CueQueue`＝[queue/mod.rs:63](../../../crates/wintf/src/ecs/cue/queue/mod.rs:63) で `TimedSchedule` に委譲する薄い ECS ラッパ・状態機械/バリア/choice/tracker 付き）と sakura `drive`（actor）の **2 つの cue 再生"制御"**が並存。wintf `ecs/cue` は生きた App に未配線（`CueQueue::new`/`dispatch_pending_cue_sheets` は doc 例＋テストのみ）＝**旧世代**。
- dola の実体は [lib.rs](../../../crates/dola/src/lib.rs) の `"Declarative Orchestration for Live Animation"`（storyboard/keyframe/transition/runtime/playback）＝アニメ統括層。
- **開発者決裁**: 「wintf ecs/cue は旧世代・撤去可・コードは 1 か所に集約・アニメ制御は dola 層が筋・正しく検討設計して提案せよ」→ 提案（責務→住処の層分け）承認→「**(A) 本 spec へ畳む**」。
- **解（D7）**: cue 再生ランタイムを dola `cue` へ集約——(1) 変換 1 本（先頭待ち保存・絶対アンカー・horizon）(2) `CuePlayer` 受動ランタイム（状態機械・完了 horizon・バリア seam・Choice・broadcast fan-out）(3) `CueSink` 1 本＋`cue_target_of`（SurfaceSink/TextSink 統合）(4) 配送エンベロープ型。sakura は front-end＋talk glue に縮小（配送再実装せず）。seriko/emo-text は `dola::CueSink` 実装。**wintf `ecs/cue` 撤去**（compile_sheet 含む）。dola は受動ライブラリのまま（アクター化は sakura）・cue runtime は storyboard runtime と別モデルの兄弟として cue モジュールへ。
- **3 topic 同時解消**: Topic 1（horizon）＝dola runtime 本来責務／Topic 2（二重ログ/2 sink trait）＝CueSink 1 本で消滅／Topic 3（変換二重化/wintf duration 欠落）＝変換 1 本＋wintf 撤去で消滅。
- **反映**: requirements R11 新設（6 AC）・In scope 追記。design Overview 統合節・D7・File Structure（dola `runtime.rs`/`sink.rs` 新規・`sheet.rs` 変換統合・`drive.rs` 縮小・`contract.rs` 移設・wintf 撤去）・emo-text/seriko/ghost を CueSink 化。記憶 [[areka-cue-runtime-consolidated-in-dola]] 新設。
- **詳細設計初手（未確定）**: dola 既存 `runtime`（storyboard 系: interpolator/timeline_manager/instance_manager…）と cue runtime の結線は cue モジュール内新設で確定（storyboard runtime へ混ぜない・精読済）。

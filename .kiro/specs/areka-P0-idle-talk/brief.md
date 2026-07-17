# Brief: areka-P0-idle-talk

> **種別**: 本坑（main）増分。③ kanade 帰属（M-life 構成要素）。roadmap 増分「③ kanade: `idle-talk`（OnSecondChange 自発会話）」の brief 化。
> **調査日**: 2026-07-16（再入精査⑦・実装偵察で**背骨は既に配線済み**と判明→「正典充足＋実機サインオフ」へ再スコープ）。
> **並走性**: 編集面は kanade のみ＝実装中の `areka-P0-cue-playback-duration`（dola/sakura/emo-text/seriko）と**真に並走可**。**【2026-07-17 訂正】**「編集面は kanade のみ」は要件討議 #2 の `Status` 拡充で失効——実編集面は **kanade＋shiori-host32-host＋ShioriBackend 実装（areka-ghost/areka）の4クレート**（research §3/§9.4 が正本）。cue モデル（ゲート定義面）との交差ゼロは真＝並走可の結論は不変（cue-playback は 2026-07-17 完了済み）。

## Problem

emo2 の心臓部は **OnSecondChange**（毎秒・これが pasta 内部の OnTalk/OnHour/コールバックを駆動＝`doc/emo2-conformance-scope.md` §1「最重要・心臓部」）。ところが areka の OnSecondChange リクエストは**正典 Reference/共通ヘッダを満たしていない**疑いが濃く（Ref3 以外の充足状況が未検証・`Status` ヘッダ不送出）、**自発会話（放置トーク）の実機動作も一度も検証されていない**。M-boot の R9.3 サインオフは boot talk→close のみで、「放っておくと喋り出す」という伺かの基本体裁が未証明。

## Current State（2026-07-16 実装偵察）

- **自発会話の背骨は既に在る**: Steady フェーズの Tick pump（`crates/areka-kanade/src/schedule/steady.rs:55-80`）が、active talk 無しなら **GET** で OnSecondChange（`:63-66`・Ref3="1"）→ 応答 `Value` を **StartTalk として起動**（`steady.rs:92-103`＝`Steady{None}`＋Value→StartTalk＋`Steady{Some}` 遷移）。active talk 中は **NOTIFY**（`:69-76`・Ref3="0"）で Value 破棄（`:110-120`・DD-6 防御）。**「イベント→talk」経路の新設は不要**。
- **正典とのギャップ（本 spec の実体）**:
  - **Reference 充足**: ukadoc 正典は Ref0=OS 連続起動時間（hour）・Ref1=見切れ（1/0）・Ref2=重なり（1/0）・Ref3=トーク再生可能（1/0）。現実装 `events.rs:86-101` `on_second_change(now, talk_playable)` の Ref0〜2 充足状況を design 冒頭で実査し、不足を埋める。
  - **`Status` 共通ヘッダ不送出**: emo2 は `Status`（talking/choosing/online 等）で**OnSecondChange の発火制御を行う**（scope doc §1・実需確定）。kanade は talk 有無を知っている（`Steady{talk}`）のに Status を載せていない——M1 は `talking`（active talk 中）を最小実装（choosing は M-dialogue で増分）。
  - **NOTIFY 側にも正典義務**: ukadoc「トーク再生不能な時は、Reference3 が 0 になった上で NOTIFY でイベント通知される。返されたスクリプトは無視される」＝現実装は正典どおり（檻で固定する）。
- **送信イベント集合の規律**: `OnTalk`/`OnHour` は**送ってはいけない**（emo2 が OnSecondChange 内で内部生成・二重発火防止＝scope doc §1）。現実装は送っていない（Input/Action enum に該当なし）——**檻が無い**ので回帰檻を作る。
- **実機観測ゼロ**: emo2 fixture の dic に `OnSecondChange` ハンドラ名は無いが `hour.pasta`（時報系）が在る＝pasta が OnSecondChange から内部駆動する構図。放置→自発会話の人間サインオフが未実施。

## Desired Outcome

OnSecondChange リクエストが**正典 Reference（Ref0〜3）＋`Status` ヘッダ**を満たし、実機で emo2 を放置すると**自発会話（時報等）が発火**し、active talk 中は割り込まない。送信イベント集合（OnTalk/OnHour 不送出）が回帰檻で固定される。

**✔ 観測（単一 pass/fail）**: 決定論（mock shiori・注入 Tick・sleep 不使用）＝(a) Steady{None} Tick→GET・Ref0〜3 の値が期待列 (b) Steady{Some} Tick→NOTIFY・Ref3="0"・`Status: talking`・Value 破棄 (c) 送信イベント名のホワイトリスト固定（OnTalk/OnHour が現れない）。＋実機＝実 emo2・実 pasta で放置→自発会話の人間サインオフ。

## Approach

1. **design 冒頭の実査**: `events.rs:86-101` の現 Reference 充足を確認し、正典4値（Ref0=OS 連続起動 hour・Ref1=見切れ・Ref2=重なり・Ref3=cantalk）との差分表を design.md に載せる。
2. **Ref0〜2 の値源と縮退**: Ref0＝OS uptime（`GetTickCount64` 系・注入可能な時刻源に載せ決定論維持）。**Ref1（見切れ）/Ref2（重なり）は M1 固定 0＋算出シーム**（真値は窓 geometry＝UI スレッド側の知識。kanade worker へ運ぶには Tick メッセージへの付帯が要る＝**TickInfo 拡張の口だけ**設計し、実測は増分へ。emo2 dic が見切れ/重なりを実際に使うかを design で grep 確認し、未使用なら固定 0 の正当性を記録）。
3. **`Status` ヘッダ**: kanade の ShioriCall 生成（`events.rs`）へ共通ヘッダ注入の口を設け、`Steady{talk}` から `talking` を導出（talk 無し時は省略 or `Status` 無し＝SSP 挙動を ukadoc/実物で確認）。~~ヘッダ付与は `Shiori3Client` の既存汎用 request 経路に乗る（host 側改変なしを design で確認・`build_request` は汎用ヘッダ対応済み）。~~ **【2026-07-17 反証済み＝この前提を信じないこと】** `build_request` は固定ヘッダ集合のみで任意ヘッダ注入機構は無い（`shiori3.rs:58-118` 実測）＝**host32（`shiori-host32-host`）の破壊的改変が必須**（research §3 の file:line 証拠・要件討議 #1 で受容済みコスト）。
4. **回帰檻**: 送信イベント集合ホワイトリスト（mock shiori が受けた ID 列を検査）・NOTIFY Value 破棄・Status 遷移（None→無/Some→talking）を決定論テスト化（[[deterministic-test-coverage-mandate]]）。
5. **実機サインオフ**: 実 emo2 を数分放置→自発会話（hour.pasta 系）の発火を目視。talk 中の非割込みも確認。

## クロスユニット契約（並走を詰ませない事前考慮・2026-07-16）

- **cue-playback-duration と交差面ゼロ**: 編集面は `crates/areka-kanade`（steady/events）＋テストのみ。dola/sakura/emo-text/seriko 不触＝時限ゲート非該当。**ただし実機サインオフは talk 再生品質（#3/#4）の影響を受ける**——サインオフ判定は「自発 talk が発火する」ことに限定し、再生タイミングの正しさは cue-playback の受け入れに帰属（判定を混ぜない）。
- **position-persist との近接**: 双方 kanade を触るが boot.rs（persist＝初回ゲート）vs steady.rs/events.rs（本 spec）——`events.rs` は双方が additive（persist=OnFirstBoot Ref0・本 spec=OnSecondChange Ref/Status）＝別関数・並走可。
- **input-events との契約整合**: `Status: choosing`（選択肢表示中）は M-dialogue の増分＝本 spec はヘッダ注入の**口**（enum 化）だけ用意し、値の増分（`choosing`）は **`areka-P0-choice-select-events`**（2026-07-16 名称確定・再入精査⑧）が足せる形に。
- **ticker は不触**: 毎秒 Tick の供給は ghost-setup ✅ の ticker（絶対グリッド整列）が正本——本 spec は受けた Tick の**中身の充足**のみ扱う。

## ukadoc 必読（design 着手時に ukadoc MCP `get_doc`/`search_docs` で正典参照・2026-07-16 裏取り済み）

- **`ukadoc:list_shiori_event:OnSecondChange:1`**（裏取り済み）: Ref0=OS 連続起動時間（hour）・Ref1=見切れ・Ref2=重なり・Ref3=トーク再生可能。「再生不能時は Ref3=0 で NOTIFY・返却スクリプト無視」。
- **`ukadoc:memo_shiorievent`**: SHIORI Event の GET/NOTIFY 使い分けの考え方（総論）。
- **SHIORI/3.0 リクエスト共通ヘッダ（`Status`/`Sender`/`Charset`）**: ukadoc MCP の検索では単独ページが引けなかった（2026-07-16）——design で `list_categories`→protocol 配下を確認し、見つからなければ **`doc/emo2-conformance-scope.md` §1（emo2 実需: Status 9種で発火制御）を実需正本**として採用・SSP 実挙動（talk 無し時に Status を送るか）は保守的に「active talk 中のみ talking」で開始し記録。

## Scope

- **In**: OnSecondChange の Reference 正典充足（Ref0 実値・Ref1/2 固定 0＋シーム・Ref3 既存）／`Status` ヘッダ注入（talking・拡張 enum の口）／送信イベント集合ホワイトリスト檻／NOTIFY 破棄・GET→StartTalk 経路の回帰檻強化／実機の自発会話サインオフ。
- **Out**: 見切れ/重なりの実測（増分・シームのみ）／`Status: choosing`・選択肢連動（M-dialogue）／OnMouseMove 等の入力イベント（`input-events`）／talk 再生タイミングの正しさ（`cue-playback-duration`）／secondchangeinterval 設定（plugin 領分・M1 外）／OnTalk/OnHour の送出（**恒久禁止**・檻で固定）。

## Boundary Candidates

- Reference/ヘッダ充足（events.rs 純粋生成＝全網羅可能）／Status 導出（Steady 状態→ヘッダ値の純関数）／実機サインオフ（放置観測）。

## Out of Boundary

- Tick 供給機構（ghost-setup ✅ ticker）／talk 配送・再生（dispatcher/sakura/emo-text）。

## Upstream / Downstream

- **Upstream**: `completed/areka-P0-kanade`（Steady pump・StartTalk 経路＝背骨）／`completed/areka-P0-ghost-setup`（ticker・dispatcher）／`completed/areka-P0-host32-request`（`Shiori3Client` 汎用ヘッダ経路）。
- **Downstream**: M-life（統合点）／`input-events`・choice-render（Status 拡張の口を消費）／`emo2-conformance-e2e`（放置トークを一周適合に含める）。

## Existing Spec Touchpoints

- **Extends**: `completed/areka-P0-kanade`（steady/events の additive 充足・純粋状態機械の既存決定論テスト資産は不変に保つ）。
- **Adjacent**: `areka-P0-position-persist`（kanade 近接・別フェーズ＝並走可）／`areka-P0-cue-playback-duration`（**編集面交差ゼロ**・実機判定の帰属だけ分離）。

## Constraints

- Rust 2024・新規依存なし・tokio 不使用。
- **決定論**: 注入 Tick・mock shiori のみで全経路網羅（[[deterministic-test-coverage-mandate]]）。OS uptime も注入可能な形（[[test-only-decision-branches-not-proven-wiring]]）。
- **実機受け入れ**: 実 emo2・実 pasta.dll で放置→自発会話（[[areka-placement-real-ghost-first]]）。起動は絶対パス必須（MOD_NOT_FOUND 運用注意）。
- 正典は ukadoc・emo2 は最小適合 fixture（[[ukadoc-mcp-preferred-source]]）。

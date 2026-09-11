# Requirements Document

## Project Description (Input)
kanade（ゴーストの運行を司る状態機械）が起動系列の最終段（基盤バージョン通知 `basewareversion` の応答待ち＝`BootVersion`）に滞在している間に、起動挨拶の再生完了通知（`TalkDone{Ended}`）が届くと、その通知が捨てられる。トーク枠が「再生中」のまま定常運転へ持ち越され、以後の終了の握手（`OnClose` → 終了挨拶 → `\-` → 自己停止）が二度と始まらない。

- **困っている人**: areka でゴーストを動かす利用者。終了指示を出しても終了挨拶が流れず、窓が閉じない。
- **現状**: `crates/areka-kanade/src/schedule/mod.rs` の横断遷移 `on_talk_done` は `BootVersion{talk: Some}` を突合対象に含めて防御しているのに、一致後の委譲先 `boot::step`（`crates/areka-kanade/src/schedule/boot.rs`）の「上記以外」の分岐が `warn!(event="boot_input_ignored")` を書いて通知を捨てる。発現には起動挨拶の再生完了が `basewareversion` の応答より先に届く必要があり、実機では窓が開かない（起動挨拶は数秒・応答は数ミリ秒）。決定論の検証環境では起動記録トークが空になり得るため再現できる。2026-09-10 の実機一周では点灯語 `event=boot_input_ignored` が全走行 0 行＝未発現。
- **変わるべきこと**: `BootVersion` 滞在中に届いた追跡中トークの完了通知を捨てず受理してトーク枠を空にし、起動完了後の終了の握手が成立するようにする。決定論テストで直す前は赤・直した後は緑を固定し、既存の起動系列の試験と `boot_input_ignored` の綴りは変えない。

出所: 完了仕様 `areka-P0-emo2-conformance-e2e` の決定論層の作業（タスク 5.5・2026-09-06）が構造から見つけた製品欠陥。登記は同仕様 `verification/acceptance-record.md` §13.2 行 4・design D9。起票は 2026-09-11（`/kiro-discovery`・開発者「起票候補はすべて起票せよ」）。ロードマップ W13 ②。

---

## Introduction

areka のゴーストは、起動すると「初期化 → 利用者名の照会 → 初回起動／通常起動の問い合わせ → 基盤バージョンの通知」という決まった順序で SHIORI（ゴーストの脳）と交信し、その途中で起動挨拶の再生を始める。起動挨拶は「再生し終わるのを待たずに次の交信へ進む」形で起動され、その完了通知は後から届く。kanade はこの挨拶を**追跡中のトーク**として枠（`talk: Some`）に載せたまま起動系列を終え、定常運転に入ってから完了通知と突き合わせて枠を空にする（完了仕様 `areka-P0-idle-talk` の設計判断 DD-IT-12）。

問題は、完了通知が**起動系列を終える前**——最終段の `basewareversion` の応答待ち（`BootVersion`）に居る間——に届いた場合である。突き合わせそのもの（`mod.rs` の `current_talk_id`）は `BootVersion{talk: Some}` を対象に含めており一致する。しかし一致後の処理は「相ごとの委譲」で `boot::step` へ渡され、`boot::step` は起動指示・SHIORI 応答・終了指示しか扱わず、それ以外を「起動系列に無関係な入力」として `warn!(event="boot_input_ignored")` と共に捨てる。その結果、枠は `Some` のまま定常運転（`Steady{talk: Some}`）へ持ち越される。定常運転では「再生中のトークがあるなら終了指示は保留して完了を待つ」規律があるため、もう来ない完了を待ち続け、終了の握手（`OnClose` の問い合わせ）が永遠に始まらない。利用者には「終了指示を出しても終了挨拶が流れず、窓が閉じない」と見える。

2026-09-11 時点のコードで確認した事実:
- `boot::step` が `TalkDone` を受け取るのは、横断遷移 `on_talk_done` で追跡中の talk と識別子が一致し、かつ理由が非 quit（`Ended`／`Interrupted`）だったときだけである（識別子が一致しない通知は `unknown_talk_done`（error）または 1 世代 stale の info で先に捌かれ、委譲へ来ない）。起動系列の相のうち追跡中の talk を持てるのは `BootVersion{talk: Some}` のみである。
- 追跡中の talk の理由 `Quit` は横断遷移で終了系列（Quit）へ直行し、`boot::step` へは来ない。
- 起動系列中に届いた終了指示は `pending_close` として保留され、起動完了後（`Steady{talk: None}`）の次の Tick で握手が始まる。挨拶の追跡がある場合（`Steady{talk: Some}`）は完了通知の到達時に保留を消化して握手が始まる。
- 既存の決定論テストは「起動完了後（`Steady{Some}`）に完了通知が届く」順序だけを固定しており、「起動完了前（`BootVersion{Some}`）に届く」順序は 1 本も無い。`boot_input_ignored` の発火テストは `BootInit` に Tick を入れる形で存在する。

本仕様は、この取り落としを直し、決定論テストで固定する。起動系列の順序・交信内容・areka 側の配線は変えない。

---

## Boundary Context

- **In scope（利用者・運用者から見える範囲）**
  - 起動系列の最終段（`basewareversion` の応答待ち）で追跡中の起動挨拶の完了通知が届いたとき、通知を捨てずにトーク枠を空にすること。
  - その後の終了指示（起動完了後に届くもの・起動中に届いて保留されていたもの）で、終了の握手（`OnClose` の問い合わせ）が始まること。
  - ログ語彙 `boot_input_ignored` を「本当に無関係な入力」（Tick など）にだけ残し、綴りを変えないこと。受理は沈黙させず記録すること。
  - 直す前は赤・直した後は緑となる決定論テストと、既存の起動系列テストの不変。
  - 対象は `crates/areka-kanade/src/schedule/{boot.rs, mod.rs}` とその兄弟テストファイル。
- **Out of scope**
  - 起動系列の順序そのもの（交信の順番・各段の発行内容・`basewareversion` の Status 導出）。
  - areka 側の配線（`crates/areka/src/emo2_boot/spine.rs`）・終了の握手の配線（`areka-P0-emo2-conformance-e2e` 6.9 で着地済み）・ホスト窓スレッドの pump（別仕様 `areka-P0-host32-window-thread-pump`）。
  - 定常運転・終了系列での完了通知の扱い（`steady.rs`／`close.rs`）の変更。
  - 実機での再現・実機サインオフ（実機では窓が開かないため決定論テストが唯一の検出器）。
- **Adjacent expectations（隣接する仕様・機構への期待）**
  - `areka-P0-emo2-conformance-e2e`（完了）: 手順書 §5.7 と記録 §7 が `event=boot_input_ignored` を点灯語として数えている。綴りと「捨てるときに書く」意味は保つ。`crates/areka` の決定論一周（`spine_conformance_*`）は本仕様の非回帰の検出器であり、期待値を変えない。
  - `areka-P0-idle-talk`（完了）の DD-IT-12: 起動挨拶の正規追跡（`BootVersion{talk}` → `Steady{talk}` の引き継ぎ）は保つ。本仕様は追跡の**途中で枠が空く**経路を足すだけである。
  - `areka-P0-kanade`（完了）: 「boot 中の終了指示は保留のみ・握手は定常運転で」の規律は保つ。起動中に完了通知を受理しても握手は始めない。

---

## Requirements

### Requirement 1: 起動系列の途中に届いた起動挨拶の完了通知の受理
**Objective:** As a areka でゴーストを動かす利用者, I want 起動挨拶が起動系列の途中で再生し終わっても kanade がそれを見失わないこと, so that 以後の終了指示で終了挨拶が流れ、窓が閉じる

#### Acceptance Criteria
1. While kanade 運行状態機械が起動系列の最終段（`basewareversion` の応答待ち）にあり起動挨拶トークを追跡している（`BootVersion{talk: Some}`）, when その追跡中のトークと識別子が一致する再生完了通知（理由 `Ended`）が届く, the kanade 運行状態機械 shall その通知を受理してトーク枠を空（`talk: None`）にし、`basewareversion` の応答待ちという待ち点は維持する。
2. While 同じ状態にある, when 理由が `Interrupted` の一致する完了通知が届く, the kanade 運行状態機械 shall `Ended` と同一に扱う（既存の「非 quit はフェーズ固有遷移へ」という 3 値の写像を保つ）。
3. When 1 または 2 の受理の後に `basewareversion` の応答（通知完了）が届く, the kanade 運行状態機械 shall 起動系列を完了し、トーク枠が空（`talk: None`）の定常運転へ入る。
4. While 同じ状態にある, when 理由が `Quit` の一致する完了通知が届く, the kanade 運行状態機械 shall 従来どおり終了系列（Quit）へ直行する（本仕様で変えない）。
5. When 1 または 2 の受理が起きる, the kanade 運行状態機械 shall 副作用指示（SHIORI への呼出・トーク起動・自己停止）を 1 件も発行しない（起動系列の交信の順序と内容を変えない）。
6. The kanade 運行状態機械 shall 受理の前後で、既に発行済みの `basewareversion` 通知の内容（追跡中は Status: talking）を再送も修正もしない。

### Requirement 2: 起動完了後の終了の握手の成立
**Objective:** As a areka でゴーストを動かす利用者, I want 起動挨拶が早く終わった起動でも終了指示が効くこと, so that 終了挨拶が流れて窓が自分で閉じる

#### Acceptance Criteria
1. When Requirement 1 の受理を経て定常運転（`Steady{talk: None}`）に入った後に終了指示が届く, the kanade 運行状態機械 shall 直ちに `OnClose` の問い合わせ（GET）を発行して終了の握手を始める。
2. If 起動系列の途中に終了指示が届いて保留されており（`pending_close`）、かつ Requirement 1 の受理を経て定常運転（`Steady{talk: None}`）に入った, then when 次の Tick が届く, the kanade 運行状態機械 shall 保留を消化して `OnClose` の問い合わせ（GET）を発行する。
3. While 起動系列がまだ `basewareversion` の応答待ちにある（Requirement 1 の受理の後・起動完了の前）, the kanade 運行状態機械 shall 終了の握手を始めない（`OnClose` の問い合わせを発行しない）。握手の開始は定常運転に入ってからとする（「boot 中の終了指示は保留のみ」の規律を保つ）。
4. When 起動挨拶が起動完了の後に再生し終わる（`Steady{talk: Some}` で完了通知が届く）, the kanade 運行状態機械 shall 従来どおり枠を空にし、保留があれば握手を始める（本仕様で変えない）。

### Requirement 3: ログ語彙と観測可能性
**Objective:** As a 実機一周の運用者, I want `boot_input_ignored` が「起動系列に無関係な入力を捨てた」ときにだけ点き、受理は別の語で記録されること, so that 生ログの行数で欠陥の発現と直しの効きを判定できる

#### Acceptance Criteria
1. When Requirement 1 の受理が起きる, the kanade 運行状態機械 shall `event=boot_input_ignored` を書かない。
2. When 起動系列の途中に起動進行と無関係な入力（Tick など）が届く, the kanade 運行状態機械 shall 従来どおり `event=boot_input_ignored`（warn・target `kanade`）を書いて捨てる。綴りは変えない（完了仕様 `areka-P0-emo2-conformance-e2e` の手順書 §5.7・記録 §7 の点灯語）。
3. When Requirement 1 の受理が起きる, the kanade 運行状態機械 shall 受理したことを info 以上のログ（target `kanade`・`event` フィールド・対象の talk_id 付き）でちょうど 1 行記録する（沈黙の経路を作らない）。
4. The kanade 運行状態機械 shall 追跡中のトークと識別子が一致しない完了通知の扱い（未知 talk_id の error `unknown_talk_done`・1 世代 stale の info `talk_done_stale_choice`）を変えない。

### Requirement 4: 既存の起動系列挙動の保存
**Objective:** As a 開発者, I want この直しが起動系列の他の経路を 1 つも変えないこと, so that 実機一周で固定した起動の交信列と既存の決定論テストがそのまま通る

#### Acceptance Criteria
1. The kanade 運行状態機械 shall 起動系列の交信の順序（`OnInitialize` → 利用者名の照会 → `OnFirstBoot`／`OnBoot` → `basewareversion`）と各段の発行内容を変えない。
2. While 起動挨拶が無い起動（`BootVersion{talk: None}`・`OnBoot` が 204 かつ起動記録トークも無い）にある, the kanade 運行状態機械 shall 従来どおり振る舞う（受理対象の完了通知は構造上届かず、`Steady{talk: None}` へ完了する）。
3. The kanade 運行状態機械 shall 起動系列の途中に届いた終了指示の保留（`pending_close`）と、起動完了後の保留の消化を変えない。
4. The kanade 運行状態機械 shall 起動記録トーク（初回起動で挨拶が無い場合の空の起動記録トーク）の追跡を変えない——その完了通知も Requirement 1 の受理対象に含まれる（決定論の検証環境で再現する経路）。
5. The 既存の起動系列テスト（`boot_sequence_tests.rs`・`boot_reply_branch_tests.rs`・`schedule_log_firing_tests.rs` の boot 関連）shall 期待値を変えずに通る。

### Requirement 5: 決定論テストと検証
**Objective:** As a 開発者, I want 欠陥を再現する決定論テストが直す前に赤・直した後に緑であること, so that 直しが効いたことと再発しないことを機械で示せる

#### Acceptance Criteria
1. The 本仕様 shall 次の系列を固定する決定論テストを追加する: 起動を `BootVersion{talk: Some}` まで進め、追跡中のトークの完了通知（`Ended`）を投入し、⑴ トーク枠が空になる、⑵ `event=boot_input_ignored` が 0 行、⑶ 続く `basewareversion` の応答で `Steady{talk: None}` へ入る、⑷ 続く終了指示で `OnClose` の問い合わせ（GET）が発行される。
2. The 追加する決定論テスト shall 直す前のコードで赤（失敗）であることを実装の記録で示し、直した後に緑（成功）となる（RED 先行）。
3. The 本仕様 shall 保留経路の決定論テストを追加する: 起動系列の途中に終了指示を投入して保留させ、`BootVersion{talk: Some}` で完了通知（`Ended`）を投入し、`basewareversion` の応答で `Steady{talk: None}` へ入り、次の Tick で `OnClose` の問い合わせ（GET）が発行される。
4. The 本仕様 shall 起動系列の途中の Tick が従来どおり `event=boot_input_ignored` を書くことを固定する（既存テストで足りるならそれを保つ）。
5. When 実装が完了する, the `cargo test -p areka-kanade` と `cargo test -p areka --bin areka` shall いずれも全緑で通る（終了コードで判定し、出力の一部だけを見て判定しない）。
6. The 追加するテスト shall 対象の本番ファイルの兄弟テストファイルに置き、本番ファイル・テストファイルとも 1 ファイル 1,000 行以下を保つ。

# ギャップ分析: areka-P0-emo2-conformance-e2e

> 作成日: 2026-09-02 / 対象: 確定済み `requirements.md`（Requirement 1〜12）と現行コードベースの差
> 方針: **情報を出し、決定はしない**。選択肢と根拠を並べ、判断は要件討議へ渡す。
> 本文中のコードに関する記述はすべて実ファイルを読んで `file:line` で確かめたものだけを書いている。

---

## 1. 分析サマリ

- **一周を辿るテストの母体は既に 2 系統あり、どちらも「新しい仕組みを作らない」という要件（R2.6）を満たせる。** 台本化した応答による系統（`crates/areka-ghost/tests/ghost/spine_e2e_test.rs:83` の S1 ほか）と、実ビルドの x64 テスト DLL を実際に読み込む系統（同 `inproc_e2e_test.rs:656`／`:782`）である。後者は交信の列と演出の列を**同一の走行から二重に記録して全順序で照合する**形が既にできている（`inproc_e2e_test.rs:939`・`:782`）。
- **足りないのは「入力を注入して一周を伸ばす」部分である。** 撫で・二重クリック・選択確定の注入口は実在する（`crates/areka-kanade/src/msg.rs:134`／`:138`、実行時の窓口は `crates/areka-ghost/src/runtime.rs:218`）が、**一周を辿るテストから使われている例は 0 件**であり、使用箇所は単体テストと本番配線だけである（後述 2.2 の全数一覧）。
- **見た目（バルーン・拡大率・掴んで動かす・重なり順・位置調整）を観測できるのは `areka` クレート側のテスト用組立であり、`areka-ghost/tests/` 側ではない。** 前者は実 emo2 資産・画面用の World・実の表示指令経路まで組む（`crates/areka/src/emo2_boot/spine.rs:641` 以下、表示指令の観測は `spine_display_tests.rs:8`、位置調整は `spine_move_cue_tests.rs:1-17`）。**決定論一周テストをどちらの家に置くかが最大の設計判断**であり、ロードマップの干渉台帳が想定する編集集合（`.kiro/steering/roadmap.md:99`＝`areka-ghost/tests/ghost/*` 新規）とも直結する。
- **実 SHIORI を実際に読み込む系統を撫で・メニューまで伸ばすには、凍結応答の表を増やす必要がある。** 現在の凍結表は **`OnFirstBoot` ただ 1 件**である（`crates/shiori4-testdll/src/snapshot.rs:43-48`）。増やす作業は `crates/shiori4-testdll/` の改変であり、「本番コード改変 0 が原則」（`.kiro/steering/roadmap.md:88`）および R12.1 の編集集合と衝突しうる。台本化した応答の系統を採ればこの衝突は起きない。
- **完成判定の 3 点は仕組みとしては揃っているが、「1 か所から辿れる」形にはなっていない。** テスト全通過と許諾検査は完了手続きが持ち（`.claude/skills/kiro-complete/SKILL.md:116-117`）、有界の自動終了と記録用の出力水準を使う実機走行は既存の先例が持ち（`crates/areka/tests/emo2_real_run.rs:150-164`）、人間サインオフの文言も既にある（同 `:77-79`）。本仕様は**これらを繋ぐ記録様式と項目表を作ることが実質の成果物**になる。

---

## 2. 現状の資産地図

### 2.1 一周を辿る既存テスト（2 系統）

| 系統 | 実体 | 駆動 | 記録している列 | 一周のどこまで |
|---|---|---|---|---|
| 台本化した応答 | `crates/areka-ghost/tests/ghost/spine_e2e_test.rs`（S1〜S7 は同名の分割ファイル群） | 注入した時刻のみ。待ちは `spin_pumping_ticks`（`spine_e2e_test.rs:48-66`・上限 60 秒＝同 `:22`） | backend への呼出列（`RecordedCall`＝同 `:73-88`）と演出列（`RecordingSink`） | 起動・接続失敗・死活・終了握手・終了期限・切断・2 回目起動 |
| 実 DLL を読み込む | `crates/areka-ghost/tests/ghost/inproc_e2e_test.rs` | 同上（`:715` からの Tick 注入ループ・壁時計は宙吊り防止の上限のみ） | 交信列（`ExchangeRecord`＝`recorder.rs:51-62`）＋演出列（`RecordingSink`）の**二重記録**（`inproc_e2e_test.rs:782`） | 起動 → 起動挨拶 → 正常終了（`:939` の期待列は 6 件で閉じる） |

補足:

- 実 DLL 系統の期待列は **起動系列（初期化通知 → 利用者名の照会 → 初回起動 → ベースウェア版）と終了系列（終了挨拶 → 解放）だけ**である（`inproc_e2e_test.rs:939-972`）。撫で・メニュー・選択・位置調整は 1 件も入っていない。
- 実 DLL 系統の走行に使うゴーストは、**実 emo2 の見た目資産と、実採取の応答を凍結した x64 テスト DLL の組み合わせ**である（`crates/areka-ghost/tests/ghost/inproc_fixture.rs:147-151` が emo2 の実資産を指し、同 `:211-221` が資産の実体化を行う）。組立は 1 度だけ行い共有する（同ファイルの共有取得口）。
- 交信の記録は進行状態のヘッダを**保持している**が（`recorder.rs:58-59`）、現在の照合は種別・ID・結果の 3 つ組だけで、進行状態は照合対象から明示的に外されている（`inproc_e2e_test.rs:930-938` の注記）。**R3.8（進行状態が交信のヘッダに正しく現れること）は、記録はできているが照合していない状態**である。

### 2.2 入力の注入口

- 撫で・二重クリック: `crates/areka-kanade/src/msg.rs:66`（値の形）・`:81-85`（移動と二重クリックの種別）・`:134`（メッセージの枝）。
- 選択確定: 同 `:105`（値の形）・`:138`（メッセージの枝）。選択待ちの期限は `:145` の別枝で運ばれる。
- 実行時の窓口: `crates/areka-ghost/src/runtime.rs:218`（kanade への送信端）・`:223`（dispatcher への送信端）。
- **使用箇所の全数**（`KanadeMsg::Mouse`／`KanadeMsg::Choice` を含むファイル）:
  本番配線 3 = `crates/areka/src/input_events/mod.rs:183`／`:216`、`crates/areka/src/input_events/choice_drain.rs:71`、`crates/areka-kanade/src/actor.rs:88`／`:91`。
  単体・機能別テスト = `crates/areka-kanade/tests/kanade/mouse_test*.rs`（4 ファイル）・`choice_test_*.rs`（6 ファイル）・`crates/areka-ghost/src/dispatcher_choice_tests.rs`・`crates/areka/src/input_events/input_events_tests.rs`。
  **一周を辿るテスト（`spine_e2e_test*`・`inproc_e2e_test`・`emo2_boot/spine_*`）には 1 件も無い**——要件の前提 2 のとおりである。
- 選択肢のホバー反転: 判断は純関数（`crates/areka/src/emo2_boot/hover_inject.rs:98` の巡回判定・`:124` の駆動）だが、有効化は**処理系で 1 度だけ読んで焼き付ける環境変数**である（`:29` の変数名・`:184` の 1 度きりの解決）。**テストごとの切り替えはできない**ので、見た目としてのホバーは実機層の項目になる（R1.4 の振り分け対象）。

### 2.3 選択確定の発火形（R3.5・R11.1 の根拠）

- 実装は選択肢 ID の先頭で 3 分岐する（`crates/areka-kanade/src/schedule/choice.rs:56-60`）。`On` 始まりは**同名 1 段のみ**で、正典の選択確定イベントを先行させない（同 `:22-26` の注記）。
- **適合対象の選択肢は全て `On` 始まり**である。実物の辞書 `crates/pilot/examples/shiori-host-32/fixtures/emo2/ghost/master/dic/menu.pasta:15`（`Onおしゃべり頻度メニュー`・`Onエモの位置調整メニュー`・`Onメニュー閉じる`）、同 `:33`（4 件）、同 `:62`（2 件）。**辞書に正典の選択確定イベントの受け口は 1 件も無い**（同ディレクトリ全体の検索で 0 件）。
- 正典側（ukadoc）: `\q[タイトル,ID,r2,r3...]` の項は「選択後 `OnChoiceSelectEx` が開始される」と書き、`On` 始まり ID の直接発火については**沈黙している**（`https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_5cq_5b_30bf_30a4_30c8_30eb_2cID_2cr2_2cr3..._5d:1`）。実装側の注記（`choice.rs:22-26`）が「正典は先行段の有無に沈黙し、実物は直接発火に依存する」と書いているのと整合する。
- 一方、実物定義の文書は `OnChoiceSelectEx` を「メニュー選択肢確定」として単独で挙げている（`doc/emo2-conformance-scope.md:24`）。**実装・実物・正典の三者を突き合わせると、この 1 行だけが実態とずれている**——R11.1 の訂正対象がここで確定する。

### 2.4 見た目の観測点

- **表示指令の列**: `crates/areka/src/emo2_boot/spine_display_tests.rs:8-15` が表示指令の対象を取り出し、同ファイルの有界な取り出しループが注入 Tick と交互に集める。表示指令そのものは時刻を持たない（対象・種別のみ）。
  → **R2.4 の「表示指令の列（時刻と指令）」は、時刻を持つ演出の列（`TalkCue` の `at` と指令＝`inproc_e2e_test.rs:782-797` が照合している対象）を指していると読むのが自然**。表示指令と演出の列は別物であり、どちらを固定するかは設計判断（後述 7-③）。
- **位置調整（台本の移動指令）**: `crates/areka/src/emo2_boot/spine_move_cue_tests.rs:7-17` が、台本 → 指令 → 経路 → 実際の窓移動までを 1 本で押さえている。
- **拡大率の切替・二体の隣接・バルーンの追従**: `crates/areka/src/emo2_boot/frame_chain_realign_tests.rs`（遷移後に隣接へ解き直す）・`frame_transition_atomicity_tests.rs`・`frame_balloon_offset_follow_tests.rs`・`frame_work_area_resnap_tests.rs`（作業領域の取り直しでは追従の相対位置に触れない）が既に在る。**R4.4 の 3 つの期待値は、すべて既存の決定論テストが持っている**。
- **掴んで動かしたときの追従**: `crates/areka/src/placement/follow/drag_follow.rs` と兄弟テスト（`follow_drag_tests.rs`・`follow_balloon_drag_tests.rs`・`follow_drag_end_persist_tests.rs` 他）。
- **再表示直後の重なり順**: 再断行の要求（`crates/wintf/src/ecs/window/zorder_pair.rs:124`）は公開の契約点として宣言され消費側も在る（`zorder_pair_establish.rs:6`）が、**本番コードで挿入している箇所は 1 件も無い**（`crates/areka/src/placement/spawn.rs:629` は説明の注記であり挿入ではない。挿入は `spawn_zorder_pair_export_tests.rs:54` のテストのみ）。要件の「上流から引き受けた 3 件の確認」の 1 つ（再表示直後の重なり順）は、**この未消費が実機でどう見えるかを確かめる項目**である。

### 2.5 実機走行の仕組みと記録の先例

- 有界の自動終了と記録用の出力水準は既存の実走テストが持つ: 子プロセス起動（`crates/areka/tests/emo2_real_run.rs:151-164`）で、自動終了の遅れ（`:59`＝3000ms）と出力水準（`:160`＝`info,kanade=trace`）を渡し、番犬（`:63`＝120 秒）で括る。
- 人間が目視する走行は**別立て**で、寛大な自動終了（3 分）を付けて実バイナリを直接起動する手順が同ファイルの注記に逐語である（`:105-116`）。**この 3 分の走行には番犬が無い**ことも明記されている（`:114`）。
- 判定の材料（機械可読）は 4 点を出力から探す形: 正常終了・結線成立・装着完了・折返しの解決（`:208-242`）。**R6.5 の「経路の不達・観測行 0 行を失敗として扱う」は、ログ判定側の先例が既に持っている**（`crates/areka/src/placement/transition_signoff_tests.rs:9-31`＝未設定・読めない・観測行 0 行のいずれも失敗にする）。その判定は明示実行の指定で隔離されている（同 `:59`）。
- 記録様式の先例 4 系統はいずれも実在する: `.kiro/specs/completed/areka-P0-ghost-window-zorder/verification/signoff.md`／`.kiro/specs/completed/areka-P0-window-placement/acceptance-record.md`／`.kiro/specs/completed/areka-P0-balloon-offset-dpi/signoff-2026-08-28.md`／`.kiro/specs/completed/areka-P0-collision-dpi-hittest/acceptance-record.md`。
- **食い違いの読み分け手順は完成品が在る**: `.kiro/specs/completed/areka-P0-dpi-transition-atomicity/signoff-procedure.md` の 3 つの問い＋4 行の表（同ファイルの §6.5）と、7 行の合否ブロック（同 §6.6）。R6.1・R6.7 は**この 2 つをそのまま使う**のが最短で、独自の規約を作る理由は見当たらない。

### 2.6 実 SHIORI（32bit の脳）の受け渡し

- 子プロセスへの受け渡しは 3 経路すべてを同時に渡す形で実装済み: 引数 3 本（`crates/shiori-host32-host/src/process_host.rs:243-245`）、同じ 3 値の環境変数（`:247-249`）、作業ディレクトリ（`:251`＝読み込み元へ設定）。設計意図は同ファイル `:24` の注記にある。
- **R5.8（受け渡しを実走で 1 度通したことを記録する）は、この 3 経路が実際に子で使われた証跡を実機走行の出力から拾う**ことになる。現在の実走テストが探す 4 マーカー（2.5）にはこの証跡が入っていないので、**探す語を 1 つ増やすか、別の観測点を選ぶ**判断が要る（後述 7-⑥）。

### 2.7 完成判定の 3 点

1. **テスト全通過**: 完了手続きが `cargo test --workspace` を回して確認する（`.claude/skills/kiro-complete/SKILL.md:119-123`）。直近に確認済みなら省略してよいとも書かれている（同 `:123`）。
2. **許諾の検査と第三者告知**: 設定ファイルが在るときだけ 2 コマンドを回し、無ければ「設定不在により省略」と記す（同 `:125-138`・特に `:138`）。**要件 10.2 が言う「通ったものとして扱わない」は、この省略の記し方に対応する**。
3. **実機サインオフ**: 人間の判断。文言の先例は `crates/areka/tests/emo2_real_run.rs:77-79`。
4. **アーカイブ移動の後にもう一度テストを回す門が在る**: 完了手続きは、spec を完了置き場へ移した**後**に `cargo test --workspace` を再実行することを課しており、これは省略できない（`.claude/skills/kiro-complete/SKILL.md:295`・省略不可の根拠は同 `:298-299`）。**R10.1 の「テストが成功で終わること」は 2 度計られる**ことになるので、判定手順にどちらを書くかを決める必要がある。
5. **判定手順に書くべき前提**: 32bit の橋渡し実行体を先に用意すること。完了手続きの検査項目には無く、実走テストの注記に前提として書かれているだけである（`crates/areka/tests/emo2_real_run.rs:37-39`）。R10.3 はこの位置ずれを埋める要件である。

### 2.8 間欠的な赤の 3 系統

| 系統 | 実体 | 現状の書き方 | 兄弟の書き方 |
|---|---|---|---|
| ⑴ 記録が非空になるのを待つだけ | `crates/areka-ghost/tests/ghost/spine_e2e_test_s3_helper_liveness_detected.rs:146-164` で「1 件でも出たら抜ける」待ちの後、`:175-183` で**待ちを 1 つも挟まずに** 5 呼出を数えて等値で照合する | 待ちと数えるが分離しており、数える側に待ちが無い | `crates/areka/src/emo2_boot/spine_boot_smoke_tests.rs:32-36` は**同じ 5 呼出の確認を、条件が満たされるまで待つ形**で書いている（`spin_wait_until`＝`crates/areka/src/emo2_boot/spine.rs:361`・上限 30 秒＝同 `:332`） |
| ⑵ 実窓の重なり順が他プロセスに割り込まれる | `crates/wintf/src/ecs/window/zorder_pair_maintain_always_on_top_tests.rs:411` 近傍・`:760` 近傍（実際の窓を作って隣接を測る） | 実窓の位置関係を直に測るため、環境の窓に割り込まれると崩れる | — |
| ⑶ 壁時計の期限が負荷で飢える | `crates/wintf/src/runtime/tick_bridge.rs:353-356`（500ms 以内に画面同期の通知が来ることを要求）。`crates/areka/src/emo2_boot/spine_boot_smoke_tests.rs:46-49` と `spine_talk_close_tests.rs:306-309` は上限つきの待ちの**後**に件数を主張する形 | 期限そのものが判定に効く | — |

- **⑴ の是正は要件 9.2 が「待ちを伴う形へ更新する」と明示している**。手本（`spine_boot_smoke_tests.rs:32-36`）が同一の確認について既に在るので、移植は機械的である。ただし対象ファイルは `areka-ghost/tests/ghost/` 配下＝**本仕様の編集集合の中**（R12.1）。
- **⑵⑶ は編集集合の外**（`crates/wintf/src/...`）。R9.8 が「範囲を記録し、併走する仕様と衝突しないことを確かめる」を課している根拠がここにある。
- 明示実行で隔離する既存の書き方は `crates/areka/src/placement/transition_signoff_tests.rs:59` の形（理由つきの `#[ignore]`＋環境変数）。
- 逆に「門を持たないこと」を自ら要件にしているテストがあるため無差別な隔離はできない（R9.7）。またファイル行数の上限には**明示的な例外表**があり、件数を別の定数で二重に持って暗黙の増加を防いでいる（`crates/log-capture-kit/tests/file_length_guard_test.rs:61`・`:109`＝11 件）。R2.11 の分量規律はこの仕組みで機械的に守られる。

### 2.9 正本の文書と併走

- `doc/emo2-conformance-scope.md`（92 行）: 送るイベントの一覧（`:20-26`）・省略してよい機能（`:28`・`:44`・`:67`）・生態系拡張は M1 後（`:90`）。**R11.1 の訂正対象は `:24`。R11.2 の充足済み注記もこの文書に入る。**
- `.kiro/steering/roadmap.md`: M1 ゴール文（`:15`）・W11 完走で残りは本仕様のみ（`:62`）・本仕様のゴール文（`:72`）・着手時義務 5 点と「証明に徹する」（`:88`）・干渉台帳（`:99-107`）。
- 併走の見通し（`:99`）: 本仕様の編集集合は `areka-ghost/tests/ghost/*` の新規と `doc/emo2-conformance-scope.md` とされ、同時期の 3 本（`cursor-tag-canon`・調査系 4〜5 本・`sakura-bare-tag-lexer`）とは**共有ファイル 0** と実測されている。ただし `:101` には**併走側からの保存義務**が書かれている——本番の出力先の列（`crates/areka-ghost/src/runtime.rs` の結線）に出力先が足されると、本仕様のテストが件数や順序を固定していた場合に併走側が更新する、という取り決めである。
- 実測を目的とする走行は並走させない（R5.9）。`tick-gate-adoption` は本仕様と並走不可と明記されている（`.kiro/steering/roadmap.md:95`）。

---

## 3. 要件と資産の対応表

タグ: **有**（そのまま使える）／**要拡張**（既存を伸ばす）／**不足**（新設が要る）／**未確定**（設計で決める）／**制約**（既存の決まりが縛る）

| 要件 | 対応する資産 | 判定 |
|---|---|---|
| R1.1-1.3 一周の定義・二層の分離 | 文書側の仕事。先例＝`.kiro/specs/completed/areka-P0-dpi-transition-atomicity/signoff-procedure.md` の層分け | 不足（文書） |
| R1.4 決定論で観測できない項目の明示 | ホバー反転が実例（`hover_inject.rs:29`・`:184`＝処理系で 1 度きりの環境変数） | 有（根拠あり） |
| R2.1-2.2 実時計非依存で 1 本の走行 | `inproc_e2e_test.rs:656`（注入時刻のみ）／`spine_e2e_test.rs:48-66`（待ちながら時刻を進める） | 要拡張 |
| R2.3 交信の列を全順序で照合 | `inproc_e2e_test.rs:939-972`（6 件）／`recorder.rs:51-62` | 要拡張（列を伸ばす） |
| R2.4 表示指令の列を全順序で照合 | 演出の列＝`inproc_e2e_test.rs:782-797`（`:788` で内容一致・`:793-796` で時刻の非減少）。表示指令の列＝`spine_display_tests.rs:8-15`（時刻なし） | 未確定（どちらを指すか・7-③） |
| R2.5 期待に無いものが 1 つでも出たら失敗 | 既存はいずれも等値照合＝この性質を持つ（`inproc_e2e_test.rs:975`・`:788`） | 有 |
| R2.6 新しい仕組みを発明しない | 台本化した応答（`spine_e2e_test.rs:73` 以降）・記録用の受け口・有界の待ち（`:48`）がすべて在る | 有 |
| R2.7 全ての実行主体が有界時間で終わる | `inproc_e2e_test.rs:641`（別スレッドへ逃がして期限つきで受け取る）＋終了の照合（`:900-910`） | 有 |
| R2.8 明示実行の門を持たない | 既存の一周テストはいずれも門なし | 制約（隔離の裁定と両立させる必要） |
| R2.9 本番コードを改変せずに注入 | 注入口は公開済み（`runtime.rs:218`）。ただし実 DLL 系統を伸ばすなら凍結表の追加が要る（`snapshot.rs:43-48`＝現在 1 件） | 未確定（7-①） |
| R2.10 壁時計だけの待ちを避ける | 手本 2 つ＝`spine_e2e_test.rs:48-66`／`spine.rs:361` | 有 |
| R2.11 1 ファイルの分量規律 | 機械判定つき（`file_length_guard_test.rs:61`・`:109`） | 制約 |
| R3.1 起動の列 | 実測済みの列が 2 か所に逐語で在る（`inproc_e2e_test.rs:941-963`／`spine_boot_smoke_tests.rs:50-60`） | 有 |
| R3.2 毎秒の変化通知で自発会話 | 現在の一周テストは**この通知を送らない設定**で決定論を得ている（`inproc_e2e_test.rs` の注記＝ticker 無効・dispatcher は Tick を中継しない） | 不足（7-④） |
| R3.3-3.4 撫で・二重クリックの通知 | 値の形は `msg.rs:66`・`:81-85`。単体テストは `mouse_test_event_layout_tests.rs` 他 | 要拡張 |
| R3.5 選択確定は同名 1 段のみ | 実装 `choice.rs:56-60`・実物 `menu.pasta:15`／`:33`／`:62`・単体テスト `choice_test_named_event_tests.rs` | 有（一周への移植が要る） |
| R3.6-3.7 送らないことの固定 | 「余計なものが 1 件でも出たら失敗」という等値照合で自動的に効く | 有 |
| R3.8 進行状態がヘッダに現れる | 記録はしている（`recorder.rs:58-59`）が照合から外している（`inproc_e2e_test.rs:930-938`） | 要拡張 |
| R3.9 終了握手と解放 1 回 | `inproc_e2e_test.rs:964-973`（終了挨拶 → 解放）／S4 の分割ファイル | 有 |
| R3.10 利用者名の展開 | 起動系列に利用者名の照会が入っている（`inproc_e2e_test.rs:948-952`・実 DLL は該当資源を持たないため既定値経路）。`crates/areka-ghost/tests/ghost/sylphya_integration_test.rs` が別途在る | 要拡張 |
| R4 適合検証項目表 | brief の 14 項目（`brief.md:81-98`）＋申し送り。期待値の更新分は既存テストが持つ（2.4） | 不足（文書・本仕様の中核成果物） |
| R5 実機走行の条件と記録 | 有界の自動終了と出力水準（`emo2_real_run.rs:59`・`:160`・`:105-116`）。記録様式の先例 4 系統（2.5） | 要拡張（様式の確定） |
| R5.8 子プロセスへの受け渡し | 実装は 3 経路同時（`process_host.rs:243-251`）。実走の出力に専用の証跡が無い | 不足（7-⑥） |
| R6 食い違いの読み分け | 完成品が在る（`signoff-procedure.md` の §6.5・§6.6） | 有（引用して使う） |
| R6.4 実機ログの判定は同じ純関数 | 先例＝`transition_signoff_tests.rs:34`（読み取りと判定）・`:59`（明示実行） | 有 |
| R7 判定に載せない症状の登記 | 遅れの実測値と引受先不在は上流の記録に在る（`brief.md` 追記(77)・`.kiro/specs/completed/areka-P0-present-write-coherence/`） | 不足（文書） |
| R8 欠陥の仕分け | 運用規律。先例の言及が brief にある | 不足（文書） |
| R9 間欠的な赤の隔離裁定 | 3 系統の実体は 2.8 のとおり全て特定済み。手本（待つ形）と隔離の書き方（`transition_signoff_tests.rs:59`）も在る | 要拡張＋未確定（7-⑤） |
| R10 完成判定の一本化 | 3 点の仕組みは在る（2.7）が 1 か所に無い | 不足（文書） |
| R11 正本の更新 | 訂正対象は `doc/emo2-conformance-scope.md:24` と確定（2.3） | 有（対象確定） |
| R12 併走との境界 | 干渉台帳（`.kiro/steering/roadmap.md:99-107`）・保存義務（`:101`） | 制約 |

---

## 4. 実装方式の選択肢

### 選択肢 A: 実 DLL を読み込む系統を伸ばす（`inproc_e2e_test.rs` の形の拡張）

**どこを触るか**: `crates/areka-ghost/tests/ghost/` に一周テストのファイルを新設し、`recorder.rs`・`inproc_fixture.rs`・`RecordingSink` を再利用する。あわせて `crates/shiori4-testdll/src/snapshot.rs:43-48` の凍結表へ、撫で・二重クリック・選択確定・終了に対する実採取の応答を足す。

- 利点: 交信の列と演出の列を二重に記録する形（`inproc_e2e_test.rs:782`）がそのまま使える。応答が**実 pasta の実採取**なので、期待列が実物と乖離しない。凍結値が差し替われば期待列が必ず落ちる（`:664-672` の検出の仕組み）。
- 欠点: **凍結表の追加が `crates/` 配下の改変**であり、「証明に徹する（本番コード改変 0 が原則）」（`.kiro/steering/roadmap.md:88`）および R12.1 の編集集合と衝突する。採取自体は既存の採取用ハーネス（`snapshot_capture_test.rs:1-27`）で行えるが、**実 pasta と 32bit の橋渡しが揃った実機でしか採取できない**（同 `:6-9` の 2 変数の門）。
- 欠点: **見た目（バルーン・拡大率・追従・重なり順・位置調整）は 1 つも観測できない**。この系統は窓も画面も持たない。

### 選択肢 B: 見た目まで組む系統を伸ばす（`crates/areka/src/emo2_boot/spine*.rs` の形の拡張）

**どこを触るか**: `crates/areka/src/emo2_boot/` に一周テストの兄弟ファイルを新設し、既存の組立（`spine.rs:641` の `SpineHarness`）と台本化した応答（`:669-678` の標準台本）を使う。

- 利点: **一周の全段が同じ走行の中で観測できる**——表示指令（`spine_display_tests.rs:8-15`）・位置調整（`spine_move_cue_tests.rs:7-17`）・バルーンの表示ライフサイクル（`balloon_visibility_lifecycle_e2e_tests.rs`）・拡大率遷移（`frame_transition_atomicity_tests.rs`）・二体の隣接（`frame_chain_realign_tests.rs`）。R4.2 の追補項目のうち**決定論層で押さえられる範囲が最も広い**。
- 利点: 台本化した応答なので凍結表を触らずに済み、撫で・メニュー・選択の応答を自由に書ける（`spine.rs:669-678` の形をなぞる）。実 SHIORI にも 32bit にも依存しない（R2.1 に厳密に合う）。
- 欠点: **本番クレート `areka` の `src/` にファイルが増える**。プロジェクトの決まり（実装と同じ場所に兄弟のテストファイルを置く）には合致するが、ロードマップの干渉台帳が想定する編集集合（`:99`＝`areka-ghost/tests/ghost/*`）から外れるため、**併走との共有ファイル 0 を測り直す必要がある**。
- 欠点: 画面用の World を作るため、走行時間と環境依存が A より重い（実際、`spine.rs:332` の上限は 30 秒、`:394`・`:404` は静定の最小持続を壁時計で持つ）。R12.6（常設テストの実行時間）に効く。

### 選択肢 C: 二層に分ける（交信は A の家・見た目は B の家）

**どこを触るか**: 交信の列と演出の列を A の家（`areka-ghost/tests/ghost/`）で、見た目に関わる段を B の家（`areka/src/emo2_boot/`）で、**同一の台本を共有して**押さえる。台本の共有は台本化した応答の組み立て手順を揃えることで行う（両家とも `ScriptedShioriBackend` を使える——A 側は `spine_e2e_test.rs:73` 以降、B 側は `spine.rs:669`）。

- 利点: 各層が既存の得意な観測手段をそのまま使い、**凍結表にも本番コードにも触らない**。R12.4（決定論層と実機層を独立した節に保ち、分割の裁定に耐える）とも構造が揃う。
- 利点: 実行時間の重い側（B）だけを最小限に絞れる。
- 欠点: **「1 本の走行で一周を辿る」という R2.2 の字面に反する**。2 本になる。R1.6（後段の結果で埋め合わせない）との整合も設計で書き分ける必要がある。
- 欠点: 期待値が 2 か所に散る危険。R4.1（項目表は 1 か所・写しを持たない）と同じ危険が期待列にも生じる。

### 参考: どの選択肢でも共通に要るもの

- 撫で・二重クリック・選択確定の注入は `runtime.rs:218` 経由で本番コードを触らずに行える（R2.9 を満たす）。
- 自発会話（R3.2）だけは、**現在の一周テストが決定論のために毎秒の通知を止めている**ため、どの選択肢でも「時計を止めたまま毎秒の通知だけを注入する」形を新たに書くことになる。
- 間欠的な赤 ⑴ の是正（R9.2）は A の家のファイル（`spine_e2e_test_s3_helper_liveness_detected.rs:175-183`）に対する編集であり、選択肢に依らない。

---

## 5. 規模と危険度

| かたまり | 規模 | 危険度 | 一言 |
|---|---|---|---|
| 決定論一周テスト（R2・R3） | **L**（1〜2 週） | **中** | 仕組みは全部在るが、注入 → 交信 → 演出の三点を一周分そろえるのは初めて。自発会話の注入（R3.2）が唯一の新しい形 |
| 適合検証項目表（R4） | **M** | **中** | 14 項目＋追補 6 項目以上。期待値の出典追跡（R4.9）が手間。上流の裁定が 3 件効いている（R4.4） |
| 実機走行の手順と記録（R5・R6・R7） | **M** | **低** | 先例が 4 系統＋読み分け手順の完成品が在る。引用して組み替える作業が主 |
| 間欠的な赤の隔離裁定（R9） | **S**（⑴ の是正のみ）／**M**（⑵⑶ の裁定込み） | **中** | ⑴ は手本があり機械的。⑵⑶ は編集集合の外に触れる判断が要る。R9.6 が回数の上限を先に決めることを課している |
| 完成判定の一本化と宣言（R10・R11） | **S** | **低** | 文書作業。ただし R10.4 により人間の判断を待つ |
| 正本の更新（R11.1） | **XS** | **低** | 対象は `doc/emo2-conformance-scope.md:24` の 1 行と確定済み |

**全体**: **L〜XL**、危険度 **中**。危険の主因は「一周の段数が多く、どの段も既存の観測手段が別々の家に在る」こと。技術的な未知は少ない。

---

## 6. さらに調べる必要があるもの（設計フェーズへ）

1. **凍結表を増やす場合の採取可否**: 採取ハーネスは GET のみを対象とし、通知と解放は採取できない（`snapshot_capture_test.rs:16-21`）。撫で（通知）・二重クリック（通知）は**そもそも凍結の対象外**である可能性が高い。選択肢 A を採るなら、この点を先に確かめる必要がある。
2. **自発会話の注入形**: 毎秒の変化通知を注入しつつ時計を進めない形が、既存の待ちの仕組みと両立するか。`spine_e2e_test.rs:48-66` の注記は「kanade 宛の Tick は台本外の通知を誘発する」と警告しており、**この警告が意味する副作用を逆手に取る設計になる**。
3. **走行時間の実測**: R12.6（既存の常設テストの実行時間を実用の範囲を超えて延ばさない）の判定基準。選択肢 B は画面用の World を作るため、追加 1 本あたりの所要を測ってから決めるべき。
4. **ホバー反転の実機観測条件**: 環境変数（`hover_inject.rs:29`）を実機走行で有効にするか。有効にすると本番既定と違う条件で一周することになる（R5.3 が環境変数の明示を課している理由）。
5. **⑵⑶ の隔離が失う被覆の見積り**: R9.4 が「失われる被覆」の明記を課している。`zorder_pair_maintain_always_on_top_tests.rs` と `tick_bridge.rs:346-362` が何を守っているかの棚卸しが要る。
6. **正典の再確認（ukadoc）**: 進行状態のヘッダ 9 種（`doc/emo2-conformance-scope.md:20` に記載）と、送らないと決めた更新系 4 種＋バルーン変更（同 `:28`）が任意である根拠。brief が設計着手時の必読としている（`brief.md:115-119`）。

---

## 7. 設計判断イシュー（要件討議の材料）

> いずれも**決めない**。選択肢と、それぞれが解くこと・解かないことを並べる。

### 7.0 要件討議での仕分け（2026-09-02・カテゴリ B＝設計フェーズへ先送り）

要件討議で ①〜⑨ を精査した結果、次の 6 件は **要件の what/why を変えず、how だけを決めるもの**として設計フェーズ（`/kiro-design`）へ先送りする。要件側の拘束は括弧内のとおり既に確定している。

| # | 項目 | 要件側の拘束（設計はこの中で決める） |
|---|---|---|
| ② | 凍結応答を増やすか、台本化した応答で書くか | R2.9（製品コード非改変が原則・新設は理由と挙動不変の記録つき）・R12.1（編集集合）。①の裁定に従属 |
| ③ | R2.4 の「表示指令の列」が指すもの | 「時刻と指令」を持つ列であること（R2.4 字面）。①の家が決まれば観測できる列が定まる |
| ④ | 自発会話を決定論層でどう注入するか | **決定論層に載せることは R3.2 で確定済み**（実機層へ振らない）。注入形＝毎秒の変化通知を時計を進めずに入れる形を設計で確定（§6-②） |
| ⑥ | 子プロセスへの受け渡し（R5.8）の観測点 | 3 経路のどれを「1 度通った」と言うかを先に決める。テストファイルの改変を伴う案は R12.1 の範囲記録つき |
| ⑧ | 実機走行の記録様式 | R5.6（結果を見る前に期待を宣言）＋R6.7（定型の合否ブロック）を満たす合成形が要件に最も近い |
| ⑨ | 項目表の総数とロードマップ「14 項目」の対応表現 | R4.8（総数の登記と対応の明示）を満たせばよい。「14 を基礎＋追補 N」を既定案とし、ロードマップ側は R11.2 で閉じるときに揃える |

残る ①（一周テストの置き場と「1 本の走行」の厳密さ）・⑤（間欠的な赤 ⑵⑶ の扱い）・⑦（仕様の分割）は開発者の判断を要するカテゴリ C として討議に掛ける（結果は本節の後に追記する）。

### 7.1 議題 1 の裁定（2026-09-02・開発者確認済み）

**① 決定論一周テストの置き場＝選択肢 B（見た目まで組む家・`crates/areka/src/emo2_boot/` の兄弟テスト）で確定。R2.2「1 本の走行」は字義どおり維持する。**

再評価で確かめた事実:

- 走行時間の実測: B の家の既存 spine テスト 19 本（毎回ハーネス起動・実 emo2 資産・headless GPU World）は直列で **9.24 秒**（`cargo test -p areka --bin areka emo2_boot::spine -- --test-threads=1`・2026-09-02）。一周テスト 1 本の追加は 1 秒前後で、R12.6 の懸念にならない。
- B は今日すでに門なしで常設（`spine.rs:641` 以下＝`make_world_with_gpu`・MTA COM＋WARP 可）。注入端は `GhostRuntime::kanade()`／`dispatcher()`（`crates/areka-ghost/src/runtime.rs:218`／`:223`）、交信記録は `non_status_calls`（`spine.rs:287`）、表示指令の取り出しは `drain_received`（`spine_display_tests.rs:35`）、台本の移動指令→実窓移動は `spine_move_cue_tests.rs`。製品コード改変 0 で組める（R2.9）。
- 「本番クレートの `src/` にファイルが増える」は欠点ではなく規則: `areka` は bin のみ（`emo2_boot/mod.rs:29-34` の注記）で、内部到達テストは兄弟配置が唯一の形。
- A（`areka-ghost/tests/`）は `areka` bin に依存できないため、将来も表示経路を組めない。凍結表は GET 専用（`snapshot.rs:43-48`）で、増設は実機採取が前提。A の唯一の強み＝実 pasta の実採取応答は B に持ち込めないが、「pasta が何を言うか」は実機層（R5）の責務であり、既存の A のテスト（起動挨拶の凍結）はそのまま残す＝失う被覆は無い。
- 干渉の再実測（R12.3）: 本仕様が触るのは `emo2_boot/` の新規兄弟ファイル＋`spine.rs` のモジュール宣言 1 行。W12 併走の `cursor-tag-canon`（`areka-emo-text/src`）・調査系（新規 crate＋`consumer_ledger.rs` 等への doc コメント）・⓪ `sakura-bare-tag-lexer`（`lexer.rs`／`decode.rs`）と共有 0。ロードマップ干渉台帳の e2e 行（`roadmap.md:99`）は旧前提のままなので、R11.2 でロードマップを閉じるときに書き換える。

派生: ②（凍結応答か台本か）は「台本」で事実上決まる。③（R2.4 の列）は B で観測できる列（表示指令＋時刻付きの演出）から設計が確定する。

### 7.2 議題 2 の裁定（2026-09-02・開発者確認済み）

**⑦ 仕様は分割しない。1 spec の実装段階で相 1＝決定論一周テスト（R2・R3）、相 2＝実機一周走行と完成判定（R5〜R11）の二相に切る**（ロードマップ `roadmap.md:88` の既定を採用）。理由: 完成判定（R10.1）は相 1 のテストを含む全体テストの成功を前提にするため、分けても相 2 が先に進めるわけではなく、要件→設計→タスク→完了の手続きが 2 倍になるだけ。分割が効くのは「実機走行の時間がしばらく取れない」場合に限るが、その見込みは無いとの開発者判断。R12.4 は「移送できる形」から「相の境界を節構造で表す」へ改訂。

### 7.3 議題 3 の裁定（2026-09-02・開発者確認済み）

**⑤ 間欠的な赤 ⑵⑶ の `wintf` 側テストは明示実行の門で隔離する。** 既存の書き方（理由付きの `#[ignore]`＋環境変数・`transition_signoff_tests.rs:59` の形）を使い、判定ロジックは改変しない。対象は `crates/wintf/src/ecs/window/zorder_pair_maintain_always_on_top_tests.rs`（実窓の重なり順）と `crates/wintf/src/runtime/tick_bridge.rs:353-356`（画面同期の 500ms 期限）。失う被覆と根治の引受先（`areka-P0-zorder-chain-residue` A-1／A-2）は記録に残す（R9.4）。`emo2_boot` 側の同形 2 本（`spine_boot_smoke_tests.rs:46-49`・`spine_talk_close_tests.rs:306-309`）は「条件が揃うまで待つ」形で既に書かれており、期限は打ち切りの上限にすぎないため、触らず残す。妥当性の確認は隔離前・隔離後に各 3 回を上限とする（R9.6・開発者方針「長時間試行禁止」）。編集集合の外に触れる範囲は R9.8／R12.1 に事前登記（`wintf` は W12 併走の cursor-tag・調査系・⓪ lexer のいずれも触れない＝共有 0）。却下した案: 「そのまま残す」は R10.7 と衝突、「判定の走行だけ `--skip`」は「全通過」の意味が手順書依存になる。

**① 決定論一周テストをどの家に置くか**（選択肢 A／B／C）
- A（実 DLL の家）は交信の忠実さを解き、見た目を解かない。凍結表の追加＝`crates/` 改変が発生する。
- B（見た目まで組む家）は一周の段の広さを解き、本番クレートの `src/` にファイルが増える。
- C（二層）は両方を解くが「1 本の走行」（R2.2）を解かない。
- 判断材料: ロードマップの干渉台帳は A の家を前提に共有ファイル 0 を実測している（`.kiro/steering/roadmap.md:99`）。B/C を採るなら測り直しが要る（R12.3）。

**② 凍結応答を増やすか、台本化した応答で書くか**
- 増やす: 期待列が実物と乖離しない。ただし通知は採取対象外の可能性（6-①）。`crates/shiori4-testdll/` の改変は「証明に徹する」原則と摩擦する。
- 台本で書く: 改変 0 で書けるが、応答の内容が**人が書いたもの**になるため、実物との一致は別の手段（実機走行）でしか担保されない。

**③ R2.4 の「表示指令」が指すもの**
- 演出の列（時刻つき・`inproc_e2e_test.rs:782-797` が照合している対象）と読むか。
- 表示指令の列（対象と種別・時刻なし・`spine_display_tests.rs:8-15`）と読むか。
- 要件の字面「時刻と指令」は前者に合うが、「表示指令」という語は後者の名に合う。**どちらか、または両方**を設計で確定する必要がある。

**④ 自発会話（R3.2）を決定論層に載せるか、実機層へ振るか**
- 載せる: 毎秒の通知を注入する新しい形が要る（6-②）。決定論の維持が難所。
- 振る: R1.4 に従い理由を記録すれば可能だが、「放置で自発会話・会話中は割り込まない」は brief の項目 5 であり、**結合でしか現れない事象**（R1.5）の代表例でもある。決定論層から外すと本仕様の中心的な観測が 1 つ減る。

**⑤ 間欠的な赤 ⑵⑶ の扱い**
- 明示実行で隔離する: 常設の全通過が意味を取り戻す。失う被覆と引受先の明記が要る（R9.4）。編集集合の外に触れる（R9.8）。
- そのまま残す: 編集集合を汚さないが、R10.7（間欠的な赤が残ったまま「全通過」と記録しない）と正面から衝突する。
- 第三の形: 走行を分けて数える（例: 完成判定の走行では対象を絞る）。ただし R2.8 の「門を持たない」との整合を書き分ける必要がある。

**⑥ 子プロセスへの受け渡し（R5.8）をどう観測するか**
- 実走の出力に探す語を 1 つ増やす: `crates/areka/tests/emo2_real_run.rs:208-242` の 4 マーカーに 5 つ目を足す形。ただしテストファイルの改変になる。
- 別の観測点を選ぶ: 実 SHIORI が実際に応答したこと（起動挨拶が表示されたこと）をもって受け渡しの成立とみなす。間接的だが改変 0。
- 判断材料: 受け渡しは引数・環境変数・作業ディレクトリの 3 経路を同時に渡している（`process_host.rs:243-251`）ので、「1 度通った」の意味を**どの経路について言うのか**を先に決める必要がある。

**⑦ 仕様を分割するか**（要件が保留と明記・R12.4）
- 分割する: 決定論層（R2・R3）と実機層・完成判定（R5〜R11）を別仕様に。ロードマップの分割候補と一致する（`.kiro/steering/roadmap.md:88`）。
- 分割しない: 実装段階で二相に切る。R12.4 が「そのまま移送できる形」を課しているので、どちらを選んでも文書の構造は同じになる。

**⑧ 実機走行の記録様式をどの先例に揃えるか**
- 判定表＋根拠引用型（`ghost-window-zorder/verification/signoff.md`）／実測値中心型（`window-placement/acceptance-record.md`）／機械判定が合否を決める型（`balloon-offset-dpi/signoff-2026-08-28.md`）／証跡の分類を先に決める型（`collision-dpi-hittest/acceptance-record.md`）。
- R5.6（結果を見る前に期待を宣言する）は 4 つ目の型が持つ規則であり、R6.7（決まった書式の合否ブロック）は `dpi-transition-atomicity/signoff-procedure.md` の §6.6 が持つ。**2 つの先例を合成する**のが要件に最も近い。

**⑨ 適合検証項目表の総数と、ロードマップの「14 項目」との対応**（R4.8）
- brief の 14 項目（`brief.md:83-98`）に、追補として少なくとも 6 項目（バルーンの表示ライフサイクル・拡大率の切替・掴んで動かしたときの追従・二体の隣接・再表示直後の重なり順・子プロセスへの受け渡し）が加わる（R4.2）。
- ロードマップのゴール文は「適合 14 項目一周」と書いている（`.kiro/steering/roadmap.md:72`）。**総数を書き換えるのか、14 を基礎＋追補と表現するのか**を決める必要がある（R11.2 でロードマップの M1 の節を閉じるときに整合が要る）。

---

## 8. 次の段階

- 本文書は決定を含まない。①〜⑨ を要件討議に掛け、確定したものを設計（`/kiro-design areka-P0-emo2-conformance-e2e`）へ渡す。
- 設計着手時には brief が課している正典の再確認（`brief.md:115-119`）を行うこと（6-⑥）。

---

# 設計フェーズの調査（2026-09-02・`/kiro-design` 実施分）

> 本節より下は設計フェーズで追記した。上の §1〜§8 は要件フェーズの記録であり、改変していない。
> 記載はすべて実ファイルを読んで `file:line` で確かめたものだけである。**§9.1 は §2.3 の記述を 1 点訂正する。**

## 9. 正典の再確認（ukadoc・brief `:115-119` の設計着手時義務・§6-⑥ の解決）

ukadoc の MCP 検索で総覧を引き直した。**要件・設計の内容を変えるものだけ**を挙げる。出典はすべて `https://ssp.shillest.net/ukadoc/manual/` 配下である。

### 9.1 `On` 始まりの選択肢 ID は正典が「直接発火する」と明記している（§2.3 の訂正）

§2.3 は「正典は `On` 始まり ID の直接発火について沈黙している」と書いたが、**これは誤りである**。正典には専用の項目がある。

- doc id `ukadoc:list_sakura_script:_5cq_5b_30bf_30a4_30c8_30eb_2cOnID_2cr0_2cr1_2c..._5d:1`
  URL `https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_5cq_5b_30bf_30a4_30c8_30eb_2cOnID_2cr0_2cr1_2c..._5d:1`
  題は `\q[タイトル,OnID,r0,r1,...]`。本文は「ID が "On" で始まっている場合は、選択後、SHIORI イベント OnID が開始される」と書く。引数 `r0,r1,...` は **Reference0 以降**へ入る（`OnChoiceSelectEx` のように Ref0 がラベルになる形ではない）。
- 対比項目も実在する——`\q[タイトル,ID,r2,r3...]` は `OnChoiceSelectEx` が開始される（doc id `ukadoc:list_sakura_script:_5cq_5b_30bf_30a4_30c8_30eb_2cID_2cr2_2cr3..._5d:1`）。`\__q[ID,...]`（下線 2 本）も「ID 仕様は `On` や `script:` の特別扱いまで含めて `\q` と同じ」と書く（doc id `ukadoc:list_sakura_script:_5c__q_5bID_2c..._5d:1`）。なお `\_q`（下線 1 本）はクイックセクションであって選択肢とは無関係である（doc id `ukadoc:list_sakura_script:_5c_q:1`）。
- 差込側の総覧にも「`\q` 等に指定された任意名イベント」という見出しが実在する（doc id `ukadoc:list_plugin_event:OnChoiceSelect_28Ex_29_2fOnAnchorSelect_28Ex_29_2f_5cq_7b49_306b_6307_5b9a_3055_308c_305f_4efb_610f_540d_30a4_30d9_30f3_:1`）。

**影響**: R3.5 と R11.1 の根拠が強くなる。実装（`crates/areka-kanade/src/schedule/choice.rs:57-65`）は正典どおりであり、`crates/areka-kanade/src/schedule/events.rs:28` の送出表も正典どおり「Ref0 以降＝付随参照列のみ」である。**訂正対象は `doc/emo2-conformance-scope.md:24` ただ 1 行**という結論は変わらないが、訂正文の根拠が「実装の裁定」から「正典の逐語」へ格上げされる。実装側注記（`choice.rs:22-24`）が書く「正典は先行段の有無に沈黙」は**先行段（`OnChoiceSelectEx` を先に出すか）についての沈黙**であって、直接発火そのものについての沈黙ではない——この読み分けを設計に書く。

### 9.2 `Status` ヘッダは 9 種ではなく **10 種**（brief `:118` の数が古い）

- doc id `ukadoc:spec_shiori3:Status_20_5bSSP_62e1_5f35_5d:1`
  URL `https://ssp.shillest.net/ukadoc/manual/spec_shiori3.html#Status_20_5bSSP_62e1_5f35_5d:1`
  題は「Status [SSP拡張]」。「複数ある場合はカンマでつなげたもの」。
- 値は `talking` / `choosing` / `minimizing` / `induction` / `passive` / `timecritical` / `nouserbreak` / `online` / `opening(種類)` / `balloon(ID群)` の **10 種**。素の 8 種＋引数付き 2 種である。
- **実装は既に 10 種を第一級で持っている**（`crates/areka-kanade/src/status.rs:19-38` の `ExecutionState` が正典順で 10 variant・同 `:3` の doc も「全10状態」と書く）。M1 で実導出するのは `Talking`（`:168`）と `Choosing`（`:172`）の 2 種で、残 8 種は非アクティブへ縮退する（`:175-182`）。
- **食い違いは brief `:118` の「9種」という数だけ**である。`doc/emo2-conformance-scope.md` には Status の種類数の記載が無い（`:20-26` はイベント一覧であり Status ヘッダの節を持たない）ため、正本文書の訂正対象は増えない。適合検証項目表の側に「正典 10 種・M1 実導出 2 種・残 8 種は非アクティブ」と書けば足りる。

### 9.3 起動系列の順序は正典が規定していない

- 各イベントのページは「起動時に発生」「[NOTIFY]」としか書かず、**送出順を規定する ukadoc 文書は見つからなかった**（`ukadoc:memo_shiorievent` は自ら「きちんとした仕様書ではない」と断っている）。
- `version` は SHIORI リソースであって起動イベントではない（doc id `ukadoc:list_shiori_resource:version:1`）。`basewareversion` は [NOTIFY]（doc id `ukadoc:list_shiori_event:basewareversion:1`・Ref0=バージョン番号／Ref1=本体の識別／Ref2=詳細数値）。
- **影響**: R3.1 の「逐語の順序で固定する」は**正典の写しではなく areka の実装契約の固定**である（`OnInitialize`→`username`→`OnFirstBoot`→`OnBoot`→`basewareversion`）。実装側の逐語は `crates/areka/src/emo2_boot/spine_boot_smoke_tests.rs:50-60` に既に在る。設計はこの出所（正典でなく実装契約）を明記する。
- `OnFirstBoot` は「204 が返された場合、続けて `OnBoot` が発生」（doc id `ukadoc:list_shiori_event:OnFirstBoot:1`）。標準台本が `OnFirstBoot` に `Ok(None)`（204）を返して `OnBoot` へ続くのは正典どおりである（`spine.rs:672-673`）。
- `OnClose` の正典 Reference は Ref0=終了理由・Ref1/Ref2=スコープ番号（doc id `ukadoc:list_shiori_event:OnClose:1`）。**areka は Ref0 のみを送る**（`events.rs:197-206`・同 `:199` が「Ref1/2 は単一スコープの M1 では省略する」と明記）。R11.5（判定に用いる語を実際に出力される語と一致させる）に従い、期待列は実装どおり 1 参照で書く。

### 9.4 マウス系の Reference 配置は実装と正典が一致

- `OnMouseMove`（doc id `ukadoc:list_shiori_event:OnMouseMove:1`）: Ref0=x・Ref1=y・Ref2=ホイール回転量・**Ref3=スコープ（本体 0／相方 1）**・**Ref4=当たり判定の識別子**・Ref5=常に 0・Ref6=デバイス種。
- `OnMouseDoubleClick`（doc id `ukadoc:list_shiori_event:OnMouseDoubleClick:1`）: Ref2=常に 0・Ref5=左 0／右 1、他は同じ。
- 実装は逐語で一致する（`crates/areka-kanade/src/schedule/events.rs:247-267`／`:285-310`）。適合対象の辞書も同じ読み方をする（`crates/pilot/examples/shiori-host-32/fixtures/emo2/ghost/master/dic/touch.pasta:14-15` が `var.r3`＝話者・`var.r4`＝領域）。**R3.3 の「当たり領域と話者が付随参照に載る」は Ref4／Ref3 として確定する。**

### 9.5 更新系とバルーン変更が「任意」である根拠は**発火契機の側からの導出**である

- 実在するのは 4 本ではない: `OnUpdateBegin` / `OnUpdateReady` / `OnUpdateComplete` / `OnUpdateFailure` / `OnUpdateCheckComplete` / `OnUpdateCheckFailure` / `OnUpdateOtherBegin` / `OnUpdateOtherComplete` / `OnUpdateOtherFailure` / `OnUpdateResult` / `OnUpdateResultEx` / `OnUpdateResultExplorer`（いずれも `list_shiori_event.html` の同名アンカー）。
- `OnBalloonChange`（doc id `ukadoc:list_shiori_event:OnBalloonChange:1`）の契機は「他のバルーンから切り替わった際」。さくらスクリプト側にも `\![change,balloon,バルーン名]` の項に「変更後 SHIORI イベント OnBalloonChange が通知される」とある（doc id `ukadoc:list_sakura_script:_5c_21_5bchange_2cballoon_2c_30d0_30eb_30fc_30f3_540d_5d:1`）。
- **「ベースウェア実装として任意である」と明言した文は正典に見つからなかった。** 得られるのは契機の側からの導出のみ——更新系はすべて「ネットワーク更新」を契機とし、`OnBalloonChange` は「バルーンの切替」を契機とするので、その機能を持たないベースウェアでは契機自体が発生しない。**適合検証項目表と正本文書には「正典の明示的な任意宣言ではなく、契機からの導出である」と書く**（R4.6・R11.5）。
- 適合対象の辞書は `OnUpdateBegin` / `OnUpdateReady` / `OnUpdateComplete`（2 箇所）/ `OnUpdateFailure` / `OnBalloonChange` の受け口を持つ（`dic/update.pasta`・`dic/event.pasta`）。**受け口が在るのに送らない**のが M1 の縮退であり、項目 14 が確かめるのはこの縮退で破綻しないことである。

## 10. §6（設計フェーズへ送った 6 件）の解決

| # | 問い | 解決 | 根拠 |
|---|---|---|---|
| 1 | 凍結表を増やす場合の採取可否 | **消滅**（議題 1 裁定＝家 B・台本化した応答。凍結表に触れない） | §7.1 |
| 2 | 自発会話の注入形 | **`KanadeMsg::Tick { now }` を `GhostRuntime::kanade()` へ直接投函する**。dispatcher への Tick とは別チャンネルであり、既存 spine は dispatcher へしか投げていない | `crates/areka-kanade/src/msg.rs:123`・`crates/areka-ghost/src/runtime.rs:219`（`kanade()`）／`:224`（`dispatcher()`）・`crates/areka/src/emo2_boot/spine.rs:857`（現行は dispatcher のみ）・`crates/areka-ghost/src/dispatcher.rs:126`→`on_tick` は `SakuraMsg::Tick` を中継するだけ（同 `:344`）・`spine.rs:666` が「OnSecondChange は kanade へ Tick を送らないため不要」と明記 |
| 3 | 走行時間の実測 | **19 本 9.24 秒**（`cargo test -p areka --bin areka emo2_boot::spine -- --test-threads=1`・2026-09-02）。一周 1 本の追加は 1 秒前後 | §7.1 |
| 4 | ホバー反転の実機観測条件 | **本走行では点けない**。点ける場合は**別走行として分ける**（本番既定と条件が変わるため） | `crates/areka/src/emo2_boot/hover_inject.rs:29`（`AREKA_CHOICE_HOVER_INJECT`）・同 `:184`（`OnceLock` で処理系に 1 度だけ焼き付く＝走行中の切替不可） |
| 5 | 隔離が失う被覆の見積り | 下表 §10.1 | 実測 |
| 6 | 正典の再確認 | §9 | ukadoc |

### 10.1 隔離で失う被覆（R9.4）

| 対象 | 実体 | 失う被覆 |
|---|---|---|
| `crates/wintf/src/ecs/window/zorder_pair_maintain_always_on_top_tests.rs:370-444`（`pair_fix_commands_keep_a_pair_inside_the_band_it_already_shares`・`#[test]` は `:369`） | 実の最上位窓 4 枚を作り、3 つの挿入位置指令を実 `SetWindowPos` で流す。`:411-414` は対照①（印の読み取りが常に真を返していないこと）、`:416-424` は対照②（帯の内側の窓の直前へ挿すと OS が帯へ引き込むこと） | 常設走行が「3 つの挿入位置はいずれも帯の所属を変えない」ことを**実 OS に対して**主張しなくなる。対照 2 本も同時に止まるため、残る単体判定は反証不能になる |
| 同 `:741-794`（`the_top_of_normal_band_fix_keeps_a_real_owned_pair_adjacent_and_out_of_the_band`・`#[test]` は `:740`） | 実の所有リンク（`set_window_owner`）を張った実窓 2 枚に `TopOfNormalBand` を適用し、`:767` で「バルーンがキャラのすぐ手前」を測る。`:775-788` は所有リンク無しの対照（`assert_ne!`） | 実の所有リンクが実の持ち上げ後も隣接を保つことを測る**唯一の**テストが既定で走らなくなる。隣接が「所有リンクのおかげ」か「指令の副作用」かを分ける対照も止まる |
| `crates/wintf/src/runtime/tick_bridge.rs:346-362`（`vblank_notifies_listener_then_joins_on_drop`） | `:353-354` で 500ms の壁時計期限を置き `:355-358` で通知の到達を主張する。`:360-361` の `drop` は停止→join を兼ね、終わらなければハングして落ちる | 実 DWM の垂直同期通知が待機側へ届くことと、`wintf-vsync` スレッドが清く畳まれることの証跡が既定で止まる。隣の `vsync_thread_registers_itself_with_the_vblank_role`（`#[test]` `:325`）は登録・命名しか見ないので代替にならない |

いずれも根治の引受先は `.kiro/specs/areka-P0-zorder-chain-residue/brief.md` の **A-1**（`:15`・`zorder_pair_maintain_always_on_top_tests.rs:767`／`:411` を名指し）と **A-2**（`:16`・`tick_bridge.rs:355` を名指し）である。同 brief `:34` は「e2e が先に踏んだ場合は e2e が隔離裁定を行い根治は本 spec」という分担を明記している。

## 11. 設計判断（決定と、採らなかった案）

### 決定 A: 一周テストは `SpineHarness` の兄弟テストファイルとして 3 本に分ける

- **文脈**: R2.2（1 本の走行）・R2.11（1 ファイルの分量規律）・R12.6（常設テストの実行時間）。
- **採った形**: 走行そのものは `#[test]` 1 本。台本・期待列・駆動ヘルパを主題ごとに別ファイルへ置き、`spine.rs` 末尾の接続宣言（`spine.rs:907-930` と同じ形）で繋ぐ。
- **根拠**: 上限は 1,000 行（`crates/log-capture-kit/tests/workspace_scan/mod.rs:38`）で、超過は例外表（`crates/log-capture-kit/tests/file_length_guard_test.rs:61`・件数の逐語 `:109`）への追記を強いる。例外表は「誰も触らない」と編成側で決まっている（`.kiro/steering/roadmap.md:105`）。判定は `f.lines > LINE_LIMIT` の厳密比較（`workspace_scan/mod.rs:139-142`）ゆえ 1,000 行ちょうどは緑。
- **採らなかった案**: 1 ファイルに全部書く（期待列が長く 1,000 行を超える見込み）。

### 決定 B: 自発会話は kanade へ直接 Tick を投函し、注入時刻には段ごとの頭打ちを置く

- **文脈**: R3.2・R2.1・R2.10。
- **採った形**: `ghost.kanade().send(KanadeMsg::Tick { now })`。台本には `OnSecondChange` の GET／NOTIFY 応答を積む（`spine.rs:141`／`:154` のビルダー）。段ごとに注入時刻の上限を決め、上限に達したら以後は注入せず観測だけを待つ。
- **根拠**: `spine.rs:302-331` の注記が、注入する時刻が観測を追い越すと「待っている条件そのものが破壊されて永久に不成立になる」実測（並行実行で約 2%・期限を 30 秒へ延ばしても 50 回中 3 回失敗）を記録し、頭打ちを置くことを求めている。
- **採らなかった案**: 期限だけを延ばす（同注記が「期限では直らない」と実測で否定している）。

### 決定 C: R2.4 の「表示指令の列（時刻と指令）」は〈段・指令〉の列と、採取時刻が段の宣言区間に入ることの 2 段で実現する

- **文脈**: R2.4・§7.0-③。
- **確かめた事実**: `PresentCommand` は時刻を持たない（`crates/areka-emo-present/src/command.rs:39-73` に時刻フィールドが無い）。時刻を持つのは `dola::cue::TalkCue`（`crates/dola/src/cue/command.rs:316-329`・`at: f64` は `:318`）だが、areka 側の 4 つの受け口（`spine.rs:799-804`）はいずれも cue を記録しない。`TalkClock` は cue の `at` を観測するが単一の最大値しか持たない（`crates/areka/src/emo2_boot/talk_clock.rs:23`・`:41`・`:63`）。
- **採った形**: 記録の単位を〈段名・指令〉とし列の完全一致で判定する。加えて各指令について「採取した時点の注入時刻が当該段の宣言区間に入る」ことを不変条件として判定する。
- **採らなかった案 1**: cue を記録する受け口を 5 本目として足す。`spine.rs:799-804` の受け口列・`SpineHarness` の構造（`:617-639`）・`shutdown_bounded` の分解（`:872-881`）・`spine_talk_close_tests.rs:312-321` の分解を同時に変えることになり、編集集合が既存テストへ広がる。さらに本番の受け口列への追加は併走する `property-query-channels` 側の保存義務の対象であり（`.kiro/steering/roadmap.md:100`）、受け口の本数を固定すると併走側に更新義務を発生させる。
- **採らなかった案 2**: 台本に待ちを入れて指令ごとの最小発火時刻を判定する。決定 B の根拠と同じ壊れ方（注入時刻が観測を追い越す）を新たに作る。

### 決定 D: 進行状態のヘッダは、記録を**追補**して観測する（既存の記録形は変えない）

- **文脈**: R3.8・R12.1・R12.5。
- **確かめた事実**: 受け口は進行状態を受け取っているが捨てている——`ScriptedShioriBackend::get`／`notify` の第 3 引数は `_status: Option<&str>`（`spine.rs:220`／`:241`）で、記録型 `RecordedCall`（`spine.rs:109-118`）は id と参照列しか持たない。`areka-ghost` 側の同型（`crates/areka-ghost/tests/ghost/spine_e2e_test.rs:73-82`）も同じである。進行状態を運ぶログ行は存在しない（`crates/areka-kanade/src/shiori/real.rs:126-155` は wire 値を組み立てて受け口へ渡すだけ）。
- **部分的には既に観測できる**: 会話中は `OnSecondChange` が NOTIFY・Ref3=`"0"` になり（`crates/areka-kanade/src/schedule/events.rs:171-194`）、これは参照列として記録されている。**しかし選択待ち（`choosing`）は Ref3 では会話中と区別できない**——選択待ち中も会話の枠は占有されたままで `talk_active` と `choice_active` が同時に真になる（`crates/areka-kanade/src/status.rs:211-216`・複合値 `talking,choosing`）。
- **採った形**: `ScriptedShioriBackend` に**記録の第 2 系統**（呼出 id と組み立て済み進行状態の対の列）を追加し、`ScriptedShioriHandle` に取り出し口を 1 本足す。既存の `RecordedCall`・`non_status_calls()`（`spine.rs:287`）は**一字も変えない**ため、既存の兄弟テスト 8 本の照合はすべて素通しになる。
- **R12.1 との関係（記録して縮退させる・R12.5）**: 要件は編集集合を「兄弟テストファイル＋`spine.rs` のモジュール宣言 1 行」と書いている。`spine.rs` は編集集合の中の**ファイル**であり、本決定はその中の分量が「宣言 1 行」から「宣言 1 行＋記録の追補 1 か所」へ増えるという**具体化**である。`spine.rs` は丸ごと `#[cfg(test)]`（`crates/areka/src/emo2_boot/mod.rs:33-34`）ゆえ本番コードの改変には当たらない（R8.1・`roadmap.md:88`）。挙動が変わらないことは「追加した経路を読む既存の主張が 1 つも無い」ことで示す。
- **採らなかった案**: 一周テスト専用の受け口を自前で組む（`boot_with` は `ScriptedShioriBackend` を具体型で受ける＝`spine.rs:681` ため、自前の受け口を渡すには起動手順 130 行の複製が要る。R2.6「新しいテスト機構を発明しない」に反する）。

### 決定 E: 終了は**通常の握手**で駆動し、後片付けだけ既存の畳み方を使う

- **文脈**: R2.2（終了握手を一周に含む）・R3.9（終了挨拶 → 終了指示 → 解放の順序・解放はちょうど 1 度）。
- **確かめた事実**: 既存 spine の終了は `GhostRuntime::shutdown` 経由の強制終了であり、`OnClose` は片道の NOTIFY になる（`spine.rs:675`・`spine_talk_close_tests.rs:286-292`）。通常の握手（`OnClose` GET → 終了挨拶 → `\-` → 解放）は `KanadeMsg::CloseRequest { reason }`（`crates/areka-kanade/src/msg.rs:127`）で始まり、`ClosePending` で応答スクリプトを受けて `CloseTalkWait` へ進む（`crates/areka-kanade/src/schedule/close.rs:57-72`）。終了後は `Stopped` へ落ち、kanade のアクターは受信ループを抜ける（`crates/areka-kanade/src/schedule/mod.rs:629`・`crates/areka-kanade/src/actor.rs:106`）。
- **採った形**: 一周の最終段で `ghost.kanade()` へ `CloseRequest{User}` を投函し、台本は `OnClose` の **GET** 応答に終了挨拶＋`\-` を積む。走行後の後片付けは既存の `shutdown_bounded`（`spine.rs:871`）をそのまま使う——同関数は終了指示の結果を捨てる（`:885`）ので、既に止まっている kanade への強制終了が失敗しても畳み方は成立する。
- **判定**: 解放がちょうど 1 度であることは、記録の全体から解放の件数を数えて 1 と照合する。台本の解放応答は 1 度きり消費（`spine.rs:163`・`:263`）ゆえ 2 度目が起きれば受け口が落ちる。
- **採らなかった案**: 強制終了のままにする（終了挨拶が出ないので一周の最終段が成立しない）。

### 決定 F: 子プロセスへの受け渡し（R5.8）は**実 SHIORI が応答したこと**で観測する

- **文脈**: R5.8・§7.0-⑥・R12.1（実走テストの改変は編集集合の外）。
- **確かめた事実**: 受け渡しは引数 3 本（`crates/shiori-host32-host/src/process_host.rs:243-245`）・同じ 3 値の環境変数（`:247-249`・名前は `:36` `HOST32_PARENT_HWND`／`:42` `HOST32_LOAD_DIR`／`:48` `HOST32_SHIORI_NAME`）・作業ディレクトリ（`:251`＝読み込み元）を**同時に**渡す。**成功経路にログ行は 1 つも無い**（`process_host.rs` に `tracing::` の呼出が無い）。失敗は `crates/areka-ghost/src/shiori_wiring.rs:39-86` が文字列で返し、`crates/areka-kanade/src/shiori/real.rs:275-280` の `event=connect_failed` として 1 か所に出る。正規の解放完了は同 `:204-208` の `event=unload_clean`。
- **採った形**: 「1 度通った」の意味を**3 経路の同時成立**と定める（3 つのどれが欠けても読み込みが成立しないため、経路を選り分ける必要がない）。記録は ⑴ `connect_failed` が 0 行 ⑵ 実 SHIORI 由来の起動挨拶が画面に出たこと（目視） ⑶ `unload_clean` が 1 行、の 3 点を残す。
- **採らなかった案**: 実走テストが探す語を 1 つ増やす（`crates/areka/tests/emo2_real_run.rs:208-242`）。当該ファイルは編集集合の外であり、R12.1 の事前登記に無い。

### 決定 G: 実機記録は「先に宣言する」形と「定型の合否ブロック」を合成する

- **文脈**: R5.6・R6.1・R6.7・§7.0-⑧。
- **採った形**: 骨格は `.kiro/specs/completed/areka-P0-collision-dpi-hittest/acceptance-record.md`（証跡の分類を先に置く §0＝`:46-84`・結果を見る前に狙いを宣言する規則＝`:32-38`）。読み分けと合否は `.kiro/specs/completed/areka-P0-dpi-transition-atomicity/signoff-procedure.md` の §6.5（3 問を上から当てる 4 行の表＝`:426-465`）と §6.6（7 行の合否ブロック＝`:467-481`）を**そのまま引く**。走行の同定欄は同手順書 §7（`:487-509`）の項目表を骨格にする。
- **根拠**: R6.1 が「独自の突合規約を作らない」と課している。上記 2 つは完成品であり、合成で要件を満たす。

### 決定 H: 完成判定の「テスト全通過」は**移動後の再実行**を正とする

- **文脈**: R10.1・R10.3・§2.7-4／-5。
- **確かめた事実**: 完了手続きは着手時に 1 度（`.claude/skills/kiro-complete/SKILL.md:119-123`・直近実行があれば省略可）、アーカイブ移動の後にもう 1 度（同 `:290-301`・**省略不可**・`:299` が明記）テストを回す。許諾の検査は設定ファイルがある場合のみ走る（同 `:125-138`）。**本リポジトリには `deny.toml`・`about.toml`・`about.hbs` がいずれも実在する**（リポジトリ直下で実測・2026-09-02）ため、R10.2 の「設定不在により省略」分岐は**発生しない**＝許諾の検査は実際に走る。
- **採った形**: 完成判定の ⑴ は**移動後の再実行が成功で終わったこと**を正の証跡とする（着手時の実行は前置き）。32bit の橋渡し実行体の用意は ⑴ の前提として手順に明記する（`crates/areka/tests/emo2_real_run.rs:34-42` が唯一の記載箇所であり、完了手続きの検査項目には無い）。

# Implementation Plan

- [x] 1. バルーン対話サブモジュールの雛形作成と runtime アクセサ追加
  - `crates/areka/src/input_events` 配下にバルーン対話専用のサブモジュールを新設し、モジュール宣言を追加する
  - `Emo2Wiring` に既存 `presenter()` アクセサと同型の runtime 読み口を additive に追加する
  - Observable: 新設サブモジュールと新アクセサを含めてクレートがビルド成功する
  - _Requirements: 4.1_
  - _Boundary: input_events::balloon (skeleton), Emo2Wiring_

- [x] 2. Core: 契約型と状態配線資源
- [x] 2.1 ChoiceSelection 契約型の定義
  - id／label／scope／references の 4 フィールドを持つ選択確定ワイヤ形を定義する（ordinal は含めない）
  - Clone／Debug／PartialEq／Eq を導出し、下流が型を import して消費できる形にする
  - Observable: 単体テストで 2 つの ChoiceSelection 値を構築し、等価性比較が意図通り成立する
  - _Requirements: 2.2, 2.6, 8.5_
  - _Boundary: ChoiceSelection_

- [x] 2.2 BalloonWiring 資源と ChoiceSelectionInbox シームの定義
  - scope→最後に注入した hover ordinal の自前追跡マップと、選択確定の発行シンク（mpsc Sender）を持つ NonSend 資源を定義する
  - Receiver を保持する M1 暫定受け口（下流 choice-select-events が受信処理へ置換するシーム）を定義し、`CuePlayer::resolve_choice` を直接呼び出さない境界を保つ
  - Observable: 単体テストで資源を World へ NonSend 挿入でき、シームの Receiver 経由で送信値を観測できる
  - _Requirements: 2.1, 2.4, 3.4, 5.3, 5.4, 8.2, 8.3_
  - _Boundary: BalloonWiring, ChoiceSelectionInbox_
  - _Depends: 2.1_

- [x] 3. Core: 純関数判定核
- [x] 3.1 点包含 hit 判定と重なり規則の実装
  - 行矩形群への点包含判定を純関数として実装する（半開区間・物理 px 直接比較・DPI 変換なし）。判定対象は上流が供給する選択肢行ジオメトリのみとし、選択肢以外のバルーン内リンクは対象にしない
  - 病的重なり入力に対し決定的な単一選択規則（逆順走査・最終一致）を適用する
  - Observable: 単体テストで内側／境界／外側／空配列／重なりの各ケースが実窓・GPU 不要で決定的に判定される
  - _Requirements: 1.1, 1.5, 2.3, 4.2, 5.2, 8.6_
  - _Boundary: hit_choice_row_

- [x] 3.2 hover 遷移判定関数の実装
  - 表示中フラグ・hit 結果・前回注入値から hover 遷移（無変化／自前状態のみ整合／注入）を決定する純関数を実装する
  - Observable: 単体テストで全分岐（非表示時無処理／消滅時自前整合／同値維持／新規注入／解除注入）を網羅する
  - _Requirements: 1.2, 1.3, 1.4, 3.4_
  - _Boundary: hover_action_

- [x] 3.3 クリック確定判定関数の実装（stale／原子性）
  - クリック時点の現行行ジオメトリのみから ChoiceSelection を構成する純関数を実装し、非表示中・非 hit・キャッシュのみのケースを None とする
  - Observable: 単体テストでヒット時の ChoiceSelection フィールド一致、非 hit／非表示／stale 行での非構成（None）を確認する
  - _Requirements: 2.1, 2.2, 2.3, 3.1, 3.2, 6.2, 6.3_
  - _Boundary: click_selection_
  - _Depends: 2.1, 3.1_

- [x] 4. Core: バルーンポインタハンドラ
- [x] 4.1 移動ハンドラの実装（hover 追従駆動）
  - Bubble 相のみ処理する移動ハンドラを実装し、固定順の借用規律（共有借用→スナップショット→借用解放→可変借用で inject）に従う
  - `Emo2Wiring` 不在（boot 前／失敗）は正常縮退として `debug!` ＋ no-op（donor `presenter=None` 同型）、`BalloonWiring` 不在または `RefCell` 借用失敗は構成異常として `error!`（event = balloon_wiring_missing／balloon_runtime_borrow_failed）でログし no-op 縮退する（ログ無し失敗経路を作らない）
  - hover 遷移注入時は `debug!`（event = choice_hover_inject）を発行する
  - Observable: 合成 PointerState の直接呼び出しテストで hover 追従と各縮退経路（`debug!`／`error!` それぞれ正しい方）の双方が観測できる
  - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.6, 3.1, 3.3, 4.1, 4.2, 8.4_
  - _Boundary: on_balloon_pointer_moved_
  - _Depends: 2.2, 3.2_

- [x] 4.2 押下ハンドラの実装（確定クリック→発行）
  - 左シングルクリックのみを確定として扱い（`double_click` フィールドは不参照）、ヒット時に一度だけ ChoiceSelection を送信するハンドラを実装する
  - 送信成功時は `info!`（event = choice_selected, scope, id, label, references_len）を 1 回発行する（実機サインオフの grep 対象）
  - 非表示中／非 hit での棄却は `debug!`（event = choice_click_rejected, reason）。`Emo2Wiring` 不在は `debug!` ＋ no-op（正常縮退）、`BalloonWiring` 不在／`RefCell` 借用失敗／`selection_tx.send` 失敗は構成異常として `error!`（event = balloon_wiring_missing／balloon_runtime_borrow_failed／choice_selection_send_failed）でログし no-op 縮退する
  - Observable: 合成 PointerState の直接呼び出しテストでヒット＆表示中クリックにつき一度だけの送信と対応する `choice_selected` ログ発火、非ヒット／非表示／Tunnel 相では送信ゼロと対応する棄却／縮退ログを観測する
  - _Requirements: 2.1, 2.3, 2.4, 2.5, 2.6, 3.1, 3.2, 4.2, 5.1, 8.4_
  - _Boundary: on_balloon_pointer_pressed_
  - _Depends: 2.1, 2.2, 3.3_

- [x] 5. clear_balloon_hover_on_leave の実装
  - 窓外離脱マーカー（`PointerLeave`）を読み、親チェーンがバルーン窓を所有する entity のみを対象に、既存 `hover_action(active, None, last)` を再利用して hover 状態を解除する排他システムを実装する
  - Observable: bare World テストでバルーン所有 leave のみ hover 解除され、非バルーン窓の leave は無視されることを確認する
  - _Requirements: 1.3, 3.4_
  - _Boundary: clear_balloon_hover_on_leave_
  - _Depends: 2.2, 3.2_

- [x] 6. Integration: 配線結合
- [x] 6.1 post-spawn ハンドラ装着・資源結線・スケジュール登録
  - 全バルーン窓へポインタハンドラ（moved／pressed）を post-spawn 装着し、NonSend 資源とチャネルを結線し、leave 追随システムを Input スケジュール（dispatch 後）へ登録する
  - Observable: bare World 統合テストで、全バルーン窓にハンドラが存在しキャラ窓のハンドラ集合は不変であること、かつスケジュールへの leave システム登録存在を assert で確認する
  - _Requirements: 4.3, 4.4, 5.5, 6.6_
  - _Boundary: attach_balloon_pointer_handlers, wire_balloon_choice_
  - _Depends: 4.1, 4.2, 5_

- [x] 6.2 main.rs 起動結線
  - 起動シーケンス（既存 `attach_char_pointer_handlers` 呼出の隣）へバルーン配線の結線呼び出しを追加する
  - Observable: クレートがビルド成功し、起動シーケンスが新規結線呼び出しを実行する
  - _Requirements: 4.3, 5.5, 8.1_
  - _Depends: 6.1_

- [x] 7. Validation: 決定論檻・退行防止・実機サインオフ
- [x] 7.1 貫通ポインタ列の決定論檻
  - 選択肢行 population がテストから決定論的に可能か確認したうえで、可能なら合成ポインタ列（移動→クリック）から `ChoiceSelectionInbox` 観測までの一気通貫テストを追加する。GPU 依存で不可能な場合のみ、純関数全網羅＋mpsc 観測の分解形で本要件を満たすことを明記する
  - Observable: 注入座標列に対する hover 追従と一度きりの発行、stale／非 hit での非発行が実窓・sleep 不要で観測される
  - _Requirements: 6.1, 6.2, 6.3, 6.4, 6.5_
  - _Depends: 6.1_

- [x] 7.2 資源縮退・Tunnel 素通し回帰檻
  - `Emo2Wiring`／`BalloonWiring` 不在時の no-op 縮退と、`Phase::Tunnel` での両ハンドラの素通し（副作用ゼロ・`false` 返却）をテストする
  - Observable: 縮退時の対応するログ（`debug!`／`error!`）と副作用ゼロ、Tunnel 相での副作用ゼロが確認される
  - _Requirements: 8.1, 8.4_
  - _Depends: 4.1, 4.2_

- [x] 7.3 実機サインオフ手順の実施
  - 実 emo2・実 pasta.dll・絶対パス・実 DPI（≠96）で起動し、本番ゴースト表示を先行させたうえでポインタ追従を目視し、クリック確定を有界 `AREKA_APP_SMOKE_EXIT_MS` auto-exit 後の `RUST_LOG` ログ grep（`event="choice_selected"`）で確認する
  - カスケード発火・遷移（下流 choice-select-events の領分）は本判定に混ぜない
  - Observable: 実機でポインタ追従ハイライトが目視でき、`choice_selected` ログの grep 一致でクリック確定到達が確認される
  - _Requirements: 7.1, 7.2, 7.3, 7.4, 7.5, 7.6_
  - _Depends: 6.2_

- [x] 7.4 ワークスペース全体回帰確認
  - i686 host-32 成果物を事前ビルドしたうえで `cargo test --workspace` を実行し、新規テストを含め exit 0 で成功することを確認する。新規外部依存が追加されていないこと、上流契約・cue ワイヤ形・キャラ窓 DPI 素通し規約が変更されていないことを確認する
  - Observable: 全既存テスト＋新規テストが緑で exit 0
  - _Requirements: 8.1, 8.2, 8.3, 8.5, 8.6_
  - _Depends: 7.1, 7.2, 7.3_

## Implementation Notes

- 環境: worktree の `vendors/pasta` submodule が未populate だと areka ビルドが失敗する。`git submodule update --init --recursive vendors/pasta` で解消（既知の worktree quirk・source 変更ではない）。
- 検証コマンド: 各タスクは `cargo build -p areka` ＋ `cargo test -p areka`（bin の in-source `#[cfg(test)]` を既定で走らせる）。バルーン限定は `cargo test -p areka --bin areka input_events::balloon`。最終回帰(7.4)のみ `cargo test --workspace`（i686 host-32 成果物の事前ビルド前提）。
- 上流型: `ChoiceHitRow`（`areka_emo_text::actor`・実体 `crates/areka-emo-text/src/actor.rs:150`）・`HitRectPx`（`crates/areka-emo-text/src/choice.rs:155`・`left/top/right/bottom` は `pub` f32）。areka から実型で fixture 構築可能——ローカル並行型を作らないこと（4.1/8.5）。
- **レビュアー厳守**: RED 再現やスタブ確認で `git checkout`/`git reset --hard` を絶対に使わない（未コミット実装を破棄する既知ハザード）。差替え検証はファイルバックアップ（cp）→復元で行い、復元後に diff 存在とテスト緑を必ず再確認する。
- `#[allow(dead_code)]` は各シンボルへ narrow 付与（本番未消費のうちだけ・後続タスクの本番結線で撤去見込み）。crate-wide 抑止は禁止。
- ハンドラ檻の GPU 制約: `TextLayerRuntime::choice_hit_rows` は `present_frame`(GPU) でしか `choice_snapshot` を埋めないため headless では現行 rows が常に空＝hit=None。よって「Some(ordinal) 追従」「hit→send→`choice_selected` info」の full pass-through は 4.1/4.2 の単体檻では実演不可——純関数檻(3.1/3.2/3.3)＋send機構檻(2.2)＋task 7.1/7.3 へ委譲（設計 Testing Strategy item 6 の裁定）。実 runtime は `apply_cue(TalkCue{Choice})` で活性化（fake 禁止）。
- ログ event 名（実装確定）: 移動 Emo2 不在=`choice_moved_no_emo2`(debug)／押下 Emo2 不在=`choice_pressed_no_emo2`(debug)／`balloon_wiring_missing`(error)／`balloon_runtime_borrow_failed`(error)／`balloon_marker_missing`(error・設計 Error table 520行の marker 不在＝error!+false)／`choice_hover_inject`(debug)／`choice_click_rejected`(debug, reason=inactive|no_hit)／`choice_selected`(info・R7.2 grep 対象)／`choice_selection_send_failed`(error)。
- 既知の軽微 concern（follow-up 候補・非ブロッキング）: send 失敗経路で `BalloonWiring::send_selection`(2.2) の `warn!(choice_selection_send_failed, scope)` と押下ハンドラ(4.2) の `error!(choice_selection_send_failed, scope, id)` が同一 event を 2 重発火。稀な GPU-gated 失敗経路のみ・R7.2 grep 対象(info)は無影響。整理案＝`send_selection` を Result 返却のみにしてハンドラが単一 error 行を所有。

### task 7.4 workspace 回帰の実測結果（2026-07-24・自律実行）
- **choice-interact 自体は完全に緑・非退行**: `cargo test -p areka`=405+ passed / 全 workspace 実行で areka クレート=368 passed（新規 balloon 檻すべて緑）。**新規外部依存ゼロ**（Cargo.toml/Cargo.lock 変更なし=8.2）・**上流契約不変**（areka-emo-text/wintf/spawn.rs 変更なし・新 cue variant なし=8.5）・**DPI 素通し不変**（input_events/mod.rs は `mod balloon;` の1行のみ・DD-IE-10 本文不変=8.6）・**tokio 不使用**（std::sync::mpsc のみ=8.3）。i686 host-32 成果物（shiori-host32-helper/testdll）は事前ビルド済（exit 0）。
- **`cargo test --workspace` exit 0 は pre-existing で無関係な areka-kanade フレークにより未達**（choice-interact の欠陥ではない）: 失敗は毎回 `steady_test::talk_completion_resumes_get_pump_ref3_one_status_none`（時に `close_test::boot_greeting_talkdone_resumes_get_pump` も・失敗数1〜2で変動）＝`common/mod.rs:~1019 drive_ticks_until_disconnect` の協調ループが並列 test CPU 競合で 5s deadline を逃す。**証拠**: (a) `cargo test -p areka-kanade` **単独**（choice-interact 不関与）でも並列なら再現、(b) 単一 test 隔離=緑(4.37s)、(c) `--test-threads=1` 直列=緑(36 passed,3.36s)＝実ハングでなく競合飢餓、(d) kanade＋依存閉包の diff は main と**空**（Cargo.lock 不変）=main とバイト同一で 100% pre-existing。詳細は memory [[areka-defender-rescan-starves-cooperative-test-loops]]。他 spec 所有ゆえ choice-interact からはパッチしない（upstream ownership・[[portfolio-convergence-decided-in-separate-session]]）。**判定**: 開発者の合流/DoD ゲート（settled 環境）で `cargo test --workspace` を再走させ kanade フレークの解消を確認するか、kanade 側の deadline 頑健化を upstream で行う。注: `cargo test --workspace -- --test-threads=1` は逆に wintf GPU テストを ACCESS_VIOLATION させる別 pre-existing（完了 spec wintf-gpu-test-crash 領域）ゆえ直列回避は不可。
- **【解決済み・2026-07-24・開発者裁定「kanade フレークを先に上流で修正」】**: `drive_ticks_until_disconnect` の `yield_now()` busy-spin を **200μs backoff sleep** へ置換し、producer が kanade worker を CPU 飢餓させる真因を除去（別コミット `0717c8a`・areka-kanade テストハーネス）。**検証: areka-kanade 5x 並列=0 failures／`cargo test --workspace`=exit 0・NO FAILURES（85 test-result・choice-interact 新規檻含む全緑）**。→ **task 7.4 の DoD ゲート `cargo test --workspace` exit 0 を真に達成**。kanade 修正は areka-kanade 所有ゆえ「1 feature=1 PR」の観点では別関心事＝PR 分割は開発者判断（現状は本ブランチに分離コミット `0717c8a` で同居）。memory [[areka-defender-rescan-starves-cooperative-test-loops]] に根治を追記済み。

### task 7.3 実機サインオフ（人間実行必須・AI 単独では宣言しない・R7.3/emo2_real_run.rs:77 準拠）
本 spec の `choice_selected` info ログ導線（task 4.2 で実装済）＋有界 auto-exit＋grep で判定する。ポインタ追従ハイライトの**目視**と実クリック到達は人間が実機で行う（自律エージェントは目視/実クリックを捏造しない）。手順:
1. i686 host-32 helper を実バイナリ隣へ配置: `cargo build -p shiori-host32-helper --target i686-pc-windows-msvc` → `target/i686-pc-windows-msvc/debug/shiori-host32-helper.exe` を `target/debug/shiori-host32-helper.exe`（実行プロファイル隣）へコピー。
2. 画面 DPI を 96 以外（125%/150%）に設定（実 DPI≠96・R7.1）。
3. 絶対パスで起動（相対だと pasta.dll LOAD が 0x8007007E）:
   `$env:RUST_LOG="info"; $env:AREKA_APP_SMOKE_EXIT_MS="180000"; target\debug\areka.exe <絶対 emo2 ghost_root> <絶対 emo2 balloon_root>`（ghost_root=`crates\pilot\examples\shiori-host-32\fixtures\emo2`、balloon_root=同下 `emo2-kakukaku`）。
4. 本番ゴースト表示先行後、`\q` メニューを表示させ、(a) ポインタを選択肢行上で動かし**ハイライトが実ポインタに追従**することを目視（R7.1）、(b) 行を実クリック（R7.2）。
5. 有界 auto-exit 後、捕捉ログを `event="choice_selected"` で grep 一致＝クリック確定到達を確認（R7.2/R7.6）。カスケード・遷移（下流 choice-select-events）は判定に混ぜない（R7.4）。

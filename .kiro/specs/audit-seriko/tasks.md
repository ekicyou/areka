# 実装計画: audit-seriko

> 直しの正本は design.md の「指摘ごとの直し」の表（要件 8.1 の 18 件）。「指摘 N」はその表の番号。
> 並走の印 `(P)` は付けない。設計の「System Flows」のとおり直列で進める（同じテストファイル・`bind.rs`・`looper.rs`・`state.rs`・`settle.rs` に複数のタスクが触り、後のタスクは指摘 6 でまとめた道具を使うため）。
> 消す・形を変える口は、どのタスクでも消す前にツリー全体を `rg -n '\b名前\b'` で探し、件数と探した日の `HEAD` をタスクの記録に残す（design.md「消す前の確かめ」）。
> 「赤になることを確かめる」は、確かめる行を一時的に外して赤・戻して緑を 1 回ずつ見て記録に残すこと（design.md Testing Strategy）。
> design.md の行番号は設計の時点のもの。前のタスクで行がずれるので、直す相手はテストや関数の名で探し直す。

- [ ] 1. テストの道具を受け口へまとめる（指摘 6）
- [x] 1.1 層をまたぐテストの道具の受け口を作り、`src/` の写しを寄せる
  - 最初に、着手の `HEAD` でクレートのテストの数と、後のタスクで移す・消すテストの名の一覧を記録に残す（1〜5 のタスクの「同じ数・同じ期待値」の比べる元）
  - `lib_test_support.rs` を新しく置き、`lib.rs` からテストのときだけ繋ぐ。表を読んで組む道具・既定のスコープ・記録の捕まえ（行の列とつないだ文字列）・表と記録の組・警告の抜き出し・`check_calls` の 6 種・部品の再生中・1 コマの状態を置く
  - research.md 3 節の 6 に挙げた `src/` の写し（`table.rs` の中の写し、`looper_test_support.rs`・`state_test_support.rs` を含む）を消して受け口を使う。道具の名が本番の項目と同じなら明示の `use` で受ける
  - 写しの中身が違うものは振る舞いを変えて寄せず、違いを引数で表すか、寄せずに止めて報告する
  - 完了: クレートのテストが記録した数・同じ期待値で緑。`src/` に写しの道具が残っていないことを `rg` で示した記録がある
  - _Boundary: areka-seriko の src のテストと受け口_
  - _Requirements: 2.3, 6.1, 8.1_
- [x] 1.2 役者の時計つきの組みと吹き出しの合図の写しを `actor_test_support.rs` へ寄せる
  - 4 本の Rig を、統括器の設定・名前の表・静的な着せ替えを構築の引数に受ける 1 つの組みにまとめ、3 本の吹き出しの合図を 1 つの道具にする
  - 時計を持たない `actor_stage_tests.rs` の Rig は、時計に揃えて期待値が変わるなら寄せずに止めて報告する
  - 完了: 役者のテストが記録した期待値で緑。Rig と合図の定義が受け口の 1 か所だけにある
  - _Boundary: actor_test_support.rs と actor_*_tests.rs_
  - _Requirements: 2.3, 8.1_
- [ ] 1.3 統合テストの共有の道具を `tests/common/mod.rs` にまとめる
  - 合図の組み・emo2 の面の文字列と名前の解決・話の知らせ・何もしない文字の受け手を、写しの 2 組（`cue_sequence`＝`regression`、`balloon_face_e2e`＝`bind_e2e`）から 1 か所へ移し、各テストが `mod common;` で使う
  - 使わない道具が出るテストのために受け口の先頭で未使用の警告を許す
  - 完了: 4 本の統合テストが同じ期待値で緑。写しの定義が `tests/` の各ファイルから消えている
  - _Boundary: areka-seriko の tests/_
  - _Requirements: 2.3, 8.1_
- [ ] 1.4 範囲の外の「出所」の説明 3 行を直す（統合・指摘 6 の結果）
  - 触る前に持ち主 `audit-test-kits` が走っていないかを `.kiro/specs/` と作業場の枝で確かめて記録する。走っていれば要件 7.3 に従って申し合わせてから触る
  - `log-capture-kit` の「出所は `table.rs` の `capture_logs`」の文 3 行（`event.rs`・`event_tests.rs`）の出所の書き方だけを、1.1 で移した先に合わせて直す。ほかの行は触らない
  - 完了: 3 行が移した先を指し、`git diff` の `log-capture-kit` の変更がその 3 行だけ
  - _Boundary: log-capture-kit（範囲の外・決めた行だけ）_
  - _Requirements: 7.3, 7.4, 7.6_

- [ ] 2. 名前の表を本番と同じ道で組む（指摘 3）
- [ ] 2.1 クレートの中のテストの名前の表を descript の文字列から組む形へ移す
  - 名前の表が要るテスト（`bind_tests`・`bind_named_tests`・`actor_bind_loop_tests`・`actor_clock_tests`・`actor_parts_tests`・`actor_replace_tests`・`tests/bind_e2e`）を、descript の文字列 → 着せ替えの読み → 本番の表の組み方で組む形にする。統合テスト用の着せ替えの読みは 1.3 の受け口に置く
  - 期待値が変わるテストが出たら、テスト用の口が本番と違っていた証拠として本番の答えへ直し、その件を記録に残す
  - 完了: 移したテストが全部緑で、クレートの中にテスト用の口の呼び出しが残っていない
  - _Boundary: areka-seriko の名前の表を使うテスト_
  - _Requirements: 2.3, 4.1, 7.2, 8.1_
- [ ] 2.2 テスト用の口と宣言の型を消し、範囲の外の呼び出し元を直す（統合）
  - 触る前に、`shell_carry_tests.rs` の持ち主 `audit-emo2-boot-spine` が走っていないか、同じ `bind.rs` に触る `audit-parsers-sakura` が着地したか走っているかを確かめて記録する。着地していれば main を取り込んでから消す
  - `shell_carry_tests.rs` の名前の表を descript の文字列で組む形にし、今ある `char2_names` の中身を 1 つの道具にして両方から呼ぶ（作業フォルダはワークツリーの `target\` の下の `WorkDir` のまま）
  - `BindResolver::new` と `BindOptionDecls` を消し、再輸出と `lib.rs` の doc の列、`bind.rs` の「`new` で組んだ表は…」の説明を消す。消す前のツリー全体の確かめを記録する
  - 完了: `cargo build --workspace --all-targets` が通り、`rg` で 2 つの名がツリーに 0 件。`shell_carry_tests` が緑
  - _Boundary: bind.rs・lib.rs と areka の shell_carry_tests.rs（範囲の外・呼び出し元だけ）_
  - _Requirements: 2.3, 4.4, 7.1, 7.2, 7.3, 7.4, 7.6, 8.1_

- [ ] 3. 1,000 行を越える役者のテストを分ける（指摘 2・統合）
  - 刻みの道のテストの組を新しい兄弟のファイルへ移し、`actor.rs` に接続の宣言を足す。写しの道具は消し、一番上のテストの同じ中身の道具を、1.1 の後に置かれた 1 か所の定義から使う（要るものだけ `pub(crate)` に上げる）。写しでない 1 回鳴らす道具はそのまま移す
  - 着せ替え付きの状態は、着せ替えが空の道具ではなく `shown_shell_with_binds(sid, BindSet::from_ids([1100, 1207]))` で組む（移すテストは `Show` の欄を `..` で読み飛ばすので、取り違えても緑のまま通ってしまう）
  - 移した後も 1,000 行を越えるなら、刻みの道の前の最後の組も移す
  - 1,000 行の見張りの例外表から元のファイルの行を消し、件数を main を取り込んだ後の表の行の数に合わせる（9 → 8 を決め打ちしない）。触る前に持ち主 `audit-test-kits` が走っていないかを確かめて記録する
  - 完了: 2 つのファイルとも 1,000 行未満で、移したテストが 1.1 で記録した名のまま同じ期待値で緑。`file_length_guard_test` が行を消した表で緑
  - _Boundary: actor_bind_loop_tests.rs・actor_tick_tests.rs（新）・actor.rs と log-capture-kit の例外表（範囲の外・1 行と件数だけ）_
  - _Depends: 2.1_
  - _Requirements: 6.1, 7.4, 7.6, 8.1_

- [ ] 4. 本番のコードの直し
- [ ] 4.1 一番上と部品で写した引き金の扱いを 1 か所にまとめる（指摘 4・16）
  - 窓の開け閉めを写す処理・鳴らした記録・「3 語の引き金か」の判定を `trigger.rs` の 1 つずつの口にまとめ、一番上と部品の両方から呼ぶ。記録の行は段・欄の並び・文言を今と 1 字も変えない
  - 構える時刻の引数を符号なしの整数にして変換を消し、呼び手 6 か所を直す。消す口（`is_trigger_word`・`is_trigger`）と形を変える `Armed::arm` の確かめを記録する
  - 完了: 引き金のテスト（一番上・部品・話・引き金そのもの）が記録の文字ごと同じで緑
  - _Boundary: trigger.rs・looper_trigger.rs・parts_trigger.rs・looper.rs・parts.rs_
  - _Requirements: 2.3, 7.1, 7.2, 8.1_
- [ ] 4.2 「印を戻さない」決まりと `always` の判定を `run_calls` の 1 か所へ移す（指摘 5）
  - 2 つの貸し口は渡された値を置くだけにし、止めた相手が `always` かは `run_calls` が表から引いて渡す。貸し口の不要になった欄を消す。形を変える `CallPlays::stop` の確かめを記録する
  - 新しいテスト: 当て終えた印が 500 の再生を 300 で回しても印は 500 のまま、続けて 600 で回しても 400 の `move` が 2 度当たらない
  - 完了: 新しいテストが緑で、`max` を外すと赤になることを確かめた記録がある。既存の呼び出しのテストが緑
  - _Boundary: calls.rs・looper_calls.rs・parts_calls.rs・calls_tests.rs_
  - _Requirements: 4.1, 4.2, 7.1, 7.2, 8.1_
- [ ] 4.3 文字の時刻の問い合わせで開発のビルドだけ止まる道を消す（指摘 9）
  - `debug_assert!` を消し、列に無い序数では刻みの時刻を返して `warn!` を残す今の道をどのビルドでも通す
  - 新しいテスト: 列に無い序数で呼ぶと刻みの時刻を返し、序数と時刻の欄を持つ `warn!` が 1 行出る
  - 完了: 新しいテストが緑で、`debug_assert!` を戻すと `panic` で赤になることを確かめた記録がある
  - _Boundary: looper_trigger.rs・looper_trigger_tests.rs_
  - _Requirements: 4.1, 4.2, 6.2, 8.1_
- [ ] 4.4 スコープの状態を順のある表にし、並べ替えを消す（指摘 11）
  - スコープとバルーンの状態を順のある表にし、出ている面の列を「スコープの昇順 → シェル → バルーン」で組む。一番上の並べ替えと順位の関数 `slot_rank`、状態の中の並べ替えを消し、誤った説明 3 か所を直す。`slot_rank` を消す前の確かめを記録する
  - 新しいテスト（`state_stage_tests.rs`）: スコープ 0 と 1 の両方にシェルとバルーンを出し、列が「0 シェル → 0 バルーン → 1 シェル → 1 バルーン」になる
  - 完了: 乱数を焼き込んだ抽選の順のテストが期待値を変えずに緑。新しいテストが緑で、スコープを回し終えてからバルーンを回す形に一時的に変えると赤になることを確かめた記録がある
  - _Boundary: state.rs・looper.rs・state_stage_tests.rs_
  - _Requirements: 4.1, 4.2, 7.1, 8.1_
- [ ] 4.5 公開の口を 7 つ狭め、届かない枝を消す（指摘 15・17）
  - 7 つの口を再輸出から外して `pub(crate)` にし、`lib.rs` の doc の列から外す。消す前のツリー全体の確かめを記録する
  - 部品のスコープの名前の入れ方を、無いときだけ写して入れる 1 つの分岐にし、届かない枝を消す（毎刻みの写しを避ける理由の説明は保つ）
  - 完了: `cargo build --workspace --all-targets` が通り、部品のテストが緑
  - _Boundary: lib.rs・bind.rs・settle.rs・timeline.rs・state.rs・parts.rs_
  - _Requirements: 2.3, 7.1, 7.2, 8.1_
- [ ] 4.6 書き出しの面の待ちに 10 分の上限を置く（指摘 19）
  - 進める刻みの数に上限（16ms × 37,500）を置き、越えたら面の番号・上限・作者の書いた待ちの合計を欄に持つ `warn!` を 1 行残して、上限の時刻まで進めた面を返す。上限の内は今と同じ 16ms ごとの道
  - 新しいテスト: 待ちが上限ちょうどの面は `warn!` 0 行で今と同じ面。上限の 1ms 後にコマが替わる面は `warn!` 1 行で替わる前のコマ
  - 完了: 2 本のテストが緑で、上限を外すと 2 本目が赤になることを確かめた記録がある
  - _Boundary: settle.rs・settle_tests.rs_
  - _Requirements: 4.1, 4.2, 6.2, 8.1, 8.4, 8.5_

- [ ] 5. テストと説明の直し
- [ ] 5.1 見るべきものを見ていないテストを直す（指摘 1・7）
  - `non_monotonic_tick_is_ignored` を、コマが替わる間に時刻が戻る形に差し替え、戻る刻みで指令 0 件・最後に見た時刻が 2150 であることを確かめる
  - `prefix_alike_name_is_not_bind` に記録の 1 行、`table.rs` の中の表のテストに採った本数 3、`ticks_add_no_warn` に `Show` が 1 つ以上、`word_warns` の 2 つ目の枝の較正を足す（相手は名で探す）
  - 完了: 各テストが緑で、確かめる相手（番人・`debug!`・余分な animation・刻みの道・欄の形）を外すと赤になることを 1 つずつ確かめた記録がある
  - _Boundary: looper_tests.rs・actor_bind_noevent_tests.rs・table.rs・table_calls_check_tests.rs・table_calls_dot_sakura_tests.rs_
  - _Requirements: 4.1, 4.2, 8.1_
- [ ] 5.2 試されていない分岐にテストを足す（指摘 8）
  - 本番は変えず 6 本足す: 間隔 0 の警告と、動く絵の子の待ち 0 の警告（どちらも `table.rs` の中のテスト）、名前の引きの未宣言と重なり（`bind_tests.rs`）、スコープ 2 の名前の着せ替えの排他と `mustselect`（`bind_named_tests.rs`）、`always` の前の周の終わりのコマを 1 度だけ当てる（`calls_tests.rs`）、役者を止めた後の差し替えが偽（`actor_replace_tests.rs`）
  - `table.rs` は足した後も 1,000 行未満であることを確かめる（越えるなら兄弟のテストファイルへ置く）
  - 完了: 6 本が緑で、試す分岐を外すと赤になることを 1 本ずつ確かめた記録がある
  - _Boundary: table.rs・bind_tests.rs・bind_named_tests.rs・calls_tests.rs・actor_replace_tests.rs_
  - _Requirements: 4.1, 4.2, 6.1, 6.3, 8.1_
- [ ] 5.3 使われない見本の根を消し、古い説明を直す（指摘 10・12）
  - `R_POST_AND_KOMAINU`・`r_post_and_komainu_shell_root`・その自己テストを、消す前のツリー全体の確かめを記録してから消す
  - 古い説明（`actor.rs` の 2 か所・`resolve.rs`・`state_surface_tests.rs`・`looper_parts_emo2_tests.rs` の手書きの数と一覧）を、行番号でなく中身で探して直すか消す（emo2 のテストは理由の 2 行だけ残す）
  - 完了: クレートのテストが緑で、`rg` で消した名がツリーに 0 件
  - _Boundary: sample_test_support.rs・sample_test_support_tests.rs と説明を直す 4 ファイル_
  - _Requirements: 7.1, 8.1_
- [ ] 5.4 自分と比べるだけのテストと重なったテストを消す（指摘 13・14）
  - 自分と比べるだけのテスト（`resolve.rs`・`output.rs`・`timeline_repeat_tests.rs`・`settle_tests.rs`・`timeline.rs`）と重なったテスト（`bind_tests.rs`・`state_surface_tests.rs`）を、design.md の行番号でなくテストの名と中身で探し直して消す。残るテストが同じことを見ていることを記録する
  - 完了: クレートのテストが緑で、1.1 で記録した数から消した分だけ減ったことを記録に残している
  - _Boundary: 左の 7 ファイルのテストだけ_
  - _Requirements: 2.3, 8.1_

- [ ] 6. 最後の確かめ
- [ ] 6.1 同じ作法で audit をかけ直し、新しい指摘を 0 件にする
  - main を取り込み、`audit-parsers-sakura` などと行がぶつかれば揃え、例外表の件数を取り込んだ後の行の数に合わせ直す。並走の spec が消した口を使い始めていないか探し直す
  - `crates/areka-seriko/` だけに、全件を出す・全部読む作法で `/ponytail:ponytail-audit` をかけ直し、新しい指摘を要件 2・3 に従って直し、0 件になるまでかけ直す
  - 要件 8 の 18 件が全部直ったことを表で照らし、検証の記録に「新しい指摘 0 件」と、残すと裁定された 18・20 の番号・種類・裁定・理由を残す
  - 完了: 検証の記録に 0 件と 18・20 の記録がある
  - _Boundary: areka-seriko の全体（確かめ）_
  - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5, 1.6, 2.1, 2.2, 2.4, 3.1, 3.2, 3.3, 3.4, 3.5, 5.1, 5.2, 5.3, 7.3, 7.5, 8.2, 8.3, 8.6_
- [ ] 6.2 全体テストを回して緑を確かめる
  - `pwsh -NoProfile -File tools/test-all.ps1` を 1 回回す。赤が出たら赤のテストだけに絞って直し、回し直す
  - `Cargo.lock` が変わっていないこと、`doc/`・`docs/` を名指す行を足していないこと、1,000 行を越えるファイルが無いことを確かめる。GPU に触る直しが無いことも記録する
  - 完了: 全体テストが緑で、`git diff main -- Cargo.lock` が空
  - _Boundary: ワークスペース全体（確かめ）_
  - _Requirements: 4.3, 4.5, 6.1, 6.2, 6.3, 6.4, 6.5, 6.6, 6.7_

## Implementation Notes
- 1.1（着手 HEAD `b64c6f68`）: クレートのテストは 550 件（lib 523・balloon_face_e2e 5・bind_e2e 9・cue_sequence 1・loop_integration 9・regression 3）。直した後も同じ 550 件・同じ名。寄せた先は `crate::test_support`（`src/lib_test_support.rs`）の `table_of`・`scope`・`capture_logs`（行の列）・`capture_text`（つないだ文字列。文字列を返していた `capture_logs` の新しい名）・`table_and_logs`・`warns`・`KINDS`・`part_playing`・`overlay_frame`・`one_frame_pattern`・`draw_frame`。中身の違う写しは名で分けた（`actor_trigger_fixture_tests` の `table_and_all_logs`＝畳み込みの記録も捕まえる・`table_interval_words_tests` の `table_and_text`）。`rg` で `src/` に写しが残らないことを確かめた（名が当たる本番の `bind.rs` の `fn scope`・トレイトの `fn playing` は別物）。
- 1.1 で記録した、後で移す・消すテストの名: タスク 3（移すだけ）＝`tick_with_no_shown_slot_is_complete_no_op`・`tick_boundary_cross_emits_show_carrying_pattern`・`send_tick_after_actor_stopped_logs_debug_no_panic`・`emote_surface_change_resets_loop_playback`。5.3（消す）＝`r_post_and_komainu_shell_root_holds_surfaces_txt`。5.4 指摘 13（消す）＝`resolve_is_deterministic_same_input_same_output`・`balloon_resolve_is_deterministic`・`same_input_same_answer`・`settling_twice_gives_the_same_state`、（中の確かめだけ消す）＝`balloon_display_commands_construct_and_match`・`frame_at_sakura_real_sequence_finishes_residual_permanently`。5.4 指摘 14（消す）＝`empty_resolver_resolves_all_none`・`accumulate_on_off_round_trip`・`accumulate_off_absent_is_noop`・`show_command_carries_static_binds`・`show_carries_static_default_when_no_dynamic_bind_applied`。消し終えた見込みは 540 件。
- clippy: toolchain が固定されておらず、今の clippy（0.1.99）では着手の時点から areka-seriko に 4 件の警告がある（`actor.rs:91` large_enum_variant・`looper.rs:433`・`looper_talk_tests.rs:597`・`looper_trigger_tests.rs:504` の collapsible_if）。依存の dola にも出る。各タスクは `cargo clippy -p areka-seriko --all-targets --no-deps` で警告が増えないことを見る。この 4 件は完了の棚卸で扱う。
- 1.2: Rig 4 本は `ClockRig::configured(config, names, static_binds)`（`with_config` はこれを呼ぶ）にまとめ、parts・stage・talk は `type Rig = ClockRig;` と檻ごとの impl。`send(ms, msg)` に時刻の引数が増えた。時計を持たない stage と parts の Rig も時計に揃えて寄せたが期待値は不変（parts の 4 本は最初の `\s` で時計が刻み 0 でなく合図の時刻 0 で生まれる。開始は同じ 0 で、時計の無い道は looper の `refresh(.., None, ..)` のテストが見張る）。`balloon_cue` は `cue(.., CueCommand::BalloonSurface{..}, ..)` へ。範囲の外の同名の `looper_talk_tests.rs` の `fn cue`・`parts_trigger_tests.rs` の `struct Rig` は写しでないので残した。`BindResolver::new` の呼び出しが `actor_test_support.rs` に入った（2.1 で消す相手）。

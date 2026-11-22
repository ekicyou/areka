# Implementation Plan

設計の「タスクの束と順」の T1〜T20 を、そのまま 20 本のタスクにする（案 C・設計の討議の確定 6）。指摘の番号は `research.md` の 2 節と 11・12 節の 1〜76、直し方は design.md の「指摘ごとの直し方」の表と「新しい形を引く直し」の節が正本。行番号は `research.md` の時点のものなので、実装では関数名から引き直す。

すべてのタスクに共通の完了の条件:
- 消すテスト・補助・定数・名は、消す前にツリー全体（テスト・支え・examples・fixture・設定・スキル・手順書・steering・文字列での参照・`#[path]`・`crate_source!`／`include_str!` の字面の読み・テストの名の文字列）で使われていないことを grep で確かめる（要件 3.3）。
- 形を変えた項目の名をツリー全体で grep し、呼び出し元・テスト・支えまで直し切る（要件 3.2）。
- 説明を直す指摘は、書き直す前にそのコメントがコードの言い直しかを見て、言い直しなら消し、コードだけでは分からない理由だけを残す。範囲の外と、この spec が触らないコメントは掃除しない。コードとテストの名に spec の名・要件番号・タスク番号を書かない。
- 触らないファイル（`Cargo.toml`・`Cargo.lock`・`crates/*/Cargo.toml`・`crates/dola/`・`wintf`・`vendors/`・`file_length_guard_test.rs`・`sample-ghost-kit`・`body_systems.rs`・本体 `main.rs`・`reload.rs`・本体 `update/`、`switch_to`・`take_down`・`boot_into` の中身）に差分を作らない。C26 のほかの 2 本の持ち物は、design.md の「並走の spec との継ぎ目（C26）」で申し合わせた use の道と腕の名だけを触る。
- `doc/COMPAT_ARCHITECTURE.md` は 134・177・407 行の文だけを直し、行を足さない。
- どのファイルも 1,000 行を越えさせない（design.md の「1,000 行の番人」の守り方に従う）。
- 触った crate のテストを、赤が出たら赤のテストだけに絞って回し、緑で終える（要件 8.4）。GPU のテストを何度も回さない。

- [ ] 1. 本番の振る舞いを変える直しを、縛るテストと一緒に入れる

- [x] 1.1 満了の後の抑止の間は、可視性の相が次の起床を預けないようにする（T1・1 番）
  - 起床の純判断に「今のフレームで抑止中か」の引数を足し、期限を過ぎていて抑止中なら何も預けない。期限が先なら今どおり残りを預け、抑止なしの超過は今どおり即時
  - 実装の順: 引数を足して中で読まない形にし、縛るテスト 2 本（`overdue_while_suppressed_arms_nothing`・`a_held_timeout_with_the_pointer_still_on_the_balloon_asks_for_no_wake`）が赤になるのを見る → 中身を入れて緑
  - 今の起床のテスト 4 本は第 3 引数を足して改める。即時の起床の説明から「抑止で保留中など」を消す
  - プロセス共有の起床の旗を相の後に読むテストは置かない（字面の検査は本体の `tick_gate_config_producers_tests.rs` が守る）
  - 完了: 縛るテスト 2 本が直しの前に赤・直しの後に緑で、`balloon_visibility_phase_wake_tests.rs` と本体の `tick_gate_config_producers_tests.rs` が緑
  - _Dispatch: mechanical — 判断の形とテストの期待値を設計が与える_
  - _Requirements: 3.1, 4.1, 4.2, 4.3, 4.7, 5.13, 6.4_

- [x] 1.2 ドラッグ中のシェルの窓の scope を、窓の scope の上限で比べる（T2・2 番・件 6）
  - 観測の比べを `as u32` の切り捨てから `window_scope` を通す形に替える。上限越えは `window_scope` が `warn!` を 1 行残して読み飛ばす（記録の抑えは足さない）
  - 64 bit だけで回る縛るテスト `a_dragged_shell_window_past_the_scope_limit_does_not_hold_scope_zero` を、直しの前に「scope 0 の抑止が立つ」で赤になるのを見てから立てる
  - 完了: 縛るテストが緑で、上限越えの窓は抑止を立てず `warn!` が 1 行だけ出ることを表明している
  - _Dispatch: mechanical — 置き換える式とテストを設計が与える_
  - _Requirements: 3.1, 4.1, 4.2, 4.3, 5.6, 6.2_

- [x] 1.3 (P) メニューからのシェル・バルーンの切り替えをインストール中は断り、待ちの頼みが送れたときだけ資産を組む（T3・3・74 番）
  - 切り替えの判定に「インストール中」の答えを足し、入口の「更新中」の次で、出どころがメニューの要求だけを `warn!(skin_switch_installing)` 1 行で断る。台本からの要求は断らない
  - 受理の処理を、組み立ての起こし方を受ける私有の中身に分け、公開の受理は今の署名のまま本物の起こし方を渡す。中身は kanade へ待ちの頼みを先に送り、送れたときだけ組み立てを起こす
  - 新しい子のテストのファイルに縛るテスト `installing_refuses_menu_requests_only`（直しの前は両方「文脈なし」で赤）と `a_failed_gap_request_starts_no_build`（直しの前は起こし方が 1 回呼ばれて赤）を立てる
  - `reload.rs` の受理の呼び手は触らない
  - 完了: 縛るテスト 2 本が緑で、送れない回は「文脈なし」・`error!(skin_switch_send_failed)` 1 行・組み立て 0 回・送れる回は 1 回、を表明している
  - _Boundary: shell_balloon_switch, skin_switch, shell_balloon_switch_tests（新）_
  - _Dispatch: standard — 新しい断りの腕と注入の口を組み、赤から立てる_
  - _Requirements: 3.1, 3.4, 4.1, 4.2, 4.3, 5.16, 6.2, 7.6_

- [ ] 2. ゴースト切替の入口を直す

- [x] 2.1 (P) 特別な名の解決の純関数のテストを、関数と同じ crate へ移す（T9・23・56 番）
  - `ghost_switch_tests.rs` の特別な名の解決の純関数のテスト（約 180 行・8 本と補助）を、`areka-runtime` の新しい子のテストのファイルへ移し、関数のファイルに `#[cfg(test)]` の宣言を置く。選ばない場合の補助は両方に置く
  - 解けない理由の型の取り込みを移った先に合わせ、残る側の同じ形の補助を 1 つにする（56 番）
  - 完了: 移した先で同じ数のテストが同じ名前で緑になり、`ghost_switch_tests.rs` が約 800 行になって `areka-session` のテストが緑
  - _Boundary: special_name, special_name_tests（新）, ghost_switch_tests, ghost_switch_roots_tests_
  - _Dispatch: mechanical — 移すテストと置き場を設計が与える_
  - _Requirements: 3.1, 3.3, 5.14, 6.1_

- [x] 2.2 メニューからのゴーストの切り替えを、ネットワーク更新の実行中は断る（T4・76・55 番）
  - 着手の前に、`audit-areka-install-update` からの知らせ（新しい腕の名を `install/overwrite.rs` の網羅の `match` にこちらが足してよいかの開発者の確定）を確かめる。確定していれば新しい判定 `Updating` を足し、同じ変更で相手のファイルの `NotFound | NoContext` の腕に名を 1 つだけ足す。確定しなければ腕を足さず既にある `Busy` を返す
  - 入口の「進行中」の次で、出どころがメニュー（`Manual`）の要求だけを `warn!(ghost_switch_updating)` 1 行で断り、kanade へ何も送らない。台本と手続きの要求、`reload.rs` から直に呼ばれる受理には当てない
  - 縛るテスト `updating_refuses_menu_requests_only` を、直しの前は「メニューも受理」で赤になるのを見てから立てる。置き場は 2.1 で軽くなった `ghost_switch_tests.rs`
  - 文脈なしの `warn!` を私有の関数 1 つにまとめる（55 番）
  - 完了: 縛るテストが緑で、メニューは断り・台本（名で指す）と手続き（フォルダで指す）は受理、を表明し、ワークスペース全体が組める
  - _Depends: 2.1_
  - _Dispatch: standard — 断りの腕と並走の相手のファイルの腕の名を同じ変更で入れる_
  - _Requirements: 2.1, 3.1, 3.4, 4.1, 4.2, 4.3, 5.16, 6.1, 6.2, 7.1, 7.6, 7.8_

- [ ] 3. 公開の面を絞り、本体の可視性のテストを `areka-runtime` へ戻す

- [x] 3.1 `areka-session` の切り替えの公開の面を絞る（T5・4・5・6・38・39 番・件 1・件 2）
  - シェル・バルーンの切り替えの出し直しを設計の 5 型だけに絞り、残り 5 項目は私有の取り込みにする。範囲の中のテストは `areka_runtime::skin_switch` から直に引く
  - `SwapFinish` の出し直しを消し、名の解決の 2 項目を私有の取り込みにする。範囲の外で使い手の無い足場の 2 関数（`add_balloon_copy`・`plant_last_balloon`）を `pub(crate)` にし、`areka-session` の 4 項目を狭める。`LastInstalledGhost` は今のまま
  - `audit-areka-app` の持ち物 4 か所（`reload_test_support.rs:14`・本体 `runtime_tests/frame/switch_tests.rs:37`・`:838`・`switch_finish_tests.rs:50-52`）は申し合わせのとおり use の道だけを替える。use の行がぶつかったら後から着地する側が合わせる
  - `skin_switch.rs` の頭の説明を事実に合わせる
  - 完了: ワークスペースが組め、`areka-session` と本体のテストが緑で、範囲の外の差分が use の道だけ
  - _Depends: 1.3, 2.2_
  - _Dispatch: standard — 公開の面の狭めと範囲の外の use の道を同じ変更で直す_
  - _Requirements: 3.1, 3.2, 5.1, 5.2, 7.1, 7.2, 7.7, 7.8_

- [x] 3.2 本体の可視性の相のテスト 29 本を `areka-runtime` へ戻す（T6・24・25・26・40・64・65・68・69 番・件 4）
  - 順は設計の ⑴〜⑸: 相が読まない `BalloonWiring` の据え付けと関連の欄・取り込みを消す → 本体の空の起動の資産を共有の支えの合成に替える → 2 本を記録のテストのファイルへ移して `world_at` の写しを消す → 残り 27 本を同じ名のファイルへ移す（2 つは `git mv`）→ 宣言を移した先の親に置く
  - テストの名が重なれば、移す側の名に場面の語を足す。本体に残す `notice_tests` と代わりの親は、要る宣言と取り込みだけにする
  - 移した後の親と見出しと名指しを、移した後の姿で直す（64・65・68・69 番）。組めない 1 本が出たら理由を記録して本体に残し、議題にする
  - 完了: `cargo test -p areka-runtime` で移した 29 本が同じ名前で緑になり、本体の側から消えた数と合い、本体の 2 ファイルが消えている
  - _Depends: 1.2_
  - _Dispatch: standard — 29 本の移しと据え付けの削りを組めるように直す_
  - _Requirements: 3.1, 3.3, 4.4, 4.6, 4.7, 5.4, 6.1, 6.6, 6.7_

- [x] 3.3 `areka-runtime` の公開を、テストを戻した後の使い手で数え直す（T7・7・36・37・38 番・件 3）
  - 箱の上の滞在の確定を `pub(crate)` に、ホバーの注入の mod を `pub(crate)` に、移動の cue の出し直しを私有の取り込みにする（型シームは残す）
  - 3.2 で使い手を失った渡し口・テストの開け口・支えの項目と、`areka-runtime` の 9 つ（私有 8・`pub(super)` 1）を狭める。`areka-runtime` の外に使い手があるかを項目ごとに grep で確かめる
  - 完了: ワークスペースが組め、狭めた項目の外の使い手が 0 件であることを grep で確かめ、`areka-runtime` と本体のテストが緑
  - _Depends: 3.2_
  - _Dispatch: mechanical — 狭める項目と先を設計の表が与える_
  - _Requirements: 3.1, 3.2, 5.3, 7.7_

- [ ] 4. テストの足場と檻を寄せる

- [x] 4.1 (P) 移動の cue のテストの支えと檻を直す（T8・15・19・27・44・45・46 番・件 5・件 7）
  - 偽の HWND の輪と偽の記憶の書き手の写しを、`areka_placement::test_support` の 2 つを直に使う形にする
  - 記憶の書き手の後始末の 2 か所を `join_bounded` で待ち、panic を捨てない形にする
  - 位置の基準を 2 通り（自分基準と番号基準）で回す・単体の重なり 4 本を消す・cue を組む補助を 1 つに置き換える
  - 受け手を落としてから移動を 1 件流し、panic せず `warn!` が 1 行のテストを足す（46 番）
  - 完了: `areka-runtime` の移動の cue のテストが緑で、足したテストと後始末の待ちが緑
  - _Boundary: move_cue_*_tests_
  - _Dispatch: mechanical — 寄せ先と消すテストを設計の表が与える_
  - _Requirements: 3.1, 4.1, 4.7, 5.5, 5.7, 6.3_

- [x] 4.2 `areka-session` の切り替えの足場を寄せる（T10・28・29・31・32・52・53・54 番・件 8・件 9）
  - 足場に `boot_context(root, current)` を置き、`ghost_switch_tests.rs` の写しを消す。範囲の外の写しには触らない
  - `add_rpost_shell` の写し 2 つを、周回の足場が写す元のシェルの道を受ける形と std だけの `copy_tree` の 1 か所にまとめる。検体を取る約 4 行と残す理由の 1 行は呼び手 2 か所に置く（本体 `session_update_tests.rs` を含む）
  - 事象を数える補助を `fallback_tests` の 1 つに、`kinds` を引く形に、周回のテストの時刻と道の補助を足場の 1 つに寄せる
  - `spawn_windows(rig, dpi)`・`pump_messages` を `pub` にする（こちらの呼び手は 96 を渡す）
  - 完了: `areka-session` と本体の `session_tests` が緑で、写しが 0 か所（範囲の中）になっている
  - _Depends: 2.2_
  - _Dispatch: standard — 足場の口の形を呼び手の数を見て決める_
  - _Requirements: 3.1, 5.8, 5.9, 7.1_

- [x] 4.3 本体の切り替えのセッションのテストの足場を直す（T11・26・30・57・63 番・件 10）
  - 足場 `rig` で本体の組を 1 回据え、3 本の手書きの 1 行を消す（表明は変えない）
  - 空の配線を共有の支えの `wiring().1` に、要求を数える補助と descript を書く補助を 1 つずつにし、説明の道を直して「（20 秒）」を消す
  - 完了: 本体の `session_tests/shell_balloon_switch*` が緑で、`error!(body_systems_missing)` が出なくなっている
  - _Depends: 3.1_
  - _Dispatch: mechanical — 据える行と消す行を設計が与える_
  - _Requirements: 3.1, 5.10_

- [ ] 4.4 可視性の檻の抜けと重なりを直す（T12・16・17・18・21・22・33・41・42・43・70・71 番）
  - 「0 件」の檻を欄で数え、陽性の対の表明を 2 か所に足す（16 番・単位の行が捕捉の窓に出ないときは出る場面を同じファイルに置く）
  - 消す数のテストを対照つきで 1 本足す・先に 1 回呼んでから 2 回目の記録 0 件を表明する・段落を消す
  - 重なりのテスト（時刻ありと時刻なしの 2 本・talk_end の 1 本）を、残るテストが同じ中身を縛ることを確かめてから消す
  - フレームの補助を `Frame` の組み立てに置き換え、何もしない時間切れの指定を消して表の時間切れを既定でない値にし、ホバーの補助を支えへ寄せる。見出しと名指しを観測に合わせる
  - 完了: `areka-runtime` の可視性のテストと `areka-session` の該当のテストが緑で、`balloon_visibility_tests.rs` が約 790 行
  - _Boundary: balloon_visibility_*_tests, balloon_visibility_test_support, ghost_switch_reload_tests, shell_balloon_switch_session_balloon_tests_
  - _Depends: 3.2_
  - _Dispatch: standard — 消す前の確かめと陽性の対を組む_
  - _Requirements: 3.1, 3.3, 4.1, 4.7, 6.1_

- [ ] 4.5 切り替えのテストの小さな直しを入れる（T13・14・20・47〜51 番）
  - 準備の失敗を `expect` で止める・アクターのスレッドの数を足場の中で 1 つにして呼び手の 3 行ずつを消す・後始末の前に終了の要求が無いことを表明する
  - 組み立てのワーカーへ暗黙の MTA なら panic する閉包を注入するテストを 1 本足す（48 番・依存を足さずに組めなければ議題）
  - 読み直し・起動を壊す・理由を数える補助と柵の補助を 1 つずつにし、名をバルーンに付ける補助を呼ぶ
  - 完了: `areka-session` の切り替えのテストと `switch_assets_tests.rs` が緑で、足したテストが緑
  - _Depends: 4.2_
  - _Dispatch: standard — 注入のテストを組む_
  - _Requirements: 3.1, 4.1, 6.2, 6.3, 6.6_

- [ ] 4.6 説明とテストの名から spec の名と番号を消す（T14・13 番）
  - `research.md` の 13 番の場所の一覧のとおり、括弧の番号を消し、テストの名を替え、見出しを語だけにし、`MoveMeasure` の説明を理由だけの文にする
  - 完了: 範囲の中のコードとテストの名に spec の名・要件番号・タスク番号が 0 件（grep で確かめる）で、触った crate のテストが緑
  - _Depends: 3.2, 4.1, 4.4_
  - _Dispatch: mechanical — 場所と直し方を research の一覧が与える_
  - _Requirements: 3.1, 7.7_

- [ ] 5. 負荷と説明と範囲の外を直す

- [ ] 5.1 取り出しの 1 回につき目録を 1 度だけ読み、バルーンの scope ごとの鍵を 1 回だけ作る（T16・34・35 番）
  - 入口の中身に目録を一度だけ埋める私有の引数と、目録を読む閉包を受ける注入の口を足す。公開の入口は空を渡し、取り出しの系はループの外で 1 つ作って全件に渡す。受理の後は埋めた目録を捨てる
  - 縛るテスト `unresolved_names_in_one_drain_read_the_catalog_once` をゴーストとシェル・バルーンの両方に、直しの前は閉包が 3 回で赤になるのを見てから立てる
  - 可視性の相で scope ごとの鍵を 1 回だけ作って共用する
  - 完了: 縛るテストが緑で、同じ解けない名 3 件で `warn!` が 3 行・目録の読みが 1 回、受理の後は読み直しの 1 回だけ、を表明し、`ghost_switch.rs` が 960 行を越えない
  - _Depends: 2.2, 3.1_
  - _Dispatch: standard — 目録の共用の口と数えるテストを組む_
  - _Requirements: 3.1, 4.1, 4.2, 6.1_

- [ ] 5.2 本番の説明を事実に合わせる（T15・8・9・11・12・58〜62・66・67・72・73・75 番）
  - 設計の表のとおり、記録の語彙の括弧・最後の 1 文・2 行を消し、位置の数値を作者基準 px に、窓の位置の指令の 1 行目を全体に、crate の道と呼び手の場所と `// SAFETY:` の条件と失敗の並びを直す
  - 「今日どおり」を「そのまま終了する」「通常どおり終わる」の形にする（ログの語・target・欄は変えない）
  - 完了: 直した説明の字面を読むテストと道具が 0 件であることを grep で確かめ、ワークスペースが組め、触った crate のテストが緑
  - _Depends: 5.1_
  - _Dispatch: mechanical — 直す文の形を設計の表が与える_
  - _Requirements: 3.1, 7.7_

- [ ] 5.3 (P) 範囲の外の持ち主の無い 3 件と `COMPAT` の 3 行を直す（T17・5.15・10 番・ア）
  - ㋐ 記憶の書き手の後始末を `join_bounded` で待つ。㋑ `scale_signed` を収まる答えは素通しにし、飽和の判定を同じ答えに合わせる。縛るテスト「恒等比の `i32::MIN` は `i32::MIN`」を直しの前に赤で見てから立て、固定していた檻 2 本を「恒等比は全域で素通し」に改める。㋒ コメントを消す
  - `doc/COMPAT_ARCHITECTURE.md` の 134・177・407 行の文だけを直す（抑止の理由に、箱に文字の出ている scope のシェルの窓のドラッグと文字の出ている箱の上の滞在を書き、定義点を名指す）。行は足さない
  - 完了: `areka-placement` と `areka-runtime` の該当のテストが緑で、`git diff -- doc/COMPAT_ARCHITECTURE.md` が 3 行の中の変更だけ（行の数は同じ）
  - _Boundary: areka-placement, frame_persist_test_support, shell_balloon_resolve, COMPAT_ARCHITECTURE_
  - _Dispatch: standard — 飽和の判定を合わせ、檻を改める_
  - _Requirements: 3.1, 4.1, 4.2, 4.7, 5.15, 6.3, 7.5_

- [ ] 6. 預かった件と申し送りを記録する（T18）
  - `research.md` に、件 1〜10 の片づけを出どころと指摘の番号とタスクと一緒に書き、件 12 は済んだままであることを記す
  - 規模の見立て（20 タスク）と、直さずに残す指摘 0 件を記録として残す
  - `research.md` の 6 節の件を、並走のメモと相手のセッション（`audit-areka-app`・`audit-areka-install-update`）へ申し送り、申し送った件と相手を記録する。書く前に相手が `completed/` でないことを確かめる
  - 3 番の続き（読み直しの入口が台本も断る件）は、相手が設計で「MCP からの読み直しだけを断る」と決めて片づいたことを記録する
  - `update_reload_requested` の語の替えは窓口の側の記録に及ばず、範囲の中の読み手（`shell_balloon_switch_session_update_tests.rs:243`）の直しは要らない（相手が確かめた）ことを記録する
  - 完了: `research.md` に件 1〜10・12 の行と申し送りの表がそろい、並走のメモに同じ中身が載っている
  - _Depends: 5.2, 5.3_
  - _Dispatch: mechanical — 書く中身を設計の表が与える_
  - _Requirements: 2.1, 2.2, 2.3, 2.4, 2.5, 2.6, 5.11, 5.12, 7.3, 7.8, 7.9_

- [ ] 7. 仕上げの確かめ

- [ ] 7.1 範囲へ audit をかけ直し、新しい指摘を 0 件にする（T19）
  - `main` を取り込み、範囲のファイルの数と行数を測り直す（移したファイルと新しいファイルは移した先で数える）
  - 範囲だけへ ponytail の audit を、上限 20 件を外し全部のファイルを全行読む作法でかけ直す。出た指摘は直しが生んだ物も含めて直し、0 件になるまで繰り返す
  - 5 回目で収まらず「必ず直す」と本番の振る舞いに関わるものが 0 件なら、開発者に一言知らせて打ち切る。同じ所から「必ず直す」が出続ければ Fable で総括して議題にする。前の回の直しが呼んだ連鎖は Fable でまとめて直す
  - 完了: `research.md` の「かけ直しの結果」に回ごとの件数と直しが書いてあり、最後の回が 0 件（または要件 9.6 の打ち切り）
  - _Depends: 6_
  - _Dispatch: standard — audit の結果に応じて直しを組む_
  - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5, 1.6, 1.7, 1.8, 9.1, 9.2, 9.3, 9.6, 9.7, 9.8_

- [ ] 7.2 全体テストと既定の枝との差分で約束を確かめる（T20）
  - `pwsh -NoProfile -File tools/test-all.ps1` を 1 回回し、緑で終える。赤なら赤のテストだけに絞って回し直す。emo2・pasta の実走は全体テストの中だけで回し、SSP の測りや画面の操作は回さない
  - 設計の「仕上げの確かめ」の 4 の差分の確かめ（触らないファイルの差分が空・`COMPAT_ARCHITECTURE.md` が 3 行の中だけ・C26 のほかの 2 本の持ち物が申し合わせた use の道と腕の名だけ・範囲の外の差分が一覧のファイルだけ・外への約束の変化が挙げた分だけ）を行う
  - 依存・フォーク・`[patch]`・`vendors/` の差分が 0、端末へ出す文を足していない、テストの根が `target` の下であることを確かめる
  - 完了: 全体テストの緑と差分の確かめの結果が検証の記録に書いてある
  - _Depends: 7.1_
  - _Dispatch: mechanical — 回すコマンドと確かめる差分を設計が与える_
  - _Requirements: 4.4, 4.5, 6.5, 6.6, 6.7, 6.8, 7.2, 7.3, 7.4, 7.5, 7.10, 8.1, 8.2, 8.3, 8.4, 8.5, 9.4, 9.5_

## Implementation Notes
- 3.3: 署名の鎖で開いていた可視性の型（`VisibilityAction`・`VisibilityLogEvent` ほか）も、外の使い手が 0 なので私有・`pub(crate)` へ戻した。外に使い手があって `pub` に残したのは `BalloonVisibilityState`・`VisibilityDecision` と `timeout_notice`・`ScopeObservation` の型・`Frame` の 7 つの口と `TALK_ID`・`seen`・`notify_timeout`・`configured_timeout_secs`・`phase::run_balloon_visibility_phase`。
- 2.2 は ghost_switch の filter だけで通し、同じ crate の口の束の順のテストを赤にしていた（`4cb2d09e5` で直した）。触った crate は lib 全体を回してから閉じる。

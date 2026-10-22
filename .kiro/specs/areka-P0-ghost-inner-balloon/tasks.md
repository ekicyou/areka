# Implementation Plan

## Task Overview

ゴーストのフォルダの中の `balloon\<フォルダ名>\` のバルーンを、そのゴーストにだけ見える候補として、起動・切替・メニュー・タグ・記憶・読み直し・更新・開く・インストールの後の表示名のすべてで扱う。ゴーストの書庫とシェルの書庫の同梱のバルーンは、入れるゴーストの中の `balloon\` へ入れる。目録に合わせる口を 1 つ（`balloons_for`）足して一覧を引く 6 か所を替え、`areka-nar` で同梱の宛先と入れ子の確定の順を直し、§8 と README に areka 独自の決まりとして書く。

- 検体と一時フォルダは、どのテストも `TempPath::under_target`（ワークツリーの `target\` の下）に作る（要件 10.2）。
- テストは実装の兄弟ファイルに置く。新しい外部の依存は 0。
- 新しい記録は 3 つ（info `ghost_balloon_hides_root`・info `installed_balloon_hidden_by_ghost`・warn `companion_overrides_archive_folder`）。事象名の形は `.kiro/steering/logging.md` に従う。
- 並走の注意: `areka-P0-install-live-target-hazards` と `areka-nar` の `plan.rs`・`install.rs` が重なる。同じウェーブに置かない。

---

## Tasks

- [x] 1. 目録: ゴーストの中のバルーンの走査と、根の和との合わせ方（`areka-ghost`）
- [x] 1.1 ゴーストの中の `balloon\` を、根と同じ判定で走査する
  - 根の 1 根の走査の本体を「置き場と基のフォルダを受ける 1 つの関数」へ切り出し、根の走査はそれを呼ぶだけにする（一時の置き場の印で 0 件にする扱いは根の側に残す）。根の走査の答えは 1 つも変えない
  - ゴーストのフォルダを受けて `<ゴースト>\balloon\` の直下 1 段だけを同じ関数で走査する口を足す。作る項目の `root` はそのゴーストのフォルダ、`dir` は `<ゴースト>\balloon\<フォルダ名>`。一時の置き場の印は見ない（`--ghost` で渡したゴーストでも数える）
  - 判定・素性の作り方・記録（`catalog_type_not_balloon`・`subdirs` の読めない警告・descript.txt が無いものは黙って除く）を根と同じ経路に通し、記録の無い失敗の道を作らない
  - `BalloonEntry.root` の欄の説明を「走査した基のフォルダ（根の項目は根・ゴーストの中の項目はそのゴーストのフォルダ）」に、`catalog.rs` の頭の説明の走査する場所に `<ゴースト>\balloon\` を足す
  - 新しい兄弟テスト `catalog_roots_ghost_balloon_tests.rs` に、`balloon\` の直下の `type` 無し／`type,balloon` は数える・ゴーストの直下の `type,balloon` のフォルダ・`balloon\` そのものの descript.txt・`balloon\<名>\<下>\` は数えない・`type,shell` のフォルダは除いて `catalog_type_not_balloon` を 1 件・`balloon\` が無ければ 0 件で記録 0 件、を置き、`cargo test -p areka-ghost` が緑
  - _Requirements: 1.1, 1.2, 1.3, 10.2, 10.3_

- [x] 1.2 今のゴーストの候補の一覧を作る口 `balloons_for` を足す
  - `GhostBalloons { listed, hidden }` と `balloons_for(roots, ghost_dir)` を `catalog_roots.rs` に足し、`lib.rs` の再輸出に並べる。頭の説明に「ゴーストの中との合わせ方もここ」を足す
  - `listed` はゴーストの中の項目（フォルダ名のバイト順）を先に、根の和（今の `list_balloons` の答え）からゴーストの中と `fold_key` が同じものを除いた残りを今の並びのまま後に並べる
  - `hidden` には、ゴーストの中が隠した根の項目だけを 1 件ずつ（種類・根の項目のフォルダ名・外した根の場所・隠したゴーストの中の場所）入れる。根どうしの隠れは入れない。この口は記録を残さず、保存もしない（呼ぶたびに今のファイルの状態から作る）
  - 根だけの口（`list_balloons`・`find_balloon`・`survey`・`install_root`）は 1 行も変えない
  - 兄弟テストに、ゴーストの中だけ・根だけ・両方に同じフォルダ名（大文字小文字だけ違う綴りを含む）での `listed` の中身と並び（ゴーストの中が先）と `hidden` の中身、ゴーストの中が 0 件なら `listed` が `list_balloons` と等しく `hidden` が空・根どうしの隠れの答えが不変、以前に根へ入れた同名でないバルーンが根の項目のまま出る、2 つのゴーストで呼び分けると他方の中が出ず隠れた根のものが出る、ゴーストのフォルダを消した後は出ない、を足して緑
  - _Requirements: 1.5, 2.2, 2.5, 3.1, 3.4, 6.1, 6.2, 6.4, 8.4, 10.1_

- [x] 2. 起動とゴースト切替の決め方（`areka`）
- [x] 2.1 起動とゴースト切替の候補を `balloons_for` で作り、隠れを記録する
  - `boot_config.rs` の `resolve_balloon_for_ghost` の一覧を `balloons_for(roots, ghost_dir).listed` に替える。記憶・descript の 2 鍵・同梱の読み方と `resolve_balloon` の段は変えない。引数のバルーン（段 1）は一覧を読まないまま
  - `hidden` の 1 件ごとに info `ghost_balloon_hides_root`（`folder`・`hidden`・`by`）を残す。メニュー・タグ・開く・更新では残さない
  - 「当たらなかった」記録（`last_balloon_not_found`・`descript_balloon_not_found`・`companion_balloon_not_found`）と `NoBalloon` の告知は、事象名・欄・`balloon_store`／`balloon_stores`（根の置き場の列だけ）を変えず、記録の文だけを「根にもゴーストの中にも見つからない」へ直す
  - 説明の直し: `boot_config.rs` の「ゴーストのフォルダの中の `balloon/` は読まない」、`boot_resolve.rs` の `BalloonInputs.listed`・`find_balloon_by_name` の「フォルダ名の昇順」を「ゴーストの中が先、それぞれフォルダ名の昇順」へ。判断のコードは変えない
  - `boot_config_balloon_tests.rs`: `default_balloon_path_does_not_see_ghost_inner_balloon` を「当たる」答えのテストへ書き換えて名前も改め、`balloon/claudia` は `<ゴースト>\balloon\claudia` が在っても当たらず `descript_balloon_not_found` が今の欄で 1 件・同梱の段がゴーストの中に当たる・根が空でもゴーストの中から決まり `NoBalloon` にならない・`--ghost` で渡したゴーストのフォルダでも数える・`ghost_balloon_hides_root` が隠れ 1 件ごとに 1 件、を足して緑
  - `boot_resolve_balloon_tests.rs`: 同じ `name` がゴーストの中と根に在る一覧で、descript の `balloon` がゴーストの中に当たることを足して緑
  - _Requirements: 1.4, 3.2, 3.3, 4.1, 4.2, 4.3, 4.4, 4.5, 4.7, 6.3, 10.1_

- [x] 2.2 ゴースト切替・記憶・読み直しでゴーストの中のバルーンが保たれることを確かめる
  - コードの変更は 0 の見込み（切替は `resolve_balloon_for_ghost` を通り、記憶はフォルダ名、読み直しは今のバルーンの場所で組む）。テストで崩れが見つかったら、その道だけを直す
  - `ghost_switch_balloon_tests.rs`: 切替先のゴーストの中のバルーンで決まり候補が切替先のものに入れ替わる・記憶にフォルダ名だけが書かれる（形は今と同じ）・次の切替で記憶の段から同じものが出る・本 spec の前の形の記憶がそのまま読める・記憶のバルーンがゴーストの中から消えたら `last_balloon_not_found` を残して次の段へ、を足して緑
  - `emo2_boot/reload_skin_tests.rs`: ゴーストの中のバルーンを使っているときの `\![reload,balloon]`（MCP の読み直しと同じ道）で、使うバルーンが同じ `<ゴースト>\balloon\<名>` のままであることを足して緑
  - `emo2_boot/ghost_switch_reload_tests.rs`: ゴーストの中のバルーンを使っているときのゴーストの読み直し（自分への切替）で、記憶の段から同じ `<ゴースト>\balloon\<名>` が出ることを足して緑
  - _Requirements: 2.3, 4.6, 5.2, 5.3, 5.4, 5.5, 5.6, 5.7, 8.5, 10.1_

- [x] 3. 実行中の呼び手（`areka`）
- [x] 3.1 (P) バルーンの切替と `\![open,explorer,…]` の候補を今のゴーストで引く
  - `shell_balloon_resolve.rs` の `balloon_candidates` にゴーストのフォルダの引数を足して `balloons_for(..).listed` を使い、呼び手 2 か所を直す: `shell_balloon_switch.rs` の切替の候補は `balloon_candidates(&ctx.roots, &ctx.ghost_dir)`、`readme/opener.rs` の `named_folder` は `ctx.baseware` から取り出した `root` を使って `balloon_candidates(root, &ctx.ghost_dir)`
  - `name` の引き方（`resolve_skin_target`・`skin_by_name`）と切替の記憶の書き方は変えない
  - `shell_balloon_resolve.rs` の頭の説明の「フォルダ名（大文字小文字を区別）」を「フォルダ名（`same_folder_name`＝区別しない・`name` は区別する）」へ、「バルーンは根の目録」を「今のゴーストの中と根の目録」へ直す
  - `shell_balloon_resolve_tests.rs`・`shell_balloon_switch_tests.rs`・`readme/opener_tests.rs` に、`\![change,balloon,名前]`・フォルダ名・`random`・`lastinstalled` の候補にゴーストの中が入る・同じ `name` はゴーストの中が当たる・切り替えたらフォルダ名を記憶する・`\![open,explorer,balloon,名前]` がゴーストの中のフォルダを開く、を足して緑
  - _Requirements: 2.1, 3.3, 5.1, 9.2, 10.1_
  - _Boundary: RuntimeCallers（切替・開く）_
  - _Depends: 1.2_

- [x] 3.2 (P) バルーンのメニューにゴーストの中のバルーンを先に並べる
  - `menu/balloon_frame.rs` の一覧を `balloons_for(&ctx.roots, &ctx.current.ghost.dir).listed` に替え、頭の説明を直す。今のものの印（`current.balloon.folder`）の付け方は変えない
  - `menu/balloon_frame_tests.rs` に、ゴーストの中が先・根が後に 1 つの一覧で並び、ゴーストの中のバルーンを使っているときにそこへ今と同じ印が付く、を足して緑
  - _Requirements: 2.1, 2.4, 2.5, 10.1_
  - _Boundary: RuntimeCallers（メニュー）_
  - _Depends: 1.2_

- [x] 3.3 (P) その他の更新のバルーンの名前引きを今のゴーストで引く
  - `update/desk.rs` の `updateother` のバルーンの一覧を `balloons_for(&here.roots, &here.ghost_dir).listed` に替え、説明を直す。`find` の引き方は変えない
  - `update/desk_resolve_tests.rs` に、`updateother --balloon=名前` がゴーストの中に当たる（同じ `name` はゴーストの中）・今のバルーンがゴーストの中のものなら今のバルーンの更新の対象がその `<ゴースト>\balloon\<名>` になる、を足して緑
  - _Requirements: 2.1, 3.3, 8.3, 10.1_
  - _Boundary: RuntimeCallers（更新の受付）_
  - _Depends: 1.2_

- [x] 3.4 (P) ゴーストの中のバルーンの更新で書く先をテストで固める
  - `roots/user_root_tests.rs`: ゴーストの中のバルーンの場所（親が `<実行ファイルの隣>\balloon` でない）で `plan_copy` が写さずそのまま（`AsIs`）を返すことを固める
  - `update/worker_tests.rs`（`prepare_targets`）: 起動のときのゴーストが実行ファイルの隣の根に在るとき、書ける根へ写されてから動くか（写されるならゴーストの中のバルーンの更新は写しの中を書く）を固める。写されない場合の扱いはゴーストそのものと同じでよいことをテストの説明に書く
  - 写されない道が見つかり書く先が記録に出ないときは、その道で書く先を記録に残す 1 行を足す（記録の無い失敗を作らない）
  - `areka-update` の `manifest_tests.rs`: ゴーストの更新の一覧の `balloon/<名>/descript.txt` のような下の段のパスを拒まないことが今のテストで固まっているかを確かめ、無ければ 1 本足す（`areka-update` のコードは変えない）
  - 上の各テストが緑で、ゴーストの中のバルーンの更新の書く先が決定論のテストで 1 つに決まっている
  - _Requirements: 8.1, 8.2, 8.3, 10.1, 10.3_
  - _Boundary: 更新の書く先（roots/user_root・update/worker・areka-update manifest）_

- [x] 4. インストール: 同梱の宛先と入れ子の確定（`areka-nar`・`areka` のインストールの側の期待値）
- [x] 4.1 入れ子の配置の組み上げと確定の手を足す（`install.rs` が中心・`plan.rs` の判断は触らない）
  - `Placement` に `carve_out: Vec<String>` と `within_body: bool` を足す（crate の中だけ）。`plan.rs` で配置を組む所には既定の値（空と偽）を足すだけにし、宛先と項目の判断は 1 つも変えない
  - 本体の組み上げで、下敷き（`Overlay`・`Replace{keep}`）を写すとき `carve_out` の相対のフォルダの下を写さない（比べ方は今の残す名前と同じ ASCII の大文字小文字を区別しない）
  - `within_body` の配置は、確定で組み上げのときの `ExistingState` に関わらず「無い所へ置く」1 手（親を作る → `rename` → 戻しは「消す」）にし、`ExistingState` は入れた物の報告にそのまま使う。戻しは今の逆順（後の配置を消す → 本体を戻す）のまま。`within_body` が偽の配置は今の確定のまま
  - 確定の失敗は今の失敗の記録（`[areka_nar] refused or failed`）の道を通し、根の置き場へ入れ直す道は作らない
  - `install_commit_tests.rs` に、手で組んだ `Placement`（本体＋`within_body` の同梱）で、本体 `refresh,1`＋古い同梱在りで同梱が巻き戻らず入る・本体 `Overlay`＋同梱 `balloon.refresh,1` で同梱だけ作り直される・同梱に `balloon.refresh` が無ければ古いファイルが残り新しいものが重なる・初めてのゴースト・同梱の確定の失敗で全体が戻り（同梱を消してから本体を戻し、古い同梱が元のまま）、を足す
  - `cargo test -p areka-nar -p areka` が緑（計画の答えは変わらないので `areka` の期待値は動かない）
  - _Requirements: 7.7, 8.1, 10.1, 10.3_
  - _Boundary: NestedCommit_

- [x] 4.2 同梱のバルーンの宛先をゴーストの中へ替え、入れ子の配置へつなぐ（統合）
  - `companion_placement` の宛先を `<ゴーストのフォルダ>\balloon\<companion.directory>`、`root` を本体の根にする。ゴーストのフォルダは、ゴーストの書庫では本体の宛先、シェルの書庫では `existing_target_ghost` が確かめた対象のゴースト。同梱のために `request.roots.balloon_root` を呼ばない（バルーンだけの書庫の道には残す）
  - ゴーストの書庫で置く同梱（飛ばした同梱は含めない）ごとに、本体の項目から書庫の `balloon/<companion.directory>/` の下も除き、本体の `carve_out` に `balloon\<dir>` を足し、同梱の `within_body` を真にする。除いた項目のうちどの同梱の取り出し元の中にも無いものが 1 つでも在るときだけ warn `companion_overrides_archive_folder`（`directory`）を 1 件残す
  - 名指しされない `balloon\<名>\` は今どおり本体の項目に残す。`install.txt` の鍵の読み方（`manifest.rs`）は変えない。`plan.rs` の頭の説明の宛先の形の約束を「`root.join("ghost"|"balloon").join(名)`、またはそのゴーストの宛先の下の `shell\<名>`・`balloon\<名>`」へ直す
  - `plan_tests.rs`・`plan_roots_tests.rs`・`plan_source_path_tests.rs`: ゴーストの書庫とシェルの書庫の同梱の宛先が `<ゴースト>\balloon\<dir>` で `root` が本体の根・同梱で `balloon_root` が呼ばれない・書庫の `balloon/<dir>/` の項目が本体から除かれ警告 1 件・取り出し元がその `balloon/<dir>/` 自身の書庫は警告 0 件・名指しされない `balloon\<名>\` は本体に残る・全ての宛先が根の下、を直し・足す
  - `lib_tests.rs` の `a_failure_in_the_second_root_rolls_back_the_first_root_and_records_both_work_folders`（同梱が別の根へ行く前提で陳腐化）を外し、ゴーストの書庫の入れ子の配置で同梱の確定が失敗すると本体が戻り、作業の場所（`<根>\.nar-work\…`）が 1 つの根に収まり、根の置き場へ何も入らず、失敗の記録が残る、を確かめるテストへ置き換える。`lib_skip_tests.rs` の宛先の期待値を `<ゴースト>\balloon\<dir>` へ直す
  - 同じ task の中で `areka` の期待値を直す: `install/destination_tests.rs`・`install/procedure_skip_tests.rs`（宛先の綴りに依るなら `install/procedure_tests.rs` も）の同梱の宛先を `<ゴースト>\balloon\<dir>` へ
  - `cargo test -p areka-nar -p areka` が緑
  - _Requirements: 6.4, 7.1, 7.2, 7.3, 7.5, 7.7, 7.8, 8.2, 10.1, 10.3_
  - _Boundary: CompanionPlan, NestedCommit, install destination tests (integration)_

- [ ] 5. インストールの側（`areka`）
- [x] 5.1 (P) 同梱の表示名を入れたゴーストの中から引く
  - `install/procedure.rs` の `installed_items` で、同梱（先頭より後のバルーン）の名前を、根だけの `find_balloon` でなく、`element.path` の 2 段上を `ghost_dir` として `balloons_for(&facts.roots, ghost_dir).listed` からフォルダ名で引く。見つからない・`name` が無いときは今どおりフォルダ名。`OnInstallCompleteEx` の Reference2・`skipped_notice`・`installed_record` は変えない
  - `procedure_tests.rs` に、同梱の表示名がゴーストの中の `name` で、根の同じフォルダ名の別の `name` を取らない、を足して緑
  - _Requirements: 7.6, 10.1_
  - _Boundary: InstalledItemsName_
  - _Depends: 1.2, 4.2_

- [x] 5.2 (P) バルーンだけの書庫が今のゴーストで隠れることを記録する
  - `install/desk.rs` の `InstallKind::Balloon` の枝の後で、`balloons_for(roots, &ctx.current.ghost.dir).hidden` から `same_folder_name` で入れたフォルダ名のものを探し、在れば info `installed_balloon_hidden_by_ghost`（`folder`・`installed`・`by`）を 1 件残す。控え（`lastinstalled`）と記憶の書き方は変えない。起動の文脈が無いときは何もしない
  - `desk_record_tests.rs` に、バルーンだけの書庫は根へ入る・今のゴーストの中に同じフォルダ名が在れば記録 1 件・無ければ 0 件、を足して緑
  - _Requirements: 3.5, 7.4, 10.1_
  - _Boundary: InstallDeskNotice_
  - _Depends: 1.2_

- [ ] 5.3 (P) 根の置き場の同名と以前の同梱に触らないことを確かめる
  - `install/destination_tests.rs` に、実行ファイルの隣の根に同じフォルダ名のバルーンが在っても、ゴーストの書庫・シェルの書庫の同梱で写されず・上書きされない（`copied_root` を通らない）、以前に根へ入れた同梱は根のまま残り根の項目として候補に出る、のテストを足して緑
  - _Requirements: 6.4, 7.3, 8.2, 10.1_
  - _Boundary: install destination tests_
  - _Depends: 4.2_

- [ ] 6. 作者と利用者への説明
- [ ] 6.1 `doc/COMPAT_ARCHITECTURE.md` §8 を書き直す
  - 370 行の `balloon`・`default.balloon.path` の行: 突き合わせ先を「今のゴーストの中の `balloon\` と根の置き場の和（ゴーストの中が先）」に、「ゴーストのフォルダの中は探さない」を新しい行への参照に、「大文字と小文字を区別する完全一致」を「フォルダ名は `same_folder_name`（区別しない）・`name` は区別する」に、無いテスト名 `descript_match_is_case_sensitive` を `descript_folder_match_ignores_case_but_name_match_does_not` に、`default_balloon_path_does_not_see_ghost_inner_balloon` を 2.1 で書き換えた後のテスト名に直す
  - 343 行の飛ばした同梱の置き場の綴り `<根>/balloon/<*.directory>/` を `<ゴースト>/balloon/<*.directory>/` に直す。376 行の根の並びの目録の説明に「バルーンの候補は今のゴーストの中が先に加わる」を一言足す
  - 新しい行「【areka 独自】ゴーストのフォルダの中のバルーン」: 要件 9.1 の 8 項目・根拠の SSP 2.9 の実測（2026-10-22）と SSP に合わせない理由・定義点（`balloons_for`・`scan_ghost_balloons`・`companion_placement`・`within_body`）・1〜5 で足した判定のテスト名
  - 書いたテスト名と定義点がコードに実在することを検索で確かめ終えている
  - _Requirements: 9.1, 9.2, 9.3_
  - _Depends: 2.1, 4.2, 5.3_

- [ ] 6.2 (P) `dist/README.txt` に作者向けの説明を足す
  - メニューの「バルーン」の説明を「今のゴーストのフォルダの balloon フォルダのバルーンを先に、置き場の balloon フォルダのバルーンを後に並べた一覧」へ直す
  - 「■ 置き場」か「■ .nar の入れ方」の後に作者向けの節: ゴーストのフォルダの `balloon\<フォルダ名>\` に置けること・そのゴーストにだけ見えること・同じフォルダ名なら置き場のものより先に使われること（記憶はフォルダ名なので置き場で選んだものも替わる）・ゴーストとシェルの .nar の同梱はゴーストの中へ入ること・バルーンだけの .nar は今どおり置き場へ入ること・ukadoc に無い areka 独自の決まりで SSP では違う動きになること
  - README の該当の 2 か所が上の中身になっている
  - _Requirements: 9.4, 9.5_
  - _Boundary: Docs（README）_

- [ ] 7. 通しの確かめ
- [ ] 7.1 決定論のテストの網羅と通しの緑を確かめる
  - 要件 10.1 の分かれ目（ゴーストの中だけ・根だけ・同じフォルダ名・同じ `name`・ゴーストの直下を数えない・別のゴーストからの見え方・区切り入りの `default.balloon.path`・記憶からの復帰・読み直し・並び・バルーンだけの書庫が隠れる記録・同梱の入れ先（ゴーストの書庫・シェルの書庫）・バルーンだけの書庫の入れ先・根の同名に触らない・以前に根へ入れたものを動かさない）の 1 つずつに、1〜5 のテストのどれが当たるかを突き合わせ、欠けがあれば足す
  - `areka` の中で `catalog_roots` の `list_balloons(`・`find_balloon(` を検索し、一覧を引く 6 か所（`install/procedure.rs` の `installed_items` を含む）に根だけの口の直呼びが残っていないことを確かめる。残ってよいのは、`install/desk.rs` の `InstallKind::Balloon` の枝・インストールの宛先（`install/destination.rs`）・起動時の `survey` だけ
  - 足したテストの検体と一時フォルダが `target\` の下だけに作られていることを確かめる
  - `cargo test -p areka-ghost -p areka-nar -p areka-update -p areka` と整形の検査が緑
  - _Requirements: 6.1, 10.1, 10.2_

## Implementation Notes

- 1.1: `scan_ghost_balloons` は呼び手が来るまで `#[cfg_attr(not(test), expect(dead_code, ...))]` を付けてある。1.2 で `balloons_for` から呼ぶときに外す（外し忘れは unfulfilled_lint_expectations の警告になる）。
- 範囲外: `cargo clippy -p areka-ghost --all-targets -- -D warnings` は依存の `dola` の既存の lint（`resolve.rs`・`conflict_resolver.rs` など）で止まり、このクレートまで届かない（完了の棚卸で扱う）。
- 2.1: `cargo test -p areka` の統合テスト（`mcp_get_log_real_run`・`smoke_boot_loop_exit`）は i686 の helper が要る。ワークツリーで先に `cargo build -p shiori-host32-helper --target i686-pc-windows-msvc` と `cargo build -p shiori-host32-testdll-loadu --target i686-pc-windows-msvc` を建てる（建て済み）。テストを書く道具が `\b`・`\n` を化かすことがあるので、書いたらバイトで確かめる。
- 4.1: 入れ子の確定のテストは `install_commit_tests.rs` が 1,000 行を超えるため兄弟の `install_nested_tests.rs`（`install_commit_tests.rs` から `#[path] mod nested`）に置いた。`carve_out` の綴りは `\` 区切り（`balloon\<dir>`）でないと当たらない。
- 4.2/5.1: 4.2 の「`cargo test -p areka` が緑」は 5.1（同梱の表示名）の直しを待つため、5.1 を 4.2 と同じ変更・同じコミットで進めた。同梱の確定の失敗の検体は NTFS の大文字小文字の折りたたみ（古い `balloon\Ω` に同梱 `ω`）で本当に rename を落とす。

# 実装計画: audit-emo-present

直しの中身（何を消すか・どこへ寄せるか・触ってよいファイル）は design.md の「指摘ごとの直し」の表と「タスクの切り方」の表が正本。各タスクの境界は「タスクの切り方」の表の行で、そこに無いファイルは触らない。

タスクはすべて直列（同じファイルを複数のタスクが順に触るため・design.md「直しの順」）。どのタスクも、終わるときに `cargo test -p areka-emo-present`（範囲の外を触ったタスクは触ったクレートも）が緑で、1 本ずつコミットする（push しない）。コードを消すタスクは、消す直前に `crates/` 全体を grep して使い手が無いことを確かめ、残るテストが同じ主張を持つことを 1 件ずつタスクの記録に残す。

- [x] 1. 提示の失敗を原因ごとに分け、表示の漏斗の結果を戻り値で返す（指摘 2・4）
  - 提示の失敗に「一度も表示していない」と「表示中の絵が合成メモから追い出された」の 2 つを足し、`show_target` と読み戻しが原因どおりの失敗を返す。起こるはずのない 2 つの枝は記録を残して「装着が無い」で返したまま残す（設計の討議 議題 1）
  - 表示の漏斗が結果を戻り値で返し、応答の口へ送るのは指令の入口だけにする。見える化と拡大の更新は戻り値で成否を読み、差し替えは戻り値を捨てる（失敗は漏斗が `error!` 済み）。応答のチャンネルと届かない枝は消える
  - `presenter/refresh.rs` には `audit-wintf-graphics-world` がデバイスロストからの立ち直りの数行を足す（あちらの持ち物で引き受けない）。後に着地する方が先の形へ合わせる
  - 未表示の読み戻しが「一度も表示していない」、合成メモを捨てた後の読み戻しが「追い出された」を返すテスト 2 本と、未表示の見える化の期待の書き換えが実 GPU で緑
  - 既存の拡大の失敗と表示の失敗のテストが、失敗の値を戻り値で受けて緑
  - _Requirements: 2.1, 2.3, 2.5, 4.1, 4.2, 4.3, 6.3, 6.4, 7.2, 9.1_

- [x] 2. 登録の口から使わない引数と常に成功の結果を外し、範囲の外の呼び手まで直す（指摘 6）
  - 登録の口を引数 5 つ・戻り値なしにし、差し替えの「登録し直せなかった」枝を消す
  - クレートの中の 29 ファイルと、design.md の「範囲の外に及ぶ所」の表の 17 ファイル（`crates/areka`・`crates/areka-emo-text` の既存ファイル・`crates/pilot`）の呼び方を直し、使われなくなる手伝いは他に使い手が無ければ消す。`crates/areka-emo-text` にファイルを足さない
  - 先進坑の資産の差し替えの例は、資産が尽きると失敗するので結果を残し、World の引数だけを外す
  - 実行例 2 つの doc から「登録の口は `&mut World` で呼ぶ」の記述を外す
  - `cargo test --workspace --no-run` が通り、`areka-emo-present`・`areka`・`areka-emo-text` のテストが緑で、`.attach_target(` の呼びに World の引数が残っていない
  - _Requirements: 2.3, 2.4, 7.1, 7.2, 7.4, 9.1, 9.3_

- [x] 3. バルーンの読み込みの失敗を独自の型に分け、空の採用面列の記録を守る（指摘 3・20）
  - バルーンの読み込みの失敗を「一覧に失敗」「面 0 が見つからない」「採用面列が空」「面画像を焼けない」の 4 つの変位を持つ型で返し、クレートの外へ再輸出する。起動の配線の失敗の型と面の資産の組みを新しい型へ合わせる
  - 読み込みの失敗を「全透明の合成へ畳む」とする doc を書き直す
  - 4 つの変位に 1 本ずつテストがあり、空の採用面列の読み込みが「採用面列が空」と `error!` 1 件を、空の面列の定義の読み込みが `error!` 1 件と空の定義を返すテストが緑
  - 起動の配線の呼び手の縮退（バルーン未配線・告知・採寸の見送り）が今と同じで、`areka` のテストが緑
  - _Requirements: 2.5, 4.1, 4.2, 6.3, 7.1, 9.1, 9.2_

- [x] 4. バルーンとシェルのテストの一時フォルダを `target` の下へ寄せ、検体の写しを 1 つにする（指摘 1・26・31・32）
  - バルーンのテストの一時フォルダの中身を共通の一時パスの道具に替え、型名と口は残す。印と定義のテストの自前の一時フォルダと小物の写しを消して共通の支えを使う
  - シェルの読み込みのテストの「無いフォルダ」を `target` の下の無いパスにする（fs を触らない鍵は残す）
  - バルーンの支えの検体のパスをシェルの支えの 1 つに寄せる
  - 一時パスの番人の例外表から古びた 1 項目を消し、件数を着地の前に表の行から数え直した値に合わせ、触った行をタスクの記録に残す（`audit-ghost-catalog` が同じ表を触る）
  - 一時パスの番人 `temp_path_guard_test.rs` と範囲のテストが緑で、範囲のテストが `std::env::temp_dir` と `C:\areka-test` の下へ書かない
  - _Requirements: 4.4, 7.2, 7.5, 7.7, 9.1, 9.2_

- [x] 5. 見える・当たりを同値なら書かないようにし、毎コマの変更の印を止める（指摘 5・15・19）
  - 見える化で `Visual` と `HitTest` を、今の値と同じなら書き換えない
  - `mount.rs` は `audit-wintf-graphics-world` との申し合わせの内（後に着地する方が先の形へ合わせる）
  - 見えている装着へもう一度見せても面と文字層の両方に変更の印が立たないテスト、隠す契約のテストの 2 回目からの書き換え無しの確かめ、entity を先に消した装着への 5 つの操作がそれぞれ記録を残すテストが緑
  - _Requirements: 2.5, 4.1, 4.2, 7.2, 9.1_

- [x] 6. 外で使われていない公開の口を狭め、テストだけの語彙を `#[cfg(test)]` に、拡大の政策の正規化を 1 か所にする（指摘 7・8・42・43）
  - design.md 7 番の名前をクレートの中へ狭め、再輸出を消す。`build_shell_target_with_boxes` は可視性だけを変える（署名は `audit-parsers-sakura`・後に着地する方が合わせる）。`nesting_fixture_tests.rs:4` のコメントは触らない
  - `#[allow(dead_code)]` の 5 つをテストの時だけの定義にする
  - 拡大の政策の欄を私有にし、作者 DPI の正規化を作る口の 1 か所にして、2 つ目の正規化と迂回の檻を消す
  - `SURFACE_REASON_ALL` をテストの時だけにし、`CLICKWAIT_FAMILY` は残す（要件 9.2・3.1 ⑵）
  - `cargo test --workspace --no-run` が通り、作者 DPI 0 の正規化のテストと範囲のテストが緑で、`#[allow(dead_code)]` が範囲に残っていない（`CLICKWAIT_FAMILY` の口を除く）
  - _Requirements: 2.4, 3.1, 3.5, 7.1, 7.2, 9.1, 9.2, 9.5, 9.7, 9.8_

- [x] 7. 合成メモのテストから写しと重複を消して 1,000 行未満にし、番人の例外表から外す（指摘 9・10・11・41）
  - 自分の数えた回数を確かめるだけの 4 本と部分集合の 3 本・手伝いの写し・使われない絵の手伝いを消す（消す前に残るテストが同じ主張を持つことを 1 件ずつ記録）
  - 容量の写し 3 つを本物の容量へ替え、常に真の確かめを消す。合成メモの挿入の戻り値を無くし、表示の緩衝の口を `self` を取らない形にする
  - `cache_tests.rs` に行を足さず（どのコミットでも前より少ない）、1,000 行未満になる。切らなければ design.md の例外どおり「容量 3・LRU」の節を兄弟のファイルへ移す
  - 行数の番人の例外表から項目を消し、件数を表の行から数え直した値に、自己検査の 1 件を `frame_transition_branch_tests.rs` に替え、触った行をタスクの記録に残す
  - 行数の番人 `file_length_guard_test.rs` と範囲のテストが緑で、`cache_tests.rs` が 1,000 行未満
  - _Requirements: 2.4, 6.1, 6.2, 7.5, 7.7, 9.1, 9.4_

- [x] 8. 落ちようがない確かめと部分集合の組・誤ったうたいを消す（指摘 12・16・17）
  - research.md 12 番の全部の場所の確かめを消し、網羅の `match` で守れるものは `match` だけ残す。指令の型のテストは送れることと HRESULT の表示の 2 本を残す
  - 拡大のテストの 3 つ目の組と表示のテストの「行の幅は本物の stride」のうたいを消す
  - 消したものごとに残るテストが同じ主張を持つことが記録にあり、範囲のテストが緑
  - _Requirements: 2.1, 2.4, 9.1_

- [x] 9. 提示のテストの抜けを埋める（指摘 13・14・18・22・23・25）
  - まばたきの停止のテストがいちから合成を通り、合成メモの容量のテストの負の対照が別の面で絵が変わることを確かめる
  - 当たりの資源を外した面の再表示が `warn!` 1 件を残す、隠した後と未表示・未登録の最後の組の読み、合成メモを捨てた後の最後の表示の読み、同じコマで面を替えた部分合成がいちからの合成と同じ絵になる、の各テストを足す
  - 足したテストがどれも実 GPU で決定論に緑
  - _Requirements: 4.1, 4.2, 4.3, 9.1_

- [x] 10. 読み込みのテストの抜けを埋め、矢印の族の写しを消す（指摘 21・24・27）
  - 無いフォルダの印の対の解決が `warn!` 1 件と既定の対を返すテスト、1 回目に取り出した面の表から面を 1 つ合成して外形が 0 でないことの確かめを足す（取り出しの 1 回目の結び付けを外すと赤になることを実装のときに確かめる）
  - 矢印の族の写しと古い文を消して本物の表を使う
  - 範囲のテストが緑
  - _Requirements: 4.1, 4.2, 4.4, 9.1_

- [ ] 11. 部分集合・写しのテスト 15 か所を消し、静かな読み込みの照合を 1 回の読み込みにする（指摘 28・34）
  - research.md 28 番の 15 か所を消し、一覧の失敗のパスの確かめだけは残る `matches!` へ移す（消す前に残るテストが同じ主張を持つことを 1 件ずつ記録）
  - 静かな読み込みの照合が探す文言の列を受け、1 回の読み込みで済む
  - 範囲のテストが緑で、消した 15 か所が記録と一致する
  - _Requirements: 2.4, 9.1_

- [ ] 12. 提示のテストの応答の手伝いを支えに 1 つずつ置き、写しを消す（指摘 29 の前半）
  - 表示・隠す・捨てる・装置の失敗の注入の手伝いを提示のテストの支えに 1 つずつ置き、research.md 29 番の該当ファイルの写しを消す
  - 支えのファイルが 1,000 行未満のまま、範囲のテストが緑で、写しが残っていない
  - _Requirements: 2.3, 6.1, 9.1_

- [ ] 13. 提示のテストの絵と小物の手伝いを支えに 1 つずつ置き、写しを消す（指摘 29 の後半）
  - 絵の生成・焼き・組み、配置と面の entity の取り出し、当たりの数え、記録の文言の定数、遷移の記録のテストの登録の写しを支えに寄せる
  - 支えのファイルが 1,000 行未満のまま、範囲のテストが緑で、写しが残っていない
  - _Requirements: 2.3, 6.1, 9.1_

- [ ] 14. シェルとバルーンのテストの手伝いを支えへ寄せ、渡すだけの手伝いを消す（指摘 30・33）
  - シェルの読み込みのテストの手伝い・検体のパスと絵の名前の一覧をシェルの支えへ寄せ、古い数と一覧の doc を消す
  - 渡すだけの手伝い 8 つを消して呼び元へ直に書く
  - 範囲のテストが緑で、写しと渡すだけの手伝いが残っていない
  - _Requirements: 2.3, 9.1_

- [ ] 15. バルーンの定義の読み込みで、フォルダの一覧と `descript.txt` を 1 つの入口の中で 1 回だけ取る（指摘 40）
  - 定義の読み込みが面のループの前に descript を 1 回読み、印の解決へフォルダの一覧を 1 回取って渡す。今の公開の口は包みとして残し、範囲の外の呼び手は触らない。シェルの解析の回数は触らない
  - `descript.txt` の無いバルーンで上書きを持つ面が 2 つのとき、descript が読めない `warn!` が 1 件だけのテストが緑
  - _Requirements: 4.1, 4.2, 7.2, 9.1, 9.6_

- [ ] 16. 古いコメントと記録の文言を直し、dev 依存の重複と小さな抜けを片づける（指摘 35〜39）
  - 本番とテストの古いコメント・数・タスク番号を直す。装置の失敗の記録の文言を `D2D 呼び出しが失敗` に変え、照合する 4 つのテストも同じ文言にする。テストの文脈の `"CreateSwapChain"` を今の呼び出しの名前へ替える
  - dev 依存の `windows` の重複 1 行と誤ったコメントを消し、`cargo build -p areka-emo-present --tests` の後に `git diff --exit-code -- Cargo.lock` が差分 0 であることを記録に残す
  - 合成の鍵の hash に未知の合成メソッドの名前を混ぜ、名前の違う未知のメソッドは違う鍵・同じ名前は同じ鍵になるテストを足す
  - perf の行のコマの番号の出どころを 1 つにし、使われなくなる関数とその単体テストを消す
  - 範囲のテストが緑で、`Cargo.lock` に差分が無い
  - _Requirements: 4.1, 4.2, 6.5, 6.6, 9.1_

- [ ] 17. 同じ範囲へ audit をかけ直して新しい指摘 0 件にし、全体テストを緑にする（検証）
  - `crates/areka-emo-present/` へ要件 1 と同じ作法（最大 20 件を外す・読まなかった所を残さない）で `/ponytail:ponytail-audit` をかけ直す。出た指摘はその指摘のファイルを境界に足して直し、0 件になるまで繰り返す。残してよいのは `CLICKWAIT_FAMILY` だけ
  - 新しく直さずに残す候補が出たら、ukadoc で確かめたうえで開発者の裁定を仰ぐ
  - 直した行に `doc/`・`docs/` を名指す行の追加・変更が無く、`crates/areka-emo-text/` に新しいファイルが無く、新しい機能・`vendors/` の変更が無いことを差分で確かめる
  - `pwsh -NoProfile -File tools/test-all.ps1` を 1 回回して緑。文字化けが出たら読む側を直す
  - かけ直しの結果（新しい指摘 0 件・残した `CLICKWAIT_FAMILY`）と全体テストの結果を検証の記録に残す
  - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5, 2.1, 2.2, 3.2, 3.3, 3.4, 4.5, 4.6, 5.1, 5.2, 5.3, 5.4, 6.4, 7.3, 7.6, 8.1, 8.2, 8.3, 8.4, 8.5_

## Implementation Notes
- タスク 1: 境界の外の `presenter/transition_record_tests.rs` を触った。`show.rs` の失敗の枝の字面を固定するテストがあり、`return Err(e);` へ照合の文字列と範囲（9→8 行）を合わせた（主張は同じ）。字面を固定するテストは、`show.rs` を触る後のタスクでも合わせ直しが要る
- `audit-emo-compose` との申し合わせ（11-05）: `ComposeError::ExtentTooLarge` を表示の側では `EmptyComposition` と同じ「面を隠す」縮退で扱う。後に着地する方が表示の側の `match` に腕を足す。`hit_region` が消えた main を取り込んだら `presenter_read_accessor_tests.rs` の説明 2 行（今の 373・383 行）を直す
- `audit-parsers-sakura` は `shell_target.rs` に記録の数行を足す（約 975 行の見込み・新しいファイルは足さない）。タスク 6 の可視性の変更とは署名で当たらない
- タスク 2: `cargo test -p areka` の `smoke_boot_loop_exit`・`mcp_get_log_real_run` は作業場に i686 の検体（`shiori-host32-helper.exe`・`shiori_loadu.dll`）が無いと赤。`cargo build -p shiori-host32-helper -p shiori-host32-testdll-loadu --target i686-pc-windows-msvc` で建ててから回す
- タスク 4（`audit-ghost-catalog` と同じ表）: `crates/log-capture-kit/tests/temp_path_guard_test.rs` で触った行は次の 3 つ。例外表から `crates/areka-emo-present/src/balloon_test_support.rs` の `Why::ProcessUnique` の項目 5 行（元の 155〜159 行）を消した。`ALLOWED_COUNT` を 19→18（今の 278 行）、`PROCESS_UNIQUE_COUNT` を 16→15（今の 284 行）にした。どちらも表の行から数え直した値（ProcessUnique 15・ReadOnly 2・IntentionallyFixed 1）。節 ⒜ の見出しの「13 件」は前から実数と合わないが、要件 7.7 に従って触っていない。後に着地する方は、表の行からもう一度数え直す
- タスク 4: `C:\areka-test` の綴りが 3 つ残る（`shell_target_load_tests.rs`・`shell_target_base_image_tests.rs`・`shell_target_element_base_tests.rs`）。どれも `MemoryDecoder` の鍵で fs を触らない（指摘 26 で残すと決めた分）
- タスク 6（設計との違い）: `shell_target.rs` の `build_shell_target`・`build_shell_target_with_boxes` は `pub(crate)` に狭めると本番の呼び手が無くなる（本番は `build_core` を直に呼ぶ）ので、可視性に加えて `#[cfg(test)]` を 1 行ずつ足した（951→953 行・署名は不変）。`#[allow(dead_code)]` は完了条件が許さず、テストの呼び先を `build_core` へ移すと `audit-parsers-sakura` の署名の直しと当たるため
- タスク 6: 消したテストの主張の行き先。`derive_scale_zero_author_dpi_emits_warn_log` → `scale_policy_new_zero_author_dpi_emits_warn_log`（warn は `new` へ移った）と `derive_scale_normalizes_zero_author_dpi`（k の同値）。`derive_scale_emits_each_degradation_log_independently` の前半 → `derive_scale_missing_dpi_emits_error_log`（error 1 本・k=ONE）と同じテストの後半（縮退ごとに独立）。`classify(..).author_dpi_normalized` の確かめ → 欄ごと消え、`author_dpi()` の確かめが持つ
- タスク 6 の申し送り（タスク 16 で直す）: 狭めた名前を指す doc のリンクの rustdoc 警告が 7 本ほど出る（`lib.rs:28` の `cache`、`balloon.rs:357・400` の `select_faces`、`shell_target.rs` の 5・8・20・22・202・354・542 行、`presenter.rs:3` の `ComposeCache`）。括弧を外してただの `` `名前` `` にする
- タスク 7: `cache_tests.rs` は 1,301→959 行。消したテストの主張の行き先。`miss_computes_once_hit_does_not_recompute` → `presenter_cache_capacity_tests::showing_a_face_from_a_few_frames_ago_hits_instead_of_recomposing`。`dynamic_show_reissue_different_binds_recomposes` → `presenter_compose_input_tests::bind_change_on_same_surface_updates_display`（キーの構造は `cache_tests::different_binds_on_same_surface_must_miss`）。`dynamic_show_reissue_same_binds_returns_cached` → `showing_a_face_from_a_few_frames_ago_hits_instead_of_recomposing` と `the_real_apply_path_retains_exactly_the_approved_number_of_entries`。`invalidate_all_forces_recompute` → `cache_tests::invalidate_all_clears_every_entry` と `budget_tests::a_failed_recycle_is_counted_never_swallowed`。`invalidate_all_clears_regardless_of_pattern` → `invalidate_all_clears_every_entry`。`different_surface_ids_coexist_until_the_capacity_is_exceeded` → `three_distinct_keys_are_retained_at_the_same_time` と `a_fourth_distinct_key_evicts_exactly_one_entry`。`take_recycled_yields_one_entry_only_when_the_cache_is_full` → `take_recycled_without_a_full_table_returns_none_and_leaves_cache_usable`（1 件・2 件とも None）と `insert_after_take_recycled_preserves_approved_semantics`
- タスク 7（行数の番人の表）: `crates/log-capture-kit/tests/file_length_guard_test.rs` で触った行は次の 3 つ。`cache_tests.rs` の項目 4 行（元の 66〜69 行）を消した。`OVER_LIMIT_ALLOWED_COUNT` を 9→8（今の 97 行・表の項目から数え直した値）にした。`CALIBRATION_DROPPED_ENTRY` を `crates/areka/src/emo2_boot/frame_transition_branch_tests.rs` に替えた（今の 100〜101 行）。テスト名 `the_rotating_face_fixture_differs_in_bytes_and_outnumbers_the_cache` は、常に真の確かめを消したので `the_rotating_face_fixture_differs_in_bytes` に改めた
- タスク 8: 消したものと主張の行き先（空の主張はその理由）。`command.rs` の `variants_construct_with_owned_data`・`target_id_is_copy_and_hashable_key`・`present_error_maps_compose_error_via_from` → 型と derive・`#[from]` がコンパイル時に保証（空）。所有は `present_command_is_send_static` が持つ。`ALL_DISPLAY_FAULTS.len()==4`・`ALL_FAULTS.len()==4`・`every_allocation_site_is_enumerated` → 配列の型 `[_; N]` が長さを決める（空）。網羅は `fault_marker`・`DisplayFault` の `match`・`BudgetDelta::count` の `match` が持つ。snapshot ⑸ `compose_alone_leaves_display_state_unchanged` と ⑺ の前後比較 → 照会はどれも `&self` で、内部可変は無い（空）。⑸ は欠番のまま。`shell_target_base_image_tests.rs` の重なり 0 件と和 → 直前の `used==[0,1,5]`・`shadowed==[2]` から導ける（空）。fractional の 5 か所と旧 574 行の 1 か所（research に無いが同じ形） → 直前の値の一致から導ける（空）。`balloon_series_tests.rs:15-19` の `.all` → `prefix_chain_scope0_is_canonical_then_legacy_all_own`。dpi の 3 つ目の組 → `native_size_is_supplied_on_a_cache_hit_established_show`。display の「実 stride」のうたい → 確かめられないことを doc に 1 文足し、pitch==stride の確かめは残した
- タスク 8 の申し送り（タスク 16 で直す）: `presenter/budget.rs:143-150` の doc の「『2 つある』ことごと固定する走査」は、件数の確かめを消したので言い回しが少し合わない

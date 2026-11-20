# Implementation Plan

設計の「タスクの束と順」の T1〜T27 を、そのまま 27 本のタスクにする（案 C・確定 1）。指摘の番号は `research.md` の 2 節の 1〜114、直し方は design.md の「指摘ごとの直し方」の表が正本。行番号は `research.md` の時点のものなので、実装では関数名から引き直す。

すべてのタスクに共通の完了の条件:
- 消すテスト・補助・定数・名は、消す前にツリー全体（テスト・支え・examples・fixture・設定・スキル・手順書・steering・文字列での参照・`#[path]`・テストの名の文字列）で使われていないことを grep で確かめる（要件 3.3）。
- 形を変えた項目の名をツリー全体で grep し、呼び出し元・テスト・支えまで直し切る（要件 3.2）。
- 説明を直す指摘は、書き直す前にそのコメントがコードの言い直しかを見て、言い直しなら消し、コードだけでは分からない理由だけを残す。範囲の外と、この spec が触らないコメントは掃除しない（design.md の物差し）。
- C23 で触らないファイル（`crates/areka-placement/`・`hit_region.rs`・本体の `input_events/`・`main.rs`・`ghost_session*.rs`・`app_exit.rs`・`menu/`・`Cargo.toml`・`Cargo.lock`・`log-capture-kit` の `event.rs`・`doc/COMPAT_ARCHITECTURE.md` §8・`vendors/`）に差分を作らない。
- 触った crate のテストを、赤が出たら赤のテストだけに絞って回し、緑で終える（要件 8.4）。GPU のテストを何度も回さない。

- [ ] 1. 本番のバグと危ない所を、縛るテストと一緒に直す

- [x] 1.1 文字の拡大率のテストの落ちない表明を直す（T1・1・7 番）
  - 観測器の結果を空にすると落ちることを先に見てから、結果の取り出しに表明を付ける（1 番）
  - 空撃ちを消し、2 回目も本番の経路で 1 窓だけ DPI を変えて、拾った窓がその 1 窓だけであることを見る（7 番）
  - 完了: `frame_text_scale_tests.rs` の 2 つの表明が、壊した状態で赤・直した状態で緑になることを確かめてある
  - _Dispatch: mechanical — 直しの形を設計が与える_
  - _Requirements: 3.1, 4.1, 4.2, 4.3_

- [x] 1.2 (P) 位置の指令の預かりの判定を、預かりの件数に比例する手間にする（T2・2 番）
  - 預かりの窓の集合を、同じ窓を何度足しても 1 つにしか数えない集合に替える。預かる・順を追い越さない・取りこぼさない・並べ直しが前の移動を捨てる振る舞いは変えない
  - 新しい子のテスト `blocked_keeps_one_entry_per_window_however_many_cues_wait` を、直しの前に 1,000 で赤になることを見てから立てる
  - 列が伸び続ける件はこのタスクで扱わない（T25 で `window-resetwindowpos-hold` へ渡す）
  - 完了: 新しいテストと `frame_move_hold_tests.rs` の今のテストが全部緑
  - _Boundary: frame/move_hold, frame/move_hold_tests（新）_
  - _Dispatch: mechanical — 集合の型とテストの期待値を設計が与える_
  - _Requirements: 3.1, 3.5, 4.1, 4.2, 4.3_

- [x] 1.3 装着で繋げなかった文字層を、配置先ができた最初のフレームに繋ぎ直す（T3・3 番）
  - 結線に「装着で文字層を繋げなかったスコープと面 0 の背景色」の私有の控えを足し、結線を作るたびに空にする
  - 装着の相で文字の配置先が無かったスコープを控えに入れる（今の `warn!` 1 行は変えない）
  - 文字の拡大率の相で、配置先が取れたスコープの控えを取り出して背景を渡して繋ぎ、`info!` を 1 行残す。同じフレームで繋ぐ（遅らせない）
  - 実装の順は、控えと装着の側だけを入れて縛るテスト `a_balloon_text_left_unconnected_at_attach_connects_on_the_first_frame_its_view_exists`（実 GPU）が赤になるのを見る → 相の側を入れて緑
  - 装着の側の説明を「繋ぎ直しは文字の拡大率の相が行う」に合わせる
  - 完了: 縛るテストが緑で、actor が登録され、控えが空になり、`info!` が 1 行出ることを表明している
  - _Depends: 1.1_
  - _Dispatch: standard — 相を跨ぐ新しい分岐を組む_
  - _Requirements: 3.1, 3.4, 4.1, 4.2, 4.6, 6.2, 6.4_

- [x] 1.4 窓の側の scope の上限を 1 つの関数にし、範囲の中の 9 か所を通す（T4・4 番・件 9）
  - `target_map` に上限の定数を 1 つ置き、窓の側の入口 `window_scope` を足す。上限を越えたら自分で `warn!` を 1 行残して `None` を返す
  - 9 か所（`dpi.rs`・`scale_text.rs`・`attach.rs` の 2 か所・`drain_resnap.rs` の 2 か所・`move_shift.rs`・`switch.rs`・`zorder_drain.rs`）を設計の表のとおりに替える。`dpi.rs` と `scale_text.rs` の `error!` は消す。連鎖の確定の 1 か所は「まだ表示していない」で見送る
  - 純関数のテスト `window_scope_rejects_scopes_past_i32_max` と、ハーネスのテスト `a_window_scope_past_i32_max_is_skipped_with_one_warning`（直しの前はデバッグのビルドで panic＝赤）を立てる
  - 範囲の外の `balloon_visibility_phase.rs` は触らない
  - 完了: 2 本の縛るテストが緑で、上限を越える窓への書き込みが 0・`warn!` が 1 行
  - _Depends: 1.3_
  - _Dispatch: standard — 9 か所の置き換えと赤から立てるハーネスのテスト_
  - _Requirements: 3.1, 3.2, 3.4, 4.1, 4.2, 4.3, 5.10, 6.2_

- [x] 1.5 使わない import と公開の面を狭める（T5・5・114 番・件 4・件 5）
  - 3 つのビルド（lib・lib＋test-support・test）の使い方で import を 3 種に分け、どのビルドでも使わない名を消し、テストだけの名と両方の名を設計の `cfg` で包む。`unused_imports` の許可を 0 本にする
  - 設計の挙げる 8 つの項目の公開を、私有または `pub(super)` へ狭める。外の読み手が 0 件であることを grep で確かめる
  - 完了: 3 つのビルドで `--force-warn unused_imports` を 1 回ずつ回し、警告 0 件を記録に残してある
  - _Depends: 1.4_
  - _Dispatch: standard — 3 つのビルドを測って分ける_
  - _Requirements: 3.1, 3.2, 5.4, 5.5, 7.6_

- [x] 1.6 札が外れるフレームの書き込みのテストを先に立て、DPI の変化の問いを 1 つにする（T6・6・50 番・件 13）
  - 決定論のハーネスで、札の付いた窓がシェルの切り替えを跨ぎ、札が外れるフレームにちょうど 1 回 `DpiReproject` で書かれ、札の無い窓は揃え方の変化で動くことを確かめるテスト `a_window_held_through_a_shell_switch_is_written_on_the_frame_its_hold_lifts` を立てる
  - その後で DPI の変化の問いを「DPI が変わった、または札が付いている」の 1 つにして和集合の行を消す
  - 完了: 問いを書き損じると新しいテストが赤になることを確かめ、直した後は緑
  - _Depends: 1.4_
  - _Dispatch: standard — 手順を設計が与えるが組み立てが長い_
  - _Requirements: 3.1, 4.1, 4.3, 5.15_

- [x] 1.7 作業領域の源の同期を、モニターの表が変わったフレームだけにする（T7・48・49 番）
  - World の私有の資源でモニターの変化と数と、源の資源の入れ直しを見張り、どれも変わらないフレームでは作り直さない。初回は必ず作り直す。同期の関数の署名は変えない
  - 資源が `Send + Sync` を満たさなければ同じ形を非 Send の資源で置く
  - 源を手で置き替えて同期に上書きさせるテストがあれば、モニターの component を書き替える形に改め、本数を記録に書く
  - 縛るテスト `an_unchanged_monitor_table_is_not_rebuilt_and_an_empty_one_warns_once`（直しの前は `warn!` 3 行＝赤）と `a_reinserted_monitor_snapshot_is_resynced_on_the_next_call` を立てる
  - `frame.rs` の「定常フレームは無操作」の文と、例外の説明（48 番）を事実に合わせる
  - 完了: 2 本の縛るテストと `frame_work_area_sync_tests.rs` の今のテストが緑で、モニター 0 台の `warn!` が表の変わったフレームに 1 回だけ出る
  - _Depends: 1.5_
  - _Dispatch: standard, effort high — ECS の変化の検出の取りこぼしに注意が要る_
  - _Requirements: 3.1, 3.4, 4.1, 4.2, 4.3, 4.7, 6.4_

- [ ] 2. ほかと重ならないテストの直し

- [x] 2.1 (P) 登録と終わる相のテストを本番の入口へ向ける（T8・8・9・10・57 番の schedule の分）
  - 登録の写しを消して本番の登録の関数を呼ぶ（8 番）。終わる相のテストは本番の結線の関数を直に呼び、字面の確かめを消す。置き場は今のまま（9 番）。`ghost_quit.rs` の説明を直す
  - 範囲の中の重なりの字面の照合を消す（10 番・範囲の外は触らない）。両側の `frame_schedule_tests.rs` の見出しを中身に合わせる（57 番）
  - 完了: 両側の `frame_schedule_tests.rs` と本体の終わる相のテストが緑
  - _Boundary: frame_schedule_tests（両側）, session/ghost_quit_
  - _Dispatch: mechanical — 呼ぶ関数と消す行を設計が与える_
  - _Requirements: 3.1, 3.3, 4.7, 5.6, 6.3_

- [x] 2.2 (P) 確定と装着の空振りの表明を強める（T9・11・12・13 番）
  - 隙間を読んだ位置と寸から出す（11 番）。動いた原点が元の値でないことを表明する（12 番）。装着の空振りで actor が無いことと WARN 1 行を捕まえて表明する（13 番）
  - 完了: 3 つの表明が、壊した状態で赤・直した状態で緑
  - _Boundary: frame_chain_finalize_tests, frame_attach_tests_
  - _Dispatch: mechanical — 表明の形を設計が与える_
  - _Requirements: 3.1, 4.1, 4.7_

- [x] 2.3 (P) 本体の箱の檻を強める（T11・15・16・21 番）
  - 2 段で向きを見る（15 番）。押した真と、lifecycle と kanade の受け手への中断の到着を表明する（16 番）。結線を `Option` と `take()` で持つ（21 番・前例のとおり）
  - 完了: 本体の `frame_shell_box_*_tests.rs` が実 GPU で緑
  - _Boundary: 本体 frame_shell_box_*_tests_
  - _Dispatch: standard — 実 GPU のテストの表明を組み直す_
  - _Requirements: 3.1, 4.1, 4.6, 4.7_

- [x] 2.4 (P) 本体の待ちと join を目印つきの待ちにする（T12・18・19・20 番）
  - seriko の閉鎖と停止通知の到着を、名前つきの有界の待ちにし、自前の上限の定数を消す（18・19 番）
  - アクターの join を有界の join にして、panic を捨てない（20 番）
  - 完了: 本体の `frame/switch_tests.rs`・`switch_finish_tests.rs`・`ghost_quit_logsink_tests.rs` が緑で、`let _ =` の join が範囲の中に 0 件
  - _Boundary: 本体 frame/switch*_tests, session/ghost_quit_logsink_tests_
  - _Dispatch: mechanical — 置き換える待ちの関数を設計が与える_
  - _Requirements: 3.1, 6.2, 6.3_

- [ ] 3. テストの支えを寄せる（ほかの指摘の土台）

- [x] 3.1 写しを既にある支えへ寄せ、GPU の World の組み立てを 1 つにする（T13・22〜25・30・31 番・件 11）
  - 記録の捕捉・コードだけの読み・時計の操り・重なりの支えの写しを、既にある支えから引く形にする（22〜25 番）
  - 範囲の中の偽の HWND の 5 か所を placement の支えの関数にし、私有の写しを消す（30 番・範囲の外の 2 か所は T25 で渡す）
  - GPU を入れる補助 `gpu_on` を足し、COM の初期化の結果を確かめる（`S_OK`／`S_FALSE` のほかは止める）。3 か所（装着の World・バルーンの支え・本体の GPU の World）から呼ぶ（31 番）
  - 完了: 写しの関数が範囲の中に残らず、触った支えを使うテストが緑
  - 25 番で `frame/zorder_drain.rs` の宣言を変える（1.4 と同じファイル）
  - _Depends: 1.4, 2.2_
  - _Dispatch: standard — 支えを跨いで寄せる_
  - _Requirements: 3.1, 3.2, 3.3, 4.6, 5.12, 6.2_

- [x] 3.2 切り替えのテストを同じ World で回す（T10・14・107 番・セ）
  - 1 つの GPU の World で前のゴーストを回した後、本番と同じ窓の畳み方で畳み、窓を開け直して次のゴーストを据え直して回す。GPU の許可は 1 つにする
  - 見出しの行番号を外して関数名で指し、相の数を書かず、節の番号を詰める（107 番）
  - 範囲の外の `spine_wait.rs` の 2 行の説明を事実に合わせる（セ）
  - 完了: 本体の `frame_visibility_integration_tests.rs` が許可 1 つで緑
  - _Depends: 3.1_
  - _Dispatch: standard — 本番の切り替えの手順をテストに組む_
  - _Requirements: 3.1, 3.2, 4.6, 4.7, 6.3_

- [x] 3.3 結線の手組みを 1 つの補助に寄せる（T14・26 番）
  - 共有の支えに、受け口を既定で埋める結線の補助を 1 つ置き、今の 2 つの補助と 16 か所をそこへ寄せる
  - 完了: 結線を手で組む箇所が範囲の中に残らず、寄せた 16 か所のテストが緑
  - _Depends: 3.1_
  - _Dispatch: standard — 16 か所を 1 つの形へ寄せる_
  - _Requirements: 3.1, 3.2_

- [x] 3.4 鎖と移動のテストの写しを共有の支えへ寄せる（T15・27・28 番）
  - research の直し方どおりに写しを消して共有の支えから引く。arm の私有の補助は別名にし、共有の支えの頭の使い手の説明を直す（27 番）
  - 3 体の鎖の支え 5 つを共有の支えへ移す（28 番）
  - 完了: 共有の支えが 900 行を越えず、`frame_chain_realign*`・`frame_default_pos_track_tests.rs`・`frame_move_hold_tests.rs`・`frame_resnap_tests.rs` が緑
  - _Depends: 3.3_
  - _Dispatch: standard — 支えを跨いで寄せる_
  - _Requirements: 3.1, 3.2, 6.1_

- [ ] 3.5 書き込みの補助と整地を新しい支えへ寄せる（T16・29 番・ケ）
  - 新しい `cfg(test)` の支え `frame_transition_test_support.rs` に、書き込みの抜き出し・強い方の整地・拡大・警告の数え・オフセットの読み・バルーンの寸・拡大率の定数を 1 つずつ置き、9 本から写しを消す。再スナップの配置の支えもその定数を使う
  - 原子性のテストの手写しの述語を、placement の判定の要約を読む形にする（ケ・範囲の外の判定のファイルは変えない）
  - 完了: 9 本の写しが消え、その 9 本と原子性のテストが緑
  - _Depends: 1.6, 3.4_
  - _Dispatch: standard — 9 本の写しを 1 つの支えへ寄せる_
  - _Requirements: 3.1, 3.2, 3.3_

- [ ] 4. 本体のテストを `areka-runtime` へ移し、上限越えのファイルを分ける

- [ ] 4.1 一時フォルダの形を直し、規則・揃え・clamp ⑴ を移す（T17・32・33 番の その 1）
  - 最小のゴーストを `TempPath` で包み、自前の採番・消去・`Drop` を消す（32 番・根はワークツリーの `target\` の下）
  - 保存の支えの大半を `areka-runtime` の `frame::persist_test_support` へ移す（`git mv`）。dev-dependencies を使わず、本番の継ぎ目の包みを直の資源の挿入にし、ゴーストのフォルダをパスで受ける。本体には最小のゴーストだけの同名のファイルを残す
  - 規則と揃えのテストを `git mv` で移し、clamp ⑴ を `areka-runtime` の確定のテストへ移す。両側の `frame.rs` の宣言を付け替える
  - 完了: 移した先（`cargo test -p areka-runtime`）で同じ名前のテストが同じ数だけ緑になり、本体の側から消えた数と合うことを記録に書いてある。`Cargo.toml` の差分は 0
  - _Depends: 3.4_
  - _Dispatch: standard — 2 つの crate を跨いで支えを移す_
  - _Requirements: 3.1, 3.2, 3.7, 4.4, 4.5, 6.2, 6.6, 6.7_

- [ ] 4.2 覚えた位置と窓の状態のテストを移し、保存の支えを直す（T18・17・33 番の その 2・99・106・110 番・件 10）
  - 覚えた隣り合いからの起動の補助を置き、3 か所で使う（17 番）
  - remembered ⑶⑷⑸ を新しい `areka-runtime` の remembered のテストへ移す。材料と起動の組み立ての半分を共有の支えへ置き、記録の捕捉の数行は両側に残す（33 番）
  - 窓の状態のテスト 1 本を `areka-runtime` へ移し、フックの導入を消し、絞り込みをテストの中に書く（99 番・本体の残り 4 本の補助は本体に残す）
  - 保存の支えに、書き終えた後にファイルを読む口を置き、メモリを読む口を消す（106 番）。重なるテストの前者を消し、記録の確かめを揃えの 1 本へ寄せる（110 番）
  - 完了: 移した先で同じ名前のテストが同じ数だけ緑になり、本体の側から消えた数と合うことを記録に書いてある
  - _Depends: 4.1_
  - _Dispatch: standard — 2 つの crate を跨いでテストと支えを移す_
  - _Requirements: 3.1, 3.2, 3.3, 3.7, 4.4, 5.11, 6.6, 6.7_

- [ ] 4.3 整合待ちの重なりを消し、上限越えの分岐のテストを 2 本に分ける（T19・34・35・84・94 番）
  - 整合待ちのテストの重なる 4 本を消し、向こうに無い 4 本を残し、表と見出しを直す（34 番）
  - 分岐のテストを、待ちの群の `frame_transition_branch_wait_tests.rs`（新）と残りに分け、共通の補助を T16 の支えへ寄せ、宣言を 2 本にし、対テストの持ち主の表を 2 本の名で直す（35 番）
  - 5 本目の綴りの確かめを足す（84 番・件 14）。同じ本体を 3 つの DPI で回す 1 本にし、観測の条件のテストを支えの側に 1 本にする（94 番）
  - `file_length_guard_test.rs` の上限越えの表から 1 行消し、件数を 3 から 2 にする（C23 の約束 ⑸ でこの spec だけが触る）
  - 完了: 分けた 2 本がどちらも 1,000 行を下回り、1,000 行の番人が件数 2 で緑。分ける前より長いファイルが無い
  - _Depends: 3.5_
  - _Dispatch: standard — 行を数えながら分けて重なりを消す_
  - _Requirements: 2.7, 3.1, 3.3, 4.7, 5.16, 6.1, 7.1, 7.4_

- [ ] 5. 残りの指摘をファイルのまとまりで直す

- [ ] 5.1 本番の説明と小さな形を直す（T20・36〜47・51〜53・113 番）
  - 相の並びの説明を 20 の相の並びに合わせ、テストの側は「各相」にする（36 番）。37〜40・42〜47・53 番は設計の表の文のとおりに直す（件 1・件 3・件 6・件 8）。41 番の本番の分は、spec の名や task の番号を指すだけの行を物差しで 1 行ずつ見る（理由を持たなければ消す）
  - 書く前の寸と書いた後の寸を比べて、寸の変化の補助と arm の単体のテストを消す（51 番）。重なりの計画の頭で空の台帳を早く返す（52 番）
  - 取り出しの上限の `warn!` の文を「上限に達した（残りがあれば次のフレームで到着順に適用）」にし、縛るテスト `exactly_the_cap_is_applied_in_one_frame_and_the_next_frame_has_nothing` を立てる（113 番）
  - 完了: 縛るテストが緑で、`frame.rs` と `frame/*.rs` の説明が物差しのとおりに直してある
  - _Depends: 1.7, 4.3_
  - _Dispatch: mechanical — 文と形を設計の表が与える_
  - _Requirements: 3.1, 3.3, 4.1, 4.3, 5.1, 5.3, 5.7, 5.9, 5.16_

- [ ] 5.2 `frame/` の下のテストを直す（T21・54〜60 番・57 番の `frame/` の分）
  - 字面の見張りの対象を 2 本にする（54 番）。適用の閉包に上限の表明を足す（55 番）。56・58 番は設計の文のとおり（件 12）
  - 両側の見出しを自分の持つテストだけにし、もう一方を 1 行で指す（57 番）
  - 束なしの結線で面の書き手が `error` 1 行を出し表が空のままであることのテストを足す（59 番）。窓を全部集める補助を支えに置き両方から呼ぶ（60 番）
  - 完了: `frame/` の下のテストが緑で、足したテスト（59 番）が緑
  - _Depends: 3.1_
  - _Dispatch: mechanical — 直し方を設計の表が与える_
  - _Requirements: 2.7, 3.1, 4.1, 5.6, 5.13_

- [ ] 5.3 `frame_*` のテストを直す その 1（T22・61〜78 番・41 番の `frame_attach*`・`frame_balloon_offset_*`・`frame_diag_route_tests.rs`・`frame_harness_tests.rs`・支えの分）
  - 消すテスト（62・63・64・73・78 番）は、残るテストが同じ中身を守ることを 1 本ずつ確かめてから消す。名指す範囲の外の説明 2 行（`move_cue_apply_move_tests.rs`・`assets.rs`）も直す
  - 網羅の檻を、各腕が定数の表の添字を返して一致を主張する形に強める（67 番）。探針の表明を 1 本にまとめる（68 番）
  - 手動なら自動へ戻すことを doc に書き補助を支えへ移す（70 番）。ハーネスの再スナップの口を畳み名指しを直す（71 番）。x64 の見張りを宣言の `cfg` に寄せ、支えの `const` の表明と字面の見張りを消す（72 番）
  - 61・65・66・69・74〜77 番は設計の表の文のとおり（件 14）。41 番は spec の名や task の番号を指すだけの行を物差しで 1 行ずつ見る
  - 完了: 触った `frame_*` のテストが緑で、消したテストの名がツリーに残っていない
  - _Depends: 3.3, 3.5_
  - _Dispatch: mechanical — 直し方を設計の表が与える_
  - _Requirements: 2.7, 3.1, 3.2, 3.3, 5.16_

- [ ] 5.4 `frame_*` のテストを直す その 2（T23・79〜83・85〜93・95〜97 番・41 番の `frame_chain_realign_*`・`frame_dpi*`・`frame_resnap_tests.rs`・`frame_move_order_tests.rs`・`frame_work_area_resnap*`・`frame_transition_*` の分）
  - 条件に合う行を数えて 1 と表明する（79 番）。陽性の対の呼び出しを 1 本に残す（80 番）。同寸と異寸を 1 本ずつにし、書かないことを書き込みの無さで見る（87 番）。差と長さを上限以下で見て、窓あたり 1 本以上の陽性の対を足す（97 番）
  - `debug_assert!` に依るテストに `--release` で `ignore` になる印を付け、警告の文の手写しが在ることを陽性の対で数える（92 番・件 7）
  - 見出しを行番号でなく関数名で指し、語の数を 13 に直す（93 番・件 2）
  - 81〜83・85・86・88〜91・95・96 番は設計の表のとおり。41 番は 5.3 と同じ物差しで見る
  - 完了: 触った `frame_*` のテストが緑で、`--release` で合否が変わる 1 本の名を実装の記録に残してある（`research.md` へ書くのは 6）
  - _Depends: 4.3_
  - _Dispatch: mechanical — 直し方を設計の表が与える_
  - _Requirements: 2.7, 3.1, 4.1, 5.2, 5.8, 5.16_

- [ ] 5.5 本体のテストの残りを直す（T24・57 番の本体の分・98・100〜105・108・109・111・112 番）
  - 98・105・111 番の消す表明・半分は、残るテストが同じ中身を守ることを確かめてから消す
  - 宣言の並びを移し、`super::super::*` を `super::*` にする（102 番）。受ける腕を狭める（104 番）。拡大の判定を本番の関数から引く（112 番）
  - 壁時計の待ちを目印つきの有界の待ちにし、`areka-test-wait-wording` の brief の同じ行を「この spec で済み」に書き換える（109 番）
  - 100・101・103・108 番と、本体の `frame_move_order_tests.rs` の見出し（57 番）は設計の表のとおり
  - 完了: 本体の `runtime_tests/frame*` が緑で、brief の行が書き換えてある
  - _Depends: 2.3, 2.4, 4.2_
  - _Dispatch: mechanical — 直し方を設計の表が与える_
  - _Requirements: 3.1, 3.3, 3.6, 4.1, 5.6_

- [ ] 6. 預かった件と申し送りを記録する（T25）
  - `research.md` に、件 1〜14 の片づけを出どころ（指摘の番号とタスク）と一緒に書く。件 13 の確かめは済みとして記す（要件 5.14・5.17）
  - 規模の見立て（27 タスク）と、討議で決めた直さずに残す指摘 0 件を記録として残す
  - `--release` の合否の変わる 1 本を記録に書く（92 番・件 7）
  - 範囲の外の件を、設計の「直さない・回した指摘」の表のとおりに 5 つの brief（`window-resetwindowpos-hold`・`extra-character-windows`・`test-crash-report-merge`・`audit-emo2-boot-switch`・`audit-areka-app`）へ件として書き足す。持ち主がまだ `completed/` でないことを書く前に確かめる
  - 6 つ目の brief（`areka-test-wait-wording`）は 5.5 で書き換え済みであることを確かめる
  - C23 のほかの 3 本へ申し送る件が 0 件であることを記録に書く
  - _Depends: 5.5_
  - 完了: `research.md` に件 1〜14 の行がそろい、5 つの brief に回した件が載っている
  - _Dispatch: mechanical — 書く中身を設計の表が与える_
  - _Requirements: 2.1, 2.2, 2.3, 2.4, 2.5, 2.6, 5.8, 5.14, 5.17, 7.7, 7.8, 7.10_

- [ ] 7. 仕上げの確かめ

- [ ] 7.1 範囲へ audit をかけ直し、新しい指摘を 0 件にする（T26）
  - `main` を取り込み、範囲のファイルの数と行数を測り直す（移したファイルと新しいファイルは移した先で数える）
  - 範囲だけへ ponytail の audit を、上限 20 件を外し全部のファイルを全行読む作法でかけ直す。出た指摘は直しが生んだ物も含めて直し、0 件になるまで繰り返す
  - 5 回目で収まらず「必ず直す」と本番の振る舞いに関わるものが 0 件なら、開発者に一言知らせて打ち切る。同じ所から「必ず直す」が出続ければ Fable で総括して議題にする。前の回の直しが呼んだ連鎖は Fable でまとめて直す
  - 完了: `research.md` の「かけ直しの結果」に回ごとの件数と直しが書いてあり、最後の回が 0 件（または要件 9.6 の打ち切り）
  - _Depends: 6_
  - _Dispatch: standard — audit の結果に応じて直しを組む_
  - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5, 1.6, 1.7, 1.8, 9.1, 9.2, 9.3, 9.6, 9.7, 9.8_

- [ ] 7.2 全体テストと既定の枝との差分で約束を確かめる（T27）
  - `pwsh -NoProfile -File tools/test-all.ps1` を 1 回回し、緑で終える。赤なら赤のテストだけに絞って回し直す。emo2・pasta の実走や SSP の測りは全体テストの外で回さない
  - 設計の「仕上げの確かめ」の 4 の差分の確かめ（触らないファイルの差分が空・`COMPAT_ARCHITECTURE.md` が空・`file_length_guard_test.rs` が 1 行と件数だけ・範囲の外の差分が一覧のファイルだけ・外への約束の変化が挙げた分だけ）を行う
  - 依存・フォーク・`[patch]`・`vendors/` の差分が 0、端末へ出す文を足していないことを確かめる
  - 完了: 全体テストの緑と差分の確かめの結果が検証の記録に書いてある
  - _Depends: 7.1_
  - _Dispatch: mechanical — 回すコマンドと確かめる差分を設計が与える_
  - _Requirements: 4.4, 6.1, 6.5, 6.6, 6.7, 6.8, 7.2, 7.3, 7.5, 7.6, 7.9, 8.1, 8.2, 8.3, 8.4, 8.5, 9.4, 9.5_

## Implementation Notes

- 1.1: 1 番の `.get(..).iter()` は 7 番の空撃ちと一緒に消えた（足す `.expect` は無し）。観測器が空を返すと赤になることは審査で確かめた。テストの絞り込みはモジュールの道（`frame::text_scale_tests` なら `text_scale_tests`）で指す。clippy の `-D warnings` は既存の `wintf` の警告で落ちるので、触ったファイルに警告が無いことで見る。
- 1.3: `attach.rs` の書き直した 3 行に `（R4.2）` の要件番号が残っている（審査の提案・止めない）。5.1 の 41 番の物差しで見る。
- 1.5: `--force-warn unused_imports` の警告は lib 23・lib＋test-support 16・test 11 から 3 つとも 0 件（`cargo rustc -p areka-runtime --lib [--features test-support | --profile test] -- --force-warn unused_imports`）。`CharWindowMarker` は設計の「どこでも使わない」でなくテストだけの名だったので `#[cfg(test)]` に置いた。`frame.rs` の `reconcile_reported_sizes` の説明に task 番号が残る（5.1 の 41 番で見る）。
- 1.6: 札の腕を落とすと新しいテストのほかに `frame_dpi_sync_hold_tests.rs` の既存の 2 本も赤になる（research の「縛るテストが無い」は言い過ぎだった）。新しいシェルの検体に `kero.name` が無いと scope 1 の揃え方が変わらない。
- 1.7: 源を手で置き替えて同期に上書きさせるテストを直した本数は 0 本（2 源の入れ直しは変化として拾い、本番の `prepare_ghost_windows` と同じ道で作り直す）。作り直しの後にもう一度 `SystemState::get` を呼び、自分の `insert_resource` を次の呼び出しで入れ直しと数えないようにした（設計からの小さな逸れ）。
- 2.1: 字面の対照（`lib.rs` 側）は `areka-runtime` の `zorder_wiring_tests.rs` の `t_zwi06` の末尾だけが持つ。本体の `zorder_wiring_tests.rs` の見出しも合わせた（範囲の外・要件 3.2）。L15 の「同じ字面を」は少し不正確（審査の提案・7.1 の audit で見る）。
- 2.3: 本体の箱の中断は左ダブルクリックでしか起きないので、話し中の矢印のテストの `press` に `DoubleClick` を渡す。檻のテストの絞り込みは `shell_box_integration_tests`（`shell_box` だと `input_events` の単体も拾う）。ホイールの `back_to_auto(3)` の 3 は `Cage::wheel` の `WheelLines::Lines(3)` と同じ値（審査の提案・定数にしてもよい）。
- 2.4: 本体の切り替えのテストのモジュールの道は `frame::switch::`（`switch_tests` で絞ると 1 本しか拾わない）。
- 3.1: `zorder_drain` の支えの 5 つの項目は `pub(in crate::frame)` にした（兄弟の `zorder_descript` のテストから見えるように）。`gpu_on` は COM を呼んだスレッドで初期化し、解かない（前の 3 か所と同じ）。返す許可は呼び手が World より長く持つ。
- 3.2: `dpi_world` の窓の DPI の着せ付けを `dress_dpi_windows` に抜き出した（切り替えのテストが開け直した窓にも使う）。畳みの段が抜けても台帳の表明では赤にならないので、前の窓が消え新しい窓が別の entity であることを前提の表明で見る。
- 3.3: 結線を手で組む箇所は範囲の中に 18（research の「16」は数え違いで一覧の和は 17・1.3 の 1 本で 18）。本体の側の手組みは 26 番の外。共有の支えの `wiring_with(assets, runtime, presenter)` が唯一の手組み。
- 3.4: `frame_default_pos_track_tests.rs` の `HIGH_SIZE_*`・`raise_the_scale` は共有の同名と値が違う（出現の寸の 2 倍）ので私有のまま `DOUBLED_SIZE_*`・`double_the_scale` に改名した。arm の `settled_sizes` は共有の `high_sizes` と値が同じなので別名にせず消した。共有の支えは 309 行。`LOW_DPI`・`HIGH_DPI` の写しは balloon_offset_*・dpi_sync_hold に残る（3.5 の 29 番で見る）。

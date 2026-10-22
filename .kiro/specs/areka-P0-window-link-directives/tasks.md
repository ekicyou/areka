# Implementation Plan

> 設計は `design.md`（設計の決め 1〜12）、調べの記録は `research.md`（SSP の測りは 1.10・設計ディスカッションの結果は末尾）。テストはすべて実際の画面を使わない x64 の決定論テストで、実装の隣の兄弟ファイル（`*_tests.rs`）に置く。実機の根・検体・ログは作業ツリーの `target\` の下だけ。新しい `PlacementRoute` の語は足さず、`placement/follow/window_move.rs`・`placement/diag.rs`・`placement/chain_realign.rs`・`placement/work_area_sync.rs` には、どのタスクも触らない。`DragConfig.move_window` を書くのは 1.3 の書き手だけにする（設計の決め 12）。`drag_follow.rs`（937 行）は呼び出しの数行だけを足し、1,000 行を超えそうなら本番の側をファサードの形で分ける。

- [ ] 1. 吸着の出どころ・窓の組の帳簿・連れ動きの計算・初期化の材料という土台を作る
- [ ] 1.1 キャラクターの窓に吸着の向きの出どころを持たせ、記憶へ書く形を descript の向きにそろえる
  - 窓ごとの部品 `AnchorSource { descript, by_script }` を `placement/follow/anchor.rs` に足して `follow.rs` から公開し、窓を作るとき（`spawn.rs`）に `Anchored` と同じ時に descript の向き・印なしで付ける
  - 記憶へ書く形を決める `saved_form_anchor`（`AnchorSource.descript`、無ければ `Anchored`、どちらも無ければ無し）を `persist.rs` に足し、並べ終えた時点の保存（`persist_unremembered_char_positions`）もこれで形を決める
  - 兄弟のテストで、descript が下・今の向きが `free` の窓の保存値を次の起動の読み戻し（`char_pos_from_origin_x`＋`project_restore`）へ通すと x が書いた位置に戻ること、部品の無い窓は `Anchored` の形で書かれること、作った窓に `AnchorSource` が付くことが緑になる
  - _Requirements: 4.8, 6.1, 6.2_

- [ ] 1.2 窓の組の帳簿を、拒否を値で返す純粋な核として作る
  - `placement/sticky_group_ledger.rs` に組（中身は並びに意味の無い ID の集まり・出どころは台本か descript か）・拒否の型・World の資源 `StickyGroupLedger` を作る。重なりの組の帳簿の私有の ID の読み（`parse_scope_id`）を crate の中へ開ける（1 語）
  - ID の並びの読みは、前後の空白を除いて数でない → 同じ ID の重ね書き → 異なる ID が 2 つ未満の順に検査して拒否し、上限は置かない
  - 台本の組を足す（重なれば何も変えずに拒否）・台本の組だけを解く・descript の組を取り替える（値なし・空なら組なし・台本の組と重なれば作らず拒否・台本の組は触らない）・組の一覧・「この ID はどの組か」・組の相手（自分を除く昇順）の口を持つ
  - 兄弟のテストで、2 つ以上で作る・`1,0` と `0,1` が同じ組・組を 2 つ持てる・descript の組との重なりを含む重なりで何も変わらない・2 未満・重複・数でない・あふれる数の拒否・解除で台本の組だけ消える・取り替えが台本の組を保って重なる descript の組を拒否する、が緑になる
  - _Requirements: 3.1, 3.2, 3.3, 3.4, 3.5, 3.7, 3.8, 3.9, 6.5_

- [ ] 1.3 組の相手の位置の計算と、ドラッグで窓を書く書き手の決め方を作る
  - `placement/follow/sticky_follow.rs` を新しく置いて `follow.rs` から宣言し、相手の新しい位置を「始めの位置＋量を相手の向きで射影」で出す純関数 `partner_target` を作る
  - 「wintf に窓を動かさせるのは吸着なしで組に入っていない窓だけ」の純関数 `wintf_moves_window` と、すべてのキャラクターの窓の `DragConfig.move_window` を今の `Anchored` と帳簿から付け直す `sync_drag_writer` を作る（帳簿が無ければ組なしとして付ける）
  - 兄弟のテストで、下・上・左・右・なしの相手で吸着した軸に量が乗らないこと、`wintf_moves_window` の 4 通り（吸着の有無×組の有無）、2 つの窓の World で組を据えると両方の `move_window` が偽になり組が無ければ向きどおりに戻ること、が緑になる
  - _Requirements: 2.4, 4.2, 5.2_

- [ ] 1.4 標準の位置を解く流れを切り出し、初期化の材料を起動とシェルの切り替えで同じ形にする
  - `reseed.rs` の「走っているスコープごとに窓の DPI で作者の空間のずらしを換算し、バルーンの `windowposition` を合流し、昇順で 1 度に解く」流れを `resolve_standard_scopes` として切り出し（私有の `RunningScope` を crate の中へ開ける）、`apply_shell_descript` はその結果へ記憶を重ねてから部品を入れる（振る舞いは今のまま）
  - `DefaultPlacementBasis` を「descript の値＋バルーンの材料」の形に改め、`reset_window_positions` は descript の値から配置の設定を組む。起動の準備（`placement/mod.rs`）は読み込んだ descript の値とバルーンの `windowposition`・作者の DPI を運んで同じ形で置き（ファイルを読み直さない）、シェルの切り替えの後始末（`frame/switch.rs`）も同じ形で置き直す
  - 2.2・2.3 が並んで同じ所を触らないよう、`placement/mod.rs` に `script_alignment` と `reset_balloon_pos` のモジュールの宣言と空のファイル（兄弟のテストの接続を含む）をこのタスクで先に置く
  - 今の `reseed`・`reset_pos`・`switch_finish` の兄弟のテストが、形を合わせる以外の書き換えなしで緑のまま、起動で置いた材料とシェルの切り替えで置いた材料が同じシェルで等しいことの檻が足されて緑になる
  - _Requirements: 7.4_

- [ ] 2. 吸着の指令・組の指令・バルーン位置初期化・キーの受け口を、それぞれの持ち場で作る
- [ ] 2.1 (P) 組の指令と descript の組を World に当て、記録を出す
  - 帳簿の World の側の薄い関数（台本の組を作る・台本の組を解く・起動で descript の組を据える・シェルの切り替えで descript の組を取り替える）を `sticky_group_ledger.rs` に作る。descript の値は `,` で区切って同じ規則で読む
  - 作成・解除・据え付け・取り替え・拒否のたびに、その時点のすべての組の中身と出どころを `sticky_groups` の `info!` に、拒否を `sticky_group_rejected` の `warn!`（出どころ・受け取った引数・理由）に、窓の無いスコープを含む組を受け付けたときは `sticky_member_absent` の `warn!` に出し、どの関数も最後に 1.3 の書き手で `DragConfig.move_window` を付け直す。帳簿が無いときは `warn!` で捨てる
  - 兄弟のテストで、各関数の帳簿の変化・記録の中身（`log-capture-kit` で取る）・窓の無いスコープの受け付け・付け直された `move_window` が緑になる
  - _Boundary: StickyGroupLedger_
  - _Depends: 1.2, 1.3_
  - _Requirements: 3.1, 3.4, 3.5, 3.6, 3.7, 3.8, 3.9, 6.5, 9.2_

- [ ] 2.2 (P) 台本の吸着の語を読み、現スコープの窓へ当てる
  - `placement/script_alignment.rs` に語の読み（`top`・`bottom`・`left`・`right`・`free`・`default`・読めない値・欠けた値。前後の空白を除き大文字小文字を区別しない。3 つ目以降の引数は `debug!`）と、窓への適用を作る
  - 適用は、欠けた値なら何も変えずに記録、窓や `AnchorSource` が無ければ記録して何もしない、読めない語は記録して `free` と同じ、`default` は descript の向きへ戻して印を外す。`Anchored` と印を書き、1.3 の書き手で `move_window` を付け直し、今の窓の大きさで `resize_window_to` を経路 `MoveCue` で呼んで辺へ沿わせ（バルーンは付いて動き記憶には書かない）、`alignment_changed` を出どころ付きで記録する。大きさが読めなければ向きだけ変えて記録する
  - 兄弟のテストで、6 つの語と綴りの揺れ・読めない語・欠けた値の読み、下から上・左・右へ変えると窓が辺へ沿いもう一方の軸が残る・`free` で動かない・`default` で descript の向きへ戻る・窓の無いスコープ・`move_window` の切り替わり・バルーンが付いて動き記憶に書かれないこと、が緑になる
  - _Boundary: script_alignment_
  - _Depends: 1.1, 1.3_
  - _Requirements: 1.1, 1.2, 1.3, 1.5, 1.6, 1.7, 1.8, 1.9, 2.4, 9.1_

- [ ] 2.3 (P) バルーン位置初期化の関数を作る
  - `placement/reset_balloon_pos.rs` に、きっかけ（台本・メニュー・近道）を受ける `reset_balloon_positions` を作る。材料と窓の組が無ければ `balloon_pos_reset_skipped` を記録して何もしない
  - `resolve_standard_scopes` の結果ごとに、バルーンの付き方を標準のずらしで入れ直し、キーワードのバルーンの素材を結果どおりに入れるか外し、バルーンを経路 `Restore` で置き、標準のずらしを位置の記憶へ書き、`balloon_pos_reset`（スコープ・位置・ずらし・きっかけ）を記録する。キャラクターの窓・吸着の向き・組には触らない
  - 兄弟のテストで、ドラッグでずらしたバルーンが標準の位置へ戻る・キャラクターの窓と向きと組が変わらない・記憶に標準のずらしが届く・起動で置いた材料とシェルの切り替えで置いた材料で同じ位置になる・キーワードの素材が戻る・材料が無いと記録だけ、が緑になる
  - _Boundary: reset_balloon_pos_
  - _Depends: 1.4_
  - _Requirements: 7.1, 7.2, 7.3, 7.4, 9.5_

- [ ] 2.4 (P) wintf に、キーの押下をアプリへ知らせる口を作る
  - wintf の窓の部品に `KeyDown`（仮想キー・Ctrl・Shift・Alt・繰り返し）と `OnKeyDown` を `OnFilesDropped` と同じ形で足し、`WM_KEYDOWN` は今の Esc の取り消しの後で受け渡し、`WM_SYSKEYDOWN` を振り分け表に足して同じ受け渡しを通す。修飾キーは窓の手続きの入口で読み、受け渡しの関数には値で渡す
  - 受け渡しは World が使用中なら `key_down_dropped` の `warn!` で渡さない、窓が消えていれば `debug!`、部品が無ければ渡さない、真を返せば押下を消費して `DefWindowProc` へ渡さない
  - `keyboard_tests.rs` で、2 つのメッセージで部品が呼ばれ真なら消費・偽や部品なしなら素通し・World の使用中は記録して渡さない・Esc の取り消しが今どおり、が緑になり、wintf のテストが緑のまま
  - _Boundary: wintf window_proc_
  - _Requirements: 8.1, 8.2, 8.4_

- [ ] 3. ドラッグで組の相手を連れ動かす
- [ ] 3.1 ドラッグの控えと、相手を同じ量だけ動かす手続きを作る
  - `sticky_follow.rs` に資源 `CharDragSession`（ドラッグした窓・その始めの位置・このドラッグで wintf が窓を動かすか・始めの向き・窓のある相手とその始めの位置）と、始め（`begin_char_drag`）・縛りの向き（`drag_anchor`）・書き手（`drag_wintf_moves`）・途中（`follow_partners_on_drag`）・終わり（`finish_char_drag`）を作る。wintf の `DraggingState` は読まない
  - 途中は「今の位置−始めの位置」の量で相手ごとに `partner_target` を出し、経路なしで書いてバルーンを付いて動かす（1 回ごとは記録しない）。控えが無い・別の窓の控えなら相手を動かさず、組の相手がいるときだけ `sticky_follow_skipped` を記録する。窓の無い相手は始めに `sticky_member_absent` で外す
  - 終わりは最終の量で相手を書き、相手ごとに `saved_form_anchor` の形で位置を記憶へ書き、`sticky_follow`（drag・相手・量）を 1 行記録して控えを外す。控えた向きと今の向きが違えば今の向きで当て直す
  - 兄弟のテストで（関数を直接呼ぶ World）、量が始めからの量で積もらないこと・相手の吸着の軸に量が乗らないこと・相手のバルーンが付いて動くこと・終わりに相手の位置が記憶へ届くこと・控えが無いと相手が動かず記録が出ること・途中で向きが変わると終わりに当て直すこと、が緑になる
  - _Requirements: 2.4, 3.6, 4.1, 4.2, 4.3, 4.4, 4.5, 9.3_

- [ ] 3.2 ドラッグの受け手へ連れ動きをつなぐ（結合のタスク）
  - `input_events/drag.rs` のドラッグの始めで控えを作り、`drag_follow.rs` の途中と終わりは、縛りの向きを控えから読み、「wintf が書いた位置を読むか・areka が射影して書くか」の分かれ目を今の「向きが `Free` か」から控えた書き手へ改め（組に入った吸着なしの窓は areka が書く・射影は恒等）、書いた後に相手を動かす呼び出しを 1 行ずつ足し、終わりの記憶の形を `saved_form_anchor` で決める
  - `Rig`（スコープ 0・1）のテストで、組の片方のドラッグで相手とバルーンが同じ量動く・相手が下に吸着していれば横にだけ動く・ドラッグした窓が吸着で動かなかった軸では相手も動かない・終わりに両方の位置が記憶に書かれる・取り消しで相手も始めへ戻る・終わりに `DraggingState` が無くても相手の最終位置と記憶が書かれる・ドラッグ中に向きを変えても始めの向きで縛られ終わりに当て直る・2 つとも吸着なしの組で両方が同じ回に areka から書かれどの回でも位置の関係が変わらない・バルーンのドラッグで相手が動かない、が緑になる
  - 台本で `free` にした窓をドラッグして離した位置が、次の起動の読み戻しで同じ位置に出ることを `Rig` の保存値で確かめる。今のドラッグの檻が緑のまま、`drag_follow.rs` が 1,000 行を超えない
  - テストでは帳簿を 2.1 の関数で World に直接据える（本番で起動に据えるのは 5.1）
  - _Depends: 1.1, 2.1, 3.1_
  - _Requirements: 2.1, 2.2, 2.3, 2.4, 4.1, 4.2, 4.3, 4.4, 4.5, 4.6, 4.8_

- [ ] 4. `\![move]` で組の相手を連れ動かし、`ignore-sticky-window` を効かせる（結合のタスク）
  - `sticky_follow.rs` には 3.1 の関数を変えずに足すだけにし、今の位置から相手ごとに `partner_target` を出して `move_window_to`（経路 `MoveCue`・バルーンも付いて動き記憶に書かない）で置く `follow_sticky_move` と、相手がいるときだけ `sticky_follow_ignored` を記録する `log_sticky_ignored` を作る。量が 0・相手なしなら何もせず、窓の無い相手は `sticky_member_absent` で外し、動かしたら `sticky_follow`（move）を記録する
  - `emo2_boot/move_cue_place.rs` の `apply_move_directive` で、窓を書けた後に実際に動いた量を求め、`ignore_sticky_window` が偽なら連れ動かし、真なら記録だけにする（書けなければ何もしない）。「読まない」の注記を改める
  - `move_cue_apply_move` 系の兄弟のテストで、相手が実際に動いた量だけ動く・相手の吸着の軸は動かない・`ignore-sticky-window` で相手が動かず組が残り、その後の `\![move]` で新しい関係のまま組で動く・相手を記憶に書かない・時間つきでも即時・組の無い窓は今のまま、が緑になる
  - 吸着の当て直し・表情の切り替えによる大きさの変化・作業領域の変化・`resetwindowpos` の並べ直しで相手が動かないことの檻を足し、緑になる
  - テストでは帳簿を 2.1 の関数で World に直接据える
  - _Depends: 2.1, 3.1_
  - _Requirements: 4.7, 5.1, 5.2, 5.3, 5.4, 5.5, 5.6, 5.7, 9.3, 9.4_

- [ ] 5. 台本で変えた吸着と組の寿命を正典と裁定どおりにする
- [ ] 5.1 起動で descript の組を据え、ゴーストの窓を閉じるときに帳簿と控えを外す
  - シェルの descript の `seriko.sticky-window` の生の値を、Z の順の生の値と同じ形で起動の準備の結果まで運び、`ghost_session.rs` の窓を作る閉包で `GhostWindows` と一緒に帳簿を据える（据えた後に書き手が付け直す）
  - `app_exit.rs` の `close_windows_for_restart` で帳簿とドラッグの控えを外す 2 行を足す。SHIORI だけの読み直しは触らない
  - 兄弟のテストで、起動の準備の結果が生の値を運ぶ・据えた帳簿に descript の組が入り捨てる書き方なら記録して起動が続く・`close_windows_for_restart` で帳簿と控えが外れる・SHIORI だけの読み直しで帳簿と `AnchorSource` が残る、が緑になる
  - _Depends: 2.1_
  - _Requirements: 3.8, 3.9, 6.1, 6.2, 6.3_

- [ ] 5.2 シェルの切り替えで、台本で変えた向きと組を保ち、descript の分だけを取り替える
  - `apply_shell_descript` で、`Anchored` と `BalloonFollow` の入れ方を分け、どのスコープでも `AnchorSource.descript` を新しい値にし、台本の印の無いスコープだけ `Anchored` を取り替えて向きが変わったら `alignment_changed`（shell_switch）を記録する。`DragConfig.move_window` の直書きを書き手の 1 回の呼び出しに置き換える（`BalloonFollow`・キーワードの素材・バルーンの置き直しは今どおりどのスコープにも行う）
  - シェルの切り替えの後始末（`frame/switch.rs`）で、新しいシェルの `seriko.sticky-window` で descript の組を取り替える
  - 兄弟のテストで、台本の印のある窓の向きが残り印の無い窓が新しい向きになる・どの窓も `AnchorSource.descript` が新しい値になる・台本の組が残り descript の組だけが取り替わり、重なる descript の組は作られずに記録される・`move_window` が書き手どおり、が緑になる
  - _Depends: 1.4, 2.1_
  - _Requirements: 6.4, 6.5, 9.1, 9.2_

- [ ] 6. 台本の指令を、窓の位置の線に載せて画面の側で当てる（結合のタスク）
  - `emo2_boot/move_cue.rs` の `PositionCue` に吸着・組を作る・組を解く・バルーン位置初期化の 4 種類を足し、`set,alignmenttodesktop`・`set,alignmentondesktop`・`set,sticky-window`・`reset,sticky-window`・`execute,resetballoonpos` の 5 組を選ぶ。吸着の実行者は `move` と同じ道で数として読み（読めなければ `warn!` で捨てる）、組の ID は読まずに運ぶ
  - `frame/drain_resnap.rs` の取り出しの相に 4 つの腕（各 1 行で 2.1・2.2・2.3 の関数を呼ぶ）を足す。`consumer_ledger.rs` に 5 行を `MoveSink` で登記し（31 → 36 行）、`consumer_ledger_tests.rs`・`consumer_ledger_agreement_tests.rs` を合わせる。`zorder_cue.rs` は警告の持ち主が 1 つのままである旨の注記だけを改める
  - 兄弟のテストで、5 組が `MoveSink` に届きほかの受け口に届かない・掛け合わせの組（`reset,alignmenttodesktop` など）がどこにも届かない・同じ台本の `\![set,sticky-window,0,1]` の後の `\![move]` が届いた順に当たり相手が連れ動く・`\![execute,resetballoonpos]` でバルーンが戻る、が緑になる
  - _Depends: 2.1, 2.2, 2.3, 4, 5.1_
  - _Requirements: 1.4, 1.7, 1.9, 3.1, 3.5, 3.7, 7.1_

- [ ] 7. 右クリックのメニューの「バルーン位置初期化」と、位置初期化の近道を入れる
- [ ] 7.1 (P) kanade の許可表に `resetballoonpositionbutton.caption` を足す
  - `areka-kanade` の SHIORI リソースの許可表に 1 名足し（13 → 14 名）、数と一覧の檻を合わせる
  - `areka-kanade` のテストが緑になり、ゴーストがこの名前を返したときに引けることが檻で確かめられる
  - _Boundary: areka-kanade resources_
  - _Requirements: 7.6_

- [ ] 7.2 「バルーン位置初期化」の枠をメニューへ登記して項目として出す
  - `menu/reset_balloon_pos_frame.rs` に枠を作り、`Frame::ResetBalloonPosition` を「シェル位置初期化」のすぐ後・同じ群に置く（`ORDER` は 12 個）。項目名の表に既定「バルーン位置初期化」と SHIORI の名前を足し、枠を `ghost_session_runtime.rs` で登記する
  - 選べる場面は窓の組と材料が在るとき（`reset_balloon_enabled`）。`menu/reset_pos_frame.rs` の選べる式を `reset_position_enabled` として切り出す。選んだら 2.3 の関数をきっかけ「メニュー」で呼び、ゴーストへ知らせを送らない
  - 兄弟のテストで、12 個の枠の並び・項目名の既定と SHIORI の名前・選べる場面と選べない場面・選ぶと初期化しゴーストへ何も送らない、が緑になり、`captions_tests.rs` の数が 12 に合う
  - _Depends: 2.3, 7.1_
  - _Requirements: 7.5, 7.6, 7.7_

- [ ] 7.3 位置初期化の 2 つの近道を、ゴーストの窓が入力先のときだけ効かせる
  - `input_events/shortcut.rs` に、Ctrl と Shift が押され Alt が押されず繰り返しでない F12／F10 だけを拾う純関数と、受け手を作る。F12 は「シェル位置初期化」が選べればメニューと同じ並べ直しの往復を起こし、F10 は「バルーン位置初期化」が選べれば 2.3 の関数をきっかけ「近道」で呼ぶ。選べない場面は何もせず `position_shortcut_ignored` を記録し、組み合わせが合えば押下を消費する
  - ゴーストのすべての窓（キャラクターの窓とバルーンの窓）に受け手を付ける関数を作り、`ghost_session.rs` のファイルの投げ込みの受け手を付ける所の隣で呼ぶ。全体の近道（`RegisterHotKey`）は使わない
  - 兄弟のテストで、組み合わせの判定（Alt つき・片方の修飾だけ・繰り返しは拾わない）・2 つの近道がそれぞれの働きをする・選べない場面で記録だけ・すべてのゴーストの窓に受け手が付く、が緑になる
  - _Depends: 2.4, 7.2_
  - _Requirements: 8.1, 8.2, 8.3, 8.4, 9.5_

- [ ] 8. 互換の対応表と網羅台帳を新しい振る舞いに合わせる
  - `doc/COMPAT_ARCHITECTURE.md` §8 に、読めない向きは外す・欠けた値は変えない／組の捨て方／連れ動きはドラッグと瞬間の `\![move]` だけ／ドラッグの相手も記憶に書き、書く形は descript の向き／寿命（`\![set,zorder]` の組との食い違いを含む）／バルーン位置初期化は記憶に書き対のイベントは無い／近道はゴーストの窓が入力先のときだけ／ドラッグ中に向きが変わればそのドラッグの終わりまで始めの向き／組に入った吸着なしの窓はドラッグで areka が書く、の行を足す
  - 台帳 `sakura-script.toml` の吸着の 3 行・組の 2 行・`execute,resetballoonpos`、`shiori.toml` の `resetballoonpositionbutton.caption`、`assets.toml` の `seriko.sticky-window` と `seriko.alignmenttodesktop` 系を実装済みと持ち主に改め、`property.toml` の `currentghost.seriko.sticky-window` は変えない
  - `ukadoc-survey` の `report` と `report-summary` で生成物を作り直し、`ukadoc-survey` のテストが緑のまま、台帳と生成物が同じコミットに入る。`\![set,zorder]` の食い違いの起票（要件 10.6）は完了の作業（`/kiro-complete` の棚卸）で `/kiro-discovery` を使って行う旨を COMPAT の行に添える
  - _Requirements: 10.1, 10.2, 10.3, 10.4, 10.5, 10.6_

- [ ] 9. 全体と実機で確かめる
- [ ] 9.1 触ったクレートのテストを通し、決定論の檻が要件の分岐をすべて覆うことを確かめる
  - `areka`・`areka-kanade`・`wintf` のテストを回し、赤が出たら赤のテストだけに絞って直す
  - 要件 9.6 の分岐（吸着の語の読み・縛り・組を作る規則と捨てる規則・連れ動きの量と吸着の制限・`ignore-sticky-window`・寿命・バルーン位置初期化の位置と記憶）それぞれに緑の檻が 1 つ以上あることを、テストの名前の一覧で示す
  - 1,000 行の決まりを超えたファイルが無く、`DragConfig.move_window` を書く所が書き手の 1 か所だけであることを grep で示す
  - _Requirements: 9.6_

- [ ] 9.2 ぱすた（「窓際のぱすたさん」）を動かし、組の連れ動き・`ignore-sticky-window`・次の起動での重なり・近道を実機で確かめる
  - 検体 `pasta-in-windows` を `sample fetch` で作業ツリーの `target\` の下へ取り寄せ、根も `target\` の下に絶対パスで置き、`RUST_LOG` を `sticky_groups`・`sticky_follow`・`sticky_follow_ignored`・`balloon_pos_reset`・`reset_pos_menu_outcome` が出る水準まで開けて、排他テストの机で `real-run-kit` を使って起こす。確かめの間は拡大率を変えない
  - 起動で `sticky_groups`（descript_seed・`{0,1}`）が出る・本体側の窓のドラッグ（入力の送り込みは画面のロックが無いときだけ）で相方側の窓とバルーンが同じ量動き、途中で 2 つの矩形を読んで位置の関係が変わらない・辞書の「窓あわせ」（MCP の `raise_event` か `sakurascript`）で相方側だけが動き `sticky_follow_ignored` が出る・終了して起こし直すと 2 つの窓が重なったまま出る、を窓の矩形と記録で判定する
  - ぱすたの窓をクリックしてから 2 つの近道を押し、`balloon_pos_reset`（trigger=shortcut）と `reset_pos_menu_outcome` が出ることで、F10 の届き方と入力先を確かめる。起動の連鎖の確定と記憶の寄せ直しで食い違いが出たら、原因をこの spec の中で直すか、持ち主の spec を確かめて `/kiro-discovery` で起票する
  - 結果・記録の抜粋・申し送り先を `.kiro/specs/areka-P0-window-link-directives/signoff.md` に残し、各判定の行が載る
  - _Requirements: 9.7, 9.8_

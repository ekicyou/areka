# Implementation Plan

> 指摘の番号は audit の番号（requirements.md「指摘の扱い」・design.md「指摘ごとの直し方」）。直し方の細部・テストの置き場・元に戻すと落ちるテストの中身は design.md が正本。どのタスクも `crates/wintf/src/ecs/mod.rs`・`ecs/world/mod.rs`・`ecs/visual/`・`ecs/graphics/`・`Cargo.toml`・`Cargo.lock` に触らない。`crates/areka/` に触るのは 6.1 だけ（前提つき）。範囲の外で触ってよいのは、各タスクに名指した `crates/wintf/tests/`・`crates/wintf/examples/` のファイルだけ（持ち主の `audit-wintf-runtime` は完了しているので申し合わせは要らない）。

- [x] 1. 配置の係と配置の部品
- [x] 1.1 後から付けた・外した `BoxStyle` と消えた子を、その回の配置に正しく映す
  - 木をつなぐ係が、新しく `TaffyStyle` を持った部品の親と自分を「影響を受けた親」に足す（節が前から在る部品も同じ）
  - スタイルを作る係が外れた `BoxStyle` を読み、根なら既定のスタイルへ戻し、根でなければ `TaffyStyle` と計算の結果を外す。`Arrangement` は最後の値のまま残す
  - 木をつなぐ係が外れた `TaffyStyle` の節を taffy の親から外し、計算の係が 2 つの消えた記録を毎回読み切ってから、どちらかが在れば計算する
  - taffy の 6 か所の失敗を `error!`（部品と失敗を載せる）にし、記録だけにする理由を 1 行書く。部品の名前の文字列は失敗のときだけ作る（30 番の taffy の分）
  - 新しいテストのファイルに、design の Testing Strategy の 1・2 番のテストを置く（テストの `mod` 宣言は係のファイルの中に置き、`layout/systems/mod.rs` は変えない）
  - 直す前と後に `crates/wintf/tests/layout/` の配置のテストを回す。答えが正しい方へ変わって赤になったテストは、正しい答えへ直す
  - 完了の状態: 後から `BoxStyle` を付けた子が付けた大きさで並び、外した部品の兄弟と消した真ん中の子の後ろの子がその回のうちに詰まり、消えた次の回は計算しないことを、新しいテストが緑で示す
  - _Requirements: 3.1, 3.2, 5.2, 5.3, 5.9_

- [x] 1.2 taffy の逆引きと、配置の値の型の使われない所を消す
  - taffy の部品の逆引きの表と、それを使う 2 つの関数を消す（26 番）。範囲の外のテスト 3 ファイル（`crates/wintf/tests/layout/taffy_advanced_test/mapping_management.rs`・`tree_sync.rs`・`crates/wintf/tests/layout/taffy_child_order_test.rs`）を、表を引く形から部品の節と taffy の子の並びで確かめる形へ書き換える
  - `taffy.rs` の手書きの `unsafe impl` の根拠と版の説明を今の中身に合わせる（29 番の taffy の分）
  - `BoxStyle` の変換で taffy の既定値を入れ直している 2 か所を消す（31 番）
  - 値の型 5 つから `Component` の derive を外す（33 番）。組み立てが通らない所が出たら、その呼び手だけを直す
  - 拡大率の使われない確かめの関数を消す（23 番の `validate`）
  - 消す前に、消す名前がツリー全体（テスト・例・fixture・文字列）で今も参照 0 か、テストだけかを確かめ直す
  - 完了の状態: 逆引きの名前と `validate` の参照がツリーに 0 件で、書き換えた 3 ファイルを含む wintf のテストが緑
  - _Requirements: 3.1, 3.3, 3.4, 3.5_

- [x] 1.3 `Arrangement` を足したときに、一緒に渡した `GlobalArrangement` を上書きしない
  - 直す前に `crates/wintf/src/ecs/window_proc/mouse_dblclick_wheel_tests.rs` を回して答えを控える（`audit-wintf-window` の持ち物・書き換えない）
  - 付けたときのフックを「無ければ入れる」にする。`GlobalArrangement` が無い部品には今と同じく入る（`Visual` のフックがこの連鎖に頼る）
  - `Arrangement` の手書きの `Default` を derive にする（31 番）。配置の伝播の前の `trace!` を消す（30 番）
  - 上書きを避ける回り道をしていた範囲の中のテスト 2 本（`tests_ex/entity_ex.rs`・`tests_ex/opacity.rs`）を素直な形に戻し、design の Testing Strategy の 13 番のテストにする
  - 範囲の外のテストの説明（`crates/wintf/tests/drag/window_dragging_filter_test.rs` の 2 か所・`crates/wintf/tests/ecs/tree_propagation_test.rs:35-37`）を新しい振る舞いに合わせる
  - 直した後に同じテストをもう一度回す。赤になったら書き換えずに止まり、開発者へ報告する
  - 完了の状態: 束で渡した `GlobalArrangement` の値が残ることをテストが示し、`Visual` だけを付けた部品にも `GlobalArrangement` が入る今のテスト（`crates/wintf/tests/layout/component_hooks_test.rs` など）と `mouse_dblclick_wheel_tests.rs` が前後とも緑
  - _Requirements: 2.6, 3.1, 3.2, 3.4, 3.5_

- [x] 1.4 (P) モニタの `BoxStyle` の組み立てを 1 か所にまとめる
  - モニタの物理の大きさと左上から絶対配置の `BoxStyle` を作る関数を置き、起動と新しいモニタの 2 か所はそのまま使い、値の更新は大きさと左上だけを写す（20 番）
  - モニタの更新の係の重なる `With<Monitor>` を消す（31 番）
  - 記録の行は動かすだけで文を変えない。「Redriving window DPI from updated Monitor (no WM_DPICHANGED required)」は 1 字も変えない
  - 完了の状態: `monitor_systems_tests.rs`（文の確かめを含む）と、字面を数える `monitor_systems_transition_tests.rs` が緑のまま
  - _Boundary: layout/systems/monitor_systems.rs_
  - _Requirements: 3.1, 5.9_

- [x] 2. 描く骨組みと、四角・文字
- [x] 2.1 命令の列を記録する骨組みを 1 つ置く
  - 透明の色の定数と、命令の列を記録する関数（`BeginDraw` の後は失敗しても必ず `EndDraw`・返す失敗は描く所 → `EndDraw` → 閉じるの順・変換は恒等へ戻す）を新しいファイルに置き、`widget/mod.rs` に crate の中だけの `mod` 宣言を足す
  - 結果を部品へ入れる関数を置く。成功なら命令の列を入れ、失敗なら `warn!`（部品の名前は失敗のときだけ作る）を 1 行残し、命令の列が無い部品にだけ空の列を入れる
  - 骨組みのテストのファイルに、design の Testing Strategy の 10 番の 2 行（空の列が入って次の回に対象にならない・前から列を持つ部品は絵を残す・空の列の部品も立ち直りの後に描き直される）を実 GPU で置く。この時点で骨組みに載った本番の係は無いので、テストの中に `Without<GraphicsCommandList>` で回して骨組みを呼ぶ係を置いて試す
  - 描く流れに触る前に、`crates/wintf/tests/widget/bitmap_source_integration_test.rs` と `crates/wintf/tests/graphics/device_lost_recovery_test.rs` を回して答えを控える（2.2〜4.3 の前の基準）
  - `ecs/graphics/` の型と立ち直りの係は読むだけで変えない
  - 完了の状態: 骨組みのテストが緑で、空の列を入れる条件を外すと「毎回やり直す」か「絵が消える」で赤になる
  - _Requirements: 3.1, 3.2, 5.2, 5.3, 5.6, 5.10_

- [x] 2.2 四角を骨組みへ載せ、前景色の式を 1 つにする
  - 四角の描く係を骨組みに載せ、写しの骨組み・透明の色の写し・いつも真の比べを消す（21・10・9・31 番）。`Without<GraphicsCommandList>` の立ち直りは残す
  - 部品の名前の文字列は失敗のときだけ作る（30 番）。古い説明を直す（29 番）
  - 色の別名と色の定数の集まり（`#[deprecated]` ごと）とそのテストを消す（23 番）。wintf の `serde` 機能は残す
  - `Brushes` に前景色を既定へ落とす公開の関数を足し、四角の描く係から使う（32 番）。式を写したテストをこの関数を試す形に書き換える
  - 型の形だけを試すテスト（`rectangle.rs`・`brushes.rs` の該当範囲）を消す（35 番）
  - 完了の状態: 四角の描画のテストと `brushes` のテストが緑で、消した名前の参照がツリーに 0 件
  - _Boundary: widget/shapes/rectangle.rs, widget/brushes.rs_
  - _Depends: 2.1_
  - _Requirements: 2.7, 3.1, 3.3, 5.3_

- [x] 2.3 文字を骨組みへ載せ、向きの設定を 1 つの関数にし、使われない文の配置の部品を消す
  - 読む向きと行の向きを書式へ設定する関数を置き、文字の描く係から呼ぶ。失敗と、折り返しの設定・寸法の取り出しの失敗は `warn!` を残してその部品を飛ばす（19・9 番）
  - 文字の描く係を骨組みに載せる（21・10 番）。前景色の写しを 2.2 の関数に替える（32 番の文字の分）
  - 使われない文の配置の部品 2 つ・それを入れる行・外すときのフック・再公開を消し、`layout/mod.rs` の説明の行も直す（24・34 番）。範囲の外のテスト 2 ファイル（`crates/wintf/tests/widget/label_defaults_test.rs`・`crates/wintf/tests/layout/metrics_test.rs`）からその確かめを消す
  - 古い説明を直す（29 番の文字の分）
  - 新しいテストのファイルに、design の Testing Strategy の 19 番のテスト（4 つの向き・DirectWrite だけ）を置く
  - 完了の状態: 向きのテストが緑で、消した 2 つの部品の名前がツリーに 0 件、文字の描画のテストが緑
  - _Boundary: widget/text/label.rs, widget/text/draw_labels.rs, widget/text/mod.rs, layout/metrics.rs, layout/mod.rs_
  - _Depends: 2.1, 2.2_
  - _Requirements: 3.1, 3.2, 3.3, 3.4, 5.3, 5.6_

- [x] 3. 打ち出し表示
- [x] 3.1 トークを入れ替えると新しい文になり、外すと絵が消える
  - トークを入れるたびに配置の控えを外すフックを足す（3 番）
  - トークを外すフックで命令の列を空にし、控えを外す（消すときも安全な形）。ログを出すだけの外すときのフック 2 つを消す（34 番）
  - トークの型の説明を中身に合わせる。終了の道には触らない
  - design の Testing Strategy の 3 番のテストを実 GPU で置く
  - 完了の状態: 入れ替えた後の控えの文が新しい文になり、外した後の命令の列が空であることをテストが示す
  - _Requirements: 3.1, 3.2, 5.4, 5.6_

- [x] 3.2 最後の待ちと知らせまで進めてから完了し、消えた宛先で落ちない
  - 完了の判定を「時間表の項目を最後まで進めた」に変える。進み具合の出し方は今のまま（4 番）
  - 知らせの宛先が居るかを確かめ、居なければ `debug!` を残して捨て、居れば失敗しない入れ方で入れる（5 番）
  - design の Testing Strategy の 4・5 番のテストを CPU だけで置く（時刻は引数で渡す）
  - 完了の状態: 文字が全部出た時刻は再生中のまま、1 秒後に知らせが返って完了し、宛先を消しても落ちないことをテストが示す
  - _Requirements: 3.1, 3.2, 5.2, 5.3_

- [x] 3.3 時間表と隠す範囲をクラスタと UTF-16 で数える
  - 時間表の項目を作る純な関数と、隠す範囲を出す純な関数を置き、配置の控えを作る所と描く所をそれに乗せる。`chars().count()` を使わない（6 番）
  - 時間表の使われない欄と空の時間表を作る関数を消す（23 番）。型の形だけを試すテストを消す（35 番の `typewriter_ir.rs`・`typewriter/tests.rs` の範囲）
  - design の Testing Strategy の 6 番のテスト（𠮷・絵文字・待ちの後ろの位置・隠す範囲）を CPU だけで置く
  - 完了の状態: 6 番のテストが緑で、消した欄と関数の参照がツリーに 0 件
  - _Requirements: 3.1, 3.2, 3.3, 5.2_

- [x] 3.4 打ち出し表示の描画を骨組みへ載せ、止まった表示を描き直さない
  - 文と背景の 2 つの描く係を骨組みに載せる（21・10 番）。配置の控えを作る所の向きと揃えの設定を 2.3 の関数に替え、失敗は `warn!`（19・9 番）
  - 背景の係を、大きさ・筆・トークの変化と命令の列の無い部品で回し、描けない空のトークでは命令の列を空にする（8 番）
  - 見せる数か状態が変わったときだけ変化の印を立て、文の描く係を変化のあった部品に絞る（12 番）
  - 前景色の写しを 2.2 の関数に替える（32 番の打ち出し表示の分）
  - 透明の筆と描く効果の設定の失敗は全文を描く回り道をやめ、記録の道へ返す（9 番・設計の決め 4）
  - 配置の控えの手書きの `unsafe impl` を消す（22 番・`Send`/`Sync` のテストは残す）。古い説明（29 番）と数える `debug!`（30 番）を直す
  - design の Testing Strategy の 8・12・22 番のテストを置く（8・12 番は実 GPU）
  - 完了の状態: 背景の大きさを変えると命令の列が入れ替わり、止まった表示で 2 回目の更新が印を立てず命令の列を入れ替えないことをテストが示し、打ち出し表示の全テストが緑
  - _Depends: 2.1, 2.2, 2.3_
  - _Requirements: 3.1, 3.2, 5.3, 5.6_

- [x] 4. 画像
- [x] 4.1 画像の解読を裏の作業の 1 回だけにする
  - 読み込んだ変換器をメモリの画像にして返す関数を置き、裏の作業はそれを呼ぶ。`com/wic.rs` と読み込みの関数の再公開は変えない（11 番）
  - αのマスクの係の「Send でないため同期で取る」の説明を中身に合わせ、`mod.rs` の説明も直す
  - design の Testing Strategy の 11 番のテストを置く（COM だけ）
  - 完了の状態: 11 番のテストが緑で、UI スレッドの道に解読の呼び出しが残っていない
  - _Requirements: 3.1, 3.2, 5.3_

- [x] 4.2 画像を外す・付け直すと、新しい絵と当たりになる
  - 描く係は、画像の資源が変わったときにも D2D の画像を作り直す。αのマスクの係は資源の変化で回し、マスクがまだ無いものだけ作り、入れるときは変化の印を立てない（7 番）
  - 裏の作業の結果は、部品がまだ同じ `path` の画像を持つときだけ入れ、マスクも同じ画像のときだけ入れる。違えば `debug!` を残して捨てる（7 番・設計の決め 3）
  - 外すときのフックで命令の列を空にし、画像の描画の部品を無効にし、画像の資源を外す。`HitTest` は残す（7 番）
  - 付けたときのフックを「無ければ入れる」3 つにする（27 番）
  - design の Testing Strategy の 7 番のテストを実 GPU で置く
  - 完了の状態: 付け直すと D2D の画像の番地が変わり、外した部品から資源が消えて `HitTest` が残り、遅れて着いた前の読み込みが入らないことをテストが示す
  - _Requirements: 3.1, 3.2, 5.3, 5.6_

- [x] 4.3 画像の描画を骨組みへ載せ、使われない所と古い説明を消す
  - 画像の描く係を骨組みに載せ、D2D の画像が作れない失敗も同じ道を通す（21・10・9 番）。描く後始末の `EndDraw` は骨組みが返す
  - 使われない命令の型（`unsafe impl Send` ごと）・テストの補助・タスクの溜まりの使われない 3 つの関数とそのテストを消す（23 番の画像の分）
  - 画像の描画の部品の手書きの `unsafe impl` を消す（22 番）。古い説明（29 番）・名前の文字列を失敗のときだけ作る（30 番）・重なる比べ（31 番）
  - 画像のテストの COM の初期化を、入れたときだけ後始末し想定外の失敗は落とす形にする（37 番の画像の分）。型の形だけを試すテストを消す（35 番の画像の分）
  - 描く流れの直し（2.2〜4.3）の後として、`crates/wintf/tests/widget/bitmap_source_integration_test.rs` と `crates/wintf/tests/graphics/device_lost_recovery_test.rs` を回し、2.1 で控えた答えと比べる
  - 完了の状態: その 2 本が 2.1 の控えと同じく緑で、画像のテストも緑で、消した名前の参照がツリーに 0 件
  - _Depends: 2.1_
  - _Requirements: 2.7, 3.1, 3.3, 5.3, 5.6_

- [x] 4.4 画像の `path` を読む関数を足し、範囲の中の読み手を替える（前提なし）
  - `BitmapSource` に `path` を読む公開の関数を足すだけにし、欄は公開のまま残す（ウェーブのあいだ `crates/areka/` が読むため）
  - 範囲の中の読み手（`bitmap_source.rs` の 1 か所・`bitmap_source/tests.rs` の 2 か所）をこの関数に替える
  - 「書き換えたら読み直す」形は足さない（新しい機能になるため）
  - 完了の状態: 範囲の中で欄を直接読む所が 0 件で、`crates/areka/` を含むワークスペースが組み立つ
  - _Requirements: 3.1, 3.6, 5.11, 5.12_

- [x] 5. 当たり判定・色分け画像・残りの小物
- [x] 5.1 当たり判定の規則を 1 本にする
  - 本番の当たり判定を、領域つきの当たり判定が「外れ」でないかの 1 行にし、写しを消す。`NamedRegions` はαのマスクを見ない（14 番）
  - 前景の α を 2.2 の前景色の関数から取る（32 番の当たり判定の分）
  - 点の別名を消して `PointF` と書き、範囲の中のテストと例（`crates/wintf/examples/taffy_flex_demo/main.rs:88`・`region.rs:15`）を直す（25 番）
  - 領域の表が無い部品の記録を `debug!` に下げる（36 番）。αのマスクのテストの説明を実際の道に合わせる（14 番）
  - 当たり判定のテストの COM の初期化を直し（37 番）、型の形だけを試すテストを消す（35 番の当たり判定の分）
  - design の Testing Strategy の 14 番のテスト（透明なマスクを付けた `NamedRegions` で 2 つの判定が揃って当たる・`_ex` のαのマスクの分岐を通るテスト）を置く
  - 完了の状態: 14 番のテストが緑で、範囲の外の呼び手（クリック透過・カーソル・ツールチップ・emo-present・emo-text）を書き換えずにワークスペースが組み立つ
  - _Depends: 2.2_
  - _Requirements: 3.1, 3.2, 3.4, 5.3_

- [x] 5.2 (P) 色分け画像の読み込みを守り、試す
  - 対応が 255 個を越えたら `error!` を残して失敗を返し、説明に「255 色まで」と書く（15 番）
  - 画像のパスを UTF-16 のまま渡す（17 番）
  - 使われない 2 つの型を消す（23 番）。読み込みの働き自体は消さない（後の spec が使う）
  - design の Testing Strategy の 15・17・18 番のテストを置く。PNG はワークツリーの `target/hit_region_color_map_test/` に `image` で書き、透明な画素は使わない（16 番の答えを固定しない）
  - 完了の状態: 3 本のテストが緑で、消した 2 つの型の参照がツリーに 0 件
  - _Boundary: layout/hit_region/_
  - _Requirements: 3.1, 3.2, 3.3, 4.4, 5.2, 5.5_

- [x] 5.3 (P) 残りの古い説明と窓の位置の係の小物を直す
  - `layout/mod.rs`・`rect.rs` の古い説明を直す（29 番の残り）
  - 窓の位置の係で部品の名前の文字列を失敗のときだけ作り、重なる `use` を消す（30・31 番）
  - 完了の状態: `layout/mod.rs`・`rect.rs` の 29 番の分と、窓の位置の係の 30・31 番の分が直り、wintf のテストが緑のまま（29 番の一覧の全部が直ったかは 7.1 で確かめる）
  - _Boundary: layout/mod.rs, layout/rect.rs, layout/systems/window_pos_systems.rs_
  - _Requirements: 2.7, 3.1_

- [x] 6. 前提つきの 28 番
- [x] 6.1 `placement-crate-split` の着地の後に、画像の `path` の欄を読み取り専用にする
  - 前提: `placement-crate-split` が main に着地し、この枝が main を取り込んでいること。始めに確かめ、満たされていなければ手を付けずに止まり、開発者へ尋ねる（`crates/areka/` には触らない）
  - 2026-11-07: `placement-crate-split` は PR#433（`ff31386f`）で main に着地し、この枝は同じ日に main を取り込んだ（`266cccbb`）。前提は満たされている。始めには、`mock-shell.rs:609` の `bitmap.path` がまだ在ることだけを確かめ直す
  - 満たされていれば欄を非公開にし、`crates/areka/examples/mock-shell.rs:609` を 4.4 の関数に替え、`crates/areka/examples/clickthrough_two_rects.rs:192` の説明を 4.2 の後の条件に合わせる
  - 取り込んだ main で `monitor_systems_transition_tests.rs` が見張る `ecs/world/mod.rs` の字面が変わっていれば、そのテストの期待だけを合わせる
  - 完了の状態: 欄を直接読む所がツリーに 0 件で、`crates/areka/` を含むワークスペースが組み立つ（または、前提が満たされず止まって尋ねた記録がある）
  - _Depends: 4.2, 4.4_
  - _Requirements: 3.1, 4.5, 4.6, 5.11, 5.12_

- [ ] 7. 直し終わりの確かめ
- [x] 7.1 同じ範囲へ 2 度目の audit をかけ、新しい指摘 0 件を記録する
  - 範囲を今の main と枝で測り直し、後から足されたファイル（この spec の新しい 4 ファイルを含む）も入れて全部読み、範囲の外は見直さない
  - 上限を外して `/ponytail:ponytail-audit` をかけ、直し済みの件を数え直さない。新しい指摘は直すか、残す 3 種類に当たるなら議題へ回して、もう一度かける。5 回目になったら打ち切る前に開発者へ知らせる
  - どのファイルも 1,000 行を越えていないことを確かめる。research.md の 29 番の一覧の全部が直っていることも確かめる
  - 6.1 が欄を実際に非公開にしてから始める（6.1 が前提を欠いて止まったままなら、28 番が半分のままなので始めない）
  - 完了の状態: research.md に範囲の数と「新しい指摘 0 件」が明記されている（打ち切りが承認されたなら「5 回目で打ち切った・残りは直し済み」）
  - _Depends: 6.1_
  - _Requirements: 1.1, 1.2, 1.3, 1.4, 2.1, 2.2, 2.3, 2.4, 2.5, 2.8, 4.1, 4.2, 4.3, 4.6, 5.1, 6.1, 6.2, 6.3, 6.4, 6.5, 6.7_

- [ ] 7.2 全体テストを緑にする
  - `pwsh -NoProfile -File tools/test-all.ps1` を 1 回回す（整形・ライセンスを含む）。`crates/areka/src/placement_tests/transition_judge_reobservation_tests.rs` と emo-present・emo-text の当たり判定のテストが緑のままであることを見る
  - 文字化けが出てもコンソールの文字コードで逃げない。依存は足さない
  - 完了の状態: 全体テストが緑
  - _Requirements: 5.2, 5.6, 5.7, 5.8, 6.6_

## Implementation Notes
- 1.1: 係のテストは `Schedule` を回をまたいで使い回す。回ごとに新しく作ると全部が Added/Changed に見え、直す前でも緑になる（実装係が 3 本で踏んだ）。
- 1.1: `BoxStyle` を外した部品の下に `TaffyStyle` を持つ子が居ると、その子の節は taffy の木から離れ、`TaffyComputedLayout` は古い値のまま残る。要件・設計に決めが無い（レビュー係の FYI）。7.1 の 2 度目の audit で見る。
- 2.1: 描く流れの直しの前の基準（4.3 で比べる）: `--test widget bitmap_source_integration_test` 7/7 緑・`--test graphics device_lost_recovery_test` 5/5 緑・`#[ignore]` 0 本。
- 2.1: `widget/mod.rs` の `mod record;` に付けた `expect(dead_code)` は 2.2 で四角が骨組みに載ったら消す。`cargo fmt` が CRLF を LF に変えることがあるので、fmt の後は `git diff --stat` で確かめる。
- 2.3: `.kiro/steering/structure.md` の 209・214・226 行に消した `TextLayoutResource` の記述が残る（範囲の外）。`/kiro-complete` の steering 同期で直す。
- 2.3: 文字の描く係で記録の前（書式・配置・向き・折り返し・寸法）に失敗して `continue` した部品は、命令の列を持たないので毎回対象になり `warn!` を出し直す（前からの形・design に決め無し）。7.1 で見る。
- 3.2: `update_typewriters` は完了したトークに `debug!("Typewriter talk completed")` を毎回出す（前から）。3.4 の「止まった表示」で拾う。
- 3.4: 文の無い空のトークで背景の色も命令の列も無い部品は、背景の係に毎回選ばれて何もせず抜ける（design 8 番の形どおり）。打ち出し表示の文の係も、クラスタ・寸法の取り出しに失敗して `continue` した部品は毎回選ばれ `warn!` を出し直す（2.3 の文字の係と同じ形）。7.1 で見る。手当ては空の列を入れる 1 行。
- 3.4: 3.2 の注の完了の `debug!` は、状態が変わった回だけ出るようになって解けた。
- 4.2: `crates/wintf/src/ecs/world/mod.rs:317` の説明「BitmapSourceResource追加検出時に実行」が Changed へ変わって古くなった（触らないファイル）。`/kiro-complete` の棚卸で扱う。`BitmapSource` を `insert` で別の `path` に置き換えても読み込みは始まらない（前から・要件 3.6 で「読み直す形」は足さない）。
- 4.3: 描く流れの後の確かめ: `bitmap_source_integration_test` 7/7・`device_lost_recovery_test` 5/5 緑（2.1 の控えと同じ）。35 番で `bitmap_source/tests.rs:123-136` を消したので、4.4 で範囲の中に残る `path` の読み手は `bitmap_source.rs` の 1 か所だけ。
- 4.3: 有効な `BitmapSourceGraphics` を持つ部品へ `BitmapSourceResource` を直に入れ直して D2D の画像が作れないと、前の画像が残る（前からの形・本番の道では起きない）。直すなら Err の枝で `invalidate()` の 1 行。7.1 で見る。
- 5.1: 点の別名 `PhysicalPoint`（layout::hit_test）を消すため、範囲の外の 5 ファイル（`ecs/cursor/answer.rs`・`ecs/pointer/nchittest_cache.rs`・`ecs/window_proc/mouse_click.rs`・`mouse_dblclick_wheel.rs`・`mouse_move.rs`）の `use` を 1 行ずつ `PointF` に替えた（持ち主の `audit-wintf-input`・`audit-wintf-window` は brief だけで並走なし・design の 25 番の行に追記）。`crates/areka-emo-present/src/mount_tests.rs:22-24` の説明が古くなった（範囲の外）。`/kiro-complete` の棚卸で扱う。

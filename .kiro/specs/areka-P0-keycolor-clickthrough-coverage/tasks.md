# Implementation Plan

> 対象: 完了 spec `areka-P0-shell-implicit-surface` 要件 4.10（抜き色で透明になった場所のクリックが背後の窓へ抜ける）を、焼く → 合成する → 当たりのマスクの順で通す常時テスト 1 本で固定する。製品コード（`#[cfg(test)]` の付いた項目の外）の変更は 0 行。引用は行番号でなく「何を定義している箇所か」で指す（design.md と同じ）。

- [x] 1. 抜き色で透明になった場所のクリックが背後へ抜けることを固定するテスト
- [x] 1.1 検体の受け口を presenter 配下のテストから届くようにし、新しいテストファイルを接続する
  - presenter のクレート内テストから、シェル側の検体の受け口（`R_POST_and_KOMAINU`・`konnoyayame` の 2 口）へ届くよう、テスト専用の受け口モジュールの宣言と受け口 2 口の可視性をクレート内公開へ広げる（`mod` 宣言 1 行・関数 2 行・計 3 行）。受け口の実体は 1 文字も変えず、複製も作らない。`emo2` の受け口・COM 初期化の補助・イベント捕捉の補助は広げない
  - 受け口ファイル冒頭の「このファイルを使うテストの一覧」に新しいテストファイルを足す
  - 新しいテストファイルを presenter の兄弟ファイルとして作り、presenter の末尾の接続宣言の並び（既存の予算等価テスト・キャッシュ容量テストの後）へ同じ形の接続宣言 3 行で繋ぐ。ファイル冒頭の説明文には、何を固定するか（要件 4.10）・抜かれるのは左上の画素と同じ色の画素で絵の内側にも在ること（白／緑）・製品と同じ順と入口で通すこと・読むマスクは当たり判定が読むのと同じものであること・正解は製品の段の出力から導かないこと・GPU からの読み戻しは 0 回で CPU から GPU への転送は起きること、を平易に書く（符牒を使わない）
  - 完了状態: `cargo test -p areka-emo-present --lib` がビルドでき、既存テストが全て緑のまま。`git diff` で `#[cfg(test)]` の付いた項目の外に変わった行が 0 行である
  - _Requirements: 1.7, 1.9, 1.10_
  - _Boundary: present のテスト専用の受け口, 通しテスト_

- [x] 1.2 正解の決め方・全画素の判定・3 面の通しテストを実装する
  - 正解の決め方: テスト自身が同じ PNG を製品と共通の WIC 復号器で復号し、左上の画素と 4 バイト完全一致の画素を「抜かれた」と定める（行の詰め物は読まない）。PNG が α を持たないこと、抜かれた画素の数が 0 より大きく全画素数より小さいことを前提として主張する（目安の実数は固定しない）
  - 判定: マスクの外形が PNG の外形と一致することを先に主張し、全画素について「抜かれた ⇔ `is_hit` が false」を突き合わせ、食い違いの件数と先頭 5 件の座標を失敗の文言に出して食い違い 0 を主張する。「絵の内側は全部『内』」とは主張しない
  - 通しテスト本体: GPU 付き World 1 個（既存の presenter 系テストと同じ補助で COM を初期化）の後に復号器を作り、検体を既存の共有受け口経由でのみ取り（直パス 0 か所）、同じ読み込みの入口で読んで焼き、焼き落ちが 0 件であることを主張し、`R_POST_and_KOMAINU` 面 0（236×462）・同 面 10（140×160・宣言の無い面）・`konnoyayame` 面 0（260×390・パレット形式）の 3 本を、それぞれ素の窓 entity（DPI 96）に target として装着して `ShowSurface` を適用する。`R_POST_and_KOMAINU` は読み込み 1 回・面の表は target ごとに組む
  - マスクは装着した窓の面 entity に載る当たり判定用のマスク資源から読み、キャッシュのスロットは読まない。マスクが供給されていなければ失敗させる
  - 常時テストの条件: 実窓 0・他プロセスの可視窓 0・壁時計を合否に使わない・`#[ignore]` 無し・環境変数ゲート無し・GPU からの読み戻し 0 回
  - テスト名と説明文で「抜き色で透明になった場所のクリックが背後へ抜けることを固定するテスト」であることを平易に述べ、完了 spec の要件 4.10 を指す
  - 完了状態: `cargo test -p areka-emo-present --lib keycolor_clickthrough` で通しテスト 1 本が緑になり、3 面すべてで食い違い 0 を主張している。同クレートの既存テストも全て緑
  - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5, 1.6, 1.7, 1.8, 1.9_

- [x] 2. 同じ完了 spec が残したテストの穴 3 件の相乗り
- [x] 2.1 (P) compose の検体の受け口に「指す先が実在する」の較正テストを 1 本足す
  - compose のテスト専用の受け口ファイルの中に、`konnoyayame` の受け口が `surfaces.txt` を持つフォルダを、`emo2` の受け口が `shell/master` を持つゴーストのフォルダを指すことを判定するテストを足す（present 側の既存の較正と同じ形）。失敗の文言に受け口の名前と指す先のパスを添える
  - 完了状態: `cargo test -p areka-emo-compose --lib every_sample_receptor` で 1 本が緑。受け口を存在しない先へ向けると赤になることを一度走らせて確かめ、元へ戻す
  - _Requirements: 3.1, 3.4_
  - _Boundary: compose のテスト専用の受け口_

- [x] 2.2 (P) 本文走査（シェルを自前で解析していないことの見張り）の対象に examples 3 本を足す
  - 採寸テストのファイルに、`emo-present` の組み立て・`collision-probe` の組み立て・`window-placement` の 3 本を対象とする本文走査のテストを 1 本足し、既存の走査器（生文字列の拒否 → コード部分の抽出 → シェルの解析呼び出しが 0 件）をそのまま使う。走査器は変えない。名前を持ち込むだけの 2 本は対象に含めない
  - 同ファイルの走査器の説明文にある「走査対象のファイル数」の記述を、対象が増えても正しい言い方へ改める
  - 完了状態: `cargo test -p areka --bin areka placement` で新しいテストを含めて緑。3 本のどれかにシェルの解析呼び出しを一時的に書くと赤になることを一度走らせて確かめ、元へ戻す
  - _Requirements: 3.2, 3.4_
  - _Boundary: placement の採寸テスト_

- [x] 2.3 examples 2 本の私有関数を、公開の同名関数と取り違えない名前へ改める
  - `emo-present` の組み立てと `collision-probe` の組み立ての私有関数（今日は公開の焼き関数と同名）を `load_shell_assets` へ改名する。定義・呼び出し・旧名を指す説明文だけを変え、ロジックは 0 行変えない
  - 旧名を指す説明文を、`collision-probe` の冒頭説明文（doc リンク・リンクの形は保つ）と `window-placement` の説明文でも新しい名前へ改める
  - 完了状態: `cargo build -p areka --examples` が通り、examples フォルダの中で旧名の出現が 0 件（grep）。差分に改名以外の行が無い
  - 2.2 の赤の確認が同じ examples に一時的な行を書いて戻すので、2.2 の後に行う（同時に走らせると戻しが改名を巻き込む）
  - _Requirements: 3.3, 3.4_

- [x] 3. 途中の段で抜いた α が落ちるとテストが赤になることの実証（差し替えは製品コードに残さない）
- [x] 3.1 焼く段の差し替えで赤を示し、戻して赤 0 を確かめる
  - 抜き色の正規化の腕（抜き色を用いて画素を透明にする腕）が受け取る抜き色を「無し」に置き換え、腕が画素を変えずに返す形にする（経路から外す形・平行移動は用いない）
  - 走らせ方: `cargo test -p areka-emo-atlas`・`cargo test -p areka-emo-compose`・`cargo test -p areka-emo-present` の 3 本
  - 記録: 差し替えたファイルと「その箇所が何を定義しているか」・置き換えの前後 1 行・走らせたコマンド・赤になった通しテストの名前と失敗の文言（食い違いの件数と先頭 5 件）・道連れで赤になった既存テストの一覧（抜き色の腕のテスト・合成後の左上の α を見るテスト等を隠さない）
  - 着手前に 1.x・2.1 の作業をコミット済みにしておき（誤った `git checkout` で消さないため）、差し替えるファイル単体の `git diff --stat -- <ファイル>` が空であることを確かめる。記録の後 `git checkout -- <ファイル>` で戻して同じ 3 本を走らせ、赤 0 と、同じファイル単体の `git diff --stat -- <ファイル>` が空であることを記録する
  - 完了状態: 通しテストが自身の食い違い 0 の主張で 3 面すべて赤になった記録と、戻した後の赤 0 の記録が本ファイルの完了記録に残っている
  - 2.1 の後に行うのは、compose のテストの顔ぶれを確定させてから道連れの一覧を取るため（途中で増えると記録がずれる）
  - _Requirements: 2.1, 2.2, 2.3, 2.4, 4.1, 4.3_
  - _Depends: 1.2, 2.1_

- [x] 3.2 合成段の差し替えで赤を示し、戻して赤 0 を確かめる
  - 転写ループで転写した画素の α を捨てて不透明で書く形に置き換える（トリム後の矩形の内側にも抜かれた画素が、本テストの 3 面すべてで在るので赤になる見込み。要件 2.1 の「4 面」は research.md §2.3 で実測した面の数で、本テストが見るのはそのうち 3 面）
  - 走らせ方・記録・戻し方は 3.1 と同じ。道連れの見込み（転写のテスト・合成の golden 群・予算等価テスト）を実際の出力と突き合わせて記録する
  - 完了状態: 3 面すべて赤の記録と、戻した後の赤 0 の記録が完了記録に残っている
  - _Requirements: 2.1, 2.2, 2.3, 2.4_

- [x] 3.3 マスク段の差し替えで赤を示し、戻して赤 0 を確かめる
  - 表示の適用でマスク生成へ渡す表示バッファを、同じ長さの全画素不透明のバッファに置き換える
  - 走らせ方・記録・戻し方は 3.1 と同じ。マスクの輪番を直に叩くテストが影響を受けないことも記録する
  - 完了状態: 3 面すべて赤の記録と、戻した後の赤 0 の記録が完了記録に残っている
  - _Requirements: 2.1, 2.2, 2.3, 2.4, 4.2_

- [x] 4. 最終確認と報告の数
- [x] 4.1 全体を走らせ、触ったファイルと数を確かめて報告の材料をそろえる
  - `cargo test -p areka-emo-atlas`・`cargo test -p areka-emo-compose`・`cargo test -p areka-emo-present`・`cargo test -p areka --bin areka placement`・`cargo build -p areka --examples` がすべて緑
  - `git diff --stat main...` で触ったファイルが design.md の変更の全数の表と一致し、抜き色の正規化・マスクの輪番・wintf・既存の両端のテストと合成後の α を見るテスト・`.kiro/specs/completed/` 配下・`areka-P0-alpha-release-signoff` の文書に差分が無いことを確かめる
  - 数を確かめる: 製品コードの変更行数 0（`#[cfg(test)]` の付いた項目の外）・接続宣言 3 行・可視性の書き換え 3 行・GPU からの読み戻し 0 回・拡大率 k ≠ 1 の水準 0 本（理由: マスクは原寸で作られ ÷k は wintf 側）・CPU から GPU への転送は起きる
  - 報告の文言の決まり（平易な語・「実機の確認は不要になった」と書かない・「左上の画素と同じ色の画素が抜ける」と書く・差し替え 3 段の記録と戻した後の赤 0 を含める）に沿って、本ファイルの完了記録に数と記録を書く
  - 完了状態: 上の 5 本がすべて緑で、完了記録に数（0 行・3 行・3 行・0 回・0 本）と差し替え 3 段の記録が揃っている
  - _Requirements: 1.10, 2.3, 2.4, 3.4, 4.1, 4.2, 4.3, 4.4, 4.5, 4.6, 5.1, 5.2, 5.3, 5.4_
  - _Depends: 2.2, 2.3, 3.3_

## Implementation Notes

- `crates/areka` は bin だけのパッケージで `--lib` は「no library targets found」で走らない。placement のテストは `cargo test -p areka --bin areka placement` で走らせる（tasks.md・design.md のコマンドを 2.2 で訂正済み）。
- 通しテストは面ごとに止めず、3 面の食い違いを集めて最後に 1 回だけ主張する（3.1 で改訂）。面ごとに止めると差し替えの赤が 1 面目しか観測できないため。design.md の判定の節も `keyed_out_mismatch` へ追随済み。

## 完了記録

### 3.1 焼く段の差し替え（2026-09-24）

- ⑴ 差し替えた箇所: `crates/areka-emo-atlas/src/normalize.rs` の `Normalizer::normalize` の `(UseSelfAlpha::On, AlphaSource::KeyColor)` の腕（抜き色の正規化の腕）が受け取る抜き色。腕の入力を「無し」に置き換え、腕は画素を変えずに返す（経路から外す形）。着手前の `git diff --stat -- crates/areka-emo-atlas/src/normalize.rs` は空（HEAD `526873b9`）。
- ⑵ 置き換えた 1 行: 前 `let key = Self::key_color(&img, params, has_pna);` → 後 `let key: Option<[u8; 4]> = None;`
- ⑶ 走らせたコマンド: `cargo test -p areka-emo-atlas --no-fail-fast`・`cargo test -p areka-emo-compose --no-fail-fast`・`cargo test -p areka-emo-present --no-fail-fast`
- ⑷ 赤になった通しテスト: `presenter::keycolor_clickthrough_tests::keyed_out_pixels_leave_the_hit_mask_and_the_rest_stay_inside`。前提の主張（焼き落ち 0・α 無し・0 < 抜かれた画素 < 全画素・外形一致・マスク在り）はすべて通り、自身の「食い違い 0」の主張で赤。失敗の文言（3 面を集めて主張する形で観測）:
  - `R_POST_and_KOMAINU 面 0: 抜き色の判定とマスクが 59831 画素で食い違う（先頭 5 件 (x, y, 期待は内か): [(0, 0, false), (1, 0, false), (2, 0, false), (3, 0, false), (4, 0, false)]）`
  - `R_POST_and_KOMAINU 面 10: 抜き色の判定とマスクが 11816 画素で食い違う（先頭 5 件 …: [(0, 0, false), (1, 0, false), (2, 0, false), (3, 0, false), (4, 0, false)]）`
  - `konnoyayame 面 0: 抜き色の判定とマスクが 70574 画素で食い違う（先頭 5 件 …: [(0, 0, false), (1, 0, false), (2, 0, false), (3, 0, false), (4, 0, false)]）`
  - 件数は 3 面とも抜かれた画素の数（design の目安）と一致。
- ⑸ 道連れで赤になった既存テスト（全件・見込みと完全一致・見込み外 0）:
  - areka-emo-atlas（78 passed / 5 failed / 1 ignored）: `normalize::tests::on_no_alpha_no_pna_selects_keycolor_seam`・`normalize::normalize_key_color_tests::color_off_by_one_in_a_single_component_stays_untouched`・`…::distant_pixels_of_the_key_color_become_transparent`・`…::image_of_a_single_color_is_ok_and_fully_transparent`・`…::row_padding_is_neither_read_nor_written`
  - areka-emo-compose: 赤 0（220 passed）。`base_image::tests::konnoyayame_has_no_dangling_pattern_targets_with_images` は見込みどおり緑
  - areka-emo-present（253 passed / 2 failed）: 本テスト・`shell_target::template_tests::r_post_and_komainu_shows_both_scopes_from_file_names_alone`（面 0 の左上の α が 255）
- ⑹ 戻した後: `git checkout -- crates/areka-emo-atlas/src/normalize.rs` で戻し、同ファイルの `git diff --stat` は空。同じ 3 本で atlas 83 passed / 1 ignored・compose 220 passed・present 255 passed、赤 0。

### 3.2 合成段の差し替え（2026-09-24）

- ⑴ 差し替えた箇所: `crates/areka-emo-compose/src/blit.rs` の `pub(crate) fn execute` の転写ループで、転写した画素の α を書く行（乗算済み SourceOver を α に当てる行）。転写した画素の α を捨てて不透明で書く形。着手前の同ファイルの `git diff --stat` は空（HEAD `14e48624`）・置き換える行は grep で 1 件。
- ⑵ 置き換えた 1 行: 前 `dst[di + 3] = source_over_channel(src_a, dst_a, inv_src_a);` → 後 `dst[di + 3] = 255;`（ビルドは通り、`dst_a` 未使用の警告 1 件のみ）
- ⑶ 走らせたコマンド: `cargo test -p areka-emo-atlas --no-fail-fast`・`cargo test -p areka-emo-compose --no-fail-fast`・`cargo test -p areka-emo-present --no-fail-fast`
- ⑷ 赤になった通しテスト: `presenter::keycolor_clickthrough_tests::keyed_out_pixels_leave_the_hit_mask_and_the_rest_stay_inside`。前提の主張はすべて通り、3 面を集めた最後の「食い違い 0」の主張で赤:
  - `R_POST_and_KOMAINU 面 0: 抜き色の判定とマスクが 24959 画素で食い違う（先頭 5 件 (x, y, 期待は内か): [(20, 50, false), (21, 50, false), (22, 50, false), (23, 50, false), (24, 50, false)]）`
  - `R_POST_and_KOMAINU 面 10: 抜き色の判定とマスクが 8712 画素で食い違う（先頭 5 件 …: [(0, 16, false), (1, 16, false), (2, 16, false), (3, 16, false), (4, 16, false)]）`
  - `konnoyayame 面 0: 抜き色の判定とマスクが 51984 画素で食い違う（先頭 5 件 …: [(7, 47, false), (8, 47, false), (9, 47, false), (10, 47, false), (11, 47, false)]）`
  - 件数は 3 面ともトリム後の矩形の内側にある抜かれた画素の数（design の目安）と一致。先頭の座標はどの面もトリム後の矩形の左上の角から始まる（矩形の外は透明のまま「外」）。
- ⑸ 道連れで赤になった既存テスト（全件）:
  - areka-emo-atlas: 赤 0（83 passed / 1 ignored）
  - areka-emo-compose: **赤 0（220 passed）＝design の見込み（SourceOver のテスト・合成の golden 群が赤）は外れた**。これらのテストは不透明な下地に重ねるか全画素不透明の検体しか使わず、転写後の α がもともと 255 になるため。compose クレートの中には転写した画素の α を捨てる退行を止めるテストが無く、止めるのは本テストと下の present のテストだけである。
  - areka-emo-present（246 passed / 9 failed）:
    - 見込みどおり: 本テスト・`presenter::budget_equivalence_tests::the_budget_path_produces_the_same_display_bytes_and_mask_as_a_fresh_buffer_path`・`…::the_rotating_face_fixture_differs_in_bytes_and_outnumbers_the_cache`・`…::repeating_the_same_surface_keeps_the_display_bytes_and_mask_equivalent`（3 本とも `assert_expected_is_not_empty` の「期待マスクが一様」で止まる）
    - 見込みに無かった赤（5 本・いずれも検体の前提か陽性対照の主張で止まる）: `cache::tests::mask_generated_once_from_composed_bytes_and_correct`・`cache::tests::insert_after_take_recycled_preserves_approved_semantics`（「fixture は透明画素を含む」）・`display::gpu_tests::identity_offscreen_roundtrip_equals_composed_native_bytes`（α=0 と中間 α の同居の前提）・`presenter::display_failure_tests::display_failure_on_a_same_shape_reshow_keeps_every_previous_value`（陽性対照の `assert_ne`・再表示前後のマスクが同一）・`presenter::fractional_scale_tests::alpha_mask_bits_come_from_native_bytes`（「期待マスクが一様」）
    - 3.1 で赤だった `shell_target::template_tests::r_post_and_komainu_shows_both_scopes_from_file_names_alone` は緑（面 0 の左上はトリム後の矩形の外で α 0 のまま）
- ⑹ 戻した後: `git checkout -- crates/areka-emo-compose/src/blit.rs` で戻し、同ファイルの `git diff --stat` は空。atlas 83 passed / 1 ignored・compose 220 passed・present 255 passed、赤 0。

### 3.3 マスク段の差し替え（2026-09-24）

- ⑴ 差し替えた箇所: `crates/areka-emo-present/src/presenter/show.rs` の `apply_show` にあるマスク生成の呼び出し `target.budget.regenerate_mask(retired_mask, display.bytes(), display.width(), display.height(), display.stride())` の第 2 引数（マスクを作る元の表示バッファ）。表示バッファの代わりに同じ長さの全画素不透明のバッファを渡す形。着手前の同ファイルの `git diff --stat` は空（HEAD `a63bdcd5`）・置き換える行は grep で 1 件。
- ⑵ 置き換えた 1 行: 前 `display.bytes(),` → 後 `&vec![255u8; display.bytes().len()],`
- ⑶ 走らせたコマンド: `cargo test -p areka-emo-atlas --no-fail-fast`・`cargo test -p areka-emo-compose --no-fail-fast`・`cargo test -p areka-emo-present --no-fail-fast`
- ⑷ 赤になった通しテスト: `presenter::keycolor_clickthrough_tests::keyed_out_pixels_leave_the_hit_mask_and_the_rest_stay_inside`。前提の主張はすべて通り、3 面を集めた最後の「食い違い 0」の主張で赤:
  - `R_POST_and_KOMAINU 面 0: 抜き色の判定とマスクが 59831 画素で食い違う（先頭 5 件 (x, y, 期待は内か): [(0, 0, false), (1, 0, false), (2, 0, false), (3, 0, false), (4, 0, false)]）`
  - `R_POST_and_KOMAINU 面 10: 抜き色の判定とマスクが 11816 画素で食い違う（先頭 5 件 …: [(0, 0, false), (1, 0, false), (2, 0, false), (3, 0, false), (4, 0, false)]）`
  - `konnoyayame 面 0: 抜き色の判定とマスクが 70574 画素で食い違う（先頭 5 件 …: [(0, 0, false), (1, 0, false), (2, 0, false), (3, 0, false), (4, 0, false)]）`
  - 件数は 3 面とも抜かれた画素の数そのもの（全画素が「内」になる）。
- ⑸ 道連れで赤になった既存テスト（全件）:
  - areka-emo-atlas: 赤 0（83 passed / 1 ignored）・areka-emo-compose: 赤 0（220 passed）。どちらもマスクより上流。
  - areka-emo-present（250 passed / 5 failed）:
    - 見込みどおり: 本テスト・`presenter::budget_equivalence_tests::the_budget_path_produces_the_same_display_bytes_and_mask_as_a_fresh_buffer_path`（「スロットのマスクが表示バイト由来の独立再現と違う」）・`…::repeating_the_same_surface_keeps_the_display_bytes_and_mask_equivalent`（「ヒット経路のマスクが便宜経路と違う」）。同ファイルの `the_rotating_face_fixture_differs_in_bytes_and_outnumbers_the_cache` はマスクを見ないので緑。
    - 見込みに無かった赤（2 本）: `presenter::display_failure_tests::display_failure_on_a_same_shape_reshow_keeps_every_previous_value`（陽性対照の `assert_ne`・再表示前後のマスクが同じ全面「内」になる）・`presenter::fractional_scale_tests::alpha_mask_bits_come_from_native_bytes`（「αマスク (1,0) のビットが原寸の合成バイト由来でない」）
    - `presenter/budget_tests.rs`（`presenter::budget::tests`）16 本はすべて緑。マスクの輪番を自前の画素で直に叩くので影響しない（要件 4.2）。
- ⑹ 戻した後: `git checkout -- crates/areka-emo-present/src/presenter/show.rs` で戻し、同ファイルの `git diff --stat` は空。atlas 83 passed / 1 ignored・compose 220 passed・present 255 passed、赤 0。

### 4.1 最終確認と報告（2026-09-24）

- ⑴ 走らせたコマンドと結果（HEAD `7e322eb5`・赤 0）
  - `cargo test -p areka-emo-atlas`: 83 passed / 1 ignored
  - `cargo test -p areka-emo-compose`: 220 passed（`sample_test_support::every_sample_receptor_points_at_a_real_folder` を含む）
  - `cargo test -p areka-emo-present`: 255 passed（`presenter::keycolor_clickthrough_tests::keyed_out_pixels_leave_the_hit_mask_and_the_rest_stay_inside` を含む）
  - `cargo test -p areka --bin areka placement`: 913 passed / 2 ignored（`placement::measure::template_tests::the_examples_do_not_parse_the_shell_themselves` を含む）
  - `cargo build -p areka --examples`: 成功（終了コード 0）
- ⑵ 触ったファイル（`git diff --stat main...HEAD`・起点は合流元 `92f5f448`）: design.md「変更の全数」の表の 10 本と完全一致。ほかは本 spec の文書だけ
  - `crates/areka-emo-present/src/presenter.rs`・`presenter_keycolor_clickthrough_tests.rs`（新規）・`shell_target.rs`・`shell_target_test_support.rs`
  - `crates/areka-emo-compose/src/sample_test_support.rs`
  - `crates/areka/src/placement/measure_template_tests.rs`
  - `crates/areka/examples/emo-present/setup.rs`・`collision-probe/setup.rs`・`collision-probe.rs`・`window-placement.rs`
- ⑶ 差分が無いこと（どのパスも実在を確かめたうえで `git diff --stat main...HEAD -- <パス>` が空）: 抜き色の正規化 `crates/areka-emo-atlas/src/normalize.rs`・マスクの輪番 `crates/areka-emo-present/src/presenter/budget.rs`（`MaskRotation` の定義）・`crates/wintf/` 全体・`normalize_key_color_tests.rs`・`presenter/budget_tests.rs`・`shell_target_template_tests.rs`・`.kiro/specs/completed/` 配下・`.kiro/specs/areka-P0-alpha-release-signoff/`。差し替えに使った `blit.rs`・`show.rs` も差分なし
- ⑷ 数
  - 製品コード（`#[cfg(test)]` の付いた項目の外）の変更行数: **0 行**。`src/` の中で変わった行は、テストのときだけ読み込まれるファイル（新しいテストファイル・`shell_target_test_support.rs`・`sample_test_support.rs`・`measure_template_tests.rs`）の中か、`#[cfg(test)]` の付いた項目（`presenter.rs` の接続宣言・`shell_target.rs` の `mod` 宣言）だけ。examples 4 本は製品コードではなく、改名と説明文だけでロジックは 0 行
  - 接続宣言: **3 行**（`presenter.rs` の `#[cfg(test)]`・`#[path = …]`・`mod keycolor_clickthrough_tests;`）
  - 可視性の書き換え: **3 行**（`shell_target.rs` の `mod test_support` → `pub(crate) mod` 1 行・`shell_target_test_support.rs` の受け口 2 口 `pub(super)` → `pub(crate)` 2 行）
  - GPU からの読み戻し: **0 回**（新しいテストとそれが使う補助に `read_back`・`replay_and_read_back` の呼び出しは 0 件）
  - 拡大率 k ≠ 1 の水準: **0 本**（DPI 96 だけ。マスクは原寸で作られ、÷k はマスクを読む側の wintf の当たり判定 `alpha_mask_hit` が行うため）
  - CPU から GPU への転送: **起きる**（表示の記録 `record_display` が原寸のバイト列を `CreateBitmap` で GPU へ送る）
- ⑸ 報告
  - 抜き色で透明になった場所のクリックが背後の窓へ抜けることを固定するテストを 1 本足した。検体 2 体の 3 面（`R_POST_and_KOMAINU` 面 0・面 10、`konnoyayame` 面 0）を製品と同じ順（焼く → 合成する → 当たり判定用のマスクを作る）で通し、左上の画素と同じ色の画素が抜ける（マスクで「外」になる）こと、それ以外の画素は「内」のままであることを全画素で確かめる。抜き色は絵の内側の画素（白や緑）にも当たる。正解は、テスト自身が同じ PNG を読み直して決めている。
  - 途中の 3 段を 1 つずつわざと壊すと、このテストは 3 面すべてで失敗した。焼く段で抜き色を渡さないと 59831／11816／70574 画素、合成段で α を捨てると 24959／8712／51984 画素、マスクを作る段に全面不透明を渡すと 59831／11816／70574 画素の食い違いが出た。どの段も元に戻したあとは失敗 0 に戻った（詳細は 3.1〜3.3）。
  - 見つかった穴: 合成段を壊しても compose クレート自身のテストは 1 本も失敗しなかった（3.2 に記録）。この退行を止めるのは、本テストと present クレートの一部のテストだけである。compose クレートの中に止めるテストを足す作業は、別件として提案済み。
  - このテストが止めるのは、今日通っている動きの退行だけである。実機での目視の確認（`areka-P0-alpha-release-signoff` の目視項目）はこれまでどおり残る。

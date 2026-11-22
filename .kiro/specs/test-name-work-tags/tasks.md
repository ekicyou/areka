# 実装計画

群は design.md の File Structure Plan の G0〜G8。どの群も同じ作業の木で 1 群ずつ直列に進め、1 群 1 コミットにする（`(P)` は付けない: 全部の群が同じブランチにコミットし、G3・G4 は `doc/COMPAT_ARCHITECTURE.md`、どの群も `research.md` を書くため）。G2〜G7 は互いの結果に依らないので、順は入れ替えてよい。G1 は先頭、G8 は最後に固定する。

各群の閉じ方（design.md System Flows）: ⑴ `refs <群>` の当たりが 0、あわせてその群のファイルへ最後の確かめの式 4・5 を当て、当たりが例外の表のものだけ ⑵ `cargo fmt --all` ⑶ `cargo check --workspace --all-targets` が警告 0 ⑷ 改めた crate の `cargo test -p <crate>` が前と同じ合否 ⑸ コミット。`refs` は steering と未完了の spec の文書にも当て、当たれば同じ群で直す。歴史の場所は置き換えの外なので書き換わらない。群の中でテストの合否か本数が変わったら、その群の中で直す。

- [x] 1. 着手の数え直しと改名表を作る（G0）
  - C25 の相手（`session-crate-split`・`dola-cue-test-builder`）が着地していれば main を取り込んでから数える
  - 範囲のファイルを触る別の spec（`zorder-property`・`zorder-chain-residue` ほか）が走っていないかを確かめ、走っていればそのファイルの群を後に回すことを記録する
  - 使い捨ての式（表を作る・群を置き換える・残る名指しを数える）を作業の木の `target/test-name-work-tags/` に置く（コミットしない）。表を作るときの検査（古い名の重なり・同じファイルの名とのぶつかり・予約語・同じ名の組）が崩れたら表を作らずに止まる
  - 札の形の数え直しで、起票の後に足された札つきのテストも同じ規則で表に入れ、C25 の相手の持ち物のファイルに在るものは表に入れず記録する。本体の `runtime_tests/zorder_wiring_tests.rs` と検査の見本のファイルは除く。G1〜G6 の一覧に無いファイルで見つかったものは、その場所の群（placement の zorder → G2、解決 → G3、設定と再配置 → G4、窓の作成 → G5、runtime → G6）に入れ、割り当てを記録する
  - 改める前のテストの名の一覧をモジュールのパスつきで `target/test-name-work-tags/tests-before.txt` に取る（コミットしない・タスク 9 が使う）
  - 完了の状態: `tests-before.txt` が取れていて、`rename-table.tsv`（見出しと 284 本前後の行）がコミットされ、`research.md` に数え直しの結果（ファイルごと・crate ごとの本数、同じ名の組と research.md 2.2 の 6 組との比べ、名指しの当たり、後に回したファイルの有無）が書かれている
  - _Dispatch: standard — 表を作る式と作るときの検査は新しく書く_
  - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5, 1.6, 2.3, 2.4, 3.2, 4.6, 6.4_

- [ ] 2. 本体の表が読む 10 ファイルと 2 つの表を改める（G1）
  - 手を付ける前に `dola-cue-test-builder` のセッションへ「`consumer_ledger_test_support.rs` の 28 行のコメントの名だけを直す・126 行からの塊に触らない」と知らせる
  - G1 の行を機械で置き換え、本体の `placement_tests/` の 2 つの表の名と札つきの各 1 本を同じコミットに入れる
  - 範囲の書き方（`t_zgp1`〜`t_zgp14` ほか）は番号を消して中身の言葉を残し、札だけの短い書き方は新しい名の全体へ直す。`zorder_group_ledger_state_tests.rs:20` の同じ名の名指しにはファイルを添える
  - `consumer_ledger_test_support.rs:28`・`window_state_cue_tests.rs:324`・`doc/COMPAT_ARCHITECTURE.md:226` の名だけを直す（`window_state_cue_tests.rs` は G6 のファイルだが、324 行の名指しは G1 の名なので G1 で直す）。cue の 3 ファイルの差分は `fn` の行とコメントだけで、`carrier_cue` ほかの呼び出しの行は出ない
  - 完了の状態: 閉じ方の ⑴〜⑷ が通り、`cargo test -p areka-placement`・`-p areka-runtime` と本体の表の検査 `cargo test -p areka --bin areka placement_tests::zorder` が緑で、1 つのコミットに入っている
  - _Dispatch: mechanical — 改名表どおりの置き換えと、design の示す形の手の直し_
  - _Requirements: 2.1, 2.2, 3.1, 4.1, 4.2, 4.3, 4.4, 4.6, 4.7, 4.9, 4.11, 6.2, 6.3, 6.5_

- [ ] 3. `areka-placement` の zorder の残りを改める（G2）
  - G2 の行を機械で置き換え、`zorder_property_deferral_tests.rs` の doc コメントのリンクの見出しと短い書き方を新しい名の全体へ直す
  - 本番のファイル `zorder_group_ledger.rs:667` はコメントの名指しだけを直し、同じ名なのでファイルを添える
  - `.kiro/specs/zorder-property/brief.md` の 59・81・94 行の名だけを直す（同じ行の古い道は直さない）
  - 完了の状態: 閉じ方の ⑴〜⑷ が通り（`cargo test -p areka-placement`）、本番のファイルの差分がコメントの 1 行だけのコミットになっている
  - _Dispatch: mechanical — 改名表どおりの置き換えと、design の示す形の手の直し_
  - _Requirements: 2.1, 3.1, 3.3, 4.1, 4.2, 4.5, 4.6, 4.7, 4.9, 4.11_

- [ ] 4. 配置の解決のテストを改める（G3）
  - G3 の 5 ファイルの行を機械で置き換える
  - 同じ 5 ファイルの大文字の作業番号（`T-K1`〜`T-K9`・`T-R8` ほか）を消す: 見出しや doc コメントの頭の番号は消して説明の文を残し、他のテストを指す所はファイルと新しい名で、番号でまとめて指す所はファイルと中身の言葉で書き直す
  - `doc/COMPAT_ARCHITECTURE.md:196` の短い書き方 `t_k5` を新しい名の全体へ直す（その行のほかの記述は変えない）
  - 完了の状態: 閉じ方の ⑴〜⑷ が通り（`cargo test -p areka-placement`）、5 ファイルへの式 5 の当たりが 0 のコミットになっている
  - _Dispatch: mechanical, effort medium — design の示す決まりでの注釈の言い直し（60 行余り）_
  - _Requirements: 2.1, 3.1, 4.1, 4.2, 4.4, 4.6, 4.7, 4.9, 4.10, 6.3_

- [ ] 5. 設定と再配置のテストを改める（G4）
  - G4 の行を機械で置き換える。`config.rs` は `#[cfg(test)] mod tests` の中の名とテストを指すコメントだけを変え、`mod` の宣言と本番のコードは変えない
  - `doc/COMPAT_ARCHITECTURE.md` の 354・412 行の名だけを直し、412 行の「旧 `t_c3_char_n_prefix_adds_scope_n`」と「テスト T-C3」は残す。`session-crate-split` が先に着地して隣の行（353・355・411 行）が動いていたら、相手の行を採ってから置き換えを当て直す
  - `.kiro/specs/extra-character-windows/brief.md:133` の 3 本の名を直し、131 行の「テスト T-C3」は残す
  - 完了の状態: 閉じ方の ⑴〜⑷ が通り（`cargo test -p areka-placement`）、`config.rs` の差分が `mod tests` の中だけのコミットになっている
  - _Dispatch: mechanical — 改名表どおりの置き換えと、design の示す行の直し_
  - _Requirements: 2.1, 3.1, 3.3, 4.1, 4.4, 4.5, 4.6, 4.7, 4.8, 6.3_

- [ ] 6. 窓の作成のテストを改める（G5）
  - G5 の 3 ファイルの行を機械で置き換え、`spawn_assembly_tests.rs:154` の名指しを新しい名へ直す
  - 完了の状態: 閉じ方の ⑴〜⑷ が通り（`cargo test -p areka-placement`）、1 つのコミットになっている
  - _Dispatch: mechanical — 改名表どおりの置き換え_
  - _Requirements: 2.1, 3.1, 4.1, 4.2, 4.6, 4.7, 4.9_

- [ ] 7. `areka-runtime` の残りを改め、本体の残りを申し送る（G6）
  - G6 の 3 ファイルの行を機械で置き換え、`frame_schedule_tests.rs:14` の名指しを新しい名へ直す
  - `areka-runtime` の `zorder_wiring_tests.rs` の `t_` の無い札（`zwi01／02` ほか）と `T-N10` を消して中身の言葉を残し、18 行の「（zwi05）」「（zwi08）」は番号だけを消す。差分は `fn` の行とコメントだけ
  - `.kiro/specs/audit-areka-app/brief.md` の「棚卸㊷からの申し送り」の節に 1 項目を足す: 本体の `runtime_tests/zorder_wiring_tests.rs` の 2 本の改名と同じコミットで、4・5・10・11・13 行の名指しを改名表の新しい名へ直し、番号の札を消すこと。日付つきの経緯と開発者の言葉の引用は書かず、既にある 159 行の項目は変えない
  - 完了の状態: 閉じ方の ⑴〜⑷ が通り（`cargo test -p areka-runtime`）、brief に申し送りの 1 項目が入ったコミットになっている
  - _Dispatch: mechanical — 改名表どおりの置き換えと、design の示す中身の申し送り_
  - _Requirements: 2.1, 3.1, 4.1, 4.2, 4.6, 4.7, 4.10, 6.2, 6.6_

- [ ] 8. emo-present の作業番号を消し、決まりの 1 行を足す（G7）
  - `crates/areka-emo-present/` の 2 行（`presenter_dpi_scale_tests.rs:24`・`presenter_test_support.rs:229`）の `T-N3` を消し、`display_tests.rs` の既にある名 `surface_size_matches_scaled_extent_within_one_pixel` で指す形に直す（`display_tests.rs` は改めない）
  - CLAUDE.md の `## Comments` の「`.kiro` の spec・要件・タスクの番号をコードに書かない」の行の `in code` の後に、テストの名を含むことを足す（同じ行の後ろの文と他の行は変えない）。照合のテストは足さない
  - 完了の状態: 閉じ方の ⑴〜⑷ が通り（⑴ は 2 ファイルへの式 5 の当たりが 0、⑷ は `cargo test -p areka-emo-present`）、CLAUDE.md の差分が 1 行だけのコミットになっている
  - _Dispatch: mechanical — design が示す 3 行の直し_
  - _Requirements: 4.10, 7.1, 7.2_

- [ ] 9. 最後の確かめと記録（G8）
  - 式 1〜5（札の形・数字の無い札の形・古い名の名簿・札だけの短い書き方・大文字の作業番号）を当て、式・数・当たった場所を `research.md` に書く。式 4・5 の当たりは例外の表（design.md VerificationRecord と決めた事 3）と 1 件ずつ突き合わせ、訳を書く
  - 同じ式を改める前の木（`00877299`）へ当てた較正の数（式 1 が 284、式 2 が 4 ほか）を同じ記録に書く
  - 改めた後の名の一覧を `tests-after.txt` に取り、前の一覧を改名表で写して並べ直したものと 1 行も違わないこと、crate ごとの本数が同じことを書く
  - `git diff --name-only main...HEAD` が design の挙げるファイルと spec の文書だけで、本体は `placement_tests/` の 2 ファイル、`runtime_tests/` と歴史の場所は 0 であること、触ったファイルが 1,000 行以下であることを確かめる
  - `pwsh -NoProfile -File tools/test-all.ps1 -Format -License` を回す
  - 完了の状態: 全体テストが緑で、`research.md` に式 1〜5 の結果・較正・名の一覧の比べ・触らない所・行数の記録が入ったコミットになっている
  - _Dispatch: mechanical — design が式と期待する数を示す確かめ_
  - _Requirements: 1.3, 2.3, 3.1, 3.2, 3.3, 3.4, 3.5, 4.7, 5.1, 5.2, 5.3, 5.4, 5.5, 6.1_

## Implementation Notes

- 使い捨ての式は `target/test-name-work-tags/rename.py`（build・apply・refs）。G1〜G6 はこれの apply と refs で置き換えと確かめをする
- `tests-before.txt` は改名のある 3 crate（areka・areka-placement・areka-runtime）の `cargo test -p ... -- --list` で取った。タスク 9 の `tests-after.txt` も同じ 3 crate の同じコマンドで取る
- `zorder_cue_tests.rs:148` の `t_zcs4` は design の見込みに無かった名指しで、G1 で直す

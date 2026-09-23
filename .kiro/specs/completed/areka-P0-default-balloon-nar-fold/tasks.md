# Implementation Plan

> **コミットの約束（要件 1.5）**: タスク 1 は**分割しない 1 タスク**で、完了時に **1 つのコミット**へ入れる。畳んだ `.nar`・登記・照合記録・数の追随・結合テストの付け替え・展開フォルダの削除を別々にコミットすると、「登記だけ在る」「`.nar` もフォルダも無い」などの途中状態（どれもテストが赤）が履歴に残るため。途中でレビューを挟む場合も、未コミットの作業を `git checkout`／`git stash`／`git restore` で消さないこと。タスク 2（文書）はタスク 1 と同じコミットに含めてよく、分けるなら後のコミットにする。

- [x] 1. 既定バルーンを `.nar` へ畳んで登記し、結合テストを窓口経由へ付け替え、展開フォルダを消すまでを 1 コミットで行う
  - **畳む**（展開フォルダが在るうちに）: 既存の畳む道具を `--from` で既定バルーンの展開フォルダに向けて 1 回走らせ、印字が「追跡 29 / 展開 29 / balloon/StayseeBalloon=29 / 不一致 0 件」「全ての検体で一致」・終了コード 0 であることを確かめて印字（`.nar` のバイト数込み）を控える。道具の振る舞いは変えない
  - **登記**: 登記表の末尾に、既存の行と同じ形で「名前 `StayseeBalloon`・種別バルーン・同梱バルーン 0 本」の 1 行を足す（窓口の本体は変えない）
  - **照合**（一度きり・常設のテストは足さない＝2026-09-23 裁定）: 検体のパスを印字する bin を `StayseeBalloon` で呼んで `root=`／`folder=` の 2 行と終了コード 0 を控え、`folder=` の 29 本の sha256 を完了 spec `areka-P0-default-balloon-bundle` の provenance「ハッシュ一覧」（§3.1）と 1 本ずつ突き合わせる。1 本でも不一致なら先へ進まず原因（改行変換・追跡漏れ）を切り分ける。結果を本 spec の `verification/nar-roundtrip.md` に逐語で残す（29 行の突き合わせ・不一致 0 件・使った命令・畳む道具の印字・bin の印字 2 行と終了コード 0・`git check-attr text` の `text: unset`）
  - **自己テストの追随**: 検体数を突き合わせる 2 つの assert を逐語の `6`（`SAMPLES.len()` の写しや引き算で書かない）と文言「6 つ」へ、未登録名テストの名前の一覧 2 か所に `StayseeBalloon` を足して関数名の「five」を「six」へ、検体数を語る doc 3 か所を 6 へ、種別の登記テストのバルーンのループに `StayseeBalloon` を 1 名足す（新しい assert は 0＝設計の決定 A）。配列長の 5（起動記録の配列）は検体数ではないので触らない
  - **付け替え**: 既定バルーンの根を返す関数の本体だけを、窓口から `StayseeBalloon` で取得した根をプロセス寿命の `static` で保持してそのフォルダを返す形へ差し替える（`areka-emo-compose` の既存の雛形と同じ形。戻り型は今と同じ＝テーマ別 8 ファイルは 0 行変更・期待値も 0 変更）。取得の失敗は検体名を名指しした panic にする
  - **入口の整理**: 入口ファイルから検体パスの定数とその見出し・doc を削除し、module doc の「検体パスは 1 定数だけが持つ」の節と定数を語る 2 文を「窓口から名前で引く・付け替えるのは根を返す関数の本体だけ・本ファイルは接続宣言だけを持つ入口」へ書き換える。`test_support.rs` の該当 doc と出典行も設計どおり改める
  - **外す**: 展開フォルダを追跡から外したうえで実体も消す（残すと未追跡として残り、付け替え漏れが赤にならない）
  - **判定**（フォルダを消した後に）: `cargo test -p sample-ghost-kit`・`cargo test -p areka-emo-text --test staysee_balloon_fixture_test`・`cargo test -p log-capture-kit --test sample_path_guard_test` の 3 本が全件緑（見張りは変更 0 で検出 0 件）
  - 完了状態: `vendors/sample_ghost/StayseeBalloon.nar` が追跡され `git ls-files vendors/sample_ghost/StayseeBalloon` が 0 行、`lib_tests.rs` で検体数として「5」を語る箇所が 0、結合テストの入口と `test_support.rs` に `vendors/sample_ghost/StayseeBalloon` の綴りと消えた定数への言及が 0、上の 3 本が全件緑、照合記録が在り、これら全てが 1 つのコミットに入っている
  - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5, 2.1, 2.2, 2.3, 2.4, 2.5, 3.1, 3.2, 3.3, 3.4, 3.5, 3.6, 4.1, 4.2, 4.3, 4.4, 4.5_

- [x] 2. 既定バルーンの保管を語る文書を追随させ、完了判定の検索で確かめる
- [x] 2.1 保管の綴りを語る文書と道具の doc の例を `.nar`＋登記表経由へ改める
  - 検体フォルダの README: 先頭の表に `StayseeBalloon.nar` の行（種別・29・実走のバイト数）を足し、「ここで畳んだ 4 本」を無圧縮の 4 本（`StayseeBalloon.nar` を含む）の名指しと deflate の 2 本の区別へ改め（2026-09-24 実装時の訂正: `emo2.nar` は 2026-09-20 の最新版への差し替えで全エントリが deflate になっており、「5 本」と書くと偽になる）、出どころの箇条書きを 1 行だけ足す（CC0・上流・既定バルーン・畳み直し可・同梱可・ハッシュは完了 spec の provenance §1・§3 へのポインタ。節は設けない＝設計の決定 B）
  - steering: `product.md` の既定バルーンの保管の記述を `.nar`＋登記表経由へ、`structure.md` の「検体の顔ぶれ」のバルーンを 3 本へ改め「展開フォルダのままで登記表に載っていない」の文を削除する
  - 進行中 brief 2 本（`alpha-release-signoff`・`baseware-root-layout`）: 出どころを `.nar` を窓口で展開した結果へ改め、「`nar-install` が引き受ける」の申し送りを消し、予告の文を着地済みの形へ改める
  - 畳む道具の module doc の呼び方の例を `<展開フォルダ>` の置き換えへ改める（doc コメントだけ・振る舞い 0 変更）
  - roadmap の #37 の行・#42 の行・`roadmap-history.md` は触らない（#42 は下の注記のとおり `/kiro-complete` が扱う）
  - 完了状態: README・`product.md`・`structure.md`・brief 2 本・畳む道具の 6 ファイルが設計の Modified Files どおりに改まっている
  - _Requirements: 5.1, 5.2, 5.3, 5.5_
  - _Depends: 1_

- [x] 2.2 完了判定の検索 2 本を走らせ、期待どおりの当たりであることを確かめる
  - ⒜ `grep -rn "sample_ghost/StayseeBalloon" crates/` が 0 件（テストの綴り・消えた定数・道具の doc の例の取り残しを拾う）
  - ⒝ `grep -rn "vendors/sample_ghost/StayseeBalloon/" .kiro/steering .kiro/specs/*/brief.md vendors/sample_ghost/README.md` の当たりが `roadmap.md` の #37・#42 の行と `roadmap-history.md` の 2 行の計 4 件だけで、それ以外は 0 件
  - 完了状態: ⒜ 0 件・⒝ 期待の 4 件ちょうどで、文書の変更がコミット済み（タスク 1 と同じコミットか、その後のコミット）
  - _Requirements: 4.1, 5.5, 5.6_
  - _Depends: 1, 2.1_

## 実装タスクに載せない要件

- **5.4（roadmap 台帳 #42 の完了の反映）**: 台帳の注記「状態列は `/kiro-complete` が ✅ に更新」の慣例どおり、`/kiro-complete` が状態列・段列を ✅ にし名前を `completed/` の形へ改める（設計 Modified Files）。本文は #37 の行と同じく経緯として残すので、実装タスクでは書き換えない。タスク 2.2 ⒝ が #42 の行の当たりを期待値に含めるのはこのため。

## Implementation Notes

- 2.1: 設計は README の「畳んだ 4 本」を「5 本」へ改めるとしていたが、`emo2.nar` は 2026-09-20（`a7b7eb19`）の最新版への差し替えで全エントリが deflate になっていた。無圧縮は `fold_tree` で畳んだ 4 本（`StayseeBalloon.nar` を含む）で、要件 5.1・設計・タスクの文言を実物に合わせて訂正した。文書の数を書く前に `.nar` の中身（圧縮方式）を実物から読むこと。

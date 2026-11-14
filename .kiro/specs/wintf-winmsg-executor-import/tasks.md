# 実装計画

- [x] 1. 元のリポジトリの写しを取り込む（コミット 1）
- [x] 1.1 手元の写しの状態を確かめる
  - 手元の写し `C:\home\maz\git\wintf-winmsg-executor` で GitHub から取りに行った（`git fetch origin`）後に、作業木の汚れが 0 行・`main` の先頭が `4f202758ea4eaa25b68d7d88f52af529f96e75df`・`main` と `origin/main` が同じ、の 3 つを見る
  - 作業木と `main` は動かさない
  - 1 つでも外れたら何も写さずに止まり、汚れの行・先頭の番号・2 つの番号を開発者へ示す
  - 3 つの条件の出力が記録として残り、3 つとも満たしたときだけ次へ進む
  - _Requirements: 1.5_
- [x] 1.2 クレートの中身を写してコミットする
  - 手元の写しの `4f20275` から `src`・`examples`・`README.md`・`LICENSE-MIT`・`LICENSE-APACHE`・`Cargo.toml` の 10 個だけを取り出し、`crates/wintf-winmsg-executor/` の下に同じ相対パスで置く
  - `.claude`・`.kiro`・`CLAUDE.md`・`.gitignore`・`Cargo.lock` が写しの下に無い
  - 写した 10 個の blob の番号が `4f20275` の 10 個とすべて同じ
  - コミットの本文に元のリポジトリ `ekicyou/wintf-winmsg-executor` とコミット番号 `4f202758ea4eaa25b68d7d88f52af529f96e75df` が書かれている（この時点ではワークスペースが解決できなくてよい）
  - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.6_

- [x] 2. ワークスペースへ組み込み、公開の一覧を 3 つにする（コミット 2・分けない）
- [x] 2.1 写しの manifest を areka 向けに直す
  - 変えるのは 4 点だけ: ワークスペースの指し先を足す・版をワークスペースの版に・`repository` をワークスペースのもの（areka）に・`publish = true` を足す
  - `license = "MIT OR Apache-2.0"`・edition 2021・`description`・`readme`・`keywords`・`categories`・`include`・docs.rs の設定・直書きの依存 2 つ（`async-task` 4.7.1 既定の機能なし・`windows` 0.62 の 4 つの機能）は元のまま。`authors` は足さない
  - 2.3 までの変更と同じコミットに入れるため、ここではコミットしない
  - `git diff` で写しの manifest の差が上の 4 点だけになっている
  - _Requirements: 1.7, 2.1, 2.2, 2.3, 2.4_
- [x] 2.2 使う側の依存を写しへ替え、`Cargo.lock` を揃える
  - 根の `Cargo.toml` の `[workspace.dependencies]` の `wintf-winmsg-executor` の行を、行の位置を動かさずに `dola` と同じ「版（その時点のワークスペースの版）＋木の中の置き場」の形にする
  - `crates/pilot/Cargo.toml` の直書き `"0.0.5"` をワークスペースの依存にし、その上の 0.0.5 を固定する旨の注記を、木の中の写しをワークスペースの依存で使う旨に書き換える
  - ワークスペースのクレートだけを解き直して `Cargo.lock` を揃える（外部のクレートは動かさない）
  - `git diff -- Cargo.lock` が `wintf-winmsg-executor` の項の中だけ（版の 1 行・出どころと checksum の行の消し）で、`windows`・`windows-core`・`async-task` が 1 項ずつのまま
  - `cargo tree -i wintf-winmsg-executor` で 6 つの使う側が木の中の写しを引き、`cargo metadata` で写しの版・license・repository・publish・edition が設計どおり
  - _Requirements: 3.1, 3.2, 3.3, 3.4, 3.5, 3.6, 5.10_
- [x] 2.3 公開の一覧と較正の見本と公開の段の注記を直し、コミット 2 を作る
  - 公開前の確認の道具の較正値（公開する一覧）を `dola`・`wintf`・`wintf-winmsg-executor` の 3 つにし、較正の正しい見本に写しの項を 1 つ足す。判定の関数・誤った見本・大きさの較正・包む段・公開の順の出し方（`-p` を並べるだけ）は変えない
  - 公開の段 `crates-io.yml` の 1 行目の注記を 3 つのクレートの形にする（きっかけ・権限・段・認証は変えない）
  - 道具を引数なしで回すと `green (publish list: dola, wintf, wintf-winmsg-executor)` で緑、`-Version 9.9.9` で写しの名前を含む失敗（終了コード 1）になる
  - `cargo package --list -p wintf-winmsg-executor` に `LICENSE-MIT`・`LICENSE-APACHE`・`README.md`・`src`・`examples` の物が入る
  - 2.1〜2.3 の変更が 1 つのコミットに入っている
  - _Requirements: 2.4, 2.5, 4.1, 4.2, 4.4, 4.5, 4.6_

- [x] 3. 整形だけのコミットを作る（コミット 3）
  - `cargo fmt --all` を回し、その出力だけをコミットする
  - コミットの差分が写しの `src` の下のファイルだけで、ほかのクレートのファイルが動かない
  - `cargo fmt --all -- --check` が緑
  - _Requirements: 1.3, 1.6_

- [x] 4. 文書を新しい形に書き換える（コミット 4）
- [x] 4.1 (P) 公開の手順書を 3 つのクレートの形にする
  - 冒頭と 1 節: 出すのは 3 つ・写しは `wintf` の土台・出す順は `dola` と写しの後に `wintf`（cargo が決める）・crates.io の写しの版は次のリリースで 0.0.5 からリリースの版へ飛ぶ
  - 2 節: Trusted Publishing の設定の対象を 3 つにし、写しの設定（`ekicyou`／`areka`／`crates-io.yml`／environment は空）を取り込みが本線に入った後の最初のリリースのタグより前に済ませる旨を書く。初回のリリースの前提の文は済んだ話に直す
  - 3・4 節の「2 クレート」を 3 つに。5 節に「写しの Trusted Publishing が無いまま回った」ときの止まり方とやり直し（Re-run か同じ版で Run workflow・既に出た物は飛ばす）を足す。6 節の予備の手順の例と対象を 3 つに
  - 手順書に「2 クレート」「2 つとも」の記述が残らず、3 つのクレートと写しの設定・止まり方・版の飛びが読める
  - _Boundary: 手順書_
  - _Requirements: 5.1, 5.2, 5.3, 5.4, 5.9_
- [x] 4.2 (P) steering に写しを登記する
  - `tech.md`: Key Libraries の項を木の中の写し（ワークスペースの版・「版＋path」の依存・`MIT OR Apache-2.0`・crates.io へ出す・`windows` 0.62 の 4 つの機能を直書き）に書き直し、窓のクラスの記述は残す。「crates.io への公開」で出すクレートを 3 つ・版上げで動かす根の行を 3 行に。「外部クレ化」の文を置き場が木の中に移った形に直す
  - `structure.md`: 新しい節「UI Thread Foundation Crate（wintf-winmsg-executor）」（置き場・役割・公開・ライセンス・edition・由来）を足し、「その他の最上位」の `crates-io.yml` の説明を 3 つに、「外部クレ化完了」の語を直す
  - 2 つの文書に `=0.0.5` と 2 つだけのクレートの並びが残らず、写しの節が読める
  - _Boundary: steering の登記_
  - _Requirements: 5.5, 5.6, 5.9, 5.10_
- [x] 4.3 (P) `/release` の手順の行数を 3 行にする
  - `SKILL.md` の根の `Cargo.toml` の行数を 3 行に
  - `procedure.md` の段 1.3（3 行を読み一致を判定）・段 2.1（代わりのやり方の条件と、3 行がどれもリリースの版でなければ止まる判定）・段 3.2 の表（足した行 3・消した行 3）・段 5.1・再開の表・「見直すきっかけ」を 3 行に合わせる
  - 手順に「2 行」の記述が残らず、版上げの後に 3 行を確かめて止まる判定が読める
  - _Boundary: `/release` の手順_
  - _Requirements: 5.7, 5.8, 5.9_
- [x] 4.4 (P) `/update` と README の古い記述を書き換える
  - `/update` の `SKILL.md`: 自分のクレートの例に写しを足す・版の表記に写しの `version` を足す・`=` で固定した行の一覧から写しを外す・crates.io に出しているクレートの文を 3 つに
  - `README.md` の crates.io に版を出しているクレートの文と、`doc/README.md` の手順書の説明を 3 つに
  - 書き換えた行が木の中の写しと 3 つのクレートを示している
  - _Boundary: 古い記述の書き換え_
  - _Requirements: 5.9, 5.10_
- [x] 4.5 古い記述が残っていないことを確かめ、コミット 4 を作る
  - `git grep` で `=0.0.5`・`"0.0.5"` と、`wintf・dola`／`dola・wintf` の並びを探し、完了 spec・この spec のフォルダ（`.kiro/specs/wintf-winmsg-executor-import/`）・`roadmap*.md`・`crates/pilot/examples/` の記録・`THIRD-PARTY-NOTICES.md`・`crates/shiori-host32-helper/src/main.rs` の doc コメントのほかで 0 件
  - `tools/encoding-check.ps1` が緑
  - 4.1〜4.4 の変更が 1 つのコミットに入っている
  - _Depends: 2.3_
  - _Requirements: 5.9, 5.10_

- [x] 5. 公開の確かめと全体テスト
- [x] 5.1 `-Pending` と `-Verify` を 1 回ずつ手で回す
  - `-Pending -Version {ワークスペースの版}` で `absent wintf-winmsg-executor {版}` が標準エラーに、写しの名前が標準出力に出て終了コード 0
  - `-Verify -Version {ワークスペースの版}` で 3 つとも `--dry-run` で止まり緑。crates.io へは何も上げず、0.0.1〜0.0.5 を取り下げない
  - 2 つの出力が記録として残る
  - _Requirements: 4.2, 4.3, 4.7, 4.8, 4.9_
- [x] 5.2 排他テストの机で全体テストを回し、謝辞を確かめる
  - 排他テストの机を取ってから `tools/test-all.ps1 -Format -License` を 1 回回す
  - i686 の段・ワークスペースのテスト（写しの単体テスト 15 本と doctest 1 本が緑で出る）・公開前の確認（包むだけ）・`cargo deny check`・`cargo about generate` がすべて緑
  - `THIRD-PARTY-NOTICES.md` の差分が写しの行の版だけで、Timo Kröger と寄稿者・ekicyou と寄稿者の著作権の 2 行が写しの塊に残る。外れたら `about.toml` に写しの `LICENSE-MIT` を名指しする設定を足して作り直す
  - 全体テストが緑で終わり、作り直した謝辞がコミットされ、机を返している
  - 並走で写しのテストが揺れたら、同じ題名の取り違えを `/kiro-discovery` で起票する
  - _Requirements: 2.6, 3.7, 3.8, 3.9_

- [ ] 6. 元のリポジトリを移転したと分かる状態にする
  - 許しは 2026-11-14 の設計の討議（議題 2）で design.md の固定の文と操作に対して得ているので聞き直さない。文か操作を変えるときだけ改めて許しを得る
  - このワークツリーの `target\` の下へ一時の作業の写しを取り、枝を切って README の先頭（題の行の次・バッジの前）に固定の文だけを足し、PR を作って squash マージする（手元の写しは使わない）
  - 既定の枝に入ったことを確かめてから `gh repo archive ekicyou/wintf-winmsg-executor` を行う。履歴・タグ・枝を消す操作はしない。接続先の URL や認証の情報を印字しない
  - 元のリポジトリの README の先頭に移転の案内があり、リポジトリが archive の状態になっている。失敗したら archive せず、失敗した操作と残りを記録する
  - _Depends: 5.2_
  - _Requirements: 6.1, 6.2, 6.3, 6.4, 6.5_

- [ ] 7. 完了の報告の申し送りを整える
  - 報告の前に、crates.io の写しの Trusted Publishing の設定が済んだかを開発者へ尋ねる
  - 申し送りの文に、設定の済み／残り・値（`ekicyou`／`areka`／`crates-io.yml`／environment は空）・取り込みが本線に入った後の最初のリリースのタグより前に済ませる必要、後始末の済み／残り、0.0.1〜0.0.5 を取り下げていないことが入っている
  - _Requirements: 4.8, 6.2, 7.1, 7.2_

## Implementation Notes

- 5.1 の記録（2026-11-14・ワークスペースの版 0.1.0）: `tools/crates-io.ps1 -Pending -Version 0.1.0` は終了コード 0、標準出力 `wintf-winmsg-executor`、標準エラー `OK check 'version'` / `present dola 0.1.0` / `present wintf 0.1.0` / `absent wintf-winmsg-executor 0.1.0`。`-Verify -Version 0.1.0` は終了コード 0、3 つとも `Uploading ...` の後に `aborting upload due to dry run` で止まり `green (publish list: dola, wintf, wintf-winmsg-executor)`。crates.io へは何も上げていない。
- 5.2 の記録: `tools/test-all.ps1 -Format -License` の 1 回目は `x64 workspace tests` だけ赤（`areka` の `session_end::sync_send_tests` の 2 本）。本番ソースの同期の送信を数える許可表 `ALLOWED_SYNC_SENDS` が `crates/*` を走査し、写しの `SendMessageW(` 3 か所（`util/window.rs` の `WM_NCCREATE` の送り直し 1・`lib.rs` の `#[cfg(test)] mod` の `WM_CLOSE` 2）を表に無い当たりとした。設計の想定外だったので、表に理由つきの 2 行を足して直し、赤の 4 本だけ `cargo test -p areka sync_send` で緑を確かめた。ほかの段は全部緑（写しの単体テスト 15 本・doctest 1 本・公開前の確認・`cargo deny check`・`cargo about generate`）。謝辞は写しの塊が版 0.1.0 で並びの位置が移っただけで、Timo Kröger と ekicyou の著作権の 2 行は残る。

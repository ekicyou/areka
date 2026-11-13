# 実装計画

- [x] 1. 依存の土台を替え、監査の見逃しを消す
- [x] 1.1 `dola` の任意の依存と機能を YAML から TOML に替える
  - 任意の依存の YAML のクレートを、ワークスペースの版の指定に従う任意の `toml` に替え、任意の機能を `toml` の 1 つだけにする（既定の機能は足さない）
  - docs.rs が機能 `toml` を有効にして説明書を作る指定を足す
  - 機能 `yaml` の唯一の使い手だった YAML の往復のテストの束とその見出しのコメントを同時に消す（無い機能を名指す `cfg` を残さない）
  - `Cargo.lock` は `cargo` に作り直させ、手で直さない
  - 済んだ形: `cargo build -p dola` と `cargo build -p dola --features toml` が通り、`cargo tree --workspace --all-features` の出力に `unsafe-libyaml`・`serde_yaml` が 0 件、`Cargo.lock` から `serde_yaml`・`unsafe-libyaml`・`ryu` が消えて増えた項目が 0
  - _Requirements: 1.1, 2.1, 2.7, 5.4, 5.6_
- [x] 1.2 監査の設定から `RUSTSEC-2024-0449` の見逃しを消す
  - 見逃しの 1 項目と説明のコメント 3 行だけを消し、決まりのコメントと空の `ignore` の並び・ほかの行は残す
  - 新しい見逃し・機能の除外・クレートの除外を足さない。別の勧告で赤になったら黙らせず、勧告の番号と引いているクレートを報告に残して止まる
  - 済んだ形: `cargo deny check` が終了コード 0 で終わり、`deny.toml` の差分がその 4 行の削除だけ
  - _Requirements: 1.2, 1.3, 1.4, 1.5_

- [x] 2. TOML の読み書きの入口を足す
  - 機能 `toml` の下で、文書を TOML の文字列から読む入口（validate はしない）と、TOML の文字列へ整形して書く入口を足す
  - 誤りは `toml` 自身の読み・書きの誤りの型を、`dola` から読みの誤り・書きの誤りの名前で公開して返す（`DolaError` に枝を足さない・包む型を作らない）
  - panic せず失敗は全部 `Err` で返す（`unwrap` を使わない）。既存の公開の型・関数・トレイトは変えない
  - 済んだ形: 既定の機能と機能 `toml` の両方で `cargo clippy -p dola --all-targets` が警告なく通り、差分で足された公開の項目（関数 2 つと誤りの名前 2 つ）が全部 `#[cfg(feature = "toml")]` の下にあり、既存の `pub` の項目を変えた行が 0
  - _Requirements: 2.2, 2.3, 3.1_

- [x] 3. 機能 `toml` の下のテストを書く
- [x] 3.1 (P) TOML の往復と読み書きの失敗のテストを足す
  - 完全な文書を書いて読み戻すと等しいこと、同じ文書の 2 回の書き出しが 1 バイトも違わないことを確かめる
  - 形の誤りと必須の欄の欠けの読み込みが理由の文つきの誤りになり（欠けの方は欄の名前を含む）、TOML で書けない null を持つ文書の書き出しが誤りになることを確かめる
  - 往復のテストの束の先頭の説明と、結合テストの束の先頭の説明を、JSON は開発専用・TOML は機能 `toml` と分かる文に直す。JSON の往復の 3 本は変えない
  - 済んだ形: `cargo test -p dola --features toml` で新しい 4 本が緑、`cargo test -p dola` で JSON の往復の 3 本が直しの前と同じ本数で緑
  - _Boundary: 往復のテスト_
  - _Requirements: 2.3, 2.4, 2.5, 2.8, 3.2_
- [x] 3.2 (P) TOML の NaN と無限大の境界のテストと注記を直す
  - 型の決まった数の欄（変数の初期値・遷移の遅れ・時間の倍率）と untagged の数の欄（遷移の行き先の数・ループの位置・`at` のずれ・パラメトリックのイージングのベジェの点）のそれぞれに `nan`・`inf`・`-inf` を入れた TOML が読めて、validate がその欄を名指しして数値の誤りで拒むことを確かめる
  - Object の値の中の `nan`・`inf` を持つ TOML で、validate と compile が panic せず終わることを確かめる
  - 境界のテストの束の先頭の説明（外部入力の書式）と、NaN の流入経路の注記を、TOML が入口になりうること・Object の値の中の数は数値の規則の外であることが分かる書き方に直す。既存の JSON の境界のテストの中身は変えない
  - 済んだ形: `cargo test -p dola --features toml` で新しい 2 本が緑、注記に「YAML のみ」「機能 `yaml` により既定ビルド外」の語が残らない
  - _Boundary: 境界のテスト_
  - _Requirements: 2.8, 4.1, 4.2, 4.3, 4.4, 4.5_

- [x] 4. 全体テストに機能 `toml` の段を足す
  - 全体テストの `x64 workspace tests` の段の直後に、機能 `toml` を有効にした `dola` のテストの段 `dola toml feature tests` を、検体の取り寄せが赤でも飛ばさない形（`TestStep` でなく `Step`）で足す
  - スクリプトの `.DESCRIPTION` の段の一覧に 1 行足して後ろの番号を 1 つずつ振り直し、「`cargo test` を回す段（5・6）」を「5・7」に直して、新しい段は取り寄せが赤でも回ることを書き足す
  - `tech.md` のフルテストの節の段の一覧と取り寄せが赤のときの文、このワークツリーの `.claude/skills/kiro-complete/SKILL.md` の段の一覧（433 行目）に、この段を 1 つずつ足す
  - 済んだ形: `cargo test -p dola --features toml --no-fail-fast -j 4` が 3 の 6 本を含めて緑になり、段の名前 `dola toml feature tests` がスクリプトと 3 か所の一覧で一致し、スクリプトの中で `x64 workspace tests` の段の次の行にある
  - _Depends: 3.1, 3.2_
  - _Requirements: 2.6, 3.3, 7.1_

- [ ] 5. 説明と申し送りを直しの後の形に合わせる
- [ ] 5.1 README と steering の dola の記述を直す
  - README の書式の 2 行から YAML を消し、機能 `toml` で TOML を読み書きできること・ほかの書式は serde の導出で利用者が選べることを英文で書く
  - `tech.md` の dola の依存の表の YAML の行を `toml` の行に替え、`toml` の行の dola の文を直す。`product.md`・`structure.md` の書式の機能の列挙を `toml` だけにする
  - 済んだ形: README と steering 3 本に dola の機能としての `yaml`・`json` の語が残らず（`serde_json` が開発専用の依存だという記述は残す）、`toml` の版・機能の名前・使い道が書かれている
  - _Depends: 4_
  - _Requirements: 3.3, 5.1, 5.2, 5.3_
- [ ] 5.2 (P) roadmap の申し送りの dola の項目を正味の差の 1 項目に書き直す
  - 書く前に `product.md` で公開済みの最新の版を確かめ、頭を「`v{その版}` の後」にする
  - 機能 `json` と既定の機能の削除（壊れる変更）、機能 `yaml` と `serde_yaml` の削除（壊れる変更・機能の指定を消せば直る）、機能 `toml` の継続と `toml` 0.8→1 と読み書きの入口（利用者は自分で `toml` を依存に書かなくてよい）を 1 項目に書き、未決の書き方を消す
  - 済んだ形: 申し送りに dola の項目が 1 つだけ在り、「`dola-yaml-backend` が決める」の文が無い
  - _Boundary: 申し送りの項目_
  - _Requirements: 6.1, 6.2, 6.3, 6.4, 6.5, 6.6_

- [ ] 6. 全体を確かめる
  - `crates/dola` の下のテスト・README・`Cargo.toml` に `yaml` の語が 0 件、`cargo tree --workspace --all-features` に `unsafe-libyaml`・`serde_yaml` が 0 件であることを確かめる
  - `cargo deny check` と `cargo about generate --workspace about.hbs -o THIRD-PARTY-NOTICES.md` を回し、謝辞に差分が出たら道具が作った物をそのままコミットする（手で直さない）
  - 要件 7.1・7.2 はここでは満たさず、完了の段で満たす: 完了の全体テスト（`tools/test-all.ps1 -Format -License`）は手順の決まりで `/kiro-complete` がアーカイブの後に 1 回だけ回すため、実装の段で重ねて回さない。触っていないクレートだけ赤なら、そのクレートだけ 1 回回し直し、変化が無ければ申し送りを残す
  - 済んだ形: `cargo test --workspace` と `cargo test -p dola --features toml` が緑、`cargo deny check` が終了コード 0、作り直した謝辞が作業ツリーのものと 1 バイトも違わない
  - _Requirements: 1.1, 1.3, 5.5_

## Implementation Notes

- 1.1: `src/value.rs:7` と `src/compile/mod.rs:72` のコメントに「JSON/TOML/YAML」が残る。src を触るタスク 2 で YAML を外す（タスク 6 の yaml 0 件の確かめにも掛かる）。

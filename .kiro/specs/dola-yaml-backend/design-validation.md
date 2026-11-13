# 設計の検証: dola-yaml-backend

- 日付: 2026-11-13
- 対象: `.kiro/specs/dola-yaml-backend/design.md`（作業枝 `a580794b`）
- 読んだ物: `spec.json`・`requirements.md`・`design.md`・`research.md`・steering（`product.md`・`tech.md`・`structure.md`・`roadmap.md` の C19 の行と 207 行目）
- 突き合わせたコード: `crates/dola/Cargo.toml`・`crates/dola/src/lib.rs`・`src/error.rs`（`DolaError`）・`src/value.rs`（`DynamicValue`）・`src/validate/rules.rs`・`tests/compile/boundary_test.rs`（3 行目・255〜265 行目）・`tests/general/integration_test/serialization_round_trip.rs`・同 `mod.rs`・`crates/dola/README.md`（13・17 行目）・`deny.toml`（41〜47 行目）・根の `Cargo.toml`（`[workspace.dependencies]` の `toml = "1"`）・`tools/test-all.ps1`（`.DESCRIPTION` と段の並び）・`.claude/skills/kiro-complete/SKILL.md`（433 行目）・`toml` 1.1.6 の `src/lib.rs`（`pub mod de`・`pub mod ser`・既定の機能）・C19 のほかの 7 本の作業枝（`tools/test-all.ps1` と `kiro-complete` を直す spec は 0 本）

## まとめ

設計は小さく、実物と合っている。行の番号・ファイルの中身・`DolaError` が `#[non_exhaustive]` でないこと・`toml` 1 の `de::Error`／`ser::Error` が既定の機能で公開されていること・全体テストの `Step`／`TestStep` の作りは、どれもコードの通りだった。直すべき点は 1 つで、NaN と無限大の要件 4.2・4.3 の字面と、設計の読み方（Object の値を外す）が食い違っている。これは設計の討議で決めれば済み、作りを変える話ではないので、判定は GO とする。

## 大事な点

### 🔴 1. Object の値の中の NaN・無限大が、要件 4.2・4.3 の字面を満たさない

- **何が問題か**: 要件 4.3 は「`#[serde(untagged)]` の値の欄」でも 4.2 の確かめ（読み込みで断られるか、validate が拒むか）を求めている。`DynamicValue` は `#[serde(untagged)]` の enum（`src/value.rs` 10 行目）で、Object の変数の `initial` と遷移の `to` の表の中の数はこの欄に当たる。設計の実測では、ここに入れた `nan`・`inf` は読めて、validate も通る（`rules.rs` の数値の規則は `DynamicValue` を見ない）。設計は 4.2・4.3 の「数値の欄」を「compile と実行の時刻と補間に使う欄」と読み替えて Object の値を外し、panic しないことだけを確かめる形にしている。
- **何に響くか**: 確定した要件の字面のまま実装とレビューに進むと、境界のテストが 4.2 を満たさないと判定されるか、レビューの段で同じ議論をやり直すことになる。
- **どう直すか**: 設計の討議で開発者に次のどちらかを決めてもらう。推すのは ⑴。
  - ⑴ 要件 4.2・4.3 を直し、「Object の値（`DynamicValue`）の中の数は数値の規則の外で、読めて validate も通る。境界のテストは panic しないことを確かめ、注記にその扱いを書く」と明記する（設計はそのまま。`DynamicValue` の等しさとハッシュが NaN を 1 つの値として扱う今の作りと合う）。
  - ⑵ Object の値の中の非有限の数も拒む。validate の規則（`src/validate/rules.rs`）を直すことになり、要件の範囲の外（dola のほかのモジュール）に出るので、別の spec にする。
- **要件**: 4.2・4.3
- **設計の場所**: 「NaN と無限大の扱い」の表の 3 段目と続く 2 つ目の項目・Out of Boundary の 2 項目目・Testing Strategy の `toml_non_finite_in_object_value_does_not_panic`

### 🟡 2. `.claude/skills/kiro-complete/SKILL.md` を直すことが、要件 3.3 の許す範囲に入るか

- **何が問題か**: 要件 3.3 が `crates/dola/` の外で許すのは「根の `Cargo.lock`・`THIRD-PARTY-NOTICES.md`・`deny.toml`・steering・roadmap・要件 2.6 のための全体テストの手順」。設計は `kiro-complete` の段の一覧（433 行目）に `dola toml feature tests` を 1 語足し、それを「全体テストの手順の説明」として 3.3 に入れている。字面では名指しされていない。
- **何に響くか**: 小さいが、レビューで範囲の外と言われうる。足さなければ `kiro-complete` の段の一覧が実物とずれる（この一覧は今も `sample fetch`・`sample fetch check` を挙げていない＝もともと少しずれている）。
- **どう直すか**: 設計の討議で「全体テストの段の一覧を書いている文書（`tools/test-all.ps1` の `.DESCRIPTION`・`tech.md` のフルテストの節・`kiro-complete` の段の一覧）は 3.3 の『全体テストの手順』に含む」と確かめて記録する。推すのは「含む・直す」。あわせて `tech.md` 85 行目の「`cargo test` を回す段（`x64 workspace tests`・`i686 tests (host-32)`）を skipped」の文に、新しい段は取り寄せが赤でも回ることを書き足す（新しい段も `cargo test` を回すが `Step` で回すため）。
- **要件**: 2.6・3.3
- **設計の場所**: File Structure Plan の `tools/test-all.ps1`・`.claude/skills/kiro-complete/SKILL.md`・`.kiro/steering/tech.md` の項目、「全体テストの段」の Implementation Notes

## よい点

- **入口の形の決め方が要件 3.1 を正しく守っている**: `DolaError` に枝を足さない理由（`#[non_exhaustive]` でない公開の enum に枝を足すと、網羅の `match` を書いた利用者が壊れる）はコードで確かめた通りで、`toml` 自身の誤りの型に名前を付けて出すだけにしたのは、足す物が最小で理由も持てる形になっている。`toml` を 2 に上げると公開の形が変わる危うさも Revalidation Triggers に書いてある。
- **実測が先にある**: 往復・2 回の書き出しの一致・72 通りの NaN／無限大・`Cargo.lock` の差分・見逃しを消した `cargo deny check` の終了コード 0 を、設計の前に作業枝で測ってから戻している。全体テストの段を `TestStep` でなく `Step` にした理由（検体を使わない）も今のスクリプトの作りと合っている。

## そのほかの気づき（判定に響かない）

- docs.rs は既定の機能でしか作らないので、`from_toml_str`・`to_toml_string`・`TomlReadError`・`TomlWriteError` は docs.rs に出ない。設計は Non-Goals に入れて README で名前を書く形にしている。`crates/dola/Cargo.toml` に `[package.metadata.docs.rs] features = ["toml"]` の 2 行を足せば出るので、討議で足すかどうかを聞いてもよい（要件 2 の「迷わず分かる」に効く）。
- `tools/test-all.ps1` の `.DESCRIPTION` 7 行目の「`cargo test` を回す段（5・6）」は、段を 1 つ足した後は「5・7」になる（新しい 6 は取り寄せが赤でも回る）。設計の「参照も合わせる」の中身はこれ。

## 判定

- **判定: GO**
- **理由**: 既存の作り・依存の向き・公開の形を壊さず、要件のほぼ全部にコードで裏の取れた道がある。残る食い違い（1）は要件の読み方の決めで、設計の作りは変わらない。
- **次にやること**: 設計の討議（`/kiro-design-discussion dola-yaml-backend`）で 1 と 2 を決める。1 を ⑴ で決めたら要件 4.2・4.3 の字面を直す。その後 `/kiro-spec-tasks dola-yaml-backend`。

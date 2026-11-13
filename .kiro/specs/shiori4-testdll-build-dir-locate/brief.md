# Brief: shiori4-testdll-build-dir-locate

## Problem
`cargo test --workspace` を回すたびに、`areka-ghost` と `shiori4-testdll` の 12 本のテストが赤になる。内訳は `areka-ghost` の lib が 5 本、`--test ghost` が 6 本、`shiori4-testdll` の lib が 1 本。どれも「テスト DLL が正準位置に不在」という panic。全体テスト（`tools/test-all.ps1`）が毎回赤になり、どの spec の完了でも「触っていないクレートの赤」として申し送りを重ねることになる。`main` でも同じ。

## Current State
- cargo 1.100.0（2026-11-05）の下で、`cargo test` のテストの exe の置き場所が `target/debug/build/<pkg>/<hash>/out/` に変わった。2026-11-13 に作業木 `dola-yaml-backend` で実測した。
- テスト用の cdylib `shiori4_testdll.dll` も同じく `target/debug/build/shiori4-testdll/<hash>/out/` に出来る。`target/debug/deps/shiori4_testdll.dll` は 11-10 作の古い残りだった。
- DLL を探すのは次の 3 か所で、どれもテストの exe の親ディレクトリに `shiori4_testdll::DLL_FILE_NAME` を join している。
  - `crates/areka-ghost/src/shiori_inproc_tests.rs`（`built_test_dll`）
  - `crates/areka-ghost/tests/ghost/inproc_fixture.rs`（`locate_built_test_dll`）
  - `crates/shiori4-testdll/src/lib.rs`（`built_cdylib_appears_at_single_canonical_deps_dir`。親ディレクトリの名前が `deps` であることも確かめている）
- 元の設計（D-1・設計討議#1）は次のように決めている。「単一の正準位置・glob／mtime／多段フォールバックは採らない（古い DLL を拾って壊れたビルドを隠す silent green を防ぐ）・レイアウトが変わったら spike と導出を再実証する」。今回の赤は、その「見えるように壊れる」が狙いどおり働いた結果である。
- 新しい置き場所は DLL の側の hash 付きのフォルダなので、テストの exe からは導けない。

## Desired Outcome
- `cargo test --workspace` と `tools/test-all.ps1` で、この 12 本が毎回の新しいビルドの DLL を使って緑になる。
- 「単一の正準位置・当て推量なし・置き場所が変わったら見えるように壊れる」の性質は保つ。
- D-1 の記述（3 か所のコメントと spike のテスト）を新しい並びに合わせて改める。

## Approach
要件の段で spike を行い、新しい並びでの単一の正準位置の導き方を実証してから選ぶ。候補は次のとおり（開発者の裁定が要る）。
- cargo の `build.build-dir` などの設定で、テストの exe と cdylib の並びを固定する（`.cargo/config.toml`）。
- テストのクレートの `build.rs`（または `cargo` の成果物の依存）で、cdylib のパスをコンパイル時に渡す。
- ほかに spike で見つかる、cargo が保証する導き方。

## Scope
- **In**:
  - 3 か所の DLL の導き方と、その説明（D-1 の記述）。
  - `shiori4-testdll` の配置を確かめる spike のテスト。
  - 新しい並びの下で、exe の隣を前提にするほかのテスト・道具が動くかの確かめ。たとえば `tools/test-all.ps1` の `i686 helper beside areka.exe` の段、`current_exe()` の隣を見る host-32 の e2e、`sample-ghost-kit` の devroot。
- **Out**: cargo の版を固定すること自体（`rust-toolchain.toml`）は、要件で必要と分かったときだけ扱う。

## Boundary Candidates
- テスト DLL の置き場所の導き方（`shiori4-testdll` が単一の権威を持つ形）。
- ワークスペースの cargo の設定（並びを固定する案のとき）。

## Out of Boundary
- InProc の SHIORI の実装そのもの（`areka-ghost` の `shiori_inproc`）の振る舞い。

## Upstream / Downstream
- **Upstream**: cargo 1.100.0 の成果物の並び。
- **Downstream**: 全体テストを回すすべての spec の完了（今は毎回この 12 本の赤を申し送りにしている）。

## Existing Spec Touchpoints
- **Extends**: 完了済みの、InProc の SHIORI のテスト DLL の spec（D-1・設計討議#1 を決めたもの。要件の段で `completed/` から名前を引き直す）。
- **Adjacent**: `audit-areka-ghost` 系の見直し（同じクレートを触るなら同じウェーブに並べない）。

## Constraints
- 起票: 2026-11-13・`/kiro-complete dola-yaml-backend` の冒頭の棚卸から（`dola-yaml-backend` の差分とは関係ない）。その場で直さなかった理由: 直し方が複数あり、元の設計の決まり（単一の正準位置・当て推量なし）に照らした開発者の裁定が要る。
- 優先度の区分: B（バグ。全体テストが毎回赤になる）。
- テストは実 GPU・実機を要さない（crate のテストで確かめられる）。

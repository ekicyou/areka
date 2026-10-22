# 設計の検証: areka-P0-property-name-case-fold

> 検証日 2026-10-22・本ブランチ（`21be9007`）。design.md・requirements.md・spec.json を読み、設計の主張を実物のコードで確かめた（steering は design.md が引く決まり〔兄弟テスト・ログの無い失敗の禁止〕の範囲で参照）。手順は `kiro-validate-design`（分析 → 大事な問題 → 良い点 → GO／NO-GO）。

## まとめ

設計は小さく筋が良い。名前を鍵の綴りへ直す道がもともと `parse_dotted` の 1 本で、そこ（`parse_seg`）で半角の英字を畳み、書き込みの仕分けの 1 段目だけを大小を区別しない比べにすれば、読み・自由な名前の書き・書けない名前の判定・保存された値の読みが同じ決まりにそろう。実物で確かめた限り、畳みで壊れる既存のテストや呼び手は無い。残る問題は、畳みで元の綴りが消えることを後の spec へ申し送る行が 1 つ足りない点と、小文字の約束が説明だけで守られている点で、どちらも実装を止めるものではない。

## 実物で確かめたこと

1. **鍵を作る道は 1 本。** 鏡像の点付きの区画へ書くのは `actor.rs` の受信ループの `SetDottedGlobal`／`SetDottedPerAsker` と起動時の `build_initial_image` だけ。前者の鍵は `canonicalize_or_raw`（静的な値）・`StoreWrite` の腕（自由な名前）がどちらも `parse_dotted` → `to_canonical_string` で作り、`PersistPut` と `build_initial_image` は `PersistKey::to_canonical_key` を使う。`to_canonical_key` の綴り（`areka.window.scope(N).x|y`・`areka.boot.count` など。軸は `Axis::as_str` の `x`／`y`）は全部小文字。`parse_dotted` を通らずに大文字の鍵が入る道は無い。
2. **読む口は名前を手直ししない。** MCP の `get_property::handle` は `args.property_name` を、`ShioriHostSink_Impl::GetProperty` は `key.to_string()` を、そのまま `resolve_dotted_str` へ渡す。`areka-ghost/src/runtime.rs` の起動回数・消滅回数の読みも `to_canonical_key` の小文字の綴りで聞く。
3. **既存のテストは大文字の点付きの名前を使っていない。** `crates/areka-sylphya/src`・`shiori_host.rs`・`mcp/get_property_tests.rs`・`placement/zorder_property_deferral_tests.rs`・`areka-ghost/src/sylphya_wiring.rs` の文字列を引き直し、英字の大文字を含む点付きの名前は 0 件。`PropPath`／`PathSeg`／`Selector::ByName` を手で組むのは `ledger_key_determinism_tests.rs` と `key.rs` の `mod tests` だけで、どれも小文字。効果やログの綴り（元の綴り）を期待するテストも無い。＝`parse_seg` で畳んでも既存の期待は変わらない（要件 5.5）。
4. **語彙の表はすべて小文字。** `DOTTED_ROOTS`・`GENERIC_PROP_NAMES`・`SET_EFFECTIVE`（`vocab/dotted.rs`）に英字の大文字は無い（`char*.bind.menu`・`shiori.変数名` も不変の検査 `s == s.to_ascii_lowercase()` を満たす）。
5. **テストの道具はそろっている。** `spawn_sylphya`（`SylphyaInit` に `io` を差せる）・`FakePersistIo`（`persist/io.rs`）・`save_scope`・`crate::test_log_capture::capture`・`spawn_test_sylphya_sink`・`reads_values_through_the_ghost_own_asker_on_a_real_runtime` の `names` の並びは実在する。兄弟テストを `#[path = …]` で繋ぐ形も `actor.rs` に既にある。設計の検体の名前（`test.key`・`my.flag`・`path.to.key`・`test(ABC).key`）は、根が `DOTTED_ROOTS` に無く葉が `GENERIC_PROP_NAMES` に無いので、意図どおり自由な名前（`StoreWrite`）になる。
6. **行数。** `actor.rs` 799 行・`key.rs` 411 行・`shiori_host.rs` 628 行。足すのは数行で、1,000 行の目安に収まる。

## 大事な問題（2 件）

### 問題 1: 畳みで元の綴りが消えることの申し送りが `shiori.変数名` だけ

- **中身**: 決めたこと 2 で `PropPath` は元の綴りを持たない。「Revalidation Triggers」はその影響を `shiori.変数名` の値を足す spec にだけ書いている。だが同じ影響を受ける後の持ち主がほかに 2 つある。⑴ `activeghostlist(…).ext.拡張プロパティ名`・`pluginlist(…).ext.*`（`vocab/dotted.rs` が `property.get`／`property.set` の名前を予約している）を相手のゴーストやプラグインへ中継する spec は、`PropPath` から取ると畳んだ名前を渡してしまう。⑵ `property-catalog-lists` は括弧の中の名前で一覧から選ぶ。括弧の中は畳まれて届く（`ghostlist(Emily)` → `emily`）ので、ゴーストの名前と完全一致で比べると外れる。
- **影響**: 今は値が無いので害は無い。だが後の spec が気付かずに `PropPath` を使うと、SSP で当たる聞き方が areka で外れる（要件 1.2 の決まりが後で崩れる）。
- **直し方**: 「Revalidation Triggers」に 1 行足す: 「括弧の中の名前で選ぶ spec（`property-catalog-lists`）と `ext.` の亜枝を中継する spec は、`PropPath` の名前が畳まれていることを前提に、選ぶ相手とは大小を区別せずに比べる（`eq_ignore_ascii_case`）か、元の綴りが要るなら読む口から受けた生の文字列を使う」。コードは変わらない。
- **要件**: 1.2・Boundary Context の細部 3／7
- **根拠**: design.md「Revalidation Triggers」・「設計で決めたこと」2

### 問題 2: 「`PropPath` の名前は小文字」の約束が説明だけで守られている

- **中身**: `PropPath`・`PathSeg`・`Selector` は公開の型で、各欄も公開（`lib.rs` で再輸出）。手で大文字の `PropPath` を組んで `SylphyaReader::resolve_dotted` へ渡すと、何の記録も無く「無い名前」になる。設計はこれを説明の約束と「Revalidation Triggers」の 1 行で扱う。
- **影響**: 今の呼び手はクレートの中のテストだけで実害は無い。ただし、外れたときに赤にも記録にもならない＝「ログの無い失敗の道」に近い形が 1 つ残る（`logging.md`）。
- **直し方**: どちらかで足りる。⒜ 今の設計のまま説明の約束だけにする（呼び手が増えるまで待つ）。⒝ `resolve_dotted` の頭に `debug_assert!`（全部の名前で `s == s.to_ascii_lowercase()`）を 1 行足し、テストで大文字の手組みを捕まえる。⒝ は本番の動きを変えず数行で済む。どちらを採るかは設計の議論で決める。
- **要件**: 1.1・5.5
- **根拠**: design.md「Boundary Commitments」の Revalidation Triggers 最後の行・「名前の畳み」Service Interface の Postconditions

## 良い点

1. **1 か所で畳む形が、守りの穴を構造で塞ぐ。** 仕分けの 2 段目（`is_canonical_vocab`）も `StoreWrite` の鍵も同じ `parse_dotted` の結果を見るので、畳みを足すのは `parse_seg` だけで済み、1 段目だけが生の文字列を見るという事実も実物どおりに押さえている。`MENU` を 1 段目で畳み忘れると 2 段目の `menu` に当たって分類が変わる、という落とし穴まで檻で捕まえる設計になっている。
2. **範囲の線が明確で、触らない物を実物で裏づけている。** 読む口・読み手・保存は変えず、保存の名前がもともと小文字であること、本番の結線に sylphya の `GetProperty` が無いことを確かめたうえで、要件 2.2 の証拠を決定論テストと静的な構造に置いた。新しい関数・型・依存を足さない。

## 判定

**GO**

- **理由**: 既存の作りと向きが合い、要件 1〜6 のすべてに部品とテストが対応している。実物で確かめた限り、畳みで壊れる呼び手やテストは無い。残る 2 件はどちらも後の spec への申し送りと守りの強さの話で、実装の道を塞がない。
- **次の手**: 設計の議論（`kiro-design-discussion`）で問題 1（申し送りの 1 行を足すか）と問題 2（⒜ 説明だけ／⒝ `debug_assert!` を足す）を決める。その後 `/kiro-spec-tasks areka-P0-property-name-case-fold`。

# 設計の検証（emo-balloon-merge-log-overlap）

> 2026-11-28・本ブランチ（main `b05e652f` の上）の `design.md` を、コード（`balloon.rs`・`balloon_definitions.rs`・`crates/areka-placement/src/lib.rs`・`boot_shell_tests.rs`・`crates/areka-runtime/src/assets.rs` の読みだけ）と突き合わせて検証した。対話なしで判定まで出す。

## 判定の要約

設計は前例（`resolve_balloon_faces_quiet`・`load_balloon_use_self_alpha_quiet` の「中身 1 本＋薄い入口」）に沿い、触るファイルはウェーブ C29 の約束 ⑹ の内に収まり、依存も `Cargo.toml` も変えない。コードと照らした主張（関数の位置・可視性・型・行の数）はおおむね正しく、実装の道ははっきりしている。直すべきは、資産の組み立てが面 0 の定義まで届かない起動での警告の行方の記述と、行の数の檻の「非空虚性」の書き方の 2 点で、どちらも設計の討議で文書を直せば済む。

## コードとの突き合わせ（確かめた主張）

| 主張 | 結果 |
|---|---|
| `balloon.rs` 793 行 → 830〜845 行 | 妥当（出さない入口 約 8・`_with` 約 10・`merge_scope_layers` への移しと `if records.emits()` +2・守りの腕と引数 +2・テストの宣言 +3） |
| `lib.rs` 896 行 → 915〜919 行 | 妥当。`use areka_emo_present::balloon::{…}` は 4 つの名前で中の行が 104 桁になり 1 行 → 4 行（+3）、`use areka_parsers::balloon::{BalloonModel, …}` は 78 桁で 1 行、`filter_map` の行はちょうど 100 桁、をそれぞれ数え直して一致した。`load_scope_windowpositions` の説明の 1 行目の書き足しが +1 になる余地は表に無いが、上限 1,000 の内 |
| 出さない道でも失敗の記録が残る（logging.md・要件 3.4） | 正しい。`resolve_balloon_faces_with` の面 0 の `error!`（398）は `records` の判定（410）より前、走査の失敗の `error!` も Quiet で出る。`scope_windowposition` の見送りの `warn!` 3 つは `merge` の選択に依らない |
| 約束 ⑹ のファイル（`assets.rs`・`ghost_session.rs`・emo-text `tests/support/mod.rs`）に触らない | 正しい。`balloon_definitions.rs`（e1）と `switch_assets.rs` も変えない。触るのは `balloon.rs`・`balloon_model_tests.rs`・`lib.rs`・`boot_shell_tests.rs` と新しい兄弟 2 つだけ |
| テストが private の関数に届く | 届く。`placement_balloon_merge_log_tests.rs` は crate の根の子のモジュールなので private の `load_scope_windowpositions_with` を呼べる。`balloon_merge_quiet_tests.rs` は `balloon` の子なので private の `_with`・`pub(crate)` の `test_support`（`TempDir`・`ALPHA`・`CapturedEvent`）を使える |
| テストが使う公開の名前 | 在る。`areka_emo_present::balloon_definitions::load_scope_balloon_definitions`（pub）・`ScopeBalloonDefinitions::face0()`・`FaceDefinition.model`（pub）・`areka_emo_atlas::{MemoryDecoder, AlphaParams, UseSelfAlpha}`（根から pub use）・`areka_parsers::balloon::BalloonModel`（`PartialEq`）・`ResolvedFace`（pub・`PartialEq`） |
| `boot_shell_tests.rs` の改め | 正しい。`LoadLines::per_boot` の `merge: scopes(2 * n)`（157）を `scopes(n)` にすれば 3 本の期待が scope ごとに 1 行になる |
| 関数ポインタ `fn(&Path, u32, &ResolvedFace) -> BalloonModel` | 2 つの入口の署名がそのまま型に合う |

## 重大な指摘

### 🔴 指摘 1: 資産の組み立てが面 0 の定義まで届かない起動では、2 層マージの警告が 0 行になる

- **問題**: 起動の準備の窓の位置の読みを黙らせると、2 層マージの記録を出すのは資産の組み立て（`load_scope_balloon_definitions`）だけになる。ところが `wire_runtime` は `build_shell_assets` を先に通し（`assets.rs:253` の前）、`build_balloon_assets_with_limits` は scope ごとに系列の解決 → 焼く段（`?`・491〜497）→ 定義の読み（512）の順に回す。シェルの資産の焼く段が落ちる、または scope 0 のバルーンの焼く段が落ちる（採寸は面 0 しか焼かないので、ほかの面の絵が壊れていれば採寸は通る）と、その起動では基層が読めない `warn!`・上書き層が読めない `warn!` が、今の scope ごとに 1 行（窓の位置の読みが出す分）から 0 行になる。設計の「記録の無い失敗の道は構造上生まれない」（Error Handling）と「警告を 0 行にしない」（要件 3.2 の行）は、この場合を書いていない。
- **影響**: 止まった起動の主因は焼く段の `error!` で記録されるので記録の無い失敗にはならないが、別の問題（読めない `descript.txt` など）の警告が、その起動の記録から消える。要件 3.2 の字面とずれる。
- **提案**: 直し方は案 1 のまま（警告だけを窓の位置の読みに残すと、資産の組み立ての分と合わせて 2 行に戻る。資産の組み立ての順を変えるのは約束 ⑹ の外）。Error Handling と要件 3.2 の行に「資産の組み立てが当該 scope の定義の読みより前で止まった起動では、2 層マージの警告は出ない。止めた失敗は自分の `error!` を出す」を書き、受け入れるかどうかを討議で決める。
- **対応する要件**: 3.2・3.5
- **根拠**: design.md「Error Handling」「Requirements Traceability」の 3.2 の行・`crates/areka-runtime/src/assets.rs:489〜512`

### 🔴 指摘 2: 行の数の檻の「非空虚性」は `lib.rs:411` の結線を確かめない

- **問題**: 行の数の檻 1（`placement_balloon_merge_log_tests.rs`）は、`prepare_stages_for_shell` を通さずに `load_scope_windowpositions_with(dir, &[0, 1], load_scope_balloon_model_quiet)` を直に呼んで、起動の準備の読みを組み立て直す。そのため「起動の準備の読みを出す入口へ戻すと面 0 の行が 2 行になって赤（6.5）」は 411 の書き戻しでは起きない。赤になるのはテストの引数を書き換えたときだけ。411 の書き戻しを捕まえるのは `boot_shell_tests.rs` の本番の順の檻だけで、それも emo2 の確定の `info!` の数だけを見る（警告と `debug!` の 3 種は本番の順では数えない）。
- **影響**: 前提 2 でこの分担は書かれているが、Testing Strategy の非空虚性の文が本当より強いことを言っている。要件 6.1（「起動の準備 1 回について」）と 6.5 の結び付けを読み違えやすい。
- **提案**: 檻 1 の非空虚性の文を「窓の位置の読みの入口を出す入口に替えると面 0 の行が 2 行になって赤。411 の結線は本番の順の檻が確定の `info!` の数で支える（4 種の記録は同じ入口が切り替えるので、`info!` の数で結線の全部が決まる）」に直す。本番の順の檻で 4 種のうち `info!` だけを数えることで足りる理由（4 種が同じ `records` の分岐で切り替わる）を Testing Strategy に 1 行足す。
- **対応する要件**: 6.1・6.5・6.6
- **根拠**: design.md「Testing Strategy」の行の数の檻 1・「前提」の 2

## 設計の良いところ

1. **前例の形をそのままなぞる**: `LoadRecords` を crate の中に閉じたまま核まで通し、記録の行だけを `records.emits()` と守りの腕で囲む。読み・マージ・戻り値の分岐は `records` に依らないので、出す入口と出さない入口の戻り値の一致が構造で保たれ、テストでも丸ごとの `assert_eq!` で確かめられる。`areka-placement` の側も関数ポインタで入口を選ぶだけで、pub の `load_scope_windowpositions` の形とシェルの着替えの呼び手を 1 行も変えない。
2. **境界の守りが具体的**: 約束 ⑹ の外に出ないために e1（`balloon_definitions.rs` を変えない）・t2・u1 を選んだことと、その代わりに失うもの（u1 は `switch_assets.rs` の結線を檻に入れない等）を「前提」に正直に書いている。行の数の見込みも rustfmt の折り返しまで数えてあり、コードで数え直して一致した。

## 最終の判定

- **判定: GO**
- **理由**: 構造の食い違いも要件の抜けも無く、実装の道ははっきりしている。2 つの指摘はどちらも設計の文書の記述（警告の行方の明記と、非空虚性の書き方）で済み、コードの形は変わらない。
- **次の手**: 設計の討議（`/kiro-design-discussion emo-balloon-merge-log-overlap`）で指摘 1 の受け入れと指摘 2 の書き直しを決め、`design.md` の「前提」の 5 項目と合わせて確定する。その後 `/kiro-spec-tasks emo-balloon-merge-log-overlap`。

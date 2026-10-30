# 設計の確かめ: default-balloon-selfmade

> 2026-10-29 `/kiro-validate-design`（対話なしで実行・答えは開発者に聞いていない）。読んだもの: spec.json（`ja`・`design-generated`）・requirements.md・design.md・research.md・steering（product・tech・structure）・`.claude/skills/kiro-validate-design/rules/design-review.md`。design.md の主張のうち実装の成否に効くものは、実際のコードで確かめた（下の「コードでの確かめ」）。

## まとめ

設計は要件 1〜13 をすべて表で担い手と確かめに結び付けており、本番コードの変更を定数の値 1 つと登記の行 1 つに絞る境界も、既存の型（`NarBuilder`・`SampleRoot`・`WicDecoderArm`・`TextRegion`）をそのまま使う形で守られている。コードと突き合わせた主張はすべて事実どおりで、実装に入れる。大きな穴は 1 つだけで、道具と追跡している `.nar` のずれを、人が `--check` を回さない限り誰も見つけられない点である。

## コードでの確かめ

| design.md の主張 | 確かめた所 | 結果 |
|---|---|---|
| `NarBuilder::new().file(..).done()`・`bytes()`・`install_txt` が公開 | `crates/sample-ghost-kit/src/lib.rs` 699 行目の `pub use`・`nar_writer.rs` | 合う。`file` の既定は UTF-8 の印あり・無圧縮。`bytes()` は足した順に並べ、**並べ替えない**ので、道具の側で名前順に並べてから足す必要がある（設計の「名前順に固定し」と合う） |
| `install_txt` は CRLF で連結し末尾にも CRLF | `nar_writer.rs` 306 行目 | 合う |
| `WicDecoderArm` の `probe_animation`・`decode_frames` を外のテストから呼べる | `crates/areka-emo-atlas/src/lib.rs` 250 行目の `pub use decode::wic_arm::WicDecoderArm`・`ElementDecoder` は公開の trait | 合う。`decode_frames` は `image_arm::read_frames`（crate の中だけの関数）へ回すので、`pub(crate)` に頼らずに読める |
| `AnimationInfo` の幅・高さ・コマ数、`AnimationFrame::delay_ms`、`AnimatedImage::loop_count`、`DecodedImage::has_alpha` | `crates/areka-emo-atlas/src/decode.rs` | どれも `pub` の欄。`LoopCount` も公開 |
| `areka-emo-atlas` の `image` は `png`・`webp` を持つ | `crates/areka-emo-atlas/Cargo.toml` | 合う（`default-features = false`・`features = ["png", "webp"]`）。dev-dependency に `sample-ghost-kit` と `log-capture-kit` も在る |
| 中身のテストに要る dev-dependency が `areka-emo-text` に揃っている | `crates/areka-emo-text/Cargo.toml` | 合う（`sample-ghost-kit`・`areka-emo-atlas`・`log-capture-kit` は dev、`areka-parsers`・`areka-emo-present`・`windows` は通常の依存） |
| `examples/gen_default_balloon/main.rs` を cargo が自動で拾う | `crates/areka-emo-atlas/Cargo.toml` に `autoexamples = false` も `[[example]]` も無い | 合う（cargo は `examples/<名>/main.rs` を自動で拾い、同じフォルダの他の `.rs` は `mod` で読む子になる） |
| 1,000 行の番人が example も数える | `crates/log-capture-kit/tests/file_length_guard_test.rs` の説明（`crates/**/*.rs`・`examples/` を含む） | 合う。4 ファイルに分ければ余裕がある（前例 `gen_animated_samples.rs` は 476 行） |
| 見張り ⑵ の綴りを `Path::join` で避けられる | `crates/log-capture-kit/tests/sample_path_guard_test.rs`（走査語は登記表から組む `vendors/sample_ghost/<名>/` と `…/<名>"`） | 合う。⑷ は同梱バルーンの名前だけが対象で、`areka-balloon`（同梱なしの `Balloon`）には当たらない |
| 一時のファイルを `target\` の下に置く | `temp_path_guard_test.rs` は `std::env::temp_dir` の呼び出しだけを見る | 当たらない（道具は `CARGO_MANIFEST_DIR` から根を出す。`fold-samples.rs` 297 行目と同じ形） |
| 本番の読み口が統合テストから呼べる | `resolve_balloon_faces`・`load_scope_balloon_model`・`load_balloon_use_self_alpha`・`build_balloon_target_from_faces`（`areka-emo-present/src/balloon.rs`）、`ResolvedBalloonText::resolve`（`actor.rs`）、`TextRegion::is_collapsed`・`wrap_threshold`・`inline_limit`・`start`（`region.rs`） | どれも `pub`。`staysee_balloon_fixture` が同じものを使っている |
| 描画範囲の解け方（本体側 0・0・326・149・折り返し基準 296、相方側 0・0・326・82・296） | `region.rs` の `resolve`（負の値は反対の辺から・`wordwrappoint.x` は画像の幅から） | 計算が合う。`wrap_threshold`（296）≦ `inline_limit`（326） |
| 既定の名前の定数と段 6 | `crates/areka/src/boot_resolve.rs` 34 行目・326 行目 | 合う。`pub(crate)` なので同じ crate のテストから `crate::boot_resolve::DEFAULT_BALLOON_FOLDER` で引ける（`boot_config_option_tests.rs` が `DEFAULT_GHOST_FOLDER` を同じ形で使っている） |
| 登記の数のテスト 10 → 11 | `crates/sample-ghost-kit/src/lib_tests.rs` 209・265・501 行目ほか | 直す所の見立てと合う |
| 配布の道具の 4 か所 | `tools/package.ps1` 467・496・504・570 行目 | 合う |

## 大事な指摘（1 件）

### 指摘 1: 道具と追跡している `.nar` のずれを、仕組みとして見つけられない

- **何が問題か**: 「同じ入力から同じバイト列」（要件 8.2）を確かめる手段は、道具の `--check` を人が回すことだけである。中身のテスト `areka_balloon_fixture` は、わざと道具の `texts.rs` を参照せず要件から手で書いた期待値と比べるので、道具の絵柄や文を直して `.nar` を作り直し忘れても、テストは古い `.nar` のまま緑になる。`image`・`image-webp` の版が上がった場合（`/update` の作業など）も、`--check` が違いを出すことに誰も気付かない。
- **なぜ効くか**: 要件 8.2 の狙い「作り直して差分が出たら、入力が変わったと分かる」は、比べる人がいて初めて働く。ずれたまま時間がたつと、次に絵を直す人が道具を回した瞬間に、意図しない差分（版の違いによるもの）が混ざって見分けにくくなる。配布物そのもの（`.nar`）は中身のテストが守るので、利用者への害は無い。害は「道具が真実を語らなくなる」ことに限られる。
- **直し方の案**:
  - 案 A（推す）: `crates/areka-emo-atlas/tests/` に統合テストを 1 本足し、`#[path = "../examples/gen_default_balloon/draw.rs"]` などで道具の子のファイル（`draw.rs`・`webp.rs`・`texts.rs`）をそのまま読み込み、メモリで作った 23 ファイルを `SampleRoot::acquire("areka-balloon")` で取り出した木とファイルごとにバイトで比べる。`Cargo.toml` を変えず、見張り ⑵ の綴りも書かない。そのために、組み立ての手順（ファイル名とバイト列の並びを返す関数）を `main.rs` でなく子のファイルに置く。`image` の版が上がると赤になるが、それは Revalidation Triggers が求める「作り直して `.nar` を差し替える」合図そのものである。
  - 案 B: 仕組みは足さず、tasks.md の道具のタスクと最後の確かめ（`/kiro-validate-impl`）に「`--check` が `identical` を出す」を確かめの項目として明記する。今回の作業の間のずれは防げるが、着地の後のずれは防げない。
- **要件との結び付き**: 要件 8.2・8.9・12.1。
- **設計の該当所**: 「System Flows / 道具の流れ」・「Batch の契約」の冪等・「Revalidation Triggers」の 1 つ目・「Testing Strategy / 中身のテスト」の `test_support.rs`（道具を参照しない）。

## 小さな注記（議題にしなくてよい・実装で気を付けること）

- `online.rs`（と、`online0.webp` の寸法を見る `assets.rs`）は `WicDecoderArm::new()` を呼ぶ。中で `CoCreateInstance` を呼ぶので、`bake.rs` と同じく先に `CoInitializeEx`（MTA）を張らないと失敗する。`probe_animation`・`decode_frames` 自体は WIC を使わない。
- `online.rs` の「ほかの点の中心の α が 64」は、4×4 標本で中心の画素が点に全部入る（点の半径 4 に対し、画素の標本は中心から 1.3 px 以内）ので 0.25 × 255 = 63.75 を丸めた値になる。丸めの式を `draw.rs` と食い違えないこと（切り捨てなら 63 になる）。
- `NarBuilder::bytes()` は足した順に書く。名前順は道具の側で並べてから足す。
- 要件 6.5 の 3 つの警告は、読み込みの関数でなく文字の層の登録口（`actor_attach.rs`）が書く。設計が「警告の条件と同じ判定」で固定する形は、既存の `shipped_fixture_region_test.rs` と同じ流儀で妥当。実機の記録の grep が本物の経路を見る。

## 良い所

1. **本番への変更が極小で、既存の窓口と型に乗っている**。道具は書庫を書くだけ、読む側（テスト・配布の道具・areka）は今までどおり窓口から名前で引く。新しい読み口・依存・`Cargo.toml` の変更が 0 で、見張り（検体の在処・一時のパス・1,000 行）にも当たらない形を、コードの事実を確かめたうえで選んでいる。
2. **既定の差し替えを最後の 1 タスクにまとめ、直前に main を取り込む順**と、既定に頼るテストを「定数を参照し、名前を文字で比べる 1 本を残す」形に直す決めが、次に既定を替える時の手間と「定数の値を誤っても気付かない」穴の両方を塞いでいる。research.md 3.1 の A・B・C の仕分けも、コードの綴りの数と合っている。

## 判定

- **判定: GO**
- **理由**: 要件はすべて担い手と確かめに結び付き、設計の主張はコードの事実と合っている。指摘 1 は道具の真実性の問題で、配布物の正しさは中身のテストが守るため、実装を止める理由にならない。設計の討議で案 A か案 B を選べば足りる。
- **次の手順**: 設計の討議（`/kiro-design-discussion default-balloon-selfmade`）で指摘 1 の案を決め、決めた形を design.md に反映してから `/kiro-spec-tasks default-balloon-selfmade` へ進む。

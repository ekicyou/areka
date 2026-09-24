# 設計レビュー: areka-P0-keycolor-clickthrough-coverage

> 2026-09-23。対象は本ブランチ HEAD `c10b7101`（製品コードは main `92f5f448` と同一）の `design.md`。設計が引用した関数名・可視性・置き換え対象の行・検体の数値は、すべて実物を Grep／Read で照合した（下の「照合の記録」）。本レビューは非対話で行い、判定を直接出す。

## レビュー要約

設計は「抜き色で透明になった場所のクリックが背後へ抜けることを固定するテスト」を、既存の公開 API と既存のテスト補助だけで、製品が実際に通る順（読む・焼く → 合成する → マスクを作る）どおりに 1 本で通す形になっている。引用された経路・型・可視性・置き換え対象の 3 行・検体の外形と抜き色は、すべて実物と一致した。製品コードの変更は 0 行で、足すのは接続宣言 3 行と、テスト専用ファイルの可視性の書き換え 3 行だけであることも確認できた。実装へ進んでよい。

## 重大な問題

**0 件**。実装を止める食い違いは見つからなかった。

以下は重大ではないが、実装とタスク生成のときに知っておくとよい注意点（設計の修正は要らない）。

1. **焼く段の差し替えで赤になる既存テストの見込みに、`crates/areka-emo-compose` 側が挙がっていない**。設計の表は atlas と present の 3 群を挙げるが、compose の `base_image_tests.rs` は `konnoyayame` の実物を読む（`sample_test_support::konnoyayame_shell_root` の使い手）。走らせ方は 3 クレートの `cargo test` なので取りこぼしは起きない。要件 2.2 のとおり、実際に赤になったものをそのまま記録すればよい（見込みの表を正解として写さない）。
2. **`crates/areka/examples/collision-probe.rs` の説明文の doc リンク**（``[`build_shell_target`]`` → ``[`load_shell_assets`]``）は、改名後の関数が `collision-probe/setup.rs` の私有関数なのでリンク先として解決しない可能性がある。`structure.md` のとおり `cargo doc` は関門ではないので、文言を新しい名前へ改めるだけでよい（要件 3.3 の「旧名を指したまま残さない」は満たす）。
3. **差し替えを戻す手順の `git checkout -- <ファイル>`** は、対象 3 ファイル（`normalize.rs`・`blit.rs`・`show.rs`）に本仕様の意図した変更が 1 行も無いからこそ安全である。実装中にこれらのファイルへ別の変更を置かないこと（置いた場合は戻す手順ごと消える）。

## 設計の強み

1. **正解の決め方が製品の 4 段（正規化・焼き・合成・マスク）に依らない**。同じ PNG を復号した生の画素で「左上と 4 バイト一致」を数え、加えて `has_alpha == false`・`0 < 抜かれた画素 < 全画素`・`マスクの外形 = PNG の外形` を先に主張してから全画素を突き合わせる。復号器が壊れて全画素が同色になる形も、絵が α 付きに差し替わる形も、前提の主張で原因が名指しされる。
2. **差し替え 3 段がいずれも「経路から外す」形で、かつテスト自身の主張（要件 1.4）で赤になる**ことを机上で追える。焼く＝抜き色を無しに置き換える（全画素が不透明のまま → 抜かれた画素の数だけ食い違う）、合成＝転写した画素の α を 255 で書く（トリム後の矩形の内側の抜かれた画素が「内」になる。実測で 3 面とも 0 より大きい）、マスク＝全画素不透明のバッファを渡す（全画素が「内」）。3 段とも、前提の主張（`bake_errors` 空・`has_alpha` 偽・較正・外形）は通り抜けて、食い違いの主張で止まる。

## 最終判定

**GO**。

- 根拠: 既存の構造（兄弟ファイル＋`#[cfg(test)] #[path]` の接続宣言・検体は `sample_ghost_kit::SampleRoot` 経由・GPU 付き World は `make_world_with_gpu`）に沿い、新しい公開 API・新しい依存・新しい補助を 1 つも足さない。可視性の壁の解き方（`pub(crate) mod test_support;` 1 行＋関数 2 本）は Rust の可視性の規則どおりで、`#[cfg(test)]` の付いた項目の中に収まる。要件 1〜5 のすべてに対応する設計要素が在り、実装の手順（タスク 3〜4 本）が読み手に明らかである。
- 次の手順: `/kiro-spec-tasks areka-P0-keycolor-clickthrough-coverage` でタスクを生成する。差し替えの記録の形式（設計の表 ⑴〜⑹）は tasks.md の完了記録の欄として最初から用意しておくとよい。

## 照合の記録（設計の主張 → 実物）

すべて Grep／Read で確かめた。cargo のビルド・テストは走らせていない。

### 通る経路

| 設計の主張 | 実物 | 一致 |
|---|---|---|
| `pub fn load_shell_target(shell_dir: &Path, decoder: &impl ElementDecoder) -> Result<ShellTarget, ShellLoadError>` | `crates/areka-emo-present/src/shell_target.rs` の同名の定義 | ○ |
| `build_shell_target` が `AlphaParams { use_self_alpha: UseSelfAlpha::On }` 固定で `bake` を呼ぶ | 同ファイルの `SurfaceSet` の組み立てと `bake(std::slice::from_ref(&set), decoder, PackConfig::default())` | ○ |
| `bake` が `Normalizer::key_color` を正規化の前に呼び `debug!` を出す | `crates/areka-emo-atlas/src/lib.rs` の `bake`（`let key_color = Normalizer::key_color(&decoded, set.alpha_params, has_pna);` → `Normalizer.normalize(...)` → `if let Some([b, g, r, a]) = key_color { tracing::debug!(...) }`） | ○ |
| `normalize` の `(UseSelfAlpha::On, AlphaSource::KeyColor)` の腕が `if *px == key { *px = [0, 0, 0, 0]; }` | `crates/areka-emo-atlas/src/normalize.rs` の同じ腕 | ○ |
| `ShellTarget::build_world(&self) -> EmoWorld`（何度でも呼べる）・`atlas(&self) -> &AtlasTable`・`bake_errors(&self) -> &[BakeError]` | 同 `shell_target.rs` の `impl ShellTarget` | ○ |
| `AtlasTable` は `#[derive(Clone)]`（`Arc` 共有・安価） | `crates/areka-emo-atlas/src/table.rs` の `#[derive(Clone, Debug)] pub struct AtlasTable` | ○ |
| `EmoPresenter::new()`・`attach_target(&mut self, _world, target, window, emo_world, atlas, author_dpi: u16)`・`apply(&mut self, world, cmd)` | `crates/areka-emo-present/src/presenter/hub.rs` | ○ |
| `apply_show` が `target.budget.native_scratch(|scratch| target.composer.compose_into(scratch, ...))` で合成 | `crates/areka-emo-present/src/presenter/show.rs` | ○ |
| `blit.rs` の `pub(crate) fn execute` が出力先を全透明にクリアしてから矩形の内側を転写し、`dst[di + 3] = source_over_channel(src_a, dst_a, inv_src_a);` | `crates/areka-emo-compose/src/blit.rs`（`out.resize_and_clear(extent.w, extent.h)` → `uv_rect` の内側だけを歩く） | ○ |
| `record_display`（CPU → GPU の転送）→ 席の交代 → `target.budget.regenerate_mask(retired_mask, display.bytes(), display.width(), display.height(), display.stride())` → `cache.insert` → `set_shared(entry.mask.clone())` | 同 `show.rs` | ○ |
| `FrameBudget::regenerate_mask` → `MaskRotation::regenerate` → `AlphaMask::from_pbgra32`／`regenerate_from_pbgra32` | `crates/areka-emo-present/src/presenter/budget.rs`（両方 `pub(super)`） | ○ |
| `AlphaMask::is_hit`（α ≥ 128・範囲外は false）・`width()`・`height()`・`#[derive(PartialEq)]` | `crates/wintf/src/ecs/widget/bitmap_source/alpha_mask.rs`（`ALPHA_THRESHOLD = 128`・`#[derive(Debug, Clone, PartialEq)]`） | ○ |
| `AlphaMaskResource::set_shared(Arc<AlphaMask>)`・`mask() -> Option<&AlphaMask>`・`alpha_mask_hit` が同じ資源を最優先で読む | `crates/wintf/src/ecs/layout/hit_test/mod.rs` | ○ |
| GPU からの読み戻しは経路に無い | `show.rs` に読み戻しの呼び出しは無く、`record_display` の転送だけ | ○ |

テストが呼ぶのは `load_shell_target` → `attach_target` → `show_ok`（＝本物の `apply` に `PresentCommand::ShowSurface`）で、合成やマスク生成を直接呼ぶ近道は無い。

### 可視性

| 設計の主張 | 実物 | 一致 |
|---|---|---|
| `shell_target.rs` の受け口は `#[cfg(test)] #[path = "shell_target_test_support.rs"] mod test_support;`（私有） | 同ファイル末尾の接続宣言 | ○ |
| `r_post_and_komainu_shell_dir`・`konnoyayame_shell_dir` は `pub(super) fn` | `shell_target_test_support.rs` | ○ |
| 受け口の説明文は使い手 4 本を列挙している | 同ファイル冒頭（`load_tests`・`base_image_tests`・`template_tests`・`emo2_tests`） | ○ |
| `presenter.rs` は `#[cfg(test)] #[path = "presenter_test_support.rs"] mod test_support;` を持ち、`make_world_with_gpu`・`spawn_window_with_dpi`・`show_ok`・`mount_entities` は `pub(super)` | `presenter.rs`・`presenter_test_support.rs` | ○ |
| 新しいテストを `presenter.rs` の末尾に接続すれば `presenter` の兄弟になり `super::test_support::…` と `use super::*;`（`EmoPresenter`・`TargetId`・`AlphaMaskResource`・`World` 等）が届く | `presenter_budget_equivalence_tests.rs` が同じ形で `AlphaMaskResource` と `super::test_support::{make_world_with_gpu, show_ok, spawn_window_with_dpi, ...}` を使っている | ○ |
| `mod` 宣言を `pub(crate) mod` にしないと関数を `pub(crate)` にしても届かない | Rust の可視性の規則（モジュールの経路ごとに検査）どおり。`lib.rs` は `pub mod shell_target;` なので `crate::shell_target::test_support::…` の経路は `mod` の可視性だけで決まる | ○ |
| `<stem>_<モジュール名>` の命名で前向きの衝突が無い | `crates/areka-emo-present/src/` に `presenter_keycolor…` から導出しうる別の本番ファイルは無い（一覧で確認） | ○ |

### 製品コードの変更行数（要件 1.10）

- `presenter.rs` に足す接続宣言 3 行は `#[cfg(test)]` の付いた項目。
- `shell_target.rs` の `mod test_support;` → `pub(crate) mod test_support;` は、既存の `#[cfg(test)]` 属性の付いた項目（属性は残す）。
- `shell_target_test_support.rs` は `#[cfg(test)]` の `mod` からだけ繋がるテスト専用ファイル。
- examples 2 本の改名と説明文 2 か所は `crates/areka` の examples（要件 1.10 の 4 クレートの外・要件 3.3 が許す）。

したがって「`#[cfg(test)]` の付いた項目の外で変わった行」は 0、接続宣言 3 行、可視性の書き換え 3 行。設計の表と一致する。

### 差し替え 3 段（要件 2）

| 段 | 設計が引用した行 | 実物 | 赤になる理由の妥当性 |
|---|---|---|---|
| 焼く | `let key = Self::key_color(&img, params, has_pna);` → `let key: Option<[u8; 4]> = None;` | `normalize.rs` の抜き色の腕にそのまま在る。`params`・`has_pna` は同じ関数の `select_source` で使われ続けるのでコンパイルは通る | 全画素が α=255 のまま焼かれ、トリムは全域を残し、合成もマスクも全画素「内」。正解（復号だけから決める）は変わらないので、食い違い＝抜かれた画素の数。前提の主張（`bake_errors` 空・`has_alpha` 偽・較正・外形）は通り抜ける |
| 合成 | `dst[di + 3] = source_over_channel(src_a, dst_a, inv_src_a);` → `dst[di + 3] = 255;` | `blit.rs` の転写ループにそのまま在る | 転写はトリム後の矩形の内側しか書かないので、不透明になるのは矩形の内側だけ。矩形の外の抜かれた画素は 0 のまま「外」で正解と一致し、矩形の内側の抜かれた画素だけが「内」になって食い違う。研究の実測で 3 面とも 0 より大きい（24,959／8,712／51,984） |
| マスク | `regenerate_mask(retired_mask, display.bytes(), …)` の第 2 引数 → `&vec![255u8; display.bytes().len()]` | `show.rs` にそのまま在る。`display` は局所変数で、`&Vec<u8>` は `&[u8]` へ自動で合う | 全画素が「内」になり、食い違い＝抜かれた画素の数 |

道連れの赤の見込み: `crates/areka/src` に `AlphaMaskResource`・`mask()`・`is_hit` を読むテストは 0 件なので、3 クレートの `cargo test` で道連れはすべて観測できる（`crates/areka` 側は外形しか見ない）。

### 正解の復号器（要件 1.3）

- `areka_emo_atlas::WicDecoderArm`（`lib.rs` で再輸出）・`ElementDecoder::decode(&self, &Path) -> Result<DecodedImage, DecodeError>`・`DecodedImage { width, height, stride, bgra, has_alpha }` は全欄 `pub`。`areka-emo-atlas` は `areka-emo-present` の通常の依存なので、テストから届く。
- `WicDecoderArm::new` は `CoCreateInstance` を呼ぶだけで、COM は「呼び出し側が初期化済み」が前提（同関数の説明文）。`make_world_with_gpu` が `CoInitializeEx(None, COINIT_MULTITHREADED)` を呼び解放しないので、その後に `new` を呼ぶ順序は安全。`with_com_initialized`（末尾で `CoUninitialize`）を使わない判断も妥当。
- パレット形式の PNG: WIC の変換前フォーマット（8bpp Indexed）は α 付きの集合に無いので `has_alpha = false`、`IWICFormatConverter` が `GUID_WICPixelFormat32bppPBGRA` へ展開する。製品の抜き色の判定も同じ `DecodedImage` を入力にするので、「左上と 4 バイト一致」の集合は復号の段で一意に決まり、パレットの添字と色の違いは原理的に出ない。復号は要件が「導かない」と定めた 4 段の外である。

### 検体の実測（要件 1.6・Introduction の表）

- `shell_target_template_tests.rs` の定数: `R_POST_SURFACE0 = (236, 462)`・`R_POST_SURFACE10 = (140, 160)`・`KONNOYAYAME_SURFACE0 = (260, 390)`・`KONNOYAYAME_SURFACE10 = (200, 200)`。同ファイルが `load_shell_target` 経由で実物を焼き、面 0 の左上の α=0（抜き色が効いている）と、面 10 が `surfaces.txt` に波括弧を持たないこと（`is_declared` の較正）を既に固定している。
- `crates/areka/src/placement/measure_template_tests.rs` も同じ 4 つの外形を採寸経路・表示経路の双方で固定している。
- 「先頭 5 件」の失敗の文言は `konnoyayame_blink_frame_changes_pixels_only_inside_the_frame_rect` の前例と同じ形。

### 相乗り 3 件（要件 3）

| 件 | 設計の主張 | 実物 | 一致 |
|---|---|---|---|
| ① | `crates/areka-emo-compose/src/sample_test_support.rs` は `emo2_root`・`konnoyayame_shell_root` の 2 口（`pub(crate)`）で `#[test]` は 0 本 | そのとおり。`emo2_root` は `folder()`、`konnoyayame_shell_root` は `folder().join("shell/master")` | ○ |
| ② | `measure_template_tests.rs` は 336 行、走査対象は `measure.rs` と `../emo2_boot/assets.rs` の 2 本、`assert_no_blind_spot_literals` の説明文に「走査対象の 4 ファイルは今日どちらも 0 件」 | そのとおり。examples 3 本の本文に `shell::parse`・生文字列（`r"`・`r#`）・二重引用符の文字リテラルは 0 件（正規表現で確認） | ○ |
| ③ | 私有 `fn build_shell_target` は `examples/emo-present/setup.rs`（定義・呼び出し・説明文）と `examples/collision-probe/setup.rs`（同）に在り、旧名の言及が `examples/collision-probe.rs` の冒頭説明文と `examples/window-placement.rs` の説明文にも在る | Grep で同じ 4 ファイル・同じ箇所を確認。`load_shell_assets` はリポジトリ内（`.kiro` を除く）で 0 件 | ○ |

### 変えないもの（要件 4）と報告の文言（要件 5）

- 設計は `normalize.rs` の判定・`MaskRotation`・既存の両端のテスト・`shell_target_template_tests.rs`・wintf 側のどれにも変更を置かない（Out of Boundary と Modified Files の表で明示）。
- 設計本文に「CPU → GPU の転送は起きる」が 4 か所在り、「GPU に触れない」「キャラクターの外」「実機の確認は不要」は 0 件。プロジェクト内の符牒も 0 件。

### steering との整合

- `structure.md`「Unit Tests」: 兄弟ファイル＋`#[cfg(test)] #[path]` の接続宣言・`<stem>_<モジュール名>.rs`・共有補助は `<stem>_test_support.rs` に集約——いずれも守られている。
- `tech.md`: 依存の追加は承認事項（`encoding_rs`・`miniz_oxide` の前例）。設計は新しい依存を 0 件とし、`image` crate の案を退けている。
- `workflow.md`: 同一ブランチ上で各フェーズをコミットし、`main` への統合は完了時の PR のみ。設計は分岐やブランチ操作に触れない。

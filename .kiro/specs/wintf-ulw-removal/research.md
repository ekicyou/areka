# ギャップ分析（Gap Analysis）: wintf-ulw-removal

> 種別: 削除／リファクタ spec（純粋な ULW 撤去）。本分析は「何を消し、どこが追随し、何を絶対に巻き込まないか」の全数洗い出しと、collapse の設計選択肢の提示に主眼を置く。
> 前提: requirements.md / spec.json は確定済み（本分析では変更しない）。言語=ja。
> 調査日: 2026-07-03（コードベース実査）。

---

## 1. サマリ（3-5 bullets）

- **既存パターン**: 表示バックエンドは `CompositionMode`（`ULW`＝既定／`DComp`）の 2 択で、生成時 `compute_ex_style()` が分岐し、ECS スケジュールでは ULW 系（`compositor_init_system`＋`composite_render_system`＋`ulw_present_system`）と GPU 合成系（`init_window_graphics`＝WUC）が **同一スケジュールに共存**し、各 system が `composition_mode()` を実行時チェックして「自モードでない Window を `continue` でスキップ」する二重経路構造。ULW 専用コードは 3 ファイル（`ecs/graphics/compositor.rs`・`compositor_systems/`・`com/ulw.rs`）に凝集しており、切り出し境界は明瞭。
- **欠けている capability は無い（削除 spec）**: 新規実装は不要。残す GPU 合成（WUC）経路は `wintf-dcomp-to-wuc-migration`（完了）が確立済みで、`init_window_graphics` は既に `CompositionMode::DComp` のみを処理する。撤去のコアは (a) 3 ファイル削除 (b) 4 スケジュール登録アンカーの除去／再配線 (c) `CompositionMode` collapse に伴う全呼び出し追随、の 3 点。
- **候補アプローチ**: `CompositionMode` collapse に **3 案**（enum 完全撤去／単一 variant 化／内部保持だけ削って公開 API 維持）。トレードオフは「破壊的変更の広さ」対「二度手間の回避」（後続 `wintf-dcomp-to-wuc-migration` は完了済みだが `areka-P0-emo-present`／`-window-placement` が同 API に着手予定）。
- **非破壊の安全網（巻き込み禁止領域）を実査で確認**: クリックスルー経路（`win_style.rs::apply_layered_companion()`／`ecs/clickthrough/controller.rs`）は α 源に per-widget `AlphaMask`（`hit_test_in_window`）のみを用い、ULW compositor の staging α バッファへ **一切依存しない**。WUC 保全集合（`com/wuc.rs`・`ecs/graphics/wuc_resource.rs`・`visual_manager.rs`・`systems/init.rs::init_window_graphics`）も撤去対象外。削除ブラスト半径はこれらに触れない。
- **リサーチフラグ**: 外部依存調査は不要（削除のみ・新規依存ゼロ）。要注意点は「schedule 再配線時に GPU 合成側 system 順序を byte-for-byte 保つ」「`tick_order_tests` の 13 スケジュール固定列を壊さない（`CommitComposition` を空にするか system を差し替えるかで扱いが変わる）」「テストハーネス（`tests/graphics.rs`・`tests/window.rs`）の `#[path]` モジュール宣言の除去」。

---

## 2. 要件→資産マップ（Requirement-to-Asset Map）

各要件を実コードの撤去／追随対象へ対応づけ、状態を（Delete=削除 / Rewire=再配線 / Follow=追随 / Preserve=不変 / Doc=文書）でタグ付けする。全パスは wintf/areka crate 内の相対で、絶対パスは §3 に記載。

### Requirement 1: ULW 専用描画経路の撤去
| 対象シンボル/ファイル | 状態 | 実査メモ |
|---|---|---|
| `ecs/graphics/compositor.rs`（`WindowD3D11Compositor`） | Delete | ファイル全体が ULW 専用（D2D1 合成先＋staging＋GDI HBITMAP＋MemoryDC＋DIBSection の 4 リソース管理）。DComp 非依存で他モードから参照なし。 |
| `ecs/graphics/compositor_systems/`（`init.rs`・`render/mod.rs`・`render/traverse.rs`・`render/guards.rs`・`mod.rs`） | Delete | ディレクトリ丸ごと ULW 専用。`compositor_init_system`・`composite_render_system`・`ulw_present_system` を含む。 |
| `com/ulw.rs`（`transfer_to_hbitmap`・`present_layered_window`） | Delete | ECS 非依存の純粋ユーティリティ。`UpdateLayeredWindow` 呼出はここのみ。参照元は `compositor_systems/render/mod.rs` の `use crate::com::ulw::{...}` 一箇所。 |
| `ecs/graphics/mod.rs` の `pub mod compositor;`／`pub mod compositor_systems;`（4-5 行目） | Rewire | mod 宣言 2 行の除去。 |
| `com/mod.rs` の `pub mod ulw;`（5 行目） | Rewire | mod 宣言 1 行の除去。 |

**Req1.4（推測で消さない）**: 撤去対象と変更内容を事前提示するプロセス制約。requirements/design ではなく impl 時の手順ゲート。gap 分析としては「削除対象は上表で確定・過不足なし」を提供する。

### Requirement 2: ULW system の ECS スケジュール登録解除
| アンカー（`ecs/world/mod.rs`） | 状態 | 実査メモ |
|---|---|---|
| `GraphicsSetup`: `compositor_init_system.after(init_window_graphics)`（259-266 行） | Rewire | `init_window_graphics` との `.after()` チェーンから ULW system を外す。**残すべきは `init_window_graphics` 単独登録**（GPU 合成側の順序不変）。 |
| `Composition`: `composite_render_system.after(clip_sync_system)`（321-332 行） | Rewire | Composition チェーン末尾の ULW system を外す。**先行 3 system（`visual_hierarchy_sync`→`visual_property_sync`→`clip_sync`）は WUC 側で不変**。 |
| `CommitComposition`: `ulw_present_system` 単独登録（337-340 行） | Rewire | **設計判断点**: この schedule は現状 `ulw_present_system` のみ。撤去後 CommitComposition は「空スケジュール」になる。schedule 自体は残す（`tick_order_tests` の 13 本固定列・後掲）か system を消すだけか、が争点（→§4 決定項目 D3）。 |

**Req2.2（GPU 合成側スケジュール不変）**: `PreRenderSurface`／`RenderSurface`／`Composition`（ULW system を除く 3 本）／`init_window_graphics` の登録・順序を撤去前と同一に保つ。**Preserve**。
**Req2.3（ビルド通過・矛盾なし）**: 削除後の schedule が空 World tick で矛盾しないこと。`tick_order_tests`（13 本の実行順検証・後掲）が回帰検知器になる。

### Requirement 3: CompositionMode の collapse
| 呼び出し箇所 | 状態 | 実査メモ |
|---|---|---|
| `ecs/window/components.rs`（`CompositionMode` enum 定義・99-108 行／`Window::default`＝ULW・138 行／`WindowStyle::default`＝`WS_EX_LAYERED`・175 行） | Delete/Rewire | enum 本体＋既定値。`WindowStyle::default` の `ex_style: WS_EX_LAYERED` は ULW 既定の名残（→ collapse 後は GPU 合成既定へ）。in-source `#[cfg(test)]` が ULW 既定を hard-assert（263・273 行等）。 |
| `runtime/window_factory.rs::compute_ex_style()`（64-73 行） | Rewire | Req4 の主対象（後述）。`match composition_mode` の ULW アーム除去。in-source test 3 本が ULW/DComp 分岐を検証（294-336 行）。 |
| `ecs/window_proc/lifecycle.rs::WM_PAINT`（48-71 行）／`WM_ERASEBKGND` コメント（24 行） | Follow | `composition_mode() == DComp` で分岐し、else が「ULW モード=BeginPaint/EndPaint」。collapse 後は DComp 単独ゆえ **DComp 分岐（`DefWindowProcW` 委譲）へ一本化**、ULW フォールバック分岐を除去。 |
| `ecs/clickthrough/controller.rs` in-source test（922-940 行） | Follow | test ヘルパ `spawn_live_window` が `CompositionMode::DComp` を使用。production コードは `CompositionMode` を参照しない（clickthrough は mode 非依存）。 |
| `crates/areka/src/main.rs`（225・292 行 `composition_mode: CompositionMode::DComp`／29 行 import） | Follow | areka 側の 2 窓（shell/balloon）指定。collapse 方式次第で書き換え or 削除。 |
| `crates/areka/src/tests.rs`（101-118 行） | Follow | `assert_eq!(window.composition_mode(), CompositionMode::DComp)` の 2 テスト。 |
| `crates/wintf/tests/window/composition_mode_test.rs`（7 occ） | Follow/Delete | `default_is_ulw` 等、ULW 既定を hard-assert。collapse で意味喪失。 |
| `crates/wintf/tests/window/find_owner_composition_mode_test.rs`（7 occ） | Follow | `find_owner_window_composition_mode` は **production symbol ではない**（test が同ロジックを再実装・grep で src 内に定義なし）。ULW 既定前提の assert を更新。 |
| `graphics` テスト群（`compositor_init_system_test`・`compositor_integration_test`・`compositor_lifecycle_test`・`compositor_opacity_test`・`compositor_render_system_test`・`compositor_transfer_test`・`dcomp_integration_test`・`init_window_graphics_test`） | Delete/Follow | compositor_* 6 本は ULW compositor 専用 → 削除候補。dcomp/init_window_graphics 2 本は WUC 側 → CompositionMode 参照のみ追随。 |
| `tests/graphics.rs`（12-23 行の `#[path]` 宣言 6 本）／`tests/window.rs`（2-5 行） | Rewire | 削除するテストファイルのモジュール宣言除去。 |
| examples: `multi_backend_demo.rs`（ULW 窓生成・88/115/136 行）・`clip_demo.rs`（`create_ulw_clip_window`・87/262/282 行）・`dcomp_demo.rs`・`dcomp_taffy_demo.rs`・`postmessage_click_test.rs`（ULW present 言及コメント） | Follow/Delete | ULW を明示指定する example は collapse で壊れる。`multi_backend_demo`（ULW/DComp 二本立てが主題）と `clip_demo` の ULW 窓関数は要判断（削除 or DComp 化）。 |

**Req3.5（areka のビルド通過）**: areka 側 `CompositionMode::DComp` 指定の追随責務は本 spec に帰属（Boundary Context 明示）。

### Requirement 4: compute_ex_style の分岐一本化
| 対象 | 状態 | 実査メモ |
|---|---|---|
| `runtime/window_factory.rs::compute_ex_style()`（64-73 行） | Rewire | 現状 `match`: ULW→`style.ex_style`（`WS_EX_LAYERED`）／DComp→`(ex_style & !WS_EX_LAYERED) \| WS_EX_NOREDIRECTIONBITMAP`。**collapse 後は DComp アーム 1 本へ一本化**（`WS_EX_LAYERED` を落とし `WS_EX_NOREDIRECTIONBITMAP` を付与）。生成時 `WS_EX_LAYERED` を付けない挙動を保つ（Req4.3）。 |
| in-source test（294-336 行の `ex_style_ulw_keeps_layered`・`ex_style_dcomp_*`） | Follow | ULW test を削除、DComp test を残す（＝一本化後の唯一経路の回帰検知）。 |
| `WindowStyle::default().ex_style = WS_EX_LAYERED`（components.rs 175 行） | Follow | ULW 既定の名残。GPU 合成では factory が `WS_EX_LAYERED` を落とすため実害は無いが、既定値の意味整合として要検討（コメント 173-174 行も ULW 前提）。 |

### Requirement 5: クリックスルー機構の非破壊（WS_EX_LAYERED 帰属移行）
| 対象 | 状態 | 実査メモ（**巻き込み禁止の実証**） |
|---|---|---|
| `win_style.rs::apply_layered_companion()`（401-424 行）・`apply_click_through()`（362-388 行） | Preserve | 撤去対象外。ULW 撤去後、GPU 合成窓への `WS_EX_LAYERED` の唯一源。 |
| `ecs/clickthrough/controller.rs::evaluate_targets`（145-222 行）・`prune_dead_targets`・controller 全体 | Preserve | α 源は `hit_test_in_window`（per-widget `AlphaMask::is_hit`）のみ。`compositor.rs` の staging α バッファを **参照しない**（controller に compositor import 皆無を実査確認）。Req5.4 充足。 |
| `ecs/clickthrough/registry.rs`（`layered_applied` フラグ） | Preserve | LAYERED 同伴フラグの状態管理。 |

### Requirement 6: 残す GPU 合成パスの描画非破壊
| 対象 | 状態 | 実査メモ |
|---|---|---|
| WUC 保全集合（`com/wuc.rs`・`ecs/graphics/wuc_resource.rs`・`visual_manager.rs`・`systems/init.rs`・`systems/render.rs`・`systems/window_pos.rs`） | Preserve | Out of scope（別 spec 完了物）。 |
| ワークスペース `Cargo.toml`（`opt-level='z'`・`lto=true`・94/96 行／`panic='unwind'`・100 行） | Preserve | Req6.4 の互換性ターゲット。撤去がこれらと衝突しないこと（LTO 有効時の dead-code 削除で問題が出ないかはビルド検証で確認）。 |
| 当たり判定・ウィンドウ管理・スレッド構成 | Preserve | Req6.3。撤去は描画層のみ。 |

### Requirement 7: ドキュメント残余の整合更新
| 対象 | 状態 | 実査メモ |
|---|---|---|
| `doc/COMPAT_ARCHITECTURE.md`（44 行「ULW透過」・99 行・105 行・108 行「非スコープ（残置）: ULW アーム…除去は別 spec `wintf-ulw-removal`」） | Doc | 正本。ULW を残存機構として前提する記述を GPU 合成単独へ整合。特に 108 行は本 spec 完了で「別 spec で除去」→「除去済み」へ更新。 |
| wintf コード内コメント（`compositor` 関連コメント・`lifecycle.rs:24`「ULW が全画面管理」・`components.rs:168-176`「ULW 透過ウィンドウ…」・`window_factory.rs:18-19` docstring 等） | Doc | 撤去ファイル自体は消えるが、残るファイル（lifecycle・components・window_factory・world/mod.rs のスケジュールコメント）内の ULW 前提記述を整合。 |
| steering（`tech.md`・`product.md`・`roadmap.md`）の「ULW 一択」記述 | Preserve（Req7.3） | 2026-07-01〜03 に更新済みゆえ **本 spec では再更新しない**。gap: `tech.md:83`／`roadmap.md:30` に ULW 記述が残るが steering 領分。 |

---

## 3. 撤去・追随対象の絶対パス一覧（impl 参照用）

### 完全削除候補（ファイル単位）
- `C:\home\maz\git\areka\crates\wintf\src\ecs\graphics\compositor.rs`
- `C:\home\maz\git\areka\crates\wintf\src\ecs\graphics\compositor_systems\`（`mod.rs`・`init.rs`・`render\mod.rs`・`render\traverse.rs`・`render\guards.rs`）
- `C:\home\maz\git\areka\crates\wintf\src\com\ulw.rs`
- テスト（ULW compositor 専用・要判断）: `crates\wintf\tests\graphics\compositor_init_system_test.rs`・`compositor_integration_test.rs`・`compositor_lifecycle_test.rs`・`compositor_opacity_test.rs`・`compositor_render_system_test.rs`・`compositor_transfer_test.rs`

### 編集（再配線・追随）
- `C:\home\maz\git\areka\crates\wintf\src\ecs\graphics\mod.rs`（mod 宣言 4-5 行）
- `C:\home\maz\git\areka\crates\wintf\src\com\mod.rs`（mod 宣言 5 行）
- `C:\home\maz\git\areka\crates\wintf\src\ecs\world\mod.rs`（スケジュール登録 259-266・321-332・337-340 行＋コメント 256-257・334-336 行）
- `C:\home\maz\git\areka\crates\wintf\src\ecs\window\components.rs`（`CompositionMode` 定義・既定値・`WindowStyle::default`・in-source test）
- `C:\home\maz\git\areka\crates\wintf\src\runtime\window_factory.rs`（`compute_ex_style` 64-73 行・docstring 18-19 行・in-source test）
- `C:\home\maz\git\areka\crates\wintf\src\ecs\window_proc\lifecycle.rs`（`WM_PAINT` 分岐・`WM_ERASEBKGND` コメント）
- `C:\home\maz\git\areka\crates\wintf\src\ecs\clickthrough\controller.rs`（in-source test ヘルパのみ）
- `C:\home\maz\git\areka\crates\wintf\tests\graphics.rs`（`#[path]` 宣言）・`crates\wintf\tests\window.rs`（`#[path]` 宣言）
- `C:\home\maz\git\areka\crates\wintf\tests\window\composition_mode_test.rs`・`find_owner_composition_mode_test.rs`
- `C:\home\maz\git\areka\crates\wintf\tests\graphics\dcomp_integration_test.rs`・`init_window_graphics_test.rs`（CompositionMode 参照追随）
- examples: `crates\wintf\examples\multi_backend_demo.rs`・`clip_demo.rs`・`dcomp_demo.rs`・`dcomp_taffy_demo.rs`・`postmessage_click_test.rs`
- `C:\home\maz\git\areka\crates\areka\src\main.rs`（29・225・292 行）・`crates\areka\src\tests.rs`（101-118 行）
- `C:\home\maz\git\areka\doc\COMPAT_ARCHITECTURE.md`（44・99・105・108 行）

### 絶対不変（Preserve・巻き込み禁止）
- `C:\home\maz\git\areka\crates\wintf\src\win_style.rs`（`apply_layered_companion`・`apply_click_through`）
- `C:\home\maz\git\areka\crates\wintf\src\ecs\clickthrough\`（controller production コード・registry）
- `C:\home\maz\git\areka\crates\wintf\src\com\wuc.rs`・`ecs\graphics\wuc_resource.rs`・`ecs\graphics\visual_manager.rs`・`ecs\graphics\systems\init.rs`（`init_window_graphics`）
- steering（`tech.md`・`product.md`・`roadmap.md`）

---

## 4. 実装アプローチの選択肢（Options A/B/C）

削除 spec のため「拡張 vs 新規 vs ハイブリッド」ではなく、**`CompositionMode` collapse の方式**が主たる設計選択となる。

### Option A: `CompositionMode` enum を完全撤去（最大 collapse）
GPU 合成が唯一のモードになるため、`CompositionMode` 型・`Window.composition_mode` フィールド・`composition_mode()` メソッド・全 `match`／分岐を消し、`compute_ex_style()` は引数なしで GPU 合成 ex_style を返す純関数へ。`WM_PAINT` は DComp 分岐へ無条件一本化。
- ✅ 二択 API が完全消滅・分岐が最小・「選択肢のない切替 API」の無意味さを解消（Req3.2 の enum 撤去に最も忠実）
- ✅ 将来 `CompositionMode` を見た読者の誤解が消える
- ❌ 破壊的変更が最大（areka の `Window { composition_mode: ... }` を全削除・`Window` の構造体リテラルが変わる）
- ❌ `areka-P0-emo-present`／`-window-placement`（同 API に着手予定）との rebase 衝突面が最大

### Option B: 単一 variant へ最小化（`CompositionMode::Wuc` のみ）
enum は残すが variant を GPU 合成 1 つに減らす。`Window.composition_mode` フィールドと `composition_mode()` は維持。`compute_ex_style()` は単一アームに。
- ✅ 構造体 API（`Window` のフィールド構成）が不変ゆえ areka/後続 spec の追随が最小（既定値変更のみ）
- ✅ 将来モード追加（例: 別合成方式）への拡張点を温存
- ❌ 「単一 variant の enum」は意味的に冗長（Req3.2 が明示する「単一なら enum 撤去」の趣旨とやや逆行）
- ❌ 分岐は消えても型の存在自体は残り、単純化が中途半端

### Option C: ハイブリッド（内部分岐削除＋段階的縮小）
Phase 1 で ULW variant と全 ULW 分岐・3 ファイルを撤去し `CompositionMode` を単一 variant（Option B 状態）に落とす → ビルド・描画等価を確認 → Phase 2 で enum 撤去（Option A 状態）まで縮小。areka／後続 spec の追随タイミングを Phase 境界で調整。
- ✅ 「描画非破壊の検証点」と「API 破壊の追随」を分離でき、リグレッション切り分けが容易
- ✅ 後続 spec との順序調整余地（Phase 2 を後続着手後に回せる）
- ❌ 2 段階ゆえコミット数・レビュー面が増える（ただし本ブランチ内で随時コミット可＝squash で消える）
- ❌ 中間状態（単一 variant enum）が一時的に冗長コードとして残る

**gap 分析としての整理（決定はディスカッションへ委ねる）**:
- 純粋単純化の目的最優先なら **Option A**。
- 後続 spec との衝突最小・安全側なら **Option B or C**。
- brief の「クロスユニット契約」は「順序調整が理想（本ユニット先行→emo/ghost が新 API で書く）」と述べており、**先行できるなら A**、**並行なら追随責務の帰属確定が前提**。

---

## 5. 設計決定項目（要件ディスカッションへ送る・番号付き）

1. **CompositionMode collapse 方式（A/B/C）**: enum 完全撤去 / 単一 variant 化 / 段階的。Req3.2 は「単一 variant なら enum 撤去 or 最小化」と両許容。areka・後続 spec の追随コスト・rebase 衝突と天秤。
2. **`Window.composition_mode` フィールドの去就**: Option A ではフィールド自体を削除（構造体リテラル破壊）。B/C では既定値を GPU 合成へ変えるのみ。areka `main.rs`（225/292 行）と `tests.rs` の書き換え量が変わる。
3. **`CommitComposition` スケジュールの去就**: 撤去後この schedule は唯一の system（`ulw_present_system`）を失い空になる。(a) schedule を残し空にする（`tick_order_tests` の 13 本固定列を維持・順序不変テストを壊さない）か、(b) schedule 自体を削る（13→12 本になり `tick_order_tests` の `EXPECTED_ORDER`・件数 assert を更新）か。**後者は tick 順序の不変条件テストの改変を伴う**ため、影響が Req2 の範囲を超える点に注意。
4. **`WindowStyle::default().ex_style` の既定値**: 現状 `WS_EX_LAYERED`（ULW 名残・components.rs 175 行）。GPU 合成では factory が落とすため実害は無いが、既定を `WS_EX_NOREDIRECTIONBITMAP` 相当へ整合させるか、`WINDOW_EX_STYLE(0)` にするか、現状維持か（コメント 168-176 行も要整合）。純粋非破壊を厳格に取るなら「既定値は変えず factory 側で一本化」が安全。
5. **ULW example の去就**: `multi_backend_demo.rs`（ULW/DComp 二本立てが主題）・`clip_demo.rs::create_ulw_clip_window`。削除するか DComp 単独へ書き換えるか。二本立てが主題の example は削除が自然だが、clip 検証など DComp でも成立する部分は残す判断があり得る。
6. **ULW compositor 専用テストの去就**: `compositor_*_test.rs` 6 本は撤去対象コードの単体テスト＝削除が原則。ただし `compositor_transfer_test`・`compositor_opacity_test` 等に「描画等価の観測ロジック」で流用可能な資産があれば、WUC 側の非破壊検証（Req6.1/6.2）へ転用するか要判断。
7. **描画非破壊（Req6.1/6.2）の検証手段**: 「撤去前後で同一描画」をどう担保するか。areka 起動の目視・既存 WUC 側テスト・スクリーンショット比較のいずれを受け入れ基準に採るか（削除 spec ゆえ新規テスト追加は最小限が原則だが、非破壊の実証手段は design で確定要）。
8. **`find_owner_window_composition_mode` の実在確認**: 該当 test（`find_owner_composition_mode_test.rs`）は同名 production 関数を想定するが、src 内に定義が見当たらず test がロジックを再実装している。collapse に際し、この test の存在意義（ChildOf チェーン走査）を残すか（mode 非依存の走査ロジックとして）判断が要る。

---

## 6. 工数・リスク

- **工数見積り**: **M（3-7 日）**。ファイル削除自体は S だが、collapse に伴う追随が wintf 本体＋areka＋examples＋tests＋doc に横断し、schedule 再配線と `tick_order_tests`／in-source test の整合、描画非破壊の検証が加わるため M。Option A（enum 撤去）はやや上振れ、B は下振れ。
- **リスク**: **Low〜Medium**。
  - Low 要因: 撤去コードは 3 ファイルに凝集・境界明瞭、残す WUC 経路は独立確立済み、クリックスルー非依存を実査で確認済み、新規技術・外部依存ゼロ。
  - Medium 要因: (1) schedule 再配線で GPU 合成側の system 順序を byte-for-byte 保つ必要（`Composition` チェーンの末尾除去・`GraphicsSetup` の `.after()` 解除）。(2) `CommitComposition` を空にするか削るかで `tick_order_tests`（13 本固定列）への波及が変わる（決定項目 D3）。(3) 描画非破壊の「同一性」証明が目視依存になりやすい。(4) LTO 有効ビルドでの dead-code 除去・リンク（Req6.4）は撤去後に実ビルド検証が必要。

---

## 7. デザインフェーズへの引き継ぎ

- **推奨方針**: collapse は brief の「順序調整が理想（本 spec 先行）」に沿えば Option A が単純化目的に最も忠実。後続 spec と並行着手が確定なら Option B/C で追随面を絞り、追随責務の帰属を着手時に明記（brief クロスユニット契約）。
- **リサーチ持ち越し（Research Needed）**:
  - `CommitComposition` 空スケジュール vs schedule 削除の判断（`tick_order_tests` 波及範囲の確定）。
  - 描画非破壊（Req6.1/6.2）の受け入れ基準の具体化（目視／既存テスト／スクショ比較）。
  - LTO 有効・release 最適化ビルドでの撤去後リンク検証（Req6.4）。
- **外部依存調査**: 不要（削除のみ・新規依存追加なし）。

---

> 本 gap 分析は情報と選択肢の提示であり、最終的な実装選択は要件ディスカッション／design フェーズで確定する。requirements.md・spec.json は本分析で変更していない。

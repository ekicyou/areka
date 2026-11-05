# 設計の検証: audit-wintf-graphics-world

> 2026-11-05・`/kiro-validate-design`（対話なしで実行）。読んだもの: `spec.json`・`requirements.md`（「指摘の扱い」を含む）・`design.md`・`research.md`・steering（`product.md`・`tech.md`・`structure.md`）・実コード（`crates/wintf/src/ecs/world/mod.rs`・`world/tick_gate_tests.rs`・`world/tick_diag_tests.rs`・`world/tick_gate.rs`・`graphics/systems/init.rs`・`graphics/systems/window_pos.rs`・`graphics/systems/render.rs`・`graphics/systems/surface.rs`・`graphics/components.rs`・`graphics/command_list.rs`・`graphics/visual_manager.rs`・`dola/mod.rs`・`com/d3d11.rs`・`widget/shapes/rectangle.rs`・`crates/wintf/tests/` の一覧）・依存のソース（`bevy_ecs` 0.19.1 `schedule/executor/single_threaded.rs`・`system/function_system.rs`）・`tools/test-all.ps1`。

## 設計の検証の要約

設計は、直すと決めた 1〜21・31〜35 番と 30 番の説明を、全部「指摘ごとの直し方」の表に載せ、呼び出し元と範囲の外のファイルまで挙げている。実コードと依存のソースで、主な主張（`run_if` が偽の回は係の `last_run` が進まない・`init_window_graphics` は WUC を作った回に早く戻る・無効化の係は今は複数スレッドの `Update` に居る・`DolaAnimator` の読む口が `&self`・`world/mod.rs` 944 行）はどれも成り立つ。ただし 6 番（デバイスロスト）の流れに、立ち直りの回に作ったばかりの描画面を消してしまう穴が 1 つあり、ほかに 2 点を設計に書き足せば実装へ進める。

## 確かめたこと（主張とコードの照合）

| 主張 | 確かめた所 | 結果 |
|---|---|---|
| 失いを `BeginDraw` の失敗で見つけ、`GetDeviceRemovedReason()` の包みが在る | `render.rs` の `surface_interop.begin_draw(None)` の `Err` の枝（今は `error!` して次へ）。`com/d3d11.rs` 45〜58 行の `get_device_removed_reason(&self) -> Result<()>` | 成り立つ（戻りは `Result<()>` なので、`is_device_lost` の `removed_reason: HRESULT` は `Err` の `code()` から作る） |
| `init_graphics_core` は `PreLayout`、無効化の係は複数スレッドの `Update` | `world/mod.rs` 245 行（`Update` の chain の中）・257 行（`PreLayout`）。`GraphicsSetup` は 152〜156 行で単一スレッド | 成り立つ |
| `run_if` が偽の回は `last_run` が進まず、変化の印が残る | `single_threaded.rs` の `run`: 条件を先に評価し、偽なら `continue`（係を回さない）。`last_run` は `function_system.rs` の実行の中でだけ書かれる | 成り立つ |
| `.chain()` で命令の反映が間に入る | bevy 0.19 の既定（自動の `apply_deferred` の挿入）。`init_graphics_core` は `Commands` を持つ | 成り立つ |
| `init_window_graphics` は WUC を作った回に早く戻り、`Changed<HasGraphicsResources>` を食べる | `init.rs` 222〜245 行（`insert_resource` の後に `return`） | 成り立つ（設計の調べの見つけは正しい） |
| `DolaAnimator` の読む口は `&self` で、`Sync` の根拠が崩れている | `dola/mod.rs` 113・120 行（`last_result(&self)`・`runtime(&self)`）・86〜87 行の `unsafe impl`。`Changed<DolaAnimator>` を見る係は木の中に無い（`&mut` にしても読むたびに印が立つ害は今は無い） | 成り立つ |
| `compile_fail` の型のテストが全体テストで回る | `tools/test-all.ps1` 66 行は `cargo test --workspace`（doctest を含む）。`crates/wintf/Cargo.toml` に `doctest = false` は無い。前例 `crates/sample-ghost-kit/src/lib.rs` 272 行ほか | 成り立つ |
| 単一スレッドの塊は 6 つ | `world/mod.rs` 134〜178 行（`UISetup`・`GraphicsSetup`・`PreRenderSurface`・`RenderSurface`・`Composition`・`CommitComposition`） | 成り立つ |
| `world/mod.rs` の行数と 1,000 行 | 944 行。設計の見込みは約 770 行。ほかの触るファイルは最大 322 行（`init.rs`）で、どれも 1,000 行から遠い | 成り立つ |
| `EcsWorld::new` の本文を読む構造の検査の前例 | `world/tick_diag_tests.rs` 161・187 行（`include_str!("mod.rs")`） | 成り立つ |
| 範囲の外で直すファイルの一覧 | `tests/` の `WindowGraphics::new`・`init_graphics_core`・`d2d_factory()` などの使い手を探した。`core_ecs_test.rs`・`core_accessor_test.rs`・`d2d_ext_test.rs` は形が変わらないので触らなくてよい。`component_state_pattern_test.rs` の `WindowGraphics::new(1)` は自前の写しの型 | おおむね成り立つ（抜けは下の議題 3） |
| 依存を足さない・`doc/`・`docs/` を名指す行を足さない | Technology Stack と File Structure Plan に新しい依存は無い。steering の書き換えも `doc/` を名指さない | 成り立つ |
| 直す指摘がもれなく載っている | 「指摘ごとの直し方」の表に 1〜21・30〜35 番が全部ある。22〜29 番と 30 番の README は申し送り | 成り立つ |
| バグと危ない所に、元に戻すと落ちるテスト（要件 3.2） | Testing Strategy に 1〜9・11・20・32・35 番。10 番（記録の無い失敗）は記録を足すだけの配線なので、テストを置かないのは「判断の分かれ目だけを固定する」決めと合う | 成り立つ |
| タスクの数 | 見立ては 21。開発者の裁定（1〜2 の超えは分ける理由にしない）の内 | 成り立つ |

## 重大な議題

**重大な議題 1**: 立ち直りの回に、作ったばかりの描画面を `cleanup_surface_on_commandlist_removed` が消してしまう
**懸念**: 設計は無効化の係（`GraphicsSetup` の先頭）で `GraphicsCommandList` を全部外し、同じ回の `Draw` で四角と画像の係が `Without<GraphicsCommandList>` で命令の列を作り直し、`PreRenderSurface` の `deferred_surface_creation_system` が新しい描画面を作る、という順にしている。ところが同じ `PreRenderSurface` で、その後に `cleanup_surface_on_commandlist_removed` が走り（`world/mod.rs` 337〜338 行の順）、`RemovedComponents<GraphicsCommandList>` に残っている同じ回の「外した」を読む。この係の問い合わせは命令の列が今また在るかを見ず、描画面が有効なら `SetBrush(None)` と `clear()` をする（`surface.rs` 280〜318 行）。作ったばかりの描画面が有効なので、そのまま消される。次の回は `Changed<GlobalArrangement>`・`Changed<GraphicsCommandList>`・`Changed<VisualGraphics>` のどれも立たないので作り直されない。
**影響**: 四角と画像の部品（同じ回に命令の列を作り直す部品）が、立ち直りの後ずっと空のまま残る。6 番の「立ち直れない」が部分的に残る。設計の立ち直りのテスト（3 回以内に全部作り直される）は、作られた直後の 1 回を見ると通り、次の回に消えることを見逃しうる。
**提案**: `cleanup_surface_on_commandlist_removed` の問い合わせに `Without<GraphicsCommandList>` を足す（外した後に同じ回でまた入ったものは消さない）。1 行で済み、今の「命令の列が本当に無くなった部品だけ片付ける」の意図とも合う。立ち直りのテストには「作り直しの後にさらに 2 回回しても描画面が有効で、絵の筆が付いている」を足す。
**関連する要件**: 3.1・3.2（6 番）
**設計の該当箇所**: System Flows「デバイスロストからの立ち直り」の決め「命令の列は作り直しで外す」「描画面は見た目の作り直しで作り直す」・Testing Strategy の 6 番「立ち直り」の行

**重大な議題 2**: 描き直しの無い間（ゴーストがじっとしている間）は、装置の失いを見つけられない
**懸念**: 見つける所は `render_surface` の `BeginDraw` の失敗だけで、`render_surface` は `Changed<SurfaceGraphicsDirty>` の部品にしか回らない。描き直しが 1 つも無い間は、装置が失われても誰も気づかない。設計はこの限界を Risks にも Error Handling にも書いていない。WUC の描画面の中身が装置の失いで消えるなら、その間ゴーストの絵は消えたままで、次に何かが描き直されるまで戻らない。
**影響**: 6 番の直し（議題 1「見つける所」を含む）が、ゴーストが動いていない場面で働かない。ドライバの更新の後に、触るまでゴーストが見えない、という形で残りうる。
**提案**: 画面更新の門には 30 回ごとの心拍が在る（`world/tick_gate.rs` 53 行 `TICK_HEARTBEAT_FRAMES`）ので、`GraphicsSetup` の先頭（無効化の係の前）で、装置が有効なら D3D の装置に `GetDeviceRemovedReason()` を問う小さな確かめを足せば、じっとしている間も心拍 1 回分のうちに見つかる。呼ぶのは安い 1 回で、UI のスレッドの外の道も増えない。足さない場合は、その限界（描き直しが起きるまで見つからない）を Risks に明記し、討議で開発者に確かめる。
**関連する要件**: 3.1（6 番）・5.3
**設計の該当箇所**: System Flows の決め「見つける所は `BeginDraw` の失敗だけ」・Risks

**重大な議題 3**: 直しが届く先のうち、設計の一覧に載っていないものがある
**懸念**: (a) 6 番で `GraphicsCore::note_draw_failure`（`graphics/core.rs`）が `tick_wake::GRAPHICS` を立てると、`world/tick_gate_tests.rs` の「旗を立てる本番のファイルが全部名簿に載っているか」の検査（逆向きの完全性の検査）が赤になる。名簿 `WINTF_PRODUCERS`（いま 9 行）へ `graphics/core.rs` を足す必要があるが、File Structure Plan に無い。(b) 無効化の係を `Update` から `GraphicsSetup` へ移すと、`Update` の chain の `update_typewriters.after(invalidate_dependent_components)`（`world/mod.rs` 248〜249 行）の順の相手が消えるので、登録を組み直す必要がある。設計は「`GraphicsSetup` の先頭」とは書くが、`Update` から外すことと、この順の付け替えを書いていない。(c) 範囲の外のファイルの節で、`graphics/dcomp_integration_test.rs` が「裁定で書き換えてよい」一覧（132 行）にも、「裁定の一覧に無い」一覧（133 行）にも載っている。要件の裁定の一覧には無いので、後者だけが正しい。
**影響**: (a)(b) は全体テストで赤として見つかるので害は小さいが、要件 3.4（呼び出し元をツリー全体で探して全部直す）の一覧が欠け、タスクの「触るファイル」の見積もりがずれる。(c) は申し合わせの相手と中身を誤らせる。
**提案**: File Structure Plan の `world/` に `tick_gate_tests.rs`（名簿に `graphics/core.rs` を 1 行足す）を加え、`world/mod.rs` の行に「無効化の係を `Update` の chain から外し、`update_typewriters` の順を `update_monitor_layout_system` の後へ付け替える」を書く。`dcomp_integration_test.rs` は 132 行の一覧から外す。
**関連する要件**: 3.4・3.5
**設計の該当箇所**: File Structure Plan「Directory Structure」と「範囲の外で直すファイル」・「作り直しの順」

### 軽い注意（重大ではない）
- `render_surface` の引数を `Option<ResMut<GraphicsCore>>` にすると、可変の参照を取るだけで資源の変化の印が立つ。下流は「`GraphicsCore` が変わったら控えを捨てる」ので、失いのとき以外に可変で触ると、毎回の描画で下流が全部を作り直す。`note_draw_failure` を呼ぶときだけ可変に触る、と設計に一言あるとよい（失いでないときは `Deref` だけで読む）。
- `invalidate_dependent_components` は今、`iter_mut()` で全部の画像の部品に可変で触っている（無効な回のたびに変化の印が立つ）。設計の「有効な部品だけを倒す」を実装するときは、`is_valid()` を見てから可変に触る形にする。

## 設計の良いところ

- 「今の作りの形のまま、穴を塞ぐ」を守り、足す物を祖先の走査 1 つ・実行の条件 2 つ・係 2 つに絞っている。作れない回に変化の印を食べる問題を、bevy のソースで確かめた 1 つの規則（作る係を `run_if` で回さない）で塞ぎ、`init_window_graphics` の早い戻りという設計の調べで見つけた穴まで手当てしている。
- 直す指摘のすべてに、消すものの確かめ（要件 3.3）と呼び出し元（要件 3.4）を行ごとに付け、範囲の外のファイルと申し合わせの相手（`audit-wintf-runtime`・`audit-emo-present`）を明記している。下流（emo-present・emo-text・areka）の立ち直りが要ることを設計の調べで見つけ、在る関数を呼ぶだけの手当てとして持ち込んでいる。

## 最後の判定

**判定**: GO（条件付き）
**理由**: 構えは今の作りと steering に合い、直す指摘はもれなく直し方とテストを持つ。重大な議題 3 つは、どれも設計の書き足しか 1 行の手当てで閉じ、作りの組み直しは要らない。
**次の段取り**:
1. 設計の討議（`/kiro-design-discussion audit-wintf-graphics-world`）で、議題 1〜3 の書き足しを決める（議題 1 は `Without<GraphicsCommandList>` とテストの追加、議題 2 は心拍での問い合わせを足すか限界を明記するか、議題 3 は一覧の直し）。
2. その後 `/kiro-spec-tasks audit-wintf-graphics-world` でタスクを作る。議題 3 の (a)(b) は 8 番目のタスク（作り直しの順）に入れればタスクの数は変わらない。

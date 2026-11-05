# ギャップ分析: audit-wintf-graphics-world

- 実施日: 2026-11-05
- 測った時点: main `88eb0a43` の上の作業枝（`047b0281`・コードは main と同じ）
- 物差し: ponytail 5.3.2 の `/ponytail:ponytail-audit`（作法のうち「最大 20 件」と「読まなかった所」を外した。範囲のファイルは全部読んだ）
- この文書は分析と案だけを書く。直す・直さないの決めは要件の討議で行う。

---

## 1. 受け持つ範囲（要件 1）

`crates/wintf/src/ecs/` のうち、`drag*`・`pointer*`・`clickthrough*`・`tooltip*`・`layout*`・`widget*`・`window*`（`window/`・`window_proc/` を含む）でないもの全部。着手のときより後に足されたファイルは無い（要件 1.2 の該当なし）。

| 区分 | ファイル数 | 行数 |
|---|---|---|
| コード | 43 | 8,447 |
| テスト（`_tests.rs`・`tests.rs`・`test_support.rs`） | 11 | 2,498 |
| 合計 | 54 | 10,945 |

ほかに `.rs` でないファイルが 1 つある（`visual/draw/README.md`・58 行・これも読んだ）。

読んだファイルの一覧（行数）。すべて頭から終わりまで読んだ。読まなかったファイルは無い。

| ファイル | 行 | ファイル | 行 |
|---|---|---|---|
| `mod.rs` | 64 | `graphics/systems/visual_sync.rs` | 278 |
| `app.rs` | 185 | `graphics/systems/window_pos.rs` | 153 |
| `test_support.rs`（テスト） | 34 | `graphics/tests.rs`（テスト） | 153 |
| `types.rs` | 473 | `graphics/visual.rs` | 154 |
| `common/mod.rs` | 85 | `graphics/visual_manager.rs` | 146 |
| `common/tree_iter.rs` | 329 | `graphics/wuc_resource.rs` | 212 |
| `common/tree_system.rs` | 368 | `visual/mod.rs` | 14 |
| `cursor/mod.rs` | 70 | `visual/clip.rs` | 68 |
| `cursor/answer.rs` | 275 | `visual/draw/mod.rs` | 2 |
| `cursor/answer_tests.rs`（テスト） | 152 | `visual/draw/builder.rs` | 298 |
| `cursor/cur_file.rs` | 82 | `visual/draw/command.rs` | 43 |
| `cursor/cur_file_tests.rs`（テスト） | 133 | `visual/draw_content.rs` | 125 |
| `cursor/image.rs` | 95 | `visual/transform.rs` | 41 |
| `cursor/image_tests.rs`（テスト） | 62 | `visual/version.rs` | 20 |
| `dola/mod.rs` | 477 | `visual/visual_flags.rs` | 35 |
| `graphics/mod.rs` | 20 | `visual/world_visual.rs` | 64 |
| `graphics/clip.rs` | 188 | `world/mod.rs` | 944 |
| `graphics/command_list.rs` | 35 | `world/schedule_labels.rs` | 205 |
| `graphics/components.rs` | 317 | `world/thread_registry.rs` | 185 |
| `graphics/core.rs` | 147 | `world/thread_registry_tests.rs`（テスト） | 182 |
| `graphics/systems/mod.rs` | 15 | `world/tick_diag.rs` | 363 |
| `graphics/systems/brushes.rs` | 111 | `world/tick_diag_tests.rs`（テスト） | 315 |
| `graphics/systems/clip_sync.rs` | 212 | `world/tick_gate.rs` | 222 |
| `graphics/systems/init.rs` | 322 | `world/tick_gate_tests.rs`（テスト） | 660 |
| `graphics/systems/render.rs` | 156 | `world/tick_wake.rs` | 375 |
| `graphics/systems/surface.rs` | 319 | `world/tick_wake_tests.rs`（テスト） | 397 |
| | | `world/vsync.rs` | 155 |
| | | `world/world_tick_gate_tests.rs`（テスト） | 410 |

（表の中のパスは `crates/wintf/src/ecs/` から見たもの。以下も同じ。）

---

## 2. audit の結果（要件 2）

### このコードがしていること

wintf の描画の芯である。ECS の World と 13 本のスケジュールを回す係（`world/`）、描画の装置（D3D11・D2D・DirectWrite）と WUC（Windows.UI.Composition）の合成の部品を用意し、命令の列を描画面に描いて WUC の木へ写す係（`graphics/`）、親子の木の伝播と走査（`common/`）、マウスカーソルの形（`cursor/`）、dola のアニメの ECS の包み（`dola/`）、共通の型（`types.rs`）、まだ使われていない描画の木（`visual/`）を持つ。

想定した負荷: デスクトップマスコット 1 プロセス、UI スレッド 1 本、窓は数枚（シェルとバルーン×キャラクター）、見た目の部品は数十、画面更新は 60〜120 Hz。加えて wintf は crates.io に出す汎用のライブラリなので、areka の外の使い手が公開の型を普通に使う場合も「起こりうる」と数えた。

### 数

| 組 | 件数 | うち `wintf-visual-tree` へ申し送る候補 | うち「直さずに残してよい 3 種類」の候補 |
|---|---|---|---|
| 必ず直す | 6 | 0 | 3（2・6 番＝⑵・5 番＝⑴ の条件つき） |
| 直したほうがよい | 24 | 9（22〜30 番） | 2（13・17 番＝⑵）＋申し送りの 9 件 |
| できれば | 4 | 0 | 1（34 番＝⑴） |
| 合計 | 34 | 9 | 6（申し送りの 9 件は別に数える） |

「直さずに残してよい 3 種類」は ⑴ 開発者の裁定に反する直し ⑵ わざと置いた先送りの印 ⑶ SSP との互換や ukadoc の正典に関わる振る舞いの変更。この範囲には ⑶ に当たる件は無い（描画の下まわりで、伺かの語や振る舞いに触れない）。

各件の書き方: 番号・組・場所・何が悪いか・こうなると困る（具体の場面）・いちばん小さな直し方の案・範囲の外で影響を受ける所（木全体を探した結果）・残してよい種類に当たるか・`wintf-visual-tree` の傷 1〜8 との重なり。

---

### 必ず直す（6 件）

**1. 描画の装置が作れなかった回にアプリが落ちる**
- 場所: `graphics/systems/init.rs:197`（`init_window_graphics`）・`graphics/visual_manager.rs:80`（`visual_resource_management_system`）・`graphics/systems/surface.rs:82`（`deferred_surface_creation_system`）
- 何が悪いか: この 3 つの係は `Res<GraphicsCore>`（無いと動けない形）で描画の装置を受け取る。装置の作り直しの係 `init_graphics_core` は「作れなかったら次の回にまた試す」作りだが、作れなかった回は装置が World に無い。bevy_ecs 0.19 は、無いものを `Res` で受ける係を「失敗」と扱い、既定の重さ `Panic` で止める（`bevy_ecs-0.19.1` の `system/system.rs` の `RunSystemError` と `error/handler.rs` の `match_severity` で確かめた）。
- こうなると困る: 起動の最初の回に `GraphicsCore::new()` が失敗する（7 番の場面・ドライバの入れ替え中など）と、同じ回の `GraphicsSetup` で `init_window_graphics` が走った時点でプロセスが panic で落ちる。設計が意図した「次の回に再試行」は一度も起きない。
- 直し方の案: 3 つとも `Option<Res<GraphicsCore>>` にして、無ければ何もせず戻る（同じ範囲の `render_surface`・`invalidate_dependent_components`・widget の描画の係はもうこの形）。元に戻すと落ちるテスト: 装置を入れない World でこの 3 つを回して落ちないこと。
- 範囲の外: 呼び出し元の変更なし（係の引数だけ）。`crates/wintf/tests/graphics/` に装置ありの前提のテストがあるが形は変わらない。
- 残してよい種類: 当たらない。

**2. dola のアニメの包み `DolaAnimator` の `Sync` が嘘**
- 場所: `dola/mod.rs:86-87`
- 何が悪いか: 中身の `DolaRuntime` は `Rc`（複数スレッドで数を数え合えない参照）を持つ。`Sync` を手で付けているので、読むだけの係が 2 本、別々のスレッドで同時に `last_result()` の中の `Rc` を複製できてしまう。注記（50〜85 行）自身が「将来並列に走ると参照の数の競合が起きうる」と認めている。
- こうなると困る: wintf の使い手が `Update`（複数スレッドで回るスケジュール）に `Query<&DolaAnimator>` の係を 2 本置き、変化の値（`EvaluatedValue::Object`）を複製すると、参照の数が壊れて解放済みのメモリを触る。今の areka は使っていない（`tick_dola_animators` は本番のどのスケジュールにも登録されていない・使い手は 0）。
- 直し方の案（議題 D-2）: ㋐ 使い手が 0 なので包みごと消す（`dola/mod.rs`・`ecs/mod.rs:20` の再公開・`crates/wintf/tests/ecs/dola_animator_test.rs`）。ただし `world/tick_gate_tests.rs:355` の生産者の表が `dola/mod.rs` の `ANIM` を名指しているので表と `world/tick_wake.rs:26`・`:134` の `ANIM` の説明も直す。㋑ 残すなら `Sync` を外せる形（UI スレッドだけで触る NonSend の資源にする等）へ作り替える。
- 範囲の外: `crates/wintf/tests/ecs/dola_animator_test.rs`（`audit-wintf-runtime` の持ち物の `tests/`）。areka の側の使い手は 0。
- 残してよい種類: ⑵ の候補（注記は「proposals P68 記録」と先送りを書いている）。ただし `#[allow(dead_code)]` の語彙の口ではなく、公開の型の健全さの穴なので、⑵ の「限界が今の負荷を越えていない」とは言い切れない。開発者の裁定が要る。

**3. 見た目の木の親子のつなぎが、一度つないだら二度と見直されない**
- 場所: `graphics/systems/visual_sync.rs:53`（未同期の見分けが「親の控えが空か」だけ）・`:95` と `:167`（親が見た目の部品を持たないと子を「根」と印して終わる）
- 何が悪いか: ⒜ 親を付け替えた子（`ChildOf` の変更）は、控え `parent_visual` が埋まっているので見分けられず、WUC の木では古い親の下に残る。⒝ 子より後で親が見た目を持った場合、子は先に「根」の印（控え＝自分自身）を付けられ、親の WUC の木へ二度とつながらない。
- こうなると困る: ⒜ 部品を別の親へ移すと、画面では元の親の位置・切り抜き・不透明度のまま描かれ続ける。さらに元の親が別の子の同期で `RemoveAll` されると、その子は新しい親にも古い親にもつながらず消える。⒝ 先に子を作り、次の回に親へ `Visual` を足すと、子は永久に表示されない。
- 直し方の案: 未同期の見分けに「`ChildOf` が変わった（`Changed<ChildOf>`）」と「控えの親と今の親の WUC の Visual が違う」を足す。根の印は「親が窓の外（`VisualGraphics` を持たない）」のときだけにし、親が後から見た目を持ったら印を外す。元に戻すと落ちるテストを 2 本（付け替え・親が後から）。
- 範囲の外: `crates/wintf/tests/visual/hierarchy_reparent_gap_test.rs` は今の誤りを「現行の挙動」として固定している（P47）。直すと赤になるので、正しい挙動のテストへ書き換える。持ち主は `audit-wintf-runtime`（同じウェーブで走る）＝申し合わせてから触る。
- 残してよい種類: 当たらない（P47 は先送りの記録で、開発者の裁定ではない）。

**4. 窓へ後からつないだ部品は、ずっと描かれない**
- 場所: `graphics/visual.rs:68-113`（`on_visual_add`）
- 何が悪いか: 見た目の部品（`VisualGraphics`・`SurfaceGraphics`・`SurfaceGraphicsDirty`）を入れるかどうかを、`Visual` を足した瞬間に 1 度だけ「祖先に窓があるか」で決める。その後で窓の下へつないでも入らない。ほかに入れる所は木全体に無い（`VisualGraphics::default()` を入れるのはここだけ・テストを除く）。
- こうなると困る: `spawn(部品)` で作ってから `add_child` で窓の下へ付けると、その部品は永久に画面に出ない。エラーも出ない。
- 直し方の案（議題 D-3）: `ChildOf` が付いた・変わった部品を見て、祖先に窓が在り見た目の部品が無ければ入れる係を 1 本足す（既存の `on_visual_add` と同じ判定を使い回す）。元に戻すと落ちるテスト 1 本。ただし `areka-emo-present` の `mount.rs` と `areka-emo-text` は「自前の `VisualGraphics` を入れて既定の挿入を避ける」使い方をしている。既に在れば入れない判定なので壊さないが、設計で確かめる。
- 範囲の外: `crates/areka-emo-present/src/mount.rs:37`・`crates/areka-emo-text/src/surface.rs:301`（自前で入れる側・同じウェーブの `audit-emo-present` が走る）。
- 残してよい種類: 当たらない。ただし「窓に属さない部品には入れない」が意図なのか、付け替えの見落としなのかは開発者に確かめる（D-3）。

**5. 色の受け継ぎ（ブラシ）の答えが、親と子を作った回が同じかどうかで変わる**
- 場所: `graphics/systems/brushes.rs:28`（`find_parent_brushes` を呼ぶ所）・`:62-85`
- 何が悪いか: 受け継ぎの解決は命令（`commands`）で書き戻すので、同じ回に解決を待つ親の結果は、その回の子からは見えない。子は親を飛ばして祖父母か既定の色を取る。
- こうなると困る: 背景だけ青を決めた枠（前景は受け継ぎ）と、その中の文字（ブラシ未指定）を同じ回に作ると、文字の背景は既定の色になる。1 回あとで文字を作ると青になる。作る順で色が変わる。
- 直し方の案: 解決を待つ部品を浅い順に並べ、解決した結果をその回の中の表（`HashMap`）に置いて、子はまず表を見る。元に戻すと落ちるテスト 1 本（同じ回に親子を作る）。
- 範囲の外: `crates/wintf/tests/graphics/brushes_system_test.rs:149`・`:185` が「片方だけ決まった親は飛ばす」を現行の仕様として固定している。これを仕様として残すか（D-4）で直しの形が変わる。持ち主は `audit-wintf-runtime`。
- 残してよい種類: 当たらない。「片方だけ決まった親を飛ばす」が開発者の決めなら ⑴ の候補（D-4）。

**6. 描画の装置が失われた（デバイスロスト）ときに立ち直れない**
- 場所: 見つける所が無い（`graphics/systems/render.rs:74-89`・`:144` は失敗を記録して続けるだけ）／`GraphicsCore::invalidate`（`graphics/core.rs:69`）を本番で呼ぶ所が 0（呼ぶのは `crates/wintf/examples/graphics_reinit_test.rs` とテストだけ）／`graphics/systems/window_pos.rs:132-150` の無効化は `WucGraphicsResource` と画像しか倒さず、`WindowGraphics`・`VisualGraphics`・`SurfaceGraphics` は古い装置のまま「有効」と言い続ける（同じ所の注記と P40 が認めている）／その係は複数スレッドで回る `Update` に登録されていて（`world/mod.rs:245`）、WUC の部品を UI スレッドの外で捨てうる（`world/mod.rs:141-152` の「WUC は UI スレッドだけ」の決まりに反する）／`graphics/systems/surface.rs:100-101` は装置が無効な回に早く戻り、変化の印を食べてしまう（立ち直っても描画面を作り直さない）
- こうなると困る: 動かしている間にグラフィックのドライバが更新される・GPU がリセットされる（TDR）と、窓の中身が固まるか空になり、描画の失敗の記録が出続ける。アプリを起動し直すまで戻らない。
- 直し方の案（議題 D-1）: ㋐ 描画の失敗の値（装置が消えた・作り直しが要る）を見て `GraphicsCore` を無効にし、無効化を窓・見た目・描画面まで届け、係を単スレッドのスケジュールへ移す。描画面の作り直しは「無効なら作る」に条件を足す。決定論のテストで「無効にしたら全部が作り直される」を固定。規模は大きい（3〜4 タスク）。㋑ 立ち直りは作らないと決め、使われない作り直しの道（`invalidate` 系・世代の番号・`invalidate_dependent_components`）を消す。㋒ 別の spec に切る。
- 範囲の外: `crates/wintf/examples/graphics_reinit_test.rs`・`crates/wintf/tests/graphics/window_pos_systems_test.rs`（古いまま有効、を固定する特性化のテスト）・`crates/wintf/tests/graphics/reinit_unit_test.rs`・`crates/wintf/tests/ecs/lazy_reinit_pattern_test.rs`（`audit-wintf-runtime`）。areka の側は `areka-emo-present` が `GraphicsCore` の装置を直に使う。
- 残してよい種類: ⑵ の候補（P40 として先送りの記録がある）。㋐ は「新しい機能」に当たるかもしれない（要件 3.6）。開発者の裁定が要る。

---

### 直したほうがよい（24 件）

**7. 開発用の組み立てでは、装置を作るのに Windows の追加の機能が要る**
- 場所: `graphics/core.rs:111-113`
- 何が悪いか: 開発用の組み立て（debug）は D3D11 の装置に `D3D11_CREATE_DEVICE_DEBUG` を付ける。この旗は Windows の「グラフィックス ツール」（追加の機能）が入っていないと装置の作成ごと失敗する。失敗したときに旗を外して作り直す道が無い。
- こうなると困る: その機能を入れていない PC で開発用の組み立てを動かすと、描画の装置が作れない。1 番と重なるとプロセスが落ちる。
- 直し方の案: 失敗の値が「SDK の部品が無い」（`DXGI_ERROR_SDK_COMPONENT_MISSING`）なら旗を外してもう一度作り、`warn!` を 1 行残す。
- 範囲の外: なし（`com/d3d11` の包みは形を変えない）。
- 残してよい種類: 当たらない。

**8. D2D の「工場」が 2 つあり、公開している方では描けない**
- 場所: `graphics/core.rs:46-47`（工場を作る行と装置を作る行が別々）・注記 `graphics/core.rs:29-36`・`graphics/command_list.rs:29-33`・`graphics/components.rs:38-42`
- 何が悪いか: `D2D1CreateDevice` は呼ぶたびに自分専用の工場を新しく作る（Microsoft の説明書で確かめた）。`GraphicsCore` が持つ複数スレッド用の工場 `d2d_factory` は、装置 `d2d` や描く文脈が属する工場とは別物になる。注記は「同じ工場の系列だから安全」と書いているが前提が違う（スレッドの扱いは DXGI の装置から受け継ぐので、メモリの安全はたまたま保たれている）。
- こうなると困る: 使い手が `GraphicsCore::d2d_factory()` で図形を作り、`device_context()` で描くと、D2D は「工場が違う」として断る。`crates/wintf/tests/com/d2d_ext_test.rs:133` はこれを避けるため、わざと文脈から工場を取り直している。
- 直し方の案: 持っている工場から装置を作る（`ID2D1Factory1` へ変えて `CreateDevice`）。1〜2 行。注記 3 か所を事実に合わせる。テスト 1 本（`d2d_factory()` の図形を `device_context()` で描ける）。
- 範囲の外: `crates/wintf/src/com/d2d/mod.rs:16` の `d2d_create_device`（`audit-wintf-runtime` の持ち物）を変えずに `core.rs` 側だけで直せる。
- 残してよい種類: 当たらない。`wintf-visual-tree` の傷 7（`Send`/`Sync` の根拠の注記）と近い。あちらが手本にする `GraphicsCommandList` の注記そのものが不正確なので、直したらあちらへ伝える。

**9. 親子の輪（A の親が B、B の親が A）があると UI スレッドが止まる**
- 場所: `graphics/visual.rs:50`（`owner_window_exists`）・`graphics/systems/brushes.rs:71`（`find_parent_brushes`）・`graphics/systems/visual_sync.rs:63`（深さの計算）
- 何が悪いか: 親をたどる 3 つの繰り返しに、上限も輪の見分けも無い。bevy は自分自身を親にする形だけ断り、間接の輪は作れる（各所の注記・P48 が認めている）。
- こうなると困る: 使い手の書き間違いで輪ができると、`Visual` を足した瞬間（フックの中）に無限の繰り返しへ入り、画面が固まる。記録も出ない。
- 直し方の案: 3 か所で使う「親をたどる」小さな関数を 1 つ作り、たどる回数に上限（例 1,024）を置き、越えたら `error!` を残して諦める。テスト 1 本（輪の木で止まらない）。
- 範囲の外: なし。
- 残してよい種類: 当たらない（P48 は先送りの記録）。

**10. 失敗しても記録が残らない所が 4 つある**
- 場所: `graphics/visual_manager.rs:142`（`SetRoot`）・`graphics/systems/visual_sync.rs:121`（`RemoveAll`）・`graphics/systems/surface.rs:304`（`SetBrush(None)`）・`graphics/components.rs:101`（外すときの `Remove`）
- 何が悪いか: どれも `let _ =` で失敗を捨てている。プロジェクトの決まりは「記録の無い失敗の道を作らない」（`error!`＋`Err`）。
- こうなると困る: 窓の根の Visual を付ける `SetRoot` が失敗すると、窓は空のまま。何が起きたかログから追えない。
- 直し方の案: 4 か所とも失敗を `error!`（外すとき・親が先に消えた場合は `debug!`）で残す。係は ECS の係なので `Err` を返す先が無い所は記録だけにする（理由を 1 行書く）。
- 範囲の外: なし。
- 残してよい種類: 当たらない（`SetRoot` の注記は「意図的」と書くが、開発者の裁定ではなく実装の注記）。

**11. 窓を動かすだけで、その下の描画面を全部描き直し、切り抜きも作り直す**
- 場所: `graphics/systems/surface.rs:38`（`mark_dirty_surfaces` の引き金に `Changed<GlobalArrangement>`）・`graphics/systems/clip_sync.rs:42`（同じ引き金で毎回図形と切り抜きを新しく作る）
- 何が悪いか: `GlobalArrangement` は画面の上の絶対の位置も持つ。窓の位置が変わると子孫の全部が「変わった」になり、描画面の中身（拡大の値しか見ない）を描き直し、切り抜きの WUC の部品を作り直す。
- こうなると困る: バルーンがシェルに付いて動くとき（areka の `placement/follow/window_move.rs` が窓の位置を書く）、動いている間ずっと毎回、全部の描画面を描き直す。見た目は 1 画素も変わらない。（測ってはいない。）
- 直し方の案: 描き直しの印は「大きさか拡大の値が変わったとき」だけにする（前の値を `SurfaceGraphicsDirty` に持って比べる）。切り抜きも「大きさか拡大か形が変わったとき」だけ作り直す。テスト 1 本（位置だけ変えても描き直しの印が立たない）。
- 範囲の外: なし。
- 残してよい種類: 当たらない。

**12. スケジュールの説明が古く、書いてある通りにすると何も映らない**
- 場所: `world/schedule_labels.rs:19-102`（`RenderSurface`・`Composition`・`CommitComposition` を「複数スレッドで回せる」と書く・`IDComposition…`・`Commit()` を語る・`UISetup` の説明が「D2D／DComp の呼び出しは PostLayout か Draw へ置け」と勧める）／`world/mod.rs:255`・`:298`・`:328`・`:345` の「DComp」の注記
- 何が悪いか: 今の本番は WUC で、WUC に触る係は単スレッドのスケジュールに置かないと「エラーも出さずに何も合成されない」（`world/mod.rs:141-152` に 2026-07-02 の実測として書いてある）。説明がその逆を勧めている。
- こうなると困る: 説明を信じて WUC に触る係を `PostLayout` へ置くと、窓は見えるのに中身が出ない（実際に一度起きた失敗と同じ形）。
- 直し方の案: 13 本の説明を今の実行の仕方（どれが単スレッドか）と WUC の言葉に書き直す。古い DComp の注記を消す。
- 範囲の外: なし。
- 残してよい種類: 当たらない。

**13. VSYNC の数え（カウンタ）が死んでいて、その名の関数は 1 度も画面を回さない**
- 場所: `world/vsync.rs:27`・`:31`・`:35`（`VSYNC_TICK_COUNT`・`LAST_VSYNC_TICK`・`DEBUG_WNDPROC_TICK_COUNT`）・`world/mod.rs:743-770`（`EcsWorld::try_tick_on_vsync`）
- 何が悪いか: 数えを増やす所が木全体に 0（注記も「生産者はもう存在しない」と書く）。だから `try_tick_on_vsync` は毎回「変化なし」で何もせず戻る。生きているのは、トレイトの側の再入の錠と `flush_window_pos_commands` の呼び出しだけ。`DEBUG_WNDPROC_TICK_COUNT` は書くだけで読む所が 0。
- こうなると困る: 名前と中身が食い違っていて、読む人が「ここで画面を回している」と誤る。死んだ数えが 3 つ、テストの説明も 1 つぶら下がる。
- 直し方の案（議題 D-6）: 数えの 3 つと `EcsWorld::try_tick_on_vsync` を消し、トレイトの中身を「錠を取って書き込みの指令を流す」だけにする。名前も中身に合わせて変えるなら、呼び出し元を 1 か所直す。
- 範囲の外: `crates/wintf/src/ecs/window_proc/window_pos.rs:291-295`（呼び出し元・`audit-wintf-window` の持ち物・そちらは C20 の見込みで今は走っていない）・`crates/wintf/tests/ecs/world_lifecycle_test.rs:7` の説明・`crates/wintf/src/runtime/tick_bridge.rs:162`・`:210` の説明（`audit-wintf-runtime`）。
- 残してよい種類: ⑵ の候補。注記が「ディスパッチの等価性のために保持する」と書く。わざと残した印かどうかを開発者に確かめる。

**14. World の包み `EcsWorld` に、使われない欄と関数が残っている**
- 場所: `world/mod.rs:46`・`:403`・`:431`・`:446`・`:619`（`has_systems`・必ず真なので `:619` の分岐は死んでいる）／`:47`・`:415-427`（`message_window`・本番の呼び出しが 0・テストだけ）／`:49-50`・`:490-515`・`:611`（`measure_and_log_framerate`・「不要になったらコメントアウト」と書かれたデバッグの残り）
- 何が悪いか: 毎秒の回数の記録は `tick_diag`（既定は消えている観測）が持っている。`measure_and_log_framerate` は観測が消えていても毎回時刻を取る。`tick_diag` の決まり「消えているとき時刻を 1 度も取らない」（要件 3.8 の旧 spec）の心に反する（テストは関数の外なので気づかない）。
- こうなると困る: 毎回の画面更新で無駄な時刻の取得が 1 回。読む人には「止める口がある」ように見えて実は無い。
- 直し方の案: 3 つとも消す（欄 4 つ・関数 4 つ・分岐 1 つ）。
- 範囲の外: `crates/wintf/tests/ecs/world_lifecycle_test.rs:43`・`:101-142`（`audit-wintf-runtime`）・`crates/wintf/src/ecs/window/transition_diag_tests.rs:887` の説明（`audit-wintf-window`）。`set_message_window`・`message_window` は公開の関数なので、消すのは公開の形の変更（D-5）。
- 残してよい種類: 当たらない。

**15. `world/mod.rs` が 1,000 行の番人まであと 56 行で、同じテストが 3 つある**
- 場所: `world/mod.rs:779-908`（`tick_order_tests`・約 130 行）
- 何が悪いか: 13 本の順のテストが、`mod.rs` の中に 2 本（2 本目が 1 本目を含む）、`world_tick_gate_tests.rs:51-73`・`:371-410` に 1 本ある。差し込む小道具（マクロ）も 3 回書いてある。期待する順の表 `EXPECTED_ORDER`（`:802`）は `tick_diag.rs:52` の `SCHEDULE_LABELS` と同じ中身。
- こうなると困る: 次に `mod.rs` へ何か足すと番人を越える。順を変えるときに 3 か所を直す。
- 直し方の案: `tick_order_tests` を消すか兄弟のテストのファイルへ移し、小道具と期待の表は `world_tick_gate_tests.rs` の `install_order_probes` と `SCHEDULE_LABELS` を使う。`mod.rs` は約 800 行になる。注意: `tick_diag_tests.rs:49-58`・`:159-199` は `include_str!("mod.rs")` で `try_tick_world` の本文を読む。`try_tick_world` は `mod.rs` に残す。
- 範囲の外: なし。
- 残してよい種類: 当たらない。

**16. 窓ごとに誰も使わない D2D の描く文脈を作って持っている**
- 場所: `graphics/components.rs:27`・`:77`（`get_dc`・呼び出しが木全体で 0）・`graphics/systems/init.rs:90-94`（作る所）／世代の番号 `components.rs:35`・`:65-71` と `init.rs:291-300`（本番で読む所が 0・`examples/graphics_reinit_test.rs:123` だけ・作り直しのたびに番号の数だけ回る繰り返しで値を戻す）
- 何が悪いか: 窓が開くたびに使わない文脈を 1 つ作る。世代の番号は書くだけ。
- こうなると困る: 無駄な GPU の部品と、読む人の誤解（窓ごとに描いているように見える）。`WindowGraphics` の `Sync` の注記もこの文脈を根拠に書いている。
- 直し方の案: 文脈の欄と作る行を消す。世代の番号は 6 番の決め（D-1）に従う（立ち直りを作るなら代入 1 行にする・作らないなら消す）。
- 範囲の外: `crates/wintf/tests/ecs/lazy_reinit_pattern_test.rs`（`WindowGraphics::new` の形が変わる・`audit-wintf-runtime`）・`crates/wintf/examples/graphics_reinit_test.rs`。
- 残してよい種類: 当たらない。

**17. 描画の係に、使わない引数とコメントアウトした行がたまっている**
- 場所: `graphics/systems/render.rs:36-37`（`_graphics_core`・`_frame_count`）・`:46`・`:50`・`:77`・`:114`・`:130`（`eprintln!` のコメントアウト 5 行）・`:44`（毎回の描画で名前の文字列を作る・失敗の記録にしか使わない）／`graphics/systems/surface.rs:45-55`（`count` と `_entity_name` を作って捨てる・コメントアウト 2 行）／`graphics/visual_manager.rs:79`（`_commands`・注記が P45 として先送り）
- 何が悪いか: 毎回の画面更新で、変わった描画面ごとに使わない文字列を 2 回作る。使わない引数は係の読み書きの範囲を広げ、並べ方の自由を減らす。
- こうなると困る: 小さな無駄と読みにくさ。
- 直し方の案: 引数を消し、コメントアウトを消し、名前の文字列は失敗のときだけ作る。
- 範囲の外: なし（係の引数は bevy が埋めるので呼び出し元は無い）。
- 残してよい種類: `_commands` の P45 は先送りの記録（⑵ の候補）だが、`#[allow(dead_code)]` の語彙の口ではなく、限界の注釈でもない。

**18. ほとんど同じコードが 2 組ある**
- 場所: `graphics/systems/init.rs:118-187`（`init_graphics_core` の「作り直し」と「初めて作る」の 2 つの枝がほぼ同じ）・`graphics/components.rs:207-211` と `:247-251`（`SurfaceGraphics` の `invalidate` と `clear` が同じ中身・`invalidate` は本番で呼ぶ所 0）・`:231` の説明「サイズが異なる場合のみ更新」は嘘（無条件に入れ替える）
- 直し方の案: 枝は「新しく作る → 在れば入れ替え・無ければ挿入 → 印を付け直す」の 1 本にする。`clear` か `invalidate` の片方を消す。説明を直す。
- 範囲の外: `crates/wintf/tests/graphics/components_test.rs`・`reinit_unit_test.rs`（`SurfaceGraphics` の片方を呼んでいれば直す・`audit-wintf-runtime`）。
- 残してよい種類: 当たらない。

**19. 使われない公開の小物**
- 場所: `graphics/visual_manager.rs:19-34`（`insert_visual`・`insert_visual_with`・本番の呼び出し 0・`crates/wintf/tests/visual/insert_visual_test.rs` だけ・中身は `insert` を呼ぶだけ）／`types.rs:97-104`（`Size::validate`・呼び出しが木全体で 0）／`mod.rs:31-32`（`FrameTime` と `calculate_surface_size_from_global_arrangement` の再公開・`:33` の `pub use graphics::*` と重なる）／`graphics/core.rs:105-106`（使わない `use` を `allow` で黙らせている）
- 直し方の案: 消す。テストも消す。
- 範囲の外: `crates/wintf/tests/visual/insert_visual_test.rs`・`crates/wintf/tests/visual.rs:14-15`（`audit-wintf-runtime`）。
- 残してよい種類: 公開の形を消すので D-5（crates.io の使い手への約束をどう扱うか）。

**20. `owner_window_exists` のテストが本物でなく写しを試している**
- 場所: 試される側は `graphics/visual.rs:38`。テストは `crates/wintf/tests/window/owner_window_exists_test.rs:7-30`（範囲の外）
- 何が悪いか: テストは「同じ判定を World で書き直した関数」を試していて、本物の `owner_window_exists` を呼ばない。本物を壊してもテストは緑のまま。
- 直し方の案: 本物を通す形にする（`Visual` を足してフックを走らせ、`VisualGraphics` が入るかを見る）。9 番の輪のテストと一緒にできる。
- 範囲の外: `crates/wintf/tests/window/owner_window_exists_test.rs`（`audit-wintf-runtime`）。
- 残してよい種類: 当たらない。

**21. 窓のメッセージと旗の対応が、表と `match` の 2 か所に同じ中身で書いてある**
- 場所: `world/tick_wake.rs:285-342`（`KNOWN_MESSAGE_TABLE`）・`:350-371`（`wake_bits_for_message`）
- 何が悪いか: 説明自身が「片方だけ直してはならない」と書き、食い違いを両向きのテストで捕まえている。1 つにすれば両向きのテストの片方が要らなくなる。
- 直し方の案: 関数を表の引き当てにする（約 55 件の線形の探索。メッセージ 1 通につき比べるだけで、測れるほどの差は無い見込み）。`tick_wake_tests.rs:314-350` の片方を消す。
- 範囲の外: なし。
- 残してよい種類: 当たらない。

**22〜30. 作りかけの描画の木 `visual/`（`wintf-visual-tree` へ申し送る候補）**

どれも `visual/` の中で、本番から呼ばれていない（外からの参照 0・スケジュールへの登録 0・テスト 0 を木全体で確かめた）。今は失敗の場面が起きない。`wintf-visual-tree` が本番へ上げるときに初めて効く。要件 4.4 に従い「使われていない」を理由に消す案は出さない。各件は `wintf-visual-tree` の brief の「直すべき傷」の番号と対応する。

| 番号 | 組 | 場所 | 何が悪いか（本番へ上げたときに困ること） | 傷 |
|---|---|---|---|---|
| 22 | 直したほうがよい | `visual/clip.rs:27`・`visual/draw_content.rs:23`・`visual/draw/builder.rs:246-252` | 見た目の hash に COM の部品の番地を使う。命令の列が捨てられ、同じ番地に別の絵が作られると「同じ」と見て描き直さない | 1 |
| 23 | 直したほうがよい | `visual/draw/builder.rs:17-22`・`:94-97` | 平らにする係 `build` が `VisualFlags`（見える／見えない）を読まず、見えない節も描く。世界の行列を自前で積み、`WorldVisual` を使わない | 2 |
| 24 | 直したほうがよい | `visual/draw/builder.rs:268-273` と `numerics/matrix3x2.rs:11-13` | 「軸に沿うか」の判定が 2 通り（片方は厳密・片方は 1e-4 の許容で 90 度回転も認める）。同じ節がどちらで判じたかで切り抜きの描き方が変わる | 3 |
| 25 | 直したほうがよい | `visual/draw/builder.rs:219-236`・`:276-298` と `numerics/aabb.rs:81`・`:91`・`:127` | 矩形の交わり・空の判定・変形が `numerics` の `Aabb` と二重。片方だけ直すと食い違う | 4 |
| 26 | 直したほうがよい | `visual/draw/builder.rs:239-242` | `GetBounds` の失敗を `unwrap_or_default()` で 0 の矩形に化かし、記録なしに子の木ごと間引く（記録の無い失敗の道） | 5 |
| 27 | 直したほうがよい | `visual/draw/builder.rs:75-141`（`visit` の再帰） | 深さの上限も輪の見分けも無い再帰。深い木か輪でスタックがあふれる | 6 |
| 28 | 直したほうがよい | `visual/clip.rs:15-16`・`visual/draw_content.rs:17-18` | `unsafe impl Send/Sync` に根拠の注記が無い（8 番のとおり、手本の注記も不正確） | 7 |
| 29 | 直したほうがよい | `mod.rs:14`（`pub mod visual`）と `graphics/mod.rs:6`・`mod.rs:33`（`graphics::visual` の再公開） | `ecs::visual`（描画の木）と `ecs::graphics::visual`（今の `Visual` 部品）が別物なのに同じ名前。`ecs` の根で `graphics::*` を再公開すると、`ecs::visual` はローカルの方を指し、`graphics::visual` は隠れる | 8 |
| 30 | 直したほうがよい | `visual/draw/builder.rs:57`（`#[allow(dead_code)]`）・`visual/draw/command.rs:1`（`#![allow(dead_code)]`）・`visual/draw/README.md`（`command.rs` の型の写しを載せた説明） | 使われていない印に「なぜ残すか」の説明が無い（⑵ の条件の「実在の理由の doc」を満たしていない）。README は型の写しで、片方だけ古くなる | （傷の一覧に無い・新しく見つけた） |

- 直し方の案（議題 D-7）: ㋐ この spec では直さず、9 件とも `wintf-visual-tree` へ申し送る（あちらの傷 1〜8 と同じ物なので二重の仕事を避けられる・30 番は「残す理由の doc を付ける」だけこの spec でしてもよい）。㋑ この spec で直す（hash を版の番号で取る・`numerics` へ寄せる・`error!` と描かずに飛ばす理由の記録・深さの番兵・注記・名前の変更）。名前の変更（29 番）は `wintf-visual-tree` の議題 2 と同じ問いなので、あちらへ回すのが自然。
- 範囲の外: `numerics/`（`audit-wintf-runtime` の持ち物・同じウェーブで走る）。`wintf-visual-tree` は未着手（この spec の着地を待つ）。
- 残してよい種類: 22〜29 は「未使用」の指摘ではない（作りの傷）ので、要件 4.4 の申し送りの対象は 30 番だけが文字どおり当たる。22〜29 も、直さずに渡すなら ⑵ の扱いとして開発者の裁定を得る（D-7）。

---

### できれば（4 件）

**31. 切り抜きの説明が中身と違う**
- 場所: `graphics/systems/clip_sync.rs:127`（「`Rectangle` → `CreateInsetClip()`」）・ファイル頭の `:3-5`
- 何が悪いか: 実際は角丸 0 の図形の切り抜き（`build_rounded_geometric_clip`）を使う（理由は `:146-153` に書いてある）。説明の 1 行だけ古い。
- 直し方の案: 説明を中身に合わせる。

**32. 最後に出したカーソルの控えが、窓が消えても残る**
- 場所: `cursor/answer.rs:106`（`LAST_SHOWN`）
- 何が悪いか: 窓ごとの控えを消すのは `clear_last_shown`（シェルの入れ替え）だけ。窓を閉じても控えは残る。
- こうなると困る: 閉じた窓と同じ値の HWND を新しい窓がもらい、World が借りられない間に `WM_SETCURSOR` が来ると、前の窓のカーソル（もう捨てたかもしれないもの）を出す。めったに起きない。
- 直し方の案: 窓が壊れるとき（`WM_DESTROY` の受け口か `WindowHandle` が外れるとき）にその窓の控えを 1 件消す口を足す。
- 範囲の外: 呼ぶ所は `window_proc/` か `window/`（`audit-wintf-window`・今は走っていない）。

**33. テストだけが使う関数を `#[allow(dead_code)]` で黙らせている**
- 場所: `world/vsync.rs:62-65`（`is_tick_flush_in_progress`）
- 何が悪いか: 呼ぶのは `crates/wintf/src/runtime/tick_bridge.rs` のテストだけ。`#[cfg(test)]` にすれば `allow` が要らない。
- 直し方の案: `#[cfg(test)]` に替える。

**34. 型の作りを確かめるだけのテスト**
- 場所: `world/schedule_labels.rs:114-205`（7 本のうち、ラベルの等しさ・複製・hash・表示・型の区別を確かめる 5 本＝derive の動きを試している）・`graphics/tests.rs:113-153`（空の印の複製・既定値の 0）・`graphics/clip.rs`（テストの `clip_shape_clone`・`clip_shape_debug`）
- 何が悪いか: 壊れても困らない所を試していて、本物の誤りを捕まえない。読む・回す手間だけが残る。
- 直し方の案: 消す。
- 残してよい種類: 開発者の「決定論のテストの網羅は必達」の決めと衝突しうる（⑴ の候補）。消してよいかを確かめる（D-8）。

---

### 問題にしなかった所（確かめて、指摘にならなかったもの）

- `common/tree_system.rs` は bevy の変換の伝播の写しを型引数にしたもの。上流と同じ並列の作りで、`crates/wintf/tests/ecs/tree_propagation_test.rs` が 14 本で試している。
- `world/tick_gate.rs`・`tick_wake.rs` の判断と期限: 期限は「最も早い 1 つ」しか預からないが、預ける側（areka の `balloon_visibility_phase.rs`・`char_watch.rs`）が走るたびに預け直す約束になっていて、取りこぼしは起きない。総なめのテストもある。
- `world/thread_registry.rs`・`tick_diag.rs`: 失敗の道は `error!`＋`Err`。テストもある。
- `cursor/cur_file.rs`: 長さとはみ出しの検査が揃い、テストもある。
- `graphics/wuc_resource.rs:38` の `#[allow(dead_code)]` は「捨てる順を守るために持つだけ」と理由が書いてあり、⑵ の正しい形。
- `doc/`・`docs/` を名指す行は範囲の中に 0（要件 5.9 の確認）。

---

## 3. 要件との対応表

| 要件 | 今の状態 | 足りない所 |
|---|---|---|
| 1（範囲を確定して全部読む） | 54 ファイル全部を読んだ・一覧は 1 節 | なし |
| 2（全件を記録） | 34 件・上限なし・番号と組つき | なし |
| 3.2（バグと危ない所に元に戻すと落ちるテスト） | 1〜11 番のうちテストが無いもの多数 | 1・3・4・5・7・8・9・11 番に新しいテスト（Missing） |
| 3.3（消す前に木全体で確かめる） | 13・14・16・19 番は確かめ済み（テスト・例・説明の参照まで） | 設計で消す範囲を確定するときにもう一度 |
| 3.4（形が変わる関数の呼び出し元） | 各件の「範囲の外」に列挙 | `audit-wintf-runtime`（`crates/wintf/tests/`・`examples/`・`numerics/`）と申し合わせが要る（Constraint） |
| 3.7（傷 1〜8 との重なり） | 22〜29 番が傷 1〜8、8 番が傷 7 に近い | 記録の形を設計で決める |
| 4（残す指摘を裁定へ） | 候補は 2・6・13・17・19・22〜30・34 番 | 要件の討議で裁定（下の D-1〜D-8） |
| 5.1（1,000 行） | `world/mod.rs` 944 行 | 15 番で約 800 行へ（Constraint: `try_tick_world` は `mod.rs` に残す） |
| 5.3（記録の無い失敗の道） | 10・26 番が該当 | 直す |
| 5.6（GPU のテストは実 GPU） | 1・7・8 番のテストは実の装置で回す | なし |
| 5.8（依存を足さない） | どの案も依存を足さない | なし |
| 6（2 度目の audit で 0 件） | 未実施 | 実装の最後のタスク |

---

## 4. 進め方の案

### 案 A: この spec 1 本で全部直す
- 1〜21 番と 31〜34 番を直し、22〜30 番も直す。
- タスクの見積もり: 必ず直す 6 件で 7〜9（6 番が 3〜4）・直したほうがよい 7〜21 番で 9〜11（小物はまとめる）・`visual/` の 9 件で 3〜4・できれば 1・最後の audit と全体テスト 1〜2。合計 **21〜27**。20 を越える。
- 良い点: 1 回で終わる。悪い点: 20 を越える。`visual/` は `wintf-visual-tree` と同じ所を二重に触る。

### 案 B: `visual/` の 9 件を申し送り、この spec は残りを直す
- 22〜30 番は「申し送り」として記録だけ残す（30 番の理由の doc はここで付けてもよい）。
- タスクの見積もり: **18〜23**。6 番を小さく決めれば（D-1 の ㋑）20 以下に収まる。
- 良い点: 二重の仕事が無い。`wintf-visual-tree` の傷の一覧とそのまま突き合わせられる。悪い点: 6 番の決め次第で 20 を越えうる。

### 案 C: 組ごとに 2 本へ分ける（要件 6.5）
- 1 本目＝必ず直す 1〜6 番と、それに絡む 7〜10 番・20 番。2 本目＝残りの直したほうがよい・できれば（と、`visual/` を直すならそれも）。
- タスクの見積もり: 1 本目 12〜15・2 本目 10〜13。
- 良い点: どちらも 20 以下。悪い点: 同じ範囲で 2 本走らせる手間、2 本目の audit の扱い（要件 6 の「同じ範囲で 0 件」をどちらで見るか）。

### 規模と危うさ
- 規模: M〜L（6 番の決め次第。㋐ なら L）。既存の作りの形のまま直す件が大半で、新しい作りは 6 番の立ち直りだけ。
- 危うさ: 中。3・4・5・11 番は見た目の振る舞いが変わる（正しくなる方へ）ので、`areka-emo-present`・`areka-emo-text` の画素のテストで前後を確かめる。6 番の ㋐ は実機のデバイスロストを決定論で起こしにくい（テストでは `invalidate` を呼んで無効の状態を注入する形で代える見込み）。

---

## 5. 設計へ持ち越す調べもの（Research Needed）

- 6 番 ㋐ を採るなら: WUC の `CompositionGraphicsDevice` の装置の入れ替え（`RenderingDeviceReplaced`）と D2D の作り直しの値（`D2DERR_RECREATE_TARGET`・`DXGI_ERROR_DEVICE_REMOVED`）のどちらで見つけるか。決定論のテストでどう起こすか。
- 6 番: WUC の部品（`Compositor`・`DispatcherQueueController`）を UI スレッドの外で捨てたときに何が起きるか（今は決まりに反することだけが確か）。
- 11 番: 位置だけの変化で描き直しを止めたとき、WUC の描画面の中身が位置に依らないことの確認（今の `render_surface` は位置を使わず、拡大と `BeginDraw` の位置ずれだけを使う＝読みでは依らない）。
- 4 番: 後から入れる係をどのスケジュールに置くか（`VisualGraphics` を作る `PreRenderSurface` より前）。
- 1 番: `Option` に変えたあとも、装置が無い回に変化の印を食べない作りにするか（6 番と同じ問題）。

---

## 6. 要件の討議へ持っていく議題（答えで作業が変わるもの）

- **D-1（6 番）デバイスロストの立ち直り**: ㋐ 見つけて全部を作り直す道を作る（新しい機能に当たるか・規模 L） ㋑ 立ち直りは作らないと決め、使われない作り直しの道を消す ㋒ 別の spec に切る。推す案は ㋒（この spec は「直す」に絞り、立ち直りは作りが要るため）。ただし ㋒ でも 6 番の中の「`Update` で WUC の部品を捨てる」は消すか単スレッドへ移す。
- **D-2（2 番）`DolaAnimator`**: ㋐ 使い手 0 なので消す（`ANIM` の旗の生産者も消える） ㋑ `Sync` の要らない形へ作り替えて残す。推す案は ㋐（dola の本体は消さない・ECS の包みだけ）。
- **D-3（4 番）窓に後からつないだ部品**: 「窓に属さない部品には見た目を入れない」は意図か。意図なら「後から窓へつないだら入れる」を足す。意図でない（いつでも入れる）なら別の直し方になる。推す案は前者。
- **D-4（5 番）片方だけ決まった親を飛ばす受け継ぎ**: 今のテストが固定している「前景だけ決まった親を飛ばして祖父母から取る」は仕様か。仕様なら 5 番は「同じ回の順で変わる」だけ直す。仕様でないなら「決まった方の色は親から取る」へ直す。
- **D-5（14・19 番）使われない公開の関数・型を消すか**: wintf は crates.io に出すライブラリ。`insert_visual`・`message_window`・`Size::validate` などを消すのは公開の形の変更になる。消してよいか。推す案は「消す」（版はまだ 0.x で、使い手は木の中だけ）。
- **D-6（13 番）VSYNC の数え**: ディスパッチの等価性のためにわざと残したもの（⑵）として残すか、消すか。消す場合、`VsyncTick::try_tick_on_vsync` の名前も変えるか（変えると `window_proc/window_pos.rs` を 1 か所直す）。推す案は「消して名前も変える」。
- **D-7（22〜30 番）作りかけの描画の木**: ㋐ 9 件とも `wintf-visual-tree` へ申し送る ㋑ この spec で直す。推す案は ㋐（傷 1〜8 と同じ物・名前の変更はあちらの議題 2）。30 番の「残す理由の doc」だけはこの spec で付ける案も可。
- **D-8（34 番）型の作りを試すだけのテストを消すか**: 「決定論のテストの網羅は必達」の決めとの関係。推す案は「消す」（網羅は振る舞いの網羅で、derive の動きは網羅の対象でない）。
- **D-9（進め方）**: 4 節の案 A・B・C のどれか。推す案は B（D-1 を ㋒、D-7 を ㋐ にすれば 18〜20 タスクに収まる見込み）。
- **申し合わせ（議題ではなく手順）**: `crates/wintf/tests/`・`crates/wintf/examples/`・`crates/wintf/src/com/`・`numerics/` は同じウェーブで走る `audit-wintf-runtime` の持ち物。3・5・6・14・16・18・19・20 番はそこのテストを直すので、設計の前にそのセッションと申し合わせる。`areka-emo-present`（同じウェーブの `audit-emo-present`）は 4・6・11 番の影響を受ける。

---

## 7. まとめ

- 範囲は 54 ファイル・10,945 行（コード 8,447・テスト 2,498）。全部読んだ。指摘は 34 件（必ず直す 6・直したほうがよい 24・できれば 4）。
- 必ず直すの芯は「描画の装置が作れない回に落ちる」「`DolaAnimator` の `Sync` が嘘」「見た目の木と色の受け継ぎが作る順やつなぐ順で壊れる」「デバイスロストから立ち直れない」。
- 直したほうがよいの多くは、死んだ数え・使わない欄・古い説明・二重のコードで、消せば短くなる。`world/mod.rs` は 15 番で番人から遠ざかる。
- `visual/` の 9 件は `wintf-visual-tree` の傷 1〜8 とほぼ同じ物で、申し送るのが二重の仕事を避ける道。
- タスクは全部この spec でやると 21〜27 で 20 を越える。申し送りとデバイスロストの切り出しで 18〜20 に収まる見込み。

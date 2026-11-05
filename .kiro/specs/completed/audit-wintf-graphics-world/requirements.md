# Requirements Document

## Project Description (Input)
開発者は、areka のコードに残っているバグ・危ない所・負荷に弱い所・テストの抜け・遅い所・要らないコードを、一度まとめて直したい（2026-10-19 開発者「とにかく指摘事項は全部直す方針です。」）。今までは、各 spec が自分の触る所だけを見てきて、全体を同じ物差しで見直したことはない。物差しは ponytail のプラグイン（5.3.2）の `/ponytail:ponytail-audit`。全 33 本の見直しのうち、この spec は **wintf の描画・World・見た目の木・共通の型** を受け持つ。受け持つ範囲に audit をかけ、出た指摘を 3 つの組（必ず直す・直したほうがよい・できれば）とも全部直し、直した後にもう一度 audit をかけて新しい指摘が 0 件で、全体テストが緑になっている状態にする。

## Introduction

この spec は、wintf の `crates/wintf/src/ecs/` のうち、ほかの wintf の見直し 3 本（入力・レイアウトと部品・窓）の持ち物でない所をまとめて見直し、指摘を全部直す。

要件の段では「受け持つ範囲の指摘を、3 つの組とも全部直す」という形だけを決める。指摘の中身は、ギャップ分析で audit をかけて洗い出し、要件の討議で直す対象として確定する（2026-10-19 開発者「原則、ギャップ分析でauditを実施して、設計以降で修正」）。直し方は設計で決める。

受け持つ範囲は、着手のとき（main `88eb0a43`・2026-11-05）に測り直した。起票のとき（main `e3c2cbe4`）の 48 ファイルから、その後に入ったマウスカーソルの置き場 `cursor/`（6 ファイル）の分だけ増えている。

| 測った時点 | ファイル | コード | テスト |
|---|---|---|---|
| 起票（main `e3c2cbe4`） | 48 | 8,110 行 | 1,958 行 |
| 着手（main `88eb0a43`） | 54 | 8,447 行 | 2,498 行 |

（テストには `_tests.rs`・`tests.rs`・`tests/`・`test_support.rs`・`_test_support.rs` を数えた。）

## Boundary Context

- **In scope**:
  - `crates/wintf/src/ecs/` のうち、`drag*`・`pointer*`・`clickthrough*`・`tooltip*`・`layout*`・`widget*`・`window*`（`window/`・`window_proc/` を含む）でないもの全部。着手のときの中身は `graphics/`・`world/`・`visual/`・`common/`・`dola/`・`cursor/`・`types.rs`・`app.rs`・`mod.rs`・`test_support.rs`。
  - 受け持つファイルの指摘の直しと、そのためのテストの追加と直し。
  - 直しのために要る、範囲の外の呼び出し元の直し（要件 3 の約束に従う）。
- **Out of scope**:
  - 範囲の外の見直し（ほかの 32 本の持ち物）。
  - 新しい機能。
  - `vendors/`。
  - clippy の指摘の機械的な直し（`clippy-199-lints` の持ち物。audit が同じ所を指したら、その指摘としては直してよい）。
  - 開発者の裁定（roadmap の「生きている決まり」・記憶の決めごと・完了した spec の要件）を覆す直し。直さずに議題にする。
- **Adjacent expectations**:
  - `audit-wintf-input`・`audit-wintf-layout-widget`・`audit-wintf-window`（同じ `ecs/` の残り）と `audit-wintf-runtime`（`ecs/` の外の wintf。`numerics/` はあちらの持ち物）は、範囲が重ならない。`audit-wintf-runtime` は同じウェーブで並んで走る。
  - `wintf-visual-tree`（v0.2.0）は、この spec の着地を待って、`ecs/visual/` と、そこからだけ使われる `numerics/` の働きを本番の描画層へ上げる。この spec はそれらを使われていないことを理由に消さない。
  - `doc-tree-tidy`（同じウェーブ）は `doc/`・`docs/` の名前を変える。この spec は、受け持つファイルで `doc/`・`docs/` を名指す行を足さず、変えない。
  - 同じウェーブでは、誰も依存を足さない（`Cargo.lock` の席は空けておく）。

## Requirements

### Requirement 1: 受け持つ範囲を確定して全部を見る
**Objective:** As a 開発者, I want 受け持つ範囲のファイルがもれなく見直しの対象になっていること, so that 見直しの抜けやほかの spec との二重の見直しが起きない

#### Acceptance Criteria
1. When ギャップ分析を始めるとき, the 見直しの作業 shall その時点の main で受け持つ範囲のファイルを列挙し、ファイル数とコード・テストの行数を記録に残す。
2. If 着手のときより後に `crates/wintf/src/ecs/` へ足されたファイルが、ほかの wintf の見直し 3 本の持ち物の名前（`drag*`・`pointer*`・`clickthrough*`・`tooltip*`・`layout*`・`widget*`・`window*`）に当たらないなら, the 見直しの作業 shall そのファイルを受け持つ範囲に含める。
3. The 見直しの作業 shall 受け持つ範囲のファイルを全部読み、読まなかったファイルを残さない。
4. The 見直しの作業 shall 受け持つ範囲の外のファイルを見直しの対象にしない。

### Requirement 2: 指摘を全件洗い出して記録する
**Objective:** As a 開発者, I want audit の指摘が数の上限なしに全部記録されていること, so that 直す対象を要件の討議で漏れなく確定できる

#### Acceptance Criteria
1. When ギャップ分析を行うとき, the 見直しの作業 shall 受け持つ範囲だけを対象に `/ponytail:ponytail-audit` をかける。
2. The 見直しの作業 shall audit の「最大 20 件」の上限を外し、出た指摘を全件記録して、省いた指摘を残さない。
3. The 見直しの作業 shall 指摘の一覧をギャップ分析の報告に残し、各指摘に audit の番号と組（必ず直す・直したほうがよい・できれば）をそのまま付ける。
4. When 要件の討議を行うとき, the 見直しの作業 shall 指摘の一覧を直す対象として要件に落とし、直す対象を確定する。

### Requirement 3: 指摘を 3 つの組とも全部直す
**Objective:** As a 開発者, I want 確定した指摘が 3 つの組とも全部直っていること, so that 受け持つ範囲にバグ・危ない所・要らないコードが残らない

#### Acceptance Criteria
1. The 見直しの作業 shall 要件の討議で直す対象と確定した指摘を、必ず直す・直したほうがよい・できれば の 3 つの組とも全部直す。
2. When バグか危ない所の指摘を直したとき, the 見直しの作業 shall その直しが元に戻ったら落ちるテストを、指摘 1 件につき 1 本以上置く。
3. When コードやファイルを消す前に, the 見直しの作業 shall ツリー全体（テスト・fixture・設定・文字列での参照を含む）で使われていないことを確かめる。
4. When 直しで関数や型の形が変わるとき, the 見直しの作業 shall ツリー全体で呼び出し元を探し、範囲の外のものも含めて全部直す。
5. If 直しが及ぶ範囲の外のファイルが、走っているほかの見直しの spec の持ち物なら, the 見直しの作業 shall そのセッションと申し合わせてから触る。
6. The 見直しの作業 shall 指摘の直しのほかに新しい機能を足さない。
7. If 直しが `wintf-visual-tree` の brief の「直すべき傷」1〜8 と重なるなら, the 見直しの作業 shall どの傷と重なったかを、この spec の記録に残す。

### Requirement 4: 直さずに残す指摘を裁定に回す
**Objective:** As a 開発者, I want 直さずに残す指摘が決められた 3 種類だけで、どれも自分の裁定を経ていること, so that 「全部直す」の方針が黙って崩れない

#### Acceptance Criteria
1. The 見直しの作業 shall 直さずに残してよい指摘を、次の 3 種類だけに限る。⑴ 開発者の裁定に反する直し ⑵ わざと置いた先送りの印（実在の理由の doc を付けた `#[allow(dead_code)]` の語彙の口と、限界が今の負荷を越えていない `shortcut:`・`ponytail:` の注釈） ⑶ 外から見た振る舞いが変わる直しのうち、SSP との互換や ukadoc の正典に関わるもの。
2. When 指摘が直さずに残す 3 種類のどれかに当たるとき, the 見直しの作業 shall その指摘を要件の討議の議題にし、開発者の裁定を得てから扱いを決める。
3. If 指摘が SSP との互換や ukadoc の正典に関わる振る舞いの変更なら, the 見直しの作業 shall 議題にする前に ukadoc で確かめ、出典を添える。
4. If audit が `crates/wintf/src/ecs/visual/` か、そこからだけ使われる `crates/wintf/src/numerics/` の働きを「使われていない」と指したなら, the 見直しの作業 shall 消さずに `wintf-visual-tree` へ申し送る案として要件の討議で開発者に確かめる。
5. The 見直しの作業 shall 直さずに残すと裁定された指摘を、理由と裁定とともに記録に残す。

### Requirement 5: 直しのあいだ守る決まり
**Objective:** As a 開発者, I want 直しがプロジェクトの決まりと並走の約束を破らないこと, so that 見直しが新しい問題やほかのセッションとのぶつかりを生まない

#### Acceptance Criteria
1. The 見直しの作業 shall どのファイルも 1,000 行を越えさせず、足す中身が越えさせるなら新しいファイルへ置く。
2. The 見直しの作業 shall 直しのテストを、毎回同じ結果になるテストで書く。
3. If 直しが失敗の道を持つなら, the 見直しの作業 shall その失敗をエラーのログに残し、呼び出し元へ失敗として返して、記録の無い失敗の道を作らない。
4. When 直しが終了の道に触れるとき, the 見直しの作業 shall 終了の道を正規の手順のまま保ち、その場しのぎの近道で置き換えない。
5. The 見直しの作業 shall 実機の根と一時フォルダを、ワークツリーの `target` の下だけに置く。
6. The 見直しの作業 shall GPU を使うテストを、実際の GPU で回す。
7. If 文字化けが出たなら, the 見直しの作業 shall コンソールの文字コードを変えて逃げず、読む側を直す。
8. If 直しに新しい依存が要るなら, the 見直しの作業 shall 依存を足さずに止め、開発者の承認を仰ぐ（同じウェーブでは誰も依存を足さない約束。承認されたら `tech.md` へ登記する）。
9. The 見直しの作業 shall 受け持つファイルで `doc/`・`docs/` を名指す行を足さず、変えない。

### Requirement 6: 直し終わりを確かめる
**Objective:** As a 開発者, I want 直し終わりが新しい audit と全体テストで確かめられていること, so that 「全部直った」をあとから疑わずに済む

#### Acceptance Criteria
1. When 直す対象を全部直し終えたとき, the 見直しの作業 shall 同じ受け持つ範囲へもう一度 `/ponytail:ponytail-audit` をかける。
2. If 2 度目の audit で新しい指摘（直しが生んだ指摘を含む）が出たなら, the 見直しの作業 shall それも直すか、要件 4 の 3 種類に当たるなら裁定に回し、もう一度 audit をかける。
3. The 見直しの作業 shall 最後の audit で新しい指摘が 0 件であることを、0 件と明記して記録に残す。
4. The 見直しの作業 shall 全体テスト（`pwsh -NoProfile -File tools/test-all.ps1`）が緑であることを確かめる。
5. If 直す対象の指摘が多く、タスクが 20 を越えそうなら, the 見直しの作業 shall 要件の討議で、指摘の組ごとに同じ範囲の 2 本目の spec として切る案を開発者に示す。

## 指摘の扱い（2026-11-05 要件の討議で確定）

要件 2.4 に従い、ギャップ分析（`research.md`）の指摘 34 件と、`audit-wintf-runtime` から申し送られた 1 件（35 番）を、直す対象として確定した。開発者「承認」（全議題を推奨案で確定。議題 1 と 9 は、開発者の問い「分ける先が２０タスクを超えるのか？」「どうしたいの？」を受けて推奨を改めた後に承認された）。番号は audit の番号のまま。

| 番号 | 組 | 指摘（一行） | 扱い |
|---|---|---|---|
| 1 | 必ず直す | 描画の装置が作れなかった回にアプリが落ちる | 直す |
| 2 | 必ず直す | `DolaAnimator` の `Sync` が嘘 | 直す。**消さない**（議題 2）。型は完了した `wintf-P0-dola-boundary` の要件 2 が決めたもので、`element-animation-easing` もつなぐ相手に挙げている。中を読む関数もすべて `&mut` を通す形にして、あの要件の安全の根拠（`Query<&mut>` だけで触る）を本当に成り立たせる |
| 3 | 必ず直す | 見た目の木の親子のつなぎが一度きりで、付け替えと「親が後から見た目を持つ」に追従しない（P47・`audit-wintf-runtime` の 35 番と同じ物） | 直す。今の誤りを固定する `crates/wintf/tests/visual/hierarchy_reparent_gap_test.rs` は、正しい挙動のテストへ書き換える |
| 4 | 必ず直す | 窓へ後からつないだ部品がずっと描かれない | 直す（議題 3）。「窓に属さない部品には見た目の部品を入れない」は意図どおりとして残し、後から窓へつないだら入れる |
| 5 | 必ず直す | 色の受け継ぎの答えが、親と子を作った回が同じかで変わる | 直す（議題 4）。「片方だけ決まった親を飛ばす」は仕様ではない。親が決めている色は親から取り、同じ回に作った親子でも答えを同じにする。完了した `brush-component-separation` の決め「一度決まった色は、後で親が変わっても追わない」は守る |
| 6 | 必ず直す | デバイスロストから立ち直れない | **この spec で直す**（議題 1）。`invalidate()` と世代の番号は在り、`.kiro/steering/structure.md` も対応済みと書く仕組みの仕上げなので、要件 3.6 の「新しい機能」に当たらない。見つける所・無効化を窓と見た目と描画面の部品まで届けること・単スレッドのスケジュールへの移し・無効な回に変化の印を取りこぼさないことを含む |
| 7〜12 | 直したほうがよい | 開発用の旗・D2D の工場・親子の輪・記録の無い失敗・位置だけの描き直し・古いスケジュールの説明 | 直す |
| 13 | 直したほうがよい | 死んだ VSYNC の数え | 直す（議題 6）。数え 3 つと `EcsWorld::try_tick_on_vsync` を消し、トレイトの関数も中身に合う名前へ変える。呼び出し元 `crates/wintf/src/ecs/window_proc/window_pos.rs` を 1 か所直す。2026-11-05 の設計の討議で、トレイト `VsyncTick` をやめて自由な関数 `flush_window_pos_commands_guarded()` に替えると改めた（数えを消すと中身が `self` を使わなくなるため・呼び出し元は同じ 1 か所） |
| 14 | 直したほうがよい | `EcsWorld` の使われない欄と関数 | 直す（議題 5）。公開の関数を消してよい |
| 15〜18 | 直したほうがよい | `world/mod.rs` の重複テスト・使わない描く文脈・使わない引数とコメントアウト・二重のコード | 直す |
| 19 | 直したほうがよい | 使われない公開の小物 | 直す（議題 5）。公開の形を消してよい（版はまだ 0.x・使い手はこのリポジトリの中だけ） |
| 20〜21 | 直したほうがよい | 写しを試すテスト・表と `match` の二重 | 直す |
| 22〜29 | 直したほうがよい | 作りかけの描画の木 `visual/` の傷 | **直さずに `wintf-visual-tree` へ申し送る**（議題 7・要件 4.1 の ⑵ として裁定済み）。あちらの brief の「直すべき傷」1〜8 と一対一（22＝1・23＝2・24＝3・25＝4・26＝5・27＝6・28＝7・29＝8） |
| 30 | 直したほうがよい | `visual/draw/` の `allow(dead_code)` に残す理由の説明が無い・README が型の写し | 残す理由の説明はこの spec で付ける。README の写しは `wintf-visual-tree` へ申し送る（議題 7） |
| 31〜33 | できれば | 切り抜きの説明・カーソルの控えの残り・テストだけの関数の `allow` | 直す |
| 34 | できれば | derive の動きを試すだけのテスト | 消す（議題 8）。網羅は判断の分かれ目の網羅で、derive の動きは対象にしない |
| 35 | 直したほうがよい | 単一スレッドで回すスケジュール 6 本の約束を誰も確かめていない（`audit-wintf-runtime` の 30 番から申し送り） | 直す。単一スレッドのスケジュールの一覧を `world/mod.rs` の定数に置き、`set_executor` はその一覧から回し、一覧をテストで固定する |

- **進め方**（議題 9）: この spec 1 本で直す。2 本目には切らない（要件 6.5 の案は示し、退けた）。見立ては 18〜23 タスク。同じファイルに閉じる直しを 1 タスクにまとめて数を抑える。
- **範囲の外のテストの書き換え**（2026-11-05 `audit-wintf-runtime` の討議での開発者の裁定）: `crates/wintf/tests/` と `crates/wintf/examples/` のうち、この spec の直しが及ぶもの（`hierarchy_reparent_gap_test.rs`・`brushes_system_test.rs`・`window_pos_systems_test.rs`・`reinit_unit_test.rs`・`components_test.rs`・`lazy_reinit_pattern_test.rs`・`world_lifecycle_test.rs`・`dola_animator_test.rs`・`insert_visual_test.rs`・`tests/visual.rs`・`owner_window_exists_test.rs`・`examples/graphics_reinit_test.rs`）は、本番の直しと一緒にこの spec で書き換えてよい（要件 3.5 の申し合わせは済み）。`crates/wintf/src/runtime/tick_bridge.rs` の説明の行には触らず、`audit-wintf-runtime` が着地の後に合わせる。
- **申し送り**: 22〜29 番と 30 番の README、8 番の「`GraphicsCommandList` の `Send`/`Sync` の注記そのものが不正確」は、`wintf-visual-tree` の brief の末尾へ書き足した。

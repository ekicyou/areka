# 設計の検証: audit-wintf-layout-widget

- 検証した日: 2026-11-06
- 読んだもの: `spec.json`（言語 ja・phase design-generated）・`requirements.md`（「指摘の扱い」の表まで）・`design.md`（543 行）・`research.md`・steering の `product.md`・`tech.md`・`structure.md`
- 手で確かめたコード（この枝の HEAD `d45f7fed`）:
  - 触らない約束: 設計の「触るファイル」に `crates/wintf/src/ecs/mod.rs`・`ecs/world/mod.rs`・`ecs/visual/`・`ecs/graphics/`・`Cargo.toml`・`Cargo.lock` は 1 つも入っていない。係の名前を残す形なので、`ecs/world/mod.rs` の登録の行も変わらない。
  - areka が使う名前: 25 番で消す `PhysicalPoint` は `layout/hit_test/mod.rs:50` の別名だけ。`crates/wintf/src/ecs/mod.rs:35` が外へ出しているのは `pointer` の側の `PhysicalPoint`（整数の `Point`）で、`crates/areka` が使うのもそちら（`emo2_boot/frame/dpi.rs:11`・`frame_shell_box_arrow_tests.rs:20`）。`layout` の別名を名指すのは `crates/wintf/examples/taffy_flex_demo/main.rs:88`・`region.rs:15` だけで、設計の「範囲の外で直すファイル」に入っている。23・24・26 番で消す型と関数は、`crates/areka`・`areka-emo-text`・`areka-emo-present`・`ecs/mod.rs`・`ecs/world/mod.rs` での参照が 0 件。`crates/areka` が触る公開の形は `crates/areka/examples/mock-shell.rs:609` の `bitmap.path` だけで、設計は 28 番でこれを前提つきに分けている。
  - 13 番（`Arrangement` を足したときのフックを `insert_if_new` に）の影響: 下の「13 番の確かめ」に書いた。

## 設計の検証のまとめ

指摘 36 件のそれぞれに、直し方・呼び出し元・テストの置き場・元に戻したら落ちるテストが付いていて、要件の表（1.1〜6.7）がすべて設計の節に結びついている。触らない約束（`ecs/mod.rs`・`ecs/world/mod.rs`・`ecs/visual/`・`ecs/graphics/`・`crates/areka/`・`Cargo.lock`）はコードでも守られる形になっている。残る心配は、前提つきの 28 番で完了の条件が開いたままになること・13 番の影響を見る先の書き違い・10 番の空の命令の列が一時の失敗も止めてしまうことの 3 つで、どれも設計の討議で片づけられる。

## 13 番の確かめ（取りまとめ役から頼まれた点）

- 今のフック（`crates/wintf/src/ecs/layout/arrangement.rs:39-47`）は、`Arrangement` が初めて付いたときに `(GlobalArrangement::default(), ArrangementTreeChanged)` を命令で入れ、生成のときに一緒に渡した `GlobalArrangement` を既定値で上書きする。`Visual`・`Window`・`LayoutRoot`・`Monitor`・`BitmapSource` の付けたときのフックはどれも `Arrangement` を入れるので、これらと一緒に既定でない `GlobalArrangement` を渡した生成も同じに上書きされる。
- 木の全体で「この連鎖を起こす部品」と「既定でない `GlobalArrangement`」を同じ生成（または同じ `insert`）に入れている所を機械で数えた（`crates/` の `.rs` を全部・補助の関数が返す `GlobalArrangement` も数えた）。
  - `crates/wintf/src/ecs/window_proc/mouse_dblclick_wheel_tests.rs:58` — `Window::default()` と `arrangement(100.0, 200.0, 500.0, 500.0)`（既定でない `GlobalArrangement`）を一緒に生成する。**今は窓の値が既定値に上書きされ、直した後は渡した値が残る。** ただしこのテストが押すのは全部クライアント座標 (60,60)〜(70,60)（画面 (160,260) あたり）で、子の部品（`Arrangement` を持たないので上書きされない・画面 (150,250)〜(250,300)）の中にある。押した先からは `DragConfig` を持つ祖先の窓を探すので、確かめている答え（準備の対象・開始の知らせの数）は変わらない見込み。
  - `crates/wintf/tests/layout/graphics_sync_test.rs:34` — すでに `Arrangement` を持つ部品への `insert` なので、付けたときのフックは走らない。影響なし。
  - `crates/wintf/tests/layout/graphics_sync_test.rs:86`・`crates/wintf/tests/drag/window_dragging_filter_test.rs:91` ほか — 渡しているのは既定と同じ値（単位行列・大きさ 0）。答えは変わらない（設計も同じことを書いている）。
- `crates/areka-emo-text` は影響を受けない。`marker_surface_tests.rs:319` と `surface_hit_cells_tests.rs:19` は既定でない `GlobalArrangement` を渡すが、一緒に生成するのは `HitTest`・`WindowPos`・`AlphaMaskResource`・`ChildOf` だけで、連鎖を起こす部品を含まない（`Arrangement` が付かない）。`marker_surface_tests.rs:368` の `insert(arranged(..))` は、本番の `attach` が作った部品へ後から入れるもので、そのときにはフックの命令はもう反映されている（`spawn` と `insert` は命令を流す）。`areka-emo-present` の 2 つのテスト（`display_tests.rs:82`・`mount_tests.rs:282`）は掛け算の式を確かめるために値を作るだけで、生成に使っていない。
- 本番のコードで、連鎖を起こす部品と既定でない `GlobalArrangement` を一緒に生成する所は 0 件（`layout/mod.rs:21` の説明の例だけ）。
- 伝播の穴も無い: `ArrangementTreeChanged` を「無ければ入れる」にしても、`mark_dirty_trees`（`crates/wintf/src/ecs/common/tree_system.rs:55`）は `Changed<Arrangement>` で木に印を付けるので、付け直した `Arrangement` の伝播は今と同じに起きる。渡した `GlobalArrangement` が残るのは、最初の伝播までのあいだだけ。
- 設計の書き違い: `design.md` の Risks は、影響を見る先として emo-text の `marker_surface_tests.rs` を挙げているが、上のとおりこのテストは影響を受けない。実際に答えが変わりうる範囲の外のテストは `mouse_dblclick_wheel_tests.rs` で、設計はこれを挙げていない（下の重い指摘 2）。

## 重い指摘

### 🔴 重い指摘 1: 28 番の前提つきのタスクが満たされないとき、完了の条件が決まっていない
- **心配**: 28 番の後半（欄 `path` を非公開にし、`crates/areka/examples/mock-shell.rs:609` を `bitmap.path()` に替える）は、`placement-crate-split` が main に着地し、この枝が main を取り込んだ後にだけ行い、満たされなければ「手を付けずに止まり、開発者へ尋ねる」としている（タスクの見立ての 19）。止まったときに、この spec を 28 番の半分を残したまま完了にするのか、相手の着地まで完了を待つのかが書いていない。
- **影響**: 要件 3.1（全部直す）・6.3（最後の audit で 0 件）は、28 番が半分のままだと満たせない。2 度目の audit が「欄が書き換えられるのに何も起きない」をまた指す見込みが高く、要件 6.2 の回し直しが止まらない。
- **案**: 設計の討議で、道 ⒜（相手の着地の後へ回す）を開発者に確かめると同時に、満たされなかったときの扱いを決める。推すのは「この spec の完了は `placement-crate-split` の着地を待つ（タスク 19 と 2 度目の audit・全体テストは、取り込みの後に回す）」。待てない場合は、要件 4.5・4.6 の「理由を書いて残す」の裁定を取り、2 度目の audit の 0 件の数え方から 28 番の後半を外すと書く。
- **要件**: 3.1・4.5・4.6・5.12・6.3
- **設計の場所**: 「28 番の扱い（要件 4.5・案 ⒜）」・「タスクの切り方の見立て」の 19・「設計で決めたこと」の前書き

### 🔴 重い指摘 2: 13 番で答えが変わりうるテストの挙げ先が違う
- **心配**: Risks は emo-text の `marker_surface_tests.rs` を「束で渡しているので全体テストで見る」と挙げるが、このテストは `Arrangement` の連鎖を起こさないので影響を受けない。実際に生成のときの値が残るようになるのは `crates/wintf/src/ecs/window_proc/mouse_dblclick_wheel_tests.rs:58`（`Window::default()` と既定でない `GlobalArrangement`）で、設計の「範囲の外で直すファイル」にも「前後の確かめ」にも入っていない。
- **影響**: 確かめた限りではこのテストの答えは変わらない（押す点は全部子の部品の中）。ただ、このファイルは `audit-wintf-window` の受け持つ範囲（`ecs/window_proc/`）で、この spec は書き換えられない。赤になったときに「触れないファイルの赤」で止まる道を、設計が前もって持っていない。
- **案**: Risks と「前後の確かめ」の挙げ先を `mouse_dblclick_wheel_tests.rs` に直し（emo-text の名指しは外すか「影響なし・確かめ済み」と書く）、13 番のタスクで直す前と後にこのテストを回すと書く。赤になったら、テストの説明が上書きを前提にしているかを見て、`audit-wintf-window` へ申し送る（このウェーブには居ないので申し合わせの相手は居ない＝書き換えは開発者に確かめる）。
- **要件**: 2.6・3.4・3.5・5.11
- **設計の場所**: 「指摘ごとの直し方」の 13 番の行・Risks の 2 つ目・Testing Strategy の「前後の確かめ」

### 🔴 重い指摘 3: 10 番の「空の命令の列」は、最初の 1 回の一時の失敗も恒久の失敗として扱う
- **心配**: `store_command_list` は、失敗の種類を見ずに「前から命令の列を持っていたか（`had_list`）」だけで分ける。命令の列が無い部品（初めて描く部品と、デバイスの立ち直りで全部の列を外された直後の部品）は、最初の 1 回の失敗で空の列が入り、次の内容の変化かもう一度の立ち直りまで描き直さない。
- **影響**: デバイスが失われた場合は、立ち直りの係（`crates/wintf/src/ecs/graphics/systems/window_pos.rs:159-161`）が空の列も含めて全部外すので描き直される（確かめた）。それ以外の一時の失敗（負荷の高いときの記憶の不足など）では、絵の無い部品が残り、内容が変わるまで戻らない。今は毎回やり直すので、ここは振る舞いが変わる所。設計は「設計で決めたこと」の 1 で開発者に確かめる項目に挙げているが、テスト（10 番の行）は立ち直りとのつながりを固定していない。
- **案**: 討議で 1 の扱いを確かめるとき、この差（初めて描く部品の一時の失敗は戻らない）を開発者に見せる。今の案のままでよければ、10 番のテストに「空の列が入った部品も、立ち直りで列が外されると次の回に描き直される」を 1 本足す（`audit-wintf-graphics-world` の立ち直りと、この spec の約束のつなぎ目を固定する）。
- **要件**: 3.2・5.3
- **設計の場所**: 「描く骨組み（`widget/record.rs`）」の `store_command_list` の約束・「設計で決めたこと」の 1・Testing Strategy の 10 番の行・Revalidation Triggers の 1 つ目

## 設計のよい所

- **範囲の外への響きを行の番号まで押さえている**: 記録の文「Redriving window DPI from updated Monitor (no WM_DPICHANGED required)」が `crates/areka/src/placement/transition_judge_reobservation_tests.rs:74` の逐語の写しに使われていること、`monitor_systems_transition_tests.rs:32` が `include_str!` で `ecs/world/mod.rs` の字面を見張っていること、`Visual` のフック（`ecs/graphics/visual.rs:19`・`:135`）が `Arrangement` のフックの連鎖に頼っていることを、どれも「変えない」「変わったら確かめ直す」と書いている。並走の約束を破らずに直せる道が具体的に示されている。
- **写しを 1 つにまとめる形が小さくて確かめやすい**: 描く 5 か所を `record_command_list`・`store_command_list` の 2 つの関数に、当たり判定の 2 本を `hit_test_entity_ex` の 1 本に寄せ、足すものは関数 9 つとフック 1 つ・新しいファイル 4 つに抑えている。バグと危ない所の 18 件それぞれに「元に戻したら何が起きるか」の付いたテストがあり、要件 3.2 を満たす。

## 最後の判定

- **判定: GO**
- **理由**: 層の向き・触らない約束・areka が使う公開の形は守られ、36 件の直し方と 3.2 のテストが揃っている。上の 3 つは設計の作りを変えるものではなく、完了の条件の書き足し（1）・確かめる先の書き直し（2）・テスト 1 本と開発者の確認（3）で片づく。
- **次の一歩**:
  1. 設計の討議（`/kiro-design-discussion audit-wintf-layout-widget`）で、上の 3 つと「設計で決めたこと」の 1〜6・28 番の道 ⒜ を議題にする。
  2. 決まったことを `design.md` に入れてから、`/kiro-spec-tasks audit-wintf-layout-widget` でタスクを作る。

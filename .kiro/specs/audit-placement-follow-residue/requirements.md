# Requirements Document

## Project Description (Input)

開発者は、完了した `audit-placement-follow` の範囲（配置の crate `areka-placement` のうち追随と移り変わりの判定）に残った指摘を、全部直したい（2026-10-19 開発者「とにかく指摘事項は全部直す方針です。」）。今の状況: `audit-placement-core` の audit のかけ直し（1〜9 回目）と鎖の決まりの総括（同 spec の `research.md` 18.1〜18.10）は、追随の持ち物の中にも指摘を見つけたが、そこは `audit-placement-follow` の範囲なので 1 行も触らずに申し送りへ回した。`audit-placement-follow` は 2026-11-14 に完了していて、このままでは直す spec が無い（同 `research.md` 19.2）。この spec がその受け手になる。中身は 4 つ＝⑴ 記憶から戻した窓を作業領域へ寄せるときの組が、鎖の決まり（`crates/areka-placement/src/chain_rule.rs`）の「鎖の前の窓との組」でなく「番号が続く対 (n−1, n)」で作られている ⑵ wintf がドラッグを始めた窓の位置を取れないと、ドラッグの状態に (0,0) が入り、追随がそれを基準に窓を動かしうる（164） ⑶ 共有の部品の追随の側の写し・消えた名前を指す説明・名前の無い番号 ⑷ `follow/drag_follow.rs` がちょうど 1,000 行で 1 行も足せない。変えたいこと: 上の件が全部直り、追随の寄せの組が鎖の決まりと同じになり、ドラッグの始めの位置が分からないときに (0,0) を基準に窓を動かさず記録に残し、`drag_follow.rs` が上限の下に入り、直した振る舞いの 1 つずつに壊れたら落ちるテストがあり、全体テストが緑であること。

## Introduction

この文書は、`audit-placement-core` が追随の持ち物に残した申し送りを全部直すための要件を定める。件はもう出揃っている（brief の Current State）ので、audit を新しくかけることは本体にしない。ギャップ分析は、件の行を `main` で引き直すことと、件ごとの直し方の下ごしらえにあてる。

件の番号は、どれも `audit-placement-core` の `research.md` の件の番号（83・164・431 ほか）か、同 spec のかけ直しの回（「2 回目」「4 回目」ほか）をそのまま使う。行の番号は brief の「2026-11-17 棚卸㊴の再測定」（`main` `7f41b53e`）のもので、着手のときに `main` で測り直す。

受け持つ件の塊:

| 塊 | 件 | 要件 |
|---|---|---|
| 鎖の寄せ | 18.10 の表の F の列（T6・T8）・C14（`opposite`）・C16・`RestoredWindow::mark` と `keep_restored_groups` の説明 | 1 |
| ドラッグの始めの位置 | 164（`audit-wintf-input` の申し送り件 1） | 2 |
| 1,000 行 | `follow/drag_follow.rs` の分け方 | 3 |
| 写し | 83・91 の残り・2 回目（`point`・`contained`・`Store`・記録の捕まえの写し・220・219・218）・358・426・431・432・491・511 | 4 |
| 記憶へ書く回数 | 7 回目（組のドラッグの終わり） | 5 |
| アンカーの変化の書き込み | 4 回目（`PlacementRoute::AnchorChange` の振る舞い） | 6 |
| 古い文と名前の無い番号 | 3 回目・4 回目の文・`follow.rs` の古い名前・名前の無い番号（48 ファイル・218 行と 18 ファイル・39 行）・`audit-emo2-boot-spine` の brief の申し送り ⑵ | 7 |

## Boundary Context

- **In scope**:
  - 上の表の件の直しと、そのためのテストの追加と直し。
  - `follow/drag_follow.rs` の分け方。
  - 164 のための、wintf の `ecs/drag` のドラッグの状態の型と、それを組む所・読む所の直し（範囲の外の呼び手を含む・brief の再測定の 11 か所と読み手）。
  - 491 のための、`balloon_limit.rs`（2 か所）・`reseed.rs`・`persist.rs` の矩形の式の差し替え。
  - `crates/areka-runtime/src/placement_tests/transition_*`（16 ファイル・`runtime-crate-split` で移った置き場）の古い説明と名前の無い番号。
- **Out of scope**:
  - 鎖の決まりそのものの変更（`chain_rule.rs` の規則は `audit-placement-core` が決めた仕様のまま使う）。
  - `defaultx` の読み（`defaultx-canon` の持ち物・この spec の着地の後に取る）。
  - 新しい機能。
  - `vendors/` の下。
  - clippy の指摘の機械的な直し（`clippy-199-lints` の持ち物）。
  - 1,000 行の番人の例外表とその数の行の変更（`audit-test-kits` の持ち物）。
  - 開発者の裁定（roadmap の「生きている決まり」・記憶の決めごと・完了した spec の要件・`audit-placement-core` の `research.md` 18.10「この仕様で行く」と「残す限り」の 8 つ）を覆す直し。直さずに議題にする（要件 8）。
- **Adjacent expectations**:
  - **Upstream**: `audit-placement-core`（着地済み）。鎖の決まり `chain_rule.rs` の共有の判じ方（`ChainRule`・`judge_adjacency`・`resolve_chain`・`resolver::joins_chain`）は直す所が無いものとして呼ぶ。
  - **Downstream**: `defaultx-canon`・`audit-emo2-boot-frame`・`balloon-flip-ssp-review`・`input-crate-split`・`audit-areka-input-menu` は、この spec の着地を待つ（roadmap の C23〜C25）。
  - wintf は crates.io へ出すクレート（`publish = true`）。ドラッグの状態は公開の型なので、形を変えるなら公開の段への影響を設計に書く（要件 2.6）。前例は `WindowDragContext.initial_window_pos: Option<Point>`（`crates/wintf/src/ecs/drag/context.rs`）。
  - 同じウェーブ C22 のほかの 5 本（`collisionex-regions`・`audit-emo2-boot-spine`・`audit-test-kits`・`audit-ukadoc-survey`・`audit-pilot`）との約束は要件 9 に書く。

## 確定した件（要件の討議で書き加える）

<!-- 要件の討議で、件ごとに「直す／裁定で残す」と議題の結論をここに書き加える（要件 8.5）。 -->

## Requirements

### Requirement 1: 記憶から戻した窓の寄せの組を鎖の決まりに合わせる

**Objective:** As a ゴーストの利用者, I want 記憶から戻した窓を作業領域へ寄せるとき、組が鎖の決まりと同じ「鎖の前の窓との組」で作られること, so that 辺のスコープを持つゴーストや中のスコープが欠けたゴーストでも、寄せの途中で組がばらけたり、辺のスコープの窓と結ばれたりしない

#### Acceptance Criteria

1. When 記憶から戻した窓を作業領域へ寄せる, the areka shall 窓 n の組の相手を、鎖の決まり（`audit-placement-core` の `research.md` 18.10「この仕様で行く」3）の「鎖の前の窓」＝n より下の番号で鎖に入る一番近いスコープとする。**仮定**: 鎖に入るかは、覚えた印と同じく descript の揃え方で見る（`ChainView::Saved`・印は次の起動が読むので・brief の Approach）
2. When n の 1 つ下の番号のスコープが左右の辺のスコープで、n の覚えた印が `left`／`right` である, the areka shall n を鎖の前の窓（辺のスコープを飛ばした窓）との組として寄せ、辺のスコープと n を 1 つの組にしない（18.10 の表の T6）
3. When 中のスコープが欠けたゴースト（例: スコープ 0 と 2 だけ）で、2 の覚えた印が `left`／`right` である, the areka shall 0 と 2 を 1 つの組として寄せ、窓ごとに寄せない（T8）
4. While 窓が鎖に入らない（左右の辺のスコープ）, the areka shall その窓を寄せの組の相手にしない
5. When 寄せた組が作業領域に入らない, the areka shall 反対の側へ回すかどうかを、鎖の決まりに照らして確かめた結果のとおりに扱い、その理由を説明に書く（C14）。**仮定**: 今の振る舞い（`JoinLeftOffset` は反対の側へ回さず端へ寄せる・`JoinLeft`／`JoinRight` は反対の側へ回す）が鎖の決まりと合い、振る舞いは変えずに説明だけを書く（未決 3）
6. The areka shall 寄せの後の最後の位置を、今と同じく起動の確定が鎖の決まりどおりに直す形のまま保つ
7. The 本見直し shall 寄せの組を説明する文（`RestoredWindow::mark`・`keep_restored_groups`）に、組を「鎖の前の窓との組」と書き、「番号が続く対 (n−1, n)」と書かない
8. The 本見直し shall 追随の持ち物に鎖の式の写しを残さない（`audit-placement-core` の `research.md` 18.10「これで終わる理由」2 の grep で、追随の `joined_with_prev` の当たりが消える）
9. The 本見直し shall 2 と 3 の振る舞いを、18.10 の表の T6・T8 の形の World で、壊れたら落ちるテストで固定する

### Requirement 2: ドラッグの始めの位置が分からないとき（164）

**Objective:** As a ゴーストの利用者, I want ドラッグを始めた窓の位置が取れなかったときにも、追随の窓が画面の左上 (0,0) を基準に飛ばないこと, so that まれな条件でも窓が思わぬ所へ動かず、何が起きたかが記録で分かる

#### Acceptance Criteria

1. If ドラッグを始めた窓の位置が取れない, then the areka shall (0,0) をドラッグの始めの位置として扱わない
2. If ドラッグを始めた窓の位置が取れない, then the areka shall その回のドラッグで、分からない始めの位置を基準に追随の窓（組の相手・バルーン）を動かさない。**仮定**: 追随の窓は今の位置にとどまり、ドラッグした窓自身の扱いは wintf の今の守り（位置が無いときは窓を動かさない）のままとする（未決 5）
3. If ドラッグを始めた窓の位置が取れない, then the areka shall 位置が分からなかったことを、どの窓のドラッグかが分かる形で記録に残す（記録の水準は設計で決める・記録の無い失敗の道を作らない）
4. When ドラッグを始めた窓の位置が取れる, the areka shall 今と同じ振る舞いで追随の窓を動かす
5. When ドラッグの状態の形を変える, the 本見直し shall ドラッグの状態を組む所と読む所（brief の再測定の 11 か所と読み手・wintf の中の控え `DragState::Dragging.initial_window_pos` を型でそろえるならそれも）を、ツリー全体で探して全部直す
6. The 本見直し shall 「位置が分からない」の持たせ方（型に持たせるか、位置が分からないときはドラッグの状態を作らないか）を設計で比べ、wintf の公開の段（crates.io）への影響を設計に書く
7. The 本見直し shall 1〜4 の振る舞いを、wintf の側と追随の側でそれぞれ壊れたら落ちるテストで固定する

### Requirement 3: `follow/drag_follow.rs` を 1,000 行の上限の下に入れる

**Objective:** As a 開発者, I want `follow/drag_follow.rs` が 1,000 行の番人の上限の下に入っていること, so that 164 の直しと名前の無い番号の直しをそのファイルに入れられる

#### Acceptance Criteria

1. The 本見直し shall 要件 2 と要件 7 の直しが `follow/drag_follow.rs` に触る前に、そのファイルを分けて 1,000 行の上限の下に入れる
2. When `follow/drag_follow.rs` を分ける, the areka shall ドラッグの追随の振る舞いを変えない（分けるだけの段で、今のテストがそのまま通る）
3. The 本見直し shall 分けた後のどのファイルも上限の下に保ち、番人の例外表に足さない

### Requirement 4: 共有の部品の写しを消す

**Objective:** As a 開発者, I want 追随の側で手で写した共有の部品が無くなっていること, so that 共有の側を直したときに写しだけが古いまま残らない

#### Acceptance Criteria

1. The 本見直し shall 次の写しを消し、共有の側を使う形にする: 83（`assert_no_event`）・91 の残り（`follow/move_shift.rs` の `windowposition::scale_signed` を根の `scale_signed` へ）・2 回目（`follow_test_support.rs` の `point`・`contained`・`Store` の写し、`follow_move_shift_tests.rs` の `k`＝220、`follow_sticky_follow_drag_tests.rs` の `events_of`＝219）・358 と 426（`follow_test_support.rs` の 5 つの部品を crate の中へ開き、`spawn_follow_pipeline_tests.rs`・`balloon_limit_flip_tests.rs`・`balloon_lock_drag_tests.rs` の写しを消す）・431（`follow/visibility.rs` の `clamp_x_into` を resolver の `clamp_axis` へ）・432（`follow/work_area.rs` の `MonitorSnapshot::from_monitors` の式を根の `work_area_px` へ）・491（`balloon_limit.rs` の 2 か所・`reseed.rs`・`persist.rs` の手組みの矩形を `rect_at` へ）・511（`code_only`・`SharedFakeIo`）
2. Where 記録の捕まえの手書きの写し（`follow_transition_diag_tests.rs` の `resize_under_filter`・`follow_window_move_diag_tests.rs` の `window_move_output_under_filter`）, the 本見直し shall 共有の `capture_under_filter` へ寄せず、`diag_tests.rs` と同じく捕まえ損ねを見張る番兵を足す（`env-filter` を宣言してよい crate は wintf だけ・380）
3. The 本見直し shall `follow/sticky_follow.rs` の略称 `scg`（218）を、略さない名前にする
4. When 写しを消す, the 本見直し shall 消す前に、ツリー全体（テスト・fixture・設定・文字列での参照を含む）で使い手を確かめる
5. When 本番の式を共有の部品へ差し替える（431・432・491）, the areka shall 窓の位置と矩形を今と同じに保つ
6. The 本見直し shall 432 に合わせて `follow/work_area.rs` の説明が消えた `primary_work_area` を名指さないように直し、`lib.rs` の「根を引く子」の文に 1 行足す

### Requirement 5: 組のドラッグの終わりで記憶へ 1 回で書く（7 回目）

**Objective:** As a 開発者, I want 組のドラッグが終わったとき、記憶へ書くのが 1 回で済むこと, so that 1 回のドラッグの終わりで記憶へ N+1 回書くむだが無くなり、ほかの道（`audit-placement-core` の 50）と同じ形になる

#### Acceptance Criteria

1. When 組のドラッグが終わる, the areka shall 動かした窓と組の相手の位置を 1 つの束にまとめ、記憶へ 1 回で書く
2. The areka shall 束に入れる中身（各窓の位置と印・相手の保存は当てた分を引いた元の位置という `audit-placement-follow` の議題 7 の決め）を今と同じに保つ
3. The 本見直し shall 1 の書き込みの回数を、壊れたら落ちるテストで固定する

### Requirement 6: シェルの切り替えでのアンカーの変化の書き込み（4 回目）

**Objective:** As a 開発者, I want シェルの切り替えのたびに来るアンカーの変化の書き込みが、正しい書き込みとして扱われていること, so that DPI の同期の待ちのあいだにシェルを切り替えても、見張りが正しい書き込みを誤って違反と記録しない

#### Acceptance Criteria

1. The 本見直し shall DPI の同期の待ち（`DpiSyncHold`）を持つ窓のあいだにシェルを切り替えたとき、`reseed::reanchor_char_windows` の正しい書き込みで見張りが鳴るかを、テストで確かめる
2. If 1 で正しい書き込みに見張りが鳴る, then the areka shall その書き込みを違反として記録しない。**仮定**: 鳴るなら鳴らないように直す（直し方は設計で決める・未決 4）
3. The 本見直し shall `follow/window_move.rs` の `PlacementRoute::AnchorChange` の説明（「本番では未結線・現に到達しない」）・`resize_window_to` の上流の一覧・`follow/keyword_base.rs` の `AnchorChange` の扱いの説明を、今の事実（シェルの切り替えのたびに `reanchor_char_windows` が書く）に合わせる
4. The 本見直し shall 1 で確かめた振る舞いを、壊れたら落ちるテストで固定する

### Requirement 7: 古い文と名前の無い番号

**Objective:** As a 開発者, I want 受け持つ範囲の説明が今のコードと名前を指し、番号には spec の名前が付いていること, so that 説明を読んだ人が消えた名前や出どころの分からない番号に迷わない

#### Acceptance Criteria

1. The 本見直し shall 消えた名前や今の本番の道と違う名前を指す説明を、今の事実に直す（`follow.rs` の `persist::apply_restored_placements`＝本番は `restore_and_judge`・`audit-emo2-boot-spine` の brief の申し送り ⑵ の `crates/areka-runtime/src/placement_tests/transition_judge.rs` の古い置き場と「bin crate ゆえ `main.rs` が宣言する」の文）
2. The 本見直し shall 受け持つ範囲（`crates/areka-placement/src/` の `follow*`・`transition_*`、`crates/areka-runtime/src/placement_tests/transition_*`）の名前の無い古い spec の task 番号・指摘番号・要件の番号（`follow/move_shift.rs` の「監査の指摘 1」・`follow.rs` の「task 1.4」「task 2.4」・`follow/drag_follow.rs` の「2.4」「4.5」を含む）を、消すか spec の名前を足す（`audit-placement-core` の 165・166〜207 と同じ形）
3. The 本見直し shall コードの説明に開発者の記憶の帳面の名（`follow_drag_tests.rs` の `[[test-only-decision-branches-not-proven-wiring]]`）を残さない
4. When 直し終える, the 本見直し shall 受け持つ範囲で `git grep -nE "(task|タスク) ?[0-9]+"` と `git grep -nE "指摘 ?[0-9]+"` の当たりを仕分け、残る行がすべて spec の名前の付いた行であることを示す

### Requirement 8: 直さずに残してよいもの

**Objective:** As a 開発者, I want 残す件が決まった 3 つの場合だけで、どれも裁定を経ていること, so that 「全部直す」の方針から黙って外れる件が無い

#### Acceptance Criteria

1. The 本見直し shall 受け持つ件を全部直し、直さずに残すのは次の 3 つに当たるものだけにする: ⑴ 開発者の裁定に反する直し ⑵ わざと置いた先送りの印（実在の理由の説明を付けた `#[allow(dead_code)]` の語彙の口・限界が今の負荷を越えていない `shortcut:`・`ponytail:` の注釈） ⑶ 外から見た振る舞いが変わる直しのうち、SSP との互換や ukadoc の正典に関わるもの
2. If 件の直しが開発者の裁定（roadmap の「生きている決まり」・記憶の決めごと・完了した spec の要件・`audit-placement-core` の `research.md` 18.10「この仕様で行く」と「残す限り」の 8 つ）を覆す, then the 本見直し shall 直さずに議題にする
3. If 直しが SSP との互換や ukadoc の正典に関わる振る舞いを変える, then the 本見直し shall ukadoc で確かめてから扱いを決める
4. The 本見直し shall 鎖の決まりそのもの（`chain_rule.rs` の規則）と `defaultx` の読みを変えない
5. When 要件の討議で件の扱いが確定する, the 本見直し shall 件ごとの「直す／裁定で残す」と理由を、この文書の「確定した件」に書き加える

### Requirement 9: 守る決まりと同じウェーブの約束

**Objective:** As a 開発者, I want この spec が守る決まりと、同じウェーブ C22 のほかの 5 本との約束を守ること, so that 並べて走る spec と触るファイルがぶつからず、番人も緑のまま着地できる

#### Acceptance Criteria

1. The 本見直し shall どのファイルも 1,000 行の番人（`crates/log-capture-kit/tests/file_length_guard_test.rs`）の上限を越えさせない（行を足すなら新しいファイルへ。3 の置き場では新しいファイルを足せないので、行を増やさない直しにとどめる）
2. The 本見直し shall 番人の例外表とその数の行を変えず、表に載るファイル（`follow/window_move.rs`・`crates/areka-runtime/src/placement_tests/transition_judge_verdict_tests.rs` ほか）を割らない（表は `audit-test-kits` の持ち物・割るなら次のウェーブへ申し送る）
3. The 本見直し shall `crates/areka-runtime/src/placement_tests/` の下にファイルを足さず、消さない（その `mod.rs` は `audit-emo2-boot-spine` の持ち物）。上限に近いファイル（`transition_judge.rs`・`transition_judge_tests.rs` はどちらも 999 行）は、行を増やさない直しにとどめる
4. The 本見直し shall 依存を足さない（`Cargo.toml` の依存と `Cargo.lock` を変えない）
5. When 関数や型の形を変える, the 本見直し shall ツリー全体で呼び出し元を探して全部直し、相手の範囲の spec が走っていれば、そのセッションと申し合わせてから触る
6. If 直した道で失敗が起きる, then the areka shall その失敗を記録に残す（`error!`＋`Err`・記録の無い失敗の道を作らない）
7. The 本見直し shall 実機の根と一時フォルダをワークツリーの `target` の下だけに置く
8. The 本見直し shall `vendors/` に触らず、clippy の指摘の機械的な直しをしない

### Requirement 10: 確かめ

**Objective:** As a 開発者, I want 直したことが証拠つきで確かめられていること, so that 完了の判断を報告を読むだけでなく、テストと grep の結果で下せる

#### Acceptance Criteria

1. When 着手する, the 本見直し shall brief の件の行の番号を `main` で引き直し、ギャップ分析の報告に残す
2. The 本見直し shall 直した振る舞い 1 つにつき、それが壊れたら落ちる決定論のテストを 1 本以上置く
3. When 全部直し終える, the 本見直し shall 全体テスト（`pwsh -NoProfile -File tools/test-all.ps1`）が緑であることを示す
4. When 全部直し終える, the 本見直し shall 要件 1.8・7.4 の grep の結果と、要件 3 の行の数を、完了の記録に残す
5. Where 要件の討議で、直した後に受け持つ範囲へ audit をかけ直すと決めた, the 本見直し shall かけ直しで出た指摘も同じ方針（要件 8）で扱う。**仮定**: かけ直さない（件はもう出揃っている・`audit-placement-core` の 18.11 は開発者の決めで打ち切った・未決 1）

## 未決（要件の討議で決める）

1. **直した後に audit をかけ直すか**（要件 10.5）: 仮定＝かけ直さない。brief の Approach は「要件の討議で決める」としている。
2. **164 を同じ範囲の 2 本目に切るか**: 仮定＝切らずに 1 本で進める（見立て M 14〜19 タスク）。brief の Constraints は 20 タスクを越えそうなら切ってよいとしているが、2026-11-16 の決めごと（一度切り出した spec を規模を理由に後から分けない・20 タスクは spec の上限ではない）とどちらを取るかを確かめる。
3. **C14 の `opposite`**（要件 1.5）: 仮定＝今の振る舞い（`JoinLeftOffset` は反対の側へ回さず端へ寄せる・`JoinLeft`／`JoinRight` は反対の側へ回す）を鎖の決まりに照らして確かめ、合えば振る舞いを変えずに説明を書く。要件 1 の直しで組の相手が鎖の前の窓になった後、辺のスコープとの組（520 の直しで足したもの）がこの寄せに来るかどうかもギャップ分析で確かめる。
4. **4 回目の `AnchorChange` の振る舞い**（要件 6.2）: 仮定＝正しい書き込みで見張りが鳴るなら、鳴らないように直す。鳴らないと分かれば説明の直しだけにする。
5. **位置が分からないドラッグでの見え方**（要件 2.2）: 仮定＝その回のドラッグでは追随の窓を動かさない（今の位置にとどまる）。代わりの案は、最初の移動の知らせで位置を取り直して追随を始める形。どちらにしても (0,0) は基準にしない。

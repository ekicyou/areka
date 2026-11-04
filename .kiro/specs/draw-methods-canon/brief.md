# Brief: draw-methods-canon

> 2026-10-05 起票（`/kiro-discovery`）。出どころは spec `areka-P0-element-base-method` の完了時の棚卸。同 spec の要件の討議（2026-10-05）で開発者が「残りの描画メソッドは起票」を選び、あわせて方針「合成の方法は Direct2D が支える全部に対応できるよう、wintf も含めて広げる」を示した（`.kiro/specs/completed/areka-P0-element-base-method/requirements.md` の範囲外の節）。

## Problem

- **ゴーストの作者・利用者**: `overlay`・`base` 以外の描画メソッドで書いた element定義・pattern定義が描かれない。`overlayfast`／`overlay-fast`・`replace`・`interpolate`・`asis`・`reduce`・`blend-*`（加算・乗算・スクリーン・覆い焼きなど）・pattern定義の `base` を使うシェルでは、その部品や着せ替えが消えるか、土台に別の絵が残る。
- element定義では areka が描かない行を読み込み 1 回につき `warn!` で知らせる（`element-base-method`）。pattern定義では合成の段が描かずに飛ばす。どちらも見た目は正典と違う。
- 1 つずつ場当たりに足すと、合成の段の形（今は「SourceOver だけを整数で転写する」）がそのたびに歪む。

## Current State

- 読み手 `crates/areka-parsers/src/shell/decode.rs` の `is_image_element_method` が `overlay` と `base` だけを画像の element定義の値にする。ほかの語の行は `crates/areka-parsers/src/shell/undrawn.rs` の `parse_undrawn_elements` が拾い、`crates/areka-emo-present/src/shell_target.rs` の `load_shell_target` が 1 行 1 件 `warn!` する。`Element` には描画メソッドの欄が無い。
- 合成の段 `crates/areka-emo-compose`:
  - `method.rs` の `ComposeMethod` は全部の語を型として持つ（`Overlay`・`OverlayFast`・`Interpolate`・`Replace`・`Asis`・`Base`・`Reduce`・`Auto`・`Blend(BlendMode)`・`Unknown`）。`is_implemented` は `Overlay` だけ真。
  - `blit.rs` は premultiplied SourceOver を CPU の整数演算で転写する（浮動小数を持ち込まない・決定論）。`is_implemented` が偽の命令は飛ばす。
  - `fold.rs` の `normalize_element` は element定義を描画メソッドに関係なく `Overlay` で置く。
- pattern定義の語は `areka-seriko` から `ComposeMethod::from_name` を通る。
- 網羅台帳 `doc/ukadoc-coverage/ledger/assets.toml`: element定義の行は `degraded`、`ukadoc:descript_shell_surfaces:base:1` は `degraded`（element定義では描ける・pattern定義では未対応）。`blend-*` などの語の行もある。
- 描画は emo の自前の合成（CPU）で、Direct2D は表示の段（wintf）にある。

## Desired Outcome

- ukadoc の `descript_shell_surfaces` が element定義・pattern定義で使えると書く描画メソッドを、areka が描く。範囲は「Direct2D の合成・ブレンドが支える全部」へ広げる向きで、要件の段で ukadoc と D2D の対応表を作って決める。
- element定義では、読み手が描画メソッドの欄を `Element` に運び、合成の段がその語で描く。`is_image_element_method` を広げた分だけ `parse_undrawn_elements` の警告の対象が自動で減る（「3 つの転記が element定義の行を漏れなく重なりなく分ける」テストを確かめ直す）。
- pattern定義の `base`（下の絵を使わず、指した絵から合成を始める）を描く。
- 描けない語が残るなら、記録を出す今の作りを保つ。
- 決定論のテストで語ごとの画素を固定する。合成の答えを CPU と D2D のどちらで出すかで、決定論の取り方が変わる（下の Approach）。

## Approach

要件・設計の段で次の分かれ目を決める（ここでは決めない）。

- **合成をどこで行うか**: 今の CPU の整数の合成を語ごとに広げるか、合成を Direct2D（wintf の D2D の口・ブレンドの効果）へ移すか、あるいは両方を持つか。開発者の方針は「D2D が支える全部に対応できるよう wintf も含めて広げる」で、wintf を触ることは範囲外ではない。
- **決定論**: 今の檻は CPU の整数の転写で画素を固定している。D2D へ移すなら、画素の固定の仕方（読み戻し・許容差・GPU とソフトウェアの描き手の違い）を決める。
- **語の対応表**: ukadoc の語（`blend-*` の新書式・旧書式の別名・`-fast` 付き）と D2D のブレンドの種類の対応。D2D に無い語の扱い。

## Scope

- **In**: element定義・pattern定義の残りの描画メソッド全部（pattern定義の `base` を含む）・読み手の `Element` の描画メソッドの欄・合成の段・必要なら wintf の D2D の口・台帳の該当の行・`doc/COMPAT_ARCHITECTURE.md`・決定論のテスト・実機の確認。
- **Out**: element定義のオプション（クリップなど・`element-clipping-option`）・外形の求め方（`extent-element-offset`）・動く画像の再生（`animated-image-playback`）・α の宣言（`self-alpha-declaration`）。

## Boundary Candidates

- 読み手（`shell::{model,decode}`・`undrawn.rs` の連動）
- 合成の段（`areka-emo-compose` の `method.rs`・`plan.rs`・`blit.rs`・`fold.rs`）
- D2D の合成・ブレンドの口（wintf）——合成を移す案を取るときだけ
- pattern定義の経路（`areka-seriko` から合成の段への受け渡し）

## Out of Boundary

- 描画メソッドの語の綴りの正規化（`canonical_method_name`）と登録簿（`known_method`）の写しは、広げる語の分だけ触る。登録簿の作り直しはしない。
- 表示の段（wintf の窓・スワップチェーン）の作り。

## Upstream / Downstream

- **Upstream**: 完了 `element-base-method`（語の集合の 1 関数・描けない行の警告）・`emo-compose`・`seriko-runtime`・`surface-element-nesting`。
- **Downstream**: 描画メソッドを使うシェル全般。合成を D2D へ移すなら、合成の段の決定論のテスト（golden）全部。

## Existing Spec Touchpoints

- **Extends**: なし（完了 `element-base-method` の継ぎ目 2 つ＝`is_image_element_method` と `parse_undrawn_elements` を使う）。
- **Adjacent**: `element-clipping-option`・`extent-element-offset`・`collisionex-regions`（同じ `shell/model.rs`・`shell/decode.rs`）・`animated-image-playback`（同じ `plan.rs`）——同時に走らせない。

## Constraints

- 段は**優先**（シェルの element の列・アニメーション）。
- 規模は大きい見込み（語の数と、合成を D2D へ移すかどうかで変わる）。要件の段で測り、20 タスクを大きく超えるなら分け方を議題にする。
- 1 フレーム遅らせる解・時刻を丸める解は取らない（設計の大原則）。

## `areka-P0-animated-image-playback` からの申し送り（2026-10-07・完了時の棚卸）

- 手書きの `always` の animation のコマ（pattern定義）の描画メソッドが `overlay` 以外（`overlayfast`・`move` など）だと描かれない。`always` の再生は `animated-image-playback` で動くようになったので、この語の穴が目に見えるようになった。
- あわせて、`crates/areka-emo-compose/src/plan.rs` の `flatten_surface` の「非 Overlay method の現在コマ: 不描画 skip」の `warn!` が合成のたびに出る。コマが 4 枚以上だと合成の覚えに当たらず、`always` の周ごとに出続ける（記録の洪水）。経路は `animated-image-playback` の前から。
- 描画メソッドを描く側は本 spec の範囲。描けない語が残るとき、`warn!` を状態が変わったときの 1 回にするかは本 spec の要件で決める。


## 2026-10-10 棚卸㉓の分割

- **分けた理由**: 上の再測定（初めての実測）で、CPU の整数の合成のまま広げる道で 20〜24 タスク、合成を Direct2D へ移す道で 30 を超え、どちらも上限の 20 を超えた（この spec の初めての分割）。2 本に分け、この spec は土台の 1 本目として名前を残す。
- **残す範囲（In）**:
  - 読み手が描画メソッドを運ぶ。element定義の値 `Element`（`crates/areka-parsers/src/shell/model.rs`）に描画メソッドの欄を足し、画像の element定義にする語を決める 1 関数 `is_image_element_method`（同じフォルダの `decode.rs`）と、描けない行の転記 `parse_undrawn_elements`（`undrawn.rs`）を連動させる。合成の側は、element定義を置く `normalize_element`（`crates/areka-emo-compose/src/fold.rs`・今は語に関係なく `Overlay` で置く）が運ばれた語を使う。
  - 「描ける語か」を見る 7 か所の門を、語ごとの答えに替える。場所は `crates/areka-emo-compose/src/` の `plan.rs` に 2（今のコマ・着せ替えの pattern0）、`nesting.rs` に 3（着せ替えの先・`always` の経過 0 の先・コマの先）、`plan_always.rs` に 1（`always` の経過 0 の絵）、`blit.rs` に 1（命令の転写）。どれも今は「`overlay` だけ真」の 1 関数（`method.rs` の `ComposeMethod::is_implemented` と `is_implemented_name`）を通る。
  - pattern定義の `base`。
  - 重ね方の基本の 5 語: `overlay-fast`（旧い綴り `overlayfast`）・`interpolate`・`replace`・`asis`・`reduce`。
  - 合成のたびに出る警告の洪水を止める（上の「`animated-image-playback` からの申し送り」）。
  - 「CPU の整数の道か、Direct2D の道か」の裁定を、この spec の要件の議題 1 として受ける。
  - 決定論のテスト（語ごとの画素）・網羅台帳の該当の行・`doc/COMPAT_ARCHITECTURE.md` §8・実機の確かめ。
- **出した範囲（Out・どの spec へ）**:
  - `blend-*` の全部と、その `-fast` 付き・旧い別名 2 つ（`overlaymultiply`・`overlayscreen`）→ `draw-methods-blend`（新しく起票）。上の再測定の「見つけた穴」の `BlendKind` の 9 種の抜けも、そちらが持つ。
  - wintf だけの spec は**今は起票しない**。この spec の要件で Direct2D の道が選ばれたら、そのとき wintf だけの spec を起票する（上の再測定の「分割の案」に書いた名前は例で、今は作っていない）。
  - `move`（位置の語）は議題のまま。`auto` は `element-clipping-option` の後の別件（上の再測定のとおり）。
- **順番**: この spec が先、`draw-methods-blend` が後。列の中の位置は今のまま（`element-clipping-option` の後）。
- **残した側の規模**: 13〜16 タスク（CPU の整数の道のとき）。Direct2D の道が選ばれたら、wintf の spec を出したうえで測り直す。
- **残した側が触るファイル**:
  - `crates/areka-parsers/src/shell/{model.rs, decode.rs, undrawn.rs}` と兄弟のテスト
  - `crates/areka-emo-compose/src/{method,fold,normalized,plan,plan_always,nesting,blit}.rs` と兄弟のテスト、画素の式の新しいファイル（`blit.rs` は 782 行なので式は足さない）
  - `Element` を直書きしているファイル。今の木で数え直すと 6 クレートの 39 ファイル（`areka-emo-compose` 14・`areka-parsers` 11・`areka-emo-atlas` 5・`areka-emo-present` 5・`areka-emo-text` 3・`areka` 1。`element-base-method` の実測の「約 33」から増えた）。足すのは欄 1 つぶんの機械的な直しだけ。
  - `crates/areka-seriko/src/table.rs` のテストの期待 1 か所（pattern定義の `base` のコマが「描けない」と確かめている行。上の再測定の一覧に無かった）
  - `doc/ukadoc-coverage/ledger/assets.toml` の約 10 行（element定義の行・`base`・`overlay-fast`・`overlayfast`・`interpolate`・`replace`・`asis`・`reduce`）・`doc/COMPAT_ARCHITECTURE.md` §8
- **同じウェーブで触らない約束**（破るなら止めて報告）:
  - wintf（`crates/wintf/`）に触らない。Direct2D の道が選ばれたら、この spec では触らずに wintf だけの spec を起票する。
  - `blend-*` の画素の式を足さない。網羅台帳の `blend-*` の 55 行と旧い別名 2 行に触らない。
  - 台本の側（`crates/areka-parsers/src/sakura/`・`crates/areka-sakura/`）に触らない。
  - seriko は、上のテストの期待 1 か所のほかに触らない（pattern定義の語は今も合成の `ComposeMethod::from_name` を通るので、seriko の本体は変えずに済む見込み）。
  - 外形のファイル `plan_extent.rs` の本体に触らない（兄弟のテスト `plan_extent_film_tests.rs` は `Element` を直書きしているので、欄 1 つぶんだけ直す）。
- **`draw-methods-blend` が読み手に触らずに済むための決め**: 読み手は、`blend-*` の語も含めて、描画メソッドの語を落とさずに運ぶ形にする。「描けるか」は合成の 1 関数が答え、描けない行の警告もその答えで出す。読み手の `is_image_element_method` を基本の 5 語だけに広げる形にすると、`draw-methods-blend` が読み手を触り直すことになり、分けた意味が薄れる。
- **同じウェーブに置けない相手**: 読み手の `shell/{model,decode}.rs` を分け合う `element-clipping-option`・`collisionex-regions`・`animated-image-import`・`seriko-trigger-intervals`・`seriko-script-triggers`。合成の `plan.rs`・`nesting.rs`・`plan_always.rs` を分け合う `seriko-interval-combinations`・`animated-image-import`。`plan_extent_film_tests.rs` を分け合う `extent-element-offset`。
- **この分割で引き直した正典**（ukadoc の文書 MCP・descript_shell_surfaces。文は要約）:
  - `base`（https://ssp.shillest.net/ukadoc/manual/descript_shell_surfaces.html#base ）: ベースサーフェスを新しいレイヤで丸ごと置き換える。当たり判定も、コマのサーフェスに定義されたものへ替わる。pattern定義では X,Y を無視する。着せ替えと element定義では最初（element0・pattern0）にしか使えず、それ以外は `overlay` に読み替える。
  - `overlay-fast`（SSP 2.8.36。`overlayfast` はその旧い綴り）: ベースの不透明度に応じて重ねる。`interpolate`: ベースの透明度に応じて重ねる（`overlay-fast` の対）。`replace`: 新しいレイヤの範囲の中を、透明も含めて上書きする（範囲の外は何もしない）。`asis`: 新しいレイヤの透過を無視して重ねる。`reduce`: 不透明度を掛け合わせる（色は無視・新しいレイヤの範囲の外は透明として扱う）。
- **議題**（答えで作業が変わるものだけ）:
  1. CPU の整数の道か、Direct2D の道か（開発者の方針は「Direct2D が支える全部に対応できるよう wintf も含めて広げる」）。答えで、この spec の規模・wintf の spec を出すかどうか・`draw-methods-blend` の式の持ち方が変わる。
  2. `move`（位置の語）を範囲に入れるか。
  3. pattern定義の `base` で当たり判定もコマのサーフェスのものへ替えるか（正典はそう書く）。入れるなら当たり判定の経路（`crates/areka-emo-compose/src/hit.rs`）まで及び、+1〜2 タスク。
  4. 描けない語が残るときの警告を、状態が変わったときの 1 回にするか。
- **見つけた穴**: `asis` の正典は「element定義で合成したサーフェスを、ほかのサーフェスのアニメーションの部品として `asis` で合成したときの表示は未定義」と書く。areka の決めを §8 に記す。

## 2026-10-15 `/kiro-discovery` の決め（D2D の道）

- **議題 1 は Direct2D の道に決まった**（開発者 2026-10-15「D2D合成アルゴリズムの改善も入れる予定なのです。wintfではコマンドリストの合成ルールが現在固定ですが、これをD2D合成ルールを個別に指定できるようにして、高度な合成を実現したい」）。上の分割の節の予告どおり、wintf だけの spec `wintf-d2d-composite-rules` を起票した。**この spec はその後に着手する**（前提に足す）。「同じウェーブで触らない約束」の「wintf に触らない」はそのまま効く（wintf の分は向こうが持つ）。
- **この spec が持つもの（D2D の道のとき）**: 読み手が描画メソッドを運ぶこと（分割の節のまま）／ukadoc の語 → wintf の合成の規則の対応（起票時の見立て: `overlay`＝`SOURCE_OVER`・`overlay-fast`＝`SOURCE_ATOP`・`interpolate`＝`DESTINATION_OVER`・`replace`＝`BOUNDED_SOURCE_COPY`・`reduce`＝`DESTINATION_IN`・`base`＝土台を消して描く・`asis`＝α を無視して描く。表の全体は `wintf-d2d-composite-rules` の brief）／emo の合成の段を、CPU の整数の転写から「element ごとに規則つきで描く D2D の命令の列」へ移すこと。
- **決定論のテスト**: 合成の答え（画素）のテストは D2D を CPU の描き手（WARP）で動かして固定する（開発者 2026-10-15「画像合成に関するテストは、CPUに切り替えるべき」）。今の CPU の整数の合成の檻（`golden_tests_*.rs`）の扱い（見本として残すか、WARP の見本へ移すか）は要件で決める。
- **CPU の画素に頼っている読み手の出どころが変わる**: 当たり判定の α マスク（`presenter/budget.rs`・`cache.rs`）・MCP の `dump_surface`（`presenter/read.rs`）・バルーンの背景色（`emo2_boot/balloon_background.rs`）。α は `wintf-d2d-composite-rules` の 1 ピクセルの読み戻し（開発者 2026-10-15「座標さえわかれば、１ピクセルだけGPUから書き戻すようなことはできないか？」）、`dump_surface` と背景色は全体の読み戻しへ替える見込み。どちらが受け持つかは両 spec の要件で決める。
- **規模**: D2D の道で測り直す（着手のとき）。wintf の分は外へ出したので、分割の節の「30 を超える」より小さくなる見込み。一度切り出した spec なので、上限 20 を少しまたぐ程度ではさらに削らない。

## 2026-10-23 `move` の持ち主の移し替え（`areka-P0-dot-sakura-specimen` のタスク 5.1）

- 上の議題「`move`（位置の語）を範囲に入れるか」は、開発者と決めて新しい spec `seriko-pattern-calls` へ移した（アニメーションを呼ぶ 7 語と一緒に、pattern の働きとして SERIKO の再生が受ける）。本 spec は重ね方の合成メソッドだけを持つ＝この議題はもう出さない。
- 根拠: どっとさくらのシェル `master` の `animation0.pattern1,move,0,10,0,6` などが `未知の合成メソッド名` の WARN 22 行になった（`.kiro/specs/completed/areka-P0-dot-sakura-specimen/verification.md` の 8 節の件 4）。

## 2026-10-24 「．さくら」の pattern定義の `base`（`shiori2-protocol` のタスク 5.4）

- `shiori2-protocol` の実機の 1 回（5.1・2026-10-24・記録 `target\s2p-5-1\logs\run2.stdout.log`）で、シェルを読むときに `shell: areka が描けない描画メソッドの pattern を描かない … method="base"` の WARN が 6 件（`surface_id=12 pattern=0`・`surface_id=22 pattern=0`・`surface_id=22 pattern=3` の 3 種が、シェルを 2 回読んだので 2 回ずつ）出た。前回の実機の 1 回（`areka-P0-dot-sakura-specimen`）の記録には無かった行。
- 元は「．さくら」のシェル `master` の `surfaces.txt`: `surface12` の `animation0.pattern0,base,11` と、`surface22` の `animation0.pattern0,base,0`・`pattern3,base,2`。どちらも `interval,runonce` で、`base` の後に `move` が続く（`move` は `seriko-pattern-calls` の件 2）。
- `surface12.png`・`surface22.png` は全部が透明な絵で、`base` で別の面を土台に描く前提の作り。前回は焼き込みの「全透明」の WARN を作者の誤りとして数から外したが、`base` の土台と読むのが正しい。今のままでは面 12・22 を出すと何も描かれない見込み（この 1 回では面 12・22 は出ていない）。
- 直した後の確かめ: 「．さくら」で面 12・22 を出し（`\s[12]`・`\s[22]`）、`base` の面が描かれ WARN が 0 になること。`move` が付くので `seriko-pattern-calls` と並べて見る。

## 2026-11-04 棚卸㉞の再測定（main `932406f9`）

- 前提の変化（棚卸㉔の後）: D2D の道に決まり（10-15）、先に `wintf-visual-tree`（10-21）が入った＝この spec は emo の合成を描画の木の節（element ごとに重ね方つき）へ移す側。合成の段は着地が続いて変わった＝`seriko-pattern-calls`（`move` を受けた・`bind_steps.rs`・`pattern_word.rs`）・`areka-P0-offline-surface-dump`（窓を出さずに面を PNG に書く `crates/areka/src/offline_dump/` が CPU の合成 `Composer` を直に使う・`plan_bounds.rs`・`plan_issues.rs`）・`animated-surface-log-flood`（変わった矩形だけ重ね直す `blit_rect.rs`）。D2D へ移すとき、この重ね直しの効きを失わないこと（`wintf-visual-damage-redraw` を先か同じ頃に）。
- 「描ける語か」の門は 7 か所から 8 か所（7 ファイル）になった＝`plan.rs` 1・`nesting.rs` 2・`plan_always.rs` 1・`blit.rs` 1・`bind_steps.rs` 1・`plan_bounds.rs` 1・`plan_issues.rs` 1（どれも `method.rs` の `is_implemented`／`is_implemented_name`）。`Element` を直書きするファイルは 39 から 44（`areka-emo-compose` 17・`areka-parsers` 11・`areka-emo-atlas` 6・`areka-emo-present` 5・`areka-emo-text` 3・`areka` 2＝`emo2_boot/balloon_background_tests.rs`・`placement/measure_first_frame_tests.rs`）。
- CPU の画素に頼る読み手は 5 つ＝当たりの α マスク（`crates/areka-emo-present/src/presenter/budget.rs`・`cache.rs`）・MCP の `dump_surface`（`presenter/read.rs`）・バルーンの背景色（`crates/areka/src/emo2_boot/balloon_background.rs`）・窓を出さない書き出し（`crates/areka/src/offline_dump/render.rs`・10-15 の節より後に増えた）。
- 触るファイル: 分割の節のものに、上の門の 3 ファイル・emo の表示（`crates/areka-emo-present/src/display.rs` 238 行・`mount.rs` 317 行）・上の 5 つの読み手・合成の見本 `crates/areka-emo-compose/src/golden_tests*.rs`（WARP の見本へ移すなら）。`crates/areka-seriko/src/table.rs` は 929 行（期待 1 か所だけ・858〜862 行目）。wintf には触らない約束のまま。
- 規模: D2D の道で初めて見積もると 20〜25 タスク（読み手の欄・語の対応・合成を節の木へ・表示の付け替え・門 8 か所・pattern定義の `base`・5 つの読み手・見本の移し替え・警告の洪水・台帳・実機）。
- 先に要るもの: `wintf-d2d-composite-rules`（とその前の `wintf-visual-tree`）。列の中の順は「`element-clipping-option` の後」から「前」へ（D2D の道では `--clipping` を節の切り抜きで描くので、節の木へ移すこの spec が先）。見直しは台帳の `audit-emo-compose`・`audit-seriko`・`audit-parsers-sakura`・`audit-emo-present` に、`audit-emo-atlas`・`audit-emo-text-state-actor`（`Element` の直書き）・`audit-emo2-boot-spine`（`balloon_background*`）・`audit-placement-core`（`measure_first_frame_tests.rs`）・`audit-areka-app`（`offline_dump/`）を足す。本体の crate 分けは `placement-crate-split`（placement を移す）・`runtime-module-untangle`・`runtime-crate-split`（`emo2_boot` を移す）・`session-crate-split`（`offline_dump` の参照を書き換える）の後＝6 本の鎖の終わりの後。同じウェーブに置けない相手＝`collisionex-regions`・`hit-id-pass`・`compose-f16-precision`・`element-clipping-option`・`draw-methods-blend`（v0.2.0 の中）。
- 優先度の区分: A のまま（開発者 10-05「残りの描画メソッドは起票」「Direct2D が支える全部に対応できるよう wintf も含めて広げる」）。
- 要件定義のモデル: Fable（骨組みを決める分かれ目を先例が決めていない＝CPU の合成を残して 5 つの読み手の出どころにするか、全部を GPU の読み戻しへ替えるか）。
- 分割の案: 一度切り出した spec なので、20 を少し越える程度では分けない。要件で 22 を越えたら、5 つの読み手の付け替え（`presenter/{budget,read}.rs`・`cache.rs`・`balloon_background.rs`・`offline_dump/render.rs`）を後の 1 本（名前の例 `emo-pixel-readback`）へ出す。後ろの 1 本が本体（`crates/areka`）の分をほぼ全部持つので、この spec の待ちが軽くなる。
- 見つけた穴・古くなった記述: `element-clipping-option` も `Element` に欄を足す＝同じ 44 ファイルをもう一度触る。この spec が描画メソッドの欄と一緒にオプションの並びの欄（転記だけ）を足せば、あちらは触り直さずに済む。

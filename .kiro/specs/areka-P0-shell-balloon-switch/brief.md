# Brief: areka-P0-shell-balloon-switch

> 2026-09-20 `/kiro-discovery` 再入（棚卸⑮）で起票。`areka-P0-ghost-shell-balloon-switch`（台帳 #13・規模 L）の Approach ②③「バルーン切替・シェル切替」を**単独の spec として切り出した**（台帳 #50）。名前は `doc/ukadoc-coverage/roadmap-draft.md` 段階 B 順位 2 の束「切替」の候補名そのものである。
> 正典の語彙と Ref の一覧は親 brief（`.kiro/specs/areka-P0-ghost-shell-balloon-switch/brief.md` の Desired Outcome 2・3）が正本。本 brief は**再測定で崩れた前提と、切り出したあとの境界**だけを書く。
> 本文の file:line は**起票時の実測値**（2026-09-20・main `fe157df1`）。着手時に必ず引き直すこと。

## Problem

**誰の何が困っているか**: 別のシェルを持つゴーストの利用者と、バルーンを入れ替えたい利用者。α の利用者の一周は「右クリックメニューでゴースト／シェル／バルーンを替える」を含む。

`\![change,shell,名]` と `\![change,balloon,名]` は解析は通るが消費者が居らず、**黙って何も起きない**。`OnShellChanging`／`OnShellChanged`／`OnBalloonChange` は製品コードに 0 件（`git grep -l "OnShellChang\|OnBalloonChange" -- 'crates/*.rs'` は `crates/ukadoc-survey/src/diff_tests.rs` の 1 ファイルだけ）。

## Current State——親 brief の前提が 2 つ崩れている

親 brief は「寿命を先に分離しておけば、バルーンとシェルの切替は資産を作り直すだけの仕事になる」「バルーンが最軽量」と書いた。**2026-09-20 の実測はどちらも支持しない。**

1. **走っているゴーストの中で、絵と文字の出し先を差し替える語彙が 1 つも無い。**
   - 出し先（sink）は起動時に `GhostBootOptions.sinks` として**値で渡され**、配る役（dispatcher）に固定される（`crates/areka-ghost/src/runtime.rs` の sink 配線）。
   - `DispatcherMsg` に差し替えの語は無い（`crates/areka-ghost/src/dispatcher.rs` の `DispatcherMsg` の定義）。
   - seriko・present・text・dispatcher の 4 か所を `reload|ReplaceTarget|Rebuild|swap_sink|replace_sinks` で引くと **0 件**（同じ 4 か所は `pub enum SerikoMsg|PresentCommand|TextMsg|DispatcherMsg` で 4 件当たる＝検索は効いている）。
   - 組み立ての本体 `wire_emo2_boot`（`crates/areka/src/emo2_boot/mod.rs`）は**一発の構築**である。シェルとバルーンの資産を束ねて作り、seriko を起こし、`boot_with_kanade_stop` を呼び、`add_systems` も 1 度きり。
   - つまり SHIORI を生かしたままシェルやバルーンだけを替えるには、**各アクターへの読み直しの語彙か、出し先の付け替えが新たに要る**。これが本仕様の主題である。
2. **シェル切替は「マウントの片側を差し替える」では済まない。** シェルを決める `resolve()`（`crates/areka-parsers/src/package/resolve.rs`）は `seriko.defaultsurfacedirectoryname`、無ければ `master` しか見ず、**4 か所から独立に呼ばれている**: `crates/areka-ghost/src/runtime.rs`・`crates/areka/src/emo2_boot/assets.rs`・`crates/areka/src/placement/source.rs`・`crates/areka/src/placement/persist.rs`。選んだシェル名を 4 か所すべてへ運ぶ必要がある。`GhostBootOptions` の構造体リテラルは 27 か所・16 ファイルに在る（`git grep -c "GhostBootOptions {"`）ので、欄を足す形は波及が大きい。

## Desired Outcome

完了時に次が真になっている（語彙と Ref の正本は親 brief）。

1. `\![change,shell,名]` で同じゴーストの別シェルへ替わる。SHIORI は降ろさない。`--option=raise-event` のときだけ `OnShellChanging`、替わったあと `OnShellChanged`。
2. `\![change,balloon,名]` でバルーンが替わり `OnBalloonChange` が届く。
3. 右クリックメニューの「シェル」「バルーン」の枠に、列挙された候補が並び、選ぶと同じ経路で替わる。
4. 選択は `baseware-root-layout` の「最後に使った」鍵へ書かれ、**次回起動でシェルも復元される**（起動時にシェル名を効かせる口は本仕様が作る——`baseware-root-layout` は鍵の定義と書き込みまで）。
5. 該当が無ければ `warn!` を出して無視する。切替の途中で失敗したら、`error!` を出して**元のシェル／バルーンのまま表示が続く**（ログ無し失敗経路の禁止）。

## Approach（要件で選ぶ・いまは方向だけ）

| 案 | 中身 | 見立て |
|---|---|---|
| A. 作り直しの一般形に乗る | ゴースト切替（親 spec）が作る「降ろして起こし直す」仕組みを、SHIORI だけ残して回す | 親 spec が先に着地していれば、新しい語彙が最少。代わりに SHIORI アクターを跨いで残す寿命の扱いが要る |
| B. 各アクターに読み直しの語を足す | seriko・present・text に「資産を差し替えろ」の語を 1 つずつ足す | 影響が局所だが、4 アクター分の語彙と、差し替えの途中の 1 フレームに古い絵と新しい当たり判定が混ざらない保証が要る（記憶 no-frame-delay-fixes-change-the-state-shape） |

**どちらが安いかはコードを書いてみないと分からない類の問い**なので、要件の前に**先進坑（`crates/pilot/examples/`）を 1 本掘る**ことを推す（`.kiro/steering/two-tunnel.md`）。掘る対象は「SHIORI を生かしたまま present のバルーン資産だけを差し替えて、1 フレームも崩れずに表示が続くか」の 1 点。

## Scope

- **In**: `\![change,shell|balloon,名(,--option=raise-event)]` の消費者／`OnShellChanging`／`OnShellChanged`／`OnBalloonChange` の送出と Ref／シェル名を `resolve()` の 4 呼び出し点へ運ぶ口／起動時の「最後のシェル」の適用／メニューの「シェル」「バルーン」枠への登記／2 つ目のシェルを持つ検体（`R_POST_and_KOMAINU` の `shell/master` を写して名前を変えたもの）。
- **Out**: ゴースト切替と切替要求の型 `SwitchRequest`・名前解決（親 spec）／寿命の分離（`areka-P0-app-lifetime-separation`）／`\![reload,shell|balloon]`／`\![bind,…]` 着せ替え／拡大率（`\![set,scaling,…]`）／`currentghost.shelllist.*`・`balloonlist.*` のプロパティ。

## Boundary Candidates

- 資産の差し替え（emo 側: `emo2_boot/assets.rs`・`frame/attach.rs`・`placement/spawn.rs`・各アクターの語彙）
- シェル名の運搬（`resolve.rs` と 4 呼び出し点）
- 切替の通知（kanade の 5 ファイル: `msg.rs`・`actor.rs`・`schedule/{mod,events,steady}.rs`）

## Out of Boundary

- 列挙と記憶の実体（`baseware-root-layout`）
- 切替要求の入口の型と、台本とメニューが同じ 1 本を通る保証（親 spec が作り、本仕様は種別を 2 つ足すだけ）

## Upstream / Downstream

- **Upstream**: `areka-P0-baseware-root-layout`（`list_shells`・`last.shell`・`last.balloon`）／`areka-P0-ghost-shell-balloon-switch`（`SwitchRequest`・切替の通知の入口・作り直しの一般形）／`areka-P0-default-balloon-nar-fold`（バルーン 2 つ目の検体は既定バルーンで足りる）。
- **Downstream**: `areka-P0-ghost-install`（シェル・バルーンの `.nar` を入れた直後の切替）・`areka-P0-network-update`・`areka-P0-alpha-release-signoff`・α 後の `areka-P0-property-catalog-lists`。

## Existing Spec Touchpoints

- **Extends**: なし。
- **Adjacent**: 親 spec・`ghost-install`・`network-update` と、`crates/areka/src/emo2_boot/consumer_ledger.rs`・`emo2_boot/mod.rs`・kanade の 5 ファイル・網羅台帳（`doc/ukadoc-coverage/ledger/*.toml` と生成物）を**全員が触る**＝必ず直列（親 → 本仕様 → `ghost-install` → `network-update`）。`crates/areka-kanade/src/schedule/steady.rs` は 935 行で上限が近い。`crates/sample-ghost-kit/src/{lib.rs,lib_tests.rs}` は検体を足すなら直書きの数の数え直しが要る。

## Constraints

- 規模 **M**（タスク 12〜15 本）。**要件と設計は Fable**（主題が「今は存在しない仕組みの設計」で、答えがコードからは出ない）。
- 差し替えの途中の表示が 1 フレームも崩れないこと（古い絵と新しい当たり判定の混在・空の窓の点滅）。1 フレーム遅らせて辻褄を合わせる解は取らない。
- 決定論テスト網羅は必達。資産は偽のシェル 2 つ・偽のバルーン 2 つで往復し、イベントの Ref を突き合わせる。実機は `R_POST_and_KOMAINU` の 2 シェル往復と、既定バルーン ⇄ `emo2` 同梱バルーンの往復を 1 周。
- 1 ファイル 1,000 行。

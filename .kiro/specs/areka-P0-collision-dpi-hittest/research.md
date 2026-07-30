# ギャップ分析: areka-P0-collision-dpi-hittest

> 実施日: 2026-07-31 ／ 対象: 確定済み `requirements.md`（R1〜R6）× 現行コードベース（`main` 相当の worktree ブランチ `claude/areka-p0-collision-dpi-hittest-*`・W4 `emo-dpi-scaling` 着地後）
> 手法: Grep/Glob/Read による実測（brief・W4 design の記述は**全て現物で再確認**し、陳腐化していた記述は本書で訂正した）。外部依存調査は不要（新規依存なしの spec）。
> 位置づけ: **情報提供であって決定ではない**。丸め規約・÷k の着地層・k の厳密性は本書で選択肢と根拠を並べるに留め、確定は要件ディスカッション／設計フェーズが行う。

---

## 1. 分析サマリ（5 点）

- **上流は「席」まで用意して待っている**が、**÷k 本体はコード上どこにも存在しない**。`EmoPresenter::applied_scale`（`presenter.rs:705`）は実適用 k を返し、doc `:704` が本 spec を名指しで招いている。一方 `EmoPresenter::hit_region`（`presenter.rs:867`）は無変換のまま純関数へ渡し、doc `:858-861` が「÷k は**呼び手**の責務・本メソッドの責務ではない」と明示している。
- **brief の Approach 1（`presenter.rs:452` で純関数呼出前に ÷k）は現物と矛盾する**。当該行番号は既にドリフト（実体は `:867`）であり、かつ W4 が同メソッドの doc で ÷k を**呼び手側**へ割り当てた。したがって「presenter 内で ÷k」を採るなら **W4 の doc 契約を書き替える**か、**別メソッドを新設する**かの明示的裁定が要る（brief をそのまま実装すると W4 の明文契約と衝突する）。
- **÷k のヘルパが 1 本も無い**。`ScaleRatio`（`areka-emo-compose/src/scale.rs:44`）は**乗算方向のみ**（`scale_len(u32)` `:166`／`scaled_extent` `:187`）を持ち、num/den のアクセサすら無い。当たり判定座標は `i64`（負値あり）であり `scale_len` の `u32` 契約では受けられない。**「÷k の丸め規約」は本 spec が新規に発明する権威**である。
- **k の厳密性が公開面で失われる**。presenter 内部は `applied: Option<ScaleRatio>`（`:108`・既約有理）で厳密だが、公開アクセサは `applied_scale() -> f32` 一本。同ファイル `:678-684` が「既約分母が 2 冪でない k（例 7/6）では f32 の積が権威と 1px 食い違う」と**実測付きで警告**しており、境界 1px がヒットに効く本 spec では f32 経路が R2.3（境界一貫）と R3.5（割り切れない縮約の期待値固定）を直接脅かす。W6.5 `scale-exact-rational` は `TextSlotView` 経路のみを射程にしており（同 brief Approach 1-3）、**`applied_scale` は射程外＝本 spec が自前で有理数面を要求するか f32 で妥協するかを決める必要がある**。
- **R4（実機サインオフ）の受け皿 `collision-probe.rs` は k=1.0 前提で固く縛られており、実質的な作り替えが要る**。`assert_eq!(scale, 1.0)`（`:558-561`）だけでなく、①窓 resize 先が `surface_size()`（native 原寸・`:477`/`:490-502`）である、②`read_back` anchor が collision 値を無変換で表示画素 index に使う（`:583-619`）、③DPI 追従フェーズ（`run_dpi_phase`）を持たない単独 example である、の 3 点が k≠1.0 では成立しない。**このうち①③は親エージェントの申し送りに含まれていなかった追加の実測所見**である。

---

## 2. 現状調査（Requirement-to-Asset マップ）

### 2.1 当たり判定経路の実体（4 層）

| 層 | 実体 | 座標空間の現契約 | 本 spec での扱い |
|---|---|---|---|
| 入力 | `input_events/mod.rs:310-311`／`:382-383`（`PointerState.client_point` → `i64`） | 窓 client 物理 px | 変更候補（DD-4） |
| 配線 | `MouseWiring::resolve_region`（`input_events/mod.rs:106-127`）・doc `:97`/`:104-105` が **DD-IE-10「座標は素通し＝DPI 変換なし・k=1.0 契約を継承」** を明文化 | 同上（無変換） | **R5.3 の改訂対象**・÷k 挿入点候補 |
| 結線 | `emo2_boot/hit_region.rs:69-73` `resolve_hit_region`（`shell_target(scope)` → `presenter.hit_region`）・doc `:55-56` が「k=1.0 契約によりサーフェス px と同一空間で照合」 | 同上（無変換） | **R5.3 の改訂対象**・÷k 挿入点の第一候補 |
| 読み口 | `presenter.rs:867` `EmoPresenter::hit_region` → `areka_emo_compose::hit_region`（`hit.rs:57`・`:72` が閉区間比較式） | **native サーフェス px** | 純関数は R6.1 により不改変 |

**空間の断裂点はここ 1 箇所**: `resolve_hit_region`（client 物理 px を受ける）→ `presenter.hit_region`（native サーフェス px を要求する）の呼出境界（`hit_region.rs:71`）。k≠1.0 で壊れるのはこの 1 行に集約されており、他に暗黙の座標変換は存在しない（grep で `hit_region(` の全呼出を確認済み——production 呼出は `hit_region.rs:71` と `input_events/mod.rs:116` の 2 段のみ、他は probe とテスト）。

### 2.2 k の供給面（W4 が置いた席）

| 資産 | 場所 | 型 | 備考 |
|---|---|---|---|
| 実適用 k の単一真実源 | `presenter.rs:108` `PresentTarget.applied` | `Option<ScaleRatio>`（**厳密**） | 更新点は表示成立点 1 箇所のみ（失敗経路は前値保持） |
| 公開照会 | `presenter.rs:705` `applied_scale(target)` | `Option<f32>`（**非厳密**） | doc `:704` が本 spec を名指し |
| 別経路の同値 | `presenter.rs:651` `text_slot_view().scale()`（`:665` で `as_f32`） | `f32` | text 層向けスナップショット |
| 物理寸の権威 | `presenter.rs:687` `target_physical_size(target)` | `Option<(u32,u32)>` | 丸め権威 `scaled_extent` 経由。**probe の窓 resize はこちらを使うべき** |
| k 更新の駆動 | `presenter.rs:754` `refresh_scale` ／ `:841` `take_pending_resize`・`emo2_boot/frame.rs` dpi 相 | — | R1.7（k 更新後は新 k で判定）は**既存機構で自動的に満たされる**（`applied` を毎回読むかぎり） |
| k 導出（政策） | `areka-emo-present/src/scale.rs:187` `derive_scale` | `ScaleRatio` | R1.4「DPI から独立に再導出しない」は「`derive_scale` を本 spec で呼ばない」ことと同義 |

**R1.4 の観点で重要**: `applied` は「実際に適用された k」であり、`derive_scale` の再呼出（窓 DPI からの再導出）とは**失敗経路で乖離し得る**（`refresh_scale` `:796-814` は再表示が成立しなければ `applied` を更新せず前 k を維持する）。ゆえに R1.4 は「`applied_scale`（もしくはその厳密版）だけを読む」で厳密に満たされ、`derive_scale` を呼ぶ実装は要件違反になる。

### 2.3 ÷k に使える算術（存在しないもの）

- `ScaleRatio`（`areka-emo-compose/src/scale.rs:44`）: `num`/`den` は**私有**・アクセサ無し。公開面は `new`/`mul`/`is_identity`/`as_f32`/`scale_len(u32)`/`scaled_extent`。**除算方向・符号付き入力の API は皆無**。
- 逆写像の**実在する前例が 1 つある**: `resample` の `AxisWalk`（`scale.rs:218-281`）。出力画素 d → 入力座標 `src = (d + 1/2)·den/num − 1/2` を整数（分子・剰余）で厳密に前進させる。**これは「表示画素 d に何が描かれているか」の正準な逆写像であり、÷k の丸め規約の最有力候補の根拠**（後述 DD-1 の候補 B）。
- f32 での ÷k の前例: `areka-emo-text/src/region.rs:94-96` `ScaleContract::to_image(v) = v / scale`。ただしこれは**連続量（サブピクセルオフセット）用**であり、同ファイル `:98-` の `physical_extent` は f32 起因の 1px 欠陥が**未是正のまま登記されている**（W6.5 `scale-exact-rational` の題材）。**離散画素の判定に f32 除算を持ち込むのは、いま別 spec が是正しようとしている病の再生産**である。

### 2.4 テストの檻（R3 の受け皿）

- **GPU 不要の前例あり**: `presenter.rs:2225-2246`（`attach_target` は World に触れないため素の `World` で `hit_region`＝`None` を固定）。`emo2_boot/hit_region.rs:84-99`（未表示 scope の縮退）。
- **k を注入した表示は GPU が要る**: `applied` が確定するのは表示成立点のみ。表示は `make_world_with_gpu()`（`presenter.rs:2252` 他）を要する。**したがって R3.1「GPU 不要・実窓不要で任意 k を注入」は、`EmoPresenter` を通した経路では原理的に達成できない**——÷k は**純関数として切り出され、k を引数で受け取る**形でなければ R3 を満たせない（これは設計の自由度ではなく要件からの**制約**である）。
- 純関数コアの檻の前例: `areka-emo-compose/src/hit.rs:103-211`（閉区間・画家則・反転矩形・決定性）。R3.3 の分岐名（領域内／別領域／背景／境界内側 1px／境界外側 1px）は `hit.rs:130-141` の既存檻と 1:1 対応させられる。

### 2.5 R4 実機経路（`collision-probe.rs`）の現状

| 行 | 現状 | k≠1.0 での問題 |
|---|---|---|
| `:448` | `attach_target(.., 96)`（author_dpi ハードコード） | **k≠1.0 の妨げにはならない**（作者基準 96・実モニタ 120/192 なら k=5/4・2 が出る）。ただし隣接コメント `:446-447`「本 example は k=1.0 相当」は**陳腐化**＝要改訂 |
| `:469-484` | `text_slot_view().surface_size()`（**native 原寸**）を「現表示実寸」として採用 | k≠1.0 では表示物理寸と乖離。`target_physical_size()`（`presenter.rs:687`）へ差し替えが要る |
| `:487`・`:583-619` | `read_back` の画素 index に collision 値（サーフェス px）を無変換で使用 | `read_back` は **k 適用後**の供給面。中心点を `scale_len` で k 倍しないと外れる（`:604` の範囲外 assert で loud に落ちる） |
| `:490-502` | `resize_window_to(surface_size)` | 窓 client を native 寸へ縮めると供給面（k×）と食い違う |
| `:551-561` | `GetClientRect == surface_size` ∧ `scale == 1.0` の hard assert | **R4.1 と正面衝突**。`GetClientRect == target_physical_size` ∧ `scale != 1.0` ∧ **2 水準で異なる物理寸**へ置換が要る |
| （構造） | 単独 example。`run_dpi_phase`／`refresh_scale`／`take_pending_resize` を**持たない** | 初回 `ShowSurface` 時点の `DPI` component が実モニタ値でなければ k=1.0 のまま固まる（`apply_show` `:362-363` は show 時点の DPI を読む）。**要実測**（後述 Research Needed R-1） |
| `:681` | `resolve_hit_region(&boot.presenter, 0, s2c_x, s2c_y)` | ÷k 実装後は自動的に新経路を通る（probe は resolver を私有 include するため）。**反トートロジー条件（R4.3/4.4）は現状のまま維持される**（`SetCursorPos`/`SendInput` 不使用・狙点は `GetCursorPos` 由来） |

**`#[path]` include の構造的制約（`hit_region.rs:11-22`）**: probe は `hit_region.rs` を私有 include するため、**同ファイルの非テストコードは `crate::` パスを一切使えない**（`super::target_map` と外部 crate のみ）。÷k ヘルパを `areka` bin の**新規モジュール**に置くと、probe 側にも `#[path]` include の追加が要る（成立はするが結線が増える）。**外部 crate（`areka-emo-compose` / `areka-emo-present`）に置けばこの制約に触れない。**

### 2.6 R5（文書改訂）の対象集合（実測）

| 対象 | 場所 | 現記述 |
|---|---|---|
| `collision-geometry` design の k=1.0 限定契約 | `.kiro/specs/completed/areka-P0-collision-geometry/design.md:50`（表 `:44-48` の C9 行・`:40`） | 「本 spec は k≠1.0 を実装しない」 |
| 同 Revalidation Trigger 2 | 同 `design.md:86` | 「k=1.0 契約の解除 → 4.3 の照合空間が破れ、点のスケール除算が要る。7.3 probe 再実行が必須」 |
| resolver の座標契約 doc | `crates/areka/src/emo2_boot/hit_region.rs:55-56` | 「k=1.0 契約によりサーフェス px と同一空間で照合される」 |
| DD-IE-10 素通し規約 | `crates/areka/src/input_events/mod.rs:97`・`:104-105`（加えて `:135`・`:174-175`・`:287-288` に同趣旨の再掲） | 「座標は素通し＝DPI 変換なし」 |
| presenter の ÷k 責務宣言 | `crates/areka-emo-present/src/presenter.rs:858-861` | 「変換は本メソッドの責務ではなく下流 collision-dpi-hittest の領分」→**着地後は「実装済み・どこで吸収されるか」へ改訂** |
| 純関数の Preconditions | `crates/areka-emo-compose/src/hit.rs:42-44` | 「呼び手が k=1.0 契約を保証する」→ R6.1 を維持したまま「呼び手が ÷k 済みの座標を渡す」へ |
| 受け入れ記録 | `.kiro/specs/completed/areka-P0-collision-geometry/acceptance-record.md:3-9`・`:99-101`・`:107-116` | 「k=1.0 契約下で合格・DPI追従下は本 spec の範囲外」→ R4 の新記録から**参照される側**（completed の改訂可否は DD-9） |
| バルーン側の k 無変換 | `crates/areka/src/input_events/balloon.rs:445`・`:481-483` | 「座標は物理 px 素通し（k=1.0・R4.2/8.6・DD-IE-10）」→ **R6.4 により本 spec は不改変**。R5.5 は「対象外である旨の明記」を要求 |

### 2.7 隣接ギャップ（バルーン側）の実測

`input_events/balloon.rs:481-483` は client 物理 px を `f32` へキャストしてそのまま `click_selection` へ渡す（k 乗算なし）。バルーン target も `attach_target(.., balloon_author_dpi)`（`frame.rs:517-524`）で **k を適用して表示されている**ため、**バルーン選択肢のヒットも k≠1.0 で同じ欠陥を持つ**（本 spec の R6.4／Out of scope により不改変）。担当 spec の候補と実測状況:

- `choice-render`・`choice-interact` は **completed**（消化不能）。
- W5 同居の `choice-select-events` は `input_events/balloon.rs` を編集面に持つ（roadmap 干渉台帳）が、その brief 射程は `ChoiceSelection` の drain とカスケードであり座標系ではない。
- ⇒ **担当不在**。[[deferral-requires-verified-owner]] の規律により「W6 が拾う」等のウェーブ名指しは無効で、**新規起票か既存 spec への明示編入かの計画判断**が要る（R6 の Adjacent expectations が「要件ディスカッションで開発者へ報告する」と定めたとおり）。

### 2.8 W5 同居との編集面（実測）

| spec | 主編集面 | 本 spec との交差 |
|---|---|---|
| `dpi-window-vanish` | `placement/`（spawn/follow）・`GhostWindows` 掃除 | なし |
| `choice-select-events` | `input_events/balloon.rs`（drain 増設）・`status.rs` | **なし**（本 spec が触るのは `input_events/mod.rs`。balloon.rs は R6.4 で不改変） |
| `kero-balloon` | `placement/measure.rs`・`emo2_boot/frame.rs`・`assets.rs`・`emo-text/actor.rs` | なし（本 spec は frame.rs を触らない見込み） |
| （W6.5）`scale-exact-rational` | `areka-emo-compose/src/scale.rs`・`areka-emo-present/src/presenter.rs`（`TextSlotView`）・`areka-emo-text` | **DD-3 の選択次第で交差**（`ScaleRatio` の公開面・`presenter.rs` の照会面）。本 spec が先着（W5）ゆえ W6.5 側が rebase する側だが、**公開面の名前と責務が二重化しないよう本 spec で先に宣言すべき** |

---

## 3. 要件 → 資産マップ（ギャップ分類）

| 要件 | 依拠資産 | ギャップ | 分類 |
|---|---|---|---|
| R1.1 点の ÷k 縮約 | `hit_region.rs:71` の呼出境界 | ÷k が存在しない | **Missing** |
| R1.2/1.3 領域名・None | `hit.rs:57`（不変） | なし（÷k 後は既存純関数で成立） | OK |
| R1.4 k の真実源 | `presenter.rs:705`（f32）／`:108`（厳密） | 厳密値の公開面が無い | **Missing / Constraint** |
| R1.5 k=1.0 no-op 保存 | `ScaleRatio::is_identity`（`scale.rs:132`） | 恒等分岐の明示が要る（丸め規約が恒等で素通しになることの担保） | Constraint |
| R1.6 k 取得不能時のログ縮退 | `applied_scale` が `None` を返す条件 | **`applied == None ⟹ current_surface_id == None` ゆえ presenter 経路では region が先に `None` になり、縮退分岐が構造的に空虚**（`presenter.rs:869` の `?` が先に効く） | **Unknown（要設計裁定）** |
| R1.7 DPI 変化追従 | `refresh_scale`（`:754`）＋ `applied` 更新 | なし（毎回 `applied` を読めば自動充足） | OK |
| R2.1 決定性 | 純関数＋整数演算 | f32 経路を採ると「同一入力→同一出力」は保たれるが規約が非厳密（R2.2 と衝突） | Constraint |
| R2.2 単一丸め規約 | `scale_len` の round half away from zero（乗算方向） | 除算方向の権威が**未定義**。上流との「整合」の意味が一意でない（DD-1） | **Missing** |
| R2.3 境界閉区間の保存 | `hit.rs:72` | ÷k が単調非減少なら自動保存。f32 経路では 1px 反転の危険 | Constraint |
| R2.4 重なり優先 | `hit.rs:68-73`（画家則） | なし（÷k 後の点を渡すだけ） | OK |
| R2.5 負値・窓外 | `i64` 入力・`div_euclid` | `scale_len` は `u32` 契約ゆえ流用不可。符号付き floor 除算の新設が要る | **Missing** |
| R2.6 反転/退化矩形 | `hit.rs:153-159` の既存檻 | なし | OK |
| R3.1-3.5 任意 k 注入の決定論檻 | `hit.rs` の檻の流儀 | **k を引数で受ける純関数が無ければ達成不能**（2.4 参照） | **Missing（設計制約）** |
| R3.6 `cargo test --workspace` 緑 | 既存 GPU テスト運用 | 新規テストが GPU を要さないなら影響なし | OK |
| R4.1-4.8 実機 2 水準サインオフ | `collision-probe.rs` | probe が k=1.0 前提で 5 箇所固着（2.5 参照） | **Missing（改修）** |
| R5.1-5.5 文書改訂 | 2.6 の 7 箇所 | 対象集合は特定済み。completed 配下の改訂可否のみ未定 | Constraint |
| R6.1-6.7 非退行 | `hit.rs` 不変・作者データ不変 | R6.1 の「呼び手側で完結」の解釈が DD-2 の候補 C（emo-compose に姉妹純関数）と衝突し得る | Constraint |

---

## 4. 実装アプローチの選択肢

### 4.1 ÷k の丸め規約（R2.2／R3.5 の中核）

いずれも `x: i64`・`k = num/den`（既約・正）とし、**i128 中間で整数のみ**を用いる（`div_euclid` で負値も floor 一貫）。

| 候補 | 式 | 意味 | k=2, x=100 | k=2, x=101 | k=5/4, x=1 | k=5/4, x=6 |
|---|---|---|---|---|---|---|
| **A: 素の floor** | `s = floor(x·den/num)` | 表示座標を単純に縮約 | 50 | 50 | 0 | 4 |
| **B: 画素中心（resample の逆写像）** | `s = floor((2x+1)·den / (2·num))` | **その表示画素に実際に描かれている元画素**（`AxisWalk`（`scale.rs:242-252`）の写像 `src=(d+½)·den/num−½` に対する最近傍） | 50 | 50 | 1 | 5 |
| **C: round half away from zero** | `s = round(x·den/num)` | `scale_len`（`scale.rs:166`）の丸め規約を除算方向へ鏡写し | 50 | **51** | 1 | 5 |

観測:

- **R3.2 の例（k=2.0・(100,100)→(50,50)）は 3 候補とも通る**＝要件は規約を確定していない。差が出るのは R3.5（割り切れない k）と**整数倍 k の奇数座標**である。
- **候補 C は整数倍 k で半画素ずれる**（k=2 の表示画素 101 は元画素 50 を映しているのに 51 を返す）。`scale_len` は「長さ」の丸めであって「座標」の丸めではなく、**鏡写しは意味論的に正しくない**（長さは 0 起点の区間長・座標は画素中心を持つ点）。
- **候補 B は「見えているとおりに当たる」（R1.2 の目的文）と定義的に一致する**——`resample` が実際に用いた前進写像の逆であり、拡大時は 1 元画素が占める表示画素の連なりが厳密にその元画素へ写る。境界 1px（R2.3）の内外も、写像が単調非減少ゆえ閉区間が閉区間へ写り自動保存される。
- **候補 A は k>1 で系統的に上（左）へ半画素ずれる**（k=5/4 の表示画素 1 は元画素 1 が主に見えているのに 0 を返す）。実装は最も単純。
- 「上流の寸法丸め規約と整合させる」（R2.2 の文言）は、**`scale_len` の round と字面で揃える（C）／`resample` の実写像と揃える（B）** の 2 通りに読める。**この解釈の確定が最初の設計判断**である。

**桁溢れ**: `x` は `i64`。`(2x+1)·den` は `i128` で受ける（`den ≤ u32::MAX`）。`ScaleRatio` の縮退経路（`scale.rs:106-120`）でも num/den は必ず 1 以上ゆえゼロ除算はない。

### 4.2 ÷k の着地層（brief Approach 1 と W4 doc の矛盾の解消）

| 候補 | 置き場所 | ✅ | ❌ |
|---|---|---|---|
| **A: `resolve_hit_region` で ÷k**（`emo2_boot/hit_region.rs:69-73`） | 既に `target` を算出済みで `&EmoPresenter` を持つ＝`applied_scale(target)` が**その場で引ける**。W4 doc（÷k は呼び手）と**無矛盾**。probe が私有 include するため R4 経路へ自動反映 | `crate::` フリー規律を守るヘルパ配置が要る／`resolve_hit_region` 全体を GPU 無しで檻に入れられない（純関数の切り出しが別途要る）／R6.1「呼び手側で完結」に最も素直 | k の厳密値を得るには presenter に有理数アクセサの新設が要る（DD-3） |
| **B: `EmoPresenter` に `hit_region_client(target,x,y)` を新設**（`hit_region` は不変） | 私有 `applied`（`presenter.rs:108`）を直接使え **f32 を経由せず厳密**。呼び手は 1 行 | W4 doc `:858-861` の「÷k は本メソッドの責務外」との**関係整理が必要**（既存メソッドの契約は壊さないが、同じ presenter に 2 つの座標契約が並ぶ）／GPU 無しの檻に入らない（純関数の切り出しが別途要る）／R6.1 の「呼び手側で完結」の字面と緊張 |
| **C: `areka-emo-compose` に姉妹純関数**（例 `hit_region_scaled(master,x,y,k,priority)`・`hit.rs` は不変） | **÷k と照合の合成そのものが純関数＝R3 の檻が最も強い**（配線は 1 呼出に縮む）。`ScaleRatio` は同 crate 内ゆえ厳密。wintf 非依存の憲章も守られる | `hit.rs` の「DPI を一切参照しない」宣言（`hit.rs:5`・`:40`）の**趣旨**と緊張する（k は DPI ではなく比だが、読み手には同種に見える）／R6.1 の解釈裁定が要る |
| **D: `input_events/mod.rs` で ÷k**（`:310-311`/`:382-383` 直後） | SHIORI へ送る座標も同時に ÷k される（DD-4 を一挙に解決） | scope→target 写像と presenter 参照を入力層へ持ち込む（`resolve_region_owned` `:282` の借用規律に追加負荷）／throttle（`throttle.rs:58-64`）の位置比較空間が変わる副作用 |

**組み合わせが現実的**: 「純関数（÷k 単体または ÷k＋照合）を C 相当の場所に置き、production 経路は A で 1 行呼ぶ」が、R3（GPU 不要の全網羅）と R6.1（純照合層不変・呼び手吸収）を同時に満たす最短路。ただし**純関数の粒度**（÷k だけか、÷k＋照合か）は R3.3 の「分岐の網羅」をどこで数えるかを左右する（DD-6）。

### 4.3 k の厳密性（f32 か有理数か）

| 候補 | 内容 | 評価 |
|---|---|---|
| **α: `applied_scale() -> f32` をそのまま使う** | `(x as f64 / k as f64).floor()` 等 | 実装ゼロ。**ただし `presenter.rs:678-684` が「f32 の積は権威と 1px 食い違う」と実測付きで警告している当のパターン**。境界 1px がヒットの正否を決める本 spec で採るのは、W6.5 が是正しようとしている欠陥の**新規再生産**にあたる。R2.2「単一の丸め規約」を厳密に主張できない |
| **β: `ScaleRatio` に除算方向の権威を新設**（例 `unscale_coord(self, v: i64) -> i64`）＋ presenter に `applied_ratio(target) -> Option<ScaleRatio>` | 整数のみで厳密。丸め権威が `ScaleRatio` に集中する（D4 の趣旨に沿う） | 公開面が 2 crate に増える。W6.5 `scale-exact-rational` の Approach 1（`ScaleRatio::ratio()` 新設）と**同一ファイル・近接ハンク**＝先着調整が要る（本 spec が W5 で先着） |
| **γ: presenter 内で完結**（4.2 候補 B）＋ ÷k 純関数は `ScaleRatio` を引数に取る | 公開 f32 面を増やさない | `ScaleRatio` が `areka-emo-present` の公開署名に現れる（既に `derive_scale` が返しており前例あり・`lib.rs:53`） |

**β/γ は「`ScaleRatio` に除算方向の権威を置く」点で共通**であり、差は presenter がそれを外へ出すか内で使うかだけ。**α だけが質的に異なる**（厳密性の放棄）。

### 4.4 実機 probe（R4）の改修戦略

| 候補 | 内容 | 評価 |
|---|---|---|
| **A: 既存 probe を k 対応へ改修** | `surface_size` → `target_physical_size`、anchor 中心を `scale_len` で k 倍、assert を `scale != 1.0` ＋ 2 水準の物理寸差へ、DPI 追従の駆動を追加 | 差分は中規模だが**証跡の連続性**（`acceptance-record.md` の①〜⑥プロトコルと 1:1）が保たれる。k=1.0 時の退行検出力は「2 水準の物理寸が異なる」assert が担う |
| **B: `collision-probe-dpi.rs` を新設**（既存は k=1.0 の歴史的証跡として凍結） | 既存 probe を壊さない | `#[path]` include 3 本の重複・二重保守。既存 probe は改修しないと k≠1.0 環境で**落ちる**（`assert_eq!(scale,1.0)`）ため「凍結」は成立しない＝**実質不可** |

⇒ **A が事実上の一択**（B は既存 probe が k≠1.0 実機で panic する以上、放置できない）。

---

## 5. 工数・リスク

| 項目 | 評価 | 根拠 |
|---|---|---|
| **工数** | **M（3〜7 日）** | ÷k 本体は数十行（純関数＋1 行の結線）で **S 相当**。M へ押し上げるのは (a) probe の実質作り替え＋実機 2 水準の目視サインオフ（人手・環境依存）、(b) R5 の文書改訂 7 箇所、(c) R3 の網羅檻（k×分岐の組合せ） |
| **リスク** | **Medium** | 技術的未知は小さい（写像は `resample` に実在・整数演算で閉じる）。中位なのは 3 点: ①**丸め規約の選択が実機の見え方に直結**し、机上で誤ると R4 で初めて露見する ②**probe が k≠1.0 で本当に k を得られるか未実測**（DPI component の初期化タイミング・Research Needed R-1） ③公開面が W6.5 と重なる調整コスト |

**k=1.0 で先行 landing 可**（brief の主張は現物でも成立）: ÷k は恒等 k で素通しゆえ、決定論檻までは上流と独立に緑にできる。R4 のみが実機ゲート。

---

## 6. 設計フェーズへの申し送り（設計判断項目）

> 番号付きで列挙する。**いずれも本書では決定しない**。要件ディスカッション／設計ディスカッションで開発者が裁定する。

1. **DD-1 ÷k の丸め規約**: 候補 A（素の floor）／B（画素中心＝`resample` 逆写像）／C（round half away from zero）。R2.2 の「上流の寸法丸め規約と整合」を **`scale_len` の字面（C）** と読むか **`resample` の実写像（B）** と読むかの解釈確定を含む。判定材料は §4.1 の対照表（C は整数倍 k で半画素ずれる／A は k>1 で系統的に半画素ずれる）。
2. **DD-2 ÷k の着地層**: §4.2 の A/B/C/D。**brief Approach 1（presenter 内 ÷k）は W4 が `presenter.rs:858-861` で明文化した「÷k は呼び手責務」と衝突する**ため、brief をそのまま採るなら当該 doc の改訂が前提（R5.3 の範囲に含めるか否かも同時に決める）。
3. **DD-3 k の厳密性と公開面**: §4.3 の α（f32 で妥協）／β（`ScaleRatio` に除算権威＋presenter に有理数アクセサ）／γ（presenter 内で完結）。**W6.5 `scale-exact-rational` の `ScaleRatio::ratio()` 新設と同一ファイル近接**ゆえ、本 spec（W5・先着）が置く名前と責務を先に宣言し、W6.5 側の brief へ申し送るかどうかも判断対象。
4. **DD-4 SHIORI へ送る座標（Reference0/1）の空間**: ÷k を `resolve_hit_region` 内で行うと、`KanadeMsg::Mouse(MouseInput{x,y})`（`input_events/mod.rs:153-159`／`:184-190`）へ載る座標は**client 物理 px のまま**となり、region（サーフェス px 空間で解決）と座標（表示空間）が別空間になる。R6.3 は「どのイベントをいつ送るか」の不変のみを要求しており座標値には沈黙している。**正典（ukadoc）の「ローカル座標」の空間定義と突き合わせる必要**（M2 で撫で座標を使う SHIORI が現れたときに効く）。副次論点として `plan_mouse_move`（`throttle.rs:58-64`）の位置比較を ÷k 前後どちらの空間で行うか（÷k 後だと移動検出の実効粒度が k 倍粗くなる）。
5. **DD-5 R1.6 の非空虚化**: presenter 経路では `applied == None ⟹ current_surface_id == None` ゆえ「k が取れないが判定は続行する」分岐が**構造的に到達不能**（`presenter.rs:869`）。(a) 純関数を `Option<k>` 受けにして縮退分岐を純関数側で檻に入れる、(b) 要件を「防御的分岐」として実装しログのみ檻に入れる、(c) 到達可能な経路（probe・将来の別呼び手）を明示する、のいずれを採るか。**[[areka-log-first-no-silent-failure]] と [[test-only-decision-branches-not-proven-wiring]] の両立点**。
6. **DD-6 決定論檻の粒度**: 純関数を「÷k 単体」にするか「÷k＋照合の合成」にするか。前者は `hit.rs` と完全に無干渉だが「resolver が実際に ÷k を呼んでいる」配線は檻に入らない（＝÷k の呼び忘れという本欠陥クラスそのものが無防備）。後者は R3.3 の 5 分岐を 1 本の純関数で網羅でき配線が 1 呼出に縮むが、R6.1 の「純照合層の契約不変」の解釈裁定が要る。
7. **DD-7 probe 改修の範囲**（R4 の成立条件）: ①窓 resize 先を `target_physical_size()` へ ②`read_back` anchor の中心点を `scale_len` で k 倍 ③`assert_eq!(scale,1.0)` を「`scale != 1.0` ＋ 2 水準で物理寸が異なる」へ ④`GetClientRect == physical_size` へ ⑤DPI 追従の駆動（`refresh_scale`＋`take_pending_resize` 相当）を probe に持たせるか、実モニタ上での初回 show で k が確定することを実測で確認して不要と判断するか ⑥`:446-447` の陳腐化コメント改訂。**②③④は親エージェントの申し送りに無かった追加所見**。
8. **DD-8 バルーン側 k 無変換の担当**（§2.7）: `input_events/balloon.rs:481-483` は同じ欠陥を持つが担当 spec が実在しない（`choice-render`/`choice-interact` は completed＝消化不能）。新規起票か既存 spec への明示編入かの**計画判断**。要件本文が「要件ディスカッションで開発者へ報告する」と定めた項目。
9. **DD-9 R5 改訂の対象集合と completed 配下の扱い**: §2.6 の 7 箇所のうち、`specs/completed/areka-P0-collision-geometry/{design.md,acceptance-record.md}` は**完了済み spec の成果物**である。(a) completed を直接改訂する、(b) 本 spec の `acceptance-record.md` から参照・上書き宣言する、(c) 両方、のいずれか。R5.1/5.2 は「改訂する」「消化済みへ更新する」と書いており (a) 寄りだが、completed 不可侵の運用規律があるなら (b) との折衷が要る。
10. **DD-10 W5 同居の編集面確認**（R6.7 エスケープ条項）: 本書の実測では W5 4 本と互いに素（§2.8）。ただし DD-2 で候補 D（`input_events/mod.rs` で ÷k）を採ると `choice-select-events` の隣接ファイル（`balloon.rs`）とは依然素だが、DD-3 で β を採ると `areka-emo-compose/src/scale.rs` が W6.5 `scale-exact-rational` と同一ファイルになる（後続ウェーブゆえ本 spec が先着＝許容だが**申し送り義務**）。

---

## 7. Research Needed（設計フェーズで実測すべき未確定事項）

- **R-1（最優先）**: `collision-probe.rs` の窓（`spawn_ghost_windows` 生成）が、**初回 `ShowSurface` の時点で実モニタ DPI を `DPI` component に持っているか**。`apply_show`（`presenter.rs:362-363`）は show 時点の component を読むため、既定 96 のまま初回表示すると k=1.0 で固まる（`DPI` の更新点は `wintf/src/ecs/window_proc/window_pos.rs:322`＝WM_DPICHANGED 経路のみを確認済み。**生成時の初期化経路は未追跡**）。k≠1.0 が観測できなければ R4 が成立しない。
- **R-2**: 本番経路（`emo2_boot`）で shell target の `applied` が実 DPI 由来 k になっていることの**ログ実測**（`AREKA_APP_SMOKE_EXIT_MS` ＋ `RUST_LOG` grep・[[areka-real-machine-signoff-bounded-auto-exit]]）。W4 は完了しているが、shell 面は「最初の `\s` cue まで非表示」（`frame.rs` 冒頭 doc）ゆえ表示成立点＝k 確定点が talk 開始後になる。R4 の観測手順に影響する。
- **R-3**: 正典（ukadoc）の `OnMouseMove` Reference0/1「ローカル座標」が、SSP 実装で**サーフェス座標系か窓 client 座標系か**（DD-4 の材料）。SSP は等倍前提ゆえ正典が沈黙している可能性が高く、その場合は areka 側の裁定事項になる。
- **R-4**: 2 水準の実 DPI 環境で k=5/4（120dpi）が**割り切れない縮約を実際に含む**こと（R3.5 の期待値が実機で意味を持つ）の確認。emo2 fixture は `seriko.dpi` 宣言なし＝author_dpi=96 を実測済みゆえ、120dpi モニタで k=5/4・192dpi で k=2 になる見込み（＝候補 B/C と A が実機で分岐する条件が揃う）。

---

## 8. 設計フェーズへの推奨（決定ではない）

- **丸め規約は「実写像との一致」を基準に選ぶことを推奨**（候補 B）。R1.2 の目的文「見えているとおりの部位が当たる」は、`resample` が実際に用いた写像の逆をとることと**定義的に一致**する。A/C を採る場合は「なぜ見えている画素と 1px ずれてよいか」を design で明示的に引き受ける必要がある。
- **k の厳密性は放棄しないことを推奨**（β/γ）。`presenter.rs:678-684` の実測警告と W6.5 の存在は、f32 経路が「1px 食い違う」ことを**既に証明済み**である。境界 1px が判定を決める本 spec で α を採ると、R2.2/R2.3 の主張が実質的に空証明になる。
- **檻は「÷k を引数 k で受ける純関数」を最小単位に置くこと**（R3.1 が GPU 不要・実窓不要を要求する以上、これは選択ではなく制約）。そのうえで DD-6 で照合まで含めるかを決める。
- **probe は「2 水準で物理寸が異なること」を hard assert に含めることを推奨**（R4.1 の証跡要求を機械化でき、k=1.0 への静かな退行を人手判断に委ねない）。

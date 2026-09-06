# Brief: areka-P0-balloon-canon-residue

> **種別**: 追跡 spec（正典先送り 4 点セット＝完全語彙＋縮退シーム＋**追跡 spec＋roadmap 明記**——`kero-balloon` は前 2 点を完備したが後 2 点が欠けていた 6 項目の一括登記）。⑥emo（balloon 系列解決）帰属。
> **源**: `kero-balloon` requirements.md Out of scope（:28-:34）＋ COMPAT §8（:141/:144）。2026-08-01 未登記先送り棚卸で「語彙とシームはあるが追跡 spec が無い」孤児群と判定。**2026-08-12 に `balloon-visibility` から表示寿命側の 4 項目（Problem 7〜10・COMPAT §8 の当該行）が追加登記された**ため、収載は 6 項目＋4 項目の 2 系統になっている。
> **着手ゲート**: **M1 外**。これらの語彙を実際に使うゴースト／バルーンの適合が必要になった時（M2 互換面拡大）、または `emo2-conformance-e2e` #14（未使用系列の未描画無破綻確認）が検出欠陥を just-in-time 起票する時に解禁。**e2e は検出の場であって修正の担当ではない**（同 brief Scope が明記）——検出されたら本 spec が受け皿。

## Problem

`kero-balloon` が確立した系列解決権威（`SeriesFamily` テーブル・`crates/areka-emo-present/src/balloon.rs:62`）は balloon 本体系列（`balloons`/`balloonk`/`balloonp{n}def`）のみを収載する。正典にはさらに以下があり、いずれも**縮退シーム済み・実装なし**:

1. **装飾系列の per-scope 化＋深い旧名**: `arrow*`（矢印）・`marker*`（マーカー）・`sstp_new*`・`clickwait*` 等の付属画像族。旧名がさらに深い（例: `arrows`→`arrow` の 2 段旧名）ため `SeriesFamily.scope0_legacy` が**複数要素の配列**として設計済み（COMPAT §8 :141 に「装飾族はより深い旧名」と記録済み）。テーブル 1 行追加で拡張可能。**追加軸（2026-08-27・bvc 討議 5 から登記）**: 縦書きバルーン（`vertical,1`・SSP 2.8.80/2.8.83）では `arrow0`／`arrow1` のスクロール方向が**右／左**へ再解釈される（横書きの上／下に対応する縦書き写像・正典の明文は descript `vertical` 項のみ＝bvc requirements SC10 が「クリア・改行・スクロールの縦書き挙動は未規定・矢印の説明から間接推測のみ」と疑義登記済み）。系列解決の軸（本項の従来分）とは独立の第 3 軸であり、実装時は両軸を同時に満たすこと。語彙の出典＝bvc Requirement 5.4。
2. **面 ID 偶奇＝左右向き面集合＋自動切替**: 正典は偶数面=左向き・奇数面=右向きの対を規定し、バルーンがキャラのどちら側に出るかで自動選択される。areka は面 0 固定（kero-balloon R2.6 の attach `surface_id: 0` literal）。
3. **`balloon.defaultsurface`／`kero.balloon.defaultsurface` の非追従**: descript でバルーン初期面を指定する語彙。未実装（COMPAT §8 :144 登記済み）。
4. **`\![reload,balloon]`**: 実行時のバルーン資産再解決。未実装。
5. **`balloonc*`（入力窓系列）の kero 側**: 入力ボックス用バルーン系列の per-scope 対応。未実装。
6. **多面バルーンの面別上書き実被覆**: emo2 fixture は各系列とも面 0 の 1 枚のみ＝面 ID 単位フォールバック（kero-balloon R1.3/R2.3）の**多面での実走証跡が無い**（合成 fixture 檻のみ）。多面 fixture の整備。

**注**: `\b[N]`@scope≥1 の end-to-end 檻（kero-balloon 既知シーム）は `balloon-visibility` が多面シナリオを組む際の**合流候補が第一候補**（同 brief へ申し送り済み）。**vis が要件フェーズで拒否した場合に本 spec が拾う**——二重登記ではなく受け皿序列の明記。

### `balloon-visibility` からの追加登記（2026-08-12・同 spec Requirement 7）

上記 1〜6 は `kero-balloon` 由来の**系列解決**側の残語彙である。これとは別軸に、`areka-P0-balloon-visibility` が**バルーンの表示寿命**側で先送りした 4 項目があり、同 spec の Requirement 7.7 が本 spec を受け皿として指名している。いずれも完全語彙と縮退理由を `doc/COMPAT_ARCHITECTURE.md` §8 へ記録済みで、実装のみが無い:

7. **`\![set,balloontimeout,時間]` の実導出**: バルーン表示のタイムアウト時間指定。単位ミリ秒／カウント起点はスクリプトの表示が終わってから／`0` または負数（正典の箇条書きでは「`0` か `-1`」——**同一項の中で表現が割れており `-2` の扱いが曖昧**）でタイムアウトなし／時間指定の省略で既定値へ復帰／そのスクリプト中のみ有効。現状は汎用 `\!` コマンドとして転記されるだけで、可視性の判断側は既定値（30 秒）しか読まない。**⚠ 同タグの compile 側（台本コンパイル時の干渉）は `areka-P0-sakura-time-directives` の所有**で、実導出時は双方が必要になる（COMPAT §8 の当該 2 行が住み分けを明記）。
8. **`OnBalloonClose`／`OnBalloonTimeout`／`OnBalloonBreak` の SHIORI 発火**: vis は語彙・Reference 割当・受け渡し口の型（`BalloonLifecycleNotice`・`crates/areka/src/emo2_boot/talk_lifecycle.rs:184`）までを残し、**構築側も消費側も存在しない**状態で `#[allow(dead_code)]`（同 `:183`）と理由注記の対を置いている。本 spec が実発火（UI→kanade の通知路＋kanade 送出側の受理）を着地させた時点でこの許容ごと外れる。emo2 は 3 イベントとも消費者ゼロ（fixture 実測）。
9. **`\x`／`\x[noclear]`（クリック待ち）**: `\x` はクリック後に scope が `\0` へリセットされ `\f` 系（文字装飾）の効果も解除される（`\e` で解除される指定は継続）／`\x[noclear]` はクリック後もバルーンの内容と scope を保持し `\f` 系も残る。現状は転記層で原文のままの寛容パススルー（`Instruction::Raw`）へ落ちて消費者がいない。**可視性ではなく会話の進行を止める機能**であり、vis の単一規則（可視の文字があるか）では表現できないため可視性側での近似実装を禁じてある。**相互登記（2026-08-27・bvc 討議 5 から）**: 本項の定義は文字どおり「`\f` 系の効果が解除される/残る」であり、「`\f` 状態の何がリセットされるか」の権威定義は `areka-P0-text-decoration-canon`（同日起票・`\f` 核 17 項目の所有者）が供給する——**本項単独ではリセット意味論を着地できない**（同 spec brief の Out 節と対になる相互参照）。
10. **中断で終わった会話のタイムアウト起点の精密化**: vis は中断時も正常終了と同一の起点値（台本の占有区間の終端）を用いており、誤差は必ず表示を保持する側へ倒れる。中断が起きた時刻そのものを起点に採るには中断の理由と位置を表示側から会話進行側へ渡す配線が要り、それは項目 8 の `OnBalloonBreak` の実発火に必要な情報と同一である——**8 と一体で扱い、単独で先行させない**（同じ情報を二重に作るため）。

### `balloon-vertical-canon` からの追加登記（2026-08-29・同 spec の実装完了時）

上記 1〜10 とは別軸に、`areka-P0-balloon-vertical-canon`（bvc・バルーン縦書きの正典化）が実装を完走した時点で**所有者の無い残件が 2 件**残った。いずれも bvc の受入基準に対応するが、bvc 自身の境界（語彙登記のみ・`\_l`／プロパティ／`\f` 装飾は実装しない）では着地できない。完全語彙と縮退理由は下記および `doc/COMPAT_ARCHITECTURE.md` §8 の bvc 由来 13 行が正本。

11. **縦書き字形（グリフ直立・縦書き用字形）の観測点が repo に存在しない**: bvc 要件 6.1（「日本語の文字を直立させ、縦書き用の字形を持つ文字〔句読点・括弧・長音符等〕については縦書き用の字形で描画する」）の**観測面**。実装は DirectWrite のネイティブ縦組みで達成済みで、SSP の `@` フォント機構は模倣しない（bvc 裁定 4・標準ゴシックへの自動差し替えもしない）。しかし決定論テストで固定されているのは `DirectionRecipe::for_mode` の `reading`／`flow` 方向設定だけで（`crates/areka-emo-text/src/draw_format_metrics_tests.rs`）、**字形選択そのものを観測する成果物は 0 件**である——字形が退行しても全緑のまま通る（bvc が掲げた「全緑は十分性の証拠にならない」がそのまま当てはまる残穴）。⚠ 述語の設計に注意: §8 の「フォント縦書き異体の挙動等価」行は**グリフ単位の完全一致を保証しない**と明記しているため、観測は SSP との一致ではなく「縦書きでは横書きと異なる字形が選ばれる」等の**反証可能で areka 内で閉じた述語**で組むこと。実描画の読み戻し（`draw_readback` 系）か AI vision 目視（記憶 `emo-text-byte-equiv-default-font-blindspot` の先例）が候補。

12. **`writing_mode` 未知値の警告文言が実挙動とずれている**: bvc 要件 2.7 の実装先。現行の文言は「未知の writing_mode 値のため horizontal_tb へフォールバックする」（`crates/areka-emo-text/src/writing.rs`）だが、bvc 着地後の実挙動は**「当該キーを指定なしとして扱う」**であり、`vertical` が宣言されていればそちらが採られる（`horizontal_tb` になるのは両キーとも無効なときだけ）。bvc は design の Error Handling 表が「**現行の文言・件数を維持**」と命じたため意図的に据え置いた（既存インライン `mod tests` が文言と件数を逐語固定してもいる）。**意味論そのものは §8 の `writing_mode` 行に正しく登記済み**で、ずれているのはログ文言だけ＝表示結果への影響は無い。是正時は文言と、それを逐語固定しているインラインテストを**同時に**直すこと。

> **注**: 本節の 2 件は bvc の `/kiro-validate-impl`（2026-08-29）が掘り当てた。同 spec の残る観測穴だった `emo2-choice` fixture の開始点 `(5,5)` は、同日 `choice_fixture_test.rs` へ檻を足して**その場で解消済み**（本 spec の担当外）。

## Current State

- 系列解決は単一権威（`prefix_chain`/`resolve_balloon_faces`）＝新系列はテーブル行の追加で乗る設計（kero-balloon R1.8/R1.9 の帰結）。
- 上記 6 項目すべて: 実装ゼロ・COMPAT §8 登記済み（1・3 のみ）・emo2 は未使用ゆえ M1 実害なし。
- 追加登記の 7〜10 も実装ゼロだが、**COMPAT §8 への登記は 4 項目とも完了**している（`balloon-visibility` task 7 が実施）。emo2 の辞書に `balloontimeout`・`\x`・3 イベントのハンドラがいずれも現れないため M1 実害なし（同 spec brief の fixture 実測）。
- `emo2-conformance-e2e` #14 が「`arrow`/`marker`/`online`/`balloonc`/`sstp` 未描画で破綻なし」の**負検証のみ**を持つ。

## Desired Outcome

（解禁時）対象ゴーストが実際に使う項目から、`SeriesFamily` テーブル拡張＋面選択規則＋descript 語彙＋`\![reload,balloon]` を実需要順に実装。全項目で ukadoc 全文→SSP 実挙動→COMPAT 記録の順（乖離があれば SSP 正・kero-balloon R7.6 の先例）。

## Approach

実需要駆動（M2 の実物ゴーストで使用語彙を実測してから優先順を決める）。先行して固定できるのは:
- `SeriesFamily` の複数旧名配列は実装済み構造＝装飾族は行追加＋面別上書きの層マージ再利用のみ。
- 面偶奇の自動切替は `balloon_alignment`（実装済み・kero-balloon R3.2 が消費）を入力に取る純関数になる見込み。

## Scope

- **In**: 上記 6 項目＋追加登記の 7〜10（＋vis が拒否した場合の `\b[N]`@scope≥1 e2e）。
- **Out**: balloon 本体系列の解決規則（kero-balloon で完成・不変）／**バルーン表示ライフサイクルの判断そのもの**——いつ出していつ消すかの規則と既定 30 秒の運用は `balloon-visibility` が完成させており不変。本 spec が扱うのは同 spec が実装しなかった 7〜10 の語彙だけで、着地時も既存の判断規則を置き換えない／windowposition 族（`windowposition-limit`）。

## Boundary Candidates

- 系列テーブル拡張（データ行のみ＝檻はテーブル駆動で自動拡大）。
- 面偶奇選択の純関数。
- 多面 fixture（合成でなく実 PNG 複数面——vis の多面シナリオと共用できれば 1 回で済む）。

## Out of Boundary

- placement 層全般。
- SERIKO アニメーション定義の per-scope 化そのもの（kero-balloon R5.6 で完成・バルーン表が空なのは fixture 事実であって欠陥ではない）。

## Upstream / Downstream

- **Upstream**: `kero-balloon`（系列解決権威・完成）／`balloon-visibility`（多面 fixture の先行整備者になる可能性・`\b[N]` e2e の第一受け皿・**追加登記 7〜10 の送り元**＝同 spec R7.7 が本 spec を受け皿に指名）。
- **Downstream**: M2 互換面拡大の各ゴースト適合。

## Existing Spec Touchpoints

- **Extends**: なし。
- **Adjacent**: `balloon-visibility`（受け皿序列: `\b[N]` e2e は vis 優先・本 spec は次順）／`emo2-conformance-e2e`（#14 は検出のみ・修正は本 spec へ）。

## Constraints

- M1 では着手しない（`surfaces-basepos`・`sakura-time-directives` と同じ M2 解禁ゲート棚）。
- 面引数は不透明文字列・解決は下流（areka-surface-args-opaque-string-downstream-resolve）。

---

> **📌 2026-09-02 棚卸⑫（XL＝3 軸分割推奨・項目 12 はバグ確定）**——アンカー再測定: `balloon.rs:62` `SeriesFamily` ✅・COMPAT §8 :141/:144 ✅・`attach.rs:364 surface_id: 0` ✅・e2e brief:98 ✅。**ずれ**: `talk_lifecycle.rs:184`→**:188**（enum・`#[allow(dead_code)]` は :187）・kero-balloon requirements :28-34→**:25-39** に散在・`writing.rs` の文言は **:311**。
> **項目 12 は現行 main の挙動バグ（唯一）**: `writing.rs:311` は「未知の writing_mode 値のため horizontal_tb へフォールバックする」と言うが、実挙動は同ファイル `:32` のとおり「**指定なしとして扱う**」（`vertical` 宣言があればそちらが採られる）。表示影響なし・ログ文言のみ・**S・前提なし・単独着地可**（roadmap ⓪ の任意項目＝直接修正可）。
> **項目 13（新規・zsp research §13.9 #7 由来）**: COMPAT §8 の 5 行（`:160-165`）が `roadmap.md:132` の空行を指していた件は **棚卸⑫で doc 側を直接是正済み**（本 brief の項目 7/8/9 番号と `status-execution-states` brief への引用に差替）＝消化済み・本 brief の作業なし（記録のみ）。
> **分割の継ぎ目（開発者裁定）**: ⑴ 系列解決 1〜6（emo 帰属・`balloon.rs` `SeriesFamily`＋多面 fixture）／⑵ 表示寿命 7〜10（kanade＋UI 配線・8 と 10 は一体・7 の compile 側は `sakura-time-directives`・9 は decoration 供給が前提）／⑶ bvc 残 11〜12（emo-text 帰属・11＝縦書き字形の観測檻・12＝S）。前提: M1 外（e2e #14 の検出 or M2）・項目 12 のみ前提なし。

### `emo-text-line-height-canon` からの追加登記（2026-09-06・同 spec タスク 6.3）

既存の項目 1〜12（および 📌 2026-09-02 の注が記録のみとして触れる項目 13）とは別軸に、`areka-P0-emo-text-line-height-canon`（行送りの正典化・W12 裁定枠 A′）が実装を完走した時点で、**バルーン定義の粗さ**に属する残件が 2 件残った。いずれも同 spec の裁定（2026-09-05・折返し基準と描画範囲の二段構え）で意味論は確定しているが、同 spec の境界（行送りの式と二段判定の実装まで）では着地しない。裁定の全文と正典逐語は同 spec `design.md` §4.3、要件は同 `requirements.md` Requirement 6（6.7／6.9）が正本。

14. **折返し基準が描画範囲の外に解決されるバルーン定義**: `emo2-kakukaku` の `balloonk0s.txt` が `wordwrappoint.x` を自ら上書きせず、共通 `descript.txt` の `wordwrappoint.x,-34` を継ぐため、288×203 の画像で折返し基準が **254** に解決され、描画範囲の右端 `validrect.right` の **240** の外に出る（本体側 `balloons0s.txt` は `-49`＝351 ≤ 356 を自ら上書きしており、この粗さを持たない）。areka 側の扱いは確定済みで実装も着地している——描画範囲の行内軸の遠辺（横書き `right`・縦書き `bottom`）を**絶対上限**として無条件に折り返し、文字を描画範囲の外へ置かない。供給面（文字を描く面）の寸法は描画範囲ちょうどのまま広げない（描画範囲を広げて救済する案は裁定で却下）。読み込み時に警告を 1 回記録する（`crates/areka-emo-text/src/region.rs` の `TextRegion::resolve` 末尾・欄は `balloon`／`axis`／`wrap_threshold`／`inline_limit`）。**fixture は改変していない**（`crates/pilot/examples/shiori-host-32/fixtures/emo2/` は無改変で着地）。本項が残すのは areka 側の挙動ではなく、**バルーン定義の側をどう扱うか**——粗いままでよいのか、正典に沿った定義かを確かめる道具（受理台帳や警告の集約）を持つのか——の判断である。
    - **付記（警告の `balloon` 欄が名前になっていない）**: 上の警告のバルーン名の欄は、定数 `BALLOON_NAME_PLACEHOLDER = "(名前なし)"` で埋めてある。`descript.txt` の `name,`（`emo2-kakukaku` は `name,kakukaku for emo-gs`）は実在するが、`crates/areka-parsers/src/balloon/parse.rs` の統合写像がバルーン名を写しておらず、`BalloonModel` に取得口が無い（`impl Font` の `name` はフォント名で別物）。**引受先は `areka-P0-ukadoc-survey-assets`**（balloon descript キーの受理台帳）で、2026-09-06 に同 spec の brief へ登記済み。取得口ができた時点で本項の警告の欄が名前で埋まる。
15. **行末禁則文字のぶら下がり（折返しの遅延）**: 裁定の意味論では、「」」等の行末禁則文字は折返し基準（`wordwrappoint`）を**超えてぶら下がってよい**が、描画範囲（`validrect` の当該遠辺）は超えてはならない。`emo-text-line-height-canon` は二段構え（折返し基準 soft と絶対上限 hard の 2 値・2 判定を別々に持つ形）までを実装し、**ぶら下がりそのものは実装していない**（同 spec 要件 6.9・design §4.3・§11.3）。折返し基準を絶対上限へ丸め込む案（1 値 1 判定）を採らなかったのは、まさにこのぶら下がりを後から表せるようにするためであり、実装に必要な足場（`wrap_threshold` と `inline_limit` の 2 値・配置直前の上限評価）は配置層（`crates/areka-emo-text/src/layout.rs`）に既にある。残るのは禁則文字の集合の定義と、行末での折返し遅延の規則である。**引受先の選択の経緯**: 要件 6.9 の候補欄は `areka-P0-text-decoration-canon` を挙げていたが、上の項目 14 と同じ「バルーン定義と折返しの正典」の軸にあるため本 spec へ置いた（decoration 側の brief にも同日、引受先が本 spec である旨を相互参照として登記した）。

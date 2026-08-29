# 設計検証レポート: areka-P0-scope-zorder-pinning（改訂第 2 版・所有の鎖）

**実施**: 2026-08-29 ／ **対象**: design.md 改訂第 2 版（867 行・鎖方式への全面書き替え）
**入力**: requirements.md 改訂第 2 版（確定・受入基準 70 件・要件 14 が方式を拘束）・research.md（§10 差し戻しの根拠／§11 ギャップ分析／**§12 設計前実測**）・`.kiro/steering/`・実コード
**方式**: 非対話（design-review.md の Analysis → Critical Issues → Strengths → GO/NO-GO）。設計中の file:line 主張は本ワークツリーの実コードで抜き取り検証した。
**注記**: 本レポートは 2026-08-27 付の検証レポート（初版設計＝毎巡是正方式・実機 NO-GO で撤回）を差し替える。

## Design Review Summary

本版は「前後関係を毎巡直す」を捨て、正典実装 SSP と同じ**分岐の無い所有の鎖**へ全面的に書き替えたものであり、初版 NO-GO の根因（未実測の方式選択）は §12 の設計前実測 9 本（`crates/wintf/src/api_owner_chain_probe_tests.rs`・恒久の檻として着地済み）で正面から潰されている。層の分界（wintf → areka の import を作らない・型は wintf 側に置き areka が構築する）・記録規律・檻の作法・退役計画のいずれも既存の正典と整合し、受入基準 **70 件すべてが Requirements Traceability に行を持つ**（1.1〜14.5 を機械列挙して欠落ゼロを確認）。ただし、**退役計画が要件 9.5 で保全対象と宣言した語彙の実装をそのまま削除してしまう**という自己矛盾が 1 件あり、加えて要件 7.3 の実現機構の根拠が事実と食い違い、要件 2.5 の文言と DD-3b の裁定が突き合わされていない。

### file:line 主張の抜き取り検証（初版で頻発した陳腐化の再検査・14 点）

| 主張 | 実測 | 判定 |
|---|---|---|
| `api.rs:141` `set_window_owner` ／ `:152` `clear_window_owner` | 一致 | OK |
| `api.rs:625-627` 実測檻の登記 | `#[cfg(test)]` ／ `#[path]` ／ `mod` の 3 行 | OK |
| `zorder_pair_establish.rs:169` owner 確立 | `match set_window_owner(...)` | OK |
| `zorder_pair_maintain.rs:258-262` 訂正対象の doc 段落 | 「スコープをまたぐ owner はそもそも存在しない」の段落そのもの | OK |
| `zorder_pair_maintain.rs:286` 切離し | `clear_window_owner(handle.hwnd)` | OK |
| `zorder_group_ledger.rs:256/271/276/305/333` 判定順 | UnparsableToken／ModeMixed／TooFewElements／DuplicateElement／`normalize_scope_blocks` すべて一致 | OK |
| `spawn.rs:663-671` `FrameFinalize` チェーン | 3 system の `.chain()` | OK |
| `emo2_boot/mod.rs:510` `.before(...)` | 一致 | OK |
| `frame/wiring.rs:95` 台帳の住処 ／ `:194` 種蒔き | 一致 | OK |
| `mod.rs:14/16/18` 退役 3 モジュールの登記 | 一致 | OK |
| `spawn_zorder_pair_deferred_tests.rs:59`（6 件）／`zorder_pair_deferred_vocabulary_tests.rs:76`（8 件） | 件数まで一致 | OK |
| `signoff-scan.ps1:41-50 / 122 / 140-142 / 154 / 155 / 214` | すべて一致 | OK |
| 行数主張（ledger 580・drain 495・cue 159・descript 92・group 710／403／279 ほか） | すべて一致 | OK |
| `api.rs:63` `get_window_above` ／ `zorder_pair.rs:511` `measure_*` | 前者は関数本体が `:72`（`:63` は doc 冒頭）、後者は `SIBLING_SCAN_LIMIT` 定数（`measure_*` は `:525/:564/:575/:635`） | 軽微なずれ 2 件 |

## Critical Issues（3 件）

### Critical Issue 1: 要件 9.5 で「保全する」と宣言した `[zorder-group] applied`／`rejected` の実装が、退役ファイルの中にある

**Concern**: 設計は `[zorder-group] applied`／`rejected` を保全語彙として明記し、退役の順序でも「全工程を通じて 1 度も欠かさない」と書いている。しかし実装の所在は
`crates/wintf/src/ecs/window/zorder_group.rs:653/668/700`（`log_group_applied`／`log_group_rejected`／`log_group_member_missing`）と
`zorder_group_diag.rs:41/54`（`APPLIED_TAG`／`REJECTED_TAG`）であり、**どちらも「退役するファイル」に丸ごと挙がっている**。移設先は設計本文のどこにも書かれていない（`log_group` を含む記述は traceability の 5.4 行だけ）。
さらに、これらを呼ぶ `crates/areka/src/emo2_boot/frame/zorder_descript.rs:36`（`use wintf::ecs::window::{log_group_applied, log_group_rejected};`）は**「変更しない既存ファイル」表に載っている**。`zorder_drain.rs:67` も同じ 3 関数を import している。

**Impact**: 退役の順序 (3)（退役ファイルの削除）で areka 側がコンパイル不能になるか、通すために記録を落とせば要件 9.5 が割れる。`signoff-scan.ps1` は `$TAG_GROUP_APPLIED`／`$TAG_GROUP_REJECTED` を据え置く前提（J1 の受理判定・J2）なので、語彙が消えれば実機サインオフの判定が丸ごと成立しなくなる。初版が「機械検査全緑のまま実機で不成立」になった形と同じ、**退役の段取りにだけ現れる穴**である。

**Suggestion**: 保全語彙の**新しい住所を設計で名指しする**（例: `zorder_chain_diag.rs` へ `[zorder-group] applied`／`rejected` のタグ定数と `log_group_applied`／`log_group_rejected` を移設し、target も併記する／あるいは `zorder_group_diag.rs` を「applied・rejected だけを残す小さなファイル」として退役対象から外す）。併せて (1) `zorder_descript.rs` を Modified Files へ移す（import 先が変わるため「無編集」は成立しない）、(2) `log_group_member_missing`（要件 8.4 の記録材料。`[zorder-chain] absent` へ移ったのか退役なのかが未記載）の去就を明記、(3) 記録の target が変わる場合は `signoff-scan.ps1` の `RUST_LOG` 指定（現行は `zorder_pair`＋`zorder_chain` の 2 本）へ追随させること。

**Traceability**: 9.5・5.4・8.4・9.4 ／ **Evidence**: design.md「退役するファイル」「変更しない既存ファイル」「`zorder_chain_diag`／保全する既存語彙」「実機サインオフの改訂」

### Critical Issue 2: 要件 7.3（バルーン再表示への追随）の実現根拠が事実と食い違う

**Concern**: DD-9 と traceability 7.3 は「再表示で**窓の在庫が変われば**合成結果が変わるので drain 相の内容差分が自然に検出する」として、初版の引き金（`balloon_visibility_phase.rs` の `note_balloon_shown`／`wants_group_follow_on_show`）を撤去する。しかし要件 7.3 自身が再表示を「窓の中身の絵の消去・再描画を指し、Windows 上の窓の表示状態の変化を伴わない」と定義しており、窓の在庫（`GhostWindows`＝`spawn.rs:294-309`。scope ごとにキャラ窓とバルーン窓の Entity 対を spawn 時から保持）は**再表示では 1 ミリも変わらない**。したがって `ChainGroupPlan` は同一で `dirty` は立たず、drain 相の差分は**発火しない**。

**Impact**: 撤去の根拠が誤っているため、実装者は「差分が拾ってくれるはず」と信じたまま 7.3 の檻を書かない（設計の Integration テスト一覧にも再表示の項が無い）。実際には鎖方式では再表示で順が崩れる経路が無い＝**何もしなくてよい**可能性が高いが、それは「在庫差分で拾う」とは別の理由である。理由が誤ったまま着地すると、実機サインオフで 7.3 を主張する証跡が組み立てられない（初版の申し送りと同じ轍）。

**Suggestion**: 7.3 の充足根拠を「再表示は HWND も owner も触らないので鎖が崩れる経路が無く、確認も是正も不要（構造で満たす＝1.3／7.4 と同型）」へ書き替え、traceability 7.3 の Components を「—（構造で満たす）」にする。そのうえで、(1) 撤去する引き金が他の要件を担っていないこと（`wants_group_follow_on_show` は `ZOrderGroups` 非空を条件にしていた）を確認し、(2) 決定論の檻として「バルーンの内容可視性が変わっても `ZOrderChainPlan` が変化しないこと」を 1 本置くこと。

**Traceability**: 7.3・7.4・14.5 ／ **Evidence**: design.md DD-9・Requirements Traceability 7.3・Modified Files（`balloon_visibility_phase.rs`）

### Critical Issue 3: 要件 2.5「グループに属するスコープ以外の窓を動かさない」と DD-3b が突き合わされていない

**Concern**: §12.5／§12.9-3 の実測により「鎖は 1 つの塊として動き、後押しの際に**鎖の外の窓を追い越すことがある**」ことが確定し、DD-3b はこれを許容する裁定を下している。その根拠として挙がるのは要件 6.1／6.2／3.6 だけで、**要件 2.5 は一度も参照されていない**。traceability の 2.5 行は `zorder_chain_compose`（edge を張らない）＝不変条件(3) だけを根拠にしており、後押しによる塊移動には触れていない。要件 2.5 の文言は「動かさない」であり、「相対順を規定しない」（6.1）より強い読み方ができる。

**Impact**: 実機サインオフの目視で「グループ外のキャラ窓が鎖に追い越された」場面が出たとき、それが仕様どおりなのか不成立なのかを**要件文だけでは判定できない**。初版は要件 1.1／1.2／2.1 の解釈と機構の食い違いで実機 NO-GO になっており、同じ判定不能を持ち込むリスクがある。

**Suggestion**: DD-3b の射程に 2.5 を明記し、「2.5 が禁じるのは**グループ外の窓を対象とする指令を出すこと**および**グループ外どうしの相対順を変えること**であって、鎖が塊として移動した結果の相対位置変化は含まない」と設計で線を引く。併せて COMPAT §8 の裁量へ 11 件目として登記し（正典が沈黙する領域である）、実窓檻 6 の主張（部外者どうしの前後不変）が 2.5 の証跡でもあることを Testing Strategy に書くこと。要件文そのものを触る必要がある場合は設計ディスカッションの議題へ。

**Traceability**: 2.5・6.1／6.2・3.6・12.2 ／ **Evidence**: design.md DD-3b・Requirements Traceability 2.5・research.md §12.5

## Design Strengths

1. **初版 NO-GO の根因（未実測の方式選択）を、設計に先立つ実測で正面から潰している**。`api_owner_chain_probe_tests.rs` の 9 本（張り替えの非即時性・後押しの形 3 種の比較・最小化／非表示／破棄の連動・切離しによる連動の無効化・スプライス・`clear_window_owner` の失敗・部外者への影響）が実在し、恒久の檻として着地済みであることを確認した。しかも**檻自身の非決定を 3 度潰した記録**（§12.9）があり、うち 1 件（`GW_HWNDPREV` を挿入位置に渡すと他プロセスの窓の消滅で黙って失敗する）は本番の設計判断 DD-3 を変えている。設計の主要主張が「§12 の実測 → DD → traceability」で 1 本に繋がっている。
2. **「ペア edge は鎖の部分列である」という構造上の発見により、新設が横断 edge 1 種類だけに縮んでいる**。実証済みのペア 5 ファイルが挙動非接触で残り（唯一の例外が doc 段落 1 つの訂正）、解除＝自分が張った edge の撤去だけで既定状態が構造的に復元する。層の分界も正しい——`CrossEdge`／`ChainGroupPlan` を wintf 側に置いて areka が構築する形により、**wintf → areka の import を 1 本も作らない**（不在要素を正準表記の文字列で運ぶという代償まで自覚的に記述されている）。

## Final Assessment

**判定: GO（条件付き——上記 3 件を設計ディスカッションで解消してからタスク生成へ）**

**Rationale**: 方式（要件 14）の実現可能性は §12 の実測で閉じており、荷重のかかる platform 挙動で §12 または既存の檻に裏づけの無いものは見当たらない。受入基準 70 件は全件が traceability に行を持ち、file:line 主張も 14 点中 12 点が完全一致（残る 2 点は行のずれのみで無害）。Critical Issue 1〜3 はいずれも**局所的な記述・段取りの穴**であって、アーキテクチャの是非や複雑さの不均衡ではない——1 は移設先の名指し、2 は充足根拠の書き替え、3 は裁定の射程の明記で閉じる。

**Next Steps**:
1. 設計ディスカッション（`/kiro-design-discussion areka-P0-scope-zorder-pinning`）で Critical Issue 1〜3 を裁定し design.md へ反映する。特に 1 は**タスク分割に直結する**（保全語彙の移設が退役の順序 (3) の前提になる）。
2. 反映後に `/kiro-spec-tasks areka-P0-scope-zorder-pinning` でタスクを生成する。退役の順序 (1)〜(4) を跨ぐタスクには、各段で `[zorder-pair]` 6 語と `[zorder-group] applied`／`rejected` が生存していることの検査を必ず添えること。
3. 軽微: `api.rs` の `get_window_above` 引用を `:72` へ、`zorder_pair.rs:511` の `measure_*` 引用を `:525/:635` へ直す（`:511` は `SIBLING_SCAN_LIMIT`）。

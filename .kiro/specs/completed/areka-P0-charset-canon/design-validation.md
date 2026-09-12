# 設計レビュー: areka-P0-charset-canon

> 2026-09-12 実施（`kiro-validate-design`・非対話・Fable 5.1）。入力: `design.md`（2026-09-11 生成）・`requirements.md`（12 要件・裁定 ⑴〜⑶ 確定）・`research.md`・`brief.md`・steering（`tech.md`／`structure.md`／`logging.md`）。設計の主張は実コードに当たって照合した（引用は「何の定義行か」で示す）。

## Review Summary

設計は要件 12 本（1.1〜12.5）の全 ID をトレーサビリティ表で引き受け、型の置き方（`Charset` newtype＋純粋な `CharsetNegotiator`＋codec は事実を返すだけ）と依存方向（`encoding_rs` → host32 → kanade → ghost → areka・parsers は host32 と辺を持たない）が既存の層構造と矛盾しない。実コードとの突合で設計の事実主張はほぼ全て一致した（下記「照合結果」）。見つかった問題は 3 件とも**検証手順・テスト計画・追随漏れ**の局所的な欠陥で、いずれも数行で直る。構造の作り直しは要らない。

## 照合結果（設計の主張 ⇄ 実コード）

| 設計の主張 | 結果 | 根拠（何の定義行か） |
|---|---|---|
| `Shiori3Client::new` の呼び出し点は 7（kanade 2・host32 E2E 5） | **一致** | `real.rs` の `ShioriBackend for ShioriConnection` の `get`／`notify` 2 点、`lifecycle_cyclic_e2e.rs` の `let client = Shiori3Client::new(&parent)` 2 点、`lifecycle_kill_e2e.rs` 1 点、`shiori_request_e2e.rs` 2 点 |
| `&mut negotiator` の借用形が自己参照にならない | **成立** | `ShioriConnection` は `window` と `negotiator` を別フィールドで所有し、`get(&mut self)` の本体で `Shiori3Client::new(&self.window, &mut self.negotiator)` はフィールド別の分割借用（共有＋可変で別フィールド）ゆえコンパイル可。`real_connect` の closure は `Send + 'static` を要するが、`Charset(&'static Encoding)` は `Encoding` が `static` に置かれる型（`Sync`）なので `Send`、`BTreeSet<String>` も `Send`。設計の「`Send` を要求しない」は「要求されなくても満たす」と読める（後述の軽微 3） |
| UTF-8 要求バイト列は差分 0（3.4／9.6）、in-proc は挙動 0（5.4） | **成立** | `build_request` は `String` を組み立てて `into_bytes()`。UTF-8 の `Encoding::encode` は `Cow::Borrowed` で同一バイト。`shiori_inproc.rs` の `build_input`／`map_get_outcome` は codec を再利用しており `Force(UTF_8)` はヘッダを見ない現状と同じ |
| ヘッダの ASCII 事前走査が対応集合で安全 | **成立（既知の限界 1 つは登記済み）** | Shift_JIS の後続バイトは `40–7E`／`80–FC`、EUC-JP／EUC-KR は `A1–FE`（＋`8E`／`8F`）、GBK／Big5 の後続は `40–FE`、gb18030 の 4 バイト形の 2・4 バイト目は `30–39`——いずれも `0A`／`0D`／`3A` を含まず、行分割と `:` 分割は誤らない。行頭の名前が ASCII `charset` になるには先頭バイトが ASCII でなければならず、多バイト列の途中で偽陽性は起きない。ISO-2022-JP だけは JIS 状態のまま行を跨ぐ SHIORI で `Charset` を拾えないが、設計は §8 登記 ⑺ に含めている。UTF-16 系は `NotEncodable` で型に入らない |
| 4.4／7.4 の重複抑止 | **成立** | `warned: BTreeSet<String>` の鍵「種別＋鍵」（ラベル／イベント名／文字コード名／強制時の食い違い）。毎秒の `OnSecondChange` でも警告 1 回 |
| 9.7 の変異形 | **成立** | EUC-JP の系統（`A4 A2`）＋ISO-2022-JP の別名解決。「Shift_JIS だけ」「UTF-8／Shift_JIS だけ」の両変異で赤 |
| 8.3 台帳 5 行と証拠の置き場所 | **一致** | `ledger/shiori.toml` の `spec_shiori3:Charset:1`＝`degraded`・owner 本 spec／`Charset:2`＝`absent`・owner 空。`ledger/assets.toml` の `shiori.encoding`／`shiori.forceencoding`／`descript_shell_surfaces:charset`＝`absent`・owner 本 spec。5 つの URL は `catalog.toml` に実在。README §3「定義箇所に 1 行・呼び出し側に書かない・URL の後ろに語を続けない」に対し、設計の置き場（`build_request` の `Charset:` 書き出し行の直上／`scan_charset_header` の一致の腕／`resolve.rs` の `map.get` 行の直上／`prescan.rs` の `charset` キー一致の腕）は全て定義箇所 |
| 1,000 行番人 | **一致** | `resolve.rs` は実測 959 行（4 行追加で 963）。`shiori3.rs` 598・`shiori_wiring.rs` 178。例外表は非接触。既存 URL コメントは `shiori3.rs` に 9 行（設計の数と一致） |
| ログの target／event 形 | **一致** | host32 には現状 `target:` 指定が 0 件。kanade は `"shiori-actor"`＋`event = "…"`、ghost は `"ghost-boot"`（5 件）。新設 `"shiori-charset"` は同じ形。レベルは `logging.md`（後退＝warn・ライフサイクル＝info・開発者向け＝debug）に合う |
| `charset::decode` は BOM を吸収する | **一致** | `decode.rs` の手順 4「`enc.decode(bytes)`（BOM を sniff）」。emo2 の surfaces.txt は BOM 無し実測ゆえ 6.3 の同一性は成立 |
| `DefaultEncoding::to_encoding` は `pub(crate)`・`ShioriMount` は `#[non_exhaustive]`・構築点 3＋1 | **一致** | `model.rs` の `pub(crate) fn to_encoding`／`#[non_exhaustive] pub struct ShioriMount`。`model_tests.rs` 3 か所＋`resolve.rs` の `let shiori = ShioriMount {` |
| 定数 `pub const UTF_8: Charset = Charset(&encoding_rs::UTF_8_INIT)` | **成立** | toolchain は rustc 1.98（const から static への参照は 1.83 で安定化済み） |
| 開発者方針（記録なしの後退 0・零の明示・ukadoc 由来の意味論・平易な語） | **一致** | 設計は各「変えないもの」を 0 と明記。裁定は ukadoc 引用に基づく。`design.md` に「檻」「毒化」の語は 0 件 |

## Critical Issues（3 件・いずれも局所的）

🔴 **Critical Issue 1**: 実機確認の `RUST_LOG` 指定では `charset_switched` が出ない
**Concern**: §Testing「実機確認」は `RUST_LOG=info,shiori_host32_host=debug,areka_ghost=debug` を指定するが、`tracing_subscriber` の env-filter の指令はイベントの **target** に前方一致で掛かる。設計のログは `target: "shiori-charset"` を明示するため、モジュールパス `shiori_host32_host` の指令には一致せず、`charset_switched`（debug）は `info` の既定に落ちて**記録されない**。完了仕様 emo2-conformance-e2e の走行も同じ理由で `kanade=trace`（target 名）を与えている。
**Impact**: 11.3 の期待「`charset_switched from=Shift_JIS to=UTF-8` ちょうど 1 行」が **0 行**になり、実装が正しくても実機確認が赤（または「0 行＝切替なし」と誤読）になる。
**Suggestion**: 手順の指定を `RUST_LOG=info,shiori-charset=debug,ghost-boot=debug` に改め、§Monitoring の表に「env-filter は target 名で点ける」の 1 行を添える。`signoff-record.md` の様式に「点灯の裏づけ」（同 target の別イベントが出ていること）を 1 欄置くと、0 行を沈黙と取り違えない。
**Traceability**: 11.2, 11.3（7.2）
**Evidence**: `design.md` §Testing Strategy「実機確認（11.1〜11.3）」・§Monitoring の表

🔴 **Critical Issue 2**: テスト計画が要件 9.3 と 4.8 の文言に届いていない（2 点）
**Concern**: ⑴ 9.3 は「応答による採用が**次の要求の `Charset` ヘッダとバイト列**に現れること」を固定せよと定めるが、設計の `charset_tests.rs` ⑸ は「採用後の `current()` が変わること」までしか見ない。`current()` の変化と「次の要求のバイト列が変わること」は単位が違う主張で、前者の緑は後者を証明しない。⑵ 4.8 の例外 1 本 `parse_invalid_utf8_is_parse_error` の入力 `[FF FE 00]` には status 行が無い。設計自身の事後条件「`Err(Parse)` は status 行の欠落のみ」に従えば、この入力は新 codec でも `Err(Parse)` のままで、設計が書く「`Ok`＋U+FFFD へ期待値更新」は成立しない。
**Impact**: ⑴ は「Shift_JIS 分岐に退化したら赤」の要（採用 → 次の要求）が検査に無いまま緑になる。⑵ は実装者が赤を見て事後条件の側を緩める誘惑を生む。
**Suggestion**: ⑴ `shiori3_charset_tests.rs` に 1 本足す: `CharsetNegotiator::new(SHIFT_JIS,false)` → `note_response(Some("EUC-JP"), false)` → `build_request(.. charset: neg.current())` で `Charset: EUC-JP` 行と Reference0 `A4 A2` を定数と比較（窓不要・純関数）。⑵ 例外テストは入力を「status 行あり＋`Value` に不正バイト」（例 `SHIORI/3.0 200 OK\r\nValue: \xFF\r\n\r\n`）へ差し替えて `Ok`・`decode_had_errors == true`・`Value` に U+FFFD を期待し、旧入力 `[FF FE 00]` は「status 行なし → `Err(Parse)`」の期待を**変えない**テストとして別名で残す（4.8 の「他の期待値変更 0」を守る）。
**Traceability**: 9.3, 9.7, 4.8, 10.2
**Evidence**: `design.md` §Testing Strategy「Unit Tests」の `charset_tests.rs` ⑸・`shiori3.rs` 既存テストの段落、§shiori3 codec「Postconditions」

🔴 **Critical Issue 3**: `real_connect` の呼び出し点が 1 つ漏れている
**Concern**: 設計は `real_connect` の引数追加の追随先を「`runtime.rs` の `Helper` 腕」だけとするが、`crates/areka-ghost/tests/ghost/snapshot_capture_test.rs` の「実 backend へ Recorder を合成した Custom wiring」の `let connect = real_connect(helper_exe, mount.shiori);` も呼び出している。File Structure Plan にも Revalidation Triggers にも無い。
**Impact**: `cargo test -p areka-ghost` の統合テストがコンパイルエラーになる（実行に i686 成果物と実 pasta を要するテストだが、コンパイルは x64 で常に走る）。
**Suggestion**: File Structure Plan に同ファイルの 1 行（第 3 引数 `DefaultEncoding::Ansi`——採取フィクスチャは本番の既定と同じ）を足し、Revalidation Triggers の `real_connect` 行に併記する。併せて `ShioriConnection` の構築点 2 か所（`shiori_wiring.rs`・`real_helper_test.rs`）は実測どおりで漏れ無し。
**Traceability**: 5.2, 12.1（既存テストを無改変で緑に保つ前提の側）
**Evidence**: `design.md` §Revalidation Triggers「`real_connect` の引数追加 → `runtime.rs` の `Helper` 腕」・§File Structure Plan「変更ファイル」表

## Design Strengths

1. **規則の置き場が 1 つ**: 採用規則・重複抑止・ログを `CharsetNegotiator` に閉じ、codec は事実（`replaced`／`charset_header`／`decode_had_errors`）を返すだけにした。in-proc は `Force(UTF_8)` で同じ codec を通り、ログも状態も持ち込まれない。`note_response` の表 7 行が各 1 テストに対応し、要件 4.2〜4.6 が窓無しで固定できる。
2. **「任意の文字コード」を型で守る**: `Charset` は `for_label` と定数 2 つ以外から作れず、`output_encoding() == self` の不変条件で「綴りとバイト列が食い違う要求」を構造的に排除している（10.3）。既定写像を ghost 側の 2 腕 `match` に置いて、到達不能な失敗腕（記録のない死んだ経路）を作らなかった判断も筋がよい。

## Final Assessment

**Decision: GO**

**Rationale**: 構造・依存方向・要件被覆は実コードとの照合で裏付けられ、3 件の指摘はいずれも検証手順とテスト計画の局所的な穴（合計で数行〜1 テスト）で、設計の骨格を変えない。設計ディスカッションで 3 件を `design.md` に書き戻してからタスク生成へ進めばよい。

**Next Steps**:
1. 設計ディスカッションで Critical Issue 1〜3 を `design.md` に反映（§Testing の `RUST_LOG`・9.3 の連鎖テスト・4.8 例外テストの入力・`snapshot_capture_test.rs` の追随）。
2. `/kiro-spec-tasks areka-P0-charset-canon` でタスク生成。タスクには `cargo test -p shiori-host32-host --no-run` と `cargo test -p areka-ghost --no-run`（i686 依存の E2E・統合テストのコンパイル確認）を明記する（`research.md` §9.5 の対策と同じ）。

## 軽微な所見（非ブロッキング）

1. **重複抑止の鍵は正規化してから**: 4.4 の「同じラベルにつき 1 回」の鍵に生ラベルを使うと `foo` と `FOO` で 2 回警告になり、ラベルを応答ごとに変える壊れた SHIORI では `warned` が際限なく育つ。鍵を trim＋ASCII 小文字化してから入れる（1 行）。
2. **`EncodedRequest.bytes: Vec<u8>` への `into_owned`**: UTF-8 では `Cow::Borrowed` を 1 回複製する。バイト列は同一なので 3.4 は成立するが、費用が気になるなら `build_request` の `String` を `into_bytes` へ流す形は「UTF-8 の分岐」になるので採らないのが正しい。現状のままでよい旨を書き添えると実装者が迷わない。
3. **`CharsetNegotiator` は `Send` でなければならない**: `real_connect` の closure（`Send + 'static`）へ move されるため。設計の「`Send` を要求しない」は「窓と同じ前提」の意図だが、`Rc` 等を将来入れると崩れるので「`Send` を満たす（`&'static Encoding`＋`BTreeSet<String>`）」と書く方が正確。
4. **`ParsedResponse` の構造体リテラル**: `client.rs` テスト 8 か所＋`shiori3.rs` 1 か所という数は実測と一致（`map_get_result_*` 8 本）。機械的追随のみ。
5. **2.7 の初期 Shift_JIS 要求**: emo2 の最初の NOTIFY `OnInitialize` と GET `username` は ASCII のみ（`research.md` §5.10 実測）で、pasta の 204 応答も `Charset: UTF-8` を含む（`pasta_shiori/src/shiori.rs` の `default_204_response`）ため、切替は最初の応答待ちで 1 回。設計の 11.3 期待と整合。

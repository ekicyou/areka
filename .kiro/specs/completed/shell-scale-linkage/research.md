# ギャップ分析: shell-scale-linkage

> 2026-10-31・本ブランチ（main `9604677a` の上の `dba36c6c`）のコードで確かめた。コードは「何の定義か」（関数名・型名＋ファイルパス）で指す。
> 入力: `requirements.md`（確定）・`brief.md`・steering（`product.md`・`tech.md`・`structure.md`・`roadmap.md` の C15 の行）。SSP は動かしていない（測る点は 6 節に挙げた）。

## 1. 要約

- **読む口・送る口・descript の読み・受け手の表は、どれも先例の形がそのまま使える。** 値は ✅ `mcp-scope-rect-property` の読み手の登録（`SylphyaReader::register_pull`）に載せれば MCP の `get_property` とゴーストの道の両方に同時に出る。`OnShellScaling`・`OnBalloonScaling` は UI の側から `KanadeMsg::RaiseEvent` で送る形（`frame/switch.rs` の `raise_changed`）があり、許可の表 `ALLOWED_EVENT_IDS` に 2 語を足すだけで通る。`balloon.syncscale` は `read_balloon_dontmove` とほぼ同じ形で読める。
- **足りないのは 3 つ。** ⑴ 名前の自由なイベントを出どころ `internal,property` で送る kanade の種類（`EventId`・`KanadeMsg`・`Input` に 1 つずつ、出どころの定数 1 つ）。⑵ 「絵にかかった倍率が変わった」を知る点（今は `EmoPresenter` の `applied` が変わったことを外へ知らせる口が無い）。⑶ 倍率を百分率の正確な文字にする口（`ScaleRatio` は分子・分母を外へ出さず、`as_f32` は使ってよい先が 2 つに限られている）。
- **思っていたより難しい点が 2 つ。** ⑴ シェル・バルーンの切り替えでは `ReplaceTarget` が `applied` を引き継ぐので、作者の基準の DPI が違う絵に替わると同じ target の倍率が跳ぶ＝「変わった」と取り違える（要件 5.6 を守るには切り替えの印が要る）。⑵ ukadoc は `\![get,property]` を「このスクリプトの再生後、SHIORIイベント…が通知される」と書いており、要件 1.2・1.6 の「タグに届いた時点で送る」と食い違う読みがある（SSP で測る）。
- **案は 3 つ（7 節）。** 受け手を ⒜ `areka-ghost` の中（`PropSetCueSink` の隣）に置くか、⒝ `emo2_boot` に置くか、倍率の変わりを ⒜ 毎フレーム比べるか、⒝ 表示層の更新点から押し出すかの組み合わせ。推奨の組（案 C）は「受け手は `areka-ghost`・倍率の変わりは表示層から押し出す・`syncscale` は資源の変わりで読む」。
- **規模 M（11〜16 タスク）・危険 中。** 先例どおりの所が大半で、新しいのは kanade の 1 種類と表示層の変わりの知らせ。切り替えの取り違えと SSP の往復の測りが残る。

## 2. 今あるもの（調べた場所）

### 2.1 台本の命令の受け口と受け手の表

| もの | 場所 | 分かったこと |
|---|---|---|
| 受け手の表 | `crates/areka/src/emo2_boot/consumer_ledger.rs` の `ConsumerLedger::canonical` | 42 件。`(get, property)` はまだ無く、`get` の名前の登記も他に無い（選別子つきで 1 行足せる）。`check_script` は `mcp/check_script_judge.rs` で `ledger.consumer_of(name, first)` を引くので、行を足せば要件 1.8 は自動で満たす |
| emo2 の外の受け口の先例 | `CommandConsumer::PropertyWrite`（`areka_ghost::prop_sink::PropSetCueSink`） | 受け口が `areka-ghost` に在っても表に行を持てる。合意の檻 `consumer_ledger_agreement_tests.rs` の `emo2_sink_of` は `PropertyWrite` を `None`（emo2 の受け口でない）として扱う＝同じ扱いで足せる |
| 受け口の組み立て | `emo2_boot/runtime_boot.rs` の `runtime_sinks`（13 本）／`areka-ghost/src/runtime.rs` の `boot_with_origin` | `boot_with_origin` は `spawn_dispatcher` の直前の「単一登録点」で `PropSetCueSink` を足す。その場所には `sylphya_reader`・`ghost_asker`・`kanade_tx` が全部揃っている |
| 運び手 | `dola::cue::TalkCue`（`at`・`actor`・`command`・`duration`） | 出どころ（`ScriptOrigin`）の欄は無い＝受け口は「どの出どころの台本のタグか」を知らない（要件には要らないが、返しのイベントの `SecurityLevel` を元の台本から継ぐ案は取れない） |
| `get` の特別扱い | `areka-parsers`・`areka-sakura`・`dola` | `"get"` の特別扱いは無い＝汎用の運び手で `("get", ["property", イベント名, 名前…])` として届く |
| 出どころで止めるタグ | `emo2_boot/script_restriction.rs` の `RESTRICTIONS`（5 行） | `\![get,property]` は ukadoc が制限しないので載っていない（足す必要なし） |

### 2.2 値の読み口

| もの | 場所 | 分かったこと |
|---|---|---|
| 読み手の登録 | `areka-sylphya/src/pull.rs` の `PullSource`・`PullShape`／`reader.rs` の `register_pull` | 形 `currentghost.scope(*).scaling` などで登録できる。写しに無い点付きの名前だけが読み手に届く。今、`scaling`・`currentmonitor` を写しへ出している所は無い |
| 手本 | `crates/areka/src/ghost_session_scope_rect.rs`（279 行） | 実行系ごとに `wire_reader` が `Arc<RwLock<表>>` を作って読み手に渡し、`FrameFinalize` の系 `sync_scope_window_table` が変わったときだけ表を差し替える（プッシュ）。スコープの選択子の読み `scope_selector` は私有の関数（使い回すなら `pub(crate)` に上げる） |
| 登録の場所 | `ghost_session_runtime.rs` の `rewire_runtime_consumers`（`ghost_session_scope_rect::wire` を呼ぶ）・`ghost_session.rs` の `register_systems` | ゴーストの切り替え・読み直しで表が新しくなる＝要件 4.5 は形で満たす |
| MCP | `mcp/get_property.rs` の `handle` | `resolve_dotted_str(&asker, name)` で引き、無ければ `NG:Cannot find such property name.`。`\![get,property]` の受け手も同じ関数を同じ問い手（`ghost_asker_id`）で呼べば、要件 1.1（名前の規則・大小の扱い）・1.5（名前を問わない）・3.10（同じ値）は同じ道を通るので揃う |
| ゴーストの道 | `crates/areka/src/shiori_host.rs`（`GetProperty`） | 同じ置き場を読む。本番は 32bit の中継越しで届かない＝決定論のテストで確かめる（`mcp-scope-rect-property` の確かめた点 4 と同じ） |

### 2.3 倍率の真実源

| もの | 場所 | 分かったこと |
|---|---|---|
| 実効の倍率 | `areka-emo-present/src/presenter/read.rs` の `applied_ratio`（`ScaleRatio`）・`applied_scale`（`f32`） | 「いま画面に載っている絵に実際に掛かった k」。一度も表示していない target は `None`。**更新点は `presenter/show.rs` の `target.applied = Some(scale)` の 1 か所**（と `presenter/replace.rs` の引き継ぎ） |
| k の式 | `areka-emo-present/src/scale.rs` の `derive_scale`・`ScalePolicy { author_dpi, app_scale }` | k ＝ `app_scale × (窓の dpi_x ÷ author_dpi)`。**「アプリ管理拡大率」`app_scale` はもう在り、今は `ScaleRatio::ONE`**（`ScalePolicy::new` の呼び手が 1 を渡す）。要件 3.3 の「DPI 追従の倍率 × 表示の倍率」はこの値そのもの。縦横で DPI が違えば `dpi_x` を採る（単一のスカラー）＝要件 3.6 の「横と縦に同じ数」はこの規約に合う |
| 正確な値 | `areka-emo-compose/src/scale.rs` の `ScaleRatio`（既約の分子・分母） | 分子・分母を返す公開の関数が無い。`as_f32` は doc に「裁定済みの消費者は 2 つだけ・他の用途へ拡大してはならない」とある＝百分率の文字をここから作るのは規約に反する |
| 更新が走る相 | `emo2_boot/frame.rs` の `emo2_frame_system` | k が変わる点は 1 フレームに 2 つ（`run_dpi_phase` の `refresh_scale` と `run_drain_phase` の `apply_show`）。両方の下流に `run_text_scale_phase`（毎フレーム走査・D11-4 で理由づけ済み）と、相の並びの最後の `status_report::run_status_report_phase`（「前に送った組と違うときだけ kanade へ送る」台帳つき）がある |
| `refresh_scale` の戻り値 | `frame/dpi.rs` の `dpi_phase_with` | `None` は「k が変わったが丸めた寸が同じ」「見えていない」も含む＝**戻り値は「倍率が変わった」の印に使えない**（doc が明記） |
| DPI | 窓の `wintf::ecs::DPI`（`dpi_x`・`dpi_y`） | 窓ごとの値。整合待ちの札 `DpiSyncHold`（`placement/dpi_sync.rs`）が掛かっている間や、見えていない間は、DPI が新しく k が古いフレームがある |

### 2.4 イベントを送る口（kanade）

| もの | 場所 | 分かったこと |
|---|---|---|
| 汎用の通知の入口 | `areka-kanade/src/msg.rs` の `KanadeMsg::RaiseEvent`・`schedule/change.rs` の `on_raise_event` | 許可の表に在る名前を、定常（`Phase::Steady`）でだけ送る。定常以外は `warn!` で捨てる（積まない）。UI から送る手本は `emo2_boot/frame/switch.rs` の `raise_changed`（`OnShellChanged` などを GET・返信なしで送る） |
| 許可の表 | `schedule/events.rs` の `ALLOWED_EVENT_IDS`（64 語） | `OnShellScaling`・`OnBalloonScaling` は無い。数の檻 `schedule/events_change_tests.rs` の `assert_eq!(ALLOWED_EVENT_IDS.len(), 64)` を 66 に直す |
| 名前の自由なイベントの先例 | `EventId::Choice`（`On` で始まる名前だけ）・`EventId::Sstp`（`sstp::validate_notify`） | `EventId::Sstp` は出どころが `MCP`／`external,sstp` に固定（`request_origin.rs` の `of_event`）。**`internal,property` を名乗れる種類は無い** |
| 種類が増えたときに要る腕 | `request_origin.rs` の `of_event`（既定の腕なし＝コンパイラが求める）・`actor.rs` の送出前の名札（`EventId::Sstp(_) => NOTIFY_ORIGIN`）と許可の判定（`EventId::Sstp(name) => validate_notify(...)`）・`schedule/mod.rs` の `Input` の腕・`schedule/types.rs` の `Input` | `EventId::Sstp` を足したときと同じ 5〜6 か所 |
| 出どころの語 | `areka-talk/src/origin.rs` の `SenderWord::Property`・`ScriptOrigin` の定数（`GHOST`・`MCP_SEND`・`MCP_NOTIFY`） | 語はあるが、組の定数 `PROPERTY`（`internal,property`・SSTP でない・`local`）は無い |
| 名前の検査 | `areka-kanade/src/sstp.rs` の `validate_notify` | 空・空白だけ → `EmptyEvent`、名前か Reference に CR・LF・NUL → `ControlCharacter`。要件 2.2・2.3 の規則とちょうど同じ＝そのまま使える |

### 2.5 descript

| もの | 場所 | 分かったこと |
|---|---|---|
| 手本 | `placement/descript_window_keys.rs` の `read_balloon_dontmove` | シェル `<スコープ>.balloon.X`（`sakura.`＞`char0.`・`kero.`＞`char1.`・`char<n>.`）とゴースト `balloon.X` を読む。ただし**優先の順**（シェル＞ゴースト）で、値は `0`・`1`・`true`・`false`。要件 6.3・6.4 は **論理和**（どちらかが `true`）で `true` だけが有効＝読み方の規則が違う |
| 値の流れ | `placement/mod.rs` の `prepare_stages_for_shell` → `PreparedStages`・`PreparedPlacement` の欄 → `ghost_session.rs` → `placement::balloon_lock::seed_balloon_lock`／シェルの切り替えは `frame/switch.rs` が読み直し | `balloon_dontmove` と同じ形で持つと `placement/mod.rs` に 5 行前後（読み・2 つの構造の欄・受け渡し）＝ウェーブの約束「1〜2 行」を超える |
| 抜け道 | `placement/reset_pos.rs` の `DefaultPlacementBasis { source: DescriptSource, .. }` | **起動の窓を作る閉包（`ghost_session.rs`）とシェルの切り替え（`frame/switch.rs`）の両方が、読み込んだ descript の生の KV（`ghost_kv`・`shell_kv`）ごと資源として入れ直す。** この資源の変わりを見る系を足せば、`placement/mod.rs` にも `switch.rs` にも触らずに読める |

### 2.6 網羅台帳

`doc/ukadoc-coverage/ledger/`: `property.toml` の 3 行（`currentghost.scope_28ID_29.scaling`・`currentghost.balloon.scope_28ID_29.scaling`・`currentghost.scope_28ID_29.currentmonitor.dpi`・どれも `vocabulary-only`・持ち主 `shell-scale-linkage`）・`shiori.toml` の `OnShellScaling`・`OnBalloonScaling`・`sakura-script.toml` の `\![get,property,…]`・`assets.toml` のゴーストの `balloon.syncscale` とシェルの 3 行。**`property.toml` の 2 つの `scaling` の行の注記は「利用者がベースウェアで決める拡大率」と SSP の意味で書いてある**＝実装済みへ改めるときに DPI 追従の意味へ書き直す必要がある。

### 2.7 ぱすたの実物（`target\pasta-verify\root\ghost\pasta-in-windows`・読むだけ）

- `\![get,property]` を出すのは `OnSecondChange` の応答（喋っていないとき＝Reference3 が 1）と `OnShellScaling`・`OnBalloonScaling` の応答だけ。起動の挨拶の中では出さない。どれもタグの直後が `\e`。
- `OnPastaShellScale` は何も返さない（`return nil`）。`OnPastaScaleCheck` は横の値がずれたときだけ台詞（`＊バルーン倍率のずれ`）を返す＝**返しのイベントの応答の台本が再生されないと、この台詞は出ない**（要件 1.7 に効く）。
- 値の読みは `^%s*([%d%.]+)` で小数も読む。ぱすたの注記は「Windows の窓の影はモニタの DPI で大きくなる・ぱすたの窓の影はシェルの拡大率で大きくなる」＝areka の DPI 追従ではどちらも DPI で大きくなるので、`scaling × 96 ÷ DPI` が 100 になり面 10 を選ぶのが見た目の上で正しい（要件の Introduction のとおり）。

## 3. 要件ごとの対応表

| 要件 | 使えるもの | 足りないもの（Missing／Unknown／Constraint） |
|---|---|---|
| 1.1・1.5・3.10 | `resolve_dotted_str`＋`ghost_asker_id`（MCP と同じ関数） | — |
| 1.2・1.6 | 汎用の運び手・受け口の自己選別 | **Unknown**: 送る時点（タグの時点か台本の再生の後か・ukadoc は「再生後」）。**Missing**: 受け口 |
| 1.3 | `SenderWord::Property` | **Missing**: `ScriptOrigin` の定数・`EventId` の種類・`of_event` の腕 |
| 1.4・2.4 | Reference は `Vec<String>`（欠番は空文字列・詰めない＝`RaiseEvent` と同じ規約） | **Unknown**: SSP の答え（無い名前・名前なし） |
| 1.7 | 返しの応答を再生する道（`on_shiori_reply`／`sstp::on_notify_reply`） | **Unknown**: GET か NOTIFY か・話している最中の扱い。**Missing**: 応答の受け方の腕 |
| 1.8 | `ConsumerLedger`・`check_script_judge` | **Missing**: 表に 1 行（42→43）・見本の行 |
| 2.1〜2.3 | `sstp::validate_notify` | — |
| 2.5 | — | 記録の 1 行（`debug!`） |
| 3.1〜3.3・3.6・3.8 | `applied_ratio`（`app_scale × DPI ÷ author_dpi`） | **Missing**: 分子・分母を外へ出す口（`ScaleRatio`）。**Constraint**: `as_f32` の禁止 |
| 3.5 | 窓の `DPI` 部品 | **Unknown**: k と一緒に出すか、DPI の変わりで出すか（要件 4.2 との兼ね合い） |
| 3.7・5.3 | — | **Unknown**: 小数の桁（SSP で測る）。areka の作者の基準が 96 で Windows の DPI（24 の倍数）なら百分率は必ず整数＝小数が出るのは作者の基準が 96 でないときか、利用者の独自の拡大率（例 110%＝DPI 106）のとき |
| 3.9 | `ScopeRectSource::no_window` の形（値なし） | — |
| 4.1〜4.4 | 表のプッシュの手本（`sync_scope_window_table`）・`PullSource` の無待機の約束 | **Missing**: k が変わった点の知らせ（2.3 の更新点から押し出すか、毎フレーム比べるか） |
| 4.5・4.6 | `rewire_runtime_consumers` で表が作り直される・読み手の答えは写しにも保存にも書かない | — |
| 5.1〜5.3・5.8 | `RaiseEvent`・`raise_changed` の手本 | **Missing**: 許可の表に 2 語（64→66）・送る相 |
| 5.4 | 相の並び（`emo2_frame_system`） | **Constraint**: 表を書いてからイベントを送る順を同じ相の中で守る（1 フレーム遅らせない） |
| 5.5・5.7 | — | 本体の窓（スコープ 0）の k の前後の比較 |
| 5.6 | ゴーストの切り替え・読み直しは表が新品 | **Missing**: シェル・バルーンの切り替えの印（`ReplaceTarget` が `applied` を引き継ぐため・5 節の危険 1） |
| 5.9 | `OnMouseDragEnd` は今も送っている | 実機で確かめるだけ |
| 6.1〜6.5・6.7 | `read_balloon_dontmove`・`scoped_entry`・`DefaultPlacementBasis` | **Missing**: 論理和の読み・置き場の資源・記録。**Constraint**: `placement/mod.rs` は 970 行（上限 1000）でウェーブの約束は 1〜2 行 |
| 6.6 | — | 持つだけ（働きを変えない） |
| 7.x | 偽の読み手・偽の DPI を差す手本（`ScopeRectReadSeam`・`frame_test_support.rs`・`presenter_test_support.rs`） | 新しいテスト |
| 8.5 | 台帳の 4 本 | `property.toml` の注記の書き直し |

## 4. 新しく足すものの見積り（どの案でも要るもの）

1. kanade: `ScriptOrigin::PROPERTY`（`areka-talk/src/origin.rs` に定数 1 つ）・`EventId::Property(String)`・`KanadeMsg` の新しい知らせ（`RaiseEvent` は送り手が 9 か所なので欄を足さない）・`Input` の腕・`of_event` の腕（送り手 `areka`・出どころ `PROPERTY`）・`actor.rs` の 2 腕（名札と許可の判定＝`validate_notify`）・受け付けと応答の腕（`schedule/sstp.rs` の `on_notify`／`on_notify_reply` と同じ形の新しい兄弟か `change.rs`）。
2. 許可の表に `OnShellScaling`・`OnBalloonScaling`（数の檻 64→66）。
3. 倍率と DPI の読み手（新しい `ghost_session_scope_scale.rs`・`ghost_session_scope_rect.rs` の形）と登録（`ghost_session_runtime.rs` に 1 行・`ghost_session.rs` の `register_systems` に 1 行）。
4. 受け手の表に `(get, property)` の 1 行と説明の一覧・数の檻 42→43・合意の見本。
5. `balloon.syncscale` の論理和の読み（`descript_window_keys.rs` に関数 1 つ）と置き場（資源 1 つ）と記録。
6. 網羅台帳 4 本の行（`property.toml` の注記の書き直しを含む）。

## 5. 危険と難しい点

1. **切り替えで倍率が跳ぶのを「変わった」と取り違える（要件 5.6）。** `presenter/replace.rs` の `apply_replace` は `DetachedState.applied` を新しい装着へ引き継ぎ、同じフレームの `apply_show` が新しい作者の基準の DPI で k を書き直す。シェルの `seriko.dpi`・バルーンの `dpi` が前と違えば、同じ target の k が前後で変わって見える。区別の手がかりの候補: シェルの切り替えは `frame/switch.rs` が `DefaultPlacementBasis` を入れ直す（資源の変わり）・置き換えの世代 `NEXT_EPOCH`・`SkinSwitchInFlight` の段。バルーンの切り替えには `DefaultPlacementBasis` の入れ直しが無い＝別の印が要る。どのフレームで置き換えと表示が起きるか（同じフレームか）は設計で確かめる（**Research Needed**）。
2. **「倍率が変わった」を知る点。** `refresh_scale` の戻り値は使えない（2.3）。⒜ 毎フレーム、装着済みの target の `applied_ratio` を前の値と比べる（`run_text_scale_phase`・`status_report` の先例があり、数個の比較で軽い）か、⒝ 表示層の唯一の更新点（`show.rs` の `target.applied = Some(scale)`）で前後が違うときに知らせを積み、フレームの相で取り出す（変わった時点が分かる＝プッシュ）。開発者の基準「変わった時点が分かるならプッシュ・毎フレーム比べる案は推さない」（2026-10-23）に合うのは ⒝。⒝ は emo-present を触る（同じウェーブの ⑤ `emo-bake-log-overlap` は emo-present の `shell_target.rs`・`balloon.rs` だけ＝ファイルは重ならない）。
3. **DPI の値と k の食い違い（要件 4.2）。** 整合待ちの札や見えていない間は、窓の `DPI` が先に新しくなり k は古い。`currentmonitor.dpi` を k と同じ時点で出す（絵と食い違わない）か、`Changed<DPI>` で出す（画面には正しい）かで、ぱすたの `scaling × 96 ÷ DPI` が一瞬 100 以外になりうる。前者なら食い違いは起きない。
4. **SSP の往復の意味（要件 1.2・1.6・1.7）。** ukadoc の記述例は「このスクリプトの再生後、SHIORIイベント…が通知される」。`\![raise]` は「raiseが再生された時」と書き分けてある。タグの時点で送るか台本の終わりで送るか、GET か NOTIFY か（NOTIFY なら `OnPastaScaleCheck` の台詞は捨てられる）が答えで変わる＝SSP の MCP で測る（6 節）。
5. **ファイルの行の上限。** `placement/mod.rs` 970・`ghost_session.rs` 940（上限 1000）。`balloon_dontmove` と同じ運び方は約束を超える（2.5 の抜け道で避けられる）。
6. **ウェーブの表に無いファイル。** 案によって `areka-ghost/src/runtime.rs`＋新しい受け口のファイル・`areka-emo-compose/src/scale.rs`（分子・分母の口）・`areka-emo-present/src/presenter/show.rs`（変わりの知らせ）・`emo2_boot/frame/status_report.rs`・`areka-kanade/src/schedule/events_change_tests.rs` を触る。C15 の行の ②〜⑥ の触るファイルと突き合わせて**重なりは 0**（⑤ は emo-compose `fold.rs`・emo-present `shell_target.rs`・`balloon.rs`・`placement/measure.rs`、⑥ は kanade `schedule/resources.rs`・`menu/captions.rs`、③ は `alert.rs` だけ）。ただし `roadmap.md` の ① の欄と食い違うので、設計で挙げ直して開発者に示す。

## 6. SSP で測る点（Research Needed・開発者の機械の SSP の MCP）

要件の末尾の表の 2〜5 に、今回見つけた点を足した。

| # | 点 | 変わる受け入れ基準 |
|---|---|---|
| 1 | `\![get,property]` の返しのイベントが届く時点（タグの時点か台本の再生の後か・後なら `\e` までか、後ろの文字の表示が終わるまでか） | 1.2・1.6 |
| 2 | 返しのイベントは GET か NOTIFY か・応答の台本は再生されるか・話している最中に返った台本は今の台本を置き換えるか | 1.7 |
| 3 | 返しのイベントの `SenderType`・`SecurityLevel`・`Sender`（`ssp_shiori_log.txt` で読める） | 1.3 |
| 4 | 無い名前・名前なし・窓の無いスコープの `scaling`・`currentmonitor.dpi` の答え | 1.4・2.4・3.9 |
| 5 | `OnShellScaling` を送る場面（利用者がシェルの倍率を変えたとき・画面を移ったとき・シェルの切り替え）と、`OnBalloonScaling` との順 | 5.1・5.6・5.8 |
| 6 | 割り切れない倍率の小数の桁（例 シェル倍率 110% かつ作者の基準 144） | 3.7・5.3 |
| 7 | 返しのイベントが SSP の起動の挨拶の最中・終了の台本の最中に届くか（areka の `RaiseEvent` は定常以外で捨てる） | 1.2・1.7 |

## 7. 実装の案

### 案 A: 既存の場所に足す（受け手は `emo2_boot`・倍率は毎フレーム比べる）

- 受け手: 新しい `emo2_boot/property_get_cue.rs`（`WindowStateCueSink` の形・送出端 1 本）→ `runtime_boot.rs` の `CueSinkSources`／`runtime_sinks` を 13→14 本 → UI の側の相（新しい `frame/property_get.rs`＋`frame.rs`・`frame/wiring.rs`）で受け取り、`GhostSlot` の実行系の `sylphya_reader` で引き、kanade へ送る。
- 倍率: `frame/status_report.rs` の相に「本体の窓とバルーンの `applied_ratio` を前と比べ、違えば表を書いてから `RaiseEvent`」を足す（台帳は `Emo2Wiring` の中・ゴーストごとに新品）。
- `syncscale`: `balloon_dontmove` と同じ運び方（`placement/mod.rs`・`ghost_session.rs`・`frame/switch.rs`）。
- ✅ ウェーブの表の ① の欄にほぼ沿う。表示層に触らない。
- ❌ 受け手のために線・相・結線が増える（台本のスレッド → UI → kanade の 2 段の跨ぎ）。値はタグの時点でなく UI のフレームで引く（台本のスレッドの順と最大 1 フレームずれる）。毎フレーム比べる＝開発者の基準に反する。`placement/mod.rs` の約束を超える。

### 案 B: 新しい部品で分ける（受け手は `areka-ghost`・倍率は表示層から押し出す）

- 受け手: 新しい `areka-ghost/src/prop_get_sink.rs`（`PropSetCueSink` の兄弟）を `boot_with_origin` の単一登録点で足す。そこに `sylphya_reader`・`ghost_asker`・`kanade_tx` が揃っているので、台本のスレッドでタグに届いた時点で引き、検査し、新しい kanade の知らせで送る（UI を通らない）。受け手の表は `PropertyWrite` と同じく emo2 の外の担当として 1 行（`consumer_ledger.rs` の「11 の受け口」は 11 のまま）。
- 倍率: 表示層の更新点（`presenter/show.rs`）で前後が違うときに「target・前・後」を積み、取り出す口を足す。新しいフレームの相（新しい兄弟のファイル・`frame.rs` に 1 行・`wiring.rs` に台帳 1 欄）が取り出し、本体の窓とバルーンについて表を書いてから `RaiseEvent` を送る。切り替えの印で起きた変わりは送らない。
- `syncscale`: 新しい読み手のモジュールの中の系が `DefaultPlacementBasis` の変わりを見て読み、資源に持ち、スコープごとに記録する。
- ✅ 受け手の道が最短（線も相も足さない）。変わった時点のプッシュ。`placement/mod.rs`・`frame/switch.rs` に触らない。
- ❌ ウェーブの表に無い `areka-ghost`・`areka-emo-present` を触る。表示層の公開の口が 1 つ増える。

### 案 C: 組み合わせ（推奨の候補）

- 受け手は案 B（`areka-ghost` の兄弟）。
- 倍率の変わりは案 B（表示層から押し出す）。表の書き込みとイベントの送出は 1 つの相の中で「表 → イベント」の順。
- 正確な百分率は `ScaleRatio` に分子・分母（か百分率の文字）を返す口を 1 つ足す（`as_f32` を使わない）。
- `syncscale` は案 B（資源の変わりで読む）。
- `OnShellScaling`・`OnBalloonScaling` の送出は案 A と同じ `RaiseEvent`（許可の表に 2 語）。
- ✅ 先例の形に一番多く乗り、行の上限と約束を守る。❌ ウェーブの表との食い違いを設計で示す必要がある（5 節の 6）。

## 8. 規模と危険

- **規模: M（11〜16 タスク）。** kanade の 1 種類（5〜6 か所の腕）と表示層の変わりの知らせが新しく、残りは先例の写し。
- **危険: 中。** 形は既知だが、切り替えの取り違え（5 節の 1）と SSP の往復の意味（5 節の 4）が答えで作業を変える。

## 9. 設計の段へ持ち越す決めごと（要件討議の材料）

1. `scaling` に何を出すか（要件の表の 1）: コードの根拠は `applied_ratio`＝`app_scale × DPI ÷ 作者の基準` がもう「実効拡大率」そのもので、`app_scale` は今 1。
2. 受け手の置き場: `areka-ghost`（案 B・C）か `emo2_boot`（案 A）か。
3. 返しのイベントの送る時点と GET／NOTIFY（6 節の 1・2）。
4. 返しのイベントの kanade の受け付け: 定常だけか・起動の挨拶の最中も受けるか・`SecurityLevel` は常に `local` か（受け口は元の台本の出どころを知らない）。
5. 倍率の変わりを知る点: 毎フレーム比べるか、表示層から押し出すか。
6. 百分率の正確な文字の作り方と小数の桁（`ScaleRatio` に口を足すか・6 節の 6）。
7. 切り替えで倍率が跳んだときの見分け方（シェルとバルーンで別の印が要る）。
8. `currentmonitor.dpi` を k と同じ時点で出すか、DPI の変わりで出すか。
9. `syncscale` の持ち方（資源の変わりで読むか、`balloon_dontmove` と同じ運び方か）と、`char0.`・`char1.` の別名も読むか。
10. 2 つのイベントの代表のスコープ（要件の表の 6）: ukadoc の 2 つのイベントはスコープを持たず、areka は窓ごとに k を持つ。ぱすたは `scope(0)` だけを聞く。
11. 網羅台帳 `property.toml` の `scaling` の 2 行の注記（SSP の意味で書いてある）の書き直し。
12. ウェーブの表に無いファイルを触ること（5 節の 6）を `roadmap.md` の ① の欄へ足すか。

## 10. SSP の測り（2026-11-01・要件討議の前）

開発者の機械の SSP（2.9 系）に居た「窓際のぱすたさんDEBUG」（`C:\home\maz\git\ghost_dev\debug\pasta-in-windows-debug`・150% の画面）へ MCP の `sakurascript` で台本を 2 本送り、`ghost\master\ssp_shiori_log.txt` の送る前のバイト位置から後ろを読んだ。reload・設定の変更・ゴーストの切り替えはしていない。strict の誤りの記録は 0 件。

| 送った台本 | SSP が SHIORI へ送ったもの |
|---|---|
| `\0測り1\![get,property,OnArekaProbeGet,currentghost.scope(0).scaling,no.such.name,currentghost.scope(5).scaling,currentghost.scope(0).currentmonitor.dpi]\w9\w9\w9\w9\w9\w9後ろ\e` | 台本の受け取り（`OnTranslate` 00:01:09.711）の 0.16 秒後（00:01:09.873・後ろの待ち 2.7 秒より前）に `GET`・`ID: OnArekaProbeGet`・`Sender: SSP`・`SenderType: internal,property`・`SecurityLevel: local`・`Status: talking,balloon(0=0)`・Reference0 `150,150`・Reference1 空・Reference2 空・Reference3 `144` |
| `\![get,property,OnArekaProbeNone]`（名前なし） | **何も送らない** |
| `\![get,property,SSPからArekaProbe,currentghost.scope(0).scaling]` | `ID: SSPからArekaProbe`・Reference0 `150,150`（`On` で始まらない名前も受ける） |
| `\![get,property,,currentghost.scope(0).scaling]`（空の名前） | **`ID:` を空のまま送る**・Reference0 `150,150` |
| `\![get,property,OnArekaProbeCase,CurrentGhost.Scope(0).Scaling,currentghost.balloon.scope(0).scaling,currentghost.scope(1).scaling,currentghost.scope(1).currentmonitor.dpi]` | Reference0 **空**（SSP は名前の大小を区別する）・1 `150,150`・2 `150,150`・3 `144` |

MCP の `get_property` で同じ時に読むと、`currentghost.scope(0).scaling`・`currentghost.balloon.scope(0).scaling` は `150,150`、`currentghost.scope(0).currentmonitor.dpi` は `144`、`currentghost.scope(5).scaling` は `NG:Cannot find such property name.`。

分かったこと（6 節の表の番号）:
- **1（送る時点）**: タグに届いた時点で送る（台本の再生の後ではない）。ukadoc の「このスクリプトの再生後」は、`\![get,property]` の後ろの台本の終わりを待つ意味ではなかった＝要件 1.2・1.6 のとおり。
- **2（GET か NOTIFY か）**: `GET`＝応答の台本を受け取る形（要件 1.7 のとおり）。話している最中（`Status: talking`）でも送る。応答の台本が今の台本を置き換えるかは、ぱすたが 204 を返したので測れていない。
- **3（出どころ）**: `SenderType: internal,property`・`Sender: SSP`・`SecurityLevel: local`（要件 1.3 のとおり）。
- **4（値の無いとき）**: 無い名前・窓の無いスコープは空の Reference で番号を保つ（要件 1.4・3.9 のとおり）。**名前なしは送らない**（要件 2.4 の「Reference なしで送る」と違う）。**空の名前は空の `ID` のまま送る**（要件 2.2 の「送らず警告」と違う）。名前の大小は SSP では区別される（areka は完了 `areka-P0-property-name-case-fold` で MCP の名前の規則を決めており、要件 1.1 はそれに従う）。
- **5・6（`OnShellScaling` を送る場面・小数の桁）**: 測っていない。SSP で倍率を変えるには利用者の設定か `\![set,scaling]` で開発者の SSP のぱすたの大きさを変えることになり、元へ戻す値が確かでないため。areka は作者の基準が 96 で Windows の DPI が 24 の倍数なので、表示の倍率ができるまで百分率は整数に限られる（3 節 3.7 の行）。
- **7（起動の挨拶・終了の最中）**: 測っていない。ぱすたは起動の挨拶の中で `\![get,property]` を出さない（2.7 節）。

## 11. SSP の測り その 2（2026-11-01・議題 2 のため）

排他テストの机を持ち、止まっていた SSP（`C:\wintools\ssp\ssp.exe`）を起こした（ゴーストは「窓際のぱすたさんDEBUG」・150% の画面）。測った後は `\-` でゴーストを降ろし、SSP が終わったことと 9276 番が空いたことを確かめてから机を返した。利用者の設定は変えていない。

**スコープごとの `\![set,scaling]`**（ukadoc「ユーザの設定する倍率を100％とした倍率」＝100 で利用者の設定の倍率へ戻る）:

| 送った台本 | MCP の `get_property` | SSP が SHIORI へ送ったもの |
|---|---|---|
| `\0\![set,scaling,120]\e` | `scope(0).scaling` 150→`180,180`・`balloon.scope(0).scaling` も `180,180`・`scope(1)` は `150,150` のまま | `OnShellScaling`・`OnBalloonScaling` は **0 件** |
| `\0\![set,scaling,100]\e` | `scope(0).scaling` `150,150` へ戻る | 0 件 |
| `\1\![set,scaling,120]\e` | `scope(1).scaling`・`balloon.scope(1).scaling` が `180,180`・`scope(0)` は `150,150` のまま | 0 件 |
| `\1\![set,scaling,100]\e` | `scope(1).scaling` `150,150` へ戻る | 0 件 |

**同じゴーストの記録の過去分**（`ssp_shiori_log.txt` の全体・2026-10-18〜10-20・開発者が SSP の倍率の設定を変えた時のもの）: `OnShellScaling` と `OnBalloonScaling` が 30 件ずつ。どれも `GET`・`SenderType: internal`・**Reference は 4 つだけ**（スコープを示す Reference は無い）・値は利用者の設定の百分率（150・200・100・80・66・300 の間の行き来）。1 件を除き、`OnShellScaling` の 0〜4 ms 後に `OnBalloonScaling` が続く（ぱすたは `balloon.syncscale,true`）。1 回の設定の変更で、ゴーストあたり 1 回ずつ。

分かったこと:
- SSP の 2 つのイベントは「**利用者の設定の倍率（ゴースト全体に 1 つ）が変わった**」の合図で、スコープを持たない。
- スコープごとの実効の倍率が変わっても（`\![set,scaling]`）、SSP は送らない。実効の倍率はスコープごとに `scope(ID).scaling` で読む。
- 画面を移っても送らない（`ssp-measurement.md` の段 5）＝SSP の倍率の設定は画面の DPI と関係しない。
- areka の画面の DPI への追従は、窓ごとの実効の倍率の変わりに当たる（`frame/dpi.rs` の `dpi_phase_with` は `Changed<DPI>` の窓ごとに、そのスコープの target だけを描き直す）。SSP の意味に当てると、`\![set,scaling]` と同じく「送らない」側になる。

## 12. 設計の段の調べと決め（2026-11-01・`/kiro-spec-design -y`）

> 本ブランチ `claude/kiro-start-shell-scale-linkage-773a37`（`06701198`）のコードで確かめた。発見の型は **light（既存の仕組みへの足し）**。新しい外のクレートは足さない。読んだ手順: `kiro-spec-design` の `rules/design-principles.md`・`design-discovery-light.md`・`design-synthesis.md`・`design-review-gate.md`、テンプレート `.kiro/settings/templates/specs/design.md`・`research.md`。steering は `product.md`・`tech.md`・`structure.md`・`logging.md`・`roadmap.md` の C15 の行。

### 12.1 調べて分かったこと（9 節の持ち越しの答え）

| # | 点 | 分かったこと（どこで確かめたか） | 設計への効き |
|---|---|---|---|
| 1 | 絵にかかった k の唯一の更新点 | `areka-emo-present/src/presenter/show.rs` の `apply_show` の末尾 `target.applied = Some(scale)`。`refresh_scale`（DPI の替わり）・`apply_replace`（差し替え）・`show_target`（バルーンの出し直し）はどれもこの 1 か所を通る。同じ所が `pending_resize` を積み、`take_pending_resize` で毎フレームの相が取り出す先例がある | 「k が変わった」の印はこの 1 か所で積み、フレームの相が取り出す（プッシュ）。毎フレーム装着済みの target を見比べる形は取らない |
| 2 | 切り替えで k が跳ぶ取り違え | `apply_replace` は `attach_target`（`applied: None` で登録し直す）の後に `DetachedState.applied` を書き戻すので、作者 DPI が違えば同じ target の k が前後で違って見える | 寸法の照合のための `applied` の引き継ぎは残し、**「装着ごとの起点」を別の欄に持つ**（`attach_target` が空にし、差し替えは引き継がない）。起点が空の初回は「初めて決まった」で、イベントを送らない（要件 5.7）＝状態の形で解く |
| 3 | ゴーストの切り替え・読み直しで表示層はどうなるか | ゴーストの起動（切り替えも）は `ghost_session.rs` の `boot_wired` → `emo2_boot::wire_emo2_boot` が `Emo2Wiring`（表示層 `EmoPresenter` を含む）を新しく入れ、続けて `ghost_session_runtime::rewire_runtime_consumers` を呼ぶ。SHIORI だけの読み直し（`rebuild_runtime_in_place`）は窓・表示層を残し、実行系（sylphya の読み口を含む）だけを作り直して `rewire_runtime_consumers` をやり直す。どちらも毎フレームの排他の系の外（`ghost_quit_system`・読み直しの取り出しの系）から呼ばれる | 倍率の表は `Emo2Wiring` に持たせる（ゴーストの起動ごとに新しい・SHIORI だけの読み直しでは同じ表を新しい読み口へ登録し直す）。要件 4.5 は形で満たす |
| 4 | 返しのイベントの受け口の置き場 | `areka-ghost/src/runtime.rs` の `boot_with_origin` の `spawn_dispatcher` 直前の単一登録点に、sylphya の読み口・ゴースト自身の問い手・kanade の送出端が揃っている（`PropSetCueSink` の先例）。`emo2_boot/runtime_boot.rs` の受け口の組み立ては起動の前に走るので、読み口も kanade も無い | 受け口は `areka-ghost` の `PropSetCueSink` の兄弟に置く。`emo2_boot/mod.rs`・`runtime_boot.rs` は触らない（ウェーブの表 ① の候補より狭い） |
| 5 | 返しのイベントの kanade の道 | `EventId::Sstp` を足したときの腕は `msg.rs`（種類と `as_str`）・`request_origin.rs` の `of_event`（既定の腕なし）・`actor.rs` の記録の名札と送出の許可の判定・`schedule/types.rs` の `Input`・`schedule/mod.rs` の振り分け・`actor.rs` の `KanadeMsg` → `Input` の写し。応答は `schedule/mod.rs` の `on_shiori_reply` が今日の規則で再生する（`events::value_replaces_active_talk` は `OnSecondChange` 以外で今のトークを置き換える）。応答の台本の出どころは `schedule/translate.rs` の `Replied::reply_origin` が `RequestOrigin::base_of` で決める | 新しい種類 `EventId::Property` と知らせ `KanadeMsg::PropertyEvent` を足す。応答の再生は今日の道のまま（要件 1.7） |
| 6 | 送る時点の運び | 受け口は dispatcher が再生の時刻を進めるにつれて cue を配る（`areka-ghost/src/dispatcher.rs` の `on_tick`）＝タグに届いた時点で `emit` が呼ばれる | SSP の測り（10 節）と同じく、タグの時点で値を引いて送る（要件 1.1・1.6） |
| 7 | 正確な百分率 | `ScaleRatio`（`areka-emo-compose/src/scale.rs`）は分子・分母を外へ出さない。`as_f32` は使い道が 2 つに限られている | 読むだけの口 `parts()` を 1 つ足す（不変条件を崩さない・f32 を使わない）。百分率の文字は areka の側で整数だけで作る |
| 8 | `scope(ID)` の読み | `ghost_session_scope_rect.rs` の私有の `scope_selector` が番号・負の番号・書式の誤りを読み分ける | `pub(crate)` に上げて使い回す（読みを 2 つに分けない） |
| 9 | descript の読み直しの時点 | `DefaultPlacementBasis`（descript の生の KV の写しを持つ資源）は起動の窓づくり（`ghost_session.rs`）とシェルの差し替え（`frame/switch.rs`・シェルと descript の読み直しもここを通る）の 2 か所で入れ直される | この資源の変わり（bevy の変更検出）で `balloon.syncscale` を読み直す。`placement/mod.rs` には `mod` の 1 行だけ足す（約束の 1〜2 行の内） |
| 10 | 台帳の実装済みの証拠 | `doc/ukadoc-coverage/README.md` 3 章: `implemented` は定義箇所の `// ukadoc: <URL>` の 1 行が要る（許可表の要素・分岐の腕・語彙表の 1 行） | 証拠の行を置く場所を設計に書く |
| 11 | areka 独自の名前を書き残す先 | `doc/COMPAT_ARCHITECTURE.md` の 8 節「沈黙ルール対応表」が areka 裁量・areka 拡張キー（例 `writing_mode`）の正本 | `currentghost.balloon.scope(ID).currentmonitor.dpi`・`scaling` の意味・2 つのイベントの送る場面と Reference4 をここに行で書く |

### 12.2 案の比べ（7 節の案を受けて）

| 案 | 受け手 | 倍率の変わり | 強み | 弱み・危険 | 採否 |
|---|---|---|---|---|---|
| A | `emo2_boot` の新しい受け口＋UI の相 | 毎フレーム `applied_ratio` を見比べる | 表示層を触らない | 台本のスレッド → UI → kanade の 2 段の跨ぎ・値をタグの時点で引けない・開発者の基準（毎フレーム比べる案は推さない）に反する | 不採用 |
| B | `areka-ghost` の兄弟の受け口 | 表示成立点で積み、相が取り出す | 受け手の道が最短・変わった時点のプッシュ | 表示層の公開の口が 1 つ増える | 中身を採る |
| C（採用） | B と同じ | B と同じ・切り替えは「装着ごとの起点」で見分ける | 先例の形に一番多く乗る・`placement/mod.rs` は 1 行・`frame/dpi.rs`・`emo2_boot/mod.rs`・`runtime_boot.rs` に触らない | ウェーブの表 ① に無いファイルを触る（設計の File Structure Plan に列挙して開発者の承認を仰ぐ） | 採用 |

### 12.3 設計の決め（design.md の「決めたこと」の根拠）

- **D1 返しのイベントは新しい種類で運ぶ**: 代わりの案＝`KanadeMsg::RaiseEvent` に出どころの欄を足す（送り手 9 か所に触る・許可の表の規則と混ざる）／`EventId::Sstp` を使い回す（出どころが `external,sstp` に固定）。採用＝`EventId::Property(String)`＋`KanadeMsg::PropertyEvent`。名前の検査は `areka_kanade::sstp::validate_notify`（要件 2.2・2.3 の規則とちょうど同じ）を受け口と kanade の腕の両方で使う（受け口は理由とタグを記録するため・kanade は信頼の境界のため）。
- **D2 定常でないときの返しのイベント**: `KanadeMsg::RaiseEvent` と同じく、定常（`Phase::Steady`・起動の挨拶の最中を含む）でだけ送り、ほかは `warn!` で捨てる（積まない）。終了の台本の応答で台本を差し替えると「終了イベントのトークは必ず終了で終わる」（2026-09-20 裁定）を崩すため。ぱすたは起動の挨拶と終了の中で `\![get,property]` を出さない（2.7 節）。
- **D3 k の変わりの印は表示成立点で積む**: 代わりの案＝毎フレーム比べる（開発者の基準に反する）／`refresh_scale` の戻り値を使う（「k が変わったが丸めた寸が同じ」も `None` で使えない・2.3 節）。採用＝`apply_show` の表示成立点で「装着ごとの起点」と（k・その k を出した DPI）の組が違えば `ScaleChange` を積み、毎フレームの相の最後の段（`scale_notice`）が取り出す。比べるのは k を書き換えるその場の 1 回だけで、毎フレームの走査はしない（`pending_resize` と同じ形）。
- **D4 切り替えの見分けは状態の形で**: 「装着ごとの起点」（`PresentTarget` の新しい欄）は `attach_target` が空にし、`apply_replace` は引き継がない。空からの最初の記録は表の書き込みだけでイベントを送らない（要件 5.7）。ゴーストの切り替え・読み直しは表示層ごと新しい（12.1 の 3）。1 フレーム遅らせる解・切り替えの印を別に配る解は取らない。
- **D5 `currentmonitor.dpi` は k と同じ時点の DPI**: 表示成立点で k を出したその窓の `DPI` の `dpi_x` を一緒に積む。整合待ち（`DpiSyncHold`）や見えていない間に窓の DPI だけが先に新しくなっても、答える値は描いた絵と食い違わない（要件 4.2）。ぱすたの「拡大率 × 96 ÷ DPI」は常に作者の基準どおりの 100 を保つ。
- **D6 百分率の文字**: 割り切れるときは整数、割り切れないときは小数第 3 位で四捨五入（0 から遠い側）して小数第 2 位まで書き、末尾の 0 は落とす（ただし小数点の後に少なくとも 1 桁は残す）。例: 96/96 → `100`・144/96 → `150`・120/144 → `83.33`・135/120 → `112.5`。SSP の桁は測っていない（10 節の 5・6）。areka は表示の倍率ができるまで作者の基準が 96 以外のときだけ小数が出る。測りで SSP の桁が分かったら、この 1 関数だけを直す（再確認の引き金に書く）。
- **D7 倍率の表の持ち主は `Emo2Wiring`**: 代わりの案＝World の資源に置く（ゴーストの切り替えで前のゴーストのスコープの値が残る）／`wire` のたびに作る（SHIORI だけの読み直しで表示層の値を失う）。採用＝`Emo2Wiring` の欄（ゴーストの起動ごとに新しい・SHIORI だけの読み直しでは同じ表）。
- **D8 `balloon.syncscale` は資源の変わりで読む**: 代わりの案＝`balloon_dontmove` と同じ運び方（`placement/mod.rs` に 5 行前後・約束を超える）。採用＝`DefaultPlacementBasis` が入れ直された巡で読む系 1 つ。読みは論理和（シェルの段かゴーストの段のどちらかが `true`）で、`balloon_dontmove` の「優先の順」と違う（要件 6.3）。
- **D9 areka 独自の名前の書き残し**: ukadoc に無い `currentghost.balloon.scope(ID).currentmonitor.dpi` は網羅台帳に行を足さず（要件 3.11）、`doc/COMPAT_ARCHITECTURE.md` の 8 節に行を足す。同じ表に `scaling` の意味（DPI 追従の倍率 × 表示の倍率）と 2 つのイベントの送る場面・Reference4 も書く。

### 12.4 合成のレンズ（design-synthesis）

- **一般化**: `scope(ID).scaling`・`balloon.scope(ID).scaling`・`scope(ID).currentmonitor.dpi`・`balloon.scope(ID).currentmonitor.dpi` の 4 つは「窓の種類 × 量（倍率か DPI）」の掛け合わせ＝読み手の型 1 つ（`ScopeScaleSource`）を 4 つの形で登録する。`OnShellScaling`・`OnBalloonScaling` も同じ記録から窓の種類で名前を選ぶだけ。`\![get,property]` の受け手は名前に縛らず、MCP の `get_property` と同じ `resolve_dotted_str` を通す（`property-query-channels` が後で同じ受け口と読み手を使える）。
- **作るか借りるか**: 外のクレートは足さない。値の口は ✅ `mcp-scope-rect-property` の `register_pull`、イベントの送出は `KanadeMsg::RaiseEvent`、名前の検査は `validate_notify`、受け口は `PropSetCueSink` の形、表示成立点の記録は `pending_resize` の形、descript の読みは `scoped_value`・`scope_prefixes`、選択子の読みは `scope_selector` を借りる。新しく作るのは kanade の 1 種類と表示層の記録の口だけ。
- **削ったもの**: UI の側の相を通す受け手（案 A）・毎フレームの見比べ・切り替えの印を別に配る仕組み・`syncscale` を構造の欄で運ぶ形・倍率の表をスコープの窓の印から組み直す系（`scope_rect` の `FrameFinalize` の系に当たるもの）は作らない。`syncscale` の値は持つだけで、読む下流（`shell-balloon-scale-setting`）が来るまで読み口を 1 つ（`is_synced`）に留める。

### 12.5 危険と手当て

- **表示成立点の外で k が変わる道が後から足される** → 再確認の引き金に「`applied` の更新点を足す変更」を書き、檻（emo-present の記録の檻）が表示成立点の記録を固定する。
- **`Emo2Wiring` が取り出されている最中に `rewire_runtime_consumers` が走る** → 今は 2 つの呼び手ともフレームの排他の系の外。`wire` は `Emo2Wiring` が無ければ空の表で登録して `debug!` を残す（LogSink の起動でも正しく「値なし」になる）。呼び手の場所が変わったら再確認の引き金。
- **ウェーブ C15 の兄弟との重なり** → ⑤ `emo-bake-log-overlap` は emo-present の `shell_target.rs`・`balloon.rs` と emo-compose の `fold.rs`、⑥ `popup-menu-extra-resources` は kanade の `schedule/resources.rs`・`menu/captions.rs` と網羅台帳 `shiori.toml` の別の 1 行。本 spec が触る emo-present の `presenter/` 以下・`presenter.rs`・`lib.rs`、emo-compose の `scale.rs`、kanade の `schedule/{events,types,mod,change}.rs` とファイルは重ならない（`shiori.toml` は別の行）。
- **SSP の小数の桁が後で分かる** → D6 の 1 関数だけを直す。

## 13. 設計討議の決め（2026-11-01・開発者「全部承認」）

- 議題 1（ウェーブの表 ① に無いファイル）: design.md の表のファイルを触ってよい。roadmap の ① の欄は `/kiro-complete` の roadmap の更新で書き直す（兄弟のブランチとの roadmap のぶつかりを避ける）。兄弟の触るファイルとの重なりは 0（`design-validation.md`）。
- 議題 2（最初の表示の後・定常の前の倍率の変わり）: 送り、kanade が `warn!(raise_event_not_steady)` で捨てるまま。起こるのは起動の SHIORI の往復の間に窓が画面をまたいだときだけ。表は書かれている。決定論のテストで固定する。
- 議題 3（隠れていたバルーンが出たときの `OnBalloonScaling`）: 2 つのイベントの応答は、話している最中なら今の台詞を差し替えない（`events::value_replaces_active_talk` に 2 語・`OnSecondChange` と同じ扱い・要件 5.11 を足した）。SSP では利用者の設定の変更でだけ送るので差し替えて困らないが、areka ではドラッグや表示の副作用として起きる。

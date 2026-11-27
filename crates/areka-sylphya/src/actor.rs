//! sylphya アクター: 電文 `SylphyaMsg`・効果 `Effect`・判断の自由な関数 `apply`・`spawn_sylphya`・`SylphyaPublisher`。
//!
//! ## 判断分岐は自由な関数へ寄せる
//!
//! [`apply`] は受信した [`SylphyaMsg`] を検査して **効果列 `Vec<Effect>`** を返す。
//! 住み場所への書き・reply 送信といった副作用は一切行わず、返す効果列だけが決定である。
//! 受信ループはこの効果列を実行する薄い配線に徹する。
//!
//! ## 純粋性とログの両立
//!
//! `apply` は判断点で `tracing` の warn!/debug! を発火する（断る名前・204 不在・parse 失敗の
//! 各縮退アーム）。`tracing` は subscriber 未登録なら no-op で、**返す効果列を変えない**。
//! 判断の「理由」を最もよく知るのは判断点＝`apply` なので、縮退記録はここで出す（二重ログにしない）。

use crate::asker::AskerId;
use crate::key::parse_dotted;
pub use crate::keytable::classify_set;
use crate::keytable::{ByName, Home, KEY_TABLE, Refusal, rule_of};
use crate::kvs::WriteOutcome;
use crate::persist::{PersistKey, PersistScope};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::mpsc::Sender;
use std::time::Duration;

/// ログ target（固定名）。
const LOG_TARGET: &str = "areka_sylphya::actor";

/// sylphya アクター宛メッセージ封筒（`areka-actor` envelope 規約）。
///
/// 変異側（供給者）と問い手が投函する全 variant を 1 つの enum で網羅する。判断分岐は
/// [`apply`] が担い、本 enum は搬送のみ。
pub enum SylphyaMsg {
    /// ①静的構成層の publish（ghost 結線が boot 時に投函・フラット/点付き両区画）。
    ///
    /// `flat` はゴースト相対＝`asker` の per-asker フラット区画へ着地。
    /// `dotted`（`baseware.*` 等）は大域点付き区画へ着地する。
    PublishStatic {
        /// 問い合わせ元（フラットの per-asker 着地先）。
        asker: AskerId,
        /// フラット語彙 `(名, 値)`（% 抜きの名・per-asker 区画へ）。
        flat: Vec<(String, String)>,
        /// 点付き語彙 `(正準 key, 値)`（大域点付き区画へ）。
        dotted: Vec<(String, String)>,
    },
    /// ①静的構成層のフラット名を `asker` の per-asker 区画から消す（読み直しで定義が消えた
    /// 名前＝起動時に `PublishStatic` が積まなかったのと同じ状態へ戻す）。名前は呼び手が選ぶ。
    RemoveFlat {
        /// 問い合わせ元（フラットの per-asker 区画）。
        asker: AskerId,
        /// 消すフラット名（% 抜き）。
        names: Vec<String>,
    },
    /// ④SHIORI 照会層の publish（`value=None` は 204/失敗＝不在の観測記録）。
    PublishShiori {
        /// 問い合わせ元（フラットの per-asker 着地先）。
        asker: AskerId,
        /// フラット名（% 抜き・例 `username`）。
        name: String,
        /// 照会値。`None` は 204/失敗（不在——既定値は書かない）。
        value: Option<String>,
    },
    /// 層を名指ししない名前での書き（SHIORI の SetProperty・台本から。ゴーストの層の揮発へ・投函して即返る）。
    Set {
        /// 問い合わせ元（host 書込の per-asker 着地先）。
        asker: AskerId,
        /// SET 対象 key（点付き文字列）。
        key: String,
        /// 書込値。
        value: String,
    },
    /// 永続 put（typed 4 key 族・write-through・fire-and-forget。失敗は受け口へ届く）。
    PersistPut {
        /// 永続スコープ（層別）。
        scope: PersistScope,
        /// typed 4 key 族 `(key, 値)`。
        entries: Vec<(PersistKey, String)>,
    },
    /// 永続の鍵を消す（鏡像と記憶の両方から・fire-and-forget。失敗は受け口へ届く）。
    PersistRemove {
        /// 永続スコープ（層別）。
        scope: PersistScope,
        /// 消す key。
        keys: Vec<PersistKey>,
    },
    /// 反映フェンス（投函済みメッセージの処理完了を同期観測。テスト決定論と boot prefetch の
    /// 順序保証に使用・epoch フェンス読みの予約シームの M1 形）。
    Barrier {
        /// フェンス到達の返信端。
        reply: areka_actor::ReplySender<()>,
    },
    /// 停止規約（即時停止・`areka-actor` 停止規約）。
    Close,
    /// 名前での読み（`layer` が `None` なら重ね読み・`home` は鍵の表が決めた住み場所）。
    Get {
        /// 問い手。
        asker: AskerId,
        /// 鍵（正規名）。
        key: String,
        /// 名指しの層。
        layer: Option<PersistScope>,
        /// 住み場所。
        home: Home,
        /// 値の返信端。
        reply: areka_actor::ReplySender<Option<String>>,
    },
    /// 平らな名前の読み。
    GetFlat {
        /// 問い手。
        asker: AskerId,
        /// 平らな名前（% 抜き）。
        name: String,
        /// 値の返信端。
        reply: areka_actor::ReplySender<Option<String>>,
    },
    /// トークの写し（問い手の平らな名前の全部）。
    TalkSnapshot {
        /// 問い手。
        asker: AskerId,
        /// 写しの返信端。
        reply: areka_actor::ReplySender<BTreeMap<String, String>>,
    },
    /// 層を名指しした書き・消し。
    Write {
        /// 書き手の種類。
        writer: Writer,
        /// 問い手。
        asker: AskerId,
        /// 書く層。
        layer: PersistScope,
        /// 書く（鍵, 値）。
        puts: Vec<(String, String)>,
        /// 消す鍵。
        removes: Vec<String>,
        /// 結果の返信端。
        reply: areka_actor::ReplySender<WriteOutcome>,
    },
    /// 層の置き場の付け替え（列に入れず、届いた時点で当てる）。
    SetLayerRoot {
        /// 付け替える層（シェルかバルーン）。
        layer: PersistScope,
        /// 新しい置き場（`None` は置き場の無い層）。
        root: Option<PathBuf>,
    },
}

/// 書き手の種類。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Writer {
    /// 外から来た名前で書く（鍵の表が断りうる）。
    ByName,
    /// areka の中の決まった書き手（鍵の表の「名前で書く」を見ない）。
    Fixed,
}

/// SET 分類の 3 分岐。
///
/// [`classify_set`] が key を検査して返す。各分岐は [`apply`] の Set アームで
/// 対応する [`Effect`] へ写る。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SetClass {
    /// SET 有効群の正準語彙（[`crate::vocab::dotted::SET_EFFECTIVE`] 登録）→ 書込なし・warn＋記録のみ。
    RuntimeCommand,
    /// 正準語彙外の自由 dotted key → asker 別 host 区画への反映。
    StoreWrite,
    /// SET 無効な正準語彙 → 書込なし・呼出は Ok（正典沈黙の areka 裁量・対応表記録）。
    NotSettable,
}

/// [`apply`] が返す純粋な効果（判断の出力・変異意図）。
///
/// 効果は I/O を含まない純データで、`apply` の決定論檻はこの列の同値性を突く。
/// 区画を名指す 5 つの効果の行き先: ゴースト別の平らな区画＝主の DB の `flat`、全体の点付きの
/// 区画＝アプリの層の揮発、ゴースト別の点付きの区画＝ゴーストの層の揮発。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Effect {
    /// フラット per-asker 区画へ書く（`PublishStatic.flat`・`PublishShiori{Some}`）。
    SetFlatPerAsker {
        /// 着地先 asker。
        asker: AskerId,
        /// フラット名（% 抜き）。
        name: String,
        /// 値。
        value: String,
    },
    /// フラット per-asker 区画から消す（`RemoveFlat`）。
    RemoveFlatPerAsker {
        /// 対象 asker。
        asker: AskerId,
        /// フラット名（% 抜き）。
        name: String,
    },
    /// 大域点付き区画へ書く（`PublishStatic.dotted`＝`baseware.*`）。
    SetDottedGlobal {
        /// 正準 key 文字列。
        key: String,
        /// 値。
        value: String,
    },
    /// 大域点付き区画から消す（無い key は何もしない）。
    RemoveDottedGlobal {
        /// 正準 key 文字列。
        key: String,
    },
    /// asker 別 host 点付き区画へ書く（SET StoreWrite＝自由 key の受け皿）。
    SetDottedPerAsker {
        /// 着地先 asker。
        asker: AskerId,
        /// 正準 key 文字列（parse 不能時は生 key）。
        key: String,
        /// 値。
        value: String,
    },
    /// 不在の観測記録（`PublishShiori{None}`＝204/失敗）。鏡像へは書かない（既定値は sakura 所有）。
    RecordAbsentFlat {
        /// 対象 asker。
        asker: AskerId,
        /// フラット名（% 抜き）。
        name: String,
    },
    /// 永続 put（write-through の保存側）。受信ループが [`crate::persist::save_scope`] を実行する。
    PersistSave {
        /// スコープ。
        scope: PersistScope,
        /// typed 4 key 族 `(key, 値)`。
        entries: Vec<(PersistKey, String)>,
    },
    /// 永続の鍵を消す（記憶の側）。実行は [`crate::persist::remove_scope_keys`] を呼ぶ。
    PersistRemove {
        /// スコープ。
        scope: PersistScope,
        /// 消す key。
        keys: Vec<PersistKey>,
    },
    /// SET 運行コマンド（[`SetClass::RuntimeCommand`]・warn 済み・書込なし）。
    RuntimeCommandReserved {
        /// 対象 asker。
        asker: AskerId,
        /// SET key。
        key: String,
        /// 値。
        value: String,
    },
    /// SET 無効な正準語彙（[`SetClass::NotSettable`]・warn 済み・書込なし・呼出は Ok）。
    NotSettable {
        /// 対象 asker。
        asker: AskerId,
        /// SET key。
        key: String,
        /// 反映されない値（観測用に保持・書込は行わない）。
        value: String,
    },
    /// 層を名指しした書き・消し（鍵は正準形・住み場所は受信ループが鍵の表で決める）。
    Write {
        /// 問い手。
        asker: AskerId,
        /// 書く層。
        layer: PersistScope,
        /// 書く（鍵, 値）。
        puts: Vec<(String, String)>,
        /// 消す鍵。
        removes: Vec<String>,
    },
    /// 名前では書けない鍵への名前での書き（warn 済み・書込なし・返事は断った）。
    Refused {
        /// 問い手。
        asker: AskerId,
        /// 断った鍵。
        key: String,
        /// 断る理由。
        refusal: Refusal,
    },
    /// 層の置き場の付け替え。
    SetLayerRoot {
        /// 付け替える層。
        layer: PersistScope,
        /// 新しい置き場。
        root: Option<PathBuf>,
    },
    /// 反映フェンス（受信ループが処理完了後に reply を返す）。
    Barrier,
    /// 停止（受信ループを抜ける）。
    Stop,
}

/// メッセージを検査して効果列を算出する（純関数・no I/O・決定論）。
///
/// 返す `Vec<Effect>` だけが決定である。判断点の縮退（断る名前・204 不在・parse 失敗）は
/// `tracing` で warn!/debug! を発火するが、返す効果列は同一入力で常に同一。読みの電文は効果を
/// 出さない（受信ループが電文から答える）。
pub fn apply(msg: &SylphyaMsg) -> Vec<Effect> {
    match msg {
        SylphyaMsg::PublishStatic {
            asker,
            flat,
            dotted,
        } => {
            let mut effects = Vec::with_capacity(flat.len() + dotted.len());
            for (name, value) in flat {
                effects.push(Effect::SetFlatPerAsker {
                    asker: asker.clone(),
                    name: name.clone(),
                    value: value.clone(),
                });
            }
            for (key, value) in dotted {
                effects.push(Effect::SetDottedGlobal {
                    key: canonicalize_or_raw(key),
                    value: value.clone(),
                });
            }
            effects
        }
        SylphyaMsg::RemoveFlat { asker, names } => names
            .iter()
            .map(|name| Effect::RemoveFlatPerAsker {
                asker: asker.clone(),
                name: name.clone(),
            })
            .collect(),
        SylphyaMsg::PublishShiori { asker, name, value } => match value {
            Some(v) => vec![Effect::SetFlatPerAsker {
                asker: asker.clone(),
                name: name.clone(),
                value: v.clone(),
            }],
            None => {
                // 204/失敗 → 不在記録のみ（既定値は sakura の唯一定義点・書かない）。
                tracing::debug!(
                    target: LOG_TARGET,
                    asker = asker.as_str(),
                    name = %name,
                    "shiori resource absent (204/failed); recording absence without default"
                );
                vec![Effect::RecordAbsentFlat {
                    asker: asker.clone(),
                    name: name.clone(),
                }]
            }
        },
        SylphyaMsg::Set { asker, key, value } => match rule_of(KEY_TABLE, key).by_name {
            ByName::Refused(refusal) => {
                warn_refused(asker, key, refusal);
                let (asker, key, value) = (asker.clone(), key.clone(), value.clone());
                vec![match refusal {
                    Refusal::RuntimeCommand => Effect::RuntimeCommandReserved { asker, key, value },
                    Refusal::NotSettable | Refusal::FixedWriterOnly => {
                        Effect::NotSettable { asker, key, value }
                    }
                }]
            }
            ByName::Writable => {
                let canon = canonical_for_write(asker, key);
                tracing::debug!(
                    target: LOG_TARGET,
                    asker = asker.as_str(),
                    key = %canon,
                    "SET store-write to host dotted region"
                );
                vec![Effect::SetDottedPerAsker {
                    asker: asker.clone(),
                    key: canon,
                    value: value.clone(),
                }]
            }
        },
        SylphyaMsg::Write {
            writer,
            asker,
            layer,
            puts,
            removes,
            ..
        } => {
            if *writer == Writer::ByName {
                let refused: Vec<Effect> = puts
                    .iter()
                    .map(|(key, _)| key)
                    .chain(removes)
                    .filter_map(|key| match rule_of(KEY_TABLE, key).by_name {
                        ByName::Writable => None,
                        ByName::Refused(refusal) => {
                            warn_refused(asker, key, refusal);
                            Some(Effect::Refused {
                                asker: asker.clone(),
                                key: key.clone(),
                                refusal,
                            })
                        }
                    })
                    .collect();
                if !refused.is_empty() {
                    return refused;
                }
            }
            vec![Effect::Write {
                asker: asker.clone(),
                layer: *layer,
                puts: puts
                    .iter()
                    .map(|(key, value)| (canonical_for_write(asker, key), value.clone()))
                    .collect(),
                removes: removes
                    .iter()
                    .map(|key| canonical_for_write(asker, key))
                    .collect(),
            }]
        }
        SylphyaMsg::PersistPut { scope, entries } => vec![Effect::PersistSave {
            scope: *scope,
            entries: entries.clone(),
        }],
        SylphyaMsg::PersistRemove { scope, keys } => vec![Effect::PersistRemove {
            scope: *scope,
            keys: keys.clone(),
        }],
        SylphyaMsg::SetLayerRoot { layer, root } => vec![Effect::SetLayerRoot {
            layer: *layer,
            root: root.clone(),
        }],
        SylphyaMsg::Get { .. } | SylphyaMsg::GetFlat { .. } | SylphyaMsg::TalkSnapshot { .. } => {
            Vec::new()
        }
        SylphyaMsg::Barrier { .. } => vec![Effect::Barrier],
        SylphyaMsg::Close => vec![Effect::Stop],
    }
}

/// 名前では書けない鍵への名前での書きを warn で残す（書込なし・呼び出しは成功のまま）。
fn warn_refused(asker: &AskerId, key: &str, refusal: Refusal) {
    let asker = asker.as_str();
    match refusal {
        Refusal::RuntimeCommand => tracing::warn!(
            target: LOG_TARGET,
            asker,
            key,
            "SET on runtime-command vocab: reserved seam, not wired in M1 (no write)"
        ),
        Refusal::NotSettable => tracing::warn!(
            target: LOG_TARGET,
            asker,
            key,
            "SET on non-settable canonical vocab: warn and drop (call still Ok, no write)"
        ),
        Refusal::FixedWriterOnly => tracing::warn!(
            target: LOG_TARGET,
            asker,
            key,
            "SET on a name only fixed writers may write: warn and drop (call still Ok, no write)"
        ),
    }
}

/// 名前で書く鍵を正準形へ（parse 不能は渡された綴りのままの自由な鍵として warn 記録）。
fn canonical_for_write(asker: &AskerId, key: &str) -> String {
    match parse_dotted(key) {
        Ok(path) => path.to_canonical_string(),
        Err(e) => {
            tracing::warn!(
                target: LOG_TARGET,
                asker = asker.as_str(),
                key = %key,
                error = %e,
                "SET free key failed to parse; storing under raw key (tolerant)"
            );
            key.to_string()
        }
    }
}

/// 点付き key を正準文字列へ（parse 不能は生 key を retain し warn 記録・無音失敗なし）。
fn canonicalize_or_raw(key: &str) -> String {
    match parse_dotted(key) {
        Ok(path) => path.to_canonical_string(),
        Err(e) => {
            tracing::warn!(
                target: LOG_TARGET,
                key = %key,
                error = %e,
                "static dotted key failed to parse; using raw key (tolerant)"
            );
            key.to_string()
        }
    }
}

// ============================================================================
// 送り口（受信の輪は `guardian.rs`）
// ============================================================================
//
// 停止した番人への投函は panic せず、fire-and-forget は warn で捨て、反映待ちは
// `ReplyError::Dropped` を返す（送信端で停止を観測できる）。

#[cfg(test)]
pub(crate) use crate::guardian::spawn_parts;
pub use crate::guardian::{SylphyaInit, SylphyaParts, spawn_sylphya, spawn_sylphya_with_failures};

/// 変異投函の送信端（`areka-actor` envelope 規約）。
///
/// `Clone` は内部 `Sender` の clone のみ——複数供給者（ghost 結線・kanade・SET sink）が
/// 同一 inbox を安価に共有する。全メソッドは投函のみで鏡像を直接触らない（single-writer は
/// アクターが担保）。アクター死亡後の投函は **panic せず** 縮退する（fire-and-forget は warn＋
/// 破棄・[`barrier`](SylphyaPublisher::barrier) は `Err`）。
#[derive(Clone)]
pub struct SylphyaPublisher {
    tx: Sender<SylphyaMsg>,
}

impl SylphyaPublisher {
    pub(crate) fn new(tx: Sender<SylphyaMsg>) -> Self {
        SylphyaPublisher { tx }
    }

    /// ①静的構成層を publish する（フラットは per-asker・点付きは大域区画へ着地）。
    pub fn publish_static(
        &self,
        asker: AskerId,
        flat: Vec<(String, String)>,
        dotted: Vec<(String, String)>,
    ) {
        self.send(SylphyaMsg::PublishStatic {
            asker,
            flat,
            dotted,
        });
    }

    /// ①静的構成層のフラット名を `asker` の区画から消す（無い名前は何もしない）。
    pub fn remove_flat(&self, asker: AskerId, names: Vec<String>) {
        self.send(SylphyaMsg::RemoveFlat { asker, names });
    }

    /// ④SHIORI 照会層を publish する（`value=None` は 204/失敗＝不在の観測記録）。
    pub fn publish_shiori(&self, asker: AskerId, name: String, value: Option<String>) {
        self.send(SylphyaMsg::PublishShiori { asker, name, value });
    }

    /// SET コマンドを投函する（分類・中継・host 区画書込はアクターの領分）。
    pub fn set(&self, asker: AskerId, key: String, value: String) {
        self.send(SylphyaMsg::Set { asker, key, value });
    }

    /// 永続 put を投函する（write-through・fire-and-forget）。
    pub fn persist_put(&self, scope: PersistScope, entries: Vec<(PersistKey, String)>) {
        self.send(SylphyaMsg::PersistPut { scope, entries });
    }

    /// 永続の鍵を消す投函（鏡像と記憶の両方から・fire-and-forget）。
    pub fn persist_remove(&self, scope: PersistScope, keys: Vec<PersistKey>) {
        self.send(SylphyaMsg::PersistRemove { scope, keys });
    }

    /// 反映フェンス: 投函→処理完了を待つ（期限なし。期限が要るなら [`barrier_within`](Self::barrier_within)）。
    ///
    /// 復帰時、それ以前に同一送信端から投函した全メッセージは鏡像へ反映済み（mpsc FIFO＋
    /// 直列処理）。アクター死亡（`Close`／panic 後）で投函できない、または reply 端が応答せず
    /// drop された場合は [`ReplyError`](areka_actor::ReplyError) を返す（送信端で死亡観測可能）。
    pub fn barrier(&self) -> Result<(), areka_actor::ReplyError> {
        self.post_barrier()?.recv()
    }

    /// 反映フェンス（期限つき）。期限を越えたら [`ReplyError::Timeout`](areka_actor::ReplyError)
    /// （error! を 1 件）。死亡の扱いは [`barrier`](Self::barrier) と同じ。
    pub fn barrier_within(&self, timeout: Duration) -> Result<(), areka_actor::ReplyError> {
        let result = self.post_barrier()?.recv_timeout(timeout);
        if matches!(result, Err(areka_actor::ReplyError::Timeout)) {
            tracing::error!(target: LOG_TARGET, ?timeout, "sylphya barrier timed out; writer still busy");
        }
        result
    }

    /// 反映フェンスの投函（期限なし／期限つきの共通部分）。
    fn post_barrier(&self) -> Result<areka_actor::ReplyReceiver<()>, areka_actor::ReplyError> {
        let (reply, rx) = areka_actor::reply_channel::<()>();
        if self.tx.send(SylphyaMsg::Barrier { reply }).is_err() {
            // アクター死亡 → 投函不能。warn＋Dropped（reader は最終鏡像で継続・表示系を殺さない）。
            tracing::warn!(
                target: LOG_TARGET,
                "sylphya actor stopped; barrier could not be posted (SendError → ReplyError::Dropped)"
            );
            return Err(areka_actor::ReplyError::Dropped);
        }
        Ok(rx)
    }

    /// 停止規約: `Close` を投函する（正典終了経路）。
    ///
    /// 番人は列に残った書きを 1 回ずつ待たずに試みてから止まる（受信の箱の積み残しは破棄）。
    /// 停止済みへの再送は warn＋縮退（panic しない）。停止完了は [`SylphyaParts::handle`] の join で待つ。
    pub fn close(&self) {
        self.send(SylphyaMsg::Close);
    }

    /// fire-and-forget 投函の共通経路。アクター死亡時は `SendError` を **warn 記録して縮退**
    /// （panic しない）。
    pub(crate) fn send(&self, msg: SylphyaMsg) {
        if self.tx.send(msg).is_err() {
            tracing::warn!(
                target: LOG_TARGET,
                "sylphya actor stopped; message dropped (SendError, degrading without panic)"
            );
        }
    }
}

#[cfg(test)]
#[path = "actor_tests.rs"]
mod tests;

/// 名前の畳みの書き込みの仕分けの檻。
#[cfg(test)]
#[path = "actor_case_fold_tests.rs"]
mod case_fold_tests;

/// 書き手のアクターの失敗の受け口。
#[cfg(test)]
#[path = "actor_failure_sink_tests.rs"]
mod failure_sink_tests;

/// 期限つきの反映待ちの檻。
#[cfg(test)]
#[path = "actor_barrier_within_tests.rs"]
mod barrier_within_tests;

/// アクター統合 決定論檻: spawn→publish→barrier→read の配線・barrier フェンス・
/// アクター死亡観測（SendError／ReplyError／join panic）・reader 無ブロック・write-through 投影。
///
/// 全て純 x64（偽物の置き場の口 `MemBackend`・実 FS なし・時計なし）。同期は **barrier／join** で取り、
/// `thread::sleep` を一切使わない。barrier が「それ以前の投函が反映済み」を保証するので、
/// read はレースなく確定する。
#[cfg(test)]
#[path = "actor_actor_integration_tests.rs"]
mod actor_integration_tests;

/// アクター 決定論単体テスト群（criterion-mapped cage）。
///
/// `apply` の SET 分類と publish 後の epoch 単調増加、アクター系・SET 系の判断分岐を決定論檻として
/// 集約する。全て x64 純粋（偽物の置き場の口 `MemBackend`・実 FS/実時計/実 SHIORI なし）で、同期は **barrier／join** のみ
/// （`thread::sleep` 皆無・スレッド配線が flake しない）。既存の強い檻は重複させず、監査で判明
/// した 2 つの空隙を補う:
///
/// - **(A) 鏡像 epoch の「アクター publish 経路」単調増加**: 純核単調増加
///   （`mirror::tests::epoch_monotonic_on_successors`）・`SharedMirror` 直 publish
///   （`mirror::tests::predecessor_image_unchanged_after_publish`）は既存だが、**アクターが
///   `apply` の効果列を実行して行う publish swap** で epoch が単調増加することを突く檻は無い。
///   reader は epoch を露出しない（epoch はフェンス予約シームで読み API に非露出）ため、
///   本檻は本番と同じ組み立て（[`spawn_parts`]）を通り [`SharedMirror`] の clone を保持し
///   `SharedMirror::load().epoch` で直接観測する。
/// - **(B) アクター死亡後 send の WARN 記録**（無音失敗禁止）: 死亡後投函が panic せず
///   縮退することは既存檻（[`actor_integration_tests::actor_death_via_close_makes_sends_observable_and_reader_continues`]）
///   が突くが、その縮退が **無音でない**（WARN を出す）ことは檻化されていない。本檻は
///   [`crate::test_log_capture::capture`]（共有機構への委譲・決定論）で fire-and-forget send と
///   barrier の死亡縮退 WARN を捕捉する。
///
/// ## 判断分岐 → 檻 の対応（criterion → test）
///
/// - **SET 分類 3 パターン**: `super::tests::{classify_set_effective_key_is_runtime_command,
///   classify_set_free_dotted_key_is_store_write, classify_set_canonical_non_effective_is_not_settable,
///   classify_set_unparseable_key_is_store_write}` ＋
///   apply 面 `super::tests::{apply_set_effective_emits_runtime_command_reserved,
///   apply_set_free_emits_host_store_write, apply_set_not_settable_emits_no_write}`。
/// - **epoch 単調増加（アクター publish 経路）**: 本群
///   [`epoch_increments_monotonically_through_actor_publishes`]（初出・観測は保持した
///   `SharedMirror::load().epoch`）。
/// - **Barrier フェンス決定論**: `super::actor_integration_tests::{barrier_fences_all_prior_messages_last_write_wins,
///   spawn_publish_static_barrier_then_read_sees_value, spawn_publish_barrier_read_is_deterministic_over_iterations}`。
/// - **アクター死亡縮退**: 非 panic 送信＋barrier Err＋reader 継続は
///   `super::actor_integration_tests::actor_death_via_close_makes_sends_observable_and_reader_continues`、
///   join panic 検出（[`areka_actor::ActorError::Panicked`]）は
///   `super::actor_integration_tests::actor_panic_is_detected_by_join_and_reader_continues`。
///   死亡後 send／barrier の **WARN 記録**（無音失敗禁止の檻）は本群
///   [`send_after_death_logs_warn_not_silent`]（初出・`test_log_capture`）。
#[cfg(test)]
#[path = "actor_actor_criteria_cage.rs"]
mod actor_criteria_cage;

//! sylphya アクター: `SylphyaMsg` / `SylphyaCore`（純関数中核）/ `spawn_sylphya` / `SylphyaPublisher`。
//!
//! 本タスク（5.1）が実体化するのは **メッセージ封筒 [`SylphyaMsg`]**・**純関数中核
//! [`SylphyaCore`]**（判断分岐の檻）・**SET 分類器 [`classify_set`]**・**運行コマンド配送先
//! シーム [`RuntimeCommandSink`]**（M1 は型予約のみ・未配線）である。アクター spawn／鏡像
//! swap／永続 IO／barrier reply の実配線は薄い受信ループ（Task 5.2）の領分であり、本モジュールは
//! その配線が実行する **効果列 [`Effect`]** の型と、それを算出する純関数のみを提供する。
//!
//! ## 判断分岐は純関数中核へ寄せる（記憶知見「檻に入れるのは判断分岐のみ・配線は再テストしない」）
//!
//! [`SylphyaCore::apply`] は受信した [`SylphyaMsg`] を検査して **効果列 `Vec<Effect>`** を返す。
//! 鏡像の変異・永続保存・reply 送信・sink 配送といった I/O／副作用は一切行わず、返す効果列だけが
//! 決定である。受信ループ（5.2）はこの効果列を copy-on-write 後継鏡像へ適用し・永続保存を実行し・
//! reply を返し・停止する薄い配線に徹する。この分離により決定論檻は `apply` の返す `Vec<Effect>` を
//! 突くだけで全判断分岐（SET 3 分類・204 不在・publish 着地区画・永続投影）を網羅できる。
//!
//! ## 純粋性とログの両立（R8.1 無音失敗禁止との調停）
//!
//! `apply` は判断点で `tracing` の warn!/debug! を発火する（RuntimeCommand 予約・NotSettable・
//! 204 不在・parse 失敗の各縮退アーム）。`tracing` は subscriber 未登録なら no-op で、**返す効果列を
//! 変えない**。決定論檻が突くのは「返された `Vec<Effect>` が同一入力で同一であること」であり、
//! ログ発火はこの不変を破らない（同一入力なら同一ログ・同一効果列）。判断の「理由」を最もよく知る
//! のは判断点＝`apply` なので、縮退記録はここで出す（無音失敗禁止を檻の内側で満たす）。効果列は
//! 純粋な変異意図の搬送に徹し、5.2 はそれを実行する（二重ログにしない）。

use crate::asker::AskerId;
use crate::key::parse_dotted;
use crate::mirror::{MirrorImage, SharedMirror};
use crate::persist::{
    PersistIo, PersistKey, PersistOutcome, PersistScope, ScopeRoots, load_scope, save_scope,
};
use crate::reader::SylphyaReader;
use crate::vocab::dotted::{DOTTED_ROOTS, GENERIC_PROP_NAMES, SET_EFFECTIVE};
use std::sync::Arc;
use std::sync::mpsc::Sender;

/// ログ target（steering: areka-log-first-no-silent-failure・design Monitoring 固定名）。
const LOG_TARGET: &str = "areka_sylphya::actor";

/// sylphya アクター宛メッセージ封筒（`areka-actor` envelope 規約・design Service Interface）。
///
/// 変異側（供給者）が投函する全 variant を 1 つの enum で網羅する。判断分岐は
/// [`SylphyaCore::apply`] が担い、本 enum は搬送のみ。
pub enum SylphyaMsg {
    /// ①静的構成層の publish（ghost 結線が boot 時に投函・フラット/点付き両区画）。
    ///
    /// `flat` はゴースト相対＝`asker` の per-asker フラット区画へ着地（設計討議 #1）。
    /// `dotted`（`baseware.*` 等）は大域点付き区画へ着地する（design State Management:
    /// 「dotted_global — 点付き語彙〔system.* 等〕の大域区画」）。
    PublishStatic {
        /// 問い合わせ元（フラットの per-asker 着地先）。
        asker: AskerId,
        /// フラット語彙 `(名, 値)`（% 抜きの名・per-asker 区画へ）。
        flat: Vec<(String, String)>,
        /// 点付き語彙 `(正準 key, 値)`（大域点付き区画へ）。
        dotted: Vec<(String, String)>,
    },
    /// ④SHIORI 照会層の publish（`value=None` は 204/失敗＝不在の観測記録）。
    PublishShiori {
        /// 問い合わせ元（フラットの per-asker 着地先）。
        asker: AskerId,
        /// フラット名（% 抜き・例 `username`）。
        name: String,
        /// 照会値。`None` は 204/失敗（不在——既定値は書かない・R4.2）。
        value: Option<String>,
    },
    /// SET コマンド（分類・中継・host 区画書込。即応答不要＝投函して即返る）。
    Set {
        /// 問い合わせ元（host 書込の per-asker 着地先）。
        asker: AskerId,
        /// SET 対象 key（点付き文字列）。
        key: String,
        /// 書込値。
        value: String,
    },
    /// 永続 put（typed 4 key 族・write-through・reply は任意で結果観測可）。
    PersistPut {
        /// 永続スコープ（層別）。
        scope: PersistScope,
        /// typed 4 key 族 `(key, 値)`。
        entries: Vec<(PersistKey, String)>,
        /// 保存結果の返信端（任意・`None` は fire-and-forget）。
        reply: Option<areka_actor::ReplySender<crate::persist::PersistOutcome>>,
    },
    /// 反映フェンス（投函済みメッセージの処理完了を同期観測。テスト決定論と boot prefetch の
    /// 順序保証に使用・epoch フェンス読みの予約シームの M1 形）。
    Barrier {
        /// フェンス到達の返信端。
        reply: areka_actor::ReplySender<()>,
    },
    /// 停止規約（即時停止・`areka-actor` 停止規約）。
    Close,
}

/// 運行コマンド書込（SET＝`\s[]` 等価系）の配送先シーム（M1 は型予約のみ・未登録）。
///
/// [`SetClass::RuntimeCommand`] に分類された SET を実際のランタイム（seriko 等）へ橋渡しする
/// 差替シーム。M1 では sink 登録を行わず、[`SylphyaCore::apply`] は
/// [`Effect::RuntimeCommandReserved`] を返して warn 記録するのみで実配送しない（R3.4）。
pub trait RuntimeCommandSink: Send {
    /// 運行コマンドを配送する（M1 未配線・M2 で seriko 等へ橋渡し）。
    fn dispatch(&self, asker: &AskerId, key: &str, value: &str);
}

/// SET 分類の 3 分岐（design「sylphya アクター」Responsibilities & Constraints・R3.4）。
///
/// [`classify_set`] が key を検査して返す。各分岐は [`SylphyaCore::apply`] の Set アームで
/// 対応する [`Effect`] へ写る。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SetClass {
    /// SET 有効群の正準語彙（[`SET_EFFECTIVE`] 登録）→ 運行コマンド書込（M1 未配線・warn＋記録のみ）。
    RuntimeCommand,
    /// 正準語彙外の自由 dotted key → asker 別 host 区画への反映（`ShioriHostSink` SetProperty の受け皿）。
    StoreWrite,
    /// SET 無効な正準語彙 → 書込なし・呼出は Ok（正典沈黙の areka 裁量・対応表記録）。
    NotSettable,
}

/// SET 対象 key を 3 分岐へ分類する純関数（no I/O・決定論・R3.4）。
///
/// ## 「正準語彙」の判定述語
///
/// key が次のいずれかを満たすとき **正準語彙** とみなす:
///
/// 1. [`SET_EFFECTIVE`] に厳密一致（SET 有効群）→ [`SetClass::RuntimeCommand`]（最優先判定）。
/// 2. （1 でなく）[`parse_dotted`] が成功し、**根セグメント名が [`DOTTED_ROOTS`] に属する**、
///    または **葉セグメント名が [`GENERIC_PROP_NAMES`] に属する** → 正準語彙かつ SET 無効
///    → [`SetClass::NotSettable`]。
/// 3. 上記いずれでもない（自由/カスタム dotted key、または parse 不能 key）→ [`SetClass::StoreWrite`]。
///
/// parse 不能 key は正準語彙表に載り得ない（表は全て parse 可能）ため、決定論的に自由 key
/// （StoreWrite）へ落とす（無音失敗なし——[`SylphyaCore::apply`] の StoreWrite アームが正準化失敗を
/// warn 記録し、生 key を host 区画へ retain する）。SET 有効群を最優先で判定するため、
/// `menu`/`sakura.bind.menu` 等（[`SET_EFFECTIVE`] と [`GENERIC_PROP_NAMES`] の双方に載る名）は
/// RuntimeCommand に確定する（二重帰属の曖昧さを排除）。
pub fn classify_set(key: &str) -> SetClass {
    // 1. SET 有効群（厳密一致・最優先）→ RuntimeCommand。
    if SET_EFFECTIVE.iter().any(|(k, _)| *k == key) {
        return SetClass::RuntimeCommand;
    }
    // 2. 正準語彙（根 ∈ DOTTED_ROOTS ∨ 葉 ∈ GENERIC_PROP_NAMES）だが SET 無効 → NotSettable。
    if is_canonical_vocab(key) {
        return SetClass::NotSettable;
    }
    // 3. 自由/カスタム dotted key（parse 不能を含む）→ StoreWrite。
    SetClass::StoreWrite
}

/// key が正準語彙か（根 ∈ [`DOTTED_ROOTS`] ∨ 葉 ∈ [`GENERIC_PROP_NAMES`]）。parse 不能は非正準。
fn is_canonical_vocab(key: &str) -> bool {
    match parse_dotted(key) {
        Ok(path) => {
            let root_canonical = path
                .segs
                .first()
                .is_some_and(|s| DOTTED_ROOTS.contains(&s.name.as_str()));
            let leaf_canonical = path
                .segs
                .last()
                .is_some_and(|s| GENERIC_PROP_NAMES.contains(&s.name.as_str()));
            root_canonical || leaf_canonical
        }
        // parse 不能 → 正準語彙表に載り得ない → 非正準（StoreWrite へ）。
        Err(_) => false,
    }
}

/// [`SylphyaCore::apply`] が返す純粋な効果（判断の出力・変異意図）。
///
/// 受信ループ（Task 5.2）がこれを copy-on-write 後継鏡像へ適用し・永続保存を実行し・停止する。
/// 効果は I/O を含まない純データで、`apply` の決定論檻はこの列の同値性を突く。
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
    /// 大域点付き区画へ書く（`PublishStatic.dotted`＝`baseware.*`・永続投影の `areka.*`）。
    SetDottedGlobal {
        /// 正準 key 文字列。
        key: String,
        /// 値。
        value: String,
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
    /// 不在の観測記録（`PublishShiori{None}`＝204/失敗）。鏡像へは書かない（既定値は sakura 所有・R4.2）。
    RecordAbsentFlat {
        /// 対象 asker。
        asker: AskerId,
        /// フラット名（% 抜き）。
        name: String,
    },
    /// 永続 put（write-through の保存側）。5.2 が [`crate::persist::save_scope`] を実行する。
    PersistSave {
        /// スコープ。
        scope: PersistScope,
        /// typed 4 key 族 `(key, 値)`。
        entries: Vec<(PersistKey, String)>,
    },
    /// SET 運行コマンド（[`SetClass::RuntimeCommand`]・M1 sink 未登録→配送なし・warn 済み・書込なし）。
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
    /// 反映フェンス（5.2 が処理完了後に reply を返す）。
    Barrier,
    /// 停止（5.2 が受信ループを抜ける）。
    Stop,
}

/// 判断分岐の純関数中核（design「sylphya アクター」Responsibilities）。
///
/// [`apply`](SylphyaCore::apply) が [`SylphyaMsg`] → `Vec<Effect>` を算出する。状態を持たない
/// （分類は const 台帳のみ参照）ゼロサイズ型で、I/O・チャネル・ロック・時計を含まない。
#[derive(Clone, Copy, Debug, Default)]
pub struct SylphyaCore;

impl SylphyaCore {
    /// 中核を構築する（ゼロサイズ・状態なし）。
    pub fn new() -> Self {
        SylphyaCore
    }

    /// メッセージを検査して効果列を算出する（純関数・no I/O・決定論・R2.5/R2.7/R8.3）。
    ///
    /// 返す `Vec<Effect>` だけが決定である。鏡像 swap・永続保存・reply 送信・sink 配送は 5.2 の配線が
    /// 効果列を実行する形で行う。判断点の縮退（RuntimeCommand 予約・NotSettable・204 不在・parse
    /// 失敗）は `tracing` で warn!/debug! を発火するが、返す効果列は同一入力で常に同一（ログは効果列を
    /// 変えない・R8.1 を檻の内側で満たす）。
    pub fn apply(&self, msg: &SylphyaMsg) -> Vec<Effect> {
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
            SylphyaMsg::PublishShiori { asker, name, value } => match value {
                Some(v) => vec![Effect::SetFlatPerAsker {
                    asker: asker.clone(),
                    name: name.clone(),
                    value: v.clone(),
                }],
                None => {
                    // 204/失敗 → 不在記録のみ（既定値は sakura の唯一定義点・鏡像へ書かない・R4.2）。
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
            SylphyaMsg::Set { asker, key, value } => match classify_set(key) {
                SetClass::RuntimeCommand => {
                    // M1: sink 未登録 → 予約シームのみ・実書込なし（R3.4）。
                    tracing::warn!(
                        target: LOG_TARGET,
                        asker = asker.as_str(),
                        key = %key,
                        "SET on runtime-command vocab: reserved seam, not wired in M1 (no write)"
                    );
                    vec![Effect::RuntimeCommandReserved {
                        asker: asker.clone(),
                        key: key.clone(),
                        value: value.clone(),
                    }]
                }
                SetClass::StoreWrite => {
                    // 自由 key → asker 別 host 区画へ正準形で反映（reader 往復整合）。
                    let canon = match parse_dotted(key) {
                        Ok(path) => path.to_canonical_string(),
                        Err(e) => {
                            // parse 不能 key の SET（sink 経由の不正 key）→ warn ＋ 生 key を retain
                            // （無音失敗なし・reader 側は resolve 時 NotFound へ縮退）。
                            tracing::warn!(
                                target: LOG_TARGET,
                                asker = asker.as_str(),
                                key = %key,
                                error = %e,
                                "SET free key failed to parse; storing under raw key (tolerant)"
                            );
                            key.clone()
                        }
                    };
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
                SetClass::NotSettable => {
                    // 正準語彙だが SET 無効 → warn ＋ 書込なし（呼出は Ok・正典沈黙裁量・対応表記録）。
                    tracing::warn!(
                        target: LOG_TARGET,
                        asker = asker.as_str(),
                        key = %key,
                        "SET on non-settable canonical vocab: warn and drop (call still Ok, no write)"
                    );
                    vec![Effect::NotSettable {
                        asker: asker.clone(),
                        key: key.clone(),
                        value: value.clone(),
                    }]
                }
            },
            SylphyaMsg::PersistPut { scope, entries, .. } => {
                // write-through: 各 entry を大域点付き区画へ areka.* 正準 key で投影し・スコープ保存。
                let mut effects = Vec::with_capacity(entries.len() + 1);
                for (key, value) in entries {
                    effects.push(Effect::SetDottedGlobal {
                        key: key.to_canonical_key(),
                        value: value.clone(),
                    });
                }
                effects.push(Effect::PersistSave {
                    scope: *scope,
                    entries: entries.clone(),
                });
                effects
            }
            SylphyaMsg::Barrier { .. } => vec![Effect::Barrier],
            SylphyaMsg::Close => vec![Effect::Stop],
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
// Task 5.2: アクター起動・Publisher・Barrier（薄い配線）
// ============================================================================
//
// 掲示板（マテリアライズド・ビュー）モデルの **供給側単独所有者**。`SylphyaCore::apply`
// （5.1・判断分岐の純関数中核）が返す効果列 [`Effect`] を、単一のアクタースレッドが
// 直列に実行する: copy-on-write で後継 [`MirrorImage`] を組み立て・[`SharedMirror::publish`]
// で swap し（single-writer）・[`PersistSave`](Effect::PersistSave) を注入 [`PersistIo`] で
// write-through 保存し・[`Barrier`](Effect::Barrier)/[`PersistPut`](SylphyaMsg::PersistPut) の
// reply を返し・[`Stop`](Effect::Stop) で受信ループを畳む。
//
// ## areka-actor 5 規約（design「Responsibilities & Constraints」）
//
// 1. **inbox = 単一 `Receiver<SylphyaMsg>`**: [`spawn_actor`](areka_actor::spawn_actor) が
//    内部生成した `Receiver` を body が単独所有する（送信端は [`SylphyaPublisher`] が保持）。
// 2. **停止 = `Close`（即時停止・drain せず破棄）**: [`Effect::Stop`] でループを即 `return`。
//    積み残しは `Receiver` の drop で破棄される。
// 3. **unbounded**: `std::sync::mpsc::channel`（[`spawn_actor`] 内）＝無限バッファ。
// 4. **panic はバグ観測として join 検出**: body の panic は [`ActorHandle::join`] が
//    [`ActorError::Panicked`](areka_actor::ActorError) へ写像する（本モジュールは握り潰さない）。
// 5. **拡張機構を足さない**: レジストリ・監督・stop_flag を持たない素の thread spawn。
//
// ## barrier フェンス（Postcondition・design「Ordering / delivery guarantees」）
//
// inbox は mpsc FIFO・アクターは直列処理ゆえ、[`SylphyaPublisher::barrier`] の reply が
// 復帰した時点で、それ以前に **同一送信端** から投函した全メッセージの効果は鏡像へ反映済み
// （publish は各メッセージ処理内で barrier reply 送信より前に行われる）。
//
// ## アクター死亡の観測（design「Error Handling → アクター系」・R6.7）
//
// アクター停止（`Close` 処理後の `return`／panic による unwind）で body が `Receiver` を drop
// すると、以降の送信端 `send` は `SendError` を返す。[`SylphyaPublisher`] の fire-and-forget
// メソッドはこれを **warn 記録して縮退**（panic しない）、[`barrier`](SylphyaPublisher::barrier)
// は [`ReplyError::Dropped`](areka_actor::ReplyError) を返す（＝送信端で死亡を観測可能）。
// 読み手（[`SylphyaReader`]）は最後に publish された鏡像を保持し続けるため、供給停止後も
// 最終鏡像で読み続行できる（表示系を殺さない）。

/// [`spawn_sylphya`] の起動パラメータ（design「Service Interface」）。
pub struct SylphyaInit {
    /// 層別永続スコープの保存先ルート（不在スコープは寛容ロードで空扱い）。
    pub roots: ScopeRoots,
    /// 永続 IO シーム（起動時ロード＋write-through 保存に使用・アクターへ移送）。
    pub io: Box<dyn PersistIo>,
    /// 運行コマンド配送先（M1 は `None`＝未配線・[`Effect::RuntimeCommandReserved`] は warn のみ）。
    pub runtime_sink: Option<Box<dyn RuntimeCommandSink>>,
}

/// [`spawn_sylphya`] の返却物（読み口・送信端・join ハンドルの三点セット・design「Service Interface」）。
pub struct SylphyaParts {
    /// 同期・無待機の読み口（複数消費エンジンへ clone 可）。
    pub reader: SylphyaReader,
    /// 変異投函の送信端（clone 可・複数供給者が共有）。
    pub publisher: SylphyaPublisher,
    /// アクタースレッドの join ハンドル（shutdown 時に join して panic を観測）。
    pub handle: areka_actor::ActorHandle,
}

/// 変異投函の送信端（design「Service Interface」・`areka-actor` envelope 規約）。
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

    /// ④SHIORI 照会層を publish する（`value=None` は 204/失敗＝不在の観測記録）。
    pub fn publish_shiori(&self, asker: AskerId, name: String, value: Option<String>) {
        self.send(SylphyaMsg::PublishShiori { asker, name, value });
    }

    /// SET コマンドを投函する（分類・中継・host 区画書込はアクターの領分）。
    pub fn set(&self, asker: AskerId, key: String, value: String) {
        self.send(SylphyaMsg::Set { asker, key, value });
    }

    /// 永続 put を投函する（write-through・reply なし版＝fire-and-forget）。
    pub fn persist_put(&self, scope: PersistScope, entries: Vec<(PersistKey, String)>) {
        self.send(SylphyaMsg::PersistPut {
            scope,
            entries,
            reply: None,
        });
    }

    /// 反映フェンス: 投函→処理完了を待つ（有界: 呼び側がタイムアウトを課す・design）。
    ///
    /// 復帰時、それ以前に同一送信端から投函した全メッセージは鏡像へ反映済み（mpsc FIFO＋
    /// 直列処理）。アクター死亡（`Close`／panic 後）で投函できない、または reply 端が応答せず
    /// drop された場合は [`ReplyError`](areka_actor::ReplyError) を返す（送信端で死亡観測可能・
    /// R6.7）。
    pub fn barrier(&self) -> Result<(), areka_actor::ReplyError> {
        let (reply, rx) = areka_actor::reply_channel::<()>();
        if self.tx.send(SylphyaMsg::Barrier { reply }).is_err() {
            // アクター死亡 → 投函不能。warn＋Dropped（reader は最終鏡像で継続・表示系を殺さない）。
            tracing::warn!(
                target: LOG_TARGET,
                "sylphya actor stopped; barrier could not be posted (SendError → ReplyError::Dropped)"
            );
            return Err(areka_actor::ReplyError::Dropped);
        }
        rx.recv()
    }

    /// 停止規約: `Close` を投函する（正典終了経路・design「shutdown: Close＋join」）。
    ///
    /// アクターは [`Effect::Stop`] で受信ループを即畳む（積み残しは破棄）。停止済みへの
    /// 再送は warn＋縮退（panic しない）。停止完了は [`SylphyaParts::handle`] の join で待つ。
    pub fn close(&self) {
        self.send(SylphyaMsg::Close);
    }

    /// fire-and-forget 投函の共通経路。アクター死亡時は `SendError` を **warn 記録して縮退**
    /// （panic しない・R8.1／design「アクター系: SendError＝warn＋以降縮退」）。
    fn send(&self, msg: SylphyaMsg) {
        if self.tx.send(msg).is_err() {
            tracing::warn!(
                target: LOG_TARGET,
                "sylphya actor stopped; message dropped (SendError, degrading without panic)"
            );
        }
    }
}

/// sylphya アクターを起動する（design「Service Interface」）。
///
/// 起動時に全永続スコープを寛容ロードして初期鏡像へ投影し（[`build_initial_image`]）、その像を
/// [`SharedMirror`] へ封じてから供給アクターを spawn する。初期像の構築は本関数（＝アクター
/// スレッド生成 **前**）で同期実行するため、返す [`SylphyaReader`] は結線直後から永続復元値を
/// 無待機で読める（single-writer 不変を破らない——構築中は書き手が本スレッド 1 つのみで、
/// 以降の書き手はアクタースレッド 1 つのみ・両者は時間的に重ならない）。
///
/// アクターは `areka-actor` の 5 規約（inbox 単一・`Close` 即時停止・unbounded・panic は
/// join 検出・拡張機構なし）に載る素のスレッド 1 本で、[`SylphyaCore::apply`] の効果列を直列
/// 実行する（single-writer）。結線（どの供給者が何を publish するか）は呼び出し側の領分。
pub fn spawn_sylphya(init: SylphyaInit) -> SylphyaParts {
    let SylphyaInit {
        roots,
        io,
        runtime_sink,
    } = init;

    // 1. 起動時: 全スコープを寛容ロードし初期鏡像を構築（永続 areka.* を大域点付き区画へ投影）。
    let initial = build_initial_image(&roots, io.as_ref());
    let shared = SharedMirror::new(Arc::new(initial));

    // 2. 供給アクターを spawn（inbox は spawn_actor が内部生成・body が Receiver を単独所有）。
    let actor_shared = shared.clone();
    let (tx, handle) = areka_actor::spawn_actor::<SylphyaMsg, _>("sylphya", move |rx| {
        run_actor(rx, actor_shared, roots, io, runtime_sink);
    });

    // 3. 読み口・送信端・ハンドルを返す（reader は初期像を保持する shared を共有）。
    SylphyaParts {
        reader: SylphyaReader::new(shared),
        publisher: SylphyaPublisher { tx },
        handle,
    }
}

/// 全永続スコープを寛容ロードし、初期鏡像（epoch 0）の大域点付き区画へ `areka.*` 正準 key で
/// 投影する（design「Implementation Notes: ロードは spawn init 時に全スコープ実施」）。
///
/// root 不在・読取障害・破損はすべて [`load_scope`] が空へ縮退する（panic なし・R6.7）。
/// 投影先は大域区画（[`MirrorImage::dotted_global`]）——永続は層スコープ別であって asker 別
/// ではなく、reader の per-asker → global 解決順で global 側に載る（[`PersistKey::to_canonical_key`]
/// と reader/persist の往復整合）。
fn build_initial_image(roots: &ScopeRoots, io: &dyn PersistIo) -> MirrorImage {
    let mut img = MirrorImage::empty();
    for scope in [
        PersistScope::App,
        PersistScope::Ghost,
        PersistScope::Shell,
        PersistScope::Balloon,
    ] {
        for (key, value) in load_scope(scope, roots, io) {
            img.dotted_global.insert(key.to_canonical_key(), value);
        }
    }
    img
}

/// アクター受信ループ本体（薄い配線・単一スレッド直列処理）。
///
/// 各メッセージにつき: [`SylphyaCore::apply`] で効果列を算出 → copy-on-write 後継像へ変異効果を
/// 適用 → 実変異があれば [`SharedMirror::publish`]（single-writer）→ [`Effect::PersistSave`] を
/// write-through 保存 → reply（barrier／persist put）を返す → [`Effect::Stop`] で `return`。
/// 全 `Sender` drop（Disconnected）でも正常 `return`（areka-actor 停止規約）。
fn run_actor(
    rx: std::sync::mpsc::Receiver<SylphyaMsg>,
    shared: SharedMirror,
    roots: ScopeRoots,
    io: Box<dyn PersistIo>,
    runtime_sink: Option<Box<dyn RuntimeCommandSink>>,
) {
    let core = SylphyaCore::new();
    loop {
        let msg = match rx.recv() {
            Ok(msg) => msg,
            // 全 Sender drop（Disconnected）→ 正常終了（areka-actor 停止規約）。
            Err(_) => return,
        };

        // 判断分岐は純関数中核が算出（借用）。配線はその効果列を実行するのみ。
        let effects = core.apply(&msg);

        // copy-on-write 後継像を組み立て（変異効果のみ適用）。
        let mut next = shared.load().successor();
        let mut mutated = false;
        let mut persist_outcome: Option<PersistOutcome> = None;
        let mut stop = false;

        for effect in effects {
            match effect {
                Effect::SetFlatPerAsker { asker, name, value } => {
                    next.flat_per_asker
                        .entry(asker)
                        .or_default()
                        .insert(name, value);
                    mutated = true;
                }
                Effect::SetDottedGlobal { key, value } => {
                    next.dotted_global.insert(key, value);
                    mutated = true;
                }
                Effect::SetDottedPerAsker { asker, key, value } => {
                    next.dotted_per_asker
                        .entry(asker)
                        .or_default()
                        .insert(key, value);
                    mutated = true;
                }
                Effect::RecordAbsentFlat { .. } => {
                    // 204/失敗の不在記録: 鏡像へは書かない（既定値は sakura 所有・R4.2）。
                    // apply 側で debug 記録済み（二重ログにしない）。
                }
                Effect::PersistSave { scope, entries } => {
                    // write-through: 注入 IO で当該スコープを原子的保存。失敗は save_scope が
                    // error!＋Degraded で縮退（panic しない・R6.7）——ここは結果を reply へ中継。
                    persist_outcome = Some(save_scope(scope, &roots, io.as_ref(), entries));
                }
                Effect::RuntimeCommandReserved { asker, key, value } => {
                    // SET 運行コマンド。M1 は sink 未登録（apply 側 warn 済み）→ 配送なし。
                    // sink が登録されていれば橋渡し（M2 seriko 等・M1 未配線経路の予約シーム）。
                    if let Some(sink) = runtime_sink.as_ref() {
                        sink.dispatch(&asker, &key, &value);
                    }
                }
                Effect::NotSettable { .. } => {
                    // SET 無効な正準語彙: 書込なし（apply 側 warn 済み・呼出は Ok）。
                }
                Effect::Barrier => {
                    // reply は msg 側から後段で返す（全効果適用＝publish 後にフェンス到達）。
                }
                Effect::Stop => {
                    stop = true;
                }
            }
        }

        // 実変異があった時のみ publish（epoch は実変化に対応・barrier reply より前に確定）。
        if mutated {
            shared.publish(Arc::new(next));
        }

        // reply 送信（reply 端は msg が所有・効果適用＝反映完了の後に返す）。
        match msg {
            SylphyaMsg::Barrier { reply } => {
                // 反映済みをフェンスとして通知（受信端 drop 済みなら未達値を破棄・非 panic）。
                let _ = reply.send(());
            }
            SylphyaMsg::PersistPut {
                reply: Some(reply), ..
            } => {
                let outcome = persist_outcome.unwrap_or(PersistOutcome::Degraded);
                let _ = reply.send(outcome);
            }
            _ => {}
        }

        if stop {
            // Close 即時停止: 積み残しは rx の drop で破棄（areka-actor 停止規約）。
            return;
        }
    }
}

#[cfg(test)]
#[path = "actor_tests.rs"]
mod tests;

/// Task 5.2 アクター統合 決定論檻: spawn→publish→barrier→read の配線・barrier フェンス・
/// アクター死亡観測（SendError／ReplyError／join panic）・reader 無ブロック・write-through 投影。
///
/// 全て純 x64（[`FakePersistIo`]・実 FS なし・時計なし）。同期は **barrier／join** で取り、
/// `thread::sleep` を一切使わない（決定論・記憶知見「檻に入れるのは判断分岐のみ」＋実装第一の
/// 配線檻）。barrier が「それ以前の投函が反映済み」を保証するので、read はレースなく確定する。
#[cfg(test)]
#[path = "actor_actor_integration_tests.rs"]
mod actor_integration_tests;

/// アクター 決定論単体テスト群（Task 5.3・consolidated criterion-mapped cage）。
///
/// design §Testing Strategy → Unit Tests 5（SylphyaCore SET 分類）・4（publish 後の epoch 単調
/// 増加）と Error Handling「アクター系」「SET 系」の判断分岐を決定論檻として集約する。全て
/// x64 純粋（[`FakePersistIo`]・実 FS/実時計/実 SHIORI なし）で、同期は **barrier／join** のみ
/// （`thread::sleep` 皆無・スレッド配線が flake しない）。既存の強い檻は重複させず、監査で判明
/// した 2 つの空隙を補う:
///
/// - **(A) 鏡像 epoch の「アクター publish 経路」単調増加**（R3.3/2.5）: 純核単調増加
///   （`mirror::tests::epoch_monotonic_on_successors`）・`SharedMirror` 直 publish
///   （`mirror::tests::predecessor_image_unchanged_after_publish`）は既存だが、**アクターが
///   `apply` の効果列を実行して行う publish swap** で epoch が単調増加することを突く檻は無い。
///   reader は epoch を露出しない（設計: epoch はフェンス予約シームで読み API に非露出）ため、
///   本檻は spawn 経路を忠実に再現しつつ [`SharedMirror`] の clone を保持し
///   `SharedMirror::load().epoch` で直接観測する。
/// - **(B) アクター死亡後 send の WARN 記録**（R6.7/8.1・無音失敗禁止）: 死亡後投函が panic せず
///   縮退することは既存檻（[`actor_integration_tests::actor_death_via_close_makes_sends_observable_and_reader_continues`]）
///   が突くが、その縮退が **無音でない**（WARN を出す）ことは檻化されていない。本檻は
///   [`crate::test_log_capture::capture`]（interest-keeper・決定論）で fire-and-forget send と
///   barrier の死亡縮退 WARN を捕捉する。
///
/// ## 判断分岐 → 檻 の対応（criterion → test）
///
/// - **SET 分類 3 パターン**（R3.4）: `super::tests::{classify_set_effective_key_is_runtime_command,
///   classify_set_free_dotted_key_is_store_write, classify_set_canonical_non_effective_is_not_settable,
///   classify_set_three_branches_all_reachable, classify_set_unparseable_key_is_store_write}` ＋
///   apply 面 `super::tests::{apply_set_effective_emits_runtime_command_reserved,
///   apply_set_free_emits_host_store_write, apply_set_not_settable_emits_no_write}`（既存 5.1）。
/// - **epoch 単調増加（アクター publish 経路）**（R3.3/2.5）: 本群
///   [`epoch_increments_monotonically_through_actor_publishes`]（初出・観測は保持した
///   `SharedMirror::load().epoch`）。
/// - **Barrier フェンス決定論**（R2.7）: `super::actor_integration_tests::{barrier_fences_all_prior_messages_last_write_wins,
///   spawn_publish_static_barrier_then_read_sees_value, spawn_publish_barrier_read_is_deterministic_over_iterations}`（既存 5.2）。
/// - **アクター死亡縮退**（R6.7/8.1）: 非 panic 送信＋barrier Err＋reader 継続は
///   `super::actor_integration_tests::actor_death_via_close_makes_sends_observable_and_reader_continues`、
///   join panic 検出（[`areka_actor::ActorError::Panicked`]）は
///   `super::actor_integration_tests::actor_panic_is_detected_by_join_and_reader_continues`（既存 5.2）。
///   死亡後 send／barrier の **WARN 記録**（無音失敗禁止の檻）は本群
///   [`send_after_death_logs_warn_not_silent`]（初出・`test_log_capture`）。
#[cfg(test)]
#[path = "actor_actor_criteria_cage.rs"]
mod actor_criteria_cage;

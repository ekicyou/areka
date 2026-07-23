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
use crate::persist::{PersistKey, PersistScope};
use crate::vocab::dotted::{DOTTED_ROOTS, GENERIC_PROP_NAMES, SET_EFFECTIVE};

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
            SylphyaMsg::PublishStatic { asker, flat, dotted } => {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::asker::AskerId;
    use crate::persist::{Axis, PersistKey, PersistScope};

    fn core() -> SylphyaCore {
        SylphyaCore::new()
    }

    fn asker() -> AskerId {
        AskerId::new("ghost/test")
    }

    // === SET 分類 3 分岐（决定论檻・全分岐可達）===

    #[test]
    fn classify_set_effective_key_is_runtime_command() {
        // SET 有効群の正準語彙 → RuntimeCommand。
        assert_eq!(classify_set("surface.num"), SetClass::RuntimeCommand);
        assert_eq!(classify_set("menu"), SetClass::RuntimeCommand);
        assert_eq!(classify_set("seriko.defaultsurface"), SetClass::RuntimeCommand);
    }

    #[test]
    fn classify_set_free_dotted_key_is_store_write() {
        // 正準語彙外の自由 dotted key → StoreWrite（host 区画の受け皿）。
        assert_eq!(classify_set("myplugin.customstate"), SetClass::StoreWrite);
        assert_eq!(classify_set("foo.bar.baz"), SetClass::StoreWrite);
    }

    #[test]
    fn classify_set_canonical_non_effective_is_not_settable() {
        // 正準語彙だが SET 無効 → NotSettable（正典沈黙の areka 裁量）。
        assert_eq!(classify_set("baseware.name"), SetClass::NotSettable);
        assert_eq!(classify_set("system.foo"), SetClass::NotSettable);
        // 汎用名 leaf も正準語彙（username は ShioriQuery・SET 不可）。
        assert_eq!(classify_set("username"), SetClass::NotSettable);
    }

    #[test]
    fn classify_set_three_branches_all_reachable() {
        // 3 分岐が全て到達可能（互いに素）。
        let rc = classify_set("surface.num");
        let sw = classify_set("myplugin.customstate");
        let ns = classify_set("baseware.name");
        assert_ne!(rc, sw);
        assert_ne!(sw, ns);
        assert_ne!(rc, ns);
    }

    #[test]
    fn classify_set_unparseable_key_is_store_write() {
        // parse 不能 key（正準語彙ではない）→ StoreWrite 分岐（決定論・無音失敗なし）。
        assert_eq!(classify_set("a..b"), SetClass::StoreWrite);
        assert_eq!(classify_set(""), SetClass::StoreWrite);
    }

    // === apply の効果列（純関数決定論）===

    #[test]
    fn apply_set_effective_emits_runtime_command_reserved() {
        let msg = SylphyaMsg::Set {
            asker: asker(),
            key: "surface.num".into(),
            value: "5".into(),
        };
        let effects = core().apply(&msg);
        assert_eq!(
            effects,
            vec![Effect::RuntimeCommandReserved {
                asker: asker(),
                key: "surface.num".into(),
                value: "5".into(),
            }]
        );
        // RuntimeCommand は鏡像へ書込まない（reserved seam）。
        assert!(!effects.iter().any(|e| matches!(
            e,
            Effect::SetDottedPerAsker { .. } | Effect::SetDottedGlobal { .. }
        )));
    }

    #[test]
    fn apply_set_free_emits_host_store_write() {
        let msg = SylphyaMsg::Set {
            asker: asker(),
            key: "myplugin.customstate".into(),
            value: "on".into(),
        };
        let effects = core().apply(&msg);
        // 自由 key は asker 別 host（dotted per-asker）区画へ正準形で反映。
        assert_eq!(
            effects,
            vec![Effect::SetDottedPerAsker {
                asker: asker(),
                key: "myplugin.customstate".into(),
                value: "on".into(),
            }]
        );
    }

    #[test]
    fn apply_set_not_settable_emits_no_write() {
        let msg = SylphyaMsg::Set {
            asker: asker(),
            key: "baseware.name".into(),
            value: "x".into(),
        };
        let effects = core().apply(&msg);
        assert_eq!(
            effects,
            vec![Effect::NotSettable {
                asker: asker(),
                key: "baseware.name".into(),
                value: "x".into(),
            }]
        );
        // 書込効果は一切出ない（呼出は Ok だが非反映）。
        assert!(!effects.iter().any(|e| matches!(
            e,
            Effect::SetDottedPerAsker { .. }
                | Effect::SetDottedGlobal { .. }
                | Effect::SetFlatPerAsker { .. }
        )));
    }

    #[test]
    fn apply_publish_shiori_some_sets_flat_per_asker() {
        let msg = SylphyaMsg::PublishShiori {
            asker: asker(),
            name: "username".into(),
            value: Some("Alice".into()),
        };
        assert_eq!(
            core().apply(&msg),
            vec![Effect::SetFlatPerAsker {
                asker: asker(),
                name: "username".into(),
                value: "Alice".into(),
            }]
        );
    }

    #[test]
    fn apply_publish_shiori_none_records_absent_no_default() {
        let msg = SylphyaMsg::PublishShiori {
            asker: asker(),
            name: "username".into(),
            value: None,
        };
        let effects = core().apply(&msg);
        // 204/失敗 → 不在の観測記録のみ。既定値は sakura 所有ゆえ鏡像へ書かない。
        assert_eq!(
            effects,
            vec![Effect::RecordAbsentFlat {
                asker: asker(),
                name: "username".into(),
            }]
        );
        assert!(!effects.iter().any(|e| matches!(e, Effect::SetFlatPerAsker { .. })));
    }

    #[test]
    fn apply_publish_static_flat_per_asker_and_dotted_global() {
        let msg = SylphyaMsg::PublishStatic {
            asker: asker(),
            flat: vec![
                ("selfname".into(), "さくら".into()),
                ("keroname".into(), "うにゅう".into()),
            ],
            dotted: vec![
                ("baseware.name".into(), "areka".into()),
                ("baseware.version".into(), "1.0".into()),
            ],
        };
        let effects = core().apply(&msg);
        assert_eq!(
            effects,
            vec![
                Effect::SetFlatPerAsker {
                    asker: asker(),
                    name: "selfname".into(),
                    value: "さくら".into(),
                },
                Effect::SetFlatPerAsker {
                    asker: asker(),
                    name: "keroname".into(),
                    value: "うにゅう".into(),
                },
                Effect::SetDottedGlobal {
                    key: "baseware.name".into(),
                    value: "areka".into(),
                },
                Effect::SetDottedGlobal {
                    key: "baseware.version".into(),
                    value: "1.0".into(),
                },
            ]
        );
    }

    #[test]
    fn apply_persist_put_projects_to_dotted_global_and_saves() {
        let entries = vec![
            (PersistKey::WindowPos { scope: 0, axis: Axis::X }, "10".to_string()),
            (PersistKey::BootCount, "3".to_string()),
        ];
        let msg = SylphyaMsg::PersistPut {
            scope: PersistScope::Ghost,
            entries: entries.clone(),
            reply: None,
        };
        let effects = core().apply(&msg);
        assert_eq!(
            effects,
            vec![
                Effect::SetDottedGlobal {
                    key: "areka.window.scope(0).x".into(),
                    value: "10".into(),
                },
                Effect::SetDottedGlobal {
                    key: "areka.boot.count".into(),
                    value: "3".into(),
                },
                Effect::PersistSave {
                    scope: PersistScope::Ghost,
                    entries,
                },
            ]
        );
    }

    #[test]
    fn apply_close_emits_stop() {
        assert_eq!(core().apply(&SylphyaMsg::Close), vec![Effect::Stop]);
    }

    #[test]
    fn apply_barrier_emits_barrier() {
        let (tx, _rx) = areka_actor::reply_channel::<()>();
        let msg = SylphyaMsg::Barrier { reply: tx };
        assert_eq!(core().apply(&msg), vec![Effect::Barrier]);
    }

    // === 決定論（同一入力 → 同一効果列・I/O なし）===

    #[test]
    fn apply_is_deterministic_across_variants() {
        let c = core();
        let msgs = vec![
            SylphyaMsg::PublishStatic {
                asker: asker(),
                flat: vec![("selfname".into(), "x".into())],
                dotted: vec![("baseware.name".into(), "areka".into())],
            },
            SylphyaMsg::PublishShiori {
                asker: asker(),
                name: "username".into(),
                value: None,
            },
            SylphyaMsg::Set {
                asker: asker(),
                key: "myplugin.k".into(),
                value: "v".into(),
            },
            SylphyaMsg::Set {
                asker: asker(),
                key: "surface.num".into(),
                value: "1".into(),
            },
            SylphyaMsg::Close,
        ];
        // 10 周: 同一入力 → 同一効果列（決定論檻）。
        for _ in 0..10 {
            for m in &msgs {
                assert_eq!(c.apply(m), c.apply(m));
            }
        }
    }

    #[test]
    fn apply_persist_put_deterministic_same_ref() {
        // reply を含む msg は Clone 不能ゆえ同一参照で 2 回呼んで決定論を確認。
        let msg = SylphyaMsg::PersistPut {
            scope: PersistScope::Ghost,
            entries: vec![(PersistKey::VanishCount, "0".to_string())],
            reply: None,
        };
        let c = core();
        assert_eq!(c.apply(&msg), c.apply(&msg));
    }

    // RuntimeCommandSink は型予約のみ（M1 未配線）。トレイトが Send で dispatch を持つことを型で確認。
    struct NoopSink;
    impl RuntimeCommandSink for NoopSink {
        fn dispatch(&self, _asker: &AskerId, _key: &str, _value: &str) {}
    }

    #[test]
    fn runtime_command_sink_trait_is_reserved() {
        fn assert_send<T: Send>() {}
        assert_send::<NoopSink>();
        let sink: Box<dyn RuntimeCommandSink> = Box::new(NoopSink);
        sink.dispatch(&asker(), "surface.num", "1");
    }
}

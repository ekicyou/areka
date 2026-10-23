//! `--dump-bind` の読みと、スコープの着せ替えへの当て方（要件 5.1〜5.9）。

use areka_sakura::ActorKey;
use areka_seriko::{BindResolver, NamedBindResult, ScopeStates, apply_named_bind, scope_namespace};

/// 当てるパーツ（1 つか、カテゴリ全部）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PartPick {
    One(String),
    All,
}

/// `カテゴリ,パーツ[,ON/OFF]` の 1 つ（名前は書かれたまま・前後の空白も取らない）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BindSpec {
    category: String,
    part: PartPick,
    on: bool,
}

/// 読めない指定は warn! を出して飛ばす。
#[allow(dead_code)] // 呼び手はオフラインの書き出しの走り（タスク 7.8）
pub(crate) fn read_bind_specs(raw: &str) -> Vec<BindSpec> {
    raw.split(';')
        .filter(|spec| !spec.is_empty())
        .filter_map(|spec| {
            let read = read_spec(spec);
            if read.is_none() {
                tracing::warn!(
                    event = "offline_dump_bind_unreadable",
                    spec,
                    "[offline_dump] --dump-bind の指定を カテゴリ,パーツ[,ON/OFF] として読めないので、この指定を飛ばします"
                );
            }
            read
        })
        .collect()
}

/// `カテゴリ,パーツ[,ON/OFF]` を読む（ON/OFF は `1`・`true`・省略で ON、`0`・`false` で OFF）。
fn read_spec(spec: &str) -> Option<BindSpec> {
    let fields: Vec<&str> = spec.split(',').collect();
    let (category, part, on) = match fields[..] {
        [category, part] | [category, part, "1" | "true"] => (category, part, true),
        [category, part, "0" | "false"] => (category, part, false),
        _ => return None,
    };
    let part = match part {
        "" | "__ALL__" => PartPick::All,
        part => PartPick::One(part.to_owned()),
    };
    Some(BindSpec {
        category: category.to_owned(),
        part,
        on,
    })
}

/// 書かれた順に当てる。無い名前は warn!、mustselect の OFF は info!。
#[allow(dead_code)] // 呼び手はオフラインの書き出しの走り（タスク 7.8）
pub(crate) fn apply_bind_specs(
    specs: &[BindSpec],
    scope: u32,
    resolver: &BindResolver,
    states: &mut ScopeStates,
) {
    let key = ActorKey::from(scope.to_string().as_str());
    for spec in specs {
        let category = spec.category.as_str();
        let parts = match &spec.part {
            PartPick::One(part) => vec![part.clone()],
            PartPick::All => scope_namespace(&key)
                .map(|ns| resolver.category_parts_in_order(ns, category))
                .unwrap_or_default(),
        };
        if parts.is_empty() {
            warn_missing(scope, category, "__ALL__");
        }
        for part in &parts {
            match apply_named_bind(states, resolver, &key, category, part, spec.on) {
                NamedBindResult::Applied { .. } => {}
                NamedBindResult::NoNamespace | NamedBindResult::NotFound => {
                    warn_missing(scope, category, part)
                }
                NamedBindResult::MustSelectKept { .. } => tracing::info!(
                    event = "offline_dump_bind_mustselect_kept",
                    scope,
                    category,
                    part = part.as_str(),
                    "[offline_dump] --dump-bind の OFF は mustselect のカテゴリなので外しません"
                ),
            }
        }
    }
}

fn warn_missing(scope: u32, category: &str, part: &str) {
    tracing::warn!(
        event = "offline_dump_bind_missing",
        scope,
        category,
        part,
        "[offline_dump] --dump-bind のカテゴリかパーツがそのスコープのシェルに無いので、この指定を飛ばします"
    );
}

#[cfg(test)]
#[path = "bind_spec_tests.rs"]
mod bind_spec_tests;

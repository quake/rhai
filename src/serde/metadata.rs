//! Serialization of functions metadata.
#![cfg(feature = "metadata")]

use crate::api::type_names::format_type;
use crate::module::{calc_native_fn_hash, FuncInfo};
use crate::{calc_fn_hash, Engine, FnAccess, SmartString, StaticVec, AST};
use serde::Serialize;
#[cfg(feature = "no_std")]
use std::prelude::v1::*;
use std::{borrow::Cow, cmp::Ordering, collections::BTreeMap};

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize)]
#[serde(rename_all = "camelCase")]
enum FnType {
    Script,
    Native,
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize)]
#[serde(rename_all = "camelCase")]
struct FnParam<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<&'a str>,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub typ: Option<Cow<'a, str>>,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize)]
#[serde(rename_all = "camelCase")]
struct FnMetadata<'a> {
    pub base_hash: u64,
    pub full_hash: u64,
    #[cfg(not(feature = "no_module"))]
    pub namespace: crate::FnNamespace,
    pub access: FnAccess,
    pub name: &'a str,
    #[serde(rename = "type")]
    pub typ: FnType,
    pub num_params: usize,
    #[serde(default, skip_serializing_if = "StaticVec::is_empty")]
    pub params: StaticVec<FnParam<'a>>,
    // No idea why the following is needed otherwise serde comes back with a lifetime error
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub _dummy: Option<&'a str>,
    #[serde(default, skip_serializing_if = "str::is_empty")]
    pub return_type: Cow<'a, str>,
    pub signature: SmartString,
    #[serde(default, skip_serializing_if = "StaticVec::is_empty")]
    pub doc_comments: StaticVec<&'a str>,
}

impl PartialOrd for FnMetadata<'_> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for FnMetadata<'_> {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.name.cmp(other.name) {
            Ordering::Equal => self.num_params.cmp(&other.num_params),
            cmp => cmp,
        }
    }
}

impl<'a> From<&'a FuncInfo> for FnMetadata<'a> {
    fn from(info: &'a FuncInfo) -> Self {
        let base_hash = calc_fn_hash(&info.name, info.num_params);
        let (typ, full_hash) = if info.func.is_script() {
            (FnType::Script, base_hash)
        } else {
            (
                FnType::Native,
                calc_native_fn_hash(None, &info.name, &info.param_types),
            )
        };

        Self {
            base_hash,
            full_hash,
            #[cfg(not(feature = "no_module"))]
            namespace: info.namespace,
            access: info.access,
            name: &info.name,
            typ,
            num_params: info.num_params,
            params: info
                .params_info
                .iter()
                .map(|s| {
                    let mut seg = s.splitn(2, ':');
                    let name = match seg.next().unwrap().trim() {
                        "_" => None,
                        s => Some(s),
                    };
                    let typ = seg.next().map(|s| format_type(s, false));
                    FnParam { name, typ }
                })
                .collect(),
            _dummy: None,
            return_type: format_type(&info.return_type, true),
            signature: info.gen_signature().into(),
            doc_comments: if info.func.is_script() {
                #[cfg(feature = "no_function")]
                unreachable!("script-defined functions should not exist under no_function");

                #[cfg(not(feature = "no_function"))]
                info.func
                    .get_script_fn_def()
                    .expect("script-defined function")
                    .comments
                    .iter()
                    .map(<_>::as_ref)
                    .collect()
            } else {
                info.comments.iter().map(<_>::as_ref).collect()
            },
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ModuleMetadata<'a> {
    #[cfg(feature = "metadata")]
    #[serde(skip_serializing_if = "str::is_empty")]
    pub doc: &'a str,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub modules: BTreeMap<&'a str, Self>,
    #[serde(skip_serializing_if = "StaticVec::is_empty")]
    pub functions: StaticVec<FnMetadata<'a>>,
}

impl ModuleMetadata<'_> {
    #[inline(always)]
    pub fn new() -> Self {
        Self {
            #[cfg(feature = "metadata")]
            doc: "",
            modules: BTreeMap::new(),
            functions: StaticVec::new_const(),
        }
    }
}

impl<'a> From<&'a crate::Module> for ModuleMetadata<'a> {
    fn from(module: &'a crate::Module) -> Self {
        let mut functions: StaticVec<_> = module.iter_fn().map(Into::into).collect();
        functions.sort();

        Self {
            doc: module.doc(),
            modules: module
                .iter_sub_modules()
                .map(|(name, m)| (name, m.as_ref().into()))
                .collect(),
            functions,
        }
    }
}

/// Generate a list of all functions in JSON format.
pub fn gen_metadata_to_json(
    engine: &Engine,
    ast: Option<&AST>,
    include_standard_packages: bool,
) -> serde_json::Result<String> {
    let _ast = ast;
    let mut global = ModuleMetadata::new();

    #[cfg(not(feature = "no_module"))]
    for (name, m) in &engine.global_sub_modules {
        global.modules.insert(name, m.as_ref().into());
    }

    engine
        .global_modules
        .iter()
        .filter(|m| include_standard_packages || !m.standard)
        .flat_map(|m| m.iter_fn())
        .for_each(|f| {
            #[allow(unused_mut)]
            let mut meta: FnMetadata = f.into();
            #[cfg(not(feature = "no_module"))]
            {
                meta.namespace = crate::FnNamespace::Global;
            }
            global.functions.push(meta);
        });

    #[cfg(not(feature = "no_function"))]
    if let Some(ast) = _ast {
        for f in ast.shared_lib().iter_fn() {
            #[allow(unused_mut)]
            let mut meta: FnMetadata = f.into();
            #[cfg(not(feature = "no_module"))]
            {
                meta.namespace = crate::FnNamespace::Global;
            }
            global.functions.push(meta);
        }
    }

    global.functions.sort();

    #[cfg(feature = "metadata")]
    if let Some(ast) = _ast {
        global.doc = ast.doc();
    }

    serde_json::to_string_pretty(&global)
}

#[cfg(feature = "internals")]
impl crate::api::definitions::Definitions<'_> {
    /// Generate a list of all functions in JSON format.
    ///
    /// Functions from the following sources are included:
    /// 1) Functions defined in an [`AST`][crate::AST]
    /// 2) Functions registered into the global namespace
    /// 3) Functions in static modules
    /// 4) Functions in registered global packages
    /// 5) Functions in standard packages (optional)
    #[inline(always)]
    pub fn json(&self) -> serde_json::Result<String> {
        gen_metadata_to_json(self.engine(), None, self.config().include_standard_packages)
    }
}

impl Engine {
    /// _(metadata)_ Generate a list of all functions (including those defined in an
    /// [`AST`][crate::AST]) in JSON format.
    /// Exported under the `metadata` feature only.
    ///
    /// Functions from the following sources are included:
    /// 1) Functions defined in an [`AST`][crate::AST]
    /// 2) Functions registered into the global namespace
    /// 3) Functions in static modules
    /// 4) Functions in registered global packages
    /// 5) Functions in standard packages (optional)
    #[inline(always)]
    pub fn gen_fn_metadata_with_ast_to_json(
        &self,
        ast: &AST,
        include_standard_packages: bool,
    ) -> serde_json::Result<String> {
        gen_metadata_to_json(self, Some(ast), include_standard_packages)
    }

    /// Generate a list of all functions in JSON format.
    /// Exported under the `metadata` feature only.
    ///
    /// Functions from the following sources are included:
    /// 1) Functions registered into the global namespace
    /// 2) Functions in static modules
    /// 3) Functions in registered global packages
    /// 4) Functions in standard packages (optional)
    #[inline(always)]
    pub fn gen_fn_metadata_to_json(
        &self,
        include_standard_packages: bool,
    ) -> serde_json::Result<String> {
        gen_metadata_to_json(self, None, include_standard_packages)
    }
}

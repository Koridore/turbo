use turbo_tasks_fs::FileSystemPathVc;
use turbopack_core::resolve::options::{ImportMap, ImportMapVc, ImportMapping, ImportMappingVc};

use crate::embed_js::{attached_next_js_package_path, VIRTUAL_PACKAGE_NAME};

/// Computes the Next-specific client import map.
#[turbo_tasks::function]
pub fn get_next_client_import_map(
    project_path: FileSystemPathVc,
    pages_dir: FileSystemPathVc,
) -> ImportMapVc {
    let mut import_map = ImportMap::empty();
    let package_root = attached_next_js_package_path(project_path);

    insert_next_shared_aliases(&mut import_map, package_root);

    insert_alias_to_alternatives(
        &mut import_map,
        format!("{VIRTUAL_PACKAGE_NAME}/pages/_app"),
        vec![
            request_to_import_mapping(pages_dir, "./_app"),
            request_to_import_mapping(pages_dir, "next/app"),
        ],
    );
    insert_alias_to_alternatives(
        &mut import_map,
        format!("{VIRTUAL_PACKAGE_NAME}/pages/_document"),
        vec![
            request_to_import_mapping(pages_dir, "./_document"),
            request_to_import_mapping(pages_dir, "next/document"),
        ],
    );

    import_map.cell()
}

/// Computes the Next-specific client fallback import map, which provides
/// polyfills to Node.js externals.
#[turbo_tasks::function]
pub fn get_next_client_fallback_import_map(pages_dir: FileSystemPathVc) -> ImportMapVc {
    let mut import_map = ImportMap::empty();

    for (original, alias) in NEXT_ALIASES {
        import_map.insert_exact_alias(original, request_to_import_mapping(pages_dir, alias));
        import_map.insert_exact_alias(
            format!("node:{original}"),
            request_to_import_mapping(pages_dir, alias),
        );
    }

    import_map.cell()
}

/// Computes the Next-specific server-side import map.
#[turbo_tasks::function]
pub fn get_next_server_import_map(
    project_path: FileSystemPathVc,
    pages_dir: FileSystemPathVc,
) -> ImportMapVc {
    let mut import_map = ImportMap::empty();
    let package_root = attached_next_js_package_path(project_path);

    insert_next_shared_aliases(&mut import_map, package_root);

    insert_alias_to_alternatives(
        &mut import_map,
        format!("{VIRTUAL_PACKAGE_NAME}/pages/_app"),
        vec![
            request_to_import_mapping(pages_dir, "./_app"),
            external_request_to_import_mapping("next/app"),
        ],
    );
    insert_alias_to_alternatives(
        &mut import_map,
        format!("{VIRTUAL_PACKAGE_NAME}/pages/_document"),
        vec![
            request_to_import_mapping(pages_dir, "./_document"),
            external_request_to_import_mapping("next/document"),
        ],
    );

    import_map.insert_exact_alias("next", ImportMapping::External(None).into());
    import_map.insert_wildcard_alias("next/", ImportMapping::External(None).into());
    import_map.insert_exact_alias("react", ImportMapping::External(None).into());
    import_map.insert_wildcard_alias("react/", ImportMapping::External(None).into());

    import_map.cell()
}

static NEXT_ALIASES: [(&str, &str); 23] = [
    ("asset", "next/dist/compiled/assert"),
    ("buffer", "next/dist/compiled/buffer/"),
    ("constants", "next/dist/compiled/constants-browserify"),
    ("crypto", "next/dist/compiled/crypto-browserify"),
    ("domain", "next/dist/compiled/domain-browser"),
    ("http", "next/dist/compiled/stream-http"),
    ("https", "next/dist/compiled/https-browserify"),
    ("os", "next/dist/compiled/os-browserify"),
    ("path", "next/dist/compiled/path-browserify"),
    ("punycode", "next/dist/compiled/punycode"),
    ("process", "next/dist/build/polyfills/process"),
    ("querystring", "next/dist/compiled/querystring-es3"),
    ("stream", "next/dist/compiled/stream-browserify"),
    ("string_decoder", "next/dist/compiled/string_decoder"),
    ("sys", "next/dist/compiled/util/"),
    ("timers", "next/dist/compiled/timers-browserify"),
    ("tty", "next/dist/compiled/tty-browserify"),
    ("url", "next/dist/compiled/native-url"),
    ("util", "next/dist/compiled/util/"),
    ("vm", "next/dist/compiled/vm-browserify"),
    ("zlib", "next/dist/compiled/browserify-zlib"),
    ("events", "next/dist/compiled/events/"),
    ("setImmediate", "next/dist/compiled/setimmediate"),
];

fn insert_next_shared_aliases(import_map: &mut ImportMap, package_root: FileSystemPathVc) {
    insert_package_alias(
        import_map,
        &format!("{VIRTUAL_PACKAGE_NAME}/"),
        package_root,
    );
}

/// Inserts an alias to an alternative of import mappings into an import map.
fn insert_alias_to_alternatives<'a>(
    import_map: &mut ImportMap,
    alias: impl Into<String> + 'a,
    alternatives: Vec<ImportMappingVc>,
) {
    import_map.insert_exact_alias(alias, ImportMapping::Alternatives(alternatives).into());
}

/// Inserts an alias to an import mapping into an import map.
fn insert_package_alias(import_map: &mut ImportMap, prefix: &str, package_root: FileSystemPathVc) {
    import_map.insert_wildcard_alias(
        prefix,
        ImportMapping::PrimaryAlternative("./*".to_string(), Some(package_root)).cell(),
    );
}

/// Creates a direct import mapping to the result of resolving a request
/// in a context.
fn request_to_import_mapping(context_path: FileSystemPathVc, request: &str) -> ImportMappingVc {
    ImportMapping::PrimaryAlternative(request.to_string(), Some(context_path)).cell()
}

/// Creates a direct import mapping to the result of resolving an external
/// request.
fn external_request_to_import_mapping(request: &str) -> ImportMappingVc {
    ImportMapping::External(Some(request.to_string())).into()
}
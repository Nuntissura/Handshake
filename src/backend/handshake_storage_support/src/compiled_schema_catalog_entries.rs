use std::collections::BTreeSet;

pub fn compiled_schema_catalog_entries(
    schema: &str,
    expected_table_names: &[&str],
    expected_counts: (usize, usize, usize, usize, usize, usize, usize, usize),
) -> Result<Vec<String>, String> {
    let mut entries = BTreeSet::new();
    let mut tables = BTreeSet::new();
    let mut fields = 0usize;
    let mut indexes = 0usize;
    let mut events = 0usize;
    let mut views = 0usize;
    let mut sequences = 0usize;
    let mut accesses = 0usize;
    let mut functions = 0usize;

    for raw_line in schema.lines() {
        let line = raw_line.trim();
        let tokens = line.split_whitespace().collect::<Vec<_>>();
        let identity = match tokens.as_slice() {
            ["DEFINE", "TABLE", "OVERWRITE", name, ..] => {
                let name = name.trim_end_matches(';');
                tables.insert(name.to_owned());
                if line.contains(" TYPE NORMAL AS") {
                    views += 1;
                    insert_catalog_identity(&mut entries, format!("view:{name}"))?;
                }
                Some(format!("table:{name}"))
            }
            ["DEFINE", "FIELD", "OVERWRITE", name, "ON", "TABLE", table, ..] => {
                fields += 1;
                Some(format!(
                    "field:{}:{}",
                    table.trim_end_matches(';'),
                    name.trim_end_matches(';')
                ))
            }
            ["DEFINE", "INDEX", "OVERWRITE", name, "ON", "TABLE", table, ..] => {
                indexes += 1;
                Some(format!(
                    "index:{}:{}",
                    table.trim_end_matches(';'),
                    name.trim_end_matches(';')
                ))
            }
            ["DEFINE", "EVENT", "OVERWRITE", name, "ON", "TABLE", table, ..] => {
                events += 1;
                Some(format!(
                    "event:{}:{}",
                    table.trim_end_matches(';'),
                    name.trim_end_matches(';')
                ))
            }
            ["DEFINE", "SEQUENCE", "OVERWRITE", name, ..] => {
                sequences += 1;
                Some(format!("sequence:{}", name.trim_end_matches(';')))
            }
            ["DEFINE", "SEQUENCE", "IF", "NOT", "EXISTS", name, ..] => {
                sequences += 1;
                Some(format!("sequence:{}", name.trim_end_matches(';')))
            }
            ["DEFINE", "ACCESS", "OVERWRITE", name, "ON", "DATABASE", ..]
            | ["DEFINE", "ACCESS", "IF", "NOT", "EXISTS", name, "ON", "DATABASE", ..] => {
                accesses += 1;
                Some(format!("access:{}", name.trim_end_matches(';')))
            }
            ["DEFINE", "FUNCTION", "OVERWRITE", signature, ..] => {
                functions += 1;
                Some(format!(
                    "function:{}",
                    declarative_function_name(signature)?
                ))
            }
            ["DEFINE", "ACCESS" | "FUNCTION", ..] => {
                return Err(format!(
                    "unsupported declarative authority definition: {line}"
                ));
            }
            _ => None,
        };
        if let Some(identity) = identity {
            insert_catalog_identity(&mut entries, identity)?;
        }
    }

    let expected_tables = expected_table_names
        .iter()
        .map(|name| (*name).to_owned())
        .collect::<BTreeSet<_>>();
    if tables != expected_tables {
        return Err(format!(
            "declarative table inventory differs from TABLE_NAMES: parsed={}; expected={}",
            tables.len(),
            expected_tables.len()
        ));
    }
    let observed_counts = (
        tables.len(),
        fields,
        indexes,
        events,
        views,
        sequences,
        accesses,
        functions,
    );
    if observed_counts != expected_counts {
        return Err(format!(
            "declarative schema catalog counts differ: observed={observed_counts:?}; expected={expected_counts:?}"
        ));
    }

    Ok(entries.into_iter().collect())
}

pub fn declarative_function_name(signature: &str) -> Result<&str, String> {
    let (name, _) = signature
        .split_once('(')
        .ok_or_else(|| format!("function declaration lacks argument opener: {signature}"))?;
    if !name.starts_with("fn::") || name.len() == 4 {
        return Err(format!("invalid declarative function name: {signature}"));
    }
    Ok(name)
}

pub fn insert_catalog_identity(
    entries: &mut BTreeSet<String>,
    identity: String,
) -> Result<(), String> {
    if entries.insert(identity.clone()) {
        Ok(())
    } else {
        Err(format!("duplicate declarative schema identity: {identity}"))
    }
}

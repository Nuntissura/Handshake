use uuid::Uuid;

pub fn character_ref(character_internal_id: Uuid) -> String {
    format!("atelier://character/{character_internal_id}")
}

pub fn sheet_version_ref(character_internal_id: Uuid, version_id: Uuid) -> String {
    format!("atelier://sheet/{character_internal_id}/{version_id}")
}

pub fn parse_character_ref(value: &str) -> Option<Uuid> {
    value
        .strip_prefix("atelier://character/")
        .and_then(|id| Uuid::parse_str(id).ok())
}

pub fn parse_sheet_version_ref(value: &str) -> Option<(Uuid, Uuid)> {
    let rest = value.strip_prefix("atelier://sheet/")?;
    let (character, version) = rest.split_once('/')?;
    let character = Uuid::parse_str(character).ok()?;
    let version = Uuid::parse_str(version).ok()?;
    Some((character, version))
}

pub fn validate_character_ref(value: &str) -> bool {
    parse_character_ref(value).is_some()
}

pub fn validate_sheet_version_ref(value: &str) -> bool {
    parse_sheet_version_ref(value).is_some()
}

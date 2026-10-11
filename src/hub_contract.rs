//! Guards `hub-contract.json`: the settings keys the hub edits must match
//! `AppSettings`, and every schema version keeps its key list in history.

#[cfg(test)]
mod tests {
    use crate::settings::AppSettings;
    use std::collections::BTreeSet;

    fn key_set(value: &serde_json::Value) -> BTreeSet<String> {
        value
            .as_array()
            .expect("keys is an array")
            .iter()
            .map(|k| k.as_str().expect("key is a string").to_string())
            .collect()
    }

    #[test]
    fn contract_matches_code() {
        let contract: serde_json::Value =
            serde_json::from_str(include_str!("../hub-contract.json")).unwrap();
        let history: serde_json::Value =
            serde_json::from_str(include_str!("../tests-data/hub_contract_history.json")).unwrap();

        let contract_keys = key_set(&contract["settings"]["keys"]);
        let code_keys: BTreeSet<String> = serde_json::to_value(AppSettings::default())
            .unwrap()
            .as_object()
            .unwrap()
            .keys()
            .cloned()
            .collect();
        assert_eq!(
            contract_keys, code_keys,
            "AppSettings fields changed: bump settings.schemaVersion in hub-contract.json and add a history entry"
        );

        let version = contract["settings"]["schemaVersion"].as_u64().unwrap();
        let recorded = key_set(&history[version.to_string()]);
        assert_eq!(
            recorded, contract_keys,
            "history entry for schemaVersion {version} differs from the contract keys"
        );

        let flag = contract["hubFlag"].as_str().unwrap().to_string();
        assert!(crate::cli_args::parse(["FastEnglish.exe".to_string(), flag]).hub);
    }
}

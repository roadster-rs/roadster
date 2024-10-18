use itertools::Itertools;
use std::collections::BTreeSet;

fn main() -> anyhow::Result<()> {
    use cargo_manifest::Manifest;

    let groups: Vec<BTreeSet<String>> = vec![
        vec!["email", "email-smtp"],
        vec!["email", "email-sendgrid"],
        vec!["jwt", "jwt-ietf"],
        vec!["jwt", "jwt-openid"],
        vec!["open-api", "http"],
    ]
    .into_iter()
    .map(|v| v.into_iter().map(|s| s.to_string()).collect())
    .collect_vec();

    let manifest = Manifest::from_path("../../Cargo.toml")?;
    let feature_names = manifest
        .features
        .into_iter()
        .flatten()
        .map(|f| f.0)
        .map(|n| {
            let group = groups.iter().find(|g| g.contains(&n));
            if let Some(group) = group {
                group.iter().join(",")
            } else {
                n
            }
        })
        .unique()
        .collect_vec();

    let ps = feature_names.into_iter().powerset().collect_vec();

    let ps = ps
        .into_iter()
        .filter(|s| s.len() > 1 && s.len() <= 3)
        .collect_vec();
    println!("Powerset size: {}", ps.len());

    Ok(())
}

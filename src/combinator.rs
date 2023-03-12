pub fn combine_path(prefix: &str, suffix: &str) -> String {
    // TODO: avoid allocating another string
    let suffix = suffix
        .chars()
        .filter_map(|c| match c {
            '-' | '_' => None,
            x => Some(x.to_ascii_lowercase()),
        })
        .collect::<String>();
    if prefix.is_empty() {
        return suffix;
    }
    format!("{prefix}.{suffix}",)
}

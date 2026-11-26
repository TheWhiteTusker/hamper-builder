//! Where a save or delete lands: the list as it was left (its filters kept),
//! but only ever that list. The "moved to Bin" banner is set for a delete and
//! dropped otherwise. Port of `list-return.ts`.

pub fn list_return(kind: &str, saved: &str, deleted: bool) -> String {
    let (path, query) = saved.split_once('?').unwrap_or((saved, ""));
    let mut params: Vec<&str> = if path == format!("/{kind}") {
        query.split('&').filter(|p| !p.is_empty() && *p != "deleted" && !p.starts_with("deleted=")).collect()
    } else {
        Vec::new()
    };
    if deleted {
        params.push("deleted=1");
    }
    if params.is_empty() { format!("/{kind}") } else { format!("/{kind}?{}", params.join("&")) }
}

#[cfg(test)]
mod tests {
    use super::list_return;

    #[test]
    fn keeps_the_lists_filters_and_nothing_else() {
        assert_eq!(list_return("hampers", "/hampers?q=gold&limit=100", false), "/hampers?q=gold&limit=100");
        assert_eq!(list_return("hampers", "/hampers?deleted=1&q=x", false), "/hampers?q=x");
        assert_eq!(list_return("products", "/products?category=7", true), "/products?category=7&deleted=1");
        for bad in ["//evil.com/hampers", "https://evil.com/hampers", "/products?q=1", "javascript:alert(1)", ""] {
            assert_eq!(list_return("hampers", bad, false), "/hampers", "{bad}");
        }
        assert_eq!(list_return("hampers", "", true), "/hampers?deleted=1");
    }
}

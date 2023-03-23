pub fn extract_site_url(feed_url: String, links: Vec<feed_rs::model::Link>) -> String {
    let link = links.iter().find_map(|link| {
        if link
            .rel
            .as_ref()
            .filter(|rel| rel.as_str() == "self")
            .is_some()
        {
            None
        } else {
            Some(link)
        }
    });

    if let Some(link) = link.filter(|link| !is_feed_url(link)) {
        return trim_end(link.href.to_owned());
    }

    trim_end(feed_url)
}

fn is_feed_url(link: &feed_rs::model::Link) -> bool {
    link.href.ends_with(".xml")
        || link.href.ends_with(".atom")
        || link.href.ends_with("rss/")
        || link.href.ends_with("rss")
        || link.href.ends_with("atom/")
        || link.href.ends_with("atom")
        || link.href.ends_with("feed")
        || link.href.ends_with("feed/")
}

fn trim_end(url: String) -> String {
    url.trim_end_matches('/')
        .trim_end_matches(|c| c == '/')
        .to_owned()
}

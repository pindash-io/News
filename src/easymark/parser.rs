use scraper::Html;

pub fn parser(raw: impl AsRef<str>) -> Html {
    Html::parse_fragment(raw.as_ref())
}

use askama::Template;
use salvo::prelude::Text;
use salvo::{handler, Response};

#[derive(Debug, Template)]
#[template(path = "index.html.j2")]
struct IndexTemplate {}

#[handler]
pub async fn index_page(res: &mut Response) {
    let index_template = IndexTemplate {};
    res.render(Text::Html(index_template.render().unwrap()));
}
